# TUI Architecture Analysis: aichat (sigoden/aichat)

> **Date:** 2025-06-12  
> **Version analyzed:** 0.30.0 (main branch, commit 82976d3)  
> **Purpose:** Extract streaming markdown rendering patterns for claw-code's TUI

---

## 1. Executive Summary

aichat is a Rust LLM CLI that implements a **line-oriented REPL** (via `reedline`) with **incremental terminal rendering** (via `crossterm` raw mode). It is **not a full TUI** — there is no event loop, no ratatui, no widget tree. Instead, it takes over stdout in raw mode during streaming to perform surgical "erase-and-redraw" of the current rendering buffer. This approach is remarkably effective: it provides flicker-free streaming markdown with syntax highlighting, word-wrap, and CJK support at ~20 FPS while keeping the architecture simple.

---

## 2. Architecture Overview

### 2.1 REPL vs Streaming Boundary

```
┌─────────────────────────────────────────────────────────┐
│  Repl (reedline-based)                                  │
│  - Read line via reedline (with completer, highlighter) │
│  - Dispatch to run_repl_command() / ask()               │
│  - After ask() returns, reedline resumes               │
│                                                          │
│  During streaming:                                       │
│  ┌──────────────────────────────────────────────┐       │
│  │  raw mode ON → crossterm direct writes        │       │
│  │  MarkdownRender + stream.rs markdown_stream   │       │
│  │  raw mode OFF → back to reedline              │       │
│  └──────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────┘
```

**Key insight:** aichat does NOT use a persistent TUI framework. The REPL (reedline) owns the terminal normally. When streaming starts, it enters `crossterm::raw_mode`, takes over stdout, and when done, exits raw mode and returns control to reedline. This is a **modal design**: REPL mode ↔ streaming mode.

### 2.2 Call Graph for Streaming

```
ask()
  → call_chat_completions_streaming(input, client, abort_signal)
      → tokio::join!(
            client.chat_completions_streaming(input, &mut handler),  // SSE producer
            render_stream(rx, config, abort_signal)                  // Render consumer
        )

render_stream()
  → if IS_STDOUT_TERMINAL && config.highlight:
      MarkdownRender::init(render_options)
      markdown_stream(rx, &mut render, abort_signal)   // ★ Main rendering pipeline
  → else:
      raw_stream(rx, abort_signal)                      // Plain text passthrough
```

The `tokio::join!` means SSE parsing/rendering runs concurrently — the render side processes buffered events on a 50ms tick.

---

## 3. Streaming Markdown Rendering Pipeline (Deep Dive)

### 3.1 The `markdown_stream_inner` Function

This is the core ~80-line function in `src/render/stream.rs`. Here's the algorithm:

```
1. Enable raw mode
2. Read terminal columns
3. Initialize: buffer = "", buffer_rows = 1
4. Show spinner until first event
5. Loop:
   a. gather_events() — collect all pending events with 50ms timeout
   b. For each Text event:
      i.   Replace tabs with 4 spaces
      ii.  Get cursor position (with retry, up to 3 attempts)
      iii. Fix kitty dup-line bug (if col==0 && row>0 && buffer fills columns, row--)
      iv.  Reposition cursor:
           - If cursor row+1 >= buffer_rows → MoveTo(0, row+1-buffer_rows)
           - If cursor behind → ScrollUp + MoveTo(0,0)
      v.   Clear from cursor down (terminal::Clear::FromCursorDown)
      vi.  Append text to buffer
      vii. If text contains newlines:
           - Split buffer at last newline → (head, tail)
           - render(head) → print_block (complete lines)
           - buffer = tail
      viii. Else: buffer = buffer + text
      ix.  render_line(&buffer) → render current incomplete line
      x.   If rendered output contains newlines (from wrapping):
           - print_block the head portion
           - Print the tail portion
           - Recalculate buffer_rows
      xi.  Else: Print the rendered output, calculate buffer_rows
      xii. Flush stdout
   c. For Done event: break
   d. Poll abort signal (Ctrl+C / Ctrl+D with 25ms timeout)
6. Disable raw mode
```

### 3.2 Event Batching: `gather_events()`

