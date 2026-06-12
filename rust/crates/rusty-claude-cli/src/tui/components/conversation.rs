//! Conversation pane component.
//!
//! Renders the left-pane conversation view with word-wrapping, scroll,
//! markdown content, and diff highlighting. Extracted from the standalone
//! `draw_left_pane` and `build_wrapped_conversation` functions in legacy.rs.

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tui_textarea::TextArea;
use unicode_width::UnicodeWidthChar;

use crate::keybindings::KeyMap;
use crate::theme::TuiTheme;
use crate::tui::component::Component;
use crate::tui::event::TuiEvent;
use crate::tui::legacy::{ConversationContent, ConversationLine};
use crate::tui::markdown::render_diff;
use crate::tui::markdown::ratatui_renderer::MarkdownRenderer;

/// Maximum conversation lines before trimming.
const MAX_CONVERSATION_LINES: usize = 10_000;

/// Conversation pane — the main content area of the TUI.
pub struct ConversationPane {
    /// Conversation entries (owning the content).
    conversation: Vec<ConversationLine>,
    /// Scroll offset (0 = bottom/latest, increases upward).
    scroll: u16,
    /// Whether content has changed since last render.
    dirty: bool,
    /// Markdown renderer for rich content.
    renderer: MarkdownRenderer,
    /// Pre-computed wrapped lines (rebuilt when content or width changes).
    wrapped_cache: Vec<Line<'static>>,
    /// Expand counts for each ConversationLine (for scroll math).
    expand_counts: Vec<usize>,
    /// Width the cache was built for.
    cached_width: u16,
}

impl ConversationPane {
    pub fn new(theme: TuiTheme) -> Self {
        Self {
            conversation: Vec::new(),
            scroll: 0,
            dirty: true,
            renderer: MarkdownRenderer::new(theme),
            wrapped_cache: Vec::new(),
            expand_counts: Vec::new(),
            cached_width: 0,
        }
    }

    pub fn set_theme(&mut self, theme: TuiTheme) {
        self.renderer.set_theme(theme);
        self.dirty = true;
    }

    // -------------------------------------------------------------------
    // Content mutators (update phase)
    // -------------------------------------------------------------------

    pub fn push_banner(&mut self, text: String, color: Color) {
        self.conversation.push(ConversationLine::plain(text, color, true));
        self.auto_scroll();
    }

    pub fn push_user_input(&mut self, text: &str, color: Color) {
        for raw_line in text.lines() {
            self.conversation.push(ConversationLine::plain(raw_line.to_string(), color, true));
        }
        self.auto_scroll();
    }

    pub fn push_system_message(&mut self, text: &str, color: Color) {
        for raw_line in text.lines() {
            self.conversation.push(ConversationLine::plain(raw_line.to_string(), color, false));
        }
        self.auto_scroll();
    }

    pub fn push_output(&mut self, text: &str, is_error: bool, theme: &TuiTheme) {
        if text.is_empty() {
            return;
        }
        let clean = crate::tui_update::strip_ansi(text);
        if is_error {
            let color = theme.conversation_error.to_color();
            for raw_line in clean.lines() {
                self.conversation.push(ConversationLine::plain(raw_line.to_string(), color, false));
            }
        } else if crate::tui::markdown::looks_like_markdown(&clean) {
            self.conversation.push(ConversationLine::markdown(clean));
        } else {
            let color = theme.conversation_text.to_color();
            for raw_line in clean.lines() {
                self.conversation.push(ConversationLine::plain(raw_line.to_string(), color, false));
            }
        }
        self.auto_scroll();
    }

    pub fn push_diff(&mut self, diff: &str) {
        self.conversation.push(ConversationLine::diff(diff.to_string()));
        self.auto_scroll();
    }

    pub fn clear(&mut self) {
        self.conversation.clear();
        self.scroll = 0;
        self.dirty = true;
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll = self.scroll.saturating_add(amount);
        self.dirty = true;
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll = self.scroll.saturating_sub(amount);
        self.dirty = true;
    }

