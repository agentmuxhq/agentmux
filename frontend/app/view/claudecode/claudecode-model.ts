// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { BlockNodeModel } from "@/app/block/blocktypes";
import { RpcApi } from "@/app/store/wshclientapi";
import { TabRpcClient } from "@/app/store/wshrpcutil";
import { getFileSubject, waveEventSubscribe } from "@/app/store/wps";
import { atoms, getApi, globalStore, WOS } from "@/app/store/global";
import * as services from "@/app/store/services";
import { base64ToArray, stringToBase64 } from "@/util/util";
import { atom, Atom, PrimitiveAtom } from "jotai";
import React from "react";
import { Subscription } from "rxjs";
import { TermFileName } from "./claudecode-helpers";
import { ClaudeCodeStreamParser, ParserCallbacks } from "./claudecode-parser";
import type {
    ConversationTurn,
    ResultEvent,
    SessionMeta,
    SystemEvent,
} from "./claudecode-types";
import { ClaudeCodeView } from "./claudecode-view";

export class ClaudeCodeViewModel implements ViewModel {
    viewType = "claudecode";
    blockId: string;
    nodeModel: BlockNodeModel;
    blockAtom: Atom<Block>;

    viewIcon: Atom<string>;
    viewName: Atom<string>;
    viewText: Atom<string | HeaderElem[]>;
    viewComponent: ViewComponent;
    noPadding = atom(true);

    // State
    turnsAtom: PrimitiveAtom<ConversationTurn[]>;
    sessionMetaAtom: PrimitiveAtom<SessionMeta>;
    isStreamingAtom: PrimitiveAtom<boolean>;
    showTerminalAtom: PrimitiveAtom<boolean>;
    connectedAtom: PrimitiveAtom<boolean>;
    errorAtom: PrimitiveAtom<string>;
    authUrlAtom: PrimitiveAtom<string>; // non-empty when auth is needed
    inputRef: React.RefObject<HTMLTextAreaElement>;
    shellProcStatusAtom: Atom<string>; // "init" | "running" | "done"
    shellProcExitCodeAtom: Atom<number>;

    // Internal
    private parser: ClaudeCodeStreamParser;
    private currentTurnId: string | null = null;
    private fileSubjectSub: Subscription | null = null;
    private fileSubjectRef: any = null;
    private procStatusUnsub: (() => void) | null = null;
    private shellProcFullStatusAtom: PrimitiveAtom<BlockControllerRuntimeStatus>;

    constructor(blockId: string, nodeModel: BlockNodeModel) {
        this.blockId = blockId;
        this.nodeModel = nodeModel;
        this.blockAtom = WOS.getWaveObjectAtom<Block>(`block:${blockId}`);
        this.viewComponent = ClaudeCodeView;

        this.viewIcon = atom("terminal");
        this.viewName = atom("Claude Code");

        this.turnsAtom = atom<ConversationTurn[]>([]);
        this.sessionMetaAtom = atom<SessionMeta>({
            model: "",
            inputTokens: 0,
            outputTokens: 0,
            totalCost: 0,
            sessionId: "",
            isStreaming: false,
        });
        this.isStreamingAtom = atom(false);
        this.showTerminalAtom = atom(false);
        this.connectedAtom = atom(false);
        this.errorAtom = atom("");
        this.authUrlAtom = atom("");
        this.inputRef = React.createRef<HTMLTextAreaElement>();
        this.shellProcFullStatusAtom = atom(null) as unknown as PrimitiveAtom<BlockControllerRuntimeStatus>;
        this.shellProcStatusAtom = atom((get) => {
            const fullStatus = get(this.shellProcFullStatusAtom);
            return fullStatus?.shellprocstatus ?? "init";
        });
        this.shellProcExitCodeAtom = atom((get) => {
            const fullStatus = get(this.shellProcFullStatusAtom);
            return fullStatus?.shellprocexitcode ?? 0;
        });

        // Fetch initial controller status
        services.BlockService.GetControllerStatus(blockId).then((rts) => {
            this.updateShellProcStatus(rts);
        });

        // Subscribe to controller status events (process lifecycle)
        this.procStatusUnsub = waveEventSubscribe({
            eventType: "controllerstatus",
            scope: WOS.makeORef("block", blockId),
            handler: (event) => {
                const rts = event.data as BlockControllerRuntimeStatus;
                this.updateShellProcStatus(rts);
            },
        });

        // Header text: show model + tokens + cost
        this.viewText = atom((get) => {
            const meta = get(this.sessionMetaAtom);
            const parts: HeaderElem[] = [];
            if (meta.model) {
                parts.push({ elemtype: "text", text: meta.model });
            }
            const totalTokens = meta.inputTokens + meta.outputTokens;
            if (totalTokens > 0) {
                parts.push({
                    elemtype: "text",
                    text: `${(totalTokens / 1000).toFixed(1)}k`,
                });
            }
            if (meta.totalCost > 0) {
                parts.push({
                    elemtype: "text",
                    text: `$${meta.totalCost.toFixed(3)}`,
                });
            }
            return parts;
        });

        this.parser = new ClaudeCodeStreamParser(this.buildParserCallbacks());
        this.connectToTerminal();
    }

