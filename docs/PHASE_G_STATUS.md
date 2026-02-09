# Phase G: Config System + Schema Endpoints — Status

**Date:** 2026-02-09
**Branch:** `agenta/phase-g-config-schema`
**PR:** https://github.com/a5af/wavemux/pull/224
**Version:** 0.20.10

## Status: COMPLETE

## What Was Done

### Problem
The `getfullconfig` RPC handler returned an empty stub — no settings, no terminal themes, no widgets, no presets, no bookmarks. The app started but had zero configuration data. Monaco schema validation didn't work (no HTTP server in rust-backend mode).

### Solution

| Component | Change |
|-----------|--------|
| **Default configs** | 6 JSON files copied from Go's `pkg/wconfig/defaultconfig/` into `src-tauri/src/backend/defaultconfig/` and embedded at compile time via `include_str!()` |
| **Config loading** | `load_full_config()` merges embedded defaults + user `~/.waveterm/config/` overrides with namespace clear keys and `$ENV:VAR` expansion |
| **Config writing** | `set_base_config_value()` and `set_connections_config_value()` for read-merge-write on settings.json and connections.json |
| **AppState** | Added `config_watcher: Arc<ConfigWatcher>` and `config_dir: PathBuf` |
| **RPC handlers** | `getfullconfig` returns real config, `setconfig`/`setconnectionsconfig` write + reload + broadcast |
| **Schema delivery** | `get_schema` Tauri command with 4 embedded schema JSONs, frontend uses `invoke()` in rust-backend mode |

### Files Changed

**New (6):**
- `src-tauri/src/backend/defaultconfig/settings.json`
- `src-tauri/src/backend/defaultconfig/termthemes.json`
- `src-tauri/src/backend/defaultconfig/presets.json`
- `src-tauri/src/backend/defaultconfig/presets_ai.json`
- `src-tauri/src/backend/defaultconfig/widgets.json`
- `src-tauri/src/backend/defaultconfig/mimetypes.json`

**Modified (6):**
- `src-tauri/src/backend/wconfig.rs` — +378 lines (loading, merging, writing)
- `src-tauri/src/commands/rpc.rs` — +111 lines (RPC handlers, schema command)
- `src-tauri/src/state.rs` — +8 lines (new fields)
- `src-tauri/src/rust_backend.rs` — +17 lines (config init)
- `src-tauri/src/lib.rs` — +1 line (register get_schema)
- `frontend/app/view/codeeditor/schemaendpoints.ts` — rewritten for dual-mode

### Out-of-Box Content
| Type | Count | Examples |
|------|-------|---------|
| Terminal themes | 7 | Default Dark, One Dark Pro, Dracula, Monokai, Campbell, Warm Yellow, Rose Pine |
| Default widgets | 6 | terminal, files, web, ai, sysinfo, claude code |
| Background presets | 14 | Rainbow, Ocean Depths, Sunset, Cosmic Tide, etc. |
| AI presets | 2 | Global default, Wave Proxy |
| MIME type icons | 21 | PDF, JS, Rust, Go, HTML, Markdown, etc. |

### Build Results
| Check | Result |
|-------|--------|
| `cargo check --features go-sidecar` | Compiles (no regression) |
| `cargo check --no-default-features --features rust-backend` | Compiles |
| `cargo test` (rust-backend) | 1065 passed, 4 pre-existing failures |
| wconfig tests | 29/29 passed |

## What's Deferred
- **Filesystem watching** (`notify` crate) — config changes on disk won't auto-reload yet
- **File streaming** (`/wave/stream-file`) — large file streaming endpoint
- **Profile-based config** — `profiles.json` loading

## Phase Progression
```
Phase A: SQLite store          [DONE]
Phase B: WPS broker            [DONE]
Phase C: Frontend RPC adapter  [DONE]
Phase D: Terminal PTY           [DONE]
Phase E+F: wsh IPC + file ops  [DONE]
Phase G: Config + schemas      [DONE] <-- this phase
Phase H: ???                   [TODO]
```
