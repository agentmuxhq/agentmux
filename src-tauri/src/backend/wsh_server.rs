// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

//! Local IPC server for wsh CLI connections.
//!
//! Accepts connections from `wsh` (the shell integration binary) over a local
//! socket (named pipe on Windows, Unix domain socket on macOS/Linux).
//!
//! Protocol:
//! 1. Client connects to socket
//! 2. First message must be `{"command":"authenticate","data":{"token":"..."}}` (JSON line)
//! 3. Server validates token against auth_key
//! 4. On success: registers a route in WshRouter (route_id = "proc:{conn_id}")
//! 5. Bidirectional JSON-line RPC messages flow through WshRouter
//! 6. On disconnect: unregisters route

#[cfg(feature = "rust-backend")]
mod imp {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    use interprocess::local_socket::{
        tokio::prelude::*,
        GenericNamespaced, ListenerOptions,
    };
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    use crate::backend::rpc::router::{
        make_proc_route_id, ChannelRpcClient, WshRouter,
    };

    /// Counter for unique connection IDs.
    static CONN_COUNTER: AtomicU64 = AtomicU64::new(1);

    /// Get the platform-specific socket path/name.
    ///
    /// - Windows: named pipe `\\.\pipe\agentmux-{pid}`
    /// - macOS/Linux: Unix domain socket at `{data_dir}/wave.sock`
    pub fn get_socket_path(data_dir: &std::path::Path) -> String {
        if cfg!(windows) {
            format!("agentmux-{}", std::process::id())
        } else {
            data_dir
                .join("wave.sock")
                .to_string_lossy()
                .to_string()
        }
    }

    /// Start the wsh IPC server. Returns the socket path for env injection.
    ///
    /// Spawns a background task that accepts connections and routes them
    /// through the WshRouter.
    pub fn start_wsh_server(
        router: Arc<WshRouter>,
        auth_key: String,
        data_dir: &std::path::Path,
    ) -> Result<String, String> {
        let socket_name = get_socket_path(data_dir);

        // Compute display path before moving socket_name
        let display_path = if cfg!(windows) {
            format!("\\\\.\\pipe\\{}", socket_name)
        } else {
            socket_name.clone()
        };

        // On Unix, remove stale socket file
        #[cfg(not(windows))]
        {
            let path = std::path::Path::new(&socket_name);
            if path.exists() {
                let _ = std::fs::remove_file(path);
            }
        }

        let name = socket_name
            .to_ns_name::<GenericNamespaced>()
            .map_err(|e| format!("invalid socket name: {}", e))?;

        let listener = ListenerOptions::new()
            .name(name)
            .create_tokio()
            .map_err(|e| format!("failed to bind wsh socket: {}", e))?;

        tracing::info!("wsh IPC server listening on {}", display_path);

        let auth_key = Arc::new(auth_key);

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok(stream) => {
                        let conn_id = CONN_COUNTER.fetch_add(1, Ordering::Relaxed);
                        let router = Arc::clone(&router);
                        let auth_key = Arc::clone(&auth_key);
                        tokio::spawn(async move {
                            if let Err(e) =
                                handle_wsh_connection(stream, conn_id, router, auth_key).await
                            {
                                tracing::debug!("wsh connection {} ended: {}", conn_id, e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("wsh accept error: {}", e);
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
        });

        Ok(display_path)
    }

    /// Handle a single wsh client connection.
    ///
    /// 1. Authenticate (first JSON line must contain valid token)
    /// 2. Register route in WshRouter
    /// 3. Bidirectional message forwarding
    /// 4. Unregister route on disconnect
    async fn handle_wsh_connection(
        stream: impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin + 'static,
        conn_id: u64,
        router: Arc<WshRouter>,
        auth_key: Arc<String>,
    ) -> Result<(), String> {
        let (read_half, mut write_half) = tokio::io::split(stream);
        let mut reader = BufReader::new(read_half);

        // Step 1: Read authentication message (with 10s timeout)
        let mut auth_line = String::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            reader.read_line(&mut auth_line),
        )
        .await
        .map_err(|_| "auth timeout: client did not send auth within 10s".to_string())?
        .map_err(|e| format!("read auth: {}", e))?;

        let auth_msg: serde_json::Value = serde_json::from_str(auth_line.trim())
            .map_err(|e| format!("parse auth: {}", e))?;

        let command = auth_msg
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if command != "authenticate" {
            let err_resp = serde_json::json!({"error": "first message must be authenticate"});
            let _ = write_half
                .write_all(format!("{}\n", err_resp).as_bytes())
                .await;
            return Err("missing authenticate command".to_string());
        }

        let token = auth_msg
            .get("data")
            .and_then(|d| d.get("token"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if token.is_empty() || token != auth_key.as_str() {
            let err_resp = serde_json::json!({"error": "authentication failed"});
            let _ = write_half
                .write_all(format!("{}\n", err_resp).as_bytes())
                .await;
            return Err("auth failed".to_string());
        }

        // Send auth success
        let ok_resp = serde_json::json!({"status": "ok"});
        write_half
            .write_all(format!("{}\n", ok_resp).as_bytes())
            .await
            .map_err(|e| format!("write auth ok: {}", e))?;

        // Step 2: Register route
        let route_id = make_proc_route_id(&format!("wsh-{}", conn_id));
        let (client, mut outbound_rx) = ChannelRpcClient::new();
        router.register_route(&route_id, Box::new(client));

        tracing::info!("wsh connection {} authenticated, route={}", conn_id, route_id);

        // Step 3: Bidirectional message forwarding

        // Task A: Read JSON lines from client → inject into router
        let router_read = Arc::clone(&router);
        let route_id_read = route_id.clone();
        let read_task = tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            router_read
                                .inject_message(trimmed.as_bytes().to_vec(), &route_id_read);
                        }
                    }
                    Err(e) => {
                        tracing::debug!("wsh conn {} read error: {}", conn_id, e);
                        break;
                    }
                }
            }
        });

        // Task B: Read from router outbound channel → write JSON lines to client
        let write_task = tokio::spawn(async move {
            while let Some(msg_bytes) = outbound_rx.recv().await {
                let mut data = msg_bytes;
                if !data.ends_with(b"\n") {
                    data.push(b'\n');
                }
                if write_half.write_all(&data).await.is_err() {
                    break;
                }
                if write_half.flush().await.is_err() {
                    break;
                }
            }
        });

        // Wait for either task to finish (means connection is done)
        tokio::select! {
            _ = read_task => {},
            _ = write_task => {},
        }

        // Step 4: Cleanup
        router.unregister_route(&route_id);
        tracing::info!("wsh connection {} disconnected, route={}", conn_id, route_id);

        Ok(())
    }
}

// Re-export when rust-backend is enabled
#[cfg(feature = "rust-backend")]
pub use imp::*;

// Stub when rust-backend is not enabled
#[cfg(not(feature = "rust-backend"))]
pub fn get_socket_path(_data_dir: &std::path::Path) -> String {
    String::new()
}
