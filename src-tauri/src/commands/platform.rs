use tauri::Manager;

/// Get the current OS platform name.
#[tauri::command]
pub fn get_platform() -> String {
    match std::env::consts::OS {
        "macos" => "darwin".to_string(),
        other => other.to_string(),
    }
}

/// Get the current user's username.
#[tauri::command]
pub fn get_user_name() -> String {
    whoami::username()
}

/// Get the system hostname.
#[tauri::command]
pub fn get_host_name() -> String {
    whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string())
}

/// Check if running in development mode.
#[tauri::command]
pub fn get_is_dev() -> bool {
    cfg!(debug_assertions)
}

/// Get the app data directory path.
#[tauri::command]
pub fn get_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to get data dir: {}", e))
}

/// Get the app config directory path.
#[tauri::command]
pub fn get_config_dir(app: tauri::AppHandle) -> Result<String, String> {
    app.path()
        .app_config_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to get config dir: {}", e))
}

/// Get an environment variable value.
#[tauri::command]
pub fn get_env(key: String) -> Option<String> {
    std::env::var(&key).ok()
}

/// Get details for the About modal.
#[tauri::command]
pub fn get_about_modal_details(app: tauri::AppHandle) -> serde_json::Value {
    let version = app
        .config()
        .version
        .clone()
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    let backend_info = serde_json::json!({ "rustBackend": true });

    serde_json::json!({
        "version": version,
        "buildTime": env!("CARGO_PKG_VERSION"),
        "platform": get_platform(),
        "arch": std::env::consts::ARCH,
        "backendEndpoints": backend_info,
    })
}

/// Get the documentation site URL.
#[tauri::command]
pub fn get_docsite_url(app: tauri::AppHandle) -> String {
    let _ = &app;
    "https://docs.waveterm.dev".to_string()
}
