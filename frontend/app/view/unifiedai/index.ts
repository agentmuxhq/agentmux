// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Unified AI Pane - merges Wave AI chat and agent code tools into one pane.
 *
 * Phase A-1: Types and interfaces (PR #228).
 * Phase A-2: Command bridge + state management (PR #234).
 * Phase A-3: UI components (this PR).
 * Phase B: Agent controller + MCP server (future).
 * Phase C: Chat backends ported to Rust (future).
 * Phase D: Full sidecar elimination (future).
 */

// Types and constants
export {
    // Backend types
    BACKEND_TYPE_CHAT,
    BACKEND_TYPE_AGENT,
    // Agent IDs
    AGENT_CLAUDE_CODE,
    AGENT_GEMINI_CLI,
    AGENT_CODEX_CLI,
    // Roles
    ROLE_USER,
    ROLE_ASSISTANT,
    ROLE_SYSTEM,
    ROLE_TOOL,
    // Message status
    MSG_STATUS_PENDING,
    MSG_STATUS_STREAMING,
    MSG_STATUS_COMPLETE,
    MSG_STATUS_ERROR,
    MSG_STATUS_CANCELLED,
    // Tool approval
    TOOL_APPROVAL_AUTO,
    TOOL_APPROVAL_PENDING,
    TOOL_APPROVAL_APPROVED,
    TOOL_APPROVAL_DENIED,
    // Helper functions
    isTerminalStatus,
    isRunningStatus,
    createUserMessage,
    createStreamingAssistantMessage,
    createErrorMessage,
    getFullText,
    hasToolUse,
    applyAdapterEvent,
    getToolOneLiner,
} from "./unified-types";

// Type exports
export type {
    BackendType,
    AgentId,
    MessageRole,
    MessageStatus,
    ToolApprovalStatus,
    TokenUsage,
    AgentBackendConfig,
    UnifiedMessagePart,
    TextPart,
    ReasoningPart,
    ToolUsePart,
    ToolResultPart,
    FilePart,
    DiffPart,
    MetadataPart,
    ErrorPart,
    UnifiedMessage,
    UnifiedConversation,
    AgentStatus,
    AgentStatusType,
    SpawnAgentRequest,
    SpawnAgentResponse,
    AgentInputRequest,
    AgentStatusEvent,
    AdapterEvent,
} from "./unified-types";

// Adapter interfaces
export type {
    BackendAdapter,
    ChatBackendAdapter,
    AgentBackendAdapter,
    AdapterRegistry,
} from "./adapter";

// Agent API (Tauri command bridge)
export {
    spawnAgent,
    sendAgentInput,
    sendAgentText,
    interruptAgent,
    killAgent,
    getAgentStatus,
    listAgentBackends,
    onAgentOutput,
    onAgentRawLine,
    onAgentStatus,
} from "./agent-api";

export type { AgentStatusResponse, AgentOutputPayload, AgentRawLinePayload } from "./agent-api";

// State management (Jotai atoms + React hook)
export {
    availableBackendsAtom,
    backendsLoadedAtom,
    selectedBackendAtom,
    agentStatusAtom,
    agentInstanceIdAtom,
    messagesAtom,
    isStreamingAtom,
    useUnifiedAI,
} from "./useUnifiedAI";

// UI Components (Phase A-3)
export { UnifiedAIViewModel } from "./unifiedai-model";
export { UnifiedAIView } from "./unifiedai-view";
