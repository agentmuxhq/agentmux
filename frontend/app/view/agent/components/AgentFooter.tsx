// Copyright 2024, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * AgentFooter - User input and document actions
 */

import { useSetAtom } from "jotai";
import React, { memo, useCallback, useState } from "react";
import { clearDocument, collapseAllNodes, expandAllNodes } from "../state";

interface AgentFooterProps {
    agentId: string;
    onSendMessage?: (message: string) => void;
    onExport?: (format: "markdown" | "html") => void;
}

export const AgentFooter: React.FC<AgentFooterProps> = memo(
    ({ agentId, onSendMessage, onExport }) => {
        const [message, setMessage] = useState("");
        const clearDoc = useSetAtom(clearDocument);
        const expandAll = useSetAtom(expandAllNodes);
        const collapseAll = useSetAtom(collapseAllNodes);

        const handleSend = useCallback(() => {
            if (!message.trim()) return;
            if (onSendMessage) {
                onSendMessage(message);
                setMessage("");
            }
        }, [message, onSendMessage]);

        const handleKeyDown = useCallback(
            (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
                if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    handleSend();
                }
            },
            [handleSend]
        );

        const handleExport = useCallback(
            (format: "markdown" | "html") => {
                if (onExport) {
                    onExport(format);
                }
            },
            [onExport]
        );

        const handleClear = useCallback(() => {
            if (confirm("Are you sure you want to clear the document?")) {
                clearDoc();
            }
        }, [clearDoc]);

        return (
            <div className="agent-footer">
                <div className="agent-input-container">
                    <textarea
                        className="agent-input"
                        placeholder={`Send message to ${agentId}...`}
                        value={message}
                        onChange={(e) => setMessage(e.target.value)}
                        onKeyDown={handleKeyDown}
                        rows={2}
                    />
                    <button className="agent-send-btn" onClick={handleSend} disabled={!message.trim()}>
                        Send
                    </button>
                </div>

                <div className="agent-footer-actions">
                    <button className="agent-action-btn" onClick={() => expandAll()} title="Expand all nodes">
                        Expand All
                    </button>
                    <button
                        className="agent-action-btn"
                        onClick={() => collapseAll()}
                        title="Collapse all nodes"
                    >
                        Collapse All
                    </button>
                    <button
                        className="agent-action-btn"
                        onClick={() => handleExport("markdown")}
                        title="Export as Markdown"
                    >
                        Export MD
                    </button>
                    <button
                        className="agent-action-btn"
                        onClick={() => handleExport("html")}
                        title="Export as HTML"
                    >
                        Export HTML
                    </button>
                    <button
                        className="agent-action-btn agent-clear-btn"
                        onClick={handleClear}
                        title="Clear document"
                    >
                        Clear
                    </button>
                </div>
            </div>
        );
    }
);

AgentFooter.displayName = "AgentFooter";
