# Agent Widget State Scoping Fix

**Date:** 2026-02-15
**PR:** #305
**Issue:** Global state atoms causing state bleeding between agent widget instances
**Severity:** Critical - Blocks PR merge

---

## Executive Summary

The unified agent widget implementation (Phase 5) has a fundamental architectural flaw: **all widget instances share global singleton atoms**, causing state to bleed between multiple agent widgets. The spec requires each agent widget to have independent, isolated state.

**Current State:**
- ✅ Phase 1-3: Components built
- ✅ Phase 5: Widget registered
- ❌ **Critical Bug**: Global state atoms

**Required Fix:**
- Make atoms instance-scoped (per ViewModel)
- Pass atoms from ViewModel → View component
- Each widget gets its own document, process state, filters

---

## Problem Analysis

### Current Architecture (Broken)

```typescript
// frontend/app/view/agent/state.ts
export const agentDocumentAtom = atom<DocumentNode[]>([]);     // GLOBAL!
export const agentIdAtom = atom<string>("");                   // GLOBAL!
export const agentProcessAtom = atom<AgentProcessState>({...}); // GLOBAL!
```

**Why This Fails:**

```
User creates 2 agent widgets:

┌─────────────────────┐     ┌─────────────────────┐
│ Agent Widget 1      │     │ Agent Widget 2      │
│ (blockId: abc123)   │     │ (blockId: xyz789)   │
│                     │     │                     │
│ Reads/writes to:    │     │ Reads/writes to:    │
│ agentDocumentAtom ──┼─────┼─→ agentDocumentAtom │ ← SAME ATOM!
│ agentIdAtom ────────┼─────┼─→ agentIdAtom       │ ← SAME ATOM!
│ agentProcessAtom ───┼─────┼─→ agentProcessAtom  │ ← SAME ATOM!
└─────────────────────┘     └─────────────────────┘

Result: Both widgets see the same data!
        Widget 2 overwrites Widget 1's state.
        User can only have ONE agent widget working.
```

### Bot Review Findings

```
Issues:
- frontend/app/view/agent/agent-model.ts:53
  Global singleton atoms (agentDocumentAtom, agentIdAtom, agentProcessAtom)
  shared across all widget instances - state will bleed between multiple agent widgets

- frontend/app/view/agent/agent-view.tsx:63
  Using global filteredDocumentAtom atom - multiple agent widgets will share document state
```

---

## Correct Architecture (Per-Widget State)

### Pattern: Atoms Inside ViewModel

Each `AgentViewModel` instance should **own** its atoms:

```typescript
export class AgentViewModel implements ViewModel {
    // Each instance has its OWN atoms
    private documentAtom: PrimitiveAtom<DocumentNode[]>;
    private processAtom: PrimitiveAtom<AgentProcessState>;
    private filterAtom: PrimitiveAtom<FilterState>;

    constructor(blockId: string, nodeModel: BlockNodeModel) {
        // Create instance-scoped atoms
        this.documentAtom = atom<DocumentNode[]>([]);
        this.processAtom = atom<AgentProcessState>({
            status: "idle",
            canRestart: true,
            canKill: false,
        });
        this.filterAtom = atom<FilterState>({
            showThinking: false,
            showSuccessfulTools: true,
            showFailedTools: true,
            showIncoming: true,
            showOutgoing: true,
        });
    }
}
```

**Result:**

```
User creates 2 agent widgets:

┌─────────────────────┐     ┌─────────────────────┐
│ Agent Widget 1      │     │ Agent Widget 2      │
│ (blockId: abc123)   │     │ (blockId: xyz789)   │
│                     │     │                     │
│ model1.documentAtom │     │ model2.documentAtom │ ← SEPARATE!
│ model1.processAtom  │     │ model2.processAtom  │ ← SEPARATE!
│ model1.filterAtom   │     │ model2.filterAtom   │ ← SEPARATE!
└─────────────────────┘     └─────────────────────┘

Result: Each widget has independent state!
        No bleeding, no conflicts.
```

