# Rebrand Spec: WaveMux → AgentMux + Cleanup

## Overview

Three workstreams executed as sequential PRs:

1. **PR 1: Electron cleanup** — Remove dead Electron code, fix broken imports
2. **PR 2: AgentBus rename** — Rename the comms system from "agentmux" to "agentbus"
3. **PR 3: AgentMux rebrand** — Rename the app from "WaveMux" to "AgentMux"

Order matters: cleanup first (reduces diff noise), then agentbus (smaller scope), then the big app rename last.

---

## PR 1: Electron Cleanup

Remove dead Electron-era code that no longer serves any purpose.

### Changes

**Go — Remove ElectronRoute dead code:**
- `pkg/wshutil/wshrouter.go` — delete `ElectronRoute = "electron"` constant
- `pkg/web/ws.go` — remove ElectronRoute references
- `cmd/wsh/cmd/wshcmd-notify.go` — replace `wshutil.ElectronRoute` with appropriate Tauri route
- `cmd/wsh/cmd/wshcmd-version.go` — same
- `cmd/wsh/cmd/wshcmd-web.go` — same
- `pkg/wcore/window.go` — same

**Frontend — Fix broken imports and remove compat layer:**
- `vitest.config.ts` — remove import of non-existent `electron.vite.config.ts`
- `eslint.config.js` — remove references to `emain/emain.ts` and `electron.vite.config.ts`
- `frontend/types/electron-compat.d.ts` — delete file (dead Electron.Point/Rectangle stubs)
- `frontend/tauri-init.ts` — remove "Running in Electron mode" fallback branch

**E2E tests — Remove broken Electron tests:**
- `e2e/test-close-button.ts` — delete (imports `_electron` from Playwright)
- `e2e/test-agent-debug.ts` — delete
- `e2e/debug-launch.ts` — delete
- `e2e/close-button.test.ts` — delete

**Docs — Update stale architecture refs:**
- `.roo/rules/overview.md` — update to describe Tauri architecture
- `.roo/rules/rules.md` — remove "Electron application" references
- `CONTRIBUTING.md` — remove `emain/` directory description

### Verification
- `cargo check --features go-sidecar`
- `go build ./cmd/server/...` && `go build ./cmd/wsh/...`
- `npm run build:prod` (frontend compiles)

---

## PR 2: AgentBus Rename (Comms System)

Rename the inter-agent communication system from "agentmux" to "agentbus". The external service URL (`agentmux.asaf.cc`) stays as-is for now — only internal naming changes.

### Changes

**Go backend:**
- `pkg/reactive/poller.go`:
  - `AgentMuxConfigFile` → `AgentBusConfigFile`
  - `AgentMuxConfigFileName` → `AgentBusConfigFileName` (`"agentbus.json"`)
  - `LoadAgentMuxConfigFile()` → `LoadAgentBusConfigFile()`
  - `SaveAgentMuxConfigFile()` → `SaveAgentBusConfigFile()`
  - `AgentMuxURL`/`AgentMuxToken` fields → `AgentBusURL`/`AgentBusToken`
  - Private fields: `agentmuxURL`/`agentmuxToken` → `agentbusURL`/`agentbusToken`
- `pkg/reactive/httphandler.go`:
  - `validateAgentMuxURL()` → `validateAgentBusURL()`
  - Request/response field names: `agentmux_url`/`agentmux_token` → `agentbus_url`/`agentbus_token`
- `cmd/wsh/cmd/wshcmd-agentmux.go` → rename file to `wshcmd-agentbus.go`:
  - `agentmuxCmd` → `agentbusCmd`
  - `agentmuxConfigCmd` → `agentbusConfigCmd`
  - `agentmuxStatusCmd` → `agentbusStatusCmd`
  - `Use: "agentmux"` → `Use: "agentbus"`
  - All function names and messages

**Rust backend:**
- `src-tauri/src/backend/reactive.rs`:
  - `AgentMuxConfigFile` → `AgentBusConfigFile`
  - `agentmux_url`/`agentmux_token` → `agentbus_url`/`agentbus_token`
  - `validate_agentmux_url()` → `validate_agentbus_url()`
