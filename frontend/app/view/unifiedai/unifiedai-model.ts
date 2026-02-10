// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

import { BlockNodeModel } from "@/app/block/blocktypes";
import { WOS, globalStore } from "@/app/store/global";
import { atom, Atom, PrimitiveAtom } from "jotai";
import React from "react";

import type {
    AgentBackendConfig,
    AgentStatusType,
    UnifiedMessage,
    TokenUsage,
    AdapterEvent,
} from "./unified-types";
import {
    createUserMessage,
    createErrorMessage,
    createStreamingAssistantMessage,
    applyAdapterEvent,
    BACKEND_TYPE_AGENT,
} from "./unified-types";
import {
    spawnAgent,
    sendAgentText,
    interruptAgent,
    killAgent,
    listAgentBackends,
    onAgentOutput,
    onAgentStatus,
    type AgentOutputPayload,
} from "./agent-api";
import { UnifiedAIView } from "./unifiedai-view";

/**
 * ViewModel for the Unified AI pane — one pane for all AI backends.
 *
 * Follows the ClaudeCodeViewModel pattern:
 * - Constructor takes (blockId, nodeModel) per ViewModelClass contract
 * - Creates jotai atoms for reactive state
 * - Manages agent subprocess lifecycle via Tauri commands
 * - Bridges to UnifiedAIView for rendering
 */
export class UnifiedAIViewModel implements ViewModel {
    viewType = "unifiedai";
    blockId: string;
    nodeModel: BlockNodeModel;
    blockAtom: Atom<Block>;

    // ViewModel interface
    viewIcon: Atom<string>;
    viewName: Atom<string>;
    viewText: Atom<string | HeaderElem[]>;
    viewComponent: ViewComponent;
    noPadding = atom(true);
    endIconButtons: Atom<IconButtonDecl[]>;

    // State atoms
    messagesAtom: PrimitiveAtom<UnifiedMessage[]>;
    statusAtom: PrimitiveAtom<AgentStatusType>;
    instanceIdAtom: PrimitiveAtom<string>;
    selectedBackendAtom: PrimitiveAtom<string>;
    availableBackendsAtom: PrimitiveAtom<AgentBackendConfig[]>;
    isStreamingAtom: PrimitiveAtom<boolean>;
    totalUsageAtom: PrimitiveAtom<TokenUsage>;
    sessionIdAtom: PrimitiveAtom<string>;
    totalCostAtom: PrimitiveAtom<number>;
    inputRef: React.RefObject<HTMLTextAreaElement>;

    // Internal
    private streamingMsg: UnifiedMessage | null = null;
    private unlistenFns: Array<() => void> = [];
    private backendsLoaded = false;

    constructor(blockId: string, nodeModel: BlockNodeModel) {
        this.blockId = blockId;
        this.nodeModel = nodeModel;
        this.blockAtom = WOS.getWaveObjectAtom<Block>(`block:${blockId}`);
        this.viewComponent = UnifiedAIView;

        this.viewIcon = atom("sparkles");
        this.viewName = atom("AI");

        this.messagesAtom = atom<UnifiedMessage[]>([]);
        this.statusAtom = atom<AgentStatusType>("init");
        this.instanceIdAtom = atom<string>("");
        this.selectedBackendAtom = atom<string>("");
        this.availableBackendsAtom = atom<AgentBackendConfig[]>([]);
        this.isStreamingAtom = atom(false);
        this.totalUsageAtom = atom<TokenUsage>({
            input_tokens: 0,
            output_tokens: 0,
        });
        this.sessionIdAtom = atom<string>("");
        this.totalCostAtom = atom<number>(0);
        this.inputRef = { current: null } as React.RefObject<HTMLTextAreaElement>;

        // Header text: show backend + token/cost info
        this.viewText = atom((get) => {
            const backend = get(this.selectedBackendAtom);
            const backends = get(this.availableBackendsAtom);
            const usage = get(this.totalUsageAtom);
            const cost = get(this.totalCostAtom);

            const parts: HeaderElem[] = [];

            // Show selected backend display name
            const cfg = backends.find((b) => b.id === backend);
            if (cfg) {
                parts.push({ elemtype: "text", text: cfg.display_name });
            }

            // Show token count
            const totalTokens = usage.input_tokens + usage.output_tokens;
            if (totalTokens > 0) {
                parts.push({
                    elemtype: "text",
                    text: `${(totalTokens / 1000).toFixed(1)}k`,
                });
            }

            // Show cost if available
            if (cost > 0) {
                parts.push({
                    elemtype: "text",
                    text: `$${cost.toFixed(3)}`,
                });
            }

            return parts;
        });

        // End icon buttons
        this.endIconButtons = atom(() => [
            {
                elemtype: "iconbutton",
                icon: "rotate-right",
                title: "Reset",
                click: () => this.resetSession(),
            },
        ]);

        // Auto-detect backends
        this.loadBackends();
    }

