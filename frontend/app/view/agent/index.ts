// Copyright 2024, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Unified Agent Widget - Main exports
 */

// Barrel file - re-exports for external consumers
export { AgentViewModel } from "./agent-model";
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
export { AgentHeader } from "./components/AgentHeader";
export { AgentFooter } from "./components/AgentFooter";
export { FilterControls } from "./components/FilterControls";
export { ProcessControls } from "./components/ProcessControls";
export { SetupWizard } from "./components/SetupWizard";
export { ConnectionStatus } from "./components/ConnectionStatus";

// Provider exports
export { PROVIDERS, getProvider, getProviderList } from "./providers";
export { createTranslator } from "./providers/translator-factory";
export type { OutputTranslator } from "./providers/translator";
export type { ProviderDefinition } from "./providers";
