# Build System Standardization Proposal

**Status:** Proposed
**Created:** 2026-02-12
**Priority:** High
**Complexity:** Medium

---

## Problem Statement

AgentMux currently has **four overlapping build systems** with unclear ownership and confusing entry points:

1. **npm scripts** (package.json) - Frontend builds, some wrappers
2. **Taskfile** (Taskfile.yml) - Main orchestration, backend builds
3. **PowerShell scripts** (scripts/) - Packaging, utilities
4. **Bash scripts** (repo root) - Version management

### Current Confusion

**Developer Questions:**
- "Should I run `npm run build` or `task package`?"
- "What's the difference between `npm run tauri:build` and `task package`?"
- "Why do some npm scripts call Task, but others don't?"
- "Which command builds everything?"

**Build Path Ambiguity:**
```
User wants to build
    ↓
    ├─→ npm run build? (frontend only)
    ├─→ task package? (everything)
    ├─→ npm run tauri:build? (Tauri only)
    └─→ scripts/build-release.ps1? (also everything?)
```

---

## Current State Analysis

### 1. npm Scripts (package.json)

**Purpose:** Node.js ecosystem integration
**Current commands:**
```json
{
  "scripts": {
    "dev": "task dev",                    // Wrapper → Task
    "build": "vite build",                // Direct Vite
    "build:dev": "vite build --mode dev", // Direct Vite
    "build:prod": "vite build --mode production", // Direct Vite
    "tauri": "tauri",                     // Tauri CLI
    "tauri:dev": "task dev",              // Wrapper → Task
    "tauri:build": "tauri build",         // Direct Tauri
    "package:portable": "task package:portable" // Wrapper → Task
  }
}
```

**Issues:**
- ✅ Some wrap Task (`dev`, `tauri:dev`)
- ❌ Some don't (`build`, `tauri:build`)
- ❌ Inconsistent - hard to know what each does
- ❌ `npm run build` doesn't build backend

### 2. Taskfile (Taskfile.yml)

**Purpose:** Cross-platform build orchestration
**Current commands:**
```yaml
dev:              # Development mode (hot reload)
start:            # Standalone mode (no installer)
package:          # Production installer
package:portable: # Installer + portable ZIP
build:backend:    # Go binaries (all platforms)
build:frontend:   # Frontend only
build:server:     # Backend server only
build:wsh:        # Shell integration binaries
```

**Status:**
- ✅ Most comprehensive
- ✅ Handles all platforms
- ✅ Proper dependency management
- ✅ Already primary build system
- ❌ Not discoverable (users try npm first)

### 3. PowerShell Scripts (scripts/)

**Purpose:** Platform-specific utilities
**Current files:**
```
scripts/
├── package-portable.ps1   # Called by Task ✅
├── build-release.ps1      # Standalone ❌ (duplicate?)
└── verify-version.sh      # Called by Task ✅
```

**Issues:**
- ✅ `package-portable.ps1` - Used by Task
- ❌ `build-release.ps1` - Duplicates `task package`?
- ✅ `verify-version.sh` - Used by Task

### 4. Bash Scripts (repo root)

**Purpose:** Version management
**Current files:**
```
./bump-version.sh       # Standalone version bumping
./bump-version-osx.sh   # macOS variant
```

**Status:**
- ✅ Standalone tools (not build system)
- ✅ Appropriate for repo root

---

## Proposed Solution

### Architecture: Task as Primary

```
┌────────────────────────────────────────────┐
│  Primary Entry Point: Task                 │
│  $ task <command>                          │
└──────────────┬─────────────────────────────┘
               │
               ├──→ npm (frontend builds only)
               ├──→ go build (backend binaries)
               ├──→ cargo/tauri (Rust/Tauri)
               └──→ scripts/*.ps1 (utilities)
```

### 1. Taskfile.yml - Primary Orchestration

**Keep as main build system:**

```yaml
# User-facing commands (documented in README)
dev:              # Start development mode
package:          # Build production installer
package:portable: # Build installer + portable ZIP
build:backend:    # Build Go binaries only
build:frontend:   # Build frontend only
test:             # Run all tests
clean:            # Clean build artifacts

# Internal tasks (called by others)
build:server:     # Internal - build agentmuxsrv
build:wsh:        # Internal - build wsh
tauri:copy-sidecars: # Internal - copy to Tauri
```

**Benefits:**
- Cross-platform (Windows, macOS, Linux)
- Dependency management built-in
- Parallel execution support
- Clear task hierarchy

### 2. package.json - Thin Wrappers

**Make npm scripts delegate to Task:**

```json
{
  "scripts": {
    "dev": "task dev",
    "build": "task build:frontend",
    "build:backend": "task build:backend",
    "package": "task package",
    "test": "vitest",
    "tauri": "tauri"
  }
}
```

**Rationale:**
- npm users can still use `npm run <command>`
- All commands route through Task
- Consistent behavior regardless of entry point
- `npm test` stays npm-native (vitest)
- `npm run tauri` for direct Tauri CLI access

### 3. scripts/ - Internal Utilities Only

**Move all scripts under Task control:**

```
scripts/
├── package-portable.ps1   # Keep - called by task package:portable
├── verify-version.sh      # Keep - called by task package
└── build-release.ps1      # REMOVE - duplicate of task package
```

**Rule:** Scripts in `scripts/` should **never** be run directly by users.

