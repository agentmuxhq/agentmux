use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};

use super::AppState;

pub(super) async fn handle_ws(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state))
}

async fn handle_ws_connection(mut socket: WebSocket, state: AppState) {
    let conn_id = uuid::Uuid::new_v4().to_string();

    // Extract tab_id from the first message (client sends it after connecting)
    // For now, register with empty tab_id — client will subscribe via messages
    let tab_id = String::new();

    let mut rx = state.event_bus.register_ws(&conn_id, &tab_id);

    // Forward events from the event bus to the WebSocket
    loop {
        tokio::select! {
            Some(event) = rx.recv() => {
                let msg = serde_json::to_string(&event).unwrap_or_default();
                if socket.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    Some(Ok(_)) => {
                        // TODO: Wire incoming messages to rpc::engine::RpcEngine
                        // Currently a stub — all incoming RPC messages are dropped.
                        // This causes GetFullConfigCommand and other WS-based RPCs to hang.
                    }
                    Some(Err(_)) => break,
                }
            }
        }
    }

    state.event_bus.unregister_ws(&conn_id);
}
