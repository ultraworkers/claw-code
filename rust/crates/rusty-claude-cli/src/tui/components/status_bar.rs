//! Compact status bar component.
//!
//! Renders a single-line status bar for the focused OpenCode-style layout.
//! Replaces most of the right dashboard with a compact top or bottom bar.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::TuiTheme;
use crate::tui::component::Component;
use crate::tui::event::TuiEvent;
use crate::tui::legacy::SharedDashboardState;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Compact status bar showing key info without taking the whole right pane.
pub struct StatusBar {
    state: SharedDashboardState,
    spinner_frame: usize,
    dirty: bool,
}

impl StatusBar {
    pub fn new(state: SharedDashboardState) -> Self {
        Self {
            state,
            spinner_frame: 0,
            dirty: true,
        }
    }

    pub fn tick_spinner(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
        self.dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }
}

impl Component for StatusBar {
    fn render(&self, area: Rect, frame: &mut Frame, theme: &TuiTheme) {
        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        let mut spans = vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                state.model.clone(),
                Style::default().fg(theme.dashboard_value.to_color()),
            ),
            Span::styled("  ", Style::default()),
            Span::styled(
                state.provider.clone(),
                Style::default().fg(theme.dashboard_key.to_color()),
            ),
        ];

        if state.turn_count > 0 {
            spans.push(Span::styled(
                format!("  turns:{}", state.turn_count),
                Style::default().fg(theme.conversation_dim.to_color()),
            ));
        }

        if state.context_percent > 0.0 {
            spans.push(Span::styled(
                format!("  ctx:{:.0}%", state.context_percent),
                Style::default().fg(theme.dashboard_value.to_color()),
            ));
        }

        if !state.status_message.is_empty() {
            let frame = SPINNER_FRAMES[self.spinner_frame];
            spans.push(Span::styled(
                format!("  {frame} {}", state.status_message),
                Style::default()
                    .fg(theme.spinner.to_color())
                    .add_modifier(Modifier::BOLD),
            ));
        }

        // Right-aligned key hint
        let hint = "Enter send • Shift+Enter newline • Ctrl+D exit";
        spans.push(Span::styled(
            format!("{:>width$}", hint, width = area.width as usize),
            Style::default().fg(theme.key_hint.to_color()),
        ));

        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}
