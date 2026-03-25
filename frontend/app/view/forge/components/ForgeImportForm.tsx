// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { createEffect, createSignal } from "solid-js";
import type { JSX } from "solid-js";
import { Show } from "solid-js";
import type { ForgeViewModel } from "../forge-model";

export function ForgeImportForm(props: { model: ForgeViewModel; onClose: () => void }): JSX.Element {
    const importing = props.model.importingAtom;
    const error = props.model.errorAtom;
    const [workspacePath, setWorkspacePath] = createSignal("");
    const [agentName, setAgentName] = createSignal("");

    // Auto-fill agent name from path
    createEffect(() => {
        const wp = workspacePath();
        if (wp && !agentName()) {
            const parts = wp.replace(/\\/g, "/").split("/").filter(Boolean);
            if (parts.length > 0) {
                setAgentName(parts[parts.length - 1]);
            }
        }
    });

    const handleSubmit = async (e: Event) => {
        e.preventDefault();
        if (!workspacePath().trim() || !agentName().trim()) return;
        await props.model.importFromClaw(workspacePath().trim(), agentName().trim());
        props.onClose();
    };

    return (
        <div class="forge-import-overlay">
            <div class="forge-import-dialog">
                <div class="forge-skill-form-header">
                    <span class="forge-title-sub">Import from Claw</span>
                </div>
                <form onSubmit={handleSubmit}>
                    <div class="forge-form-row">
                        <label class="forge-form-label">Workspace Path</label>
                        <input
                            class="forge-form-input"
                            value={workspacePath()}
                            onInput={(e) => setWorkspacePath(e.currentTarget.value)}
                            placeholder="~/.claw/workspaces/agent1"
                            autofocus
                            required
                        />
                    </div>
                    <div class="forge-form-row">
                        <label class="forge-form-label">Agent Name</label>
                        <input
                            class="forge-form-input"
                            value={agentName()}
                            onInput={(e) => setAgentName(e.currentTarget.value)}
                            placeholder="Agent name"
                            required
                        />
                    </div>
                    <Show when={error()}>
                        <div class="forge-form-error">{error()}</div>
                    </Show>
                    <div class="forge-form-actions">
                        <button type="submit" class="forge-btn-primary" disabled={importing() || !workspacePath().trim() || !agentName().trim()}>
                            {importing() ? "Importing..." : "Import"}
                        </button>
                        <button type="button" class="forge-btn-secondary" onClick={props.onClose} disabled={importing()}>
                            Cancel
                        </button>
                    </div>
                </form>
            </div>
        </div>
    );
}
