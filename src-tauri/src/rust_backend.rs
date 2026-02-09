// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! Rust-native backend initialization.
//! Replaces the Go sidecar (wavemuxsrv) with in-process Rust services.
//!
//! Initializes: WaveStore (SQLite), WPS Broker, RPC Engine, RPC Router.
//! The frontend communicates via Tauri IPC instead of WebSocket.

use std::sync::Arc;

use tauri::Emitter;
use tauri::Manager;

use crate::backend::rpc::engine::WshRpcEngine;
use crate::backend::rpc::router::WshRouter;
use crate::backend::storage::filestore::FileStore;
use crate::backend::storage::wstore::WaveStore;
use crate::backend::wcore;
use crate::backend::wps::{Broker, WaveEvent, WpsClient};
use crate::backend::wsh_server;
use crate::state::AppState;

// ---- TauriWpsClient: delivers WPS events to frontend via Tauri emit ----

/// WPS client that bridges the Broker to the Tauri frontend event system.
/// When the Broker publishes events (blockfile, controller status, etc.),
/// this client emits them as Tauri events so the frontend can receive them.
struct TauriWpsClient {
    handle: tauri::AppHandle,
}

impl TauriWpsClient {
    fn new(handle: tauri::AppHandle) -> Self {
        Self { handle }
    }
}

impl WpsClient for TauriWpsClient {
    fn send_event(&self, _route_id: &str, event: WaveEvent) {
        // Emit as a Tauri event that the frontend listens for
        if let Err(e) = self.handle.emit("wps-event", &event) {
            tracing::warn!("Failed to emit wps-event to frontend: {}", e);
        }
    }
}

/// Initialize the Rust-native backend.
///
/// Creates AppState with all backend services, manages it on the Tauri app,
/// bootstraps initial data, and emits backend-ready to the frontend.
///
/// This replaces `sidecar::spawn_backend()` — no external process needed.
pub fn initialize(app: &mut tauri::App) -> Result<(), String> {
    let handle = app.handle();

    let data_dir = handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get data dir: {}", e))?;

    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("failed to create data dir: {}", e))?;

    // Open SQLite store
    let db_path = data_dir.join("waveterm.db");
    tracing::info!("Opening WaveStore at {:?}", db_path);
    let store = Arc::new(
        WaveStore::open(&db_path).map_err(|e| format!("failed to open WaveStore: {}", e))?,
    );

    // Open FileStore (separate SQLite DB for file data)
    let filestore_path = data_dir.join("filestore.db");
    tracing::info!("Opening FileStore at {:?}", filestore_path);
    let file_store = Arc::new(
        FileStore::open(&filestore_path)
            .map_err(|e| format!("failed to open FileStore: {}", e))?,
    );

    // Ensure initial data (Client, Window, Workspace, Tab)
    let first_launch =
        wcore::ensure_initial_data(&store).map_err(|e| format!("ensure_initial_data: {}", e))?;
    if first_launch {
        tracing::info!("First launch — created initial client/window/workspace/tab");
    }

    // Get client data for frontend init
    let client =
        wcore::get_client(&store).map_err(|e| format!("failed to get client: {}", e))?;
    let window_id = client
        .windowids
        .first()
        .cloned()
        .ok_or_else(|| "no windows in client".to_string())?;
    let window = store
        .must_get::<crate::backend::waveobj::Window>(&window_id)
        .map_err(|e| format!("failed to get window: {}", e))?;
    let workspace = store
        .must_get::<crate::backend::waveobj::Workspace>(&window.workspaceid)
        .map_err(|e| format!("failed to get workspace: {}", e))?;
    let active_tab_id = workspace.activetabid.clone();

    // Initialize pub/sub broker with Tauri event delivery
    let broker = Arc::new(Broker::new());
    broker.set_client(Box::new(TauriWpsClient::new(handle.clone())));

    // Initialize RPC engine and router
    let (rpc_engine, _rpc_output) = WshRpcEngine::new();
    let router = WshRouter::new();

    // Load config from disk (embedded defaults + user overrides)
    let config_dir = crate::backend::wavebase::get_wave_config_dir();
    if let Err(e) = crate::backend::wavebase::ensure_wave_config_dir() {
        tracing::warn!("Could not ensure config dir: {}", e);
    }
    let full_config = crate::backend::wconfig::load_full_config(&config_dir);
    let n_themes = full_config.term_themes.len();
    let n_widgets = full_config.default_widgets.len() + full_config.widgets.len();
    let n_presets = full_config.presets.len();
    let config_watcher = Arc::new(crate::backend::wconfig::ConfigWatcher::with_config(full_config));
    tracing::info!(
        "Config loaded: {} themes, {} widgets, {} presets",
        n_themes, n_widgets, n_presets,
    );

    // Generate auth key for wsh connections and set it globally
    let auth_key = uuid::Uuid::new_v4().to_string();
    if let Err(e) = crate::backend::authkey::set_auth_key(auth_key.clone()) {
        tracing::warn!("Could not set auth key: {}", e);
    }

    // Start wsh IPC server (local socket for wsh CLI connections)
    let wsh_socket_path = wsh_server::start_wsh_server(
        Arc::clone(&router),
        auth_key.clone(),
        &data_dir,
    )
    .unwrap_or_else(|e| {
        tracing::warn!("Failed to start wsh IPC server: {}", e);
        String::new()
    });

    if !wsh_socket_path.is_empty() {
        tracing::info!("wsh IPC socket: {}", wsh_socket_path);
    }

    // Create and manage AppState
    let app_state = AppState {
        auth_key: std::sync::Mutex::new(auth_key),
        zoom_factor: std::sync::Mutex::new(1.0),
        client_id: std::sync::Mutex::new(Some(client.oid.clone())),
        window_id: std::sync::Mutex::new(Some(window_id.clone())),
        active_tab_id: std::sync::Mutex::new(Some(active_tab_id.clone())),
        window_init_status: std::sync::Mutex::new(String::new()),
        wave_store: store,
        broker,
        rpc_engine,
        router,
        file_store,
        wsh_socket_path: std::sync::Mutex::new(wsh_socket_path),
        config_watcher,
        config_dir,
    };
    app.manage(app_state);

    tracing::info!(
        "Rust backend initialized: client={}, window={}, tab={}",
        &client.oid[..8],
        &window_id[..8],
        &active_tab_id[..8],
    );

    // Emit backend-ready to frontend (matching Go sidecar protocol)
    // In rust-backend mode, there's no WebSocket — frontend uses Tauri IPC
    if let Some(webview_window) = handle.get_webview_window("main") {
        let _ = webview_window.emit(
            "backend-ready",
            serde_json::json!({
                "ws": "",
                "web": "",
                "rustBackend": true,
                "clientId": client.oid,
                "windowId": window_id,
                "tabId": active_tab_id,
            }),
        );
    }

    Ok(())
}
