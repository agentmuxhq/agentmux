// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0

import { Markdown } from "@/app/element/markdown";
import { useAtomValue } from "jotai";
import clsx from "clsx";
import React, { memo, useCallback, useEffect, useRef, useState } from "react";

import type { UnifiedAIViewModel } from "./unifiedai-model";
import type {
    UnifiedMessage,
    UnifiedMessagePart,
    TextPart,
    ReasoningPart,
    ToolUsePart,
    ToolResultPart,
    DiffPart,
    ErrorPart,
    AgentBackendConfig,
    AgentStatusType,
} from "./unified-types";
import { getToolOneLiner } from "./unified-types";
import "./unifiedai.scss";

// Max chars before truncation in tool results
const TOOL_RESULT_MAX_LENGTH = 8000;

// ============================================================
// Main View
// ============================================================

export const UnifiedAIView: React.FC<ViewComponentProps<UnifiedAIViewModel>> = memo(
    ({ model }) => {
        const messages = useAtomValue(model.messagesAtom);
        const isStreaming = useAtomValue(model.isStreamingAtom);
        const status = useAtomValue(model.statusAtom);

        return (
            <div className="unifiedai-view">
                <div className="uai-main">
                    <MessageLog messages={messages} model={model} status={status} />
                    {isStreaming && <StreamingCursor />}
                    <InputLine model={model} />
                </div>
                <StatusBar model={model} />
            </div>
        );
    }
);

// ============================================================
// Message Log
// ============================================================

const MessageLog = memo(
    ({
        messages,
        model,
        status,
    }: {
        messages: UnifiedMessage[];
        model: UnifiedAIViewModel;
        status: AgentStatusType;
    }) => {
        const scrollRef = useRef<HTMLDivElement>(null);
        const backends = useAtomValue(model.availableBackendsAtom);
        const selectedBackend = useAtomValue(model.selectedBackendAtom);

        // Auto-scroll on new messages
        useEffect(() => {
            if (scrollRef.current) {
                scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
            }
        }, [messages]);

        return (
            <div className="uai-log" ref={scrollRef}>
                <div className="uai-log-spacer" />
                {messages.length === 0 && (
                    <EmptyState
                        backends={backends}
                        selectedBackend={selectedBackend}
                        status={status}
                        onSelectBackend={(id) => model.selectBackend(id)}
                    />
                )}
                {messages.map((msg) => (
                    <MessageView key={msg.id} message={msg} />
                ))}
            </div>
        );
    }
);

// ============================================================
// Empty State
// ============================================================

const EmptyState = memo(
    ({
        backends,
        selectedBackend,
        status,
        onSelectBackend,
    }: {
        backends: AgentBackendConfig[];
        selectedBackend: string;
        status: AgentStatusType;
        onSelectBackend: (id: string) => void;
    }) => {
        return (
            <div className="uai-empty">
                <div className="uai-empty-header">
                    <span className="uai-empty-icon">{"\u2728"}</span> Unified AI
                </div>
                {backends.length === 0 && (
                    <div className="uai-empty-status">
                        No AI backends detected. Install Claude Code, Gemini CLI, or Codex CLI.
                    </div>
                )}
                {backends.length > 0 && (
                    <>
                        <div className="uai-empty-status">
                            {status === "init" ? "Type a message to start." : `Status: ${status}`}
                        </div>
                        {backends.length > 1 && (
                            <div className="uai-backend-selector">
                                {backends.map((b) => (
                                    <button
                                        key={b.id}
                                        className={clsx("uai-backend-btn", {
                                            selected: b.id === selectedBackend,
                                        })}
                                        onClick={() => onSelectBackend(b.id)}
                                    >
                                        {b.display_name}
                                    </button>
                                ))}
                            </div>
                        )}
                    </>
                )}
            </div>
        );
    }
);

// ============================================================
// Message View
// ============================================================

const MessageView = memo(({ message }: { message: UnifiedMessage }) => {
    if (message.role === "user") {
        return <UserMessage message={message} />;
    }
    return <AssistantMessage message={message} />;
});

const UserMessage = memo(({ message }: { message: UnifiedMessage }) => {
    const text = message.parts
        .filter((p): p is TextPart => p.type === "text")
        .map((p) => p.text)
        .join("");

    return (
        <div className="uai-turn">
            <div className="uai-prompt">
                <span className="uai-prompt-char">{"\u276f"}</span>
                <span className="uai-prompt-text">{text}</span>
            </div>
        </div>
    );
});

