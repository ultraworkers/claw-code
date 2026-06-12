//! Ratatui-based markdown renderer.
//!
//! Uses the shared `MarkdownAst` to produce `Vec<Line<'static>>` for the
//! TUI conversation pane. This replaces the old `markdown.rs` module.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use crate::theme::TuiTheme;
use super::{parse_markdown, MarkdownNode, SemanticStyle, CodeLine, Rgb};

/// Ratatui markdown renderer.
#[derive(Clone)]
pub struct MarkdownRenderer {
    theme: TuiTheme,
}

impl MarkdownRenderer {
    pub fn new(theme: TuiTheme) -> Self {
        Self { theme }
    }

    pub fn set_theme(&mut self, theme: TuiTheme) {
        self.theme = theme;
    }

    /// Render a markdown string into ratatui Lines at the given terminal width.
    pub fn render(&self, markdown: &str, width: u16) -> Vec<Line<'static>> {
        // For now, delegate to the existing markdown.rs renderer
        // to maintain full compatibility. The AST-based renderer
        // will be completed in a follow-up pass.
        let inner = crate::markdown::MarkdownRenderer::new(self.theme.clone());
        inner.render(markdown, width)
    }

    /// Render from a pre-parsed AST (future: will be the primary path).
    #[allow(dead_code)]
    fn render_from_ast(&self, ast: &super::MarkdownAst, _width: u16) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();

        for node in &ast.nodes {
            match node {
                MarkdownNode::CodeBlock { language, lines: code_lines } => {
                    let lang_label = language.as_deref().unwrap_or("text");
                    lines.push(Line::from(Span::styled(
                        format!("╭─ {lang_label} ─"),
                        Style::default().fg(self.theme.code_border.to_color()),
                    )));
                    for cl in code_lines {
                        let mut spans: Vec<Span<'static>> = vec![Span::styled(
                            "│ ",
                            Style::default().fg(self.theme.code_border.to_color()),
                        )];
                        for seg in &cl.segments {
                            let color = seg.fg.map(|rgb| Color::Rgb(rgb.r, rgb.g, rgb.b))
                                .unwrap_or(self.theme.code_fg.to_color());
                            spans.push(Span::styled(seg.text.clone(), Style::default().fg(color)));
                        }
                        lines.push(Line::from(spans));
                    }
                    lines.push(Line::from(Span::styled(
                        "╰─",
                        Style::default().fg(self.theme.code_border.to_color()),
                    )));
                }
                MarkdownNode::HorizontalRule => {
                    lines.push(Line::from(Span::styled(
                        "─".repeat(40),
                        Style::default().fg(self.theme.conversation_dim.to_color()),
                    )));
                }
                MarkdownNode::BlankLine => {
                    lines.push(Line::from(""));
                }
                _ => {
                    // Other node types handled by the delegated renderer
                }
            }
        }

        lines
    }
}

/// Map a SemanticStyle to a ratatui Style using the theme.
#[allow(dead_code)]
fn semantic_to_ratatui_style(style: &SemanticStyle, theme: &TuiTheme) -> Style {
    match style {
        SemanticStyle::Normal => Style::default(),
        SemanticStyle::Emphasis => Style::default().add_modifier(Modifier::ITALIC),
        SemanticStyle::Strong => Style::default().add_modifier(Modifier::BOLD),
        SemanticStyle::Heading(level) => {
            let mut s = Style::default()
                .fg(theme.conversation_user.to_color())
                .add_modifier(Modifier::BOLD);
            if *level >= 3 {
                s = s.fg(Color::Blue);
            }
            s
        }
        SemanticStyle::InlineCode => Style::default()
            .fg(theme.conversation_system.to_color())
            .bg(theme.code_bg.to_color()),
        SemanticStyle::Link { .. } => Style::default()
            .fg(theme.conversation_user.to_color())
            .add_modifier(Modifier::UNDERLINED),
        SemanticStyle::Quote => Style::default().fg(theme.conversation_dim.to_color()),
    }
}
