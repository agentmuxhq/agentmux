// Copyright 2025, a5af.
// SPDX-License-Identifier: Apache-2.0

export interface ProviderDefinition {
    id: string;
    displayName: string;
    cliCommand: string;
    defaultArgs: string[];
    outputFormat: "claude-stream-json" | "gemini-json" | "codex-json";
    authType: "oauth" | "api-key";
    installCommand: string;
    docsUrl: string;
    icon: string;
}

export const PROVIDERS: Record<string, ProviderDefinition> = {
    claude: {
        id: "claude",
        displayName: "Claude Code",
        cliCommand: "claude",
        defaultArgs: ["--output-format", "stream-json"],
        outputFormat: "claude-stream-json",
        authType: "oauth",
        installCommand: "npm install -g @anthropic-ai/claude-code",
        docsUrl: "https://docs.anthropic.com/claude-code",
        icon: "sparkles",
    },
    gemini: {
        id: "gemini",
        displayName: "Gemini CLI",
        cliCommand: "gemini",
        defaultArgs: ["--output-format", "json"],
        outputFormat: "gemini-json",
        authType: "api-key",
        installCommand: "npm install -g @anthropic-ai/gemini-cli",
        docsUrl: "https://ai.google.dev/gemini-cli",
        icon: "diamond",
    },
    codex: {
        id: "codex",
        displayName: "Codex CLI",
        cliCommand: "codex",
        defaultArgs: ["--output-format", "json"],
        outputFormat: "codex-json",
        authType: "api-key",
        installCommand: "npm install -g @openai/codex",
        docsUrl: "https://platform.openai.com/docs/codex",
        icon: "robot",
    },
};

export function getProvider(id: string): ProviderDefinition | undefined {
    return PROVIDERS[id];
}

export function getProviderList(): ProviderDefinition[] {
    return Object.values(PROVIDERS);
}
