# Claude Agent Development Guide - AgentMux

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

- **Name:** AgentMux
- **GitHub:** https://github.com/a5af/agentmux
- **Type:** Tauri v2 terminal application
- **Version:** 0.27.10
- **Build System:** Task (Taskfile.yml)

## Git & Pull Requests

- Push branches to https://github.com/a5af/agentmux
- Open PRs against a5af/agentmux main branch
- Branch naming: `agent[X]/feature-name` (e.g., `agentx/fix-version`)

---

## Development Workflow

### Commands (Use Correctly!)

| Command | Use When | Auto-Updates? |
|---------|----------|---------------|
| `task dev` | **Development** (normal work) | ✅ Yes - hot reload |
| `task start` | Standalone testing (rare) | ❌ No |
| `task package` | **Final release builds ONLY** | ❌ No |

**CRITICAL:** Never launch from `make/` during development - it's stale and will crash with "agentmuxsrv.x64.exe ENOENT"

### Build System

**Primary:** Task (Taskfile.yml)
- All builds go through `task <command>`
- npm scripts are thin wrappers that delegate to Task
- Run `task --list` to see all available commands

**Common Tasks:**
- `task dev` - Development mode
- `task package` - Production installer
- `task package:portable` - Portable ZIP
- `task build:backend` - Go binaries only
- `task build:frontend` - Frontend only
- `task test` - Run tests
- `task clean` - Clean artifacts

**npm Users:** Can use `npm run <command>` - it delegates to Task.

### After Code Changes

- **TypeScript/React** → Auto-reloads in `task dev` ✅
- **Go backend** → `task build:backend` then restart `task dev`
- **Test package** → `task package` then extract/install artifact

### Architecture

AgentMux is built on **Tauri v2** (NOT Electron):

- **agentmux.exe** = Tauri app (Rust + single webview)
- **agentmuxsrv** = Go backend sidecar (auto-spawned, don't run manually)
- **wsh** = Shell integration binary (must be versioned correctly)

**Important:** All Electron code has been removed (Phase 14). Only Tauri is supported.

---

## Version Management

**CRITICAL:** Always use the versioning scripts - never manually edit version numbers.

**See [README.md](README.md) for complete guide.**

### Mandatory Workflow

**Step 1: Bump version** (updates ALL files automatically)
```bash
./bump-version.sh patch --message "Description"
# OR
./bump-version.sh minor --message "Description"
# OR
./bump-version.sh major --message "Description"
```

This script updates:
- `package.json`
- `package-lock.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src-tauri/tauri.conf.json`
- `cmd/server/main-server.go` (ExpectedVersion constant)
- `VERSION_HISTORY.md`

**Step 2: Verify consistency** (ALWAYS run after bump)
```bash
bash scripts/verify-version.sh
```

Expected output:
```
✓ package-lock.json: X.Y.Z
✓ version.cjs: X.Y.Z
✓ VERSION_HISTORY.md contains X.Y.Z-fork
✓ All version checks passed!
```

**Step 3: Rebuild binaries** (required for Go binaries)
```bash
task build:backend
```

**Step 4: Push with tags**
```bash
git push origin <branch> --tags
```

### Common Issues

❌ **WRONG:** Manually editing version in `package.json`
- Results in version mismatches across files
- Breaks builds with "ExpectedVersion" errors
- Causes shell integration failures

✅ **RIGHT:** Always use `./bump-version.sh`
- Ensures all files stay synchronized
- Automatically updates VERSION_HISTORY.md
- Creates git tag

### Verification Failures

If `verify-version.sh` reports errors:

1. **Version mismatch** → Re-run `./bump-version.sh`
2. **Missing binaries** → Run `task build:backend`
3. **Outdated references** → Update code references manually

**Current version:** See [VERSION_HISTORY.md](VERSION_HISTORY.md)

---

## Agent Workspace Pattern

### Bare Repository

- **Location:** `D:\Code\projects\agentmux.git`
- **Type:** Git bare repository (no working directory)
- **Remote:** https://github.com/a5af/agentmux

### Agent Worktrees

- **Location:** `D:\Code\agent-workspaces\agent[X]\agentmux\`
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
# Build all binaries (agentmuxsrv, wsh for all platforms)
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
