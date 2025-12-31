use serde::Serialize;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use crate::db::{Database, EnrichmentData};
use crate::embed;
use crate::enrich::OpenLibrary;
use crate::error::Result;
use crate::import;

/// Progress update sent via Tauri events
#[derive(Clone, Serialize)]
pub struct SyncProgress {
    pub stage: String,
    pub message: String,
    pub current: Option<usize>,
    pub total: Option<usize>,
}

/// Final statistics from sync
#[derive(Clone, Serialize)]
pub struct SyncStats {
    pub imported: usize,
    pub enriched: usize,
    pub embedded: usize,
}

const ENRICH_DELAY: Duration = Duration::from_millis(250);

/// Full sync pipeline: import -> enrich -> embed
pub fn sync(
    app: &AppHandle,
    db: &Database,
    import_path: Option<&Path>,
    model_dir: &Path,
) -> Result<SyncStats> {
    let mut stats = SyncStats {
        imported: 0,
        enriched: 0,
        embedded: 0,
    };

    let emit = |progress: SyncProgress| {
        let _ = app.emit("sync-progress", progress);
    };

    // Stage 1: Import (if import file provided)
    if let Some(path) = import_path {
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());

        emit(SyncProgress {
            stage: "import".into(),
            message: format!("Reading {}...", filename),
            current: None,
            total: None,
        });

        let books = import::parse_import_file(path)?;

        emit(SyncProgress {
            stage: "import".into(),
            message: format!("Found {} books", books.len()),
            current: None,
            total: None,
        });

        if !books.is_empty() {
            stats.imported = db.import_books(&books)?;

            if stats.imported > 0 {
                db.rebuild_fts()?;
                emit(SyncProgress {
                    stage: "import".into(),
                    message: format!("Imported {} new books", stats.imported),
                    current: None,
                    total: None,
                });
            } else {
                emit(SyncProgress {
                    stage: "import".into(),
                    message: "No new books to import".into(),
                    current: None,
                    total: None,
                });
            }
        }
    }

    // Stage 2: Enrich
    let books_to_enrich = db.get_books_without_metadata()?;
    let total_to_enrich = books_to_enrich.len();

    if total_to_enrich == 0 {
        emit(SyncProgress {
            stage: "enrich".into(),
            message: "All books already enriched".into(),
            current: None,
            total: None,
        });
    } else {
        emit(SyncProgress {
            stage: "enrich".into(),
            message: format!("Enriching {} books...", total_to_enrich),
            current: Some(0),
            total: Some(total_to_enrich),
        });

        let ol = OpenLibrary::new()?;
        let start = Instant::now();

        for (i, book) in books_to_enrich.iter().enumerate() {
            match ol.search(&book.title, &book.authors)? {
                Some(data) => {
                    db.save_metadata(&book.asin, &data)?;
                    if !data.description.is_empty() {
                        stats.enriched += 1;
                    }
                }
                None => {
                    // Save empty metadata to mark as attempted
                    db.save_metadata(
                        &book.asin,
                        &EnrichmentData {
                            openlibrary_key: String::new(),
                            description: String::new(),
                            subjects: Vec::new(),
                            isbn: None,
                            publish_year: None,
                        },
                    )?;
                }
            }

            let elapsed = start.elapsed();
            let eta = estimate_eta(i + 1, total_to_enrich, elapsed);
            let title = truncate_title(&book.title, 40);

            emit(SyncProgress {
                stage: "enrich".into(),
                message: format!(
                    "\"{}\" ({} elapsed, ~{} remaining)",
                    title,
                    format_duration(elapsed),
                    format_duration(eta)
                ),
                current: Some(i + 1),
                total: Some(total_to_enrich),
            });

            if i < total_to_enrich - 1 {
                thread::sleep(ENRICH_DELAY);
            }
        }

        db.rebuild_fts()?;
        emit(SyncProgress {
            stage: "enrich".into(),
            message: format!(
                "Enriched {}/{} with descriptions",
                stats.enriched, total_to_enrich
            ),
            current: Some(total_to_enrich),
            total: Some(total_to_enrich),
        });
    }

    // Stage 3: Embed
    let books_to_embed = db.get_books_for_embedding()?;
    let total_to_embed = books_to_embed.len();

    // Check if model exists before attempting to embed
    let model_path = model_dir.join("model.onnx");
    let model_available = model_path.exists();

    if total_to_embed == 0 {
        emit(SyncProgress {
            stage: "embed".into(),
            message: "All enriched books already have embeddings".into(),
            current: None,
            total: None,
        });
    } else if !model_available {
        emit(SyncProgress {
            stage: "embed".into(),
            message: "Skipping embeddings (model not downloaded)".into(),
            current: None,
            total: None,
        });
    } else {
        emit(SyncProgress {
            stage: "embed".into(),
            message: "Loading embedding model...".into(),
            current: None,
            total: None,
        });

        embed::init_embedder(model_dir)?;

        emit(SyncProgress {
            stage: "embed".into(),
            message: format!("Generating embeddings for {} books...", total_to_embed),
            current: Some(0),
            total: Some(total_to_embed),
        });

        let start = Instant::now();

        for (i, book) in books_to_embed.iter().enumerate() {
            let text = embed::get_embedding_text(&book.title, &book.authors, &book.description);
            let embedding = embed::embed_text(&text)?;
            db.save_embedding(&book.asin, &embedding)?;
            stats.embedded += 1;

            let elapsed = start.elapsed();
            let eta = estimate_eta(i + 1, total_to_embed, elapsed);
            let title = truncate_title(&book.title, 40);

            emit(SyncProgress {
                stage: "embed".into(),
                message: format!(
                    "\"{}\" ({} elapsed, ~{} remaining)",
                    title,
                    format_duration(elapsed),
                    format_duration(eta)
                ),
                current: Some(i + 1),
                total: Some(total_to_embed),
            });
        }

        emit(SyncProgress {
            stage: "embed".into(),
            message: format!("Generated {} embeddings", stats.embedded),
            current: Some(total_to_embed),
            total: Some(total_to_embed),
        });
    }

    // Final complete event
    emit(SyncProgress {
        stage: "complete".into(),
        message: format!(
            "Sync complete: {} imported, {} enriched, {} embedded",
            stats.imported, stats.enriched, stats.embedded
        ),
        current: None,
        total: None,
    });

    Ok(stats)
}

/// Estimate remaining time based on current progress
fn estimate_eta(current: usize, total: usize, elapsed: Duration) -> Duration {
    if current == 0 {
        return Duration::ZERO;
    }
    let rate = current as f64 / elapsed.as_secs_f64();
    let remaining = total - current;
    if rate > 0.0 {
        Duration::from_secs_f64(remaining as f64 / rate)
    } else {
        Duration::ZERO
    }
}

/// Format duration as human-readable string
fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        let mins = secs / 60;
        let secs = secs % 60;
        format!("{}m{}s", mins, secs)
    } else {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        format!("{}h{}m", hours, mins)
    }
}

/// Truncate title to max length
fn truncate_title(title: &str, max_len: usize) -> String {
    if title.len() <= max_len {
        title.to_string()
    } else {
        format!("{}...", &title[..max_len - 3])
    }
}
