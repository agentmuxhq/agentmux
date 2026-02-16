// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package wcore

import (
	"context"
	"encoding/json"
	"fmt"
	"log"

	"github.com/a5af/agentmux/pkg/waveobj"
	"github.com/a5af/agentmux/pkg/wstore"
)

// MigrateOrphanedLayouts scans all tabs and cleans up orphaned block references.
// This should be run once on app startup to fix existing orphaned layouts.
// Orphaned blocks are those in layout.LeafOrder but not in tab.BlockIds.
func MigrateOrphanedLayouts(ctx context.Context) error {
	log.Println("MigrateOrphanedLayouts: checking for orphaned layout references...")

	// Get ALL tabs from the database directly (not just via workspace references)
	// This catches tabs that exist but aren't listed in any workspace.TabIds
	allTabs, err := wstore.DBGetAllObjsByType[*waveobj.Tab](ctx, "tab")
	if err != nil {
		return fmt.Errorf("error getting all tabs: %w", err)
	}
	log.Printf("MigrateOrphanedLayouts: found %d total tabs in database", len(allTabs))

	fixedTabCount := 0
	totalOrphanCount := 0

	for _, tab := range allTabs {
		if tab == nil {
			continue
		}

		layout, err := wstore.DBGet[*waveobj.LayoutState](ctx, tab.LayoutState)
		if err != nil || layout == nil {
			log.Printf("MigrateOrphanedLayouts: tab %s has no layout or error: %v", tab.OID, err)
			continue
		}

		// Log current state for debugging
		log.Printf("MigrateOrphanedLayouts: checking tab %s - %d blocks in tab.BlockIds, layout.LeafOrder=%v, layout.RootNode=%v",
			tab.OID, len(tab.BlockIds), layout.LeafOrder != nil, layout.RootNode != nil)

		// Build set of valid block IDs from tab
		// ALSO verify each blockId points to a real Block in the database
		blockIdSet := make(map[string]bool)
		deletedBlockIds := []string{}
		for _, bid := range tab.BlockIds {
			block, err := wstore.DBGet[*waveobj.Block](ctx, bid)
			if err != nil {
				log.Printf("MigrateOrphanedLayouts: ERROR reading blockId %s in tab %s: %v", bid, tab.OID, err)
				deletedBlockIds = append(deletedBlockIds, bid)
			} else if block == nil {
				log.Printf("MigrateOrphanedLayouts: blockId %s in tab %s points to NULL block", bid, tab.OID)
				deletedBlockIds = append(deletedBlockIds, bid)
			} else {
				log.Printf("MigrateOrphanedLayouts: blockId %s exists in DB", bid)
				blockIdSet[bid] = true
			}
		}

		// Remove deleted blocks from tab.BlockIds first
		if len(deletedBlockIds) > 0 {
			log.Printf("MigrateOrphanedLayouts: found %d deleted blocks in tab %s", len(deletedBlockIds), tab.OID)
			cleanedBlockIds := make([]string, 0)
			for _, blockId := range tab.BlockIds {
				found := false
				for _, deletedId := range deletedBlockIds {
					if blockId == deletedId {
						found = true
						break
					}
				}
				if !found {
					cleanedBlockIds = append(cleanedBlockIds, blockId)
				}
			}
			tab.BlockIds = cleanedBlockIds
			err = wstore.DBUpdate(ctx, tab)
			if err != nil {
				log.Printf("MigrateOrphanedLayouts: error updating tab after removing deleted blocks: %v", err)
			} else {
				log.Printf("MigrateOrphanedLayouts: removed %d deleted blocks from tab.BlockIds", len(deletedBlockIds))
			}
		}

		// Find orphaned blocks (in layout but not in tab.BlockIds OR deleted from DB)
		orphanedBlocks := []string{}
		if layout.LeafOrder != nil {
			for _, leaf := range *layout.LeafOrder {
				if !blockIdSet[leaf.BlockId] {
					orphanedBlocks = append(orphanedBlocks, leaf.BlockId)
				}
			}
		}
		orphanedBlocks = append(orphanedBlocks, deletedBlockIds...)

		// Log the comparison for debugging
		if layout.LeafOrder != nil && len(*layout.LeafOrder) > 0 {
			layoutBlockIds := make([]string, 0)
			for _, leaf := range *layout.LeafOrder {
				layoutBlockIds = append(layoutBlockIds, leaf.BlockId)
				block, err := wstore.DBGet[*waveobj.Block](ctx, leaf.BlockId)
				if err != nil {
					log.Printf("MigrateOrphanedLayouts: LAYOUT blockId %s ERROR: %v", leaf.BlockId, err)
				} else if block == nil {
					log.Printf("MigrateOrphanedLayouts: LAYOUT blockId %s is NULL", leaf.BlockId)
				} else {
					log.Printf("MigrateOrphanedLayouts: LAYOUT blockId %s exists in DB", leaf.BlockId)
				}
			}
			log.Printf("MigrateOrphanedLayouts: tab %s - tab.BlockIds=%v, layout blocks=%v",
				tab.OID, tab.BlockIds, layoutBlockIds)
		}

		// Directly clean tree structure
		if len(orphanedBlocks) > 0 {
			log.Printf("MigrateOrphanedLayouts: found %d orphaned blocks in tab %s", len(orphanedBlocks), tab.OID)

			orphanSet := make(map[string]bool)
			for _, blockId := range orphanedBlocks {
				orphanSet[blockId] = true
			}

			if layout.RootNode != nil {
				layout.RootNode = removeOrphanedNodesFromTree(layout.RootNode, orphanSet)
			}

			if layout.LeafOrder != nil {
				cleanedLeafOrder := make([]waveobj.LeafOrderEntry, 0)
				for _, leaf := range *layout.LeafOrder {
					if !orphanSet[leaf.BlockId] {
						cleanedLeafOrder = append(cleanedLeafOrder, leaf)
					}
				}
				layout.LeafOrder = &cleanedLeafOrder
			}

			layout.Version++
			err = wstore.DBUpdate(ctx, layout)
			if err != nil {
				log.Printf("MigrateOrphanedLayouts: error updating layout for tab %s: %v", tab.OID, err)
				continue
			}

			fixedTabCount++
			totalOrphanCount += len(orphanedBlocks)
			log.Printf("MigrateOrphanedLayouts: cleaned %d orphans from tab %s", len(orphanedBlocks), tab.OID)
		}

		// REVERSE CHECK: Find blocks in tab.BlockIds but not in layout
		layoutBlockSet := make(map[string]bool)
		if layout.LeafOrder != nil {
			for _, leaf := range *layout.LeafOrder {
				layoutBlockSet[leaf.BlockId] = true
			}
		}

		deadBlocks := []string{}
		for _, blockId := range tab.BlockIds {
			if !layoutBlockSet[blockId] {
				deadBlocks = append(deadBlocks, blockId)
			}
		}

		if len(deadBlocks) > 0 {
			log.Printf("MigrateOrphanedLayouts: found %d dead blocks (in tab but not layout) in tab %s", len(deadBlocks), tab.OID)

			cleanedBlockIds := make([]string, 0)
			for _, blockId := range tab.BlockIds {
				if layoutBlockSet[blockId] {
					cleanedBlockIds = append(cleanedBlockIds, blockId)
				}
			}

			tab.BlockIds = cleanedBlockIds
			err = wstore.DBUpdate(ctx, tab)
			if err != nil {
				log.Printf("MigrateOrphanedLayouts: error updating tab %s: %v", tab.OID, err)
				continue
			}

			if fixedTabCount == 0 {
				fixedTabCount = 1
			}
			totalOrphanCount += len(deadBlocks)
			log.Printf("MigrateOrphanedLayouts: removed %d dead blocks from tab %s", len(deadBlocks), tab.OID)
		}

		// Prune empty parent nodes from the layout tree
		// This catches empty containers left by previous orphan removals
		if layout.RootNode != nil {
			beforeJson, _ := json.Marshal(layout.RootNode)
			prunedRoot := pruneEmptyParentNodes(layout.RootNode)
			afterJson, _ := json.Marshal(prunedRoot)
			if string(beforeJson) != string(afterJson) {
				log.Printf("MigrateOrphanedLayouts: pruned empty parent nodes from tab %s", tab.OID)
				layout.RootNode = prunedRoot
				layout.Version++
				err = wstore.DBUpdate(ctx, layout)
				if err != nil {
					log.Printf("MigrateOrphanedLayouts: error updating layout after pruning for tab %s: %v", tab.OID, err)
				} else {
					if fixedTabCount == 0 {
						fixedTabCount = 1
					}
				}
			}
		}
	}

	log.Printf("MigrateOrphanedLayouts: complete - fixed %d tabs, cleaned %d orphaned blocks", fixedTabCount, totalOrphanCount)
	return nil
}

