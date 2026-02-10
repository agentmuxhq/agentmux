// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Jotai atoms and React hook for unified AI pane state management.
 *
 * Manages the full lifecycle of an agent backend session:
 * - Available backends (auto-detected from PATH)
 * - Active agent status per pane
 * - Conversation state (messages, streaming)
 * - Event listener setup/teardown
 *
 * Usage:
 *   const { messages, status, sendMessage, interrupt, kill } = useUnifiedAI(paneId);
 */

import { atom, useAtom, useAtomValue, useSetAtom } from "jotai";
import { atomFamily } from "jotai/utils";
import { useCallback, useEffect, useRef } from "react";

import type { UnlistenFn } from "@tauri-apps/api/event";

import {
    type AgentBackendConfig,
    type AgentStatusType,
    type UnifiedMessage,
    type AdapterEvent,
    applyAdapterEvent,
    createUserMessage,
    createStreamingAssistantMessage,
    createErrorMessage,
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

// ---- Atoms ----

/** Available agent backends (populated on first load). */
export const availableBackendsAtom = atom<AgentBackendConfig[]>([]);

/** Whether backends have been loaded. */
export const backendsLoadedAtom = atom(false);

/** Currently selected backend ID per pane. */
export const selectedBackendAtom = atomFamily((_paneId: string) => atom<string>(""));

/** Agent status per pane. */
export const agentStatusAtom = atomFamily((_paneId: string) =>
    atom<AgentStatusType>("init")
);

/** Agent instance ID per pane. */
export const agentInstanceIdAtom = atomFamily((_paneId: string) => atom<string>(""));

/** Conversation messages per pane. */
export const messagesAtom = atomFamily((_paneId: string) =>
    atom<UnifiedMessage[]>([])
);

/** Whether the agent is currently streaming a response. */
export const isStreamingAtom = atomFamily((_paneId: string) => atom(false));

// ---- Hook ----

/**
 * React hook for managing a unified AI pane session.
 *
 * Handles agent lifecycle, message state, and event listeners.
 */
export function useUnifiedAI(paneId: string) {
    const [messages, setMessages] = useAtom(messagesAtom(paneId));
    const [status, setStatus] = useAtom(agentStatusAtom(paneId));
    const [instanceId, setInstanceId] = useAtom(agentInstanceIdAtom(paneId));
    const [selectedBackend, setSelectedBackend] = useAtom(selectedBackendAtom(paneId));
    const [isStreaming, setIsStreaming] = useAtom(isStreamingAtom(paneId));
    const [availableBackends, setAvailableBackends] = useAtom(availableBackendsAtom);
    const [backendsLoaded, setBackendsLoaded] = useAtom(backendsLoadedAtom);

    const unlistenRefs = useRef<UnlistenFn[]>([]);
    const streamingMsgRef = useRef<UnifiedMessage | null>(null);
    const selectedBackendRef = useRef(selectedBackend);

    // Keep ref in sync with atom value
    useEffect(() => {
        selectedBackendRef.current = selectedBackend;
    }, [selectedBackend]);

    // Load available backends on first mount
    useEffect(() => {
        if (!backendsLoaded) {
            listAgentBackends()
                .then((backends) => {
                    setAvailableBackends(backends);
                    setBackendsLoaded(true);
                    // Auto-select first backend if none selected
                    if (!selectedBackend && backends.length > 0) {
                        setSelectedBackend(backends[0].id);
                    }
                })
                .catch((err) => {
                    console.error("Failed to list agent backends:", err);
                    setBackendsLoaded(true);
                });
        }
    }, [backendsLoaded]);

    // Set up event listeners when agent is running
    useEffect(() => {
        if (!instanceId || status === "init" || status === "done" || status === "error") {
            return;
        }

        const setupListeners = async () => {
            // Listen for adapter events (streaming response)
            const unlistenOutput = await onAgentOutput(paneId, (payload: AgentOutputPayload) => {
                setMessages((prev) => {
                    const updated = [...prev];
                    let currentMsg = streamingMsgRef.current;

                    for (const event of payload.events) {
                        // If we don't have a streaming message yet, create one
                        if (!currentMsg) {
                            const msgId = `msg-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
                            currentMsg = createStreamingAssistantMessage(
                                msgId,
                                BACKEND_TYPE_AGENT,
                                selectedBackendRef.current
                            );
                            updated.push(currentMsg);
                        }

                        // Apply the event to update the message
                        currentMsg = applyAdapterEvent(currentMsg, event);

                        // Update the last message in the array
                        updated[updated.length - 1] = currentMsg;

                        // Check if message is complete
                        if (event.type === "message_end" || event.type === "error") {
                            streamingMsgRef.current = null;
                            setIsStreaming(false);
                            // Next event batch will create a new message
                            currentMsg = null;
                            continue;
                        }
                    }

                    if (currentMsg) {
                        streamingMsgRef.current = currentMsg;
                        setIsStreaming(true);
                    }

                    return updated;
                });
            });

            // Listen for status changes
            const unlistenStatus = await onAgentStatus(paneId, (payload) => {
                setStatus(payload.status.status as AgentStatusType);
                if (payload.status.status === "done" || payload.status.status === "error") {
                    setIsStreaming(false);
                    streamingMsgRef.current = null;
                }
            });

            unlistenRefs.current = [unlistenOutput, unlistenStatus];
        };

        setupListeners();

        return () => {
            for (const unlisten of unlistenRefs.current) {
                unlisten();
            }
            unlistenRefs.current = [];
        };
    }, [paneId, instanceId, status]);

    // ---- Actions ----

    /** Start an agent session with the selected backend. */
    const startAgent = useCallback(
        async (cwd?: string, initialPrompt?: string) => {
            if (!selectedBackend) {
                throw new Error("No backend selected");
            }

            try {
                const response = await spawnAgent({
                    pane_id: paneId,
                    backend_id: selectedBackend,
                    cwd,
                    initial_prompt: initialPrompt,
                });

                setInstanceId(response.instance_id);
                setStatus("running");

                if (initialPrompt) {
                    const msgId = `msg-${Date.now()}-user`;
                    const userMsg = createUserMessage(msgId, initialPrompt, BACKEND_TYPE_AGENT);
                    setMessages((prev) => [...prev, userMsg]);
                }
            } catch (err) {
                const errMsg = err instanceof Error ? err.message : String(err);
                setStatus("error");
                const errorMsg = createErrorMessage(
                    `msg-${Date.now()}-err`,
                    errMsg,
                    BACKEND_TYPE_AGENT
                );
                setMessages((prev) => [...prev, errorMsg]);
                throw err;
            }
        },
        [paneId, selectedBackend]
    );

    /** Send a message to the running agent. */
    const sendMessage = useCallback(
        async (text: string) => {
            // Add user message to conversation
            const msgId = `msg-${Date.now()}-user`;
            const userMsg = createUserMessage(msgId, text, BACKEND_TYPE_AGENT);
            setMessages((prev) => [...prev, userMsg]);
            setIsStreaming(true);

            try {
                await sendAgentText(paneId, text);
            } catch (err) {
                setIsStreaming(false);
                const errMsg = err instanceof Error ? err.message : String(err);
                const errorMsg = createErrorMessage(
                    `msg-${Date.now()}-err`,
                    errMsg,
                    BACKEND_TYPE_AGENT
                );
                setMessages((prev) => [...prev, errorMsg]);
            }
        },
        [paneId]
    );

    /** Interrupt the running agent (SIGINT). */
    const interrupt = useCallback(async () => {
        try {
            await interruptAgent(paneId);
        } catch (err) {
            console.error("Failed to interrupt agent:", err);
        }
    }, [paneId]);

    /** Kill the running agent. */
    const kill = useCallback(async () => {
        try {
            await killAgent(paneId);
            setStatus("done");
            setIsStreaming(false);
            streamingMsgRef.current = null;
        } catch (err) {
            console.error("Failed to kill agent:", err);
        }
    }, [paneId]);

    /** Clear conversation and reset state. */
    const reset = useCallback(() => {
        setMessages([]);
        setInstanceId("");
        setStatus("init");
        setIsStreaming(false);
        streamingMsgRef.current = null;
    }, []);

    return {
        // State
        messages,
        status,
        instanceId,
        isStreaming,
        selectedBackend,
        availableBackends,

        // Setters
        setSelectedBackend,

        // Actions
        startAgent,
        sendMessage,
        interrupt,
        kill,
        reset,
    };
}
