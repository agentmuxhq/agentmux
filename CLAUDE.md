# Claude Agent Development Guide - WaveMux

---

## 📚 Agent Messaging Terminology

| Verb | Meaning | Tool | Description |
|------|---------|------|-------------|
| **mux** | Send a message | `send_message` MCP | Async mailbox delivery - recipient reads when ready |
| **ject** | Inject a message | `inject_terminal` MCP | Direct terminal injection - recipient processes immediately |

**Examples:**
- "mux to AgentY" = Send a mailbox message (`send_message`)
- "ject to AgentA" = Inject into terminal (`inject_terminal`)

**Note:** Both tools route through AgentMux cloud for cross-host delivery.

---

## Repository

- **Name:** WaveMux
- **GitHub:** https://github.com/a5af/wavemux
- **Type:** Tauri v2 terminal application
- **Version:** 0.19.0

## Git & Pull Requests

- Push branches to https://github.com/a5af/wavemux
- Open PRs against a5af/wavemux main branch
- Branch naming: `agent[X]/feature-name` (e.g., `agentx/fix-version`)

---

## Development Workflow

### Commands (Use Correctly!)

| Command | Use When | Auto-Updates? |
|---------|----------|---------------|
| `task dev` | **Development** (normal work) | ✅ Yes - hot reload |
| `task start` | Standalone testing (rare) | ❌ No |
| `task package` | **Final release builds ONLY** | ❌ No |

**CRITICAL:** Never launch from `make/` during development - it's stale and will crash with "wavemuxsrv.x64.exe ENOENT"

### After Code Changes

- **TypeScript/React** → Auto-reloads in `task dev` ✅
- **Go backend** → `task build:backend` then restart `task dev`
- **Test package** → `task package` then extract/install artifact

### Architecture

WaveMux is built on **Tauri v2** (NOT Electron):

