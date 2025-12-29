"""Generate embeddings for books using sentence-transformers."""

import json
import os
import sqlite3
import warnings
from pathlib import Path

import numpy as np  # type: ignore

from . import db

# Suppress FutureWarning from transformers about tokenization
warnings.filterwarnings("ignore", message=".*clean_up_tokenization_spaces.*")
os.environ.setdefault("TOKENIZERS_PARALLELISM", "false")

# Model optimized for semantic search - trained on 215M Q&A pairs
MODEL_NAME = "multi-qa-mpnet-base-cos-v1"


def get_onnx_cache_dir() -> Path:
    """Get the ONNX model cache directory."""
    data_dir = os.environ.get("KCCI_DATA_DIR")
    if data_dir:
        return Path(data_dir) / "onnx-model"
    return Path.home() / ".kcci" / "onnx-model"


def get_model():
    """Load the sentence transformer model (for batch operations)."""
    from sentence_transformers import SentenceTransformer  # type: ignore
    return SentenceTransformer(MODEL_NAME)


def get_onnx_model():
    """Load ONNX model for fast query encoding. Returns (tokenizer, model).

    Uses optimum/transformers if available, otherwise falls back to lightweight
    onnxruntime-only approach.
    """
    if not ONNX_CACHE_DIR.exists():
        # Need transformers/optimum to export the model initially
        from optimum.onnxruntime import ORTModelForFeatureExtraction  # type: ignore
        from transformers import AutoTokenizer  # type: ignore

        ONNX_CACHE_DIR.parent.mkdir(parents=True, exist_ok=True)
        hf_model = f"sentence-transformers/{MODEL_NAME}"
        tokenizer = AutoTokenizer.from_pretrained(hf_model)
        model = ORTModelForFeatureExtraction.from_pretrained(hf_model, export=True)
        model.save_pretrained(ONNX_CACHE_DIR)
        tokenizer.save_pretrained(ONNX_CACHE_DIR)
        return tokenizer, model

    # Try lightweight approach first (no torch needed)
    try:
        return _get_onnx_model_lightweight()
    except Exception:
        # Fall back to transformers if lightweight fails
        from optimum.onnxruntime import ORTModelForFeatureExtraction  # type: ignore
        from transformers import AutoTokenizer  # type: ignore

        tokenizer = AutoTokenizer.from_pretrained(ONNX_CACHE_DIR)
        model = ORTModelForFeatureExtraction.from_pretrained(ONNX_CACHE_DIR)
        return tokenizer, model


def _get_onnx_model_lightweight():
    """Load ONNX model using only tokenizers + onnxruntime (no torch)."""
    import onnxruntime as ort  # type: ignore
    from tokenizers import Tokenizer  # type: ignore

    tokenizer_path = ONNX_CACHE_DIR / "tokenizer.json"
    model_path = ONNX_CACHE_DIR / "model.onnx"

    if not tokenizer_path.exists() or not model_path.exists():
        raise FileNotFoundError("ONNX model not cached")

    tokenizer = Tokenizer.from_file(str(tokenizer_path))
    session = ort.InferenceSession(str(model_path))
    return tokenizer, session


def embed_query_onnx(query: str) -> list[float]:
    """Generate embedding using fast ONNX backend."""
    tokenizer, model = get_onnx_model()

    # Check if we got the lightweight model (onnxruntime session)
    if hasattr(model, 'run'):
        # Lightweight path: tokenizers + onnxruntime
        encoded = tokenizer.encode(query)
        input_ids = np.array([encoded.ids], dtype=np.int64)
        attention_mask = np.array([encoded.attention_mask], dtype=np.int64)

        outputs = model.run(None, {
            'input_ids': input_ids,
            'attention_mask': attention_mask,
        })
        token_embs = outputs[0]  # Shape: [1, seq_len, hidden_size]
    else:
        # Full path: transformers/optimum
        inputs = tokenizer(query, return_tensors="np", padding=True, truncation=True)
        outputs = model(**inputs)
        attention_mask = inputs["attention_mask"]
        token_embs = outputs.last_hidden_state
        input_ids = None  # Not needed for this path

    # Mean pooling (matching sentence-transformers)
    if hasattr(model, 'run'):
        # Lightweight path
        input_mask_expanded = np.expand_dims(attention_mask, -1)
    else:
        # Full path
        input_mask_expanded = np.expand_dims(attention_mask, -1)

    sum_embs = np.sum(token_embs * input_mask_expanded, axis=1)
    sum_mask = np.clip(np.sum(input_mask_expanded, axis=1), a_min=1e-9, a_max=None)
    embedding = (sum_embs / sum_mask)[0]

    # L2 normalize (sentence-transformers does this automatically)
    norm = np.linalg.norm(embedding)
    if norm > 0:
        embedding = embedding / norm

    return embedding.tolist()


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


def embed_query(query: str, model=None) -> list[float]:
    """Generate embedding for a search query."""
    if model is None:
        model = get_model()
    return model.encode(query).tolist()
