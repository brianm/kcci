# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Version Control

This project uses **jj (Jujutsu)**, not git. Work on the `redo` bookmark - do not make changes to `main`.

## Project Overview

Keith's Card Catalog Index (KCCI) - a Python-based personal book indexing system with semantic search capabilities. Fetches book metadata from OpenLibrary and Library of Congress, generates embeddings using sentence-transformers, and stores them in SQLite with vector search (sqlite-vss).

## Commands

```bash
# Install dependencies
uv sync

# Run tests
uv run pytest

# Run single test
uv run pytest src/kcci/tests/test_db_stuff.py::test_name

# Format code
uv run black .

# Type check
pyright

# CLI commands
uv run kcci basics <query> [-n NUM]    # Semantic search
uv run kcci ol -t TITLE [-a AUTHOR]    # OpenLibrary lookup
uv run kcci loc -t TITLE [-a AUTHOR]   # Library of Congress lookup
```

## Architecture

```
src/kcci/
├── book_finder.py    # Book class + API lookups (OpenLibrary, LoC)
├── ss_play.py        # Sentence transformer semantic search experiments
├── db_play.py        # SQLite-vss vector storage experiments
└── tests/            # pytest tests

scripts/kcci          # Click-based CLI entry point
```

**Data flow:** External APIs -> book_finder -> sentence-transformers (embeddings) -> sqlite-vss (storage/search)

## Key Dependencies

- `sentence-transformers` - SBERT models for semantic embeddings
- `sqlite-vss` - Vector search extension for SQLite
- `click` - CLI framework
- `requests`/`httpx` - HTTP clients for API calls
