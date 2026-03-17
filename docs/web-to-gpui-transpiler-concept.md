# Web-to-GPUI Transpiler: Concept & Architecture

**Date:** 2026-03-11
**Status:** Concept / Research
**Author:** AgentY

---

## The Core Idea

A compile-time transpiler that takes **React/TSX + Tailwind CSS** as source and emits **Rust GPUI code** as output. The browser's work — layout, style resolution, DOM diffing — happens once at build time. What ships is a native binary with zero browser overhead: just GPUI primitive draw calls executing at 120fps on Metal/DX11/Vulkan.

The philosophical ancestor is **Svelte**: a compiler that makes the framework disappear at build time. Svelte compiles HTML templates to imperative DOM calls; this compiles React components to GPUI element calls. The framework is not shipped. The runtime is not shipped. Only the output is shipped.

This is not a runtime HTML renderer (that's Ultralight, that's Blink). This is a source-to-source compiler where the source language is the web stack and the target language is GPUI.

---

## Why Tailwind Makes This Tractable

GPUI's styling API is **intentionally named after Tailwind**. This is the single most important fact for feasibility:

| Tailwind class | GPUI method |
|---|---|
| `flex` | `.flex()` |
| `flex-col` | `.flex_col()` |
| `items-center` | `.items_center()` |
| `justify-between` | `.justify_between()` |
| `p-4` | `.p_4()` |
| `px-2` | `.px_2()` |
| `gap-2` | `.gap_2()` |
| `w-full` | `.w_full()` |
| `rounded-lg` | `.rounded_lg()` |
| `border` | `.border()` |
| `flex-1` | `.flex_1()` |
| `flex-grow` | `.flex_grow()` |
| `text-sm` | `.text_sm()` |
| `bg-blue-500` | `.bg(blue_500())` |
| `text-white` | `.text_color(white())` |
| `shadow-md` | `.shadow_md()` |

This mapping is essentially a lookup table. An atomic CSS class has exactly one property with one value — no cascade, no specificity, no inheritance to resolve. Every Tailwind class in the source maps to exactly one GPUI method call in the output.

Non-Tailwind CSS (custom classes, CSS-in-JS) breaks this. The transpiler's tractable surface area is precisely: **React + Tailwind**. Which is also the most common modern React codebase pattern.

---

## Pipeline Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        SOURCE                                │
│            React TSX + Tailwind CSS classes                  │
└───────────────────────────┬─────────────────────────────────┘
                            │
                    Stage 1: Parse
                            │
               ┌────────────▼────────────┐
               │  OXC (Rust)             │
               │  TSX → typed AST        │
               │  TypeScript types       │
               │  Tailwind class extract │
               └────────────┬────────────┘
                            │
                    Stage 2: CSS Resolution
                            │
               ┌────────────▼────────────┐
               │  Tailwind Resolver      │
               │  class → property map   │
               │  compute-time only      │
               │  dynamic cn() flagged   │
               └────────────┬────────────┘
                            │
                    Stage 3: JSX Lowering
                            │
               ┌────────────▼────────────┐
               │  Element Mapper         │
               │  <div> → div()          │
               │  <span> → div()         │
               │  <p> → div() + text     │
               │  <img> → img()          │
               │  <svg> → svg()          │
               │  text nodes → strings   │
               └────────────┬────────────┘
                            │
                    Stage 4: Style Emission
                            │
               ┌────────────▼────────────┐
               │  GPUI Style Builder     │
               │  Tailwind → .method()   │
               │  conditional → match    │
               │  responsive → skipped   │
               └────────────┬────────────┘
                            │
                    Stage 5: State Transform
                            │
               ┌────────────▼────────────┐
               │  Hook Lifter            │
               │  useState → Entity<T>   │
               │  useEffect → subscribe  │
               │  useMemo → derived      │
               │  Jotai atoms → Entity   │
               └────────────┬────────────┘
                            │
                    Stage 6: Event Binding
                            │
               ┌────────────▼────────────┐
               │  Event Mapper           │
               │  onClick → on_click     │
               │  onKeyDown → on_key_down│
               │  onChange → on_input    │
               │  onHover → on_hover     │
               └────────────┬────────────┘
                            │
                    Stage 7: Type Resolution
                            │
               ┌────────────▼────────────┐
               │  TS → Rust Type Mapper  │
               │  string → String/&str   │
               │  number → f32/usize     │
               │  boolean → bool         │
               │  T | null → Option<T>   │
               │  any → flagged manual   │
               └────────────┬────────────┘
                            │
                    Stage 8: Rust Emission
                            │
               ┌────────────▼────────────┐
               │  Code Generator         │
               │  Rust source output     │
               │  gpui crate imports     │
               │  impl Render trait      │
               └────────────┬────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                        OUTPUT                                │
