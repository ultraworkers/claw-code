//! Input bar component.
//!
//! Handles text input, submit, history navigation, and tab completion.
//! Extracted from the TuiApp god-struct.

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::text::{Line, Span};
use ratatui::Frame;
use tui_textarea::TextArea;

use crate::keybindings::{Action, KeyMap, KeyPreset, VimMode};
use crate::theme::TuiTheme;
use crate::tui::component::Component;
use crate::tui::event::TuiEvent;

/// Input bar outcome — what should happen after a key press.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputOutcome {
    /// No action — still editing.
    None,
    /// User submitted text.
    Submit(String),
    /// User cancelled (Ctrl+C on empty).
    Cancel,
    /// User wants to exit (Ctrl+D).
    Exit,
    /// User wants provider swap.
    ProviderSwap,
    /// User wants team toggle.
    TeamToggle,
    /// User wants agent view.
    ToggleAgentView,
}

/// Input bar with history and tab completion.
pub struct InputBar {
    textarea: TextArea<'static>,
    /// Input history (most recent last).
    history: Vec<String>,
    /// Current position in history (None = not navigating).
    history_index: Option<usize>,
    /// Available slash commands for tab completion.
    slash_completions: Vec<String>,
    /// Whether completions popup is visible.
    showing_completions: bool,
    /// Index in the completions list.
    completion_index: usize,
    /// Whether a turn is in progress (disables submit).
    turn_in_progress: bool,
    /// Dirty flag.
    dirty: bool,
}

