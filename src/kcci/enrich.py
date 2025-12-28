"""Enrich book metadata from OpenLibrary."""

import json
import re
import time
from typing import Optional
import requests
import sqlite3

from . import db


def normalize_author_for_search(author: str) -> str:
    """Convert 'Last, First' to 'First Last' for API search."""
    if ',' in author:
        parts = author.split(',', 1)
        return f"{parts[1].strip()} {parts[0].strip()}"
    return author.strip()


def normalize_title_for_search(title: str) -> str:
    """Clean up title for API search."""
    # Remove series info in parens
    title = re.sub(r'\s*\([^)]*\)', '', title)
    # Remove subtitle after colon
    title = re.sub(r':.*$', '', title)
    return title.strip()


def search_openlibrary(title: str, authors: list[str]) -> Optional[dict]:
    """Search OpenLibrary for a book by title and author."""
    clean_title = normalize_title_for_search(title)

    # Try with author first
    if authors:
        author = normalize_author_for_search(authors[0])
        params = {"title": clean_title, "author": author, "limit": 5}

        try:
            r = requests.get(
                "https://openlibrary.org/search.json",
                params=params,
                timeout=10,
            )
            r.raise_for_status()
            data = r.json()

            if data.get("docs"):
                return data["docs"][0]
        except requests.RequestException:
            pass

    # Fall back to title-only search
    try:
        r = requests.get(
            "https://openlibrary.org/search.json",
            params={"title": clean_title, "limit": 5},
            timeout=10,
        )
        r.raise_for_status()
        data = r.json()

        if data.get("docs"):
            return data["docs"][0]
    except requests.RequestException:
        pass

    return None


def get_work_details(work_key: str) -> Optional[dict]:
    """Fetch full work details including description."""
    try:
        r = requests.get(
            f"https://openlibrary.org{work_key}.json",
            timeout=10,
        )
        r.raise_for_status()
        return r.json()
    except requests.RequestException:
        return None


def extract_description(work: dict) -> str:
    """Extract description from work details."""
    desc = work.get("description", "")
    if isinstance(desc, dict):
        desc = desc.get("value", "")
    return desc


def enrich_book(conn: sqlite3.Connection, asin: str, title: str, authors: list[str],
                delay: float = 1.0) -> bool:
    """Enrich a single book with OpenLibrary metadata. Returns True if successful."""
    # Search for the book
    search_result = search_openlibrary(title, authors)
    if not search_result:
        # Save empty metadata to mark as attempted
        db.save_metadata(conn, asin, "", "", [], None, None)
        return False

    work_key = search_result.get("key", "")
    subjects = search_result.get("subject", [])[:20]  # Limit subjects
    publish_year = search_result.get("first_publish_year")
    isbn = None
    if search_result.get("isbn"):
        isbn = search_result["isbn"][0]

    # Get full work details for description
    description = ""
    if work_key:
        time.sleep(delay)  # Rate limit between API calls
        work = get_work_details(work_key)
        if work:
            description = extract_description(work)

    db.save_metadata(conn, asin, work_key, description, subjects, isbn, publish_year)
    return bool(description)


def enrich_batch(conn: sqlite3.Connection, limit: Optional[int] = None, delay: float = 1.0,
                 progress_callback=None) -> tuple[int, int]:
    """Enrich a batch of books. Returns (success_count, total_attempted)."""
    books = db.get_books_without_metadata(conn, limit)
    success = 0

    for i, book in enumerate(books):
        authors = json.loads(book["authors"])
        if enrich_book(conn, book["asin"], book["title"], authors, delay):
            success += 1

        if progress_callback:
            progress_callback(i + 1, len(books), book["title"])

        if i < len(books) - 1:
            time.sleep(delay)  # Rate limit between books

    return success, len(books)
