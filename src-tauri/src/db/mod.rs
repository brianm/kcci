use rusqlite::{ffi::sqlite3_auto_extension, params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Once;

use crate::commands::Filter;
use crate::error::Result;

static SQLITE_VEC_INIT: Once = Once::new();

/// Book data from the books table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub asin: String,
    pub title: String,
    pub authors: Vec<String>,
    pub cover_url: Option<String>,
    pub percent_read: i32,
    pub resource_type: Option<String>,
    pub origin_type: Option<String>,
}

/// Book with full metadata for display/search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookWithMeta {
    pub asin: String,
    pub title: String,
    pub authors: Vec<String>,
    pub cover_url: Option<String>,
    pub percent_read: i32,
    pub resource_type: Option<String>,
    pub origin_type: Option<String>,
    pub description: Option<String>,
    pub subjects: Vec<String>,
    pub publish_year: Option<i32>,
    pub isbn: Option<String>,
    pub openlibrary_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<f64>,
}

/// Book for embedding (has enriched metadata)
#[derive(Debug, Clone)]
pub struct BookForEmbedding {
    pub asin: String,
    pub title: String,
    pub authors: Vec<String>,
    pub description: String,
}

/// Database statistics
#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    pub total_books: usize,
    pub enriched: usize,
    pub with_embeddings: usize,
}

/// Enrichment result to save
#[derive(Debug, Clone)]
pub struct EnrichmentData {
    pub openlibrary_key: String,
    pub description: String,
    pub subjects: Vec<String>,
    pub isbn: Option<String>,
    pub publish_year: Option<i32>,
}

/// A single search filter chip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilter {
    /// Field to search: "all", "title", "author", "description", "subject"
    pub field: String,
    /// Search term
    pub value: String,
}

