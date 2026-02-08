use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

/// State returned after successfully spawning the backend.
pub struct BackendSpawnResult {
    pub ws_endpoint: String,
    pub web_endpoint: String,
}

/// Spawn the wavemuxsrv Go backend as a Tauri sidecar.
///
/// Replaces emain/emain-wavemuxsrv.ts.
///
/// The backend prints a line to stderr when ready:
///   WAVESRV-ESTART ws:<addr> web:<addr> version:<ver> buildtime:<time>
///
/// We parse that line to get the WebSocket and HTTP endpoints,
/// then the frontend connects to them directly.
pub async fn spawn_backend(app: &tauri::AppHandle) -> Result<BackendSpawnResult, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get data dir: {}", e))?;

    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get config dir: {}", e))?;

    // Ensure directories exist
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data dir: {}", e))?;
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;

    // Get auth key from app state
    let auth_key = {
        let state = app.state::<crate::state::AppState>();
        state.auth_key.clone()
    };

    let shell = app.shell();

    let sidecar_cmd = shell
        .sidecar("wavemuxsrv")
        .map_err(|e| format!("Failed to find wavemuxsrv sidecar: {}", e))?;

    let (mut rx, child) = sidecar_cmd
        .args([
            "--wavedata",
            &data_dir.to_string_lossy(),
        ])
        .env("WAVETERM_AUTH_KEY", &auth_key)
        .env("WAVETERM_CONFIG_HOME", &config_dir.to_string_lossy().to_string())
        .env("WAVETERM_DATA_HOME", &data_dir.to_string_lossy().to_string())
        .env("WAVETERM_DEV", if cfg!(debug_assertions) { "1" } else { "" })
        .spawn()
        .map_err(|e| format!("Failed to spawn wavemuxsrv: {}", e))?;

    // Store child handle for graceful shutdown
    {
        let state = app.state::<crate::state::AppState>();
        let mut sidecar = state.sidecar_child.lock().unwrap();
        *sidecar = Some(child);
    }

    // Wait for WAVESRV-ESTART line from stderr
    let (tx, mut endpoint_rx) = tokio::sync::mpsc::channel::<(String, String)>(1);
    let app_handle = app.clone();

    tokio::spawn(async move {
        use tauri_plugin_shell::process::CommandEvent;

        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stderr(line) => {
                    let line = String::from_utf8_lossy(&line);
                    for l in line.lines() {
                        if l.starts_with("WAVESRV-ESTART") {
                            let parts: Vec<&str> = l.split_whitespace().collect();
                            let ws = parts
                                .iter()
                                .find(|p| p.starts_with("ws:"))
                                .map(|p| p[3..].to_string())
                                .unwrap_or_default();
                            let web = parts
                                .iter()
                                .find(|p| p.starts_with("web:"))
                                .map(|p| p[4..].to_string())
                                .unwrap_or_default();

                            tracing::info!("Backend started: ws={}, web={}", ws, web);
                            let _ = tx.send((ws, web)).await;
                        } else if l.starts_with("WAVESRV-EVENT:") {
                            let event_data = &l[14..];
                            handle_backend_event(&app_handle, event_data);
                        } else {
                            tracing::debug!("[wavemuxsrv] {}", l);
                        }
                    }
                }
                CommandEvent::Stdout(line) => {
                    let line = String::from_utf8_lossy(&line);
                    tracing::debug!("[wavemuxsrv stdout] {}", line.trim());
                }
                CommandEvent::Error(err) => {
                    tracing::error!("[wavemuxsrv error] {}", err);
                }
                CommandEvent::Terminated(status) => {
                    tracing::warn!("[wavemuxsrv] terminated with status: {:?}", status);
                    // Emit quit event to frontend
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.emit("backend-terminated", serde_json::json!({
                            "code": status.code,
                            "signal": status.signal,
                        }));
                    }
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for endpoints with timeout
    let timeout = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        endpoint_rx.recv(),
    )
    .await
    .map_err(|_| "Timeout waiting for wavemuxsrv to start (30s)".to_string())?
    .ok_or_else(|| "wavemuxsrv channel closed before sending endpoints".to_string())?;

    Ok(BackendSpawnResult {
        ws_endpoint: timeout.0,
        web_endpoint: timeout.1,
    })
}

/// Handle WAVESRV-EVENT messages from the backend.
/// These are forwarded to the frontend via Tauri events.
fn handle_backend_event(app: &tauri::AppHandle, event_data: &str) {
    tracing::debug!("Backend event: {}", event_data);

    // Forward raw event to frontend
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("wavesrv-event", event_data.to_string());
    }
}
