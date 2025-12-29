use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;

// Store the server port globally so we can access it
static SERVER_PORT: AtomicU16 = AtomicU16::new(0);

/// Wait for the server to be ready by polling the health endpoint
fn wait_for_server(port: u16) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{}/", port);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Try for up to 30 seconds
    for attempt in 1..=60 {
        match client.get(&url).send() {
            Ok(response) if response.status().is_success() => {
                log::info!("Server ready after {} attempts", attempt);
                return Ok(());
            }
            Ok(response) => {
                log::debug!("Server returned status {}, retrying...", response.status());
            }
            Err(e) => {
                log::debug!("Server not ready (attempt {}): {}", attempt, e);
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    Err("Server did not become ready within 30 seconds".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Set up logging in debug mode
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Debug)
                        .build(),
                )?;
            }

            // Clone the app handle for use in the async task
            let app_handle = app.handle().clone();

            // Spawn the Python sidecar
            let sidecar = app.shell().sidecar("kcci-server")
                .map_err(|e| format!("Failed to create sidecar command: {}", e))?;

            let (mut rx, _child) = sidecar.spawn()
                .map_err(|e| format!("Failed to spawn sidecar: {}", e))?;

            // Store the child process handle (it will be killed when dropped)
            // Note: Tauri manages the sidecar lifecycle automatically

            // Spawn a task to read the sidecar output and get the port
            let app_handle_clone = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut port: Option<u16> = None;

                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stdout(line) => {
                            let line_str = String::from_utf8_lossy(&line);
                            log::info!("Sidecar stdout: {}", line_str);

                            // Look for PORT:XXXXX
                            if let Some(port_str) = line_str.strip_prefix("PORT:") {
                                if let Ok(p) = port_str.trim().parse::<u16>() {
                                    log::info!("Sidecar server port: {}", p);
                                    port = Some(p);
                                    SERVER_PORT.store(p, Ordering::SeqCst);

                                    // Wait for server to be ready in a blocking thread
                                    let port_copy = p;
                                    let app_handle_inner = app_handle_clone.clone();
                                    std::thread::spawn(move || {
                                        match wait_for_server(port_copy) {
                                            Ok(()) => {
                                                // Navigate to the server and show the window
                                                if let Some(window) = app_handle_inner.get_webview_window("main") {
                                                    let url = format!("http://127.0.0.1:{}/", port_copy);
                                                    log::info!("Navigating to {}", url);
                                                    if let Err(e) = window.navigate(url.parse().unwrap()) {
                                                        log::error!("Failed to navigate: {}", e);
                                                    }
                                                    // Show the window after navigation
                                                    std::thread::sleep(Duration::from_millis(100));
                                                    if let Err(e) = window.show() {
                                                        log::error!("Failed to show window: {}", e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                log::error!("Server failed to start: {}", e);
                                            }
                                        }
                                    });
                                }
                            }
                        }
                        CommandEvent::Stderr(line) => {
                            let line_str = String::from_utf8_lossy(&line);
                            log::warn!("Sidecar stderr: {}", line_str);
                        }
                        CommandEvent::Error(err) => {
                            log::error!("Sidecar error: {}", err);
                        }
                        CommandEvent::Terminated(status) => {
                            log::info!("Sidecar terminated with status: {:?}", status);
                            break;
                        }
                        _ => {}
                    }
                }

                if port.is_none() {
                    log::error!("Sidecar exited without providing a port");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