const AssistantMessage = memo(({ message }: { message: UnifiedMessage }) => {
    return (
        <div className="uai-response">
            {message.parts.map((part, i) => (
                <PartView key={`${message.id}-${i}`} part={part} />
            ))}
            {message.status === "error" && message.parts.length === 0 && (
                <div className="uai-error">
                    <span className="uai-error-icon">{"\u26a0"}</span>
                    <span>Unknown error</span>
                </div>
            )}
        </div>
    );
});

// ============================================================
// Part Renderers
// ============================================================

const PartView = memo(({ part }: { part: UnifiedMessagePart }) => {
    switch (part.type) {
        case "text":
            return <TextPartView part={part} />;
        case "reasoning":
            return <ReasoningPartView part={part} />;
        case "tool_use":
            return <ToolUsePartView part={part} />;
        case "tool_result":
            return <ToolResultPartView part={part} />;
        case "diff":
            return <DiffPartView part={part} />;
        case "error":
            return <ErrorPartView part={part} />;
        case "file":
            return <FilePartView filename={part.filename} />;
        case "metadata":
            return null; // metadata is internal, not rendered
        default:
            return null;
    }
});

const TextPartView = memo(({ part }: { part: TextPart }) => {
    if (!part.text) return null;
    return (
        <div className="uai-text">
            <Markdown text={part.text} />
        </div>
    );
});

const ReasoningPartView = memo(({ part }: { part: ReasoningPart }) => {
    const [expanded, setExpanded] = useState(false);

    if (!part.text) return null;
    return (
        <div className="uai-reasoning" onClick={() => setExpanded(!expanded)}>
            <div className="uai-reasoning-header">
                <span className="uai-reasoning-chevron">{expanded ? "\u25be" : "\u25b8"}</span>
                <span className="uai-reasoning-label">Thinking...</span>
            </div>
            {expanded && (
                <div className="uai-reasoning-content">
                    <Markdown text={part.text} />
                </div>
            )}
        </div>
    );
});

const ToolUsePartView = memo(({ part }: { part: ToolUsePart }) => {
    const [isCollapsed, setIsCollapsed] = useState(true);
    const summary = getToolOneLiner(part.name, part.input);

    const handleClick = useCallback(() => {
        setIsCollapsed((prev) => !prev);
    }, []);

    return (
        <div className={clsx("uai-tool", { expanded: !isCollapsed })} onClick={handleClick}>
            <div className="uai-tool-line">
                <span className="uai-tool-chevron">{isCollapsed ? "\u25b8" : "\u25be"}</span>
                <span className="uai-tool-name">{part.name}</span>
                <span className="uai-tool-summary">{summary}</span>
                {part.approval === "pending" && (
                    <span className="uai-tool-pending">{"\u23f3"}</span>
                )}
            </div>
            {!isCollapsed && (
                <div className="uai-tool-detail" onClick={(e) => e.stopPropagation()}>
                    {part.name === "Edit" && part.input?.file_path ? (
                        <div className="uai-tool-file-path">{part.input.file_path}</div>
                    ) : part.name === "Bash" && part.input?.command ? (
                        <BashInputBlock command={part.input.command} />
                    ) : part.input ? (
                        <pre className="uai-tool-output">
                            {JSON.stringify(part.input, null, 2)}
                        </pre>
                    ) : null}
                </div>
            )}
        </div>
    );
});

const ToolResultPartView = memo(({ part }: { part: ToolResultPart }) => {
    const [showFull, setShowFull] = useState(false);
    const isTruncated = part.content.length > TOOL_RESULT_MAX_LENGTH;
    const displayContent = isTruncated && !showFull ? part.content.slice(0, TOOL_RESULT_MAX_LENGTH) : part.content;

    // Don't render empty results
    if (!part.content) return null;

    return (
        <div className={clsx("uai-tool-result", { "is-error": part.is_error })}>
            <pre className="uai-tool-result-content">{displayContent}</pre>
            {isTruncated && !showFull && (
                <button
                    className="uai-tool-show-more"
                    onClick={() => setShowFull(true)}
                >
                    Show full output ({(part.content.length / 1024).toFixed(0)}KB)
                </button>
            )}
        </div>
    );
});

const DiffPartView = memo(({ part }: { part: DiffPart }) => {
    const lines = part.content.split("\n");
    return (
        <pre className="uai-diff">
            <div className="uai-diff-header">{part.path}</div>
            {lines.map((line, i) => {
                const cls = line.startsWith("+")
                    ? "uai-diff-add"
                    : line.startsWith("-")
                      ? "uai-diff-del"
                      : line.startsWith("@")
                        ? "uai-diff-hunk"
                        : "uai-diff-ctx";
                return (
                    <div key={i} className={cls}>
                        {line}
                    </div>
                );
            })}
        </pre>
    );
});