---

## Implementation Plan

### Phase 1: Refactor State Management

**Files to Modify:**
1. `frontend/app/view/agent/state.ts` - Remove global atoms, add factory functions
2. `frontend/app/view/agent/agent-model.ts` - Create instance atoms in constructor
3. `frontend/app/view/agent/agent-view.tsx` - Accept atoms as props

#### Step 1.1: Update `state.ts`

**Before:**
```typescript
// Global atoms - WRONG!
export const agentDocumentAtom = atom<DocumentNode[]>([]);
export const agentIdAtom = atom<string>("");
export const agentProcessAtom = atom<AgentProcessState>({...});
export const documentStateAtom = atom<DocumentState>({...});
```

**After:**
```typescript
// Remove global atoms, keep only types and helper functions

export interface AgentAtoms {
    documentAtom: PrimitiveAtom<DocumentNode[]>;
    processAtom: PrimitiveAtom<AgentProcessState>;
    filterAtom: PrimitiveAtom<FilterState>;
    documentStateAtom: PrimitiveAtom<DocumentState>;
}

// Factory function to create fresh atoms for each widget instance
export function createAgentAtoms(agentId: string): AgentAtoms {
    const documentAtom = atom<DocumentNode[]>([]);
    const processAtom = atom<AgentProcessState>({
        status: "idle",
        canRestart: true,
        canKill: false,
    });
    const filterAtom = atom<FilterState>({
        showThinking: false,
        showSuccessfulTools: true,
        showFailedTools: true,
        showIncoming: true,
        showOutgoing: true,
    });
    const documentStateAtom = atom<DocumentState>({
        collapsedNodes: new Set<string>(),
        scrollPosition: 0,
        selectedNode: null,
        filter: {
            showThinking: false,
            showSuccessfulTools: true,
            showFailedTools: true,
            showIncoming: true,
            showOutgoing: true,
        },
    });

    return {
        documentAtom,
        processAtom,
        filterAtom,
        documentStateAtom,
    };
}

// Derived atom factory: filtered document
export function createFilteredDocumentAtom(
    documentAtom: PrimitiveAtom<DocumentNode[]>,
    filterAtom: PrimitiveAtom<FilterState>
): Atom<DocumentNode[]> {
    return atom((get) => {
        const doc = get(documentAtom);
        const filter = get(filterAtom);

        return doc.filter((node) => {
            if (node.type === "markdown" && node.metadata?.thinking) {
                return filter.showThinking;
            }
            if (node.type === "tool") {
                if (node.status === "success") return filter.showSuccessfulTools;
                if (node.status === "failed") return filter.showFailedTools;
            }
            if (node.type === "agent_message") {
                if (node.direction === "incoming") return filter.showIncoming;
                if (node.direction === "outgoing") return filter.showOutgoing;
            }
            return true;
        });
    });
}

// Action creators remain the same, but take atoms as parameters
export function createToggleNodeCollapsed(documentStateAtom: PrimitiveAtom<DocumentState>) {
    return atom(null, (get, set, nodeId: string) => {
        const state = get(documentStateAtom);
        const newCollapsed = new Set(state.collapsedNodes);
        if (newCollapsed.has(nodeId)) {
            newCollapsed.delete(nodeId);
        } else {
            newCollapsed.add(nodeId);
        }
        set(documentStateAtom, { ...state, collapsedNodes: newCollapsed });
    });
}
```

#### Step 1.2: Update `agent-model.ts`

**Before:**
```typescript
export class AgentViewModel implements ViewModel {
    constructor(blockId: string, nodeModel: BlockNodeModel) {
        // Uses global atoms
        globalStore.set(agentIdAtom, blockId);
    }

    sendMessage = (text: string): void => {
        // Writes to global agentDocumentAtom
        const currentDoc = globalStore.get(agentDocumentAtom);
        globalStore.set(agentDocumentAtom, [...currentDoc, ...nodes]);
    };
}
```

