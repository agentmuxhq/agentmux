# AGENTMUX_LOCAL_URL Pane Injection

**Status:** Approved
**Date:** 2026-03-16
**Author:** AgentY

---

## Problem

`inject_terminal` (jekt) times out for host agents.

The `agentbus-client` MCP server has two delivery paths:

1. **Local** — `POST {AGENTMUX_LOCAL_URL}/wave/reactive/inject` → AgentMux writes directly to the PTY. Sub-millisecond, no cloud involved.
2. **Cloud polling** — polls `agentbus.asaf.cc/reactive/pending/{agentId}` every 5s, delivers via `console.error()` on the MCP server's stderr. Stderr from an MCP child process is not fed back as user input in Claude Code, so injections are silently lost.

Path 1 is the correct path. Path 2 doesn't work for in-pane injection.

`AGENTMUX_LOCAL_URL` is only active when it's present in the MCP server's environment. Since each agent's MCP config is static (`.mcp.json`), we can't hardcode a fixed port — `agentmuxsrv-rs` binds to `127.0.0.1:0` (OS-assigned port) on every startup.

The fix: **AgentMux injects `AGENTMUX_LOCAL_URL` into each pane's environment at creation time.** Since AgentMux knows its own backend port, each pane automatically gets the correct URL for the instance that owns it.

---

## Why Not a Fixed Port

- Two instances running simultaneously (e.g. different versions) would conflict.
- Port 0 binding is intentional — avoids "port already in use" failures.
- A fixed port would require user configuration and break multi-version installs.

---

## Solution

Inject `AGENTMUX_LOCAL_URL=http://127.0.0.1:{web_port}` into every shell pane's environment, alongside the existing `AGENTMUX_AGENT_ID`, `AGENTMUX_VERSION`, etc.

**Multi-instance correctness:** each AgentMux instance injects its own backend port. A pane owned by instance A on port 58341 points to `http://127.0.0.1:58341`. A pane owned by instance B on port 62288 points to `http://127.0.0.1:62288`. No conflicts.

**agentbus-client behavior** (already implemented in v1.0.15):

```javascript
// client.js getConfigFromEnv()
localUrl: process.env.AGENTMUX_LOCAL_URL || process.env.WAVEMUX_LOCAL_URL

// index.js startup
if (config.localUrl) {
    // Path A: local delivery, cloud polling disabled
    console.error(`[AgentBus MCP] Local delivery: ${config.localUrl} (cloud polling disabled)`);
} else if (config.token) {
    // Path B: cloud polling (fallback only)
    stopPolling = startInjectionPolling(config, 5000);
}
```

When `AGENTMUX_LOCAL_URL` is set, the client routes all `inject_terminal` calls to the local AgentMux backend. No cloud round-trip, no polling, no timeout.

---

## Implementation

### The Port Discovery Problem

`shell.rs` (where env vars are injected) has no access to the web address:

- `web_addr` is computed in `main.rs` at startup via `TcpListener::bind("127.0.0.1:0")`
- It's emitted to stderr as `WAVESRV-ESTART web:<addr>` for the Tauri frontend
- It is **not** stored in `AppState` and **not** passed to `ShellController`

### Approach: `OnceLock<String>` Module-Level Global

Since `web_addr` is set once at startup and never changes, a module-level `OnceLock` is the minimal, clean solution. No constructor chain changes needed.

**New file:** `agentmuxsrv-rs/src/server_addr.rs`

```rust
use std::sync::OnceLock;

static BACKEND_WEB_ADDR: OnceLock<String> = OnceLock::new();

/// Called once from main.rs after the web listener binds.
pub fn set(addr: &str) {
    let _ = BACKEND_WEB_ADDR.set(addr.to_string());
}

/// Returns `http://127.0.0.1:{port}` or None if not yet set.
pub fn local_url() -> Option<String> {
    BACKEND_WEB_ADDR.get().map(|addr| format!("http://{}", addr))
}
```

**`main.rs`** — after `web_addr` is bound, before `WAVESRV-ESTART`:

```rust
crate::server_addr::set(&web_addr.to_string());
eprintln!("WAVESRV-ESTART ws:{} web:{} ...", ws_addr, web_addr, ...);
```

**`shell.rs`** — in the env var injection block, after existing `AGENTMUX_*` vars:

```rust
// Inject local AgentMux URL so agentbus-client uses direct PTY injection
// instead of cloud polling. Each pane gets the URL for the backend instance
// that owns it, which handles multiple AgentMux versions running simultaneously.
if let Some(local_url) = crate::server_addr::local_url() {
    c.env("AGENTMUX_LOCAL_URL", &local_url);
}
```

**`lib.rs`** — declare the new module:

```rust
pub mod server_addr;
```

---

## Affected Files

| File | Change |
|------|--------|
| `agentmuxsrv-rs/src/server_addr.rs` | New file — OnceLock storage |
| `agentmuxsrv-rs/src/lib.rs` | Add `pub mod server_addr;` |
| `agentmuxsrv-rs/src/main.rs` | Call `crate::server_addr::set(...)` after binding |
| `agentmuxsrv-rs/src/backend/blockcontroller/shell.rs` | Inject `AGENTMUX_LOCAL_URL` env var |

---

## No Changes Required

- `agentbus-client` already handles `AGENTMUX_LOCAL_URL` (v1.0.15+)
- `.mcp.json` templates don't need updating — env var comes from the pane
- Container agents (Agent1-5) are unaffected — they connect via cloud path or host networking

---

## Testing

After building and running `task dev`:

1. Open a new pane — verify `echo $AGENTMUX_LOCAL_URL` shows `http://127.0.0.1:{port}`
2. From an agent pane, call `inject_terminal` targeting itself — should return `status: delivered` within 1-2s
3. Start a second AgentMux instance — verify each instance's panes show different ports
