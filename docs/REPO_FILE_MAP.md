# AgentMux Repository File Map

**Generated:** 2026-02-21
**Version:** 0.31.9

A one-line description of every source file in the repository.

---

## src-tauri/src/ (Rust — Tauri Application)

### Core
- main.rs — Wrapper that calls agentmux_lib::run(); suppresses Windows console window
- lib.rs — Tauri application entry point; initializes plugins, state, commands, backend sidecar, logging, crash handler, heartbeat
- state.rs — Shared application state (AppState); manages auth key, backend endpoints, zoom factor, client/window/tab IDs
- sidecar.rs — Spawns agentmuxsrv-rs backend as subprocess; manages endpoints file for multi-window reuse
- menu.rs — Builds and handles application menus (File, Edit, View, Workspace, Window); zoom controls; devtools toggle
- tray.rs — System tray icon and menu; show/hide window on click
- crash.rs — Panic hook for crash reporting; writes timestamped crash logs to app data directory
- heartbeat.rs — Periodic heartbeat writer (every 5 seconds); allows external tools to detect if app is running

### commands/
- mod.rs — Module exports for all command submodules
- auth.rs — get_auth_key command; returns auth key for backend communication
- platform.rs — Platform queries: OS, user, hostname, dev mode, data/config dirs, environment variables, about modal details
- window.rs — Window management: open new window, zoom, cursor position, close, minimize, maximize, focus
- backend.rs — Backend communication: get endpoints, get wave init options, frontend logging
- devtools.rs — DevTools toggle and status check
- contextmenu.rs — Context menu builder from JSON; supports separators, checkboxes, regular items
- stubs.rs — Placeholder commands for future features: download, quicklook, workspace/tab management
- providers.rs — CLI provider management: detect installed CLIs (claude, gemini, codex); store/retrieve provider config
- claudecode.rs — Legacy stubs for Claude Code auth (now handled by `claude auth login`)

---

## frontend/ (TypeScript/React — UI)

### Root Files
- wave.ts — Main application initialization: global state, RPC, object loading, event subscriptions, font loading, React render
- tauri-bootstrap.ts — Tauri bootstrap entry point with verbose logging; checks for backend startup errors; imports wave.ts
- tauri-init.ts — Tauri API initialization; pre-fetches cached values and installs API shim on window.api

### types/
- custom.d.ts — Global TypeScript types for UI components, Jotai atoms, ViewModels, AppApi interface, WaveObj structures
- gotypes.d.ts — Auto-generated Go backend type definitions for RPC messages, blocks, connections, metadata, config
- jsx.d.ts — JSX runtime reference type definition
- media.d.ts — Vite asset module declarations for images, videos, audio, fonts
- vite-env.d.ts — Vite client environment type definitions for CSS modules and web workers

### util/
- endpoints.ts — Lazy-loaded helpers to get web and WebSocket server endpoints from environment variables
- fetchutil.ts — Wrapper around fetch API for network requests with CORS handling via Tauri
- focusutil.ts — DOM utilities to find focused elements, extract block IDs, detect text selection
- fontutil.ts — Loads custom fonts (JetBrains Mono, Hack Nerd Font, Inter) into FontFaceSet at runtime
- getenv.ts — Retrieves environment variables from host process via window globals or Tauri IPC
- historyutil.ts — Manages navigation history stacks (back/forward) for file paths and URLs
- ijson.ts — Incremental JSON path manipulation library for setting/getting/deleting values via path arrays
- isdev.ts — Lazy-loaded functions to detect development builds and Vite dev server mode
- keyutil.ts — Comprehensive keyboard event handling with cross-platform modifier mapping
- notification.ts — Tauri-based native OS notification system with fallback for in-app notifications
- platformutil.ts — Platform detection utilities (macOS vs Windows) and native file manager labels
- sharedconst.ts — Shared constant for keyboard chord timeout (2000ms)
- tauri-api.ts — Tauri API shim implementing AppApi interface using Tauri invoke/listen
- util.ts — General utilities: base64, string validation, deep equality, icon classes, Jotai atom helpers
- waveutil.ts — CSS URL processing for backgrounds and remote path to web server URL conversion
- wsutil.ts — WebSocket abstraction layer supporting Node.js and browser WebSocket

---

