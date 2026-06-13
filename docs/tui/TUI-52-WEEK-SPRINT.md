# TUI 52-Week Delivery Sprint Plan — Extreme Detail

**Repository:** `ultraworkers/claw-code`  
**Branch:** `feat/tui` → `main`  
**Status:** Draft for upstream PR review  
**Date:** 2026-06-12  
**Authors:** TheArchitectit, Claude  

---

## 1. Executive Summary & Prior Failure Analysis

### 1.1 The Monolith Is Bigger Than We Thought

The `rusty-claude-cli/src/main.rs` file is **11,282 lines** (as of 2026-06-12), not the 3,159 lines documented in the March 2026 plan. It contains:

- `LiveCli` struct and its `impl` block (~4,500 lines)
- All slash command handlers (~2,800 lines)
- All `format_*` report functions (~1,200 lines)
- Session management (create, resume, list, switch, persist) (~1,100 lines)
- Streaming output handling (~800 lines)
- Tool call display and permission prompting (~600 lines)
- Arg parsing and CLI dispatch (~282 lines)

This means **every prior TUI attempt was trying to build on quicksand**. The monolith grew while we were planning.

### 1.2 Post-Mortem of Every Prior Attempt

| Branch | Period | What Happened | Root Cause |
|---|---|---|---|
| `feat/ui-hardening` | Earliest | Patched REPL reliability inside monolith. Produced commit `a13b1c2` but was absorbed into `fix/ui-parity`. | Hardening a monolith is a treadmill — every fix adds more code to the same file. |
| `fix/ui-parity` | Apr 2026 | Scoped DOWN from real TUI. Commit message: *"Rejected: full multi-pane typeahead overlay — too large for this UI-only parity slice."* Ported slash command completion and hints. | Correctly identified extraction was needed first, but extraction was deferred and never done. |
| `feat/uiux-redesign` | Mar–Apr 2026 | Big swing: vim mode, LSP integration, git slash commands, HTTP server. 18 commits were README credit wars (`oh-my-opencode` add/remove/re-add). | Credit bikeshedding consumed more history than feature work. The branch never merged cleanly. |
| `rcc/ui-polish` | Mar 2026 | Empty branch. No code pushed. | Created, then abandoned when `feat/uiux-redesign` looked like it would subsume it. Waiting became permanent. |
| `rust/TUI-ENHANCEMENT-PLAN.md` | Mar 31 2026 | Excellent 6-phase plan identifying monolith as #1 risk. Proposed Phase 0 extraction. | Was a plan without a branch. No owner, no timeline, no commits. Phase 0 was labeled "essential" but no one signed up. |

### 1.3 The Ten Failure Modes & Our Guardrails

| # | Failure Mode | Guardrail |
|---|---|---|
| 1 | **The Monolith Trap** | Weeks 1–6 are *exclusively* extraction. No TUI code until `main.rs` <200 lines. |
| 2 | **Scope Panic** | Every slice ≤400 lines. The phrase "too large for this slice" triggers an immediate scope cut. |
| 3 | **Credit Wars** | No README edits during TUI work. Attribution is via `Co-Authored-By` commit trailers only. |
| 4 | **Placeholder Branches** | Every branch has ≥1 passing test and ≥1 visible change within 24 hours of creation. |
| 5 | **Plan Without Execution** | Each week has a concrete deliverable, a merge point, and a rollback strategy. |
| 6 | **Feature Flags Before Function** | No `--tui` feature flag until Week 20. Build it first; gate it later. |
| 7 | **REPL Replacement Fear** | TUI is `claw tui`, a separate entrypoint. REPL is untouched forever. |
| 8 | **Streaming-Not-First** | First real conversation feature is streaming rendering. Static screens come later. |
| 9 | **Dependency Bloat Fear** | Only 3 new crates: `ratatui`, `tui-textarea`, `unicode-width`. No optional deps until Week 20. |
| 10 | **No Test Coverage for UI** | Every widget has `#[cfg(test)]` with `TestBackend` assertions. No widget ships without one. |

### 1.4 Confirmation From Peer Projects

Research into `codex-rs/tui`, `aichat`, `crush`, and ecosystem guides confirms:

- **`ratatui` + `crossterm`** is the standard Rust TUI stack (`codex-rs` uses this).
- **Extraction-first** is mandatory — `codex-rs/tui` has `app.rs`, `chat_widget.rs`, `composer.rs`, `event_broker.rs` as separate modules.
- **Thin TUI shell, shared core** — `crush` separates shell/parser; we separate TUI/`ConversationRuntime`.
- **Streaming-first** — `aichat` renders markdown incrementally from SSE deltas.

---

## 2. Architecture Target State

### 2.1 Module Structure (End of Week 6)

```text
rust/crates/rusty-claude-cli/src/
├── main.rs              # Entrypoint only. Calls LiveCli::run() or tui::run().
│                        # Target: <150 lines.
│
├── cli.rs               # CLI argument parsing + CliAction enum.
│                        # Target: <300 lines.
│
├── app.rs               # LiveCli struct, REPL loop, turn execution.
│                        # Moves from main.rs. Target: <800 lines.
│
├── format.rs            # All report formatting functions.
│                        # Moves from main.rs. Target: <600 lines.
│
├── session_mgr.rs       # Session CRUD: create, resume, list, switch, persist, compact.
│                        # Moves from main.rs. Target: <500 lines.
│
├── init.rs              # Repo initialization (unchanged).
│
├── input.rs             # Line editor with rustyline (unchanged, minor extensions).
│
├── render.rs            # TerminalRenderer, Spinner, MarkdownStreamState.
│                        # Extended but mostly unchanged. Target: <700 lines.
│
├── setup_wizard.rs      # Interactive provider wizard (already exists, extracted in recent commits).
│
└── tui/                 # NEW: All TUI components.
    ├── mod.rs           # Public entry: run(env) -> io::Result<()>
    │                    # Terminal init (alternate screen, raw mode),
    │                    # event loop runner, cleanup on exit.
    │
    ├── app.rs           # App state machine.
    │                    # Screen enum, message state, input state, theme.
    │                    # Target: <400 lines.
    │
    ├── event.rs         # EventBroker.
    │                    # Bridges crossterm::event::EventStream → AppEvent.
    │                    # Target: <250 lines.
    │
    ├── theme.rs         # Color themes, terminal capability detection.
    │                    # Target: <200 lines.
    │
    └── widgets/
        ├── chat.rs      # ChatWidget: scrollable message list.
        │                # Renders Vec<MessageCell> as vertical Layout of Paragraphs.
        │                # Target: <300 lines.
        │
        ├── input.rs     # ComposerWidget: wraps tui_textarea::TextArea.
        │                # Handles Enter-to-send, Shift+Enter newline, history.
        │                # Target: <200 lines.
        │
        ├── markdown.rs  # MarkdownWidget: pulldown-cmark + syntect → ratatui Paragraph.
        │                # Adapted from render.rs. Target: <400 lines.
        │
        ├── status_bar.rs # Bottom status line: model, tokens, cost, session.
        │                 # Target: <150 lines.
        │
        ├── tool_call.rs  # ToolCallWidget: inline spinner + result display.
        │                 # Target: <200 lines.
        │
        ├── permission.rs # PermissionPromptWidget: modal Y/N overlay.
        │                 # Target: <150 lines.
        │
        └── sidebar.rs    # (Week 37+) Optional right sidebar.
                          # Target: <250 lines.
```

### 2.2 Runtime Integration (Zero Wrappers)

The TUI consumes these existing types **directly**:

```
tui::app::App
  ├─ uses runtime::ConversationRuntime (via shared Arc)
  ├─ uses runtime::Session (for persistence)
  ├─ uses runtime::SessionStore (for resume)
  ├─ uses runtime::ToolExecutor (for tool call dispatch)
  ├─ uses runtime::PermissionMode (for status display)
  ├─ uses runtime::TurnProgressReporter (implemented as TuiProgressReporter)
  └─ uses render.rs logic (adapted to MarkdownWidget)
```

If the `ConversationRuntime` API is awkward for the TUI, we fix the runtime API — **never** create a wrapper.

### 2.3 Event Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              EVENT FLOW                                       │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌──────────────┐     ┌──────────────────────┐     ┌──────────────┐       │
│   │  Crossterm   │────▶│   EventBroker          │────▶│     App        │       │
│   │  (keyboard,  │     │   (tokio::select!)     │     │   (state machine) │     │
│   │  resize)     │     │                        │     │                │     │
│   └──────────────┘     └──────────────────────┘     └───────┬────────┘     │
│                                                             │              │
│                                                             ▼              │
│   ┌──────────────┐     ┌──────────────────────┐     ┌──────────────┐       │
│   │  API Client  │────▶│  TuiProgressReporter │────▶│  AppEvent::   │       │
│   │  (SSE stream)│     │  (mpsc channel)      │     │  AssistantDelta │      │
│   └──────────────┘     └──────────────────────┘     └──────────────┘       │
│                                                             │              │
│                                                             ▼              │
│   ┌──────────────────────────────────────────────────────────────┐         │
│   │                    ratatui::Frame::render()                    │         │
│   │   ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │         │
│   │   │ ChatWidget  │  │ Sidebar     │  │ StatusBarWidget     │  │         │
│   │   │ (top/left)  │  │ (right, opt)│  │ (bottom)            │  │         │
│   │   └─────────────┘  └─────────────┘  └─────────────────────┘  │         │
│   └──────────────────────────────────────────────────────────────┘         │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.4 State Machine

```rust
// tui/app.rs

pub struct App {
    pub screen: Screen,
    pub messages: Vec<MessageCell>,
    pub scroll: ScrollState,
    pub input: ComposerState,
    pub status: StatusState,
    pub theme: Theme,
    pub session_id: Option<String>,
}

pub enum Screen {
    Chat,
    PermissionPrompt(PermissionRequest),
    SessionPicker(Vec<SessionSummary>),
    Help,
    Error(String),
    Quit,
}

pub struct MessageCell {
    pub role: MessageRole,
    pub content: Vec<ContentBlock>,         // completed blocks
    pub streaming: Option<StreamingCell>,   // in-flight block
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tool_calls: Vec<ToolCallCell>,
}

pub struct StreamingCell {
    pub text_buffer: String,
    pub parser_state: StreamingMarkdownState, // incremental parse state
}

pub struct ToolCallCell {
    pub name: String,
    pub status: ToolCallStatus,
    pub result_summary: String,
    pub full_result: Option<String>,
    pub expanded: bool,
}

pub enum ToolCallStatus {
    Pending,
    Running { start_time: Instant },
    Succeeded { duration: Duration },
    Failed { duration: Duration, error: String },
}
```

---

## 3. Sprint-by-Sprint Breakdown (Weeks 1–52)

### Sprint Structure

- **1 sprint = 2 weeks** (26 sprints total).
- **Every sprint ships.** If scope is too large, cut features — never delay.
- **Merge target:** `feat/tui` via ≤400-line PRs.
- **Review rule:** No PR >400 lines. Split or cut.

---

### Phase A: Foundation (Weeks 1–6) — Extract or Die

**Goal:** `main.rs` <150 lines. No TUI code written until then.

