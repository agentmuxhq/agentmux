# AgentMux Multi-Instance Support Specification

**Status:** Design Phase
**Date:** 2026-02-12
**Author:** AgentX
**Goal:** Enable multiple AgentMux executables to run simultaneously

---

## Executive Summary

Currently, AgentMux enforces single-instance execution through **two independent mechanisms**:

1. **Tauri UI Layer** - `tauri-plugin-single-instance` focuses existing window
2. **Go Backend Layer** - File lock on `~/.agentmux/wave.lock` prevents duplicate servers

While the backend supports multi-instance via `--instance=<name>` flag, the Tauri plugin prevents the UI from launching. This spec proposes **removing the Tauri plugin** to allow native multi-instance support.

---

## Current State

### Single-Instance Enforcement Layers

| Layer | Implementation | Location | Behavior |
|-------|----------------|----------|----------|
| **Desktop (Tauri)** | `tauri-plugin-single-instance` | `src-tauri/src/lib.rs:34-41` | Focuses existing window instead of creating new |
| **Backend (Go)** | File lock via `wave.lock` | `pkg/wavebase/wavebase.go` | Exits with error message |

### File Lock Implementation

**Windows** (`pkg/wavebase/wavebase-win.go`):
```go
func AcquireWaveLock() (FDLock, error) {
    lockFileName := filepath.Join(dataHomeDir, "wave.lock")
    m, err := filemutex.New(lockFileName)
    err = m.TryLock() // Non-blocking, fails if locked
    return m, nil
}
```

**POSIX** (`pkg/wavebase/wavebase-posix.go`):
```go
func AcquireWaveLock() (FDLock, error) {
    lockFileName := filepath.Join(dataHomeDir, "wave.lock")
    fd, _ := os.OpenFile(lockFileName, os.O_RDWR|os.O_CREATE, 0600)
    err := unix.Flock(int(fd.Fd()), unix.LOCK_EX|unix.LOCK_NB)
    return fd, nil
}
```

### Existing Multi-Instance Support

The backend **already supports** multiple instances via `--instance` flag:

```bash
Wave.exe --instance=test
# Data: ~/.agentmux-test/Data
# Config: ~/.config/agentmux/ (shared)
# Lock: ~/.agentmux-test/Data/wave.lock
```

**Problem:** Tauri plugin prevents this from working because it intercepts the second launch.

---

## Design Options

### Option 1: Remove Tauri Single-Instance Plugin ✅ **RECOMMENDED**

**Approach:** Remove the plugin entirely, rely solely on Go backend locking.

**Pros:**
- ✅ Simple implementation (delete code)
- ✅ No breaking changes for users
- ✅ Backend already handles multi-instance correctly
- ✅ Each instance gets isolated data directory
- ✅ Works with existing `--instance` flag

**Cons:**
- ⚠️ Multiple windows can launch briefly before backend lock fails
- ⚠️ User sees error message instead of silent focus

**Implementation:**
1. Remove plugin from `src-tauri/Cargo.toml`
2. Remove plugin initialization from `src-tauri/src/lib.rs`
3. Test multi-instance behavior

**User Experience:**

**Current (with plugin):**
```
User: Launches agentmux.exe
User: Launches agentmux.exe again
Result: Second launch focuses first window (no error)
```

**After removal:**
```
User: Launches agentmux.exe
User: Launches agentmux.exe again
Result: Second window opens briefly, shows error:
  "ERROR: Another instance of Wave is already running"
  "To run multiple instances, use: Wave.exe --instance=test"
Window closes after 5 seconds
```

---

### Option 2: Conditional Plugin Based on CLI Args

**Approach:** Disable plugin if `--instance` arg is present.

**Pros:**
- ✅ Preserves single-instance UX for default case
- ✅ Explicit multi-instance support

**Cons:**
- ❌ Tauri plugin doesn't support conditional initialization
- ❌ Can't access CLI args before plugin initialization
- ❌ Complex workaround required (environment variables)

**Verdict:** Not feasible with current Tauri architecture.

---

### Option 3: Generate Unique Instance IDs Automatically

**Approach:** Auto-generate instance ID if lock fails.

**Pros:**
- ✅ Zero user configuration
- ✅ Always allows multiple instances

