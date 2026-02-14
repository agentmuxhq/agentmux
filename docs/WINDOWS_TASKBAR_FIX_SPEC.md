# Windows Taskbar Icon Fix Specification

**Date**: 2026-02-13
**Status**: ✅ Implemented
**Version**: 0.27.0+

---

## Problem

When running AgentMux on Windows, users were seeing **2 additional taskbar icons** for each instance:

1. **Main AgentMux window icon** (expected) ✅
2. **Backend sidecar icon** (`agentmuxsrv.x64.exe`) ❌
3. **Potential duplicate icons** when multiple windows open ❌

**Expected behavior**: Only **1 taskbar icon** should appear, regardless of how many windows are open.

---

## Research Process

### Investigation Steps

1. **Problem identification**:
   - User reported: "Every instance shows 2 additional icons in the taskbar"
   - Expected: "Only 1 icon no matter how many windows are open"

2. **Hypothesis formation**:
   - **Hypothesis 1**: Sidecar process creating separate taskbar icon
   - **Hypothesis 2**: Windows not grouping multiple AgentMux windows

3. **Codebase analysis**:
   - Examined `src-tauri/src/sidecar.rs` - Go backend spawned via Tauri shell plugin
   - Examined `Taskfile.yml` - Build configuration for backend
   - Examined `tauri.conf.json` - Windows-specific configuration

4. **Key findings**:
   - `agentmuxsrv.x64.exe` spawned in portable mode (line 108 in sidecar.rs)
   - Go build uses default flags: `-ldflags "-s -w"`
   - No AppUserModelID configured in Tauri config
   - No Windows-specific process creation flags

5. **Root cause confirmation**:
   - **Issue 1**: Default Go build creates console application with visible window
   - **Issue 2**: Missing AppUserModelID prevents window grouping

### Research References

