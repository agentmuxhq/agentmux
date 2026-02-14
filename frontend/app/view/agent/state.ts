// Copyright 2024, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * State management for agent widget using Jotai atoms
 */

import { atom } from "jotai";
import {
    AgentProcessState,
    DocumentNode,
    DocumentState,
    MessageRouterState,
    StreamingState,
} from "./types";

/**
 * Agent ID for this widget instance
 * Each widget instance monitors one specific agent
 */
export const agentIdAtom = atom<string>("");

/**
 * Document nodes that make up the agent's markdown document
 * This is the core data structure - an ordered list of nodes
 */
export const agentDocumentAtom = atom<DocumentNode[]>([]);

/**
 * Document state (collapsed nodes, scroll position, filters)
 */
export const documentStateAtom = atom<DocumentState>({
    collapsedNodes: new Set<string>(),
    scrollPosition: 0,
    selectedNode: null,
    filter: {
        showThinking: false, // Hide thinking by default
        showSuccessfulTools: true, // Show successful tools
        showFailedTools: true, // Always show failures
        showIncoming: true, // Show incoming messages
        showOutgoing: true, // Show outgoing messages
    },
});

/**
 * Streaming state (active, buffer size, last event time)
 */
export const streamingStateAtom = atom<StreamingState>({
    active: false,
    agentId: null,
    bufferSize: 0,
    lastEventTime: 0,
});

/**
 * Agent process state (pid, status, can restart/kill)
 */
export const agentProcessAtom = atom<AgentProcessState>({
    pid: undefined,
    agentId: "",
    status: "idle",
    canRestart: true,
    canKill: false,
});

/**
 * Message router state (backend connection)
 */
export const messageRouterAtom = atom<MessageRouterState>({
    backend: "local",
    connected: false,
    endpoint: "",
});

/**
 * Derived atom: Filtered document nodes
 * Applies filter state to document nodes
 */
export const filteredDocumentAtom = atom((get) => {
    const document = get(agentDocumentAtom);
    const state = get(documentStateAtom);

    return document.filter((node) => {
        // Filter thinking blocks
        if (node.type === "markdown" && node.metadata?.thinking && !state.filter.showThinking) {
            return false;
        }

        // Filter successful tools
        if (
            node.type === "tool" &&
            node.status === "success" &&
            !state.filter.showSuccessfulTools
        ) {
            return false;
        }

        // Filter failed tools
        if (node.type === "tool" && node.status === "failed" && !state.filter.showFailedTools) {
            return false;
        }

        // Filter incoming agent messages
        if (
            node.type === "agent_message" &&
            node.direction === "incoming" &&
            !state.filter.showIncoming
        ) {
            return false;
        }

        // Filter outgoing agent messages
        if (
            node.type === "agent_message" &&
            node.direction === "outgoing" &&
            !state.filter.showOutgoing
        ) {
            return false;
        }

        return true;
    });
});

/**
 * Derived atom: Document statistics
 */
export const documentStatsAtom = atom((get) => {
    const document = get(agentDocumentAtom);

    const stats = {
        totalNodes: document.length,
        markdownNodes: 0,
        toolNodes: 0,
        successfulTools: 0,
        failedTools: 0,
        runningTools: 0,
        agentMessages: 0,
        userMessages: 0,
    };

    document.forEach((node) => {
        switch (node.type) {
            case "markdown":
                stats.markdownNodes++;
                break;
            case "tool":
                stats.toolNodes++;
                if (node.status === "success") stats.successfulTools++;
                if (node.status === "failed") stats.failedTools++;
                if (node.status === "running") stats.runningTools++;
                break;
            case "agent_message":
                stats.agentMessages++;
                break;
            case "user_message":
                stats.userMessages++;
                break;
        }
    });

    return stats;
});

/**
 * Action: Toggle node collapsed state
 */
export const toggleNodeCollapsed = atom(null, (get, set, nodeId: string) => {
    const state = get(documentStateAtom);
    const collapsed = new Set(state.collapsedNodes);

    if (collapsed.has(nodeId)) {
        collapsed.delete(nodeId);
    } else {
        collapsed.add(nodeId);
    }

    set(documentStateAtom, { ...state, collapsedNodes: collapsed });
});

/**
 * Action: Expand all nodes
 */
export const expandAllNodes = atom(null, (get, set) => {
    const state = get(documentStateAtom);
    set(documentStateAtom, { ...state, collapsedNodes: new Set() });
});

/**
 * Action: Collapse all nodes
 */
export const collapseAllNodes = atom(null, (get, set) => {
    const document = get(agentDocumentAtom);
    const allNodeIds = document.map((node) => node.id);

    const state = get(documentStateAtom);
    set(documentStateAtom, { ...state, collapsedNodes: new Set(allNodeIds) });
});

/**
 * Action: Append node to document
 */
export const appendDocumentNode = atom(null, (get, set, node: DocumentNode) => {
    const document = get(agentDocumentAtom);
    set(agentDocumentAtom, [...document, node]);
});

/**
 * Action: Update existing node (by ID)
 * Used to update tool nodes when results arrive
 */
export const updateDocumentNode = atom(null, (get, set, updatedNode: DocumentNode) => {
    const document = get(agentDocumentAtom);
    const index = document.findIndex((node) => node.id === updatedNode.id);

    if (index !== -1) {
        const newDocument = [...document];
        newDocument[index] = updatedNode;
        set(agentDocumentAtom, newDocument);
    }
});

/**
 * Action: Clear document
 */
export const clearDocument = atom(null, (get, set) => {
    set(agentDocumentAtom, []);
    set(documentStateAtom, {
        collapsedNodes: new Set(),
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
});

/**
 * Action: Update filter state
 */
export const updateFilter = atom(null, (get, set, filterUpdates: Partial<DocumentState["filter"]>) => {
    const state = get(documentStateAtom);
    set(documentStateAtom, {
        ...state,
        filter: { ...state.filter, ...filterUpdates },
    });
});
