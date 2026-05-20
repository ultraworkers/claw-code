# TUI Enhancement Plan — Brew Code (`rusty-claude-cli`)

## Executive Summary

This plan covers a comprehensive analysis of the current terminal user interface and proposes phased enhancements that will transform the existing REPL/prompt CLI into a polished, modern TUI experience — while preserving the existing clean architecture and test coverage.

---

## 1. Current Architecture Analysis

### Crate Map

| Crate | Purpose | Lines | TUI Relevance |
|---|---|---|---|
| `rusty-claude-cli` | Main binary: REPL loop, arg parsing, rendering, API bridge | ~3,600 | **Primary TUI surface** |
| `runtime` | Session, conversation loop, config, permissions, compaction | ~5,300 | Provides data/state |
| `api` | Anthropic HTTP client + SSE streaming | ~1,500 | Provides stream events |
| `commands` | Slash command metadata/parsing/help | ~470 | Drives command dispatch |
| `tools` | 18 built-in tool implementations | ~3,500 | Tool execution display |

### Current TUI Components

| Component | File | What It Does Today | Quality |
|---|---|---|---|
| **Input** | `input.rs` (269 lines) | `rustyline`-based line editor with slash-command tab completion, Shift+Enter newline, history | ✅ Solid |
| **Rendering** | `render.rs` (641 lines) | Markdown→terminal rendering (headings, lists, tables, code blocks with syntect highlighting, blockquotes), spinner widget | ✅ Good |
| **App/REPL loop** | `main.rs` (3,159 lines) | The monolithic `LiveCli` struct: REPL loop, all slash command handlers, streaming output, tool call display, permission prompting, session management | ⚠️ Monolithic |
| **Alt App** | `app.rs` (398 lines) | An earlier `CliApp` prototype with `ConversationClient`, stream event handling, `TerminalRenderer`, output format support | ⚠️ Appears unused/legacy |

### Key Dependencies

- **crossterm 0.28** — terminal control (cursor, colors, clear)
- **pulldown-cmark 0.13** — Markdown parsing
- **syntect 5** — syntax highlighting
- **rustyline 15** — line editing with completion
- **serde_json** — tool I/O formatting

### Strengths

1. **Clean rendering pipeline**: Markdown rendering is well-structured with state tracking, table rendering, code highlighting
2. **Rich tool display**: Tool calls get box-drawing borders (`╭─ name ─╮`), results show ✓/✗ icons
3. **Comprehensive slash commands**: 15 commands covering model switching, permissions, sessions, config, diff, export
4. **Session management**: Full persistence, resume, list, switch, compaction
5. **Permission prompting**: Interactive Y/N approval for restricted tool calls
6. **Thorough tests**: Every formatting function, every parse path has unit tests

### Weaknesses & Gaps

1. **`main.rs` is a 3,159-line monolith** — all REPL logic, formatting, API bridging, session management, and tests in one file
2. **No alternate-screen / full-screen layout** — everything is inline scrolling output
3. **No progress bars** — only a single braille spinner; no indication of streaming progress or token counts during generation
4. **No visual diff rendering** — `/diff` just dumps raw git diff text
5. **No syntax highlighting in streamed output** — markdown rendering only applies to tool results, not to the main assistant response stream
6. **No status bar / HUD** — model, tokens, session info not visible during interaction
7. **No image/attachment preview** — `SendUserMessage` resolves attachments but never displays them
8. **Streaming is char-by-char with artificial delay** — `stream_markdown` sleeps 8ms per whitespace-delimited chunk
9. **No color theme customization** — hardcoded `ColorTheme::default()`
10. **No resize handling** — no terminal size awareness for wrapping, truncation, or layout
11. **Dual app structs** — `app.rs` has a separate `CliApp` that duplicates `LiveCli` from `main.rs`
12. **No pager for long outputs** — `/status`, `/config`, `/memory` can overflow the viewport
13. **Tool results not collapsible** — large bash outputs flood the screen
14. **No thinking/reasoning indicator** — when the model is in "thinking" mode, no visual distinction
15. **No auto-complete for tool arguments** — only slash command names complete

---

## 2. Enhancement Plan

### Phase 0: Structural Cleanup (Foundation)

**Goal**: Break the monolith, remove dead code, establish the module structure for TUI work.

