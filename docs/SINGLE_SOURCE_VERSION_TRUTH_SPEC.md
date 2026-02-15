# Single Source of Version Truth - Specification

**Date:** 2025-02-12
**Status:** Draft
**Author:** AgentA

## Problem Statement

### Current Version Mismatch Issue

**Observed Behavior:**
- Title bar displays: `AgentMux 0.27.7`
- Context menu/About modal displays: `AgentMux v0.27.8`

**Root Cause:** Multiple independent version sources exist across the codebase, leading to:
1. Inconsistent version display across UI components
2. Manual synchronization burden during version bumps
3. High risk of human error when updating versions
4. Difficult debugging when versions don't match

### Current Version Sources (As-Is Architecture)

After comprehensive analysis, **6 independent version sources** were identified:

| # | Location | Format | Used By | Set Method |
|---|----------|--------|---------|------------|
| 1 | `package.json` | `"version": "0.27.8"` | npm, build scripts | Manual edit |
| 2 | `src-tauri/tauri.conf.json` | `"version": "0.27.8"` | About modal, Tauri bundle | Manual edit |
| 3 | `src-tauri/Cargo.toml` | `version = "0.27.8"` | Window title (`env!("CARGO_PKG_VERSION")`) | Manual edit |
| 4 | `src-tauri/Cargo.lock` | Auto-generated from Cargo.toml | Cargo build system | `cargo update` |
| 5 | `package-lock.json` | Auto-generated from package.json | npm install | `npm install` |
| 6 | `pkg/wavebase/wavebase.go` | `var WaveVersion = "0.0.0"` | Backend runtime, shell integration | Build ldflags |

**Version Flow Paths:**

```
USER BUMPS VERSION
    ↓
┌─────────────────────────────────────────────────────────┐
│ MANUAL EDITS REQUIRED (Current Process)                │
├─────────────────────────────────────────────────────────┤
│ 1. package.json           → "version": "X.Y.Z"         │
│ 2. tauri.conf.json        → "version": "X.Y.Z"         │
│ 3. Cargo.toml             → version = "X.Y.Z"          │
└─────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────┐
│ AUTO-GENERATED (Dependencies)                           │
├─────────────────────────────────────────────────────────┤
│ 4. Cargo.lock             → cargo update -p agentmux   │
│ 5. package-lock.json      → npm install                │
└─────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────┐
│ BUILD-TIME INJECTION                                    │
├─────────────────────────────────────────────────────────┤
│ 6. wavebase.go WaveVersion ← ldflags during build      │
│    - agentmuxsrv: -X main.WaveVersion=X.Y.Z            │
│    - wsh: -X main.WaveVersion=X.Y.Z                    │
└─────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────┐
│ RUNTIME VERSION DISPLAY                                 │
├─────────────────────────────────────────────────────────┤
│ Window Title:     env!("CARGO_PKG_VERSION") [Cargo.toml]│
│ About Modal:      app.config().version [tauri.conf.json]│
│ Context Menu:     getAboutModalDetails() [tauri.conf.json]│
│ Backend Logs:     WaveVersion [ldflags]                │
│ wsh --version:    WaveVersion [ldflags]                │
└─────────────────────────────────────────────────────────┘
```

**Current Issues:**
- ❌ No validation that all sources match
- ❌ Easy to forget updating one file (as seen: Cargo.toml = 0.27.7, tauri.conf.json = 0.27.8)
- ❌ Build doesn't fail if versions mismatch
- ❌ No single command to update all sources atomically

---

## Proposed Solution: Single Source of Truth (SSOT)

### Design Principles

1. **Single Source:** `package.json` version is the **canonical source**
2. **Derived Sources:** All other files read/derive from `package.json`
3. **Build-Time Validation:** Fail build if any version mismatch detected
4. **Atomic Updates:** One command updates all sources consistently
5. **Developer Experience:** No manual file editing for version bumps

### Target Architecture (To-Be)

```
┌─────────────────────────────────────────────────────────┐
│ SINGLE SOURCE OF TRUTH                                  │
│ package.json: "version": "X.Y.Z"                        │
└─────────────────────────────────────────────────────────┘
    ↓
    ↓ (sync-version.sh reads package.json)
    ↓
┌─────────────────────────────────────────────────────────┐
│ AUTO-SYNCED FILES                                       │
├─────────────────────────────────────────────────────────┤
│ 1. tauri.conf.json    ← sync script updates            │
│ 2. Cargo.toml         ← sync script updates            │
│ 3. Cargo.lock         ← cargo update (if needed)       │
└─────────────────────────────────────────────────────────┘
    ↓
    ↓ (build reads package.json)
    ↓
┌─────────────────────────────────────────────────────────┐
│ BUILD PROCESS                                           │
├─────────────────────────────────────────────────────────┤
│ - Validate all versions match package.json             │
│ - Inject version into Go via ldflags                   │
│ - Build with consistent version everywhere             │
└─────────────────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────────────────┐
│ RUNTIME (All sources show same version)                │
└─────────────────────────────────────────────────────────┘
```

---

## Implementation Plan

