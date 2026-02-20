# Spec: Rewrite wsh in Rust

## 1. Overview

**wsh** (Wave Shell Helper) is a CLI tool and RPC client that lets users control AgentMux from the terminal. It currently exists as a Go binary (~10.7 MB) with 54+ subcommands across ~10,700 lines of Go code. This spec proposes rewriting it in Rust to reduce binary size, improve startup latency, and unify the codebase with the Rust backend (agentmuxsrv-rs).

---

## 2. Current Go Architecture

### 2.1 Codebase Structure

| Component | Files | Lines | Purpose |
|-----------|-------|-------|---------|
| `cmd/wsh/main-wsh.go` | 1 | 19 | Entry point |
| `cmd/wsh/cmd/` | 38 | 4,770 | Cobra command implementations |
| `pkg/wshutil/` | 9 | 2,754 | RPC transport, OSC encoding, routing |
| `pkg/wshrpc/wshclient/` | 3 | 776 | 70+ RPC call wrappers |
| `pkg/wshrpc/wshremote/` | 2 | 977 | Remote wsh client, sysinfo |
| `pkg/wshrpc/wshserver/` | 4 | 1,408 | Server-side RPC handlers |
| **Total** | **57** | **10,704** | |

### 2.2 Subcommands (54+)

| Category | Commands | Lines |
|----------|----------|-------|
| Block management | blocks list/create/delete | 393 |
| File operations | file list/cat/info/write/append/rm/cp/mv, readfile | 819 |
| Metadata/vars | getmeta, setmeta, getvar, setvar | 574 |
| Connections | conn status/connect/disconnect/ensure/reinstall, ssh, wsl | 381 |
| Terminal/views | term, view, web get/open, editor, launch | 470 |
| Configuration | setconfig, editconfig, setbg | 303 |
| AI integration | ai (attach files, messages, auto-submit) | 193 |
| Shell integration | shell (unix/win), rcfiles | 93 |
| Infrastructure | connserver, debug, test, token, version, wavepath, agentmux, notify, run, workspace | 544 |

### 2.3 Communication Architecture

```
User Shell
    |
    v
wsh binary (Cobra CLI)
    |
    +-- Terminal Mode: OSC 23198 escape sequences (bidirectional JSON)
    |   Used when wsh runs inside a Wave terminal pane
    |
    +-- Token Mode: Unix domain socket + JWT auth
        Used when WAVE_JWT_TOKEN is set (non-interactive)
    |
    v
agentmuxsrv (backend) --> Tauri App (frontend)
```

### 2.4 Key Go Dependencies

| Dependency | Purpose | Rust Equivalent |
|------------|---------|-----------------|
| `spf13/cobra` | CLI framework | `clap` |
| `golang.org/x/term` | Terminal raw mode, TTY detection | `crossterm` or `termion` |
| `golang-jwt/jwt/v5` | JWT token parsing | `jsonwebtoken` |
| `google/uuid` | UUID generation | `uuid` (already in Cargo.toml) |
| `gorilla/websocket` | WebSocket | `tokio-tungstenite` |
| `kevinburke/ssh_config` | SSH config parsing | `ssh2-config` |
| `ubuntu/gowsl` | WSL integration | Windows API calls directly |
| `creack/pty` | PTY operations | `portable-pty` (already in Cargo.toml) |
| `shirou/gopsutil/v4` | System metrics | `sysinfo` (already in Cargo.toml) |

---

## 3. Expected Size & Performance Comparisons

### 3.1 Binary Size

| Metric | Go (current) | Rust (expected) | Reduction |
|--------|-------------|-----------------|-----------|
| Windows x64 | 11.2 MB | 1.5-3.0 MB | 70-85% |
| Linux x64 | 10.7 MB | 1.2-2.5 MB | 75-88% |
| macOS ARM64 | 10.2 MB | 1.5-2.8 MB | 72-85% |

