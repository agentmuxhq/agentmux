# Layout Orphan Block Prevention Specification

## Problem Statement

Layouts can contain references to deleted blocks, creating "orphaned" block references. This occurs when:

1. A block is deleted via `DeleteBlock()` in `pkg/wcore/block.go`
2. The block is removed from `tab.BlockIds` (line 206-208)
3. The block controller is stopped and a close event is sent
4. **BUT** the layout's `rootnode` and `leaforder` still reference the deleted block

### Real-World Example

From database inspection on 2026-02-12:
```json
// Tab e1590f0d-5622-47c4-b620-805cbe4a1443
{
  "blockids": ["70bffb64-29d4-40ef-a2dd-e8d0efb0e061"],  // Only 1 block
  "layoutstate": "08182ac3-6b82-4da5-979c-99a597e0f014"
}

// Layout 08182ac3-6b82-4da5-979c-99a597e0f014
{
  "focusednodeid": "2d56009e-9336-4e05-a1da-c64f40f1f7d7",
  "leaforder": [
    {"nodeid": "2d56009e-9336-4e05-a1da-c64f40f1f7d7",
     "blockid": "0789da55-bb3b-44e0-b8a2-309b94e5e54f"},  // ORPHANED - block doesn't exist
    {"nodeid": "54ebb5dd-ade8-437f-b29d-37db22d08a37",
     "blockid": "70bffb64-29d4-40ef-a2dd-e8d0efb0e061"}   // Valid
  ],
  "rootnode": {
    "children": [
      {"id": "2d56009e-9336-4e05-a1da-c64f40f1f7d7",
       "data": {"blockId": "0789da55-bb3b-44e0-b8a2-309b94e5e54f"}},  // ORPHANED
      {"id": "54ebb5dd-ade8-437f-b29d-37db22d08a37",
       "data": {"blockId": "70bffb64-29d4-40ef-a2dd-e8d0efb0e061"}}    // Valid
    ],
    "flexDirection": "row"
  }
}
```

**Issues:**
- Block `0789da55-bb3b-44e0-b8a2-309b94e5e54f` doesn't exist in `db_block`
- Layout still references it in both `rootnode` and `leaforder`
- Focused node points to the orphaned block
- Tab's `blockids` array is correct, but layout is stale

## Impact

### User-Visible Issues
1. **Rendering errors**: Frontend may try to render non-existent blocks
2. **Focus issues**: Focused node may point to deleted block
3. **Layout corruption**: Split panes show empty/broken panes
4. **Inconsistent state**: Database shows incorrect pane count

### Development Issues
1. **Debug confusion**: Layout doesn't match reality
2. **Test failures**: E2E tests may encounter unexpected state
3. **Race conditions**: Frontend cleanup (`cleanupNodeModels`) vs backend state

## Architecture Analysis

### Current Flow: Block Deletion

**File:** `pkg/wcore/block.go:152-185`

```go
func DeleteBlock(ctx context.Context, blockId string, recursive bool) error {
    // 1. Get block
    block, err := wstore.DBMustGet[*waveobj.Block](ctx, blockId)

    // 2. Recursively delete subblocks
    for _, subBlockId := range block.SubBlockIds {
        DeleteBlock(ctx, subBlockId, recursive)
    }

    // 3. Delete block object (removes from tab.BlockIds)
    parentBlockCount, err := deleteBlockObj(ctx, blockId)

    // 4. Stop controller
    go blockcontroller.StopBlockController(blockId)

    // 5. Send close event
    sendBlockCloseEvent(blockId)

    // ❌ MISSING: Update layout to remove block reference

    return nil
}
```

### Current Flow: Layout Actions

**File:** `pkg/wcore/layout.go:80-112`

Layout updates use a **queue-based action system**:
- Actions queued via `QueueLayoutAction()` or `QueueLayoutActionForTab()`
- Actions stored in `LayoutState.PendingBackendActions`
- Frontend processes actions and updates `rootnode` and `leaforder`

**Available Action Types** (line 18-25):
- `LayoutActionDataType_Insert` - Insert node
- `LayoutActionDataType_InsertAtIndex` - Insert at specific position
- `LayoutActionDataType_Remove` - **Delete node** ✅
- `LayoutActionDataType_ClearTree` - Clear entire tree
- `LayoutActionDataType_Replace` - Replace node
- `LayoutActionDataType_SplitHorizontal` - Split pane horizontally
- `LayoutActionDataType_SplitVertical` - Split pane vertically

