use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use serde::Deserialize;
use serde_json::json;

use crate::backend::ai::chatstore::get_default_chat_store;
use crate::backend::rpc::engine::WshRpcEngine;
use crate::backend::rpc_types::{
    CommandSetMetaData, RpcMessage, COMMAND_EVENT_SUB, COMMAND_EVENT_UNSUB,
    COMMAND_EVENT_UNSUB_ALL, COMMAND_GET_FULL_CONFIG, COMMAND_GET_WAVE_AI_CHAT,
    COMMAND_GET_WAVE_AI_RATE_LIMIT, COMMAND_ROUTE_ANNOUNCE, COMMAND_ROUTE_UNANNOUNCE,
    COMMAND_SET_META,
};
use super::service::update_object_meta;

use super::AppState;

/// Incoming WebSocket message envelope.
/// Supports both ping/pong messages and wscommand-based RPC.
#[derive(Deserialize)]
struct WSIncoming {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    #[allow(dead_code)]
    stime: Option<i64>,
    wscommand: Option<String>,
    message: Option<RpcMessage>,
    // Fields for setblocktermsize / blockinput
    blockid: Option<String>,
}

pub(super) async fn handle_ws(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state))
}

async fn handle_ws_connection(mut socket: WebSocket, state: AppState) {
    let conn_id = uuid::Uuid::new_v4().to_string();
    let tab_id = String::new();

    let mut event_rx = state.event_bus.register_ws(&conn_id, &tab_id);

    // Create RPC engine for this connection
    let (engine, mut rpc_output_rx) = WshRpcEngine::new();

    // Register handlers
    register_handlers(&engine, state.clone());

    // Periodic ping interval (10 seconds)
    let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(10));
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            // Forward event bus events → WebSocket
            Some(event) = event_rx.recv() => {
                let msg = serde_json::to_string(&event).unwrap_or_default();
                if socket.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }

            // Forward RPC engine output → WebSocket (wrapped as eventtype:rpc)
            Some(rpc_msg) = rpc_output_rx.recv() => {
                let wrapped = json!({
                    "eventtype": "rpc",
                    "data": rpc_msg,
                });
                let msg = serde_json::to_string(&wrapped).unwrap_or_default();
                if socket.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }

            // Incoming WebSocket messages → parse & dispatch
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Text(text))) => {
                        if let Err(send_err) = handle_incoming_text(&text, &engine, &mut socket).await {
                            if send_err {
                                break;
                            }
                        }
                    }
                    Some(Ok(_)) => {
                        // Binary or other message types — ignore
                    }
                    Some(Err(_)) => break,
                }
            }

            // Periodic ping
            _ = ping_interval.tick() => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                let ping = json!({ "type": "ping", "stime": now });
                let msg = serde_json::to_string(&ping).unwrap_or_default();
                if socket.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        }
    }

    state.event_bus.unregister_ws(&conn_id);
}

/// Handle an incoming text message, returns Err(true) if the socket send failed.
async fn handle_incoming_text(
    text: &str,
    engine: &Arc<WshRpcEngine>,
    socket: &mut WebSocket,
) -> Result<(), bool> {
    let incoming: WSIncoming = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("ws: invalid JSON: {}", e);
            return Ok(());
        }
    };

    // Handle ping/pong by type field
    if let Some(ref msg_type) = incoming.msg_type {
        match msg_type.as_str() {
            "ping" => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                let pong = json!({ "type": "pong", "stime": now });
                let msg = serde_json::to_string(&pong).unwrap_or_default();
                if socket.send(Message::Text(msg.into())).await.is_err() {
                    return Err(true);
                }
                return Ok(());
            }
            "pong" => {
                // Ignore pong responses
                return Ok(());
            }
            _ => {}
        }
    }

    // Handle wscommand-based messages
    if let Some(ref wscommand) = incoming.wscommand {
        match wscommand.as_str() {
            "rpc" => {
                if let Some(rpc_msg) = incoming.message {
                    engine.handle_message(rpc_msg);
                } else {
                    tracing::warn!("ws: rpc wscommand missing message field");
                }
            }
            "setblocktermsize" | "blockinput" => {
                // Convert to controllerinput RPC — stub for now, log and ignore
                tracing::debug!(
                    "ws: {} for block {:?} (stub)",
                    wscommand,
                    incoming.blockid
                );
            }
            other => {
                tracing::warn!("ws: unknown wscommand: {}", other);
            }
        }
    }

    Ok(())
}

