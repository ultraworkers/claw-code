# TUI MOA Ecosystem Report: Rust TUI Patterns for claw-code

**Date:** 2026-06-12  
**Purpose:** Survey the broader Rust TUI ecosystem for best-in-class solutions to claw-code's TUI architecture problems.

---

## Table of Contents

1. [Streaming Text Rendering in Ratatui](#1-streaming-text-rendering-in-ratatui)
2. [Output Isolation / stdout Capture in Rust](#2-output-isolation--stdout-capture-in-rust)
3. [Async Runtime + Ratatui Patterns](#3-async-runtime--ratatui-patterns)
4. [Scroll State Management](#4-scroll-state-management)
5. [Permission Prompts in TUI](#5-permission-prompts-in-tui)
6. [Testing TUIs in Rust](#6-testing-tuis-in-rust)

---

## 1. Streaming Text Rendering in Ratatui

### 1.1 tui-markdown: Live Markdown Rendering

**Crate:** `tui-markdown` (by Josh McKinney / joshka, ratatui core maintainer)  
**Repo:** https://github.com/joshka/tui-markdown  
**Status:** Experimental proof-of-concept, actively maintained

**How it works:**
- Uses `pulldown-cmark` to parse markdown into events
- Converts those events into a `ratatui::text::Text` value
- Renders the `Text` widget directly via `frame.render_widget(text, area)`

**Key API:**
```rust
use tui_markdown::from_str;

let markdown = "# Heading\n\n**bold** text with `code`";
let text = from_str(markdown);
text.render(area, &mut buf);
```

**For streaming:** There is no built-in streaming/incremental mode. The entire markdown string is parsed each time. The pattern for live-updating is:

```rust
// In your app state, accumulate streaming content
struct AppState {
    stream_buffer: String,
}

// On each token arrival, append and re-render
fn on_token(&mut self, token: &str) {
    self.stream_buffer.push_str(token);
    // Re-convert the entire buffer on each render
    let text = tui_markdown::from_str(&self.stream_buffer);
    // render...
}
```

**Performance consideration:** For long conversations, full re-parse each frame is O(n). Mitigations:
- Only re-parse when new tokens arrive (set a dirty flag)
- Cache the `Text` and only re-convert on change
- For very long histories, consider truncating the visible portion

**Supported features:** Headings, bold, italic, strikethrough, ordered/unordered lists, code blocks, block quotes, task lists, metadata blocks, footnotes, links. **NOT supported:** Tables, images, HTML, math.

**Code highlighting:** Optional `highlight-code` feature uses `syntect` for syntax-colored code blocks, which then get converted via `ansi-to-tui` back to ratatui `Text`.

### 1.2 ansi-to-tui: Preserving ANSI Colors

**Crate:** `ansi-to-tui`  
**Docs:** https://docs.rs/ansi-to-tui  
**Purpose:** Convert ANSI escape sequences to ratatui `Text` with proper `Color` and `Style`

This is critical for displaying child process output that contains ANSI color codes (e.g., `git diff --color`, test runners, etc.).

```rust
use ansi_to_tui::IntoText;

let bytes = b"\x1b[38;2;225;192;203mcolored text\x1b[0m".to_vec();
let text = bytes.into_text()?;
frame.render_widget(text, area);
```

**Supports:** 3/4-bit named colors, 8-bit indexed, 24-bit truecolor RGB, bold/italic/underline/strikethrough modifiers. Has optional SIMD acceleration via `simdutf8` feature.

### 1.3 Virtual Scrolling / Viewport Management with Variable-Height Rows

**No dedicated crate for virtual scrolling of variable-height rows exists in the ratatui ecosystem.** This is the hardest unsolved problem for chat-style UIs.

**Available approaches:**

#### tui-scrollview (by joshka, official ratatui org)

**Crate:** `tui-scrollview`  
**Docs:** https://docs.rs/tui-scrollview  
**Repo:** Part of https://github.com/ratatui/tui-widgets

Provides a `ScrollView` widget that renders content to an off-screen buffer and supports scrolling. However, it requires a **fixed content size** upfront:

```rust
use tui_scrollview::{ScrollView, ScrollViewState};

let content_size = Size::new(100, 30); // Must know total size
let mut scroll_view = ScrollView::new(content_size);
scroll_view.render_widget(Paragraph::new(text), Rect::new(0, 0, 100, 30));
scroll_view.render(buf.area, buf, state);
```

**Limitation for chat UIs:** You must know the total content height in advance. For word-wrapped variable-height rows, you'd need to pre-compute heights, which requires knowing the terminal width at layout time.

#### DIY Virtual Scroll Pattern (recommended)

For claw-code's chat history with variable-height rows, implement your own:

```rust
struct VirtualScroll {
    /// Each item has a computed height (in lines)
    item_heights: Vec<u16>,
    /// Cumulative height sums for binary search
    height_offsets: Vec<usize>,
    /// Current scroll offset (in total lines from top)
    scroll_offset: usize,
    /// Visible area height
    viewport_height: u16,
}

impl VirtualScroll {
    /// Find which item is at the top of the viewport
    fn top_item(&self) -> usize {
        self.height_offsets.partition_point(|&h| h <= self.scroll_offset)
    }
    
    /// Get all items visible in current viewport
    fn visible_items(&self) -> impl Iterator<Item = (usize, u16)> {
        let start = self.top_item();
        let mut y = 0u16;
        self.item_heights[start..].iter().enumerate().take_while(|&(_, h)| {
            y += h; y <= self.viewport_height
        }).map(move |(i, &h)| (start + i, h))
    }
    
    /// Recalculate heights after resize (must be called on width change)
    fn recalc_heights(&mut self, items: &[ChatItem], width: u16) {
        self.item_heights = items.iter()
            .map(|item| item.wrap_height(width))
            .collect();
        self.height_offsets = std::iter::once(0)
            .chain(self.item_heights.iter().scan(0, |acc, &h| {
                *acc += h as usize; Some(*acc)
            }))
            .collect();
    }
}
```

**Key insight:** Height computation must happen *after* you know the render width. On resize, recompute all heights. This is unavoidable for variable-height content.

### 1.4 Lazy/Incremental Rendering of Large Chat Histories

No existing crate. The pattern used by aichat and similar tools:

```rust
struct ChatBuffer {
    /// Ring buffer of recent messages (fixed-size)
    messages: VecDeque<ChatMessage>,
    /// Max messages to keep in memory
    max_messages: usize,
    /// Scroll state
    scroll: VirtualScroll,
}

impl ChatBuffer {
    fn push(&mut self, msg: ChatMessage) {
        if self.messages.len() >= self.max_messages {
            self.messages.pop_front();
            // Adjust scroll offset
        }
        self.messages.push_back(msg);
        // Mark scroll as needing recalculation
    }
}
```

**aichat's approach:** Uses a `Vec<ChatMessage>` with custom scroll tracking. Messages are rendered as `Paragraph` widgets with `.wrap(Wrap { trim: false })`. Auto-scroll is maintained by tracking whether the user was at the bottom before the last update.

---

## 2. Output Isolation / stdout Capture in Rust

This is the critical problem: when tool execution (child processes, library code) writes to stdout, it must not corrupt the TUI rendering.

### 2.1 The Problem

When ratatui draws to the terminal via crossterm's alternate screen, any `println!` or child process stdout write goes to the *same* file descriptor (fd 1). Since crossterm writes raw escape sequences to fd 1, interleaved output corrupts the display.

### 2.2 Approaches Compared

#### Approach A: `gag` crate — RAII stdout/stderr suppression

**Crate:** `gag` (by ipetkov)  
**Docs:** https://docs.rs/gag  
**How it works:** Uses `libc::dup` / `libc::dup2` to swap out fd 1/2 with `/dev/null`, restoring on drop.

```rust
use gag::Gag;

// Suppress stdout for the duration of this scope
let _gag = Gag::stdout().unwrap();
// Everything printed here goes to /dev/null
println!("this is suppressed");
// _gag dropped here, stdout restored
```

**Pros:** Simple RAII pattern, works for blocking the current thread.  
**Cons:** Not thread-safe! If another thread writes to stdout while gagged, it's also suppressed. Global fd-level operation — affects entire process.

**Verdict:** ❌ Not suitable for TUI apps with async runtimes. The global nature conflicts with other threads.

#### Approach B: `capture-stdio` crate — pipe-then-dup

**Crate:** `capture-stdio`  
**Docs:** https://docs.rs/capture-stdio  
**How it works:** Two methods:
1. `std::io::set_output_capture` — same mechanism as `cargo test` uses for capturing test output. Nightly-only internal API.
2. Pipe-then-dup — creates a pipe, replaces the fd, reads from the pipe.

```rust
use capture_stdio::pipe::PipedStdout;

let mut capture = PipedStdout::capture().unwrap();
// stdout now goes to the pipe
println!("captured!");
let output = capture.read_to_string().unwrap();
capture.restore().unwrap(); // restore original stdout
```

**Pros:** Can actually capture and read the output.  
**Cons:** Same global fd problem. `PipedStdout::read_to_string()` blocks until the pipe is closed. Not async-friendly.

**Verdict:** ⚠️ Better than `gag` for testing, but still globally affects process fds. Not great for production TUI.

#### Approach C: `os_pipe` — Low-level pipe primitives (now std::io::pipe)

**Crate:** `os_pipe`  
**Docs:** https://docs.rs/os_pipe  
**Note:** Rust 1.87 added `std::io::pipe()`, making this crate unnecessary for new code.

```rust
let (reader, writer) = os_pipe::pipe()?;
let writer_clone = writer.try_clone()?;

let mut cmd = Command::new("some-tool");
cmd.stdout(writer).stderr(writer_clone);

let mut handle = cmd.spawn()?;
drop(cmd); // Close parent's copy of writer to avoid deadlock

let mut output = String::new();
reader.read_to_string(&mut output)?;
```

**Pros:** Clean, no fd swapping. Each child process gets its own pipe.  
**Cons:** Requires you to spawn the process yourself (not just call a function that happens to print).

**Verdict:** ✅ **Recommended approach for child processes.** Use `Command::stdout(Stdio::piped())` or `os_pipe` for child process output capture. This is the safe, non-global approach.

#### Approach D: Per-thread stdout via thread-local override

**Not a real crate, but the pattern:**

```rust
use std::cell::RefCell;
use std::io::Write;

thread_local! {
    static STDOUT_OVERRIDE: RefCell<Option<Box<dyn Write + Send>>> = RefCell::new(None);
}

// Override println-like behavior for your helpers
fn safe_println(s: &str) {
    STDOUT_OVERRIDE.with(|ov| {
        if let Some(ref mut w) = *ov.borrow_mut() {
            let _ = w.write_all(s.as_bytes());
            let _ = w.write_all(b"\n");
        } else {
            println!("{}", s);
        }
    });
}
```

**Verdict:** ⚠️ Requires modifying all output paths. Not practical for third-party code.

#### Approach E: Redirect stdout for tool threads using libc::dup2 (what claw-code likely has)

```rust
fn with_captured_stdout<F, R>(f: F) -> (R, String) 
where F: FnOnce() -> R {
    // Create pipe
    let (read_fd, write_fd) = {
        let mut fds = [0i32; 2];
        unsafe { libc::pipe(fds.as_mut_ptr()) };
        (fds[0], fds[1])
    };
    
    // Save original stdout
    let saved = unsafe { libc::dup(1) };
    
    // Replace stdout with pipe writer
    unsafe { libc::dup2(write_fd, 1) };
    unsafe { libc::close(write_fd) };
    
    // Run the function
    let result = f();
    
    // Restore original stdout
    unsafe { libc::dup2(saved, 1) };
    unsafe { libc::close(saved) };
    
    // Read captured output
    let mut output = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { libc::read(read_fd, buf.as_mut_ptr() as *mut libc::c_void, 4096) };
        if n <= 0 { break; }
        output.extend_from_slice(&buf[..n as usize]);
    }
    unsafe { libc::close(read_fd) };
    
    (result, String::from_utf8_lossy(&output).to_string())
}
```

**Verdict:** ⚠️ Works but has race conditions with other threads. The dup2 syscall is process-wide.

### 2.3 Recommended Architecture for claw-code

**The winning pattern is: don't fight fd 1 at all.**

1. **For child process execution:** Use `Command::stdout(Stdio::piped())` + `Command::stderr(Stdio::piped())`. Read from the pipes on a tokio task and forward output to the TUI via an `mpsc` channel. This is clean, no fd manipulation needed.

```rust
async fn run_tool(cmd: &mut Command, tx: &mpsc::Sender<AppEvent>) -> Result<ExitStatus> {
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    
    // Read in parallel using tokio
    let stdout_reader = tokio::io::BufReader::new(stdout);
    let stderr_reader = tokio::io::BufReader::new(stderr);
    
    // Spawn tasks to read lines and forward to TUI
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut lines = stdout_reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = tx_clone.send(AppEvent::ToolOutput { stream: "stdout", line }).await;
        }
    });
    // Similarly for stderr...
    
    child.wait().await
}
```

2. **For library code that prints to stdout:** Wrap it in `std::io::set_output_capture` (nightly/unstable) or accept that you need `dup2` with proper synchronization. The better long-term solution is to refactor such libraries to accept a `Write` sink.

3. **For the TUI itself:** Always write to the alternate screen via crossterm. The crossterm backend writes directly to `std::io::stderr()` by default (since ratatui 0.26+ with `CrosstermBackend::new(std::io::stderr())`), which means stdout is free for capture without affecting TUI rendering.

```rust
// Key insight: use stderr for the TUI backend, not stdout
let backend = CrosstermBackend::new(std::io::stderr());
let mut terminal = Terminal::new(backend)?;
```

This is the **#1 most important fix**: if ratatui renders to stderr, then stdout capture for tools becomes trivial because they operate on different file descriptors.

---

## 3. Async Runtime + Ratatui Patterns

### 3.1 The Right Architecture

Based on the ratatui FAQ and the official `async-template` (now `ratatui/templates`), the recommended architecture for a tokio + ratatui app is:

```
┌─────────────────────────────────────────────────────────┐
│                      Tokio Runtime                       │
│                                                         │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ Event Task  │  │ App Logic    │  │ Render Task    │  │
│  │             │  │ Tasks        │  │ (optional)     │  │
│  │ crossterm   │  │              │  │                │  │
│  │ events →   │──│→ mpsc       ──│──│→ terminal.draw │  │
│  │ tx          │  │ channel      │  │                │  │
│  └─────────────┘  └──────────────┘  └───────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 3.2 Event Handling: Poll vs EventStream

**Option A: `crossterm::event::poll()` + `read()` (recommended)**

```rust
fn spawn_event_reader(tx: mpsc::Sender<AppEvent>) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            // Poll with timeout to allow checking for shutdown
            if crossterm::event::poll(Duration::from_millis(100)).unwrap() {
                let event = crossterm::event::read().unwrap();
                if tx.send(AppEvent::Crossterm(event)).await.is_err() {
                    break; // Receiver dropped, shutdown
                }
            }
        }
    })
}
```

**Option B: `crossterm::EventStream` (async-stream)**

```rust
use crossterm::event::EventStream;
use futures::StreamExt;

fn spawn_event_stream(tx: mpsc::Sender<AppEvent>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut reader = EventStream::new();
        while let Some(event) = reader.next().await {
            match event {
                Ok(event) => {
                    if tx.send(AppEvent::Crossterm(event)).await.is_err() {
                        break;
                    }
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
    })
}
```

**Recommendation:** Use **Option A (poll-based)**. Here's why:
- `EventStream` pulls in `futures-core` and has had quirky edge cases around terminal teardown
- Poll-based is simpler and more reliable across platforms
- The 100ms poll timeout doubles as both event sampling and an idle timer
- Works identically on all platforms without the `event-stream` feature flag

### 3.3 Main Loop Pattern

```rust
async fn run_app(mut terminal: Terminal<CrosstermBackend<Stderr>>, mut app: App) -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<AppEvent>(100);
    
    // Spawn event reader
    let event_task = spawn_event_reader(tx.clone());
    
    // Spawn LLM streaming task
    let llm_task = spawn_llm_handler(tx.clone());
    
    // Tick rate: how often we get "tick" events for animations/updates
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();
    
    loop {
        // Draw the UI
        terminal.draw(|f| app.draw(f))?;
        
        // Wait for the next event, with a tick timeout
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        let event = tokio::select! {
            event = rx.recv() => event,
            _ = tokio::time::sleep(timeout) => Some(AppEvent::Tick),
        };
        
        match event {
            Some(AppEvent::Crossterm(event)) => app.handle_event(event)?,
            Some(AppEvent::Tick) => {
                last_tick = Instant::now();
                app.on_tick();
            }
            Some(AppEvent::LlmToken(token)) => app.on_token(token),
            Some(AppEvent::ToolOutput { stream, line }) => app.on_tool_output(stream, line),
            None => break, // Channel closed = shutdown
        }
    }
    
    Ok(())
}
```

### 3.4 Tick Rate / Frame Rate Strategy

**Key insight from the ratatui FAQ and async-template:**

- **Tick rate** (app logic updates): 4-10 Hz (100-250ms). Controls how often animations, progress bars, and streaming text updates are pushed.
- **Frame rate** is effectively "render on every event + tick." Since ratatui uses double-buffering and only renders diffs, even 60fps rendering is cheap.
- **For streaming chat:** Don't render on every single token. Batch tokens and render on ticks. This reduces CPU usage with no visible degradation.

```rust
struct App {
    /// Accumulated but not-yet-rendered tokens
    pending_tokens: String,
    /// Whether we need to re-render
    dirty: bool,
}

impl App {
    fn on_token(&mut self, token: &str) {
        self.pending_tokens.push_str(token);
        self.dirty = true;
        // Don't render here — let the tick/render loop handle it
    }
    
    fn on_tick(&mut self) {
        if self.dirty {
            // Flush pending tokens to the display buffer
            self.flush_tokens();
            self.dirty = false;
        }
    }
}
```

### 3.5 Windows Key Event Gotcha

From ratatui FAQ — on Windows, key events fire twice (Press + Release). Filter:

```rust
CrosstermEvent::Key(key) => {
    if key.kind == KeyEventKind::Press {
        tx.send(AppEvent::Key(key)).await.unwrap();
    }
}
```

---

## 4. Scroll State Management

### 4.1 The Core Problem

Word-wrapped content in ratatui produces variable-height rows. `Paragraph::wrap()` handles rendering, but scroll state must be managed externally because:
- `Paragraph` with `.scroll((offset, 0))` scrolls by *lines*, not by *items*
- With variable-height items, line-based scroll doesn't map to item-based scroll
- The total number of lines is unknown until you render at a given width

### 4.2 How tui-textarea Handles Scroll-to-Cursor

**Crate:** `tui-textarea` (by rhysd)  
**Repo:** https://github.com/rhysd/tui-textarea

`tui-textarea` maintains its own scroll state with a sophisticated model:

```rust
pub struct TextArea<'a> {
    // Lines of text (each line is a Spans)
    lines: Vec<Line<'a>>,
    // Cursor position (row, col in the logical text)
    cursor: (usize, usize),
    // Scroll offset (row, col in the viewport)
    scroll: (u16, u16),
    // Viewport size (updated on each render)
    size: (u16, u16),
}
```

**Scroll-to-cursor logic:**
1. On each render, check if the cursor is within the visible viewport
2. If cursor is above the viewport → scroll up
3. If cursor is below the viewport → scroll down
4. The scroll offset is in *line* units, not character units
5. Soft wrapping is handled by breaking lines into visual lines and tracking visual line count

**Key takeaway:** The textarea keeps a flat array of visual lines after wrapping, and scroll operates on visual-line indices. This is the model claw-code should adopt for chat history.

### 4.3 Custom Virtual List / Lazy Rendering

No existing crate provides this for variable-height content. Here's the recommended data structure:

```rust
/// A chat history with efficient scrolling over variable-height items.
pub struct ChatHistory {
    /// All messages in the conversation
    messages: Vec<ChatMessage>,
    /// Pre-computed visual line heights for each message at the current width
    /// (message_index → number of visual lines)
    visual_heights: Vec<u16>,
    /// Prefix sums of visual_heights for O(log n) item→line and line→item lookups
    line_offsets: Vec<usize>,
    /// Current scroll position (visual line index from top)
    scroll_line: usize,
    /// Viewport height in lines
    viewport_height: u16,
    /// Last width used for height computation (to detect resize)
    cached_width: u16,
    /// Whether to auto-scroll to bottom on new content
    auto_scroll: bool,
}

impl ChatHistory {
    /// Recompute visual heights after width change or new message
    pub fn recompute(&mut self, width: u16) {
        if width == self.cached_width && self.visual_heights.len() == self.messages.len() {
            return; // Nothing changed
        }
        self.cached_width = width;
        self.visual_heights = self.messages.iter()
            .map(|msg| msg.visual_line_count(width))
            .collect();
        self.line_offsets = std::iter::once(0usize)
            .chain(self.visual_heights.iter().scan(0, |acc, &h| {
                *acc += h as usize;
                Some(*acc)
            }))
            .collect();
    }
    
    /// Find the message index at a given visual line
    pub fn message_at_line(&self, line: usize) -> usize {
        self.line_offsets.partition_point(|&offset| offset <= line).saturating_sub(1)
    }
    
    /// Get the range of messages visible in the current viewport
    pub fn visible_range(&self) -> Range<usize> {
        let start = self.message_at_line(self.scroll_line);
        let end_line = self.scroll_line + self.viewport_height as usize;
        let end = self.message_at_line(end_line.min(self.total_lines() - 1)) + 1;
        start..end.min(self.messages.len())
    }
    
    /// Total visual lines
    pub fn total_lines(&self) -> usize {
        self.line_offsets.last().copied().unwrap_or(0)
    }
    
    /// Add a message and update scroll
    pub fn push(&mut self, msg: ChatMessage, width: u16) {
        self.messages.push(msg);
        self.recompute(width);
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }
    
    pub fn scroll_to_bottom(&mut self) {
        let total = self.total_lines();
        self.scroll_line = total.saturating_sub(self.viewport_height as usize);
    }
}
```

### 4.4 ratatui's Built-in Scroll

`Paragraph` supports `.scroll((y_offset, x_offset))` where `y_offset` is in visual lines. For a single-paragraph chat view (flattening all messages into one `Text`), this works but loses per-message metadata.

`List` widget supports `ListState` with `.scroll()` but operates on item indices, not visual lines — no help with word-wrapped items.

---

## 5. Permission Prompts in TUI

### 5.1 tui-popup: Modal Overlay Widget

**Crate:** `tui-popup` (by joshka, official ratatui org)  
**Docs:** https://docs.rs/tui-popup  
**Repo:** Part of https://github.com/ratatui/tui-widgets

The most mature popup/overlay widget for ratatui. Supports:

```rust
use tui_popup::Popup;

fn render_permission_popup(frame: &mut Frame) {
    let popup = Popup::new("Allow file write to /etc/hosts?\n\n[y] Yes  [n] No  [a] Always allow")
        .title("Permission Required")
        .style(Style::new().white().on_red());
    frame.render_widget(popup, frame.area());
}
```

**Features:**
- Auto-centers on the terminal area
- Auto-sizes to content
- Movable (via `PopupState` and arrow keys)
- Draggable by mouse (via `PopupState::mouse_down/up/drag()`)
- Supports wrapping arbitrary widgets (via `KnownSizeWrapper`) for scrollable popups
- Border style customization

**Stateful usage for interactive popups:**

```rust
use tui_popup::{Popup, PopupState};

fn render_stateful(frame: &mut Frame, state: &mut PopupState) {
    let popup = Popup::new("Allow this action?")
        .title("Permission")
        .style(Style::new().white().on_blue());
    frame.render_stateful_widget(popup, frame.area(), state);
}
```

### 5.2 tui-prompts: Input/Prompt Widgets

**Crate:** `tui-prompts` (by joshka, official ratatui org)  
**Docs:** https://docs.rs/tui-prompts  

Note: This is for text input prompts (like readline), not permission dialogs. But its `Confirm` prompt type (planned) could be useful.

Currently supports:
- `TextPrompt` — text input with emacs-style keybindings
- Password and invisible render styles
- Multi-line input with soft wrapping
- Prompt completion flow (Enter → complete, Escape → abort)

```rust
use tui_prompts::{Prompt, TextPrompt, TextState};

struct App<'a> {
    state: TextState<'a>,
}