### Frontend Orphan Cleanup

**File:** `frontend/layout/lib/layoutModel.ts:1152-1163`

The frontend already has orphan cleanup logic:

```typescript
/**
 * Remove orphaned node models when their corresponding leaf is deleted.
 * @param leafOrder The new leaf order array to use when locating orphaned nodes.
 */
private cleanupNodeModels(leafOrder: LeafOrderEntry[]) {
    const orphanedNodeModels = [...this.nodeModels.keys()].filter(
        (id) => !leafOrder.find((leafEntry) => leafEntry.nodeid == id)
    );
    for (const id of orphanedNodeModels) {
        this.nodeModels.delete(id);
    }
}
```

**However:** This only cleans up frontend state, not the backend database.

## Solution Design

### Approach 1: Backend Layout Update (Recommended)

**Modify `DeleteBlock()` to queue a layout removal action:**

```go
// pkg/wcore/block.go
func DeleteBlock(ctx context.Context, blockId string, recursive bool) error {
    block, err := wstore.DBMustGet[*waveobj.Block](ctx, blockId)
    if err != nil {
        return fmt.Errorf("error getting block: %w", err)
    }
    if block == nil {
        return nil
    }

    // Get parent tab to access layout
    parentORef := waveobj.ParseORefNoErr(block.ParentORef)
    var tabId string
    if parentORef != nil && parentORef.OType == waveobj.OType_Tab {
        tabId = parentORef.OID
    }

    // Recursively delete subblocks
    if len(block.SubBlockIds) > 0 {
        for _, subBlockId := range block.SubBlockIds {
            err := DeleteBlock(ctx, subBlockId, recursive)
            if err != nil {
                return fmt.Errorf("error deleting subblock %s: %w", subBlockId, err)
            }
        }
    }

    // Delete block from database
    parentBlockCount, err := deleteBlockObj(ctx, blockId)
    if err != nil {
        return fmt.Errorf("error deleting block: %w", err)
    }

    // **NEW: Queue layout removal action**
    if tabId != "" {
        err = QueueLayoutActionForTab(ctx, tabId, waveobj.LayoutActionData{
            ActionType: LayoutActionDataType_Remove,
            BlockId:    blockId,
        })
        if err != nil {
            log.Printf("warning: failed to queue layout removal for block %s: %v", blockId, err)
            // Don't fail block deletion if layout update fails
        }
    }

    // Stop controller and send events
    go blockcontroller.StopBlockController(blockId)
    sendBlockCloseEvent(blockId)

    return nil
}
```

**Pros:**
- ✅ Centralized fix in one location
- ✅ Uses existing layout action system
- ✅ Frontend already handles `LayoutActionDataType_Remove`
- ✅ Works for all block deletion paths

**Cons:**
- ⚠️ Adds dependency from `block.go` to `layout.go`
- ⚠️ Layout update failure doesn't block deletion (by design)

### Approach 2: Database Constraint/Trigger

Add a database trigger or post-delete cleanup:

```go
// pkg/wstore/wstore.go - Add to DBDelete()
func DBDelete(ctx context.Context, otype string, oid string) error {
    // ... existing delete logic ...

    // If deleting a block, cleanup orphaned layout references
    if otype == waveobj.OType_Block {
        go cleanupOrphanedLayoutReferences(context.Background(), oid)
    }

    return nil
}

func cleanupOrphanedLayoutReferences(ctx context.Context, blockId string) {
    // Find all layouts that reference this block
    layouts, err := findLayoutsReferencingBlock(ctx, blockId)
    if err != nil {
        log.Printf("error finding layouts for cleanup: %v", err)
        return
    }

    // Queue removal actions for each layout
    for _, layoutId := range layouts {
        QueueLayoutAction(ctx, layoutId, waveobj.LayoutActionData{
            ActionType: LayoutActionDataType_Remove,
            BlockId:    blockId,
        })
    }
}
```

**Pros:**
- ✅ Catches all deletion paths automatically
- ✅ Lower-level enforcement

**Cons:**
- ❌ Requires complex JSON querying to find layouts
- ❌ Performance impact on every block deletion
- ❌ Harder to test and debug

### Approach 3: Validation + Periodic Cleanup

