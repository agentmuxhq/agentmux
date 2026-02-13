# Portable Build Specification

**Version**: 1.0.0
**Date**: 2026-02-12
**Status**: Implementation

---

## Problem Statement

The current Tauri build produces two artifacts:

1. **Standalone executable**: `agentmux.exe` (28 MB)
   - ❌ Doesn't work - white screen
   - ❌ Missing backend binaries (agentmuxsrv, wsh)
   - ❌ Tauri only bundles sidecars in installer, not standalone

2. **NSIS Installer**: `AgentMux_0.26.0_x64-setup.exe`
   - ✅ Works - includes all binaries
   - ❌ Requires installation
   - ❌ Not portable

**Need**: True portable build as a ZIP archive with all dependencies bundled.

---

## Portable Build Requirements

### FR-1: ZIP Archive Structure

```
agentmux-0.26.0-x64-portable.zip
├── agentmux.exe              (Tauri frontend - 28 MB)
├── agentmuxsrv.x64.exe       (Go backend - 33 MB)
├── wsh-0.26.0-windows.x64.exe (Shell integration - 11 MB)
└── README.txt                (Usage instructions)
```

**Total size**: ~72 MB compressed to ~40-50 MB ZIP

### FR-2: Binary Locations

The portable should look for binaries in this order:

1. **Same directory as agentmux.exe** (portable mode)
2. Bundled resources (installer mode)
3. PATH (development mode)

### FR-3: Data Directory Isolation

Portable mode should use a local data directory instead of `~/.agentmux`:

```
agentmux-0.26.0-x64-portable/
├── agentmux.exe
├── agentmuxsrv.x64.exe
├── wsh-0.26.0-windows.x64.exe
└── Data/                     (Created on first run)
    ├── config/
    ├── db/
    └── logs/
```

### FR-4: README.txt Content

```txt
AgentMux v0.26.0 - Portable Edition

Quick Start:
1. Extract this ZIP to any folder
2. Run agentmux.exe
3. Data will be stored in the "Data" subfolder

Requirements:
- Windows 10/11 x64
- No installation needed
- No admin rights required

Files:
- agentmux.exe: Main application
- agentmuxsrv.x64.exe: Backend server (auto-launched)
- wsh-*.exe: Shell integration binary

Support: https://github.com/a5af/agentmux
```

---

## Implementation

### Option A: Post-Build PowerShell Script (Recommended)

**File**: `scripts/package-portable.ps1`

```powershell
# Package portable build for AgentMux
param(
    [string]$Version = "0.26.0",
    [string]$OutputDir = "dist"
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path $PSScriptRoot
$BuildDir = "$RepoRoot\src-tauri\target\release"
$PortableDir = "$OutputDir\agentmux-$Version-x64-portable"
$ZipPath = "$OutputDir\agentmux-$Version-x64-portable.zip"

Write-Host "Packaging AgentMux $Version Portable Build..." -ForegroundColor Cyan

# Clean previous build
if (Test-Path $PortableDir) {
    Remove-Item $PortableDir -Recurse -Force
}
if (Test-Path $ZipPath) {
    Remove-Item $ZipPath -Force
}

# Create portable directory
New-Item -ItemType Directory -Force -Path $PortableDir | Out-Null

# Copy main executable
Write-Host "Copying agentmux.exe..." -ForegroundColor Gray
Copy-Item "$BuildDir\agentmux.exe" "$PortableDir\agentmux.exe" -Force

# Copy backend
Write-Host "Copying backend binaries..." -ForegroundColor Gray
Copy-Item "$RepoRoot\dist\bin\agentmuxsrv.x64.exe" "$PortableDir\agentmuxsrv.x64.exe" -Force

# Copy wsh
Write-Host "Copying wsh binary..." -ForegroundColor Gray
Copy-Item "$RepoRoot\dist\bin\wsh-$Version-windows.x64.exe" "$PortableDir\wsh-$Version-windows.x64.exe" -Force

# Create README
Write-Host "Creating README.txt..." -ForegroundColor Gray
@"
AgentMux v$Version - Portable Edition

Quick Start:
1. Extract this ZIP to any folder
2. Run agentmux.exe
3. Data will be stored in the "Data" subfolder

Requirements:
- Windows 10/11 x64
- No installation needed
- No admin rights required

Files:
- agentmux.exe: Main application (Tauri frontend)
- agentmuxsrv.x64.exe: Backend server (auto-launched)
- wsh-$Version-windows.x64.exe: Shell integration binary

Support: https://github.com/a5af/agentmux

Build Date: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
"@ | Out-File "$PortableDir\README.txt" -Encoding UTF8

# Create ZIP
Write-Host "Creating ZIP archive..." -ForegroundColor Gray
Compress-Archive -Path "$PortableDir\*" -DestinationPath $ZipPath -Force

# Cleanup temp directory
Remove-Item $PortableDir -Recurse -Force

# Show results
$ZipSize = (Get-Item $ZipPath).Length / 1MB
Write-Host ""
Write-Host "✓ Portable build created:" -ForegroundColor Green
Write-Host "  File: $ZipPath" -ForegroundColor White
Write-Host "  Size: $([math]::Round($ZipSize, 2)) MB" -ForegroundColor White
```

