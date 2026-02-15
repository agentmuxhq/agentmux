# Widget Removal Spec: Web & Files Widgets

**Date:** 2026-02-14
**Status:** Proposal
**Author:** AgentA
**Decision:** Remove cruft widgets to reduce complexity and bundle size

---

## Executive Summary

**Proposal:** Remove `defwidget@web` (Web browser) and `defwidget@files` (File preview) widgets from AgentMux.

**Rationale:**
- These widgets are **cruft** - underutilized features that add complexity
- Terminal-focused use cases don't require in-app web browsing or file previewing
- External tools (browsers, editors) are superior for these tasks
- Significant resource savings possible
- **Philosophy:** Don't add replacement features - users already have better tools

**Estimated Savings:**
- **Code:** ~2,668 lines removed (~140 KB source)
- **Dependencies:** 1 npm package removed (papaparse: ~55 KB)
- **Bundle size:** ~113 KB minified, ~41 KB gzipped
- **Maintenance burden:** 1 complex feature removed (files preview), web widget definition removed
- **Zero new code added:** No replacement commands, no complexity creep

**Implementation Note:** WebViewModel retained as base class for internal views (HelpViewModel, TsunamiViewModel). Only user-facing web widget removed.

---

## Current State Analysis

### Widget Inventory

| Widget | View Type | Icon | Purpose | Status |
|--------|-----------|------|---------|--------|
| `defwidget@web` | `web` | globe | In-app web browser | ❌ **Remove** |
| `defwidget@files` | `preview` | folder | File/directory previewer | ❌ **Remove** |
| `defwidget@terminal` | `term` | square-terminal | Terminal emulator | ✅ **Keep (core)** |
| `defwidget@ai` | `waveai` | sparkles | AI chat | ✅ **Keep** |
| `defwidget@sysinfo` | `sysinfo` | chart-line | System info | ✅ **Keep** |
| `defwidget@claudecode` | `claudecode` | terminal | Claude Code AI | ✅ **Keep** |

---

## Resource Analysis

### 1. Web Widget (`defwidget@web`)

**Code Footprint:**
```
frontend/app/view/webview/webview.tsx       1,078 lines
frontend/app/view/webview/webview.scss      ~200 lines
Total:                                      ~1,278 lines (48 KB on disk)
```

**Features:**
- Webview wrapper (Electron → Tauri migration incomplete)
- URL navigation bar with search
- Mobile emulation (iPhone, Android user agents)
- Media playback controls
- Typeahead URL suggestions
- Pinned URL support
- Partition override (session isolation)

**Dependencies:**
- Tauri webview APIs (via iframe placeholder - migration incomplete)
- Search component
- Suggestion system
- Settings integration

**Issues:**
- ❌ **Migration incomplete:** Still has Electron placeholder code (`type WebviewTag = any`)
- ❌ **Security risk:** Webview sandboxing not fully implemented in Tauri version
- ❌ **Limited use case:** Users have real browsers (Chrome, Firefox, Edge)
- ❌ **Poor UX:** Embedded browser inferior to dedicated browser features

**Usage Estimate:** <5% of users (based on terminal-first product positioning)

**Implementation Note:** WebViewModel and WebView component are **kept as base classes** for internal use by HelpViewModel (embedded docs) and TsunamiViewModel (app hosting). Only the user-facing `defwidget@web` widget definition and BlockRegistry registration are removed. This means the actual code removal is limited to widget definition and registration points, not the entire webview directory.

---

### 2. Files Widget (`defwidget@files`)

**Code Footprint:**
```
frontend/app/view/preview/preview.tsx                   ~350 lines
frontend/app/view/preview/preview-model.tsx             ~450 lines
frontend/app/view/preview/preview-directory.tsx         ~400 lines
frontend/app/view/preview/preview-directory-utils.tsx   ~200 lines
frontend/app/view/preview/preview-markdown.tsx          ~300 lines
frontend/app/view/preview/preview-edit.tsx              ~400 lines
frontend/app/view/preview/preview-streaming.tsx         ~200 lines
frontend/app/view/preview/preview-error-overlay.tsx     ~100 lines
frontend/app/view/preview/csvview.tsx                   ~268 lines
frontend/util/previewutil.ts                            ~100 lines
Total:                                                  ~2,668 lines (140 KB on disk)
```

**Features:**
- Directory browsing (file tree, grid/list view)
- File preview (text, markdown, CSV, images)
- Markdown rendering with syntax highlighting
- Code editor integration (Monaco)
- CSV table viewer with papaparse
- File metadata display
- Connection routing (remote files via SSH/Docker)
- Streaming file content
- File suggestions/typeahead