Add validation when loading layouts and periodic cleanup:

```go
// pkg/wcore/layout.go
func ValidateLayoutState(ctx context.Context, layoutStateId string) error {
    layout, err := wstore.DBGet[*waveobj.LayoutState](ctx, layoutStateId)
    if err != nil {
        return err
    }

    if layout.LeafOrder == nil {
        return nil
    }

    // Check each block in leaforder exists
    orphanedBlocks := []string{}
    for _, leaf := range *layout.LeafOrder {
        exists, err := wstore.DBExists(ctx, waveobj.OType_Block, leaf.BlockId)
        if err != nil || !exists {
            orphanedBlocks = append(orphanedBlocks, leaf.BlockId)
        }
    }

    // Queue removal actions for orphaned blocks
    if len(orphanedBlocks) > 0 {
        actions := make([]waveobj.LayoutActionData, len(orphanedBlocks))
        for i, blockId := range orphanedBlocks {
            actions[i] = waveobj.LayoutActionData{
                ActionType: LayoutActionDataType_Remove,
                BlockId:    blockId,
            }
        }
        return QueueLayoutAction(ctx, layoutStateId, actions...)
    }

    return nil
}
```

**Pros:**
- ✅ Defensive programming - catches issues from any source
- ✅ Can be run on-demand or periodically
- ✅ Doesn't affect normal deletion flow

**Cons:**
- ❌ Reactive, not preventive
- ❌ Adds overhead to layout loading
- ❌ Doesn't prevent the issue, just fixes it

## Recommended Solution

**Implement Approach 1 (Backend Layout Update) + Approach 3 (Validation)**

1. **Primary Fix:** Modify `DeleteBlock()` to queue layout removal action
2. **Safety Net:** Add validation check when loading layouts
3. **Migration:** Run validation on startup to clean existing orphans

### Implementation Plan

