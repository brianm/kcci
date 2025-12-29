"""Enrich book metadata from OpenLibrary."""

import json
import re
import time
from typing import Optional
import requests
import sqlite3

from . import db

# User agent for API requests
USER_AGENT = "KCCI/1.0 (https://github.com/brianm/kcci; brianm@skife.org)"

# Default delay between API calls
DEFAULT_DELAY = 0.25


def _make_request(url: str, params: dict = None, max_retries: int = 5) -> Optional[requests.Response]:
    """Make HTTP request with exponential backoff on 429 errors."""
    headers = {"User-Agent": USER_AGENT}
    delay = 1.0  # Initial backoff delay

    for attempt in range(max_retries):
        try:
            r = requests.get(url, params=params, headers=headers, timeout=10)

            if r.status_code == 429:
                # Rate limited - exponential backoff
                retry_after = r.headers.get("Retry-After")
                if retry_after:
                    delay = float(retry_after)
                time.sleep(delay)
                delay *= 2  # Exponential backoff
                continue

            r.raise_for_status()
            return r

        except requests.RequestException:
            if attempt < max_retries - 1:
                time.sleep(delay)
                delay *= 2
            else:
                return None

    return None


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

        r = _make_request("https://openlibrary.org/search.json", params=params)
        if r:
            data = r.json()
            if data.get("docs"):
                return data["docs"][0]

    # Fall back to title-only search
    r = _make_request(
        "https://openlibrary.org/search.json",
        params={"title": clean_title, "limit": 5},
    )
    if r:
        data = r.json()
        if data.get("docs"):
            return data["docs"][0]

    return None


def get_work_details(work_key: str) -> Optional[dict]:
    """Fetch full work details including description."""
    r = _make_request(f"https://openlibrary.org{work_key}.json")
    if r:
        return r.json()
    return None


def extract_description(work: dict) -> str:
    """Extract description from work details."""
    desc = work.get("description", "")
    if isinstance(desc, dict):
        desc = desc.get("value", "")
    return desc


def enrich_book(conn: sqlite3.Connection, asin: str, title: str, authors: list[str],
                delay: float = DEFAULT_DELAY) -> bool:
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


def enrich_batch(conn: sqlite3.Connection, limit: Optional[int] = None, delay: float = DEFAULT_DELAY,
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
