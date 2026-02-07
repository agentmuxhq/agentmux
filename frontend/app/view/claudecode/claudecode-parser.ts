// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import type {
    ResultEvent,
    StreamEvent,
    StreamEventWrapper,
    SystemEvent,
    ToolResultBlock,
} from "./claudecode-types";

// Callback interface for parsed events
export interface ParserCallbacks {
    onSystemEvent: (event: SystemEvent) => void;
    onMessageStart: (role: string, model?: string, usage?: any) => void;
    onTextDelta: (text: string) => void;
    onToolUseStart: (id: string, name: string) => void;
    onToolUseFinish: (id: string, parsedInput: Record<string, any>) => void;
    onToolResult: (toolUseId: string, content: string, isError?: boolean) => void;
    onMessageStop: () => void;
    onUsageUpdate: (usage: any) => void;
    onResultEvent: (event: ResultEvent) => void;
    onError: (errorType: string, message: string) => void;
}

/**
 * Parser for Claude Code CLI --output-format stream-json.
 *
 * Handles stream_event wrappers, top-level system/result events,
 * incremental text_delta and input_json_delta accumulation,
 * and NDJSON line splitting with buffer for partial lines.
 */
export class ClaudeCodeStreamParser {
    private buffer: string = "";
    private callbacks: ParserCallbacks;

    // Track tool input accumulation
    private activeToolId: string | null = null;
    private toolInputBuffer: string = "";

    constructor(callbacks: ParserCallbacks) {
        this.callbacks = callbacks;
    }

    feedData(data: string): void {
        this.buffer += data;
        const lines = this.buffer.split("\n");
        this.buffer = lines.pop() ?? "";

        for (const line of lines) {
            const trimmed = line.trim();
            if (trimmed === "") continue;
            this.parseLine(trimmed);
        }
    }

    private parseLine(line: string): void {
        let parsed: any;
        try {
            parsed = JSON.parse(line);
        } catch {
            return; // Non-JSON output, ignore
        }

        if (!parsed || !parsed.type) return;

        if (parsed.type === "stream_event") {
            this.handleStreamEvent((parsed as StreamEventWrapper).event);
        } else if (parsed.type === "result") {
            this.callbacks.onResultEvent(parsed as ResultEvent);
        } else if (parsed.type === "system") {
            this.callbacks.onSystemEvent(parsed as SystemEvent);
        }
    }

    private handleStreamEvent(event: StreamEvent): void {
        switch (event.type) {
            case "message_start": {
                const msg = event.message;
                if (msg.role === "user" && msg.content) {
                    // Tool result messages come as role:"user" with tool_result content
                    for (const block of msg.content) {
                        if (block.type === "tool_result") {
                            const tr = block as ToolResultBlock;
                            this.callbacks.onToolResult(tr.tool_use_id, tr.content, tr.is_error);
                        }
                    }
                } else {
                    this.callbacks.onMessageStart(msg.role, msg.model, msg.usage);
                }
                break;
            }

            case "content_block_start": {
                const cb = event.content_block;
                if (cb.type === "text" && cb.text) {
                    this.callbacks.onTextDelta(cb.text);
                } else if (cb.type === "tool_use") {
                    this.activeToolId = cb.id;
                    this.toolInputBuffer = "";
                    this.callbacks.onToolUseStart(cb.id, cb.name);
                }
                break;
            }

            case "content_block_delta": {
                const delta = event.delta;
                if (delta.type === "text_delta") {
                    this.callbacks.onTextDelta(delta.text);
                } else if (delta.type === "input_json_delta") {
                    this.toolInputBuffer += delta.partial_json;
                }
                break;
            }

            case "content_block_stop": {
                if (this.activeToolId) {
                    let parsedInput: Record<string, any> = {};
                    try {
                        parsedInput = JSON.parse(this.toolInputBuffer);
                    } catch {
                        parsedInput = { _raw: this.toolInputBuffer };
                    }
                    this.callbacks.onToolUseFinish(this.activeToolId, parsedInput);
                    this.activeToolId = null;
                    this.toolInputBuffer = "";
                }
                break;
            }

            case "message_delta": {
                if (event.usage) {
                    this.callbacks.onUsageUpdate(event.usage);
                }
                break;
            }

            case "message_stop": {
                this.callbacks.onMessageStop();
                break;
            }

            case "error": {
                this.callbacks.onError(event.error.type, event.error.message);
                break;
            }

            case "ping":
                break; // Keep-alive, ignore
        }
    }

    flush(): void {
        if (this.buffer.trim()) {
            this.parseLine(this.buffer.trim());
        }
        this.buffer = "";
    }

    reset(): void {
        this.buffer = "";
        this.activeToolId = null;
        this.toolInputBuffer = "";
    }
}
