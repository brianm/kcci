# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Version Control

This project uses **jj (Jujutsu)**, not git. Work on the `redo` bookmark - do not make changes to `main`.

## Project Overview

Keith's Card Catalog Index (KCCI) - a Tauri desktop app for personal book indexing with semantic search. Imports Kindle library from Safari webarchive, fetches metadata from OpenLibrary, generates embeddings using ONNX runtime, and stores them in SQLite with vector search (sqlite-vec).

## Commands

```bash
# Development
cd ui && npm run dev          # Start Svelte dev server (port 5173)
cargo tauri dev               # Run Tauri app in dev mode

# Build
cd ui && npm run build        # Build Svelte frontend
cargo tauri build             # Build release app bundle

# Rust
cargo check                   # Type check Rust code
cargo test                    # Run Rust tests
cargo fmt                     # Format Rust code
```

## Architecture

```
src-tauri/src/
├── main.rs           # Entry point
├── lib.rs            # Tauri app setup + command registration
├── error.rs          # KcciError enum with thiserror
├── commands.rs       # Tauri commands (get_stats, search, sync_library, etc.)
├── db/
│   ├── mod.rs        # SQLite + sqlite-vec operations
│   └── schema.sql    # Database schema
├── embed.rs          # ONNX inference with ort crate
├── enrich.rs         # OpenLibrary API client
├── sync.rs           # Import pipeline orchestration
└── webarchive.rs     # Safari webarchive parser (plist)

ui/src/
├── App.svelte        # Main app with routing
├── lib/api.ts        # TypeScript Tauri command wrappers
├── routes/
│   ├── Search.svelte # Live search with keyboard navigation
│   ├── Browse.svelte # Infinite scroll book list
│   └── Import.svelte # Webarchive import with progress
└── components/
    └── BookCard.svelte

src-tauri/binaries/onnx-model/
├── model.onnx        # all-MiniLM-L6-v2 embedding model
└── tokenizer.json    # HuggingFace tokenizer
```

**Data flow:** Safari webarchive -> webarchive.rs (parse) -> db (store) -> enrich.rs (OpenLibrary) -> embed.rs (ONNX) -> sqlite-vec (search)

## Key Dependencies

### Rust (src-tauri/Cargo.toml)
- `rusqlite` + `sqlite-vec` - SQLite with vector search
- `ort` - ONNX Runtime for embedding inference
- `tokenizers` - HuggingFace tokenizer
- `plist` - Apple binary plist parsing
- `reqwest` - HTTP client for OpenLibrary API
- `tauri` - Desktop app framework

### Frontend (ui/package.json)
- `svelte` - Reactive UI framework
- `@tauri-apps/api` - Tauri IPC bindings
- `@tauri-apps/plugin-dialog` - File picker dialog
- `@tauri-apps/plugin-shell` - Open external links in browser
- `marked` - Markdown rendering for book descriptions