| Task | Description | Effort |
|---|---|---|
| 0.1 | **Extract `LiveCli` into `app.rs`** — Move the entire `LiveCli` struct, its impl, and helpers (`format_*`, `render_*`, session management) out of `main.rs` into focused modules: `app.rs` (core), `format.rs` (report formatting), `session_manager.rs` (session CRUD) | M |
| 0.2 | **Remove or merge the legacy `CliApp`** — The existing `app.rs` has an unused `CliApp` with its own `ConversationClient`-based rendering. Either delete it or merge its unique features (stream event handler pattern) into the active `LiveCli` | S |
| 0.3 | **Extract `main.rs` arg parsing** — The current `parse_args()` is a hand-rolled parser that duplicates the clap-based `args.rs`. Consolidate on the hand-rolled parser (it's more feature-complete) and move it to `args.rs`, or adopt clap fully | S |
| 0.4 | **Create a `tui/` module** — Introduce `crates/rusty-claude-cli/src/tui/mod.rs` as the namespace for all new TUI components: `status_bar.rs`, `layout.rs`, `tool_panel.rs`, etc. | S |

### Phase 1: Status Bar & Live HUD

**Goal**: Persistent information display during interaction.

| Task | Description | Effort |
|---|---|---|
| 1.1 | **Terminal-size-aware status line** — Use `crossterm::terminal::size()` to render a bottom-pinned status bar showing: model name, permission mode, session ID, cumulative token count, estimated cost | M |
| 1.2 | **Live token counter** — Update the status bar in real-time as `AssistantEvent::Usage` and `AssistantEvent::TextDelta` events arrive during streaming | M |
| 1.3 | **Turn duration timer** — Show elapsed time for the current turn (the `showTurnDuration` config already exists in Config tool but isn't wired up) | S |
| 1.4 | **Git branch indicator** — Display the current git branch in the status bar (already parsed via `parse_git_status_metadata`) | S |

### Phase 2: Enhanced Streaming Output

**Goal**: Make the main response stream visually rich and responsive.

| Task | Description | Effort |
|---|---|---|
| 2.1 | **Live markdown rendering** — Instead of raw text streaming, buffer text deltas and incrementally render Markdown as it arrives (heading detection, bold/italic, inline code). The existing `TerminalRenderer::render_markdown` can be adapted for incremental use | L |
| 2.2 | **Thinking indicator** — When extended thinking/reasoning is active, show a distinct animated indicator (e.g., `🧠 Reasoning...` with pulsing dots or a different spinner) instead of the generic `🦀 Thinking...` | S |
| 2.3 | **Streaming progress bar** — Add an optional horizontal progress indicator below the spinner showing approximate completion (based on max_tokens vs. output_tokens so far) | M |
| 2.4 | **Remove artificial stream delay** — The current `stream_markdown` sleeps 8ms per chunk. For tool results this is fine, but for the main response stream it should be immediate or configurable | S |

### Phase 3: Tool Call Visualization

**Goal**: Make tool execution legible and navigable.

| Task | Description | Effort |
|---|---|---|
| 3.1 | **Collapsible tool output** — For tool results longer than N lines (configurable, default 15), show a summary with `[+] Expand` hint; pressing a key reveals the full output. Initially implement as truncation with a "full output saved to file" fallback | M |
| 3.2 | **Syntax-highlighted tool results** — When tool results contain code (detected by tool name — `bash` stdout, `read_file` content, `REPL` output), apply syntect highlighting rather than rendering as plain text | M |
| 3.3 | **Tool call timeline** — For multi-tool turns, show a compact summary: `🔧 bash → ✓ | read_file → ✓ | edit_file → ✓ (3 tools, 1.2s)` after all tool calls complete | S |
| 3.4 | **Diff-aware edit_file display** — When `edit_file` succeeds, show a colored unified diff of the change instead of just `✓ edit_file: path` | M |
| 3.5 | **Permission prompt enhancement** — Style the approval prompt with box drawing, color the tool name, show a one-line summary of what the tool will do | S |

### Phase 4: Enhanced Slash Commands & Navigation

**Goal**: Improve information display and add missing features.

| Task | Description | Effort |
|---|---|---|
| 4.1 | **Colored `/diff` output** — Parse the git diff and render it with red/green coloring for removals/additions, similar to `delta` or `diff-so-fancy` | M |
| 4.2 | **Pager for long outputs** — When `/status`, `/config`, `/memory`, or `/diff` produce output longer than the terminal height, pipe through an internal pager (scroll with j/k/q) or external `$PAGER` | M |
| 4.3 | **`/search` command** — Add a new command to search conversation history by keyword | M |
| 4.4 | **`/undo` command** — Undo the last file edit by restoring from the `originalFile` data in `write_file`/`edit_file` tool results | M |
| 4.5 | **Interactive session picker** — Replace the text-based `/session list` with an interactive fuzzy-filterable list (up/down arrows to select, enter to switch) | L |
| 4.6 | **Tab completion for tool arguments** — Extend `SlashCommandHelper` to complete file paths after `/export`, model names after `/model`, session IDs after `/session switch` | M |

### Phase 5: Color Themes & Configuration

**Goal**: User-customizable visual appearance.

| Task | Description | Effort |
|---|---|---|
| 5.1 | **Named color themes** — Add `dark` (current default), `light`, `solarized`, `catppuccin` themes. Wire to the existing `Config` tool's `theme` setting | M |
| 5.2 | **ANSI-256 / truecolor detection** — Detect terminal capabilities and fall back gracefully (no colors → 16 colors → 256 → truecolor) | M |
| 5.3 | **Configurable spinner style** — Allow choosing between braille dots, bar, moon phases, etc. | S |
| 5.4 | **Banner customization** — Make the ASCII art banner optional or configurable via settings | S |

### Phase 6: Full-Screen TUI Mode (Stretch)

**Goal**: Optional alternate-screen layout for power users.

| Task | Description | Effort |
|---|---|---|
| 6.1 | **Add `ratatui` dependency** — Introduce `ratatui` (terminal UI framework) as an optional dependency for the full-screen mode | S |
| 6.2 | **Split-pane layout** — Top pane: conversation with scrollback; Bottom pane: input area; Right sidebar (optional): tool status/todo list | XL |
| 6.3 | **Scrollable conversation view** — Navigate past messages with PgUp/PgDn, search within conversation | L |
| 6.4 | **Keyboard shortcuts panel** — Show `?` help overlay with all keybindings | M |
| 6.5 | **Mouse support** — Click to expand tool results, scroll conversation, select text for copy | L |

---

## 3. Priority Recommendation

### Immediate (High Impact, Moderate Effort)

1. **Phase 0** — Essential cleanup. The 3,159-line `main.rs` is the #1 maintenance risk and blocks clean TUI additions.
2. **Phase 1.1–1.2** — Status bar with live tokens. Highest-impact UX win: users constantly want to know token usage.
3. **Phase 2.4** — Remove artificial delay. Low effort, immediately noticeable improvement.
4. **Phase 3.1** — Collapsible tool output. Large bash outputs currently wreck readability.

### Near-Term (Next Sprint)

5. **Phase 2.1** — Live markdown rendering. Makes the core interaction feel polished.
6. **Phase 3.2** — Syntax-highlighted tool results.
7. **Phase 3.4** — Diff-aware edit display.
8. **Phase 4.1** — Colored diff for `/diff`.

### Longer-Term

9. **Phase 5** — Color themes (user demand-driven).
10. **Phase 4.2–4.6** — Enhanced navigation and commands.
11. **Phase 6** — Full-screen mode (major undertaking, evaluate after earlier phases ship).

---

## 4. Architecture Recommendations

### Module Structure After Phase 0

```
crates/rusty-claude-cli/src/
├── main.rs              # Entrypoint, arg dispatch only (~100 lines)
├── args.rs              # CLI argument parsing (consolidate existing two parsers)
├── app.rs               # LiveCli struct, REPL loop, turn execution
├── format.rs            # All report formatting (status, cost, model, permissions, etc.)
├── session_mgr.rs       # Session CRUD: create, resume, list, switch, persist
├── init.rs              # Repo initialization (unchanged)
├── input.rs             # Line editor (unchanged, minor extensions)
├── render.rs            # TerminalRenderer, Spinner (extended)
└── tui/
    ├── mod.rs           # TUI module root
    ├── status_bar.rs    # Persistent bottom status line
    ├── tool_panel.rs    # Tool call visualization (boxes, timelines, collapsible)
    ├── diff_view.rs     # Colored diff rendering
    ├── pager.rs         # Internal pager for long outputs
    └── theme.rs         # Color theme definitions and selection
```

### Key Design Principles

1. **Keep the inline REPL as the default** — Full-screen TUI should be opt-in (`--tui` flag)
2. **Everything testable without a terminal** — All formatting functions take `&mut impl Write`, never assume stdout directly
3. **Streaming-first** — Rendering should work incrementally, not buffering the entire response
4. **Respect `crossterm` for all terminal control** — Don't mix raw ANSI escape codes with crossterm (the current codebase does this in the startup banner)
5. **Feature-gate heavy dependencies** — `ratatui` should be behind a `full-tui` feature flag

---

## 5. Risk Assessment

| Risk | Mitigation |
|---|---|
| Breaking the working REPL during refactor | Phase 0 is pure restructuring with existing test coverage as safety net |
| Terminal compatibility issues (tmux, SSH, Windows) | Rely on crossterm's abstraction; test in degraded environments |
| Performance regression with rich rendering | Profile before/after; keep the fast path (raw streaming) always available |
| Scope creep into Phase 6 | Ship Phases 0–3 as a coherent release before starting Phase 6 |
| `app.rs` vs `main.rs` confusion | Phase 0.2 explicitly resolves this by removing the legacy `CliApp` |

---

*Generated: 2026-03-31 | Workspace: `rust/` | Branch: `dev/rust`*