## frontend/app/ (React Components)

### app.tsx / app-bg.tsx
- app.tsx — Main app component with providers and key handlers
- app-bg.tsx — Background styling component

### aipanel/ (Wave AI Chat)
- agentai-focus-utils.ts — Utility functions for detecting focus state within AI panel DOM
- agentai-model.tsx — Singleton model managing AI chat state, file uploads, and message handling
- ai-utils.ts — Helper functions for file validation, mime-type normalization, image resizing
- aidroppedfiles.tsx — Dropped/uploaded files display as thumbnail chips with remove buttons
- aifeedbackbuttons.tsx — Thumbs up/down/copy feedback UI for AI responses
- aimessage.tsx — Renders individual AI and user messages with tool use groups and file attachments
- aipanel.tsx — Main AI panel with chat interface, drag-drop file handling, and rate limiting
- aipanelheader.tsx — Header bar with title, widget context toggle, and menu button
- aipanelinput.tsx — Textarea input with file upload button and send functionality
- aipanelmessages.tsx — Container managing message list auto-scroll and message rendering
- airatelimitstrip.tsx — Rate limit warning strip when premium/basic quota is low
- aitooluse.tsx — Tool use display (file reads, etc.) with approval buttons
- aitypes.ts — TypeScript type definitions for Wave UI messages and message parts
- telemetryrequired.tsx — Telemetry opt-in message shown before using Wave AI

### block/ (Block/Pane System)
- autotitle.test.ts — Test suite for auto-title generation from block metadata
- autotitle.ts — Generates block titles from cwd, URLs, filenames with agent detection
- block-model.ts — Model for managing block highlights and visual state
- block.tsx — Core block rendering component with registry system for view types
- blockframe.tsx — Frame wrapper with header, title bar, connection button, and content area
- blocktypes.ts — TypeScript interfaces for block props and component models
- blockutil.tsx — Utility functions for block icons, colors, title parsing, and connection status
- titlebar.tsx — Editable pane title bar with icon and color customization

### element/ (Reusable UI Components)
- ansiline.tsx — ANSI color code parser and renderer for terminal text
- avatar.tsx — User avatar with initials or image and online/offline status
- button.tsx — Reusable button component with solid/outline/ghost variants
- collapsiblemenu.tsx — Hierarchical collapsible menu with nested items
- copybutton.tsx — Button that copies text and shows "copied" feedback
- donutchart.tsx — Recharts donut/pie chart with center label
- emojibutton.tsx — Button showing emoji/icon with floating animation on click
- emojipalette.tsx — Emoji picker with search and grid display
- errorboundary.tsx — React error boundary for error containment
- expandablemenu.tsx — Expandable menu with controlled/uncontrolled group state
- flyoutmenu.tsx — Hierarchical flyout menu with submenu positioning
- iconbutton.tsx — Simple icon button with spin animation support
- input.tsx — Controlled input with optional left/right elements
- linkbutton.tsx — Styled anchor tag as button
- magnify.tsx — Icon indicating magnified block state
- markdown.tsx — React Markdown renderer with syntax highlighting, mermaid, custom plugins
- markdown-contentblock-plugin.ts — Remark plugin for custom wave content blocks
- markdown-util.ts — Utilities for markdown block transformation and remote file resolution
- menubutton.tsx — Button that opens a flyout menu
- modal.tsx — Modal dialog with backdrop and content sections
- multilineinput.tsx — Textarea with auto-expand on content and max-rows limit
- notification.tsx — Notification/toast component
- popover.tsx — Floating UI popover with placement and dismissal
- progressbar.tsx — Horizontal progress bar with percentage label
- quickelems.tsx — Simple centered divs for loading and empty states
- quicktips.tsx — Comprehensive help guide with keybindings, wsh commands, and tips
- remark-mermaid-to-tag.ts — Remark plugin converting code blocks to mermaid HTML tags
- search.tsx — Search UI with floating position and regex/case-sensitive options
- streamdown.tsx — Markdown renderer using Streamdown library with code syntax highlighting (lazy-loads shiki)
- toggle.tsx — Checkbox styled as toggle switch
- tooltip.tsx — Floating tooltip with hover and force-open support
- typingindicator.tsx — Three-dot typing animation
- windowdrag.tsx — Container for draggable window regions
- zoomindicator.tsx — Displays current zoom percentage

