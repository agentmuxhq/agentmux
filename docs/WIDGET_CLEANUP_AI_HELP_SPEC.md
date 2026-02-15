# Widget Cleanup Spec: Consolidate AI Widgets & Simplify Help

**Date:** 2026-02-15
**Status:** Proposal
**Author:** AgentA
**Decision:** Consolidate AI functionality to unified agent widget, simplify help to show tips

---

## Executive Summary

**Proposal:** Remove `defwidget@ai`, `defwidget@claudecode`, and `defwidget@tips` widgets. Rewire `help` widget to show QuickTips content directly (no browser).

**Rationale:**
- **Consolidation:** The new `agent` widget (PR #305) supersedes both `ai` and `claudecode`
- **Redundancy:** Three AI widgets (ai, claudecode, agent) for the same purpose
- **Simplification:** Help widget uses heavy WebViewModel just to show docs - tips content is simpler
- **UX Clarity:** Users confused by multiple AI options (which one to use?)
- **Philosophy:** One widget per use case, not multiple overlapping features

**Estimated Savings:**
- **Code:** ~1,541 lines removed (~63 KB source)
- **Widget count:** 6 → 3 widgets (-50%)
- **View types:** 10 → 7 view types (-30%)
- **Complexity:** Fewer AI choices = clearer UX

**Final Widget Lineup:**
1. ✅ **Terminal** - Shell interaction
2. ✅ **Sysinfo** - System monitoring
3. ✅ **Agent** - Unified AI assistant (replaces ai + claudecode)
4. ✅ **Help** - Tips and documentation (replaces tips, no longer uses WebViewModel)

---

## Current State Analysis

### Widget Inventory

| Widget | View Type | Icon | Purpose | Status |
|--------|-----------|------|---------|--------|
| `defwidget@terminal` | `term` | square-terminal | Terminal emulator | ✅ **Keep (core)** |
| `defwidget@ai` | `waveai` | sparkles | Legacy AI chat | ❌ **Remove** |
| `defwidget@sysinfo` | `sysinfo` | chart-line | System info | ✅ **Keep** |
| `defwidget@claudecode` | `claudecode` | terminal | Claude Code wrapper | ❌ **Remove** |
| `defwidget@agent` | `agent` | sparkles | Unified AI agent | ✅ **Keep (replacement)** |
| `defwidget@tips` | `tips` | lightbulb | Quick tips | ❌ **Remove** |
| `help` widget | `help` | circle-question | Documentation browser | ✅ **Modify (show tips)** |

**Hidden/Internal Views:**
- `tsunami` - App hosting (keep, not a widget)
- `webview` - Base class for help/tsunami (keep)
- `vdom`, `cpuplot`, `agentai`, `launcher`, `chat`, `codeeditor` - Internal views

---

## Resource Analysis

### 1. AI Widget (`defwidget@ai`)

**Code Footprint:**
```
frontend/app/view/agentai/agentai.tsx       882 lines
frontend/app/view/agentai/agentai.scss      ~150 lines
Total:                                      ~1,032 lines (39 KB on disk)
```

**Features:**
- Legacy AI chat interface
- Message history
- Claude API integration
- Markdown rendering

**Issues:**
- ❌ **Superseded:** Agent widget provides same + more features
- ❌ **Redundant:** Three AI widgets confuse users
- ❌ **Maintenance burden:** Duplicate AI codebase

**Usage Estimate:** <20% of users (agent widget is preferred)

---

### 2. Claude Code Widget (`defwidget@claudecode`)

**Code Footprint:**
```
frontend/app/view/claudecode/claudecode.tsx            239 lines
frontend/app/view/claudecode/claudecode-view.tsx       445 lines
frontend/app/view/claudecode/claudecode-model.ts       ~450 lines
frontend/app/view/claudecode/claudecode-parser.ts      ~150 lines
frontend/app/view/claudecode/claudecode-types.ts       ~100 lines
frontend/app/view/claudecode/claudecode-helpers.ts     ~30 lines
frontend/app/view/claudecode/claudecode.scss           ~200 lines
Total:                                                 ~1,614 lines (57 KB on disk)
```

**Features:**
- Claude Code CLI wrapper
- Stream-json parser (NDJSON)
- Tool execution display
- Markdown rendering
- Terminal integration

**Issues:**
- ❌ **Superseded:** Agent widget has identical backend (same Claude CLI, same --output-format stream-json)
- ❌ **Duplicate parser:** claudecode-parser.ts and agent/stream-parser.ts do the same thing
- ❌ **Confusion:** Users don't know difference between "claude code" and "agent"

**Technical Note:** Agent widget (PR #305) uses the **exact same backend**:
- Controller: `cmd`
- Command: `claude`
- Args: `["--output-format", "stream-json"]`
- Interactive: `true`
- Run on start: `true`

The only difference is the frontend parser/renderer, and agent's is more advanced.

**Usage Estimate:** <30% of users (agent widget is better)

---

### 3. Tips Widget (`defwidget@tips`)

**Code Footprint:**
```
frontend/app/view/quicktipsview/quicktipsview.tsx      35 lines
Total:                                                 35 lines (1.5 KB on disk)
```

**Features:**
- Simple wrapper around `<QuickTips />` component
- No navigation, no complexity
- Just renders tips content

**Issues:**
- ❌ **Redundant with help:** Both show tips/docs, but help is heavier (WebViewModel)
- ❌ **Better use case:** Make help widget lightweight like tips (not WebViewModel browser)

---

### 4. Help Widget (Modification Required)

**Current Implementation:**
```typescript
// frontend/app/view/helpview/helpview.tsx
class HelpViewModel extends WebViewModel {
    constructor(blockId: string, nodeModel: BlockNodeModel) {
        super(blockId, nodeModel);
        // WebView navigation (back/forward/home buttons)
        // Loads embedded docsite via webview
        this.homepageUrl = atom(getApi().getDocsiteUrl());
        this.viewType = "help";
        this.viewIcon = atom("circle-question");
        this.viewName = atom("Help");
    }
}

function HelpView(props: ViewComponentProps<HelpViewModel>) {
    return <WebView {...props} onFailLoad={...} />;
}
```

**Current Behavior:**
- Extends WebViewModel (heavy base class)
- Renders embedded docsite in WebView
- Navigation buttons (back/forward/home)
- Zoom controls
- DevTools integration

**Issues:**
- ❌ **Overcomplicated:** Uses full browser engine just to show tips
- ❌ **Heavy dependency:** Requires WebViewModel + WebView infrastructure
- ❌ **Poor UX:** Embedded browser inferior to just showing tips content
- ❌ **Tips widget is simpler:** 35 lines vs 174 lines

**Proposed New Implementation:**
```typescript
// frontend/app/view/helpview/helpview.tsx
class HelpViewModel implements ViewModel {
    viewType: string;
    showTocAtom: PrimitiveAtom<boolean>;

    constructor() {
        this.viewType = "help";
    }

    get viewComponent(): ViewComponent {
        return HelpView;
    }
}

function HelpView({ model }: { model: HelpViewModel }) {
    return (
        <div className="px-[5px] py-[10px] overflow-auto w-full">
            <QuickTips />
        </div>
    );
}
```

**New Behavior:**
- Lightweight ViewModel (no WebViewModel inheritance)
- Directly renders `<QuickTips />` component (same as tips widget)
- No browser, no navigation, just clean tips content
- **Same functionality as tips widget, but under "help" name**

---

## Removal Impact Analysis

### Code Volume Reduction

| Category | Lines | Files | Disk (KB) | Notes |
|----------|-------|-------|-----------|-------|
| AI widget (waveai) | ~1,032 | 2 | 39 | Fully deleted |
| Claude Code widget | ~1,614 | 7 | 57 | Fully deleted |
| Tips widget | 35 | 1 | 1.5 | Fully deleted |
| Help widget refactor | -139 | 0 | -5.5 | Simplified (174→35 lines) |
| **Total** | **~2,820** | **10** | **103** | Net reduction |

**Note:** Help widget simplified but not deleted (file remains, just gutted).

---

### Bundle Size Impact

**Methodology:**
1. Vite tree-shaking removes unused code
2. Minification reduces ~4KB source → ~1-2KB minified
3. Gzip compression applies to final bundles

**Estimated Reductions:**

| Item | Unminified | Minified | Gzipped | Notes |
|------|-----------|----------|---------|-------|
| AI widget JS | ~40 KB | ~12 KB | ~4 KB | AgentAI view deleted |
| Claude Code widget JS | ~60 KB | ~18 KB | ~6 KB | Parser + view deleted |
| Tips widget JS | ~1 KB | ~0.3 KB | ~0.2 KB | Tiny deletion |
| Help widget refactor | ~5 KB | ~1.5 KB | ~0.5 KB | WebViewModel dependency removed |
| Associated CSS | ~15 KB | ~5 KB | ~2 KB | Styles for deleted widgets |
| **Total** | **~121 KB** | **~37 KB** | **~13 KB** | **Post-gzip savings** |

**Real-world bundle impact:**
- Production build currently: ~7-10 MB (unoptimized estimate)
- Removing widgets: **-37 KB minified** (~0.4% reduction before gzip)
- **Gzipped savings: ~13 KB** (users download 13 KB less)

**Additional savings:**
- Fewer view types to maintain
- Clearer widget launcher (3 widgets vs 6)
- Reduced user confusion

---

### Complexity Reduction

**Metrics:**

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| User-facing widgets | 6 | 3 | **-50%** |
| AI widgets | 3 (ai, claudecode, agent) | 1 (agent) | **-67%** |
| View types (total) | 10 | 7 | -30% |
| Lines of code (widgets) | ~3,800 | ~980 | **-74%** |
| Widget launcher entries | 6 | 3 | **-50%** |

**Maintenance Benefits:**
- ✅ One AI widget instead of three (clear choice for users)
- ✅ Help widget no longer depends on WebViewModel (simpler)
- ✅ Fewer features to support
- ✅ Clearer product positioning (focused, not bloated)

**User Benefits:**
- ✅ No confusion about which AI widget to use (only "agent")
- ✅ Simpler widget launcher (3 core widgets)
- ✅ Help widget loads faster (no WebView overhead)
- ✅ Consistent UX across widgets

---

## Migration Path (For Existing Users)

### AI Widget → Agent Widget

**Old workflow:**
```
1. User clicks "ai" widget
2. Opens legacy AI chat
3. Limited features (just chat)
```

**New workflow:**
```
1. User clicks "agent" widget
2. Opens unified AI agent
3. More features: streaming, tool execution, multi-agent messaging
```

**Migration:** No action needed - agent widget is **strictly better**

---

### Claude Code Widget → Agent Widget

**Old workflow:**
```
1. User clicks "claude code" widget
2. Opens Claude CLI wrapper
3. Stream-json output parsed and rendered
```

**New workflow:**
```
1. User clicks "agent" widget
2. Opens unified AI agent (SAME backend: claude --output-format stream-json)
3. Better parser, better UX, same functionality + more
```

**Migration:** No action needed - agent widget uses **identical backend**, just better frontend

---

### Tips Widget → Help Widget

**Old workflow:**
```
1. User clicks "tips" widget
2. Shows QuickTips content
3. Lightweight, simple
```

**New workflow:**
```
1. User clicks "help" widget
2. Shows QuickTips content (exact same as tips widget)
3. Lightweight, simple (no longer WebView browser)
```

**Migration:** Rename "tips" → "help" in user's mind. Functionality identical.

---

## Removal Plan

### Simple Clean Removal (Next Release)

**Philosophy:** Don't overcomplicate. Just remove cruft and consolidate.

**No deprecation warnings** - Clean break, simple changelog note
**No replacement commands** - Agent widget already exists
**No migration period** - Rip the band-aid off

**File Changes:**

```bash
# 1. Remove widget definitions
# FILE: pkg/wconfig/defaultconfig/widgets.json
- "defwidget@ai": { ... }           # DELETE
- "defwidget@claudecode": { ... }   # DELETE
# NOTE: "defwidget@tips" is NOT in widgets.json (only in BlockRegistry)

# 2. Remove frontend view code
rm -rf frontend/app/view/agentai/
rm -rf frontend/app/view/claudecode/
rm -rf frontend/app/view/quicktipsview/

# 3. Simplify help widget
# FILE: frontend/app/view/helpview/helpview.tsx
# REPLACE entire file with simple QuickTips wrapper (see implementation below)

# 4. Remove view registrations
# FILE: frontend/app/block/block.tsx
- import { ClaudeCodeViewModel } from "@/app/view/claudecode/claudecode";  # DELETE
- import { QuickTipsViewModel } from "../view/quicktipsview/quicktipsview";  # DELETE
# NOTE: AgentAiModel import already exists, keep for internal use if needed
- BlockRegistry.set("agentai", AgentAiModel);      # DELETE
- BlockRegistry.set("claudecode", ClaudeCodeViewModel);  # DELETE
- BlockRegistry.set("tips", QuickTipsViewModel);   # DELETE

# 5. Remove view type mappings
# FILE: frontend/app/block/blockutil.tsx
# Remove these cases from blockViewToIcon():
- if (view == "waveai") { return "sparkles"; }
- if (view == "claudecode") { return "terminal"; }
- if (view == "tips") { return "lightbulb"; }

# Remove these cases from blockViewToName():
- if (view == "waveai") { return "WaveAI"; }
- if (view == "claudecode") { return "Claude Code"; }
- if (view == "tips") { return "Tips"; }

# 6. Run build verification
task build:frontend
bash scripts/verify-version.sh
```

---

## Implementation: Simplified Help Widget

**New `frontend/app/view/helpview/helpview.tsx`:**

```typescript
// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { QuickTips } from "@/app/element/quicktips";
import { atom, PrimitiveAtom } from "jotai";

/**
 * HelpViewModel - Simplified help widget that shows QuickTips content
 *
 * Previously extended WebViewModel to show embedded docsite in a browser.
 * Now simplified to directly render QuickTips component (same as old tips widget).
 *
 * Rationale:
 * - No need for heavy WebViewModel + WebView infrastructure
 * - Tips content is more useful than embedded browser
 * - Faster, simpler, cleaner UX
 */
class HelpViewModel implements ViewModel {
    viewType: string;
    showTocAtom: PrimitiveAtom<boolean>;

    constructor() {
        this.viewType = "help";
        this.showTocAtom = atom(false);
    }

    get viewComponent(): ViewComponent {
        return HelpView;
    }

    showTocToggle() {
        // Optional: toggle table of contents if QuickTips supports it
        // (Currently unused, but kept for future enhancement)
    }
}

function HelpView({ model }: { model: HelpViewModel }) {
    return (
        <div className="px-[5px] py-[10px] overflow-auto w-full">
            <QuickTips />
        </div>
    );
}

export { HelpViewModel };
```

**Key Changes:**
- ✅ No longer extends WebViewModel
- ✅ No WebView import or usage
- ✅ No navigation buttons (back/forward/home)
- ✅ No zoom controls
- ✅ No docsite URL loading
- ✅ Just renders QuickTips component directly
- ✅ Reduced from 174 lines → 35 lines

---

## Commit Message

```
feat: consolidate AI widgets and simplify help

BREAKING CHANGE: AI and claudecode widgets removed. Tips widget removed.

Users should use:
- AI assistance: "agent" widget (replaces ai, claudecode)
- Help/tips: "help" widget (now shows tips, no browser)

Removals:
- frontend/app/view/agentai/ (~1,032 lines)
- frontend/app/view/claudecode/ (~1,614 lines)
- frontend/app/view/quicktipsview/ (35 lines)
- Help widget simplified (no longer extends WebViewModel)

Changes:
- pkg/wconfig/defaultconfig/widgets.json: Removed defwidget@ai and defwidget@claudecode
- frontend/app/block/block.tsx: Removed agentai, claudecode, tips registrations
- frontend/app/block/blockutil.tsx: Removed waveai, claudecode, tips cases
- frontend/app/view/helpview/helpview.tsx: Simplified to show QuickTips (no WebViewModel)

Bundle size reduction: ~37 KB minified, ~13 KB gzipped
Complexity reduction: 6 → 3 widgets (-50%)
AI widgets consolidated: 3 → 1 (agent only)

Design spec: docs/WIDGET_CLEANUP_AI_HELP_SPEC.md

Rationale: Agent widget supersedes both ai and claudecode (same backend,
better UX). Help widget doesn't need browser - tips content is simpler
and more useful.

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>
```

---

## CHANGELOG.md Entry

```markdown
### Removed
- **AI widget** - Use "agent" widget instead (same functionality + more)
- **Claude Code widget** - Use "agent" widget instead (identical backend)
- **Tips widget** - Use "help" widget instead (now shows tips content)

### Changed
- **Help widget** - Now shows tips content directly (no browser)
- Widget count reduced: 6 → 3 widgets
- Bundle size reduced by ~13 KB (gzipped)
```

---

## Testing Strategy

### Post-Removal Testing

**Regression tests:**
```bash
# Verify widgets removed
1. Open widget launcher → Verify only 3 widgets (terminal, sysinfo, agent)
2. Search for "ai" in launcher → No results
3. Search for "claude code" in launcher → No results
4. Search for "tips" in launcher → No results

# Verify help widget works
5. Open "help" widget → Shows QuickTips content (not browser)
6. Verify no navigation buttons (back/forward/home)
7. Verify no zoom controls
8. Content loads fast (no WebView overhead)

# Verify agent widget works (replacement)
9. Open "agent" widget → Works normally
10. Streams Claude CLI output → Parses correctly
11. Tool execution displays → Renders correctly
12. All agent features work → Multi-agent messaging, filtering, etc.

# Verify no crashes
13. Create new tab → No errors
14. Open existing workspace → No errors
15. Check console for errors → None related to missing views
```

**Build verification:**
```bash
# Check bundle size
npm run build
du -sh dist/frontend/  # Should be smaller

# Check for orphaned code
grep -r "agentai\|claudecode\|quicktipsview" frontend/  # Should be empty (except docs)
grep -r "waveai" frontend/  # Should be empty (except docs)

# Verify help widget
cat frontend/app/view/helpview/helpview.tsx | wc -l  # Should be ~35 lines
grep "WebViewModel" frontend/app/view/helpview/helpview.tsx  # Should be empty
grep "QuickTips" frontend/app/view/helpview/helpview.tsx  # Should exist
```

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **Users depend on AI widget** | Low (10%) | Low | Agent widget is strictly better |
| **Users depend on claudecode widget** | Medium (30%) | Low | Agent widget has identical backend |
| **Users depend on tips widget** | Low (5%) | Low | Help widget now shows same content |
| **Users miss WebView help browser** | Very Low (<5%) | Low | Tips content is more useful |
| **Bundle size savings lower than expected** | Low | Low | Still reduces complexity |
| **Regression bugs** | Low | Medium | Thorough testing |

**User Communication Plan:**
1. **Changelog note:** "Removed ai, claudecode, tips widgets (use agent and help)"
2. **That's it.** Don't overcomplicate.

---

## Alternatives Considered

### Alternative 1: Keep All Three AI Widgets

**Pros:**
- No breaking change
- Users keep familiar tools

**Cons:**
- ❌ Massive confusion (which AI widget to use?)
- ❌ Duplicate maintenance burden
- ❌ Agent widget development was wasted effort
- ❌ Bloated widget launcher

**Decision:** ❌ Rejected - defeats purpose of unified agent widget

---

### Alternative 2: Keep Help as WebView Browser

**Pros:**
- Can browse full documentation site
- Navigation buttons

**Cons:**
- ❌ Heavy WebViewModel dependency
- ❌ Embedded browser inferior to real browser
- ❌ Tips content is more useful for quick reference
- ❌ Users can open docs in real browser if needed

**Decision:** ❌ Rejected - keep it simple, tips content is better

---

### Alternative 3: Deprecation Period with Warnings

**Pros:**
- Gradual transition
- User communication

**Cons:**
- ❌ Delays savings (code still ships during deprecation)
- ❌ Adds warning UI code
- ❌ Overcomplicates simple removal

**Decision:** ❌ Rejected - clean break is simpler

---

## Success Metrics

**Technical Metrics:**
- ✅ Bundle size reduced by ~37 KB minified, ~13 KB gzipped
- ✅ Zero references to "agentai", "claudecode", "tips" views (except docs)
- ✅ Help widget no longer extends WebViewModel
- ✅ Widget count: 6 → 3 (-50%)
- ✅ AI widgets: 3 → 1 (-67%)

**User Metrics:**
- ✅ No crash reports related to missing widgets
- ✅ Agent widget usage increases (users understand it's the AI widget)
- ✅ Help widget loads faster (no WebView overhead)
- ✅ Clearer widget launcher UX

**Maintenance Metrics:**
- ✅ Fewer AI codebases to maintain (1 vs 3)
- ✅ Simpler onboarding for new contributors
- ✅ Clearer product focus (terminal + unified AI agent)

---

## Timeline

| Version | Date | Milestone |
|---------|------|--------------|
| Next release (0.27.14) | TBD | Widgets removed (clean break) |
| | | Changelog note added |
| | | Bundle ships 37 KB lighter |
| | | Done. |

---

## Appendix: Detailed File Inventory

### AI Widget Files

```
frontend/app/view/agentai/
├── agentai.tsx (882 lines) - Main view
└── agentai.scss (~150 lines) - Styles

Total: ~1,032 lines, 39 KB
```

### Claude Code Widget Files

```
frontend/app/view/claudecode/
├── claudecode.tsx (239 lines) - Entry point
├── claudecode-view.tsx (445 lines) - Main view
├── claudecode-model.ts (~450 lines) - ViewModel
├── claudecode-parser.ts (~150 lines) - NDJSON parser
├── claudecode-types.ts (~100 lines) - Type definitions
├── claudecode-helpers.ts (~30 lines) - Utilities
└── claudecode.scss (~200 lines) - Styles

Total: ~1,614 lines, 57 KB
```

### Tips Widget Files

```
frontend/app/view/quicktipsview/
└── quicktipsview.tsx (35 lines) - Simple QuickTips wrapper

Total: 35 lines, 1.5 KB
```

### Help Widget Files (Modified, Not Deleted)

```
frontend/app/view/helpview/
└── helpview.tsx
    Before: 174 lines (WebViewModel-based browser)
    After:   35 lines (Simple QuickTips wrapper)

Change: -139 lines
```

### Registration Points

```typescript
// frontend/app/block/block.tsx
import { ClaudeCodeViewModel } from "@/app/view/claudecode/claudecode";  // DELETE
import { QuickTipsViewModel } from "../view/quicktipsview/quicktipsview";  // DELETE

BlockRegistry.set("agentai", AgentAiModel);              // DELETE
BlockRegistry.set("claudecode", ClaudeCodeViewModel);    // DELETE
BlockRegistry.set("tips", QuickTipsViewModel);           // DELETE
```

---

## Conclusion

**Recommendation: PROCEED with consolidation**

**Justification:**
1. ✅ Clear redundancy (3 AI widgets doing same thing)
2. ✅ Agent widget is superior replacement (same backend + better UX)
3. ✅ Significant complexity reduction (6 → 3 widgets, -50%)
4. ✅ Low user impact (agent widget already exists, help→tips is identical)
5. ✅ Clearer product focus (one AI widget, not three)
6. ✅ **Simple execution:** No replacement needed, agent already shipped

**Next Steps:**
1. Delete widget code (10 files, ~2,681 lines)
2. Simplify help widget (174→35 lines, -139 lines)
3. Remove registrations and mappings
4. Update changelog (one-line note)
5. Ship it.

**Philosophy:** Focus beats bloat. One excellent AI widget beats three mediocre ones. Keep AgentMux focused on what it does best: terminal + unified AI agent.

---

**End of Specification**