#### Week 1: Extract `app.rs`

| Field | Value |
|---|---|
| **Deliverable** | `LiveCli` struct and its `impl` block moved from `main.rs` to `app.rs`. `main.rs` now only imports `LiveCli` and calls `run()`. |
| **Files touched** | `main.rs (-4500/+20)`, `app.rs (+4520/-0)` |
| **New files** | `app.rs` (extracted from `main.rs`) |
| **Test strategy** | All existing tests in `main.rs` move to `app.rs`. Add `#[test] fn test_app_module_compiles()` as a smoke test. |
| **Merge criteria** | `cargo test --workspace` passes. `cargo clippy --workspace` clean. `main.rs` <200 lines. |
| **Rollback strategy** | `git checkout HEAD~1 -- main.rs` restores the monolith in one command. |
| **Dependencies** | None — pure file move. |
| **PR title** | `refactor: extract LiveCli to app.rs` |
| **PR description** | • Moves `LiveCli` struct and impl from `main.rs` to new `app.rs`. • No behavioral changes — pure extraction. • All tests preserved. • `main.rs` reduced from ~11,000 to ~200 lines. |
| **Review checklist** | ① No logic changes — only moves. ② All `main.rs` tests still exist in `app.rs`. ③ `cargo test --workspace` passes. ④ `main.rs` <200 lines. ⑤ No new dependencies. |
| **Risk** | Medium — large file move can lose tests. Mitigated by `git diff --stat` verification. |
| **Docs** | Update `rust/README.md` workspace layout diagram. |
| **Harness impact** | None — pure refactor, no behavioral change. |

**Verify this week:**
```bash
cd rust
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
wc -l crates/rusty-claude-cli/src/main.rs  # should be <200
```

---

#### Week 2: Extract `format.rs`

| Field | Value |
|---|---|
| **Deliverable** | All `format_*` functions moved from `app.rs`/`main.rs` to `format.rs`. |
| **Files touched** | `app.rs (-1200/+50)`, `format.rs (+1200/-0)`, `main.rs (-0/+10)` |
| **New files** | `format.rs` |
| **Functions moved** | `format_status_report`, `format_cost_report`, `format_model_report`, `format_permissions_report`, `format_compact_report`, `format_resume_report`, `format_sandbox_report`, `format_commit_preflight_report`, `format_commit_skipped_report`, `format_pr_report`, `format_issue_report`, `format_ultraplan_report`, `format_bughunter_report`, `format_doctor_report`, `format_auto_compaction_notice` |
| **Test strategy** | Move all `format_*` tests from `app.rs` to `format.rs`. Add `#[test] fn test_format_module_exports()` smoke test. |
| **Merge criteria** | All format tests pass. No changes to output strings. |
| **Rollback strategy** | `git checkout HEAD~1 -- app.rs` — but format functions are pure, so rollback is trivial. |
| **Dependencies** | None. |
| **PR title** | `refactor: extract report formatting to format.rs` |
| **PR description** | • Extracts 15 `format_*` functions from `app.rs` to `format.rs`. • No output changes — pure move. • All unit tests preserved. |
| **Review checklist** | ① Every `format_*` function has a test. ② Output strings unchanged. ③ `cargo test` passes. ④ `cargo clippy` clean. ⑤ No new deps. |
| **Risk** | Low — pure code move with test coverage. |
| **Docs** | Update `rust/README.md` crate responsibilities. |
| **Harness impact** | None. |

---

#### Week 3: Extract `session_mgr.rs`

| Field | Value |
|---|---|
| **Deliverable** | Session CRUD (create, resume, list, switch, persist, compaction) moved from `app.rs` to `session_mgr.rs`. |
| **Files touched** | `app.rs (-1100/+80)`, `session_mgr.rs (+1100/-0)` |
| **New files** | `session_mgr.rs` |
| **Functions moved** | `create_session`, `resume_session`, `list_sessions`, `switch_session`, `persist_session`, `compact_session`, `load_latest_session`, `resolve_session_path` |
| **Test strategy** | Move session tests. Add `SessionManager` struct with `new() -> Self` and `list() -> Vec<SessionSummary>` methods. Unit-test with temp dirs. |
| **Merge criteria** | Session persistence round-trip tested. `/session` slash commands still work in REPL. |
| **Rollback strategy** | `git checkout HEAD~1 -- app.rs` restores session functions. |
| **Dependencies** | None. |
| **PR title** | `refactor: extract session management to session_mgr.rs` |
| **PR description** | • Extracts session lifecycle from `app.rs` to `session_mgr.rs`. • Introduces `SessionManager` struct for centralized session operations. • Tests use temp dirs for isolation. |
| **Review checklist** | ① `/session list`, `/resume`, `/compact` work in REPL. ② Session files written to correct path. ③ Tests use `tempdir`. ④ `cargo test` passes. ⑤ No behavioral changes. |
| **Risk** | Low — session code is well-tested. |
| **Docs** | Update `USAGE.md` session section if `SessionManager` adds new CLI surfaces. |
| **Harness impact** | Session save/resume is harness-tested; verify no regressions. |

---

#### Week 4: Create `tui/mod.rs` Stub

| Field | Value |
|---|---|
| **Deliverable** | `tui/mod.rs` created with `pub fn run(env: CliEnvironment) -> io::Result<()>` stub. `claw tui` CLI dispatch added. Prints "TUI mode coming soon." and exits. |
| **Files touched** | `cli.rs (+30)`, `main.rs (+15)` |
| **New files** | `tui/mod.rs` (40 lines) |
| **Test strategy** | Integration test: `#[test] fn test_tui_subcommand_exists()` that spawns `claw tui --help` and asserts exit code 0. |
| **Merge criteria** | `claw tui --help` works. `claw tui` exits cleanly with status 0. |
| **Rollback strategy** | Remove `tui/` directory and revert `cli.rs`/`main.rs` changes. |
| **Dependencies** | None — stub only. |
| **PR title** | `feat: add tui module stub and `claw tui` command` |
| **PR description** | • Adds empty `tui/` namespace. • Adds `claw tui` CLI subcommand as stub. • Integration test verifies CLI surface. |
| **Review checklist** | ① `claw tui --help` prints help. ② `claw tui` exits 0. ③ No new dependencies yet. ④ `cargo test` passes. ⑤ Stub is clearly marked with `// TODO` comments. |
| **Risk** | Very low — no runtime code. |
| **Docs** | Update `rust/README.md` with `claw tui` in CLI flags. |
| **Harness impact** | Add `tui_stub_smoke` scenario to mock parity harness. |

**Code skeleton:**
```rust
// tui/mod.rs
use std::io;

pub fn run() -> io::Result<()> {
    println!("TUI mode coming soon.");
    Ok(())
}
```

---

#### Week 5: Runtime Trait Boundary

| Field | Value |
|---|---|
| **Deliverable** | `ConversationRuntime` access is abstracted through a trait so REPL and TUI can share it without coupling. |
| **Files touched** | `app.rs (+80)`, `tui/mod.rs (+30)` |
| **New files** | None |
| **Trait defined** | `trait RuntimeHandle { fn run_turn(&mut self, prompt: &str) -> Result<TurnResult, RuntimeError>; fn session(&self) -> &Session; }` |
| **Test strategy** | Mock implementation of `RuntimeHandle` for testing. |
| **Merge criteria** | `LiveCli` compiles with new trait. No behavioral changes to REPL. |
| **Rollback strategy** | Inline the trait methods back into direct `ConversationRuntime` calls. |
| **Dependencies** | None. |
| **PR title** | `refactor: runtime access through trait boundary` |
| **PR description** | • Introduces `RuntimeHandle` trait for shared runtime access. • Enables REPL/TUI duality without duplication. • No behavioral changes. |
| **Review checklist** | ① Trait has all methods REPL needs. ② Mock impl works in tests. ③ `cargo test` passes. ④ REPL unaffected. ⑤ Documentation on trait purpose. |
| **Risk** | Low — adds abstraction without changing behavior. |
| **Docs** | Document the trait in code comments. |
| **Harness impact** | None — trait is compile-time only. |

---

#### Week 6: Ratatui Skeleton with TestBackend

| Field | Value |
|---|---|
| **Deliverable** | `ratatui` and `crossterm` (event-stream feature) added as dependencies. Minimal alternate-screen demo: shows "Hello, claw" `Paragraph`, exits on `q`. Uses `TestBackend` in tests. |
| **Files touched** | `Cargo.toml (+3)`, `tui/mod.rs (+120)` |
| **New files** | `tui/event.rs` (stub, 30 lines) |
| **Dependencies added** | `ratatui = "0.29"`, `crossterm = { version = "0.28", features = ["event-stream"] }` (crossterm already present, added feature) |
| **Test strategy** | `#[test] fn test_tui_init_buffer()` renders to `TestBackend` and asserts buffer contains "Hello, claw". |
| **Merge criteria** | `cargo test --workspace` passes. `cargo check` with `--features tui` passes (but feature flag not added yet). `claw tui` opens alternate screen and exits on `q`. |
| **Rollback strategy** | Remove ratatui from `Cargo.toml`, delete `tui/` changes. |
| **PR title** | `feat: ratatui skeleton with TestBackend tests` |
| **PR description** | • Adds `ratatui` dependency. • `claw tui` opens alternate screen, renders "Hello, claw", exits on `q`. • TestBackend verifies rendered output in CI. |
| **Review checklist** | ① Terminal restored on exit (no ANSI leak). ② `TestBackend` test passes. ③ No REPL changes. ④ `q` key exits cleanly. ⑤ Cargo.lock updated correctly. |
| **Risk** | Low — minimal code, well-tested dependency. |
| **Docs** | Add ratatui to `rust/README.md` dependencies list. |
| **Harness impact** | Add `tui_smoke_open_close` to mock parity harness. |

**Code skeleton:**
```rust
// tui/mod.rs
use std::io;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    backend::{Backend, CrosstermBackend, TestBackend},
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

pub fn run() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal);
    ratatui::restore();
    result
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f))?;
        if let Event::Key(KeyEvent { code: KeyCode::Char('q'), .. }) = event::read()? {
            return Ok(());
        }
    }
}

fn ui(frame: &mut Frame) {
    let paragraph = Paragraph::new("Hello, claw")
        .block(Block::default().borders(Borders::ALL).title("claw"));
    frame.render_widget(paragraph, frame.area());
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    #[test]
    fn test_tui_init_buffer() {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f)).unwrap();
        let buffer = terminal.backend().buffer();
        assert!(buffer.content.iter().any(|c| c.symbol() == "H"));
    }
}
```

---

### Phase B: Core TUI Shell (Weeks 7–12) — Make It Run

**Goal:** `claw tui` opens, renders a mock chat, accepts input, scrolls, and exits cleanly.

#### Week 7: Event Broker