### hook/
- useDimensions.tsx — Custom hooks for element resize observation with debouncing
- useLongClick.tsx — Hook for detecting long-click vs short-click on elements

### menu/
- base-menus.ts — Factory functions for creating context menus (tab bar, widgets)
- menu-builder.ts — Builder class for composing context menus with separators and submenus

### modals/
- about.tsx — About dialog showing version, build info, and links
- conntypeahead.tsx — Connection type selection modal with autocomplete
- messagemodal.tsx — Generic message dialog with buttons
- modal.tsx — Base modal component structure and composition
- modalregistry.tsx — Registry mapping modal IDs to components
- modalsrenderer.tsx — Portal-based modal renderer
- typeaheadmodal.tsx — Generic autocomplete/typeahead modal
- userinputmodal.tsx — Text input dialog modal

### notification/
- notificationbubbles.tsx — Container displaying stacked notification bubbles
- notificationitem.tsx — Individual notification item with auto-dismiss
- notificationpopover.tsx — Popover showing notification history
- updatenotifier.tsx — Component checking and displaying update availability
- usenotification.tsx — Hook for managing notification state

### onboarding/
- fakechat.tsx — Demo chat messages for onboarding screen
- onboarding.tsx — Main onboarding/welcome flow component
- onboarding-command.tsx — Command input section of onboarding
- onboarding-features.tsx — Feature showcase section
- onboarding-layout.tsx — Layout wrapper for onboarding flow
- onboarding-upgrade.tsx — Upgrade prompt section

### shadcn/
- chart.tsx — Chart utility components
- form.tsx — Form utilities
- label.tsx — Label component
- lib/utils.ts — Class name merging utility

### store/ (State Management)
- contextmenu.ts — Model for managing context menu display
- focusManager.ts — Manager for keyboard focus state between blocks and panels
- global.ts — Central Jotai atom store with all global application state
- jotaiStore.ts — Jotai store instance configuration
- keymodel.ts — Keyboard event handlers and key binding logic
- modalmodel.ts — Modal stack and display management
- services.ts — Backend service clients
- tabrpcclient.ts — RPC client for tab-level operations
- wos.ts — Wave Object Store utilities for object references
- wps.ts — Wave Pub-Sub for event subscriptions
- ws.ts — WebSocket client
- wshclient.ts — Wave Shell client
- wshclientapi.ts — RPC API definitions for Wave Shell
- wshrouter.ts — RPC message routing
- wshrpcutil.ts — RPC utility functions
- wshrpcutil-base.ts — Base RPC utilities
- zoom.ts — Zoom level management with percentage calculation

### suggestion/
- suggestion.tsx — Suggestion/autocomplete component

### tab/
- tab.tsx — Tab container component
- tabbar-model.ts — Model for tab bar state
- tabcontent.tsx — Tab content area with block layout
- widgetbar.tsx — Widget bar component
- workspaceeditor.tsx — Workspace/tab editor modal
- workspaceswitcher.tsx — Workspace/tab switcher component

### view/agent/ (Agent Widget)
- agent-model.ts — ViewModel for agent widget display
- agent-view.tsx — Main agent widget rendering component
- api-client.ts — API client for agent operations
- init-monitor.ts — Initialization monitoring utilities
- state.ts — Agent widget state management
- stream-parser.ts — Parser for agent stream JSON format
- types.ts — Type definitions for agent document nodes
- index.ts — Module exports
- components/AgentFooter.tsx — Footer with action buttons
- components/AgentHeader.tsx — Header with title and status
- components/AgentMessageBlock.tsx — Message rendering component
- components/BashOutputViewer.tsx — Bash command output display
- components/ConnectionStatus.tsx — Connection status indicator
- components/DiffViewer.tsx — File diff viewer
- components/FilterControls.tsx — Search/filter UI
- components/InitializationPrompt.tsx — Setup wizard for agent
- components/MarkdownBlock.tsx — Markdown content renderer
- components/ProcessControls.tsx — Process control buttons
- components/SetupWizard.tsx — Initial setup flow
- components/ToolBlock.tsx — Tool execution display
- providers/claude-translator.ts — Claude API response translator
- providers/codex-translator.ts — Codex API response translator
- providers/gemini-translator.ts — Gemini API response translator
- providers/index.ts — Provider exports
- providers/translator.ts — Base translator interface
- providers/translator-factory.ts — Factory for selecting provider translator

