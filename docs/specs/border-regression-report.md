# Border Regression Report

**Date:** 2026-01-15
**Issue:** Colored agent borders are missing from terminal panes

---

## Summary

The 2px colored borders for agent-identified terminal panes are **not a regression** - they were **never merged to main**.

---

## Investigation

### The Border Feature Commit

```
commit 0e920c57844ddae52be4e35430e132f5b37d26d3
Author: AgentA
Date:   Wed Jan 14 07:49:43 2026 -0800

    feat: 2px borders with agent color on focus
```

This commit added:
- `has-agent-color` CSS class
- `--block-agent-color` CSS variable
- Agent color computation in `BlockFrame_Default_Component`
- CSS rules in `block.scss` for colored borders

### Where Is This Commit?

```bash
$ git branch -a --contains 0e920c5
  agenta/fix-version-build
  remotes/origin/agenta/fix-version-build
```

**The border feature exists ONLY on `agenta/fix-version-build` branch.**

It was never merged to:
- `main`
- `agenta/fix-duplicate-title` (current branch)
- Any released version

---

## Branch Divergence Problem

```
                     agenta/fix-version-build (has borders)
                    /
... ─── main ─────┬─── agenta/fix-duplicate-title (no borders)
                  │
                  └─── agenta/fix-hwaccel-regression
```

Multiple feature branches diverged from main at different points. Work done on one branch wasn't carried to others.

### Current Branch State

| Branch | Has Borders | Has Per-Pane Agent | Has Duplicate Fix |
|--------|-------------|--------------------|--------------------|
| main | No | ? | No |
| agenta/fix-version-build | **Yes** | Yes | No |
| agenta/fix-duplicate-title | No | Yes (just fixed) | **Yes** |

---

## Why This Happened

1. **Multiple parallel branches** - Different features developed on different branches
2. **No merge to main** - Border feature completed but never merged
3. **Started new branch from main** - `fix-duplicate-title` branched from main, missing border work
4. **No integration testing** - Didn't verify all features present before releasing

---

## What We Lost

From `agenta/fix-version-build`, these changes are not in current branch:

### 1. Border Styling (`BlockFrame_Default_Component`)
```typescript
// Compute agent color for border styling
let agentColor: string | null = null;
if (!preview && blockData?.meta?.view === "term") {
    const blockEnv = blockData.meta["cmd:env"] as Record<string, string> | undefined;
    // ... agent detection logic
    if (agentId) {
        agentColor = detectAgentColor(mergedEnv, agentId);
    }
}
```

### 2. CSS Class and Variable
```typescript
className={clsx("block", "block-frame-default", {
    // ...
    "has-agent-color": !!agentColor,  // MISSING
})}
style={{
    "--block-agent-color": agentColor ?? "transparent",  // MISSING
}}
```

### 3. SCSS Rules (`block.scss`)
```scss
.block.has-agent-color {
    &.block-focused .block-mask {
        border-color: var(--block-agent-color);
    }
}
```

---

## Recovery Options

### Option A: Cherry-pick Border Commit
```bash
git cherry-pick 0e920c5
```
May have conflicts with current agent detection code.

### Option B: Manually Re-implement
Add the border code back to current branch, adapted to current agent detection logic.

### Option C: Merge fix-version-build
```bash
git merge agenta/fix-version-build
```
Risky - may bring in other unwanted changes.

---

## Recommended Fix

**Option B: Manually re-implement** the border feature in current branch.

The border code needs to:
1. Compute `agentColor` in `BlockFrame_Default_Component` (similar to Header)
2. Add `has-agent-color` class based on `!!agentColor`
3. Set `--block-agent-color` CSS variable
4. Ensure `block.scss` has the border styling rules

This keeps us on a clean branch and avoids merge conflicts.

---

## Lessons Learned

1. **Always merge features to main** - Don't leave completed work on feature branches
2. **Branch from latest** - New feature branches should include all previous work
3. **Integration checklist** - Before release, verify all expected features are present
4. **Single source of truth** - Main branch should always be the reference

---

## Action Items

- [ ] Re-implement border feature in current branch
- [ ] Merge current branch to main after testing
- [ ] Delete stale feature branches
- [ ] Create release checklist document
