# Sprint 2: Streaming & Markdown Rendering

> **Duration:** 5 days | **Stories:** 6 | **Goal:** Real-time response display + rich markdown rendering
> **Depends on:** Sprint 0 (extraction), Sprint 1 (bug fixes)

---

## Why This Sprint Is 5 Days (Not 3)

Opus review correctly identified:
1. **Streaming rendering is missing entirely** — table stakes for any TUI competitor
2. **Syntect API errors** — `HighlightLines::new()` returns `Result`, code won't compile
3. **Syntect initialization is expensive** (10-50ms) — needs lazy loading
4. **Markdown rendering on every frame is too slow** — needs per-entry caching
5. **`looks_like_markdown()` heuristics** need careful handling

---

## S2-1: Add streaming response rendering

**Priority:** P0 — Critical  
**Estimate:** 1.5 days  

### Description
Currently, assistant responses are captured into a `Vec<u8>` buffer and rendered only after the turn completes. All competitors (Claude Code, OpenCode, Codex CLI) stream responses in real-time. The user should see tokens appear as they're generated.

### Implementation

**File:** `src/tui.rs` — add streaming state to `TuiApp`:

```rust
pub struct TuiApp {
    // ... existing fields
    
    /// Streaming state for current assistant response
    stream_buffer: String,
    stream_is_active: bool,
    stream_last_update: std::time::Instant,
}

impl TuiApp {
    /// Called by the streaming callback as new tokens arrive.
    /// Appends to the streaming buffer and triggers a redraw.
    pub fn push_stream_token(&mut self, token: &str) {
        self.stream_buffer.push_str(token);
        self.stream_is_active = true;
        self.stream_last_update = std::time::Instant::now();
        self.needs_redraw = true;
    }

    /// Called when the assistant's response is complete.
    /// Moves stream buffer into permanent conversation and resets.
    pub fn finish_stream(&mut self) {
        if !self.stream_buffer.is_empty() {
            // Check if the complete response looks like markdown
            if looks_like_markdown(&self.stream_buffer) {
                self.conversation.push(ConversationLine {
                    content: ConversationContent::Markdown {
                        source: self.stream_buffer.clone(),
                    },
                    rendered_cache: None,  // Will be rendered on next frame
                });
            } else {
                for line in self.stream_buffer.lines() {
                    self.conversation.push(ConversationLine {
                        content: ConversationContent::Plain {
                            text: line.to_string(),
                            color: Color::White,
                            bold: false,
                        },
                        rendered_cache: None,
                    });
                }
            }
        }
        self.stream_buffer.clear();
        self.stream_is_active = false;
        self.auto_scroll();
    }
}
```

**File:** `src/tui_repl.rs` — modify turn execution:

```rust
// In the turn execution loop, instead of buffering to Vec<u8>:
let stream_state = app.stream_state_ref();
let result = cli.run_turn_streaming(&trimmed, |token: &str| {
    // Called by the runtime as each token arrives
    stream_state.push_stream_token(token);
});
app.finish_stream();
```

**File:** `src/tui.rs` — render streaming content in conversation pane:

```rust
fn build_conversation_lines(&self, width: u16) -> Vec<Line<'static>> {
    let mut all_lines = Vec::new();
    
    // Render completed conversation entries (cached)
    for entry in &self.conversation {
        if let Some(cached) = &entry.rendered_cache {
            all_lines.extend(cached.iter().cloned());
        } else {
            let rendered = self.render_entry(entry, width);
            all_lines.extend(rendered.iter().cloned());
        }
    }
    
    // Render in-progress streaming content
    if self.stream_is_active && !self.stream_buffer.is_empty() {
        // Simple rendering for streaming — full markdown render happens on finish
        for line in self.stream_buffer.lines() {
            all_lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(Color::White),
            )));
        }
        // Show cursor at end of stream
        all_lines.push(Line::from(Span::styled(
            "▊",
            Style::default().fg(Color::Cyan),
        )));
    }
    
    all_lines
}
```

### Architecture Note

The streaming callback approach requires the runtime to support streaming. If the runtime doesn't have a streaming callback API, implement it as:
1. Background thread runs the turn
2. Token sender channel sends tokens to TUI thread
3. TUI polls the channel in its 16ms event loop

