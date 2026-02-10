// Copyright 2026, AgentMux Contributors
// SPDX-License-Identifier: Apache-2.0

// Hook for notification and flash error management.
// Provides a clean API for pushing/removing notifications from view components.

import { useCallback } from "react";
import { globalStore, atoms, pushFlashError, pushNotification, removeFlashError, removeNotification, removeNotificationById } from "@/app/store/global";

type NotificationActions = {
    push: (notif: NotificationType) => void;
    remove: (id: string) => void;
    removeById: (id: string) => void;
};

type FlashErrorActions = {
    push: (error: FlashErrorType) => void;
    remove: (id: string) => void;
};

/**
 * Hook that returns notification management actions.
 * Use this instead of importing individual functions from global.ts.
 */
export function useNotificationActions(): NotificationActions {
    return {
        push: useCallback((notif: NotificationType) => pushNotification(notif), []),
        remove: useCallback((id: string) => removeNotification(id), []),
        removeById: useCallback((id: string) => removeNotificationById(id), []),
    };
}

/**
 * Hook that returns flash error management actions.
 */
export function useFlashErrorActions(): FlashErrorActions {
    return {
        push: useCallback((error: FlashErrorType) => pushFlashError(error), []),
        remove: useCallback((id: string) => removeFlashError(id), []),
    };
}
