// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package wcore

import (
	"context"
	"fmt"
	"log"
	"time"

	"github.com/google/uuid"
	"github.com/a5af/wavemux/pkg/waveobj"
	"github.com/a5af/wavemux/pkg/wstore"
)

const (
	LayoutActionDataType_Insert          = "insert"
	LayoutActionDataType_InsertAtIndex   = "insertatindex"
	LayoutActionDataType_Remove          = "delete"
	LayoutActionDataType_ClearTree       = "clear"
	LayoutActionDataType_Replace         = "replace"
	LayoutActionDataType_SplitHorizontal = "splithorizontal"
	LayoutActionDataType_SplitVertical   = "splitvertical"
)

type PortableLayout []struct {
	IndexArr []int             `json:"indexarr"`
	Size     *uint             `json:"size,omitempty"`
	BlockDef *waveobj.BlockDef `json:"blockdef"`
	Focused  bool              `json:"focused"`
}

func GetStarterLayout() PortableLayout {
	// Simple layout: 1 terminal + 1 sysinfo panel
	// Reverted from 4-terminal layout to fix gamerlove startup issues.
	// The 4-terminal layout caused resource exhaustion on Windows sandbox.
	// Users can manually create additional terminals as needed.
	// Layout:
	//   +-----------------+
	//   | terminal        |
	//   | (focused)       |
	//   +-----------------+
	//   | sysinfo         |
	//   +-----------------+
	return PortableLayout{
		{IndexArr: []int{0}, BlockDef: &waveobj.BlockDef{
			Meta: waveobj.MetaMapType{
				waveobj.MetaKey_View:       "term",
				waveobj.MetaKey_Controller: "shell",
			},
		}, Focused: true},
		{IndexArr: []int{1}, BlockDef: &waveobj.BlockDef{
			Meta: waveobj.MetaMapType{
				waveobj.MetaKey_View: "sysinfo",
			},
		}},
	}
}

func GetNewTabLayout() PortableLayout {
	return PortableLayout{
		{IndexArr: []int{0}, BlockDef: &waveobj.BlockDef{
			Meta: waveobj.MetaMapType{
				waveobj.MetaKey_View:       "term",
				waveobj.MetaKey_Controller: "shell",
			},
		}, Focused: true},
	}
}

func GetLayoutIdForTab(ctx context.Context, tabId string) (string, error) {
	tabObj, err := wstore.DBGet[*waveobj.Tab](ctx, tabId)
	if err != nil {
		return "", fmt.Errorf("unable to get layout id for given tab id %s: %w", tabId, err)
	}
	return tabObj.LayoutState, nil
}

func QueueLayoutAction(ctx context.Context, layoutStateId string, actions ...waveobj.LayoutActionData) error {
	layoutStateObj, err := wstore.DBGet[*waveobj.LayoutState](ctx, layoutStateId)
	if err != nil {
		return fmt.Errorf("unable to get layout state for given id %s: %w", layoutStateId, err)
	}

	for i := range actions {
		if actions[i].ActionId == "" {
			actions[i].ActionId = uuid.New().String()
		}
	}

	if layoutStateObj.PendingBackendActions == nil {
		layoutStateObj.PendingBackendActions = &actions
	} else {
		*layoutStateObj.PendingBackendActions = append(*layoutStateObj.PendingBackendActions, actions...)
	}

	err = wstore.DBUpdate(ctx, layoutStateObj)
	if err != nil {
		return fmt.Errorf("unable to update layout state with new actions: %w", err)
	}
	return nil
}

func QueueLayoutActionForTab(ctx context.Context, tabId string, actions ...waveobj.LayoutActionData) error {
	layoutStateId, err := GetLayoutIdForTab(ctx, tabId)
	if err != nil {
		return err
	}

	return QueueLayoutAction(ctx, layoutStateId, actions...)
}

func ApplyPortableLayout(ctx context.Context, tabId string, layout PortableLayout, recordTelemetry bool) error {
	actions := make([]waveobj.LayoutActionData, len(layout)+1)
	actions[0] = waveobj.LayoutActionData{ActionType: LayoutActionDataType_ClearTree}
	for i := 0; i < len(layout); i++ {
		layoutAction := layout[i]

		blockData, err := CreateBlockWithTelemetry(ctx, tabId, layoutAction.BlockDef, &waveobj.RuntimeOpts{}, recordTelemetry)
		if err != nil {
			return fmt.Errorf("unable to create block to apply portable layout to tab %s: %w", tabId, err)
		}

		actions[i+1] = waveobj.LayoutActionData{
			ActionType: LayoutActionDataType_InsertAtIndex,
			BlockId:    blockData.OID,
			IndexArr:   &layoutAction.IndexArr,
			NodeSize:   layoutAction.Size,
			Focused:    layoutAction.Focused,
		}
	}

	err := QueueLayoutActionForTab(ctx, tabId, actions...)
	if err != nil {
		return fmt.Errorf("unable to queue layout actions for portable layout: %w", err)
	}

	return nil
}

