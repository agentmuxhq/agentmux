// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Tauri IPC bridge for agent backend commands.
 *
 * Wraps Tauri's invoke() and listen() APIs into typed functions
 * that the unified AI pane uses to manage agent subprocesses.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
    AgentBackendConfig,
    AgentStatusEvent,
    AdapterEvent,
    SpawnAgentRequest,
    SpawnAgentResponse,
    AgentInputRequest,
    AgentStatusType,
} from "./unified-types";

// ---- Response types ----

export interface AgentStatusResponse {
    instance_id: string;
    status: { status: AgentStatusType; exit_code?: number; message?: string };
    backend_id: string;
}

export interface AgentOutputPayload {
    pane_id: string;
    instance_id: string;
    events: AdapterEvent[];
}

export interface AgentRawLinePayload {
    pane_id: string;
    line: string;
}

// ---- Commands ----

/**
 * Spawn a new agent subprocess for a pane.
 *
 * @returns The instance ID and initial status.
 */
export async function spawnAgent(request: SpawnAgentRequest): Promise<SpawnAgentResponse> {
    return invoke<SpawnAgentResponse>("spawn_agent", { request });
}

/**
 * Send text input to a running agent's stdin.
 */
export async function sendAgentInput(request: AgentInputRequest): Promise<void> {
    return invoke<void>("send_agent_input", { request });
}

/**
 * Send text to a running agent (convenience wrapper).
 */
export async function sendAgentText(paneId: string, text: string): Promise<void> {
    return sendAgentInput({ pane_id: paneId, text });
}

/**
 * Send SIGINT to interrupt a running agent.
 */
export async function interruptAgent(paneId: string): Promise<void> {
    return invoke<void>("interrupt_agent", { pane_id: paneId });
}

/**
 * Force-kill an agent subprocess.
 */
export async function killAgent(paneId: string): Promise<void> {
    return invoke<void>("kill_agent", { pane_id: paneId });
}

/**
 * Get the current status of an agent for a pane.
 */
export async function getAgentStatus(paneId: string): Promise<AgentStatusResponse> {
    return invoke<AgentStatusResponse>("get_agent_status", { pane_id: paneId });
}

/**
 * List available agent backends (auto-detected from PATH).
 */
export async function listAgentBackends(): Promise<AgentBackendConfig[]> {
    return invoke<AgentBackendConfig[]>("list_agent_backends");
}

// ---- Event listeners ----

/**
 * Listen for adapter events (parsed NDJSON) from an agent.
 *
 * Events arrive as the agent streams its response. Each payload
 * contains one or more AdapterEvents that should be applied to
 * the current UnifiedMessage via applyAdapterEvent().
 */
export function onAgentOutput(
    paneId: string,
    callback: (payload: AgentOutputPayload) => void
): Promise<UnlistenFn> {
    return listen<AgentOutputPayload>(`agent-output:${paneId}`, (event) => {
        callback(event.payload);
    });
}

/**
 * Listen for raw (non-NDJSON) lines from an agent's stdout.
 *
 * These are typically startup messages or log output that doesn't
 * parse as structured events.
 */
export function onAgentRawLine(
    paneId: string,
    callback: (payload: AgentRawLinePayload) => void
): Promise<UnlistenFn> {
    return listen<AgentRawLinePayload>(`agent-raw:${paneId}`, (event) => {
        callback(event.payload);
    });
}

/**
 * Listen for agent status changes (started, stopped, error).
 */
export function onAgentStatus(
    paneId: string,
    callback: (payload: AgentStatusEvent) => void
): Promise<UnlistenFn> {
    return listen<AgentStatusEvent>(`agent-status:${paneId}`, (event) => {
        callback(event.payload);
    });
}