    pub fn scroll_top(&mut self) {
        self.scroll = u16::MAX;
        self.dirty = true;
    }

    pub fn scroll_bottom(&mut self) {
        self.scroll = 0;
        self.dirty = true;
    }

    fn auto_scroll(&mut self) {
        if self.conversation.len() > MAX_CONVERSATION_LINES {
            let drain_count = self.conversation.len() - MAX_CONVERSATION_LINES;
            self.conversation.drain(..drain_count);
        }
        self.scroll = 0;
        self.dirty = true;
    }

    // -------------------------------------------------------------------
    // Render cache
    // -------------------------------------------------------------------

    /// Rebuild the wrapped-line cache if content or width changed.
    fn ensure_cache(&mut self, width: u16, theme: &TuiTheme) {
        if !self.dirty && self.cached_width == width {
            return;
        }
        let content_width = (width as usize).saturating_sub(2);
        let (wrapped, expand_counts) = build_wrapped_conversation(
            &self.conversation,
            content_width,
            &self.renderer,
            theme,
        );
        self.wrapped_cache = wrapped;
        self.expand_counts = expand_counts;
        self.cached_width = width;
        self.dirty = false;
    }
}

impl Component for ConversationPane {
    fn render(&self, area: Rect, frame: &mut Frame, theme: &TuiTheme) {
        let pane_rows = (area.height.saturating_sub(1) as usize).max(1);
        let total_visual = self.wrapped_cache.len();

        let scroll = self.scroll as usize;
        let max_offset = total_visual.saturating_sub(pane_rows);
        let offset = scroll.min(max_offset);
        let start = total_visual.saturating_sub(pane_rows + offset);
        let visible: Vec<Line> = self.wrapped_cache.iter().skip(start).take(pane_rows).cloned().collect();

        let conversation_widget = Paragraph::new(visible).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(theme.border.to_color()))
                .title(Span::styled(
                    " Conversation ",
                    Style::default().fg(theme.border.to_color()),
                )),
        );
        frame.render_widget(conversation_widget, area);

        // Scroll indicator when content overflows
        if total_visual > pane_rows {
            let scroll_label = format!(" {}/{} ", offset, max_offset);
            let scroll_area = Rect {
                x: area.x + area.width.saturating_sub(scroll_label.len() as u16 + 1),
                y: area.y,
                width: scroll_label.len() as u16 + 1,
                height: 1,
            };
            frame.render_widget(
                Paragraph::new(Span::styled(scroll_label, Style::default().fg(theme.conversation_dim.to_color()))),
                scroll_area,
            );
        }
    }

    fn handle_key(&mut self, _key: KeyEvent, _keymap: &KeyMap) -> bool {
        false // Scroll handled by the app
    }

    fn handle_event(&mut self, _event: &TuiEvent) {}

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

// ---------------------------------------------------------------------------
// Word-wrap + build helpers
// ---------------------------------------------------------------------------

/// Word-wrap a single text line into visual lines that fit `width` columns.
fn wrap_line<'a>(text: &str, width: usize, style: Style) -> Vec<Line<'a>> {
    if width == 0 {
        return vec![Line::from(Span::styled(text.to_string(), style))];
    }
    let mut result: Vec<Line<'a>> = Vec::new();
    if text.is_empty() {
        result.push(Line::from(Span::styled(String::new(), style)));
        return result;
    }
    let mut current_line = String::new();
    let mut current_width: usize = 0;
    let mut last_break_byte: usize = 0;

    for ch in text.chars() {
        let cw = ch.width().unwrap_or(0);
        if current_width + cw > width && !current_line.is_empty() {
            if last_break_byte > 0 {
                let keep = current_line[..last_break_byte].trim_end().to_string();
                let rest = current_line[last_break_byte..].to_string();
                result.push(Line::from(Span::styled(keep, style.clone())));
                current_width = rest.chars().map(|c| c.width().unwrap_or(0)).sum();
                current_line = rest;
                last_break_byte = 0;
            } else {
                result.push(Line::from(Span::styled(current_line.clone(), style.clone())));
                current_line.clear();
                current_width = 0;
                last_break_byte = 0;
            }
        }
        current_line.push(ch);
        current_width += cw;
        if ch == ' ' || ch == '-' || ch == '_' || ch == '/' {
            last_break_byte = current_line.len();
        }
    }
    if !current_line.is_empty() {
        result.push(Line::from(Span::styled(current_line, style)));
    }
    if result.is_empty() {
        result.push(Line::from(Span::styled(String::new(), style)));
    }
    result
}