### Phase 1: Version Sync Script

**File:** `scripts/sync-version.sh`

**Purpose:** Read `package.json` version and update all derived files

**Algorithm:**
```bash
#!/usr/bin/env bash
set -euo pipefail

# 1. Extract version from package.json (SSOT)
VERSION=$(node -p "require('./package.json').version")

# 2. Update tauri.conf.json
jq --arg ver "$VERSION" '.version = $ver' src-tauri/tauri.conf.json > tmp.json
mv tmp.json src-tauri/tauri.conf.json

# 3. Update Cargo.toml
sed -i "s/^version = .*/version = \"$VERSION\"/" src-tauri/Cargo.toml

# 4. Update Cargo.lock (if Cargo.toml changed)
cd src-tauri && cargo update -p agentmux && cd ..

echo "✅ All version files synced to $VERSION"
```

**Usage:**
```bash
# After editing package.json version:
npm version patch  # OR: manually edit package.json
./scripts/sync-version.sh
```

---

### Phase 2: Build Validation

**File:** `scripts/verify-version.sh` (already exists, enhance it)

**Purpose:** Fail build if version mismatch detected

**Checks:**
```bash
#!/usr/bin/env bash
set -euo pipefail

PACKAGE_VERSION=$(node -p "require('./package.json').version")
TAURI_VERSION=$(jq -r '.version' src-tauri/tauri.conf.json)
CARGO_VERSION=$(grep '^version =' src-tauri/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')

if [[ "$PACKAGE_VERSION" != "$TAURI_VERSION" ]]; then
    echo "❌ Version mismatch: package.json ($PACKAGE_VERSION) != tauri.conf.json ($TAURI_VERSION)"
    exit 1
fi

if [[ "$PACKAGE_VERSION" != "$CARGO_VERSION" ]]; then
    echo "❌ Version mismatch: package.json ($PACKAGE_VERSION) != Cargo.toml ($CARGO_VERSION)"
    exit 1
fi

echo "✅ All versions consistent: $PACKAGE_VERSION"
```

**Integration:**
```yaml
# Taskfile.yml
version: '3'

tasks:
  validate:
    desc: Validate version consistency
    cmds:
      - bash scripts/verify-version.sh

  build:backend:
    desc: Build backend binaries
    deps: [validate]  # ← Enforce validation before build
    cmds:
      - go build ...
```

---

### Phase 3: Bump-Version Script Enhancement

**File:** `bump-version.sh` (already exists, modify it)

**Current behavior:**
- Manually edits multiple files
- Prone to missing one file

**New behavior:**
```bash
#!/usr/bin/env bash
set -euo pipefail

# 1. Bump package.json using npm version (atomic, creates git tag)
npm version "$1" --no-git-tag-version -m "$2"

# 2. Auto-sync all derived files
./scripts/sync-version.sh

# 3. Validate everything matches
./scripts/verify-version.sh

# 4. Commit changes
NEW_VERSION=$(node -p "require('./package.json').version")
git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: bump version to $NEW_VERSION

$2"

# 5. Create git tag
git tag -a "v$NEW_VERSION" -m "Version $NEW_VERSION"

echo "✅ Version bumped to $NEW_VERSION"
echo "Next: git push origin main --tags"
```

**Usage:**
```bash
./bump-version.sh patch --message "fix: portable wsh deployment"
# ↑ Updates package.json → syncs all files → validates → commits → tags
```

---

### Phase 4: CI/CD Integration

**File:** `.github/workflows/build.yml` (or similar)

**Pre-Build Step:**
```yaml
- name: Verify Version Consistency
  run: bash scripts/verify-version.sh
```

**Benefits:**
- PR builds fail if version mismatch
- Catches errors before merge
- Forces developer to run sync script

---

## Version Display Audit

### Current Implementation

| Component | Source | Code Location | Current Value |
|-----------|--------|---------------|---------------|
| **Window Title** | `env!("CARGO_PKG_VERSION")` | `src-tauri/src/lib.rs:118` | Cargo.toml |
| **About Modal** | `app.config().version` | `frontend/app/modals/about.tsx:32` | tauri.conf.json |
| **Context Menu** | `getAboutModalDetails()` | `frontend/app/menu/base-menus.ts:14` | tauri.conf.json |
| **Backend Logs** | `WaveVersion` | `pkg/wavebase/wavebase.go:25` | Build ldflags |
| **wsh --version** | `WaveVersion` | `cmd/wsh/main-wsh.go` | Build ldflags |

### Proposed Changes

**Option A: All read from Tauri config (Recommended)**
- ✅ Tauri provides canonical version via `app.config().version`
- ✅ Rust can access via `app.package_info().version`
- ✅ Frontend already has API: `getAboutModalDetails()`
- ❌ Go backend can't easily access Tauri config

**Option B: All read from package.json at build time**
- ✅ True single source (package.json)
- ✅ Works for all languages (read at build time)
- ❌ Requires passing version to all build steps

