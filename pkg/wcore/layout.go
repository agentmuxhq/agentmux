// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

package wcore

import (
	"context"
	"fmt"
	"log"

	"github.com/google/uuid"
	"github.com/a5af/agentmux/pkg/waveobj"
	"github.com/a5af/agentmux/pkg/wstore"
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
	return ApplyPortableLayoutWithMode(ctx, tabId, layout, recordTelemetry, false)
}

// ApplyPortableLayoutWithMode applies a portable layout either synchronously or asynchronously.
// synchronous=true: Directly builds layout tree in database (prevents race conditions on bootstrap)
// synchronous=false: Queues actions for frontend to process (existing behavior)
func ApplyPortableLayoutWithMode(ctx context.Context, tabId string, layout PortableLayout, recordTelemetry bool, synchronous bool) error {
	if synchronous {
		return applyPortableLayoutDirect(ctx, tabId, layout, recordTelemetry)
	}

	// Original async behavior: queue actions for frontend
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

// applyPortableLayoutDirect builds the layout tree synchronously and writes directly to the database.
// This prevents race conditions where blocks are created before the layout tree is populated.
func applyPortableLayoutDirect(ctx context.Context, tabId string, layout PortableLayout, recordTelemetry bool) error {
	// Get the layout state ID
	layoutStateId, err := GetLayoutIdForTab(ctx, tabId)
	if err != nil {
		return fmt.Errorf("unable to get layout id for tab: %w", err)
	}

	layoutState, err := wstore.DBGet[*waveobj.LayoutState](ctx, layoutStateId)
	if err != nil {
		return fmt.Errorf("unable to get layout state: %w", err)
	}

	// Create blocks and build tree nodes
	var rootNode map[string]any
	var leafOrder []waveobj.LeafOrderEntry
	var focusedNodeId string

	if len(layout) == 0 {
		// Empty layout
		rootNode = nil
		leafOrder = []waveobj.LeafOrderEntry{}
	} else if len(layout) == 1 {
		// Single block - no container needed
		layoutAction := layout[0]
		blockData, err := CreateBlockWithTelemetry(ctx, tabId, layoutAction.BlockDef, &waveobj.RuntimeOpts{}, recordTelemetry)
		if err != nil {
			return fmt.Errorf("unable to create block: %w", err)
		}

		nodeId := uuid.NewString()
		rootNode = map[string]any{
			"id": nodeId,
			"data": map[string]any{
				"blockId": blockData.OID,
			},
		}

		leafOrder = []waveobj.LeafOrderEntry{
			{NodeId: nodeId, BlockId: blockData.OID},
		}

		if layoutAction.Focused {
			focusedNodeId = nodeId
		}
	} else {
		// Multiple blocks - create vertical container
		children := make([]any, len(layout))
		leafOrder = make([]waveobj.LeafOrderEntry, len(layout))

		for i, layoutAction := range layout {
			blockData, err := CreateBlockWithTelemetry(ctx, tabId, layoutAction.BlockDef, &waveobj.RuntimeOpts{}, recordTelemetry)
			if err != nil {
				return fmt.Errorf("unable to create block %d: %w", i, err)
			}

			nodeId := uuid.NewString()
			children[i] = map[string]any{
				"id": nodeId,
				"data": map[string]any{
					"blockId": blockData.OID,
				},
			}

			leafOrder[i] = waveobj.LeafOrderEntry{
				NodeId:  nodeId,
				BlockId: blockData.OID,
			}

			if layoutAction.Focused {
				focusedNodeId = nodeId
			}
		}

		rootNode = map[string]any{
			"id":            uuid.NewString(),
			"flexDirection": "column",
			"children":      children,
		}
	}

	// Update layout state with new tree
	layoutState.RootNode = rootNode
	layoutState.LeafOrder = &leafOrder
	layoutState.FocusedNodeId = focusedNodeId
	layoutState.Version++

	err = wstore.DBUpdate(ctx, layoutState)
	if err != nil {
		return fmt.Errorf("unable to update layout state: %w", err)
	}

	log.Printf("applyPortableLayoutDirect: applied %d blocks to tab %s synchronously", len(layout), tabId)
	return nil
}
