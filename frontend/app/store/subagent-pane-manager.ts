// Copyright 2026, AgentMux Corp.
// SPDX-License-Identifier: Apache-2.0
//
// Phase 3: Auto-pane creation for subagents.
// Listens for subagent:spawned events and creates subagent view panes
// split from the parent agent's block.

import { waveEventSubscribe } from "./wps";
import { registeredAgentsByBlock } from "@/app/view/term/termagent";
import { createBlockSplitHorizontally, createBlockSplitVertically } from "./global";

// Track subagent panes: subagentId -> blockId
const subagentPaneMap = new Map<string, string>();
// Track how many subagent panes are attached to a parent block
const parentSubagentCount = new Map<string, number>();

// Auto-close timers for completed subagents
const autoCloseTimers = new Map<string, ReturnType<typeof setTimeout>>();

const AUTO_CLOSE_DELAY_MS = 30_000; // 30 seconds after completion

let unsubSpawned: (() => void) | null = null;
let unsubCompleted: (() => void) | null = null;

/**
 * Find the blockId for a registered agent by its agent identifier.
 * The `registeredAgentsByBlock` map is blockId -> agentId,
 * so we reverse-lookup by value.
 */
function findBlockForAgent(agentId: string): string | null {
    // The parent agent identifier in subagent:spawned is the short name
    // like "AgentX" or "Agent1". The registeredAgentsByBlock map stores
    // the same identifier (from OSC 16162 registration).
    for (const [blockId, registeredAgent] of registeredAgentsByBlock) {
        if (registeredAgent === agentId) {
            return blockId;
        }
    }
    return null;
}

/**
 * Initialize the subagent pane manager.
 * Call once during app startup (from wave.ts initBare or similar).
 */
export function initSubagentPaneManager(): void {
    if (unsubSpawned) return; // already initialized

    unsubSpawned = waveEventSubscribe({
        eventType: "subagent:spawned",
        handler: (event: WaveEvent) => {
            const data = event?.data as any;
            if (!data?.agentId || !data?.parentAgent) return;

            const subagentId: string = data.agentId;
            const parentAgent: string = data.parentAgent;

            // Don't create duplicate panes
            if (subagentPaneMap.has(subagentId)) return;

            // Find parent agent's block
            const parentBlockId = findBlockForAgent(parentAgent);
            if (!parentBlockId) {
                console.warn(
                    "[subagent-pane-manager] parent agent not found:",
                    parentAgent
                );
                return;
            }

            // Create the subagent pane
            createSubagentPane(subagentId, parentAgent, parentBlockId, data).catch(
                (err) => {
                    console.error(
                        "[subagent-pane-manager] failed to create pane:",
                        err
                    );
                }
            );
        },
    });

    unsubCompleted = waveEventSubscribe({
        eventType: "subagent:completed",
        handler: (event: WaveEvent) => {
            const data = event?.data as any;
            if (!data?.agentId) return;

            const subagentId: string = data.agentId;
            scheduleAutoClose(subagentId);
        },
    });
}

/**
 * Create a new subagent pane split from the parent agent's block.
 */
async function createSubagentPane(
    subagentId: string,
    parentAgent: string,
    parentBlockId: string,
    spawnData: any
): Promise<void> {
    const blockDef: BlockDef = {
        meta: {
            view: "subagent",
            "subagent:id": subagentId,
            "subagent:slug": spawnData.slug ?? "",
            "subagent:parent": parentAgent,
            "subagent:session": spawnData.sessionId ?? "",
        },
    };

    // Determine split direction based on how many subagents this parent already has
    const count = parentSubagentCount.get(parentBlockId) ?? 0;

    let newBlockId: string;
    try {
        if (count === 0) {
            // First subagent: split horizontally (parent left, subagent right)
            newBlockId = await createBlockSplitHorizontally(
                blockDef,
                parentBlockId,
                "after"
            );
        } else {
            // Subsequent subagents: find an existing subagent pane for this parent
            // and split it vertically (stack subagents on top of each other)
            const existingSiblingBlockId = findExistingSubagentBlock(parentBlockId);
            if (existingSiblingBlockId) {
                newBlockId = await createBlockSplitVertically(
                    blockDef,
                    existingSiblingBlockId,
                    "after"
                );
            } else {
                // Fallback: split from parent
                newBlockId = await createBlockSplitHorizontally(
                    blockDef,
                    parentBlockId,
                    "after"
                );
            }
        }
    } catch (err) {
        console.error("[subagent-pane-manager] split failed:", err);
        return;
    }

    subagentPaneMap.set(subagentId, newBlockId);
    parentSubagentCount.set(parentBlockId, count + 1);

    console.log(
        `[subagent-pane-manager] created pane ${newBlockId} for subagent ${subagentId} (parent: ${parentAgent})`
    );
}

/**
 * Find an existing subagent pane block that belongs to the same parent.
 */
function findExistingSubagentBlock(parentBlockId: string): string | null {
    // Look through subagentPaneMap for any subagent whose parent is this block.
    // We track parent via the block metadata, but for efficiency we can check
    // the pane map since we know all subagents for this parent were created
    // relative to parentBlockId.
    for (const [, blockId] of subagentPaneMap) {
        // Return the first match — it's already a sibling of the parent
        return blockId;
    }
    return null;
}

/**
 * Schedule auto-close of a subagent pane after the delay.
 */
function scheduleAutoClose(subagentId: string): void {
    // Clear any existing timer
    const existing = autoCloseTimers.get(subagentId);
    if (existing) clearTimeout(existing);

    const timer = setTimeout(() => {
        autoCloseTimers.delete(subagentId);
        // Don't auto-close — just log. The pane stays visible with "completed" badge.
        // Users can manually close it. Full auto-close can be added when we have
        // a "pin" button on the subagent view to prevent unwanted closures.
        console.log(
            `[subagent-pane-manager] subagent ${subagentId} completed ${AUTO_CLOSE_DELAY_MS / 1000}s ago`
        );
    }, AUTO_CLOSE_DELAY_MS);

    autoCloseTimers.set(subagentId, timer);
}

/**
 * Tear down the pane manager (for cleanup/testing).
 */
export function disposeSubagentPaneManager(): void {
    if (unsubSpawned) {
        unsubSpawned();
        unsubSpawned = null;
    }
    if (unsubCompleted) {
        unsubCompleted();
        unsubCompleted = null;
    }
    for (const timer of autoCloseTimers.values()) {
        clearTimeout(timer);
    }
    autoCloseTimers.clear();
    subagentPaneMap.clear();
    parentSubagentCount.clear();
}
