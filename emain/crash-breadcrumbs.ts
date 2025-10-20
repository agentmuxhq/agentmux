// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

import * as fs from "fs";
import * as path from "path";
import { getWaveDataDir } from "./platform";
import { log } from "./log";

interface Breadcrumb {
    timestamp: number;
    type: string;
    data: any;
}

const MAX_BREADCRUMBS = 100;
const breadcrumbs: Breadcrumb[] = [];

/**
 * Record a breadcrumb event for crash debugging
 * Breadcrumbs are recent user actions/events that help understand what led to a crash
 */
export function writeCrashBreadcrumb(type: string, data: any) {
    const breadcrumb: Breadcrumb = {
        timestamp: Date.now(),
        type,
        data,
    };

    breadcrumbs.push(breadcrumb);

    // Keep only last N breadcrumbs
    while (breadcrumbs.length > MAX_BREADCRUMBS) {
        breadcrumbs.shift();
    }

    // Write to file immediately so we don't lose breadcrumbs if we crash
    try {
        saveBreadcrumbsToFile();
    } catch (e) {
        // Don't throw - breadcrumb writing failures should not crash the app
        log("Warning: Failed to save breadcrumbs:", e);
    }
}

/**
 * Get all breadcrumbs from memory
 */
export function getCrashBreadcrumbs(): Breadcrumb[] {
    return [...breadcrumbs];
}

/**
 * Clear all breadcrumbs
 */
export function clearBreadcrumbs() {
    breadcrumbs.length = 0;
    try {
        const file = getBreadcrumbsFilePath();
        if (fs.existsSync(file)) {
            fs.unlinkSync(file);
        }
    } catch (e) {
        log("Warning: Failed to clear breadcrumbs file:", e);
    }
}

function getBreadcrumbsFilePath(): string {
    return path.join(getWaveDataDir(), "crash-breadcrumbs.json");
}

function saveBreadcrumbsToFile() {
    const file = getBreadcrumbsFilePath();
    const data = JSON.stringify(breadcrumbs, null, 2);
    fs.writeFileSync(file, data, "utf-8");
}

/**
 * Load breadcrumbs from file (from previous session)
 */
export function loadBreadcrumbsFromFile(): Breadcrumb[] {
    try {
        const file = getBreadcrumbsFilePath();
        if (fs.existsSync(file)) {
            const content = fs.readFileSync(file, "utf-8");
            const loaded = JSON.parse(content);
            if (Array.isArray(loaded)) {
                return loaded;
            }
        }
    } catch (e) {
        log("Warning: Failed to load breadcrumbs from file:", e);
    }
    return [];
}

/**
 * Simplified helper to track common events
 */
export function trackEvent(type: string, data?: any) {
    writeCrashBreadcrumb(type, data || {});
}

/**
 * Format breadcrumbs as a human-readable string
 */
export function formatBreadcrumbs(crumbs: Breadcrumb[], limit?: number): string {
    const items = limit ? crumbs.slice(-limit) : crumbs;

    if (items.length === 0) {
        return "No breadcrumbs recorded";
    }

    return items
        .map((b) => {
            const time = new Date(b.timestamp).toISOString();
            const dataStr = typeof b.data === "object" ? JSON.stringify(b.data) : String(b.data);
            return `[${time}] ${b.type}: ${dataStr}`;
        })
        .join("\n");
}