│         Rust GPUI components — pure GPU primitives           │
│         No browser. No layout engine. No DOM.                │
└─────────────────────────────────────────────────────────────┘
```

---

## Stage-by-Stage Detail

### Stage 1: Parse — OXC

**Tool:** [OXC](https://github.com/oxc-project/oxc) — Rust-native TSX/JSX parser
**Why OXC over SWC:** 3x faster, 30% less memory, 2MB vs 37MB package size. Arena-allocated AST (bumpalo), no heap churn. Already production-ready for `oxc_ast` and `oxc_parser`. Rolldown, Nuxt, Vue are already using it.

OXC produces a typed AST that distinguishes `BindingIdentifier`, `IdentifierReference`, and `IdentifierName` — not a generic estree `Identifier`. This precision matters for the type resolution stage.

**What it handles:** TSX, JSX, TypeScript annotations, Stage 3 decorators, latest ECMAScript.

**Output:** Typed OXC AST + extracted Tailwind class strings per element.

---

### Stage 2: CSS Resolution — Tailwind Resolver

Tailwind is run at compile time (it already does this for dead-code elimination). The resolver maps each class name to its CSS property/value pair:

```
p-4         → { padding: 16px }
flex        → { display: flex }
items-center → { align-items: center }
bg-blue-500 → { background-color: rgb(59, 130, 246) }
```

**Dynamic classes** — the transpiler's first hard boundary:

```tsx
// STATIC — fully resolvable at compile time
<div className="flex p-4 bg-blue-500">

// CONDITIONAL — partially resolvable
<div className={cn("flex", isActive && "bg-blue-500")}>
// → emits a match arm in Rust

// DYNAMIC — must be flagged for manual porting
<div className={buildClassName(props)}>
// → cannot resolve, emits a TODO comment + manual bridge marker
```

The `cn()` / `clsx()` pattern is common and solvable: both branches are known Tailwind classes, resolvable at compile time into a Rust conditional expression. Fully dynamic string building is the only case that escapes the resolver.

---

### Stage 3: JSX Lowering — Element Mapper

JSX elements map to GPUI elements. GPUI's primary element is `div()`. Everything in HTML is either a div or one of the 5 GPUI primitives.

```tsx
// Source
<div className="flex flex-col gap-4">
  <span>Hello</span>
  <img src={icon} />
</div>

// Output
div()
  .flex()
  .flex_col()
  .gap_4()
  .child(
    div().child("Hello")
  )
  .child(
    img().source(ImageSource::from(icon))
  )
```

**Element mapping table:**

| HTML/JSX | GPUI | Notes |
|---|---|---|
| `<div>` | `div()` | Direct |
| `<span>` | `div()` | Inline → block, acceptable |
| `<p>` | `div().child("text")` | Paragraph → div |
| `<button>` | `div().on_click(...)` | No native button, add handler |
| `<input>` | Custom element | Requires GPUI input component |
| `<img>` | `img()` | Direct |
| `<svg>` | `svg()` | Direct if simple |
| `<a>` | `div().on_click(open_link)` | Link → click handler |
| `{text}` | `"text"` / `.child(text)` | String literals inline |
| `<Fragment>` | `div()` with children | Flatten |

---

### Stage 4: Style Emission

The Tailwind class list from Stage 2 gets chained as GPUI style method calls:

```rust
// Input: className="flex flex-col items-center gap-4 p-6 rounded-lg bg-zinc-900"
div()
    .flex()
    .flex_col()
    .items_center()
    .gap_4()
    .p_6()
    .rounded_lg()
    .bg(gpui::black())  // zinc-900 → color constant