```rust
// Channel-based streaming if callback API isn't available:
let (tx, rx) = std::sync::mpsc::channel::<String>();

std::thread::spawn(move || {
    // Run turn, send tokens to channel
    cli.run_turn_with_callback(&input, |token| {
        let _ = tx.send(token.to_string());
    });
});

// In TUI event loop:
while let Ok(token) = rx.try_recv() {
    app.push_stream_token(&token);
}
```

### Acceptance Criteria
- [ ] Assistant tokens appear in conversation as they arrive
- [ ] Blinking cursor shows at end of streaming content
- [ ] Markdown rendering happens only after stream completes (not per-token)
- [ ] Streaming doesn't block TUI event loop (UI stays responsive)
- [ ] Fast streams (e.g., Haiku) don't flicker
- [ ] Slow streams (e.g., Opus thinking) show smooth character-by-character display

---

## S2-2: Create ConversationContent enum with render cache

**Priority:** P1 — High  
**Estimate:** 0.25 day  

### Description
Replace flat `ConversationLine` with enum + per-entry render cache. This enables markdown rendering without per-frame re-parsing.

### Implementation

**File:** `src/tui.rs`

```rust
#[derive(Debug, Clone)]
pub enum ConversationContent {
    Plain {
        text: String,
        color: Color,
        bold: bool,
    },
    Markdown {
        source: String,
    },
    CodeDiff {
        diff: String,
    },
}

#[derive(Debug, Clone)]
pub struct ConversationLine {
    pub content: ConversationContent,
    /// Cached rendered output. None = needs rendering.
    /// Stored as Vec<Line> to avoid re-parsing markdown every frame.
    pub rendered_cache: Option<Vec<Line<'static>>>,
}

impl ConversationLine {
    pub fn plain(text: String, color: Color, bold: bool) -> Self {
        Self {
            content: ConversationContent::Plain { text, color, bold },
            rendered_cache: None,
        }
    }
    
    pub fn markdown(source: String) -> Self {
        Self {
            content: ConversationContent::Markdown { source },
            rendered_cache: None,
        }
    }
    
    pub fn diff(diff: String) -> Self {
        Self {
            content: ConversationContent::CodeDiff { diff },
            rendered_cache: None,
        }
    }
}
```

### Acceptance Criteria
- [ ] `ConversationContent` enum compiles
- [ ] `rendered_cache` field exists on `ConversationLine`
- [ ] All existing push methods migrated to use enum
- [ ] Constructor methods (`plain`, `markdown`, `diff`) work

---

## S2-3: Create MarkdownRenderer with lazy syntect init

**Priority:** P1 — High  
**Estimate:** 1 day  

### Description
Create the markdown renderer. **Fix the syntect compilation errors** identified by Opus. Use `once_cell::Lazy` for expensive syntax/theme set loading.

### Dependencies

Add to `Cargo.toml`:
```toml
once_cell = "1"
```

### Implementation

**New file:** `src/markdown.rs`

