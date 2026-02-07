// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import type { ClaudeCodeEvent } from "./claudecode-types";

export class ClaudeCodeStreamParser {
    private buffer: string = "";
    private onEvent: (event: ClaudeCodeEvent) => void;

    constructor(onEvent: (event: ClaudeCodeEvent) => void) {
        this.onEvent = onEvent;
    }

    feedData(data: string): void {
        this.buffer += data;
        const lines = this.buffer.split("\n");
        this.buffer = lines.pop() ?? "";

        for (const line of lines) {
            const trimmed = line.trim();
            if (trimmed === "") continue;

            try {
                const event = JSON.parse(trimmed) as ClaudeCodeEvent;
                this.onEvent(event);
            } catch {
                // Non-JSON output (startup banner, prompts, etc.)
                // Emit as system message so UI can display it
                this.onEvent({
                    type: "system",
                    subtype: "raw",
                    message: trimmed,
                });
            }
        }
    }

    flush(): void {
        if (this.buffer.trim()) {
            try {
                const event = JSON.parse(this.buffer.trim()) as ClaudeCodeEvent;
                this.onEvent(event);
            } catch {
                // ignore incomplete buffer
            }
        }
        this.buffer = "";
    }

    reset(): void {
        this.buffer = "";
    }
}