```rust
async fn gather_events(rx: &mut UnboundedReceiver<SseEvent>) -> Vec<SseEvent> {
    let mut texts = vec![];
    let mut done = false;
    tokio::select! {
        _ = async {
            while let Some(reply_event) = rx.recv().await {
                match reply_event {
                    SseEvent::Text(v) => texts.push(v),
                    SseEvent::Done => { done = true; break; }
                }
            }
        } => {}
        _ = tokio::time::sleep(Duration::from_millis(50)) => {}
    };
    // Merge all text chunks into one, preserving order
    if !texts.is_empty() {
        events.push(SseEvent::Text(texts.join("")))
    }
    ...
}
```

**Design:** Collect all pending text events, join them into a single string, and process in one render pass. The 50ms timeout prevents blocking indefinitely. This means the render loop runs at approximately **20 FPS max** (1000/50), which is smooth enough for terminal output.

**Why this matters:** Batching reduces the number of full re-render cycles. Instead of re-rendering for every SSE chunk (which can arrive every few ms), it batches them every 50ms. This is crucial for avoiding visual glitching.

### 3.3 The "Erase and Redraw" Strategy

This is the most important pattern. Instead of trying to do incremental diffs on the terminal, aichat:

1. **Tracks how many rows the current buffer occupies** (`buffer_rows`)
2. **Moves cursor back to the start of the buffer** (using the stored row position)
3. **Clears everything below cursor** (`Clear::FromCursorDown`)
4. **Re-renders the full buffer** through MarkdownRender
5. **Prints and tracks** the new row count

This is simple but effective because:
- Terminal writes are fast (ANSI escape sequences)
- The buffer is typically small (a few KB of text)
- `Clear::FromCursorDown` is a single escape sequence — very fast

### 3.4 Buffer Management

```
buffer: String      // The raw, un-rendered text of the current "paragraph" being streamed
buffer_rows: u16    // How many terminal rows the rendered buffer currently occupies

After each text event:
  - Newlines that appear → split into "complete lines" (rendered+printed) and "tail" (new buffer)
  - The buffer always holds the incomplete last line
  - render_line() is called on the buffer for the in-progress line
```

**The split pattern:**
```rust
if text.contains('\n') {
    let text = format!("{buffer}{text}");
    let (head, tail) = split_line_tail(&text);
    let output = render.render(head);       // render complete lines
    print_block(writer, &output, columns)?;
    buffer = tail.to_string();               // keep incomplete line as buffer
} else {
    buffer = format!("{buffer}{text}");
}
```

`split_line_tail` splits at the last newline — everything before is "committed" lines, everything after is the incomplete line.

### 3.5 Cursor Position Recovery

```rust
let mut attempts = 0;
let (col, mut row) = loop {
    match cursor::position() {
        Ok(pos) => break pos,
        Err(_) if attempts < 3 => attempts += 1,
        Err(e) => return Err(e.into()),
    }
};

// Fix kitty terminal duplicate line bug
if col == 0 && row > 0 && display_width(&buffer) == columns as usize {
    row -= 1;
}
```

**Defensive:** Cursor position queries can fail — retries up to 3 times. The kitty fix handles a specific terminal quirk where the auto-wrap creates an extra row.

### 3.6 Scroll Handling

```rust
if row + 1 >= buffer_rows {
    // Normal case: cursor is at or below buffer start
    queue!(writer, cursor::MoveTo(0, row + 1 - buffer_rows))?;
} else {
    // Edge case: cursor is above buffer start (scroll happened)
    let scroll_rows = buffer_rows - row - 1;
    queue!(
        writer,
        terminal::ScrollUp(scroll_rows),
        cursor::MoveTo(0, 0),
    )?;
}
```

**No user-scroll distinction.** aichat always auto-scrolls. There's no scrollback buffer or user-scroll detection. This is a limitation of the line-oriented approach — once text scrolls off-screen, it's gone from the render buffer (though still in the terminal scrollback).

---

## 4. MarkdownRender — Syntax Highlighting & Wrapping

