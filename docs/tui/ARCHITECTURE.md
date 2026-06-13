# Claw-Code TUI Architecture Plan

**Status:** MoA-synthesized design — 5 expert reports consolidated  
**Target:** `origin/feat/tui` branch → upstream PR  
**Precedent:** codex-rs (OpenAI), aichat (sigoden)

---

## 0. Executive Summary

The three previous TUI attempts all failed for the same root reason: **the runtime was built to print to stdout, and the TUI was grafted on top.** Every output-bleeding fix (suspend/resume, buffer capture, libc::dup, gag::BufferRedirect, leave-for-turn) was a workaround for this architectural mismatch. Every scroll bug stemmed from an inverted offset model that fought ratatui's `Paragraph::scroll()` semantics.

The correct architecture — proven by codex-rs — is:

1. **The runtime emits structured events, not stdout bytes**
2. **The TUI consumes events via a channel and renders them as cells**
3. **Scroll operates on cells with forward offsets (0 = top)**
4. **The TUI renders to stderr, keeping stdout free for child process capture**

---

## 1. Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                       Main Event Loop                        │
│               tokio::select! { ... }                         │
│                                                              │
│  ┌───────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │
│  │ TuiEvents │ │AppEvents │ │TurnEvents│ │PermissionRx  │  │
│  │(crossterm)│ │(bounded) │ │(bounded) │ │(oneshot resp)│  │
│  └─────┬─────┘ └────┬─────┘ └────┬─────┘ └──────┬───────┘  │
│        │            │            │               │           │
│        └────────────┴────────────┴───────────────┘           │
│                          │                                    │
│                     App::handle()                            │
│                          │                                    │
│         ┌────────────────┼────────────────┐                  │
│         ▼                ▼                ▼                   │
│    ChatWidget      BottomPane       PermissionOverlay         │
│    ┌──────────┐   ┌──────────┐      ┌──────────┐             │
│    │Committed  │   │Composer  │      │[Y/N/A]  │             │
│    │Cells[]   │   │(textarea)│      │prompt    │             │
│    │+Active   │   │+History  │      └──────────┘             │
│    │  Cell    │   └──────────┘                               │
│    └──────────┘                                              │
│         │                                                     │
│         ▼                                                     │
│    FrameRequester → rate-limited draw                         │
│         │                                                     │
│         ▼                                                     │
│    ratatui render → crossterm → stderr                        │
└─────────────────────────────────────────────────────────────┘

                    │ ▲
                    │ │  TuiTurnEvent channel
                    │ │
┌───────────────────▼─┴──────────────────────────────────────┐
│                   TurnWorker (tokio::spawn)                  │
│                                                              │
│   ConversationRuntime::run_turn()                           │
│       │                                                      │
│       ├── ApiClient::stream() → Vec<AssistantEvent>         │
│       │   └── Forward each event via tx.send()              │
│       │       before collecting for build_assistant_message  │
│       │                                                      │
│       ├── PermissionPrompter::decide()                       │
│       │   └── Send PermissionRequired via channel            │
│       │       with oneshot::Sender for response              │
│       │                                                      │
│       └── TurnProgressReporter::on_tool_result()             │
│           └── Forward as TuiTurnEvent::ToolResultReady       │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Crate Stack

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.29+ | Framework — features: `unstable-rendered-line-info`, `unstable-widget-ref` |
| `crossterm` | 0.28+ | Terminal backend — features: `bracketed-paste`, `event-stream` |
| `tokio` | 1 | Async runtime — features: `rt-multi-thread`, `macros`, `sync`, `time`, `process`, `signal` |
| `tui-textarea` | 0.7+ | Input composer (proven in codex-rs) |
| `pulldown-cmark` | 0.12+ | Markdown → ratatui `Text` rendering |
| `syntect` | 5+ | Syntax highlighting for code blocks |
| `textwrap` | 0.18+ | Word-wrap with CJK support |
| `unicode-width` | 0.2+ | Character width calculation (CJK = 2) |
| `insta` | 1+ (dev) | Snapshot testing for widget rendering |
| `ansi-to-tui` | 0.5+ (opt) | Child process ANSI output → ratatui Text |

**Critical:** Render the TUI to **stderr**, not stdout:
```rust
let backend = CrosstermBackend::new(std::io::stderr());
let mut terminal = Terminal::new(backend)?;
```
This frees stdout for child process capture AND prevents output bleeding by design.

---

## 3. Event Architecture

### 3.1 TuiTurnEvent — Runtime → TUI Channel

