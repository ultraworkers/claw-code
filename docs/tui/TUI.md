# TUI Research & Build Plan for Claw Code

**Status:** Draft | **Date:** 2026-06-12 | **Branch:** `feat/tui`

## Executive Summary

This document synthesizes research on how similar tools build terminal UIs and proposes a phased plan for adding a first-class TUI to `claw`. The existing CLI already embeds `crossterm`, `rustyline`, `pulldown-cmark`, and `syntect` — giving us a strong foundation. The TUI should be a **new mode** (`claw tui` or `claw --tui`) rather than a replacement, sharing the `ConversationRuntime`, `Session`, and tool surfaces already in `crates/runtime`.

## Prior Art: How Peers Build TUIs

### 1. OpenAI Codex CLI (`codex-rs/tui`)
The closest direct prior art — a Rust TUI for an AI coding assistant.

| Component | Crate / Pattern |
|---|---|
| Framework | `ratatui` + `crossterm` |
| Event loop | `tokio`-async with `EventBroker` and frame-rate limiter |
| Layout | `App` (top-level state machine) → `ChatWidget` (history cells + in-flight streaming cell) → `bottom_pane` (composer/input) |
| Markdown | `pulldown-cmark` |
| Syntax highlight | `syntect` |
| Clipboard | `arboard` |
| Testing | `insta` snapshot testing |

Key architectural decision: **top-level state machine** (`App`) that owns widget dispatch. Chat history is a scrollable list of "cells" (completed turns + one in-progress streaming cell). Input is a separate widget. This cleanly separates streaming concerns from input handling.

### 2. `aichat` (sigoden)
A popular all-in-one LLM CLI.

| Component | Crate / Pattern |
|---|---|
| REPL | `reedline` (nushell line editor) + `crossterm` |
| Prompts | `inquire` for interactive dialogs |
| Streaming | `render/` module with `MarkdownRender`, `markdown_stream`, `raw_stream` |
| Syntax highlight | `syntect` |
| Color | `ansi_colours` |

Key insight: `aichat` shows that `reedline` is a stronger modern alternative to `rustyline` for rich REPLs, but the existing `rustyline` integration in `claw` can stay untouched if the TUI is a separate surface. The TUI should be a **new surface**, not a REPL replacement.

### 3. `crush` (liljencrantz)
An advanced Rust shell.

| Component | Crate / Pattern |
|---|---|
| REPL | `rustyline` (with file history) |
| Terminal control | `termion` |
| Layout | `unicode-width` for proper text measurement |

**Key insight for `claw`:** `crush` demonstrates shell/parser separation — the REPL is thin, and the heavy lifting (parsing, execution) lives in shared modules. This mirrors how `claw` should build its TUI: thin TUI shell, shared `runtime` core.

### 4. Other Rust TUI Patterns (from ecosystem research)
- **Pimalaya** (`himalaya` mail client): TUI as an add-on to existing CLI using feature flags; core logic is model-driven, UI is view-only.
- **Vuls** (security scanner): Terminal viewer uses vim-like keybindings; scan results shared between JSON/CLI/TUI outputs.
- **halp**: Rust CLI tool with optional TUI mode via feature flags or runtime detection.
- **TUIfying guides** (Jack Bisceglia, Isakdl): Show layering a TUI atop CLI logic without regressing headless usage — confirming the dual-surface strategy.

## Crate Ecosystem: Recommended Stack

| Layer | Crate | Rationale |
|---|---|---|
| **TUI Framework** | `ratatui` | De-facto standard; layout engine, widget system, backend abstraction over `crossterm`/`termion`. Already the choice of `codex-rs`. |
| **Backend** | `crossterm` (already in use) | Cross-platform; `claw` already depends on it. |
| **Async bridge** | `tokio` (already in use) | `ratatui` + `crossterm` + `tokio` via `crossterm::event::EventStream` for async event handling. |
| **Input widget** | `tui-textarea` or `tui-input` | Pre-built text input widget with scrolling, wrapping, key handling. `tui-textarea` is mature and widely used. |
| **Markdown render** | `pulldown-cmark` → custom widget (already in use) | `claw` already has `render.rs` using `pulldown-cmark` + `syntect`. Port/adapt to a `ratatui::widgets::Widget`. |
| **Syntax highlight** | `syntect` (already in use) | No change needed — wire existing `HighlightLines` into a ratatui Paragraph widget. |
| **Clipboard** | `arboard` (optional) | For "copy code block" or "copy response" — nice-to-have for MVP. |

Crate **not** recommended:
- `tui-rs` — deprecated, succeeded by `ratatui`.
- `termion` — `crossterm` is already in the dependency tree; doubling backends adds pain.
- `reedline` — great for REPL, but the TUI is a full-screen app; `ratatui` owns input.

## Architecture: Proposed Design

### Guiding Principles
1. **Non-breaking**: `claw prompt`, `claw --help`, and the REPL stay exactly as they are.
2. **Shared core**: The TUI consumes `ConversationRuntime`, `Session`, `ToolExecutor` — no duplication.
3. **Alt-screen only**: TUI runs in the alternate screen buffer; exiting returns to the shell cleanly.
4. **Feature-flag friendly**: Start as always-on dependency, gate behind `tui` feature if binary bloat becomes an issue.

### Module Layout

```text
rust/crates/rusty-claude-cli/src/
  main.rs              # existing — add `claw tui` dispatch
  cli.rs               # existing — add `Tui` to CliAction
  tui/
    mod.rs             # public entry: run(env) -> Result
    app.rs             # App: top-level state machine (Screen enum)
    event.rs           # EventBroker: bridges crossterm events → App events
    widgets/
      chat.rs          # ChatWidget: scrollable message list
      input.rs         # ComposerWidget: multi-line input + send
      sidebar.rs       # Optional: session list / tool status
    render.rs          # Markdown-to-ratatui Paragraph rendering (adapts existing render.rs)
```