```

**Color mapping:** Tailwind's full color palette (zinc, blue, red, etc. with 50–950 scales) maps to `rgb()` calls with known hex values. The full mapping is a static lookup table — 500 entries, no computation required.

**Responsive prefixes (`sm:`, `md:`, `lg:`):** GPUI has no media query equivalent. These are flagged as manual. For AgentMux specifically, responsive breakpoints are rare (it's a desktop app), so this is a low-frequency escape hatch.

**Pseudo-classes (`hover:`, `focus:`, `active:`):**
```tsx
// Source
<div className="bg-zinc-800 hover:bg-zinc-700">

// Output
div()
    .bg(zinc_800())
    .hover(|style| style.bg(zinc_700()))
// GPUI supports .hover() modifier — direct mapping
```

---

### Stage 5: State Transform — Hook Lifter

This is the pipeline's hardest stage. React hooks and GPUI entities are conceptually aligned (both are reactive) but structurally different.

**`useState` → GPUI Entity field:**

```tsx
// Source
function Counter() {
  const [count, setCount] = useState(0);
  return <button onClick={() => setCount(c => c + 1)}>{count}</button>;
}

// Output
struct Counter {
    count: i32,
}

impl Counter {
    fn new(cx: &mut Context<Self>) -> Self {
        Self { count: 0 }
    }

    fn increment(&mut self, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }
}

impl Render for Counter {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .on_click(cx.listener(|this, _, cx| this.increment(cx)))
            .child(self.count.to_string())
    }
}
```

**`useEffect` → GPUI subscription:**
```tsx
// Source
useEffect(() => {
  const sub = dataStream.subscribe(handler);
  return () => sub.unsubscribe();
}, [dataStream]);

// Output (in Entity constructor)
cx.observe(&data_stream_handle, |this, handle, cx| {
    // handler body
});
// Drop of subscription is handled by GPUI's Subscription type
```

**Jotai atoms → GPUI entities:**
Jotai's atom graph maps reasonably to GPUI's entity model. A `atom(initialValue)` becomes an `Entity<T>`. `useAtom` becomes `cx.observe()`. The main difference is Jotai allows cross-component atom sharing via module scope; GPUI requires explicit entity handle passing. The transpiler emits entity handles as constructor arguments, mirroring dependency injection.

**What can't be auto-transformed:**
- `useReducer` with complex action types → emits a TODO + manual bridge
- `useContext` for deep prop injection → requires architectural decision
- `useRef` for DOM manipulation → no DOM in GPUI, flag for manual porting
- `useCallback` / `useMemo` → mostly eliminable, optimizer handles this

---

### Stage 6: Event Binding

```tsx
// Source
<div
  onClick={handleClick}
  onKeyDown={handleKey}
  onMouseEnter={() => setHovered(true)}
>

// Output
div()
  .on_click(cx.listener(|this, event: &ClickEvent, cx| {
      this.handle_click(event, cx);
  }))
  .on_key_down(cx.listener(|this, event: &KeyDownEvent, cx| {
      this.handle_key(event, cx);
  }))
  .on_hover(|hovered, cx| {
      // hovered: bool
  })
