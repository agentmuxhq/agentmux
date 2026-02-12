# Version Verification & Binary Cache Management Specification

## Problem Statement

### Issue Discovered: 2026-02-12

During development of orphaned layout prevention (PR #270), the dev build loaded a **cached wavemuxsrv v0.24.3** binary instead of the newly built **v0.24.11**, causing the migration code to not run despite successful backend compilation.

**Root Cause:**
Tauri dev builds cache sidecar binaries in `src-tauri/target/debug/` and `src-tauri/target/release/`, which are NOT automatically updated when `task build:backend` rebuilds `dist/bin/wavemuxsrv.x64.exe`.

### Impact

- **Development**: Features appear not to work despite code being correct
- **Testing**: Developers test old code unknowingly
- **Debugging**: Hours wasted investigating "bugs" that are actually stale binaries
- **CI/CD**: Build artifacts may have mismatched versions

### Evidence

```bash
# Backend build output
task: [build:server:internal] ... -X main.WaveVersion=0.24.11

# But runtime shows
[wavemuxsrv] wave version: 0.24.3 (202602120125)
```

## Architecture Analysis

### Binary Locations

```
agentmux/
├── dist/bin/
│   └── wavemuxsrv.x64.exe          # Built by task build:backend
├── src-tauri/target/
│   ├── debug/
│   │   └── wavemuxsrv.exe          # ⚠️ CACHED - Used by task dev
│   └── release/
│       └── wavemuxsrv.exe          # ⚠️ CACHED - Used by packaged builds
```

### Current Build Flow

```
task build:backend
  → Builds dist/bin/wavemuxsrv.x64.exe (v0.24.11)

task dev
  → Starts Tauri dev server
  → Tauri spawns sidecar from src-tauri/target/debug/wavemuxsrv.exe (v0.24.3 ❌)
  → Never copies from dist/bin!
```

### Version Variables

| Component | Version Source | Where Set |
|-----------|----------------|-----------|
| **Frontend** | package.json | ./bump-version.sh |
| **Tauri** | tauri.conf.json | ./bump-version.sh |
| **Rust** | Cargo.toml | ./bump-version.sh |
| **Go Backend** | -X main.WaveVersion | build command |
| **wsh CLI** | -X main.WaveVersion | build command |

## Solution Design

### Phase 1: Automatic Binary Sync (Immediate)

**Modify `task dev` to copy binaries before starting:**

```yaml
# Taskfile.yml
dev:
  desc: Run Tauri development server with hot reload
  deps:
    - build:backend
  cmds:
    # Copy fresh backend binary to Tauri cache
    - task: sync:dev:binaries
    - npm run dev

sync:dev:binaries:
  desc: Sync built binaries to Tauri dev cache
  cmds:
    - powershell Copy-Item -Force dist/bin/wavemuxsrv.x64.exe src-tauri/target/debug/wavemuxsrv.exe
    - echo "✓ Synced wavemuxsrv to Tauri dev cache"
  platforms: [windows]

sync:dev:binaries:
  desc: Sync built binaries to Tauri dev cache
  cmds:
    - cp -f dist/bin/wavemuxsrv.* src-tauri/target/debug/
    - echo "✓ Synced wavemuxsrv to Tauri dev cache"
  platforms: [linux, darwin]
```

**Pros:**
- ✅ Simple, immediate fix
- ✅ Works with existing workflow
- ✅ No code changes needed

**Cons:**
- ⚠️ Doesn't prevent manual builds from getting stale
- ⚠️ No verification that sync actually worked

### Phase 2: Version Verification (Safety Net)

**Add runtime version check at server startup:**

```go
// cmd/server/main-server.go

const ExpectedVersion = "0.24.11" // Auto-updated by bump-version.sh

func main() {
	log.SetFlags(log.LstdFlags | log.Lmicroseconds)
	log.SetPrefix("[wavemuxsrv] ")

	// Verify version consistency
	if WaveVersion != ExpectedVersion {
		log.Printf("========================================")
		log.Printf("⚠️  VERSION MISMATCH DETECTED")
		log.Printf("========================================")
		log.Printf("Expected: %s", ExpectedVersion)
		log.Printf("Actual:   %s", WaveVersion)
		log.Printf("BuildTime: %s", BuildTime)
		log.Printf("")
		log.Printf("This likely means:")
		log.Printf("  1. Stale binary in src-tauri/target/")
		log.Printf("  2. Binary not rebuilt after version bump")
		log.Printf("")
		log.Printf("To fix:")
		log.Printf("  task build:backend")
		log.Printf("  task sync:dev:binaries")
		log.Printf("========================================")
		// Continue anyway in dev, but log prominently
	}

	wavebase.WaveVersion = WaveVersion
	wavebase.BuildTime = BuildTime
	// ... rest of main
}
```

**Update bump-version.sh to set ExpectedVersion:**

```bash
# bump-version.sh

# Update Go backend expected version
sed -i "s/const ExpectedVersion = .*/const ExpectedVersion = \"$NEW_VERSION\"/" \
  cmd/server/main-server.go
```

**Pros:**
- ✅ Catches version mismatches at runtime
- ✅ Clear error message with fix instructions
- ✅ Doesn't break builds, just warns

**Cons:**
- ⚠️ Requires code change + version bump script update

### Phase 3: Pre-Run Version Check (Prevention)

**Add version check to `task dev` before starting:**

```yaml
# Taskfile.yml
dev:
  desc: Run Tauri development server with hot reload
  deps:
    - build:backend
  cmds:
    - task: check:versions
    - task: sync:dev:binaries
    - npm run dev

check:versions:
  desc: Verify all version files are consistent
  cmds:
    - bash scripts/verify-version.sh --strict

# New flag for verify-version.sh
```

**Enhance `scripts/verify-version.sh`:**

```bash
#!/bin/bash
# scripts/verify-version.sh

STRICT_MODE=false
if [[ "$1" == "--strict" ]]; then
  STRICT_MODE=true
fi

# ... existing version checks ...

# Check binary versions
if [ -f "dist/bin/wavemuxsrv.x64.exe" ]; then
  BINARY_VERSION=$(strings dist/bin/wavemuxsrv.x64.exe | grep -o "wave version: [0-9.]*" | head -1 | cut -d' ' -f3)
  if [ "$BINARY_VERSION" != "$EXPECTED_VERSION" ]; then
    echo "❌ wavemuxsrv binary version mismatch"
    echo "   Expected: $EXPECTED_VERSION"
    echo "   Binary:   $BINARY_VERSION"
    if [ "$STRICT_MODE" = true ]; then
      exit 1
    fi
  fi
fi

# Check Tauri cached binary
if [ -f "src-tauri/target/debug/wavemuxsrv.exe" ]; then
  CACHED_VERSION=$(strings src-tauri/target/debug/wavemuxsrv.exe | grep -o "wave version: [0-9.]*" | head -1 | cut -d' ' -f3)
  if [ "$CACHED_VERSION" != "$EXPECTED_VERSION" ]; then
    echo "❌ Tauri cached binary is stale!"
    echo "   Expected: $EXPECTED_VERSION"
    echo "   Cached:   $CACHED_VERSION"
    echo ""
    echo "Run: task sync:dev:binaries"
    if [ "$STRICT_MODE" = true ]; then
      exit 1
    fi
  fi
fi
```

**Pros:**
- ✅ Prevents running dev server with stale binaries
- ✅ Clear error messages
- ✅ Can be optional (--strict flag)

**Cons:**
- ⚠️ Adds startup time
- ⚠️ Requires `strings` command (may not be available on all systems)

### Phase 4: CI/CD Verification

**Add to GitHub Actions workflow:**

```yaml
# .github/workflows/build.yml

- name: Verify version consistency
  run: bash scripts/verify-version.sh --strict

- name: Verify binary versions
  run: |
    echo "Checking wavemuxsrv version..."
    VERSION=$(cat package.json | jq -r '.version')
    BINARY_VERSION=$(strings dist/bin/wavemuxsrv.x64.exe | grep "wave version" | head -1)
    echo "Expected: $VERSION"
    echo "Binary: $BINARY_VERSION"
    if [[ ! "$BINARY_VERSION" =~ "$VERSION" ]]; then
      echo "❌ Version mismatch!"
      exit 1
    fi
```

## Recommended Implementation

### Immediate (v0.24.12)

1. ✅ Add `sync:dev:binaries` task
2. ✅ Make `task dev` depend on it
3. ✅ Test that dev builds work with fresh code

### Short-term (v0.25.0)

1. ✅ Add runtime version verification in main-server.go
2. ✅ Update bump-version.sh to set ExpectedVersion
3. ✅ Enhance verify-version.sh with binary checking

### Long-term (v0.26.0)

1. ✅ Add CI/CD version verification
2. ✅ Add pre-commit hook to check version consistency
3. ✅ Document versioning system in CONTRIBUTING.md

## Testing Requirements

### Unit Tests

**None required** - this is build tooling, not runtime code.

### Manual Tests

1. **Fresh build test:**
   ```bash
   git checkout main
   ./bump-version.sh patch --message "Test version bump"
   task build:backend
   task dev
   # Verify logs show new version
   ```

2. **Stale binary detection:**
   ```bash
   # Manually edit package.json to v0.99.0 without rebuilding
   task check:versions
   # Should fail with clear error
   ```

3. **Auto-sync test:**
   ```bash
   rm src-tauri/target/debug/wavemuxsrv.exe
   task dev
   # Should auto-copy before starting
   ```

## Migration Guide

### For Developers

**If you encounter version mismatches:**

```bash
# Option 1: Clean rebuild
rm -rf src-tauri/target/
task build:backend
task dev

# Option 2: Sync binaries
task sync:dev:binaries
task dev

# Option 3: Nuclear option
./scripts/clean-all.sh
task build:backend
task dev
```

### For CI/CD

**Update workflows to verify versions:**

```bash
# Before packaging
bash scripts/verify-version.sh --strict
```

## Monitoring & Alerts

### Metrics to Track

- **Version mismatch incidents** - How often do devs hit this?
- **Average dev build time** - Ensure sync doesn't add significant overhead
- **CI/CD build failures** - Track version verification failures

### Logs to Watch

```bash
# Grep for version mismatches
grep "VERSION MISMATCH" logs/*.log

# Check for stale binary warnings
grep "Tauri cached binary is stale" logs/*.log
```

## Related Issues

- **PR #270** - Orphaned layout prevention (discovered this issue)
- **Issue #XXX** - To be created for tracking implementation

## References

### Code Files

- `Taskfile.yml` - Build task definitions
- `scripts/verify-version.sh` - Version consistency checker
- `bump-version.sh` - Version bumping script
- `cmd/server/main-server.go` - Server entry point

### Documentation

- `README.md` - Build instructions
- `BUILD.md` - Detailed build guide
- `VERSION_HISTORY.md` - Version changelog

---

**Document Version:** 1.0
**Created:** 2026-02-12
**Author:** AgentA
**Status:** Draft - Pending Implementation
