# Keith's Card Catalog Index (KCCI)

A desktop app for personal book library management with semantic search. Import your Kindle library, enrich with OpenLibrary metadata, and search by meaning rather than just keywords.

## Features

- **Import Kindle Library**: Parse Safari webarchive exports from read.amazon.com
- **Metadata Enrichment**: Automatically fetch book details from OpenLibrary (descriptions, subjects, publish dates)
- **Semantic Search**: Find books by meaning using sentence embeddings (all-MiniLM-L6-v2)
- **Keyword Search**: Full-text search with SQLite FTS5
- **Keyboard Navigation**: Arrow keys to navigate results, Enter to expand

## Tech Stack

- **Frontend**: Svelte + TypeScript + Vite
- **Backend**: Rust + Tauri v2
- **Database**: SQLite + sqlite-vec (vector search)
- **Embeddings**: ONNX Runtime (all-MiniLM-L6-v2 model)

## Development

```bash
# Install dependencies
cd ui && npm install

# Start dev server
cd ui && npm run dev    # Svelte dev server on port 5173
cargo tauri dev         # Run Tauri app in dev mode

# Build release
cd ui && npm run build
cargo tauri build
```

## Data Location

- Database: `~/Library/Application Support/KCCI/books.db`
- The ONNX model is bundled with the app (~416MB)

## How to Import Your Kindle Library

1. Open Safari and go to [read.amazon.com](https://read.amazon.com)
2. Sign in with your Amazon account
3. **Scroll down repeatedly** until all your books are loaded (the page lazy-loads as you scroll)
4. From the menu bar, choose **File > Save As...**
5. Set Format to **"Web Archive"**
6. Save the file, then use KCCI's Import tab to select it

## Notice

This tool processes user-provided HTML files and makes no network requests to Amazon. Users are responsible for compliance with any applicable terms for services they use. KCCI is intended for personal library management.

## License

Apache-2.0

See [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt) for third-party component licenses.

Note: The embedding model (all-MiniLM-L6-v2) is Apache-2.0 licensed, but its training data includes datasets with commercial use restrictions.
