// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { getApi } from "@/store/global";
import { memo } from "react";
import "./window-controls.scss";

interface WindowControlsProps {
    platform: string;
    showNativeControls: boolean;
}

const WindowControls = memo(({ platform, showNativeControls }: WindowControlsProps) => {
    const handleNewWindow = async () => {
        try {
            const newWindowLabel = await getApi().openNewWindow();
            console.log("[WindowControls] Opened new window:", newWindowLabel);
        } catch (error) {
            console.error("[WindowControls] Failed to open new window:", error);
        }
    };

    return (
        <div className="window-controls" data-tauri-drag-region="false" data-testid="window-controls">
            <button
                className="window-control-btn new-window-btn"
                onClick={handleNewWindow}
                title="Open New Window"
                data-testid="new-window-btn"
            >
                <i className="fa fa-window-restore" />
                <span>agentmux</span>
            </button>
        </div>
    );
});

export { WindowControls };