```rust
use once_cell::sync::Lazy;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

// Lazy-loaded — initialized once on first use (~10-50ms), then cached for process lifetime
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| SyntaxSet::load_defaults_newlines());
static THEME_SET: Lazy<ThemeSet> = Lazy::new(|| ThemeSet::load_defaults());

pub struct MarkdownRenderer {
    code_theme_name: String,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            code_theme_name: "base16-ocean.dark".to_string(),
        }
    }

    pub fn set_code_theme(&mut self, name: &str) {
        self.code_theme_name = name.to_string();
    }

    pub fn render(&self, markdown: &str, width: u16) -> Vec<Line<'static>> {
        let parser = Parser::new(markdown);
        let events: Vec<Event> = parser.collect();
        self.render_events(&events, width)
    }

    fn render_events(&self, events: &[Event], width: u16) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut current_spans: Vec<Span<'static>> = Vec::new();
        let mut style_stack: Vec<Style> = Vec::new();
        let mut in_code_block = false;
        let mut code_block_lang: Option<String> = None;
        let mut code_block_content = String::new();
        let mut list_depth: usize = 0;

        for event in events {
            match event {
                Event::Start(tag) => {
                    match tag {
                        Tag::Heading { .. } => {
                            flush_line(&mut lines, &mut current_spans);
                            style_stack.push(Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD));
                        }
                        Tag::Paragraph => {
                            style_stack.push(Style::default());
                        }
                        Tag::CodeBlock(kind) => {
                            flush_line(&mut lines, &mut current_spans);
                            in_code_block = true;
                            code_block_content.clear();
                            code_block_lang = match kind {
                                pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                                    let lang_str = lang.to_string();
                                    if lang_str.is_empty() { None } else { Some(lang_str) }
                                }
                                _ => None,
                            };
                        }
                        Tag::List(_) => {
                            list_depth += 1;
                        }
                        Tag::Item => {
                            let indent = "  ".repeat(list_depth.saturating_sub(1));
                            current_spans.push(Span::raw(format!("{indent}• ")));
                        }
                        Tag::BlockQuote => {
                            current_spans.push(Span::styled(
                                "│ ",
                                Style::default().fg(Color::DarkGray),
                            ));
                            style_stack.push(Style::default().fg(Color::DarkGray));
                        }
                        Tag::Emphasis => {
                            style_stack.push(Style::default().add_modifier(Modifier::ITALIC));
                        }
                        Tag::Strong => {
                            style_stack.push(Style::default().add_modifier(Modifier::BOLD));
                        }
                        Tag::Link { .. } => {
                            style_stack.push(Style::default()
                                .fg(Color::Blue)
                                .add_modifier(Modifier::UNDERLINED));
                        }
                        _ => {}
                    }
                }
                Event::End(tag_end) => {
                    match tag_end {
                        TagEnd::Heading(_) => {
                            flush_line(&mut lines, &mut current_spans);
                            lines.push(Line::from(""));
                            style_stack.pop();
                        }
                        TagEnd::Paragraph => {
                            flush_line(&mut lines, &mut current_spans);
                            lines.push(Line::from(""));
                            style_stack.pop();
                        }
                        TagEnd::CodeBlock => {
                            let code_lines = self.render_code_block(
                                &code_block_content,
                                code_block_lang.as_deref(),
                            );
                            lines.extend(code_lines);
                            in_code_block = false;
                            code_block_lang = None;
                        }
                        TagEnd::List(_) => {
                            list_depth = list_depth.saturating_sub(1);
                        }
                        TagEnd::Item => {
                            flush_line(&mut lines, &mut current_spans);
                        }
                        TagEnd::BlockQuote => {
                            flush_line(&mut lines, &mut current_spans);
                            style_stack.pop();
                        }
                        TagEnd::Emphasis | TagEnd::Strong | TagEnd::Link => {
                            style_stack.pop();
                        }
                        _ => {}
                    }
                }
                Event::Text(text) => {
                    if in_code_block {
                        code_block_content.push_str(&text);
                    } else {
                        let style = style_stack.last().copied().unwrap_or_default();
                        current_spans.push(Span::styled(text.to_string(), style));
                    }
                }
                Event::Code(code) => {
                    let style = Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::DarkGray);
                    current_spans.push(Span::styled(
                        format!(" {code} "),
                        style,
                    ));
                }
                Event::SoftBreak | Event::HardBreak => {
                    if in_code_block {
                        code_block_content.push('\n');
                    } else {
                        flush_line(&mut lines, &mut current_spans);
                    }
                }
                Event::Rule => {
                    flush_line(&mut lines, &mut current_spans);
                    lines.push(Line::from(Span::styled(
                        "─".repeat(width as usize),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                _ => {}
            }
        }

        flush_line(&mut lines, &mut current_spans);
        lines
    }

    fn render_code_block(&self, code: &str, language: Option<&str>) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();
        let syntax = language
            .and_then(|lang| SYNTAX_SET.find_syntax_by_token(lang))
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

        // FIX: ThemeSet::load_defaults() may not contain our theme name.
        // Use .get() with fallback instead of [] indexing (which panics).
        let theme = THEME_SET.themes
            .get(&self.code_theme_name)
            .or_else(|| THEME_SET.themes.get("base16-ocean.dark"))
            .unwrap_or_else(|| THEME_SET.themes.values().next().expect("no themes available"));

        // Top border
        let lang_label = language.unwrap_or("text");
        lines.push(Line::from(Span::styled(
            format!("╭─ {lang_label} ─"),
            Style::default().fg(Color::DarkGray),
        )));

        // FIX: syntect 5.x HighlightLines::new() returns Result, not direct value.
        let mut highlighter = match syntect::easy::HighlightLines::new(syntax, theme) {
            Ok(h) => h,
            Err(_) => {
                // Fallback: render without highlighting
                for line in code.lines() {
                    lines.push(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
                        Span::raw(line.to_string()),
                    ]));
                }
                lines.push(Line::from(Span::styled("╰─", Style::default().fg(Color::DarkGray))));
                return lines;
            }
        };

        for line in code.lines() {
            match highlighter.highlight_line(line, &SYNTAX_SET) {
                Ok(ranges) => {
                    let spans: Vec<Span<'static>> = ranges
                        .into_iter()
                        .map(|(style, text)| {
                            let fg = style.foreground;
                            Span::styled(
                                text.to_string(),
                                Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b)),
                            )
                        })
                        .collect();
                    let mut line_spans = vec![
                        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
                    ];
                    line_spans.extend(spans);
                    lines.push(Line::from(line_spans));
                }
                Err(_) => {
                    lines.push(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(Color::DarkGray)),
                        Span::raw(line.to_string()),
                    ]));
                }
            }
        }

        lines.push(Line::from(Span::styled(
            "╰─",
            Style::default().fg(Color::DarkGray),
        )));
        lines
    }
}

/// Flush accumulated spans into a new line.
fn flush_line(lines: &mut Vec<Line<'static>>, spans: &mut Vec<Span<'static>>) {
    if !spans.is_empty() {
        lines.push(Line::from(spans.clone()));
        spans.clear();
    }
}

/// Heuristic: does this text look like markdown?
pub fn looks_like_markdown(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let has_header = lines.iter().any(|l| l.starts_with('#'));
    let has_code_block = text.contains("```");
    let has_list = lines.iter().any(|l| l.starts_with("- ") || l.starts_with("* "));
    let has_bold = text.contains("**");
    let has_inline_code = text.matches('`').count() >= 2;  // At least one pair
    let multi_line = lines.len() > 3;

    has_code_block
        || (has_header && multi_line)
        || (has_list && multi_line)
        || (has_bold && has_inline_code)
}
```

### Syntect Fixes Applied

1. **`HighlightLines::new()` returns `Result`** — now handled with `match` and fallback to plain text
2. **Theme name may not exist** — `.get()` with fallback chain instead of `[]` indexing
3. **Lazy initialization** — `SYNTAX_SET` and `THEME_SET` loaded once via `once_cell::Lazy`, not per-call

### Acceptance Criteria
- [ ] `MarkdownRenderer::new()` initializes without panic
- [ ] `render()` returns `Vec<Line>` for basic markdown input
- [ ] Headers, code blocks, inline code, bold, lists, blockquotes, rules render correctly
- [ ] Syntect compilation succeeds (no `Result` type mismatch)
- [ ] Theme fallback works when requested theme doesn't exist
- [ ] `looks_like_markdown()` has reasonable precision

---

## S2-4: Integrate markdown rendering with caching

**Priority:** P1 — High  
**Estimate:** 1 day  

### Description
Wire the markdown renderer into the conversation pane. **Cache rendered output per entry** so we don't re-parse markdown every frame.

### Implementation

**File:** `src/tui.rs`

1. Add renderer to `TuiApp`:
```rust
pub struct TuiApp {
    // ... existing fields
    markdown_renderer: MarkdownRenderer,
}
```

2. Add rendering method that populates cache:
```rust
fn ensure_rendered(&self, entry: &mut ConversationLine, width: u16) {
    if entry.rendered_cache.is_some() {
        return;  // Already cached
    }
    
    let rendered = match &entry.content {
        ConversationContent::Plain { text, color, bold } => {
            let wrapped = wrap_line(text, width as usize);
            let style = if *bold {
                Style::default().fg(*color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(*color)
            };
            wrapped.into_iter()
                .map(|line| Line::from(Span::styled(line, style)))
                .collect()
        }
        ConversationContent::Markdown { source } => {
            self.markdown_renderer.render(source, width)
        }
        ConversationContent::CodeDiff { diff } => {
            render_diff(diff)
        }
    };
    
    // Note: we can't mutate through &self. See approach below.
}
```

3. **Cache invalidation strategy:**
   - Invalidate all caches on terminal resize
   - Invalidate specific entry when it's first rendered
   - Never invalidate during normal scrolling (re-render is cheap with cache hit)

```rust
fn build_conversation_lines(&mut self, width: u16) -> Vec<Line<'static>> {
    let mut all_lines = Vec::new();
    
    for entry in &mut self.conversation {
        if entry.rendered_cache.is_none() {
            let rendered = match &entry.content {
                ConversationContent::Plain { text, color, bold } => {
                    render_plain(text, *color, *bold, width)
                }
                ConversationContent::Markdown { source } => {
                    self.markdown_renderer.render(source, width)
                }
                ConversationContent::CodeDiff { diff } => {
                    render_diff(diff)
                }
            };
            entry.rendered_cache = Some(rendered);
        }
        
        if let Some(cached) = &entry.rendered_cache {
            all_lines.extend(cached.iter().cloned());
        }
    }
    
    all_lines
}
```

4. `push_output()` routes through `looks_like_markdown()`:
```rust
pub fn push_output(&mut self, text: &str, is_error: bool) {
    let clean = strip_ansi(text);
    if is_error {
        self.conversation.push(ConversationLine::plain(
            clean, Color::Red, false,
        ));
    } else if looks_like_markdown(&clean) {
        self.conversation.push(ConversationLine::markdown(clean));
    } else {
        for line in clean.lines() {
            self.conversation.push(ConversationLine::plain(
                line.to_string(), Color::White, false,
            ));
        }
    }
    self.auto_scroll();
}
```

### Acceptance Criteria
- [ ] Markdown content rendered once, cached per entry
- [ ] Cache invalidated on resize
- [ ] `looks_like_markdown()` correctly identifies assistant responses
- [ ] Plain text responses still render correctly
- [ ] Error messages still render in red
- [ ] No per-frame markdown re-parsing (verify with timer/logging)

---

## S2-5: Add CodeDiff renderer

**Priority:** P2 — Medium  
**Estimate:** 0.25 day  

### Description
Render diff content with `+`/`-` coloring.

### Implementation

**File:** `src/markdown.rs`

```rust
pub fn render_diff(diff: &str) -> Vec<Line<'static>> {
    diff.lines().map(|raw_line| {
        let (text, color) = if raw_line.starts_with("+++") || raw_line.starts_with("---") {
            (raw_line.to_string(), Color::White)
        } else if raw_line.starts_with("@@") {
            (raw_line.to_string(), Color::Cyan)
        } else if raw_line.starts_with('+') {
            (raw_line.to_string(), Color::Green)
        } else if raw_line.starts_with('-') {
            (raw_line.to_string(), Color::Red)
        } else {
            (raw_line.to_string(), Color::DarkGray)
        };
        Line::from(Span::styled(text, Style::default().fg(color)))
    }).collect()
}
```

### Acceptance Criteria
- [ ] Additions in green, deletions in red
- [ ] Hunk headers in cyan, file headers in white
- [ ] Context lines in dark gray

---

## S2-6: Write markdown and streaming tests

**Priority:** P1 — High  
**Estimate:** 0.5 day  

### Description
Comprehensive test coverage for markdown rendering, streaming, and caching.

### Implementation

**New file:** `tests/markdown_tests.rs`

```rust
use rusty_claude_cli::markdown::{MarkdownRenderer, looks_like_markdown, render_diff};

