"""Database module for KCCI book storage with vector search."""

import sqlite3
import json
from pathlib import Path
from typing import Optional
import sqlite_vec  # type: ignore


def get_db_path() -> Path:
    """Get the default database path."""
    return Path.home() / ".kcci" / "books.db"


def connect(db_path: Optional[Path] = None) -> sqlite3.Connection:
    """Connect to the database, creating it if necessary."""
    if db_path is None:
        db_path = get_db_path()

    db_path.parent.mkdir(parents=True, exist_ok=True)

    db = sqlite3.connect(db_path)
    db.row_factory = sqlite3.Row
    db.enable_load_extension(True)
    sqlite_vec.load(db)
    db.enable_load_extension(False)

    return db


def init_schema(db: sqlite3.Connection) -> None:
    """Initialize the database schema."""
    db.executescript("""
        -- Core book data from Kindle export
        CREATE TABLE IF NOT EXISTS books (
            asin TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            authors TEXT NOT NULL,  -- JSON array
            cover_url TEXT,
            percent_read INTEGER DEFAULT 0,
            resource_type TEXT,
            origin_type TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        -- Enriched metadata from OpenLibrary
        CREATE TABLE IF NOT EXISTS metadata (
            asin TEXT PRIMARY KEY REFERENCES books(asin),
            openlibrary_key TEXT,
            description TEXT,
            subjects TEXT,  -- JSON array
            isbn TEXT,
            publish_year INTEGER,
            enriched_at TEXT DEFAULT CURRENT_TIMESTAMP
        );

        -- Full-text search (external content table)
        CREATE VIRTUAL TABLE IF NOT EXISTS books_fts USING fts5(
            title,
            authors,
            description,
            content='books_fts_content',
            content_rowid='rowid',
            tokenize='porter'
        );

        -- Content table for FTS
        CREATE TABLE IF NOT EXISTS books_fts_content (
            rowid INTEGER PRIMARY KEY,
            asin TEXT UNIQUE,
            title TEXT,
            authors TEXT,
            description TEXT
        );
    """)

    # Create vector table for embeddings (768 dimensions for msmarco model)
    # sqlite-vec uses a different syntax than sqlite-vss
    db.execute("""
        CREATE VIRTUAL TABLE IF NOT EXISTS books_vec USING vec0(
            asin TEXT PRIMARY KEY,
            embedding FLOAT[768]
        )
    """)

    db.commit()


def import_kindle_json(db: sqlite3.Connection, json_path: Path) -> int:
    """Import books from Kindle export JSON. Returns count of new books."""
    with open(json_path) as f:
        books = json.load(f)

    count = 0
    for book in books:
        cursor = db.execute("""
            INSERT OR IGNORE INTO books (asin, title, authors, cover_url, percent_read, resource_type, origin_type)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        """, (
            book["asin"],
            book["title"],
            json.dumps(book.get("authors", [])),
            book.get("coverUrl"),
            book.get("percentageRead", 0),
            book.get("resourceType"),
            book.get("originType"),
        ))
        count += cursor.rowcount

    db.commit()
    return count


def update_fts(db: sqlite3.Connection, asin: str) -> None:
    """Update FTS index for a single book."""
    row = db.execute("""
        SELECT b.asin, b.title, b.authors, m.description
        FROM books b
        LEFT JOIN metadata m ON b.asin = m.asin
        WHERE b.asin = ?
    """, (asin,)).fetchone()

    if row:
        # Insert or replace in content table
        db.execute("""
            INSERT OR REPLACE INTO books_fts_content (asin, title, authors, description)
            VALUES (?, ?, ?, ?)
        """, (row["asin"], row["title"], row["authors"], row["description"] or ""))
        # Rebuild FTS index for this entry
        db.execute("INSERT INTO books_fts(books_fts) VALUES('rebuild')")
        db.commit()


