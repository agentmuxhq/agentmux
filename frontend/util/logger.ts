// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

/**
 * Enhanced logging framework for AgentMux frontend.
 *
 * Provides structured logging with categories, log levels, and performance timing.
 * Logs are sent to backend for persistent storage.
 */

export enum LogLevel {
    TRACE = 0,
    DEBUG = 1,
    INFO = 2,
    WARN = 3,
    ERROR = 4,
}

class Logger {
    private level: LogLevel = LogLevel.INFO;
    private enabled: boolean = true;
    private timers: Map<string, number> = new Map();

    /**
     * Set minimum log level
     */
    setLevel(level: LogLevel) {
        this.level = level;
        console.log(`[Logger] Level set to: ${LogLevel[level]}`);
    }

    /**
     * Enable/disable logging
     */
    setEnabled(enabled: boolean) {
        this.enabled = enabled;
    }

    /**
     * Check if a log level should be output
     */
    private shouldLog(level: LogLevel): boolean {
        return this.enabled && level >= this.level;
    }

    /**
     * Format and send log message
     */
    private async log(level: LogLevel, category: string, ...args: any[]) {
        if (!this.shouldLog(level)) return;

        const timestamp = new Date().toISOString();
        const levelStr = LogLevel[level];
        const emoji = this.getLevelEmoji(level);

        // Format message
        const msgParts = args.map((arg) => {
            if (typeof arg === "object") {
                try {
                    return JSON.stringify(arg);
                } catch {
                    return String(arg);
                }
            }
            return String(arg);
        });
        const msg = msgParts.join(" ");

        // Console output with emoji
        const consoleMsg = `${emoji} [${timestamp}] [${levelStr}] [${category}] ${msg}`;

        switch (level) {
            case LogLevel.ERROR:
                console.error(consoleMsg);
                break;
            case LogLevel.WARN:
                console.warn(consoleMsg);
                break;
            default:
                console.log(consoleMsg);
        }

        // Send to backend (non-blocking)
        try {
            if ((window as any).api?.sendLog) {
                await (window as any).api.sendLog(consoleMsg);
            }
        } catch (err) {
            // Silently fail to avoid infinite loop
        }
    }

    /**
     * Get emoji for log level
     */
    private getLevelEmoji(level: LogLevel): string {
        switch (level) {
            case LogLevel.TRACE:
                return "🔍";
            case LogLevel.DEBUG:
                return "🐛";
            case LogLevel.INFO:
                return "ℹ️";
            case LogLevel.WARN:
                return "⚠️";
            case LogLevel.ERROR:
                return "❌";
            default:
                return "📝";
        }
    }

    /**
     * Log trace message (most verbose)
     */
    trace(category: string, ...args: any[]) {
        this.log(LogLevel.TRACE, category, ...args);
    }

    /**
     * Log debug message
     */
    debug(category: string, ...args: any[]) {
        this.log(LogLevel.DEBUG, category, ...args);
    }

    /**
     * Log info message
     */
    info(category: string, ...args: any[]) {
        this.log(LogLevel.INFO, category, ...args);
    }

    /**
     * Log warning message
     */
    warn(category: string, ...args: any[]) {
        this.log(LogLevel.WARN, category, ...args);
    }

    /**
     * Log error message
     */
    error(category: string, ...args: any[]) {
        this.log(LogLevel.ERROR, category, ...args);
    }

    /**
     * Start a performance timer
     */
    time(label: string) {
        this.timers.set(label, performance.now());
        this.debug("perf", `Timer started: ${label}`);
    }

    /**
     * Log a lap time
     */
    lap(label: string, milestone: string) {
        const start = this.timers.get(label);
        if (start === undefined) {
            this.warn("perf", `Timer not found: ${label}`);
            return;
        }
        const elapsed = performance.now() - start;
        this.info("perf", `⏱️  ${label} - ${milestone}: ${elapsed.toFixed(2)}ms`);
    }

    /**
     * End a performance timer
     */
    timeEnd(label: string) {
        const start = this.timers.get(label);
        if (start === undefined) {
            this.warn("perf", `Timer not found: ${label}`);
            return;
        }
        const elapsed = performance.now() - start;
        this.info("perf", `✅ ${label} completed: ${elapsed.toFixed(2)}ms`);
        this.timers.delete(label);
    }

    /**
     * Log window lifecycle event
     */
    windowEvent(event: string, details?: any) {
        this.info("window", `🪟 ${event}`, details || "");
    }

    /**
     * Log startup milestone
     */
    startupMilestone(milestone: string, elapsed?: number) {
        if (elapsed !== undefined) {
            this.info("startup", `🚀 ${milestone} (+${elapsed.toFixed(0)}ms)`);
        } else {
            this.info("startup", `🚀 ${milestone}`);
        }
    }
}

// Singleton instance
export const logger = new Logger();

/**
 * Initialize logger based on environment
 */
export function initLogger() {
    // Development mode: more verbose
    if (import.meta.env.DEV) {
        logger.setLevel(LogLevel.DEBUG);
        logger.info("logger", "Logger initialized (DEV mode)");
    } else {
        logger.setLevel(LogLevel.INFO);
        logger.info("logger", "Logger initialized (PROD mode)");
    }

    // Allow override via localStorage
    try {
        const storedLevel = localStorage.getItem("agentmux-log-level");
        if (storedLevel) {
            const level = parseInt(storedLevel) as LogLevel;
            logger.setLevel(level);
            logger.info("logger", `Log level overridden from localStorage: ${LogLevel[level]}`);
        }
    } catch (err) {
        // Ignore localStorage errors
    }

    // Allow override via URL param (?debug=1)
    try {
        const params = new URLSearchParams(window.location.search);
        if (params.get("debug") === "1") {
            logger.setLevel(LogLevel.DEBUG);
            logger.info("logger", "Debug mode enabled via URL parameter");
        }
        if (params.get("trace") === "1") {
            logger.setLevel(LogLevel.TRACE);
            logger.info("logger", "Trace mode enabled via URL parameter");
        }
    } catch (err) {
        // Ignore URL parsing errors
    }
}

// Export convenience function for backward compatibility
export function debugLog(...args: any[]) {
    logger.debug("legacy", ...args);
}