**Why smaller:**
- Go embeds its runtime + GC (~2-4 MB baseline) in every binary
- Rust has no runtime overhead; `#[no_std]` parts compile to bare metal
- Rust's LTO (link-time optimization) aggressively eliminates dead code
- `strip` + `opt-level = "z"` can further reduce size
- UPX compression available as a last resort (Go binaries don't compress well)

**Note:** Size depends heavily on which features are included. A minimal wsh with just core commands (blocks, files, meta, term) would be ~1.5 MB. Full feature parity including SSH/WSL integration would be ~2.5-3.0 MB.

### 3.2 Startup Latency

| Metric | Go (current) | Rust (expected) | Improvement |
|--------|-------------|-----------------|-------------|
| Cold start (no cache) | 15-30 ms | 1-5 ms | 3-10x |
| Warm start (cached) | 8-15 ms | <1-3 ms | 3-8x |

**Why faster:**
- No Go runtime initialization (~5-10 ms for goroutine scheduler, GC setup)
- No reflection-based flag parsing (Cobra uses reflect; clap uses compile-time macros)
- Rust's zero-cost abstractions mean CLI parsing is near-instant

### 3.3 Memory Usage

| Metric | Go (current) | Rust (expected) | Reduction |
|--------|-------------|-----------------|-----------|
| Baseline RSS | 8-15 MB | 1-3 MB | 70-85% |
| Peak (file ops) | 20-50 MB | 5-15 MB | 60-75% |

**Why lower:**
- Go GC requires ~2x heap overhead for efficient collection
- Rust's ownership model means memory is freed deterministically
- No goroutine stack overhead (each goroutine: 2-8 KB)

### 3.4 Cross-compilation

| Metric | Go | Rust |
|--------|-----|------|
| Cross-compile ease | Excellent (`GOOS/GOARCH`) | Good (requires target toolchain) |
| Static linking | Default | Default with musl |
| CGo dependency risk | Low (wsh is pure Go) | None |

Go has an edge here. Rust cross-compilation requires installing target toolchains, but tools like `cross` (Docker-based) make this manageable. The project already cross-compiles Rust (agentmuxsrv-rs) so the infrastructure exists.

---

## 4. Rust Architecture

### 4.1 Proposed Crate Structure

```
wsh-rs/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, clap CLI setup
│   ├── cli/
│   │   ├── mod.rs            # CLI command definitions
│   │   ├── blocks.rs         # blocks list/create/delete
│   │   ├── file.rs           # file list/cat/info/write/append/rm/cp/mv
│   │   ├── meta.rs           # getmeta, setmeta
│   │   ├── vars.rs           # getvar, setvar
│   │   ├── conn.rs           # conn status/connect/disconnect/ensure
│   │   ├── term.rs           # term (create terminal block)
│   │   ├── view.rs           # view, web, editor, launch
│   │   ├── config.rs         # setconfig, editconfig, setbg
│   │   ├── ai.rs             # ai (attach files, messages)
│   │   ├── shell.rs          # shell integration (unix/win)
│   │   ├── debug.rs          # debug, test
│   │   ├── info.rs           # version, wavepath, workspace, notify
│   │   └── connserver.rs     # connserver, ssh, wsl
│   ├── rpc/
│   │   ├── mod.rs            # RPC client initialization
│   │   ├── client.rs         # RPC call wrappers (70+ functions)
│   │   ├── transport.rs      # Socket/WebSocket transport
│   │   ├── router.rs         # Message routing, request tracking
│   │   └── osc.rs            # OSC 23198 terminal-mode encoding
│   ├── auth/
│   │   ├── mod.rs
│   │   └── jwt.rs            # JWT token parsing
│   └── util/
│       ├── mod.rs
│       ├── csscolor.rs       # CSS color map
│       └── path.rs           # Wave path utilities
```

### 4.2 Dependency Map

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }     # CLI framework
serde = { version = "1", features = ["derive"] }     # Serialization
serde_json = "1"                                     # JSON
tokio = { version = "1", features = ["rt", "net", "io-util", "time"] }
uuid = { version = "1", features = ["v4"] }          # UUIDs
jsonwebtoken = "9"                                   # JWT
base64 = "0.22"                                      # Base64
crossterm = "0.28"                                   # Terminal raw mode
dirs = "5"                                           # Platform dirs
```

**Optional (for full feature parity):**
```toml
# Only needed if SSH/WSL/remote features are included
ssh2 = "0.9"                                         # SSH connections
sysinfo = "0.34"                                     # System metrics (connserver mode)
```

### 4.3 Shared Code with agentmuxsrv-rs

These types/modules already exist in the Rust backend and can be extracted into a shared crate:

| Module | Current Location | Shared Types |
|--------|-----------------|-------------|
| `rpc_types.rs` | `agentmuxsrv-rs/src/backend/` | `RpcMessage`, `CommandGetMetaData`, `CommandSetMetaData`, all `Command*Data` structs, `TimeSeriesData` |
| `wps.rs` | `agentmuxsrv-rs/src/backend/` | `WaveEvent`, `SubscriptionRequest` |
| `oref.rs` | `agentmuxsrv-rs/src/backend/` | `ORef` |
| `waveobj.rs` | `agentmuxsrv-rs/src/backend/` | `Block`, `MetaMapType`, `TermSize` |

**Strategy:** Create a `wavemux-types` workspace crate shared between `agentmuxsrv-rs` and `wsh-rs`.

### 4.4 Communication Protocol

The RPC transport implementation must support two modes:

**Mode 1: Token Mode (domain socket)**
```
1. Read WAVE_JWT_TOKEN env var
2. Connect to Unix domain socket at WAVETERM_DATA_HOME/wave.sock
3. Send authenticate RPC with JWT
4. Exchange JSON RpcMessages over socket
```

**Mode 2: Terminal Mode (OSC escape sequences)**
```
1. Detect running inside Wave terminal (env vars)
2. Use OSC 23198 encoding to wrap JSON RPC messages
3. Read/write via stdin/stdout in raw mode
4. Parse incoming OSC sequences from terminal output
```

---

## 5. Implementation Phases

### Phase 1: Core CLI + Token Mode RPC (MVP)
**Estimated effort: ~2,000 lines of Rust**

- [ ] `clap` CLI skeleton with all 54+ subcommands (argument parsing only)
- [ ] RPC client over domain socket (token mode)
- [ ] JWT authentication
- [ ] Core commands: `version`, `getmeta`, `setmeta`, `getvar`, `setvar`
- [ ] Block commands: `blocks list`, `blocks create`, `blocks delete`
- [ ] Terminal: `term` (create terminal block)

### Phase 2: File Operations + View Commands
**Estimated effort: ~1,500 lines**

- [ ] File operations: `file list/cat/info/write/append/rm/cp/mv`
- [ ] `readfile` (streaming read)
- [ ] View commands: `view`, `web get/open`, `editor`, `launch`
- [ ] `setconfig`, `editconfig`, `setbg`
- [ ] `wavepath`, `workspace`, `notify`

### Phase 3: Terminal Mode (OSC) + Shell Integration
**Estimated effort: ~1,500 lines**

- [ ] OSC 23198 encoder/decoder
- [ ] Terminal raw mode I/O with `crossterm`
- [ ] Shell integration scripts (bash/zsh/fish/pwsh)
- [ ] `shell` command (unix + windows variants)
- [ ] `rcfiles` command

### Phase 4: Connection Management
**Estimated effort: ~1,000 lines**

- [ ] `conn status/connect/disconnect/ensure/reinstall`
- [ ] `ssh` command
- [ ] `wsl` command
- [ ] `connserver` mode

### Phase 5: AI + Advanced Features
**Estimated effort: ~500 lines**

- [ ] `ai` command (attach files, messages, auto-submit)
- [ ] `run` command
- [ ] `debug` utilities
- [ ] `agentmux` subcommands

### Phase 6: Shared Types Crate + Build Integration
**Estimated effort: ~500 lines (mostly refactoring)**

- [ ] Extract `wavemux-types` crate from agentmuxsrv-rs
- [ ] Update both wsh-rs and agentmuxsrv-rs to use shared crate
- [ ] Integrate into Taskfile.yml build system
- [ ] Cross-compilation for all platforms
- [ ] Update `bump-version.sh` to include wsh-rs

---

## 6. Migration Strategy

### 6.1 Parallel Operation Period

During development, both Go and Rust wsh binaries will coexist:
- Go wsh continues shipping as the production binary
- Rust wsh available as `wsh-rs` for testing
- Command-by-command parity testing via automated comparison

### 6.2 Cutover Criteria

- [ ] All 54+ commands pass integration tests
- [ ] Terminal mode (OSC) working on bash, zsh, fish, pwsh
- [ ] Cross-compiled for all 6 platform targets
- [ ] Binary size < 3 MB on all platforms
- [ ] No regressions in shell integration
- [ ] `connserver` mode working for remote connections

### 6.3 Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Cross-compilation complexity | Build failures on some targets | Use `cross` tool, test in CI |
| OSC terminal mode edge cases | Shell integration breaks | Port tests from Go, test on all shells |
| SSH/WSL platform-specific code | Windows-only features break | Feature-gate behind `#[cfg(windows)]` |
| Shared types drift | Backend and wsh types diverge | Single shared crate, CI enforced |

---

## 7. Success Metrics

| Metric | Target |
|--------|--------|
| Binary size (Windows x64) | < 3.0 MB (vs 11.2 MB Go) |
| Cold start latency | < 5 ms (vs 15-30 ms Go) |
| Memory baseline | < 3 MB RSS (vs 8-15 MB Go) |
| Code lines (Rust) | ~7,000 (vs 10,700 Go) |
| All commands passing | 54/54 |
| Platform targets | 6/6 (win/linux/mac x x64/arm64) |
