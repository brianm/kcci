-- Rebuild FTS index to ensure all books are indexed
-- This fixes cases where the FTS content table became empty or out of sync

DELETE FROM books_fts_content;

INSERT INTO books_fts_content (asin, title, authors, description, subjects)
SELECT b.asin, b.title, b.authors, COALESCE(m.description, ''), COALESCE(m.subjects, '')
FROM books b
LEFT JOIN metadata m ON b.asin = m.asin;

INSERT INTO books_fts(books_fts) VALUES('rebuild');