def rebuild_fts(db: sqlite3.Connection) -> None:
    """Rebuild entire FTS index."""
    # Populate the content table
    db.execute("DELETE FROM books_fts_content")
    db.execute("""
        INSERT INTO books_fts_content (asin, title, authors, description)
        SELECT b.asin, b.title, b.authors, COALESCE(m.description, '')
        FROM books b
        LEFT JOIN metadata m ON b.asin = m.asin
    """)
    # Rebuild the FTS index
    db.execute("INSERT INTO books_fts(books_fts) VALUES('rebuild')")
    db.commit()


def search_fts(db: sqlite3.Connection, query: str, limit: int = 10) -> list[dict]:
    """Full-text search across title, authors, description."""
    rows = db.execute("""
        SELECT b.*, m.description, m.subjects, bm25(books_fts) as rank
        FROM books_fts f
        JOIN books_fts_content c ON f.rowid = c.rowid
        JOIN books b ON c.asin = b.asin
        LEFT JOIN metadata m ON b.asin = m.asin
        WHERE books_fts MATCH ?
        ORDER BY rank
        LIMIT ?
    """, (query, limit)).fetchall()

    return [dict(row) for row in rows]


def search_semantic(db: sqlite3.Connection, embedding: list[float], limit: int = 10) -> list[dict]:
    """Semantic search using vector similarity."""
    import struct

    # sqlite-vec expects embeddings as binary blobs
    def serialize_float32(vec: list[float]) -> bytes:
        return struct.pack(f"{len(vec)}f", *vec)

    rows = db.execute("""
        SELECT b.*, m.description, m.subjects, v.distance
        FROM books_vec v
        JOIN books b ON v.asin = b.asin
        LEFT JOIN metadata m ON b.asin = m.asin
        WHERE embedding MATCH ?
          AND k = ?
        ORDER BY distance
    """, (serialize_float32(embedding), limit)).fetchall()

    return [dict(row) for row in rows]


def get_books_without_metadata(db: sqlite3.Connection, limit: Optional[int] = None) -> list[dict]:
    """Get books that haven't been enriched yet."""
    if limit is None:
        rows = db.execute("""
            SELECT b.*
            FROM books b
            LEFT JOIN metadata m ON b.asin = m.asin
            WHERE m.asin IS NULL
        """).fetchall()
    else:
        rows = db.execute("""
            SELECT b.*
            FROM books b
            LEFT JOIN metadata m ON b.asin = m.asin
            WHERE m.asin IS NULL
            LIMIT ?
        """, (limit,)).fetchall()

    return [dict(row) for row in rows]


def save_metadata(db: sqlite3.Connection, asin: str, openlibrary_key: str,
                  description: str, subjects: list[str], isbn: Optional[str],
                  publish_year: Optional[int]) -> None:
    """Save enriched metadata for a book."""
    db.execute("""
        INSERT OR REPLACE INTO metadata (asin, openlibrary_key, description, subjects, isbn, publish_year)
        VALUES (?, ?, ?, ?, ?, ?)
    """, (asin, openlibrary_key, description, json.dumps(subjects), isbn, publish_year))
    db.commit()
    update_fts(db, asin)


def save_embedding(db: sqlite3.Connection, asin: str, embedding: list[float]) -> None:
    """Save embedding for a book."""
    import struct

    # sqlite-vec expects embeddings as binary blobs
    def serialize_float32(vec: list[float]) -> bytes:
        return struct.pack(f"{len(vec)}f", *vec)

    db.execute("""
        INSERT OR REPLACE INTO books_vec (asin, embedding)
        VALUES (?, ?)
    """, (asin, serialize_float32(embedding)))
    db.commit()


def get_stats(db: sqlite3.Connection) -> dict:
    """Get database statistics."""
    total = db.execute("SELECT COUNT(*) FROM books").fetchone()[0]
    enriched = db.execute("SELECT COUNT(*) FROM metadata").fetchone()[0]
    with_embeddings = db.execute("SELECT COUNT(*) FROM books_vec").fetchone()[0]

    return {
        "total_books": total,
        "enriched": enriched,
        "with_embeddings": with_embeddings,
    }
