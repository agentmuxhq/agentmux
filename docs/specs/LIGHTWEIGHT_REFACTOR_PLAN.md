# AgentMux Lightweight Refactor Plan

## Overview

This document outlines features that can be removed or simplified to create a lighter AgentMux build optimized for agent workflows.

---

## Priority 1: Immediate Removals (Quick Wins)

### 1. Startup Tutorial/Onboarding - DISABLE FIRST

**Files:**
- `frontend/app/onboarding/` (76KB, 6 files)
- `frontend/app/modals/modalsrenderer.tsx` (trigger logic)

**How to Disable:**
```typescript
// In modalsrenderer.tsx, comment out or remove:
// Lines 32-36: New install modal trigger
// Lines 38-49: Upgrade modal trigger
```

**Or set TOS agreed by default in backend.**

---

### 2. Telemetry System - REMOVE

**Files to Delete:**
- `pkg/telemetry/` (28KB, 656 LOC)
- `pkg/wcloud/` (306 LOC) - telemetry sending

**Frontend References to Remove:**
- Telemetry toggle in onboarding
- `telemetry:enabled` settings references

**Estimated Savings:** ~50KB code + DB overhead

---

### 3. Documentation Site - REMOVE

**Directory to Delete:**
- `docs/` (9MB total)

**Backend to Simplify:**
- `pkg/docsite/docsite.go` (1.1KB) - remove or stub

**Estimated Savings:** ~9MB

---

### 4. Storybook - REMOVE

**Files to Delete:**
- `.storybook/` directory (21KB)
- All `*.stories.tsx` files (16 files)

**package.json Changes:**
Remove all `@storybook/*` dev dependencies

**Estimated Savings:** ~50MB dev dependencies

---

## Priority 2: Feature Removals (Medium Effort)

### 5. AI/Chat System - REMOVE

**Frontend Files:**
- `frontend/app/aipanel/` (120KB, 14 files, 2,572 LOC)
- `frontend/app/view/chat/` (53KB, 7 files)
- `frontend/app/view/waveai/` (44KB)

**Backend Files:**
- `pkg/waveai/` (44KB, 6 Go files, 1,049 LOC)
  - waveai.go
  - anthropicbackend.go
  - googlebackend.go
  - openaibackend.go
  - perplexitybackend.go
  - cloudbackend.go

**Config:**
- `pkg/wconfig/defaultconfig/presets/ai.json`

**package.json Dependencies to Remove:**
```json
"@ai-sdk/react": "^2.0.76",
"ai": "^5.0.44"
```

**Integration Points to Update:**
- `frontend/app/workspace/workspace-layout-model.tsx` - Remove AI panel init
- `frontend/app/store/keymodel.ts` - Remove AI keybindings
- `frontend/app/modals/` - Remove AI-related modals

**Estimated Savings:** ~250KB code + ~10MB dependencies

---

### 6. Heavy Visualization Libraries - REMOVE

**Dependencies:**
| Package | Size | Used By |
|---------|------|---------|
| `recharts` | 2.5MB | donutchart.tsx, chart.tsx |
| `@observablehq/plot` | 1.2MB | sysinfo.tsx |
| `mermaid` | 3MB | markdown preview |

**Files to Simplify/Remove:**
- `frontend/app/element/donutchart.tsx`
- `frontend/app/shadcn/chart.tsx`
- `frontend/app/view/sysinfo/sysinfo.tsx` (or simplify)

**Estimated Savings:** ~6MB dependencies

---

### 7. Preview System - SIMPLIFY

**Files (140KB, 10 components):**
- `frontend/app/view/preview/`
  - csvview.tsx (uses papaparse)
  - markdownview.tsx
  - previewmodel.tsx
  - previewview.tsx
  - etc.

**Consider:** Keep basic file viewing, remove advanced features

**Estimated Savings:** ~140KB+ code

---

## Priority 3: Optional Removals

### 8. Unused Dependencies

```json
// package.json - potentially removable
"html-to-image": "1.11.13",      // screenshot/export
"react-zoom-pan-pinch": "3.7.0", // preview zooming
"react-frame-component": "5.2.7", // iframe preview
"fast-average-color": "9.5.0",   // image color extraction
"pngjs": "^7.0.0"                // PNG manipulation
```

---

## Implementation Order

### Phase 1: Disable Tutorial (Immediate)
1. Edit `modalsrenderer.tsx` to skip onboarding triggers
2. Test build

### Phase 2: Remove Telemetry + Docs + Storybook
1. Delete `pkg/telemetry/`, `pkg/wcloud/`
2. Delete `docs/` directory
3. Delete `.storybook/` and story files
4. Update package.json
5. Test build

### Phase 3: Remove AI System
1. Delete AI frontend directories
2. Delete AI backend directory
3. Remove AI dependencies
4. Update workspace layout model
5. Test build

### Phase 4: Simplify Views
1. Remove heavy viz libraries
2. Simplify preview system
3. Remove unused dependencies
4. Test build

---

## Summary Table

| Feature | Size | Priority | Effort | Status |
|---------|------|----------|--------|--------|
| Tutorial/Onboarding | 76KB | P1 | Low | TODO |
| Telemetry | 50KB | P1 | Low | TODO |
| Docs Site | 9MB | P1 | Low | TODO |
| Storybook | 50MB dev | P1 | Low | TODO |
| AI/Chat | 250KB + 10MB deps | P2 | Medium | TODO |
| Viz Libraries | 6MB | P2 | Medium | TODO |
| Preview System | 140KB | P3 | Medium | TODO |
| Misc Deps | 3MB | P3 | Low | TODO |

**Total Potential Savings: ~15-20MB code/deps**

---

## Notes

- Keep core terminal functionality intact
- Keep wsh shell integration
- Keep basic file viewing
- Remove all cloud/telemetry phone-home
- Remove marketing/onboarding fluff
