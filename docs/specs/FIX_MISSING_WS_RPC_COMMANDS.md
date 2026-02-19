# Fix: Missing WebSocket RPC Commands

> **Status:** Ready to implement
> **Priority:** High — blocks 3 widgets from functioning
> **Branch:** `agenta/fix-missing-ws-rpc`

---

## Problem

The frontend calls three WS RPC commands that the Rust backend silently drops with `ws: unknown command: <name>`:

| Command               | Broken Widget         | Effect                              |
|------------------------|----------------------|-------------------------------------|
| `setmeta`             | All blocks           | Block metadata never persists        |
| `getwaveaichat`       | AI Chat panel        | AI chat panel shows nothing          |
| `getwaveairatelimit`  | AI Chat panel        | Rate limit UI broken, console errors |

Root cause: `register_handlers()` in `agentmuxsrv-rs/src/server/websocket.rs` only registers 6 commands. These three exist as constants in `rpc_types.rs` but have no handler.

---

## Fix Overview

Three changes, all in `agentmuxsrv-rs/`:

1. **`setmeta`** — Wire to existing `update_object_meta` logic, add event broadcast
2. **`getwaveaichat`** — Wire to existing `ChatStore`, add to `AppState`
3. **`getwaveairatelimit`** — Return static unlimited response (AgentMux has no rate limits)

Plus one structural change: `register_handlers()` must receive `AppState` (not just `config_watcher`) so handlers can access `wstore`, `event_bus`, and `chat_store`.

---

## File-by-File Changes

### 1. `agentmuxsrv-rs/src/backend/ai/chatstore.rs`