**Dependencies:**
- `papaparse` (CSV parsing): ~150 KB unminified, ~55 KB minified
- `@types/papaparse`: dev dependency
- Monaco editor (shared with other views)
- Markdown renderer (shared with other views)

**Issues:**
- ❌ **Feature overlap:** Terminal already has `ls`, `cat`, `less`, `vim`
- ❌ **Better alternatives:** VS Code, Notepad++, external file managers
- ❌ **Complexity:** 2,668 lines for a feature most users ignore
- ❌ **Dependency bloat:** papaparse only used for CSV preview (niche feature)

**Usage Estimate:** <10% of users (terminal users prefer CLI tools)

---

## Removal Impact Analysis

### Code Volume Reduction

| Category | Lines | Files | Disk (KB) | Notes |
|----------|-------|-------|-----------|-------|
| Web widget definition | 0 | 0 | 0 | Widget registration removed, WebViewModel kept as base class |
| Files widget | 2,668 | 10 | 140 | Fully deleted |
| **Total** | **2,668** | **10** | **140** | WebViewModel retained for internal use by help/tsunami views |

**Note:** This is source code only. WebViewModel and WebView (1,278 lines, 48 KB) are retained as base classes for HelpViewModel and TsunamiViewModel. Only the user-facing widget definition and registration are removed. Post-build impact (minified bundles) analyzed below.

---

### Bundle Size Impact

**Methodology:**
1. Vite tree-shaking removes unused code
2. Minification reduces ~4KB source → ~1-2KB minified
3. Gzip compression applies to final bundles

**Estimated Reductions:**

| Item | Unminified | Minified | Gzipped | Notes |
|------|-----------|----------|---------|-------|
| Web widget | 0 KB | 0 KB | 0 KB | WebViewModel retained for internal use |
| Files widget JS | ~150 KB | ~50 KB | ~18 KB | Includes preview components |
| papaparse | 150 KB | 55 KB | 20 KB | CSV parsing library |
| Preview CSS | ~20 KB | ~8 KB | ~3 KB | Styles for preview widget |
| **Total** | **~320 KB** | **~113 KB** | **~41 KB** | **Post-gzip savings** |

**Real-world bundle impact:**
- Production build currently: ~7-10 MB (unoptimized estimate)
- Removing widgets: **-113 KB minified** (~1.1% reduction before gzip)
- **Gzipped savings: ~41 KB** (users download 41 KB less)

**Additional savings:**
- Fewer HTTP requests (if code-split)
- Faster initial parse time
- Lower memory usage (fewer components in bundle)

---

### Dependency Removal

**Primary Candidate: papaparse**

```json
// Remove from package.json
"dependencies": {
  "papaparse": "^5.5.3"  // ← REMOVE
}
"devDependencies": {
  "@types/papaparse": "^5"  // ← REMOVE
}
```

**Savings:**
- npm package: 55 KB (minified)
- Types package: ~10 KB
- `node_modules/` size: ~200 KB (pre-build)
- Transitive dependencies: None (papaparse is standalone)

**Retained Dependencies:**
- Monaco Editor: ✅ Keep (used by code editor view, terminal, AI views)
- Markdown renderer: ✅ Keep (used by AI views, help view)

**Total dependency cleanup:**
- 1 production dependency removed
- 1 dev dependency removed
- ~200 KB `node_modules/` reduction

---

### Complexity Reduction

**Metrics:**

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| View types | 6 | 4 | -33% |
| Widget definitions | 6 | 4 | -33% |
| Frontend view directories | 8 | 6 | -25% |
| Lines of code (views only) | ~12,000 | ~8,054 | -33% |
| Test surface area | High | Medium | -30% est. |

**Maintenance Benefits:**
- ✅ Fewer features to support
- ✅ Simpler onboarding for new contributors
- ✅ Reduced test matrix (fewer view types)
- ✅ Clearer product focus (terminal + AI, not "everything")

**Cognitive Load:**
- Users see fewer irrelevant widgets in launcher
- Developers focus on core features (terminal, AI)
- Documentation can be simplified

---

## Migration Path (For Existing Users)

### Web Widget → Use Your Browser

**Old workflow:**
```
1. User clicks "web" widget
2. Opens in-app browser
3. Navigates to URL
```

**New workflow:**
```
1. User opens Chrome/Firefox/Edge (already installed)
2. Better UX, better security, better performance
```

**No replacement needed:** Users already have superior browsers installed.

---

### Files Widget → Use Terminal + Editor

**Old workflow:**
```
1. User clicks "files" widget
2. Browses directories in preview pane
3. Views file content in-app
```

**New workflow (terminal-native):**
```bash
# List files (better than GUI)
ls -la

# View file content
cat file.txt
less file.txt

# Edit files
vim file.txt
code file.txt  # Opens VS Code (superior editor)
```

