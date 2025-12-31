mod commands;
mod db;
mod embed;
mod enrich;
mod error;
mod sync;
mod webarchive;

use commands::{get_db_path, DbState};
use db::Database;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            // Set up logging in debug mode
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Debug)
                        .build(),
                )?;
            }

            // Initialize database
            let db_path = get_db_path(app.handle())?;
            log::info!("Opening database at {:?}", db_path);

            let database = Database::open(db_path)?;
            database.init_schema()?;
            log::info!("Database initialized");

            app.manage(DbState(Mutex::new(database)));

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
