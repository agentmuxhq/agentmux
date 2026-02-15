# Next Steps - v0.27.13 Multi-Instance Fix

**Current Status:** 2026-02-15 11:47 UTC

---

## Current State

### ✅ Completed
- [x] All fixes implemented and tested locally
- [x] 4 commits pushed to `agentx/fix-data-dir-path` branch
- [x] PR #308 opened and updated with comprehensive description
- [x] Comment added explaining latest changes
- [x] Portable build tested and verified working
  - Instance 1: `backend reused: false` → unique window/tab
  - Instance 2: `backend reused: true` → NEW window/tab
  - No websocket collision
  - No grey screen
- [x] Portable build available at `C:\Users\asafe\Desktop\agentmux-test\`

### ⏳ Pending
- [ ] User testing of portable build
- [ ] PR approval (ReAgent has stale review blocking merge)
- [ ] Merge to main
- [ ] Official release build
- [ ] Branch cleanup

---

## Immediate Next Steps

### 1. User Testing
**What:** Validate the fix works in real-world usage

**How:**
```powershell
# Kill any running instances
taskkill /F /IM agentmux.exe
taskkill /F /IM agentmuxsrv.x64.exe

# Clean stale data (optional)
del "$env:LOCALAPPDATA\com.a5af.agentmux\instances\default\wave-*.lock"
del "$env:APPDATA\com.a5af.agentmux\instances\default\wave-endpoints.json"

# Launch first instance
C:\Users\asafe\Desktop\agentmux-test\agentmux.exe

# Launch second instance (double-click exe again)
C:\Users\asafe\Desktop\agentmux-test\agentmux.exe
```

**Expected behavior:**
- Both windows open with independent terminal sessions
- Typing in one doesn't affect the other
- Both connect to same backend (one `agentmuxsrv.x64.exe` process)
- No grey screen on startup

**What to test:**
- [ ] Both windows load successfully (no grey screen)
- [ ] Terminal input works in both windows independently
- [ ] Can create/switch tabs in both windows
- [ ] Closing one window doesn't kill the other
- [ ] Closing both windows kills the backend process

---

### 2. PR Approval

**Current blocker:** ReAgent has "changes requested" review from initial push (about version 0.27.11 → 0.27.13 jump)

**Issue:** This is a false positive - 0.27.12 was already merged in PR #307

**Options:**

#### Option A: Wait for ReAgent (recommended if not urgent)
- ReAgent isn't currently online (`agentbus list-agents` showed only AgentX and github-consumer)
- When ReAgent comes back online, jekt them to re-review PR #308
- They should see the updated commits and description

#### Option B: Request Copilot Review
```bash
cd /c/Users/asafe/.claw/agentx-workspace/agentmux
gh pr view 308
# Check if Copilot review is available
```

#### Option C: Merge anyway (if user approves)
- The stale review is about a non-issue (version jump was correct)
- Latest commit (1a35df4) has all fixes
- Tested locally and working
- User can approve and merge directly

---

### 3. Post-Merge Tasks

#### A. Official Release Build
```bash
cd /c/Users/asafe/.claw/agentx-workspace/agentmux
task package:portable    # Create official v0.27.13 portable build
task package             # Create installer
```

**Artifacts:**
- `dist/AgentMux_0.27.13_x64_en-US.msi` (Windows installer)
- `dist/agentmux-portable-0.27.13.zip` (Portable)

#### B. Copy to Desktop for Daily Use
```powershell
# Extract official portable build
Expand-Archive dist/agentmux-portable-0.27.13.zip -DestinationPath "$env:USERPROFILE\Desktop\agentmux-0.27.13\"

# Or install MSI
msiexec /i dist\AgentMux_0.27.13_x64_en-US.msi
```

#### C. Branch Cleanup
```bash
git checkout main
git pull origin main
git branch -d agentx/fix-data-dir-path
git push origin --delete agentx/fix-data-dir-path
```

#### D. GitHub Release (Optional)
If this is a release-worthy version:
```bash
gh release create v0.27.13 \
  --title "v0.27.13 - Multi-Instance Fix" \
  --notes "Fixes grey screen and multi-instance window collision issues" \
  dist/AgentMux_0.27.13_x64_en-US.msi \
  dist/agentmux-portable-0.27.13.zip
```

---

## Known Issues / Future Work

### Not in Scope for v0.27.13
- Tray icon management (moved to backend in earlier PR)
- Auto-updater (stub commands exist)
- macOS/Linux builds (Windows-only for now)

### Potential Follow-ups
- Add integration test for multi-instance scenario
- Document multi-instance behavior in README
- Consider instance cleanup on exit (remove lock files, endpoint files)

---

## Quick Reference

| Item | Location |
|------|----------|
| **Branch** | `agentx/fix-data-dir-path` |
| **PR** | https://github.com/a5af/agentmux/pull/308 |
| **Test Build** | `C:\Users\asafe\Desktop\agentmux-test\` |
| **Retro** | `RETRO-v0.27.13-multi-instance-fix.md` |
| **Latest Commit** | `1a35df4` (Multi-instance backend reuse with independent windows) |
| **Version** | 0.27.13 |

---

**Decision Point:** User testing → approval decision → merge → release