    // ---- Public API ----

    async loadBackends(): Promise<void> {
        if (this.backendsLoaded) return;
        try {
            const backends = await listAgentBackends();
            globalStore.set(this.availableBackendsAtom, backends);
            this.backendsLoaded = true;

            // Auto-select first if none selected
            const current = globalStore.get(this.selectedBackendAtom);
            if (!current && backends.length > 0) {
                globalStore.set(this.selectedBackendAtom, backends[0].id);
            }
        } catch (err) {
            console.error("Failed to list agent backends:", err);
        }
    }

    async startAgent(cwd?: string, initialPrompt?: string): Promise<void> {
        const backend = globalStore.get(this.selectedBackendAtom);
        if (!backend) {
            this.addErrorMessage("No backend selected");
            return;
        }

        try {
            const response = await spawnAgent({
                pane_id: this.blockId,
                backend_id: backend,
                cwd,
                initial_prompt: initialPrompt,
            });

            globalStore.set(this.instanceIdAtom, response.instance_id);
            globalStore.set(this.statusAtom, "running");

            if (initialPrompt) {
                const msgId = `msg-${Date.now()}-user`;
                const userMsg = createUserMessage(msgId, initialPrompt, BACKEND_TYPE_AGENT);
                globalStore.set(this.messagesAtom, (prev) => [...prev, userMsg]);
            }

            this.setupListeners();
        } catch (err) {
            const errMsg = err instanceof Error ? err.message : String(err);
            globalStore.set(this.statusAtom, "error");
            this.addErrorMessage(errMsg);
        }
    }

    async sendMessage(text: string): Promise<void> {
        const status = globalStore.get(this.statusAtom);

        // If agent isn't running yet, start it with the initial prompt
        if (status === "init" || status === "done" || status === "error") {
            await this.startAgent(undefined, text);
            return;
        }

        // Add user message
        const msgId = `msg-${Date.now()}-user`;
        const userMsg = createUserMessage(msgId, text, BACKEND_TYPE_AGENT);
        globalStore.set(this.messagesAtom, (prev) => [...prev, userMsg]);
        globalStore.set(this.isStreamingAtom, true);

        try {
            await sendAgentText(this.blockId, text);
        } catch (err) {
            globalStore.set(this.isStreamingAtom, false);
            const errMsg = err instanceof Error ? err.message : String(err);
            this.addErrorMessage(errMsg);
        }
    }

    async interrupt(): Promise<void> {
        try {
            await interruptAgent(this.blockId);
        } catch (err) {
            console.error("Failed to interrupt agent:", err);
        }
    }

    async kill(): Promise<void> {
        try {
            await killAgent(this.blockId);
            globalStore.set(this.statusAtom, "done");
            globalStore.set(this.isStreamingAtom, false);
            this.streamingMsg = null;
        } catch (err) {
            console.error("Failed to kill agent:", err);
        }
    }

    resetSession(): void {
        this.cleanupListeners();
        globalStore.set(this.messagesAtom, []);
        globalStore.set(this.instanceIdAtom, "");
        globalStore.set(this.statusAtom, "init");
        globalStore.set(this.isStreamingAtom, false);
        globalStore.set(this.totalUsageAtom, { input_tokens: 0, output_tokens: 0 });
        globalStore.set(this.sessionIdAtom, "");
        globalStore.set(this.totalCostAtom, 0);
        this.streamingMsg = null;
    }

    selectBackend(backendId: string): void {
        globalStore.set(this.selectedBackendAtom, backendId);
    }

    // ---- ViewModel interface ----

    giveFocus(): boolean {
        if (this.inputRef.current) {
            requestAnimationFrame(() => this.inputRef.current?.focus());
            return true;
        }
        return false;
    }

