"""Import and query OpenLibrary data dumps for fast enrichment."""

import gzip
import json
import sqlite3
from pathlib import Path
from typing import Optional, Iterator

from . import db


def init_ol_schema(conn: sqlite3.Connection) -> None:
    """Create tables for OpenLibrary dump data."""
    conn.executescript("""
        -- OpenLibrary works from dump
        CREATE TABLE IF NOT EXISTS ol_works (
            key TEXT PRIMARY KEY,  -- e.g., /works/OL123W
            title TEXT,
            title_lower TEXT,  -- lowercase for matching
            description TEXT,
            subjects TEXT,  -- JSON array
            authors TEXT,  -- JSON array of author keys
            first_publish_year INTEGER
        );

        -- Index for title matching
        CREATE INDEX IF NOT EXISTS idx_ol_works_title ON ol_works(title_lower);

        -- OpenLibrary authors from dump (optional, for author name lookup)
        CREATE TABLE IF NOT EXISTS ol_authors (
            key TEXT PRIMARY KEY,  -- e.g., /authors/OL123A
            name TEXT,
            name_lower TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_ol_authors_name ON ol_authors(name_lower);
    """)
    conn.commit()


def parse_dump_line(line: str) -> Optional[dict]:
    """Parse a single line from OpenLibrary dump."""
    parts = line.strip().split('\t')
    if len(parts) < 5:
        return None

    type_key, key, revision, last_modified, json_data = parts[:5]
    try:
        return json.loads(json_data)
    except json.JSONDecodeError:
        return None


def extract_description(record: dict) -> str:
    """Extract description from OpenLibrary record."""
    desc = record.get("description", "")
    if isinstance(desc, dict):
        desc = desc.get("value", "")
    return desc if isinstance(desc, str) else ""


def import_works_dump(conn: sqlite3.Connection, dump_path: Path,
                      progress_callback=None) -> int:
    """Import OpenLibrary works dump into local database."""
    init_ol_schema(conn)

    count = 0
    batch = []
    batch_size = 1000

    opener = gzip.open if str(dump_path).endswith('.gz') else open

    with opener(dump_path, 'rt', encoding='utf-8', errors='replace') as f:
        for line in f:
            record = parse_dump_line(line)
            if not record:
                continue

            # Extract fields
            key = record.get("key", "")
            title = record.get("title", "")
            if not title:
                continue

            description = extract_description(record)
            subjects = record.get("subjects", [])
            if isinstance(subjects, list):
                subjects = subjects[:20]  # Limit
            else:
                subjects = []

            # Authors are stored as references like {"author": {"key": "/authors/OL123A"}}
            author_keys = []
            for author_ref in record.get("authors", []):
                if isinstance(author_ref, dict):
                    author = author_ref.get("author", {})
                    if isinstance(author, dict):
                        author_keys.append(author.get("key", ""))

            first_publish_year = record.get("first_publish_year")

            batch.append((
                key,
                title,
                title.lower(),
                description,
                json.dumps(subjects),
                json.dumps(author_keys),
                first_publish_year
            ))

            if len(batch) >= batch_size:
                conn.executemany("""
                    INSERT OR REPLACE INTO ol_works
                    (key, title, title_lower, description, subjects, authors, first_publish_year)
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                """, batch)
                conn.commit()
                count += len(batch)
                if progress_callback:
                    progress_callback(count)
                batch = []

    # Final batch
    if batch:
        conn.executemany("""
            INSERT OR REPLACE INTO ol_works
            (key, title, title_lower, description, subjects, authors, first_publish_year)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        """, batch)
        conn.commit()
        count += len(batch)

    return count


def import_authors_dump(conn: sqlite3.Connection, dump_path: Path,
                        progress_callback=None) -> int:
    """Import OpenLibrary authors dump into local database."""
    init_ol_schema(conn)

    count = 0
    batch = []
    batch_size = 1000

    opener = gzip.open if str(dump_path).endswith('.gz') else open

    with opener(dump_path, 'rt', encoding='utf-8', errors='replace') as f:
        for line in f:
            record = parse_dump_line(line)
            if not record:
                continue

            key = record.get("key", "")
            name = record.get("name", "")
            if not name:
                continue

            batch.append((key, name, name.lower()))

            if len(batch) >= batch_size:
                conn.executemany("""
                    INSERT OR REPLACE INTO ol_authors (key, name, name_lower)
                    VALUES (?, ?, ?)
                """, batch)
                conn.commit()
                count += len(batch)
                if progress_callback:
                    progress_callback(count)
                batch = []

    if batch:
        conn.executemany("""
            INSERT OR REPLACE INTO ol_authors (key, name, name_lower)
            VALUES (?, ?, ?)
        """, batch)
        conn.commit()
        count += len(batch)

    return count