// In render:
TextPrompt::from("Allow? (y/n)").draw(frame, area, &mut app.state);

// In event handler:
if app.state.is_completed() {
    let answer = app.state.value();
    // Process answer
}
```

### 5.3 Custom Overlay Pattern (recommended for claw-code)

For a permission prompt with [y/n/a] options, neither tui-popup nor tui-prompts is a perfect fit. The recommended approach is a custom overlay:

```rust
struct PermissionDialog {
    title: String,
    message: String,
    options: Vec<PermissionOption>,
    selected: usize,
    // Position for dragging
    position: (u16, u16),
}

enum PermissionOption {
    Allow,
    Deny,
    AlwaysAllow,
    AllowThisSession,
}

impl Widget for &PermissionDialog {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 1. Dim the background
        Block::default().style(Style::default().bg(Color::DarkGray)).render(area, buf);
        
        // 2. Calculate popup area (centered, sized to content)
        let popup_area = centered_rect(area, 60, 30);
        
        // 3. Draw the popup with border
        let block = Block::bordered()
            .title(self.title.as_str())
            .style(Style::default().fg(Color::Yellow));
        let inner = block.inner(popup_area);
        block.render(popup_area, buf);
        
        // 4. Render the message and options
        let text = Text::from(vec![
            Line::from(self.message.as_str()),
            Line::from(""),
            Line::from(self.options.iter().enumerate().map(|(i, opt)| {
                let label = match opt {
                    PermissionOption::Allow => "[y] Yes",
                    PermissionOption::Deny => "[n] No",
                    PermissionOption::AlwaysAllow => "[a] Always",
                    PermissionOption::AllowThisSession => "[s] Session",
                };
                if i == self.selected {
                    Span::styled(label, Style::default().fg(Color::Cyan).bold())
                } else {
                    Span::raw(label)
                }
            }).collect::<Vec<_>>()),
        ]);
        Paragraph::new(text).render(inner, buf);
    }
}

