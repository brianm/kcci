"""Generate embeddings for books using sentence-transformers."""

import json
import sqlite3
import warnings
from sentence_transformers import SentenceTransformer  # type: ignore

from . import db

# Suppress FutureWarning from transformers about tokenization
warnings.filterwarnings("ignore", message=".*clean_up_tokenization_spaces.*")

# Model for semantic search - good balance of quality and speed
MODEL_NAME = "msmarco-distilbert-base-tas-b"


def get_model() -> SentenceTransformer:
    """Load the sentence transformer model."""
    return SentenceTransformer(MODEL_NAME)


def get_embedding_text(title: str, authors: list[str], description: str) -> str:
    """Combine book fields into text for embedding."""
    author_str = ", ".join(authors) if authors else ""
    parts = [title]
    if author_str:
        parts.append(f"by {author_str}")
    if description:
        parts.append(description)
    return " ".join(parts)


def get_books_for_embedding(conn: sqlite3.Connection, limit: int | None = None) -> list[dict]:
    """Get enriched books that don't have embeddings yet."""
    query = """
        SELECT b.asin, b.title, b.authors, COALESCE(m.description, '') as description
        FROM books b
        JOIN metadata m ON b.asin = m.asin
        LEFT JOIN books_vec v ON b.asin = v.asin
        WHERE v.asin IS NULL
    """
    if limit:
        query += " LIMIT ?"
        rows = conn.execute(query, (limit,)).fetchall()
    else:
        rows = conn.execute(query).fetchall()

    return [dict(row) for row in rows]


def embed_batch(conn: sqlite3.Connection, limit: int = 100,
                progress_callback=None) -> int:
    """Generate embeddings for a batch of books. Returns count embedded."""
    books = get_books_for_embedding(conn, limit)
    if not books:
        return 0

    model = get_model()

    for i, book in enumerate(books):
        authors = json.loads(book["authors"])
        text = get_embedding_text(book["title"], authors, book["description"])
        embedding = model.encode(text).tolist()
        db.save_embedding(conn, book["asin"], embedding)

        if progress_callback:
            progress_callback(i + 1, len(books), book["title"])

    return len(books)


def embed_query(query: str, model: SentenceTransformer = None) -> list[float]:
    """Generate embedding for a search query."""
    if model is None:
        model = get_model()
    return model.encode(query).tolist()