| Field | Value |
|---|---|
| **Deliverable** | `tui/event.rs`: `EventBroker` bridges `crossterm::event::EventStream` → `AppEvent` enum. Unit-test key translation. |
| **Files touched** | `tui/mod.rs (+40)`, `tui/app.rs` (created, stub) |
| **New files** | `tui/event.rs` (180 lines) |
| **Types defined** | `enum AppEvent { Tick, Key(KeyEvent), Resize(u16, u16), AssistantDelta(String), ToolStart(String), ToolResult(String, String), Error(String), Quit }` |
| **Test strategy** | `#[test] fn test_key_translation()` maps `Event::Key(KeyCode::Char('q'))` → `AppEvent::Key(...)`. `#[test] fn test_resize_event()`. |
| **Merge criteria** | `EventBroker` runs in tests without a real terminal. All key events translate correctly. |
| **Rollback strategy** | Revert `tui/event.rs` to stub. |
| **PR title** | `feat: tui event broker with async event stream` |
| **PR description** | • `EventBroker` consumes `crossterm::event::EventStream` and produces `AppEvent`. • Supports key, resize, and custom backend events. • Unit tests for key translation. |
| **Review checklist** | ① All key variants handled. ② Resize produces correct dimensions. ③ `AppEvent` is `Clone + Send`. ④ Tests use `TestBackend`. ⑤ No blocking in test. |
| **Risk** | Low — crossterm event handling is well-documented. |
| **Docs** | Add event flow diagram to `docs/TUI.md`. |
| **Harness impact** | None — event layer doesn't touch the runtime. |

---

#### Week 8: App State Machine

| Field | Value |
|---|---|
| **Deliverable** | `tui/app.rs`: `App` struct with `Screen::Chat`, renders static mock messages as `Paragraph` list. Handles `q` quit, arrows scroll. |
| **Files touched** | `tui/mod.rs (+60)`, `tui/app.rs` (created, 200 lines) |
| **New files** | `tui/app.rs` |
| **Types defined** | `struct App { messages: Vec<MockMessage>, scroll: ScrollState, should_quit: bool }`, `struct MockMessage { role: String, text: String }`, `struct ScrollState { offset: usize }` |
| **Test strategy** | `#[test] fn test_app_scroll()` — add 100 messages, scroll down 5, assert offset == 5. `#[test] fn test_app_quit_on_q()` — process `KeyCode::Char('q')`, assert `should_quit`. |
| **Merge criteria** | App renders mock chat. Arrow keys scroll. `q` quits. Tests pass without TTY. |
| **Rollback strategy** | Revert `tui/app.rs` to stub. |
| **PR title** | `feat: tui app state machine with mock messages` |
| **PR description** | • `App` state machine with `Screen::Chat`. • Renders static mock messages. • Supports scroll via arrow keys. • `q` to quit. |
| **Review checklist** | ① Mock messages visible in TestBackend. ② Scroll offset correct. ③ `q` quits. ④ 100 messages don't panic. ⑤ Tests headless. |
| **Risk** | Low — no runtime integration yet. |
| **Docs** | Update `docs/TUI.md` state machine section. |
| **Harness impact** | None. |

---

#### Week 9: Composer Input Widget

| Field | Value |
|---|---|
| **Deliverable** | `tui/widgets/input.rs`: Composer widget wrapping `tui_textarea::TextArea`. Enter sends, Shift+Enter newline. Mock: prints submitted text to debug overlay. |
| **Files touched** | `Cargo.toml (+1)`, `tui/app.rs (+60)`, `tui/widgets/input.rs` (created, 150 lines) |
| **New files** | `tui/widgets/input.rs` |
| **Dependencies added** | `tui-textarea = "0.7"` |
| **Test strategy** | `#[test] fn test_composer_enter_sends()` — simulate Enter, assert `Submit("hello")` event. `#[test] fn test_composer_shift_enter_newline()` — assert cursor moved down. |
| **Merge criteria** | Text input works. Enter submits. Shift+Enter inserts newline. History (up arrow) cycles past inputs. |
| **Rollback strategy** | Remove `tui-textarea` from `Cargo.toml`, revert input widget. |
| **PR title** | `feat: tui composer input widget` |
| **PR description** | • Adds `ComposerWidget` using `tui-textarea`. • Enter submits, Shift+Enter inserts newline. • Up/down arrow for history. • Tests verify input events. |
| **Review checklist** | ① Enter produces submit event. ② Shift+Enter doesn't submit. ③ History works. ④ Unicode input handled. ⑤ TestBackend verifies buffer. |
| **Risk** | Low — `tui-textarea` is mature, ~2,000 GitHub stars. |
| **Docs** | Add keybindings table to `docs/TUI.md`. |
| **Harness impact** | None. |

---

#### Week 10: Chat Message List Widget

| Field | Value |
|---|---|
| **Deliverable** | `tui/widgets/chat.rs`: `ChatWidget` renders `Vec<MessageCell>` as vertical `Layout` of `Paragraph`s. Supports PgUp/PgDn. |
| **Files touched** | `tui/app.rs (+80)`, `tui/widgets/chat.rs` (created, 200 lines) |
| **New files** | `tui/widgets/chat.rs` |
| **Test strategy** | `#[test] fn test_chat_renders_messages()` — 5 messages, assert all visible. `#[test] fn test_chat_scroll_pagination()` — 50 messages in 24-line terminal, PgDn advances by page size. |
| **Merge criteria** | Messages render with roles (user/assistant) styled differently. Scroll works. PgUp/PgDn jump by viewport height. |
| **Rollback strategy** | Revert `chat.rs` to stub. |
| **PR title** | `feat: tui chat message list widget` |
| **PR description** | • `ChatWidget` renders message history with role-based styling. • Scrollable with arrows and PgUp/PgDn. • Supports 100+ messages without frame drops. |
| **Review checklist** | ① User messages right-aligned or differently colored. ② Assistant messages left-aligned. ③ Scroll offset clamped to message count. ④ PgUp/PgDn jump correctly. ⑤ Tests headless. |
| **Risk** | Low — pure rendering, no external state. |
| **Docs** | Document chat layout in `docs/TUI.md`. |
| **Harness impact** | None. |

---

#### Week 11: Terminal Resize & Static Status Bar

| Field | Value |
|---|---|
| **Deliverable** | Terminal resize handling. Status bar at bottom showing model name (static string), permission mode (static), terminal dimensions. |
| **Files touched** | `tui/app.rs (+60)`, `tui/widgets/status_bar.rs` (created, 120 lines) |
| **New files** | `tui/widgets/status_bar.rs` |
| **Test strategy** | `#[test] fn test_status_bar_renders()` — assert model name visible. `#[test] fn test_resize_updates_layout()` — assert layout recalculates on `Resize` event. |
| **Merge criteria** | Resize event updates layout. Status bar always visible at bottom. Content area adjusts. |
| **Rollback strategy** | Remove status bar widget, revert resize handler. |
| **PR title** | `feat: terminal resize handling and static status bar` |
| **PR description** | • Status bar shows model, permission mode, terminal size. • Resize events recalculate layout. • Content area shrinks/grows correctly. |
| **Review checklist** | ① Status bar at bottom in TestBackend. ② Resize doesn't panic. ③ Layout constraints correct (status bar height = 3). ④ Content area height = total - 3. ⑤ Tests pass. |
| **Risk** | Low — layout math with `ratatui::Layout`. |
| **Docs** | Document layout constraints. |
| **Harness impact** | None. |

---

#### Week 12: Integration — Mock Chat + Input + Status Bar

| Field | Value |
|---|---|
| **Deliverable** | `claw tui` opens a full-screen app with mock chat + input + status bar. Smoke test runs in CI (skips if no TTY). |
| **Files touched** | `tui/mod.rs (+40)`, `tui/app.rs (+80)` |
| **New files** | `tests/tui_smoke.rs` (integration test, 80 lines) |
| **Test strategy** | Integration test with `TestBackend`: send mock messages, assert rendered. CI test that spawns `claw tui` with `timeout 2` and asserts clean exit. |
| **Merge criteria** | `claw tui` runs for 2 seconds and exits cleanly. Full layout visible in TestBackend. CI passes. |
| **Rollback strategy** | Revert `tui/` changes, keep `app.rs`/`format.rs`/`session_mgr.rs` extractions. |
| **PR title** | `feat: claw tui opens full-screen TUI with mock data` |
| **PR description** | • Full TUI layout: chat area, input composer, status bar. • Mock conversation data. • Integration test with TestBackend and CI smoke test. |
| **Review checklist** | ① Full layout visible in test render. ② `claw tui` exits on `q`. ③ CI test passes. ④ No REPL changes. ⑤ Memory usage flat over 10-second run. |
| **Risk** | Low — integration of already-tested components. |
| **Docs** | Add TUI screenshot instructions to `USAGE.md` (ASCII art or capture). |
| **Harness impact** | Add `tui_full_layout_smoke` to mock parity harness. |

---

### Phase C: Live Conversation (Weeks 13–20) — Make It Talk

**Goal:** Real conversations with the model. Streaming text, tool calls, session save/resume, permission modal.

#### Week 13: Wire ConversationRuntime

| Field | Value |
|---|---|
| **Deliverable** | `ConversationRuntime` wired into TUI event loop. `TuiProgressReporter` implements `TurnProgressReporter`, sends `AppEvent::AssistantDelta` via `tokio::sync::mpsc`. |
| **Files touched** | `tui/app.rs (+120)`, `tui/event.rs (+40)`, `tui/mod.rs (+60)` |
| **New files** | None |
| **Types defined** | `struct TuiProgressReporter { tx: mpsc::UnboundedSender<AppEvent> } impl TurnProgressReporter for TuiProgressReporter { ... }` |
| **Test strategy** | Mock `ConversationRuntime` that emits fake deltas. `#[test] fn test_progress_reporter_sends_delta()` — assert `AssistantDelta` received. |
| **Merge criteria** | TUI receives fake streaming events. Events route to App state. No panics. |
| **Rollback strategy** | Replace `TuiProgressReporter` with no-op. |
| **PR title** | `feat: wire ConversationRuntime into tui event loop` |
| **PR description** | • `TuiProgressReporter` bridges runtime events to TUI channel. • Tokio async event loop processes backend + user events. • Mock runtime tests verify event flow. |
| **Review checklist** | ① `AssistantDelta` events reach App. ② No deadlocks. ③ Channel bounded correctly. ④ Tests use mock runtime. ⑤ Runtime trait boundary clean. |
| **Risk** | Medium — async event loop is first real complexity. Mitigated by bounded channels. |
| **Docs** | Update event flow diagram in `docs/TUI.md`. |
| **Harness impact** | Add `tui_streaming_mock` scenario. |

---

#### Week 14: Streaming Assistant Text

| Field | Value |
|---|---|
| **Deliverable** | `AssistantEvent::TextDelta` appends to active message cell in real time. No markdown rendering yet — raw text. |
| **Files touched** | `tui/app.rs (+80)`, `tui/widgets/chat.rs (+60)` |
| **New files** | None |
| **Test strategy** | `#[test] fn test_streaming_appends_text()` — send 5 deltas, assert message text == concatenated deltas. `#[test] fn test_streaming_cursor_position()` — last line visible. |
| **Merge criteria** | Text appears character-by-character in chat widget. Auto-scroll to bottom. No flicker. |
| **Rollback strategy** | Buffer deltas instead of displaying incrementally. |
| **PR title** | `feat: real-time streaming assistant text in tui` |
| **PR description** | • Assistant text streams into chat in real time. • Auto-scroll to latest delta. • Raw text (no markdown yet). |
| **Review checklist** | ① Text visible within 50ms of delta. ② No duplicate text. ③ Auto-scroll works. ④ 1000-character stream doesn't OOM. ⑤ Tests headless. |
| **Risk** | Medium — rendering performance on fast streams. |
| **Docs** | Document streaming behavior and latency targets. |
| **Harness impact** | Add `tui_streaming_text` scenario. |

