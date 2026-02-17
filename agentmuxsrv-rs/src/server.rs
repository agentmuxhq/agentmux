use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        Path as AxumPath, Query, Request, State, WebSocketUpgrade,
    },
    http::{header, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde_json::json;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};

use crate::backend::eventbus::EventBus;
use crate::backend::reactive::{self, InjectionRequest, Poller, PollerConfig, ReactiveHandler};
use crate::backend::service::{self, WebCallType, WebReturnType};
use crate::backend::storage::filestore::FileStore;
use crate::backend::storage::wstore::WaveStore;
use crate::backend::waveobj::*;
use crate::backend::wps::Broker;
use crate::backend::{ai, docsite, schema, wcore};

// ---- AppState ----

#[derive(Clone)]
pub struct AppState {
    pub auth_key: String,
    pub version: String,
    pub app_path: String,
    pub wstore: Arc<WaveStore>,
    pub filestore: Arc<FileStore>,
    pub event_bus: Arc<EventBus>,
    pub broker: Arc<Broker>,
    pub reactive_handler: &'static ReactiveHandler,
    pub poller: Arc<Poller>,
}

/// Build the Axum router with all routes, auth middleware, and CORS.
pub fn build_router(state: AppState) -> Router {
    // CORS: allow all origins, methods, headers (matching Go pkg/web/web.go:536-573)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(vec![
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            "X-Session-Id".parse().unwrap(),
            "X-AuthKey".parse().unwrap(),
            "X-Requested-With".parse().unwrap(),
            "x-vercel-ai-ui-message-stream".parse().unwrap(),
        ]);

    // No-auth routes (matching Go SkipAuth: true — localhost-only reactive endpoints)
    let reactive_routes = Router::new()
        .route("/wave/reactive/inject", post(handle_reactive_inject))
        .route("/wave/reactive/agents", get(handle_reactive_agents))
        .route("/wave/reactive/agent", get(handle_reactive_agent))
        .route("/wave/reactive/audit", get(handle_reactive_audit))
        .route("/wave/reactive/register", post(handle_reactive_register))
        .route(
            "/wave/reactive/unregister",
            post(handle_reactive_unregister),
        )
        .route(
            "/wave/reactive/poller/stats",
            get(handle_reactive_poller_stats),
        )
        .route(
            "/wave/reactive/poller/config",
            post(handle_reactive_poller_config),
        )
        .route(
            "/wave/reactive/poller/status",
            get(handle_reactive_poller_status),
        );

    // Authed routes
    let vdom_router = Router::new()
        .route("/{uuid}", get(stub_501))
        .route("/{uuid}/*rest", get(stub_501));

    let authed_routes = Router::new()
        .route("/ws", get(handle_ws))
        .route("/wave/service", post(handle_service))
        .route("/wave/file", get(handle_wave_file))
        .route("/wave/stream-file", get(stub_501))
        .route("/wave/stream-file/*path", get(stub_501))
        .route("/wave/stream-local-file", get(stub_501))
        .route("/wave/aichat", post(handle_ai_chat))
        .nest("/vdom", vdom_router)
        .route("/api/post-chat-message", get(stub_501).post(stub_501))
        .route("/docsite/*path", get(handle_docsite))
        .route("/schema/*path", get(handle_schema))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // Health endpoint (no auth)
    let health = Router::new().route("/", get(health_handler));

    Router::new()
        .merge(health)
        .merge(reactive_routes)
        .merge(authed_routes)
        .layer(cors)
        .with_state(state)
}

// ---- Health ----

async fn health_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "version": state.version,
    }))
}

async fn stub_501() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({"error": "not implemented"})),
    )
}

// ---- Auth Middleware ----

/// Auth middleware matching Go pkg/authkey/authkey.go:18-42.
async fn auth_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    if req.method() == Method::OPTIONS {
        return next.run(req).await;
    }

    let auth_key = req
        .headers()
        .get("X-AuthKey")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let auth_key = auth_key.or_else(|| {
        req.uri().query().and_then(|q| {
            q.split('&')
                .filter_map(|pair| pair.split_once('='))
                .find(|(k, _)| *k == "authkey")
                .map(|(_, v)| v.to_string())
        })
    });

    match auth_key {
        Some(key) if key == state.auth_key => next.run(req).await,
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "unauthorized"})),
        )
            .into_response(),
    }
}

// ====================================================================
// Service dispatch (POST /wave/service)
// ====================================================================

