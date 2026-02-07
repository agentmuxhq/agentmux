mod commands;
mod sidecar;
mod state;

use tauri::Manager;

/// Initialize and run the WaveMux Tauri application.
///
/// This replaces the Electron main process (emain/emain.ts).
/// The Go backend (wavemuxsrv) is spawned as a sidecar process,
/// and the React frontend connects to it via WebSocket/HTTP.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        // Plugins
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_websocket::init())
        .plugin(
            tauri_plugin_single_instance::init(|app, _args, _cwd| {
                // Focus the main window when a second instance is launched
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_focus();
                }
            }),
        )
        // Managed state
        .manage(state::AppState::default())
        // Commands (IPC handlers replacing Electron's ipcMain)
        .invoke_handler(tauri::generate_handler![
            // Platform commands
            commands::platform::get_platform,
            commands::platform::get_user_name,
            commands::platform::get_host_name,
            commands::platform::get_is_dev,
            commands::platform::get_data_dir,
            commands::platform::get_config_dir,
            commands::platform::get_env,
            commands::platform::get_about_modal_details,
            commands::platform::get_docsite_url,
            // Auth commands
            commands::auth::get_auth_key,
            // Window commands
            commands::window::open_new_window,
            commands::window::get_zoom_factor,
            commands::window::set_zoom_factor,
            commands::window::get_cursor_point,
            // Backend commands
            commands::backend::get_backend_endpoints,
            commands::backend::fe_log,
            // Stub commands (to be implemented in later phases)
            commands::stubs::show_context_menu,
            commands::stubs::download_file,
            commands::stubs::quicklook,
            commands::stubs::update_wco,
            commands::stubs::set_keyboard_chord_mode,
            commands::stubs::register_global_webview_keys,
            commands::stubs::create_workspace,
            commands::stubs::switch_workspace,
            commands::stubs::delete_workspace,
            commands::stubs::set_active_tab,
            commands::stubs::create_tab,
            commands::stubs::close_tab,
            commands::stubs::set_window_init_status,
            commands::stubs::set_waveai_open,
            commands::stubs::install_update,
        ])
        // Application setup
        .setup(|app| {
            let handle = app.handle().clone();

            // Initialize logging
            init_logging(&handle);

            // Spawn the Go backend as a sidecar
            tauri::async_runtime::spawn(async move {
                match sidecar::spawn_backend(&handle).await {
                    Ok(backend_state) => {
                        // Store backend endpoints in app state
                        let state = handle.state::<state::AppState>();
                        let mut endpoints = state.backend_endpoints.lock().unwrap();
                        endpoints.ws_endpoint = backend_state.ws_endpoint;
                        endpoints.web_endpoint = backend_state.web_endpoint;
                        tracing::info!("Backend ready: ws={}, web={}",
                            endpoints.ws_endpoint, endpoints.web_endpoint);

                        // Emit event to frontend that backend is ready
                        if let Some(window) = handle.get_webview_window("main") {
                            let _ = window.emit("backend-ready", serde_json::json!({
                                "ws": endpoints.ws_endpoint.clone(),
                                "web": endpoints.web_endpoint.clone(),
                            }));
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to start backend: {}", e);
                        // Show error dialog and quit
                        let _ = handle.emit("backend-error", e.clone());
                    }
                }
            });

            Ok(())
        })
        // Window event handling
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    // Graceful shutdown: kill the backend sidecar
                    let state = window.app_handle().state::<state::AppState>();
                    let mut sidecar = state.sidecar_child.lock().unwrap();
                    if let Some(child) = sidecar.take() {
                        tracing::info!("Shutting down backend sidecar");
                        let _ = child.kill();
                    }
                    // Allow the close to proceed
                    drop(api);
                }
                tauri::WindowEvent::Focused(focused) => {
                    if let Some(w) = window.app_handle().get_webview_window("main") {
                        let _ = w.emit("window-focused", focused);
                    }
                }
                tauri::WindowEvent::Resized(size) => {
                    let _ = window.emit("window-resized", serde_json::json!({
                        "width": size.width,
                        "height": size.height,
                    }));
                }
                _ => {}
            }
        });

    builder
        .run(tauri::generate_context!())
        .expect("error while running WaveMux");
}

fn init_logging(handle: &tauri::AppHandle) {
    use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

    let log_dir = handle
        .path()
        .app_log_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let _ = std::fs::create_dir_all(&log_dir);

    let file_appender = tracing_appender::rolling::daily(&log_dir, "wavemux.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Keep the guard alive for the lifetime of the app
    // by leaking it (acceptable for a long-running app)
    std::mem::forget(_guard);

    let subscriber = tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("wavemux=info,warn")),
        )
        .with(fmt::layer().with_writer(non_blocking))
        .with(fmt::layer().with_writer(std::io::stderr));

    tracing::subscriber::set_global_default(subscriber).ok();
    tracing::info!("WaveMux starting");
}
