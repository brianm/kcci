mod commands;
mod db;
mod embed;
mod enrich;
mod error;
mod import;
mod sync;

use commands::{get_db_path, DbState};
use db::Database;
use std::sync::Mutex;
use tauri::menu::{AboutMetadata, Menu, Submenu};
use tauri::Manager;

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
