# codex-rs/tui Architecture Deep-Dive

**Source:** `https://github.com/openai/codex` — `codex-rs/tui/`
**Date analyzed:** 2026-06-12
**Purpose:** Extract patterns for claw-code CLI (Rust TUI AI coding assistant)

---

## Table of Contents

1. [Crate Versions & Dependencies](#1-crate-versions--dependencies)
2. [Event Loop Architecture](#2-event-loop-architecture)
3. [Streaming Assistant Response Rendering](#3-streaming-assistant-response-rendering)
4. [ChatWidget / Message List](#4-chatwidget--message-list)
5. [Input Composer](#5-input-composer)
6. [Tool Call Execution & Inline Display](#6-tool-call-execution--inline-display)
7. [Alternate Screen Enter/Exit](#7-alternate-screen-enterexit)
8. [Output Bleeding Prevention](#8-output-bleeding-prevention)
9. [Patterns to Adopt](#9-patterns-to-adopt)
10. [Anti-Patterns & Limitations to Avoid](#10-anti-patterns--limitations-to-avoid)

---

## 1. Crate Versions & Dependencies

| Crate | Version | Notes |
|-------|---------|-------|
| **ratatui** | Forked: `nornagon/ratatui` rev `9b2ad12` | Based on 0.29.0 with custom patches; uses `unstable-backend-writer`, `unstable-rendered-line-info`, `unstable-widget-ref`, `scrolling-regions` features |
| **crossterm** | Forked: `nornagon/crossterm` rev `87db8bf` | Based on 0.28.1 with custom patches; uses `bracketed-paste`, `event-stream` features |
| **tokio** | `1` (workspace) | `rt-multi-thread`, `io-std`, `macros`, `process`, `signal`, `test-util`, `time` |
| **tokio-stream** | `0.1.18` | `sync` feature |
| **textwrap** | `0.16.2` | Word wrapping |
| **unicode-segmentation** | `1.12.0` | Word boundary detection for textarea |
| **unicode-width** | `0.2` | CJK-aware width calculations |
| **pulldown-cmark** | `0.10` | Markdown rendering |
| **syntect** | `5` | Syntax highlighting |
| **two-face** | `0.5` | Syntect theme support |
| **image** | workspace | `jpeg`, `png`, `gif`, `webp` features (for ambient pet images) |
| **arboard** | workspace | Clipboard (not on Android) |

**Critical observation:** codex-rs forks both ratatui and crossterm. The ratatui fork adds `unstable-backend-writer` for direct buffer access and `unstable-rendered-line-info` for paragraph line-count inspection. The crossterm fork likely adds `SynchronizedUpdate` support or fixes stdin-stealing bugs. **claw-code should plan to vendor or fork these crates too**, or wait for upstream stabilization.

---

## 2. Event Loop Architecture

### Top-Level Loop

The main event loop lives in `App::run()` (`app.rs` ~line 1148):

```rust
tokio::select! {
    tui_events = tui_events.next() => { /* handle key/resize/paste/draw */ },
    app_events = self.app_event_rx.recv() => { /* handle protocol/app events */ },
}
```

This is a standard `tokio::select!` over two async streams: **TUI events** (from crossterm + draw notifications) and **app events** (protocol messages, streaming deltas, approval requests, etc.).

### TUI Event Stream (`tui/event_stream.rs`)

The `TuiEventStream` implements `tokio_stream::Stream` with a **round-robin** polling strategy between:
1. **Draw events** — from a `broadcast::Receiver<()>` (one draw notification per scheduled frame)
2. **Crossterm events** — from a shared `EventBroker` wrapping `crossterm::EventStream`

```rust
fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let draw_first = self.poll_draw_first;
    self.poll_draw_first = !self.poll_draw_first;  // Round-robin fairness
    // ... poll both streams alternating priority
}
```

Key mappings (`map_crossterm_event`):
- `Event::Key` → `TuiEvent::Key`
- `Event::Resize` → `TuiEvent::Resize`
- `Event::Paste` → `TuiEvent::Paste`
- `Event::FocusGained` → `TuiEvent::Draw` (triggers palette re-query)
- `Event::FocusLost` → `None` (dropped)
- Mouse events → `None` (dropped entirely!)
- SIGTSTP (Ctrl+Z) → suspend handled in-stream, returns `TuiEvent::Draw`

### EventBroker — Pause/Resume Pattern

The `EventBroker` wraps crossterm's `EventStream` in a `Mutex<EventBrokerState<S>>`:

```rust
enum EventBrokerState<S: EventSource> {
    Paused,     // Stream dropped — stdin fully released
    Start,      // Will create new EventStream on next poll
    Running(S), // Active event source
}
```

**Why drop instead of just stop polling?** The doc comment explains clearly: crossterm's reader thread continues consuming stdin even when the stream is in a pending state, stealing input from child processes (like `vim`). Dropping the stream is the only safe way to fully relinquish stdin.

### Frame Rate Limiting (`tui/frame_rate_limiter.rs`)

```rust
const MIN_FRAME_INTERVAL: Duration = Duration::from_nanos(8_333_334); // ~120 FPS
```

The `FrameRateLimiter` enforces this ceiling — any draw request within `MIN_FRAME_INTERVAL` of the last draw is suppressed. This prevents wasted GPU/CPU on frames faster than the terminal can display.

### Frame Scheduler Actor (`tui/frame_requester.rs`)

An actor pattern (cf. [Alice Ryhl's actors-with-tokio](https://ryhl.io/blog/actors-with-tokio/)):

```
FrameRequester (mpsc::UnboundedSender<Instant>)
    → FrameScheduler (actor task)
        → coalesces requests
        → broadcast::Sender<()> (draw notification)
```

The `FrameScheduler` receives frame-request timestamps via an unbounded channel and **coalesces** them: if multiple components request a redraw within the same frame interval, only one draw notification is broadcast. This is effectively a software VSync.

### AppEvent Channel

`AppEventSender` wraps `tokio::sync::mpsc::UnboundedSender<AppEvent>`. The app uses an **unbounded** channel for app events — no backpressure. This is acceptable because the consumer (the main loop) processes events as fast as it can render frames, and the producers are protocol handlers that shouldn't block.

---

## 3. Streaming Assistant Response Rendering

This is the most sophisticated part of codex-rs/tui. The streaming pipeline has four layers:

### Layer 1: StreamState (queue primitives)

`StreamState` (`streaming/mod.rs`) owns:
- `MarkdownStreamCollector` — newline-gated source accumulator
- `VecDeque<QueuedLine>` — FIFO queue of committed render lines with `enqueued_at: Instant` timestamps

Key invariant: **all drains pop from the front**, and enqueue records arrival timestamps so policy code can reason about queue age.

### Layer 2: StreamCore / StreamController (two-region model)

`StreamController` (`streaming/controller.rs`) partitions rendered markdown into:

1. **Stable region** — committed to scrollback via the animation queue. These lines are immutable.
2. **Tail region** — mutable, displayed in the `active_cell` slot. These lines can change on every delta.

The boundary is tracked by `enqueued_stable_len` vs `emitted_stable_len` vs `rendered_lines.len()`:

```
Invariant: emitted_stable_len <= enqueued_stable_len <= rendered_lines.len()
```

The tail starts at `enqueued_stable_len` (NOT `emitted_stable_len`), which prevents duplicate content — lines queued but not yet emitted won't reappear in the active cell.

### Layer 3: Chunking Policy (`streaming/chunking.rs`)

`AdaptiveChunkingPolicy` decides how many lines to drain per commit tick:
- **Steady mode** — drain one line per tick (smooth animation)
- **CatchUp mode** — batch drain when queue pressure exceeds threshold (oldest queued age > target)

This creates a typewriter-like animation that smoothly catches up if the stream outruns the display.

### Layer 4: Commit Tick Orchestrator (`streaming/commit_tick.rs`)

`run_commit_tick()` bridges policy → controller drains:
1. Collect `QueueSnapshot` (total queued lines + oldest age)
2. Ask `AdaptiveChunkingPolicy` for a `ChunkingDecision`
3. Apply the `DrainPlan` (Single or Batch) to both `StreamController` and `PlanStreamController`
4. Return `CommitTickOutput` with emitted `HistoryCell`s

### Table Holdback

A particularly clever mechanism: when a markdown pipe table is detected in the stream (header + delimiter pair), the **entire table region is withheld as mutable tail** until the stream finalizes. This prevents visual glitching where adding a row reshapes all prior columns.

`TableHoldbackScanner` incrementally scans the raw source and transitions through states `None → PendingHeader → Confirmed`. The `active_tail_budget_lines()` method returns the number of rendered tail lines to withhold based on the detected table boundary.

### Finalization & Consolidation

When a stream finishes:
1. `StreamController::finalize()` drains remaining lines, returns raw source
2. `App::handle_consolidate_agent_message()` replaces the trailing run of streaming `AgentMessageCell`s with a single source-backed `AgentMarkdownCell`
3. This canonical cell owns the raw markdown and can re-render at any width for resize reflow

---

## 4. ChatWidget / Message List

### Architecture

`ChatWidget` (`chatwidget.rs`, ~2030 lines) is the main chat surface. It:
- Consumes protocol events
- Builds and updates `HistoryCell`s
- Drives rendering via the `Renderable` trait

### Committed vs Active Cells

The UI has two kinds of cells:
- **Committed cells** (`transcript_cells: Vec<Arc<dyn HistoryCell>>`) — finalized, stored in scrollback
- **Active cell** (`transcript.active_cell: Option<Box<dyn HistoryCell>>`) — mutable in-place, represents streaming output or a coalesced exec/tool group

### Rendering Pipeline

`ChatWidget` implements `Renderable` (in `chatwidget/rendering.rs`):

```rust
impl Renderable for ChatWidget {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.as_renderable().render(area, buf);
    }
    fn desired_height(&self, width: u16) -> u16 {
        self.as_renderable().desired_height(width)
    }
}
```

`as_renderable()` builds a `FlexRenderable`:
1. Active cell (flex=1, grows to fill)
2. Active hook cell (flex=0)
3. Pending token activity output (flex=1)
4. Bottom pane / composer (flex=0, fixed height)

`FlexRenderable` is inspired by Flutter's Flex widget — children with `flex > 0` share remaining space proportionally.

### Scrolling Strategy

**No virtualization!** The `TranscriptAreaRenderable` renders a `Paragraph::scroll((y, 0))` where `y` is the overflow count:

```rust
fn render(&self, area: Rect, buf: &mut Buffer) {
    let lines = self.child.display_lines(area.width);
    let paragraph = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
    let y = paragraph.line_count(area.width).saturating_sub(area.height as usize);
    Clear.render(area, buf);
    paragraph.scroll((y as u16, 0)).render(area, buf);
}
```

This means **the entire transcript is re-rendered every frame**. The scroll position is always "pinned to bottom" — there's no arbitrary scroll offset. This is a significant limitation for long conversations.

### Transcript Overlay (Ctrl+T)

A separate overlay view shows committed cells plus a cached live-tail from the active cell. Cache invalidation uses `active_cell_transcript_key()` — a key that changes when the active cell mutates or its output is time-dependent.

### HistoryCell Trait

```rust
trait HistoryCell: Send + Sync {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>>;
    fn raw_lines(&self) -> Vec<Line<'static>>;
    fn display_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine>;
    fn transcript_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine>;
    fn desired_height(&self, width: u16) -> u16;
    // ...
}
```

Every cell computes its own height as a function of width. This enables full resize reflow — each cell can re-wrap when the terminal width changes.

### CompositeHistoryCell

Cells can be composed: `CompositeHistoryCell` concatenates multiple `HistoryCell`s with blank-line separators. This allows building complex entries (header + body + footer) from simple parts.

---

## 5. Input Composer

### Custom TextArea (NOT tui-textarea!)

codex-rs built their own `TextArea` in `bottom_pane/textarea.rs` with:

- **Wrap cache** — `WrapCache` stores wrapped lines keyed by width, avoiding re-wrap on every keystroke
- **Kill buffer** — `KillBufferKind::{Characterwise, Linewise}` for clipboard-like kill/yank
- **Vim mode** — Full `VimMode` with `VimNormalKeymap`, `VimOperatorKeymap`, `VimTextObjectKeymap`
- **Element placeholders** — Special rendering for `@mentions` and other inline elements
- **Keymap indirection** — `RuntimeKeymap` → `EditorKeymap` allows different keybinding schemes
- **Word boundaries** — Uses `unicode_segmentation` for proper Unicode word movement

### ChatComposer (`bottom_pane/chat_composer.rs`)

The composer wraps `TextArea` and adds:
- **Popup routing** — Slash commands, file search, `@mentions` each have their own popup overlay
- **History navigation** — Persistent (across sessions) + local (within session) input history
- **Ctrl+R reverse search** — Interactive search through history
- **Multi-line input** — Enter submits (with Shift+Enter for newline)

### Bottom Pane Layered Input Routing (`bottom_pane/mod.rs`)

Input events route through a layered system:
1. **Active view** (popups like slash-command picker, file search) → if consumed, stop
2. **Composer** → text editing, submission
3. **ChatWidget** → interrupt (Ctrl+C), quit (Ctrl+D)

Key handling:
- `Ctrl+C` while composing → clear input; during agent turn → interrupt
- `Ctrl+D` on empty input → quit
- Time-based redraw scheduling for transient views

---

## 6. Tool Call Execution & Inline Display

### Tool Call Representation

Tool calls are represented as specialized `HistoryCell` implementations:

- **`UnifiedExecInteractionCell`** (`history_cell/exec.rs`) — background terminal commands with command display and stdin
- **`UnifiedExecProcessesCell`** — lists of running background terminals with recent output chunks
- **`McpToolCallCell`** (`history_cell/mcp.rs`) — MCP (Model Context Protocol) tool invocations
- **`McpInvocation`** — individual MCP tool call cells
- **Approval cells** (`history_cell/approvals.rs`) — `ApprovalDecisionCell` with symbol + summary for approved/denied/timeout
- **Patch cells** (`history_cell/patches.rs`) — file change diff display

### Coalescing

Multiple exec tool calls within a single agent turn are **coalesced** into a single active cell that can mutate in place. This avoids visual noise from many small cells appearing sequentially.

### Approval Flow

1. Agent requests approval (exec command, file write, network access)
2. `AppEvent::ApprovalRequest` arrives
3. Bottom pane shows approval UI with key hints
4. User approves/denies → `ReviewDecision` cell added to transcript
5. Display shows: `✓ User approved codex to run: npm test` or `✗ User denied`

### Diff Rendering

`diff_render.rs` (2481 lines!) provides inline diff display for file changes with syntax highlighting via `syntect`/`two-face`. This is the largest single rendering module — a full diff viewer built from scratch.

---

## 7. Alternate Screen Enter/Exit

### Entry (`Tui::enter_alt_screen`)

```rust
fn enter_alt_screen(&mut self) -> Result<()> {
    if !self.alt_screen_enabled { return Ok(()); }
    execute!(self.terminal.backend_mut(), EnterAlternateScreen);
    execute!(self.terminal.backend_mut(), EnableAlternateScroll);
    // Save inline viewport, expand to full terminal
    self.alt_saved_viewport = Some(self.terminal.viewport_area);
    self.terminal.set_viewport_area(Rect::new(0, 0, size.width, size.height));
    self.terminal.clear();
    self.alt_screen_active.store(true, Ordering::Relaxed);
}
```

### Exit (`Tui::leave_alt_screen`)

```rust
fn leave_alt_screen(&mut self) -> Result<()> {
    if !self.alt_screen_enabled { return Ok(()); }
    execute!(self.terminal.backend_mut(), DisableAlternateScroll);
    execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
    // Restore previously saved inline viewport
    if let Some(saved) = self.alt_saved_viewport.take() {
        self.terminal.set_viewport_area(saved);
    }
    self.alt_screen_active.store(false, Ordering::Relaxed);
}
```

### External Program Handoff (`Tui::with_restored`)

This is the critical code path for spawning editors, pagers, etc.:

```rust
async fn with_restored<R, F, Fut>(&mut self, mode: RestoreMode, f: F) -> R {
    self.pause_events();               // Drop crossterm EventStream → release stdin
    
    let was_alt_screen = self.is_alt_screen_active();
    if was_alt_screen {
        self.leave_alt_screen();       // Return to main screen
    }
    
    mode.restore()?;                   // Restore terminal modes (disable raw mode, etc.)
    terminal_stderr::pause()?;         // Restore stderr (undo dup2 suppression)
    
    let output = f().await;            // Run external program
    
    terminal_stderr::resume()?;        // Re-suppress stderr
    set_modes()?;                      // Re-enable raw mode, etc.
    flush_terminal_input_buffer();     // Clear stale keypresses
    
    if was_alt_screen {
        self.enter_alt_screen();       // Return to alt screen
    }
    
    self.resume_events();              // Recreate crossterm EventStream
    output
}
```

**Key insight:** The pause/resume of the EventBroker is essential. If you merely stop polling crossterm's stream, its internal reader thread continues consuming stdin, stealing input from the child process.

### SIGTSTP (Ctrl+Z) Handling (`tui/job_control.rs`)

On Unix:
1. `SuspendContext` stores cursor position via `Arc<AtomicU16>`
2. On SIGTSTP, leaves alt screen, restores terminal modes, sends SIGSTOP to self
3. On SIGCONT (resume), checks `ResumeAction`:
   - `Inline` — restore to inline mode (no alt screen)
   - `AltScreen` — re-enter alternate screen
4. Flushes terminal input buffer after resume

---

## 8. Output Bleeding Prevention

This is a **dual-layer defense** — the #1 problem for TUI apps:

### Layer 1: Compile-Time Prevention (`lib.rs`)

```rust
#![deny(clippy::print_stdout, clippy::print_stderr)]
```

This makes `print!()` / `eprint!()` a **compile error**. Any accidental stdout/stderr write from the TUI binary itself is caught at build time.

### Layer 2: macOS stderr suppression (`tui/terminal_stderr.rs`)

On macOS, system frameworks (Security framework, Keychain, WebKit) write to stderr without going through the app's logging framework. This causes output to appear in the middle of the TUI.

The solution: **redirect fd 2 to `/dev/null`** via `dup2()` while the TUI is active:

```rust
static STDERR_STATE: Mutex<StderrState> = ...;

pub fn resume() -> Result<()> {  // TUI is starting — suppress stderr
    let saved = dup(2)?;          // Save original stderr
    let devnull = open("/dev/null", ...)?;
    dup2(devnull, 2)?;            // Redirect fd 2 → /dev/null
    // ... store saved fd for later restoration
}

pub fn pause() -> Result<()> {   // Child process needs stderr — restore it
    dup2(saved_fd, 2)?;          // Restore original stderr
}
```

`TerminalStderrGuard` provides RAII semantics — the guard restores stderr on drop.

### Layer 3: Synchronized Update (crossterm)

The forked crossterm adds `SynchronizedUpdate` support — this wraps each frame's terminal writes in a synchronized update block, telling the terminal emulator to apply all changes atomically. This prevents partial-frame tearing.

### Layer 4: Event Stream Drop/Recreate

As described above, the `EventBroker` drops crossterm's `EventStream` before handing the terminal to child processes. This prevents crossterm's internal reader thread from writing escape sequences to stdout while the child process is running.

---

## 9. Patterns to Adopt

### ✅ 1. Frame Scheduler Actor with Coalescing

The `FrameRequester` → `FrameScheduler` → `broadcast::Sender<()>` pattern is elegant:
- Multiple components can request redraws without coordination
- The scheduler coalesces requests within the same frame interval
- The 120 FPS ceiling prevents wasted work

**Adopt this** for claw-code. It's cleaner than having every component call `terminal.draw()` directly.

### ✅ 2. EventBroker Pause/Resume for External Programs

The drop/recreate pattern for crossterm's EventStream is essential correctness:
- **Must adopt** — without this, spawned editors/pagers will have missing keystrokes
- The `watch::Sender<()>` for resumption notification is clean

### ✅ 3. Compile-Time Output Bleeding Prevention

```rust
#![deny(clippy::print_stdout, clippy::print_stderr)]
```

Zero-cost, high-value. **Must adopt.**

### ✅ 4. stderr dup2 Suppression (macOS)

The RAII guard pattern with `dup2()` is the only reliable way to handle system framework stderr on macOS. **Adopt for macOS builds.**

### ✅ 5. Two-Region Streaming Model

The stable/tail partition is the right abstraction for streaming AI responses:
- Committed lines are immutable → cache-friendly
- Tail is mutable → handles streaming updates without scrollback API
- Table holdback prevents visual glitching

**Adopt this architecture** for claw-code's streaming pipeline.

### ✅ 6. HistoryCell Trait with Width-Dependent Height

Making every cell compute its own desired height as a function of width enables:
- Clean resize reflow without stored scroll offsets
- Each cell re-wraps independently
- `desired_height()` is cheap (often cached)

**Adopt the trait pattern.**

### ✅ 7. Custom FlexRenderable / ColumnRenderable Layout

Building a Flutter-style flex layout system instead of using ratatui's built-in layout is more flexible for a chat-style TUI:
- Flex children grow/shrink proportionally
- Fixed children get their natural height
- No constraint solver overhead

**Adopt the FlexRenderable pattern.**

### ✅ 8. SynchronizedUpdate for Atomic Frame Writes

**Must adopt** — prevents partial-frame visual tearing that users notice immediately.

### ✅ 9. Flush Terminal Input Buffer After Resume

After returning from an external program (editor, pager), stale keypresses in the terminal's input buffer can cause spurious events. codex-rs explicitly flushes this buffer.

### ✅ 10. Transcript Consolidation

Replacing streaming cells with a single source-backed canonical cell on finalization is critical:
- Reduces memory (transcript cells are deduplicated)
- Enables resize reflow from raw source
- Prevents stale partial renders

**Must adopt.**

---

## 10. Anti-Patterns & Limitations to Avoid

### ❌ 1. No Virtualization — Full Re-render Every Frame

codex-rs re-renders the **entire transcript** on every frame using `Paragraph::scroll()`. This works for short conversations but will degrade badly with 1000+ lines:
- `display_lines(width)` is called for every cell every frame
- The full `Paragraph` is constructed and laid out every frame
- No viewport culling — off-screen cells are still rendered into the buffer

**For claw-code:** Implement **viewport-clipped rendering** — only render cells whose y-range intersects the visible area. Track cumulative heights to compute skip counts.

### ❌ 2. No Bidirectional Scrolling

The scroll position is always "pinned to bottom" via overflow calculation. There's no scroll offset, no scroll-up history, no PgUp/PgDn. Users cannot review earlier output while streaming.

**For claw-code:** Implement proper scroll state with:
- Stored scroll offset (in rendered lines, not pixels)
- Auto-follow mode (scroll to bottom on new content) with user override
- PgUp/PgDn, arrow keys, and mouse wheel scrolling

### ❌ 3. Mouse Events Dropped Entirely

`map_crossterm_event` returns `None` for all mouse events. No mouse scrolling, no mouse selection, no click-to-focus.

**For claw-code:** At minimum, support mouse wheel for scrolling. Click-to-focus for approval buttons would also be valuable.

### ❌ 4. Forked ratatui + crossterm

Both ratatui and crossterm use custom forks. This means:
- Can't update to upstream bug fixes without manual merging
- Binary size may include unneeded patches
- Community support doesn't apply to forked versions

**For claw-code:** Try to use upstream crates first. If `unstable-backend-writer` or `SynchronizedUpdate` are needed, use feature flags or minimal patches that can be upstreamed. Track ratatui 0.30+ which may stabilize these features.

### ❌ 5. Unbounded AppEvent Channel

Using `mpsc::UnboundedSender<AppEvent>` means no backpressure. If the protocol layer produces events faster than the UI can consume them, memory grows without bound.

**For claw-code:** Use a bounded channel with a reasonable capacity (e.g., 1024). When full, drop the oldest low-priority events or apply coalescing (the FrameRequester pattern shows they already know how).

### ❌ 6. Monolithic ChatWidget

`ChatWidget` at ~2030 lines handles too many concerns:
- Protocol event handling
- Cell management (committed + active)
- Streaming animation
- Transcript overlay
- Slash command dispatch
- Approval routing
- MCP status
- Pet images / ambient features
- Terminal title management

**For claw-code:** Split into:
- `Transcript` — cell storage and retrieval
- `StreamManager` — streaming controllers and commit ticks
- `ChatView` — renderable composition
- `EventHandler` — protocol event → state mutation

### ❌ 7. No Diff Rendering Strategy for Large Files

`diff_render.rs` is 2481 lines and renders full file diffs. For large files (1000+ lines), this could be extremely slow with no virtualization.

**For claw-code:** Implement windowed diff rendering — only render the visible portion of the diff, with context lines above/below changes.

### ❌ 8. Global Static for stderr Suppression

`STDERR_STATE: Mutex<StderrState>` is a global static. This makes testing harder and prevents multiple TUI instances.

**For claw-code:** If adopting stderr suppression, make it owned by the `Tui` struct. Pass it through the initialization chain rather than using a global.

### ❌ 9. No Incremental Rendering for Markdown

Every streaming delta triggers a full re-render of the accumulated markdown source (`recompute_streaming_render()`). For long responses, this means re-parsing and re-rendering the entire markdown on every chunk.

**For claw-code:** Consider incremental markdown rendering — parse new content only, cache rendered fragments, and splice them into the line array.

---

## Summary: Architecture at a Glance

```
┌─────────────────────────────────────────────────────────────────┐
│                        tokio::select!                           │
│  ┌──────────────────┐              ┌──────────────────────────┐  │
│  │   TUI Events     │              │      App Events          │  │
│  │  (TuiEventStream)│              │  (mpsc UnboundedReceiver)│  │
│  │                  │              │                          │  │
│  │  ┌────────────┐ │              │  Protocol events         │  │
│  │  │EventBroker │ │              │  Streaming deltas        │  │
│  │  │(crossterm  │ │              │  Approval requests       │  │
│  │  │ EventStream)│ │              │  Consolidation           │  │
│  │  └────────────┘ │              └──────────────────────────┘  │
│  │  ┌────────────┐ │                     │                      │
│  │  │Draw events │ │                     │                      │
│  │  │(broadcast │ │                     ▼                      │
│  │  │ channel)  │ │           ┌───────────────────┐           │
│  │  └────────────┘ │           │      App          │           │
│  └──────────────────┘           │  (event_dispatch) │           │
│           │                      └─────────┬─────────┘           │
│           │                                │                     │
│           ▼                                ▼                     │
│  ┌─────────────────────────────────────────────────────────┐     │
│  │                    ChatWidget                           │     │
│  │  ┌─────────────────┐  ┌─────────────────────────────┐  │     │
│  │  │  transcript_cells│  │    active_cell (mutable)    │  │     │
│  │  │  (committed)     │  │    ┌────────────────────┐   │  │     │
│  │  │  Vec<Arc<dyn     │  │    │ StreamController   │   │  │     │
│  │  │  HistoryCell>>   │  │    │ ┌──────────────┐   │   │  │     │
│  │  │                  │  │    │ │ StreamCore   │   │   │  │     │
│  │  │  AgentMessage    │  │    │ │ stable | tail│   │   │  │     │
│  │  │  AgentMarkdown   │  │    │ └──────────────┘   │   │  │     │
│  │  │  ExecInteraction │  │    │ TableHoldback       │   │  │     │
│  │  │  Approval        │  │    └────────────────────┘   │  │     │
│  │  │  McpToolCall      │  │    CommitTick (chunking)    │  │     │
│  │  │  Diff             │  └─────────────────────────────┘  │     │
│  │  └─────────────────┘                                     │     │
│  │  ┌─────────────────────────────────────────────────────┐│     │
│  │  │  BottomPane (composer)                               ││     │
│  │  │  TextArea (custom) → ChatComposer                    ││     │
│  │  └─────────────────────────────────────────────────────┘│     │
│  └─────────────────────────────────────────────────────────┘     │
│                                                                  │
│  Render: ChatWidget.as_renderable() → FlexRenderable → terminal │
│  Frame: FrameRequester → FrameScheduler (coalesce) → broadcast  │
│  Rate:  FrameRateLimiter (120 FPS ceiling)                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Takeaways for claw-code

1. **Must have:** EventBroker pause/resume, stderr dup2 suppression, `#![deny(print_stdout/print_stderr)]`, SynchronizedUpdate
2. **Must have:** Two-region streaming (stable + mutable tail) with table holdback
3. **Must have:** Transcript consolidation (streaming → canonical source-backed cell)
4. **Should have:** Frame scheduler actor with coalescing
5. **Should have:** Custom FlexRenderable layout system
6. **Should have:** HistoryCell trait with width-dependent height for resize reflow
7. **Improve on:** Add viewport clipping/virtualization, bidirectional scrolling, mouse wheel support
8. **Improve on:** Split ChatWidget into focused modules
9. **Improve on:** Use bounded app event channel
10. **Improve on:** Minimize ratatui/crossterm forking — try upstream features first
