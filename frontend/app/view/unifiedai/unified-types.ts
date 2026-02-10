// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Unified AI message types for the merged AI pane.
 *
 * TypeScript equivalents of the Rust types in `src-tauri/src/backend/ai/unified.rs`.
 * These normalize output from both chat backends (HTTP/SSE) and agent backends
 * (subprocess NDJSON streams) into a common format for the unified AI pane.
 */

// ---- Backend type constants ----

/** Chat backend: HTTP/SSE streaming (Wave AI's existing providers). */
export const BACKEND_TYPE_CHAT = "chat" as const;

/** Agent backend: subprocess with NDJSON stream protocol (Claude Code, etc.). */
export const BACKEND_TYPE_AGENT = "agent" as const;

export type BackendType = typeof BACKEND_TYPE_CHAT | typeof BACKEND_TYPE_AGENT;

// ---- Agent backend identifiers ----

export const AGENT_CLAUDE_CODE = "claudecode" as const;
export const AGENT_GEMINI_CLI = "gemini-cli" as const;
export const AGENT_CODEX_CLI = "codex-cli" as const;

export type AgentId = typeof AGENT_CLAUDE_CODE | typeof AGENT_GEMINI_CLI | typeof AGENT_CODEX_CLI | string;

// ---- Message roles ----

export const ROLE_USER = "user" as const;
export const ROLE_ASSISTANT = "assistant" as const;
export const ROLE_SYSTEM = "system" as const;
export const ROLE_TOOL = "tool" as const;

export type MessageRole = typeof ROLE_USER | typeof ROLE_ASSISTANT | typeof ROLE_SYSTEM | typeof ROLE_TOOL;

// ---- Unified message status ----

export const MSG_STATUS_PENDING = "pending" as const;
export const MSG_STATUS_STREAMING = "streaming" as const;
export const MSG_STATUS_COMPLETE = "complete" as const;
export const MSG_STATUS_ERROR = "error" as const;
export const MSG_STATUS_CANCELLED = "cancelled" as const;

export type MessageStatus =
    | typeof MSG_STATUS_PENDING
    | typeof MSG_STATUS_STREAMING
    | typeof MSG_STATUS_COMPLETE
    | typeof MSG_STATUS_ERROR
    | typeof MSG_STATUS_CANCELLED;

// ---- Tool approval status ----

export const TOOL_APPROVAL_AUTO = "auto" as const;
export const TOOL_APPROVAL_PENDING = "pending" as const;
export const TOOL_APPROVAL_APPROVED = "approved" as const;
export const TOOL_APPROVAL_DENIED = "denied" as const;

export type ToolApprovalStatus =
    | typeof TOOL_APPROVAL_AUTO
    | typeof TOOL_APPROVAL_PENDING
    | typeof TOOL_APPROVAL_APPROVED
    | typeof TOOL_APPROVAL_DENIED;

// ---- Token usage ----

export interface TokenUsage {
    input_tokens: number;
    output_tokens: number;
    cache_read_tokens?: number;
    cache_write_tokens?: number;
}

// ---- Agent backend configuration ----

export interface AgentBackendConfig {
    /** Unique identifier (e.g., "claudecode", "gemini-cli"). */
    id: string;
    /** Human-readable display name (e.g., "Claude Code"). */
    display_name: string;
    /** Path to the executable binary. */
    executable: string;
    /** Command-line arguments for streaming mode. */
    args?: string[];
    /** Environment variables for the subprocess. */
    env?: Record<string, string>;
    /** Working directory for the subprocess. */
    cwd?: string;
    /** Stream protocol: "ndjson", "sse", "raw". */
    stream_protocol?: string;
    /** Whether this agent supports MCP server connection. */
    supports_mcp?: boolean;
    /** Whether this agent supports pane-awareness tools. */
    supports_pane_awareness?: boolean;
    /** Auto-detected from PATH. */
    auto_detected?: boolean;
}

// ---- Unified message parts ----

export interface TextPart {
    type: "text";
    text: string;
}

export interface ReasoningPart {
    type: "reasoning";
    text: string;
}

export interface ToolUsePart {
    type: "tool_use";
    call_id: string;
    name: string;
    input: any;
    summary?: string;
    approval?: ToolApprovalStatus | "";
}

export interface ToolResultPart {
    type: "tool_result";
    call_id: string;
    content: string;
    is_error?: boolean;
}

export interface FilePart {
    type: "file";
    filename: string;
    mimetype?: string;
    size?: number;
    url?: string;
}

export interface DiffPart {
    type: "diff";
    path: string;
    content: string;
}

export interface MetadataPart {
    type: "metadata";
    data: any;
}

export interface ErrorPart {
    type: "error";
    message: string;
}

