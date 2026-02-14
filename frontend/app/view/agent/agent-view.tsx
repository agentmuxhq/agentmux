// Copyright 2024, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * AgentView - Unified agent widget with living markdown document
 *
 * Displays agent activity as an interactive document with:
 * - Markdown paragraphs
 * - Collapsible tool blocks
 * - Agent-to-agent messages
 * - User messages
 */

import { useAtomValue, useSetAtom } from "jotai";
import clsx from "clsx";
import React, { memo, useCallback, useEffect, useRef } from "react";
import type { DocumentNode } from "./types";
import {
    documentStateAtom,
    filteredDocumentAtom,
    toggleNodeCollapsed,
} from "./state";
import { MarkdownBlock } from "./components/MarkdownBlock";
import { ToolBlock } from "./components/ToolBlock";
import { AgentMessageBlock } from "./components/AgentMessageBlock";
import "./agent-view.scss";

interface AgentViewProps {
    agentId: string;
}

export const AgentView: React.FC<AgentViewProps> = memo(({ agentId }) => {
    const document = useAtomValue(filteredDocumentAtom);
    const documentState = useAtomValue(documentStateAtom);
    const toggleCollapse = useSetAtom(toggleNodeCollapsed);
    const scrollRef = useRef<HTMLDivElement>(null);

    // Auto-scroll to bottom on new nodes
    useEffect(() => {
        if (scrollRef.current) {
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
    }, [document.length]);

    const renderNode = useCallback(
        (node: DocumentNode) => {
            const isCollapsed = documentState.collapsedNodes.has(node.id);

            switch (node.type) {
                case "markdown":
                    return <MarkdownBlock key={node.id} node={node} />;

                case "tool":
                    return (
                        <ToolBlock
                            key={node.id}
                            node={node}
                            collapsed={isCollapsed || node.collapsed}
                            onToggle={() => toggleCollapse(node.id)}
                        />
                    );

                case "agent_message":
                    return (
                        <AgentMessageBlock
                            key={node.id}
                            node={node}
                            collapsed={isCollapsed || node.collapsed}
                            onToggle={() => toggleCollapse(node.id)}
                        />
                    );

                case "user_message":
                    return (
                        <div key={node.id} className="agent-user-message">
                            <div className="agent-user-message-icon">👤</div>
                            <div className="agent-user-message-content">
                                <pre>{node.message}</pre>
                            </div>
                        </div>
                    );

                case "section":
                    return (
                        <div key={node.id} className={clsx("agent-section", `level-${node.level}`)}>
                            {node.level === 1 && <h1>{node.title}</h1>}
                            {node.level === 2 && <h2>{node.title}</h2>}
                            {node.level === 3 && <h3>{node.title}</h3>}
                        </div>
                    );

                default:
                    return null;
            }
        },
        [documentState.collapsedNodes, toggleCollapse]
    );

    return (
        <div className="agent-view">
            <div className="agent-document" ref={scrollRef}>
                {document.length === 0 ? (
                    <div className="agent-empty">
                        <div className="agent-empty-icon">🤖</div>
                        <div className="agent-empty-text">
                            Agent {agentId} is idle
                            <br />
                            Waiting for activity...
                        </div>
                    </div>
                ) : (
                    document.map(renderNode)
                )}
            </div>
        </div>
    );
});

AgentView.displayName = "AgentView";