fn register_handlers(engine: &Arc<WshRpcEngine>, state: AppState) {
    // getfullconfig → return full config as JSON
    let config_watcher = state.config_watcher.clone();
    engine.register_handler(
        COMMAND_GET_FULL_CONFIG,
        Box::new(move |_data, _ctx| {
            let cw = config_watcher.clone();
            Box::pin(async move {
                let config = cw.get_full_config();
                match serde_json::to_value(config.as_ref()) {
                    Ok(v) => Ok(Some(v)),
                    Err(e) => Err(format!("failed to serialize config: {}", e)),
                }
            })
        }),
    );

    // routeannounce → log + no-op (fire-and-forget, may have no reqid)
    engine.register_handler(
        COMMAND_ROUTE_ANNOUNCE,
        Box::new(|data, _ctx| {
            Box::pin(async move {
                tracing::debug!("routeannounce: {:?}", data);
                Ok(None)
            })
        }),
    );

    // routeunannounce → no-op
    engine.register_handler(
        COMMAND_ROUTE_UNANNOUNCE,
        Box::new(|_data, _ctx| Box::pin(async move { Ok(None) })),
    );

    // eventsub → accept, log, no-op
    engine.register_handler(
        COMMAND_EVENT_SUB,
        Box::new(|data, _ctx| {
            Box::pin(async move {
                tracing::debug!("eventsub: {:?}", data);
                Ok(None)
            })
        }),
    );

    // eventunsub → accept, no-op
    engine.register_handler(
        COMMAND_EVENT_UNSUB,
        Box::new(|_data, _ctx| Box::pin(async move { Ok(None) })),
    );

    // eventunsuball → accept, no-op
    engine.register_handler(
        COMMAND_EVENT_UNSUB_ALL,
        Box::new(|_data, _ctx| Box::pin(async move { Ok(None) })),
    );

    // setmeta → update object metadata in the DB, broadcast update event
    let wstore_sm = state.wstore.clone();
    let event_bus_sm = state.event_bus.clone();
    engine.register_handler(
        COMMAND_SET_META,
        Box::new(move |data, _ctx| {
            let wstore = wstore_sm.clone();
            let event_bus = event_bus_sm.clone();
            Box::pin(async move {
                let cmd: CommandSetMetaData =
                    serde_json::from_value(data).map_err(|e| format!("setmeta: {e}"))?;
                let oref_str = cmd.oref.to_string();
                update_object_meta(&wstore, &oref_str, &cmd.meta)?;
                // Broadcast waveobj:update so all WS clients refresh their atoms
                event_bus.broadcast_event(&crate::backend::eventbus::WSEventType {
                    eventtype: "waveobj:update".to_string(),
                    oref: oref_str,
                    data: None,
                });
                Ok(None)
            })
        }),
    );

    // getwaveaichat → return UIChat for the given chatid (null if not found)
    engine.register_handler(
        COMMAND_GET_WAVE_AI_CHAT,
        Box::new(|data, _ctx| {
            Box::pin(async move {
                let obj: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_value(data).map_err(|e| format!("getwaveaichat: {e}"))?;
                let chat_id = obj
                    .get("chatid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let result = get_default_chat_store().get_as_ui_chat(&chat_id);
                Ok(result) // None → JSON null, Some(v) → JSON object
            })
        }),
    );

    // getwaveairatelimit → AgentMux has no rate limits; return unlimited/unknown
    engine.register_handler(
        COMMAND_GET_WAVE_AI_RATE_LIMIT,
        Box::new(|_data, _ctx| {
            Box::pin(async move {
                Ok(Some(serde_json::json!({
                    "req": 9999,
                    "reqlimit": 9999,
                    "preq": 9999,
                    "preqlimit": 9999,
                    "resetepoch": 0,
                    "unknown": true
                })))
            })
        }),
    );
}
