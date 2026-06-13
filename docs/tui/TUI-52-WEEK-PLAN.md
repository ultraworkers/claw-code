# TUI Failure Analysis & 52-Week Delivery Plan

**Status:** Draft | **Date:** 2026-06-12 | **Branch:** `feat/tui`

This document is a post-mortem of every prior TUI attempt in this repository, followed by a week-by-week plan designed specifically to avoid those failure modes. It is meant to be read as a working agreement: if you deviate from this plan, you should write down why.

---

## Part 1: Post-Mortem — Every Prior TUI Branch

### Branch: `feat/ui-hardening` (earliest attempt)

**What happened:**
- Built on top of a 3,159-line `main.rs` that combined REPL loop, arg parsing, slash command handlers, streaming output, tool call display, permission prompting, and session management.
- Tried to make the REPL "feel more reliable and discoverable" by patching behavior inside the monolith.
- No structural extraction happened. The REPL code grew fatter, not cleaner.

**Why it failed:**
- Hardening a monolith is a treadmill: every fix adds more code to the same file, increasing the surface area of the next bug.
- No alternate-screen mode was attempted; the work was constrained to inline scrolling, which limits what a "TUI" can even mean.
- The branch produced useful commits (`a13b1c2`) but they were absorbed into `fix/ui-parity` rather than shipping independently.

**Lesson:** Do not patch the monolith. Extract first, then improve.

---

### Branch: `fix/ui-parity` (the "parity pass")

**What happened:**
- Explicitly scoped DOWN from a real TUI. Commit `bcaf6e0` literally says: *"Rejected: Rework the REPL into a full multi-pane typeahead overlay | too large for this UI-only parity slice."*
- Instead, it ported REPL affordances to match TypeScript patterns: ranked slash suggestions, alias completion, trailing-space acceptance, argument hints.
- This was good work (merged), but it was a REPL polish pass, not a TUI.

**Why the TUI part failed:**
- The author correctly identified that a full TUI needed structural extraction first, but that extraction was never done.
- Without Phase 0 (breaking `main.rs` into modules), any TUI work would have meant adding ratatui code directly into `main.rs`, creating an unmaintainable 4,000+ line file.
- The plan became "ship the parity pass now, do TUI later" — and later never came because the monolith became even harder to refactor.

**Lesson:** If you need extraction first, do the extraction first. Do not defer it.

---

### Branch: `feat/uiux-redesign` (the big swing)

**What happened:**
- The most ambitious branch. It added:
  - Vim keybinding mode (normal/insert/visual/command)
  - LSP client integration (diagnostics, definitions, references)
  - Git slash commands (branch, commit, commit-push-pr, worktree)
  - An HTTP/SSE server crate with axum
- But it also produced **18 commits** that were purely README credit warfare: adding, removing, and re-adding `oh-my-opencode` and `Jobdori` credits.

**Why it failed:**
- Credit wars consumed more branch history than feature work. This is a classic community/coordination failure: when contributors can't agree on attribution, the actual product stalls.
- The LSP integration and vim mode were large features that required review cycles, but the branch was mired in docs disputes.
- The branch never merged cleanly. Its useful features (git slash commands, LSP) were absorbed piecemeal into other branches, while the TUI part was abandoned.

**Lesson:** Establish a CONTRIBUTING.md attribution policy upfront and enforce it. Do not let credit bikeshedding block the build.

---

### Branch: `rcc/ui-polish`

**What happened:**
- Empty diff against `main`. This was likely a tracking or experiment branch that never got code.

**Why it failed:**
- Branch created, no work pushed, then abandoned. Probably because the author saw how much was already on `feat/uiux-redesign` and decided to wait.
- Waiting for another branch is a form of coordination failure.

**Lesson:** If you start a branch, write a singleline commit to it within 24 hours, or delete it.

---

### Document: `rust/TUI-ENHANCEMENT-PLAN.md`

