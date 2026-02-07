// Copyright 2026, a5af.
// SPDX-License-Identifier: Apache-2.0
//
// Vite configuration for Tauri development mode.
// This replaces electron-vite for the Tauri build.
// Only the renderer (frontend) configuration is needed —
// Tauri handles the "main process" in Rust.

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react-swc";
import { defineConfig } from "vite";
import { ViteImageOptimizer } from "vite-plugin-image-optimizer";
import { viteStaticCopy } from "vite-plugin-static-copy";
import svgr from "vite-plugin-svgr";
import tsconfigPaths from "vite-tsconfig-paths";

export default defineConfig({
    root: ".",
    build: {
        target: ["es2021", "chrome97", "safari13"],
        sourcemap: process.env.NODE_ENV === "development",
        outDir: "dist/frontend",
        rollupOptions: {
            input: {
                index: "index.html",
            },
            output: {
                manualChunks(id) {
                    const p = id.replace(/\\/g, "/");
                    if (p.includes("node_modules/monaco") || p.includes("node_modules/@monaco")) return "monaco";
                    if (p.includes("node_modules/mermaid") || p.includes("node_modules/@mermaid")) return "mermaid";
                    if (p.includes("node_modules/katex") || p.includes("node_modules/@katex")) return "katex";
                    if (p.includes("node_modules/shiki") || p.includes("node_modules/@shiki")) return "shiki";
                    if (p.includes("node_modules/cytoscape") || p.includes("node_modules/@cytoscape"))
                        return "cytoscape";
                    return undefined;
                },
            },
        },
    },
    optimizeDeps: {
        include: ["monaco-yaml/yaml.worker.js"],
    },
    server: {
        port: 5173,
        open: false,
        watch: {
            ignored: ["dist/**", "**/*.go", "**/go.mod", "**/go.sum", "**/*.md", "**/*.json", "emain/**", "src-tauri/**"],
        },
    },
    css: {
        preprocessorOptions: {
            scss: {
                silenceDeprecations: ["mixed-decls"],
            },
        },
    },
    plugins: [
        tsconfigPaths(),
        { ...ViteImageOptimizer(), apply: "build" },
        svgr({
            svgrOptions: { exportType: "default", ref: true, svgo: false, titleProp: true },
            include: "**/*.svg",
        }),
        react({}),
        tailwindcss(),
        viteStaticCopy({
            targets: [{ src: "node_modules/monaco-editor/min/vs/*", dest: "monaco" }],
        }),
    ],

    // Environment variable prefix for Tauri
    envPrefix: ["VITE_", "TAURI_"],
});
