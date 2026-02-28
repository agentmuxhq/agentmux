# SPEC: Frontend Bundle Size Optimization

**Date:** 2026-02-21
**Version:** 0.31.5
**Status:** Draft
**Priority:** Medium — improves portable ZIP size, startup time, and memory footprint

---

## Current State

### Portable Package
- **ZIP:** 22 MB (compressed)
- **dist/ uncompressed:** ~65.7 MB

### Size Breakdown

| Component | Size | % of dist/ | Notes |
|-----------|------|-----------|-------|
| Frontend assets/ | 25 MB | 38% | JS + CSS + KaTeX fonts |
| Monaco static copy | 16 MB | 24% | Copied from node_modules |
| Fonts (system) | 9.5 MB | 14% | Hack Nerd Mono TTF = 9.2 MB |
| Docsite | 8.4 MB | 13% | Bundled as Tauri resource |
| Binaries | 5.4 MB | 8% | agentmuxsrv-rs + wsh |
| FontAwesome | 1.4 MB | 2% | Icon fonts |
| Logos | 80 KB | <1% | |
| Schema | 24 KB | <1% | |

### Top JS Chunks (in assets/)

| Chunk | Size | gzip | Loading | Source |
|-------|------|------|---------|--------|
| shiki | 9.0 MB | 1,649 KB | **Static import** | streamdown.tsx:10 |
| ts.worker | 6.8 MB | — | Worker | Monaco TypeScript |
| mermaid (2 chunks) | 4.1 MB | 1,220 KB | **Dynamic import** | markdown.tsx:35 |
| css.worker | 1.0 MB | — | Worker | Monaco CSS |
| yamlworker | 713 KB | — | Worker | Monaco YAML |
| html.worker | 679 KB | — | Worker | Monaco HTML |
| cytoscape | 630 KB | 196 KB | Static import | mermaid dep |
| json.worker | 377 KB | — | Worker | Monaco JSON |
| katex | 260 KB | 78 KB | Static import | rehype-katex |
| editor.worker | 248 KB | — | Worker | Monaco core |
| index (app) | 27 KB | 8 KB | Entry | App code |

### Monaco Static Directory (16 MB)

Contains a **second** set of workers plus 60+ language definitions and 10 NLS locale packs:

| Subcomponent | Size | Notes |
|-------------|------|-------|
| ts.worker (duplicate) | 6.7 MB | Also in assets/ (6.8 MB) |
| NLS locale packs (10) | ~1.5 MB | ru, ja, fr, it, es, de, ko, tr, pl, pt, zh |
| 60+ language defs | ~1 MB | SQL, Python, Go, Rust, etc. |
| Editor core + themes | ~7 MB | |

### Fonts Directory (9.5 MB)

| Font | Format | Size | Used By |
|------|--------|------|---------|
| Hack Nerd Mono Regular | TTF | 2.3 MB | Terminal (xterm) |
| Hack Nerd Mono Bold | TTF | 2.3 MB | Terminal |
| Hack Nerd Mono Italic | TTF | 2.3 MB | Terminal |
| Hack Nerd Mono Bold Italic | TTF | 2.3 MB | Terminal |
| Inter Variable | woff2 | 338 KB | UI text |
| JetBrains Mono (3 weights) | woff2 | 62 KB | Code display |

---

## Prior Optimization Work

### Completed
- Manual chunk splitting for 5 heavy deps (vite.config.tauri.ts)
- Monaco lazy-loaded on idle callback (wave.ts:602)
- Mermaid dynamically imported (markdown.tsx:35)
- Image optimization plugin (vite-plugin-image-optimizer, ~63% savings)
- No source maps in production builds
- Backend rewrite: Go -> Rust (9x smaller binary, 3.6x less memory)

### Documented but not implemented
- `SPEC_PERF_NEW_WINDOW.md` — SQLite transaction batching, PRAGMA tuning
- Phase 12 production readiness — performance benchmarking infrastructure exists but not run

### Benchmark Infrastructure
- `scripts/benchmarks/measure-performance.sh` / `.ps1` — app startup + memory
- `agentmuxsrv-rs/bench.sh` + `BENCHMARK_REPORT.md` — Go vs Rust comparison

