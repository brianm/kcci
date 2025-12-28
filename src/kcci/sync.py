"""Unified sync pipeline: import -> enrich -> embed."""

import json
import sqlite3
import time
from pathlib import Path
from typing import Optional, Callable

from . import db
from . import enrich
from . import embed
from . import webarchive


def format_time(seconds: float) -> str:
    """Format seconds into human-readable string."""
    if seconds < 60:
        return f"{int(seconds)}s"
    elif seconds < 3600:
        mins = int(seconds // 60)
        secs = int(seconds % 60)
        return f"{mins}m{secs}s"
    else:
        hours = int(seconds // 3600)
        mins = int((seconds % 3600) // 60)
        return f"{hours}h{mins}m"


class ProgressTracker:
    """Track progress and estimate remaining time."""

    def __init__(self):
        self.start_time = time.time()

    def get_eta(self, current: int, total: int) -> tuple[float, float]:
        """Return (elapsed_seconds, eta_seconds)."""
        elapsed = time.time() - self.start_time
        if current == 0:
            return elapsed, 0.0
        rate = current / elapsed
        remaining = total - current
        eta = remaining / rate if rate > 0 else 0
        return elapsed, eta


def sync(
    conn: sqlite3.Connection,
    webarchive_path: Optional[Path] = None,
    enrich_delay: float = 1.0,
    progress_callback: Optional[Callable] = None,
) -> dict:
    """
    Full sync pipeline: import -> enrich -> embed.

    Args:
        conn: Database connection
        webarchive_path: Optional path to Safari webarchive to import
        enrich_delay: Delay between OpenLibrary API calls
        progress_callback: Called with (stage, message) for progress updates

    Returns:
        Dict with counts: imported, enriched, embedded
    """

    def report(stage: str, message: str):
        if progress_callback:
            progress_callback(stage, message)

    stats = {"imported": 0, "enriched": 0, "embedded": 0}

    # Stage 1: Import (if webarchive provided)
    if webarchive_path:
        report("import", f"Reading {webarchive_path.name}...")
        books = webarchive.parse_webarchive(webarchive_path)
        report("import", f"Found {len(books)} books in webarchive")

        if books:
            # Save to temp JSON and import
            import tempfile
            with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
                json.dump(books, f)
                temp_path = Path(f.name)

            count = db.import_kindle_json(conn, temp_path)
            temp_path.unlink()
            stats["imported"] = count

            if count > 0:
                db.rebuild_fts(conn)
                report("import", f"Imported {count} new books")
            else:
                report("import", "No new books to import")

    # Stage 2: Enrich
    books_to_enrich = db.get_books_without_metadata(conn)
    total_to_enrich = len(books_to_enrich)

    if total_to_enrich == 0:
        report("enrich", "All books already enriched")
    else:
        report("enrich", f"Enriching {total_to_enrich} books...")
        tracker = ProgressTracker()

        for i, book in enumerate(books_to_enrich):
            authors = json.loads(book["authors"])
            if enrich.enrich_book(conn, book["asin"], book["title"], authors, enrich_delay):
                stats["enriched"] += 1

            elapsed, eta = tracker.get_eta(i + 1, total_to_enrich)
            title = book["title"][:40]
            report(
                "enrich",
                f"{i + 1}/{total_to_enrich} \"{title}\" "
                f"({format_time(elapsed)} elapsed, ~{format_time(eta)} remaining)"
            )

            if i < total_to_enrich - 1:
                time.sleep(enrich_delay)

        db.rebuild_fts(conn)
        report("enrich", f"Enriched {stats['enriched']}/{total_to_enrich} with descriptions")

    # Stage 3: Embed
    books_to_embed = embed.get_books_for_embedding(conn, limit=None)
    total_to_embed = len(books_to_embed)

    if total_to_embed == 0:
        report("embed", "All enriched books already have embeddings")
    else:
        report("embed", "Loading embedding model...")
        model = embed.get_model()

        report("embed", f"Generating embeddings for {total_to_embed} books...")
        tracker = ProgressTracker()

        for i, book in enumerate(books_to_embed):
            authors = json.loads(book["authors"])
            text = embed.get_embedding_text(book["title"], authors, book["description"])
            embedding = model.encode(text).tolist()
            db.save_embedding(conn, book["asin"], embedding)
            stats["embedded"] += 1

            elapsed, eta = tracker.get_eta(i + 1, total_to_embed)
            title = book["title"][:40]
            report(
                "embed",
                f"{i + 1}/{total_to_embed} \"{title}\" "
                f"({format_time(elapsed)} elapsed, ~{format_time(eta)} remaining)"
            )

        report("embed", f"Generated {stats['embedded']} embeddings")

    return stats