Add a global singleton (matches Go's `DefaultChatStore`):

```rust
use once_cell::sync::Lazy;
use std::sync::Arc;

pub static DEFAULT_CHAT_STORE: Lazy<Arc<ChatStore>> = Lazy::new(|| Arc::new(ChatStore::new()));
```

Add `get_as_ui_chat()` helper that converts `AIChat` to the JSON shape the frontend expects:

```rust
/// Returns the chat as a JSON value matching Go's UIChat:
/// { chatid, apitype, model, apiversion, messages: [{id, role, parts}] }
pub fn get_as_ui_chat(&self, chat_id: &str) -> Option<serde_json::Value> {
    let chat = self.get(chat_id)?;
    let messages: Vec<serde_json::Value> = chat.messages.iter().map(|m| {
        serde_json::json!({
            "id": m.messageid,
            "role": m.data.get("role").cloned().unwrap_or(serde_json::Value::Null),
            "parts": m.data.get("content").map(|c| serde_json::json!([{"type":"text","text":c}]))
                        .unwrap_or(serde_json::json!([])),
        })
    }).collect();
    Some(serde_json::json!({
        "chatid": chat.chatid,
        "apitype": chat.apitype,
        "model": chat.model,
        "apiversion": chat.apiversion,
        "messages": messages,
    }))
}
```

---

### 2. `agentmuxsrv-rs/src/server/mod.rs`

Add `chat_store` to `AppState` (if needed to pass into handlers). Alternatively, use the global singleton directly in the handler — the singleton approach is simpler and matches Go.

No `AppState` change needed if using the global singleton.

---

### 3. `agentmuxsrv-rs/src/server/service.rs`

Change `update_object_meta` from `fn` to `pub(crate) fn` so it can be called from `websocket.rs`:

```rust
// Change line 464:
fn update_object_meta(          // before
pub(crate) fn update_object_meta(   // after
```

---

### 4. `agentmuxsrv-rs/src/server/websocket.rs` — Main change

**Change `register_handlers` signature** to accept `AppState`:

```rust
// Before:
fn register_handlers(engine: &Arc<WshRpcEngine>, config_watcher: Arc<crate::backend::wconfig::ConfigWatcher>)

// After:
fn register_handlers(engine: &Arc<WshRpcEngine>, state: AppState)
```

Update the call site in `handle_ws_connection`:

```rust
// Before:
register_handlers(&engine, config_watcher);

// After:
register_handlers(&engine, state.clone());
```

**Add three new handlers** to `register_handlers`:

#### Handler 1: `setmeta`

```rust
use crate::backend::rpc_types::{CommandSetMetaData, COMMAND_SET_META};
use crate::server::service::update_object_meta;

// Clone state pieces for the closure
let wstore_sm = state.wstore.clone();
let event_bus_sm = state.event_bus.clone();

engine.register_handler(
    COMMAND_SET_META,
    Box::new(move |data, _ctx| {
        let wstore = wstore_sm.clone();
        let event_bus = event_bus_sm.clone();
        Box::pin(async move {
            let cmd: CommandSetMetaData = match data {
                Some(v) => serde_json::from_value(v).map_err(|e| e.to_string())?,
                None => return Err("setmeta: missing data".into()),
            };
            let oref_str = cmd.oref.to_string(); // "block:uuid" format
            update_object_meta(&wstore, &oref_str, &cmd.meta)
                .map_err(|e| e)?;
            // Broadcast WaveObj update so frontend reactively re-renders
            event_bus.send_wave_obj_update(&oref_str);
            Ok(None)
        })
    }),
);
```

**Note:** `event_bus.send_wave_obj_update` may not exist yet — see EventBus section below.

#### Handler 2: `getwaveaichat`

```rust
use crate::backend::ai::chatstore::DEFAULT_CHAT_STORE;
use crate::backend::rpc_types::COMMAND_GET_WAVE_AI_CHAT;

engine.register_handler(
    COMMAND_GET_WAVE_AI_CHAT,
    Box::new(|data, _ctx| {
        Box::pin(async move {
            let chat_id: String = match data {
                Some(v) => {
                    let obj: serde_json::Map<String, serde_json::Value> =
                        serde_json::from_value(v).map_err(|e| e.to_string())?;
                    obj.get("chatid")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                }
                None => return Err("getwaveaichat: missing data".into()),
            };
            let result = DEFAULT_CHAT_STORE.get_as_ui_chat(&chat_id);
            Ok(result) // None → JSON null, Some(v) → JSON object
        })
    }),
);
```

#### Handler 3: `getwaveairatelimit`

AgentMux has no rate limits. Return a static unlimited response so the frontend doesn't error:

```rust
use crate::backend::rpc_types::COMMAND_GET_WAVE_AI_RATE_LIMIT;

engine.register_handler(
    COMMAND_GET_WAVE_AI_RATE_LIMIT,
    Box::new(|_data, _ctx| {
        Box::pin(async move {
            // AgentMux has no AI rate limits — return unlimited
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
```

---

### 5. `agentmuxsrv-rs/src/backend/eventbus.rs` — Check broadcast

After `setmeta` updates the DB, the frontend must be notified via a `waveobj:update` event so Jotai atoms refresh. Check `EventBus` for an existing method that sends this event type.

**Expected event shape** (matching Go's `wcore.SendWaveObjUpdate`):
```json
{
    "eventtype": "waveobj:update",
    "scopes": ["block:uuid"],
    "data": { /* the updated object as JSON */ }
}
```

If `EventBus` doesn't have `send_wave_obj_update(oref_str)`, add it:

```rust
pub fn send_wave_obj_update(&self, oref_str: &str) {
    // Load fresh object from wstore and broadcast
    // Exact impl depends on EventBus/WPS broker internals
}
```

**Fallback:** If wiring the event is complex, the frontend will still work on next page load since the DB is updated. Ship without broadcast first and add it in a follow-up.

---

## Import additions in `websocket.rs`

```rust
use crate::backend::rpc_types::{
    CommandSetMetaData,
    COMMAND_SET_META,
    COMMAND_GET_WAVE_AI_CHAT,
    COMMAND_GET_WAVE_AI_RATE_LIMIT,
    // existing:
    COMMAND_EVENT_SUB, COMMAND_EVENT_UNSUB, COMMAND_EVENT_UNSUB_ALL,
    COMMAND_GET_FULL_CONFIG, COMMAND_ROUTE_ANNOUNCE, COMMAND_ROUTE_UNANNOUNCE,
};
use crate::backend::ai::chatstore::DEFAULT_CHAT_STORE;
use super::service::update_object_meta;
```

---

## Build and Verify

```bash
# Build Rust backend
export PATH="/c/Systems/go/bin:/c/Systems/zig-windows-x86_64-0.13.0:$PATH"
cd /c/Systems/agentmux
task build:backend:rust

# Run Rust tests
cd agentmuxsrv-rs
cargo test 2>&1 | tail -5

# Launch dev mode, open browser console
# Confirm absence of: "ws: unknown command: setmeta"
# Confirm absence of: "ws: unknown command: getwaveaichat"
# Confirm absence of: "ws: unknown command: getwaveairatelimit"
```

---

## Risk and Scope

| Item | Risk | Note |
|------|------|------|
| `setmeta` without event broadcast | Low | DB correct, UI stale until reload |
| `setmeta` with event broadcast | Medium | Need EventBus method |
| `getwaveaichat` returning `null` for new chats | None | Matches Go behavior |
| `getwaveairatelimit` static response | None | Frontend just disables rate limit UI |
| `update_object_meta` visibility change | None | Only changes `pub(crate)` scope |

**Recommended order:** implement `getwaveairatelimit` first (trivial), then `getwaveaichat`, then `setmeta`.

---

## What This Fixes

- Block titles can be renamed (setmeta on block metadata)
- Tab names update when set by the frontend
- AI chat panel can render past conversations
- Rate limit console errors stop
- All 3 widgets functional without console errors