---

## Optimization Opportunities

### Phase 1: Quick Wins (no code changes, config only)

#### 1A. Convert Hack Nerd Mono TTF to WOFF2
**Savings: ~7.5 MB** (estimated 70-80% compression vs TTF)

TTF is uncompressed. WOFF2 uses Brotli compression. The 4 Hack Nerd Mono files at 2.3 MB each (9.2 MB total) would compress to ~500-600 KB each (~2.3 MB total).

```bash
# Using google/woff2 or fonttools
woff2_compress hacknerdmono-regular.ttf
woff2_compress hacknerdmono-bold.ttf
woff2_compress hacknerdmono-italic.ttf
woff2_compress hacknerdmono-bolditalic.ttf
```

Update CSS `@font-face` declarations to reference `.woff2` instead of `.ttf`.

**Risk:** Low. All modern browsers and xterm.js support WOFF2. Tauri's webview (WebView2/WebKit) supports WOFF2.

#### 1B. Remove duplicate Monaco ts.worker
**Savings: ~6.7 MB**

Both `dist/frontend/assets/ts.worker-*.js` (6.8 MB) and `dist/frontend/monaco/assets/ts.worker-*.js` (6.7 MB) exist. Only one is needed.

Investigate: does the `viteStaticCopy` of `node_modules/monaco-editor/min/vs/*` copy workers that Vite also bundles? If so, configure Vite to skip bundling Monaco workers OR skip copying them in `viteStaticCopy`.

**Risk:** Medium. Need to verify which worker path Monaco actually loads at runtime.

#### 1C. Strip unused Monaco NLS locales
**Savings: ~1.5 MB**

The app is English-only. Remove the 10 NLS locale packs (ru, ja, fr, etc.) from the static copy.

```typescript
// vite.config.tauri.ts - filter static copy
viteStaticCopy({
    targets: [{
        src: "node_modules/monaco-editor/min/vs/*",
        dest: "monaco",
        // Exclude NLS packs
        filter: (path) => !path.includes('/nls.')
    }],
}),
```

**Risk:** Low. No i18n infrastructure exists.

#### 1D. Strip unused Monaco language definitions
**Savings: ~500 KB - 1 MB**

AgentMux uses Monaco for YAML editing (config) and general code viewing. It does not need all 60+ language grammars. Keep: yaml, json, typescript, javascript, go, rust, python, shell, markdown, html, css. Remove the rest.

**Risk:** Medium. Users may paste/view code in any language. Could be done as opt-in pruning.

---

### Phase 2: Lazy-Load Shiki (code change)

#### 2A. Dynamic import for Shiki
**Impact: Removes 9.0 MB from initial bundle, loads on demand**

Shiki is the single largest chunk and is statically imported in `streamdown.tsx:10`:
```typescript
import { bundledLanguages, codeToHtml } from "shiki/bundle/web";
```

Change to dynamic import:
```typescript
let shikiModule: typeof import("shiki/bundle/web") | null = null;

async function getShiki() {
    if (!shikiModule) {
        shikiModule = await import("shiki/bundle/web");
    }
    return shikiModule;
}
```

The `CodeHighlight` component already uses `useEffect` + `useState` to render highlighted code, so it's already async-ready. The change is small.

**Additionally:** Consider using `shiki/bundle/web` with a custom language subset instead of all bundled languages. The `bundledLanguages` object includes 100+ grammars. Most users need <20.

```typescript
import { createHighlighter } from "shiki";

const highlighter = await createHighlighter({
    themes: ["github-dark-high-contrast"],
    langs: ["javascript", "typescript", "python", "rust", "go", "bash",
            "json", "yaml", "html", "css", "markdown", "sql", "toml"],
});
```

**Estimated savings from language subsetting:** 3-5 MB additional reduction.

**Risk:** Low. Code highlighting appears after content renders anyway. Users won't notice the ~50ms async load.

---

### Phase 3: Reduce Mermaid Bundle

#### 3A. Use mermaid/dist/mermaid.min.js directly
**Savings: Potentially 1-2 MB**

Mermaid is split into two chunks totaling 4.1 MB. The library includes diagram types that may not be used (Gantt, Git graph, Journey, etc.). Mermaid supports modular registration of diagram types.

