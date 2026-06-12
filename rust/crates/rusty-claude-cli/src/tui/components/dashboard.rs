//! Dashboard component (right pane).
//!
//! Renders model info, token counts, context gauge, LSP status, team info,
//! and key hints. Extracted from the standalone `draw_right_pane` function.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Wrap};
use ratatui::Frame;

use crate::theme::TuiTheme;
use crate::tui::component::Component;
use crate::tui::event::TuiEvent;
use crate::tui::legacy::{DashboardState, SharedDashboardState};

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const KV_KEY_WIDTH: usize = 12;

/// Dashboard component — the right-side info panel.
pub struct Dashboard {
    state: SharedDashboardState,
    spinner_frame: usize,
    dirty: bool,
}

impl Dashboard {
    pub fn new(state: SharedDashboardState) -> Self {
        Self { state, spinner_frame: 0, dirty: true }
    }

    pub fn tick_spinner(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
        self.dirty = true;
    }

    pub fn set_status(&mut self, msg: &str) {
        if let Ok(mut s) = self.state.write() {
            s.status_message = msg.to_string();
        }
        self.dirty = true;
    }
}

impl Component for Dashboard {
    fn render(&self, area: Rect, frame: &mut Frame, theme: &TuiTheme) {
        let state = self.state.read().unwrap_or_else(|e| e.into_inner());
        let mut lines: Vec<Line> = Vec::new();
        #[allow(unused_assignments)]
        let mut gauge_row: Option<usize> = None;

        // Connection
        lines.push(section("Connection", theme));
        lines.push(kv("Model", &state.model, theme.dashboard_value.to_color(), theme));
        lines.push(kv("Provider", &state.provider, theme.dashboard_key.to_color(), theme));
        lines.push(kv("URL", &state.provider_url, theme.conversation_dim.to_color(), theme));
        lines.push(kv("Mode", &state.permission_mode, theme.conversation_system.to_color(), theme));
        if let Some(ref branch) = state.git_branch {
            lines.push(kv("Branch", branch, theme.agent_done.to_color(), theme));
        }
        lines.push(Line::from(""));

        // Tokens
        lines.push(section("Tokens", theme));
        lines.push(kv("Turns", &state.turn_count.to_string(), theme.dashboard_value.to_color(), theme));
        lines.push(kv("Input", &state.input_tokens.to_string(), theme.dashboard_value.to_color(), theme));
        lines.push(kv("Output", &state.output_tokens.to_string(), theme.dashboard_value.to_color(), theme));
        lines.push(kv("Cache R", &state.cache_read_tokens.to_string(), theme.dashboard_key.to_color(), theme));
        lines.push(kv("Cache W", &state.cache_creation_tokens.to_string(), theme.dashboard_key.to_color(), theme));
        lines.push(kv("Cost", &format!("${:.4}", state.cost_usd), theme.conversation_system.to_color(), theme));
        lines.push(Line::from(""));

        // Context
        let pct = state.context_percent;
        let _gauge_color = if pct > 80.0 { theme.gauge_fill_red.to_color() }
            else if pct > 50.0 { theme.gauge_fill_yellow.to_color() }
            else { theme.gauge_fill_green.to_color() };
        lines.push(section("Context", theme));
        lines.push(kv("Used", &format!("{:.1}% of {}", pct, state.context_window), theme.dashboard_value.to_color(), theme));
        gauge_row = Some(lines.len());
        lines.push(Line::from(""));
        lines.push(kv("Compactions", &state.compaction_count.to_string(), theme.dashboard_key.to_color(), theme));
        lines.push(Line::from(""));

        // LSP
        if !state.lsp_servers.is_empty() {
            lines.push(section("LSP", theme));
            for lsp in &state.lsp_servers {
                let c = match lsp.status.as_str() {
                    "connected" => theme.agent_done.to_color(),
                    "starting" => theme.agent_waiting.to_color(),
                    _ => theme.agent_failed.to_color(),
                };
                lines.push(kv(&lsp.language, &lsp.status, c, theme));
            }
            lines.push(Line::from(""));
        }

        // Team
        if let Some(ref team) = state.team {
            lines.push(section("Team", theme));
            lines.push(kv("Name", &team.team_name, theme.dashboard_value.to_color(), theme));
            let progress = format!("{}/{} done, {} fail, {} run",
                team.completed_agents, team.total_agents, team.failed_agents, team.running_agents);
            lines.push(kv("Status", &progress, theme.agent_done.to_color(), theme));
            for agent in &team.agents {
                let c = match agent.status.as_str() {
                    "completed" => theme.agent_done.to_color(),
                    "failed" => theme.agent_failed.to_color(),
                    _ => theme.agent_running.to_color(),
                };
                let label = format!("● {}", agent.name);
                let detail = format!("({})", agent.subagent_type.as_deref().unwrap_or("?"));
                lines.push(Line::from(vec![
                    Span::styled(format!("  {:<KV_KEY_WIDTH$}", label), Style::default().fg(c)),
                    Span::styled(detail, Style::default().fg(theme.dashboard_key.to_color())),
                ]));
            }
            lines.push(Line::from(""));
        }

        // Session
        lines.push(section("Session", theme));
        lines.push(kv("ID", state.session_id.as_deref().unwrap_or("-"), theme.dashboard_key.to_color(), theme));

        // Status / spinner
        if !state.status_message.is_empty() {
            let frame = SPINNER_FRAMES[self.spinner_frame];
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("{frame} {}", state.status_message),
                Style::default().fg(theme.spinner.to_color()),
            )));
        }

        // Key hints
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("─ Keys ─", Style::default().fg(theme.key_hint.to_color()))));
        lines.push(Line::from(Span::styled("  Enter Submit  Shift+Enter ↵", Style::default().fg(theme.key_hint.to_color()))));
        lines.push(Line::from(Span::styled("  ^P Swap  ^T Team  ^C ⊘  ^D Exit", Style::default().fg(theme.key_hint.to_color()))));

        let widget = Paragraph::new(lines)
            .block(Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(theme.border.to_color()))
                .title(Span::styled(" Dashboard ",
                    Style::default().fg(theme.dashboard_header.to_color()).add_modifier(Modifier::BOLD))))
            .wrap(Wrap { trim: false });
        frame.render_widget(widget, area);

        // Context gauge overlay
        if let Some(row) = gauge_row {
            let gauge_area = Rect {
                x: area.x + 2,
                y: area.y + 1 + row as u16,
                width: area.width.saturating_sub(4),
                height: 1,
            };
            let gauge_fill = if pct > 80.0 { theme.gauge_fill_red.to_color() }
                else if pct > 50.0 { theme.gauge_fill_yellow.to_color() }
                else { theme.gauge_fill_green.to_color() };
            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(gauge_fill).bg(theme.gauge_bg.to_color()))
                .ratio(if pct > 0.0 { (pct / 100.0).min(1.0) } else { 0.0 });
            frame.render_widget(gauge, gauge_area);
        }
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn section<'a>(label: &str, theme: &TuiTheme) -> Line<'a> {
    Line::from(Span::styled(
        format!("─ {label} ─"),
        Style::default().fg(theme.dashboard_header.to_color()).add_modifier(Modifier::BOLD),
    ))
}

fn kv<'a>(key: &str, val: &str, val_color: ratatui::style::Color, theme: &TuiTheme) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {:<KV_KEY_WIDTH$}", key), Style::default().fg(theme.dashboard_key.to_color())),
        Span::styled(val.to_string(), Style::default().fg(val_color)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};

    fn test_theme() -> TuiTheme {
        TuiTheme::builtin("default").unwrap()
    }

    #[test]
    fn test_dashboard_new() {
        let state = Arc::new(RwLock::new(DashboardState::new()));
        let db = Dashboard::new(state);
        assert!(db.dirty);
    }

    #[test]
    fn test_dashboard_set_status() {
        let state = Arc::new(RwLock::new(DashboardState::new()));
        let mut db = Dashboard::new(state);
        db.set_status("Thinking...");
        let s = db.state.read().unwrap();
        assert_eq!(s.status_message, "Thinking...");
    }
}