**After:**
```typescript
import { createAgentAtoms, createFilteredDocumentAtom, createToggleNodeCollapsed } from "./state";

export class AgentViewModel implements ViewModel {
    // Instance-scoped atoms
    atoms: AgentAtoms;
    filteredDocumentAtom: Atom<DocumentNode[]>;
    toggleNodeCollapsed: WritableAtom<null, [string], void>;

    constructor(blockId: string, nodeModel: BlockNodeModel) {
        this.blockId = blockId;
        this.nodeModel = nodeModel;
        this.blockAtom = WOS.getWaveObjectAtom<Block>(`block:${blockId}`);
        this.viewComponent = AgentViewWrapper as any;

        // Create fresh atoms for THIS widget instance
        this.agentIdValue = blockId;
        this.atoms = createAgentAtoms(blockId);
        this.filteredDocumentAtom = createFilteredDocumentAtom(
            this.atoms.documentAtom,
            this.atoms.filterAtom
        );
        this.toggleNodeCollapsed = createToggleNodeCollapsed(this.atoms.documentStateAtom);

        // ... rest of initialization
    }

    sendMessage = (text: string): void => {
        // Writes to THIS instance's atoms
        const currentDoc = globalStore.get(this.atoms.documentAtom);
        globalStore.set(this.atoms.documentAtom, [...currentDoc, ...nodes]);
    };

    private async handleTerminalData(msg: any): Promise<void> {
        // ... parse NDJSON ...

        // Update THIS instance's document
        const currentDoc = globalStore.get(this.atoms.documentAtom);
        globalStore.set(this.atoms.documentAtom, [...currentDoc, ...nodes]);
    }
}
```

#### Step 1.3: Update `agent-view.tsx`

**Before:**
```typescript
export const AgentViewInner: React.FC<AgentViewProps> = memo(
    ({ agentId, onSendMessage, onExport, ... }) => {
        // Uses global atoms - WRONG!
        const document = useAtomValue(filteredDocumentAtom);
        const documentState = useAtomValue(documentStateAtom);
        const toggleCollapse = useSetAtom(toggleNodeCollapsed);
        // ...
    }
);
```

**After:**
```typescript
interface AgentViewProps {
    agentId: string;
    atoms: AgentAtoms;  // NEW: Pass atoms from ViewModel
    filteredDocumentAtom: Atom<DocumentNode[]>;  // NEW
    toggleNodeCollapsed: WritableAtom<null, [string], void>;  // NEW
    onSendMessage?: (message: string) => void;
    onExport?: (format: "markdown" | "html") => void;
    // ...
}

export const AgentViewInner: React.FC<AgentViewProps> = memo(
    ({ agentId, atoms, filteredDocumentAtom, toggleNodeCollapsed, ... }) => {
        // Uses instance-scoped atoms from props
        const document = useAtomValue(filteredDocumentAtom);
        const documentState = useAtomValue(atoms.documentStateAtom);
        const toggleCollapse = useSetAtom(toggleNodeCollapsed);
        // ...
    }
);

export const AgentViewWrapper: React.FC<ViewComponentProps<AgentViewModel>> = memo(({ model }) => {
    return (
        <AgentViewInner
            agentId={model.agentIdValue}
            atoms={model.atoms}  // Pass instance atoms
            filteredDocumentAtom={model.filteredDocumentAtom}
            toggleNodeCollapsed={model.toggleNodeCollapsed}
            onSendMessage={model.sendMessage}
            onExport={model.exportDocument}
            onPause={model.pauseAgent}
            onResume={model.resumeAgent}
            onKill={model.killAgent}
            onRestart={model.restartAgent}
        />
    );
});
```

#### Step 1.4: Update Component Props

**Files to update:**
- `AgentHeader.tsx` - Accept `processAtom` as prop
- `AgentFooter.tsx` - Accept `documentAtom` as prop
- `FilterControls.tsx` - Accept `filterAtom` as prop