```

**Event mapping table:**

| React event | GPUI equivalent |
|---|---|
| `onClick` | `.on_click()` |
| `onMouseEnter` / `onMouseLeave` | `.on_hover()` |
| `onKeyDown` / `onKeyUp` | `.on_key_down()` / `.on_key_up()` |
| `onChange` (input) | Custom input element callback |
| `onFocus` / `onBlur` | `.on_focus_in()` / `.on_focus_out()` |
| `onScroll` | `.on_scroll()` |

---

### Stage 7: TypeScript → Rust Type Mapping

The most fragile stage. TypeScript is structurally typed and gradually typed; Rust is nominally typed with strict ownership.

**Mechanical mappings (automatable):**

| TypeScript | Rust |
|---|---|
| `string` | `SharedString` (GPUI type) or `String` |
| `number` | `f64` (default) or `i32`/`usize` with annotation |
| `boolean` | `bool` |
| `T \| null` | `Option<T>` |
| `T \| undefined` | `Option<T>` |
| `T[]` | `Vec<T>` |
| `Record<string, T>` | `HashMap<String, T>` |
| `() => void` | `Box<dyn Fn()>` or GPUI listener |
| `Promise<T>` | `Task<T>` (GPUI async) |

**Flagged for manual resolution:**

| TypeScript | Problem |
|---|---|
| `any` | No Rust equivalent — emit `/* TODO: type */ ()` |
| Union types beyond `T \| null` | Requires enum definition |
| Structural interface types | Requires struct + trait definition |
| Prototype methods | Requires impl block |
| Closures over mutable state | Borrow checker — manual review required |
| Async generators | No direct equivalent |

The `any` escape hatch is common in real TypeScript codebases. Every `any` becomes a manual bridge marker. High `any` density in a component = low transpiler confidence score for that component.

---

### Stage 8: Code Generation

The emitter walks the transformed IR and writes Rust source:

```rust
// Generated file header
// AUTO-GENERATED by web-to-gpui transpiler
// Source: frontend/app/view/agent/agent-view.tsx
// Confidence: 87% (3 manual bridges required)
// Manual: see TODO comments tagged [BRIDGE]

use gpui::prelude::*;
use gpui::{div, img, InteractiveElement, ParentElement, Styled};

pub struct AgentView {
    agent_id: EntityId,
    // ... fields from useState hooks
}

impl AgentView {
    pub fn new(cx: &mut Context<Self>) -> Self { ... }
}

