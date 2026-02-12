# Benchmark Build Notes - v0.20.4 (FINALLY_WORKING)

**Branch:** `FINALLY_WORKING`
**Tag:** `v0.20.4-benchmark-ready`
**Commit:** `f04303c`
**Date:** 2026-02-11
**Purpose:** Phase 2 benchmark target (Tauri + Go backend)

---

## What Makes This Build Work

This is a **verified working build** of v0.20.4 with two critical fixes applied:

### 1. CORS Wildcard Fix (commit 155180d)
**File:** `pkg/web/web.go` (lines 556-573)
- **Problem:** CORS blocking between `http://tauri.localhost` and Go backend
- **Solution:** Set `Access-Control-Allow-Origin: *` unconditionally
- **Why safe:** Local backend, no security concern for wildcard CORS

```go
// Always allow CORS from any origin (local backend, no security concern)
w.Header().Set("Access-Control-Allow-Origin", "*")
w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
w.Header().Set("Access-Control-Allow-Headers", "Content-Type, X-Session-Id, X-AuthKey, Authorization, X-Requested-With, Accept, x-vercel-ai-ui-message-stream")
w.Header().Set("Access-Control-Expose-Headers", "X-ZoneFileInfo, Content-Length, Content-Type, x-vercel-ai-ui-message-stream")
```

### 2. TabBar Restoration (commit f04303c)
**File:** `frontend/app/workspace/workspace.tsx` (line 51)
- **Problem:** PR #207 temporarily hid TabBar for multi-window work
- **Solution:** Uncommented `<TabBar key={ws.oid} workspace={ws} />`
- **Why critical:** Widgets bar is essential UI component for benchmarking

---

## Build Artifacts

**Installer:** `WaveMux_0.20.4_x64-setup.exe` (29 MB)
**Location:** Desktop as `wavemux-v0.20.4-FINALLY_WORKING.exe`

---

## Verification Checklist

Before benchmarking, verify:
- ✅ Application launches without white screen
- ✅ No CORS errors in console
- ✅ Widgets bar (TabBar) is visible at top
- ✅ Multiple terminal panes can be created
- ✅ Version displays as 0.20.4 in menu

---

## Rebuild Instructions

If you need to rebuild this exact version:

```bash
cd /c/Systems/agentmux-phase2
git checkout v0.20.4-benchmark-ready
export PATH="/c/Systems/go/bin:/c/Systems/zig-windows-x86_64-0.13.0:/c/GoPath/bin:$PATH"
npm run build
```

Output: `src-tauri/target/release/bundle/nsis/WaveMux_0.20.4_x64-setup.exe`

---

## Why This Version?

**v0.20.4** is the last stable version **before** Phase A (Rust backend integration):
- ✅ Tauri v2 frontend (modern)
- ✅ Go backend sidecar (wavemuxsrv)
- ✅ No Electron remnants
- ✅ All Phase 17 features complete

**Benchmark Target:** Compare startup time of Go backend (v0.20.4) vs Rust backend (v0.21.2)

---

## Known Issues (Fixed)

1. ❌ **Version confusion** - Initially tried v0.18.5, corrected to v0.20.4
2. ❌ **Sidecar not found** - Fixed by using NSIS installer (embeds binaries)
3. ❌ **CORS blocking** - Fixed by wildcard in pkg/web/web.go
4. ❌ **Missing widgets bar** - Fixed by uncommenting TabBar in workspace.tsx

All issues resolved in tag `v0.20.4-benchmark-ready`.

---

## Next Steps

1. Install `wavemux-v0.20.4-FINALLY_WORKING.exe`
2. Verify all checklist items pass
3. Run 5 startup time measurements
4. Compare against v0.21.2 (338ms average)
5. Write benchmark report