def normalize_title(title: str) -> str:
    """Normalize title for matching."""
    import re
    # Remove series info in parens, e.g., "(Book 1)"
    title = re.sub(r'\s*\([^)]*\)\s*$', '', title)
    # Remove leading "The ", "A ", "An "
    title = re.sub(r'^(the|a|an)\s+', '', title, flags=re.IGNORECASE)
    # Lowercase and strip
    return title.lower().strip()


def normalize_author(author: str) -> str:
    """Normalize author name for matching."""
    # Handle "Last, First" format
    if ',' in author:
        parts = author.split(',', 1)
        author = f"{parts[1].strip()} {parts[0].strip()}"
    return author.lower().strip()


def find_work_by_title(conn: sqlite3.Connection, title: str,
                       authors: list[str] = None) -> Optional[dict]:
    """Find a work in the local dump by title, optionally filtering by author."""
    normalized = normalize_title(title)

    # Try exact match first
    rows = conn.execute("""
        SELECT key, title, description, subjects, authors, first_publish_year
        FROM ol_works
        WHERE title_lower = ? AND description != ''
        LIMIT 10
    """, (normalized,)).fetchall()

    if not rows:
        # Try prefix match
        rows = conn.execute("""
            SELECT key, title, description, subjects, authors, first_publish_year
            FROM ol_works
            WHERE title_lower LIKE ? AND description != ''
            LIMIT 10
        """, (normalized + '%',)).fetchall()

    if not rows:
        return None

    # If we have authors, try to match
    if authors:
        normalized_authors = [normalize_author(a) for a in authors]
        for row in rows:
            work_author_keys = json.loads(row[4]) if row[4] else []
            # Look up author names
            for author_key in work_author_keys:
                author_row = conn.execute(
                    "SELECT name_lower FROM ol_authors WHERE key = ?",
                    (author_key,)
                ).fetchone()
                if author_row:
                    for na in normalized_authors:
                        if na in author_row[0] or author_row[0] in na:
                            return {
                                "key": row[0],
                                "title": row[1],
                                "description": row[2],
                                "subjects": json.loads(row[3]) if row[3] else [],
                                "first_publish_year": row[5]
                            }

    # Return first result with description
    row = rows[0]
    return {
        "key": row[0],
        "title": row[1],
        "description": row[2],
        "subjects": json.loads(row[3]) if row[3] else [],
        "first_publish_year": row[5]
    }


def enrich_from_dump(conn: sqlite3.Connection, limit: Optional[int] = None,
                     progress_callback=None) -> tuple[int, int]:
    """Enrich books from local OpenLibrary dump. Returns (success, total)."""
    books = db.get_books_without_metadata(conn, limit)
    success = 0

    for i, book in enumerate(books):
        authors = json.loads(book["authors"])
        work = find_work_by_title(conn, book["title"], authors)

        if work and work.get("description"):
            db.save_metadata(
                conn,
                book["asin"],
                work["key"],
                work["description"],
                work.get("subjects", []),
                None,  # ISBN not in works dump
                work.get("first_publish_year")
            )
            success += 1
        else:
            # Mark as attempted with empty metadata
            db.save_metadata(conn, book["asin"], "", "", [], None, None)

        if progress_callback:
            progress_callback(i + 1, len(books), book["title"])

    return success, len(books)


def get_dump_stats(conn: sqlite3.Connection) -> dict:
    """Get statistics about imported dump data."""
    try:
        works = conn.execute("SELECT COUNT(*) FROM ol_works").fetchone()[0]
        works_with_desc = conn.execute(
            "SELECT COUNT(*) FROM ol_works WHERE description != ''"
        ).fetchone()[0]
        authors = conn.execute("SELECT COUNT(*) FROM ol_authors").fetchone()[0]
    except sqlite3.OperationalError:
        works = 0
        works_with_desc = 0
        authors = 0

    return {
        "works": works,
        "works_with_descriptions": works_with_desc,
        "authors": authors
    }