```rust
#[derive(Debug)]
pub enum TuiTurnEvent {
    // === Streaming ===
    TurnStarted { user_input: String },
    ThinkingDelta { text: String },
    TextDelta { text: String },
    ToolUseStarted { id: String, name: String, input_preview: String },
    ToolResultReady { tool_name: String, result: Result<String, String> },

    // === Permission (interactive — blocks turn worker) ===
    PermissionRequired {
        request: PermissionRequest,
        response_tx: oneshot::Sender<PermissionPromptDecision>,
    },

    // === Lifecycle ===
    TurnCompleted { summary: TurnSummary },
    TurnFailed { error: String },

    // === Progress ===
    IterationProgress { iteration: usize, max_iterations: usize },
}
```

### 3.2 AppEvent — Internal TUI Channel

```rust
#[derive(Debug)]
pub enum AppEvent {
    // From turn worker
    TurnEvent(TuiTurnEvent),

    // From crossterm
    Key(KeyEvent),
    Paste(String),
    Resize { width: u16, height: u16 },

    // Frame scheduling
    ScheduleFrame,

    // Streaming animation tick (~4Hz for UI updates)
    StreamTick,
}
```

### 3.3 Channel Setup

```rust
// Bounded channels with backpressure
let (app_tx, app_rx) = tokio::sync::mpsc::channel::<AppEvent>(256);
let (turn_tx, turn_rx) = tokio::sync::mpsc::channel::<TuiTurnEvent>(256);

// Turn worker receives turn_tx, sends events
// Main loop multiplexes turn_rx → app_tx, crossterm events → app_tx
```

---

## 4. Key Components

### 4.1 App (State Machine)

```rust
pub struct App {
    // Layout
    screen: Screen,

    // Chat state
    cells: Vec<Box<dyn MessageCell>>,  // committed cells
    active_cell: Option<Box<dyn MessageCell>>,  // streaming cell

    // Input
    composer: Composer,

    // Scroll
    scroll_offset: usize,  // FORWARD: 0 = top, max = bottom
    user_scrolled: bool,    // true = user scrolled up, disable auto-scroll

    // Turn worker
    turn_handle: Option<JoinHandle<()>>,
    turn_tx: Sender<TuiTurnEvent>,

    // Frame scheduling
    frame_requester: FrameRequester,
}
```

Screen states:
```rust
pub enum Screen {
    Chat,           // Normal chat mode
    PermissionPrompt,  // Modal overlay for [Y/N/A]
    Help,           // Keybinding help overlay
}
```

### 4.2 MessageCell Trait (Codex-rs Pattern)

```rust
pub trait MessageCell: Send {
    /// Push streaming content into this cell
    fn push_delta(&mut self, delta: &str);

    /// Compute desired height at given width
    fn desired_height(&self, width: u16) -> u16;

    /// Render into ratatui area
    fn render(&self, area: Rect, buf: &mut Buffer);

    /// Whether this cell is still streaming
    fn is_streaming(&self) -> bool;

    /// Finalize — consolidate from fragmented stream state to
    /// a single re-renderable markdown cell (post-stream consolidation)
    fn consolidate(self: Box<Self>) -> Box<dyn MessageCell>;
}
```

Cell types:
- `UserMessageCell` — user input + timestamp
- `AssistantMarkdownCell` — completed markdown (re-renderable from source on resize)
- `AssistantStreamingCell` — active streaming cell (mutable, flush on completion)
- `ToolResultCell` — tool name + collapsed/expanded output
- `ThinkingCell` — collapsible thinking/reasoning block
- `SystemCell` — status messages, errors

### 4.3 Scroll State (Forward Offset)

```rust
pub struct ScrollState {
    /// Forward offset: 0 = top of conversation, max = pinned to bottom
    offset: usize,
    /// Maximum valid offset (recomputed on content change)
    max_offset: usize,
    /// Whether user has manually scrolled (disables auto-scroll)
    user_scrolled: bool,
}

impl ScrollState {
    /// Auto-scroll to bottom (on new content while not user-scrolled)
    pub fn auto_scroll_to_bottom(&mut self, total_lines: usize, viewport_height: usize) {
        if !self.user_scrolled {
            self.offset = total_lines.saturating_sub(viewport_height);
            self.max_offset = self.offset;
        } else {
            self.max_offset = total_lines.saturating_sub(viewport_height);
            self.offset = self.offset.min(self.max_offset);
        }
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.offset = self.offset.saturating_sub(amount);
        self.user_scrolled = true;
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.offset = (self.offset + amount).min(self.max_offset);
        if self.offset >= self.max_offset {
            self.user_scrolled = false;  // Reached bottom, re-enable auto-scroll
        }
    }
}
```

