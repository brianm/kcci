use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

use crate::db::{BookWithMeta, Database, Stats};
use crate::embed;
use crate::error::Result;
use crate::sync::{self, SyncStats};

/// Thread-safe database wrapper for Tauri state
pub struct DbState(pub Mutex<Database>);

/// Pagination info
#[derive(serde::Serialize)]
pub struct PaginatedBooks {
    pub books: Vec<BookWithMeta>,
    pub page: usize,
    pub per_page: usize,
    pub total: usize,
    pub total_pages: usize,
}

// === Tauri Commands ===

/// Get database statistics
#[tauri::command]
pub fn get_stats(db: State<DbState>) -> Result<Stats> {
    let db = db.0.lock().unwrap();
    db.get_stats()
}

/// Search for books (FTS or semantic)
#[tauri::command]
pub fn search(
    db: State<DbState>,
    app: AppHandle,
    query: String,
    mode: String,
    limit: Option<usize>,
) -> Result<Vec<BookWithMeta>> {
    let limit = limit.unwrap_or(100);

    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let db = db.0.lock().unwrap();

    if mode == "semantic" {
        let model_dir = get_model_dir(&app)?;
        embed::init_embedder(&model_dir)?;
        let embedding = embed::embed_text(&query)?;
        db.search_semantic(&embedding, limit)
    } else {
        db.search_fts(&query, limit)
    }
}

/// Get a single book by ASIN
#[tauri::command]
pub fn get_book(db: State<DbState>, asin: String) -> Result<Option<BookWithMeta>> {
    let db = db.0.lock().unwrap();
    db.get_book_by_asin(&asin)
}

/// Get paginated list of all books
#[tauri::command]
pub fn list_books(
    db: State<DbState>,
    page: Option<usize>,
    per_page: Option<usize>,
) -> Result<PaginatedBooks> {
    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(50);
    let offset = (page - 1) * per_page;

    let db = db.0.lock().unwrap();
    let books = db.get_all_books(per_page, offset)?;
    let total = db.get_book_count()?;
    let total_pages = (total + per_page - 1) / per_page;

    Ok(PaginatedBooks {
        books,
        page,
        per_page,
        total,
        total_pages,
    })
}

/// Sync library: import, enrich, embed
#[tauri::command]
pub async fn sync_library(
    app: AppHandle,
    db: State<'_, DbState>,
    webarchive_path: Option<String>,
) -> Result<SyncStats> {
    let path = webarchive_path.map(PathBuf::from);
    let model_dir = get_model_dir(&app)?;

    // Run sync in blocking thread since it's CPU/IO bound
    let app_clone = app.clone();
    let db_lock = db.0.lock().unwrap();

    // We need to run this in a blocking context
    // Since Database is not Send, we need to handle this carefully
    let stats = sync::sync(&app_clone, &db_lock, path.as_deref(), &model_dir)?;

    Ok(stats)
}

/// Get the ONNX model directory
fn get_model_dir(app: &AppHandle) -> Result<PathBuf> {
    // Try resource directory first (bundled with app)
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("binaries").join("onnx-model");
        if bundled.exists() {
            return Ok(bundled);
        }
    }

    // Fall back to app data directory
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| crate::error::KcciError::Io(std::io::Error::other(e.to_string())))?;
    Ok(app_data.join("onnx-model"))
}

/// Get the database path (~/Library/Application Support/KCCI/books.db for backward compatibility)
pub fn get_db_path(_app: &AppHandle) -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| crate::error::KcciError::Io(std::io::Error::other("HOME not set")))?;
    let kcci_dir = PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("KCCI");
    std::fs::create_dir_all(&kcci_dir)?;
    Ok(kcci_dir.join("books.db"))
}