---

#### Week 15: Session Save & Resume

| Field | Value |
|---|---|
| **Deliverable** | On TUI exit, save session to JSONL (same format as REPL). `claw tui --resume` loads it. |
| **Files touched** | `tui/app.rs (+80)`, `tui/mod.rs (+40)`, `session_mgr.rs (+20)` |
| **New files** | None |
| **Test strategy** | `#[test] fn test_session_save_roundtrip()` — create messages, save, load, assert equal. |
| **Merge criteria** | Session persists across `claw tui` invocations. Messages load with correct timestamps. |
| **Rollback strategy** | Don't save on exit (lose history). |
| **PR title** | `feat: tui session save and resume` |
| **PR description** | • TUI auto-saves session on exit. • `--resume` flag loads last session. • Same JSONL format as REPL sessions. |
| **Review checklist** | ① Session file written on exit. ② `--resume` loads it. ③ Format compatible with REPL sessions. ④ No data loss. ⑤ Tests verify round-trip. |
| **Risk** | Low — session persistence already works in REPL. |
| **Docs** | Update `USAGE.md` with `claw tui --resume`. |
| **Harness impact** | Add `tui_session_save_resume` scenario. |

---

#### Week 16: Inline Tool Call Rendering

| Field | Value |
|---|---|
| **Deliverable** | Tool call start shows spinner + name inline. Finish shows ✓/✗ with truncated result (first 5 lines). |
| **Files touched** | `tui/app.rs (+60)`, `tui/widgets/tool_call.rs` (created, 220 lines) |
| **New files** | `tui/widgets/tool_call.rs` |
| **Test strategy** | `#[test] fn test_tool_call_lifecycle()` — `ToolStart("bash")` → spinner → `ToolResult("bash", "ok")` → ✓. |
| **Merge criteria** | Tool calls animate inline. Results truncated with `[+]` expand hint. |
| **Rollback strategy** | Show plain text tool names instead of widgets. |
| **PR title** | `feat: inline tool call spinner and result summary` |
| **PR description** | • Tool calls show animated spinner. • Results show ✓/✗ with truncated preview. • Expandable full output. |
| **Review checklist** | ① Spinner visible during tool call. ② ✓/✗ correct based on result. ③ Truncation at 5 lines. ④ 10 concurrent tools don't break layout. ⑤ Tests headless. |
| **Risk** | Low — tool execution already works in runtime. |
| **Docs** | Document tool visualization in `docs/TUI.md`. |
| **Harness impact** | Add `tui_tool_call_display` scenario. |

---

#### Week 17: Permission Prompt Modal

| Field | Value |
|---|---|
| **Deliverable** | Permission prompt overlay (centered block) for Y/N approval. Does not block event loop — shows as `Screen::PermissionPrompt` state. |
| **Files touched** | `tui/app.rs (+60)`, `tui/widgets/permission.rs` (created, 200 lines) |
| **New files** | `tui/widgets/permission.rs` |
| **Test strategy** | `#[test] fn test_permission_modal_renders()` — assert centered block visible. `#[test] fn test_permission_y_approves()` — assert approval event sent. |
| **Merge criteria** | Modal renders over chat. `y` approves, `n` denies. Other keys ignored. Event loop continues. |
| **Rollback strategy** | Always deny or always approve (degraded mode). |
| **PR title** | `feat: permission prompt modal overlay in tui` |
| **PR description** | • Modal overlay for tool approval. • `y` to approve, `n` to deny. • Non-blocking event loop. • Styled with box drawing. |
| **Review checklist** | ① Modal centered and visible. ② `y`/`n` responses correct. ③ Background dimmed. ④ Escape cancels. ⑤ Tests headless. |
| **Risk** | Medium — modal state transitions must not deadlock. |
| **Docs** | Document permission flow in `docs/TUI.md`. |
| **Harness impact** | Add `tui_permission_prompt` scenario. |

---

#### Week 18: Error Handling

| Field | Value |
|---|---|
| **Deliverable** | Network errors, API rate limits, runtime errors render as styled error messages in chat. No panics. |
| **Files touched** | `tui/app.rs (+40)`, `tui/widgets/chat.rs (+30)` |
| **New files** | None |
| **Test strategy** | `#[test] fn test_error_rendered_in_chat()` — inject error, assert red text visible. |
| **Merge criteria** | All error paths show user-friendly messages. Panic-free. Stack traces suppressed (logged to file only). |
| **Rollback strategy** | Panic on error (ugly but functional). |
| **PR title** | `feat: graceful error rendering in tui chat` |
| **PR description** | • Errors rendered as styled messages in chat. • No panics. • Stack traces logged, not displayed. • Auto-scroll to error message. |
| **Review checklist** | ① Network error → "Connection failed" visible. ② API error → "API error: ..." visible. ③ No panic. ④ Stack trace in log file. ⑤ Tests verify all error paths. |
| **Risk** | Low — error types already exist in runtime. |
| **Docs** | Document error handling strategy. |
| **Harness impact** | Add `tui_error_handling` scenario. |

---

#### Week 19: Multi-Turn Conversation

| Field | Value |
|---|---|
| **Deliverable** | Send follow-up messages. Scrollback preserves full history. Input clears after send. |
| **Files touched** | `tui/app.rs (+40)`, `tui/widgets/input.rs (+20)` |
| **New files** | None |
| **Test strategy** | `#[test] fn test_multi_turn_history()` — 3 turns, assert 6 messages. `#[test] fn test_input_cleared_after_send()`. |
| **Merge criteria** | Each turn starts a new streaming cycle. History is complete. Input field resets. |
| **Rollback strategy** | Single-turn only (restart TUI for next turn). |
| **PR title** | `feat: multi-turn conversation in tui` |
| **PR description** | • Follow-up messages work. • Full scrollback history. • Input cleared after send. • Session auto-saves between turns. |
| **Review checklist** | ① New turn triggers streaming. ② Old turns frozen (not re-streamed). ③ Input empty after send. ④ History scrollable. ⑤ 10-turn session doesn't OOM. |
| **Risk** | Low — builds on Weeks 13–18. |
| **Docs** | Update `USAGE.md` with multi-turn examples. |
| **Harness impact** | Add `tui_multi_turn` scenario. |

---

#### Week 20: Feature Flag

| Field | Value |
|---|---|
| **Deliverable** | `tui` feature flag added to `rusty-claude-cli/Cargo.toml`. Default OFF. CI builds with `--features tui`. Binary size documented. |
| **Files touched** | `Cargo.toml (+5)`, `main.rs (+10)`, `.github/workflows/rust-ci.yml (+10)` |
| **New files** | `docs/TUI-BINARY-SIZE.md` |
| **Test strategy** | `#[test] fn test_tui_feature_compiles()` — compile with feature flag. `#[test] fn test_without_tui_feature()` — verify `claw tui` is absent without flag. |
| **Merge criteria** | `cargo build --workspace` works without feature. `cargo build --workspace --features tui` works with feature. Binary size documented. |
| **Rollback strategy** | Make `tui` feature default ON (effectively removing the flag). |
| **PR title** | `feat: gate tui behind optional feature flag` |
| **PR description** | • `tui` feature flag in `Cargo.toml`. • Default OFF to avoid binary bloat for REPL-only users. • CI builds both configurations. • Binary size impact documented. |
| **Review checklist** | ① Build without feature passes. ② Build with feature passes. ③ Binary size documented. ④ CI matrix includes both. ⑤ Flag not on by default. |
| **Risk** | Low — mechanical change. |
| **Docs** | Document feature flag usage in `USAGE.md`. |
| **Harness impact** | CI builds with `--features tui`. |

---

### Phase D: Rich Rendering (Weeks 21–28) — Make It Beautiful

**Goal:** Markdown rendering, syntax highlighting, themes, diffs, performance.

#### Week 21: Markdown Widget (Headings, Lists, Code Blocks)

| Field | Value |
|---|---|
| **Deliverable** | `tui/widgets/markdown.rs`: Port `render.rs` (`pulldown-cmark` + `syntect`) to ratatui `Paragraph`. Renders headings, lists, inline code, code blocks with syntax highlighting. |
| **Files touched** | `render.rs (+0, read-only reference)`, `tui/widgets/markdown.rs` (created, 300 lines) |
| **New files** | `tui/widgets/markdown.rs` |
| **Test strategy** | `#[test] fn test_heading_renders_bold()` — assert bold style. `#[test] fn test_code_block_highlighted()` — assert syntax colors. |
| **Merge criteria** | Markdown renders in chat. Code blocks have syntax highlighting. Lists are indented. |
| **Rollback strategy** | Use plain text rendering (ugly but readable). |
| **PR title** | `feat: markdown widget with syntax highlighting` |
| **PR description** | • `MarkdownWidget` renders pulldown-cmark events to ratatui `Paragraph`. • Code blocks with syntect highlighting. • Headings, lists, inline code, blockquotes. |
| **Review checklist** | ① H1/H2 bold and colored. ② Code blocks highlighted. ③ Lists indented. ④ Inline code colored. ⑤ 1000-line markdown renders in <50ms. |
| **Risk** | Medium — markdown parsing + rendering is complex. |
| **Docs** | Document supported markdown elements. |
| **Harness impact** | Add `tui_markdown_render` scenario. |

---

#### Week 22: Table Rendering

| Field | Value |
|---|---|
| **Deliverable** | `pulldown-cmark` table events → ratatui `Table` widget with borders. |
| **Files touched** | `tui/widgets/markdown.rs (+100)` |
| **New files** | None |
| **Test strategy** | `#[test] fn test_table_renders_with_borders()` — assert `Table` widget visible. |
| **Merge criteria** | Tables render with borders and alignment. Fit within terminal width. |
| **Rollback strategy** | Render tables as plain text. |
| **PR title** | `feat: table rendering in markdown widget` |
| **PR description** | • Markdown tables render as ratatui `Table`. • Borders, headers, cell alignment. • Truncation for narrow terminals. |
| **Review checklist** | ① Borders visible. ② Headers bold. ③ Cells aligned. ④ Wide table truncates gracefully. ⑤ Tests verify. |
| **Risk** | Low — `Table` widget is standard in ratatui. |
| **Docs** | Add table examples to docs. |
| **Harness impact** | None. |

---

#### Week 23: Blockquote & Link Rendering

