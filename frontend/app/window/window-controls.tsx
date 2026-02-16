// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import { getApi } from "@/store/global";
import { WindowService } from "@/app/store/services";
import { fireAndForget } from "@/util/util";
import { memo } from "react";
import "./window-controls.scss";

interface WindowControlsProps {
    platform: string;
    showNativeControls: boolean;
}

const WindowControls = memo(({ platform, showNativeControls }: WindowControlsProps) => {
    // On macOS with native controls, don't show custom window controls
    if (platform === "darwin" && showNativeControls) {
        return null;
    }

    const handleNewWindow = () => {
        fireAndForget(async () => {
            await WindowService.CreateWindow(null, "");
        });
    };

    const handleMinimize = () => {
        getApi().minimizeWindow();
    };

    const handleMaximize = () => {
        getApi().maximizeWindow();
    };

    return (
        <div className="window-controls" data-tauri-drag-region="false">
            <button
                className="window-control-btn new-window-btn"
                onClick={handleNewWindow}
                title="Open New Window"
            >
                <i className="fa fa-window-restore" />
                <span>agentmux</span>
            </button>
            <button
                className="window-control-btn minimize-btn"
                onClick={handleMinimize}
                title="Minimize Window"
            >
                <i className="fa fa-window-minimize" />
            </button>
            <button
                className="window-control-btn maximize-btn"
                onClick={handleMaximize}
                title="Maximize Window"
            >
                <i className="fa fa-window-maximize" />
            </button>
        </div>
    );
});

export { WindowControls };