### 4.1 State Machine: LineType

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineType {
    Normal,      // Outside code block
    CodeBegin,   // ```lang line
    CodeInner,   // Inside code block
    CodeEnd,     // Closing ``` line
}
```

The render state machine tracks whether each line is inside a code block or not. This is critical because:
- Code blocks get special syntax highlighting
- Code blocks have different wrapping rules
- The code language (from ` ```lang `) determines which syntect syntax to use

### 4.2 Per-Line Rendering

```rust
fn render_line_mut(&mut self, line: &str) -> String {
    let (line_type, code_syntax, is_code) = self.check_line(line);
    let output = if is_code {
        self.highlight_code_line(line, &code_syntax)
    } else {
        self.highlight_line(line, &self.md_syntax, false)
    };
    self.prev_line_type = line_type;
    self.code_syntax = code_syntax;
    output
}
```

**Two render modes:**
1. **`render(&mut self, text)`** — Stateful, tracks LineType across lines. Used for full re-renders.
2. **`render_line(&self, line)`** — Stateless snapshot, used for the in-progress buffer line during streaming.

### 4.3 Syntax Highlighting (syntect)

```rust
fn highlight_line(&self, line: &str, syntax: &SyntaxReference, is_code: bool) -> String {
    // Strip leading whitespace, highlight trimmed part, re-add whitespace
    let ws: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    let trimmed_line: &str = &line[ws.len()..];
    
    if let Some(theme) = &self.options.theme {
        let mut highlighter = HighlightLines::new(syntax, theme);
        if let Ok(ranges) = highlighter.highlight_line(trimmed_line, &self.syntax_set) {
            return format!("{ws}{}", as_terminal_escaped(&ranges, self.options.truecolor));
        }
    }
    // Fallback: no theme → plain text
    self.wrap_line(line.into(), is_code)
}
```

**Key pattern:** aichat uses syntect's `HighlightLines` which is designed for streaming — you create a highlighter per line and it maintains state across lines (for multi-line strings, comments, etc.). The `as_terminal_escaped` function converts syntect style ranges to ANSI escape codes.

### 4.4 Truecolor vs 256-Color

```rust
fn convert_color(c: SyntectColor, truecolor: bool) -> Color {
    if truecolor {
        Color::Rgb { r: c.r, g: c.g, b: c.b }
    } else {
        let value = (c.r, c.g, c.b).to_ansi256();
        let value = match value {
            7 | 15 | 231 | 252..=255 => 252,  // Lower contrast for readability
            _ => value,
        };
        Color::AnsiValue(value)
    }
}
```

Checks `COLORTERM=truecolor` env var. If not available, falls back to 256-color with contrast adjustments for readability.

### 4.5 Alpha Blending for Foreground

```rust
fn blend_fg_color(fg: SyntectColor, bg: SyntectColor) -> SyntectColor {
    if fg.a == 0xff { return fg; }
    let ratio = u32::from(fg.a);
    let r = (u32::from(fg.r) * ratio + u32::from(bg.r) * (255 - ratio)) / 255;
    // ... same for g, b
    SyntectColor { r, g, b, a: 255 }
}
```

Handles syntect themes that specify semi-transparent foreground colors by alpha-blending with the theme background. This prevents washed-out colors on dark backgrounds.

### 4.6 Word Wrapping

```rust
fn wrap_line(&self, line: String, is_code: bool) -> String {
    if let Some(width) = self.wrap_width {
        if is_code && !self.options.wrap_code {
            return line;  // Don't wrap code unless explicitly configured
        }
        wrap(&line, width as usize)
    } else {
        line
    }
}

fn wrap(text: &str, width: usize) -> String {
    let indent: usize = text.chars().take_while(|c| *c == ' ').count();
    let wrap_options = textwrap::Options::new(width)
        .wrap_algorithm(textwrap::WrapAlgorithm::FirstFit)
        .initial_indent(&text[0..indent]);
    textwrap::wrap(&text[indent..], wrap_options).join("\n")
}
```

**Uses `textwrap` crate** with `FirstFit` algorithm (fast, greedy). Preserves leading whitespace as initial indent. The `wrap_width` is computed at init time from `terminal::size()` (or config value).

**CJK support:** `textwrap` handles CJK characters correctly via `unicode-width` (a dependency of textwrap) — CJK characters are width 2, which is respected in wrapping calculations.

### 4.7 Row Calculation