const ErrorPartView = memo(({ part }: { part: ErrorPart }) => {
    return (
        <div className="uai-error">
            <span className="uai-error-icon">{"\u26a0"}</span>
            <span className="uai-error-text">{part.message}</span>
        </div>
    );
});

const FilePartView = memo(({ filename }: { filename: string }) => {
    return (
        <div className="uai-file">
            <span className="uai-file-icon">{"\ud83d\udcc4"}</span>
            <span className="uai-file-name">{filename}</span>
        </div>
    );
});

// ============================================================
// Tool-Specific Sub-Renderers
// ============================================================

const BashInputBlock = memo(({ command }: { command: string }) => {
    return (
        <div className="uai-bash">
            <div className="uai-bash-cmd">
                <span className="uai-bash-dollar">$</span> {command}
            </div>
        </div>
    );
});

// ============================================================
// Streaming Indicator
// ============================================================

const StreamingCursor = memo(() => {
    return (
        <div className="uai-streaming">
            <span className="uai-streaming-cursor">{"\u2588"}</span>
        </div>
    );
});

// ============================================================
// Input Line
// ============================================================

const InputLine = memo(({ model }: { model: UnifiedAIViewModel }) => {
    const [text, setText] = useState("");
    const isStreaming = useAtomValue(model.isStreamingAtom);
    const status = useAtomValue(model.statusAtom);
    const backends = useAtomValue(model.availableBackendsAtom);
    const inputDisabled = backends.length === 0;

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
            if (e.key === "Escape" && isStreaming) {
                model.interrupt();
            }
            if (e.key === "l" && (e.ctrlKey || e.metaKey)) {
                e.preventDefault();
                model.resetSession();
            }
        },
        [handleSend, model, isStreaming]
    );

    // Auto-focus when streaming ends
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

    const placeholder = backends.length === 0
        ? "No AI backends found"
        : status === "done"
          ? "Agent exited. Type to restart."
          : "Ask AI...";

    return (
        <div className="uai-input">
            <span className="uai-input-prompt">{"\u276f"}</span>
            <textarea
                ref={model.inputRef as React.RefObject<HTMLTextAreaElement>}
                className="uai-input-textarea"
                value={text}
                onChange={(e) => setText(e.target.value)}
                onKeyDown={handleKeyDown}
                disabled={inputDisabled}
                placeholder={placeholder}
                rows={1}
                spellCheck={false}
            />
        </div>
    );
});

// ============================================================
// Status Bar
// ============================================================

const StatusBar = memo(({ model }: { model: UnifiedAIViewModel }) => {
    const status = useAtomValue(model.statusAtom);
    const isStreaming = useAtomValue(model.isStreamingAtom);
    const usage = useAtomValue(model.totalUsageAtom);
    const selectedBackend = useAtomValue(model.selectedBackendAtom);
    const backends = useAtomValue(model.availableBackendsAtom);

    const cfg = backends.find((b) => b.id === selectedBackend);
    const totalTokens = usage.input_tokens + usage.output_tokens;

    const statusText =
        status === "done"
            ? "\u25cf exited"
            : status === "error"
              ? "\u25cf error"
              : isStreaming
                ? "\u25cf streaming"
                : status === "running"
                  ? "\u25cb idle"
                  : "\u25cb ready";

    return (
        <div className="uai-statusbar">
            <span
                className={clsx("uai-status-item", {
                    "uai-status-error": status === "error" || status === "done",
                })}
            >
                {statusText}
            </span>
            {cfg && <span className="uai-status-item">{cfg.display_name}</span>}
            {totalTokens > 0 && (
                <span className="uai-status-item">{(totalTokens / 1000).toFixed(1)}k tokens</span>
            )}
            <span className="uai-status-spacer" />
            {backends.length > 1 && (
                <select
                    className="uai-backend-select"
                    value={selectedBackend}
                    onChange={(e) => model.selectBackend(e.target.value)}
                >
                    {backends.map((b) => (
                        <option key={b.id} value={b.id}>
                            {b.display_name}
                        </option>
                    ))}
                </select>
            )}
            <button
                className="uai-status-btn"
                onClick={() => model.interrupt()}
                disabled={!isStreaming}
            >
                [^C]
            </button>
            <button className="uai-status-btn" onClick={() => model.resetSession()}>
                [reset]
            </button>
        </div>
    );
});