/// Utility: calculate centered rect
fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
```

This is exactly the pattern used in ratatui's own examples (`popup.rs`) and in codex-rs.

### 5.4 Inline Permission Prompt (alternative)

Instead of a modal overlay, some TUIs (like aichat) use an inline prompt at the bottom of the screen:

```
┌─ Chat ──────────────────────────────────────────────┐
│ Assistant: I'll create the file config.toml ...      │
│                                                       │
├─ Permission ────────────────────────────────────────┤
│ Allow write to config.toml? [y=Yes, n=No, a=Always] │
└───────────────────────────────────────────────────────┘
```

This is simpler (no overlay logic) and more accessible. Implementation: modify the layout to split off a bottom panel when a permission prompt is active.

---

## 6. Testing TUIs in Rust

### 6.1 How codex-rs Tests Their TUI

codex-rs uses a layered approach (based on public source analysis):

1. **Business logic**: Pure functions / state machines with standard `#[test]` unit tests
2. **Widget rendering**: Test the output of `Widget::render()` to a `Buffer`, then assert on buffer contents
3. **No snapshot testing of TUI output** (no insta for buffer comparison found in their public repo)

### 6.2 Widget Unit Testing Pattern (recommended)

The core pattern for testing ratatui widgets:

```rust
#[test]
fn test_chat_message_render() {
    let message = ChatMessage::assistant("Hello, world!");
    
    // Create a buffer to render into
    let area = Rect::new(0, 0, 40, 3);
    let mut buf = Buffer::empty(area);
    
    // Render the widget
    message.render(area, &mut buf);
    
    // Assert on buffer contents
    assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "A");
    assert!(buf.cell((2, 0)).unwrap().symbol().starts_with("Hello"));
    
    // Or check a whole line
    let line_content: String = (0..40)
        .map(|x| buf.cell((x, 0)).unwrap().symbol())
        .collect();
    assert!(line_content.contains("Hello, world!"));
}
```