```rust
fn need_rows(text: &str, columns: u16) -> u16 {
    let buffer_width = display_width(text).max(1) as u16;
    buffer_width.div_ceil(columns)
}
```

Uses `textwrap::core::display_width` for Unicode-aware width calculation, then divides by columns to get row count. This correctly handles CJK (2-width chars), emojis (2-width), and combining characters.

---

## 5. SSE Stream Pipeline

### 5.1 SseHandler (Producer)

```rust
pub struct SseHandler {
    sender: UnboundedSender<SseEvent>,
    abort_signal: AbortSignal,
    buffer: String,        // Full text accumulated
    tool_calls: Vec<ToolCall>,
}
```

The handler receives SSE events from the HTTP client and:
1. Appends text to both `self.buffer` (full accumulation) and sends `SseEvent::Text` to the render channel
2. Sends `SseEvent::Done` when the stream completes
3. Accumulates tool calls separately

**Dual purpose of `buffer`:** The `SseHandler.buffer` accumulates the **complete** response text (for session storage/tool-call context), while `SseEvent::Text` sends just the **incremental** chunk to the renderer. This separation is important — the render pipeline only needs the delta.

### 5.2 Two Stream Protocols

```rust
// SSE (Server-Sent Events) — OpenAI-compatible
pub async fn sse_stream<F>(builder: RequestBuilder, mut handle: F) -> Result<()>

// JSON stream — AWS Bedrock, etc.
pub async fn json_stream<S, F, E>(mut stream: S, mut handle: F) -> Result<()>
```

Both funnel through the same `SseHandler` → `UnboundedReceiver<SseEvent>` pipeline.

### 5.3 JSON Stream Parser

The `JsonStreamParser` is a hand-written incremental JSON parser that:
1. Buffers incoming characters
2. Tracks `{`/`}` and `[`/`]` balance
3. Respects string quoting (with escape handling)
4. Extracts complete JSON objects as they're balanced
5. Handles both NDJSON and JSON array formats
6. Handles partial UTF-8 at chunk boundaries

This is notably robust — it doesn't depend on newlines for framing, and handles arbitrary chunk splits from the HTTP layer.

---

## 6. Error Recovery & Abort Handling

### 6.1 AbortSignal

```rust
pub struct AbortSignalInner {
    ctrlc: AtomicBool,
    ctrld: AtomicBool,
}
```

Shared via `Arc`, using `AtomicBool` with `SeqCst` ordering. Both the REPL and the streaming loop check for abort:
- **REPL:** Sets `ctrlc`/`ctrld` on keypress via reedline's Signal handling
- **Streaming:** Checks via `poll_abort_signal` which uses `crossterm::event::poll` with 25ms timeout

### 6.2 Error During Stream

```rust
// In call_chat_completions_streaming:
let (send_ret, render_ret) = tokio::join!(
    client.chat_completions_streaming(input, &mut handler),
    render_stream(rx, client.global_config(), abort_signal.clone()),
);

// After join:
if handler.abort().aborted() {
    bail!("Aborted.");
}
render_ret?;   // Propagate render errors

match send_ret {
    Ok(_) => {
        if !text.is_empty() && !text.ends_with('\n') {
            println!();  // Ensure trailing newline on clean completion
        }
        Ok((text, eval_tool_calls(client.global_config(), tool_calls)?))
    }
    Err(err) => {
        if !text.is_empty() {
            println!();  // Ensure separation before error display
        }
        Err(err)
    }
}
```

**Partial output preserved:** If the stream errors, the accumulated `buffer` from `SseHandler.take()` still contains whatever was received. The error is reported, but partial text is not thrown away. The `markdown_stream_inner` also ensures raw mode is disabled on error path via the `disable_raw_mode()` call after `markdown_stream_inner` returns.

### 6.3 Connection Drops

The SSE stream handler (`sse_stream`) maps `EventSourceError` variants:
- `StreamEnded` → Normal completion, no error
- `InvalidStatusCode` → Parse error response body and surface the error message
- `InvalidContentType` → Surface the unexpected content type
- Other errors → Generic error propagation

There's no automatic retry. Connection drops result in partial output + error message.

---

## 7. Spinner

