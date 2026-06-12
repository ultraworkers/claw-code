//! Markdown → ratatui `Line` renderer with syntect syntax highlighting.
//!
//! Syntect assets (`SyntaxSet`, `ThemeSet`) are loaded once via `once_cell::Lazy`.

use once_cell::sync::Lazy;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

/// Loaded once on first access (~10-50ms), cached for process lifetime.
pub static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
pub static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

#[derive(Clone)]
pub struct MarkdownRenderer {
    code_theme_name: String,
    theme: crate::theme::TuiTheme,
}

impl MarkdownRenderer {
    pub fn new(theme: crate::theme::TuiTheme) -> Self {
        let code_theme_name = theme.syntax_theme.clone();
        Self {
            code_theme_name,
            theme,
        }
    }

    pub fn set_code_theme(&mut self, name: &str) {
        self.code_theme_name = name.to_string();
    }

    pub fn set_theme(&mut self, theme: crate::theme::TuiTheme) {
        self.code_theme_name = theme.syntax_theme.clone();
        self.theme = theme;
    }

    /// Render a markdown string into ratatui Lines at the given terminal width.
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
                Event::Start(tag) => match tag {
                    Tag::Heading { .. } => {
                        flush_line(&mut lines, &mut current_spans);
                        style_stack.push(
                            Style::default()
                                .fg(self.theme.conversation_user.to_color())
                                .add_modifier(Modifier::BOLD),
                        );
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
                                let s = lang.to_string();
                                if s.is_empty() {
                                    None
                                } else {
                                    Some(s)
                                }
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
                    Tag::BlockQuote(_) => {
                        current_spans
                            .push(Span::styled("│ ", Style::default().fg(self.theme.conversation_dim.to_color())));
                        style_stack.push(Style::default().fg(self.theme.conversation_dim.to_color()));
                    }
                    Tag::Emphasis => {
                        style_stack.push(Style::default().add_modifier(Modifier::ITALIC));
                    }
                    Tag::Strong => {
                        style_stack.push(Style::default().add_modifier(Modifier::BOLD));
                    }
                    Tag::Link { .. } => {
                        style_stack.push(
                            Style::default()
                                .fg(self.theme.conversation_user.to_color())
                                .add_modifier(Modifier::UNDERLINED),
                        );
                    }
                    _ => {}
                },
                Event::End(tag_end) => match tag_end {
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
                        let code_lines =
                            self.render_code_block(&code_block_content, code_block_lang.as_deref());
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
                    TagEnd::BlockQuote(_) => {
                        flush_line(&mut lines, &mut current_spans);
                        style_stack.pop();
                    }
                    TagEnd::Emphasis | TagEnd::Strong | TagEnd::Link => {
                        style_stack.pop();
                    }
                    _ => {}
                },
                Event::Text(text) => {
                    if in_code_block {
                        code_block_content.push_str(&text);
                    } else {
                        let style = style_stack.last().copied().unwrap_or_default();
                        current_spans.push(Span::styled(text.to_string(), style));
                    }
                }
                Event::Code(code) => {
                    let style = Style::default().fg(self.theme.conversation_system.to_color()).bg(self.theme.code_bg.to_color());
                    current_spans.push(Span::styled(format!(" {code} "), style));
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
                        Style::default().fg(self.theme.conversation_dim.to_color()),
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

        // Theme lookup with fallback chain — never panic on missing theme
        let theme = THEME_SET
            .themes
            .get(&self.code_theme_name)
            .or_else(|| THEME_SET.themes.get("base16-ocean.dark"))
            .or_else(|| THEME_SET.themes.values().next())
            .expect("syntect has no themes");

        let lang_label = language.unwrap_or("text");
        lines.push(Line::from(Span::styled(
            format!("╭─ {lang_label} ─"),
            Style::default().fg(self.theme.code_language_label.to_color()),
        )));

        // syntect 5.x: HighlightLines::new() returns directly (not Result)
        let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);

        for line in code.lines() {
            match highlighter.highlight_line(line, &SYNTAX_SET) {
                Ok(ranges) => {
                    let mut line_spans: Vec<Span<'static>> = vec![Span::styled(
                        "│ ",
                        Style::default().fg(self.theme.code_border.to_color()),
                    )];
                    for (style, text) in ranges {
                        let fg = style.foreground;
                        line_spans.push(Span::styled(
                            text.to_string(),
                            Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b)),
                        ));
                    }
                    lines.push(Line::from(line_spans));
                }
                Err(_) => {
                    lines.push(Line::from(vec![
                        Span::styled("│ ", Style::default().fg(self.theme.code_border.to_color())),
                        Span::raw(line.to_string()),
                    ]));
                }
            }
        }

        lines.push(Line::from(Span::styled(
            "╰─",
            Style::default().fg(self.theme.code_border.to_color()),
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

/// Heuristic: does this text look like it contains markdown formatting?
pub fn looks_like_markdown(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let has_header = lines.iter().any(|l| l.starts_with('#'));
    let has_code_block = text.contains("```");
    let has_list = lines
        .iter()
        .any(|l| l.starts_with("- ") || l.starts_with("* "));
    let has_bold = text.contains("**");
    let has_inline_code = text.matches('`').count() >= 2;
    let multi_line = lines.len() > 3;

    has_code_block || (has_header && multi_line) || (has_list && multi_line) || (has_bold && has_inline_code)
}

/// Render a unified diff with color-coded lines.
pub fn render_diff(diff: &str, theme: &crate::theme::TuiTheme) -> Vec<Line<'static>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_theme() -> crate::theme::TuiTheme {
        crate::theme::TuiTheme::builtin("default").unwrap()
    }

    // --- Markdown rendering ---

    #[test]
    fn test_plain_paragraph() {
        let r = MarkdownRenderer::new(test_theme());
        let lines = r.render("Just a paragraph.", 80);
        assert_eq!(lines.len(), 2); // text + blank
    }

    #[test]
    fn test_h1_rendering() {
        let r = MarkdownRenderer::new(test_theme());
        let lines = r.render("# Hello", 80);
        assert!(lines[0]
            .spans
            .iter()
            .any(|s| s.style.fg == Some(Color::Cyan)));
    }

    #[test]
    fn test_rust_code_block() {
        let r = MarkdownRenderer::new(test_theme());
        let lines = r.render("```rust\nfn main() {\n    println!(\"hi\");\n}\n```", 80);
        // At minimum: top border + at least 1 code line + bottom border
        assert!(lines.len() >= 3);
        // Should contain code border characters
        let text: String = lines.iter().map(|l| {
            l.spans.iter().map(|s| s.content.as_ref()).collect::<String>()
        }).collect();
        assert!(text.contains("╭─") && text.contains("╰─"));
    }

    #[test]
    fn test_python_code_block() {
        let r = MarkdownRenderer::new(test_theme());
        let lines = r.render("```python\ndef hello():\n    pass\n```", 80);
        assert!(lines.len() >= 3);
        let text: String = lines.iter().map(|l| {
            l.spans.iter().map(|s| s.content.as_ref()).collect::<String>()
        }).collect();
        assert!(text.contains("python"));
    }

    #[test]
    fn test_inline_code() {
        let r = MarkdownRenderer::new(test_theme());
        let lines = r.render("Use `git status` to check", 80);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_bold_and_italic() {
        let r = MarkdownRenderer::new(test_theme());
        let lines = r.render("**bold** and *italic*", 80);
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_nested_list() {
        let r = MarkdownRenderer::new(test_theme());
        let lines = r.render("- item 1\n- item 2\n  - nested", 80);
        // At least 2 list items rendered (nested may be part of item 2)
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_blockquote() {
        let r = MarkdownRenderer::new(test_theme());
        let lines = r.render("> quoted\n> text", 80);
        // 2 blockquote lines + possible blank lines
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_horizontal_rule() {
        let r = MarkdownRenderer::new(test_theme());
        let lines = r.render("---", 80);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_empty_input() {
        let r = MarkdownRenderer::new(test_theme());
        let lines = r.render("", 80);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_mixed_content() {
        let r = MarkdownRenderer::new(test_theme());
        let md = "# Title\n\nSome text with `code`.\n\n```rust\nfn main() {}\n```\n\n- list item";
        let lines = r.render(md, 80);
        // Header + blank + text + blank + code block (3+) + blank + list item + blanks
        assert!(lines.len() >= 5);
    }

    #[test]
    fn test_theme_fallback() {
        let mut r = MarkdownRenderer::new(test_theme());
        r.set_code_theme("nonexistent_theme");
        // Should not panic — falls back gracefully
        let lines = r.render("```rust\nfn main() {}\n```", 80);
        assert!(lines.len() >= 3);
    }

    // --- looks_like_markdown ---

    #[test]
    fn test_markdown_detection_code_block() {
        assert!(looks_like_markdown(
            "some text\n```rust\ncode\n```\nmore text"
        ));
    }

    #[test]
    fn test_markdown_detection_header() {
        assert!(looks_like_markdown(
            "# Title\n\nParagraph one.\n\nParagraph two."
        ));
    }

    #[test]
    fn test_markdown_detection_list() {
        assert!(looks_like_markdown(
            "Items:\n\n- one\n- two\n- three"
        ));
    }

    #[test]
    fn test_markdown_detection_plain() {
        assert!(!looks_like_markdown(
            "Just a simple response without any formatting."
        ));
    }

    #[test]
    fn test_markdown_detection_short() {
        assert!(!looks_like_markdown("Short"));
    }

    // --- Diff renderer ---

    #[test]
    fn test_diff_additions_green() {
        let theme = test_theme();
        let lines = render_diff("+new line", &theme);
        assert_eq!(lines[0].spans[0].style.fg, Some(theme.agent_done.to_color()));
    }

    #[test]
    fn test_diff_deletions_red() {
        let theme = test_theme();
        let lines = render_diff("-old line", &theme);
        assert_eq!(lines[0].spans[0].style.fg, Some(theme.conversation_error.to_color()));
    }

    #[test]
    fn test_diff_hunk_header() {
        let theme = test_theme();
        let lines = render_diff("@@ -1,3 +1,4 @@", &theme);
        assert_eq!(lines[0].spans[0].style.fg, Some(theme.conversation_user.to_color()));
    }

    #[test]
    fn test_diff_file_headers() {
        let theme = test_theme();
        let lines = render_diff("--- a/file.rs\n+++ b/file.rs", &theme);
        assert_eq!(lines[0].spans[0].style.fg, Some(theme.conversation_text.to_color()));
        assert_eq!(lines[1].spans[0].style.fg, Some(theme.conversation_text.to_color()));
    }
}
