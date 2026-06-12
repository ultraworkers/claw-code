//! Command palette overlay component.
//!
//! Wraps the existing `CommandPalette` state into a Component + Overlay.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::keybindings::{Action, KeyMap};
use crate::theme::TuiTheme;
use crate::tui::component::{Component, Overlay};
use crate::tui::event::TuiEvent;
use crate::command_palette::CommandPalette;

/// Command palette overlay — wraps the existing CommandPalette.
pub struct CommandPaletteOverlay {
    palette: CommandPalette,
    dirty: bool,
}

impl CommandPaletteOverlay {
    pub fn new() -> Self {
        Self {
            palette: CommandPalette::new(),
            dirty: false,
        }
    }

    pub fn selected_action(&self) -> Option<Action> {
        self.palette.selected_action()
    }
}

impl Component for CommandPaletteOverlay {
    fn render(&self, area: Rect, frame: &mut Frame, theme: &TuiTheme) {
        if !self.palette.active { return; }

        let popup_w = (area.width * 60 / 100).min(60);
        let popup_h = (area.height * 50 / 100).min(20);
        let popup = Rect::new(
            (area.width.saturating_sub(popup_w)) / 2,
            (area.height.saturating_sub(popup_h)) / 2,
            popup_w,
            popup_h,
        );

        frame.render_widget(ratatui::widgets::Clear, popup);

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(vec![
            Span::styled("🔍 ", Style::default().fg(theme.key_hint.to_color())),
            Span::styled(self.palette.query.clone(), Style::default().fg(theme.input_fg.to_color())),
            Span::styled("█", Style::default().fg(theme.input_cursor_bg.to_color())),
        ]));
        lines.push(Line::from(""));

        for (i, &idx) in self.palette.filtered.iter().enumerate() {
            let entry = &self.palette.entries[idx];
            let is_sel = i == self.palette.selected;
            let (fg, bg) = if is_sel {
                (theme.completion_selected_fg.to_color(), theme.completion_selected_bg.to_color())
            } else {
                (theme.completion_fg.to_color(), ratatui::style::Color::Reset)
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", entry.label), Style::default().fg(fg).bg(bg)),
                Span::styled(format!("  {}  ", entry.description), Style::default().fg(theme.key_hint.to_color())),
                Span::styled(&entry.key_hint, Style::default().fg(theme.key_hint.to_color())),
            ]));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_active.to_color()))
            .title(" Command Palette ");

        frame.render_widget(Paragraph::new(lines).block(block), popup);
    }

    fn handle_key(&mut self, key: KeyEvent, _keymap: &KeyMap) -> bool {
        if !self.palette.active { return false; }
        match key.code {
            KeyCode::Esc => { self.palette.close(); self.dirty = true; true }
            KeyCode::Enter => { self.dirty = true; true }
            KeyCode::Up => { self.palette.select_prev(); self.dirty = true; true }
            KeyCode::Down => { self.palette.select_next(); self.dirty = true; true }
            KeyCode::Backspace => { self.palette.backspace(); self.dirty = true; true }
            KeyCode::Char(c) => { self.palette.input(c); self.dirty = true; true }
            _ => true, // Consume all keys when active
        }
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl Overlay for CommandPaletteOverlay {
    fn is_active(&self) -> bool {
        self.palette.active
    }

    fn open(&mut self) {
        self.palette.open();
        self.dirty = true;
    }

    fn close(&mut self) {
        self.palette.close();
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_open_close() {
        let mut cp = CommandPaletteOverlay::new();
        assert!(!cp.is_active());
        cp.open();
        assert!(cp.is_active());
        cp.close();
        assert!(!cp.is_active());
    }
}