impl Render for AgentView {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            // ... generated element tree
    }
}
```

Each generated file includes a **confidence score** — the percentage of the component that was automatically transpiled vs. flagged for manual work. Low-confidence files are good candidates for human review prioritization.

---

## Prior Art

### Svelte — The Philosophical Model
**What it is:** Compile-time JS framework. Templates → imperative DOM calls. Framework ships zero runtime bytes.
**What we take:** The compile-time-not-runtime philosophy. The idea that the source language (HTML-like templates) can compile away into the target language (JS DOM calls) with no runtime overhead.
**Where it stops:** Svelte targets the DOM. It doesn't cross the DOM/native boundary. This transpiler does.

### Makepad — The GPU Shader Analog
**What it is:** Rust UI framework where the UI DSL compiles to GPU shaders. No DOM. Makepad's creator (Rik Arends) spent 6 years rewriting browser rendering in WebGL before abandoning HTML entirely.
**What we take:** Proof that "compile UI description → GPU calls" is buildable and shippable. Makepad 1.0 shipped in 2025.
**Where it stops:** Makepad uses its own DSL (not React/Tailwind). It's not a transpiler from existing web code. The source language is Makepad-native.
**Repo:** [github.com/makepad/makepad](https://github.com/makepad/makepad)

### Ultralight — The GPU Command List Model
**What it is:** Lightweight WebKit fork for game embedding. Instead of Skia CPU rasterization, emits a `GPUDriver` command list (abstract draw calls) plugged into D3D11/Metal/OpenGL.
**What we take:** The `GPUDriver` abstraction — the idea that HTML/CSS layout can produce an abstract GPU command list. This is architecturally what a web→GPUI transpiler would produce, except at compile time rather than runtime.
**Where it stops:** Still a runtime engine. Parses HTML/CSS at runtime. Carries a WebKit layout engine (~50MB). Not a transpiler.
**URL:** [ultralig.ht](https://ultralig.ht)

### Ekioh Flow — The GPU-First Browser
**What it is:** Browser engine with a "GPU first" policy. HTML/CSS layout resolves to triangle batches sent to GPU in one draw call. Nearly all operations batched together — extremely fast on embedded/TV hardware.
**What we take:** Proof that CSS layout can be compiled into GPU triangle batches without a Skia/Chromium-style CPU rasterizer in the path. The rendering model is closer to GPUI than to Chrome.
**Where it stops:** Runtime, not compile-time. Still a full browser engine.
**URL:** [ekioh.com/devblog/gpu-rendering](https://www.ekioh.com/devblog/gpu-rendering/)

### Dioxus — The RSX Model
**What it is:** React-inspired Rust framework. RSX macro (JSX-like) → native rendering via WebView or WGPU renderer. v0.6 shipped December 2024.
**What we take:** Proof that a JSX-like syntax in Rust compiling to native GPU output is viable and maintainable. Shows the RSX→GPUI element pipeline architecture.
**Where it stops:** Not a transpiler from existing React code. Requires rewriting in RSX. No CSS mapping.
**URL:** [dioxuslabs.com](https://dioxuslabs.com)

### Vello — The Alternative GPU Target
**What it is:** Rust/wgpu 2D vector renderer from Linebender. Compute-shader based — all 2D drawing (paths, text, gradients, SVG) encoded as GPU compute operations.
**What we take:** A more capable GPU target than GPUI's 5 primitives. Vello handles gradients, arbitrary paths, and SVG — things GPUI cannot. A web→native transpiler could target Vello instead of (or alongside) GPUI for the rendering layer.
**Why it matters:** CSS has gradients, filters, and clip-paths that GPUI cannot express. Vello can. Using Vello as the render backend would dramatically increase the CSS surface area the transpiler can handle automatically.
**URL:** [github.com/linebender/vello](https://github.com/linebender/vello)

### OXC — The Parser
**What it is:** Rust-native JS/TS/JSX/TSX parser and transformer. 3x faster than SWC, 5x faster than Biome. 2MB vs SWC's 37MB. Arena-allocated AST. Production-ready as of 2024.
**What we take:** The parser stage. OXC becomes stage 1 of the pipeline. Already used by Rolldown, Nuxt, Vue.
**URL:** [github.com/oxc-project/oxc](https://github.com/oxc-project/oxc)

---

## The "Building a Browser" Problem — Narrowed

The general form of this problem is: "build a browser." A full-spec browser (Blink) is ~25 million lines of code, built over decades, handling every CSS property, every HTML element, every JavaScript quirk.

The narrowed form is different:

| Scope | Browser (Blink) | This Transpiler |
|---|---|---|
| CSS properties | All ~500 | Tailwind subset ~60 used in practice |
| HTML elements | All ~140 | ~10 used in typical React component |
| Layout modes | 6 (flex, grid, block, inline, table, absolute) | 1 (flex only, via Taffy) |
| JavaScript | Full runtime (V8) | Compile-time type analysis only |
| Runtime | Always running | Once at build time |
| Dynamic content | Arbitrary | Flagged as manual bridge |

The transpiler does not need to handle the full CSS spec. It needs to handle the CSS actually present in the codebase — which for a Tailwind project is a finite, known, enumerable set of classes. The scope is a small fraction of the browser problem.

What remains hard is not CSS breadth but **TypeScript→Rust semantic translation**: ownership, lifetimes, the borrow checker, and the absence of `null`/`undefined` as first-class values. Every JavaScript `any`, every mutable closure, every runtime-type-checked union type is a manual bridge.

---

## Confidence Classification System

Each transpiled component gets a confidence score:

| Score | Meaning | Action |
|---|---|---|
| 90–100% | Fully automatic | Ship as-is |
| 70–89% | Minor manual bridges | Review TODO comments |
| 50–69% | Significant manual work | Use as scaffold only |
| < 50% | Architecture mismatch | Manual port recommended |

**High confidence components** (typical in AgentMux):
- Static layout panels (tab bars, sidebars, headers)
- Pure display components (status badges, icons, labels)
- Simple interactive elements (buttons, toggles)

**Low confidence components** (require manual work):
- Terminal pane — xterm.js has no GPUI equivalent, requires native PTY renderer
- Sysinfo charts — Observable Plot (SVG) → needs Vello or custom path rendering
- Agent markdown view — streaming markdown with code highlighting → custom GPUI element
- Webview pane — no browser embedding in GPUI, architectural removal required

---

## What the Transpiler Would NOT Handle (Hard Boundaries)

These require manual porting regardless of transpiler sophistication:

1. **xterm.js terminal** — a complete terminal emulator. No GPUI equivalent exists. Would need `alacritty-terminal` crate + custom PTY renderer in GPUI.

2. **Observable Plot / SVG charts** — SVG rendering is outside GPUI's 5 primitives. Target: Vello (Linebender), which handles arbitrary 2D vector graphics.

3. **Monaco / shiki** — code display with syntax highlighting. GPUI has no syntax highlighter. Zed built their own (tree-sitter based). That's the model: `tree-sitter` crate + custom glyph tinting.

4. **Mermaid diagrams** — SVG-based. Same as charts — Vello target.

5. **WebSocket/RPC layer** — the `ws.ts`, `wshclient.ts`, `wshrpcutil.ts` files are AgentMux's data layer. These transpile to Rust `tokio` + `axum` code, which is already where `agentmuxsrv-rs` lives. This is actually a simplification, not a problem.

6. **Any DOM imperative API** (`document.createElement`, `ref.current.*`) — flags as manual bridge.

---

## Implementation Sketch (What Building This Looks Like)

```
web-to-gpui/
├── crates/
│   ├── parser/         # OXC wrapper, TSX→IR
│   ├── css-resolver/   # Tailwind class → property map
│   ├── jsx-lowerer/    # JSX AST → Element IR
│   ├── style-emitter/  # CSS properties → GPUI method chains
│   ├── state-lifter/   # React hooks → GPUI entity model
│   ├── event-binder/   # DOM events → GPUI events
│   ├── type-mapper/    # TypeScript types → Rust types
│   ├── codegen/        # IR → Rust source text
│   └── confidence/     # Manual bridge detection + scoring
├── data/
│   ├── tailwind-map.json     # Tailwind class → GPUI method (lookup table)
│   ├── color-map.json        # Tailwind colors → rgb() values
│   └── element-map.json      # HTML elements → GPUI elements
└── cli/
    └── main.rs         # transpile <src-dir> --out <dest-dir>