#### Phase 1: Core Fix
**File:** `pkg/wcore/block.go`
- [ ] Import `waveobj` types for layout actions
- [ ] Add layout removal action after `deleteBlockObj()`
- [ ] Handle error gracefully (log warning, don't fail delete)
- [ ] Add unit test for layout action queuing

#### Phase 2: Validation
**File:** `pkg/wcore/layout.go`
- [ ] Add `ValidateLayoutState()` function
- [ ] Add `DBExists()` helper to wstore if needed
- [ ] Call validation when loading tab layout
- [ ] Add unit test for orphan detection

#### Phase 3: Migration
**File:** `pkg/wcore/layout.go`
- [ ] Add `CleanupAllOrphanedBlocks()` function
- [ ] Run on app startup (first boot only, via migration flag)
- [ ] Log cleanup results for telemetry

#### Phase 4: Testing
**File:** `pkg/wcore/block_test.go`
- [ ] Test block deletion queues layout action
- [ ] Test orphan detection finds deleted blocks
- [ ] Test cleanup removes orphaned references
- [ ] E2E test: delete block, verify layout updated

## Database Schema Considerations

**No schema changes required** - uses existing structures:

- `LayoutState.PendingBackendActions` - Already supports action queue
- `LayoutActionData` - Already has `Remove` action type
- `Tab.BlockIds` - Already accurate (this is the source of truth)
- `LayoutState.RootNode` - Updated by frontend processing actions
- `LayoutState.LeafOrder` - Updated by frontend processing actions

## Migration Strategy

### For Existing Orphaned Layouts

```go
// Run once on app startup (v0.20.0+)
func MigrateOrphanedLayouts(ctx context.Context) error {
    log.Println("Checking for orphaned layout references...")

    // Get all tabs
    tabs, err := wstore.DBGetAll[*waveobj.Tab](ctx)
    if err != nil {
        return err
    }

    fixedCount := 0
    for _, tab := range tabs {
        layout, err := wstore.DBGet[*waveobj.LayoutState](ctx, tab.LayoutState)
        if err != nil || layout.LeafOrder == nil {
            continue
        }

        // Find orphaned blocks (in layout but not in tab.BlockIds)
        blockIdSet := make(map[string]bool)
        for _, bid := range tab.BlockIds {
            blockIdSet[bid] = true
        }

        orphanedBlocks := []string{}
        for _, leaf := range *layout.LeafOrder {
            if !blockIdSet[leaf.BlockId] {
                orphanedBlocks = append(orphanedBlocks, leaf.BlockId)
            }
        }

        // Queue cleanup actions
        if len(orphanedBlocks) > 0 {
            log.Printf("Found %d orphaned blocks in tab %s", len(orphanedBlocks), tab.OID)
            actions := make([]waveobj.LayoutActionData, len(orphanedBlocks))
            for i, blockId := range orphanedBlocks {
                actions[i] = waveobj.LayoutActionData{
                    ActionType: LayoutActionDataType_Remove,
                    BlockId:    blockId,
                }
            }
            QueueLayoutActionForTab(ctx, tab.OID, actions...)
            fixedCount++
        }
    }

    log.Printf("Migration complete: fixed %d tabs with orphaned blocks", fixedCount)
    return nil
}
```

## Testing Requirements

### Unit Tests

1. **Block Deletion Queues Layout Action**
   - Create tab with 2 blocks
   - Delete one block
   - Verify `PendingBackendActions` contains `Remove` action
   - Verify action has correct `BlockId`

2. **Orphan Detection**
   - Create layout with valid and invalid block references
   - Run validation
   - Verify orphaned blocks detected
   - Verify cleanup actions queued

3. **Migration**
   - Create tab with orphaned layout references
   - Run migration
   - Verify cleanup actions queued
   - Verify source of truth (tab.BlockIds) unchanged

### Integration Tests

1. **E2E Block Deletion**
   - Create tab with split layout (2+ blocks)
   - Delete one block via RPC
   - Wait for frontend to process actions
   - Verify layout.RootNode no longer references deleted block
   - Verify layout.LeafOrder updated

2. **Frontend Rendering**
   - Create orphaned layout state
   - Load tab in frontend
   - Verify no errors in console
   - Verify orphaned pane not rendered
   - Verify remaining panes resize to fill space

## Metrics & Monitoring

Add telemetry for orphan cleanup:

```go
telemetry.RecordTEvent(ctx, &telemetrydata.TEvent{
    Event: "layout:orphan_cleanup",
    Props: telemetrydata.TEventProps{
        OrphanCount: len(orphanedBlocks),
        TabId:       tabId,
        LayoutId:    layoutId,
    },
})
```

Track in logs:
- Number of orphaned blocks cleaned per tab
- Cleanup action success/failure rate
- Frontend errors related to missing blocks (before fix)

## Edge Cases

### Case 1: Concurrent Deletion
**Scenario:** Multiple blocks deleted simultaneously
**Solution:** Each deletion queues its own action; frontend processes sequentially

### Case 2: Subblock Deletion
**Scenario:** Deleting a block with subblocks
**Solution:** Recursive `DeleteBlock()` handles each subblock independently; each queues removal action

### Case 3: Last Block in Tab
**Scenario:** Deleting the only remaining block
**Solution:** Layout becomes empty; frontend shows empty state or default layout

### Case 4: Focused Node is Orphaned
**Scenario:** Deleted block was focused
**Solution:** Frontend `deleteNode()` already handles focus updates (see `layoutTree.ts:354`)

### Case 5: Layout Action Queue Failure
**Scenario:** Database error prevents queuing action
**Solution:** Log warning but complete block deletion; validation will catch orphan later

## Rollout Plan

### Version 0.20.0
1. ✅ Implement core fix (Approach 1)
2. ✅ Add validation (Approach 3)
3. ✅ Run migration on first startup
4. ✅ Add unit tests

### Version 0.20.1
1. Monitor telemetry for orphan cleanup events
2. Fix any edge cases discovered
3. Add E2E tests

### Version 0.21.0
1. Remove migration code (assume all instances upgraded)
2. Keep validation as permanent safety net

## References

### Code Files
- `pkg/wcore/block.go:152-227` - Block deletion logic
- `pkg/wcore/layout.go:80-112` - Layout action system
- `frontend/layout/lib/layoutModel.ts:1152-1163` - Frontend orphan cleanup
- `frontend/layout/lib/layoutTree.ts:354` - Frontend node deletion

### Database Tables
- `db_block` - Block objects
- `db_tab` - Tab objects (contains `blockids` array)
- `db_layout` - Layout state (contains `rootnode` and `leaforder`)

### Related Issues
- (To be created) GitHub issue for tracking implementation

---

**Document Version:** 1.0
**Created:** 2026-02-12
**Author:** AgentA
**Status:** Draft - Pending Review