### view/chat/ (Chat Widget)
- channels.tsx — Channel list component
- chat.tsx — Main chat view component
- chatbox.tsx — Chat input box
- chatmessages.tsx — Chat message history display
- data.tsx — Mock chat data
- userlist.tsx — User list sidebar

### view/codeeditor/
- codeeditor.tsx — Code editor view (Monaco-based)
- schemaendpoints.ts — API schema endpoints

### view/helpview/
- helpview.tsx — Help/documentation view

### view/launcher/
- launcher.tsx — Application launcher view

### view/sysinfo/
- sysinfo.tsx — System information display view

### view/term/ (Terminal Widget)
- fitaddon.ts — XTerm.js fit addon wrapper
- ijson.tsx — Interactive JSON viewer
- term.tsx — Terminal view component with xterm.js
- termsticker.tsx — Terminal status stickers/indicators
- termtheme.ts — Terminal color theme configuration
- termutil.ts — Terminal utility functions
- termwrap.ts — Terminal wrapper utilities
- term-wsh.tsx — Wave Shell integration for terminal

### view/tsunami/
- tsunami.tsx — Tsunami view component

### view/vdom/
- vdom.tsx — Virtual DOM view component
- vdom-model.tsx — ViewModel for vdom
- vdom-utils.tsx — Utility functions for vdom

### view/webview/
- webview.tsx — Web view component

### window/ (Window Chrome)
- action-widgets.tsx — Action widget buttons in header
- system-status.tsx — System status display with config errors and window action buttons
- update-banner.tsx — Update notification banner
- window-controls.tsx — New window button and version display
- window-header.tsx — Main window header bar composing controls, drag area, and status

### workspace/
- widgets.tsx — Widget container component
- workspace.tsx — Main workspace/layout component
- workspace-layout-model.ts — Model for workspace layout state and AI panel management

---

## Root Configuration

- package.json — npm project metadata; version 0.31.9; Tauri 2.10.x; React 19, TypeScript, Vite, Tailwind
- Taskfile.yml — Task automation: dev, build, package, backend compilation, testing, docsite
- vite.config.tauri.ts — Vite config: Monaco static copy, KaTeX font stripping, image optimization, manual chunks
- tsconfig.json — TypeScript compiler options; ES6 target, bundler module resolution, path aliases
- index.html — HTML entry point; loads Tauri bootstrap and FontAwesome
- bump-version.sh — Version bump script updating all manifest files atomically

## scripts/
- verify-version.sh — Checks version consistency across all manifest files
- verify-tauri-versions.sh — Verifies Tauri core and plugin versions are aligned
- verify-package.sh — Validates package integrity after build
- update-tauri.sh — Updates Tauri versions across npm and Cargo manifests
- sync-version.sh — Synchronizes version across all files
- parity-test.sh — Tests frontend/backend version parity
- build-release.ps1 — PowerShell script for release builds (Windows)
- build-appimage.sh — Shell script for Linux AppImage packaging
- package-portable.ps1 — PowerShell script for portable ZIP packaging (Windows)
- benchmarks/ — Performance measurement scripts and documentation
- dev-tools/ — Screenshot, devtools, console interaction scripts

## docs/
- index.mdx — Landing page and overview
- gettingstarted.mdx — Getting Started guide
- config.mdx — Configuration reference
- connections.mdx — Remote connection setup
- customization.mdx — Customization options
- customwidgets.mdx — Custom widget development
- faq.mdx — FAQ
- layout.mdx — Layout and pane management
- tabs.mdx — Tab management
- workspaces.mdx — Workspace management
- keybindings.mdx — Keyboard shortcuts reference
- presets.mdx — Preset configurations
- ai-presets.mdx — AI provider presets
- widgets.mdx — Widget reference
- waveai.mdx — WaveAI feature documentation
- wsh.mdx — wsh shell integration reference
- wsh-reference.mdx — wsh detailed command reference
- releasenotes.mdx — Version release notes
- telemetry.mdx — Telemetry documentation
- retros/ — Retrospective documents for past incidents
- specs/ — Technical specification documents