// removeOrphanedNodesFromTree recursively removes nodes with orphaned blockIds from the layout tree
func removeOrphanedNodesFromTree(node any, orphanSet map[string]bool) any {
	if node == nil {
		return nil
	}

	// Cast to map to access fields
	nodeMap, ok := node.(map[string]any)
	if !ok {
		return node
	}

	// Check if this node has data with an orphaned blockId
	if data, hasData := nodeMap["data"].(map[string]any); hasData {
		if blockId, hasBlockId := data["blockId"].(string); hasBlockId {
			if orphanSet[blockId] {
				nodeId := nodeMap["id"]
				log.Printf("removeOrphanedNodesFromTree: removing orphaned node %v with blockId %s", nodeId, blockId)
				return nil
			}
		}
	}

	// Recursively clean children
	if children, hasChildren := nodeMap["children"].([]any); hasChildren && len(children) > 0 {
		cleanedChildren := make([]any, 0)
		for _, child := range children {
			cleaned := removeOrphanedNodesFromTree(child, orphanSet)
			if cleaned != nil {
				cleanedChildren = append(cleanedChildren, cleaned)
			}
		}

		if len(cleanedChildren) > 0 {
			nodeMap["children"] = cleanedChildren
			// If only one child remains, promote it to replace this parent
			if len(cleanedChildren) == 1 {
				log.Printf("removeOrphanedNodesFromTree: promoting single child to replace parent %v", nodeMap["id"])
				return cleanedChildren[0]
			}
		} else {
			// All children were removed - this parent node is now empty, remove it too
			log.Printf("removeOrphanedNodesFromTree: removing empty parent node %v (all children were orphaned)", nodeMap["id"])
			return nil
		}
	}

	return nodeMap
}

