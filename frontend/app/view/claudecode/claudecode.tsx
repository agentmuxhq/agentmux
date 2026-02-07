// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { BlockNodeModel } from "@/app/block/blocktypes";
import { Markdown } from "@/app/element/markdown";
import { atoms, globalStore, WOS } from "@/store/global";
import { atom, Atom, PrimitiveAtom, useAtomValue } from "jotai";
import clsx from "clsx";
import { memo, useCallback, useEffect, useRef, useState } from "react";
import { ClaudeCodeStreamParser } from "./claudecode-parser";
import type {
    ClaudeCodeEvent,
    ConversationTurn,
    SessionMeta,
    TextTurnBlock,
    ToolTurnBlock,
    TurnBlock,
} from "./claudecode-types";
import "./claudecode.scss";

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

    // Internal
    private parser: ClaudeCodeStreamParser;
    private currentTurnId: string | null = null;

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

        this.parser = new ClaudeCodeStreamParser((event) => this.handleEvent(event));
    }

    // --- Event handling ---

    handleEvent(event: ClaudeCodeEvent): void {
        switch (event.type) {
            case "system":
                this.handleSystemEvent(event);
                break;
            case "assistant":
                this.handleAssistantEvent(event);
                break;
            case "result":
                this.handleResultEvent(event);
                break;
        }
    }

    private handleSystemEvent(event: any): void {
        if (event.model) {
            const meta = globalStore.get(this.sessionMetaAtom);
            globalStore.set(this.sessionMetaAtom, { ...meta, model: event.model });
        }
        if (event.session_id) {
            const meta = globalStore.get(this.sessionMetaAtom);
            globalStore.set(this.sessionMetaAtom, { ...meta, sessionId: event.session_id });
        }
    }

    private handleAssistantEvent(event: any): void {
        globalStore.set(this.isStreamingAtom, true);
        const msg = event.message;
        if (!msg?.content) return;

        // Build blocks from content
        const newBlocks: TurnBlock[] = [];
        for (const block of msg.content) {
            if (block.type === "text" && block.text) {
                newBlocks.push({ type: "text", text: block.text });
            } else if (block.type === "tool_use") {
                newBlocks.push({
                    type: "tool",
                    toolId: block.id,
                    name: block.name,
                    input: block.input,
                    isCollapsed: true,
                });
            }
        }

        if (newBlocks.length === 0) return;

        const turns = globalStore.get(this.turnsAtom);
        if (this.currentTurnId) {
            // Append to current turn
            const updated = turns.map((t) => {
                if (t.id === this.currentTurnId) {
                    return { ...t, blocks: [...t.blocks, ...newBlocks] };
                }
                return t;
            });
            globalStore.set(this.turnsAtom, updated);
        }

        // Update token counts from usage
        if (msg.usage) {
            const meta = globalStore.get(this.sessionMetaAtom);
            globalStore.set(this.sessionMetaAtom, {
                ...meta,
                inputTokens: meta.inputTokens + (msg.usage.input_tokens ?? 0),
                outputTokens: meta.outputTokens + (msg.usage.output_tokens ?? 0),
                model: msg.model ?? meta.model,
            });
        }
    }

    private handleResultEvent(event: any): void {
        globalStore.set(this.isStreamingAtom, false);
        if (event.total_cost != null || event.usage != null) {
            const meta = globalStore.get(this.sessionMetaAtom);
            globalStore.set(this.sessionMetaAtom, {
                ...meta,
                totalCost: event.total_cost ?? meta.totalCost,
                inputTokens: event.usage?.input_tokens ?? meta.inputTokens,
                outputTokens: event.usage?.output_tokens ?? meta.outputTokens,
            });
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

        // TODO: Phase 2 - send to hidden terminal via ControllerInputCommand
        // For now, simulate with parser feed for testing
    }

    toggleTerminal(): void {
        const current = globalStore.get(this.showTerminalAtom);
        globalStore.set(this.showTerminalAtom, !current);
    }

    interrupt(): void {
        // TODO: Phase 2 - send Ctrl+C to hidden terminal
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
        // TODO: Phase 2 - restart claude process
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
        this.parser.reset();
    }
}

// ============================================================
// React Components
// ============================================================

const ClaudeCodeView: React.FC<ViewComponentProps<ClaudeCodeViewModel>> = memo(
    ({ blockId, blockRef, contentRef, model }) => {
        const turns = useAtomValue(model.turnsAtom);
        const isStreaming = useAtomValue(model.isStreamingAtom);
        const showTerminal = useAtomValue(model.showTerminalAtom);

        return (
            <div ref={contentRef} className="claudecode-view">
                {!showTerminal && (
                    <div className="cc-log-container">
                        <ConversationLog turns={turns} model={model} />
                        {isStreaming && <StreamingCursor />}
                        <InputLine model={model} />
                    </div>
                )}
                {showTerminal && (
                    <div className="cc-raw-terminal">
                        <div className="cc-terminal-placeholder">
                            Raw terminal output (Phase 2)
                        </div>
                    </div>
                )}
                <StatusBar model={model} />
            </div>
        );
    }
);

// --- Conversation Log ---

const ConversationLog = memo(
    ({ turns, model }: { turns: ConversationTurn[]; model: ClaudeCodeViewModel }) => {
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
                        <span className="cc-empty-prompt">{">"}</span> Claude Code
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
        const handleClick = useCallback(() => {
            model.toggleToolCollapse(turnId, block.toolId);
        }, [model, turnId, block.toolId]);

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
                    <div className="cc-tool-detail">
                        {block.name === "Edit" && block.result ? (
                            <DiffBlock input={block.input} result={block.result} />
                        ) : block.name === "Bash" ? (
                            <BashBlock input={block.input} result={block.result} />
                        ) : (
                            <pre className="cc-tool-output">
                                {block.result ?? JSON.stringify(block.input, null, 2)}
                            </pre>
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
        },
        [handleSend, model]
    );

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

    return (
        <div className="cc-statusbar">
            <span className="cc-status-item">
                {isStreaming ? "\u25cf streaming" : "\u25cb idle"}
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