/// Imported book from webarchive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedBook {
    pub asin: String,
    pub title: String,
    pub authors: Vec<String>,
    pub cover_url: Option<String>,
    pub percentage_read: i32,
    pub resource_type: String,
    pub origin_type: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create the database at the given path
    pub fn open(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Register sqlite-vec extension once (before opening any connection)
        SQLITE_VEC_INIT.call_once(|| unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        });

        let conn = Connection::open(&path)?;

        Ok(Self { conn })
    }

    /// Initialize the database schema
    pub fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(include_str!("schema.sql"))?;

        // vec0 table needs separate CREATE (can't be in batch)
        self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS books_vec USING vec0(
                asin TEXT PRIMARY KEY,
                embedding FLOAT[768]
            )",
            [],
        )?;

        Ok(())
    }

    /// Import books from webarchive parse result
    pub fn import_books(&self, books: &[ImportedBook]) -> Result<usize> {
        let mut count = 0;
        for book in books {
            let authors_json = serde_json::to_string(&book.authors)?;
            let rows = self.conn.execute(
                "INSERT OR IGNORE INTO books (asin, title, authors, cover_url, percent_read, resource_type, origin_type)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    book.asin,
                    book.title,
                    authors_json,
                    book.cover_url,
                    book.percentage_read,
                    book.resource_type,
                    book.origin_type,
                ],
            )?;
            count += rows;
        }
        Ok(count)
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<Stats> {
        let total_books: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM books", [], |row| row.get(0))?;
        let enriched: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM metadata", [], |row| row.get(0))?;
        let with_embeddings: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM books_vec", [], |row| row.get(0))?;

        Ok(Stats {
            total_books,
            enriched,
            with_embeddings,
        })
    }

    /// Full-text search across title, authors, description
    pub fn search_fts(&self, query: &str, limit: usize) -> Result<Vec<BookWithMeta>> {
        let mut stmt = self.conn.prepare(
            "SELECT b.asin, b.title, b.authors, b.cover_url, b.percent_read,
                    b.resource_type, b.origin_type,
                    m.description, m.subjects, m.publish_year, m.isbn, m.openlibrary_key,
                    bm25(books_fts) as rank
             FROM books_fts f
             JOIN books_fts_content c ON f.rowid = c.rowid
             JOIN books b ON c.asin = b.asin
             LEFT JOIN metadata m ON b.asin = m.asin
             WHERE books_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let rows = stmt.query_map(params![query, limit], map_book_row_with_rank)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    /// Search with structured filters (chips)
    /// Each filter is AND-ed together
    pub fn search_filtered(
        &self,
        filters: &[SearchFilter],
        limit: usize,
        offset: usize,
        sort_by: Option<&str>,
        sort_dir: Option<&str>,
    ) -> Result<Vec<BookWithMeta>> {
        if filters.is_empty() {
            // No filters, return all books
            return self.get_all_books(limit, offset, sort_by, sort_dir, &[]);
        }

        // Build FTS MATCH query from filters
        // FTS5 column filter syntax: column:term or column:"phrase"
        // Multiple terms are AND-ed by default
        //
        // Special handling for "author" field: split into separate terms because
        // authors are stored as JSON arrays like ["Card", "Orson Scott"] where
        // name parts aren't adjacent. Other fields use phrase search.
        let match_parts: Vec<String> = filters
            .iter()
            .flat_map(|f| {
                let escaped_value = f.value.replace('"', "\"\"");

                match f.field.as_str() {
                    "author" => {
                        // Split author into words: "orson card" -> authors:"orson" authors:"card"
                        f.value
                            .split_whitespace()
                            .map(|word| {
                                let escaped = word.replace('"', "\"\"");
                                format!("authors:\"{}\"", escaped)
                            })
                            .collect::<Vec<_>>()
                    }
                    "title" => vec![format!("title:\"{}\"", escaped_value)],
                    "description" => vec![format!("description:\"{}\"", escaped_value)],
                    "subject" => vec![format!("subjects:\"{}\"", escaped_value)],
                    // "all" or unknown: search all columns as phrase
                    _ => vec![format!("\"{}\"", escaped_value)],
                }
            })
            .collect();

        let match_query = match_parts.join(" ");

        let order_clause = match sort_by {
            Some("author") => {
                let dir = if sort_dir == Some("desc") { "DESC" } else { "ASC" };
                format!("json_extract(b.authors, '$[0]') {}", dir)
            }
            Some("year") => {
                let dir = if sort_dir == Some("desc") {
                    "DESC NULLS LAST"
                } else {
                    "ASC NULLS LAST"
                };
                format!("m.publish_year {}", dir)
            }
            Some("rank") => "rank".to_string(),
            _ => {
                let dir = if sort_dir == Some("desc") { "DESC" } else { "ASC" };
                format!("b.title {}", dir)
            }
        };

        let sql = format!(
            "SELECT b.asin, b.title, b.authors, b.cover_url, b.percent_read,
                    b.resource_type, b.origin_type,
                    m.description, m.subjects, m.publish_year, m.isbn, m.openlibrary_key,
                    bm25(books_fts) as rank
             FROM books_fts f
             JOIN books_fts_content c ON f.rowid = c.rowid
             JOIN books b ON c.asin = b.asin
             LEFT JOIN metadata m ON b.asin = m.asin
             WHERE books_fts MATCH ?1
             ORDER BY {}
             LIMIT ?2 OFFSET ?3",
            order_clause
        );

        let mut stmt = self.conn.prepare(&sql)?;

        let rows = stmt.query_map(params![match_query, limit, offset], map_book_row_with_rank)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    /// Get count of books matching filters
    pub fn get_filtered_count(&self, filters: &[SearchFilter]) -> Result<usize> {
        if filters.is_empty() {
            return self.get_book_count();
        }

        // Same logic as search_filtered: only split author field
        let match_parts: Vec<String> = filters
            .iter()
            .flat_map(|f| {
                let escaped_value = f.value.replace('"', "\"\"");

                match f.field.as_str() {
                    "author" => f
                        .value
                        .split_whitespace()
                        .map(|word| {
                            let escaped = word.replace('"', "\"\"");
                            format!("authors:\"{}\"", escaped)
                        })
                        .collect::<Vec<_>>(),
                    "title" => vec![format!("title:\"{}\"", escaped_value)],
                    "description" => vec![format!("description:\"{}\"", escaped_value)],
                    "subject" => vec![format!("subjects:\"{}\"", escaped_value)],
                    _ => vec![format!("\"{}\"", escaped_value)],
                }
            })
            .collect();

        let match_query = match_parts.join(" ");

        let count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM books_fts WHERE books_fts MATCH ?1",
            params![match_query],
            |row| row.get(0),
        )?;

        Ok(count)
    }

    /// Semantic search using vector similarity
    pub fn search_semantic(&self, embedding: &[f32], limit: usize) -> Result<Vec<BookWithMeta>> {
        let blob = serialize_embedding(embedding);

        let mut stmt = self.conn.prepare(
            "SELECT b.asin, b.title, b.authors, b.cover_url, b.percent_read,
                    b.resource_type, b.origin_type,
                    m.description, m.subjects, m.publish_year, m.isbn, m.openlibrary_key,
                    v.distance
             FROM books_vec v
             JOIN books b ON v.asin = b.asin
             LEFT JOIN metadata m ON b.asin = m.asin
             WHERE embedding MATCH ?1
               AND k = ?2
             ORDER BY distance",
        )?;

        let rows = stmt.query_map(params![blob, limit], map_book_row_with_distance)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    /// Get paginated list of all books with optional sorting and filtering
    pub fn get_all_books(
        &self,
        limit: usize,
        offset: usize,
        sort_by: Option<&str>,
        sort_dir: Option<&str>,
        filters: &[Filter],
    ) -> Result<Vec<BookWithMeta>> {
        let order_clause = match sort_by {
            Some("author") => {
                let dir = if sort_dir == Some("desc") { "DESC" } else { "ASC" };
                format!("json_extract(b.authors, '$[0]') {}", dir)
            }
            Some("year") => {
                let dir = if sort_dir == Some("desc") { "DESC NULLS LAST" } else { "ASC NULLS LAST" };
                format!("m.publish_year {}", dir)
            }
            _ => {
                let dir = if sort_dir == Some("desc") { "DESC" } else { "ASC" };
                format!("b.title {}", dir)
            }
        };

        let (where_clause, params) = build_filter_clause(filters);

        let sql = format!(
            "SELECT b.asin, b.title, b.authors, b.cover_url, b.percent_read,
                    b.resource_type, b.origin_type,
                    m.description, m.subjects, m.publish_year, m.isbn, m.openlibrary_key
             FROM books b
             LEFT JOIN metadata m ON b.asin = m.asin
             {}
             ORDER BY {}
             LIMIT ?1 OFFSET ?2",
            where_clause, order_clause
        );

        let mut stmt = self.conn.prepare(&sql)?;

        // Build parameter list: limit, offset, then filter values
        let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = vec![
            Box::new(limit),
            Box::new(offset),
        ];
        for p in params {
            all_params.push(Box::new(p));
        }
        let param_refs: Vec<&dyn rusqlite::ToSql> = all_params.iter().map(|p| p.as_ref()).collect();

        let books: Vec<BookWithMeta> = stmt
            .query_map(param_refs.as_slice(), map_book_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(books)
    }

    /// Get distinct subjects for filtering
    pub fn get_subjects(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT subjects FROM metadata WHERE subjects IS NOT NULL AND subjects != '[]'"
        )?;

        let rows = stmt.query_map([], |row| {
            let subjects_json: String = row.get(0)?;
            Ok(subjects_json)
        })?;

        let mut all_subjects: std::collections::HashSet<String> = std::collections::HashSet::new();
        for row in rows {
            let subjects_json = row?;
            let subjects: Vec<String> = serde_json::from_str(&subjects_json).unwrap_or_default();
            for subject in subjects {
                all_subjects.insert(subject);
            }
        }

        let mut subjects: Vec<String> = all_subjects.into_iter().collect();
        subjects.sort();
        Ok(subjects)
    }

    /// Get book count with optional filters
    pub fn get_book_count_filtered(&self, filters: &[Filter]) -> Result<usize> {
        if filters.is_empty() {
            return self.get_book_count();
        }

        let (where_clause, params) = build_filter_clause_with_offset(filters, 1);
        let sql = format!(
            "SELECT COUNT(*) FROM books b
             LEFT JOIN metadata m ON b.asin = m.asin
             {}",
            where_clause
        );

        let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = vec![];
        for p in params {
            all_params.push(Box::new(p));
        }
        let param_refs: Vec<&dyn rusqlite::ToSql> = all_params.iter().map(|p| p.as_ref()).collect();

        let count: usize = self.conn.query_row(&sql, param_refs.as_slice(), |row| row.get(0))?;
        Ok(count)
    }

    /// Get a single book by ASIN
    pub fn get_book_by_asin(&self, asin: &str) -> Result<Option<BookWithMeta>> {
        let mut stmt = self.conn.prepare(
            "SELECT b.asin, b.title, b.authors, b.cover_url, b.percent_read,
                    b.resource_type, b.origin_type,
                    m.description, m.subjects, m.publish_year, m.isbn, m.openlibrary_key
             FROM books b
             LEFT JOIN metadata m ON b.asin = m.asin
             WHERE b.asin = ?1",
        )?;

        let mut rows = stmt.query_map(params![asin], map_book_row)?;

        match rows.next() {
            Some(Ok(book)) => Ok(Some(book)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Get books without metadata (for enrichment)
    pub fn get_books_without_metadata(&self) -> Result<Vec<Book>> {
        let mut stmt = self.conn.prepare(
            "SELECT b.asin, b.title, b.authors, b.cover_url, b.percent_read,
                    b.resource_type, b.origin_type
             FROM books b
             LEFT JOIN metadata m ON b.asin = m.asin
             WHERE m.asin IS NULL",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Book {
                asin: row.get(0)?,
                title: row.get(1)?,
                authors: parse_json_array(row.get::<_, String>(2)?),
                cover_url: row.get(3)?,
                percent_read: row.get(4)?,
                resource_type: row.get(5)?,
                origin_type: row.get(6)?,
            })
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    /// Get books with metadata but without embeddings
    pub fn get_books_for_embedding(&self) -> Result<Vec<BookForEmbedding>> {
        let mut stmt = self.conn.prepare(
            "SELECT b.asin, b.title, b.authors, m.description
             FROM books b
             JOIN metadata m ON b.asin = m.asin
             LEFT JOIN books_vec v ON b.asin = v.asin
             WHERE v.asin IS NULL AND m.description IS NOT NULL",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(BookForEmbedding {
                asin: row.get(0)?,
                title: row.get(1)?,
                authors: parse_json_array(row.get::<_, String>(2)?),
                description: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            })
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    /// Save enriched metadata for a book
    pub fn save_metadata(&self, asin: &str, data: &EnrichmentData) -> Result<()> {
        let subjects_json = serde_json::to_string(&data.subjects)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO metadata (asin, openlibrary_key, description, subjects, isbn, publish_year)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                asin,
                data.openlibrary_key,
                data.description,
                subjects_json,
                data.isbn,
                data.publish_year,
            ],
        )?;
        Ok(())
    }

    /// Save embedding for a book
    pub fn save_embedding(&self, asin: &str, embedding: &[f32]) -> Result<()> {
        let blob = serialize_embedding(embedding);
        self.conn.execute(
            "INSERT OR REPLACE INTO books_vec (asin, embedding) VALUES (?1, ?2)",
            params![asin, blob],
        )?;
        Ok(())
    }

    /// Rebuild the full-text search index
    pub fn rebuild_fts(&self) -> Result<()> {
        self.conn.execute("DELETE FROM books_fts_content", [])?;
        self.conn.execute(
            "INSERT INTO books_fts_content (asin, title, authors, description, subjects)
             SELECT b.asin, b.title, b.authors, COALESCE(m.description, ''), COALESCE(m.subjects, '')
             FROM books b
             LEFT JOIN metadata m ON b.asin = m.asin",
            [],
        )?;
        self.conn
            .execute("INSERT INTO books_fts(books_fts) VALUES('rebuild')", [])?;
        Ok(())
    }

    /// Get total book count (for pagination)
    pub fn get_book_count(&self) -> Result<usize> {
        let count: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM books", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Clear all metadata to allow re-enrichment
    pub fn clear_metadata(&self) -> Result<usize> {
        let count = self.conn.execute("DELETE FROM metadata", [])?;
        // Also clear embeddings since they depend on enriched descriptions
        self.conn.execute("DELETE FROM books_vec", [])?;
        self.conn.execute("DELETE FROM books_fts_content", [])?;
        self.conn.execute("INSERT INTO books_fts(books_fts) VALUES('rebuild')", [])?;
        Ok(count)
    }
}

/// Serialize a float32 vector to little-endian binary blob (matches Python struct.pack)
fn serialize_embedding(vec: &[f32]) -> Vec<u8> {
    vec.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Parse a JSON array string into Vec<String>
fn parse_json_array(json: String) -> Vec<String> {
    serde_json::from_str(&json).unwrap_or_default()
}

/// Parse optional JSON string to Vec<String> (handles NULL from database)
fn parse_optional_json_array(json: Option<String>) -> Vec<String> {
    json.map(parse_json_array).unwrap_or_default()
}

/// Map a database row to BookWithMeta (base columns 0-11)
/// Columns: asin, title, authors, cover_url, percent_read, resource_type, origin_type,
///          description, subjects, publish_year, isbn, openlibrary_key
fn map_book_row(row: &rusqlite::Row) -> rusqlite::Result<BookWithMeta> {
    Ok(BookWithMeta {
        asin: row.get(0)?,
        title: row.get(1)?,
        authors: parse_json_array(row.get::<_, String>(2)?),
        cover_url: row.get(3)?,
        percent_read: row.get(4)?,
        resource_type: row.get(5)?,
        origin_type: row.get(6)?,
        description: row.get(7)?,
        subjects: parse_optional_json_array(row.get(8)?),
        publish_year: row.get(9)?,
        isbn: row.get(10)?,
        openlibrary_key: row.get(11)?,
        distance: None,
        rank: None,
    })
}

/// Map a database row to BookWithMeta with rank (column 12)
fn map_book_row_with_rank(row: &rusqlite::Row) -> rusqlite::Result<BookWithMeta> {
    let mut book = map_book_row(row)?;
    book.rank = row.get(12)?;
    Ok(book)
}

/// Map a database row to BookWithMeta with distance (column 12)
fn map_book_row_with_distance(row: &rusqlite::Row) -> rusqlite::Result<BookWithMeta> {
    let mut book = map_book_row(row)?;
    book.distance = row.get(12)?;
    Ok(book)
}

/// Build a WHERE clause from a list of filters
/// Returns (where_clause, params)
/// start_idx specifies the first parameter index (e.g., 3 if ?1=limit, ?2=offset already used)
fn build_filter_clause_with_offset(filters: &[Filter], start_idx: usize) -> (String, Vec<String>) {
    if filters.is_empty() {
        return (String::new(), Vec::new());
    }

    let mut conditions = Vec::new();
    let mut params = Vec::new();
    let mut param_idx = start_idx;

    for filter in filters {
        let column = match filter.field.as_str() {
            "title" => "b.title",
            "author" => "b.authors",
            "description" => "m.description",
            "subject" => "m.subjects",
            _ => continue, // Skip unknown fields
        };

        // Build condition based on operation
        match filter.op.as_str() {
            "contains" => {
                // Case-insensitive contains using LIKE
                conditions.push(format!("{} LIKE '%' || ?{} || '%'", column, param_idx));
                params.push(filter.value.clone());
                param_idx += 1;
            }
            "has" => {
                // For subject arrays, match the exact subject in JSON
                // Use LIKE with quotes to match exact array element
                conditions.push(format!("{} LIKE '%\"' || ?{} || '\"%'", column, param_idx));
                params.push(filter.value.clone());
                param_idx += 1;
            }
            _ => continue, // Skip unknown operations
        }
    }

    if conditions.is_empty() {
        return (String::new(), Vec::new());
    }

    let where_clause = format!("WHERE {}", conditions.join(" AND "));
    (where_clause, params)
}

/// Convenience wrapper for get_all_books (params start at ?3)
fn build_filter_clause(filters: &[Filter]) -> (String, Vec<String>) {
    build_filter_clause_with_offset(filters, 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_serialization() {
        let embedding = vec![1.0f32, 2.0, 3.0];
        let blob = serialize_embedding(&embedding);
        assert_eq!(blob.len(), 12); // 3 * 4 bytes

        // Verify little-endian format
        let first: f32 = f32::from_le_bytes(blob[0..4].try_into().unwrap());
        assert_eq!(first, 1.0);
    }

    #[test]
    fn test_parse_json_array() {
        let result = parse_json_array(r#"["Alice", "Bob"]"#.to_string());
        assert_eq!(result, vec!["Alice", "Bob"]);

        // Invalid JSON returns empty
        let result = parse_json_array("not json".to_string());
        assert!(result.is_empty());
    }
}