**Cons:**
- ❌ Unpredictable data directories
- ❌ User can't identify which instance is which
- ❌ Breaks existing `--instance` workflow

**Verdict:** Poor UX, not recommended.

---

### Option 4: Desktop Environment Integration

**Approach:** Use OS-specific multi-instance APIs.

**Pros:**
- ✅ Native OS behavior

**Cons:**
- ❌ Different implementation per platform
- ❌ Tauri plugin already abstracts this
- ❌ Removes cross-platform consistency

**Verdict:** Over-engineered, not recommended.

---

## Recommended Solution: Option 1

**Remove the Tauri single-instance plugin** and rely on Go backend locking.

### Rationale

1. **Backend already solves the problem** - File locking with helpful error messages
2. **Simplest implementation** - Remove code, don't add complexity
3. **Consistent with terminal paradigm** - Terminals allow multiple windows
4. **Preserves existing multi-instance support** - `--instance` flag already works
5. **Better error visibility** - Users see why second launch failed

### Trade-offs

| Aspect | Before | After |
|--------|--------|-------|
| **Single instance** | Silent window focus | Error message + window close |
| **Multi-instance** | Broken (plugin blocks) | Works (via `--instance`) |
| **User friction** | None (automatic) | Minimal (read error message) |
| **Developer complexity** | Plugin dependency | None |

---

## Implementation Plan

### Phase 1: Remove Tauri Plugin ✅ Immediate

**Files to modify:**

1. **`src-tauri/Cargo.toml`** - Remove dependency
   ```diff
   - tauri-plugin-single-instance = "2"
   ```

2. **`src-tauri/src/lib.rs`** - Remove plugin initialization
   ```diff
   - .plugin(
   -     tauri_plugin_single_instance::init(|app, _args, _cwd| {
   -         if let Some(window) = app.get_webview_window("main") {
   -             let _ = window.set_focus();
   -         }
   -     }),
   - )
   ```

3. **Rebuild and test**
   ```bash
   task package
   ```

---

### Phase 2: Improve Backend Error UX (Optional)

**Current error message:**
```
========================================
ERROR: Another instance of Wave is already running
========================================

To run multiple instances simultaneously, launch with the --instance flag:
  Example: Wave.exe --instance=test
  Example: Wave.exe --instance=dev

Each instance will have its own isolated data.
========================================
```

**Suggested improvements:**

1. **Add countdown timer** - Window auto-closes after 5 seconds
2. **Add "Copy Command" button** - Copies `Wave.exe --instance=test` to clipboard
3. **Add "Launch Instance" button** - Prompts for instance name and launches
4. **Persist error in log file** - So users can reference it later

**Implementation:**
- Modify `cmd/server/main-server.go` error handling
- Add Tauri event to display error in UI before exit
- Add countdown timer to error dialog

---

### Phase 3: Documentation Updates

**Files to update:**

1. **`README.md`** - Add multi-instance section
   ```markdown
   ## Running Multiple Instances

   AgentMux supports multiple simultaneous instances with isolated data:

   ```bash
   # Main instance
   agentmux.exe

   # Test instance (separate data directory)
   agentmux.exe --instance=test

   # Development instance
   agentmux.exe --instance=dev
   ```

   Each instance maintains:
   - ✅ Isolated workspace data
   - ✅ Independent connection history
   - ✅ Separate terminal sessions
   - 🔗 Shared user settings
   ```

2. **`docs/FORK.md`** - Update instance documentation
3. **`docs/MULTI_INSTANCE_SPEC.md`** - This file (reference guide)

---

## Testing Strategy

### Test 1: Single Instance (Default Behavior)

**Steps:**
1. Launch `agentmux.exe`
2. Wait for app to fully load
3. Launch `agentmux.exe` again (same executable)
4. Observe backend error message
5. Verify first instance still running

**Expected:**
- First instance: Loads normally
- Second instance: Shows error, exits after 5 seconds
- First instance: Unaffected

---

### Test 2: Multi-Instance with --instance Flag

**Steps:**
1. Launch `agentmux.exe`
2. Launch `agentmux.exe --instance=test`
3. Launch `agentmux.exe --instance=dev`
4. Verify all 3 instances running independently

**Expected:**
- Default instance: Data in `~/.agentmux/Data`
- Test instance: Data in `~/.agentmux-test/Data`
- Dev instance: Data in `~/.agentmux-dev/Data`
- All instances fully functional