| Field | Value |
|---|---|
| **Deliverable** | Blockquote: styled indentation + left border. Links: underlined + clickable (placeholder for mouse week). |
| **Files touched** | `tui/widgets/markdown.rs (+80)`, `tui/theme.rs (+20)` |
| **New files** | None |
| **Test strategy** | `#[test] fn test_blockquote_has_left_border()`. `#[test] fn test_link_is_underlined()`. |
| **Merge criteria** | Blockquotes visually distinct. Links underlined. |
| **Rollback strategy** | Render as plain text. |
| **PR title** | `feat: blockquote and link rendering` |
| **PR description** | • Blockquotes with left border and muted color. • Links underlined. • Mouse click support deferred to Week 35. |
| **Review checklist** | ① Blockquote left border visible. ② Muted text color. ③ Links underlined. ④ URL displayed on hover or inline. ⑤ Tests verify. |
| **Risk** | Low — style changes only. |
| **Docs** | Document styling choices. |
| **Harness impact** | None. |

---

#### Week 24: Incremental Markdown Streaming

| Field | Value |
|---|---|
| **Deliverable** | Incremental parse of markdown as text deltas arrive. Don't re-render from scratch every frame — append to parser state. |
| **Files touched** | `tui/widgets/markdown.rs (+120)`, `tui/app.rs (+40)` |
| **New files** | `tui/widgets/markdown.rs` adds `StreamingMarkdownState` struct |
| **Test strategy** | `#[test] fn test_incremental_parse()` — feed "# He" then "llo", assert heading recognized. |
| **Merge criteria** | Streaming text parses incrementally. No re-render flicker. Performance: <16ms per delta. |
| **Rollback strategy** | Buffer full response, then render once (slower but correct). |
| **PR title** | `feat: incremental markdown streaming render` |
| **PR description** | • Markdown parser state persists across deltas. • Incremental render avoids full re-parse. • No flicker. • Sub-16ms per delta. |
| **Review checklist** | ① Heading detected mid-stream. ② Code block fenced detected. ③ No flicker. ④ Performance <16ms. ⑤ Tests incremental state. |
| **Risk** | High — incremental parsing is tricky. Mitigated by fallback to full re-parse on parse error. |
| **Docs** | Document incremental parsing strategy. |
| **Harness impact** | None — optimization only. |

---

#### Week 25: Dark/Light Theme System

| Field | Value |
|---|---|
| **Deliverable** | Dark/light theme system. Respect `NO_COLOR`. Detect truecolor vs. 256-color vs. 16-color terminals and degrade gracefully. |
| **Files touched** | `tui/theme.rs` (created, 250 lines), `tui/app.rs (+20)` |
| **New files** | `tui/theme.rs` |
| **Test strategy** | `#[test] fn test_theme_detects_no_color()`. `#[test] fn test_theme_16_color_fallback()`. |
| **Merge criteria** | `NO_COLOR=1` disables colors. Truecolor terminal shows full palette. 16-color terminal degrades correctly. |
| **Rollback strategy** | Hardcode dark theme (no customization). |
| **PR title** | `feat: color theme system with terminal capability detection` |
| **PR description** | • Dark, light, solarized themes. • `NO_COLOR` support. • Truecolor → 256 → 16 fallback. • Configurable via `.claw.json`. |
| **Review checklist** | ① `NO_COLOR=1` → no ANSI. ② Truecolor → 24-bit colors. ③ 256-color → approximated. ④ 16-color → nearest. ⑤ Configurable. |
| **Risk** | Low — color detection is well-understood. |
| **Docs** | Document theme configuration. |
| **Harness impact** | None — cosmetic only. |

---

#### Week 26: Collapsible/Expandable Tool Results

| Field | Value |
|---|---|
| **Deliverable** | Press `Enter` on a tool result to expand/collapse full output. Scrollable code view for expanded results. |
| **Files touched** | `tui/widgets/tool_call.rs (+120)`, `tui/app.rs (+40)` |
| **New files** | None |
| **Test strategy** | `#[test] fn test_tool_expand_toggle()`. `#[test] fn test_expanded_scrollable()`. |
| **Merge criteria** | Toggle works. Expanded result scrollable. Collapsed result shows summary. |
| **Rollback strategy** | Always show full result (no collapsing). |
| **PR title** | `feat: collapsible/expandable tool results` |
| **PR description** | • Tool results toggle expand/collapse on Enter. • Expanded view scrollable. • Summary visible when collapsed. |
| **Review checklist** | ① Enter toggles state. ② Expanded view scrolls. ③ Collapsed shows summary. ④ 10 expanded tools don't break layout. ⑤ Tests verify. |
| **Risk** | Low — state toggle + scroll. |
| **Docs** | Document tool interaction in `docs/TUI.md`. |
| **Harness impact** | None. |

---

#### Week 27: Colored Diff Rendering

| Field | Value |
|---|---|
| **Deliverable** | When `edit_file` or `/diff` produces output, render unified diff with green additions and red removals. |
| **Files touched** | `tui/widgets/diff.rs` (created, 200 lines), `tui/app.rs (+20)` |
| **New files** | `tui/widgets/diff.rs` |
| **Test strategy** | `#[test] fn test_diff_green_additions()`. `#[test] fn test_diff_red_removals()`. |
| **Merge criteria** | Diff lines colored correctly. Context lines neutral. Header lines dimmed. |
| **Rollback strategy** | Render diff as plain text. |
| **PR title** | `feat: colored diff rendering in tui` |
| **PR description** | • Unified diff with green additions, red removals. • Context lines in default color. • Header dimmed. |
| **Review checklist** | ① `+` lines green. ② `-` lines red. ③ Context lines neutral. ④ Headers dimmed. ⑤ Tests verify all line types. |
| **Risk** | Low — diff parsing is straightforward. |
| **Docs** | Add diff rendering to feature list. |
| **Harness impact** | None. |

---

#### Week 28: Performance Pass

| Field | Value |
|---|---|
| **Deliverable** | Profile with `cargo flamegraph`. Ensure 60fps on 100-message session. Optimize if below 30fps. |
| **Files touched** | Various — depends on profiler output |
| **New files** | `benches/tui_render.rs` (criterion bench) |
| **Test strategy** | Criterion benchmark: render 100 messages, measure frame time. Target: p99 <16ms. |
| **Merge criteria** | `cargo bench` shows p99 frame time <16ms. No regressions in existing tests. |
| **Rollback strategy** | Accept 30fps (degraded but functional). |
| **PR title** | `perf: rendering performance pass` |
| **PR description** | • Criterion benchmark for 100-message render. • Flamegraph analysis. • Optimizations: reduced allocations, cached widget state. • Target: 60fps. |
| **Review checklist** | ① Benchmark exists. ② p99 <16ms. ③ No allocations in hot path. ④ Flamegraph reviewed. ⑤ No test regressions. |
| **Risk** | Medium — performance tuning can introduce subtle bugs. |
| **Docs** | Document performance targets. |
| **Harness impact** | None — performance only. |

---

### Phase E: Navigation & Power Features (Weeks 29–38) — Make It Fast to Use

**Goal:** Keyboard shortcuts, search, session picker, slash commands, mouse, sidebar.

#### Week 29: Conversation Search (`/`)

| Field | Value |
|---|---|
| **Deliverable** | Incremental search highlighting matching text. `n`/`N` for next/prev match. |
| **Files touched** | `tui/app.rs (+80)`, `tui/widgets/search.rs` (created, 200 lines) |
| **New files** | `tui/widgets/search.rs` |
| **Test strategy** | `#[test] fn test_search_finds_text()`. `#[test] fn test_search_highlight()`. |
| **Merge criteria** | `/` opens search. Type filters. `n`/`N` navigates. `Escape` closes. |
| **Rollback strategy** | No search (use external grep). |
| **PR title** | `feat: conversation search with incremental highlighting` |
| **PR description** | • `/` opens search overlay. • Incremental filtering. • Match highlighting. • `n`/`N` navigation. |
| **Review checklist** | ① `/` opens search. ② Typing filters. ③ Matches highlighted. ④ `n`/`N` cycles. ⑤ Escape closes. |
| **Risk** | Low — string search + highlight. |
| **Docs** | Add search keybindings. |
| **Harness impact** | None. |

#### Week 30: Help Overlay (`?`)

| Field | Value |
|---|---|
| **Deliverable** | `?` help overlay: keyboard shortcuts panel showing all keybindings. |
| **Files touched** | `tui/widgets/help.rs` (created, 150 lines), `tui/app.rs (+20)` |
| **New files** | `tui/widgets/help.rs` |
| **Test strategy** | `#[test] fn test_help_overlay_renders()`. |
| **Merge criteria** | `?` toggles overlay. All keybindings listed. `Escape` or `?` closes. |
| **Rollback strategy** | No help overlay (document keybindings in README only). |
| **PR title** | `feat: keyboard shortcuts help overlay` |
| **PR description** | • `?` toggles help overlay. • All keybindings listed with descriptions. • Styled with boxes. |
| **Review checklist** | ① `?` opens. ② All keys listed. ③ Escape closes. ④ Doesn't block input. ⑤ Tests verify. |
| **Risk** | Low — static overlay. |
| **Docs** | Update keybindings table. |
| **Harness impact** | None. |

#### Week 31: Session Picker (`Ctrl+S`)

| Field | Value |
|---|---|
| **Deliverable** | `Ctrl+S` opens fuzzy-filterable list of saved sessions. Up/down arrows, Enter to switch. |
| **Files touched** | `tui/widgets/session_picker.rs` (created, 200 lines), `tui/app.rs (+40)` |
| **New files** | `tui/widgets/session_picker.rs` |
| **Test strategy** | `#[test] fn test_session_picker_lists_sessions()`. `#[test] fn test_session_picker_filters()`. |
| **Merge criteria** | `Ctrl+S` opens picker. Typing filters. Enter loads session. Escape cancels. |
| **Rollback strategy** | Use `/session list` slash command within TUI. |
| **PR title** | `feat: interactive session picker` |
| **PR description** | • `Ctrl+S` opens session list. • Fuzzy filter. • Up/down to navigate. • Enter to load. |
| **Review checklist** | ① Sessions listed. ② Filter works. ③ Enter loads. ④ Escape cancels. ⑤ Tests verify. |
| **Risk** | Low — list + filter pattern. |
| **Docs** | Document session picker. |
| **Harness impact** | None. |

#### Week 32: Slash Command Picker

| Field | Value |
|---|---|
| **Deliverable** | Type `/` in input to trigger slash command picker (like REPL). Commands execute within TUI (e.g., `/model`, `/permissions`). |
| **Files touched** | `tui/widgets/input.rs (+80)`, `tui/app.rs (+60)` |
| **New files** | None |
| **Test strategy** | `#[test] fn test_slash_picker_opens()`. `#[test] fn test_slash_command_executes()`. |
| **Merge criteria** | `/` opens picker. Tab completion. Enter executes. Commands like `/model` work. |
| **Rollback strategy** | No slash commands in TUI (use REPL for config changes). |
| **PR title** | `feat: slash command picker and execution in tui` |
| **PR description** | • `/` triggers slash command picker. • Tab completion. • In-TUI execution. • Share logic with REPL. |
| **Review checklist** | ① `/` opens picker. ② Tab completes. ③ Enter executes. ④ `/model` switches model. ⑤ Tests verify. |
| **Risk** | Medium — command execution can affect runtime state. |
| **Docs** | Document TUI slash commands. |
| **Harness impact** | None. |

#### Week 33: `/compact` with Toast

