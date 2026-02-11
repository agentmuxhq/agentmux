# Grey Screen UI Failure - Emergency Fix

**Date:** 2026-02-11
**Version:** 0.22.0
**Severity:** CRITICAL (App completely non-functional)

---

## Issue

After building v0.22.0 release, the app shows only a grey screen with the window menu bar. **No UI content loads at all.**

This is NOT the white/grey flash issue - this is a **complete UI failure**.

---

## Root Cause

**Content Security Policy (CSP) blocks asset loading in production builds.**

When the CSP was restored in commit `20f3a03`, it was missing `http://tauri.localhost` in the `default-src` directive.

### What Happened

```
Timeline of CSP changes:
1. Original CSP had wavefile: scheme
2. Commit 42e7009: Removed CSP entirely (set to null) to fix dev mode white screen
3. Commit 20f3a03: Restored CSP but used old version
4. Commit 4438991: Updated wavefile: → muxfile: in CSP
5. BUG: Never added http://tauri.localhost to default-src
```

### Technical Details

**Bad CSP (blocking assets):**
```
default-src 'self' tauri: ipc: http://ipc.localhost;
                                                    ↑ Missing http://tauri.localhost
```

**Good CSP (allows assets):**
```
default-src 'self' tauri: ipc: http://ipc.localhost http://tauri.localhost;
                                                                            ↑ Added
```

In Tauri v2 production builds:
- Assets are served from `tauri://localhost/` or `http://tauri.localhost/`
- The CSP only allowed `tauri:` protocol, not `http://tauri.localhost`
- Browser blocked loading JavaScript/CSS from `http://tauri.localhost`
- Result: Grey screen, React never mounts

---

## Fix Applied

**File:** `src-tauri/tauri.conf.json`

**Change:**
```diff
  "security": {
-   "csp": "default-src 'self' tauri: ipc: http://ipc.localhost; connect-src ..."
+   "csp": "default-src 'self' tauri: ipc: http://ipc.localhost http://tauri.localhost; connect-src ..."
  }
```

**Full corrected CSP:**
```
default-src 'self' tauri: ipc: http://ipc.localhost http://tauri.localhost;
connect-src 'self' tauri: ipc: http://ipc.localhost http://tauri.localhost;
script-src 'self' 'unsafe-eval';
style-src 'self' 'unsafe-inline';
img-src 'self' data: blob: muxfile:;
media-src 'self' muxfile:;
frame-src 'self' muxfile:;
font-src 'self' data:;
```

---

## Testing

After rebuild:

1. ✅ **Verify assets load:** Check browser console (F12), no CSP errors
2. ✅ **Verify UI renders:** React mounts, see full AgentMux interface
3. ✅ **Verify muxfile:// works:** File streaming still functional
4. ✅ **Verify dev mode:** `task dev` still works

---

## Rebuild Steps

```bash
cd /c/Systems/agentmux/src-tauri
cargo build --release
```

Output: `target/release/agentmux.exe`

Then package installer:
```bash
cd /c/Systems/agentmux
task package
```

Output: `src-tauri/target/release/bundle/nsis/AgentMux_0.22.0_x64-setup.exe`

---

## Why This Happened

**Lack of CSP testing in CI/CD.** The CSP was manually edited multiple times:
1. To remove `wavefile:` references (rebrand)
2. To fix dev mode (set to null)
3. To restore security (bring back CSP)

Each edit was done manually by copying old CSP strings, not by understanding the full CSP requirements for Tauri v2.

---

## Prevention

### Add CSP Validation Test

```typescript
// test/csp.test.ts
import tauriConfig from '../src-tauri/tauri.conf.json';

test('CSP includes required Tauri protocols', () => {
  const csp = tauriConfig.app.security.csp;

  expect(csp).toContain('tauri:');
  expect(csp).toContain('http://tauri.localhost');
  expect(csp).toContain('muxfile:'); // Custom protocol

  // Ensure default-src has all required sources
  const defaultSrc = csp.match(/default-src ([^;]+)/)?.[1];
  expect(defaultSrc).toContain('tauri:');
  expect(defaultSrc).toContain('http://tauri.localhost');
});
```

### Document CSP Requirements

Add to `BUILD.md`:

```markdown
## Content Security Policy

AgentMux uses a strict CSP. Required directives:

**default-src:**
- `'self'` - Same origin
- `tauri:` - Tauri protocol
- `http://ipc.localhost` - IPC communication
- `http://tauri.localhost` - Asset loading (CRITICAL)

**Custom protocols:**
- `muxfile:` - File streaming protocol

When editing CSP:
1. Test in production build (not just dev mode)
2. Check browser console for CSP violations
3. Verify assets load correctly
```

---

## Related Issues

- PR #255: Fixed dev mode white screen (different CSP issue)
- PR #254: Fixed grey screen from missing RPC stub
- **This issue:** Production grey screen from CSP blocking assets

All three are "grey screen" bugs but with completely different root causes:
1. Dev mode: Wrong webview URL
2. RPC failure: Missing backend stub
3. **Production (this):** CSP blocking asset loading

---

## Verification Checklist

After applying fix:

- [ ] Build completes without errors
- [ ] Installer created successfully
- [ ] Install on clean system
- [ ] Launch app
- [ ] UI loads (not grey screen)
- [ ] No CSP errors in console
- [ ] File operations work (muxfile://)
- [ ] Dev mode still works
- [ ] Repeat on all platforms (Windows, Linux, macOS)

---

## Emergency Rollback

If this fix causes issues, rollback options:

**Option 1: Remove CSP entirely (like dev mode fix)**
```json
"security": {
  "csp": null
}
```

**Option 2: Use dangerousDisableAssetCspModification**
```json
"security": {
  "csp": "...",
  "dangerousDisableAssetCspModification": true
}
```

Both options disable strict CSP and rely on Tauri's capability system for security.

---

## Status

- ✅ Root cause identified
- ✅ Fix applied to `tauri.conf.json`
- 🔄 Rebuilding release binary
- ⏳ Testing pending
- ⏳ New installer pending

Expected timeline: 10-15 minutes (Rust compile time)

---

## Next Steps

1. Wait for build to complete
2. Test the new binary
3. Verify UI loads correctly
4. Create new installer
5. Copy to desktop for distribution
6. Bump version to 0.22.1 with this fix
7. Create PR with CSP fix + tests