- **wavemux.exe** = Tauri app (Rust + single webview)
- **wavemuxsrv** = Go backend sidecar (auto-spawned, don't run manually)
- **wsh** = Shell integration binary (must be versioned correctly)

**Important:** All Electron code has been removed (Phase 14). Only Tauri is supported.

---

## Version Management

**See [README.md](README.md) for complete guide.**

### Quick Reference

```bash
# Bump version (updates ALL files)
./bump-version.sh patch --message "Description"

# Rebuild binaries with new version
task build:backend

# Verify consistency
bash scripts/verify-version.sh

# Push
git push origin <branch> --tags
```

**Current version:** See [VERSION_HISTORY.md](VERSION_HISTORY.md)

---

## Agent Workspace Pattern

### Bare Repository

- **Location:** `D:\Code\projects\wavemux.git`
- **Type:** Git bare repository (no working directory)
- **Remote:** https://github.com/a5af/wavemux

### Agent Worktrees

- **Location:** `D:\Code\agent-workspaces\agent[X]\wavemux\`
- **Branch:** `agent[X]/feature-name`
- **Setup:** Created on-demand via `git worktree add`

### Workflow

```bash
# Create feature branch
git checkout -b agentx/feature-name

# Make changes, commit
git commit -m "feat: description"

# Push to remote
git push -u origin agentx/feature-name

# Create PR via GitHub
gh pr create --title "Feature" --body "Description"
```

---

## Testing

```bash
# Run all tests
npm test

# Run e2e tests
npm test -- app.e2e.test.ts

# Generate coverage
npm run coverage
```

**E2E Test Suite:** 14 tests covering version display, wsh deployment, shell integration, and error handling.

---

## Build System

### Backend (Go)

```bash
# Build all binaries (wavemuxsrv, wsh for all platforms)
task build:backend

# Build specific platform
GOOS=linux GOARCH=amd64 go build -o dist/bin/wsh-0.12.15-linux.x64 ./cmd/wsh
```

### Frontend (TypeScript/React)

```bash
# Development build (fast)
npm run build:dev

# Production build (optimized)
npm run build:prod
```

### Package Release

```bash
# Create distributable package
task package

# Output: dist/Wave-win32-x64-0.12.15.zip (or platform-specific)
```

---

## Common Issues

### Issue: wsh binary not found

**Cause:** Version mismatch between package.json and built binaries

**Fix:**
```bash
# Check version
cat package.json | grep version

# Rebuild with correct version
task build:backend

# Verify binaries exist
ls -lh dist/bin/wsh-*
```

### Issue: Title bar shows wrong version

**Cause:** Frontend not using dynamic version API

**Fix:** Ensure `frontend/wave.ts` uses `getApi().getAboutModalDetails().version`

### Issue: Test failures

**Cause:** Old binaries in dist/bin from previous versions

**Fix:**
```bash
# Remove old binaries
cd dist/bin && ls wsh-* | grep -v "0.12.15" | xargs rm -f

# Rebuild current version
task build:backend
```

---

## Reference

- **Master Guide:** `D:\Code\shared-docs\MASTER_GUIDE_AGENT_ESSENTIALS.md`
- **Project Docs:** `./README.md`, `./VERSION_HISTORY.md`
- **Build Guide:** `./BUILD.md`
- **Test Results:** `./test-results.xml`

---

# Review

Context for ReAgent (automated PR review system).

## Architecture Overview

WaveMux is an AI-native terminal application built on a **three-tier architecture**:

### Tier 1: Tauri Shell (Rust)
- **Location:** `src-tauri/src/`
- **Role:** Native window management, system tray, menus, crash handling, logging, heartbeat monitoring, and Go sidecar lifecycle management.
- **Key files:** `lib.rs` (app setup, plugin registration, IPC handler registration), `sidecar.rs` (Go backend spawn/communication), `commands/` (Tauri IPC command handlers for platform, auth, window, backend, devtools, RPC bridge), `state.rs` (shared app state), `menu.rs`, `tray.rs`, `crash.rs`, `heartbeat.rs`.
- **Backend modes:** Feature-gated via Cargo -- `go-sidecar` (default, spawns wavemuxsrv) or `rust-backend` (in-process, experimental). See `rust_backend.rs` and `backend/` directory.
- **Tauri plugins:** shell, dialog, notification, clipboard, global-shortcut, fs, opener, process, store, window-state, websocket, single-instance.

### Tier 2: Go Backend Sidecar (`wavemuxsrv`)
- **Location:** `cmd/server/`, `pkg/`
- **Role:** Core business logic, terminal session management, WebSocket/HTTP API, database (SQLite via sqlx), remote connections (SSH), AI integrations (OpenAI, Google Generative AI), file storage, event bus, config management, telemetry, cloud sync.
- **Key packages:** `pkg/wcore/` (core logic), `pkg/waveobj/` (object model), `pkg/wconfig/` (configuration), `pkg/shellexec/` (terminal shell execution), `pkg/remote/` (SSH connections), `pkg/waveai/` (AI chat), `pkg/wshrpc/` (RPC protocol), `pkg/web/` (HTTP handlers), `pkg/wps/` (pub/sub), `pkg/eventbus/`, `pkg/blockcontroller/`, `pkg/service/`.
- **Database:** SQLite with golang-migrate migrations in `db/migrations-wstore/` and `db/migrations-filestore/`.
- **Communication:** Backend announces readiness via stderr (`WAVESRV-ESTART ws:<addr> web:<addr>`), frontend connects to WebSocket/HTTP endpoints. Backend events forwarded via `WAVESRV-EVENT:` prefix.

### Tier 3: Frontend (React/TypeScript)
- **Location:** `frontend/`
- **Role:** Terminal UI, workspace/tab management, AI chat panel, code editor, web previews, block-based layout system.
- **Entry point:** `frontend/tauri-bootstrap.ts` -> `frontend/tauri-init.ts` -> `frontend/wave.ts`
- **State management:** Jotai atoms (`frontend/app/store/global.ts`, `frontend/app/store/jotaiStore.ts`)
- **Key views:** `frontend/app/view/term/` (xterm.js terminal), `frontend/app/view/waveai/` (AI panel), `frontend/app/view/codeeditor/` (Monaco editor), `frontend/app/view/preview/` (file/web preview), `frontend/app/view/chat/`, `frontend/app/view/launcher/`, `frontend/app/view/claudecode/`.
- **RPC layer:** `frontend/app/store/wshrpcutil.ts`, `frontend/app/store/wshclientapi.ts` -- typed RPC client over WebSocket to Go backend.
- **Build:** Vite with React SWC plugin, Tailwind CSS v4, SCSS modules. Config in `vite.config.tauri.ts`.
- **Key dependencies:** xterm.js (terminal emulation), Monaco Editor (code editing), Mermaid (diagrams), react-markdown, recharts, react-dnd, jotai, rxjs, Vercel AI SDK (`ai`, `@ai-sdk/react`).

### Shell Integration (`wsh`)
- **Location:** `cmd/wsh/`
- **Role:** CLI binary deployed to user machines and remote hosts for shell integration, remote RPC, and command execution.
- **Cross-platform builds:** darwin/arm64, darwin/amd64, linux/arm64, linux/amd64, linux/mips, linux/mips64, windows/amd64, windows/arm64.
- **Shell integration scripts:** `shell-integration/` directory.

### Tsunami (Widget Framework)
- **Location:** `tsunami/`
- **Role:** Embeddable widget/app framework with its own Go module, frontend build, scaffold templates, and VDOM engine.
- **Sub-modules:** `tsunami/engine/`, `tsunami/vdom/`, `tsunami/frontend/`, `tsunami/templates/`, `tsunami/prompts/`.

### Infrastructure
- **Location:** `infra/`
- **Role:** AWS CDK deployment (Lambda webhook router), deployment scripts.
- **Config schemas:** `schema/` directory (settings.json, connections.json, widgets.json, aipresets.json).

## Review Checklist

### All PRs

- [ ] Version consistency: changes to `package.json` version must also update `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` (use `bump-version.sh` or `version.cjs`).
- [ ] No hardcoded secrets, auth keys, or API tokens. Auth key is generated at runtime in `state.rs` and passed via environment variable.
- [ ] No direct pushes to main; PR required.
- [ ] Commit messages follow conventional format (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`).

