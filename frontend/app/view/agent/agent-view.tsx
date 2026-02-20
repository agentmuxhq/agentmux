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
import { Atom } from "jotai";
import clsx from "clsx";
import React, { memo, useCallback, useEffect, useRef, useState } from "react";
import type { DocumentNode } from "./types";
import type { AgentViewModel } from "./agent-model";
import type { AgentAtoms } from "./state";
import { MarkdownBlock } from "./components/MarkdownBlock";
import { ToolBlock } from "./components/ToolBlock";
import { AgentMessageBlock } from "./components/AgentMessageBlock";
import { AgentHeader } from "./components/AgentHeader";
import { AgentFooter } from "./components/AgentFooter";
import { ConnectionStatus } from "./components/ConnectionStatus";
import { FilterControls } from "./components/FilterControls";
import { SetupWizard } from "./components/SetupWizard";
import "./agent-view.scss";

interface AgentViewProps {
    agentId: string;
    atoms: AgentAtoms; // Instance-scoped atoms from ViewModel
    filteredDocumentAtom: Atom<DocumentNode[]>;
    documentStatsAtom: Atom<any>;
    toggleNodeCollapsed: any;
    expandAllNodes: any;
    collapseAllNodes: any;
    clearDocument: any;
    updateFilter: any;
    onSendMessage?: (message: string) => void;
    onExport?: (format: "markdown" | "html") => void;
    onPause?: () => void;
    onResume?: () => void;
    onKill?: () => void;
    onRestart?: () => void;
    onStartLogin?: () => void;
}

/**
 * Wrapper component that adapts ViewComponentProps to AgentViewProps.
 * Gates on provider setup_complete — shows SetupWizard if not configured.
 */
export const AgentViewWrapper: React.FC<ViewComponentProps<AgentViewModel>> = memo(({ model }) => {
    const providerConfig = useAtomValue(model.atoms.providerConfigAtom);

    const handleSetupComplete = useCallback(
        (config: ProviderConfig) => {
            model.startWithProvider(config);
        },
        [model]
    );

    if (!providerConfig || !providerConfig.setup_complete) {
        return <SetupWizard onSetupComplete={handleSetupComplete} />;
    }

    return (
        <AgentViewInner
            agentId={model.agentIdValue}
            atoms={model.atoms}
            filteredDocumentAtom={model.filteredDocumentAtom}
            documentStatsAtom={model.documentStatsAtom}
            toggleNodeCollapsed={model.toggleNodeCollapsed}
            expandAllNodes={model.expandAllNodes}
            collapseAllNodes={model.collapseAllNodes}
            clearDocument={model.clearDocument}
            updateFilter={model.updateFilter}
            onSendMessage={model.sendMessage}
            onExport={model.exportDocument}
            onPause={model.pauseAgent}
            onResume={model.resumeAgent}
            onKill={model.killAgent}
            onRestart={model.restartAgent}
            onStartLogin={model.startAuthLogin}
        />
    );
});

AgentViewWrapper.displayName = "AgentViewWrapper";

export const AgentViewInner: React.FC<AgentViewProps> = memo(
    ({
        agentId,
        atoms,
        filteredDocumentAtom,
        documentStatsAtom,
        toggleNodeCollapsed,
        expandAllNodes,
        collapseAllNodes,
        clearDocument,
        updateFilter,
        onSendMessage,
        onExport,
        onPause,
        onResume,
        onKill,
        onRestart,
        onStartLogin,
    }) => {
        // Use instance-scoped atoms from props
        const document = useAtomValue(filteredDocumentAtom);
        const documentState = useAtomValue(atoms.documentStateAtom);
        const authState = useAtomValue(atoms.authAtom);
        const toggleCollapse = useSetAtom(toggleNodeCollapsed);
        const scrollRef = useRef<HTMLDivElement>(null);
        const [showFilters, setShowFilters] = useState(false);

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
                <AgentHeader
                    agentId={agentId}
                    processAtom={atoms.processAtom}
                    streamingStateAtom={atoms.streamingStateAtom}
                    messageRouterAtom={atoms.messageRouterAtom}
                    onPause={onPause}
                    onResume={onResume}
                    onKill={onKill}
                    onRestart={onRestart}
                />

                <div className="agent-main-container">
                    {showFilters && (
                        <div className="agent-sidebar">
                            <FilterControls
                                documentStateAtom={atoms.documentStateAtom}
                                documentStatsAtom={documentStatsAtom}
                                updateFilter={updateFilter}
                            />
                        </div>
                    )}

                    <div className="agent-document" ref={scrollRef}>
                        <button
                            className="agent-filter-toggle"
                            onClick={() => setShowFilters(!showFilters)}
                            title="Toggle filters"
                        >
                            🔍 {showFilters ? "Hide" : "Show"} Filters
                        </button>

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

                {/* Footer: Show connection UI or message input based on auth state */}
                {authState.status === "disconnected" ? (
                    <ConnectionStatus
                        authAtom={atoms.authAtom}
                        userInfoAtom={atoms.userInfoAtom}
                        providerConfigAtom={atoms.providerConfigAtom}
                        onRestart={onRestart}
                        onStartLogin={onStartLogin}
                    />
                ) : (
                    <AgentFooter agentId={agentId} onSendMessage={onSendMessage} />
                )}
            </div>
        );
    }
);

AgentViewInner.displayName = "AgentViewInner";

// Re-export wrapper as AgentView for backward compatibility
export const AgentView = AgentViewWrapper;