**Key rule:** `PageUp` = `scroll_up` (decrease offset = see older content). `PageDown` = `scroll_down` (increase offset = see newer content). This is **forward offset** — matches `Paragraph::scroll()` and user intuition. No more inverted model.

### 4.4 Composer (Input Area)

```rust
pub struct Composer {
    textarea: TextArea<'static>,  // tui-textarea
    history: Vec<String>,
    history_index: usize,
    submitting: bool,
}
```

- `tui-textarea` for multi-line editing with proper CJK/selection support
- `Enter` = submit (configurable: ctrl+enter for newline)
- `Up/Down` when at top/bottom of textarea = history navigation
- `Tab` = autocomplete (slash commands, file paths)
- `Ctrl+C` when empty = interrupt turn

### 4.5 TurnWorker (Runtime Bridge)

```rust
pub struct TurnWorker {
    runtime: ConversationRuntime,
    tx: Sender<TuiTurnEvent>,
}

impl TurnWorker {
    pub async fn run_turn(
        &mut self,
        user_input: String,
        mut runtime: ConversationRuntime,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Notify TUI that turn started
        self.tx.send(TuiTurnEvent::TurnStarted { user_input: user_input.clone() }).await?;

        // 2. Wrap the API client to intercept streaming events
        //    BEFORE they're collected into Vec<AssistantEvent>
        let streaming_tx = self.tx.clone();

        // 3. Create a TUI-aware PermissionPrompter
        let prompter = TuiPermissionPrompter {
            tx: self.tx.clone(),
        };

        // 4. Run the turn
        //    PROBLEM: run_turn() is synchronous and internally calls
        //    api_client.stream() which returns Vec<AssistantEvent>.
        //    We need to intercept events as they arrive.

        // Option A: Wrap ApiClient with a streaming interceptor
        // Option B: Modify ApiClient::stream() to accept a callback
        // Option C: Run on a separate thread with captured stdout (see §6)

        let result = runtime.run_turn(user_input, Some(&mut prompter));

        match result {
            Ok(summary) => {
                self.tx.send(TuiTurnEvent::TurnCompleted { summary }).await?;
            }
            Err(e) => {
                self.tx.send(TuiTurnEvent::TurnFailed { error: e.to_string() }).await?;
            }
        }

        Ok(())
    }
}
```

### 4.6 TuiPermissionPrompter (Channel Bridge)

```rust
pub struct TuiPermissionPrompter {
    tx: Sender<TuiTurnEvent>,
}

impl PermissionPrompter for TuiPermissionPrompter {
    fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision {
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        // Send permission request to TUI event loop
        let event = TuiTurnEvent::PermissionRequired {
            request: request.clone(),
            response_tx,
        };

        // Use try_send or blocking send (we're on the worker thread)
        if self.tx.blocking_send(event).is_err() {
            return PermissionPromptDecision::Deny {
                reason: "TUI disconnected".into(),
            };
        }

        // Block until TUI responds (this pauses the turn worker)
        response_rx.recv().unwrap_or(PermissionPromptDecision::Deny {
            reason: "No response".into(),
        })
    }
}
```

---

## 5. Event Loop

```rust
pub async fn run_app(mut app: App, mut terminal: Terminal<CrosstermBackend<Stderr>>) -> Result<()> {
    let (app_tx, mut app_rx) = tokio::sync::mpsc::channel::<AppEvent>(256);

    // Spawn crossterm event reader
    let crossterm_tx = app_tx.clone();
    tokio::spawn(async move {
        loop {
            if crossterm::event::poll(Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(event) = crossterm::event::read() {
                    match event {
                        CrosstermEvent::Key(k) => { let _ = crossterm_tx.send(AppEvent::Key(k)).await; }
                        CrosstermEvent::Paste(s) => { let _ = crossterm_tx.send(AppEvent::Paste(s)).await; }
                        CrosstermEvent::Resize(w, h) => { let _ = crossterm_tx.send(AppEvent::Resize { width: w, height: h }).await; }
                        _ => {}
                    }
                }
            }
        }
    });

    // Streaming tick (4Hz — smooth enough, not wasteful)
    let tick_tx = app_tx.clone();
    let tick_interval = tokio::time::interval(Duration::from_millis(250));
    tokio::spawn(async move {
        let mut interval = tick_interval;
        loop {
            interval.tick().await;
            let _ = tick_tx.send(AppEvent::StreamTick).await;
        }
    });

    // Frame rate limiter (cap at 30 FPS for terminal)
    let mut last_frame = Instant::now();
    const MIN_FRAME_INTERVAL: Duration = Duration::from_millis(33);

    loop {
        let event = app_rx.recv().await;

        match event {
            Some(AppEvent::Key(key)) => app.handle_key(key),
            Some(AppEvent::Paste(text)) => app.handle_paste(text),
            Some(AppEvent::Resize { width, height }) => {
                terminal.resize(Rect::new(0, 0, width, height))?;
                app.request_frame();
            }
            Some(AppEvent::TurnEvent(te)) => {
                app.handle_turn_event(te);
                app.request_frame();
            }
            Some(AppEvent::StreamTick) => {
                if app.is_streaming() {
                    app.request_frame();
                }
            }
            Some(AppEvent::ScheduleFrame) => {
                let now = Instant::now();
                if now.duration_since(last_frame) >= MIN_FRAME_INTERVAL {
                    terminal.draw(|f| app.render(f))?;
                    last_frame = now;
                }
            }
            None => break,
        }
    }

    Ok(())
}
```