### 6.3 Insta Snapshot Testing of Rendered Output

**Crate:** `insta`  
**Docs:** https://insta.rs  

Insta can be used to snapshot-test ratatui `Buffer` contents:

```rust
use insta::assert_snapshot;

#[test]
fn test_permission_dialog() {
    let dialog = PermissionDialog::new("Allow write?", vec![
        PermissionOption::Allow,
        PermissionOption::Deny,
    ]);
    
    let area = Rect::new(0, 0, 40, 5);
    let mut buf = Buffer::empty(area);
    dialog.render(area, &mut buf);
    
    // Snapshot the buffer as a string
    let rendered = buffer_to_string(&buf, area);
    assert_snapshot!(rendered);
}

/// Convert a buffer to a human-readable string representation
fn buffer_to_string(buf: &Buffer, area: Rect) -> String {
    let mut out = String::new();
    for y in area.top..area.bottom {
        for x in area.left..area.right {
            out.push_str(buf.cell((x, y)).unwrap().symbol());
        }
        out.push('\n');
    }
    out
}
```

**Workflow:**
1. Run `cargo test` — new snapshots are written to `.snap.new` files
2. Run `cargo insta review` — interactively accept/reject snapshots
3. Accepted snapshots are saved as `.snap` files in `snapshots/` directory

