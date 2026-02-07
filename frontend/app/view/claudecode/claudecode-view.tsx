// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { Markdown } from "@/app/element/markdown";
import { getFileSubject } from "@/app/store/wps";
import { base64ToArray } from "@/util/util";
import { useAtomValue } from "jotai";
import clsx from "clsx";
import React, { memo, useCallback, useEffect, useRef, useState } from "react";
import { getToolOneLiner, RAW_OUTPUT_MAX_LENGTH, TermFileName, TOOL_RESULT_MAX_LENGTH } from "./claudecode-helpers";
import type { ClaudeCodeViewModel } from "./claudecode-model";
import type { ConversationTurn, TextTurnBlock, ToolTurnBlock } from "./claudecode-types";
import "./claudecode.scss";

// ============================================================
// Main View
// ============================================================

const ClaudeCodeView: React.FC<ViewComponentProps<ClaudeCodeViewModel>> = memo(
    ({ model }) => {
        const turns = useAtomValue(model.turnsAtom);
        const isStreaming = useAtomValue(model.isStreamingAtom);
        const showTerminal = useAtomValue(model.showTerminalAtom);
        const connected = useAtomValue(model.connectedAtom);
        const error = useAtomValue(model.errorAtom);

        return (
            <div className="claudecode-view">
                {!showTerminal && (
                    <div className="cc-log-container">
                        <ConversationLog turns={turns} model={model} connected={connected} />
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

// ============================================================
// Sub-components
// ============================================================

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
                <div className="cc-log-spacer" />
                {turns.length === 0 && (
                    <div className="cc-empty">
                        <div className="cc-empty-header">
                            <span className="cc-empty-prompt">{"\u276f"}</span> Claude Code
                        </div>
                        {!connected && (
                            <div className="cc-empty-status">Waiting for claude process...</div>
                        )}
                        {connected && (
                            <div className="cc-empty-status">Ready. Type a message below.</div>
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
                        <ToolBlockView key={block.toolId} block={block} turnId={turn.id} model={model} />
                    );
                }
                return null;
            })}
            <div className="cc-turn-separator" />
        </div>
    );
});

const TextBlockView = memo(({ block }: { block: TextTurnBlock }) => {
    return (
        <div className="cc-text">
            <Markdown text={block.text} />
        </div>
    );
});

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
        const displayResult =
            isTruncated && !showFull ? block.result!.slice(0, TOOL_RESULT_MAX_LENGTH) : block.result;

        return (
            <div className={clsx("cc-tool", { expanded: !block.isCollapsed })} onClick={handleClick}>
                <div className="cc-tool-line">
                    <span className="cc-tool-chevron">{block.isCollapsed ? "\u25b8" : "\u25be"}</span>
                    <span className="cc-tool-name">{block.name}</span>
                    <span className="cc-tool-summary">{summary}</span>
                    {block.isError && <span className="cc-tool-error">{"\u2717"}</span>}
                    {!block.isError && block.result != null && <span className="cc-tool-ok">{"\u2713"}</span>}
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
                                onClick={(e) => {
                                    e.stopPropagation();
                                    setShowFull(true);
                                }}
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

const StreamingCursor = memo(() => {
    return (
        <div className="cc-streaming">
            <span className="cc-streaming-cursor">{"\u2588"}</span>
        </div>
    );
});

const InputLine = memo(({ model }: { model: ClaudeCodeViewModel }) => {
    const [text, setText] = useState("");
    const isStreaming = useAtomValue(model.isStreamingAtom);
    const procStatus = useAtomValue(model.shellProcStatusAtom);
    const inputDisabled = isStreaming || procStatus === "done";

    const handleSend = useCallback(() => {
        if (!text.trim() || inputDisabled) return;
        model.sendMessage(text);
        setText("");
    }, [text, inputDisabled, model]);

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

    // Auto-focus when streaming ends (deferred to avoid layout effect loop)
    useEffect(() => {
        if (!isStreaming && model.inputRef.current) {
            requestAnimationFrame(() => model.inputRef.current?.focus());
        }
    }, [isStreaming, model]);

    // Auto-expand textarea
    useEffect(() => {
        const el = model.inputRef.current;
        if (el) {
            el.style.height = "auto";
            el.style.height = el.scrollHeight + "px";
        }
    }, [text, model]);

    return (
        <div className="cc-input">
            <span className="cc-input-prompt">{"\u276f"}</span>
            <textarea
                ref={model.inputRef as React.RefObject<HTMLTextAreaElement>}
                className="cc-input-textarea"
                value={text}
                onChange={(e) => setText(e.target.value)}
                onKeyDown={handleKeyDown}
                disabled={inputDisabled}
                placeholder={procStatus === "done" ? "Process exited. Click [restart]." : "Ask Claude..."}
                rows={1}
                spellCheck={false}
            />
        </div>
    );
});

const StatusBar = memo(({ model }: { model: ClaudeCodeViewModel }) => {
    const meta = useAtomValue(model.sessionMetaAtom);
    const isStreaming = useAtomValue(model.isStreamingAtom);
    const showTerminal = useAtomValue(model.showTerminalAtom);
    const procStatus = useAtomValue(model.shellProcStatusAtom);
    const exitCode = useAtomValue(model.shellProcExitCodeAtom);

    const statusText =
        procStatus === "done"
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
            {meta.totalCost > 0 && <span className="cc-status-item">${meta.totalCost.toFixed(3)}</span>}
            <span className="cc-status-spacer" />
            {procStatus === "done" && (
                <button className="cc-status-btn cc-restart-btn" onClick={() => model.restartProcess()}>
                    [restart]
                </button>
            )}
            <button className="cc-status-btn" onClick={() => model.toggleTerminal()}>
                {showTerminal ? "[chat]" : "[term]"}
            </button>
            <button className="cc-status-btn" onClick={() => model.interrupt()} disabled={!isStreaming}>
                [^C]
            </button>
            <button className="cc-status-btn" onClick={() => model.resetSession()}>
                [reset]
            </button>
        </div>
    );
});

const ErrorBanner = memo(({ message }: { message: string }) => {
    return (
        <div className="cc-error">
            <span className="cc-error-icon">{"\u26a0"}</span>
            <span className="cc-error-text">{message}</span>
        </div>
    );
});

export { ClaudeCodeView };