| Field | Value |
|---|---|
| **Deliverable** | `/compact` in TUI runs compaction, shows toast notification with tokens saved. |
| **Files touched** | `tui/app.rs (+40)`, `tui/widgets/toast.rs` (created, 80 lines) |
| **New files** | `tui/widgets/toast.rs` |
| **Test strategy** | `#[test] fn test_toast_shows_message()`. |
| **Merge criteria** | Toast appears for 3 seconds. Shows tokens saved. Does not block input. |
| **Rollback strategy** | Print compaction result in chat (no toast). |
| **PR title** | `feat: compact command with toast notification` |
| **PR description** | • `/compact` shows toast. • Auto-dismisses after 3s. • Non-blocking. |
| **Review checklist** | ① Toast visible. ② Message correct. ③ Auto-dismisses. ④ Input not blocked. ⑤ Tests verify. |
| **Risk** | Low — timed notification. |
| **Docs** | Document toast behavior. |
| **Harness impact** | None. |

#### Week 34: Copy to Clipboard

| Field | Value |
|---|---|
| **Deliverable** | `y` on a message or code block copies to clipboard via `arboard`. |
| **Files touched** | `Cargo.toml (+1)`, `tui/app.rs (+40)`, `tui/widgets/chat.rs (+30)` |
| **New files** | None |
| **Dependencies added** | `arboard = "3"` |
| **Test strategy** | `#[test] fn test_copy_event_sent()`. Full clipboard test is platform-dependent; mock the clipboard backend. |
| **Merge criteria** | `y` copies selected message. Toast confirms. Works on Linux (wlroots/X11), macOS, Windows. |
| **Rollback strategy** | No copy feature (use terminal selection). |
| **PR title** | `feat: copy message to clipboard` |
| **PR description** | • `y` copies message to clipboard. • `arboard` handles cross-platform paste. • Toast confirms. |
| **Review checklist** | ① `y` copies. ② Toast confirms. ③ Linux (X11/Wayland). ④ macOS. ⑤ Windows. |
| **Risk** | Low — `arboard` is mature. |
| **Docs** | Document copy keybinding. |
| **Harness impact** | None. |

#### Week 35: Mouse Support

| Field | Value |
|---|---|
| **Deliverable** | Click to expand tool results. Scroll wheel on conversation. Click to focus input. |
| **Files touched** | `tui/event.rs (+60)`, `tui/app.rs (+40)`, `tui/widgets/chat.rs (+20)` |
| **New files** | None |
| **Test strategy** | `#[test] fn test_mouse_scroll_updates_offset()`. Mouse click tests use `TestBackend` with synthetic mouse events. |
| **Merge criteria** | Mouse scroll works. Click on tool result toggles expand. Click on input focuses. |
| **Rollback strategy** | Keyboard-only (no mouse). |
| **PR title** | `feat: mouse support for tool expansion and scrolling` |
| **PR description** | • Scroll wheel scrolls conversation. • Click toggles tool expand. • Click focuses input. |
| **Review checklist** | ① Scroll wheel works. ② Click toggles expand. ③ Click focuses input. ④ Keyboard still works. ⑤ Tests verify. |
| **Risk** | Low — crossterm mouse events are standard. |
| **Docs** | Document mouse support. |
| **Harness impact** | None. |

#### Week 36: Internal Pager

| Field | Value |
|---|---|
| **Deliverable** | For long outputs (`/status`, `/config`), open full-screen pager with `j`/`k`/`q`. |
| **Files touched** | `tui/widgets/pager.rs` (created, 180 lines), `tui/app.rs (+40)` |
| **New files** | `tui/widgets/pager.rs` |
| **Test strategy** | `#[test] fn test_pager_scrolls()`. `#[test] fn test_pager_quits_on_q()`. |
| **Merge criteria** | Long outputs open pager. `j`/`k` scroll. `q` quits back to chat. |
| **Rollback strategy** | Dump long output directly in chat (scrolling mess). |
| **PR title** | `feat: internal pager for long outputs` |
| **PR description** | • Long outputs open pager overlay. • `j`/`k` scroll. • `q` returns to chat. |
| **Review checklist** | ① Pager opens for long output. ② `j`/`k` scroll. ③ `q` quits. ④ Line numbers optional. ⑤ Tests verify. |
| **Risk** | Low — scrollable text view. |
| **Docs** | Document pager keybindings. |
| **Harness impact** | None. |

#### Week 37: Optional Right Sidebar

| Field | Value |
|---|---|
| **Deliverable** | `Ctrl+P` toggles right sidebar showing active tool status or todo list. |
| **Files touched** | `tui/widgets/sidebar.rs` (created, 250 lines), `tui/app.rs (+60)` |
| **New files** | `tui/widgets/sidebar.rs` |
| **Test strategy** | `#[test] fn test_sidebar_toggles()`. `#[test] fn test_sidebar_shows_tools()`. |
| **Merge criteria** | Sidebar toggles on/off. Shows tool list. Doesn't break chat layout. |
| **Rollback strategy** | No sidebar (single-pane only). |
| **PR title** | `feat: optional right sidebar for tool status` |
| **PR description** | • `Ctrl+P` toggles sidebar. • Shows active tools and status. • Resizes chat area. |
| **Review checklist** | ① Toggle works. ② Tool status visible. ③ Chat area shrinks. ④ No layout breaks. ⑤ Tests verify. |
| **Risk** | Low — layout toggle. |
| **Docs** | Document sidebar. |
| **Harness impact** | None. |

#### Week 38: Custom Keybindings

| Field | Value |
|---|---|
| **Deliverable** | Read keybindings from `.claw.json`. Remap any action to any key chord. |
| **Files touched** | `tui/event.rs (+60)`, `tui/app.rs (+20)` |
| **New files** | `tui/keybindings.rs` (created, 150 lines) |
| **Test strategy** | `#[test] fn test_custom_keybinding_loaded()`. `#[test] fn test_keybinding_override()`. |
| **Merge criteria** | Config file read on startup. Overrides apply. Invalid config shows error in chat. |
| **Rollback strategy** | Hardcoded keybindings (ignore config). |
| **PR title** | `feat: user-configurable keybindings` |
| **PR description** | • Keybindings from `.claw.json`. • Override any action. • Validation with error messages. |
| **Review checklist** | ① Config loaded. ② Overrides apply. ③ Invalid config errors. ④ Defaults documented. ⑤ Tests verify. |
| **Risk** | Low — config parsing. |
| **Docs** | Document keybinding config format. |
| **Harness impact** | None. |

---

### Phase F: Polish & Hardening (Weeks 39–46) — Make It Production-Ready

**Goal:** Stable, tested, accessible, well-documented.

#### Week 39: Screen Reader Accessibility

| Field | Value |
|---|---|
| **Deliverable** | Screen reader support: `crossterm` title updates, status announcements via `SetTitle`. |
| **Files touched** | `tui/app.rs (+40)`, `tui/event.rs (+20)` |
| **New files** | None |
| **Test strategy** | `#[test] fn test_title_updates_on_message()`. |
| **Merge criteria** | Terminal title reflects app state. Status messages announced. |
| **Rollback strategy** | No accessibility features. |
| **PR title** | `feat: screen reader accessibility annotations` |
| **PR description** | • Terminal title updates. • Status announcements. • ARIA-like labels on widgets. |
| **Review checklist** | ① Title updates. ② Status announced. ③ No visual changes. ④ Tested with `orca` or `NVDA`. ⑤ Tests verify. |
| **Risk** | Low — title updates only. |
| **Docs** | Document accessibility features. |
| **Harness impact** | None. |

#### Week 40: Small Terminal Degradation

| Field | Value |
|---|---|
| **Deliverable** | If terminal <30 rows or <80 cols, show warning banner and disable sidebar. |
| **Files touched** | `tui/app.rs (+80)`, `tui/theme.rs (+20)` |
| **New files** | None |
| **Test strategy** | `#[test] fn test_small_terminal_warning()`. `#[test] fn test_sidebar_disabled_narrow()`. |
| **Merge criteria** | Warning visible on small terminals. Sidebar auto-disabled. Chat still functional. |
| **Rollback strategy** | No degradation (layout may break on small terminals). |
| **PR title** | `feat: small terminal graceful degradation` |
| **PR description** | • Warning on <30 rows or <80 cols. • Sidebar disabled. • Layout adjusted. |
| **Review checklist** | ① Warning visible. ② Sidebar disabled. ③ Chat scrollable. ④ No panic. ⑤ Tests verify. |
| **Risk** | Low — conditional layout. |
| **Docs** | Document minimum terminal size. |
| **Harness impact** | None. |

#### Week 41: Unicode Width Handling

| Field | Value |
|---|---|
| **Deliverable** | Use `unicode-width` for CJK and emoji so layouts don't break. |
| **Files touched** | `Cargo.toml (+1)`, `tui/widgets/chat.rs (+30)`, `tui/widgets/markdown.rs (+30)` |
| **New files** | None |
| **Dependencies added** | `unicode-width = "0.2"` |
| **Test strategy** | `#[test] fn test_cjk_width()`. `#[test] fn test_emoji_width()`. |
| **Merge criteria** | CJK characters take 2 columns. Emoji take 2 columns (on supported terminals). Layouts don't break. |
| **Rollback strategy** | Assume ASCII width (breaks on CJK/emoji). |
| **PR title** | `fix: unicode width for layout correctness` |
| **PR description** | • `unicode-width` for correct column counting. • CJK and emoji handled. • Layout integrity. |
| **Review checklist** | ① CJK correct. ② Emoji correct. ③ Layout intact. ④ No truncation. ⑤ Tests verify. |
| **Risk** | Low — well-tested crate. |
| **Docs** | Document unicode support. |
| **Harness impact** | None. |

#### Week 42: Snapshot Testing with `insta`

| Field | Value |
|---|---|
| **Deliverable** | `insta` snapshot tests for complex conversations: tables, code blocks, diffs. |
| **Files touched** | `Cargo.toml (+1, dev-dependency)`, `tests/snapshots/` (new dir) |
| **New files** | `tests/tui_snapshots.rs` (150 lines) |
| **Dependencies added** | `insta = "1"` (dev-dependency) |
| **Test strategy** | Render complex markdown to `TestBackend`, snapshot with `insta::assert_snapshot`. |
| **Merge criteria** | Snapshots committed. CI checks them. Any rendering change updates snapshot intentionally. |
| **Rollback strategy** | Remove snapshot tests (more manual QA needed). |
| **PR title** | `test: snapshot tests for tui rendering` |
| **PR description** | • `insta` snapshot tests for rendering. • Tables, code blocks, diffs covered. • CI enforces no drift. |
| **Review checklist** | ① Snapshots committed. ② CI checks. ③ `cargo insta review` documented. ④ Coverage: table, code, diff. ⑤ No test regressions. |
| **Risk** | Low — adds tests only. |
| **Docs** | Document snapshot workflow in `CONTRIBUTING.md`. |
| **Harness impact** | None. |

#### Week 43: Stress Testing

