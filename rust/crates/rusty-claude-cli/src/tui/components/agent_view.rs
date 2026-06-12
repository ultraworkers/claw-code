//! Agent view overlay component.
//!
//! Wraps the existing `AgentView` state into a Component + Overlay.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::keybindings::KeyMap;
use crate::theme::TuiTheme;
use crate::tui::component::{Component, Overlay};
use crate::tui::event::TuiEvent;
use crate::agent_view::{AgentSession, AgentStatus, AgentView, FilterState, SortField};

/// Agent view overlay — wraps the existing AgentView.
pub struct AgentViewOverlay {
    view: AgentView,
    dirty: bool,
}

impl AgentViewOverlay {
    pub fn new() -> Self {
        Self {
            view: AgentView::new(),
            dirty: false,
        }
    }

    /// Access the underlying AgentView for session updates.
    pub fn view(&mut self) -> &mut AgentView {
        &mut self.view
    }
}

impl Component for AgentViewOverlay {
    fn render(&self, area: Rect, frame: &mut Frame, theme: &TuiTheme) {
        if !self.view.active { return; }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(6),
            ])
            .split(area);

        // Header
        let filter_label = format!("Filter: {:?}", self.view.filter);
        let sort_label = format!("Sort: {:?}", self.view.sort_by);
        let header = Paragraph::new(Line::from(vec![
            Span::styled("  Agent View  ", Style::default().fg(theme.dashboard_header.to_color()).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  [{filter_label}]  [{sort_label}]  Tab:filter  S:sort  Esc:close"), Style::default().fg(theme.key_hint.to_color())),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border_active.to_color())));
        frame.render_widget(header, chunks[0]);

        // Session list
        let sessions: Vec<&AgentSession> = self.view.filtered_sessions();
        let selected = self.view.selected;
        let items: Vec<ListItem> = sessions.iter().enumerate().map(|(i, s)| {
            let status_color = match s.status {
                AgentStatus::Running => theme.agent_running.to_color(),
                AgentStatus::WaitingForInput => theme.agent_waiting.to_color(),
                AgentStatus::Done => theme.agent_done.to_color(),
                AgentStatus::Failed => theme.agent_failed.to_color(),
                AgentStatus::Cancelled => theme.agent_cancelled.to_color(),
            };
            let elapsed = s.started_at.elapsed().as_secs();
            let elapsed_str = if elapsed < 60 { format!("{elapsed}s") } else { format!("{}m{}s", elapsed / 60, elapsed % 60) };
            let line = Line::from(vec![
                Span::styled(format!(" {} ", s.status.icon()), Style::default().fg(status_color)),
                Span::styled(format!("{:<20}", s.name), Style::default().fg(theme.conversation_text.to_color())),
                Span::styled(format!("{:<16}", s.model), Style::default().fg(theme.dashboard_key.to_color())),
                Span::styled(format!("{} turns  ", s.turn_count), Style::default().fg(theme.dashboard_value.to_color())),
                Span::styled(elapsed_str, Style::default().fg(theme.key_hint.to_color())),
            ]);
            let style = if i == selected { Style::default().bg(theme.completion_selected_bg.to_color()) } else { Style::default() };
            ListItem::new(line).style(style)
        }).collect();

        let list = List::new(items).block(
            Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border.to_color())).title(" Sessions "),
        );
        frame.render_widget(list, chunks[1]);

        // Detail
        if let Some(session) = sessions.get(selected) {
            let detail = Paragraph::new(vec![
                Line::from(vec![Span::styled("  ID: ", Style::default().fg(theme.dashboard_key.to_color())), Span::styled(&session.id, Style::default().fg(theme.dashboard_value.to_color()))]),
                Line::from(vec![Span::styled("  Task: ", Style::default().fg(theme.dashboard_key.to_color())), Span::styled(session.task_subject.as_deref().unwrap_or("(none)"), Style::default().fg(theme.dashboard_value.to_color()))]),
                Line::from(vec![Span::styled("  Dir: ", Style::default().fg(theme.dashboard_key.to_color())), Span::styled(&session.working_dir, Style::default().fg(theme.dashboard_value.to_color()))]),
            ])
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border.to_color())).title(" Details "))
            .wrap(Wrap { trim: false });
            frame.render_widget(detail, chunks[2]);
        }
    }

    fn handle_key(&mut self, key: KeyEvent, _keymap: &KeyMap) -> bool {
        if !self.view.active { return false; }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { self.view.close(); self.dirty = true; true }
            KeyCode::Tab => { self.view.cycle_filter(); self.dirty = true; true }
            KeyCode::Char('s') => { self.view.cycle_sort(); self.dirty = true; true }
            KeyCode::Down | KeyCode::Char('j') => { self.view.select_next(); self.dirty = true; true }
            KeyCode::Up | KeyCode::Char('k') => { self.view.select_prev(); self.dirty = true; true }
            _ => true, // Consume all keys when active
        }
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl Overlay for AgentViewOverlay {
    fn is_active(&self) -> bool {
        self.view.active
    }

    fn open(&mut self) {
        self.view.open();
        self.dirty = true;
    }

    fn close(&mut self) {
        self.view.close();
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_open_close() {
        let mut av = AgentViewOverlay::new();
        assert!(!av.is_active());
        av.open();
        assert!(av.is_active());
        av.close();
        assert!(!av.is_active());
    }
}
