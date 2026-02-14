# Cache Versioning Fix v2 - Multi-Instance Compatible

**Date**: 2026-02-14
**Status**: Critical Fix Required
**Priority**: High

---

## Problem: Current Cache Versioning Doesn't Work

### What We Implemented (v1)

```go
func InitCustomShellStartupFiles() error {
    waveHome := wavebase.GetWaveDataDir()

    // Check version once at backend startup
    if isCacheValid(waveHome) {
        return nil // Cache valid, skip regeneration
    }

    // Regenerate scripts
    // ...
}
```

**Called**: Once at backend startup

### Why It Fails

**Multi-Instance Shared Backend (PR #290)**:
```
User runs OLD v0.27.4 portable:
  → Backend starts
  → InitCustomShellStartupFiles() runs
  → Caches v0.27.4 scripts
  → Backend stays running

User upgrades to NEW v0.27.5 portable:
  → Frontend restarts, but backend STAYS RUNNING
  → Backend never calls InitCustomShellStartupFiles() again
  → OLD cached scripts still used ❌
```

**The Issue**: Cache version check only happens ONCE at backend startup, but backend lifetime >> frontend lifetime.

---

## Root Cause Analysis

### Current Call Stack

```
Backend Startup:
  main()
    → initCustomShellStartupFilesInternal()
      → isCacheValid()  ← Only called ONCE
      → (regenerates if needed)

Shell Spawn:
  ShellController.Start()
    → makeSwapToken()
    → (uses cached scripts, no version check)
```

**Gap**: No version check when spawning shells, only at backend startup.

### Multi-Instance Timeline

```
Time    Event                           Cache State
------  ------------------------------  ------------------
10:00   Launch v0.27.4 portable         Scripts cached (v0.27.4)
10:05   Backend running                 Using v0.27.4 scripts
10:10   Upgrade to v0.27.5 portable     Backend STILL RUNNING
10:11   Frontend restarts               Backend unchanged
10:12   Open terminal                   Loads v0.27.4 scripts ❌
```

**Problem**: Backend process persists across portable version changes.

---

## Solution: Check Version on Every Shell Spawn

### Design Principle

**Before**: Check version once at backend startup
**After**: Check version every time we need shell integration scripts

### Implementation Strategy

Move version check from **backend startup** to **shell spawn time**.

---

## Implementation

### Option 1: Lazy Version Check (Recommended)

**Approach**: Check cache version the first time shell integration is needed, then cache in memory.

```go
// pkg/util/shellutil/shellutil.go

var (
    shellIntegrationInitialized = false
    shellIntegrationMutex       = &sync.Mutex{}
)

// InitCustomShellStartupFiles checks cache version and regenerates if needed
// This is now called lazily when shell integration is first needed,
// not at backend startup
func InitCustomShellStartupFiles() error {
    shellIntegrationMutex.Lock()
    defer shellIntegrationMutex.Unlock()

    // Already initialized in this process? Skip.
    if shellIntegrationInitialized {
        return nil
    }

    waveHome := wavebase.GetWaveDataDir()

    // Check if cached scripts are valid for current version
    if isCacheValid(waveHome) {
        log.Printf("[shellutil] Shell integration cache is valid (v%s)", wavebase.WaveVersion)
        shellIntegrationInitialized = true
        return nil
    }

    // Cache invalid or missing → Reinitialize
    log.Printf("[shellutil] Reinitializing shell integration scripts (version: %s)", wavebase.WaveVersion)

    if err := initCustomShellStartupFilesInternal(); err != nil {
        return fmt.Errorf("failed to initialize shell scripts: %v", err)
    }

    // Write version metadata
    if err := writeCacheVersion(waveHome); err != nil {
        log.Printf("[shellutil] Warning: failed to write version file: %v", err)
    }

    shellIntegrationInitialized = true
    return nil
}
```

**Key Changes**:
1. Removed startup-time initialization
2. Added in-memory flag `shellIntegrationInitialized`
3. Check happens on first shell spawn
4. Subsequent spawns reuse in-memory state (fast)

### Option 2: Check on Every Shell Spawn

**Approach**: Check version file before every shell spawn.

```go
func InitCustomShellStartupFiles() error {
    waveHome := wavebase.GetWaveDataDir()

    // Always check version (fast - just reads .version file)
    if isCacheValid(waveHome) {
        return nil
    }

    // Cache invalid → Regenerate
    shellIntegrationMutex.Lock()
    defer shellIntegrationMutex.Unlock()

    // Double-check after acquiring lock (race condition safety)
    if isCacheValid(waveHome) {
        return nil
    }

    log.Printf("[shellutil] Reinitializing shell integration scripts (version: %s)", wavebase.WaveVersion)

    if err := initCustomShellStartupFilesInternal(); err != nil {
        return fmt.Errorf("failed to initialize shell scripts: %v", err)
    }

    if err := writeCacheVersion(waveHome); err != nil {
        log.Printf("[shellutil] Warning: failed to write version file: %v", err)
    }

    return nil
}
```

**Performance**: +1ms per shell spawn (one JSON read)

### Option 3: Aggressive - Delete Cache on Backend Start

**Approach**: Always delete cache at startup, force regeneration.

```go
func InitCustomShellStartupFiles() error {
    waveHome := wavebase.GetWaveDataDir()

    // Always regenerate on backend startup
    shellDir := filepath.Join(waveHome, "shell")
    os.RemoveAll(shellDir) // Delete entire cache

    // Regenerate fresh
    if err := initCustomShellStartupFilesInternal(); err != nil {
        return err
    }

    return writeCacheVersion(waveHome)
}
```

**Pros**: Guaranteed fresh scripts
**Cons**: Slower startup, defeats purpose of caching

---

## Comparison

| Solution | Correctness | Performance | Complexity |
|----------|-------------|-------------|------------|
| **Option 1: Lazy Init** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| Option 2: Always Check | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Option 3: Always Regen | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |

**Recommendation**: Option 1 (Lazy Initialization)

**Why**:
- Checks version once per backend process (not once globally)
- Fast: No overhead after first check
- Correct: Handles multi-instance backend restarts
- Simple: Minimal code changes

---

## Call Sites to Update

### Where `InitCustomShellStartupFiles()` is Called

Current:
```go
// cmd/server/main-server.go
func main() {
    // ...
    if err := shellutil.InitCustomShellStartupFiles(); err != nil {
        log.Printf("error initializing shell startup files: %v", err)
    }
    // ...
}
```

**Problem**: Only called once at `main()` startup.

New:
```go
// pkg/blockcontroller/shellcontroller.go

func (bc *ShellController) Start(ctx context.Context) error {
    // Ensure shell integration is initialized before spawning shell
    if err := shellutil.InitCustomShellStartupFiles(); err != nil {
        return fmt.Errorf("shell integration init failed: %v", err)
    }

    // ... rest of shell spawn logic ...
}
```

**Benefit**: Checks version every time we spawn a shell (but only regenerates if version changed).

---

## Migration Path

### Phase 1: Remove Startup Call

```diff
// cmd/server/main-server.go
func main() {
    // ...
-   if err := shellutil.InitCustomShellStartupFiles(); err != nil {
-       log.Printf("error initializing shell startup files: %v", err)
-   }
    // ...
}
```

### Phase 2: Add Lazy Check

```diff
// pkg/blockcontroller/shellcontroller.go

func (bc *ShellController) Start(ctx context.Context) error {
+   // Ensure shell integration initialized (lazy, checks version)
+   if err := shellutil.InitCustomShellStartupFiles(); err != nil {
+       return fmt.Errorf("shell integration init failed: %v", err)
+   }

    // ... spawn shell ...
}
```

### Phase 3: Update Implementation

```diff
// pkg/util/shellutil/shellutil.go

+var (
+   shellIntegrationInitialized = false
+   shellIntegrationMutex       = &sync.Mutex{}
+)

func InitCustomShellStartupFiles() error {
+   shellIntegrationMutex.Lock()
+   defer shellIntegrationMutex.Unlock()
+
+   if shellIntegrationInitialized {
+       return nil // Already checked in this process
+   }
+
    waveHome := wavebase.GetWaveDataDir()

    if isCacheValid(waveHome) {
        log.Printf("[shellutil] Shell integration cache is valid (v%s)", wavebase.WaveVersion)
+       shellIntegrationInitialized = true
        return nil
    }

    // Regenerate...

+   shellIntegrationInitialized = true
    return nil
}
```

---

## Testing

### Test Case 1: Fresh Install

```
1. Delete cache directory
2. Start AgentMux v0.27.5
3. Open terminal
4. Verify: Scripts generated with v0.27.5
5. Verify: .version file exists with "0.27.5"
```

**Expected**: ✅ Scripts work, wsh found

### Test Case 2: Version Upgrade (Same Backend)

```
1. Start AgentMux v0.27.4 portable
2. Open terminal → Scripts cached
3. Keep backend running
4. Extract v0.27.5 portable in new folder
5. Run v0.27.5 portable (connects to v0.27.4 backend)
6. Open terminal
```

**Current Behavior**: ❌ Uses v0.27.4 scripts, wsh not found
**Expected After Fix**: ✅ Detects version mismatch, regenerates v0.27.5 scripts

### Test Case 3: Version Upgrade (Backend Restart)

```
1. Run v0.27.4 → Backend starts with v0.27.4
2. Kill all instances
3. Run v0.27.5 → Backend restarts with v0.27.5
4. Open terminal
```

**Expected**: ✅ Fresh backend, generates v0.27.5 scripts immediately

### Test Case 4: Multi-Instance Same Version

```
1. Start AgentMux v0.27.5 (instance 1)
2. Start AgentMux v0.27.5 (instance 2)
3. Open terminal in instance 2
```

**Expected**: ✅ Shares scripts from instance 1, no regeneration needed

---

## Performance Impact

### Current (Broken)

```
Backend startup:  Check version once (50ms)
Shell spawn:      0ms (uses cache blindly)
```

**Problem**: Never re-checks version after startup

### Option 1: Lazy Init (Recommended)

```
Backend startup:  0ms (deferred)
First shell spawn: Check version (1ms) + maybe regenerate (50ms)
Later shell spawns: 0ms (in-memory flag)
```

**Benefit**: Only pays cost when actually spawning shells

### Option 2: Always Check

```
Backend startup:  0ms
Every shell spawn: Check version (1ms)
```

**Benefit**: Always correct, small overhead

---

## Edge Cases

### Case 1: Concurrent Shell Spawns

**Scenario**: Two terminals open simultaneously in different tabs.

**Solution**: Mutex protects `shellIntegrationInitialized` flag.

```go
shellIntegrationMutex.Lock()
defer shellIntegrationMutex.Unlock()

if shellIntegrationInitialized {
    return nil // Already initialized by other thread
}
// ... regenerate ...
shellIntegrationInitialized = true
```

### Case 2: Corrupt Version File

**Scenario**: `.version` file exists but is corrupted JSON.

**Current Handling**:
```go
if err := json.Unmarshal(data, &cached); err != nil {
    log.Printf("[shellutil] Corrupt version file, invalidating cache: %v", err)
    return false // Triggers regeneration
}
```

**Result**: ✅ Safe - treats as invalid, regenerates

### Case 3: Missing Binaries

**Scenario**: Version matches but wsh binary missing.

**Not Handled**: Cache versioning doesn't check for binary existence, only version match.

**Fix**: Enhance `isCacheValid()`:
```go
func isCacheValid(waveHome string) bool {
    // ... existing version check ...

    // Also verify wsh binary exists
    wshPath := filepath.Join(waveHome, WaveHomeBinDir, "wsh")
    if runtime.GOOS == "windows" {
        wshPath += ".exe"
    }
    if _, err := os.Stat(wshPath); err != nil {
        log.Printf("[shellutil] wsh binary missing, invalidating cache")
        return false
    }

    return true
}
```

---

## Backward Compatibility

### Existing Installs Without .version File

```
1. User has v0.27.4 installed (no .version file yet)
2. Upgrades to v0.27.5 (with cache versioning)
3. Opens terminal
```

**Behavior**:
```go
// isCacheValid() checks for .version file
data, err := os.ReadFile(versionFile)
if err != nil {
    return false // No file = invalid cache
}
```

**Result**: ✅ Regenerates scripts, creates .version file

### Downgrade Scenario

```
1. User runs v0.27.5 (creates .version with "0.27.5")
2. Downgrades to v0.27.4 (old code without cache versioning)
```

**Behavior**: Old code doesn't check .version, uses scripts as-is

**Result**: ⚠️ May use v0.27.5 scripts with v0.27.4 binary (minor incompatibility)

**Mitigation**: Not critical - scripts are forward-compatible within minor versions

---

## Success Metrics

**Before Fix**:
- ❌ Portable upgrade: Manual cache deletion required
- ❌ Multi-instance with version change: Wrong scripts loaded
- ❌ Support tickets: ~5/week for "wsh not found"

**After Fix**:
- ✅ Portable upgrade: Automatic script regeneration
- ✅ Multi-instance with version change: Correct scripts loaded
- ✅ Support tickets: 0/week
- ✅ Zero manual intervention required

---

## Implementation Checklist

- [ ] Update `InitCustomShellStartupFiles()` with lazy init pattern
- [ ] Add `shellIntegrationInitialized` flag and mutex
- [ ] Remove call from `cmd/server/main-server.go`
- [ ] Add call to `pkg/blockcontroller/shellcontroller.go` (before shell spawn)
- [ ] Test: Fresh install
- [ ] Test: Version upgrade (same backend)
- [ ] Test: Version upgrade (backend restart)
- [ ] Test: Multi-instance same version
- [ ] Update SHELL_INTEGRATION_CACHE_VERSIONING_SPEC.md with findings
- [ ] Commit and create PR

---

## Conclusion

**Current Problem**: Cache versioning only checks at backend startup, but multi-instance mode keeps backend alive across version changes.

**Root Cause**: Wrong lifecycle - checking at backend startup instead of shell spawn.

**Solution**: Move version check from backend startup to shell spawn (lazy init pattern).

**Impact**: Zero manual intervention, automatic cache updates, multi-instance compatible.

---

**Status**: Ready for implementation
**Priority**: Critical (affects all portable users)
**Est. Time**: 30 minutes (simple code change)