- `src-tauri/src/commands/rpc.rs`:
  - Parameter names: `agentmux_url`/`agentmux_token` → `agentbus_url`/`agentbus_token`

**Frontend:**
- `frontend/util/tauri-rpc.ts`:
  - Parameter names in `reactivePollerConfig()`: `agentmuxUrl`/`agentmuxToken` → `agentbusUrl`/`agentbusToken`

**Config migration:**
- Add migration logic: if `agentmux.json` exists but `agentbus.json` doesn't, rename it

**Docs:**
- Update all spec files in `docs/specs/` that reference "agentmux" as the comms system
- Update `CLAUDE.md` terminology table

### Verification
- `cargo check --features go-sidecar`
- `go build ./cmd/wsh/...`
- `wsh agentbus status` works
- `wsh agentbus config <url> <token>` works

---

## PR 3: AgentMux Rebrand (App Name)

Rename the application from "WaveMux" to "AgentMux".

### Phase 3a: Configuration & Metadata
- `package.json`: `name`, `productName`, `build.appId` → `agentmux`, `AgentMux`, `com.a5af.agentmux`
- `src-tauri/Cargo.toml`: `name = "agentmux"`, update description
- `src-tauri/tauri.conf.json`: `productName`, `identifier`, window `title`
- `go.mod`: `module github.com/a5af/agentmux` (if repo rename happens) OR keep as-is if repo stays `wavemux`

### Phase 3b: Binary Names
- `wavemuxsrv` → `agentmuxsrv` (Go sidecar)
- Update `src-tauri/tauri.conf.json` `externalBin`
- Update `Taskfile.yml` all binary references
- Update `build-wavesrv.ps1` → `build-agentmuxsrv.ps1`
- Update CI workflow binary build paths
- Update `src-tauri/src/sidecar.rs` binary name parsing

### Phase 3c: Go Module Path (if repo renames)
- `go.mod` module path
- All 200+ Go import statements (sed/find-replace)
- `go.sum` regenerated via `go mod tidy`
- All `replace` directives in `go.mod` updated

### Phase 3d: Rust Crate
- `Cargo.toml` `name` and `lib.name`
- `Cargo.lock` auto-regenerated
- Any internal `wavemux_lib::` references

### Phase 3e: Frontend & Shell Integration
- Window title in `lib.rs`
- `shell-integration/wavemux-agent.sh` → `agentmux-agent.sh`
- Shell integration scripts in `pkg/util/shellutil/shellintegration/`
- E2E tests referencing binary names

### Phase 3f: Infrastructure
- `infra/cdk/lib/wavemux-webhook-stack.ts` → rename references
- Lambda handler references
- CDK test file

### Phase 3g: Documentation
- `README.md`, `BUILD.md`, `CONTRIBUTING.md`, `CLAUDE.md`
- All docs in `docs/`
- `VERSION_HISTORY.md`

### Decision: GitHub Repo Rename?
If the GitHub repo stays `a5af/wavemux`:
- Go module path stays `github.com/a5af/wavemux` (avoids 200+ file import changes)
- Only user-facing names change (binary, window title, package metadata)
- Much smaller diff

If the GitHub repo renames to `a5af/agentmux`:
- Full Go module path rename (200+ files)
- GitHub handles redirects for old URL
- Cleaner but much larger change

**Recommendation:** Keep repo as `wavemux` for now, rename user-facing strings only. Repo rename can be a follow-up.

### Verification
- `cargo check` (both features)
- `go build ./...`
- `task dev` — window title shows "AgentMux"
- `task package` — installer named AgentMux
- Binary names correct in dist/

---

## Execution Order

```
PR 1: Electron cleanup          (small, safe, no renames)
PR 2: AgentBus rename           (medium, comms-only scope)
PR 3: AgentMux rebrand          (large, app-wide rename)
```

Each PR merged before starting the next. Version bump after PR 3.