### State Machine: `App`

```rust
enum Screen {
    Chat {          // primary chat view
        messages: Vec<MessageCell>,
        scroll: ScrollState,
        input: InputState,
        streaming: Option<StreamingCell>,
    },
    SessionPicker,  // list saved sessions (nice-to-have)
    Help,           // keybindings overlay
    Quit,           // shutdown signal
}
```

### Event Flow

```
┌─────────────┐     ┌─────────────────┐     ┌──────────┐
│ Crossterm   │────▶│ EventBroker     │────▶│ App      │
│ (keyboard)  │     │ (tokio stream)  │     │ (update) │
└─────────────┘     └─────────────────┘     └────┬─────┘
                                                  │
                                 ┌────────────────┘
                                 ▼
                         ┌───────────────┐
                         │ ratatui::Frame│────▶ terminal draw
                         └───────────────┘
```

### Integration with Existing Runtime

The TUI reuses these existing types directly (no wrappers):

| Existing Type | TUI Usage |
|---|---|
| `ConversationRuntime` | Drive turns; receive `AssistantEvent` / `TurnProgressReporter` |
| `Session` / `SessionStore` | Persist and resume chat history |
| `ToolExecutor` / `execute_tool` | Execute tool calls from streaming responses |
| `render::TerminalRenderer` | Adapt to ratatui `Paragraph` / `Text` |
| `input::LineEditor` | Not used; TUI has its own `tui-textarea` input |

The runtime emits events via `TurnProgressReporter` — the TUI provides its own reporter that writes into a `tokio::sync::mpsc` channel consumed by the event loop.

## Implementation Roadmap

### Phase 0: Prep (this PR)
- Create `feat/tui` branch ✓
- Add `ratatui`, `tui-textarea` (and optionally `crossterm` with `event-stream` feature) to `rusty-claude-cli/Cargo.toml`
- Ensure `cargo check --workspace` passes

### Phase 1: Skeleton (MVP)
1. **`tui/mod.rs`** — `run_tui()` entrypoint: init terminal (alternate screen, raw mode), run event loop, restore on exit.
2. **`tui/event.rs`** — `EventBroker`: `crossterm::event::EventStream` → `AppEvent` (`Tick`, `Key`, `Resize`, `Backend(AssistantEvent)`).
3. **`tui/app.rs`** — Minimal `App` with one `Screen::Chat`. State: message list (mock data) + input field.
4. **`tui/widgets/chat.rs`** — `ChatWidget`: renders `Vec<MessageCell>` as `List` or vertical `Layout` of `Paragraph`s.
5. **`tui/widgets/input.rs`** — `ComposerWidget`: wraps `tui_textarea::TextArea`, handles Enter-to-send, Shift+Enter for newline.
6. **Navigation** — `Ctrl+C` or `q` to quit; `Ctrl+L` to clear; arrow keys to scroll.
7. **CLI hook** — Add `claw tui` command that calls `run_tui()`.

### Phase 2: Live Conversation
1. Wire `ConversationRuntime::run_turn()` into the TUI event loop.
2. Create a `TuiProgressReporter` implementing `TurnProgressReporter` that sends `AppEvent::AssistantDelta` into the channel.
3. Render streaming assistant responses in real-time (append to `StreamingCell`, convert to `MessageCell` on finish).
4. Render tool calls (spinner + result) inline in the chat.
5. Session persistence: auto-save on exit, `/resume` integration.

### Phase 3: Polish
1. **Markdown widget** — Port `render.rs` (`pulldown-cmark` + `syntect`) to a ratatui widget with proper soft-wrap, code blocks, and inline formatting.
2. **Sidebar** — Show session list or active tool status in a third pane.
3. **Search / filter** — Slash command picker, message search.
4. **Copy / clipboard** — `arboard` integration for copying code blocks.
5. **Theme** — Respect `NO_COLOR`, dark/light palette, user config.

### Phase 4: Hardening
1. Graceful degradation when terminal is too small.
2. Mouse support (optional, via `crossterm`).
3. `cargo test` — unit tests for widgets, snapshot tests for render output.
4. Clippy clean, format check, docs.

## Open Questions

1. **Should the TUI replace the REPL?** No — keep both. The REPL is lightweight for quick queries; the TUI is immersive for long sessions.
2. **Should we add a `tui` feature flag?** Start without one (simpler). Add if binary size becomes a concern.
3. **How to handle permission prompts in the TUI?** Modal overlay (`ratatui-popup` or custom overlay widget) rather than spawning a subprocess.
4. **Should the TUI support multiple conversation tabs?** Nice-to-have; defer to Phase 3+.

## Sources

- [OpenAI Codex CLI (codex-rs/tui)](https://github.com/openai/codex/tree/main/codex-rs/tui) — primary prior art
- [sigoden/aichat](https://github.com/sigoden/aichat) — REPL + streaming patterns
- [liljencrantz/crush](https://github.com/liljencrantz/crush) — shell/parser separation
- [Ratatui](https://github.com/ratatui/ratatui) — TUI framework
- [Pimalaya blog: Designing a TUI](https://pimalaya.org/blog/designing-a-tui-to-preview-markdown-files/) — architectural separation
- [Jack Bisceglia: TUIfying Rust CLIs](https://dev.jackbisceglia.com/posts/tuify/) — layering TUI on CLI
- [Isakdl: How I TUI-fy my Rust CLIs](https://www.isakdl.com/TUIfy) — practical guide
- [halp](https://halp.cli.rs/) — optional TUI mode
- [Vuls](https://vuls.io/en/) — terminal viewer patterns