```typescript
import mermaid from "mermaid/dist/mermaid.core.min";
import flowchart from "@mermaid-js/mermaid-flowchart-v2";
import sequence from "@mermaid-js/mermaid-sequence-diagram";
// Register only needed diagrams
mermaid.registerDiagram(flowchart);
mermaid.registerDiagram(sequence);
```

**Risk:** Medium. Need to audit which diagram types users actually render. Mermaid's modular API changed across versions.

---

### Phase 4: Evaluate Feature Removal

#### 4A. Audit docsite necessity in portable package
**Savings: 8.4 MB**

The docsite is bundled as a Tauri resource (`src-tauri/tauri.conf.json` line 40). Is it actually rendered in-app, or is it legacy from upstream WaveTerm?

If the docs are accessible via a URL, remove from the bundle and link externally.

**Risk:** Need to verify if any in-app help/docs UI references local docsite files.

#### 4B. Audit FontAwesome usage
**Savings: up to 1.4 MB**

Is the full FontAwesome icon set needed, or can it be subset to only used icons?

Tools like `fonttools` or PurgeCSS can identify used glyphs.

#### 4C. Audit KaTeX font formats
**Savings: ~700 KB**

KaTeX includes fonts in 3 formats: woff2, woff, and ttf (72 files, 1 MB total). Modern browsers only need woff2. Remove ttf and woff fallbacks.

**Risk:** Low. Tauri WebView2 and WebKit both support woff2.

---

### Phase 5: Build Infrastructure

#### 5A. Add bundle size tracking to CI
```bash
# Add to build pipeline
du -sb dist/frontend/assets/*.js | sort -rn > bundle-sizes.txt
# Compare against baseline, fail if >10% regression
```

#### 5B. Add rollup-plugin-visualizer
```bash
npm install -D rollup-plugin-visualizer
```

Add to vite.config.tauri.ts to generate treemap HTML on each build for visual size analysis.

#### 5C. Implement size budgets in Vite
```typescript
build: {
    chunkSizeWarningLimit: 500, // Currently defaults to 500 KB, many chunks exceed this
}
```

---

## Estimated Impact Summary

| Optimization | Effort | Savings | Risk |
|-------------|--------|---------|------|
| 1A: WOFF2 fonts | 1 hour | ~7.5 MB | Low |
| 1B: Deduplicate ts.worker | 2 hours | ~6.7 MB | Medium |
| 1C: Strip NLS locales | 30 min | ~1.5 MB | Low |
| 1D: Strip language defs | 1 hour | ~0.5-1 MB | Medium |
| 2A: Lazy-load Shiki | 1 hour | 9.0 MB (deferred) | Low |
| 2A+: Shiki lang subset | 2 hours | 3-5 MB | Low |
| 3A: Mermaid modular | 3 hours | 1-2 MB | Medium |
| 4A: Remove docsite | 1 hour | 8.4 MB | Needs audit |
| 4B: Subset FontAwesome | 2 hours | up to 1.4 MB | Low |
| 4C: KaTeX font cleanup | 30 min | ~0.7 MB | Low |
| **Total potential** | | **~35-40 MB** | |

### Priority Order (ROI)
1. **1A** (WOFF2 fonts) — biggest bang, simplest change
2. **2A** (Lazy Shiki) — 9 MB off initial load, small code change
3. **1B** (Dedupe ts.worker) — 6.7 MB, needs investigation
4. **4A** (Docsite audit) — 8.4 MB if removable
5. **1C** (NLS locales) — easy 1.5 MB
6. **4C** (KaTeX fonts) — easy 0.7 MB
7. Everything else

---

## Baseline Measurements

Run before implementing any changes to establish baseline:

```bash
# Total dist size
du -sh dist/

# Frontend assets
du -sh dist/frontend/assets/ dist/frontend/monaco/ dist/frontend/fonts/

# Portable ZIP
ls -lh agentmux-*-portable.zip

# Individual chunks
ls -lhS dist/frontend/assets/*.js | head -15
```

Record in `BENCHMARK_REPORT_FRONTEND.md` for tracking.
