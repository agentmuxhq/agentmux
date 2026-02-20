# Retro: Rust Backend Session 3 — Widgets Fix

**Date:** 2026-02-18
**Branch:** `agento/fix-service-arg-indices`
**Status:** Widgets fix committed, verifying in dev

---

## What Was Accomplished This Session

### 1. Root Cause Found: Widgets in Wrong Field

**Problem:** App rendered with only "help" and "devtools" buttons. The 3 default widget buttons (terminal, agent, sysinfo) were missing from the top-right toolbar.

**Investigation path:**
1. Verified `load_default_config()` was compiled into the binary (strings search found "defwidget", "sparkles", "square-terminal" embedded in binary)
2. Wrote a WebSocket test script to call `getfullconfig` directly — found the response had `defaultwidgets` populated but `widgets` empty
3. Traced Go backend's `readConfigPart("widgets", ...)` — it reads `defaultconfig/widgets.json` into `fullConfig.Widgets` (the `widgets` JSON field), NOT into `defaultwidgets`
4. Frontend `widgets.tsx` line 87: `const widgetsMap = fullConfig?.widgets ?? {}` — reads from `widgets`, not `defaultwidgets`

**Root cause:** In `main.rs` `load_default_config()`, the parsed `widgets.json` was assigned to `config.default_widgets` (serialized as `"defaultwidgets"`) instead of `config.widgets` (serialized as `"widgets"`).

**Fix:** One-line change in `main.rs`:
```rust
// BEFORE (wrong field):
config.default_widgets = widgets;

// AFTER (correct field, mirrors Go behavior):
config.widgets = widgets;
```

**Commit:** `4bb84bd fix(rust-backend): widgets in correct field — fix missing sidebar widgets`

### 2. Verification

Wrote `test-getfullconfig.mjs` — starts Rust backend, connects via WebSocket, calls `getfullconfig`, checks response:
```
widgets keys: [ 'defwidget@agent', 'defwidget@sysinfo', 'defwidget@terminal' ]
PASS: widgets populated correctly
```

### 3. Dev Session Issue

- Port 5173 was already in use from previous session, causing `task dev` to fail
- Old AgentMux window (PID 2108, 3:59 AM) was still showing with the old binary
- Fixed by killing all processes and restarting `task dev`

---

## Current State

### Branch: `agento/fix-service-arg-indices`
Commits (newest first):
1. `4bb84bd` — fix: widgets in correct field
2. `528f00a` — fix: sync agentmuxsrv-rs in dev binaries + fix Go ExpectedVersion
3. `9e5205a` — debug: add sendLog checkpoints to initWave
4. `883ded8` — fix: service arg index off-by-one (blank screen root fix)

### What's Working
- ✅ Rust backend builds and starts (emits WAVESRV-ESTART)
- ✅ All 13 server integration tests pass
- ✅ 7/7 parity tests vs Go backend pass
- ✅ `getfullconfig` WebSocket RPC returns correct widgets
- ✅ App renders React UI (no blank grey screen)
- ✅ `widgets.json` (terminal, agent, sysinfo) now in correct `widgets` field

### Pending Verification
- [ ] App visually shows terminal, agent, sysinfo widget buttons in top-right toolbar
- [ ] Widgets can be clicked to open blocks (terminal, agent panel, sysinfo)

### Still Needs Work (known gaps vs Go backend)
- Many service methods return 501 stub (reactive/conn/layout endpoints)
- Config file watching (dynamic reload on file change) not yet implemented
- WshRPC routing for block-scoped commands incomplete
- PR #343 (linux native decorations) still open/unmerged

---

## Key Learnings

### Go vs Rust Config Field Mapping

| Go field | Go JSON tag | Rust field | Rust serde tag |
|---|---|---|---|
| `Widgets` | `"widgets"` | `widgets` | `"widgets"` (default) |
| `DefaultWidgets` | `"defaultwidgets"` | `default_widgets` | `"defaultwidgets"` |

Go's `readConfigPart("widgets", ...)` reads both `defaultconfig/widgets.json` AND user's `~/.config/agentmux/widgets.json` → populates `fullConfig.Widgets`.

The `DefaultWidgets` field is for overrides that ship with the app default config, but in practice `defaultconfig/defaultwidgets.json` does not exist — so `DefaultWidgets` is always empty in Go too.

### Debugging WebSocket RPC

To test WebSocket RPC directly:
1. Match on `rpc.resid === reqId` (response has `resid`, not `reqid`)
2. WS URL: `ws://127.0.0.1:{WS_PORT}/ws?tabid=test-tab&authkey={AUTH}`
3. Send `routeannounce` first, then the actual command
4. Message wrapper: `{ wscommand: "rpc", message: { command, reqid, source, route } }`

### Task Dev Port Conflicts

Always kill existing processes before restarting:
```bash
taskkill /F /IM agentmux.exe /T
netstat -ano | grep ":5173 " | awk '{print $5}' | xargs taskkill /F /PID
```

---

## Files Modified This Session

| File | Change |
|---|---|
| `agentmuxsrv-rs/src/main.rs` | `config.default_widgets` → `config.widgets` in `load_default_config()` |

## Test Scripts Created (then cleaned up)

- `test-getfullconfig.mjs` — WebSocket test for getfullconfig response (deleted after use)
- `parity.mjs` — HTTP parity test vs Go backend (was `/tmp/parity.mjs`, deleted)
