# WaveMux Go Module Independence Refactor Plan

**Created:** 2025-10-20
**Status:** Planned
**Target Version:** v0.12.17

---

## 🎯 Objective

Make WaveMux truly independent from the upstream Wave Terminal by:
1. Changing Go module path from `github.com/wavetermdev/waveterm` to `github.com/a5af/wavemux`
2. Building all Go binaries (wavemuxsrv + wsh) from source with proper module resolution
3. Ensuring all tests pass and the application runs correctly

---

## 📋 Current State (v0.12.16)

### ✅ Completed
- [x] Renamed all `wavesrv` references to `wavemuxsrv` in TypeScript/JS code
- [x] Renamed `wavesrv` to `wavemuxsrv` in documentation
- [x] Updated electron-builder config to expect `wavemuxsrv.x64.exe`
- [x] Rebranded product name, app ID, and homepage

### ❌ Blockers
- [ ] Go module path still points to `github.com/wavetermdev/waveterm`
- [ ] Cannot build Go binaries from source due to module resolution errors
- [ ] Relying on manually copied binaries from upstream fork

---

## 🔧 Refactor Steps

### Phase 1: Go Module Path Change

**Files to modify:**
```
go.mod                          # Change module path
go.sum                          # Will regenerate
```

**Change:**
```diff
- module github.com/wavetermdev/waveterm
+ module github.com/a5af/wavemux
```

### Phase 2: Update All Import Paths

**Estimate:** ~100-150 import statements across Go files

**Pattern:**
```diff
- import "github.com/wavetermdev/waveterm/pkg/waveobj"
+ import "github.com/a5af/wavemux/pkg/waveobj"
```

**Key directories:**
- `cmd/` - Command entry points
- `pkg/` - Core packages
- `emain/` - Electron main process (TypeScript, may reference Go types)

**Automation:**
```bash
# Find all Go files with old imports
find . -name "*.go" -exec grep -l "github.com/wavetermdev/waveterm" {} \;

# Replace all imports
find . -name "*.go" -exec sed -i 's|github.com/wavetermdev/waveterm|github.com/a5af/wavemux|g' {} +
```

### Phase 3: Update Build Scripts

**Files to modify:**
```
Taskfile.yml                    # Task runner definitions
build-wavesrv.ps1              # Windows build script
build-wavesrv.sh               # Unix build script (if exists)
```

**Key changes:**
- Ensure build scripts reference correct module path
- Update output binary names (`wavesrv.x64.exe` → `wavemuxsrv.x64.exe`)

### Phase 4: Update Documentation

**Files to check:**
```
README.md                      # Build instructions
BUILD.md                       # Detailed build guide
CLAUDE.md                      # Agent development guide
CONTRIBUTING.md                # Contribution guidelines
```

**Update:**
- Module path references
- Build command examples
- Import path examples

### Phase 5: Dependency Resolution

**Tasks:**
```bash
# Clean Go module cache
go clean -modcache

# Tidy dependencies
go mod tidy

# Verify dependencies
go mod verify

# Download dependencies
go mod download
```

**Handle external dependencies:**
- Ensure no transitive dependencies still reference old module
- Check for any hardcoded paths in vendor/ (if used)

### Phase 6: Build from Source

**Build commands:**
```bash
# Build backend server
go build -o dist/bin/wavemuxsrv.x64.exe ./cmd/server

# Build wsh for all platforms
task build:wsh

# Or manually:
GOOS=windows GOARCH=amd64 go build -o dist/bin/wsh-${VERSION}-windows.x64.exe ./cmd/wsh
GOOS=darwin GOARCH=arm64 go build -o dist/bin/wsh-${VERSION}-darwin.arm64 ./cmd/wsh
# ... etc for all platforms
```

### Phase 7: Testing

**Test suite:**
1. **Unit tests:**
   ```bash
   go test ./...
   ```

2. **E2E tests:**
   ```bash
   npm test -- app.e2e.test.ts
   ```

3. **Smoke test:**
   - Build package
   - Extract and run WaveMux.exe
   - Verify:
     - Application launches
     - wave-data directory created
     - Backend server starts
     - UI renders
     - Terminal works
     - Version displays correctly

4. **Integration test:**
   - Run `task dev`
   - Verify hot reload works
   - Test wsh shell integration

### Phase 8: Final Verification

**Checklist:**
- [ ] All Go files compile without errors
- [ ] All tests pass (unit + E2E)
- [ ] `wavemuxsrv.x64.exe` built from source
- [ ] All `wsh-${VERSION}-*` binaries built from source
- [ ] Smoke test passes
- [ ] No references to `github.com/wavetermdev/waveterm` in code
- [ ] No references to `wavesrv` (should be `wavemuxsrv`)
- [ ] Package builds successfully
- [ ] Documentation updated

