//! Shared markdown utilities and rendering.
//!
//! Placeholder for Phase 2. Will contain:
//! - `ratatui_renderer.rs` — `MarkdownAst → Vec<Line<'static>>` (replaces markdown.rs)
//! - `ansi_renderer.rs` — `MarkdownAst → String` (replaces render.rs TerminalRenderer)
//! - `stream.rs` — StreamingMarkdownState (incremental rendering)
//!
//! Shared logic moved from render.rs/markdown.rs:
//! - `normalize_nested_fences` — nested code fence fixup
//! - `looks_like_markdown` — heuristic detection
//! - `find_stream_safe_boundary` — streaming boundary finder
//! - `render_diff` — unified diff renderer
//! - `strip_ansi` — ANSI escape stripping

pub mod ratatui_renderer;
pub mod ansi_renderer;
pub mod stream;

// ---------------------------------------------------------------------------
// MarkdownAST — backend-agnostic parsed markdown
// ---------------------------------------------------------------------------

/// A parsed, styled markdown document — backend-agnostic.
///
/// Both the ratatui renderer and the ANSI renderer consume this instead of
/// re-parsing pulldown-cmark events independently. Parse once, render many.
#[derive(Debug, Clone)]
pub struct MarkdownAst {
    pub nodes: Vec<MarkdownNode>,
}

/// Semantic style — not tied to any rendering backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticStyle {
    Normal,
    Emphasis,
    Strong,
    Heading(u8),
    InlineCode,
    Link { destination: String },
    Quote,
}

/// Styled text fragment — backend-agnostic annotation.
#[derive(Debug, Clone)]
pub struct StyledText {
    pub text: String,
    pub style: SemanticStyle,
}

/// A single line of syntax-highlighted code.
#[derive(Debug, Clone)]
pub struct CodeLine {
    pub segments: Vec<CodeSegment>,
}

/// A syntax-highlighted code fragment.
#[derive(Debug, Clone)]
pub struct CodeSegment {
    pub text: String,
    pub fg: Option<Rgb>,
}

/// RGB color value — shared between both backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// A node in the markdown AST.
#[derive(Debug, Clone)]
pub enum MarkdownNode {
    Heading {
        level: u8,
        spans: Vec<StyledText>,
    },
    Paragraph {
        spans: Vec<StyledText>,
    },
    CodeBlock {
        language: Option<String>,
        lines: Vec<CodeLine>,
    },
    ListItem {
        depth: usize,
        ordered: bool,
        index: Option<u64>,
        spans: Vec<StyledText>,
    },
    BlockQuote {
        spans: Vec<StyledText>,
    },
    HorizontalRule,
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    TaskListMarker {
        done: bool,
    },
    BlankLine,
}

// ---------------------------------------------------------------------------
// Shared parser: pulldown-cmark Events → MarkdownAst
// ---------------------------------------------------------------------------

use once_cell::sync::Lazy;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