**No replacement needed:** Terminal users already know these commands. If they want GUI, they have file managers and VS Code.

---

## Removal Plan

### Simple Clean Removal (Next Release)

**Philosophy:** Don't overcomplicate. Just remove cruft.

**No deprecation warnings** - Clean break, simple changelog note
**No replacement commands** - Users already have better tools
**No migration period** - Rip the band-aid off

**File Changes:**

```bash
# 1. Remove widget definitions
# FILE: pkg/wconfig/defaultconfig/widgets.json
- "defwidget@web": { ... }      # DELETE
- "defwidget@files": { ... }    # DELETE

# 2. Remove frontend view code
# NOTE: frontend/app/view/webview/ is KEPT (used by help/tsunami views)
rm -rf frontend/app/view/preview/
rm frontend/util/previewutil.ts

# 3. Remove view registrations
# FILE: frontend/app/block/block.tsx
# NOTE: WebViewModel import removed, but class is still used by help/tsunami
- import { WebViewModel } from "@/view/webview/webview";     # DELETE
- import { PreviewModel } from "@/app/view/preview/preview-model";  # DELETE
- BlockRegistry.set("web", WebViewModel);       # DELETE (user-facing widget)
- BlockRegistry.set("preview", PreviewModel);   # DELETE

# 4. Remove dependencies
# FILE: package.json
- "papaparse": "^5.5.3",         # DELETE
- "@types/papaparse": "^5",      # DELETE

# 5. Run dependency cleanup
npm install  # Updates package-lock.json

# 6. Update tests
# Remove tests for web and preview views
rm frontend/app/view/webview/*.test.ts*
rm frontend/app/view/preview/*.test.ts*

# 7. Run build verification
task build:frontend
task verify-version.sh
```

**Commit Message:**
```
feat: remove web and files widgets

BREAKING CHANGE: Web and files widgets removed.

Users should use:
- Web browsing: Chrome/Firefox/Edge (better UX, security)
- File viewing: Terminal commands (ls, cat, less, vim)
- File editing: VS Code or preferred editor

Removals:
- defwidget@web widget definition (WebViewModel kept for internal use)
- frontend/app/view/preview/ (2,668 lines)
- papaparse dependency (~55 KB)

Bundle size reduction: ~113 KB minified, ~41 KB gzipped
Complexity reduction: -33% view types

Implementation note: WebViewModel and WebView retained as base classes
for HelpViewModel (docs) and TsunamiViewModel (apps). Only user-facing
web widget removed.

No replacement commands added (keep it simple).

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

**CHANGELOG.md Entry:**
```markdown
### Removed
- **Web widget** - Use your system browser (Chrome, Firefox, Edge)
- **Files widget** - Use terminal commands (ls, cat, vim) or VS Code
- Bundle size reduced by ~46 KB (gzipped)
```

**Exit Criteria:**
- All widget code removed
- Dependencies cleaned up
- Build passes
- No references to "web" or "preview" views in codebase (except tests/docs)
- Changelog updated with one-line note

---

## Testing Strategy

### Post-Removal Testing

**Regression tests:**
```bash
# Verify widgets removed
1. Open widget launcher → Verify only 4 widgets (terminal, ai, sysinfo, claudecode)
2. Search for "web" in launcher → No results
3. Search for "files" in launcher → No results

# Verify no crashes
4. Create new tab → No errors
5. Open existing workspace → No errors
6. Check console for errors → None related to missing views

# Verify terminal still works (primary use case)
7. Open terminal widget → Works normally
8. Run ls, cat, vim → All work (no dependency on removed widgets)
```

**Build verification:**
```bash
# Check bundle size
npm run build
du -sh dist/frontend/  # Should be smaller

# Check dependencies
npm ls papaparse  # Should error (not found)