---

## 🚨 Risk Assessment

### High Risk
- **Module resolution errors:** May need to resolve circular dependencies or missing packages
- **Binary compatibility:** New binaries must be compatible with existing data formats
- **Version tracking:** Ensure version numbers embedded in binaries are correct

### Medium Risk
- **Build system complexity:** Taskfile and build scripts may have hidden dependencies
- **Cross-platform builds:** Need to test on Windows, macOS, Linux
- **Shell integration:** wsh binaries must work across all supported platforms

### Low Risk
- **Documentation updates:** Straightforward find/replace
- **Frontend changes:** Already completed in v0.12.16

---

## 📦 Deliverables

### v0.12.17 Release Artifacts
1. **Binaries (all built from source):**
   - `wavemuxsrv.x64.exe` (Windows)
   - `wavemuxsrv.arm64` (macOS)
   - `wavemuxsrv.x64` (macOS)
   - `wsh-0.12.17-windows.x64.exe`
   - `wsh-0.12.17-windows.arm64.exe`
   - `wsh-0.12.17-darwin.arm64`
   - `wsh-0.12.17-darwin.x64`
   - `wsh-0.12.17-linux.x64`
   - `wsh-0.12.17-linux.arm64`
   - `wsh-0.12.17-linux.mips`
   - `wsh-0.12.17-linux.mips64`

2. **Packages:**
   - `WaveMux-win32-x64-0.12.17.zip` (Windows)
   - `WaveMux-darwin-arm64-0.12.17.zip` (macOS ARM)
   - `WaveMux-darwin-x64-0.12.17.zip` (macOS Intel)
   - `WaveMux-linux-x64-0.12.17.AppImage` (Linux)

3. **Documentation:**
   - Updated BUILD.md with new module path
   - Updated CONTRIBUTING.md with refactored structure
   - Updated README.md with correct import examples

---

## 🛠️ Development Workflow

### For v0.12.16 (Current)
```bash
# Working directory
cd D:/Code/agent-workspaces/agentx/wavemux

# Using pre-built binaries from waveterm fork
# Just rename wavesrv → wavemuxsrv
```

### For v0.12.17 (After Refactor)
```bash
# Clean build from source
task build:backend    # Builds wavemuxsrv + wsh from source
npm run build:prod    # Builds frontend
task package          # Creates distributable

# Development mode
task dev              # Hot reload + live backend
```

---

## 📅 Timeline Estimate

| Phase | Est. Time | Complexity |
|-------|-----------|------------|
| 1. Module path change | 5 min | Low |
| 2. Import path updates | 15 min | Low |
| 3. Build script updates | 10 min | Medium |
| 4. Documentation updates | 10 min | Low |
| 5. Dependency resolution | 20 min | High |
| 6. Build from source | 30 min | High |
| 7. Testing | 45 min | High |
| 8. Final verification | 30 min | Medium |
| **Total** | **~2.5 hours** | **Medium-High** |

---

## ✅ Success Criteria

1. ✅ Module path is `github.com/a5af/wavemux`
2. ✅ All binaries built from source (no copied files)
3. ✅ All tests pass (unit + E2E + smoke)
4. ✅ Application runs without errors
5. ✅ No upstream module references in codebase
6. ✅ Documentation reflects new structure
7. ✅ Release artifacts are properly named and versioned

---

## 🔄 Rollback Plan

If refactor fails:
1. Revert to main branch (v0.12.16 with renamed references)
2. Continue using pre-built binaries temporarily
3. Address blockers identified during refactor
4. Retry with fixes applied

---

## 📝 Notes

- This refactor makes WaveMux **truly independent** from Wave Terminal
- After completion, no upstream dependencies or binaries needed
- All future development happens purely in `github.com/a5af/wavemux`
- Upstream sync becomes a manual merge process (import Wave features as desired)

---

## 🤖 Agent Instructions

When implementing this refactor:

1. **Create feature branch:**
   ```bash
   git checkout -b refactor/go-module-independence
   ```

2. **Follow phases sequentially** - don't skip steps

3. **Test after each phase** - ensure nothing breaks

4. **Document blockers** - write to REFACTOR_BLOCKERS.md if stuck

5. **Commit frequently** with clear messages:
   ```
   refactor(go): update module path to github.com/a5af/wavemux
   refactor(go): replace all import paths
   refactor(build): update Taskfile for wavemuxsrv
   test(e2e): verify all tests pass with new module
   ```

6. **Final PR:** Merge to main only after ALL success criteria met

---

**Status:** ⏳ Ready to implement
**Branch:** `refactor/go-module-independence` (to be created)
**Assignee:** Next available agent