// --- Markdown rendering tests ---

#[test]
fn test_plain_paragraph() {
    let r = MarkdownRenderer::new();
    let lines = r.render("Just a paragraph.", 80);
    assert_eq!(lines.len(), 2); // text + blank
}

#[test]
fn test_h1_rendering() {
    let r = MarkdownRenderer::new();
    let lines = r.render("# Hello", 80);
    assert!(lines[0].spans.iter().any(|s| s.style.fg == Some(Color::Cyan)));
}

#[test]
fn test_rust_code_block() {
    let r = MarkdownRenderer::new();
    let lines = r.render("```rust\nfn main() {\n    println!(\"hi\");\n}\n```", 80);
    assert!(lines.len() >= 6); // top + 4 code lines + bottom
}

#[test]
fn test_python_code_block() {
    let r = MarkdownRenderer::new();
    let lines = r.render("```python\ndef hello():\n    pass\n```", 80);
    assert!(lines.len() >= 5);
}

#[test]
fn test_inline_code() {
    let r = MarkdownRenderer::new();
    let lines = r.render("Use `git status` to check", 80);
    assert_eq!(lines.len(), 2);
}

#[test]
fn test_bold_and_italic() {
    let r = MarkdownRenderer::new();
    let lines = r.render("**bold** and *italic*", 80);
    assert_eq!(lines.len(), 2);
}