### Option B: Task Integration

**Add to `Taskfile.yml`:**

```yaml
package:portable:
  desc: Create portable ZIP build
  deps:
    - build:backend
    - tauri:copy-sidecars
  cmds:
    - npm run tauri build
    - powershell -Command "& ./scripts/package-portable.ps1"
```

### Option C: npm Script Alias

**Add to `package.json`:**

```json
{
  "scripts": {
    "package:portable": "task package:portable"
  }
}
```

---

## Backend Binary Discovery

The Tauri app needs to find binaries in the same directory. This requires updating the Rust backend.

**File**: `src-tauri/src/backend/mod.rs` (or wherever binary paths are resolved)

### Current Logic

```rust
// Current: Only looks for bundled resources
let sidecar_path = app.path_resolver()
    .resolve_resource("binaries/agentmuxsrv.exe")
    .expect("failed to resolve agentmuxsrv binary");
```

### Updated Logic

```rust
use std::path::PathBuf;

fn find_backend_binary(app: &tauri::AppHandle, binary_name: &str) -> Result<PathBuf, String> {
    // 1. Check same directory as executable (portable mode)
    if let Ok(exe_dir) = app.path_resolver().app_dir() {
        let portable_path = exe_dir.join(binary_name);
        if portable_path.exists() {
            return Ok(portable_path);
        }
    }

    // 2. Check bundled resources (installer mode)
    if let Some(bundled_path) = app.path_resolver()
        .resolve_resource(format!("binaries/{}", binary_name)) {
        if bundled_path.exists() {
            return Ok(bundled_path);
        }
    }

    // 3. Check PATH (development mode)
    if let Ok(path_binary) = which::which(binary_name) {
        return Ok(path_binary);
    }

    Err(format!("Could not find {} in portable dir, bundled resources, or PATH", binary_name))
}
```

**Usage**:

```rust
let agentmuxsrv_path = find_backend_binary(&app, "agentmuxsrv.x64.exe")?;
let wsh_path = find_backend_binary(&app, "wsh-0.26.0-windows.x64.exe")?;
```

---

## Data Directory for Portable Mode

### Current Behavior

AgentMux stores data in:
- **Windows**: `C:\Users\{user}\AppData\Roaming\AgentMux\Data`
- **macOS**: `~/Library/Application Support/AgentMux/Data`
- **Linux**: `~/.config/AgentMux/Data`

### Portable Mode Behavior

When running from portable directory, store data locally:

```
{portable_dir}/Data/
├── config/
├── db/
└── logs/
```

### Implementation

**Detect portable mode**:

```rust
fn is_portable_mode(app: &tauri::AppHandle) -> bool {
    // If agentmuxsrv.x64.exe exists in same directory as exe, we're portable
    if let Ok(exe_dir) = app.path_resolver().app_dir() {
        exe_dir.join("agentmuxsrv.x64.exe").exists()
    } else {
        false
    }
}

fn get_data_dir(app: &tauri::AppHandle) -> PathBuf {
    if is_portable_mode(app) {
        // Portable: Use {exe_dir}/Data
        app.path_resolver()
            .app_dir()
            .unwrap()
            .join("Data")
    } else {
        // Installed: Use system AppData
        app.path_resolver()
            .app_data_dir()
            .unwrap()
    }
}
```

---

## Testing

### Test 1: Extract and Run

1. Extract ZIP to `C:\Temp\AgentMux\`
2. Run `agentmux.exe`
3. Verify no white screen
4. Verify backend starts (check Task Manager for `agentmuxsrv.x64.exe`)
5. Verify `Data/` folder is created in same directory

### Test 2: Version Display

1. Check tabbar shows "AgentMux v0.26.0"
2. Open About dialog - version matches

### Test 3: Multiple Instances

1. Extract ZIP to two different folders
2. Run both instances with `--instance` flag
3. Verify both work independently

### Test 4: No Admin Required

1. Extract ZIP to user's Downloads folder (no admin)
2. Run executable
3. Should work without UAC prompt

---

## Files to Create/Modify

- [ ] `scripts/package-portable.ps1` (new)
- [ ] `Taskfile.yml` (add package:portable task)
- [ ] `package.json` (add script alias)
- [ ] `src-tauri/src/backend/*.rs` (update binary discovery)
- [ ] `src-tauri/src/backend/*.rs` (add portable mode data dir)
- [ ] `README.md` (document portable build)

---

## Future Enhancements

1. **Auto-updater for portable**: Check for updates, download new ZIP
2. **Cross-platform portables**: macOS .app bundle, Linux .tar.gz
3. **Portable settings**: Checkbox in app to "use portable data directory"
4. **Launcher script**: `agentmux.bat` that handles PATH and launches

---

## References

- Tauri Resource Resolution: https://tauri.app/v2/guides/features/resources/
- Tauri Sidecar Binaries: https://tauri.app/v2/guides/bundling/sidecar/
- VERSIONED_ARTIFACTS_SPEC.md: Naming conventions for artifacts
