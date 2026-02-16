use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthStatus {
    pub connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}

/// Open system browser to Claude Code OAuth authorization page.
///
/// This initiates the OAuth flow:
/// 1. Opens browser to https://claude.ai/code/auth?redirect_uri=agentmux://auth
/// 2. User logs in via browser
/// 3. Browser redirects to agentmux://auth?code=ABC123
/// 4. Deep link handler (in lib.rs) captures the code
/// 5. Code is exchanged for token via backend RPC
#[tauri::command]
pub async fn open_claude_code_auth(app: AppHandle) -> Result<(), String> {
    tracing::info!("Opening Claude Code OAuth authorization page");

    // TODO: Make this URL configurable or get from backend
    let auth_url = "https://claude.ai/code/auth?redirect_uri=agentmux://auth";

    // Open system browser using Tauri opener plugin
    use tauri_plugin_opener::OpenerExt;
    if let Err(e) = app.opener().open_url(auth_url, None::<&str>) {
        tracing::error!("Failed to open browser for Claude Code auth: {}", e);
        return Err(format!("Failed to open browser: {}", e));
    }

    tracing::info!("Browser opened successfully, waiting for redirect...");

    // Emit event to frontend to show "connecting" state
    app.emit("claude-code-auth-started", ()).ok();

    Ok(())
}

/// Get current Claude Code authentication status.
///
/// Queries the backend for stored auth token and returns connection status.
#[tauri::command]
pub async fn get_claude_code_auth(_state: tauri::State<'_, AppState>) -> Result<AuthStatus, String> {
    tracing::debug!("Checking Claude Code auth status");

    // TODO: Call backend RPC to get auth status
    // For now, return disconnected status
    Ok(AuthStatus {
        connected: false,
        email: None,
        expires_at: None,
    })
}

/// Disconnect from Claude Code (clear stored token).
///
/// Removes the stored auth token from backend storage.
#[tauri::command]
pub async fn disconnect_claude_code(_state: tauri::State<'_, AppState>) -> Result<(), String> {
    tracing::info!("Disconnecting from Claude Code");

    // TODO: Call backend RPC to clear auth token

    Ok(())
}

/// Handle the OAuth redirect callback with authorization code.
///
/// Called by the deep link handler when the browser redirects to
/// agentmux://auth?code=ABC123
///
/// This function:
/// 1. Extracts the authorization code
/// 2. Calls backend RPC to exchange code for token
/// 3. Emits success/failure event to frontend
pub async fn handle_auth_callback(
    app: AppHandle,
    code: String,
) -> Result<(), String> {
    tracing::info!("Handling OAuth callback with code: {}...", &code[..8.min(code.len())]);

    // TODO: Call backend RPC to exchange code for token
    // For now, just emit a mock success event

    // Emit success event to frontend
    let auth_status = AuthStatus {
        connected: true,
        email: Some("user@example.com".to_string()),
        expires_at: Some(chrono::Utc::now().timestamp() + 3600),
    };

    app.emit("claude-code-auth-success", auth_status)
        .map_err(|e| format!("Failed to emit auth success event: {}", e))?;

    tracing::info!("OAuth flow completed successfully");

    Ok(())
}
