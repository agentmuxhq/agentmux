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
