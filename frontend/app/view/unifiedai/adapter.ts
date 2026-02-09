// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Adapter interface for normalizing different AI backends into UnifiedMessage format.
 *
 * Each adapter translates its native event format (SSE events, NDJSON lines, etc.)
 * into AdapterEvent values that the unified pane controller processes.
 */

import type { AdapterEvent, AgentBackendConfig, BackendType, UnifiedMessage } from "./unified-types";

/**
 * Backend adapter interface.
 *
 * Implementations translate native backend events into AdapterEvent values.
 * The unified pane controller calls `processEvent()` as events arrive and
 * uses the adapter's identity for labeling messages.
 */
export interface BackendAdapter {
    /** Backend type: "chat" or "agent". */
    readonly backendType: BackendType;

    /** Specific backend identifier (e.g., "claudecode", "openai"). */
    readonly backendId: string;

    /** Human-readable display name. */
    readonly displayName: string;

    /**
     * Process a native event from the backend.
     * Returns zero or more AdapterEvent values to apply to the current message.
     *
     * @param event - Native event data (format depends on adapter)
     * @returns Array of normalized AdapterEvent values
     */
    processEvent(event: unknown): AdapterEvent[];

    /**
     * Check if the backend binary is available.
     * For agent backends, this checks if the executable exists in PATH.
     * For chat backends, this checks if API credentials are configured.
     *
     * @returns Promise resolving to true if available
     */
    isAvailable(): Promise<boolean>;

    /**
     * Get the configuration for this backend.
     * For agent backends, returns the AgentBackendConfig.
     * For chat backends, returns the AI options.
     */
    getConfig(): AgentBackendConfig | Record<string, unknown>;
}

/**
 * Chat backend adapter.
 *
 * Wraps the existing Wave AI streaming (AIStreamEvent) and translates
 * each event into AdapterEvent values. This lets the existing chat
 * backends work with the unified pane without modification.
 */
export interface ChatBackendAdapter extends BackendAdapter {
    readonly backendType: "chat";

    /** Process an AIStreamEvent from the existing streaming pipeline. */
    processStreamEvent(event: unknown): AdapterEvent[];
}

/**
 * Agent backend adapter.
 *
 * Wraps an agent subprocess (Claude Code, Gemini CLI, etc.) and translates
 * the subprocess's NDJSON output into AdapterEvent values.
 */
export interface AgentBackendAdapter extends BackendAdapter {
    readonly backendType: "agent";

    /** Process a line of NDJSON output from the agent subprocess. */
    processLine(line: string): AdapterEvent[];

    /** Process raw/pre-JSON output (e.g., Claude Code auth URLs). */
    processRawOutput?(text: string): AdapterEvent[];

    /** Get the agent-specific configuration. */
    getConfig(): AgentBackendConfig;
}

/**
 * Registry of available backend adapters.
 *
 * The unified pane queries this to discover which backends are available
 * and to get the appropriate adapter for a given backend ID.
 */
export interface AdapterRegistry {
    /** Get all registered adapters. */
    getAll(): BackendAdapter[];

    /** Get adapter by backend ID. */
    get(backendId: string): BackendAdapter | undefined;

    /** Get all available adapters (binary exists, credentials configured). */
    getAvailable(): Promise<BackendAdapter[]>;

    /** Register a new adapter. */
    register(adapter: BackendAdapter): void;
}