**Why poll-based crossterm instead of EventStream:**
- `EventStream` requires `crossterm/event-stream` feature + `tokio-stream`
- Poll-based is simpler, no lifetime issues, works on all platforms
- 100ms poll interval means ≤100ms latency for keypresses — imperceptible

**Why 4Hz stream tick instead of 120Hz:**
- codex-rs uses 120Hz for smooth typing animation — luxury for a terminal
- 4Hz (250ms) is sufficient for visible streaming progress and MUCH cheaper
- Can increase to 8-10Hz if users want smoother animation
- Tokens arrive from API at ~5-50 tokens/sec anyway — 4Hz catches most chunks

---

## 6. The Streaming Problem: How to Get Live Tokens into the TUI

This is the **hardest architectural decision**. `ApiClient::stream()` returns `Vec<AssistantEvent>` (collected), not a live stream. We need tokens as they arrive.

### Option A: Wrap `ApiClient` with a Streaming Interceptor

Create a `TuiApiClient<C: ApiClient>` that wraps the real client:

```rust
struct TuiApiClient<C: ApiClient> {
    inner: C,
    tx: Sender<TuiTurnEvent>,
}

impl<C: ApiClient> ApiClient for TuiApiClient<C> {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        // Call the real stream() — but we can't intercept mid-call
        // because it returns Vec, not an iterator

        // UNLESS we modify the trait to accept a callback:
        let events = self.inner.stream(request)?;
        // This arrives as a complete Vec — too late for live rendering
        Ok(events)
    }
}
```

**Problem:** We can't intercept events mid-stream because `stream()` is synchronous and returns a collected `Vec`. The events are already all arrived by the time we get the return value. We'd need to modify the `ApiClient` trait.

### Option B: Modify `ApiClient::stream()` to Return an Async Stream

```rust
pub trait ApiClient {
    fn stream(&mut self, request: ApiRequest)
        -> Pin<Box<dyn Stream<Item = Result<AssistantEvent, RuntimeError>> + '_>>;
}
```

Then the `TurnWorker` can iterate events as they arrive and forward each one to the TUI before collecting for `build_assistant_message()`.

**Pros:** Clean, correct, future-proof
**Cons:** Requires modifying the `ApiClient` trait — more invasive, affects CLI mode too

### Option C: Run `ConversationRuntime::run_turn()` on a Thread with Captured stdout

```rust
// In TurnWorker:
let runtime = self.runtime.take().unwrap();
let tx = self.tx.clone();

std::thread::spawn(move || {
    // Capture stdout at the OS level
    let _guard = gag::BufferRedirect::stdout().unwrap();

    // Run the turn — runtime prints to captured stdout
    let result = runtime.run_turn(user_input, Some(&mut prompter));

    // Read captured output and send to TUI
    let mut output = String::new();
    _guard.into_inner().read_to_string(&mut output).unwrap();
    // Parse output into cells...
});
```

**Problem:** We get the output as a blob after the turn, not streaming. Same fundamental issue.

### Decision: Option B — Modify `ApiClient::stream()` to Return an Async Stream

This is the only approach that gives us live streaming. The modification is:

1. Change the `ApiClient` trait to return `impl Stream<Item = Result<AssistantEvent, RuntimeError>>`
2. In CLI mode, `.collect::<Vec<_>>()` at the call site — behavior unchanged
3. In TUI mode, iterate the stream and forward each event via the channel before collecting

