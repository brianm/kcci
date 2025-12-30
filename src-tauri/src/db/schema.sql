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