    // --- Parser callbacks ---

    private buildParserCallbacks(): ParserCallbacks {
        return {
            onRawLine: (line: string) => {
                // Detect auth URLs in pre-protocol plain text output
                const urlMatch = line.match(
                    /https:\/\/(?:console\.anthropic\.com|auth\.anthropic\.com)[^\s)>\]"]*/
                );
                if (urlMatch) {
                    globalStore.set(this.authUrlAtom, urlMatch[0]);
                    this.openAuthUrl(urlMatch[0]);
                }
            },

            onSystemEvent: (event: SystemEvent) => {
                const meta = globalStore.get(this.sessionMetaAtom);
                const updates: Partial<SessionMeta> = {};
                if (event.model) updates.model = event.model;
                if (event.session_id) {
                    updates.sessionId = event.session_id;
                    // Auth succeeded — clear auth banner
                    globalStore.set(this.authUrlAtom, "");
                }
                if (Object.keys(updates).length > 0) {
                    globalStore.set(this.sessionMetaAtom, { ...meta, ...updates });
                }
            },

            onMessageStart: (_role: string, model?: string, usage?: any) => {
                globalStore.set(this.isStreamingAtom, true);
                globalStore.set(this.errorAtom, "");
                globalStore.set(this.authUrlAtom, "");
                if (model || usage) {
                    const meta = globalStore.get(this.sessionMetaAtom);
                    globalStore.set(this.sessionMetaAtom, {
                        ...meta,
                        model: model ?? meta.model,
                        inputTokens: usage?.input_tokens ?? meta.inputTokens,
                    });
                }
                if (!this.currentTurnId) {
                    const turnId = crypto.randomUUID();
                    this.currentTurnId = turnId;
                    const turns = globalStore.get(this.turnsAtom);
                    globalStore.set(this.turnsAtom, [
                        ...turns,
                        {
                            id: turnId,
                            userInput: "(continued)",
                            blocks: [],
                            timestamp: Date.now(),
                        },
                    ]);
                }
            },

            onTextDelta: (text: string) => {
                if (!this.currentTurnId) return;
                const turns = globalStore.get(this.turnsAtom);
                const updated = turns.map((t) => {
                    if (t.id !== this.currentTurnId) return t;
                    const blocks = [...t.blocks];
                    const last = blocks[blocks.length - 1];
                    if (last && last.type === "text") {
                        blocks[blocks.length - 1] = {
                            ...last,
                            text: last.text + text,
                        };
                    } else {
                        blocks.push({ type: "text", text });
                    }
                    return { ...t, blocks };
                });
                globalStore.set(this.turnsAtom, updated);
            },

            onToolUseStart: (id: string, name: string) => {
                if (!this.currentTurnId) return;
                const turns = globalStore.get(this.turnsAtom);
                const updated = turns.map((t) => {
                    if (t.id !== this.currentTurnId) return t;
                    return {
                        ...t,
                        blocks: [
                            ...t.blocks,
                            {
                                type: "tool" as const,
                                toolId: id,
                                name,
                                input: {},
                                isCollapsed: true,
                            },
                        ],
                    };
                });
                globalStore.set(this.turnsAtom, updated);
            },

            onToolUseFinish: (id: string, parsedInput: Record<string, any>) => {
                if (!this.currentTurnId) return;
                const turns = globalStore.get(this.turnsAtom);
                const updated = turns.map((t) => {
                    if (t.id !== this.currentTurnId) return t;
                    return {
                        ...t,
                        blocks: t.blocks.map((b) => {
                            if (b.type === "tool" && b.toolId === id) {
                                return { ...b, input: parsedInput };
                            }
                            return b;
                        }),
                    };
                });
                globalStore.set(this.turnsAtom, updated);
            },

            onToolResult: (toolUseId: string, content: string, isError?: boolean) => {
                const turns = globalStore.get(this.turnsAtom);
                const updated = turns.map((t) => ({
                    ...t,
                    blocks: t.blocks.map((b) => {
                        if (b.type === "tool" && b.toolId === toolUseId) {
                            return { ...b, result: content, isError: isError ?? false };
                        }
                        return b;
                    }),
                }));
                globalStore.set(this.turnsAtom, updated);
            },

            onMessageStop: () => {},

            onUsageUpdate: (usage: any) => {
                if (!usage) return;
                const meta = globalStore.get(this.sessionMetaAtom);
                globalStore.set(this.sessionMetaAtom, {
                    ...meta,
                    outputTokens: usage.output_tokens ?? meta.outputTokens,
                });
            },

            onResultEvent: (event: ResultEvent) => {
                globalStore.set(this.isStreamingAtom, false);
                this.currentTurnId = null;
                const meta = globalStore.get(this.sessionMetaAtom);
                globalStore.set(this.sessionMetaAtom, {
                    ...meta,
                    totalCost: event.cost_usd ?? meta.totalCost,
                    sessionId: event.session_id ?? meta.sessionId,
                });
                if (event.is_error && event.result) {
                    globalStore.set(this.errorAtom, event.result);
                }
            },

            onError: (errorType: string, message: string) => {
                globalStore.set(this.errorAtom, `${errorType}: ${message}`);
            },
        };
    }

    // --- Terminal connection ---

    private connectToTerminal(): void {
        try {
            this.fileSubjectRef = getFileSubject(this.blockId, TermFileName);
            this.fileSubjectSub = this.fileSubjectRef.subscribe((msg: any) => {
                this.handleTerminalData(msg);
            });
            globalStore.set(this.connectedAtom, true);
        } catch (e) {
            console.error("[claudecode] Failed to connect to terminal file subject:", e);
            globalStore.set(this.connectedAtom, false);
        }
    }

    private handleTerminalData(msg: any): void {
        if (msg.fileop === "truncate") {
            this.parser.reset();
            return;
        }
        if (msg.fileop === "append" && msg.data64) {
            const bytes = base64ToArray(msg.data64);
            const text = new TextDecoder().decode(bytes);
            this.parser.feedData(text);
        }
    }

    // --- User actions ---

    sendMessage(text: string): void {
        if (!text.trim()) return;

        const turnId = crypto.randomUUID();
        this.currentTurnId = turnId;

        const newTurn: ConversationTurn = {
            id: turnId,
            userInput: text,
            blocks: [],
            timestamp: Date.now(),
        };

        const turns = globalStore.get(this.turnsAtom);
        globalStore.set(this.turnsAtom, [...turns, newTurn]);
        globalStore.set(this.isStreamingAtom, true);

        const b64data = stringToBase64(text + "\n");
        RpcApi.ControllerInputCommand(TabRpcClient, {
            blockid: this.blockId,
            inputdata64: b64data,
        }).catch((e: any) => {
            console.error("[claudecode] Failed to send input:", e);
            globalStore.set(this.isStreamingAtom, false);
        });
    }

    openAuthUrl(url?: string): void {
        const authUrl = url ?? globalStore.get(this.authUrlAtom);
        if (!authUrl) return;
        // Only open known Anthropic domains
        if (
            authUrl.startsWith("https://console.anthropic.com") ||
            authUrl.startsWith("https://auth.anthropic.com")
        ) {
            getApi().openExternal(authUrl);
        }
    }

    toggleTerminal(): void {
        const current = globalStore.get(this.showTerminalAtom);
        globalStore.set(this.showTerminalAtom, !current);
    }

    interrupt(): void {
        RpcApi.ControllerInputCommand(TabRpcClient, {
            blockid: this.blockId,
            signame: "SIGINT",
        }).catch((e: any) => {
            console.error("[claudecode] Failed to send SIGINT:", e);
        });
        globalStore.set(this.isStreamingAtom, false);
    }

    resetSession(): void {
        globalStore.set(this.turnsAtom, []);
        globalStore.set(this.isStreamingAtom, false);
        globalStore.set(this.sessionMetaAtom, {
            model: "",
            inputTokens: 0,
            outputTokens: 0,
            totalCost: 0,
            sessionId: "",
            isStreaming: false,
        });
        this.currentTurnId = null;
        this.parser.reset();

        const b64data = stringToBase64("/clear\n");
        RpcApi.ControllerInputCommand(TabRpcClient, {
            blockid: this.blockId,
            inputdata64: b64data,
        }).catch((e: any) => {
            console.error("[claudecode] Failed to send /clear:", e);
        });
    }

    private updateShellProcStatus(rts: BlockControllerRuntimeStatus): void {
        if (rts == null) return;
        const cur = globalStore.get(this.shellProcFullStatusAtom);
        if (cur == null || cur.version < rts.version) {
            globalStore.set(this.shellProcFullStatusAtom, rts);
            if (rts.shellprocstatus === "running") {
                globalStore.set(this.connectedAtom, true);
            }
        }
    }

    restartProcess(): void {
        globalStore.set(this.shellProcFullStatusAtom, null as any);
        RpcApi.ControllerResyncCommand(TabRpcClient, {
            tabid: globalStore.get(atoms.staticTabId),
            blockid: this.blockId,
            forcerestart: true,
        }).catch((e: any) => {
            console.error("[claudecode] Failed to restart process:", e);
            globalStore.set(this.errorAtom, "Failed to restart claude process");
        });
    }

    toggleToolCollapse(turnId: string, toolId: string): void {
        const turns = globalStore.get(this.turnsAtom);
        const updated = turns.map((t) => {
            if (t.id !== turnId) return t;
            return {
                ...t,
                blocks: t.blocks.map((b) => {
                    if (b.type === "tool" && b.toolId === toolId) {
                        return { ...b, isCollapsed: !b.isCollapsed };
                    }
                    return b;
                }),
            };
        });
        globalStore.set(this.turnsAtom, updated);
    }

    // Defer focus to avoid infinite render loop in BlockFull's layout effect chain.
    // BlockFull calls giveFocus() synchronously from useLayoutEffect; a synchronous
    // .focus() call triggers onFocusCapture before the new handler is committed,
    // creating a setState cycle.
    giveFocus(): boolean {
        if (this.inputRef.current) {
            requestAnimationFrame(() => this.inputRef.current?.focus());
            return true;
        }
        return false;
    }

    dispose(): void {
        if (this.fileSubjectSub) {
            this.fileSubjectSub.unsubscribe();
            this.fileSubjectSub = null;
        }
        if (this.fileSubjectRef) {
            this.fileSubjectRef.release();
            this.fileSubjectRef = null;
        }
        if (this.procStatusUnsub) {
            this.procStatusUnsub();
            this.procStatusUnsub = null;
        }
        this.parser.reset();
    }
}
