# PR #305 Analysis: Unified Agent Widget State Management Issues

**Date**: 2026-02-15
**PR**: #305 - Phase 5: Unified Agent Widget Registration & Integration
**Status**: Changes Requested (ReAgent Review)
**Branch**: `agentx/unified-agent-widget-phase5`

---

## Executive Summary

PR #305 completes the final phase of the Unified Agent Widget feature, which consolidates AgentMux's AI agent interface into a single, reusable widget. While the feature is functionally complete, ReAgent identified **critical architectural issues** that prevent multiple instances of the widget from working correctly.

**Root Cause:** Global singleton Jotai atoms cause state to bleed between widget instances.

**Impact:** Opening 2+ agent widgets = shared state (messages, documents, process status).

**Fix Complexity:** Medium - requires refactoring atoms to be per-instance.

---

## Feature Background

### What is the Unified Agent Widget?

A single, comprehensive widget that replaces two separate widgets:
- `defwidget@ai` - Old AI panel
- `defwidget@claudecode` - Old Claude Code widget

**New widget:** `defwidget@agent`

### Multi-Phase Implementation

| Phase | PR | Status | Description |
|-------|-----|--------|-------------|
| **Phase 1** | #296 | ✅ Merged | Streaming intermediary (NDJSON → DocumentNode parser) |
| **Phase 2** | #298 | ✅ Merged | Interactive markdown UI (collapsible tools, syntax highlighting) |
| **Phase 3** | #299 | ✅ Merged | Multi-agent features (messaging, filtering, controls) |
| **Phase 5** | **#305** | **⚠️ This PR** | **Widget registration & integration** |
| Phase 4 | N/A | Optional | Smart sections, tool grouping (deferred) |

**Why Phase 5 before Phase 4?** Phase 5 wires everything together for testing. Phase 4 is optional enhancements.

### How Did This Come Up Now?