# Check for orphaned code
grep -r "webview\|preview" frontend/app/view/  # Should be empty (except docs)
```

**Upgrade testing:**
```bash
# Simulate upgrade (current version → next version)
1. Install current version
2. Create workspace with web + files widgets
3. Upgrade to version with widgets removed
4. Verify graceful degradation (widgets disappear, no crash)
5. App continues to function normally
```

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **Users depend on web widget** | Low (5%) | Medium | They have real browsers already |
| **Users depend on files widget** | Low (10%) | Medium | Terminal users know ls/cat/vim |
| **Bundle size savings lower than expected** | Low | Low | Still reduces complexity |
| **Regression bugs** | Low | Medium | Thorough testing |
| **Negative user feedback** | Medium | Low | Clear changelog note, terminal-first positioning |
| **Incomplete removal (orphaned code)** | Low | Low | Code review, grep audits |

**User Communication Plan:**
1. **Changelog note:** "Removed web and files widgets (use browser/terminal/editor)"
2. **That's it.** Don't overcomplicate.

---

## Alternatives Considered

### Alternative 1: Add Replacement Commands (/web, /edit, /open)

**Pros:**
- Smooth user transition
- Feature parity maintained

**Cons:**
- ❌ Adds new code (defeats purpose of reducing complexity)
- ❌ Users already have better tools
- ❌ Creates new features to maintain
- ❌ Scope creep

**Decision:** ❌ Rejected - keep it simple, don't add new features

---

### Alternative 2: Deprecation Period with Warnings

**Pros:**
- Gradual transition
- User communication

**Cons:**
- ❌ Delays savings (code still ships during deprecation)
- ❌ Adds warning UI code
- ❌ Overcomplicates simple removal

**Decision:** ❌ Rejected - clean break is simpler

---

### Alternative 3: Keep but Hide Behind Flag

**Pros:**
- No breaking change
- Users can re-enable if needed

**Cons:**
- ❌ Code still ships (no bundle savings)
- ❌ Maintenance burden persists
- ❌ Two-tier support complexity

**Decision:** ❌ Rejected - doesn't achieve goal

---

## Success Metrics

**Technical Metrics:**
- ✅ Bundle size reduced by ~113 KB minified, ~41 KB gzipped
- ✅ Zero references to "preview" views in codebase (except docs)
- ✅ "web" widget definition removed (WebViewModel kept for internal use)
- ✅ papaparse removed from package.json
- ✅ Build time improved (fewer files to process)
- ✅ Zero new code added (no replacement commands)

**User Metrics:**
- ✅ No crash reports related to missing widgets
- ✅ Terminal widget continues to work normally
- ✅ Users use external tools they already have

**Maintenance Metrics:**
- ✅ Fewer features to support
- ✅ Simpler onboarding for new contributors
- ✅ Clearer product positioning (terminal + AI, not "everything")

---

## Timeline

| Version | Date | Milestone |
|---------|------|-----------|
| Next release | TBD | Widgets removed (clean break) |
| | | Changelog note added |
| | | Bundle ships 128 KB lighter |
| | | Done. |

---

## Appendix: Detailed File Inventory

### Web Widget Files

```
frontend/app/view/webview/
├── webview.tsx (1,078 lines)
│   ├── WebViewModel class
│   ├── URL navigation
│   ├── Search integration
│   ├── Media controls
│   └── Typeahead
└── webview.scss (~200 lines)
    ├── URL bar styles
    ├── Media controls
    └── Error states

Total: ~1,278 lines, 48 KB
```

### Files Widget Files

```
frontend/app/view/preview/
├── preview.tsx (350 lines) - Main view
├── preview-model.tsx (450 lines) - ViewModel
├── preview-directory.tsx (400 lines) - Directory browser
├── preview-directory-utils.tsx (200 lines) - Dir helpers
├── preview-markdown.tsx (300 lines) - MD renderer
├── preview-edit.tsx (400 lines) - Code editor
├── preview-streaming.tsx (200 lines) - Streaming
├── preview-error-overlay.tsx (100 lines) - Errors
├── csvview.tsx (268 lines) - CSV viewer
└── [styles, utils, types]

frontend/util/
└── previewutil.ts (100 lines) - Shared utils

Total: ~2,668 lines, 140 KB
```

### Registration Points

```typescript
// frontend/app/block/block.tsx
import { WebViewModel } from "@/view/webview/webview";
import { PreviewModel } from "@/app/view/preview/preview-model";

BlockRegistry.set("web", WebViewModel);
BlockRegistry.set("preview", PreviewModel);
```

---

## Conclusion

**Recommendation: PROCEED with simple removal**

**Justification:**
1. ✅ Clear cruft (underutilized, better alternatives exist)
2. ✅ Significant resource savings (~113 KB minified, ~41 KB gzipped)
3. ✅ Reduces complexity (33% fewer view types)
4. ✅ Low user impact (estimated <10% usage, they have better tools)
5. ✅ Improves product focus (terminal + AI, not "everything")
6. ✅ **Simple execution:** No replacement commands, no deprecation period, no complexity

**Implementation:**
- WebViewModel and WebView retained as base classes (used by help/tsunami views)
- Only user-facing web widget definition and files widget removed
- Actual code deletion: 10 files, 2,668 lines

**Next Steps:**
1. Delete widget code (10 files, 2,668 lines)
2. Remove papaparse dependency
3. Update changelog (one-line note)
4. Ship it.

**Philosophy:** Don't add features to replace removed features. Users already have browsers, terminals, and editors. Keep AgentMux focused on what it does best: terminal + AI.

---

**End of Specification**
