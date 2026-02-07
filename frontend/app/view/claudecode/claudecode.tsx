// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { BlockNodeModel } from "@/app/block/blocktypes";
import { Markdown } from "@/app/element/markdown";
import { RpcApi } from "@/app/store/wshclientapi";
import { TabRpcClient } from "@/app/store/wshrpcutil";
import { getFileSubject, waveEventSubscribe } from "@/app/store/wps";
import { atoms, globalStore, WOS } from "@/store/global";
import * as services from "@/store/services";
import { base64ToArray, stringToBase64 } from "@/util/util";
import { atom, Atom, PrimitiveAtom, useAtomValue } from "jotai";
import clsx from "clsx";
import { memo, useCallback, useEffect, useRef, useState } from "react";
import { Subscription } from "rxjs";
import { ClaudeCodeStreamParser, ParserCallbacks } from "./claudecode-parser";
import type {
    ConversationTurn,
    ResultEvent,
    SessionMeta,
    SystemEvent,
    TextTurnBlock,
    ToolTurnBlock,
} from "./claudecode-types";
import "./claudecode.scss";

const TermFileName = "term";
const TOOL_RESULT_MAX_LENGTH = 10 * 1024; // 10KB max for tool result display

// ============================================================
// ViewModel
// ============================================================

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

    // Internal
    private parser: ClaudeCodeStreamParser;
    private currentTurnId: string | null = null;
    private fileSubjectSub: Subscription | null = null;
    private fileSubjectRef: any = null;
    private procStatusUnsub: (() => void) | null = null;
    errorAtom: PrimitiveAtom<string>;
    private shellProcFullStatusAtom: PrimitiveAtom<BlockControllerRuntimeStatus>;
    shellProcStatusAtom: Atom<string>; // "init" | "running" | "done"
    shellProcExitCodeAtom: Atom<number>;

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

    // --- Parser callbacks (handle all stream-json event types) ---

    private buildParserCallbacks(): ParserCallbacks {
        return {
            onSystemEvent: (event: SystemEvent) => {
                const meta = globalStore.get(this.sessionMetaAtom);
                const updates: Partial<SessionMeta> = {};
                if (event.model) updates.model = event.model;
                if (event.session_id) updates.sessionId = event.session_id;
                if (Object.keys(updates).length > 0) {
                    globalStore.set(this.sessionMetaAtom, { ...meta, ...updates });
                }
            },

            onMessageStart: (_role: string, model?: string, usage?: any) => {
                globalStore.set(this.isStreamingAtom, true);
                globalStore.set(this.errorAtom, "");
                if (model || usage) {
                    const meta = globalStore.get(this.sessionMetaAtom);
                    globalStore.set(this.sessionMetaAtom, {
                        ...meta,
                        model: model ?? meta.model,
                        inputTokens: usage?.input_tokens ?? meta.inputTokens,
                    });
                }
                // Ensure we have an active turn
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
                        // Append to existing text block
                        blocks[blocks.length - 1] = {
                            ...last,
                            text: last.text + text,
                        };
                    } else {
                        // Start new text block
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
                // Find the tool block across all turns and attach the result
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

            onMessageStop: () => {
                // Message complete, but session may continue (multi-turn)
            },

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

        // Send input to the claude process via controller
        const b64data = stringToBase64(text + "\n");
        RpcApi.ControllerInputCommand(TabRpcClient, {
            blockid: this.blockId,
            inputdata64: b64data,
        }).catch((e: any) => {
            console.error("[claudecode] Failed to send input:", e);
            globalStore.set(this.isStreamingAtom, false);
        });
    }

    toggleTerminal(): void {
        const current = globalStore.get(this.showTerminalAtom);
        globalStore.set(this.showTerminalAtom, !current);
    }

    interrupt(): void {
        // Send SIGINT to the claude process
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

        // Send /clear to reset Claude Code session, then Ctrl+C + restart
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

    giveFocus(): boolean {
        return true;
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

// ============================================================
// React Components
// ============================================================

const ClaudeCodeView: React.FC<ViewComponentProps<ClaudeCodeViewModel>> = memo(
    ({ contentRef, model }) => {
        const turns = useAtomValue(model.turnsAtom);
        const isStreaming = useAtomValue(model.isStreamingAtom);
        const showTerminal = useAtomValue(model.showTerminalAtom);
        const connected = useAtomValue(model.connectedAtom);
        const error = useAtomValue(model.errorAtom);

        return (
            <div ref={contentRef} className="claudecode-view">
                {!showTerminal && (
                    <div className="cc-log-container">
                        <ConversationLog
                            turns={turns}
                            model={model}
                            connected={connected}
                        />
                        {error && <ErrorBanner message={error} />}
                        {isStreaming && <StreamingCursor />}
                        <InputLine model={model} />
                    </div>
                )}
                {showTerminal && (
                    <div className="cc-raw-terminal">
                        <RawTerminalView blockId={model.blockId} />
                    </div>
                )}
                <StatusBar model={model} />
            </div>
        );
    }
);

// --- Raw Terminal View (for [term] toggle) ---

const RAW_OUTPUT_MAX_LENGTH = 256 * 1024; // 256KB max for raw terminal buffer

const RawTerminalView = memo(({ blockId }: { blockId: string }) => {
    const scrollRef = useRef<HTMLDivElement>(null);
    const [rawOutput, setRawOutput] = useState("");

    useEffect(() => {
        const fileSubject = getFileSubject(blockId, TermFileName);
        const sub = fileSubject.subscribe((msg: any) => {
            if (msg.fileop === "append" && msg.data64) {
                const bytes = base64ToArray(msg.data64);
                const text = new TextDecoder().decode(bytes);
                setRawOutput((prev) => {
                    const combined = prev + text;
                    if (combined.length > RAW_OUTPUT_MAX_LENGTH) {
                        return combined.slice(-RAW_OUTPUT_MAX_LENGTH);
                    }
                    return combined;
                });
            } else if (msg.fileop === "truncate") {
                setRawOutput("");
            }
        });

        return () => {
            sub.unsubscribe();
            fileSubject.release();
        };
    }, [blockId]);

    useEffect(() => {
        if (scrollRef.current) {
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
    }, [rawOutput]);

    return (
        <div className="cc-raw-output" ref={scrollRef}>
            <pre>{rawOutput}</pre>
        </div>
    );
});

// --- Conversation Log ---

const ConversationLog = memo(
    ({
        turns,
        model,
        connected,
    }: {
        turns: ConversationTurn[];
        model: ClaudeCodeViewModel;
        connected: boolean;
    }) => {
        const scrollRef = useRef<HTMLDivElement>(null);

        useEffect(() => {
            if (scrollRef.current) {
                scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
            }
        }, [turns]);

        return (
            <div className="cc-log" ref={scrollRef}>
                {turns.length === 0 && (
                    <div className="cc-empty">
                        <div className="cc-empty-header">
                            <span className="cc-empty-prompt">{"\u276f"}</span> Claude Code
                        </div>
                        {!connected && (
                            <div className="cc-empty-status">
                                Waiting for claude process...
                            </div>
                        )}
                        {connected && (
                            <div className="cc-empty-status">
                                Ready. Type a message below.
                            </div>
                        )}
                    </div>
                )}
                {turns.map((turn) => (
                    <TurnView key={turn.id} turn={turn} model={model} />
                ))}
            </div>
        );
    }
);

// --- Conversation Turn ---

const TurnView = memo(({ turn, model }: { turn: ConversationTurn; model: ClaudeCodeViewModel }) => {
    return (
        <div className="cc-turn">
            <div className="cc-prompt">
                <span className="cc-prompt-char">{"\u276f"}</span>
                <span className="cc-prompt-text">{turn.userInput}</span>
            </div>
            {turn.blocks.map((block, i) => {
                if (block.type === "text") {
                    return <TextBlockView key={i} block={block} />;
                }
                if (block.type === "tool") {
                    return (
                        <ToolBlockView
                            key={block.toolId}
                            block={block}
                            turnId={turn.id}
                            model={model}
                        />
                    );
                }
                return null;
            })}
            <div className="cc-turn-separator" />
        </div>
    );
});

// --- Text Block ---

const TextBlockView = memo(({ block }: { block: TextTurnBlock }) => {
    return (
        <div className="cc-text">
            <Markdown text={block.text} />
        </div>
    );
});

// --- Tool Block ---

const ToolBlockView = memo(
    ({
        block,
        turnId,
        model,
    }: {
        block: ToolTurnBlock;
        turnId: string;
        model: ClaudeCodeViewModel;
    }) => {
        const summary = getToolOneLiner(block.name, block.input);
        const [showFull, setShowFull] = useState(false);
        const handleClick = useCallback(() => {
            model.toggleToolCollapse(turnId, block.toolId);
        }, [model, turnId, block.toolId]);

        const isTruncated = block.result != null && block.result.length > TOOL_RESULT_MAX_LENGTH;
        const displayResult = isTruncated && !showFull
            ? block.result!.slice(0, TOOL_RESULT_MAX_LENGTH)
            : block.result;

        return (
            <div className={clsx("cc-tool", { expanded: !block.isCollapsed })} onClick={handleClick}>
                <div className="cc-tool-line">
                    <span className="cc-tool-chevron">{block.isCollapsed ? "\u25b8" : "\u25be"}</span>
                    <span className="cc-tool-name">{block.name}</span>
                    <span className="cc-tool-summary">{summary}</span>
                    {block.isError && <span className="cc-tool-error">{"\u2717"}</span>}
                    {!block.isError && block.result != null && (
                        <span className="cc-tool-ok">{"\u2713"}</span>
                    )}
                </div>
                {!block.isCollapsed && (
                    <div className="cc-tool-detail" onClick={(e) => e.stopPropagation()}>
                        {block.name === "Edit" && displayResult ? (
                            <DiffBlock input={block.input} result={displayResult} />
                        ) : block.name === "Bash" ? (
                            <BashBlock input={block.input} result={displayResult} />
                        ) : (
                            <pre className="cc-tool-output">
                                {displayResult ?? JSON.stringify(block.input, null, 2)}
                            </pre>
                        )}
                        {isTruncated && !showFull && (
                            <button
                                className="cc-tool-show-more"
                                onClick={(e) => { e.stopPropagation(); setShowFull(true); }}
                            >
                                Show full output ({(block.result!.length / 1024).toFixed(0)}KB)
                            </button>
                        )}
                    </div>
                )}
            </div>
        );
    }
);

// --- Diff Block ---

const DiffBlock = memo(({ input, result }: { input: any; result: string }) => {
    const lines = result.split("\n");
    return (
        <pre className="cc-diff">
            <div className="cc-diff-header">{input.file_path}</div>
            {lines.map((line, i) => {
                const cls = line.startsWith("+")
                    ? "cc-diff-add"
                    : line.startsWith("-")
                      ? "cc-diff-del"
                      : line.startsWith("@")
                        ? "cc-diff-hunk"
                        : "cc-diff-ctx";
                return (
                    <div key={i} className={cls}>
                        {line}
                    </div>
                );
            })}
        </pre>
    );
});

// --- Bash Block ---

const BashBlock = memo(({ input, result }: { input: any; result?: string }) => {
    return (
        <div className="cc-bash">
            <div className="cc-bash-cmd">
                <span className="cc-bash-dollar">$</span> {input.command}
            </div>
            {result && <pre className="cc-bash-output">{result}</pre>}
        </div>
    );
});

// --- Streaming Cursor ---

const StreamingCursor = memo(() => {
    return (
        <div className="cc-streaming">
            <span className="cc-streaming-cursor">{"\u2588"}</span>
        </div>
    );
});

// --- Input Line ---

const InputLine = memo(({ model }: { model: ClaudeCodeViewModel }) => {
    const [text, setText] = useState("");
    const isStreaming = useAtomValue(model.isStreamingAtom);
    const textareaRef = useRef<HTMLTextAreaElement>(null);

    const handleSend = useCallback(() => {
        if (!text.trim() || isStreaming) return;
        model.sendMessage(text);
        setText("");
    }, [text, isStreaming, model]);

    const handleKeyDown = useCallback(
        (e: React.KeyboardEvent) => {
            if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleSend();
            }
            if (e.key === "Escape") {
                model.interrupt();
            }
            if (e.key === "l" && (e.ctrlKey || e.metaKey)) {
                e.preventDefault();
                model.resetSession();
            }
        },
        [handleSend, model]
    );

    // Auto-focus input when not streaming
    useEffect(() => {
        if (!isStreaming && textareaRef.current) {
            textareaRef.current.focus();
        }
    }, [isStreaming]);

    // Auto-expand textarea
    useEffect(() => {
        const el = textareaRef.current;
        if (el) {
            el.style.height = "auto";
            el.style.height = el.scrollHeight + "px";
        }
    }, [text]);

    return (
        <div className="cc-input">
            <span className="cc-input-prompt">{"\u276f"}</span>
            <textarea
                ref={textareaRef}
                className="cc-input-textarea"
                value={text}
                onChange={(e) => setText(e.target.value)}
                onKeyDown={handleKeyDown}
                disabled={isStreaming}
                placeholder="Ask Claude..."
                rows={1}
                spellCheck={false}
            />
        </div>
    );
});

// --- Status Bar ---

const StatusBar = memo(({ model }: { model: ClaudeCodeViewModel }) => {
    const meta = useAtomValue(model.sessionMetaAtom);
    const isStreaming = useAtomValue(model.isStreamingAtom);
    const showTerminal = useAtomValue(model.showTerminalAtom);
    const procStatus = useAtomValue(model.shellProcStatusAtom);
    const exitCode = useAtomValue(model.shellProcExitCodeAtom);

    const statusText = procStatus === "done"
        ? `\u25cf exited (${exitCode})`
        : isStreaming
            ? "\u25cf streaming"
            : procStatus === "running"
                ? "\u25cb idle"
                : "\u25cb init";

    return (
        <div className="cc-statusbar">
            <span className={clsx("cc-status-item", { "cc-status-exited": procStatus === "done" })}>
                {statusText}
            </span>
            {meta.model && <span className="cc-status-item">{meta.model}</span>}
            {meta.inputTokens + meta.outputTokens > 0 && (
                <span className="cc-status-item">
                    {((meta.inputTokens + meta.outputTokens) / 1000).toFixed(1)}k
                </span>
            )}
            {meta.totalCost > 0 && (
                <span className="cc-status-item">${meta.totalCost.toFixed(3)}</span>
            )}
            <span className="cc-status-spacer" />
            {procStatus === "done" && (
                <button className="cc-status-btn cc-restart-btn" onClick={() => model.restartProcess()}>
                    [restart]
                </button>
            )}
            <button className="cc-status-btn" onClick={() => model.toggleTerminal()}>
                {showTerminal ? "[chat]" : "[term]"}
            </button>
            <button
                className="cc-status-btn"
                onClick={() => model.interrupt()}
                disabled={!isStreaming}
            >
                [^C]
            </button>
            <button className="cc-status-btn" onClick={() => model.resetSession()}>
                [reset]
            </button>
        </div>
    );
});

// --- Error Banner ---

const ErrorBanner = memo(({ message }: { message: string }) => {
    return (
        <div className="cc-error">
            <span className="cc-error-icon">{"\u26a0"}</span>
            <span className="cc-error-text">{message}</span>
        </div>
    );
});

// --- Helpers ---

function getToolOneLiner(name: string, input: any): string {
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

export { ClaudeCodeView };
