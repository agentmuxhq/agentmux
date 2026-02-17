# Rust Backend Migration Spec

## Part 1: Tree-Shake Dead Rust Code

### Current State

The Rust codebase in `src-tauri/src/` has **34,313 LOC across 91 files**. Of this, **21,783 LOC (63%)** is dead code from an abandoned effort to build the backend directly into the Tauri binary.

The entire `src-tauri/src/backend/` directory is gated behind a feature flag that is never enabled:

```rust
// lib.rs line 1
#[cfg(feature = "rust-backend")] mod backend;
```

```toml
# Cargo.toml
[features]
default = []  # rust-backend is never enabled
```

### What's Actually Used (~3,000 LOC)

| Module | LOC | Purpose |
|--------|-----|---------|
| `lib.rs` | 250 | App init, window setup, backend spawning |
| `sidecar.rs` | 390 | Spawns Go backend, deploys wsh |
| `state.rs` | 56 | Shared app state (auth, endpoints, zoom) |
| `heartbeat.rs` | 42 | Periodic heartbeat file |
| `crash.rs` | 54 | Panic handler |
| `menu.rs` | 281 | App menu (currently disabled) |
| `commands/` | 579 | IPC commands (platform, auth, window, devtools) |
| **Total** | ~1,650 | |

### What's Dead (~21,783 LOC)

The entire `backend/` directory duplicates Go backend functionality:

| Module | LOC | Go Equivalent |
|--------|-----|---------------|
| `ai/` (chatstore, openai, anthropic, tools) | 2,239 | `pkg/waveai` |
| `remote/` (sshclient, conncontroller) | 2,475 | `pkg/remote` |
| `storage/` (wstore, filestore, migrations) | 2,084 | `pkg/wstore`, `pkg/filestore` |
| `rpc/` (engine, router) | 1,521 | `pkg/wshrpc` |
| `wshutil/` | 1,335 | `pkg/wshutil` |
| `blockcontroller/` | 1,140 | `pkg/blockcontroller` |
| `reactive.rs` | 1,442 | `pkg/reactive` |
| `wconfig.rs` | 1,273 | `pkg/wconfig` |
| `vdom.rs` | 1,236 | `pkg/vdom` |
| `telemetry.rs` | 934 | `pkg/telemetry` |
| 30+ small modules | ~6,100 | Various `pkg/*` |
| **Total** | ~21,783 | |

### Cleanup Actions

1. **Delete** `src-tauri/src/backend/` (entire directory, 77 files, 21,783 LOC)
2. **Remove** `#[cfg(feature = "rust-backend")] mod backend;` from `lib.rs`
3. **Remove** `rust-backend` feature definition from `Cargo.toml`
4. **Remove** unused dependencies only needed by dead code:
   - `rusqlite` (bundled SQLite, ~500KB binary impact) -- **KEEP for Phase 2**
   - `base64`, `dirs`, `libc`, `subtle` -- review if used by active code
5. **Remove** `menu.rs` if frameless window is permanent

### Expected Impact

- **Binary size**: -3-5MB (12-20% reduction)
- **Compile time**: Significant improvement (21K fewer LOC to analyze)
- **Clarity**: No ambiguity about what code is running

---

## Part 2: Rewrite Go Backend in Rust (Separate Binary)

### Why Rewrite

1. **No GC pauses**: Terminal apps are latency-sensitive. Go's GC runs every ~2min, causing 10-40ms spikes (see Discord's rewrite). Rust has zero runtime overhead.
2. **Single toolchain**: The Tauri shell is already Rust. A Rust backend eliminates Go from the build pipeline.
3. **Lower memory**: Tokio tasks are ~64 bytes vs goroutines at ~2KB. Matters for many PTY sessions + SSH connections.
4. **Type safety**: RPC protocol, waveobj schema, and WebSocket messages verified at compile time.
5. **Existing work**: 21,783 LOC of Rust backend code already exists (ports of Go packages). This is a starting point, not a greenfield effort.

### Go Backend Scope (~35,000 LOC)

| Component | Go Package | Rust Equivalent Crate |
|-----------|-----------|----------------------|
| HTTP + WebSocket server | gorilla/mux, gorilla/websocket | **axum** (unified HTTP+WS) |
| SQLite database | mattn/go-sqlite3 | **rusqlite** (already in deps) |
| RPC protocol | custom wshrpc | **serde_json** + axum WS |
| PTY spawning | photostorm/pty | **portable-pty** (from WezTerm) |
| SSH connections | golang.org/x/crypto/ssh | **russh** (used by VS Code) |
| File watching | fsnotify/fsnotify | **notify** v8 |
| AI/LLM | sashabaranov/go-openai | **async-openai** or **reqwest** |
| Telemetry | go.opentelemetry.io | **tracing** + **tracing-opentelemetry** |
| Config | custom wconfig | **serde** + toml/json |
| Shell integration | custom shellutil | custom (reuse existing Rust code) |

### Architecture

```
[Tauri App (Rust)] --spawns--> [agentmuxsrv-rs (Rust binary)]
                                    |
                                    +-- axum HTTP/WS server (same ports)
                                    +-- rusqlite waveobj storage
                                    +-- portable-pty shell management
                                    +-- russh SSH connections
                                    +-- tracing telemetry
                                    |
                                    +-- emits WAVESRV-ESTART on stderr
                                        (same protocol as Go backend)
```

The Rust backend is a **drop-in replacement** for `agentmuxsrv`. Same startup protocol, same HTTP/WS API, same `wave-endpoints.json` file. The frontend and `sidecar.rs` require zero changes.

