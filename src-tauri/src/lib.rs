mod amazon_import;
mod commands;
mod db;
mod embed;
mod enrich;
mod error;
mod import;
mod sync;

use commands::{get_db_path, DbState};
use db::Database;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{AboutMetadata, Menu, Submenu};
use tauri::{AppHandle, Emitter, Manager};

/// Get the ONNX model directory (bundled or downloaded)
fn get_model_dir(app: &AppHandle) -> Option<PathBuf> {
    // Try resource directory first (bundled with app)
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("binaries").join("onnx-model");
        if bundled.join("model.onnx").exists() {
            return Some(bundled);
        }
    }

    // Fall back to app data directory
    if let Ok(app_data) = app.path().app_data_dir() {
        let downloaded = app_data.join("onnx-model");
        if downloaded.join("model.onnx").exists() {
            return Some(downloaded);
        }
    }

    None
}

/// Auto-generate embeddings for books that need them (runs in background)
fn auto_embed_books(app: AppHandle) {
    std::thread::spawn(move || {
        let model_dir = match get_model_dir(&app) {
            Some(dir) => dir,
            None => {
                log::debug!("Model not available, skipping auto-embedding");
                return;
            }
        };

        let db_state: tauri::State<DbState> = app.state();
        let db = db_state.0.lock().unwrap();

        let books = match db.get_books_for_embedding() {
            Ok(books) => books,
            Err(e) => {
                log::error!("Failed to get books for embedding: {}", e);
                return;
            }
        };

        if books.is_empty() {
            log::debug!("No books need embedding");
            return;
        }

        log::info!("Auto-embedding {} books in background", books.len());
        let _ = app.emit("auto-embed-start", books.len());

        if let Err(e) = embed::init_embedder(&model_dir) {
            log::error!("Failed to init embedder: {}", e);
            return;
        }

        for (i, book) in books.iter().enumerate() {
            let text = embed::get_embedding_text(&book.title, &book.authors, &book.description);
            match embed::embed_text(&text) {
                Ok(embedding) => {
                    if let Err(e) = db.save_embedding(&book.asin, &embedding) {
                        log::error!("Failed to save embedding for {}: {}", book.asin, e);
                    }
                }
                Err(e) => {
                    log::error!("Failed to embed {}: {}", book.asin, e);
                }
            }

            if (i + 1) % 50 == 0 {
                log::info!("Auto-embedded {}/{} books", i + 1, books.len());
            }
        }

        log::info!("Auto-embedding complete: {} books", books.len());
        let _ = app.emit("auto-embed-complete", books.len());
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            // Create custom menu with proper About metadata
            let about_metadata = AboutMetadata {
                name: Some("Ook".into()),
                version: Some(env!("CARGO_PKG_VERSION").into()),
                authors: Some(vec!["Brian McCallister".into()]),
                comments: Some("A personal book catalog with semantic search".into()),
                license: Some("Apache-2.0".into()),
                website: Some("https://github.com/brianm/ook".into()),
                ..Default::default()
            };

            let menu = Menu::with_items(
                app,
                &[
                    &Submenu::with_id_and_items(
                        app,
                        "app",
                        "Ook",
                        true,
                        &[
                            &tauri::menu::PredefinedMenuItem::about(
                                app,
                                Some("About Ook"),
                                Some(about_metadata),
                            )?,
                            &tauri::menu::PredefinedMenuItem::separator(app)?,
                            &tauri::menu::PredefinedMenuItem::services(app, Some("Services"))?,
                            &tauri::menu::PredefinedMenuItem::separator(app)?,
                            &tauri::menu::PredefinedMenuItem::hide(app, Some("Hide Ook"))?,
                            &tauri::menu::PredefinedMenuItem::hide_others(
                                app,
                                Some("Hide Others"),
                            )?,
                            &tauri::menu::PredefinedMenuItem::show_all(app, Some("Show All"))?,
                            &tauri::menu::PredefinedMenuItem::separator(app)?,
                            &tauri::menu::PredefinedMenuItem::quit(app, Some("Quit Ook"))?,
                        ],
                    )?,
                    &Submenu::with_id_and_items(
                        app,
                        "edit",
                        "Edit",
                        true,
                        &[
                            &tauri::menu::PredefinedMenuItem::undo(app, Some("Undo"))?,
                            &tauri::menu::PredefinedMenuItem::redo(app, Some("Redo"))?,
                            &tauri::menu::PredefinedMenuItem::separator(app)?,
                            &tauri::menu::PredefinedMenuItem::cut(app, Some("Cut"))?,
                            &tauri::menu::PredefinedMenuItem::copy(app, Some("Copy"))?,
                            &tauri::menu::PredefinedMenuItem::paste(app, Some("Paste"))?,
                            &tauri::menu::PredefinedMenuItem::select_all(app, Some("Select All"))?,
                        ],
                    )?,
                    &Submenu::with_id_and_items(
                        app,
                        "window",
                        "Window",
                        true,
                        &[
                            &tauri::menu::PredefinedMenuItem::minimize(app, Some("Minimize"))?,
                            &tauri::menu::PredefinedMenuItem::maximize(app, Some("Zoom"))?,
                            &tauri::menu::PredefinedMenuItem::separator(app)?,
                            &tauri::menu::PredefinedMenuItem::close_window(
                                app,
                                Some("Close Window"),
                            )?,
                        ],
                    )?,
                ],
            )?;

            // Set the menu on the main window
            if let Some(window) = app.get_webview_window("main") {
                window.set_menu(menu)?;
            }
            // Set up logging in debug mode
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Debug)
                        .build(),
                )?;
            }

            // Initialize database (migrations run automatically in open())
            let db_path = get_db_path(app.handle())?;
            log::info!("Opening database at {:?}", db_path);

            let database = Database::open(db_path)?;
            log::info!("Database initialized");

            app.manage(DbState(Mutex::new(database)));

            // Auto-generate embeddings for books that need them (background)
            auto_embed_books(app.handle().clone());

            // Show the main window (it starts hidden in tauri.conf.json)
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_stats,
            commands::search,
            commands::get_book,
            commands::list_books,
            commands::get_subjects,
            commands::browse_filtered,
            commands::sync_library,
            commands::clear_metadata,
            commands::get_model_status,
            commands::download_model,
            commands::export_csv,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
