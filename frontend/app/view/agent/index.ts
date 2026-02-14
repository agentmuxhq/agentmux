// Copyright 2024, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Unified Agent Widget - Main exports
 */

export { AgentView } from "./agent-view";
export { ClaudeCodeStreamParser } from "./stream-parser";
export * from "./types";
export * from "./state";

// Component exports
export { MarkdownBlock } from "./components/MarkdownBlock";
export { ToolBlock } from "./components/ToolBlock";
export { AgentMessageBlock } from "./components/AgentMessageBlock";
export { DiffViewer } from "./components/DiffViewer";
export { BashOutputViewer } from "./components/BashOutputViewer";
