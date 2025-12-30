use rusqlite::{ffi::sqlite3_auto_extension, params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Once;

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

        let rows = stmt.query_map(params![query, limit], |row| {
            Ok(BookWithMeta {
                asin: row.get(0)?,
                title: row.get(1)?,
                authors: parse_json_array(row.get::<_, String>(2)?),
                cover_url: row.get(3)?,
                percent_read: row.get(4)?,
                resource_type: row.get(5)?,
                origin_type: row.get(6)?,
                description: row.get(7)?,
                subjects: parse_json_array(row.get::<_, Option<String>>(8)?.unwrap_or_default()),
                publish_year: row.get(9)?,
                isbn: row.get(10)?,
                openlibrary_key: row.get(11)?,
                distance: None,
                rank: row.get(12)?,
            })
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.into())
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

        let rows = stmt.query_map(params![blob, limit], |row| {
            Ok(BookWithMeta {
                asin: row.get(0)?,
                title: row.get(1)?,
                authors: parse_json_array(row.get::<_, String>(2)?),
                cover_url: row.get(3)?,
                percent_read: row.get(4)?,
                resource_type: row.get(5)?,
                origin_type: row.get(6)?,
                description: row.get(7)?,
                subjects: parse_json_array(row.get::<_, Option<String>>(8)?.unwrap_or_default()),
                publish_year: row.get(9)?,
                isbn: row.get(10)?,
                openlibrary_key: row.get(11)?,
                distance: row.get(12)?,
                rank: None,
            })
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.into())
    }

    /// Get paginated list of all books
    pub fn get_all_books(&self, limit: usize, offset: usize) -> Result<Vec<BookWithMeta>> {
        let mut stmt = self.conn.prepare(
            "SELECT b.asin, b.title, b.authors, b.cover_url, b.percent_read,
                    b.resource_type, b.origin_type,
                    m.description, m.subjects, m.publish_year, m.isbn, m.openlibrary_key
             FROM books b
             LEFT JOIN metadata m ON b.asin = m.asin
             ORDER BY b.title
             LIMIT ?1 OFFSET ?2",
        )?;

        let rows = stmt.query_map(params![limit, offset], |row| {
            Ok(BookWithMeta {
                asin: row.get(0)?,
                title: row.get(1)?,
                authors: parse_json_array(row.get::<_, String>(2)?),
                cover_url: row.get(3)?,
                percent_read: row.get(4)?,
                resource_type: row.get(5)?,
                origin_type: row.get(6)?,
                description: row.get(7)?,
                subjects: parse_json_array(row.get::<_, Option<String>>(8)?.unwrap_or_default()),
                publish_year: row.get(9)?,
                isbn: row.get(10)?,
                openlibrary_key: row.get(11)?,
                distance: None,
                rank: None,
            })
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.into())
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

        let mut rows = stmt.query_map(params![asin], |row| {
            Ok(BookWithMeta {
                asin: row.get(0)?,
                title: row.get(1)?,
                authors: parse_json_array(row.get::<_, String>(2)?),
                cover_url: row.get(3)?,
                percent_read: row.get(4)?,
                resource_type: row.get(5)?,
                origin_type: row.get(6)?,
                description: row.get(7)?,
                subjects: parse_json_array(row.get::<_, Option<String>>(8)?.unwrap_or_default()),
                publish_year: row.get(9)?,
                isbn: row.get(10)?,
                openlibrary_key: row.get(11)?,
                distance: None,
                rank: None,
            })
        })?;

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
            "INSERT INTO books_fts_content (asin, title, authors, description)
             SELECT b.asin, b.title, b.authors, COALESCE(m.description, '')
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
}

/// Serialize a float32 vector to little-endian binary blob (matches Python struct.pack)
fn serialize_embedding(vec: &[f32]) -> Vec<u8> {
    vec.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Parse a JSON array string into Vec<String>
fn parse_json_array(json: String) -> Vec<String> {
    serde_json::from_str(&json).unwrap_or_default()
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
