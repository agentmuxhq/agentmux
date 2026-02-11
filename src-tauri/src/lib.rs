pub mod domain;
pub mod infrastructure;
pub mod services;
mod backend;
mod commands;
mod crash;
mod heartbeat;
mod menu;
mod rust_backend;
mod state;
mod tray;

use tauri::Emitter;
use tauri::Manager;

/// Initialize and run the AgentMux Tauri application.
///
/// Uses in-process Rust backend (no external sidecar process).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        // Plugins
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
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
        // TODO: enable once first GitHub Release with latest.json is published
        // .plugin(tauri_plugin_updater::Builder::new().build())
        ;

    // Register muxfile:// custom protocol for file streaming
    let builder = builder.register_asynchronous_uri_scheme_protocol(
        "muxfile",
        |_ctx, request, responder| {
            crate::backend::filestream::handle_muxfile_protocol(request, responder);
        },
    );

    let builder = builder
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
            commands::window::close_window,
            commands::window::get_window_label,
            commands::window::is_main_window,
            commands::window::list_windows,
            commands::window::focus_window,
            commands::window::get_zoom_factor,
            commands::window::set_zoom_factor,
            commands::window::get_cursor_point,
            // Backend commands
            commands::backend::get_backend_endpoints,
            commands::backend::get_wave_init_opts,
            commands::backend::fe_log,
            // Devtools commands
            commands::devtools::toggle_devtools,
            commands::devtools::is_devtools_open,
            // Context menu (native popup)
            commands::contextmenu::show_context_menu,
            // Workspace commands
            commands::workspace::create_workspace,
            commands::workspace::switch_workspace,
            commands::workspace::delete_workspace,
            // Tab commands
            commands::tab::set_active_tab,
            commands::tab::create_tab,
            commands::tab::close_tab,
            // Misc commands
            commands::stubs::download_file,
            commands::stubs::quicklook,
            commands::stubs::update_wco,
            commands::stubs::set_keyboard_chord_mode,
            commands::stubs::register_global_webview_keys,
            commands::stubs::set_window_init_status,
            commands::stubs::set_waveai_open,
            commands::updater::install_update,
            // Agent backend commands (unified AI pane)
            commands::agent::spawn_agent,
            commands::agent::send_agent_input,
            commands::agent::interrupt_agent,
            commands::agent::kill_agent,
            commands::agent::get_agent_status,
            commands::agent::list_agent_backends,
            // RPC bridge commands
            commands::rpc::rpc_request,
            commands::rpc::service_request,
            commands::rpc::set_block_term_size,
            // File and reactive commands
            commands::rpc::file::fetch_wave_file,
            commands::rpc::reactive::reactive_register,
            commands::rpc::reactive::reactive_unregister,
            commands::rpc::reactive::reactive_inject,
            commands::rpc::reactive::reactive_poller_config,
            commands::rpc::schema::get_schema,
        ])
        // Application setup
        .setup(|app| {
            let handle = app.handle().clone();

            // Initialize logging
            let log_dir = init_logging(&handle);

            // Initialize crash handler
            crash::init_crash_handler(log_dir.clone());

            // Start heartbeat monitoring
            let data_dir = handle.path().app_data_dir().unwrap_or_else(|_| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            });
            tauri::async_runtime::spawn(heartbeat::start_heartbeat(data_dir.clone()));

            // Build and set application menu
            match menu::build_app_menu(&handle) {
                Ok(app_menu) => {
                    if let Err(e) = handle.set_menu(app_menu) {
                        tracing::error!("Failed to set application menu: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to build application menu: {}", e);
                }
            }

            // Build system tray icon
            if let Err(e) = tray::build_tray(&handle) {
                tracing::error!("Failed to build system tray: {}", e);
            }

            // Set window title with version
            if let Some(window) = handle.get_webview_window("main") {
                let version = env!("CARGO_PKG_VERSION");
                let title = format!("AgentMux {}", version);
                if let Err(e) = window.set_title(&title) {
                    tracing::error!("Failed to set window title: {}", e);
                }

                // In dev mode, navigate to Vite dev server.
                // generate_context!() embeds frontendDist, but in dev we want
                // to load from the Vite dev server for hot reload.
                #[cfg(debug_assertions)]
                {
                    let dev_url = std::env::var("TAURI_DEV_URL")
                        .or_else(|_| std::env::var("VITE_DEV_SERVER_URL"))
                        .unwrap_or_else(|_| "http://localhost:5173".to_string());
                    tracing::info!("dev mode: navigating webview to {}", dev_url);
                    if let Ok(url) = dev_url.parse::<tauri::Url>() {
                        let _ = window.navigate(url);
                    }
                }
            }

            // ---- Backend initialization ----

            // Initialize Rust-native backend (no external process)
            // This creates and manages AppState on the app handle
            match rust_backend::initialize(app) {
                Ok(()) => {
                    tracing::info!("Rust-native backend initialized successfully");
                }
                Err(e) => {
                    tracing::error!("Failed to initialize Rust backend: {}", e);
                    let _ = app.handle().emit("backend-error", e.clone());
                }
            }

            // TODO: enable once first GitHub Release with latest.json is published
            // let check_handle = app.handle().clone();
            // tauri::async_runtime::spawn(
            //     commands::updater::check_for_updates_background(check_handle),
            // );

            Ok(())
        })
        // Menu event handling
        .on_menu_event(|app, event| {
            menu::handle_menu_event(app, event);
        })
        // Window event handling
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    // Kill any running agent subprocesses
                    {
                        let registry = window
                            .app_handle()
                            .state::<std::sync::Arc<backend::ai::agent::AgentRegistry>>();
                        let process_store = window
                            .app_handle()
                            .state::<std::sync::Arc<commands::agent::AgentProcessStore>>();
                        let pane_ids = registry.active_pane_ids();
                        if !pane_ids.is_empty() {
                            tracing::info!(
                                "Shutting down {} active agent(s)",
                                pane_ids.len()
                            );
                            let ps = std::sync::Arc::clone(&process_store);
                            tauri::async_runtime::spawn(async move {
                                for pane_id in pane_ids {
                                    if let Some(mut proc) = ps.remove(&pane_id).await {
                                        let _ = proc.kill().await;
                                    }
                                }
                            });
                        }
                    }

                    // Clean up heartbeat file
                    if let Ok(data_dir) = window.app_handle().path().app_data_dir() {
                        heartbeat::cleanup_heartbeat(&data_dir);
                    }

                    // Allow the close to proceed
                    let _ = api;
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

    let builder = builder.manage(commands::updater::PendingUpdate(std::sync::Mutex::new(None)));

    // Agent backend state (unified AI pane — available in all modes)
    let builder = builder
        .manage(std::sync::Arc::new(
            backend::ai::agent::AgentRegistry::new(),
        ))
        .manage(std::sync::Arc::new(
            commands::agent::AgentProcessStore::new(),
        ));

    builder
        .run(tauri::generate_context!())
        .expect("error while running AgentMux");
}

fn init_logging(handle: &tauri::AppHandle) -> std::path::PathBuf {
    use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter};

    let log_dir = handle
        .path()
        .app_log_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let _ = std::fs::create_dir_all(&log_dir);

    let file_appender = tracing_appender::rolling::daily(&log_dir, "agentmux.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Keep the guard alive for the lifetime of the app
    // by leaking it (acceptable for a long-running app)
    std::mem::forget(_guard);

    let subscriber = tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("agentmux=info,warn")),
        )
        .with(fmt::layer().with_writer(non_blocking))
        .with(fmt::layer().with_writer(std::io::stderr));

    tracing::subscriber::set_global_default(subscriber).ok();
    tracing::info!("AgentMux starting");

    log_dir
}
