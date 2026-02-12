# Orphaned Block Cleanup - Frontend Investigation Spec

## Issue Status: ACTIVE INVESTIGATION

**Created:** 2026-02-12
**Status:** Orphan persists after backend migration + frontend fix
**Priority:** HIGH

---

## Problem Summary

Orphaned blocks (blocks in layout tree but not in tab.BlockIds) persist despite:
1. ✅ Backend migration queuing DeleteNode actions
2. ✅ Frontend handler enhanced to find orphaned nodes in tree
3. ❌ Orphan **still present** in database after migration runs

---

## Current State

### Database Evidence (v0.24.12)

```sql
-- Layout leaforder (2 blocks)
SELECT json_extract(data, '$.leaforder') FROM db_layout WHERE oid = '08182ac3-6b82-4da5-979c-99a597e0f014';
-- Result: [{"nodeid":"2d56009e...","blockid":"0789da55..."},  ← ORPHAN
--          {"nodeid":"54ebb5dd...","blockid":"70bffb64..."}]  ← VALID

-- Tab blockids (1 block)
SELECT json_extract(data, '$.blockids') FROM db_tab WHERE oid = 'e1590f0d-5622-47c4-b620-805cbe4a1443';
-- Result: ["70bffb64-29d4-40ef-a2dd-e8d0efb0e061"]  ← Only valid block

-- Layout rootnode (2 nodes in tree)
SELECT json_extract(data, '$.rootnode') FROM db_layout WHERE oid = '08182ac3-6b82-4da5-979c-99a597e0f014';
-- Result: {"children":[
--   {"data":{"blockId":"0789da55-bb3b-44e0-b8a2-309b94e5e54f"}...},  ← ORPHAN IN TREE!
--   {"data":{"blockId":"70bffb64-29d4-40ef-a2dd-e8d0efb0e061"}...}
-- ]}
```

**Key Finding:** Orphaned block `0789da55...` exists in BOTH `leaforder` AND `rootnode` tree structure!

---

## Work Completed

### Backend (PR #270 - Merged v0.24.11)

**File:** `pkg/wcore/block.go`
- ✅ DeleteBlock() queues layout removal action
- ✅ Migration `MigrateOrphanedLayouts()` detects orphans at startup
- ✅ Logs show: "found 1 orphaned blocks", "cleaned 1 orphaned blocks"

**Result:** Migration runs successfully and queues DeleteNode actions.

### Frontend (v0.24.12)

**File:** `frontend/layout/lib/layoutNode.ts`
- ✅ Added `findNodeByBlockId()` to recursively search tree

**File:** `frontend/layout/lib/layoutModel.ts`
- ✅ Enhanced DeleteNode handler to use tree search as fallback
- ✅ When orphan found, directly deletes via `treeReducer(DeleteNode)`

**Code:**
```typescript
case LayoutTreeActionType.DeleteNode: {
    let leaf = this?.getNodeByBlockId(action.blockid);

    // Fallback: search tree directly for orphaned blocks
    if (!leaf && this.treeState.rootNode) {
        leaf = findNodeByBlockId(this.treeState.rootNode, action.blockid);
        if (leaf) {
            // Delete directly from tree
            this.treeReducer({
                type: LayoutTreeActionType.DeleteNode,
                nodeId: leaf.id,
            }, false);
            break;
        }
    }
    // ... existing closeNode logic
}
```

**Result:** Handler should now find and delete orphaned nodes. But orphan persists!

---

## Root Cause Analysis

### Why Migration Fails

**Theory 1: Timing Issue**
- Migration runs at backend startup before frontend loads layout
- Frontend processes queued DeleteNode action
- But `getNodeByBlockId()` fails because leafs array not populated yet
- Fallback tree search also fails due to timing

**Theory 2: Tree State Persistence Bug**
- Frontend deletes node from in-memory tree
- But `persistToBackend()` doesn't execute or fails silently
- Database never updated with cleaned tree

**Theory 3: Tree Reconstruction**
- After deleting orphan from tree, `updateTree()` rebuilds leafOrder from leafs
- But somehow orphan gets re-added from database rootnode
- Circular issue: DB has orphan, tree loads orphan, delete fails, persist fails