### Rust (src-tauri/) Changes

- [ ] Feature gates used correctly: `#[cfg(feature = "go-sidecar")]` vs `#[cfg(feature = "rust-backend")]` -- these are mutually exclusive backend modes.
- [ ] Tauri IPC commands registered in `lib.rs` `invoke_handler` if new commands are added.
- [ ] Tauri plugin capabilities properly declared in `src-tauri/capabilities/`.
- [ ] No `unwrap()` on user-facing code paths -- use proper error handling with `Result` and `thiserror`.
- [ ] Sidecar spawn/shutdown lifecycle handled correctly (graceful kill on close).
- [ ] Release profile optimizations preserved (`strip`, `lto`, `codegen-units = 1`).
- [ ] CSP in `tauri.conf.json` updated if new external resources are loaded.

### Go Backend (cmd/, pkg/) Changes

- [ ] Database migrations are additive-only (no destructive changes to existing migrations in `db/migrations-*`).
- [ ] RPC methods registered and have corresponding TypeScript bindings (generated via `cmd/generatets/`).
- [ ] CGO dependencies accounted for (SQLite requires CGO_ENABLED=1 with `sqlite_omit_load_extension` tag).
- [ ] Cross-compilation considered: `wsh` builds for 8 platform/arch targets, `wavemuxsrv` builds per-platform.
- [ ] No breaking changes to the `WAVESRV-ESTART` or `WAVESRV-EVENT:` stderr protocol (Rust sidecar.rs parses these).
- [ ] `go.mod` `replace` directives preserved for forked dependencies (ssh_config, pty, tsunami).
- [ ] AI provider integrations (OpenAI, Google) handle API errors gracefully and respect rate limits.

### Frontend (frontend/) Changes

- [ ] Jotai atoms follow existing patterns in `frontend/app/store/global.ts`.
- [ ] New views/blocks registered in the layout system.
- [ ] No direct DOM manipulation -- use React patterns and refs.
- [ ] WebSocket reconnection and error handling maintained in RPC layer.
- [ ] Monaco editor and xterm.js lifecycle cleanup on unmount (no memory leaks).
- [ ] Tailwind CSS v4 used (not v3 `@apply` patterns); check `frontend/tailwindsetup.css`.
- [ ] Large dependencies chunked in `vite.config.tauri.ts` `manualChunks` if added.
- [ ] SCSS modules scoped properly; global styles only in `app.scss`, `reset.scss`, `theme.scss`.

### Build System (Taskfile.yml) Changes

- [ ] Task dependencies chain correctly (e.g., `dev` depends on `npm:install`, `docsite:build:embedded`, `build:backend`).
- [ ] Platform-conditional logic uses correct Taskfile syntax (`{{if eq OS "windows"}}`).
- [ ] Sidecar copy task (`tauri:copy-sidecars`) updated if binary naming changes.
- [ ] Version variable (`VERSION`) sourced from `version.cjs` consistently.

### Infrastructure (infra/) Changes

- [ ] CDK constructs follow existing patterns in `infra/cdk/lib/`.
- [ ] Lambda functions in `infra/lambda/` have proper error handling and logging.
- [ ] No secrets committed -- use AWS Secrets Manager references.

### Testing

- [ ] Unit tests pass: `npm test` (Vitest).
- [ ] E2E tests pass: Playwright tests in `e2e/`.
- [ ] Go tests pass: `go test ./...` from repo root.
- [ ] Version verification: `bash scripts/verify-version.sh`.
