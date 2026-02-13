# Versioned Install Path Fix

## Problem

The NSIS installer prompts to uninstall prior versions because all versions install to the same directory:
```
C:\Program Files\AgentMux\
```

This causes conflicts when:
1. Users want to keep multiple versions installed
2. Upgrading from one version to another
3. Installing a new version alongside an old one for testing

## Root Cause

The Tauri NSIS bundle configuration doesn't include version in the installation path. All versions share the same:
- Install directory: `%ProgramFiles%\AgentMux\`
- Registry keys (without version differentiation)
- Start menu shortcuts

## Solution

Append version to the install folder to enable side-by-side installations:
```
C:\Program Files\AgentMux\0.26.0\
C:\Program Files\AgentMux\0.25.0\
C:\Program Files\AgentMux\0.24.0\
```

## Implementation

### Option 1: NSIS Template Override (Recommended)

Create custom NSIS template in `src-tauri/nsis/installer.nsi`:

```nsis
!define INSTALLDIR "$PROGRAMFILES\${PRODUCTNAME}\${VERSION}"
```

### Option 2: Tauri Bundle Configuration

Update `src-tauri/tauri.conf.json`:

```json
{
  "bundle": {
    "windows": {
      "nsis": {
        "installMode": "perMachine",
        "installerIcon": "icons/icon.ico",
        "installDirectory": "AgentMux\\${version}",
        "template": "nsis/installer.nsi"
      }
    }
  }
}
```

### Option 3: Package Name Versioning

Include version in product name:

```json
{
  "productName": "AgentMux 0.26.0",
  "identifier": "com.a5af.agentmux-0-26-0"
}
```

**Pros:** Simple, no custom template needed
**Cons:** Clutters UI with version numbers everywhere

## Recommended Approach

Use **Option 2** with custom NSIS template:

1. Create `src-tauri/nsis/installer.nsi` template
2. Configure `tauri.conf.json` to use versioned install directory
3. Test clean install and upgrade scenarios

## Considerations

### Start Menu Shortcuts

With versioned install paths, shortcuts should still go to:
```
Start Menu > AgentMux (not "AgentMux 0.26.0")
```

Latest installed version becomes the default shortcut.

### Registry Keys

Should include version to avoid conflicts:
```
HKLM\Software\AgentMux\0.26.0
```

### Uninstaller

Each version should have its own uninstaller in:
```
Control Panel > Programs > AgentMux 0.26.0
Control Panel > Programs > AgentMux 0.25.0
```

## Migration Path

For existing users upgrading from 0.25.0 → 0.26.0:

1. New installer detects old installation at `C:\Program Files\AgentMux\`
2. Prompts: "Previous version detected. Install alongside or replace?"
3. Default: Install to versioned path, remove old version's shortcuts
4. Advanced: Keep both versions installed

## Testing Checklist

- [ ] Clean install to `C:\Program Files\AgentMux\0.26.0\`
- [ ] Install 0.25.0, then 0.26.0 → Both coexist
- [ ] Shortcuts point to latest version
- [ ] Uninstall one version doesn't affect the other
- [ ] Upgrade path from non-versioned (0.25.0) to versioned (0.26.0)
- [ ] Registry keys don't conflict

## Files to Modify

1. `src-tauri/tauri.conf.json` - Add NSIS configuration
2. `src-tauri/nsis/installer.nsi` - Custom NSIS template (create new)
3. `Taskfile.yml` - Ensure package task includes NSIS template
4. `.gitignore` - Don't ignore `src-tauri/nsis/` directory

## References

- [Tauri NSIS Configuration](https://v2.tauri.app/reference/config/#nsisconfig)
- [NSIS Documentation](https://nsis.sourceforge.io/Docs/)
- [Tauri Bundle Guide](https://v2.tauri.app/distribute/)

## Status

- **Created:** 2026-02-12
- **Status:** Planned
- **Priority:** High
- **Target Version:** 0.27.0 or 0.26.1