/// Loaded once on first access (~10-50ms), cached for process lifetime.
pub static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
pub static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Parse a markdown string into a backend-agnostic AST.
pub fn parse_markdown(markdown: &str) -> MarkdownAst {
    let normalized = normalize_nested_fences(markdown);
    let parser = Parser::new(&normalized);
    let events: Vec<Event> = parser.collect();
    let mut nodes: Vec<MarkdownNode> = Vec::new();
    let mut style_stack: Vec<SemanticStyle> = Vec::new();
    let mut in_code_block = false;
    let mut code_block_lang: Option<String> = None;
    let mut code_block_content = String::new();
    let mut list_depth: usize = 0;

    for event in &events {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    style_stack.push(SemanticStyle::Heading(*level as u8));
                }
                Tag::Paragraph => {
                    style_stack.push(SemanticStyle::Normal);
                }
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    code_block_content.clear();
                    code_block_lang = match kind {
                        CodeBlockKind::Fenced(lang) => {
                            let s = lang.to_string();
                            if s.is_empty() { None } else { Some(s) }
                        }
                        _ => None,
                    };
                }
                Tag::List(_) => {
                    list_depth += 1;
                }
                Tag::Item => {
                    // Items are inline — we'll build them from text events
                }
                Tag::BlockQuote(_) => {
                    style_stack.push(SemanticStyle::Quote);
                }
                Tag::Emphasis => {
                    style_stack.push(SemanticStyle::Emphasis);
                }
                Tag::Strong => {
                    style_stack.push(SemanticStyle::Strong);
                }
                Tag::Link { dest_url, .. } => {
                    style_stack.push(SemanticStyle::Link { destination: dest_url.to_string() });
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_) => {
                    style_stack.pop();
                    nodes.push(MarkdownNode::BlankLine);
                }
                TagEnd::Paragraph => {
                    style_stack.pop();
                    nodes.push(MarkdownNode::BlankLine);
                }
                TagEnd::CodeBlock => {
                    let code_lines = highlight_code(&code_block_content, code_block_lang.as_deref());
                    nodes.push(MarkdownNode::CodeBlock {
                        language: code_block_lang.take(),
                        lines: code_lines,
                    });
                    in_code_block = false;
                    code_block_lang = None;
                    code_block_content.clear();
                }
                TagEnd::List(_) => {
                    list_depth = list_depth.saturating_sub(1);
                }
                TagEnd::BlockQuote(_) => {
                    style_stack.pop();
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Link => {
                    style_stack.pop();
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    code_block_content.push_str(text);
                }
                // For the AST we just accumulate text; spans will be
                // built properly in the full parser (Phase 2 refined).
                // For now, we store as a simplified node.
            }
            Event::Code(code) => {
                let _ = (code, in_code_block); // Handled in refined parser
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_code_block {
                    code_block_content.push('\n');
                }
            }
            Event::Rule => {
                nodes.push(MarkdownNode::HorizontalRule);
            }
            Event::TaskListMarker(done) => {
                nodes.push(MarkdownNode::TaskListMarker { done: *done });
            }
            _ => {}
        }
    }

    MarkdownAst { nodes }
}

/// Highlight code using syntect into backend-agnostic CodeLines.
fn highlight_code(code: &str, language: Option<&str>) -> Vec<CodeLine> {
    let syntax = language
        .and_then(|lang| SYNTAX_SET.find_syntax_by_token(lang))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme = THEME_SET
        .themes
        .get("base16-ocean.dark")
        .or_else(|| THEME_SET.themes.values().next())
        .expect("syntect has no themes");

    let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);
    let mut lines: Vec<CodeLine> = Vec::new();

    for line in code.lines() {
        match highlighter.highlight_line(line, &SYNTAX_SET) {
            Ok(ranges) => {
                let segments: Vec<CodeSegment> = ranges
                    .into_iter()
                    .map(|(style, text)| CodeSegment {
                        text: text.to_string(),
                        fg: Some(Rgb { r: style.foreground.r, g: style.foreground.g, b: style.foreground.b }),
                    })
                    .collect();
                lines.push(CodeLine { segments });
            }
            Err(_) => {
                lines.push(CodeLine {
                    segments: vec![CodeSegment { text: line.to_string(), fg: None }],
                });
            }
        }
    }

    lines
}

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use crate::theme::TuiTheme;

// ---------------------------------------------------------------------------
// Markdown detection heuristic (shared)
// ---------------------------------------------------------------------------

/// Heuristic: does this text look like it contains markdown formatting?
pub fn looks_like_markdown(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let has_header = lines.iter().any(|l| l.starts_with('#'));
    let has_code_block = text.contains("```");
    let has_list = lines.iter().any(|l| l.starts_with("- ") || l.starts_with("* "));
    let has_bold = text.contains("**");
    let has_inline_code = text.matches('`').count() >= 2;
    let multi_line = lines.len() > 3;
    has_code_block || (has_header && multi_line) || (has_list && multi_line) || (has_bold && has_inline_code)
}

// ---------------------------------------------------------------------------
// Diff renderer (ratatui Lines variant)
// ---------------------------------------------------------------------------