```

**Phased build:**
1. **Phase 1:** Parser + CSS resolver + static layout components only (no state, no events). Get static UI panels transpiling correctly. Validate fidelity.
2. **Phase 2:** Event binding + simple `useState` → entity. Get interactive components working.
3. **Phase 3:** Complex state (effects, subscriptions, Jotai atoms).
4. **Phase 4:** Type mapper hardening. Reduce manual bridge rate.
5. **Phase 5:** Vello integration for SVG/charts/gradients.

Phase 1 alone would cover a meaningful portion of AgentMux's UI — all the static chrome, tab bars, status panels, icon rows.

---

## Verdict

This is not a browser. A browser runs HTML/CSS at runtime against an arbitrary document model. This is a compiler that processes a specific, known React + Tailwind codebase once at build time and emits GPU draw calls.

The scope is bounded. The Tailwind → GPUI style mapping is a lookup table. The JSX → GPUI element tree is mechanical. The hard parts are TypeScript → Rust type translation and the handful of components with no GPUI primitive equivalent (terminal, charts).

No equivalent tool exists today. The closest are: Makepad (GPU-native, own DSL), Ultralight (runtime WebKit → GPU commands), Dioxus (RSX in Rust, not a transpiler). The compile-time-from-web-source approach is genuinely novel.

Whether it's worth building depends on whether the performance gains of native GPU rendering justify the migration cost — and for AgentMux specifically, whether agent monitoring at scale will eventually make Chromium's overhead a real problem rather than a theoretical one.

---

## Sources

- [Makepad GitHub](https://github.com/makepad/makepad) — GPU shader UI framework, Rust
- [Ekioh GPU Rendering Devblog](https://www.ekioh.com/devblog/gpu-rendering/) — GPU-first HTML/CSS browser engine
- [Ultralight Architecture](https://docs.ultralig.ht/docs/architecture) — lightweight WebKit → GPU command list
- [Dioxus](https://dioxuslabs.com/blog/introducing-dioxus/) — RSX → native Rust UI framework
- [OXC Project](https://github.com/oxc-project/oxc) — Rust-native TSX parser, 3x faster than SWC
- [Vello](https://github.com/linebender/vello) — Rust/wgpu 2D vector GPU renderer
- [Zed GPUI Blog](https://zed.dev/blog/videogame) — GPUI architecture and rendering philosophy
- [GPUI Community Edition](https://github.com/gpui-ce/gpui-ce) — GPUI styling API documentation
- [Svelte Compiler](https://daily.dev/blog/svelte-compiler-how-it-works) — compile-time framework model
- [What is Blink](https://developer.chrome.com/docs/web-platform/blink) — Chrome rendering engine