### Migration Strategy (Incremental, from Turborepo)

**Do not big-bang rewrite.** Port component by component while both backends run.

#### Phase 1: Scaffold & Storage (Weeks 1-3)

Create `agentmuxsrv-rs` binary that:
1. Accepts same CLI args as Go backend (`--wavedata`, env vars)
2. Starts an Axum server on a random port
3. Emits `WAVESRV-ESTART ws:ADDR web:ADDR version:VER` on stderr
4. Implements SQLite waveobj CRUD (reuse existing `storage/` Rust code)
5. Serves static health check endpoint

**Test**: Point `sidecar.rs` at the Rust binary, verify frontend connects.

#### Phase 2: RPC & WebSocket (Weeks 4-6)

1. Implement WebSocket upgrade endpoint matching Go's `/ws` path
2. Port the RPC message protocol (reuse `rpc/` and `rpc_types.rs`)
3. Implement core RPC methods: `getmeta`, `setmeta`, `createblock`, `deleteblock`
4. Frontend can now read/write blocks via Rust backend

#### Phase 3: PTY & Shell (Weeks 7-10)

1. Integrate `portable-pty` for local shell spawning
2. Port `blockcontroller` logic (reuse existing Rust code)
3. Implement shell integration file deployment (reuse `shellutil.rs`)
4. Port `wsh` binary to Rust (or keep Go wsh temporarily)

#### Phase 4: SSH & Remote (Weeks 11-14)

1. Integrate `russh` for SSH client connections
2. Port connection controller and SSH config parsing
3. Implement remote PTY spawning over SSH
4. Port wsh-over-SSH deployment

#### Phase 5: AI & Polish (Weeks 15-18)

1. Port AI/LLM integration (reuse `ai/` Rust code)
2. Port reactive messaging system
3. Port telemetry with OpenTelemetry bridge
4. Port remaining utility functions
5. Remove Go backend from build pipeline

### Crate Dependencies

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP + WebSocket server
axum = { version = "0.7", features = ["ws"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }

# Database
rusqlite = { version = "0.32", features = ["bundled"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# SSH
russh = "0.44"
russh-keys = "0.44"

# PTY
portable-pty = "0.9"

# File watching
notify = "8.2"

# AI/LLM
async-openai = "0.25"

# Telemetry
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-opentelemetry = "0.28"

# Utilities
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
thiserror = "2"
anyhow = "1"
reqwest = { version = "0.12", features = ["json", "stream"] }
```

### Binary Structure

```
agentmuxsrv-rs/
  Cargo.toml
  src/
    main.rs           # CLI args, signal handling, WAVESRV-ESTART
    server.rs         # Axum HTTP + WS server
    config.rs         # Configuration (env vars, files)
    db/
      mod.rs          # Connection pool, migrations
      waveobj.rs      # waveobj CRUD
    rpc/
      mod.rs          # Dispatcher
      protocol.rs     # Message types
      handlers.rs     # Method implementations
    shell/
      mod.rs          # PTY management
      controller.rs   # Block controller
      integration.rs  # Shell RC files, wsh deployment
    ssh/
      mod.rs          # SSH client
      connpool.rs     # Connection lifecycle
    ai/
      mod.rs          # Provider abstraction
      openai.rs       # OpenAI
      anthropic.rs    # Anthropic
    telemetry/
      mod.rs          # Tracing + OpenTelemetry
```

### Cross-Platform Considerations

| Concern | macOS | Linux | Windows |
|---------|-------|-------|---------|
| PTY | native openpty | native openpty | ConPTY via portable-pty |
| SSH | russh (pure Rust) | russh (pure Rust) | russh (pure Rust) |
| SQLite | rusqlite bundled | rusqlite bundled | rusqlite bundled |
| File watching | FSEvents | inotify | ReadDirectoryChanges |
| Shell | bash/zsh | bash/sh | cmd/powershell |

All recommended crates are cross-platform. `portable-pty` (from WezTerm) is battle-tested on all three platforms.

### Key Risks

| Risk | Mitigation |
|------|-----------|
| Behavioral regressions | Write integration tests against Go backend first, run same tests against Rust |
| SSH edge cases | russh is used by VS Code (Microsoft fork), well-tested |
| Windows ConPTY quirks | portable-pty from WezTerm handles these |
| Build complexity during transition | Feature flags to switch between Go and Rust backends |
| Existing Rust code quality | 21K LOC was never tested -- validate before reusing |

### Success Criteria

- [ ] `agentmuxsrv-rs` passes all existing integration tests
- [ ] Frontend works identically with Rust and Go backends
- [ ] No GC-related latency spikes in terminal I/O
- [ ] Binary size equal or smaller than Go backend (~30MB)
- [ ] Go removed from build pipeline entirely
- [ ] Cross-platform CI passing (macOS, Linux, Windows)

### References

- [Turborepo Go-to-Rust migration (Vercel)](https://vercel.com/blog/turborepo-migration-go-rust)
- [Discord: Why switching from Go to Rust](https://discord.com/blog/why-discord-is-switching-from-go-to-rust)
- [Grab: Counter service rewrite in Rust (70% savings)](https://engineering.grab.com/counter-service-how-we-rewrote-it-in-rust)
- [russh - SSH library used by VS Code](https://github.com/Eugeny/russh)
- [portable-pty - PTY from WezTerm](https://docs.rs/portable-pty)
- [axum - HTTP framework by Tokio team](https://github.com/tokio-rs/axum)
- [Tauri sidecar pattern](https://v2.tauri.app/develop/sidecar/)