### 6.4 Mock Crossterm Events for Testing

No dedicated crate exists for mocking crossterm events. The recommended patterns:

#### Pattern A: Abstract the event source

```rust
trait EventSource {
    fn next_event(&mut self) -> Option<AppEvent>;
}

struct CrosstermEventSource;
impl EventSource for CrosstermEventSource {
    fn next_event(&mut self) -> Option<AppEvent> {
        if crossterm::event::poll(Duration::from_millis(100)).ok()? {
            let ev = crossterm::event::read().ok()?;
            Some(AppEvent::from(ev))
        } else {
            None
        }
    }
}

struct MockEventSource {
    events: Vec<AppEvent>,
    index: usize,
}
impl EventSource for MockEventSource {
    fn next_event(&mut self) -> Option<AppEvent> {
        if self.index < self.events.len() {
            let ev = self.events[self.index].clone();
            self.index += 1;
            Some(ev)
        } else {
            None
        }
    }
}
```

#### Pattern B: Channel-based event injection

```rust
#[cfg(test)]
fn test_app_with_events(events: Vec<AppEvent>) {
    let (tx, rx) = mpsc::channel();
    for event in events {
        tx.send(event).unwrap();
    }
    drop(tx); // Close channel to end test
    
    let mut app = App::new();
    while let Ok(event) = rx.recv() {
        app.handle_event(event);
    }
    // Assert on final app state
}
```