**Recommended: Hybrid Approach**
1. **SSOT:** `package.json` version
2. **Sync:** All files synced from package.json before build
3. **Display:** Each runtime uses its natural source (Tauri config for Rust/TS, ldflags for Go)
4. **Validation:** Build fails if any source doesn't match package.json

---

## Migration Plan

### Step 1: Add Sync Script
```bash
# Create scripts/sync-version.sh
# Test it manually
./scripts/sync-version.sh
git diff  # Verify correct files updated
```

### Step 2: Update Taskfile
```yaml
# Add validation task
tasks:
  validate:version:
    desc: Verify version consistency
    cmds:
      - bash scripts/verify-version.sh
```

### Step 3: Update bump-version.sh
```bash
# Modify to call sync-version.sh
# Test with dry-run mode
```

### Step 4: Add CI Check
```yaml
# .github/workflows/verify-version.yml
name: Verify Version
on: [pull_request, push]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: bash scripts/verify-version.sh
```

### Step 5: Document in README
```markdown
## Version Management

AgentMux uses a single source of truth for versioning:
- **Source:** `package.json` version field
- **Sync:** Run `./scripts/sync-version.sh` to update derived files
- **Bump:** Use `./bump-version.sh patch|minor|major --message "..."`
```

---

## Testing Strategy

### Manual Testing
```bash
# 1. Edit package.json version to 0.99.0
sed -i 's/"version": ".*"/"version": "0.99.0"/' package.json

# 2. Run sync script
./scripts/sync-version.sh

# 3. Verify all files updated
grep -r "0.99.0" package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml

# 4. Build and check displayed versions
task build:backend
./dist/bin/wsh-0.99.0-windows.x64.exe --version  # Should show 0.99.0

task package
# Extract portable, run agentmux.exe
# - Window title should show "AgentMux 0.99.0"
# - About modal should show "Client Version 0.99.0"
# - Context menu should show "AgentMux v0.99.0"

# 5. Revert changes
git checkout package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
```

### Automated Testing
```bash
# Add to test suite
test_version_consistency() {
    ./scripts/verify-version.sh || fail "Version mismatch detected"
}
```

---

## Success Criteria

### Must-Have (MVP)
- ✅ Single command to update all version files
- ✅ Build fails if version mismatch detected
- ✅ All UI components show same version
- ✅ Documentation updated with new workflow

### Should-Have (V2)
- ✅ CI validates version consistency on every PR
- ✅ bump-version.sh creates git tags automatically
- ✅ Version displayed in error reports/logs

### Nice-to-Have (Future)
- ✅ Pre-commit hook runs verify-version.sh
- ✅ Version changelog auto-generated
- ✅ Release notes template includes version

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Developer forgets to run sync script | Medium | Add to Taskfile as pre-build dependency |
| Sync script has bugs | High | Extensive testing, dry-run mode |
| CI breaks due to version check | Medium | Make check optional initially (warn only) |
| Go backend can't access package.json | Low | Continue using ldflags (already validated) |

---

## Alternatives Considered

### Alternative 1: Use Tauri.conf.json as SSOT
- ❌ Not at repo root (less discoverable)
- ❌ Tauri-specific (couples build to Tauri)
- ❌ npm version command doesn't work

### Alternative 2: Use Cargo.toml as SSOT
- ❌ Rust-specific (frontend devs may not know Cargo)
- ❌ Requires TOML parsing in JS build scripts
- ❌ Not standard for Node projects

### Alternative 3: Hardcode version in single .VERSION file
- ✅ Very simple, language-agnostic
- ❌ Breaks npm version command
- ❌ Not standard practice
- ❌ Requires custom tooling

### Alternative 4: Runtime version fetching (query package.json at runtime)
- ❌ Adds I/O overhead
- ❌ Requires bundling package.json in builds
- ❌ Doesn't work for compiled binaries (wsh)

**Selected: package.json as SSOT** (standard for Node projects, works with npm version, at repo root)

---

## References

- Current version bump script: `bump-version.sh`
- Current verification script: `scripts/verify-version.sh`
- Build system: `Taskfile.yml`
- Tauri config: `src-tauri/tauri.conf.json`
- Backend version injection: `Taskfile.yml` task `build:server:internal` (ldflags)
- Frontend version API: `frontend/util/tauri-api.ts:84` (`get_about_modal_details`)

---

## Appendix: Complete File Change Matrix

| Action | File | Before | After | Tool |
|--------|------|--------|-------|------|
| Bump | `package.json` | `"version": "0.27.7"` | `"version": "0.27.8"` | `npm version` |
| Sync | `src-tauri/tauri.conf.json` | `"version": "0.27.7"` | `"version": "0.27.8"` | `jq` |
| Sync | `src-tauri/Cargo.toml` | `version = "0.27.7"` | `version = "0.27.8"` | `sed` |
| Auto | `src-tauri/Cargo.lock` | (auto-generated) | (auto-updated) | `cargo update` |
| Auto | `package-lock.json` | (auto-generated) | (auto-updated) | `npm install` |
| Build | `wavebase.go::WaveVersion` | `"0.0.0"` → `"0.27.8"` | (runtime) | Build ldflags |

---

**End of Specification**