**Pattern:**
```typescript
// Before: uses global atoms
const FilterControls = () => {
    const filter = useAtomValue(filterStateAtom);  // GLOBAL
};

// After: receives atom via props
interface FilterControlsProps {
    filterAtom: PrimitiveAtom<FilterState>;
}
const FilterControls = ({ filterAtom }: FilterControlsProps) => {
    const filter = useAtomValue(filterAtom);  // INSTANCE
};
```

---

### Phase 2: Update All Subcomponents

**Files:**
1. `frontend/app/view/agent/components/AgentHeader.tsx`
2. `frontend/app/view/agent/components/AgentFooter.tsx`
3. `frontend/app/view/agent/components/FilterControls.tsx`
4. `frontend/app/view/agent/components/ProcessControls.tsx`

**Changes:**
- Add atom props to component interfaces
- Pass atoms from `AgentViewInner` → subcomponents
- Replace `useAtomValue(globalAtom)` with `useAtomValue(props.atom)`

---

### Phase 3: Testing Strategy

#### Manual Test Cases

**Test 1: Single Widget**
1. Create one agent widget
2. Send message → verify it appears
3. Expand/collapse tool → verify state persists
4. Filter document → verify filtering works

**Test 2: Multiple Widgets (Critical)**
1. Create agent widget 1
2. Send message to agent 1 → note content
3. Create agent widget 2
4. Send different message to agent 2
5. ✅ **VERIFY:** Agent 1 shows ONLY its messages
6. ✅ **VERIFY:** Agent 2 shows ONLY its messages
7. ✅ **VERIFY:** No state bleeding between widgets

**Test 3: Widget Lifecycle**
1. Create agent widget
2. Send messages, add documents
3. Close widget (delete block)
4. Create new agent widget
5. ✅ **VERIFY:** New widget starts with empty state
6. ✅ **VERIFY:** No data from deleted widget

**Test 4: Filter Independence**
1. Create 2 agent widgets
2. Widget 1: Enable "show thinking"
3. Widget 2: Disable "show thinking"
4. ✅ **VERIFY:** Each widget respects its own filter settings

#### Automated Tests

```typescript
// test/agent-state-isolation.test.ts
import { createAgentAtoms } from "@/app/view/agent/state";

describe("Agent State Isolation", () => {
    it("should create independent atoms for each instance", () => {
        const atoms1 = createAgentAtoms("agent-1");
        const atoms2 = createAgentAtoms("agent-2");

        // Atoms are different objects
        expect(atoms1.documentAtom).not.toBe(atoms2.documentAtom);
        expect(atoms1.processAtom).not.toBe(atoms2.processAtom);
    });

    it("should not share state between instances", () => {
        const atoms1 = createAgentAtoms("agent-1");
        const atoms2 = createAgentAtoms("agent-2");

        // Write to atoms1
        globalStore.set(atoms1.documentAtom, [{ id: "1", type: "markdown", content: "Test 1" }]);

        // atoms2 should still be empty
        const doc2 = globalStore.get(atoms2.documentAtom);
        expect(doc2).toEqual([]);
    });
});
```

---

## Migration Path

### Backward Compatibility

**No compatibility concerns** - this is a new feature (Phase 5 not yet merged):
- No users have agent widgets yet
- No state to migrate
- Can make breaking changes safely

### Rollout Plan

1. **Fix PR #305** with state scoping changes
2. **Re-request bot review** → should pass
3. **Manual testing** of multi-widget scenarios
4. **Merge to main**
5. **Document** multi-agent usage in user guide

---

## Success Criteria

### Definition of Done

- [ ] All global atoms removed from `state.ts`
- [ ] `AgentViewModel` creates instance-scoped atoms
- [ ] All components accept atoms via props (no global imports)
- [ ] Bot review passes (no state bleeding warnings)
- [ ] Manual test: 2+ widgets with independent state
- [ ] Manual test: Filters work independently per widget
- [ ] Automated tests: State isolation verified
- [ ] Build passes with no TypeScript errors
- [ ] Documentation updated (if needed)

