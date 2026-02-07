// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

// Types for Claude Code pane stream-json parsing

export type ClaudeCodeEvent =
    | SystemEvent
    | AssistantMessageEvent
    | ContentBlockStartEvent
    | ContentBlockDeltaEvent
    | ContentBlockStopEvent
    | ToolUseEvent
    | ToolResultEvent
    | ResultEvent;

export interface SystemEvent {
    type: "system";
    subtype?: string;
    message?: string;
    session_id?: string;
    tools?: string[];
    mcp_servers?: string[];
    model?: string;
}

export interface AssistantMessageEvent {
    type: "assistant";
    message: {
        role: "assistant";
        content: ContentBlock[];
        model?: string;
        stop_reason?: string;
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
    delta: {
        type: string;
        text?: string;
        partial_json?: string;
    };
}

export interface ContentBlockStopEvent {
    type: "content_block_stop";
    index: number;
}

export interface ToolUseEvent {
    type: "tool_use";
    id: string;
    name: string;
    input: Record<string, any>;
}

export interface ToolResultEvent {
    type: "tool_result";
    tool_use_id: string;
    content: string;
    is_error?: boolean;
}

export interface ResultEvent {
    type: "result";
    subtype?: string;
    duration_ms?: number;
    duration_api_ms?: number;
    is_error?: boolean;
    num_turns?: number;
    result?: string;
    session_id?: string;
    total_cost?: number;
    usage?: TokenUsage;
}

export interface TokenUsage {
    input_tokens: number;
    output_tokens: number;
    cache_creation_input_tokens?: number;
    cache_read_input_tokens?: number;
}

export type ContentBlock = TextBlock | ToolUseBlock;

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

// UI model types

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