**Migration path:**

```rust
// New trait
pub trait ApiClient {
    fn stream_events(
        &mut self,
        request: ApiRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<AssistantEvent, RuntimeError>> + Send + '_>>;

    // Convenience for CLI — backward compat
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        let stream = self.stream_events(request);
        // Block on collection
        futures::executor::block_on(async {
            stream.try_collect().await
        })
    }
}
```

This is a breaking change to the trait, but it's contained to the `runtime` crate. The CLI call site already handles the collected `Vec`, so adding `stream_events()` as the primary method with `stream()` as a backward-compat wrapper minimizes disruption.

**Alternative if trait change is rejected:** Run `run_turn()` on a blocking task in `tokio::spawn_blocking()`. Accept that we won't see streaming tokens — the TUI will show a spinner during thinking and update cells after each tool result (via `TurnProgressReporter`). Text rendering updates only after the full response is collected. This is acceptable for v1 but means no live typing animation.

---

## 7. Streaming Markdown Rendering

### 7.1 Active Cell + Committed Cells (Codex-rs Pattern)

During streaming, there is one **mutable active cell** (`AssistantStreamingCell`) that receives `push_delta()` calls. When streaming ends, the active cell is **consolidated** into an `AssistantMarkdownCell` and moved to the committed cells list.

```rust
pub struct AssistantStreamingCell {
    raw_markdown: String,           // Accumulating source markdown
    cached_text: Option<Text<'static>>,  // Rendered text (dirty flag)
    cached_width: Option<u16>,     // Width at which cache was built
    dirty: bool,
}

impl MessageCell for AssistantStreamingCell {
    fn push_delta(&mut self, delta: &str) {
        self.raw_markdown.push_str(delta);
        self.dirty = true;
    }

    fn desired_height(&self, width: u16) -> u16 {
        let text = self.get_or_rebuild_text(width);
        text.height() as u16
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let text = self.get_or_rebuild_text(area.width);
        Paragraph::new(text).render(area, buf);
    }

    fn is_streaming(&self) -> bool { true }

    fn consolidate(self: Box<Self>) -> Box<dyn MessageCell> {
        Box::new(AssistantMarkdownCell {
            raw_markdown: self.raw_markdown,
            cached_text: None,
            cached_width: None,
        })
    }
}
```

### 7.2 Markdown → ratatui Text Conversion

Use `pulldown-cmark` → custom `Text` builder (same approach as `tui-markdown` but with syntect highlighting):

```rust
pub fn markdown_to_text(md: &str, theme: &ColorTheme) -> Text<'static> {
    let mut lines = vec![];
    let mut in_code_block = false;
    let mut code_lang = None;

    for event in pulldown_cmark::Parser::new_ext(md, Options::all()) {
        match event {
            Event::Start(Tag::Heading(level)) => { /* push styled heading line */ }
            Event::Start(Tag::CodeBlock(lang)) => {
                in_code_block = true;
                code_lang = Some(lang.to_string());
            }
            Event::End(Tag::CodeBlock(_)) => { in_code_block = false; }
            Event::Text(text) => {
                if in_code_block {
                    // syntect highlight → ansi-to-tui → Line
                } else {
                    // Style with bold/italic/code spans
                }
            }
            _ => {}
        }
    }

    Text::from(lines)
}
```

### 7.3 Adaptive Chunking (From Codex-rs)

During streaming, batch tokens on a 250ms tick (4Hz). On each tick:
- If `< 3 new lines` → smooth mode: render 1 line (typing animation)
- If `≥ 3 new lines` → catchup mode: render all pending lines immediately
- On stream end: consolidate active cell → committed cell

---

## 8. Scroll Implementation

### 8.1 Cell-Based Scroll with Prefix-Sum Heights

```rust
pub struct ChatScroll {
    /// Per-cell visual heights (recomputed on resize)
    cell_heights: Vec<u16>,
    /// Prefix sums of cell_heights (for O(log n) offset → cell lookup)
    height_offsets: Vec<usize>,
    /// Forward offset from top (0 = top, max = pinned to bottom)
    offset: usize,
    /// Whether user has scrolled (disables auto-scroll)
    user_scrolled: bool,
}

impl ChatScroll {
    /// Recompute when cells change or terminal resizes
    pub fn rebuild(&mut self, cells: &[Box<dyn MessageCell>], width: u16) {
        self.cell_heights = cells.iter().map(|c| c.desired_height(width)).collect();
        self.height_offsets = Vec::with_capacity(self.cell_heights.len() + 1);
        self.height_offsets.push(0);
        let mut acc = 0usize;
        for &h in &self.cell_heights {
            acc += h as usize;
            self.height_offsets.push(acc);
        }
        // Clamp offset
        let total = *self.height_offsets.last().unwrap_or(&0);
        let viewport = ???; // needs viewport height
        self.offset = self.offset.min(total.saturating_sub(viewport));
    }

    /// Find which cell is at the top of the viewport
    pub fn top_cell_index(&self) -> usize {
        self.height_offsets.partition_point(|&h| h <= self.offset) - 1
    }
}
```