#### Pattern C: VHS for integration testing

**Tool:** VHS (by charmbracelet)  
**Repo:** https://github.com/charmbracelet/vhs  

VHS lets you record terminal sessions from a script. Useful for end-to-end visual testing:

```bash
# Record a demo / test
Output tui_demo.gif
Type "hello world"
Enter
Sleep 2s
```

This is more for documentation than CI testing, but can be automated.

### 6.5 Testing Async TUIs

For testing the integration between tokio and ratatui:

```rust
#[tokio::test]
async fn test_streaming_display() {
    let (tx, rx) = mpsc::channel(100);
    
    // Simulate streaming tokens
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        for word in ["Hello", ", ", "world", "!"] {
            tx_clone.send(AppEvent::LlmToken(word.to_string())).await.unwrap();
        }
    });
    
    // Run app logic without actual terminal
    let mut app = App::new();
    while let Ok(event) = rx.recv().await {
        app.handle_event(event);
    }
    
    assert_eq!(app.chat_buffer, "Hello, world!");
}
```

---

## Summary: Recommended Crate Stack for claw-code TUI

| Problem | Recommended Solution | Crate |
|---------|---------------------|-------|
| Markdown rendering | `tui-markdown` | `tui-markdown = "0.x"` |
| ANSI color passthrough | `ansi-to-tui` | `ansi-to-tui = "4.x"` |
| Scrollable viewport | `tui-scrollview` | `tui-scrollview = "0.x"` |
| Popup/modal dialogs | `tui-popup` + custom overlay | `tui-popup = "0.x"` |
| Text input prompts | `tui-prompts` | `tui-prompts = "0.x"` |
| Child process output capture | `Stdio::piped()` + tokio async read | std + tokio |
| Event handling | `crossterm::event::poll()` | (crossterm, no event-stream feature) |
| Snapshot testing | `insta` | `insta = "1.x"` |
| TUI rendering backend | `CrosstermBackend::new(std::io::stderr())` | ratatui + crossterm |
| Logging inside TUI | `tui-logger` (optional) | `tui-logger = "0.x"` |
| Virtual scroll / chat history | Custom implementation | (no crate — see §4.3) |

### Critical Architecture Decisions

1. **Render to stderr, not stdout.** This solves 90% of the output isolation problem. Use `CrosstermBackend::new(std::io::stderr())`.

2. **Use `Stdio::piped()` for child processes.** Don't use `dup2`/`gag`/fd capture. Read from pipes on tokio tasks.

3. **Poll-based event handling** over `EventStream`. Simpler, more reliable, no extra deps.

4. **Custom virtual scroll** for chat history. No existing crate handles variable-height items well. Build your own with prefix-sum height lookups.

5. **Custom overlay for permission dialogs.** `tui-popup` is good for simple popups, but permission prompts with [y/n/a] options warrant a custom widget that integrates with the app's keybinding system.

6. **Batch streaming tokens on tick.** Don't re-render on every token. Flush pending tokens on tick (4-10 Hz), which is imperceptibly different from per-token rendering but much more efficient.

7. **Insta snapshots** for widget render tests. Combined with `Buffer`-based rendering assertions, this gives comprehensive coverage.