// pruneEmptyParentNodes walks the tree and removes parent nodes that have no children
// and no blockId. This handles cases where previous migrations left empty containers.
func pruneEmptyParentNodes(node any) any {
	if node == nil {
		return nil
	}

	nodeMap, ok := node.(map[string]any)
	if !ok {
		return node
	}

	// If this node has children, recursively prune them first
	if children, hasChildren := nodeMap["children"].([]any); hasChildren && len(children) > 0 {
		cleanedChildren := make([]any, 0)
		for _, child := range children {
			pruned := pruneEmptyParentNodes(child)
			if pruned != nil {
				cleanedChildren = append(cleanedChildren, pruned)
			}
		}

		if len(cleanedChildren) > 0 {
			nodeMap["children"] = cleanedChildren
			// Promote single child to replace parent
			if len(cleanedChildren) == 1 {
				log.Printf("pruneEmptyParentNodes: promoting single child to replace parent %v", nodeMap["id"])
				return cleanedChildren[0]
			}
		} else {
			// No children left - check if this node is a leaf with a blockId
			if data, hasData := nodeMap["data"].(map[string]any); hasData {
				if _, hasBlockId := data["blockId"].(string); hasBlockId {
					// Has a blockId, keep it as a leaf
					delete(nodeMap, "children")
					return nodeMap
				}
			}
			// No children and no blockId - remove this empty container
			log.Printf("pruneEmptyParentNodes: removing empty parent node %v", nodeMap["id"])
			return nil
		}
	} else {
		// No children (key missing or empty) - check if it's a valid leaf (has blockId)
		if data, hasData := nodeMap["data"].(map[string]any); hasData {
			if _, hasBlockId := data["blockId"].(string); hasBlockId {
				return nodeMap // Valid leaf node
			}
		}
		// No children and no blockId - this is an empty container node, remove it
		log.Printf("pruneEmptyParentNodes: removing childless parent node %v (no children, no blockId)", nodeMap["id"])
		return nil
	}

	return nodeMap
}
