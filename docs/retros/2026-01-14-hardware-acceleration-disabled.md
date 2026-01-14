# Retrospective: Hardware Acceleration Accidentally Left Disabled

**Date:** 2026-01-14
**Severity:** Medium (performance regression)
**Versions Affected:** 0.13.5 - 0.15.0

---

## What Happened

During debugging of a window rendering issue in Windows Sandbox/RDP environments, hardware acceleration was disabled as a troubleshooting step. The actual root cause turned out to be unrelated to GPU acceleration, but the disable flag was left in place and shipped in v0.13.5.

**Commit:** `fbd3875` - "fix: disable hardware acceleration to prevent renderer crashes in Windows Sandbox/RDP"

The commit message incorrectly attributes the fix to disabling hardware acceleration, when in reality this was just a debugging step that happened to be in place when the actual fix was applied.

---

## Impact

- **CPU usage increased significantly** on all Windows installations
- Software rendering for all terminal output, UI animations, scrolling
- Users on capable hardware get no benefit from their GPU
- Battery drain increased on laptops

---

## Root Cause Analysis

1. Debugging session for Sandbox/RDP crash issue
2. Hardware acceleration disabled as one of many troubleshooting steps
3. Actual fix was something else (likely unrelated to GPU)
4. Acceleration disable was not reverted before commit
5. Commit message written to justify the change rather than question it
6. No performance regression testing caught the issue

---

## Resolution

**Immediate fix:** Revert to the original behavior where hardware acceleration is enabled by default but can be disabled via settings for users who need it.

```typescript
// emain/emain.ts - appMain()

// Restore original behavior: only disable if explicitly configured
const launchSettings = getLaunchSettings();
if (launchSettings?.["window:disablehardwareacceleration"]) {
    console.log("disabling hardware acceleration, per launch settings");
    electronApp.disableHardwareAcceleration();
}
```

---

## Prevention

1. **Revert debugging changes before commit** - Always review diffs for temporary debugging code
2. **Question the commit message** - If writing a commit message feels like justifying a hack, reconsider the change
3. **Performance baseline testing** - Monitor CPU usage before/after significant changes
4. **Separate debugging from fixes** - Don't mix troubleshooting steps with actual solutions in the same commit

---

## Action Items

- [ ] Revert hardware acceleration to enabled-by-default
- [ ] Bump version to 0.15.1
- [ ] Deploy fix to area54 and claudius
- [ ] Document the `window:disablehardwareacceleration` setting for users who need it
