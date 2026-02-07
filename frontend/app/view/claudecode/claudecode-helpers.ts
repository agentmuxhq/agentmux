// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

export const TermFileName = "term";
export const TOOL_RESULT_MAX_LENGTH = 10 * 1024; // 10KB max for tool result display
export const RAW_OUTPUT_MAX_LENGTH = 256 * 1024; // 256KB max for raw terminal buffer

export function getToolOneLiner(name: string, input: any): string {
    switch (name) {
        case "Read":
            return input?.file_path ?? "";
        case "Write":
            return input?.file_path ?? "";
        case "Edit":
            return input?.file_path ?? "";
        case "Bash":
            return input?.command?.length > 60
                ? input.command.substring(0, 60) + "\u2026"
                : input?.command ?? "";
        case "Glob":
            return input?.pattern ?? "";
        case "Grep":
            return `/${input?.pattern ?? ""}/ ${input?.path ?? ""}`;
        case "Task":
            return input?.description ?? "";
        case "WebSearch":
            return input?.query ?? "";
        case "WebFetch":
            return input?.url ?? "";
        default:
            return "";
    }
}