---

### Test 3: Duplicate Instance Name

**Steps:**
1. Launch `agentmux.exe --instance=test`
2. Launch `agentmux.exe --instance=test` (same name)

**Expected:**
- Second launch shows error (lock already held)
- First instance unaffected

---

### Test 4: Cross-Platform Verification

**Platforms to test:**
- Windows 11 x64
- Windows 11 ARM64 (if available)
- macOS Intel
- macOS Apple Silicon
- Linux (Ubuntu/Debian)

**Verify:**
- File locking works correctly on each platform
- Error messages display properly
- Data directories created in expected locations

---

## Edge Cases & Considerations

### 1. Lock File Cleanup

**Problem:** If AgentMux crashes, `wave.lock` may persist.

**Current handling:**
- Lock is released when process exits (file handle closed)
- File locks are automatically released by OS on crash
- Manual cleanup: Delete `~/.agentmux/wave.lock`

**No changes needed** - OS handles this correctly.

---

### 2. Network Port Conflicts

**Problem:** Multiple instances may try to bind same ports.

**Current handling:**
- Backend uses ephemeral ports (OS assigns)
- No hard-coded port binding
- Each instance gets unique port

**No changes needed** - Already handled.

---

### 3. Configuration File Conflicts

**Problem:** Multiple instances might write to shared config simultaneously.

**Current implementation:**
- Config directory: `~/.config/agentmux/` (shared)
- Data directory: `~/.agentmux-{instance}/Data` (isolated)

**Potential issue:**
- Race condition if multiple instances save settings simultaneously
- File-based config lacks atomic writes

**Mitigation:**
- Config reads/writes are infrequent (startup/settings change)
- Most state lives in isolated data directories
- Low probability of conflicts in practice

**Future improvement:**
- Add file locking for config writes
- Use atomic file replacement (write to temp, rename)

---

### 4. Database Conflicts

**Problem:** SQLite databases don't support concurrent access across processes.

**Current handling:**
- Each instance has isolated database in data directory
- DB path: `~/.agentmux-{instance}/Data/agentmux.db`

**No changes needed** - Already isolated.

---

### 5. User Confusion

**Problem:** Users might accidentally launch multiple instances.

**Mitigation:**
- Clear error messages explaining what happened
- Instructions on how to use `--instance` flag intentionally
- Documentation with use cases (testing, development, separate workflows)

---

## Success Criteria

✅ **Functional:**
- Multiple instances can run with `--instance` flag
- Default instance still enforces single execution
- No crashes or data corruption

✅ **User Experience:**
- Clear error messages when duplicate launch detected
- Documentation explains multi-instance workflow
- Minimal friction for legitimate use cases

✅ **Technical:**
- No regressions in single-instance mode
- Cross-platform compatibility maintained
- File locking works on all platforms

---

## Migration Path

### For Existing Users

**No migration needed** - Removing plugin is non-breaking:

- Default behavior: Same (single instance enforced by backend)
- Multi-instance: Now works (previously broken)
- Data directories: Unchanged
- Settings: Unchanged

### For Developers

**Dependency change:**
```bash
# After removing plugin
cd src-tauri
cargo update
cargo build
```

**Testing:**
```bash
# Build portable
task package

# Test single instance
./agentmux.exe

# Test multi-instance
./agentmux.exe --instance=test
```

---

## Alternative Approaches Considered

### Desktop Shortcuts with Instance IDs

**Idea:** Create desktop shortcuts pre-configured with `--instance` flags.

**Pros:**
- User-friendly (click to launch)
- No command-line knowledge needed

**Cons:**
- Doesn't solve core problem (plugin still blocks)
- Manual setup required per instance

**Verdict:** Good complement, not a replacement.

---

### Environment Variable-Based Instances

**Idea:** Use `WAVEMUX_INSTANCE` environment variable.

**Pros:**
- Standard Unix pattern
- Works with scripts

**Cons:**
- Desktop shortcuts don't pass env vars easily on Windows
- CLI flag is more explicit

**Verdict:** Could add as secondary mechanism.

---

### Window Title Differentiation

**Idea:** Show instance ID in window title.

**Current:** "AgentMux"
**Proposed:** "AgentMux [test]" or "AgentMux (dev)"

