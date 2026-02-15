# Specification: macOS CGO Code Signing Issue

**Author:** Claude
**Date:** 2026-02-14
**Status:** Documented - Temporary Workaround in Place
**Related:** macOS Build Task (#TBD), Version 0.27.8

---

## Problem Statement

AgentMux builds successfully on macOS but crashes immediately on launch with a grey screen. Investigation revealed a **SIGTRAP crash during CGO execution** caused by macOS security policies blocking adhoc-signed binaries.

### Symptoms
- ✅ Build completes successfully (`task package:macos`)
- ✅ DMG created with .app bundle
- ❌ App launches but shows grey/black screen
- ❌ Backend crashes with exit code 2
- ❌ Frontend error: `TypeError: Attempted to assign to readonly property`

### Root Cause
macOS terminates the backend with **SIGTRAP (trace trap)** during SQLite CGO operations:

```
[agentmuxsrv] SIGTRAP: trace trap
PC=0x19fe679ba m=12 sigcode=0
signal arrived during cgo execution
```

**Why this happens:**
1. Backend uses `mattn/go-sqlite3` which requires CGO (`CGO_ENABLED=1`)
2. Tauri builds with **adhoc signature** (no Apple Developer certificate)
3. Modern macOS (especially Apple Silicon) **blocks adhoc-signed CGO binaries** for security
4. Backend crashes immediately after starting, causing grey screen

---

## Current State (v0.27.8)

### Build Configuration
**Backend** (`cmd/server/main-server.go`):
- Uses `mattn/go-sqlite3` (CGO required)
- Built with: `CGO_ENABLED=1 go build -tags "osusergo,sqlite_omit_load_extension"`
- Signed with: **Adhoc signature** (local development only)

**Code Signing Status:**
```bash
$ codesign -dv /Applications/AgentMux.app
Identifier=agentmux-f7973193da6e5e5c
Signature=adhoc
TeamIdentifier=not set
```

**Available Certificates:**
```bash
$ security find-identity -v -p codesigning
0 valid identities found
```

### Limitations
- ⚠️ **Local development only** - cannot distribute to other users
- ⚠️ Requires Gatekeeper bypass to run: `sudo spctl --master-disable`
- ⚠️ Backend crashes on launch due to SIGTRAP
- ⚠️ Not suitable for production distribution

---

## Temporary Workaround

**For local development/testing:**

```bash
# Option 1: Disable Gatekeeper system-wide (not recommended)
sudo spctl --master-disable

# Option 2: Remove quarantine attribute from app
sudo xattr -rd com.apple.quarantine /Applications/AgentMux.app

# Option 3: Add specific app to Gatekeeper allowlist
sudo spctl --add /Applications/AgentMux.app
```

**Trade-offs:**
- ✅ Allows local testing of builds
- ❌ Requires sudo/admin access
- ❌ Reduces system security
- ❌ Not viable for distribution

---

## Long-Term Solutions

### Option 1: Apple Developer Certificate (Recommended for Distribution)

**Requirements:**
- Apple Developer Program membership ($99/year)
- Developer ID Application certificate
- Notarization setup

**Configuration:**
Add to `src-tauri/tauri.conf.json`:
```json
{
  "bundle": {
    "macOS": {
      "minimumSystemVersion": "10.15",
      "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)",
      "entitlements": "src-tauri/entitlements.plist"
    }
  }
}
```

**Benefits:**
- ✅ Proper code signing for CGO binaries
- ✅ No SIGTRAP crashes
- ✅ Can distribute to other users
- ✅ Notarization support
- ✅ Professional distribution

**Drawbacks:**
- ⚠️ $99/year cost
- ⚠️ Signing adds 30-60s to each build
- ⚠️ Certificate management overhead

---

### Option 2: Pure-Go SQLite (Recommended for Local Dev)

**Replace CGO SQLite with pure-Go alternative:**

```go
// Replace in go.mod:
// github.com/mattn/go-sqlite3 v1.14.33 (requires CGO)
// with:
modernc.org/sqlite v1.28.0  // Pure Go, no CGO
```

**Build tags approach:**
```go
// database_cgo.go
//go:build !purego

import _ "github.com/mattn/go-sqlite3"

// database_purego.go
//go:build purego

import _ "modernc.org/sqlite"
```

**Build commands:**
```bash
# Local dev (pure Go, no signing needed)
task build:backend -- -tags purego

# Production (CGO + signing)
task package:macos
```

**Benefits:**
- ✅ No code signing needed for local dev
- ✅ Faster build iterations
- ✅ No SIGTRAP crashes
- ✅ Simpler dev workflow
- ✅ Cross-platform builds easier

**Drawbacks:**
- ⚠️ Pure-Go SQLite ~10-20% slower than CGO version
- ⚠️ Slight performance trade-off

---

## Distribution Strategy

### For Open Source Terminal Apps

**NOT recommended:**
- ❌ **App Store** - sandboxing restrictions prevent terminal functionality

**Recommended distribution channels:**
1. **GitHub Releases** (signed DMGs)
   - Requires: Developer ID certificate
   - Example: iTerm2, Warp, Alacritty

2. **Homebrew Cask** (signed app)
   - Requires: Developer ID certificate
   - Automatic updates via `brew upgrade`

3. **Direct Download** (signed DMG from website)
   - Requires: Developer ID certificate + notarization

**All require:**
- Apple Developer Program ($99/year)
- Developer ID Application certificate
- Notarization for macOS 10.15+

---

## Retrospective: What Went Wrong

### Timeline of Investigation

1. **Initial symptom:** Grey screen on launch
2. **First hypothesis:** Version mismatch between frontend/backend
   - ✅ Fixed: `ExpectedVersion` constant in `main-server.go`
   - ❌ Didn't solve grey screen

3. **Second hypothesis:** Code signing issues (crash reports showed SIGKILL)
   - ⚠️ Partially correct - old crash was signing issue
   - ❌ New crash was different (SIGTRAP, not SIGKILL)

4. **Third hypothesis:** Frontend JavaScript error
   - Found: `TypeError: Attempted to assign to readonly property`
   - ❌ Was symptom, not cause (backend crash caused this)

5. **Root cause discovered:** SIGTRAP during CGO execution
   - Backend crashes immediately after successful startup
   - macOS blocks adhoc-signed CGO binaries
   - No frontend error - just can't connect to dead backend

### Lessons Learned

**Build System Issues:**
- ⚠️ `bump-version.sh` failed to update all version files on macOS
- ⚠️ Manual fixes needed for: `Cargo.toml`, `tauri.conf.json`, `main-server.go`
- 📝 **TODO:** Fix `bump-version.sh` sed commands for macOS compatibility

**Version Management:**
- ✅ Version verification system worked correctly (detected mismatches)
- ⚠️ Too many version sources (package.json, Cargo.toml, tauri.conf.json, Go const)
- 📝 **TODO:** Single source of truth for version number

**macOS Signing:**
- ⚠️ CGO + adhoc signing = incompatible on modern macOS
- ⚠️ SIGTRAP during CGO is hard to diagnose (looks like many other issues)
- ⚠️ No clear error message - just silent crash
- 📝 **Lesson:** Always check `codesign -dv` and logs for SIGTRAP

**Testing Gaps:**
- ⚠️ Build succeeded but app didn't run - need runtime testing in CI
- ⚠️ No automated test for "does the app actually launch?"
- 📝 **TODO:** Add smoke test to CI that launches app and checks for grey screen

---

## Recommendations

### Immediate (v0.27.8)
- ✅ Document CGO signing limitation
- ✅ Add README warning about local dev requirements
- ✅ Provide workaround instructions (Gatekeeper bypass)

### Short-term (v0.28.x)
- 🔄 Implement pure-Go SQLite build option
- 🔄 Fix `bump-version.sh` for macOS
- 🔄 Add build validation tests

### Long-term (v1.0)
- 📋 Get Apple Developer certificate
- 📋 Set up proper code signing pipeline
- 📋 Notarization automation
- 📋 Distribute via Homebrew + GitHub Releases

---

## Testing Checklist

### Before declaring "fixed":
- [ ] Clean build creates DMG
- [ ] DMG installs to /Applications
- [ ] App launches without grey screen
- [ ] Backend starts successfully (check logs)
- [ ] Frontend loads UI (not grey screen)
- [ ] Can create terminal session
- [ ] Multi-window support works
- [ ] No SIGTRAP or SIGKILL in logs

### Signing validation:
```bash
# Check signature
codesign -dv --verbose=4 /Applications/AgentMux.app

# Verify no SIGTRAP in logs
grep -i "SIGTRAP\|trace trap" ~/Library/Logs/com.a5af.agentmux/*.log

# Test backend standalone
/Applications/AgentMux.app/Contents/MacOS/agentmuxsrv --version
```

---

## References

- **CGO on macOS:** https://github.com/golang/go/issues/11100
- **Code Signing Guide:** https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution
- **Tauri Signing:** https://tauri.app/v1/guides/distribution/sign-macos
- **Pure-Go SQLite:** https://gitlab.com/cznic/sqlite

---

## Related Issues

- Version mismatch detection: `ExpectedVersion` constant
- `bump-version.sh` macOS compatibility issues
- Multi-window architecture requires proper backend startup
- DMG creation via `bundle_dmg.sh` (working as of v0.27.8)

---

## Status Summary

**Current state:**
- ✅ Builds successfully on macOS
- ✅ Creates .app and .dmg bundles
- ⚠️ Requires Gatekeeper bypass to run
- ⚠️ Local development only (cannot distribute)

**Next steps:**
1. User decides: Apple Developer cert OR pure-Go SQLite
2. Implement chosen solution
3. Re-test full workflow
4. Update distribution docs

**Decision pending:** Signing strategy for v1.0 release