impl InputBar {
    pub fn new(theme: &TuiTheme) -> Self {
        let mut textarea = TextArea::new(vec![String::new()]);
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.input_border.to_color()))
                .title(" > "),
        );
        textarea.set_style(Style::default().fg(theme.input_fg.to_color()));
        textarea.set_cursor_style(
            Style::default()
                .fg(theme.input_cursor_fg.to_color())
                .bg(theme.input_cursor_bg.to_color())
                .add_modifier(Modifier::BOLD),
        );
        Self {
            textarea,
            history: Vec::new(),
            history_index: None,
            slash_completions: Vec::new(),
            showing_completions: false,
            completion_index: 0,
            turn_in_progress: false,
            dirty: true,
        }
    }

    pub fn set_theme(&mut self, theme: &TuiTheme) {
        self.textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.input_border.to_color()))
                .title(" > "),
        );
        self.textarea.set_style(Style::default().fg(theme.input_fg.to_color()));
        self.textarea.set_cursor_style(
            Style::default()
                .fg(theme.input_cursor_fg.to_color())
                .bg(theme.input_cursor_bg.to_color())
                .add_modifier(Modifier::BOLD),
        );
        self.dirty = true;
    }

    pub fn set_completions(&mut self, completions: Vec<String>) {
        self.slash_completions = completions;
    }

    pub fn set_turn_in_progress(&mut self, in_progress: bool) {
        self.turn_in_progress = in_progress;
        self.dirty = true;
    }

    /// Push text to history.
    pub fn push_history(&mut self, text: &str) {
        if !text.trim().is_empty() {
            self.history.push(text.to_string());
            if self.history.len() > 500 {
                self.history.remove(0);
            }
        }
        self.history_index = None;
    }

    /// Process a key event and return the outcome.
    pub fn process_key(&mut self, key: KeyEvent, keymap: &mut KeyMap) -> InputOutcome {
        // Vim mode transition
        if keymap.preset() == KeyPreset::Vim
            && key.code == crossterm::event::KeyCode::Esc
            && key.modifiers.is_empty()
            && keymap.vim_mode() == VimMode::Insert
        {
            keymap.set_vim_mode(VimMode::Normal);
            self.dirty = true;
            return InputOutcome::None;
        }

        // Tab completion when active
        if self.showing_completions && key.code == crossterm::event::KeyCode::Tab {
            self.handle_tab();
            self.dirty = true;
            return InputOutcome::None;
        }

        let action = keymap.resolve(key);
        match action {
            Some(Action::Submit) => {
                if self.turn_in_progress {
                    return InputOutcome::None;
                }
                let lines = self.textarea.lines();
                let text = lines.join("\n");
                self.textarea.select_all();
                self.textarea.cut();
                if text.trim().is_empty() {
                    return InputOutcome::None;
                }
                self.showing_completions = false;
                self.dirty = true;
                InputOutcome::Submit(text)
            }
            Some(Action::Cancel) => {
                self.showing_completions = false;
                self.textarea.select_all();
                self.textarea.cut();
                self.dirty = true;
                InputOutcome::Cancel
            }
            Some(Action::Exit) => {
                self.dirty = true;
                InputOutcome::Exit
            }
            Some(Action::Newline) => {
                self.textarea.insert_newline();
                self.dirty = true;
                InputOutcome::None
            }
            Some(Action::ProviderSwap) => {
                self.textarea.select_all();
                self.textarea.cut();
                self.dirty = true;
                InputOutcome::ProviderSwap
            }
            Some(Action::TeamToggle) => {
                self.textarea.select_all();
                self.textarea.cut();
                self.dirty = true;
                InputOutcome::TeamToggle
            }
            Some(Action::ToggleAgentView) => {
                self.dirty = true;
                InputOutcome::ToggleAgentView
            }
            _ => {
                // History navigation
                if key.code == crossterm::event::KeyCode::Up && !self.showing_completions {
                    self.history_up();
                    self.dirty = true;
                    return InputOutcome::None;
                }
                if key.code == crossterm::event::KeyCode::Down && !self.showing_completions {
                    self.history_down();
                    self.dirty = true;
                    return InputOutcome::None;
                }
                self.showing_completions = false;
                self.textarea.input(key);
                self.dirty = true;
                InputOutcome::None
            }
        }
    }

    /// Get the current input text.
    pub fn text(&self) -> String {
        self.textarea.lines().join("\n")
    }

    fn handle_tab(&mut self) {
        if !self.showing_completions {
            let current_text: String = self.textarea.lines().join("");
            if current_text.starts_with('/') {
                let prefix = &current_text;
                let matches: Vec<&String> = self.slash_completions.iter()
                    .filter(|c| c.starts_with(prefix))
                    .collect();
                if matches.len() == 1 {
                    self.textarea.select_all();
                    self.textarea.cut();
                    for ch in matches[0].chars() {
                        self.textarea.insert_char(ch);
                    }
                    self.showing_completions = false;
                } else if !matches.is_empty() {
                    self.showing_completions = true;
                    self.completion_index = 0;
                }
            }
        } else {
            self.completion_index = self.completion_index.wrapping_add(1);
        }
    }

    fn history_up(&mut self) {
        if self.history.is_empty() { return; }
        let new_idx = match self.history_index {
            Some(i) => i.saturating_add(1).min(self.history.len() - 1),
            None => self.history.len() - 1,
        };
        self.history_index = Some(new_idx);
        let entry = &self.history[self.history.len() - 1 - new_idx];
        self.textarea = TextArea::new(vec![entry.clone()]);
    }

    fn history_down(&mut self) {
        match self.history_index {
            Some(0) => {
                self.history_index = None;
                self.textarea = TextArea::new(vec![String::new()]);
            }
            Some(i) => {
                let new_idx = i - 1;
                self.history_index = Some(new_idx);
                let entry = &self.history[self.history.len() - 1 - new_idx];
                self.textarea = TextArea::new(vec![entry.clone()]);
            }
            None => {}
        }
    }

    /// Render the completions popup (called after the main input render).
    fn render_completions(&self, area: Rect, frame: &mut Frame, theme: &TuiTheme) {
        if !self.showing_completions { return; }
        let current_text: String = self.textarea.lines().join("");
        let matches: Vec<&String> = self.slash_completions.iter()
            .filter(|c| c.starts_with(current_text.as_str()))
            .collect();
        if matches.is_empty() { return; }

        let items: Vec<ListItem> = matches.iter().enumerate().map(|(i, m)| {
            let style = if i == self.completion_index % matches.len() {
                Style::default().bg(theme.completion_selected_bg.to_color()).fg(theme.completion_selected_fg.to_color())
            } else {
                Style::default().fg(theme.completion_fg.to_color())
            };
            ListItem::new(Line::from(Span::styled(m.as_str(), style)))
        }).collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border.to_color())),
        );
        let popup = Rect {
            x: area.x,
            y: area.y.saturating_sub(matches.len().min(8) as u16 + 2),
            width: area.width.min(40),
            height: (matches.len() as u16 + 2).min(10),
        };
        frame.render_widget(ratatui::widgets::Clear, popup);
        frame.render_widget(list, popup);
    }
}

impl Component for InputBar {
    fn render(&self, area: Rect, frame: &mut Frame, theme: &TuiTheme) {
        frame.render_widget(ratatui::widgets::Clear, area);
        let widget = self.textarea.clone();
        frame.render_widget(&widget, area);
        self.render_completions(area, frame, theme);
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_theme() -> TuiTheme {
        TuiTheme::builtin("default").unwrap()
    }

    #[test]
    fn test_input_bar_new() {
        let theme = test_theme();
        let bar = InputBar::new(&theme);
        assert!(bar.text().is_empty());
        assert!(!bar.turn_in_progress);
    }

    #[test]
    fn test_push_history() {
        let theme = test_theme();
        let mut bar = InputBar::new(&theme);
        bar.push_history("hello");
        bar.push_history("world");
        assert_eq!(bar.history.len(), 2);
    }

    #[test]
    fn test_push_history_ignores_empty() {
        let theme = test_theme();
        let mut bar = InputBar::new(&theme);
        bar.push_history("   ");
        assert_eq!(bar.history.len(), 0);
    }

    #[test]
    fn test_set_turn_in_progress() {
        let theme = test_theme();
        let mut bar = InputBar::new(&theme);
        bar.set_turn_in_progress(true);
        assert!(bar.turn_in_progress);
    }
}
