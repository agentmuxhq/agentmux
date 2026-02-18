use axum::{extract::State, response::Json};
use serde_json::json;

use crate::backend::service::{self, CloseTabRtnType, WebCallType, WebReturnType};
use crate::backend::storage::wstore::WaveStore;
use crate::backend::waveobj::*;
use crate::backend::wcore;

use super::AppState;

pub(super) async fn handle_service(
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> Json<WebReturnType> {
    let call: WebCallType = match serde_json::from_slice(&body) {
        Ok(c) => c,
        Err(e) => return Json(WebReturnType::error(format!("invalid request body: {e}"))),
    };
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
                    serde_json::to_value(&CloseTabRtnType {
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
