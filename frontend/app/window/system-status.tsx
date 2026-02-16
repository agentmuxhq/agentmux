// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * SystemStatus - Right side of window header
 * Contains action widgets, update banner, config errors, and close button
 */

import { Button } from "@/app/element/button";
import { modalsModel } from "@/app/store/modalmodel";
import { atoms, getApi } from "@/store/global";
import { useAtomValue } from "jotai";
import { forwardRef, memo } from "react";
import { ActionWidgets } from "./action-widgets";
import { UpdateStatusBanner } from "@/app/tab/updatebanner";
import "./system-status.scss";

const ConfigErrorMessage = () => {
    const fullConfig = useAtomValue(atoms.fullConfigAtom);

    if (fullConfig?.configerrors == null || fullConfig?.configerrors.length == 0) {
        return (
            <div className="config-error-message">
                <h3>Configuration Clean</h3>
                <p>There are no longer any errors detected in your config.</p>
            </div>
        );
    }
    if (fullConfig?.configerrors.length == 1) {
        const singleError = fullConfig.configerrors[0];
        return (
            <div className="config-error-message">
                <h3>Configuration Error</h3>
                <div>
                    {singleError.file}: {singleError.err}
                </div>
            </div>
        );
    }
    return (
        <div className="config-error-message">
            <h3>Configuration Error</h3>
            <ul>
                {fullConfig.configerrors.map((error, index) => (
                    <li key={index}>
                        {error.file}: {error.err}
                    </li>
                ))}
            </ul>
        </div>
    );
};

const ConfigErrorIcon = forwardRef<HTMLElement>((_, ref) => {
    const fullConfig = useAtomValue(atoms.fullConfigAtom);

    function handleClick() {
        modalsModel.pushModal("MessageModal", { children: <ConfigErrorMessage /> });
    }

    if (fullConfig?.configerrors == null || fullConfig?.configerrors.length == 0) {
        return null;
    }
    return (
        <Button
            ref={ref as React.RefObject<HTMLButtonElement>}
            className="config-error-button red"
            onClick={handleClick}
        >
            <i className="fa fa-solid fa-exclamation-triangle" />
            Config Error
        </Button>
    );
});

const CloseButton = memo(() => {
    const handleClose = () => {
        getApi().closeWindow();
    };

    return (
        <div
            className="close-button"
            onClick={handleClose}
            title="Close Window"
            data-tauri-drag-region="false"
        >
            <i className="fa fa-times" />
        </div>
    );
});

interface SystemStatusProps {
    updateStatusBannerRef?: React.RefObject<HTMLButtonElement>;
    configErrorRef?: React.RefObject<HTMLElement>;
}

const SystemStatus = memo(({ updateStatusBannerRef, configErrorRef }: SystemStatusProps) => {
    return (
        <div className="system-status">
            <ActionWidgets />
            <UpdateStatusBanner ref={updateStatusBannerRef} />
            <ConfigErrorIcon ref={configErrorRef} />
            <CloseButton />
        </div>
    );
});

export { SystemStatus, ConfigErrorMessage };