#[test]
fn test_nested_list() {
    let r = MarkdownRenderer::new();
    let lines = r.render("- item 1\n- item 2\n  - nested", 80);
    assert_eq!(lines.len(), 3);
}

#[test]
fn test_blockquote() {
    let r = MarkdownRenderer::new();
    let lines = r.render("> quoted\n> text", 80);
    assert_eq!(lines.len(), 2);
}

#[test]
fn test_horizontal_rule() {
    let r = MarkdownRenderer::new();
    let lines = r.render("---", 80);
    assert_eq!(lines.len(), 1);
}

#[test]
fn test_mixed_content() {
    let r = MarkdownRenderer::new();
    let md = "# Title\n\nSome text with `code`.\n\n```rust\nfn main() {}\n```\n\n- list item";
    let lines = r.render(md, 80);
    assert!(lines.len() >= 10);
}

#[test]
fn test_empty_input() {
    let r = MarkdownRenderer::new();
    let lines = r.render("", 80);
    assert!(lines.is_empty());
}

#[test]
fn test_link_rendering() {
    let r = MarkdownRenderer::new();
    let lines = r.render("[click here](https://example.com)", 80);
    assert_eq!(lines.len(), 2);
}

// --- looks_like_markdown tests ---

#[test]
fn test_markdown_detection_code_block() {
    assert!(looks_like_markdown("some text\n```rust\ncode\n```\nmore text"));
}