/// Build the full conversation as wrapped visual lines.
fn build_wrapped_conversation<'a>(
    conversation: &[ConversationLine],
    content_width: usize,
    markdown_renderer: &MarkdownRenderer,
    theme: &TuiTheme,
) -> (Vec<Line<'a>>, Vec<usize>) {
    let mut all_lines: Vec<Line<'a>> = Vec::new();
    let mut expand_counts: Vec<usize> = Vec::new();

    for cl in conversation {
        match &cl.content {
            ConversationContent::Plain { text, color, bold } => {
                let mut style = Style::default().fg(*color);
                if *bold { style = style.add_modifier(Modifier::BOLD); }
                let wrapped = wrap_line(text, content_width, style);
                let count = wrapped.len().max(1);
                expand_counts.push(count);
                all_lines.extend(wrapped);
            }
            ConversationContent::Markdown { source } => {
                let rendered = markdown_renderer.render(source, content_width as u16);
                let count = rendered.len().max(1);
                expand_counts.push(count);
                all_lines.extend(rendered.into_iter().map(|l: Line<'static>| {
                    Line::from(l.spans.into_iter().map(|s| {
                        Span::styled(s.content.into_owned(), s.style)
                    }).collect::<Vec<_>>())
                }));
            }
            ConversationContent::CodeDiff { diff } => {
                let rendered = render_diff(diff, theme);
                let count = rendered.len().max(1);
                expand_counts.push(count);
                all_lines.extend(rendered.into_iter().map(|l: Line<'static>| {
                    Line::from(l.spans.into_iter().map(|s| {
                        Span::styled(s.content.into_owned(), s.style)
                    }).collect::<Vec<_>>())
                }));
            }
        }
    }
    (all_lines, expand_counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_theme() -> TuiTheme {
        TuiTheme::builtin("default").unwrap()
    }

    #[test]
    fn test_wrap_empty() {
        let lines = wrap_line("", 80, Style::default());
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_wrap_exact_width() {
        let lines = wrap_line("hello", 5, Style::default());
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_wrap_overflow_with_break() {
        let lines = wrap_line("hello world foo bar", 10, Style::default());
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_wrap_hyphen_break() {
        let lines = wrap_line("well-known-author", 10, Style::default());
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_wrap_no_break_point() {
        let lines = wrap_line("abcdefghijklmnop", 5, Style::default());
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_conversation_pane_push_and_scroll() {
        let theme = test_theme();
        let mut pane = ConversationPane::new(theme.clone());
        pane.push_user_input("Hello", theme.conversation_user.to_color());
        assert_eq!(pane.conversation.len(), 1);
        pane.scroll_up(5);
        assert_eq!(pane.scroll, 5);
        pane.scroll_bottom();
        assert_eq!(pane.scroll, 0);
    }

    #[test]
    fn test_conversation_pane_clear() {
        let theme = test_theme();
        let mut pane = ConversationPane::new(theme);
        pane.push_system_message("test", Color::Yellow);
        assert!(!pane.conversation.is_empty());
        pane.clear();
        assert!(pane.conversation.is_empty());
    }
}