async fn handle_service(
    State(state): State<AppState>,
    Json(call): Json<WebCallType>,
) -> Json<WebReturnType> {
    let result = dispatch_service(&state, &call);
    Json(result)
}

fn dispatch_service(state: &AppState, call: &WebCallType) -> WebReturnType {
    let store = &state.wstore;
    let args = &call.args;

    match (call.service.as_str(), call.method.as_str()) {
        // ---- ObjectService ----
        ("object", "GetObject") => {
            let oref_str: String = match service::get_arg(args, 0) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match get_object_by_oref(store, &oref_str) {
                Ok(data) => WebReturnType::success(data),
                Err(e) => WebReturnType::error(e),
            }
        }
        ("object", "GetObjects") => {
            let orefs: Vec<String> = match service::get_arg(args, 0) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            let mut results = Vec::new();
            for oref_str in &orefs {
                match get_object_by_oref(store, oref_str) {
                    Ok(data) => results.push(data),
                    Err(_) => results.push(serde_json::Value::Null),
                }
            }
            WebReturnType::success(serde_json::json!(results))
        }
        ("object", "CreateBlock") => {
            let tab_id = match call
                .uicontext
                .as_ref()
                .map(|ctx| ctx.active_tab_id.clone())
            {
                Some(id) if !id.is_empty() => id,
                _ => return WebReturnType::error("missing uicontext.activetabid"),
            };
            let block_def: BlockDef = match service::get_arg(args, 1) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match wcore::create_block(store, &tab_id, block_def.meta) {
                Ok(block) => {
                    let data = serde_json::to_value(&block).unwrap_or_default();
                    WebReturnType::success(data)
                }
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("object", "DeleteBlock") => {
            let tab_id = match call
                .uicontext
                .as_ref()
                .map(|ctx| ctx.active_tab_id.clone())
            {
                Some(id) if !id.is_empty() => id,
                _ => return WebReturnType::error("missing uicontext.activetabid"),
            };
            let block_id: String = match service::get_arg(args, 1) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match wcore::delete_block(store, &tab_id, &block_id) {
                Ok(()) => WebReturnType::success_empty(),
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("object", "UpdateObjectMeta") => {
            let oref_str: String = match service::get_arg(args, 1) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            let meta_update: MetaMapType = match service::get_arg(args, 2) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match update_object_meta(store, &oref_str, &meta_update) {
                Ok(()) => WebReturnType::success_empty(),
                Err(e) => WebReturnType::error(e),
            }
        }
        ("object", "UpdateTabName") => {
            let tab_id: String = match service::get_arg(args, 1) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            let name: String = match service::get_arg(args, 2) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match store.must_get::<Tab>(&tab_id) {
                Ok(mut tab) => {
                    tab.name = name;
                    match store.update(&mut tab) {
                        Ok(_) => WebReturnType::success_empty(),
                        Err(e) => WebReturnType::error(e.to_string()),
                    }
                }
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }

        // ---- ClientService ----
        ("client", "GetClientData") => match wcore::get_client(store) {
            Ok(client) => {
                WebReturnType::success(serde_json::to_value(&client).unwrap_or_default())
            }
            Err(e) => WebReturnType::error(e.to_string()),
        },
        ("client", "GetTab") => {
            let tab_id: String = match service::get_arg(args, 0) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match store.must_get::<Tab>(&tab_id) {
                Ok(tab) => WebReturnType::success(serde_json::to_value(&tab).unwrap_or_default()),
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("client", "FocusWindow") => {
            let window_id: String = match service::get_arg(args, 0) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match wcore::focus_window(store, &window_id) {
                Ok(()) => WebReturnType::success_empty(),
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("client", "AgreeTos") => match wcore::get_client(store) {
            Ok(mut client) => {
                client.tosagreed = chrono::Utc::now().timestamp_millis();
                match store.update(&mut client) {
                    Ok(_) => WebReturnType::success_empty(),
                    Err(e) => WebReturnType::error(e.to_string()),
                }
            }
            Err(e) => WebReturnType::error(e.to_string()),
        },
        ("client", "GetAllConnStatus") => {
            // Return empty — connection manager not yet wired
            // Go returns success with no data (nil slice omitted by omitempty)
            WebReturnType::success_empty()
        }
        ("client", "TelemetryUpdate") => {
            // Accept but ignore — telemetry not implemented
            WebReturnType::success_empty()
        }

        // ---- WindowService ----
        ("window", "GetWindow") => {
            let window_id: String = match service::get_arg(args, 0) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match store.must_get::<Window>(&window_id) {
                Ok(win) => WebReturnType::success(serde_json::to_value(&win).unwrap_or_default()),
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("window", "CreateWindow") => {
            let ws_id: String = match service::get_arg(args, 2) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match wcore::create_window(store, &ws_id) {
                Ok(win) => {
                    // Add to client window list
                    if let Ok(mut client) = wcore::get_client(store) {
                        client.windowids.push(win.oid.clone());
                        let _ = store.update(&mut client);
                    }
                    WebReturnType::success(serde_json::to_value(&win).unwrap_or_default())
                }
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("window", "CloseWindow") => {
            let window_id: String = match service::get_arg(args, 1) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match wcore::close_window(store, &window_id) {
                Ok(()) => WebReturnType::success_empty(),
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("window", "SwitchWorkspace") => {
            let window_id: String = match service::get_arg(args, 1) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            let ws_id: String = match service::get_arg(args, 2) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match wcore::switch_workspace(store, &window_id, &ws_id) {
                Ok(()) => WebReturnType::success_empty(),
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("window", "SetWindowPosAndSize") => {
            let window_id: String = match service::get_arg(args, 1) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            let pos: Option<Point> = service::get_optional_arg(args, 2).unwrap_or(None);
            let size: Option<WinSize> = service::get_optional_arg(args, 3).unwrap_or(None);
            match store.must_get::<Window>(&window_id) {
                Ok(mut win) => {
                    if let Some(p) = pos {
                        win.pos = p;
                    }
                    if let Some(s) = size {
                        win.winsize = s;
                    }
                    match store.update(&mut win) {
                        Ok(_) => WebReturnType::success_empty(),
                        Err(e) => WebReturnType::error(e.to_string()),
                    }
                }
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }

        // ---- WorkspaceService ----
        ("workspace", "CreateWorkspace") => {
            let name: String = service::get_arg(args, 1).unwrap_or_default();
            let icon: String = service::get_arg(args, 2).unwrap_or_default();
            let color: String = service::get_arg(args, 3).unwrap_or_default();
            match wcore::create_workspace(store, &name, &icon, &color) {
                Ok(ws) => WebReturnType::success(serde_json::to_value(&ws).unwrap_or_default()),
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("workspace", "GetWorkspace") => {
            let ws_id: String = match service::get_arg(args, 0) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match wcore::get_workspace(store, &ws_id) {
                Ok(ws) => WebReturnType::success(serde_json::to_value(&ws).unwrap_or_default()),
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("workspace", "DeleteWorkspace") => {
            let ws_id: String = match service::get_arg(args, 0) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match wcore::delete_workspace(store, &ws_id) {
                Ok(()) => WebReturnType::success_empty(),
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("workspace", "ListWorkspaces") => match wcore::list_workspaces(store) {
            Ok(list) => WebReturnType::success(serde_json::to_value(&list).unwrap_or_default()),
            Err(e) => WebReturnType::error(e.to_string()),
        },
        ("workspace", "CreateTab") => {
            let ws_id: String = match service::get_arg(args, 0) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match wcore::create_tab(store, &ws_id) {
                Ok(tab) => WebReturnType::success(serde_json::to_value(&tab).unwrap_or_default()),
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("workspace", "SetActiveTab") => {
            let ws_id: String = match service::get_arg(args, 0) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            let tab_id: String = match service::get_arg(args, 1) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match wcore::set_active_tab(store, &ws_id, &tab_id) {
                Ok(()) => WebReturnType::success_empty(),
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("workspace", "CloseTab") => {
            let ws_id: String = match service::get_arg(args, 1) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            let tab_id: String = match service::get_arg(args, 2) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match wcore::delete_tab(store, &ws_id, &tab_id) {
                Ok(()) => WebReturnType::success(
                    serde_json::to_value(&service::CloseTabRtnType {
                        closewindow: false,
                        newactivetabid: String::new(),
                    })
                    .unwrap_or_default(),
                ),
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("workspace", "GetColors") => {
            WebReturnType::success(json!(wcore::WORKSPACE_COLORS))
        }
        ("workspace", "GetIcons") => {
            WebReturnType::success(json!(wcore::WORKSPACE_ICONS))
        }
        ("workspace", "UpdateWorkspace") => {
            let ws_id: String = match service::get_arg(args, 1) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            let name: Option<String> = service::get_optional_arg(args, 2).unwrap_or(None);
            let icon: Option<String> = service::get_optional_arg(args, 3).unwrap_or(None);
            let color: Option<String> = service::get_optional_arg(args, 4).unwrap_or(None);
            match store.must_get::<Workspace>(&ws_id) {
                Ok(mut ws) => {
                    if let Some(n) = name {
                        ws.name = n;
                    }
                    if let Some(i) = icon {
                        ws.icon = i;
                    }
                    if let Some(c) = color {
                        ws.color = c;
                    }
                    match store.update(&mut ws) {
                        Ok(_) => WebReturnType::success_empty(),
                        Err(e) => WebReturnType::error(e.to_string()),
                    }
                }
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("workspace", "UpdateTabIds") => {
            let ws_id: String = match service::get_arg(args, 1) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            let tab_ids: Vec<String> = match service::get_arg(args, 2) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            let pinned_tab_ids: Vec<String> = service::get_arg(args, 3).unwrap_or_default();
            match store.must_get::<Workspace>(&ws_id) {
                Ok(mut ws) => {
                    ws.tabids = tab_ids;
                    ws.pinnedtabids = pinned_tab_ids;
                    match store.update(&mut ws) {
                        Ok(_) => WebReturnType::success_empty(),
                        Err(e) => WebReturnType::error(e.to_string()),
                    }
                }
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }
        ("workspace", "ChangeTabPinning") => {
            let ws_id: String = match service::get_arg(args, 1) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            let tab_id: String = match service::get_arg(args, 2) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            let pinned: bool = match service::get_arg(args, 3) {
                Ok(v) => v,
                Err(e) => return WebReturnType::error(e),
            };
            match store.must_get::<Workspace>(&ws_id) {
                Ok(mut ws) => {
                    ws.pinnedtabids.retain(|id| id != &tab_id);
                    if pinned {
                        ws.pinnedtabids.push(tab_id);
                    }
                    match store.update(&mut ws) {
                        Ok(_) => WebReturnType::success_empty(),
                        Err(e) => WebReturnType::error(e.to_string()),
                    }
                }
                Err(e) => WebReturnType::error(e.to_string()),
            }
        }

        // ---- UserInputService ----
        ("userinput", "SendUserInputResponse") => {
            // Accept but drop — user input routing not yet wired
            WebReturnType::success_empty()
        }

        // ---- BlockService ----
        ("block", "SendCommand") | ("block", "GetControllerStatus") | ("block", "SaveTerminalState") => {
            // Block controller not yet wired
            WebReturnType::error("block service not yet implemented")
        }

        _ => WebReturnType::error(format!(
            "unknown service method: {}.{}",
            call.service, call.method
        )),
    }
}

/// Resolve an "otype:oid" string to the corresponding wave object JSON.
fn get_object_by_oref(store: &WaveStore, oref_str: &str) -> Result<serde_json::Value, String> {
    let oref = crate::backend::ORef::parse(oref_str).map_err(|e| e.to_string())?;
    // Use wave_obj_to_value to include "otype" field, matching Go's ToJsonMap behavior
    match oref.otype.as_str() {
        OTYPE_CLIENT => store
            .must_get::<Client>(&oref.oid)
            .map(|o| wave_obj_to_value(&o))
            .map_err(|e| e.to_string()),
        OTYPE_WINDOW => store
            .must_get::<Window>(&oref.oid)
            .map(|o| wave_obj_to_value(&o))
            .map_err(|e| e.to_string()),
        OTYPE_WORKSPACE => store
            .must_get::<Workspace>(&oref.oid)
            .map(|o| wave_obj_to_value(&o))
            .map_err(|e| e.to_string()),
        OTYPE_TAB => store
            .must_get::<Tab>(&oref.oid)
            .map(|o| wave_obj_to_value(&o))
            .map_err(|e| e.to_string()),
        OTYPE_LAYOUT => store
            .must_get::<LayoutState>(&oref.oid)
            .map(|o| wave_obj_to_value(&o))
            .map_err(|e| e.to_string()),
        OTYPE_BLOCK => store
            .must_get::<Block>(&oref.oid)
            .map(|o| wave_obj_to_value(&o))
            .map_err(|e| e.to_string()),
        _ => Err(format!("unknown otype: {}", oref.otype)),
    }
}

/// Update object meta by oref string. Merges meta into existing object.
fn update_object_meta(
    store: &WaveStore,
    oref_str: &str,
    meta_update: &MetaMapType,
) -> Result<(), String> {
    let oref = crate::backend::ORef::parse(oref_str).map_err(|e| e.to_string())?;
    match oref.otype.as_str() {
        OTYPE_CLIENT => {
            let mut obj = store.must_get::<Client>(&oref.oid).map_err(|e| e.to_string())?;
            obj.meta = merge_meta(&obj.meta, meta_update, true);
            store.update(&mut obj).map_err(|e| e.to_string())?;
        }
        OTYPE_WINDOW => {
            let mut obj = store.must_get::<Window>(&oref.oid).map_err(|e| e.to_string())?;
            obj.meta = merge_meta(&obj.meta, meta_update, true);
            store.update(&mut obj).map_err(|e| e.to_string())?;
        }
        OTYPE_WORKSPACE => {
            let mut obj = store
                .must_get::<Workspace>(&oref.oid)
                .map_err(|e| e.to_string())?;
            obj.meta = merge_meta(&obj.meta, meta_update, true);
            store.update(&mut obj).map_err(|e| e.to_string())?;
        }
        OTYPE_TAB => {
            let mut obj = store.must_get::<Tab>(&oref.oid).map_err(|e| e.to_string())?;
            obj.meta = merge_meta(&obj.meta, meta_update, true);
            store.update(&mut obj).map_err(|e| e.to_string())?;
        }
        OTYPE_BLOCK => {
            let mut obj = store.must_get::<Block>(&oref.oid).map_err(|e| e.to_string())?;
            obj.meta = merge_meta(&obj.meta, meta_update, true);
            store.update(&mut obj).map_err(|e| e.to_string())?;
        }
        _ => return Err(format!("cannot update meta for otype: {}", oref.otype)),
    }
    Ok(())
}

// ====================================================================
// File endpoint (GET /wave/file)
// ====================================================================

#[derive(serde::Deserialize)]
struct FileQueryParams {
    zoneid: Option<String>,
    name: Option<String>,
    #[serde(default)]
    offset: i64,
}

async fn handle_wave_file(
    State(state): State<AppState>,
    Query(params): Query<FileQueryParams>,
) -> Response {
    let zone_id = match &params.zoneid {
        Some(z) if !z.is_empty() => z.as_str(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "missing zoneid"})),
            )
                .into_response()
        }
    };
    let name = match &params.name {
        Some(n) if !n.is_empty() => n.as_str(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "missing name"})),
            )
                .into_response()
        }
    };

    // Get file metadata
    let file_info = match state.filestore.stat(zone_id, name) {
        Ok(Some(info)) => info,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "file not found"})),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    // Read file data
    let (_, data) = match state.filestore.read_at(zone_id, name, params.offset, 0) {
        Ok(result) => result,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    // Build X-ZoneFileInfo header (base64-encoded JSON metadata)
    let file_info_json = serde_json::to_string(&file_info).unwrap_or_default();
    let file_info_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &file_info_json);

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/octet-stream")
        .header("X-ZoneFileInfo", file_info_b64)
        .body(Body::from(data))
        .unwrap_or_else(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to build response",
            )
                .into_response()
        })
}

// ====================================================================
// Reactive endpoints (no auth)
// ====================================================================

async fn handle_reactive_inject(
    State(state): State<AppState>,
    Json(req): Json<InjectionRequest>,
) -> Json<serde_json::Value> {
    let resp = state.reactive_handler.inject_message(req);
    Json(serde_json::to_value(&resp).unwrap_or_default())
}

async fn handle_reactive_agents(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let agents = state.reactive_handler.list_agents();
    Json(serde_json::to_value(&agents).unwrap_or(json!([])))
}

#[derive(serde::Deserialize)]
struct AgentQuery {
    id: Option<String>,
}

async fn handle_reactive_agent(
    State(state): State<AppState>,
    Query(params): Query<AgentQuery>,
) -> Response {
    let id = match &params.id {
        Some(id) if !id.is_empty() => id.as_str(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "missing id param"})),
            )
                .into_response()
        }
    };
    match state.reactive_handler.get_agent(id) {
        Some(agent) => Json(serde_json::to_value(&agent).unwrap_or_default()).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "agent not found"})),
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
struct AuditQuery {
    #[serde(default = "default_audit_limit")]
    limit: usize,
}
fn default_audit_limit() -> usize {
    100
}

async fn handle_reactive_audit(
    State(state): State<AppState>,
    Query(params): Query<AuditQuery>,
) -> Json<serde_json::Value> {
    let log = state.reactive_handler.get_audit_log(params.limit);
    Json(serde_json::to_value(&log).unwrap_or(json!([])))
}

#[derive(serde::Deserialize)]
struct RegisterRequest {
    agent_id: String,
    block_id: String,
    tab_id: Option<String>,
}

async fn handle_reactive_register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Response {
    match state
        .reactive_handler
        .register_agent(&req.agent_id, &req.block_id, req.tab_id.as_deref())
    {
        Ok(()) => Json(json!({"success": true})).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e})),
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
struct UnregisterRequest {
    agent_id: String,
}

async fn handle_reactive_unregister(
    State(state): State<AppState>,
    Json(req): Json<UnregisterRequest>,
) -> Json<serde_json::Value> {
    state.reactive_handler.unregister_agent(&req.agent_id);
    Json(json!({"success": true}))
}

async fn handle_reactive_poller_stats(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let stats = state.poller.stats();
    Json(serde_json::to_value(&stats).unwrap_or(json!({})))
}

#[derive(serde::Deserialize)]
struct PollerConfigRequest {
    url: Option<String>,
    token: Option<String>,
}

async fn handle_reactive_poller_config(
    State(state): State<AppState>,
    Json(req): Json<PollerConfigRequest>,
) -> Json<serde_json::Value> {
    state.poller.reconfigure(req.url, req.token);
    Json(json!({"success": true}))
}

async fn handle_reactive_poller_status(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let status = state.poller.status();
    Json(serde_json::to_value(&status).unwrap_or(json!({})))
}

// ====================================================================
// WebSocket (GET /ws)
// ====================================================================

async fn handle_ws(
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
                        // Handle incoming messages (subscriptions, etc.) — pass-through for now
                    }
                    Some(Err(_)) => break,
                }
            }
        }
    }

    state.event_bus.unregister_ws(&conn_id);
}

// ====================================================================
// AI Chat (POST /wave/aichat)
// ====================================================================

async fn handle_ai_chat(
    State(_state): State<AppState>,
    Json(req): Json<ai::AIStreamRequest>,
) -> Response {
    let backend = ai::select_backend(&req.opts);
    let (event_tx, mut event_rx) = mpsc::channel::<ai::AIStreamEvent>(64);

    // Spawn the streaming task
    tokio::spawn(async move {
        let _ = backend.stream_completion(req, event_tx).await;
    });

    // Build SSE response body
    let stream = async_stream::stream! {
        while let Some(event) = event_rx.recv().await {
            let json = serde_json::to_string(&event).unwrap_or_default();
            yield Ok::<_, std::convert::Infallible>(format!("data: {}\n\n", json));
        }
    };

    let body = Body::from_stream(stream);
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body)
        .unwrap_or_else(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to build SSE response",
            )
                .into_response()
        })
}

// ====================================================================
// Schema (GET /schema/*path)
// ====================================================================

async fn handle_schema(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    let app_path = if state.app_path.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "app path not configured"})),
        )
            .into_response();
    } else {
        PathBuf::from(&state.app_path)
    };

    let schema_dir = schema::get_schema_dir(&app_path);
    let name = match schema::normalize_schema_request(&path) {
        Some(n) => n,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid schema path"})),
            )
                .into_response()
        }
    };

    match schema::resolve_schema_path(&schema_dir, &name) {
        Some(file_path) => match std::fs::read(&file_path) {
            Ok(data) => Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", schema::SCHEMA_CONTENT_TYPE)
                .body(Body::from(data))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ====================================================================
// Docsite (GET /docsite/*path)
// ====================================================================

async fn handle_docsite(AxumPath(path): AxumPath<String>) -> Response {
    match docsite::resolve_docsite_path(&path) {
        Some(file_path) => {
            let content_type = mime_from_path(&file_path);
            match std::fs::read(&file_path) {
                Ok(data) => Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", content_type)
                    .body(Body::from(data))
                    .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn mime_from_path(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        _ => "application/octet-stream",
    }
}

// ====================================================================
// Tests
// ====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn test_state() -> AppState {
        let wstore = Arc::new(WaveStore::open_in_memory().unwrap());
        let filestore = Arc::new(FileStore::open_in_memory().unwrap());
        let event_bus = Arc::new(EventBus::new());
        let broker = Arc::new(Broker::new());
        let reactive_handler = reactive::get_global_handler();
        let poller = Arc::new(Poller::new(
            PollerConfig {
                agentmux_url: None,
                agentmux_token: None,
                poll_interval_secs: 30,
            },
            reactive_handler,
        ));

        // Bootstrap initial data
        wcore::ensure_initial_data(&wstore).unwrap();

        AppState {
            auth_key: "test-secret-key".to_string(),
            version: "0.28.20".to_string(),
            app_path: String::new(),
            wstore,
            filestore,
            event_bus,
            broker,
            reactive_handler,
            poller,
        }
    }

    fn test_router() -> Router {
        build_router(test_state())
    }

    #[tokio::test]
    async fn health_returns_200() {
        let app = test_router();
        let req = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["version"], "0.28.20");
    }

    #[tokio::test]
    async fn auth_rejects_bad_key() {
        let app = test_router();
        let req = Request::builder()
            .uri("/wave/service")
            .method("POST")
            .header("X-AuthKey", "wrong-key")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"service":"client","method":"GetClientData"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn auth_rejects_missing_key() {
        let app = test_router();
        let req = Request::builder()
            .uri("/wave/service")
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"service":"client","method":"GetClientData"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn auth_accepts_valid_header() {
        let app = test_router();
        let req = Request::builder()
            .uri("/wave/service")
            .method("POST")
            .header("X-AuthKey", "test-secret-key")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"service":"client","method":"GetClientData"}"#,
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn auth_accepts_query_param() {
        let app = test_router();
        let req = Request::builder()
            .uri("/wave/service?authkey=test-secret-key")
            .method("POST")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"service":"client","method":"GetClientData"}"#,
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn reactive_routes_skip_auth() {
        let app = test_router();
        let req = Request::builder()
            .uri("/wave/reactive/agents")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_ne!(resp.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn cors_headers_present() {
        let app = test_router();
        let req = Request::builder()
            .method(Method::OPTIONS)
            .uri("/")
            .header("Origin", "http://localhost:5173")
            .header("Access-Control-Request-Method", "GET")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert!(resp
            .headers()
            .contains_key("access-control-allow-origin"));
    }

    #[tokio::test]
    async fn service_get_client_data() {
        let app = test_router();
        let req = Request::builder()
            .uri("/wave/service")
            .method("POST")
            .header("X-AuthKey", "test-secret-key")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"service":"client","method":"GetClientData"}"#,
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert!(json["data"]["oid"].is_string());
        assert!(json["data"]["windowids"].is_array());
    }

    #[tokio::test]
    async fn service_list_workspaces() {
        let app = test_router();
        let req = Request::builder()
            .uri("/wave/service")
            .method("POST")
            .header("X-AuthKey", "test-secret-key")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"service":"workspace","method":"ListWorkspaces"}"#,
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert!(json["data"].is_array());
    }

    #[tokio::test]
    async fn service_unknown_method_returns_error() {
        let app = test_router();
        let req = Request::builder()
            .uri("/wave/service")
            .method("POST")
            .header("X-AuthKey", "test-secret-key")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"service":"foo","method":"Bar"}"#,
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // success=false is skipped by serde (skip_serializing_if), so it's null
        assert!(!json["success"].as_bool().unwrap_or(false));
        assert!(json["error"].as_str().unwrap().contains("unknown"));
    }

    #[tokio::test]
    async fn reactive_agents_returns_empty_list() {
        let app = test_router();
        let req = Request::builder()
            .uri("/wave/reactive/agents")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.is_array());
    }

    #[tokio::test]
    async fn reactive_poller_status() {
        let app = test_router();
        let req = Request::builder()
            .uri("/wave/reactive/poller/status")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.is_object());
    }

    #[tokio::test]
    async fn workspace_colors_and_icons() {
        let state = test_state();
        let app = build_router(state);
        let req = Request::builder()
            .uri("/wave/service")
            .method("POST")
            .header("X-AuthKey", "test-secret-key")
            .header("Content-Type", "application/json")
            .body(Body::from(
                r#"{"service":"workspace","method":"GetColors"}"#,
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["success"].as_bool().unwrap());
        assert!(json["data"].is_array());
        assert!(json["data"].as_array().unwrap().len() > 0);
    }
}
