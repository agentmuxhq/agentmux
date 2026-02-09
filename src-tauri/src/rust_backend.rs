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
use crate::backend::storage::wstore::WaveStore;
use crate::backend::wcore;
use crate::backend::wps::{Broker, WaveEvent, WpsClient};
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

    // Create and manage AppState
    let app_state = AppState {
        auth_key: std::sync::Mutex::new(uuid::Uuid::new_v4().to_string()),
        zoom_factor: std::sync::Mutex::new(1.0),
        client_id: std::sync::Mutex::new(Some(client.oid.clone())),
        window_id: std::sync::Mutex::new(Some(window_id.clone())),
        active_tab_id: std::sync::Mutex::new(Some(active_tab_id.clone())),
        window_init_status: std::sync::Mutex::new(String::new()),
        wave_store: store,
        broker,
        rpc_engine,
        router,
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