### Verification Needed

Check frontend logs for:
- `[BUG-TRACE] handleBackendAction DeleteNode triggered for blockId: 0789da55...`
- `[BUG-TRACE] Found orphaned block in tree: ...`
- Browser console errors during layout initialization
- `persistToBackend()` execution logs

---

## Investigation Path

### Step 1: Add Comprehensive Logging

**File:** `frontend/layout/lib/layoutModel.ts`

Add logging to trace full lifecycle:

```typescript
case LayoutTreeActionType.DeleteNode: {
    console.log(`[ORPHAN-DEBUG] DeleteNode action received:`, {
        blockId: action.blockid,
        rootNodeExists: !!this.treeState.rootNode,
        leafsCount: this.getter(this.leafs)?.length
    });

    let leaf = this?.getNodeByBlockId(action.blockid);
    console.log(`[ORPHAN-DEBUG] getNodeByBlockId result:`, leaf);

    if (!leaf && this.treeState.rootNode) {
        leaf = findNodeByBlockId(this.treeState.rootNode, action.blockid);
        console.log(`[ORPHAN-DEBUG] findNodeByBlockId result:`, leaf);

        if (leaf) {
            console.log(`[ORPHAN-DEBUG] Deleting orphan via treeReducer:`, {
                nodeId: leaf.id,
                blockId: action.blockid
            });

            this.treeReducer({
                type: LayoutTreeActionType.DeleteNode,
                nodeId: leaf.id,
            }, false);

            console.log(`[ORPHAN-DEBUG] After treeReducer, rootNode:`,
                JSON.stringify(this.treeState.rootNode, null, 2));
            break;
        }
    }
    // ... rest of handler
}
```

Add logging to `persistToBackend()`:

```typescript
private persistToBackend() {
    console.log(`[ORPHAN-DEBUG] persistToBackend called, leafOrder:`,
        this.treeState.leafOrder);
    // ... existing code
}
```

Add logging to `updateTree()`:

```typescript
updateTree() {
    console.log(`[ORPHAN-DEBUG] updateTree START, rootNode:`,
        this.treeState.rootNode);
    // ... existing code
    this.treeState.leafOrder = getLeafOrder(newLeafs, newAdditionalProps);
    console.log(`[ORPHAN-DEBUG] updateTree calculated leafOrder:`,
        this.treeState.leafOrder);
}
```

### Step 2: Test Migration with Logging

1. Build frontend with logging
2. Clear browser cache
3. Start dev build
4. Open browser DevTools console
5. Watch for `[ORPHAN-DEBUG]` messages during migration
6. Capture full log sequence

### Step 3: Manual Deletion Test

If migration fails, test manual deletion:

1. In browser console:
   ```javascript
   // Get layout model for active tab
   const tabId = globalStore.get(atoms.staticTabId);
   const layoutStateAtom = getLayoutStateAtomFromTab(tabId);
   const layoutModel = globalStore.get(layoutStateAtom);

   // Find orphaned node
   const orphanBlockId = "0789da55-bb3b-44e0-b8a2-309b94e5e54f";
   const orphanNode = findNodeByBlockId(layoutModel.treeState.rootNode, orphanBlockId);

   console.log("Found orphan:", orphanNode);

   // Try deleting it
   layoutModel.treeReducer({
       type: "DeleteNode",
       nodeId: orphanNode.id
   }, false);

   layoutModel.updateTree();
   layoutModel.persistToBackend();

   // Check if removed
   console.log("After delete, leafOrder:", layoutModel.treeState.leafOrder);
   ```

2. If manual deletion works, issue is with migration timing
3. If manual deletion fails, issue is with tree deletion logic

---

## Potential Solutions

### Option A: Force Tree Rebuild on Migration

Instead of queuing DeleteNode actions, directly clean the tree structure:

**Backend:** `pkg/wcore/layout.go`