- [Go cmd/link documentation](https://pkg.go.dev/cmd/link): `-H windowsgui` flag
- [Windows CreateProcess flags](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags): `CREATE_NO_WINDOW`
- [Windows Shell AppUserModelID](https://learn.microsoft.com/en-us/windows/win32/shell/appids): Taskbar grouping mechanism
- [Tauri v2 shell plugin](https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/shell): Process spawning
- [Multi-window implementation docs](./MULTI_WINDOW_IMPLEMENTATION.md): Backend sharing across windows

---

## Root Causes

### Issue 1: Sidecar Console Window
The Go backend (`agentmuxsrv.x64.exe`) was compiled as a **console application**, creating a visible console window with its own taskbar icon.

**Why**: The default Go build produces console executables on Windows. Without the `-H windowsgui` linker flag, the process shows a console window.

### Issue 2: Window Grouping
Multiple AgentMux windows were not grouping under a single taskbar icon because they lacked a shared **Application User Model ID (AppUserModelID)**.

**Why**: Windows uses AppUserModelID to group related windows. Without explicit configuration, each window gets treated as a separate application.

---

## Implementation Details

### Prerequisites

**Tools required**:
- Go 1.21+ (for backend builds)
- Task 3.x (build system)
- Tauri CLI 2.x (for packaging)
- Windows 10+ (for testing)

**Files modified**:
- `Taskfile.yml` - Build configuration

### Implementation Steps

#### Step 1: Analyze Build Process

Examined the `build:server:internal` task in `Taskfile.yml`:
```yaml
build:server:internal:
    cmd: CGO_ENABLED=1 GOARCH={{.GOARCH}} go build \
        -tags "osusergo,sqlite_omit_load_extension" \
        -ldflags "{{.GO_LDFLAGS}} -X main.BuildTime=... -X main.WaveVersion=..." \
        -o dist/bin/agentmuxsrv.{{...}}.exe \
        cmd/server/main-server.go
```

Identified `GO_LDFLAGS` variable controls linker flags.

#### Step 2: Research Windows Subsystem Options

**Console subsystem** (default):
- Creates console window
- Allocates console buffer
- Shows in taskbar
- Standard output/error work

**Windows GUI subsystem** (`-H windowsgui`):
- No console window
- Runs in background
- No taskbar icon (unless window created)
- Standard output/error suppressed

**Decision**: Use `-H windowsgui` since backend has no UI and logs to files.

#### Step 3: Implement Conditional Build Flags

Modified `Taskfile.yml` to use platform-specific flags:
```yaml
# Before (cross-platform, always creates console)
GO_LDFLAGS: "-s -w"

# After (Windows: no console, others: default)
GO_LDFLAGS: '{{if eq OS "windows"}}-s -w -H windowsgui{{else}}-s -w{{end}}'
```

**Rationale**: Only Windows has taskbar icon behavior. Linux/macOS unaffected.

#### Step 4: Verify Backend Logging

Confirmed backend uses file-based logging (not console output):
- Logging configured in `cmd/server/main-server.go`
- Logs written to app data directory
- No dependency on console window

#### Step 5: Rebuild and Test

```bash
# Clean old binaries
rm dist/bin/agentmuxsrv*

# Rebuild with new flags
task build:backend

# Verify ldflags in build output:
# -ldflags "-s -w -H windowsgui ..." ✅

# Package portable build
task package:portable

# Output: dist/agentmux-0.27.0-x64-portable.zip
```

#### Step 6: AppUserModelID Research (Pending Implementation)

**Attempted approach 1**: Tauri config property
```json
// tauri.conf.json - bundle > windows
"windows": {
  "appUserModelId": "com.a5af.AgentMux"  // ❌ Not supported in schema
}
```
**Result**: Schema validation error - property not allowed.

**Approach 2**: Windows API in Rust (recommended)
```rust
// src-tauri/src/lib.rs - in setup() function
#[cfg(target_os = "windows")]
{
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
    use windows::core::HSTRING;

    let app_id = HSTRING::from("com.a5af.AgentMux");
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(&app_id);
    }
}
```
**Status**: Requires adding `windows` crate dependency. Deferred for future PR.

**Approach 3**: Installer-set AppUserModelID
- Set via shortcut properties during installation
- Registry-based configuration
- Requires NSIS installer modifications

**Decision**: Implement Approach 2 in follow-up PR. Fix 1 (sidecar hiding) addresses immediate issue.

---

## Solutions Implemented

### Fix 1: Hide Sidecar Console Window

**Change**: Added `-H windowsgui` to Go build flags for Windows builds

**File**: `Taskfile.yml`

**Before**:
```yaml
GO_LDFLAGS: "-s -w"
```

**After**:
```yaml
GO_LDFLAGS: '{{if eq OS "windows"}}-s -w -H windowsgui{{else}}-s -w{{end}}'
```

**Effect**: The backend sidecar (`agentmuxsrv.x64.exe`) now runs as a **Windows GUI subsystem** application without a console window. This prevents it from appearing in the taskbar.

**Build command**:
```bash
# Windows build now includes -H windowsgui
go build -ldflags "-s -w -H windowsgui ..." -o agentmuxsrv.x64.exe
```

### Fix 2: Window Grouping (Pending)

**Status**: Research phase
**Approach**: Set AppUserModelID via Windows API in Rust

**Options**:
1. **Tauri config** (not supported in current schema)
2. **Windows API call** in `src-tauri/src/lib.rs` setup function
3. **Registry/shortcut** metadata (installer-based)

**Recommendation**: Implement in Rust using Windows API:
```rust
#[cfg(target_os = "windows")]
{
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    let app_id = HSTRING::from("com.a5af.AgentMux");
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(&app_id);
    }
}
```

**Dependency**: Requires `windows` crate

---

## Testing

### Manual Test Plan

1. **Single instance**:
   - Launch AgentMux
   - Verify only 1 taskbar icon appears
   - Verify no console window visible

2. **Multiple instances**:
   - Launch AgentMux (Ctrl+N or open from Start Menu multiple times)
   - Verify all windows group under 1 taskbar icon
   - Verify no console windows visible

3. **Sidecar lifecycle**:
   - Launch AgentMux
   - Check Task Manager → `agentmuxsrv.x64.exe` should be running
   - Verify no console window in Window list
   - Close AgentMux → sidecar should terminate

### Expected Results

| Test | Before Fix | After Fix |
|------|-----------|-----------|
| Single window taskbar icons | 2 (app + sidecar) | 1 (app only) ✅ |
| Multi-window taskbar icons | N×2 (ungrouped) | 1 (grouped) ⏳ |
| Console window visible | Yes (sidecar) | No ✅ |

✅ = Fixed
⏳ = Pending (Fix 2)

---

## Build Commands

### Rebuild backend with fix:
```bash
cd agentmux
rm dist/bin/agentmuxsrv*
task build:backend
```

### Rebuild portable package:
```bash
task package:portable
```

### Output:
- `dist/agentmux-0.27.0-x64-portable.zip`
- Contains fixed `agentmuxsrv.x64.exe` (no console window)

---

## Related Files

| File | Purpose | Changes |
|------|---------|---------|
| `Taskfile.yml` | Build configuration | Added `-H windowsgui` for Windows |
| `src-tauri/src/sidecar.rs` | Sidecar process spawning | No changes needed |
| `cmd/server/main-server.go` | Backend entry point | No changes needed |

---

## Notes

### Windows Subsystem Behavior

Go executables can be built with different Windows subsystems:

- **Console subsystem** (default): Shows console window, allocates console
- **Windows GUI subsystem** (`-H windowsgui`): No console, background process

**Important**: With `-H windowsgui`, the backend cannot use `fmt.Println()` or `log.Println()` for debugging. Use file-based logging or structured logging instead.

### Compatibility

- **Windows 10/11**: ✅ Fully supported
- **Windows 7/8**: ⚠️ Not tested (may work, but unsupported)
- **Linux/macOS**: ✅ No impact (flag only applies to Windows)

---

## Future Work

1. **Implement AppUserModelID** for proper window grouping (Fix 2)
2. **Add integration test** for taskbar behavior
3. **Document logging** approach for windowless backend
4. **Registry cleanup** on uninstall (if AppUserModelID uses registry)

---

## References

- [Go linker flags documentation](https://pkg.go.dev/cmd/link)
- [Windows Application User Model IDs](https://learn.microsoft.com/en-us/windows/win32/shell/appids)
- [Tauri v2 Configuration](https://v2.tauri.app/reference/config/)