### Performance Considerations

**Concern:** Does creating atoms per-instance impact performance?

**Answer:** No. Jotai atoms are lightweight:
- ~100 bytes per atom
- 4 atoms per widget = ~400 bytes
- 10 widgets = ~4 KB total (negligible)

**Benchmark:** ClaudeCodeViewModel already creates atoms per-instance:
```typescript
// claudecode-model.ts (existing pattern)
this.turnsAtom = atom<ConversationTurn[]>([]);        // Per-instance
this.sessionMetaAtom = atom<SessionMeta>({...});      // Per-instance
this.isStreamingAtom = atom<boolean>(false);          // Per-instance
```

No performance issues reported. Safe to follow same pattern.

---

## Reference Implementations

### Similar Patterns in Codebase

**1. ClaudeCodeViewModel** (`frontend/app/view/claudecode/claudecode-model.ts`)
```typescript
export class ClaudeCodeViewModel implements ViewModel {
    turnsAtom: PrimitiveAtom<ConversationTurn[]>;
    sessionMetaAtom: PrimitiveAtom<SessionMeta>;

    constructor(blockId: string, nodeModel: BlockNodeModel) {
        // Instance atoms
        this.turnsAtom = atom<ConversationTurn[]>([]);
        this.sessionMetaAtom = atom<SessionMeta>({...});
    }
}
```

**2. PreviewModel** (`frontend/app/view/preview/preview-model.tsx`)
```typescript
export class PreviewModel implements ViewModel {
    statAtom: Atom<FileInfo>;
    fullFile: Atom<FullFile>;

    constructor(...) {
        // Instance atoms
        this.statAtom = WOS.getWaveObjectAtom(...);
        this.fullFile = atom((get) => {...});
    }
}
```

**Pattern:** All ViewModels create instance-scoped atoms. AgentViewModel should do the same.

---

## Estimated Effort

| Task | Complexity | Time | Priority |
|------|------------|------|----------|
| Refactor `state.ts` | Medium | 1 hour | P0 |
| Update `agent-model.ts` | Medium | 1 hour | P0 |
| Update `agent-view.tsx` | Low | 30 min | P0 |
| Update subcomponents (4 files) | Low | 1 hour | P0 |
| Write tests | Low | 30 min | P1 |
| Manual testing | Low | 30 min | P0 |
| Documentation | Low | 15 min | P2 |
| **Total** | | **4.75 hours** | |

---

## Next Steps

1. **Approve Implementation Plan** ✅
2. **Execute Phase 1** (state refactor)
3. **Execute Phase 2** (component updates)
4. **Execute Phase 3** (testing)
5. **Commit & Push** to PR #305
6. **Wait for bot re-review**
7. **Merge when approved**

---

## Questions & Risks

### Q: Why not use context API instead of atom props?

**A:** Atoms are more flexible:
- Jotai atoms work across component boundaries
- Can derive/compose atoms easily
- Consistent with existing codebase patterns (ClaudeCodeViewModel)
- Context requires provider wrapping (adds complexity)

### Q: Will this break existing widgets (ai, claudecode)?

**A:** No. They use separate state systems:
- `waveai` → uses `@ai-sdk/react` state
- `claudecode` → uses instance atoms (already correct)
- `agent` → new, no users yet

### Risk: Atom lifecycle management

**Mitigation:** Atoms are garbage collected when ViewModel is disposed. No manual cleanup needed. Jotai handles this automatically.

---

## Conclusion

The global state issue is **critical** but **straightforward to fix**. The pattern already exists in `ClaudeCodeViewModel`. Estimated fix time: ~5 hours including testing.

**Recommendation:** Proceed with implementation immediately. This blocks Phase 5 merge and prevents users from using the unified agent widget.
