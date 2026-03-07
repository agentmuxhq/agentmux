# Instance Isolation Analysis: Dev vs Production

**Date:** 2026-03-07
**Context:** Investigating silent exits when running `task dev` alongside a production instance.

## TL;DR

Dev builds now use a fixed Tauri identifier (`ai.agentmux.app.dev`) via config merging, giving them a completely separate data directory from any production build. This prevents silent exits caused by WebView2 User Data Folder conflicts.

---

## Problem

Running `task dev` while a production instance of the **same version** is open causes silent exits. Root cause: both instances share the same Tauri identifier, which means:

1. **WebView2 UDF conflict** — WebView2 enforces single-process access to its User Data Folder. Second process fails silently.
2. **SQLite contention** — Both backends open the same `wave.db` and `filestore.db`.
3. **Dev cache clearing** — Debug builds delete `{data_dir}\EBWebView\Default\Cache\` on startup, potentially crashing the running prod instance.

## Fix

`src-tauri/tauri.dev.conf.json` overrides only the identifier:
```json
{ "identifier": "ai.agentmux.app.dev" }
```

`task dev` and `task quickdev` pass `--config src-tauri/tauri.dev.conf.json` to `tauri dev`, which merges it on top of `tauri.conf.json`.

**Result:** Dev builds always use `%LOCALAPPDATA%\ai.agentmux.app.dev\` regardless of version, completely separate from production.

---

## Verification

### Data directory isolation

| Build | Identifier | Data Dir |
|-------|-----------|----------|
| Dev (`task dev`) | `ai.agentmux.app.dev` | `%LOCALAPPDATA%\ai.agentmux.app.dev\` |
| Prod v0.31.74 | `ai.agentmux.app.v0-31-74` | `%LOCALAPPDATA%\ai.agentmux.app.v0-31-74\` |
| Prod v0.31.79 | `ai.agentmux.app.v0-31-79` | `%LOCALAPPDATA%\ai.agentmux.app.v0-31-79\` |

### Resources confirmed isolated

| Resource | Dev | Prod | Conflict? |
|----------|-----|------|-----------|
| WebView2 UDF | `..app.dev\EBWebView\` | `..app.v0-31-79\EBWebView\` | No |
| SQLite databases | `..app.dev\instances\v{VER}\db\` | `..app.v0-31-79\instances\v{VER}\db\` | No |
| Heartbeat file | `..app.dev\agentmux.heartbeat` | `..app.v0-31-79\agentmux.heartbeat` | No |
| Config dir | `%APPDATA%\ai.agentmux.app.dev\` | `%APPDATA%\ai.agentmux.app.v0-31-79\` | No |
| Backend ports | `127.0.0.1:0` (random) | `127.0.0.1:0` (random) | No |
| Log files | `~/.agentmux/logs/agentmux-host-v{VER}.log.*` | Same dir, different filename | No |
| Vite dev server | `localhost:5173` | Not used (bundled frontend) | No |

### Tauri config merging verified

Tauri v2 `--config` flag performs a deep merge. Only the `identifier` field is overridden; all other config (window settings, CSP, plugins, bundle config) comes from the base `tauri.conf.json`.

### Production builds unaffected

`task package` does NOT pass `--config`, so production builds continue using the version-specific identifier from `tauri.conf.json`.

---

## Remaining gaps

1. **No single-instance guard** — Two dev instances can still collide (same `ai.agentmux.app.dev` identifier). Consider adding `tauri-plugin-single-instance` in the future.
2. **Unused WaveLock** — `wavebase.rs:191-238` defines file-based locking but it's never called during startup.