    keyDownHandler(e: WaveKeyboardEvent): boolean {
        // Escape to interrupt
        if (e.key === "Escape") {
            const isStreaming = globalStore.get(this.isStreamingAtom);
            if (isStreaming) {
                this.interrupt();
                return true;
            }
        }
        return false;
    }

    dispose(): void {
        this.cleanupListeners();
        // Kill agent if still running
        const status = globalStore.get(this.statusAtom);
        if (status === "running" || status === "busy" || status === "starting") {
            this.kill().catch(() => {});
        }
    }

    // ---- Internal ----

    private async setupListeners(): Promise<void> {
        this.cleanupListeners();

        const paneId = this.blockId;

        // Listen for adapter events
        const unlistenOutput = await onAgentOutput(paneId, (payload: AgentOutputPayload) => {
            // Handle session-level events first (they don't modify messages)
            for (const event of payload.events) {
                this.handleSessionEvent(event);
            }

            // Then handle message-level events
            globalStore.set(this.messagesAtom, (prev) => {
                const updated = [...prev];
                let currentMsg = this.streamingMsg;

                for (const event of payload.events) {
                    // Skip session-level events (already handled above)
                    if (event.type === "session_start" || event.type === "session_end") {
                        continue;
                    }

                    if (!currentMsg) {
                        const msgId = `msg-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
                        const backend = globalStore.get(this.selectedBackendAtom);
                        currentMsg = createStreamingAssistantMessage(
                            msgId,
                            BACKEND_TYPE_AGENT,
                            backend
                        );
                        updated.push(currentMsg);
                    }

                    currentMsg = applyAdapterEvent(currentMsg, event);
                    updated[updated.length - 1] = currentMsg;

                    if (event.type === "message_end" || event.type === "error") {
                        // Accumulate usage
                        if (event.type === "message_end" && currentMsg.usage) {
                            globalStore.set(this.totalUsageAtom, (prev) => ({
                                input_tokens: prev.input_tokens + (currentMsg!.usage?.input_tokens ?? 0),
                                output_tokens: prev.output_tokens + (currentMsg!.usage?.output_tokens ?? 0),
                                cache_read_tokens:
                                    (prev.cache_read_tokens ?? 0) +
                                    (currentMsg!.usage?.cache_read_tokens ?? 0) || undefined,
                                cache_write_tokens:
                                    (prev.cache_write_tokens ?? 0) +
                                    (currentMsg!.usage?.cache_write_tokens ?? 0) || undefined,
                            }));
                        }
                        this.streamingMsg = null;
                        globalStore.set(this.isStreamingAtom, false);
                        currentMsg = null;
                        continue;
                    }
                }

                if (currentMsg) {
                    this.streamingMsg = currentMsg;
                    globalStore.set(this.isStreamingAtom, true);
                }

                return updated;
            });
        });

        // Listen for status changes
        const unlistenStatus = await onAgentStatus(paneId, (payload) => {
            globalStore.set(this.statusAtom, payload.status.status as AgentStatusType);
            if (payload.status.status === "done" || payload.status.status === "error") {
                globalStore.set(this.isStreamingAtom, false);
                this.streamingMsg = null;
            }
        });

        this.unlistenFns = [unlistenOutput, unlistenStatus];
    }

    private cleanupListeners(): void {
        for (const unlisten of this.unlistenFns) {
            unlisten();
        }
        this.unlistenFns = [];
    }

    private handleSessionEvent(event: AdapterEvent): void {
        if (event.type === "session_start") {
            globalStore.set(this.sessionIdAtom, event.session_id);
            if (event.model) {
                // Could update model display if needed
            }
        } else if (event.type === "session_end") {
            globalStore.set(this.totalCostAtom, event.total_cost_usd);
            // Update total usage from the authoritative result event
            if (event.usage) {
                globalStore.set(this.totalUsageAtom, {
                    input_tokens: event.usage.input_tokens,
                    output_tokens: event.usage.output_tokens,
                    cache_read_tokens: event.usage.cache_read_tokens,
                    cache_write_tokens: event.usage.cache_write_tokens,
                });
            }
        }
    }

    private addErrorMessage(message: string): void {
        const errorMsg = createErrorMessage(
            `msg-${Date.now()}-err`,
            message,
            BACKEND_TYPE_AGENT
        );
        globalStore.set(this.messagesAtom, (prev) => [...prev, errorMsg]);
    }
}
