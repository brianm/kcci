use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use futures::StreamExt;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::{BookWithMeta, Database, SearchFilter, Stats};
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

/// Filter condition for querying books
#[derive(serde::Deserialize, Clone)]
pub struct Filter {
    pub field: String,  // "title", "author", "description", "subject"
    pub op: String,     // "contains", "has"
    pub value: String,
}

/// Model availability status
#[derive(serde::Serialize)]
pub struct ModelStatus {
    pub available: bool,
    pub size_mb: u64,
}

/// Model download progress
#[derive(Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub percent: f32,
    pub file: String,
}

/// HuggingFace model files to download
const MODEL_FILES: &[(&str, u64)] = &[
    ("model.onnx", 435_826_548),
    ("tokenizer.json", 711_649),
    ("config.json", 612),
    ("tokenizer_config.json", 1_578),
    ("special_tokens_map.json", 964),
    ("vocab.txt", 231_508),
];

const MODEL_BASE_URL: &str = "https://huggingface.co/sentence-transformers/multi-qa-mpnet-base-cos-v1/resolve/main";

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

/// Get paginated list of all books with optional sorting and filtering
#[tauri::command]
pub fn list_books(
    db: State<DbState>,
    page: Option<usize>,
    per_page: Option<usize>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    filters: Option<Vec<Filter>>,
) -> Result<PaginatedBooks> {
    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(50);
    let offset = (page - 1) * per_page;
    let filters = filters.unwrap_or_default();

    let db = db.0.lock().unwrap();
    let books = db.get_all_books(
        per_page,
        offset,
        sort_by.as_deref(),
        sort_dir.as_deref(),
        &filters,
    )?;
    let total = db.get_book_count_filtered(&filters)?;
    let total_pages = (total + per_page - 1) / per_page;

    Ok(PaginatedBooks {
        books,
        page,
        per_page,
        total,
        total_pages,
    })
}

/// Get all distinct subjects for filtering
#[tauri::command]
pub fn get_subjects(db: State<DbState>) -> Result<Vec<String>> {
    let db = db.0.lock().unwrap();
    db.get_subjects()
}

/// Browse books with structured filters (search chips)
#[tauri::command]
pub fn browse_filtered(
    db: State<DbState>,
    filters: Vec<SearchFilter>,
    page: Option<usize>,
    per_page: Option<usize>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
) -> Result<PaginatedBooks> {
    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(50);
    let offset = (page - 1) * per_page;

    let db = db.0.lock().unwrap();
    let books = db.search_filtered(
        &filters,
        per_page,
        offset,
        sort_by.as_deref(),
        sort_dir.as_deref(),
    )?;
    let total = db.get_filtered_count(&filters)?;
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

    let app_clone = app.clone();
    let db_lock = db.0.lock().unwrap();

    // Use block_in_place to allow blocking operations (reqwest::blocking)
    // within this async context
    let stats = tokio::task::block_in_place(|| {
        sync::sync(&app_clone, &db_lock, path.as_deref(), &model_dir)
    })?;

    Ok(stats)
}

/// Clear all metadata to allow re-enrichment
#[tauri::command]
pub fn clear_metadata(db: State<DbState>) -> Result<usize> {
    let db = db.0.lock().unwrap();
    db.clear_metadata()
}

/// Check if the embedding model is available
#[tauri::command]
pub fn get_model_status(app: AppHandle) -> Result<ModelStatus> {
    let model_dir = get_model_dir(&app)?;
    let model_path = model_dir.join("model.onnx");
    Ok(ModelStatus {
        available: model_path.exists(),
        size_mb: 437,
    })
}

/// Download the embedding model from HuggingFace
#[tauri::command]
pub async fn download_model(app: AppHandle) -> Result<()> {
    let model_dir = get_download_dir(&app)?;
    fs::create_dir_all(&model_dir)?;

    let client = reqwest::Client::new();
    let total_size: u64 = MODEL_FILES.iter().map(|(_, size)| size).sum();
    let mut downloaded: u64 = 0;

    for (filename, expected_size) in MODEL_FILES {
        let url = if *filename == "model.onnx" {
            format!("{}/onnx/{}", MODEL_BASE_URL, filename)
        } else {
            format!("{}/{}", MODEL_BASE_URL, filename)
        };

        let dest_path = model_dir.join(filename);
        let temp_path = model_dir.join(format!("{}.tmp", filename));

        // Download with progress
        let response = client.get(&url).send().await.map_err(|e| {
            crate::error::KcciError::Io(std::io::Error::other(format!(
                "Failed to download {}: {}",
                filename, e
            )))
        })?;

        if !response.status().is_success() {
            return Err(crate::error::KcciError::Io(std::io::Error::other(format!(
                "Failed to download {}: HTTP {}",
                filename,
                response.status()
            ))));
        }

        let mut file = File::create(&temp_path)?;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                crate::error::KcciError::Io(std::io::Error::other(format!(
                    "Download error for {}: {}",
                    filename, e
                )))
            })?;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;

            let _ = app.emit(
                "model-download-progress",
                DownloadProgress {
                    bytes_downloaded: downloaded,
                    total_bytes: total_size,
                    percent: (downloaded as f32 / total_size as f32) * 100.0,
                    file: filename.to_string(),
                },
            );
        }

        // Verify size
        let actual_size = fs::metadata(&temp_path)?.len();
        if actual_size != *expected_size {
            fs::remove_file(&temp_path)?;
            return Err(crate::error::KcciError::Io(std::io::Error::other(format!(
                "Size mismatch for {}: expected {} bytes, got {}",
                filename, expected_size, actual_size
            ))));
        }

        // Atomic rename
        fs::rename(&temp_path, &dest_path)?;
    }

    Ok(())
}

/// Get the directory where the model should be downloaded (app data dir)
fn get_download_dir(app: &AppHandle) -> Result<PathBuf> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| crate::error::KcciError::Io(std::io::Error::other(e.to_string())))?;
    Ok(app_data.join("onnx-model"))
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