```rust
pub struct SpinnerInner {
    index: usize,
    message: String,
}
```

Runs as a separate tokio task, stepping at 50ms intervals. Uses braille pattern animation (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏). Hides cursor while active, restores on stop. Communicates via `mpsc::UnboundedSender<SpinnerEvent>`.

**Key detail:** When the first `SseEvent::Text` arrives during streaming, the spinner is explicitly stopped before rendering begins. This prevents the spinner from interfering with the render output.

---

## 8. Dependency Stack

| Crate | Purpose |
|-------|---------|
| `crossterm` 0.28 | Raw mode, cursor control, terminal clearing, color output |
| `syntect` 5.0 | Syntax highlighting (regex-onig backend, parsing + plist-load) |
| `textwrap` 0.16 | Unicode-aware word wrapping (CJK-width aware) |
| `unicode-width` 0.2 | Character width calculation (CJK=2, emoji=2) |
| `ansi_colours` 1.2 | Truecolor → 256-color conversion |
| `reedline` 0.40 | REPL line editing (not used during streaming) |
| `nu-ansi-term` 0.50 | ANSI color/styling output |
| `terminal_colorsaurus` 0.4 | Detect terminal light/dark color scheme |
| `reqwest-eventsource` 0.6 | SSE event parsing |

---

## 9. Patterns to Adopt for claw-code's TUI

### 9.1 ✅ ADOPT: The Erase-and-Redraw Pattern

aichat's approach of "move cursor to buffer start → clear below → re-render buffer" is the simplest correct solution for terminal streaming. It avoids the complexity of diffing rendered output while being fast enough for typical LLM output rates.

**For claw-code:** If using ratatui, this maps to a `LineArea` widget that knows its row count and can be fully re-rendered. If not using ratatui, directly adopt the crossterm queue pattern.

### 9.2 ✅ ADOPT: Event Batching with 50ms Tick

The `gather_events` pattern is critical for smooth rendering. Don't render on every SSE chunk — batch for 50ms. This gives:
- Reduced render cycles (from hundreds per second to ~20)
- Better visual consistency (no partial-word renders)
- Lower CPU usage

**For claw-code:** Use the same `tokio::select!` + `sleep(50ms)` pattern for event batching.

### 9.3 ✅ ADOPT: Buffer Tracking (buffer + buffer_rows)

The `buffer`/`buffer_rows` pattern for tracking the in-progress rendering state is clean and correct. The `buffer_rows` variable is essential for cursor repositioning after the clear.

**For claw-code:** Track the current rendering extent (rows) so you know where to start the next clear-and-redraw cycle.

### 9.4 ✅ ADOPT: State Machine for Code Block Detection

The `LineType` state machine (Normal → CodeBegin → CodeInner → CodeEnd) is the right way to handle streaming code blocks. You can't do this with a single pass parser because the closing ````` hasn't arrived yet during streaming.

**For claw-code:** Must implement a similar state machine. During streaming, you're rendering a partial markdown document — the code block status of the current line depends on whether an opening ````` was seen but no closing one.

### 9.5 ✅ ADOPT: Split-at-Last-Newline Buffer Pattern

Rendering complete lines (those ended by `\n`) immediately and keeping only the incomplete last line in the buffer reduces the amount of text that needs to be re-rendered each cycle.

**For claw-code:** Same pattern — commit rendered output for complete lines, only re-render the in-progress line.

### 9.6 ✅ ADOPT: Defensive Cursor Position Queries

The retry pattern for `cursor::position()` and the kitty terminal workaround are essential for production reliability.

**For claw-code:** Always retry cursor position queries (terminals can return errors during resize, etc.).

### 9.7 ✅ ADOPT: Truecolor Fallback Chain

`COLORTERM=truecolor` → RGB colors, else → 256-color with contrast adjustments. The contrast adjustment (replacing near-white values with 252) prevents readability issues on dark backgrounds.

**For claw-code:** Implement the same color depth detection and contrast adjustment.

### 9.8 ✅ ADOPT: textwrap for Word Wrapping

`textwrap` with `FirstFit` + `unicode-width` is battle-tested for CJK and emoji. Don't roll your own wrapping.