| Field | Value |
|---|---|
| **Deliverable** | 1,000-message session. No memory leaks, OOM, or frame drops. |
| **Files touched** | `benches/tui_stress.rs` (new) |
| **New files** | `benches/tui_stress.rs` |
| **Test strategy** | Criterion benchmark: create 1000 messages, render, measure memory and time. |
| **Merge criteria** | Memory stable (<100MB). Render time <100ms for 1000 messages. |
| **Rollback strategy** | Document performance limits. |
| **PR title** | `test: stress test for large sessions` |
| **PR description** | • 1000-message stress test. • Memory and time measured. • No leaks. • Documented limits. |
| **Review checklist** | ① 1000 messages created. ② Memory <100MB. ③ Render <100ms. ④ No leaks. ⑤ Documented. |
| **Risk** | Low — test only. |
| **Docs** | Document performance limits. |
| **Harness impact** | None. |

#### Week 44: Windows Testing

| Field | Value |
|---|---|
| **Deliverable** | Verified in PowerShell and Windows Terminal. Fix cursor/resize issues. |
| **Files touched** | `tui/event.rs (+20)`, `tui/mod.rs (+20)` |
| **New files** | None |
| **Test strategy** | Manual QA on Windows. Automated tests already run in CI (GitHub Actions `windows-latest`). |
| **Merge criteria** | `claw tui` works on Windows. Resize handled. No cursor glitches. |
| **Rollback strategy** | Document Windows as best-effort. |
| **PR title** | `fix: windows terminal compatibility` |
| **PR description** | • Windows Terminal compatibility verified. • Resize fixes. • Cursor fixes. • PowerShell tested. |
| **Review checklist** | ① Opens on Windows. ② Resize works. ③ No cursor artifacts. ④ CI passes. ⑤ PowerShell smoke test. |
| **Risk** | Medium — Windows terminal quirks are unpredictable. |
| **Docs** | Document Windows setup. |
| **Harness impact** | None. |

#### Week 45: TUI User Guide

| Field | Value |
|---|---|
| **Deliverable** | `docs/TUI-USAGE.md` — comprehensive user guide with ASCII art screenshots, keybindings, tips. |
| **Files touched** | `docs/TUI-USAGE.md` (created, 300 lines) |
| **New files** | `docs/TUI-USAGE.md` |
| **Test strategy** | None — documentation only. |
| **Merge criteria** | Document covers: installation, first run, keybindings, slash commands, tips, troubleshooting. |
| **Rollback strategy** | N/A — docs. |
| **PR title** | `docs: tui user guide` |
| **PR description** | • Comprehensive TUI guide. • ASCII art screenshots. • Keybindings table. • Tips and tricks. |
| **Review checklist** | ① All keys documented. ② ASCII art accurate. ③ Troubleshooting section. ④ Installation steps. ⑤ Proofread. |
| **Risk** | None — docs only. |
| **Docs** | Self-documenting. |
| **Harness impact** | None. |

#### Week 46: `claw doctor --tui` Diagnostic

| Field | Value |
|---|---|
| **Deliverable** | `claw doctor --tui` runs a self-test: opens TUI, verifies rendering, reports issues. |
| **Files touched** | `cli.rs (+30)`, `tui/mod.rs (+60)` |
| **New files** | None |
| **Test strategy** | `#[test] fn test_doctor_tui_smoke()`. |
| **Merge criteria** | `claw doctor --tui` reports TUI health. Useful for bug reports. |
| **Rollback strategy** | No `--tui` flag for doctor. |
| **PR title** | `feat: tui diagnostic mode in doctor` |
| **PR description** | • `--tui` flag for `claw doctor`. • Self-test rendering. • Reports terminal capabilities, colors, size. |
| **Review checklist** | ① `--tui` flag works. ② Reports terminal info. ③ Detects issues. ④ Useful output. ⑤ Tests verify. |
| **Risk** | Low — diagnostic only. |
| **Docs** | Document doctor TUI mode. |
| **Harness impact** | Add `doctor_tui` scenario. |

---

### Phase G: Advanced Features (Weeks 47–52) — Make It Best-in-Class

**Goal:** Features that differentiate claw from other AI CLI TUIs.

#### Week 47: Image Support (Vision Input)

| Field | Value |
|---|---|
| **Deliverable** | When vision input lands in the API (ROADMAP #220), render image attachments inline using `sixel` or `iTerm2` image protocols. Text fallback for unsupported terminals. |
| **Files touched** | `tui/widgets/image.rs` (created, 200 lines), `tui/app.rs (+20)` |
| **New files** | `tui/widgets/image.rs` |
| **Test strategy** | `#[test] fn test_image_fallback_text()`. `#[test] fn test_sixel_detection()`. |
| **Merge criteria** | Images render on iTerm2/kitty. Fallback text on other terminals. |
| **Rollback strategy** | Text placeholder: `[image: filename.png]`. |
| **PR title** | `feat: inline image rendering in tui` |
| **PR description** | • Sixel and iTerm2 image protocols. • Inline rendering. • Text fallback. |
| **Review checklist** | ① iTerm2 renders. ② kitty renders. ③ Fallback on standard terminal. ④ No panic. ⑤ Tests verify. |
| **Risk** | High — image protocols are complex and terminal-specific. |
| **Docs** | Document image support. |
| **Harness impact** | Deferred until vision API lands. |

#### Week 48: Notebook Editing

| Field | Value |
|---|---|
| **Deliverable** | `/notebook` opens notebook editor in split pane (VS Code-like notebook view). |
| **Files touched** | `tui/widgets/notebook.rs` (created, 250 lines), `tui/app.rs (+40)` |
| **New files** | `tui/widgets/notebook.rs` |
| **Test strategy** | `#[test] fn test_notebook_renders_cells()`. `#[test] fn test_notebook_cell_edit()`. |
| **Merge criteria** | Notebook cells render. Cells editable. Save/execute work. |
| **Rollback strategy** | No notebook editing in TUI (use REPL or external editor). |
| **PR title** | `feat: notebook editing in tui` |
| **PR description** | • Notebook cell view. • Edit cells in TUI. • Save back to file. |
| **Review checklist** | ① Cells render. ② Editable. ③ Save works. ④ Execute works. ⑤ Tests verify. |
| **Risk** | Medium — notebook format is complex. |
| **Docs** | Document notebook editing. |
| **Harness impact** | None. |

#### Week 49: Agent Team Dashboard

| Field | Value |
|---|---|
| **Deliverable** | `Ctrl+A` shows real-time view of running agents, their status, outputs — like `htop` for agents. |
| **Files touched** | `tui/widgets/agent_dashboard.rs` (created, 250 lines), `tui/app.rs (+40)` |
| **New files** | `tui/widgets/agent_dashboard.rs` |
| **Test strategy** | `#[test] fn test_dashboard_shows_agents()`. `#[test] fn test_dashboard_updates()`. |
| **Merge criteria** | Agents listed with status. Output streams visible. Sortable by status. |
| **Rollback strategy** | No dashboard (use `/agents` slash command). |
| **PR title** | `feat: agent team dashboard in tui` |
| **PR description** | • Real-time agent monitoring. • Like `htop` for agents. • Output streams. • Sortable. |
| **Review checklist** | ① Agents listed. ② Status visible. ③ Output streams. ④ Sort works. ⑤ Tests verify. |
| **Risk** | Medium — requires agent runtime integration. |
| **Docs** | Document dashboard. |
| **Harness impact** | None. |

#### Week 50: MCP Server Inspector

| Field | Value |
|---|---|
| **Deliverable** | `Ctrl+M` opens pane showing connected MCP servers, their tools, recent calls. |
| **Files touched** | `tui/widgets/mcp_inspector.rs` (created, 200 lines), `tui/app.rs (+20)` |
| **New files** | `tui/widgets/mcp_inspector.rs` |
| **Test strategy** | `#[test] fn test_mcp_inspector_shows_servers()`. |
| **Merge criteria** | MCP servers listed. Tools visible. Recent calls scrollable. |
| **Rollback strategy** | No inspector (use `/mcp` slash command). |
| **PR title** | `feat: mcp server inspector sidebar` |
| **PR description** | • MCP server list. • Tool inventory. • Recent call log. |
| **Review checklist** | ① Servers listed. ② Tools visible. ③ Calls logged. ④ Scrollable. ⑤ Tests verify. |
| **Risk** | Low — MCP data is already available in runtime. |
| **Docs** | Document MCP inspector. |
| **Harness impact** | None. |

#### Week 51: Teleport Mode (Fuzzy File Finder)

| Field | Value |
|---|---|
| **Deliverable** | `Ctrl+G` opens fuzzy file finder (like `fzf`). Jump to file, send `/read` tool call. |
| **Files touched** | `tui/widgets/teleport.rs` (created, 180 lines), `tui/app.rs (+40)` |
| **New files** | `tui/widgets/teleport.rs` |
| **Test strategy** | `#[test] fn test_teleport_lists_files()`. `#[test] fn test_teleport_selects_file()`. |
| **Merge criteria** | `Ctrl+G` opens finder. Typing filters. Enter selects file. `/read` sent automatically. |
| **Rollback strategy** | No teleport (use `/read` manually). |
| **PR title** | `feat: fuzzy file finder teleport in tui` |
| **PR description** | • `Ctrl+G` opens file finder. • Fuzzy filter. • Enter sends `/read`. |
| **Review checklist** | ① Files listed. ② Filter works. ③ Enter selects. ④ `/read` sent. ⑤ Tests verify. |
| **Risk** | Low — file listing + filter. |
| **Docs** | Document teleport mode. |
| **Harness impact** | None. |

#### Week 52: TUI as Default

| Field | Value |
|---|---|
| **Deliverable** | TUI is the default for `claw` when a TTY is detected. `--no-tui` or piped input uses REPL. Update ROADMAP.md. Cut release notes. |
| **Files touched** | `main.rs (+20)`, `cli.rs (+15)`, `ROADMAP.md (+5)`, `docs/RELEASE-NOTES.md` (new) |
| **New files** | `docs/RELEASE-NOTES.md` |
| **Test strategy** | `#[test] fn test_default_tui_on_tty()`. `#[test] fn test_repl_on_pipe()`. |
| **Merge criteria** | `claw` opens TUI on TTY. `claw --no-tui` opens REPL. Piped input uses REPL. |
| **Rollback strategy** | Make REPL default again (one-line change in `main.rs`). |
| **PR title** | `feat: make tui the default interactive mode` |
| **PR description** | • TUI is default when TTY detected. • `--no-tui` for REPL. • Piped input uses REPL. • Release notes. |
| **Review checklist** | ① TTY → TUI. ② `--no-tui` → REPL. ③ Pipe → REPL. ④ Release notes written. ⑤ ROADMAP updated. |
| **Risk** | Low — behavioral toggle only. |
| **Docs** | Update all docs, release notes. |
| **Harness impact** | Update harness for default TUI. |

---

## 4. Implementation Specs (Weeks 1–12 in Detail)

### Week 1: Extract `app.rs` — Detailed Spec

**Function signatures to preserve:**

```rust
// app.rs
pub struct LiveCli {
    pub client: Arc<ConversationRuntime>,
    pub session: Session,
    pub config: RuntimeConfig,
    pub permission_mode: PermissionMode,
    pub tool_executor: Arc<dyn ToolExecutor>,
}

impl LiveCli {
    pub fn new(/* ... */) -> Self;
    pub fn run(&mut self) -> Result<(), RuntimeError>;
    pub fn run_turn(&mut self, prompt: &str) -> Result<TurnResult, RuntimeError>;
}
```

**Extraction steps:**
1. Copy `main.rs` to `app.rs`.
2. In `app.rs`, remove `fn main()` and arg parsing.
3. In `main.rs`, remove `LiveCli` struct and impl, keep only `fn main()` and `let mut cli = LiveCli::new(...); cli.run();`.
4. Add `mod app;` and `use app::LiveCli;` in `main.rs`.
5. Verify `cargo check`.
6. Run tests.
7. Verify `main.rs` <200 lines.

**Test preservation checklist:**
- [ ] `test_repl_smoke` → still in `app.rs`
- [ ] `test_streaming_output` → still in `app.rs`
- [ ] `test_tool_call_display` → still in `app.rs`
- [ ] `test_permission_prompt` → still in `app.rs`
- [ ] `test_session_save` → still in `app.rs`

---

### Week 4: TUI Stub — Detailed Spec

```rust
// cli.rs
pub enum CliAction {
    // ... existing variants ...
    Tui,
}

// In main.rs match on CliAction::Tui:
CliAction::Tui => {
    tui::run()?;
}
```

```rust
// tui/mod.rs
use std::io;

pub fn run() -> io::Result<()> {
    eprintln!("TUI mode coming soon.");
    Ok(())
}
```

**Integration test:**
```rust
// tests/tui_smoke.rs
use std::process::Command;

#[test]
fn test_tui_subcommand_exists() {
    let output = Command::new("cargo")
        .args(["run", "-p", "rusty-claude-cli", "--", "tui", "--help"])
        .output()
        .expect("cargo run failed");
    assert!(output.status.success(), "claw tui --help should succeed");
}
```

---

### Week 6: Ratatui Skeleton — Detailed Spec

```rust
// tui/mod.rs
use std::io;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    widgets::{Block, Borders, Paragraph},
    Terminal, Frame,
};

pub fn run() -> io::Result<()> {
    ratatui::init();
    let result = run_app();
    ratatui::restore();
    result
}

fn run_app() -> io::Result<()> {
    let mut terminal = ratatui::try_init()?;
    loop {
        terminal.draw(|f| ui(f))?;
        if let Event::Key(KeyEvent { code: KeyCode::Char('q'), .. }) = event::read()? {
            return Ok(());
        }
    }
}

fn ui(frame: &mut Frame) {
    let area = frame.area();
    let paragraph = Paragraph::new("Hello, claw")
        .block(Block::default().borders(Borders::ALL).title("claw TUI"));
    frame.render_widget(paragraph, area);
}
```

**Cargo.toml changes:**
```toml
[dependencies]
# existing deps ...
ratatui = "0.29"
crossterm = { version = "0.28", features = ["event-stream"] }
```

**TestBackend test:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_tui_init_buffer() {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f)).unwrap();
        let buffer = terminal.backend().buffer();
        // "Hello, claw" is at least 11 chars
        let content: String = buffer.content.iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Hello, claw"));
    }
}
```

---

### Week 9: Composer Widget — Detailed Spec

```rust
// tui/widgets/input.rs
use tui_textarea::TextArea;