```go
func MigrateOrphanedLayouts(ctx context.Context) error {
    // ... find orphaned blocks ...

    if len(orphanedBlocks) > 0 {
        // Instead of queuing actions, directly clean rootnode
        layout.RootNode = removeOrphanedNodesFromTree(layout.RootNode, orphanedBlocks)
        layout.LeafOrder = rebuildLeafOrderFromTree(layout.RootNode)
        layout.Version++

        err = wstore.DBUpdate(ctx, layout)
        if err != nil {
            log.Printf("error updating layout: %v", err)
        }
    }
}

func removeOrphanedNodesFromTree(node *LayoutNode, orphanIds []string) *LayoutNode {
    if node == nil {
        return nil
    }

    orphanSet := make(map[string]bool)
    for _, id := range orphanIds {
        orphanSet[id] = true
    }

    // If this node is orphaned, return nil
    if node.Data != nil && orphanSet[node.Data.BlockId] {
        return nil
    }

    // Recursively clean children
    if node.Children != nil {
        cleanChildren := []*LayoutNode{}
        for _, child := range node.Children {
            cleaned := removeOrphanedNodesFromTree(child, orphanIds)
            if cleaned != nil {
                cleanChildren = append(cleanChildren, cleaned)
            }
        }
        node.Children = cleanChildren
    }

    return node
}
```

**Pros:**
- ✅ Bypasses frontend action queue
- ✅ Direct database update
- ✅ No timing issues

**Cons:**
- ⚠️ Duplicates tree manipulation logic (also in frontend)
- ⚠️ Requires Go tree traversal implementation

### Option B: Frontend-Side Cleanup on Load

Run cleanup after layout is fully loaded:

**File:** `frontend/layout/lib/layoutModel.ts`

```typescript
constructor(waveObjectAtom, getter, setter) {
    // ... existing init ...

    // Schedule orphan cleanup after initialization
    setTimeout(() => {
        this.cleanupOrphanedBlocks();
    }, 1000);
}

private cleanupOrphanedBlocks() {
    const waveObj = this.getter(this.waveObjectAtom);
    if (!waveObj || !this.treeState.rootNode) return;

    // Get valid block IDs from backend
    RpcApi.GetTabCommand(TabRpcClient, { tabid: this.tabId }).then(tab => {
        const validBlockIds = new Set(tab.blockids);

        // Find orphans in tree
        const orphans = this.findOrphansInTree(this.treeState.rootNode, validBlockIds);

        if (orphans.length > 0) {
            console.log(`[CLEANUP] Found ${orphans.length} orphaned blocks, removing...`);

            for (const orphan of orphans) {
                this.treeReducer({
                    type: LayoutTreeActionType.DeleteNode,
                    nodeId: orphan.id
                }, false);
            }

            this.updateTree();
            this.persistToBackend();
        }
    });
}

private findOrphansInTree(node: LayoutNode, validBlockIds: Set<string>): LayoutNode[] {
    const orphans: LayoutNode[] = [];

    if (node.data?.blockId && !validBlockIds.has(node.data.blockId)) {
        orphans.push(node);
    }

    if (node.children) {
        for (const child of node.children) {
            orphans.push(...this.findOrphansInTree(child, validBlockIds));
        }
    }

    return orphans;
}
```

**Pros:**
- ✅ Runs after frontend fully initialized
- ✅ No timing issues
- ✅ Can verify against backend state

**Cons:**
- ⚠️ Adds 1s delay to layout loading
- ⚠️ Runs on every tab load (could cache cleaned state)

### Option C: Fix deleteNode in layoutTree.ts

The `deleteNode` function might not properly handle root-level children:

**File:** `frontend/layout/lib/layoutTree.ts`

Current code (lines 354-379):
```typescript
export function deleteNode(layoutState: LayoutTreeState, action: LayoutTreeDeleteNodeAction) {
    if (!action?.nodeId) {
        console.error("no delete node action provided");
        return;
    }
    if (!layoutState.rootNode) {
        console.error("no root node");
        return;
    }
    if (layoutState.rootNode.id === action.nodeId) {
        layoutState.rootNode = undefined;
    } else {
        const parent = findParent(layoutState.rootNode, action.nodeId);
        if (parent) {
            const node = parent.children.find((child) => child.id === action.nodeId);
            removeChild(parent, node);
            if (layoutState.focusedNodeId === node.id) {
                layoutState.focusedNodeId = undefined;
            }
        } else {
            console.error("unable to delete node, not found in tree");
        }
    }
}
```

