# Mermaid Error Resilience Spec

## Problem

When a mermaid diagram fails to render, the error can crash the entire block view via React's ErrorBoundary, displaying a raw error like:

```
Error: Cannot read properties of undefined (reading 'ts')
TypeError: Cannot read properties of undefined (reading 'ts')
    at D8e (mermaid-VLURNSYL-C6PE3i19.js:297:868)
    ...
```

The error originates entirely within mermaid.js internals (v11.12.2). It can be triggered by:
- Malformed or unsupported diagram syntax
- Edge cases in mermaid's internal parser/renderer
- Race conditions during async initialization

The error propagates through React's component tree and gets caught by the block-level `ErrorBoundary` in `block.tsx`, which replaces the **entire block content** with a `<pre>` error dump. This is confusing because:
1. The error appears to come from whatever block is visible (e.g., sysinfo)
2. The actual mermaid diagram that failed may be in a different component (markdown, streamdown, AI chat)
3. The raw stack trace in minified mermaid code is useless to users

## Root Cause

Two mermaid rendering paths exist:

### Path 1: `markdown.tsx` Mermaid component (lines 71-123)
- `mermaidInstance.run({ nodes: [ref.current] })` is wrapped in try/catch
- Errors set local state: `setError("Failed to render diagram: ...")`
- **Issue**: If mermaid throws asynchronously (outside the promise chain), or if the error occurs during React render rather than in the useEffect, it escapes the try/catch and propagates to ErrorBoundary

### Path 2: `streamdown.tsx` WaveStreamdown component (lines 208-327)
- Uses `<Streamdown mermaid: true mermaidConfig={{ theme: "dark", darkMode: true }}>` from the `streamdown` npm package
- Mermaid rendering is handled internally by the streamdown library
- **No error boundary** around the Streamdown component — mermaid errors propagate directly to the parent ErrorBoundary

## Fix

### 1. Wrap Mermaid component in its own ErrorBoundary (`markdown.tsx`)

Add an ErrorBoundary around the `<Mermaid>` component so mermaid crashes are contained within the diagram area, not the entire block.

### 2. Wrap Streamdown component in an ErrorBoundary (`streamdown.tsx`)

The `<Streamdown>` component with `mermaid: true` can throw if mermaid fails internally. Wrap it in an ErrorBoundary with a graceful fallback.

### 3. Graceful error display

Instead of showing a raw stack trace, show a styled error message like:
- "Failed to render diagram" (for mermaid-specific errors)
- The original text content as a fallback so the user can still see the diagram source

## Files Changed

- `frontend/app/element/markdown.tsx` — Wrap Mermaid in ErrorBoundary
- `frontend/app/element/streamdown.tsx` — Wrap Streamdown in ErrorBoundary