pub struct ComposerWidget {
    textarea: TextArea<'static>,
    history: Vec<String>,
    history_index: Option<usize>,
}

impl ComposerWidget {
    pub fn new() -> Self;
    pub fn handle_event(&mut self, key: KeyEvent) -> ComposerAction;
    pub fn text(&self) -> &str;
    pub fn clear(&mut self);
}

pub enum ComposerAction {
    None,
    Submit(String),
    HistoryPrev,
    HistoryNext,
}
```

**Key mappings:**
- `Enter` → `Submit(text)`
- `Shift+Enter` → insert newline (handled by `TextArea`)
- `Ctrl+C` → clear
- `Up` → `HistoryPrev`
- `Down` → `HistoryNext`

---

## 5. Quality Assurance & Testing

### 5.1 Unit Test Strategy

| Module | Test Approach | Backend |
|---|---|---|
| `tui/event.rs` | Key event translation | No backend (pure functions) |
| `tui/app.rs` | State transitions | `TestBackend` |
| `tui/widgets/chat.rs` | Message rendering, scroll | `TestBackend` |
| `tui/widgets/input.rs` | Key handling, history | `TestBackend` |
| `tui/widgets/markdown.rs` | Markdown parsing → buffer | `TestBackend` |
| `tui/widgets/status_bar.rs` | Layout, content | `TestBackend` |
| `tui/widgets/tool_call.rs` | Lifecycle, expand | `TestBackend` |
| `tui/widgets/permission.rs` | Modal rendering | `TestBackend` |

### 5.2 Integration Test Strategy

```rust
// tests/tui_integration.rs
#[tokio::test]
async fn test_full_conversation() {
    let mut app = App::new(mock_runtime());
    // Simulate user typing "hello"
    app.handle_event(AppEvent::Key(KeyCode::Char('h')));
    // ... simulate Enter ...
    app.handle_event(AppEvent::AssistantDelta("Hi!".to_string()));
    // Assert message count == 2
    assert_eq!(app.messages.len(), 2);
}
```

### 5.3 Mock Parity Harness — New Scenarios

| Scenario | Week Added | What It Tests |
|---|---|---|
| `tui_stub_smoke` | 4 | `claw tui` exits cleanly |
| `tui_smoke_open_close` | 6 | Alternate screen opens and closes |
| `tui_full_layout_smoke` | 12 | Full layout renders with mock data |
| `tui_streaming_mock` | 13 | Event loop processes streaming events |
| `tui_streaming_text` | 14 | Text appears in chat |
| `tui_session_save_resume` | 15 | Session persistence |
| `tui_tool_call_display` | 16 | Tool call inline rendering |
| `tui_permission_prompt` | 17 | Modal overlay |
| `tui_error_handling` | 18 | Error messages in chat |
| `tui_multi_turn` | 19 | Multiple conversation turns |
| `tui_markdown_render` | 21 | Markdown widget |
| `doctor_tui` | 46 | `claw doctor --tui` |

### 5.4 CI Strategy

Add to `.github/workflows/rust-ci.yml`:

```yaml
- name: Build with TUI feature
  run: cargo build --workspace --features tui

- name: Build without TUI feature
  run: cargo build --workspace

- name: Run TUI tests
  run: cargo test --workspace --features tui

- name: Run benchmarks
  run: cargo bench --workspace --features tui
  if: github.ref == 'refs/heads/main'
```

---

## 6. Risk Management & Rollback

### 6.1 Phase Rollback Matrix

| Phase | Rollback Command | Functionality Lost | Time to Rollback |
|---|---|---|---|
| A (Weeks 1–6) | `git revert <week-6-commit>` | TUI stub, `claw tui` command | 5 min |
| B (Weeks 7–12) | `git checkout HEAD~6 -- tui/` | Full-screen TUI | 10 min |
| C (Weeks 13–20) | `git revert <week-20-commit>` | Live conversation, streaming | 15 min |
| D (Weeks 21–28) | Disable markdown widget, use plain text | Rich rendering | 20 min |
| E (Weeks 29–38) | Disable search, sidebar, mouse | Power features | 30 min |
| F (Weeks 39–46) | Remove snapshot tests, docs | Tests, docs | 10 min |
| G (Weeks 47–52) | `--no-tui` default | Default-on TUI | 5 min |

### 6.2 Nuclear Option

If the TUI introduces critical instability:

```bash
# Remove TUI code, keep extractions
git checkout main -- rust/crates/rusty-claude-cli/src/tui/
git revert HEAD~N..HEAD  # where N = weeks since Week 4

# The REPL and all other functionality are untouched
# `claw tui` command is removed
# `app.rs`, `format.rs`, `session_mgr.rs` remain (pure wins)
```

---

## 7. Reviewer Guide & PR Template

### 7.1 How to Review a TUI PR

1. **Check the week number** — does this PR match the week's deliverable? If not, ask for scope justification.
2. **Verify line count** — is it ≤400 lines? If not, ask for a split.
3. **Check tests** — does every new file have at least one `#[cfg(test)]` block with a `TestBackend` assertion?
4. **Check no REPL changes** — does the PR touch `input.rs`, `render.rs`, or the REPL loop in ways that could break existing behavior?
5. **Run `cargo test --workspace`** — does it pass?

### 7.2 PR Template for TUI PRs

```markdown
## Week X: [Deliverable Name]

### What changed
- [One-line summary]

### Files
- `tui/foo.rs` (+N/-M)
- `app.rs` (+N/-M)

### Tests
- [ ] Every new module has a `TestBackend` test
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` clean
- [ ] `cargo fmt --all --check` passes

### Verification
```bash
cd rust
cargo test --workspace
cargo run -p rusty-claude-cli -- tui --help
```

### Rollback
If this PR causes issues: `git revert HEAD`

### Risk
[low/medium/high] — [one sentence]
```

---

## 8. Appendices

### A. Dependency Matrix

| Crate | Version | License | Purpose | Added Week |
|---|---|---|---|---|
| `ratatui` | `0.29` | MIT | TUI framework, layout engine, widgets | 6 |
| `crossterm` (event-stream) | `0.28` | MIT | Already present; event-stream feature added for async events | 6 |
| `tui-textarea` | `0.7` | MIT | Multi-line text input widget | 9 |
| `unicode-width` | `0.2` | MIT | Correct column counting for CJK/emoji | 41 |
| `arboard` | `3` | MIT | Cross-platform clipboard | 34 |
| `insta` (dev) | `1` | Apache-2.0 | Snapshot testing | 42 |

**MSRV impact:** None. All crates support Rust 1.70+.

### B. Performance Budget

| Metric | Target | Measured At |
|---|---|---|
| Frame rate | 60 FPS (p99 <16ms) | Week 28 |
| Memory per 1000 messages | <100 MB | Week 43 |
| Binary size (without TUI) | No change | Week 20 |
| Binary size (with TUI) | +2–3 MB | Week 20 |
| Clean build time | <5 min | Week 6 |
| Incremental build time | <30 sec | Week 6 |

### C. Glossary

| Term | Definition |
|---|---|
| **TUI** | Terminal User Interface — full-screen, alternate-buffer terminal application |
| **REPL** | Read-Eval-Print Loop — the existing line-based interactive shell |
| **Monolith** | The 11,282-line `main.rs` containing all CLI logic |
| **TestBackend** | ratatui's in-memory terminal buffer for headless testing |
| **EventBroker** | Bridges crossterm events to TUI application events |
| **Composer** | The text input widget at the bottom of the chat |
| **Streaming cell** | The in-progress assistant message being rendered incrementally |

---

*Generated: 2026-06-12 | Author: TheArchitectit | Review status: Draft for upstream PR*  
*Based on post-mortem of `feat/ui-hardening`, `fix/ui-parity`, `feat/uiux-redesign`, `rcc/ui-polish`, and `rust/TUI-ENHANCEMENT-PLAN.md`*