**Potential Issue:** If the orphan node is a direct child of rootNode, `findParent()` might fail or `removeChild()` might not work correctly.

**Enhanced deleteNode:**
```typescript
export function deleteNode(layoutState: LayoutTreeState, action: LayoutTreeDeleteNodeAction) {
    if (!action?.nodeId) {
        console.error("no delete node action provided");
        return;
    }
    if (!layoutState.rootNode) {
        console.error("no root node");
        return;
    }

    // If deleting root node
    if (layoutState.rootNode.id === action.nodeId) {
        layoutState.rootNode = undefined;
        return;
    }

    // Try to find parent
    const parent = findParent(layoutState.rootNode, action.nodeId);
    if (!parent) {
        console.error("unable to delete node, parent not found in tree", {
            nodeId: action.nodeId,
            rootNodeId: layoutState.rootNode.id,
            rootChildren: layoutState.rootNode.children?.map(c => c.id)
        });
        return;
    }

    const node = parent.children?.find((child) => child.id === action.nodeId);
    if (!node) {
        console.error("unable to delete node, not in parent's children", {
            nodeId: action.nodeId,
            parentId: parent.id
        });
        return;
    }

    console.log(`[DELETE-DEBUG] Removing node ${action.nodeId} from parent ${parent.id}`);
    removeChild(parent, node);

    if (layoutState.focusedNodeId === node.id) {
        layoutState.focusedNodeId = undefined;
    }

    console.log(`[DELETE-DEBUG] After removal, parent children:`,
        parent.children?.map(c => c.id));
}
```

---

## Recommended Next Steps

1. **Immediate (Day 1):**
   - Add comprehensive logging to DeleteNode handler, updateTree, persistToBackend
   - Test migration with logging enabled
   - Capture full browser console output during migration
   - Determine if DeleteNode action is even being processed

2. **Short-term (Day 2-3):**
   - If logging shows DeleteNode fails, implement Option C (enhanced deleteNode logging)
   - If logging shows DeleteNode succeeds but persist fails, investigate persistToBackend
   - If timing issue confirmed, implement Option B (frontend-side cleanup on load)

3. **Long-term (Week 2):**
   - If all frontend fixes fail, implement Option A (backend tree cleanup)
   - Add e2e test that creates orphaned block and verifies cleanup
   - Document final solution in CONTRIBUTING.md

---

## Testing Checklist

- [ ] Migration runs without errors
- [ ] Browser console shows `[ORPHAN-DEBUG]` logs
- [ ] DeleteNode action received for orphaned blockId
- [ ] Tree search finds orphaned node
- [ ] treeReducer executes without errors
- [ ] updateTree() recalculates leafOrder correctly
- [ ] persistToBackend() executes and updates database
- [ ] Database query confirms orphan removed from rootnode
- [ ] Database query confirms orphan removed from leaforder
- [ ] UI does not show phantom block

---

## Related Files

**Backend:**
- `pkg/wcore/layout.go` - MigrateOrphanedLayouts()
- `pkg/wcore/block.go` - DeleteBlock() with layout cleanup
- `pkg/waveobj/waveobj.go` - LayoutState, LayoutNode types

**Frontend:**
- `frontend/layout/lib/layoutModel.ts` - Layout state manager, DeleteNode handler
- `frontend/layout/lib/layoutTree.ts` - Tree manipulation functions
- `frontend/layout/lib/layoutNode.ts` - Tree traversal (findNode, findNodeByBlockId)

**Specs:**
- `docs/LAYOUT_ORPHAN_PREVENTION_SPEC.md` - Original orphan prevention spec
- `docs/VERSION_VERIFICATION_SPEC.md` - Binary caching issue

---

## Success Criteria

Migration is successful when:
1. `MigrateOrphanedLayouts()` runs at startup
2. Orphaned blocks detected and logged
3. Layout tree cleaned (rootnode has no orphan nodes)
4. LeafOrder cleaned (no orphan entries)
5. Database persists cleaned state
6. Subsequent app restarts show no orphans
7. UI does not render phantom blocks

---

**Document Version:** 1.0
**Last Updated:** 2026-02-12
**Status:** Investigation Phase - Orphan persists despite fixes
