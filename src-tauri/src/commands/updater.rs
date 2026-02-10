// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0
//
// Auto-updater commands using tauri-plugin-updater v2.
// Background check runs on startup; install_update is called from frontend.

use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_updater::UpdaterExt;

/// Holds a pending update so install_update() can retrieve it later.
pub struct PendingUpdate(pub std::sync::Mutex<Option<tauri_plugin_updater::Update>>);

/// Background update check spawned from setup().
/// Waits a few seconds for the app to settle, then checks GitHub Releases.
pub async fn check_for_updates_background(app: tauri::AppHandle) {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let _ = app.emit("app-update-status", "checking");

    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Failed to create updater: {}", e);
            let _ = app.emit("app-update-status", "error");
            return;
        }
    };

    match updater.check().await {
        Ok(Some(update)) => {
            tracing::info!("Update available: {}", update.version);
            *app.state::<PendingUpdate>().0.lock().unwrap() = Some(update);
            let _ = app.emit("app-update-status", "ready");
        }
        Ok(None) => {
            tracing::info!("App is up to date");
            let _ = app.emit("app-update-status", "up-to-date");
        }
        Err(e) => {
            tracing::error!("Update check failed: {}", e);
            let _ = app.emit("app-update-status", "error");
        }
    }
}

/// Install a pending app update.
/// Called by the frontend when the user clicks "Install Update" in the banner.
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    let update = app.state::<PendingUpdate>().0.lock().unwrap().take();
    let Some(update) = update else {
        return Err("no pending update".to_string());
    };

    let _ = app.emit("app-update-status", "downloading");

    let app_handle = app.clone();
    if let Err(e) = update
        .download_and_install(
            |_chunk: usize, _total: Option<u64>| {},
            || {
                let _ = app_handle.emit("app-update-status", "installing");
            },
        )
        .await
    {
        tracing::error!("Update install failed: {}", e);
        let _ = app.emit("app-update-status", "error");
        return Err(format!("install failed: {}", e));
    }

    tracing::info!("Update installed, restarting...");
    app.restart();
}
