-- v002: Remove cover_url and percent_read columns from books table
-- These fields are not useful for the application

-- SQLite 3.35.0+ supports ALTER TABLE DROP COLUMN
ALTER TABLE books DROP COLUMN cover_url;
ALTER TABLE books DROP COLUMN percent_read;