**For claw-code:** Use `textwrap` directly, with the same indent-preserving pattern.

### 9.9 ⚠️ CONSIDER: Full TUI vs Line-Oriented Approach

aichat's line-oriented approach works because it's a **chat interface** — input at the bottom, output scrolling up. For claw-code, which likely needs:
- Split pane (conversation + code preview)
- Interactive elements (file tree, diff view)
- Scroll position management (preserve scroll while streaming)

...a full TUI framework (ratatui) would be more appropriate. However, the **rendering principles** from aichat should be preserved:
- Event batching (don't re-render on every chunk)
- Buffer tracking (know your rendered extent)
- Erase-and-redraw within the streaming area

### 9.10 ⚠️ IMPROVE: Auto-Scroll vs User-Scroll

aichat has **NO user-scroll detection**. Once text scrolls off the visible area, it's gone from the render buffer. For claw-code:
- Detect user scroll (e.g., via mouse wheel or Page Up)
- Pause auto-scroll when user has scrolled up
- Resume auto-scroll when user scrolls to bottom
- This is only feasible with a TUI framework that manages a scrollable view

### 9.11 ⚠️ IMPROVE: Terminal Resize Handling

aichat captures `columns` once at the start of streaming. If the terminal is resized mid-stream, the wrap width will be wrong and rendering will break. For claw-code:
- Listen for SIGWINCH / crossterm resize events
- Recalculate wrap width on resize
- Re-render the full buffer with new width

### 9.12 ⚠️ IMPROVE: Incremental Highlighting

aichat re-highlights the entire current line on each render. For very long lines, this could be slow. For claw-code:
- Consider caching the syntect highlighter state per line
- Only re-highlight from the change point forward (though this is hard with markdown)

### 9.13 ❌ DON'T ADOPT: Raw Mode Takeover

aichat's approach of entering raw mode and doing direct cursor manipulation works for its REPL model but would conflict with a TUI framework like ratatui, which needs to own the terminal state. If using ratatui:
- Use ratatui's terminal abstraction
- Apply the **logical patterns** (erase-and-redraw, buffering, batching) but through the framework's widget lifecycle

---

## 10. Summary: Key Architectural Patterns

| Pattern | aichat's Implementation | Applicability to claw-code |
|---------|------------------------|---------------------------|
| Streaming render | Erase-and-redraw in raw mode | Adopt logic, adapt to ratatui widget |
| Event batching | 50ms `gather_events` | ✅ Direct adopt |
| Buffer state | `buffer` + `buffer_rows` tracking | ✅ Direct adopt |
| Code block state machine | `LineType` enum | ✅ Direct adopt |
| Syntax highlighting | syntect `HighlightLines` per line | ✅ Direct adopt |
| Word wrapping | textwrap + unicode-width | ✅ Direct adopt |
| Color depth | truecolor → 256 fallback | ✅ Direct adopt |
| Scroll management | None (auto-only) | ❌ Must improve for claw-code |
| Resize handling | None (one-time size) | ❌ Must improve for claw-code |
| Error recovery | Partial output preserved | ✅ Adopt the pattern |
| Cursor recovery | Retry + kitty workaround | ✅ Direct adopt |
| REPL boundary | Modal (reedline ↔ raw mode) | Adapt to TUI event loop |

---

## 11. Code References

All source from: `https://github.com/sigoden/aichat` (main branch)

| File | Purpose |
|------|---------|
| `src/render/mod.rs` | Render module entry, `render_stream()` dispatcher |
| `src/render/markdown.rs` | `MarkdownRender` — stateful markdown→ANSI renderer |
| `src/render/stream.rs` | `markdown_stream` — core streaming render loop ★ |
| `src/client/stream.rs` | SSE/JSON stream parsing, `SseHandler`, `SseEvent` |
| `src/client/common.rs` | `call_chat_completions_streaming()` — tokio::join of send+render |
| `src/repl/mod.rs` | REPL loop, `ask()` function |
| `src/config/mod.rs` | `render_options()`, `print_markdown()`, theme loading |
| `src/utils/abort_signal.rs` | `AbortSignal` — atomic Ctrl+C/D tracking |
| `src/utils/spinner.rs` | Spinner animation during generation wait |