#[test]
fn test_markdown_detection_header() {
    assert!(looks_like_markdown("# Title\n\nParagraph one.\n\nParagraph two."));
}

#[test]
fn test_markdown_detection_list() {
    assert!(looks_like_markdown("Items:\n\n- one\n- two\n- three"));
}

#[test]
fn test_markdown_detection_plain() {
    assert!(!looks_like_markdown("Just a simple response without markdown."));
}

#[test]
fn test_markdown_detection_short() {
    assert!(!looks_like_markdown("Short"));
}

#[test]
fn test_markdown_detection_inline_code_only() {
    assert!(!looks_like_markdown("Use `foo` and `bar`"));
    // Needs bold too for inline-only detection
    assert!(looks_like_markdown("Use `foo` and **bar**"));
}

// --- Diff renderer tests ---

#[test]
fn test_diff_additions_green() {
    let lines = render_diff("+new line");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].spans[0].style.fg, Some(Color::Green));
}

#[test]
fn test_diff_deletions_red() {
    let lines = render_diff("-old line");
    assert_eq!(lines[0].spans[0].style.fg, Some(Color::Red));
}

#[test]
fn test_diff_hunk_header() {
    let lines = render_diff("@@ -1,3 +1,4 @@");
    assert_eq!(lines[0].spans[0].style.fg, Some(Color::Cyan));
}

#[test]
fn test_diff_file_header() {
    let lines = render_diff("--- a/file.rs\n+++ b/file.rs");
    assert_eq!(lines[0].spans[0].style.fg, Some(Color::White));
    assert_eq!(lines[1].spans[0].style.fg, Some(Color::White));
}

// --- Syntect safety tests ---

#[test]
fn test_theme_fallback() {
    let mut r = MarkdownRenderer::new();
    r.set_code_theme("nonexistent_theme_name");
    // Should not panic — falls back gracefully
    let lines = r.render("```rust\nfn main() {}\n```", 80);
    assert!(lines.len() >= 3);
}
```

### Acceptance Criteria
- [ ] All 20+ test cases pass
- [ ] Syntect theme fallback doesn't panic
- [ ] `looks_like_markdown()` has reasonable precision (no false positives on short text)
- [ ] Diff renderer colors correct

---

## Sprint 2 Definition of Done

- [ ] All 6 stories completed
- [ ] Streaming rendering works for assistant responses
- [ ] Markdown rendering with per-entry caching
- [ ] Syntect compilation errors fixed (`Result` handling, theme fallback)
- [ ] Lazy initialization with `once_cell`
- [ ] `cargo test -p rusty-claude-cli` passes
- [ ] `cargo clippy -p rusty-claude-cli` has no new warnings
- [ ] Manual test: send prompt that returns markdown, verify rich rendering appears incrementally