**Timeline:**
1. **Earlier today** - User and AgentX worked on robust shell integration (PR #304)
2. **PR #304 merged** - Bumped version to 0.27.10
3. **User requested** - "Check reviews, merge once approved"
4. **Found PR #305** - Open PR from earlier work, now has merge conflicts
5. **Rebased on main** - Triggered ReAgent re-review
6. **ReAgent flagged issues** - 3 critical problems discovered

**This PR was created before shell integration work, sat waiting for review, and surfaced issues when rebased on 0.27.10.**

---

## ReAgent Review Findings

### Issue 1: Missing Version Bump ❌

**Location:** `package.json:3`

**Problem:** PR adds 300 lines of new code but doesn't bump version.

**ReAgent Message:**
```
Missing version bump for code changes (version field unchanged from main branch 0.27.10)
```

**Fix:** Bump to 0.27.11

**Why this matters:** All code changes require version bumps per CLAUDE.md guidelines.

---

### Issue 2: Global Singleton Atoms ❌ CRITICAL

**Location:** `frontend/app/view/agent/agent-model.ts:53`

**Problem:** Atoms are defined at module scope = global singletons.

**Code:**
```typescript
// ❌ WRONG - Global atoms shared across all instances
export const agentDocumentAtom = atom<DocumentNode[]>([]);
export const agentIdAtom = atom<string | null>(null);
export const agentProcessAtom = atom<AgentProcess | null>(null);
export const filteredDocumentAtom = atom<DocumentNode[]>([]);
```

**Impact:**
- User opens **Agent Widget A** (blockId: "abc123")
- User opens **Agent Widget B** (blockId: "def456")
- Widget B updates `agentDocumentAtom` → **Widget A's document also changes**
- Both widgets show the same agent output, process status, and state

**Why this happens:**

Jotai atoms are singletons when defined at module scope. The AgentMux block system creates multiple instances of `AgentViewModel`, but they all share the same atoms.

**Example scenario:**

```
Widget Instance 1 (Block ID: block-1):
  AgentViewModel instance A
    ↓ uses
  agentDocumentAtom (GLOBAL)
  agentIdAtom (GLOBAL)

Widget Instance 2 (Block ID: block-2):
  AgentViewModel instance B
    ↓ uses
  agentDocumentAtom (GLOBAL)  ← Same atoms!
  agentIdAtom (GLOBAL)        ← Same atoms!

Result: Widget 2's changes overwrite Widget 1's state
```

**ReAgent Message:**
```
Global singleton atoms (agentDocumentAtom, agentIdAtom, agentProcessAtom)
shared across all widget instances - state will bleed between multiple agent widgets
```

---

### Issue 3: Shared Document State ❌ CRITICAL

**Location:** `frontend/app/view/agent/agent-view.tsx:63`

**Problem:** View component uses global `filteredDocumentAtom` directly.

**Code:**
```typescript
// ❌ WRONG - Uses global atom
const filteredDocument = useAtomValue(filteredDocumentAtom);
```

**Impact:** Same as Issue #2 - all instances share the same filtered document state.

**ReAgent Message:**
```
Using global filteredDocumentAtom atom - multiple agent widgets will share document state
```

---

## Root Cause Analysis

### Architecture Mismatch

**AgentMux Block System:**
- Designed for **multiple independent instances** of the same widget type
- Each instance has a unique `blockId`
- Instances should have **isolated state**

**Current Implementation:**
- Uses **global Jotai atoms** (module-level singletons)
- Assumes **only one instance** will ever exist
- No per-instance state isolation

### Why Global Atoms?

Likely copied pattern from `claudecode` widget, which assumes single instance:

```typescript
// Old pattern (works for single-instance widgets)
const documentAtom = atom<Document>(...);

export function ClaudeCodeView() {
  const doc = useAtomValue(documentAtom);
  // Only one widget = no problem
}
```

**Problem:** This pattern doesn't scale to multiple instances.

---

## Solution: Per-Instance Atom Stores

### Architecture: Atom Store Pattern

**Key Insight:** Create atom store keyed by `blockId`.

**Pattern:**
```typescript
// Map of blockId → atoms for that instance
const atomStores = new Map<string, {
  documentAtom: Atom<DocumentNode[]>;
  agentIdAtom: Atom<string | null>;
  processAtom: Atom<AgentProcess | null>;
}>();

function getOrCreateAtomStore(blockId: string) {
  if (!atomStores.has(blockId)) {
    atomStores.set(blockId, {
      documentAtom: atom<DocumentNode[]>([]),
      agentIdAtom: atom<string | null>(null),
      processAtom: atom<AgentProcess | null>(null)
    });
  }
  return atomStores.get(blockId)!;
}
```

**Now each widget instance gets its own atoms:**

```
Widget Instance 1 (Block ID: block-1):
  AgentViewModel("block-1")
    ↓ uses
  atomStores.get("block-1").documentAtom
  atomStores.get("block-1").agentIdAtom

Widget Instance 2 (Block ID: block-2):
  AgentViewModel("block-2")
    ↓ uses
  atomStores.get("block-2").documentAtom  ← Different atoms!
  atomStores.get("block-2").agentIdAtom   ← Different atoms!

Result: Widgets have isolated state ✅
```

---

## Implementation Fix

### Fix 1: Add Version Bump

**File:** `package.json`

```diff
- "version": "0.27.10",
+ "version": "0.27.11",
```

**Also update:**
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src-tauri/tauri.conf.json`
- `cmd/server/main-server.go` (ExpectedVersion constant)
- `VERSION_HISTORY.md`

**Use bump script:**
```bash
./bump-version.sh patch --message "Unified agent widget phase 5"
```

---

### Fix 2: Refactor agent-model.ts to Per-Instance Atoms

**File:** `frontend/app/view/agent/agent-model.ts`

**Current Code (WRONG):**

```typescript
// Global atoms - shared across all instances
export const agentDocumentAtom = atom<DocumentNode[]>([]);
export const agentIdAtom = atom<string | null>(null);
export const agentProcessAtom = atom<AgentProcess | null>(null);

export class AgentViewModel implements ViewModel {
  constructor(public blockId: string) {
    this.blockAtom = ...;
  }

  // Uses global atoms - all instances share state
  getDocumentAtom() {
    return agentDocumentAtom;
  }
}
```

**Fixed Code:**

```typescript
// Per-instance atom store
interface AgentAtomStore {
  documentAtom: PrimitiveAtom<DocumentNode[]>;
  agentIdAtom: PrimitiveAtom<string | null>;
  processAtom: PrimitiveAtom<AgentProcess | null>;
  filteredDocumentAtom: Atom<DocumentNode[]>;
}

// Map blockId → atom store
const atomStoreMap = new Map<string, AgentAtomStore>();

function getOrCreateAtomStore(blockId: string): AgentAtomStore {
  if (!atomStoreMap.has(blockId)) {
    // Create fresh atoms for this instance
    const documentAtom = atom<DocumentNode[]>([]);
    const agentIdAtom = atom<string | null>(null);
    const processAtom = atom<AgentProcess | null>(null);

    // Derived atom for filtering (per-instance)
    const filteredDocumentAtom = atom((get) => {
      const doc = get(documentAtom);
      const agentId = get(agentIdAtom);
      // Filter logic here
      return doc;
    });

    atomStoreMap.set(blockId, {
      documentAtom,
      agentIdAtom,
      processAtom,
      filteredDocumentAtom
    });
  }

  return atomStoreMap.get(blockId)!;
}

export class AgentViewModel implements ViewModel {
  private atomStore: AgentAtomStore;

  constructor(public blockId: string) {
    // Get per-instance atom store
    this.atomStore = getOrCreateAtomStore(blockId);
    this.blockAtom = ...;
  }

  // Return instance-specific atoms
  getDocumentAtom() {
    return this.atomStore.documentAtom;
  }

  getAgentIdAtom() {
    return this.atomStore.agentIdAtom;
  }

  getProcessAtom() {
    return this.atomStore.processAtom;
  }

  getFilteredDocumentAtom() {
    return this.atomStore.filteredDocumentAtom;
  }
}
```

---

### Fix 3: Update agent-view.tsx to Use Instance Atoms

**File:** `frontend/app/view/agent/agent-view.tsx`

**Current Code (WRONG):**

```typescript
export function AgentViewWrapper({ model }: ViewComponentProps<AgentViewModel>) {
  // ❌ Uses global atom directly
  const filteredDocument = useAtomValue(filteredDocumentAtom);

  return (
    <AgentView
      document={filteredDocument}
      onSendMessage={...}
    />
  );
}
```

**Fixed Code:**

```typescript
export function AgentViewWrapper({ model }: ViewComponentProps<AgentViewModel>) {
  // ✅ Get instance-specific atom from model
  const filteredDocument = useAtomValue(model.getFilteredDocumentAtom());

  return (
    <AgentView
      document={filteredDocument}
      onSendMessage={...}
    />
  );
}
```

---

### Fix 4: Cleanup on Widget Close

**Important:** Add cleanup logic to prevent memory leaks.

**Add to AgentViewModel:**

```typescript
export class AgentViewModel implements ViewModel {
  // ... existing code ...

  dispose() {
    // Clean up atom store when widget is closed
    atomStoreMap.delete(this.blockId);

    // Cancel any pending subscriptions
    if (this.fileSubject) {
      this.fileSubject.release();
    }
  }
}
```

**Ensure `dispose()` is called** when widget is closed (check block system integration).

---

## Complete Implementation Checklist

### Step 1: Version Bump

- [ ] Run `./bump-version.sh patch --message "Unified agent widget phase 5"`
- [ ] Verify all version files updated (run `bash scripts/verify-version.sh`)

### Step 2: Refactor Atoms

- [ ] Create `AgentAtomStore` interface
- [ ] Create `atomStoreMap` Map
- [ ] Implement `getOrCreateAtomStore(blockId)` function
- [ ] Update `AgentViewModel` constructor to get per-instance store
- [ ] Add getter methods: `getDocumentAtom()`, `getAgentIdAtom()`, etc.
- [ ] Update all internal references to use `this.atomStore.XXX`

### Step 3: Update View Component

- [ ] Update `AgentViewWrapper` to use `model.getFilteredDocumentAtom()`
- [ ] Update any other direct atom references to use model getters

### Step 4: Add Cleanup

- [ ] Implement `dispose()` method in `AgentViewModel`
- [ ] Call `atomStoreMap.delete(blockId)` in dispose
- [ ] Verify block system calls `dispose()` on widget close

### Step 5: Test Multi-Instance

- [ ] Build: `task dev`
- [ ] Open Agent Widget #1
- [ ] Send message to Widget #1
- [ ] Open Agent Widget #2
- [ ] Send message to Widget #2
- [ ] **Verify:** Both widgets show different messages (no state bleed)
- [ ] Close Widget #1
- [ ] **Verify:** Widget #2 still works
- [ ] **Verify:** No memory leaks (atom store cleaned up)

---

## Alternative Solutions Considered

### Option 1: Jotai Provider Scoping

**Approach:** Use Jotai `Provider` to create isolated atom scopes.

```typescript
<Provider scope={blockId}>
  <AgentView />
</Provider>
```

**Pros:**
- Clean Jotai-native solution
- Automatic cleanup

**Cons:**
- Requires restructuring component tree
- Provider must wrap at block level (may require block system changes)
- More invasive refactor

**Decision:** Not chosen due to complexity.

---

### Option 2: React Context for State

**Approach:** Use React Context instead of Jotai.

```typescript
const AgentContext = createContext<AgentState>(...)

export function AgentViewWrapper({ model }) {
  const [state, setState] = useState(...)

  return (
    <AgentContext.Provider value={state}>
      <AgentView />
    </AgentContext.Provider>
  )
}
```

**Pros:**
- No atom singleton issues
- React-native state management

**Cons:**
- Loses Jotai's performance benefits
- Breaks existing architecture (other widgets use Jotai)
- More boilerplate

**Decision:** Not chosen - keep Jotai consistency.

---

### Option 3: Atom Families (Jotai Built-in)

**Approach:** Use Jotai's `atomFamily` utility.

```typescript
const agentDocumentAtomFamily = atomFamily((blockId: string) =>
  atom<DocumentNode[]>([])
);

// Usage
const docAtom = agentDocumentAtomFamily(blockId);
```

**Pros:**
- Jotai-native solution
- Built-in cleanup with `atomFamily.remove(blockId)`

**Cons:**
- Requires `jotai/utils` dependency
- Slightly different API

**Decision:** Not chosen - manual Map is simpler and more explicit.

---

## **RECOMMENDED SOLUTION:** Manual Atom Store Map

**Why:**
- ✅ Simple and explicit
- ✅ Full control over cleanup
- ✅ No additional dependencies
- ✅ Minimal changes to existing code
- ✅ Easy to understand and maintain

---

## Testing Strategy

### Unit Tests (Optional but Recommended)

**Test: Multiple instances don't share state**

```typescript
describe('AgentViewModel', () => {
  it('should isolate state between instances', () => {
    const model1 = new AgentViewModel('block-1');
    const model2 = new AgentViewModel('block-2');

    // Update model1's document
    const docAtom1 = model1.getDocumentAtom();
    // ... set atom value ...

    // Verify model2's document is unaffected
    const docAtom2 = model2.getDocumentAtom();
    expect(docAtom1).not.toBe(docAtom2); // Different atom instances
  });

  it('should cleanup atom store on dispose', () => {
    const model = new AgentViewModel('block-test');
    const atomStore = getOrCreateAtomStore('block-test');

    model.dispose();

    // Verify cleanup
    expect(atomStoreMap.has('block-test')).toBe(false);
  });
});
```

### Manual Testing Checklist

**Scenario 1: Basic Multi-Instance**
1. Open Agent Widget A
2. Send message "Test A"
3. Verify Widget A shows "Test A"
4. Open Agent Widget B
5. Send message "Test B"
6. **Expected:** Widget A shows "Test A", Widget B shows "Test B" (no bleed)

**Scenario 2: Simultaneous Streaming**
1. Open Agent Widget A and Widget B
2. Start agent process in Widget A
3. Start agent process in Widget B
4. **Expected:** Both widgets stream independently

**Scenario 3: Widget Close Cleanup**
1. Open 5 agent widgets
2. Close all 5 widgets
3. Check `atomStoreMap.size` (should be 0)
4. Open 1 new widget
5. **Expected:** New widget works correctly

---

## Risk Assessment

### Low Risk Changes
- ✅ Version bump
- ✅ Adding getter methods to `AgentViewModel`

### Medium Risk Changes
- ⚠️ Atom store refactor
  - **Risk:** Breaking existing functionality if atom references missed
  - **Mitigation:** Thorough testing, code review

### High Risk Changes
- ❌ None (using conservative approach)

---

## Rollout Plan

### Phase 1: Fix and Test Locally
1. Implement fixes on `agentx/unified-agent-widget-phase5` branch
2. Run `task dev` and test multi-instance scenarios
3. Verify no regressions

### Phase 2: Push and Re-Review
1. Commit fixes with message:
   ```
   fix: per-instance atoms for agent widget

   - Refactor global atoms to per-instance atom stores
   - Add cleanup on widget disposal
   - Bump version to 0.27.11
   ```
2. Push to remote
3. Wait for ReAgent review

### Phase 3: Merge and Deploy
1. Once approved, squash merge to main
2. Build portable package: `task package:portable`
3. Test final build

---

## Follow-Up Work

### Immediate (Before Merge)
- Fix the 3 issues identified by ReAgent
- Test multi-instance functionality

### Short-Term (After Merge)
- Document the per-instance atom pattern in CLAUDE.md
- Add to widget development guidelines

### Long-Term (Future PRs)
- Phase 4: Smart sections and tool grouping
- Migrate existing `defwidget@ai` and `defwidget@claudecode` widgets to new unified widget
- Remove old widget code (deprecation)

---

## References

### Related PRs
- **PR #296** - Phase 1: Streaming intermediary
- **PR #298** - Phase 2: Interactive markdown UI
- **PR #299** - Phase 3: Multi-agent features
- **PR #304** - Robust shell integration (merged, caused rebase)

### Documentation
- `docs/SPEC_UNIFIED_AGENT_WIDGET.md` - Full feature spec
- `CLAUDE.md` - Version management guidelines

### Code Files
- `frontend/app/view/agent/agent-model.ts` - ViewModel implementation
- `frontend/app/view/agent/agent-view.tsx` - View wrapper component
- `frontend/app/block/block.tsx` - Widget registration
- `pkg/wconfig/defaultconfig/widgets.json` - Widget config

---

## Questions & Answers

**Q: Why not just limit to one agent widget instance?**

A: Users need multiple agent widgets for:
- Running multiple concurrent tasks
- Comparing agent outputs side-by-side
- Dedicated agents per project/context

**Q: Could we use a different state management library?**

A: No - AgentMux uses Jotai throughout. Switching would be inconsistent.

**Q: Is this a regression or new bug?**

A: Neither - this is unreleased code. The bug was caught during review before merge.

**Q: Will this affect existing `defwidget@ai` or `defwidget@claudecode`?**

A: No - they are separate widgets and continue working as-is.

---

## Conclusion

PR #305 implements a critical feature (unified agent widget) but has a fundamental architectural flaw: global atoms prevent multiple widget instances from working correctly.

**The fix is straightforward:** Refactor atoms to be per-instance using a Map keyed by `blockId`.

**Estimated effort:** 2-3 hours (refactor + testing)

**Risk:** Medium (state management changes require thorough testing)

**Priority:** High (blocks PR from merging)

---

**Next Steps:**
1. Implement fixes outlined above
2. Test multi-instance scenarios
3. Push for ReAgent re-review
4. Merge once approved

**Status:** Ready for implementation