export type UnifiedMessagePart =
    | TextPart
    | ReasoningPart
    | ToolUsePart
    | ToolResultPart
    | FilePart
    | DiffPart
    | MetadataPart
    | ErrorPart;

// ---- Unified message ----

export interface UnifiedMessage {
    /** Unique message ID. */
    id: string;
    /** Message role. */
    role: MessageRole;
    /** Backend type that produced this message. */
    backend_type: BackendType;
    /** Specific backend/agent ID. */
    backend_id?: string;
    /** Message parts. */
    parts: UnifiedMessagePart[];
    /** Message status. */
    status: MessageStatus;
    /** Model that generated this response. */
    model?: string;
    /** Token usage. */
    usage?: TokenUsage;
    /** Unix timestamp (ms). */
    timestamp: number;
}

// ---- Unified conversation ----

export interface UnifiedConversation {
    /** Unique conversation ID. */
    id: string;
    /** Backend type. */
    backend_type: BackendType;
    /** Specific backend ID. */
    backend_id: string;
    /** Ordered messages. */
    messages: UnifiedMessage[];
    /** Current model. */
    model?: string;
    /** Cumulative token usage. */
    total_usage?: TokenUsage;
}

// ---- Agent status ----

export type AgentStatusType = "init" | "starting" | "running" | "busy" | "done" | "error";

export interface AgentStatusInit {
    status: "init";
}

export interface AgentStatusStarting {
    status: "starting";
}

export interface AgentStatusRunning {
    status: "running";
}

export interface AgentStatusBusy {
    status: "busy";
}

export interface AgentStatusDone {
    status: "done";
    exit_code: number;
}

export interface AgentStatusError {
    status: "error";
    message: string;
}

export type AgentStatus =
    | AgentStatusInit
    | AgentStatusStarting
    | AgentStatusRunning
    | AgentStatusBusy
    | AgentStatusDone
    | AgentStatusError;

// ---- Agent IPC types (Tauri commands) ----

export interface SpawnAgentRequest {
    pane_id: string;
    backend_id: string;
    cwd?: string;
    env?: Record<string, string>;
    initial_prompt?: string;
}

export interface SpawnAgentResponse {
    instance_id: string;
    status: AgentStatus;
}

export interface AgentInputRequest {
    pane_id: string;
    text?: string;
    signal?: string;
}

export interface AgentStatusEvent {
    pane_id: string;
    instance_id: string;
    status: AgentStatus;
    error?: string;
}

// ---- Adapter events ----

export interface AdapterMessageStart {
    type: "message_start";
    message_id: string;
    model?: string;
}

export interface AdapterTextDelta {
    type: "text_delta";
    text: string;
}

export interface AdapterReasoningDelta {
    type: "reasoning_delta";
    text: string;
}

export interface AdapterToolUseStart {
    type: "tool_use_start";
    call_id: string;
    name: string;
    summary?: string;
}

export interface AdapterToolUseInput {
    type: "tool_use_input";
    call_id: string;
    input: any;
}

export interface AdapterToolApprovalNeeded {
    type: "tool_approval_needed";
    call_id: string;
    name: string;
    input: any;
}

export interface AdapterToolResult {
    type: "tool_result";
    call_id: string;
    content: string;
    is_error?: boolean;
}

export interface AdapterMessageEnd {
    type: "message_end";
    usage?: TokenUsage;
}

export interface AdapterError {
    type: "error";
    message: string;
}

export interface AdapterSessionStart {
    type: "session_start";
    session_id: string;
    model?: string;
    tools: string[];
    cwd: string;
}

export interface AdapterSessionEnd {
    type: "session_end";
    total_cost_usd: number;
    usage?: TokenUsage;
    is_error: boolean;
    num_turns: number;
    duration_ms: number;
}

export type AdapterEvent =
    | AdapterMessageStart
    | AdapterTextDelta
    | AdapterReasoningDelta
    | AdapterToolUseStart
    | AdapterToolUseInput
    | AdapterToolApprovalNeeded
    | AdapterToolResult
    | AdapterMessageEnd
    | AdapterError
    | AdapterSessionStart
    | AdapterSessionEnd;

// ---- Helper functions ----

/** Check if a status is terminal (done or error). */
export function isTerminalStatus(status: AgentStatus): boolean {
    return status.status === "done" || status.status === "error";
}

/** Check if a status indicates the agent is running. */
export function isRunningStatus(status: AgentStatus): boolean {
    return status.status === "running" || status.status === "busy" || status.status === "starting";
}

/** Create a new user message. */
export function createUserMessage(id: string, text: string, backendType: BackendType): UnifiedMessage {
    return {
        id,
        role: ROLE_USER,
        backend_type: backendType,
        parts: [{ type: "text", text }],
        status: MSG_STATUS_COMPLETE,
        timestamp: Date.now(),
    };
}

