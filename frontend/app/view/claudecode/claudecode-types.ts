// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

// Types for Claude Code CLI --output-format stream-json.
// Events arrive as {"type":"stream_event","event":{...}} wrappers,
// except the final "result" event which is top-level.

// --- Top-level JSON line types ---

export type StreamJsonLine = StreamEventWrapper | ResultEvent | SystemEvent;

export interface StreamEventWrapper {
    type: "stream_event";
    event: StreamEvent;
}

// --- Stream events (inside wrapper) ---

export type StreamEvent =
    | MessageStartEvent
    | ContentBlockStartEvent
    | ContentBlockDeltaEvent
    | ContentBlockStopEvent
    | MessageDeltaEvent
    | MessageStopEvent
    | PingEvent
    | StreamErrorEvent;

export interface MessageStartEvent {
    type: "message_start";
    message: {
        id: string;
        role: "assistant" | "user";
        content: ContentBlock[];
        model?: string;
        stop_reason?: string | null;
        usage?: TokenUsage;
    };
}

export interface ContentBlockStartEvent {
    type: "content_block_start";
    index: number;
    content_block: ContentBlock;
}

export interface ContentBlockDeltaEvent {
    type: "content_block_delta";
    index: number;
    delta: TextDelta | InputJsonDelta;
}

export interface TextDelta {
    type: "text_delta";
    text: string;
}

export interface InputJsonDelta {
    type: "input_json_delta";
    partial_json: string;
}

export interface ContentBlockStopEvent {
    type: "content_block_stop";
    index: number;
}

export interface MessageDeltaEvent {
    type: "message_delta";
    delta: {
        stop_reason?: string;
        stop_sequence?: string | null;
    };
    usage?: TokenUsage;
}

export interface MessageStopEvent {
    type: "message_stop";
}

export interface PingEvent {
    type: "ping";
}

export interface StreamErrorEvent {
    type: "error";
    error: {
        type: string;
        message: string;
    };
}

// --- Top-level events (not wrapped) ---

export interface SystemEvent {
    type: "system";
    subtype?: string;
    message?: string;
    session_id?: string;
    tools?: string[];
    mcp_servers?: string[];
    model?: string;
}

export interface ResultEvent {
    type: "result";
    subtype?: string;
    session_id?: string;
    cost_usd?: number;
    num_turns?: number;
    duration_ms?: number;
    is_error?: boolean;
    result?: string;
}

// --- Shared ---

export interface TokenUsage {
    input_tokens: number;
    output_tokens: number;
    cache_creation_input_tokens?: number;
    cache_read_input_tokens?: number;
}

export type ContentBlock = TextBlock | ToolUseBlock | ToolResultBlock;

export interface TextBlock {
    type: "text";
    text: string;
}

export interface ToolUseBlock {
    type: "tool_use";
    id: string;
    name: string;
    input: Record<string, any>;
}

export interface ToolResultBlock {
    type: "tool_result";
    tool_use_id: string;
    content: string;
    is_error?: boolean;
}

// --- UI model types ---

export interface ConversationTurn {
    id: string;
    userInput: string;
    blocks: TurnBlock[];
    timestamp: number;
}

export type TurnBlock = TextTurnBlock | ToolTurnBlock;

export interface TextTurnBlock {
    type: "text";
    text: string;
}

export interface ToolTurnBlock {
    type: "tool";
    toolId: string;
    name: string;
    input: Record<string, any>;
    result?: string;
    isError?: boolean;
    isCollapsed: boolean;
}

export interface SessionMeta {
    model: string;
    inputTokens: number;
    outputTokens: number;
    totalCost: number;
    sessionId: string;
    isStreaming: boolean;
}