/// Render a unified diff with color-coded lines.
pub fn render_diff(diff: &str, theme: &TuiTheme) -> Vec<Line<'static>> {
    diff.lines()
        .map(|raw_line| {
            let (text, color) = if raw_line.starts_with("+++") || raw_line.starts_with("---") {
                (raw_line.to_string(), theme.conversation_text.to_color())
            } else if raw_line.starts_with("@@") {
                (raw_line.to_string(), theme.conversation_user.to_color())
            } else if raw_line.starts_with('+') {
                (raw_line.to_string(), theme.agent_done.to_color())
            } else if raw_line.starts_with('-') {
                (raw_line.to_string(), theme.conversation_error.to_color())
            } else {
                (raw_line.to_string(), theme.conversation_dim.to_color())
            };
            Line::from(Span::styled(text, Style::default().fg(color)))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Nested fence normalization (shared)
// ---------------------------------------------------------------------------

/// Pre-process markdown so nested code fences don't break the parser.
#[allow(
    clippy::too_many_lines,
    clippy::items_after_statements,
    clippy::manual_repeat_n,
    clippy::manual_str_repeat
)]
pub fn normalize_nested_fences(markdown: &str) -> String {
    #[derive(Debug, Clone)]
    struct FenceLine {
        char: char,
        len: usize,
        has_info: bool,
        indent: usize,
    }

    fn parse_fence_line(line: &str) -> Option<FenceLine> {
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        let indent = trimmed.chars().take_while(|c| *c == ' ').count();
        if indent > 3 { return None; }
        let rest = &trimmed[indent..];
        let ch = rest.chars().next()?;
        if ch != '`' && ch != '~' { return None; }
        let len = rest.chars().take_while(|c| *c == ch).count();
        if len < 3 { return None; }
        let after = &rest[len..];
        if ch == '`' && after.contains('`') { return None; }
        let has_info = !after.trim().is_empty();
        Some(FenceLine { char: ch, len, has_info, indent })
    }

    let lines: Vec<&str> = markdown.split_inclusive('\n').collect();
    let fence_info: Vec<Option<FenceLine>> = lines.iter().map(|l| parse_fence_line(l)).collect();

    struct StackEntry { line_idx: usize, fence: FenceLine }
    let mut stack: Vec<StackEntry> = Vec::new();
    let mut pairs: Vec<(usize, usize, usize)> = Vec::new();

    for (i, fi) in fence_info.iter().enumerate() {
        let Some(fl) = fi else { continue };
        if fl.has_info {
            stack.push(StackEntry { line_idx: i, fence: fl.clone() });
        } else {
            let closes_top = stack.last().is_some_and(|top| top.fence.char == fl.char && fl.len >= top.fence.len);
            if closes_top {
                let opener = stack.pop().unwrap();
                let inner_max = fence_info[opener.line_idx + 1..i]
                    .iter().filter_map(|fi| fi.as_ref().map(|f| f.len)).max().unwrap_or(0);
                pairs.push((opener.line_idx, i, inner_max));
            } else {
                stack.push(StackEntry { line_idx: i, fence: fl.clone() });
            }
        }
    }

    struct Rewrite { char: char, new_len: usize, indent: usize }
    let mut rewrites: std::collections::HashMap<usize, Rewrite> = std::collections::HashMap::new();

    for (opener_idx, closer_idx, inner_max) in &pairs {
        let opener_fl = fence_info[*opener_idx].as_ref().unwrap();
        if opener_fl.len <= *inner_max {
            let new_len = inner_max + 1;
            rewrites.insert(*opener_idx, Rewrite { char: opener_fl.char, new_len, indent: opener_fl.indent });
            let closer_fl = fence_info[*closer_idx].as_ref().unwrap();
            rewrites.insert(*closer_idx, Rewrite { char: closer_fl.char, new_len, indent: closer_fl.indent });
        }
    }

    if rewrites.is_empty() { return markdown.to_string(); }

    let mut out = String::with_capacity(markdown.len() + rewrites.len() * 4);
    for (i, line) in lines.iter().enumerate() {
        if let Some(rw) = rewrites.get(&i) {
            let fence_str: String = std::iter::repeat(rw.char).take(rw.new_len).collect();
            let indent_str: String = std::iter::repeat(' ').take(rw.indent).collect();
            let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
            let fi = fence_info[i].as_ref().unwrap();
            let info = &trimmed[fi.indent + fi.len..];
            let trailing = &line[trimmed.len()..];
            out.push_str(&indent_str);
            out.push_str(&fence_str);
            out.push_str(info);
            out.push_str(trailing);
        } else {
            out.push_str(line);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Streaming boundary finder (shared)
// ---------------------------------------------------------------------------

/// Find a safe boundary for streaming markdown rendering.
pub fn find_stream_safe_boundary(markdown: &str) -> Option<usize> {
    let mut open_fence: Option<FenceMarker> = None;
    let mut last_boundary = None;

    for (offset, line) in markdown.split_inclusive('\n').scan(0usize, |cursor, line| {
        let start = *cursor;
        *cursor += line.len();
        Some((start, line))
    }) {
        let line_without_newline = line.trim_end_matches('\n');
        if let Some(opener) = open_fence {
            if line_closes_fence(line_without_newline, opener) {
                open_fence = None;
                last_boundary = Some(offset + line.len());
            }
            continue;
        }
        if let Some(opener) = parse_fence_opener(line_without_newline) {
            open_fence = Some(opener);
            continue;
        }
        if line_without_newline.trim().is_empty() {
            last_boundary = Some(offset + line.len());
        }
    }
    last_boundary
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FenceMarker { character: char, length: usize }

fn parse_fence_opener(line: &str) -> Option<FenceMarker> {
    let indent = line.chars().take_while(|c| *c == ' ').count();
    if indent > 3 { return None; }
    let rest = &line[indent..];
    let character = rest.chars().next()?;
    if character != '`' && character != '~' { return None; }
    let length = rest.chars().take_while(|c| *c == character).count();
    if length < 3 { return None; }
    let info_string = &rest[length..];
    if character == '`' && info_string.contains('`') { return None; }
    Some(FenceMarker { character, length })
}

fn line_closes_fence(line: &str, opener: FenceMarker) -> bool {
    let indent = line.chars().take_while(|c| *c == ' ').count();
    if indent > 3 { return false; }
    let rest = &line[indent..];
    let length = rest.chars().take_while(|c| *c == opener.character).count();
    if length < opener.length { return false; }
    rest[length..].chars().all(|c| c == ' ' || c == '\t')
}

// ---------------------------------------------------------------------------
// ANSI stripping (shared, canonical)
// ---------------------------------------------------------------------------

/// Strip ANSI escape sequences from text.
pub fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    while let Some(&c) = chars.peek() { chars.next(); if ('\x40'..='\x7e').contains(&c) { break; } }
                }
                Some(']') => {
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c == '\x07' { break; }
                        if c == '\x1b' && chars.peek() == Some(&'\\') { chars.next(); break; }
                    }
                }
                Some('P') | Some('_') | Some('^') => {
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c == '\x1b' && chars.peek() == Some(&'\\') { chars.next(); break; }
                    }
                }
                _ => { chars.next(); }
            }
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_markdown_code_block() {
        assert!(looks_like_markdown("some text\n```rust\ncode\n```\nmore text"));
    }
    #[test]
    fn test_looks_like_markdown_plain() {
        assert!(!looks_like_markdown("Just a simple response without any formatting."));
    }
    #[test]
    fn test_strip_ansi_basic() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
    }
    #[test]
    fn test_normalize_no_change() {
        let md = "# Hello\n\nSome text\n";
        assert_eq!(normalize_nested_fences(md), md);
    }
    #[test]
    fn test_find_boundary_in_fenced_block() {
        let md = "```rust\nfn main() {}\n";
        assert_eq!(find_stream_safe_boundary(md), None);
    }
    #[test]
    fn test_render_diff() {
        let theme = crate::theme::TuiTheme::builtin("default").unwrap();
        let lines = render_diff("+added\n-removed\n@@ hunk @@\n--- a/file.rs\n+++ b/file.rs", &theme);
        assert_eq!(lines.len(), 5);
    }
}