func BootstrapStarterLayout(ctx context.Context) error {
	ctx, cancelFn := context.WithTimeout(ctx, 2*time.Second)
	defer cancelFn()
	client, err := wstore.DBGetSingleton[*waveobj.Client](ctx)
	if err != nil {
		log.Printf("unable to find client: %v\n", err)
		return fmt.Errorf("unable to find client: %w", err)
	}

	if len(client.WindowIds) < 1 {
		return fmt.Errorf("error bootstrapping layout, no windows exist")
	}

	windowId := client.WindowIds[0]

	window, err := wstore.DBMustGet[*waveobj.Window](ctx, windowId)
	if err != nil {
		return fmt.Errorf("error getting window: %w", err)
	}

	workspace, err := wstore.DBMustGet[*waveobj.Workspace](ctx, window.WorkspaceId)
	if err != nil {
		return fmt.Errorf("error getting workspace: %w", err)
	}

	tabId := workspace.ActiveTabId

	starterLayout := GetStarterLayout()
	err = ApplyPortableLayout(ctx, tabId, starterLayout, false)
	if err != nil {
		return fmt.Errorf("error applying starter layout: %w", err)
	}

	return nil
}

// MigrateOrphanedLayouts scans all tabs and cleans up orphaned block references.
// This should be run once on app startup to fix existing orphaned layouts.
// Orphaned blocks are those in layout.LeafOrder but not in tab.BlockIds.
func MigrateOrphanedLayouts(ctx context.Context) error {
	log.Println("MigrateOrphanedLayouts: checking for orphaned layout references...")

	// Get all workspaces to find all tabs
	client, err := wstore.DBGetSingleton[*waveobj.Client](ctx)
	if err != nil {
		return fmt.Errorf("error getting client: %w", err)
	}

	fixedTabCount := 0
	totalOrphanCount := 0

	// Iterate through all windows and workspaces to find tabs
	for _, windowId := range client.WindowIds {
		window, err := wstore.DBGet[*waveobj.Window](ctx, windowId)
		if err != nil || window == nil {
			continue
		}

		workspace, err := wstore.DBGet[*waveobj.Workspace](ctx, window.WorkspaceId)
		if err != nil || workspace == nil {
			continue
		}

		// Check each tab in the workspace
		for _, tabId := range workspace.TabIds {
			tab, err := wstore.DBGet[*waveobj.Tab](ctx, tabId)
			if err != nil || tab == nil {
				continue
			}

			layout, err := wstore.DBGet[*waveobj.LayoutState](ctx, tab.LayoutState)
			if err != nil || layout == nil || layout.LeafOrder == nil {
				continue
			}

			// Build set of valid block IDs from tab
			blockIdSet := make(map[string]bool)
			for _, bid := range tab.BlockIds {
				blockIdSet[bid] = true
			}

			// Find orphaned blocks (in layout but not in tab.BlockIds)
			orphanedBlocks := []string{}
			for _, leaf := range *layout.LeafOrder {
				if !blockIdSet[leaf.BlockId] {
					orphanedBlocks = append(orphanedBlocks, leaf.BlockId)
				}
			}

			// Directly clean tree structure (more reliable than frontend actions)
			if len(orphanedBlocks) > 0 {
				log.Printf("MigrateOrphanedLayouts: found %d orphaned blocks in tab %s", len(orphanedBlocks), tab.OID)

				// Create orphan set for fast lookup
				orphanSet := make(map[string]bool)
				for _, blockId := range orphanedBlocks {
					orphanSet[blockId] = true
				}

				// Clean rootnode tree
				if layout.RootNode != nil {
					layout.RootNode = removeOrphanedNodesFromTree(layout.RootNode, orphanSet)
				}

				// Clean leaforder
				if layout.LeafOrder != nil {
					cleanedLeafOrder := make([]waveobj.LeafOrderEntry, 0)
					for _, leaf := range *layout.LeafOrder {
						if !orphanSet[leaf.BlockId] {
							cleanedLeafOrder = append(cleanedLeafOrder, leaf)
						}
					}
					layout.LeafOrder = &cleanedLeafOrder
				}

				// Increment version and persist
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
		} else {
			delete(nodeMap, "children")
		}
	}

	return nodeMap
}