### 8.2 Auto-Scroll Logic

```rust
// On new content (streaming delta, tool result, etc.)
fn on_content_changed(&mut self) {
    if !self.scroll.user_scrolled {
        // Auto-scroll to bottom
        let total = self.scroll.height_offsets.last().copied().unwrap_or(0);
        self.scroll.offset = total.saturating_sub(self.viewport_height);
    }
    // Re-render
    self.request_frame();
}

// On user keypress
fn handle_scroll_key(&mut self, key: KeyEvent) {
    match key.code {
        KeyCode::PageUp => {
            self.scroll.user_scrolled = true;
            self.scroll.offset = self.scroll.offset.saturating_sub(self.viewport_height / 2);
        }
        KeyCode::PageDown => {
            self.scroll.offset += self.viewport_height / 2;
            let max = self.scroll.max_offset(self.viewport_height);
            if self.scroll.offset >= max {
                self.scroll.offset = max;
                self.scroll.user_scrolled = false;  // Reached bottom
            }
        }
        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
            self.scroll.user_scrolled = true;
            self.scroll.offset = self.scroll.offset.saturating_sub(1);
        }
        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
            self.scroll.offset += 1;
            // Auto-reenable at bottom
        }
        _ => {}
    }
}
```

---

## 9. Permission Prompts

### 9.1 Inline Overlay (Recommended for v1)

When `PermissionRequired` event arrives:

1. Pause the turn worker (it's blocked on the `oneshot::Receiver`)
2. Render a styled panel at the bottom of the chat area:
   ```
   ┌─ Permission Required ─────────────────────────┐
   │ Tool: bash                                     │
   │ Input: rm -rf /tmp/test                        │
   │                                                 │
   │ [Y] Allow  [N] Deny  [A] Allow for session    │
   └─────────────────────────────────────────────────┘
   ```
3. On `[Y/N/A]` keypress → send response via `oneshot::Sender`
4. Turn worker unblocks and continues

### 9.2 Mouse Support

Not for v1. Focus on keyboard-only interaction.

---

## 10. Layout

```
┌──────────────────────────────────────────────────────────┐
│  ╔════════════════════════════════════════════════════╗   │
│  ║  [Chat Area — scrolls]                             ║   │
│  ║                                                     ║   │
│  ║  User: What does this function do?                  ║   │
│  ║                                                     ║   │
│  ║  Assistant: This function processes...█            ║   │  ← active cell (streaming)
│  ║                                                     ║   │
│  ╚════════════════════════════════════════════════════╝   │
│                                                           │
│  ┌─ Input ─────────────────────────────────────────────┐  │
│  │ >  Ask me anything...                               │  │  ← tui-textarea
│  └──────────────────────────────────────────────────────┘  │
│  │ Model: glm-5 │ Tokens: 1.2k/3.4k │ Status: streaming│  ← status bar
└──────────────────────────────────────────────────────────┘
```

Constraint-based layout:
- Chat area: flex=1 (takes remaining space)
- Input area: natural height (1-5 lines based on textarea content)
- Status bar: fixed 1 line

---

## 11. Output Bleeding Prevention (The Complete Solution)

All three layers must work together:

### Layer 1: Render to stderr
```rust
let backend = CrosstermBackend::new(std::io::stderr());
```
Stdout is now free — child processes can write to it without corrupting the TUI. The TUI never reads from fd 1 for rendering.

### Layer 2: Capture child process stdout
When executing tools that spawn processes (bash, git, etc.):
```rust
// In tool executor:
let output = Command::new("bash")
    .arg("-c")
    .arg(command)
    .stdout(Stdio::piped())   // Capture, don't inherit
    .stderr(Stdio::piped())
    .output()
    .await?;
```
Tool output goes to `ToolResultCell` via the channel, not to the terminal.

### Layer 3: Suppress internal runtime prints
The runtime's `TerminalRenderer` / `LiveCli` prints go through the `Write` trait. In TUI mode:
- Replace `io::stdout()` with a no-op writer for the turn duration, OR
- Better: implement `TurnProgressReporter` that forwards events instead of printing

### Layer 4: Alternate screen + synchronized updates
```rust
execute!(stderr, EnterAlternateScreen)?;
enable_raw_mode()?;
// ... TUI loop ...
execute!(stderr, LeaveAlternateScreen)?;
disable_raw_mode()?;
```

Use `crossterm`'s `SynchronizedUpdate` to batch terminal writes and prevent partial frames.

### Layer 5: TerminalStderrGuard (macOS)
On macOS, system frameworks may write to fd 2. Redirect fd 2 to `/dev/null` during TUI ownership (codex-rs pattern). On Linux, this is typically not needed.

---

## 12. Testing Strategy

### 12.1 Widget Unit Tests (ratatui TestBackend)
```rust
#[test]
fn renders_user_message() {
    let cell = UserMessageCell::new("Hello world");
    let area = Rect::new(0, 0, 80, 5);
    let mut buf = Buffer::empty(area);
    cell.render(area, &mut buf);

    // Assert on buffer contents
    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "H");
}
```

### 12.2 Snapshot Tests (insta)
```rust
#[test]
fn snapshot_chat_widget() {
    let mut app = App::new_test();
    app.add_user_message("Hello");
    app.add_assistant_message("Hi there!");

    let area = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(area);
    app.render_chat(area, &mut buf);

    insta::assert_snapshot!(format!("{:?}", buf));
}
```

### 12.3 Event-Based Integration Tests
```rust
#[tokio::test]
async fn test_scroll_after_streaming() {
    let (app_tx, app_rx) = mpsc::channel(256);
    let mut app = App::new_with_channel(app_tx.clone());

    // Simulate streaming events
    app_tx.send(AppEvent::TurnEvent(TuiTurnEvent::TurnStarted { user_input: "test".into() })).await.unwrap();
    app_tx.send(AppEvent::TurnEvent(TuiTurnEvent::TextDelta { text: "Hello world".into() })).await.unwrap();
    app_tx.send(AppEvent::TurnEvent(TuiTurnEvent::TurnCompleted { summary: /* ... */ })).await.unwrap();

    // Process events
    while let Ok(event) = app_rx.try_recv() {
        app.handle_event(event);
    }

    assert_eq!(app.cells.len(), 2); // user + assistant
    assert!(!app.scroll_state.user_scrolled);
}
```

### 12.4 OpenClaw Integration (Future PR)
```bash
# clawcode --tui-stdio mode
# Spawns TUI, exposes JSON-over-stdio protocol:
# - Input: {"type":"key","key":"Enter"} or {"type":"prompt","text":"hello"}
# - Output: {"type":"frame","cells":[...]} or {"type":"state","scroll":0,"cells":2}
# OpenClaw can drive the TUI like a user, verify screen state, catch regressions
```

---

## 13. File Structure

```
rust/crates/rusty-claude-cli/src/tui/
├── mod.rs                  // TUI module entry + `claw tui` command
├── app.rs                  // App state machine + event loop
├── event.rs                // AppEvent, TuiTurnEvent enums
├── frame.rs                // FrameRequester + rate limiter
├── scroll.rs               // ScrollState + ChatScroll with prefix sums
├── run.rs                  // TurnWorker + TuiPermissionPrompter
├── render/
│   ├── mod.rs              // markdown → Text conversion
│   ├── cells.rs            // MessageCell trait + all cell types
│   └── theme.rs            // ColorTheme ported from render.rs
├── components/
│   ├── mod.rs
│   ├── chat.rs             // ChatWidget (renders cells + scroll)
│   ├── composer.rs         // Input composer wrapping tui-textarea
│   ├── status_bar.rs       // Model, tokens, status display
│   └── permission.rs       // Permission prompt overlay
└── test_utils.rs           // Test helpers, mock event sources
```

---

## 14. Implementation Phases

### Phase 0: Prerequisites (1-2 days) ✅
- [x] Streaming error fixes (reasoning_content, optional delta, JSON/HTML errors)
- [x] CLI module extraction (agent.rs, team.rs, permission.rs, etc.)
- [ ] Modify `ApiClient::stream()` to return async stream (Option B from §6)
- [ ] Add `TuiTurnEvent` enum to runtime crate

### Phase 1: Skeleton (3-4 days)
- [ ] `tui/mod.rs` — `claw tui` subcommand entry point
- [ ] `app.rs` — App struct, event loop, stderr backend
- [ ] `event.rs` — All event types
- [ ] `frame.rs` — FrameRequester + 30 FPS cap
- [ ] Alternate screen enter/exit with cleanup on panic
- [ ] Basic layout: chat area + input area + status bar
- [ ] Composer with tui-textarea
- [ ] Exit on Ctrl+C / `/exit`

**Milestone:** Can type a prompt, see it echoed, and exit cleanly.

### Phase 2: Live Streaming (5-7 days) — THE CRITICAL PHASE
- [ ] `run.rs` — TurnWorker with channel bridge
- [ ] `render/cells.rs` — MessageCell trait + AssistantStreamingCell
- [ ] `render/mod.rs` — Markdown → Text with syntect highlighting
- [ ] `render/theme.rs` — ColorTheme from render.rs
- [ ] Streaming event handling: TextDelta → push_delta → render on tick
- [ ] ThinkingCell with collapsible display (default: collapsed)
- [ ] ToolResultCell with collapsed/expanded toggle
- [ ] Post-stream consolidation (streaming → markdown cell)
- [ ] Permission prompts with oneshot channel bridge

**Milestone:** Can submit a prompt, see the response stream live in the TUI, handle tool calls and permissions.

### Phase 3: Scroll & Polish (3-4 days)
- [ ] `scroll.rs` — Forward-offset ScrollState with prefix sums
- [ ] `components/chat.rs` — ChatWidget with cell-based scroll
- [ ] Auto-scroll with user-scroll detection
- [ ] Mouse wheel scroll (crossterm ScrollUp/ScrollDown events)
- [ ] Shift+Up/Down for fine scroll
- [ ] Status bar with model name, token count, spinner

**Milestone:** Smooth scrolling, auto-scroll during streaming, manual scroll to review history.

### Phase 4: Polish & Testing (2-3 days)
- [ ] Resize handling (rebuild cell heights, clamp scroll)
- [ ] CJK rendering test cases
- [ ] Widget unit tests + insta snapshots
- [ ] Event-based integration tests
- [ ] Help overlay (`?` key)
- [ ] Chat history persistence across sessions
- [ ] Slash command dispatch
- [ ] `cargo clippy` + `cargo test` green

**Milestone:** PR-ready. All tests passing, no warnings.

### Phase 5: OpenClaw Integration (Separate PR)
- [ ] `--tui-stdio` mode for programmatic driving
- [ ] OpenClaw `clawcode` tool extension
- [ ] Automated TUI regression tests in CI

---

## 15. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `ApiClient` trait change rejected by upstream | Medium | High | Fallback: v1 without live streaming, spinner-only during thinking |
| `tui-textarea` conflicts with crossterm event handling | Low | Medium | Vendored copy (codex-rs does this) with patches |
| Terminal emulators with broken alternate screen | Low | Medium | Test on: kitty, alacritty, iterm2, windows terminal, foot |
| `gag::BufferRedirect` not available on Windows | Medium | Low | Feature-gate; Windows uses `Stdio::piped()` for child processes |
| Token streaming too fast for 4Hz tick | Low | Low | Adaptive chunking: catchup mode drains all pending on tick |
| `RefCell` borrow conflicts in render | Medium | Medium | Use interior mutability via `Cell<Option<Cache>>` instead of `RefCell` |

---

## 16. Key Decisions Summary

| Decision | Choice | Reason |
|----------|--------|--------|
| Render to stdout or stderr? | **stderr** | Frees stdout for child capture, prevents bleeding |
| Event loop: poll or EventStream? | **Poll** | Simpler, no lifetime issues, cross-platform |
| Frame rate | **30 FPS cap** | Terminal doesn't need 120Hz; saves CPU |
| Stream tick rate | **4 Hz (250ms)** | Smooth enough for streaming text; can increase |
| Scroll model | **Forward offset (0=top)** | Matches Paragraph::scroll(), matches user intuition |
| Scroll unit | **Cells, not lines** | Stable on resize; each cell handles its own wrapping |
| Live streaming | **Modify ApiClient trait** | Only way to get live tokens; fallback: spinner-only v1 |
| Input widget | **tui-textarea** | Proven in codex-rs; CJK, selection, multi-line |
| Markdown rendering | **Custom pulldown-cmark → Text** | tui-markdown is experimental; we need syntect |
| Permission prompts | **Inline overlay** | Simpler than popup; blocks turn worker via oneshot |
| Testing | **insta snapshots + event injection** | Proven pattern; no real terminal needed |
