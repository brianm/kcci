"""Generate embeddings for books using ONNX runtime."""

import json
import os
import sqlite3
import sys
from pathlib import Path

import numpy as np  # type: ignore

from . import db

os.environ.setdefault("TOKENIZERS_PARALLELISM", "false")

# Model optimized for semantic search - trained on 215M Q&A pairs
MODEL_NAME = "multi-qa-mpnet-base-cos-v1"

# Cache loaded model to avoid reloading for each embedding
_cached_model = None


def get_onnx_cache_dir() -> Path:
    """Get the ONNX model directory.

    Checks for bundled model first (PyInstaller), then falls back to user cache.
    """
    # Check for bundled model (PyInstaller)
    if getattr(sys, 'frozen', False):
        return Path(sys._MEIPASS) / "onnx-model"
    # Check for KCCI_DATA_DIR override
    data_dir = os.environ.get("KCCI_DATA_DIR")
    if data_dir:
        return Path(data_dir) / "onnx-model"
    # Fall back to user cache
    return Path.home() / ".kcci" / "onnx-model"


def get_onnx_model():
    """Load ONNX model for encoding. Returns (tokenizer, session).

    Uses lightweight tokenizers + onnxruntime (no torch needed).
    Model must be pre-exported and bundled with the app.
    """
    global _cached_model
    if _cached_model is not None:
        return _cached_model

    import onnxruntime as ort  # type: ignore
    from tokenizers import Tokenizer  # type: ignore

    cache_dir = get_onnx_cache_dir()
    tokenizer_path = cache_dir / "tokenizer.json"
    model_path = cache_dir / "model.onnx"

    if not tokenizer_path.exists() or not model_path.exists():
        raise FileNotFoundError(
            f"ONNX model not found at {cache_dir}. "
            "Model must be pre-exported and bundled with the app."
        )

    tokenizer = Tokenizer.from_file(str(tokenizer_path))
    session = ort.InferenceSession(str(model_path))
    _cached_model = (tokenizer, session)
    return _cached_model


def embed_text_onnx(text: str) -> list[float]:
    """Generate embedding for any text using ONNX backend."""
    tokenizer, session = get_onnx_model()

    # Tokenize
    encoded = tokenizer.encode(text)
    input_ids = np.array([encoded.ids], dtype=np.int64)
    attention_mask = np.array([encoded.attention_mask], dtype=np.int64)

    # Run inference
    outputs = session.run(None, {
        'input_ids': input_ids,
        'attention_mask': attention_mask,
    })
    token_embs = outputs[0]  # Shape: [1, seq_len, hidden_size]

    # Mean pooling (matching sentence-transformers)
    input_mask_expanded = np.expand_dims(attention_mask, -1)
    sum_embs = np.sum(token_embs * input_mask_expanded, axis=1)
    sum_mask = np.clip(np.sum(input_mask_expanded, axis=1), a_min=1e-9, a_max=None)
    embedding = (sum_embs / sum_mask)[0]

    # L2 normalize (sentence-transformers does this automatically)
    norm = np.linalg.norm(embedding)
    if norm > 0:
        embedding = embedding / norm

    return embedding.tolist()


# Alias for backward compatibility
embed_query_onnx = embed_text_onnx


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

    # Pre-load model once for the batch
    get_onnx_model()

    for i, book in enumerate(books):
        authors = json.loads(book["authors"])
        text = get_embedding_text(book["title"], authors, book["description"])
        embedding = embed_text_onnx(text)
        db.save_embedding(conn, book["asin"], embedding)

        if progress_callback:
            progress_callback(i + 1, len(books), book["title"])

    return len(books)
