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
- **Version:** 0.31.0
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
- `task build:backend` - Rust binaries (agentmuxsrv-rs + wsh-rs)
- `task build:frontend` - Frontend only
- `task test` - Run tests
- `task clean` - Clean artifacts

**npm Users:** Can use `npm run <command>` - it delegates to Task.

### After Code Changes

- **TypeScript/React** → Auto-reloads in `task dev` ✅
- **Rust backend** → `task build:backend` then restart `task dev`
- **Test package** → `task package` then extract/install artifact

### Architecture

AgentMux is built on **Tauri v2** with a **100% Rust backend**:

- **agentmux.exe** = Tauri app (Rust + single webview)
- **agentmuxsrv-rs** = Rust backend sidecar (auto-spawned, don't run manually)
- **wsh** = Rust shell integration binary (wsh-rs crate, must be versioned correctly)

**Important:** All Go and Electron code has been removed. Only Rust + Tauri is supported.

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
- `agentmuxsrv-rs/Cargo.toml`
- `wsh-rs/Cargo.toml`
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

**Step 3: Rebuild binaries** (required for Rust binaries)
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

### Tauri Version Management

**CRITICAL:** Tauri versions MUST be synchronized across all packages to prevent build failures.

#### Why This Matters

Tauri consists of **core packages** and **plugins** that must align on the same **major.minor** version:

**Core packages:**
- Rust crate `tauri` (backend) - in `src-tauri/Cargo.toml`
- NPM package `@tauri-apps/cli` (build tool) - in `package.json`
- NPM package `@tauri-apps/api` (frontend API) - in `package.json`

**Plugins (examples):**
- Rust crate `tauri-plugin-shell` + NPM `@tauri-apps/plugin-shell`
- Rust crate `tauri-plugin-fs` + NPM `@tauri-apps/plugin-fs`
- Rust crate `tauri-plugin-opener` + NPM `@tauri-apps/plugin-opener`
- And 8+ more plugins...

**Example of valid alignment:**
- ✅ Core: All on 2.10.x (CLI: 2.10.0, API: 2.10.1, crate: 2.10.2)
- ✅ Plugins: shell on 2.3.x, opener on 2.5.x (npm matches Cargo major.minor)
- ❌ Mix of 2.9.x and 2.10.x (FAILS with version mismatch error)
- ❌ Plugin shell npm 2.2.x but Cargo 2.3.x (FAILS with plugin version mismatch)

#### Before ANY Build

**ALWAYS verify Tauri versions before building:**
```bash
./scripts/verify-tauri-versions.sh
```

Expected output:
```
✅ All Tauri core packages aligned on 2.10.x

🔌 Checking Tauri plugin alignment...
  ✅ plugin-shell: npm 2.3.5, cargo 2.3.5 (2.3.x)
  ✅ plugin-opener: npm 2.5.3, cargo 2.5.3 (2.5.x)
  ✅ plugin-fs: npm 2.4.5, cargo 2.4.5 (2.4.x)
  ✅ plugin-notification: npm 2.3.3, cargo 2.3.3 (2.3.x)

✅ All Tauri packages and plugins aligned!
   Build should succeed!
```

If you see a mismatch error, **DO NOT** proceed with the build - fix versions first!

#### Updating Tauri

**NEVER** manually edit package.json or Cargo.toml for Tauri versions.

**Use the update script:**
```bash
# Update core packages only
./scripts/update-tauri.sh 2.11.0

# Update core packages AND plugins
./scripts/update-tauri.sh 2.11.0 --plugins
```

This automatically:
1. Updates npm packages to exact versions (no ^)
2. Updates Cargo.toml to match major.minor
3. Updates both lock files
4. Optionally updates all plugins (with --plugins flag)
5. Verifies alignment

#### Version Pinning Strategy

**package.json:** Uses exact versions (NO `^` prefix)
```json
"@tauri-apps/cli": "2.10.0",
"@tauri-apps/api": "2.10.1",
"@tauri-apps/plugin-shell": "2.3.5",
"@tauri-apps/plugin-opener": "2.5.3"
```

**Cargo.toml:** Uses `=MAJOR.MINOR` range
```toml
tauri = { version = "=2.10", features = [...] }
tauri-plugin-shell = "=2.3"
tauri-plugin-opener = "=2.5"
```

This allows patch updates (2.10.2 → 2.10.3) but prevents minor version drift (2.10 → 2.11).

**Important:** Both core packages AND plugins must use this pinning strategy to prevent build failures.

#### Troubleshooting

**Build fails with "version mismatch" error:**
1. Run `./scripts/verify-tauri-versions.sh` to see versions
2. Run `./scripts/update-tauri.sh <version>` to fix
3. Commit **both** `package-lock.json` and `Cargo.lock`

**After npm install, versions drift:**
- This means package.json still has `^` prefixes
- Remove `^` and pin exact versions
- Run `npm install` to regenerate lock file

**See:** [docs/RETRO_TAURI_VERSION_MISMATCH.md](docs/RETRO_TAURI_VERSION_MISMATCH.md) for detailed analysis

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

### Backend (Rust)

```bash
# Build all Rust binaries (agentmuxsrv-rs + wsh-rs)
task build:backend

# Build only the backend server
task build:backend:rust

# Build only wsh
task build:wsh
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

# Create portable ZIP (Windows)
task package:portable
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