/** Create a new empty streaming assistant message. */
export function createStreamingAssistantMessage(
    id: string,
    backendType: BackendType,
    backendId: string
): UnifiedMessage {
    return {
        id,
        role: ROLE_ASSISTANT,
        backend_type: backendType,
        backend_id: backendId,
        parts: [],
        status: MSG_STATUS_STREAMING,
        timestamp: Date.now(),
    };
}

/** Create an error message. */
export function createErrorMessage(id: string, message: string, backendType: BackendType): UnifiedMessage {
    return {
        id,
        role: ROLE_ASSISTANT,
        backend_type: backendType,
        parts: [{ type: "error", message }],
        status: MSG_STATUS_ERROR,
        timestamp: Date.now(),
    };
}

/** Get all text from a message's parts. */
export function getFullText(msg: UnifiedMessage): string {
    return msg.parts
        .filter((p): p is TextPart => p.type === "text")
        .map((p) => p.text)
        .join("");
}

/** Check if a message has any tool use parts. */
export function hasToolUse(msg: UnifiedMessage): boolean {
    return msg.parts.some((p) => p.type === "tool_use");
}

/**
 * Apply an adapter event to a UnifiedMessage being built.
 * Returns a new message (immutable update for React state).
 */
export function applyAdapterEvent(msg: UnifiedMessage, event: AdapterEvent): UnifiedMessage {
    const updated = { ...msg, parts: [...msg.parts] };

    switch (event.type) {
        case "message_start":
            if (event.model) {
                updated.model = event.model;
            }
            break;

        case "text_delta": {
            const lastPart = updated.parts[updated.parts.length - 1];
            if (lastPart && lastPart.type === "text") {
                // Merge with existing text part (immutable)
                updated.parts[updated.parts.length - 1] = {
                    ...lastPart,
                    text: lastPart.text + event.text,
                };
            } else {
                updated.parts.push({ type: "text", text: event.text });
            }
            break;
        }

        case "reasoning_delta": {
            const lastPart = updated.parts[updated.parts.length - 1];
            if (lastPart && lastPart.type === "reasoning") {
                updated.parts[updated.parts.length - 1] = {
                    ...lastPart,
                    text: lastPart.text + event.text,
                };
            } else {
                updated.parts.push({ type: "reasoning", text: event.text });
            }
            break;
        }

        case "tool_use_start":
            updated.parts.push({
                type: "tool_use",
                call_id: event.call_id,
                name: event.name,
                input: null,
                summary: event.summary ?? "",
                approval: "",
            });
            break;

        case "tool_use_input": {
            const idx = updated.parts.findIndex(
                (p) => p.type === "tool_use" && (p as ToolUsePart).call_id === event.call_id
            );
            if (idx >= 0) {
                updated.parts[idx] = { ...(updated.parts[idx] as ToolUsePart), input: event.input };
            }
            break;
        }

        case "tool_approval_needed": {
            const idx = updated.parts.findIndex(
                (p) => p.type === "tool_use" && (p as ToolUsePart).call_id === event.call_id
            );
            if (idx >= 0) {
                updated.parts[idx] = {
                    ...(updated.parts[idx] as ToolUsePart),
                    approval: TOOL_APPROVAL_PENDING,
                };
            }
            break;
        }

        case "tool_result":
            updated.parts.push({
                type: "tool_result",
                call_id: event.call_id,
                content: event.content,
                is_error: event.is_error ?? false,
            });
            break;

        case "message_end":
            updated.status = MSG_STATUS_COMPLETE;
            if (event.usage) {
                updated.usage = event.usage;
            }
            break;

        case "error":
            updated.parts.push({ type: "error", message: event.message });
            updated.status = MSG_STATUS_ERROR;
            break;

        // Session-level events don't modify individual messages
        case "session_start":
        case "session_end":
            break;
    }

    return updated;
}

/**
 * Get a human-readable one-liner for a tool use (for collapse headers).
 * Matches the existing claudecode-helpers.ts getToolOneLiner pattern.
 */
export function getToolOneLiner(name: string, input: any): string {
    if (!input) return name;

    switch (name) {
        case "Read":
        case "read_file":
            return input.file_path || input.path || name;
        case "Write":
        case "write_file":
            return `Write ${input.file_path || input.path || "file"}`;
        case "Edit":
            return `Edit ${input.file_path || input.path || "file"}`;
        case "Bash":
            return input.command ? `$ ${input.command}` : name;
        case "Grep":
        case "grep":
            return input.pattern ? `grep "${input.pattern}"` : name;
        case "Glob":
        case "glob":
            return input.pattern || name;
        case "read_dir":
            return input.path || name;
        default:
            return name;
    }
}