### 4. Repo Root Scripts - Standalone Tools

**Keep for version management:**

```
./bump-version.sh       # Keep - standalone tool
./bump-version-osx.sh   # Keep - platform variant
```

**Rule:** Only version management tools in repo root.

---

## Migration Plan

### Phase 1: Audit & Document (1 hour)

1. Identify all duplicate functionality
2. Document what each command currently does
3. Create mapping: old command → new command

### Phase 2: Update npm Scripts (30 min)

**Before:**
```json
{
  "build": "vite build",
  "build:prod": "vite build --mode production",
  "tauri:build": "tauri build"
}
```

**After:**
```json
{
  "build": "task build:frontend",
  "package": "task package",
  "dev": "task dev"
}
```

### Phase 3: Remove Duplicates (30 min)

1. **Remove:** `scripts/build-release.ps1` (duplicate of `task package`)
2. **Keep:** `scripts/package-portable.ps1` (called by Task)
3. **Keep:** `scripts/verify-version.sh` (called by Task)

### Phase 4: Update Documentation (1 hour)

**Update these files:**
- `README.md` - Build instructions
- `CLAUDE.md` - Developer guide
- `docs/BUILD_SYSTEM_SPEC.md` - If exists
- `CONTRIBUTING.md` - If exists

**New section for README.md:**

```markdown
## Building

AgentMux uses [Task](https://taskfile.dev/) for build orchestration.

### Quick Start

```bash
# Development mode (hot reload)
task dev

# Production build (installer)
task package

# Portable build (ZIP)
task package:portable
```

### Available Commands

| Command | Description |
|---------|-------------|
| `task dev` | Start development mode with hot reload |
| `task package` | Build production installer |
| `task package:portable` | Build installer + portable ZIP |
| `task build:backend` | Build Go binaries only |
| `task build:frontend` | Build frontend only |
| `task test` | Run all tests |
| `task clean` | Clean build artifacts |

### npm Users

If you prefer npm:

```bash
npm run dev       # → task dev
npm run package   # → task package
npm test          # → vitest (native)
```
```

### Phase 5: Add Deprecation Warnings (Optional)

Add warnings to deprecated scripts:

```powershell
# scripts/build-release.ps1
Write-Warning "DEPRECATED: Use 'task package' instead"
Write-Host "This script will be removed in v0.27.0"
exit 1
```

---

## Benefits

### For Developers

✅ **Single source of truth:** Always use `task <command>`
✅ **Discoverability:** `task --list` shows all commands
✅ **Consistency:** Same behavior on all platforms
✅ **Speed:** Parallel builds, proper caching

### For CI/CD

✅ **Reliability:** One build system to test
✅ **Maintenance:** Update Taskfile, not 4 systems
✅ **Clarity:** Logs show clear task hierarchy

### For Documentation

✅ **Simple:** Document Task commands only
✅ **Accurate:** No ambiguity about what builds what
✅ **Maintainable:** Update one place

---

## Risks & Mitigations

### Risk 1: npm Users Expect npm Scripts

**Risk:** Developers used to `npm run build` get confused

**Mitigation:**
- Keep npm scripts as thin wrappers
- Add clear README section
- `npm run build` still works, just delegates

### Risk 2: Breaking CI/CD

**Risk:** Existing CI workflows break

**Mitigation:**
- Audit all GitHub Actions workflows first
- Add both old and new commands during transition
- Test on a feature branch first

### Risk 3: Windows-Specific Scripts

**Risk:** Some scripts are PowerShell-only

**Mitigation:**
- Task handles platform detection
- Keep PowerShell scripts for Windows-only tasks
- Document platform requirements

---

## Success Criteria

1. ✅ All builds go through Task
2. ✅ npm scripts are thin wrappers only
3. ✅ No duplicate build scripts
4. ✅ README shows Task commands
5. ✅ CI/CD uses Task commands
6. ✅ Developers know which command to run

---

## Timeline

**Total Effort:** ~4 hours

| Phase | Time | Owner |
|-------|------|-------|
| Audit & Document | 1h | Dev |
| Update npm Scripts | 30m | Dev |
| Remove Duplicates | 30m | Dev |
| Update Documentation | 1h | Dev |
| Test All Workflows | 1h | Dev |

---

## References

- [Taskfile Documentation](https://taskfile.dev/)
- [npm Scripts Best Practices](https://docs.npmjs.com/cli/v10/using-npm/scripts)
- Current: `Taskfile.yml`
- Current: `package.json`

---

## Appendix: Command Mapping

### Current → Proposed

| Current Command | Proposed | Notes |
|----------------|----------|-------|
| `npm run build` | `task build:frontend` | Frontend only |
| `npm run build:prod` | `task build:frontend` | Same as above |
| `npm run tauri:build` | `task package` | Full build |
| `task dev` | `task dev` | ✅ Already correct |
| `task package` | `task package` | ✅ Already correct |
| `scripts/build-release.ps1` | **REMOVE** | Use `task package` |
| `./bump-version.sh` | `./bump-version.sh` | ✅ Keep as-is |

### Quick Reference Card

```
Want to...                    Run this:
────────────────────────────  ───────────────────
Develop locally               task dev
Build for testing             task package
Build portable version        task package:portable
Build backend only            task build:backend
Build frontend only           task build:frontend
Run tests                     task test
Clean build artifacts         task clean
Bump version                  ./bump-version.sh patch
```