**Pros:**
- Easy to identify which instance is which
- Minimal implementation

**Cons:**
- None

**Verdict:** Should implement regardless of choice.

---

## Implementation Checklist

### Code Changes

- [ ] Remove `tauri-plugin-single-instance` from `src-tauri/Cargo.toml`
- [ ] Remove plugin initialization from `src-tauri/src/lib.rs`
- [ ] Update window title to show instance ID
- [ ] Add countdown timer to backend error display
- [ ] Improve backend error message formatting

### Documentation

- [ ] Update `README.md` with multi-instance section
- [ ] Update `docs/FORK.md` with instance examples
- [ ] Create `docs/MULTI_INSTANCE_SPEC.md` (this file)
- [ ] Add troubleshooting section for lock file issues

### Testing

- [ ] Test single instance on Windows
- [ ] Test multi-instance with --instance flag on Windows
- [ ] Test single instance on macOS
- [ ] Test multi-instance on macOS
- [ ] Test single instance on Linux
- [ ] Test multi-instance on Linux
- [ ] Test duplicate instance name error
- [ ] Test concurrent config writes (edge case)

### Release

- [ ] Create PR with changes
- [ ] Update CHANGELOG.md
- [ ] Create GitHub release notes
- [ ] Update documentation site

---

## Timeline Estimate

| Phase | Task | Time |
|-------|------|------|
| **Phase 1** | Remove Tauri plugin | 15 minutes |
| | Build and smoke test | 10 minutes |
| | Multi-instance testing | 30 minutes |
| **Phase 2** | Improve error UX (optional) | 2 hours |
| | Window title differentiation | 1 hour |
| **Phase 3** | Documentation updates | 1 hour |
| | Cross-platform testing | 2 hours |
| **Phase 4** | PR review and merge | 1 day |

**Total:** ~1 day of work (Phase 1 can ship in 1 hour)

---

## Risks & Mitigation

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| File lock doesn't release on crash | High | Low | OS handles cleanup automatically |
| Config file race condition | Medium | Low | Document, add locking later if needed |
| User confusion about instances | Low | Medium | Clear error messages + docs |
| Platform-specific lock issues | High | Very Low | Existing code battle-tested |

---

## Post-Implementation Monitoring

**Metrics to track:**
- Number of multi-instance users (telemetry)
- Lock acquisition failures (logs)
- Crash reports related to instance conflicts

**User feedback channels:**
- GitHub issues tagged `multi-instance`
- Discord discussions
- Support tickets

---

## Future Enhancements

### 1. Instance Manager UI

**Concept:** Built-in panel to manage multiple instances.

**Features:**
- List running instances
- Launch new instance with custom name
- Switch between instances
- Stop specific instance

**Priority:** Low (CLI flag sufficient for now)

---

### 2. Shared Clipboard Across Instances

**Concept:** Copy/paste between AgentMux instances.

**Implementation:**
- Shared memory segment for clipboard data
- Cross-instance IPC

**Priority:** Low (OS clipboard already works)

---

### 3. Instance Templates

**Concept:** Pre-configured instance profiles.

**Example:**
```bash
agentmux.exe --template=python-dev
# Auto-configures instance with Python-specific settings
```

**Priority:** Low (power user feature)

---

## References

- Tauri Plugin Documentation: https://tauri.app/plugin/single-instance
- Go filemutex Library: https://github.com/alexflint/go-filemutex
- Unix Flock Manual: https://man7.org/linux/man-pages/man2/flock.2.html
- AgentMux FORK.md: `docs/FORK.md`

---

## Appendix: Current Lock File Locations

| Platform | Default Instance | Named Instance |
|----------|------------------|----------------|
| **Windows** | `%LOCALAPPDATA%\agentmux\Data\wave.lock` | `%LOCALAPPDATA%\agentmux-{name}\Data\wave.lock` |
| **macOS** | `~/Library/Application Support/agentmux/Data/wave.lock` | `~/Library/Application Support/agentmux-{name}/Data/wave.lock` |
| **Linux** | `~/.local/share/agentmux/Data/wave.lock` | `~/.local/share/agentmux-{name}/Data/wave.lock` |

---

**Next Steps:** Review this spec, get user approval, implement Phase 1.