**What happened:**
- A very good, comprehensive plan written 2026-03-31 by someone who clearly understood the codebase.
- It correctly identified the 3,159-line `main.rs` as the #1 risk.
- It proposed 6 phases (0=structural cleanup, 1=status bar, 2=streaming, 3=tool viz, 4=slash commands, 5=themes, 6=full-screen).

**Why it failed:**
- It was a *plan*, not a *commitment*. No branch was ever created to execute it.
- Phases 1–4 were all "moderate effort" or "large effort" tasks bundled together. Phase 6 (ratatui) was explicitly labeled a "stretch" that requires Phase 0 first — but Phase 0 itself was a multi-file refactor that no one signed up to do.
- The plan said "Feature-gate heavy dependencies — `ratatui` should be behind a `full-tui` feature flag." This is a premature optimization: adding a feature flag before you even know if the code works adds build-system friction that kills momentum.

**Lesson:** A 6-phase plan with no owner and no timeline is a wishlist. Convert it into weekly tasks with a single owner.

---

### Root Failure Modes (Summary)

| # | Failure Mode | How We Avoid It |
|---|---|---|
| 1 | **The Monolith Trap** | Week 1–3 of the plan below are *exclusively* `main.rs` extraction. No TUI code is written until the monolith is gone. |
| 2 | **Scope Panic** | "Too large for this slice" is the killer phrase. Every slice below is ≤200 lines and ships independently. |
| 3 | **Credit Wars** | Attribution is handled by the single AUTHOR/CO-AUTHOR commit trailers. No README edits during TUI work. |
| 4 | **Placeholder Branches** | No empty branches. Every branch has at least one passing test and one visible change. |
| 5 | **Plan Without Execution** | Each week has a concrete deliverable, a merge point, and a rollback strategy. |
| 6 | **Feature Flags Before Function** | No `--tui` feature flag until Week 12. Build the code first; gate it later. |
| 7 | **REPL Replacement Fear** | The TUI is `claw tui`, a separate binary entrypoint. The REPL (`claw` with no args) is untouched forever. |
| 8 | **Streaming-Not-First** | The TUI's first real conversation feature is streaming rendering. Static screens come later. |
| 9 | **Dependency Bloat Fear** | Only 3 new crates: `ratatui`, `tui-textarea`, `crossterm` event-stream feature. No optional deps until Week 20. |
| 10 | **No Test Coverage for UI** | Every widget module has a `#[cfg(test)]` block that renders to a `TestBackend` (ratatui's in-memory buffer). |

---

## Part 2: The 52-Week Plan

### Sprint Structure

- **Sprints are 2 weeks** (26 sprints total).
- **Every sprint ships.** If a sprint can't ship, you cut scope, not delay.
- **Merge target:** `feat/tui` → `main` via PR at end of each sprint.
- **Review rule:** No PR >400 lines. If a task is bigger, split it.

---

### Phase A: Foundation (Weeks 1–6) — Extract or Die

**Goal:** Kill the monolith. No TUI code is written until `main.rs` is <200 lines.

| Week | Deliverable | Lines (est) | How It Ships |
|---|---|---|---|
| **1** | Extract `app.rs` from `main.rs`: move `LiveCli` struct and its `impl` to `app.rs`. `main.rs` now only imports and calls `LiveCli::run()`. | ~300 | PR: `refactor: extract LiveCli to app.rs` |
| **2** | Extract `format.rs`: move all `format_*` functions (status, cost, model, permissions, compact, etc.) out of `main.rs` and `app.rs`. | ~400 | PR: `refactor: extract report formatting to format.rs` |
| **3** | Extract `session_mgr.rs`: move session CRUD (create, resume, list, switch, persist, compaction) out of `app.rs`. | ~350 | PR: `refactor: extract session management to session_mgr.rs` |
| **4** | Create `tui/mod.rs` as an empty namespace with a `pub fn run() -> io::Result<()>` stub. Add `claw tui` CLI dispatch that calls it and prints "TUI mode coming soon." | ~50 | PR: `feat: add tui module stub and claw tui command` |
| **5** | Wire `ConversationRuntime` into `app.rs` through a trait boundary. This is prep for sharing the runtime between REPL and TUI without duplication. | ~200 | PR: `refactor: runtime access through trait boundary` |
| **6** | Add ratatui dependency, `crossterm` event-stream feature. Create a minimal alternate-screen demo in `tui/mod.rs` that shows a "Hello, claw" `Paragraph` and exits on `q`. Tests use `TestBackend`. | ~150 | PR: `feat: ratatui skeleton with TestBackend tests` |

**Sprint 1–3 merge checkpoint:** `main.rs` is <200 lines. All tests pass. No behavioral changes.

---

### Phase B: Core TUI Shell (Weeks 7–12) — Make It Run

**Goal:** The TUI opens, renders, and exits cleanly. No conversation yet.

| Week | Deliverable | Lines (est) | How It Ships |
|---|---|---|---|
| **7** | `tui/event.rs`: `EventBroker` that bridges `crossterm::event::EventStream` → `AppEvent` enum (`Tick`, `Key`, `Resize`, `Quit`). Unit-test key translation. | ~180 | PR: `feat: tui event broker with async event stream` |
| **8** | `tui/app.rs`: `App` state machine with one screen. Renders a static list of mock messages (using `Paragraph`). Handles `q` to quit, arrow keys to scroll. | ~200 | PR: `feat: tui app state machine with mock messages` |
| **9** | `tui/widgets/input.rs`: Composer widget wrapping `tui_textarea::TextArea`. Enter sends, Shift+Enter inserts newline. Mock: prints submitted text to a debug overlay. | ~150 | PR: `feat: tui composer input widget` |
| **10** | `tui/widgets/chat.rs`: `ChatWidget` renders a scrollable `Vec<MessageCell>` as vertical `Layout` of `Paragraph`s. Supports PgUp/PgDn. | ~200 | PR: `feat: tui chat message list widget` |
| **11** | Terminal resize handling. Status bar at bottom showing model name (static), permission mode (static), terminal dimensions. | ~120 | PR: `feat: terminal resize handling and static status bar` |
| **12** | Integration week: `claw tui` opens a full-screen app with mock chat + input + status bar. Add smoke test in `tests/` that runs in CI (skips if no TTY). Add `--tui` feature flag? **No.** Feature flag deferred until we have a reason. | ~100 | PR: `feat: claw tui opens full-screen TUI with mock data` |

**Sprint 4–6 merge checkpoint:** `claw tui` runs. It looks like a chat app with fake data. It exits cleanly. CI passes.

---

### Phase C: Live Conversation (Weeks 13–20) — Make It Talk

**Goal:** The TUI has real conversations with the model, streaming text, tool calls, and session persistence.

| Week | Deliverable | Lines (est) | How It Ships |
|---|---|---|---|
| **13** | Wire `ConversationRuntime` into TUI event loop. Create `TuiProgressReporter` that sends `AssistantEvent` deltas into the TUI channel. | ~250 | PR: `feat: wire ConversationRuntime into tui event loop` |
| **14** | Streaming assistant text: `AssistantEvent::TextDelta` appends to the active message cell in real time. No markdown rendering yet — raw text. | ~180 | PR: `feat: real-time streaming assistant text in tui` |
| **15** | Message persistence: on exit, save the session to the same JSONL format as the REPL. On `claw tui --resume`, load it. | ~200 | PR: `feat: tui session save and resume` |
| **16** | Tool call rendering: when a tool call starts, show a spinner + tool name inline. When it finishes, show ✓/✗ with truncated result (first 5 lines). | ~220 | PR: `feat: inline tool call spinner and result summary` |
| **17** | Permission prompt overlay: modal widget (centered block) for Y/N approval when a tool needs permission. Does not block the event loop — shows up as a `Screen::PermissionPrompt` state. | ~200 | PR: `feat: permission prompt modal overlay in tui` |
| **18** | Error handling: network errors, API rate limits, and runtime errors render as styled error messages in the chat, not panics. | ~150 | PR: `feat: graceful error rendering in tui chat` |
| **19** | Multi-turn conversation: send follow-up messages. Scrollback preserves full history. Input clears after send. | ~100 | PR: `feat: multi-turn conversation in tui` |
| **20** | **Feature flag week:** Add `tui` feature to `rusty-claude-cli/Cargo.toml`. Default OFF. CI builds with `--features tui`. Binary size documented. | ~50 | PR: `feat: gate tui behind optional feature flag` |

**Sprint 7–10 merge checkpoint:** `cargo run -p rusty-claude-cli -- tui --resume` loads a session and chats with the model. Tool calls show inline. Errors are graceful. CI passes.

---

### Phase D: Rich Rendering (Weeks 21–28) — Make It Beautiful

**Goal:** Markdown rendering, syntax highlighting, themes, and tool visualization.

| Week | Deliverable | Lines (est) | How It Ships |
|---|---|---|---|
| **21** | Port existing `render.rs` (`pulldown-cmark` + `syntect`) to a `tui/widgets/markdown.rs` widget. Renders headings, lists, inline code, code blocks with syntax highlighting in ratatui `Paragraph`s. | ~300 | PR: `feat: markdown widget with syntax highlighting` |
| **22** | Table rendering: `pulldown-cmark` table events → ratatui `Table` widget with borders. | ~150 | PR: `feat: table rendering in markdown widget` |
| **23** | Blockquote rendering: styled indentation + left border. Link rendering: underlined + clickable (if mouse support enabled later). | ~100 | PR: `feat: blockquote and link rendering` |
| **24** | Streaming markdown: incremental parse of markdown as text deltas arrive. Don't re-render from scratch every frame — append to parser state. | ~200 | PR: `feat: incremental markdown streaming render` |
| **25** | Dark/light theme system. Respect `NO_COLOR`. Detect truecolor vs. 256-color vs. 16-color terminals and degrade gracefully. | ~250 | PR: `feat: color theme system with terminal capability detection` |
| **26** | Tool result expansion: press `Enter` on a tool result to expand/collapse full output. Use `tui-textarea` or custom widget for scrollable code view. | ~180 | PR: `feat: collapsible/expandable tool results` |
| **27** | Colored `/diff` display: when `edit_file` or `/diff` produces output, render unified diff with green additions and red removals in the chat. | ~200 | PR: `feat: colored diff rendering in tui` |
| **28** | Performance pass: profile rendering with `cargo flamegraph`. Ensure 60fps on a 100-message session. Optimize if below 30fps. | ~100 | PR: `perf: rendering performance pass` |

**Sprint 11–14 merge checkpoint:** The TUI renders markdown, tables, code blocks with syntax highlighting, and colored diffs. Themes work. Performance is smooth.

---

### Phase E: Navigation & Power Features (Weeks 29–38) — Make It Fast to Use

**Goal:** Keyboard shortcuts, search, session picker, and slash command integration.

| Week | Deliverable | Lines (est) | How It Ships |
|---|---|---|---|
| **29** | `/` search within conversation: incremental search highlighting matching text. `n`/`N` for next/prev match. | ~200 | PR: `feat: conversation search with incremental highlighting` |
| **30** | `?` help overlay: keyboard shortcuts panel showing all keybindings. | ~150 | PR: `feat: keyboard shortcuts help overlay` |
| **31** | Session picker: `Ctrl+S` opens a fuzzy-filterable list of saved sessions (up/down arrows, Enter to switch). | ~200 | PR: `feat: interactive session picker` |
| **32** | Slash command integration: type `/` in the input to trigger slash command picker (like the REPL). Commands execute within the TUI (e.g., `/model`, `/permissions`). | ~250 | PR: `feat: slash command picker and execution in tui` |
| **33** | `/compact` in TUI: runs compaction, shows a toast notification with tokens saved. | ~100 | PR: `feat: compact command with toast notification` |
| **34** | Copy to clipboard: `y` on a message or code block copies it to clipboard via `arboard`. | ~120 | PR: `feat: copy message to clipboard` |
| **35** | Mouse support: click to expand tool results, scroll wheel on conversation, click to focus input. | ~150 | PR: `feat: mouse support for tool expansion and scrolling` |
| **36** | Pager mode: for very long outputs (e.g., `/status`, `/config`), open a full-screen pager with `j`/`k`/`q`. | ~180 | PR: `feat: internal pager for long outputs` |
| **37** | Multi-pane layout (optional): `Ctrl+P` toggles a right sidebar showing active tool status or todo list. This is the first "full TUI" power feature. | ~250 | PR: `feat: optional right sidebar for tool status` |
| **38** | Custom keybindings: read keybindings from `.claw.json` (e.g., remap `Ctrl+S` to something else). | ~150 | PR: `feat: user-configurable keybindings` |

**Sprint 15–19 merge checkpoint:** The TUI is a power-user tool. Keyboard shortcuts, search, mouse, session picker, and slash commands all work.

---

### Phase F: Polish & Hardening (Weeks 39–46) — Make It Production-Ready

**Goal:** Stable, tested, accessible, and well-documented.

| Week | Deliverable | Lines (est) | How It Ships |
|---|---|---|---|
| **39** | Accessibility: screen reader support via `crossterm` title updates and status announcements. | ~100 | PR: `feat: screen reader accessibility annotations` |
| **40** | Small-terminal graceful degradation: if terminal is <30 rows or <80 cols, show a warning banner and disable sidebar. | ~120 | PR: `feat: small terminal graceful degradation` |
| **41** | Unicode width handling: use `unicode-width` to prevent CJK and emoji from breaking layouts. | ~100 | PR: `fix: unicode width for layout correctness` |
| **42** | Snapshot testing: use `insta` to snapshot-render complex conversations (tables, code blocks, diffs) and catch regressions. | ~150 | PR: `test: snapshot tests for tui rendering` |
| **43** | Stress testing: 1,000-message session. Ensure no memory leaks, OOM, or frame drops. | ~50 (test-only) | PR: `test: stress test for large sessions` |
| **44** | Windows testing: verify in PowerShell and Windows Terminal. Fix cursor/resize issues. | ~100 | PR: `fix: windows terminal compatibility` |
| **45** | Documentation: `docs/TUI-USAGE.md` — comprehensive user guide with screenshots (ASCII art), keybindings, tips. | ~200 (docs only) | PR: `docs: tui user guide` |
| **46** | `claw doctor` TUI diagnostic: `claw doctor --tui` runs a self-test that opens the TUI, verifies rendering, and reports any issues. | ~150 | PR: `feat: tui diagnostic mode in doctor` |

**Sprint 20–23 merge checkpoint:** The TUI is production-ready. Tests cover rendering, large sessions, and cross-platform compatibility. Docs are complete.

---

### Phase G: Advanced Features (Weeks 47–52) — Make It Best-in-Class

**Goal:** Features that differentiate claw from other AI CLI TUIs.

| Week | Deliverable | Lines (est) | How It Ships |
|---|---|---|---|
| **47** | Image support: when vision input lands in the API (see ROADMAP #220), the TUI renders image attachments inline using `sixel` or `iTerm2` image protocols, with a text fallback. | ~200 | PR: `feat: inline image rendering in tui` |
| **48** | Notebook editing: `/notebook` opens the notebook editor in a split pane (like VS Code's notebook view). | ~250 | PR: `feat: notebook editing in tui` |
| **49** | Team/agent dashboard: `Ctrl+A` shows a real-time view of running agents, their status, and their outputs (like `htop` for agents). | ~250 | PR: `feat: agent team dashboard in tui` |
| **50** | MCP server inspector: `Ctrl+M` opens a pane showing connected MCP servers, their tools, and recent calls. | ~200 | PR: `feat: mcp server inspector sidebar` |
| **51** | Teleport mode: `Ctrl+G` opens a fuzzy file finder (like `fzf`) to jump to any file in the workspace, then sends a `/read` tool call. | ~180 | PR: `feat: fuzzy file finder teleport in tui` |
| **52** | Release polish: final pass of the whole plan. Update ROADMAP.md. Cut a release note. The TUI is now the default for `claw` when a TTY is detected (with `--no-tui` to opt out). | ~100 | PR: `feat: make tui the default interactive mode` |

**Final merge checkpoint:** `claw` with no args opens the TUI by default (when TTY detected). The TUI supports images, notebooks, agent dashboards, MCP inspection, and fuzzy file search. This is a 52-week execution of the vision.

---

## Part 3: Rules of Engagement

To avoid repeating the failures above, these rules are binding for anyone working on the TUI:

1. **No README edits during TUI work.** Attribution is via `Co-Authored-By` trailers in commits only. README credit updates happen in separate PRs, never mixed with TUI feature PRs.

2. **No empty branches.** If you create a branch, commit something testable within 24 hours. If you haven't, delete the branch.

3. **No PR >400 lines.** Any task that produces more than 400 lines must be split. This is enforced by reviewer policy, not tooling.

4. **No feature flags before Week 20.** The `tui` feature flag is added only after the TUI is demonstrably working. Before that, `ratatui` is a normal dependency. This removes the "feature flag complexity" tax that killed momentum in prior plans.

5. **Every widget has a `#[cfg(test)]` block.** Use `ratatui::backend::TestBackend` to assert rendered buffer state. No widget ships without at least one buffer assertion.

6. **Every sprint ends with `cargo test --workspace` passing.** Not "mostly passing." Green CI is the definition of done.

7. **The REPL is sacred.** `claw` with no args may eventually default to TUI (Week 52), but the REPL mode (`claw --repl` or piped input) is supported forever. No TUI change may break the REPL.

8. **Streaming is the default UX.** The TUI should never buffer an entire response before rendering it. Every message cell supports incremental appending from the first week it exists.

9. **Runtime types are reused directly.** No wrapper structs around `ConversationRuntime`, `Session`, or `ToolExecutor`. If the runtime API is awkward for the TUI, fix the runtime API, not the TUI.

10. **Defer, don't abandon.** If a week is running long, cut scope and merge what works. Move the cut items to a "stretch" backlog. Never let a week slip into the next week.

---

## Part 4: Emergency Rollback Strategy

If the TUI work introduces instability:

1. **Week 1–6 (extraction):** Each extraction is a pure refactor. `git revert` of any single PR restores the old code in `main.rs` because the old code was moved, not deleted, during the transition.

2. **Week 7–12 (skeleton):** The TUI is opt-in via `claw tui`. Reverting the `tui/` directory and the CLI dispatch removes the feature entirely without affecting the REPL.

3. **Week 13+ (live conversation):** If the streaming integration breaks runtime stability, the `TuiProgressReporter` can be swapped for a no-op reporter in a single-line revert.

4. **Nuclear option:** `git checkout main -- rust/crates/rusty-claude-cli/src/tui/` and remove the `claw tui` dispatch. The REPL and all other functionality are untouched.

---

*Generated: 2026-06-12 | Based on post-mortem of `feat/ui-hardening`, `fix/ui-parity`, `feat/uiux-redesign`, `rcc/ui-polish`, and `rust/TUI-ENHANCEMENT-PLAN.md` | Author: TheArchitectit*
