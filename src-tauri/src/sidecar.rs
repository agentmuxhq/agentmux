use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

/// State returned after successfully spawning the backend.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BackendSpawnResult {
    pub ws_endpoint: String,
    pub web_endpoint: String,
    pub auth_key: String,
}

/// Spawn the agentmuxsrv Go backend as a Tauri sidecar.
///
/// Replaces emain/emain-agentmuxsrv.ts.
///
/// The backend prints a line to stderr when ready:
///   WAVESRV-ESTART ws:<addr> web:<addr> version:<ver> buildtime:<time>
///
/// We parse that line to get the WebSocket and HTTP endpoints,
/// then the frontend connects to them directly.
pub async fn spawn_backend(app: &tauri::AppHandle) -> Result<BackendSpawnResult, String> {
    tracing::info!("🚀 spawn_backend() called");

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get data dir: {}", e))?;

    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to get config dir: {}", e))?;

    tracing::info!("Using config_dir: {}", config_dir.display());
    tracing::info!("Using data_dir: {}", data_dir.display());

    // Check if backend is already running by looking for endpoints file
    let endpoints_file = config_dir.join("wave-endpoints.json");
    tracing::info!("Checking for existing backend at: {}", endpoints_file.display());

    if endpoints_file.exists() {
        tracing::info!("Found existing endpoints file, attempting to reuse backend");
        if let Ok(contents) = std::fs::read_to_string(&endpoints_file) {
            tracing::info!("Read endpoints file: {}", contents);
            if let Ok(existing) = serde_json::from_str::<BackendSpawnResult>(&contents) {
                // Test if the existing backend is still responsive
                // The web_endpoint is like "127.0.0.1:PORT", need to add http://
                let test_url = if existing.web_endpoint.starts_with("http") {
                    existing.web_endpoint.clone()
                } else {
                    format!("http://{}", existing.web_endpoint)
                };

                tracing::info!("Testing connection to existing backend at: {}", test_url);
                match reqwest::get(&test_url).await {
                    Ok(resp) => {
                        if resp.status().is_success() || resp.status().is_client_error() {
                            // Any HTTP response means backend is alive (even 404 is fine)
                            tracing::info!("Successfully connected to existing backend (status: {})", resp.status());

                            // Reuse the auth key from the existing backend
                            let key_preview = existing.auth_key.chars().take(8).collect::<String>();
                            tracing::info!("Reusing auth key from existing backend: {}...", key_preview);
                            let state = app.state::<crate::state::AppState>();
                            let mut auth_key_guard = state.auth_key.lock().unwrap();
                            *auth_key_guard = existing.auth_key.clone();

                            return Ok(existing);
                        } else {
                            tracing::warn!("Backend returned unexpected status: {}", resp.status());
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to connect to existing backend: {}", e);
                    }
                }
                tracing::warn!("Existing backend not responsive, will spawn new one");
            } else {
                tracing::warn!("Failed to parse endpoints file");
            }
        } else {
            tracing::warn!("Failed to read endpoints file");
        }
        // Endpoints file exists but backend is dead, remove stale file
        tracing::info!("Removing stale endpoints file");
        let _ = std::fs::remove_file(&endpoints_file);
    } else {
        tracing::info!("No existing endpoints file found, will spawn new backend");
    }

    // Ensure directories exist
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create data dir: {}", e))?;
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;

    // Get auth key from app state
    let auth_key = app.state::<crate::state::AppState>().auth_key.lock().unwrap().clone();
    let key_preview = auth_key.chars().take(8).collect::<String>();
    tracing::info!("Spawning agentmuxsrv with auth key: {}", key_preview);

    let shell = app.shell();

    // Try to find agentmuxsrv in portable mode first (same dir as exe)
    let portable_path = std::env::current_exe().ok().and_then(|exe_path| {
        let exe_dir = exe_path.parent()?;
        let portable_binary = exe_dir.join("agentmuxsrv.x64.exe");
        if portable_binary.exists() {
            tracing::info!("Using portable agentmuxsrv at: {:?}", portable_binary);
            Some(portable_binary)
        } else {
            None
        }
    });

    let sidecar_cmd = if let Some(portable_exe) = portable_path {
        // Portable mode: run executable from same directory
        shell.command(portable_exe)
    } else {
        // Installer mode: use bundled sidecar
        shell
            .sidecar("agentmuxsrv")
            .map_err(|e| format!("Failed to find agentmuxsrv sidecar: {}", e))?
    };

    let (mut rx, child) = sidecar_cmd
        .args([
            "--wavedata",
            &data_dir.to_string_lossy(),
        ])
        .env("WAVETERM_AUTH_KEY", &auth_key)
        .env("WAVETERM_CONFIG_HOME", config_dir.to_string_lossy().to_string())
        .env("WAVETERM_DATA_HOME", data_dir.to_string_lossy().to_string())
        .env("WAVETERM_DEV", if cfg!(debug_assertions) { "1" } else { "" })
        .env("WCLOUD_ENDPOINT", "https://api.waveterm.dev/central")
        .env("WCLOUD_WS_ENDPOINT", "wss://wsapi.waveterm.dev/")
        .spawn()
        .map_err(|e| format!("Failed to spawn agentmuxsrv: {}", e))?;

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
                                .find_map(|p| p.strip_prefix("ws:"))
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            let web = parts
                                .iter()
                                .find_map(|p| p.strip_prefix("web:"))
                                .map(|s| s.to_string())
                                .unwrap_or_default();

                            tracing::info!("Backend started: ws={}, web={}", ws, web);
                            let _ = tx.send((ws, web)).await;
                        } else if let Some(event_data) = l.strip_prefix("WAVESRV-EVENT:") {
                            handle_backend_event(&app_handle, event_data);
                        } else {
                            tracing::info!("[agentmuxsrv] {}", l);
                        }
                    }
                }
                CommandEvent::Stdout(line) => {
                    let line = String::from_utf8_lossy(&line);
                    tracing::info!("[agentmuxsrv stdout] {}", line.trim());
                }
                CommandEvent::Error(err) => {
                    tracing::error!("[agentmuxsrv error] {}", err);
                }
                CommandEvent::Terminated(status) => {
                    tracing::warn!("[agentmuxsrv] terminated with status: {:?}", status);
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
    .map_err(|_| "Timeout waiting for agentmuxsrv to start (30s)".to_string())?
    .ok_or_else(|| "agentmuxsrv channel closed before sending endpoints".to_string())?;

    let result = BackendSpawnResult {
        ws_endpoint: timeout.0,
        web_endpoint: timeout.1,
        auth_key: auth_key.clone(),
    };

    let key_preview = result.auth_key.chars().take(8).collect::<String>();
    tracing::info!("Backend successfully started with endpoints: ws={}, web={}, auth_key={}...", result.ws_endpoint, result.web_endpoint, key_preview);

    // Save endpoints for other instances to reuse
    let endpoints_file = config_dir.join("wave-endpoints.json");
    tracing::info!("Attempting to save endpoints to: {}", endpoints_file.display());

    match serde_json::to_string_pretty(&result) {
        Ok(json) => {
            tracing::info!("Serialized endpoints: {}", json);
            match std::fs::write(&endpoints_file, &json) {
                Ok(_) => {
                    tracing::info!("✅ Successfully saved endpoints to {} for multi-window reuse", endpoints_file.display());
                    // Verify the file was written
                    if endpoints_file.exists() {
                        if let Ok(contents) = std::fs::read_to_string(&endpoints_file) {
                            tracing::info!("Verified file contents: {}", contents);
                        }
                    } else {
                        tracing::error!("❌ File doesn't exist after write!");
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Failed to write endpoints file: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to serialize endpoints: {}", e);
        }
    }

    Ok(result)
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
