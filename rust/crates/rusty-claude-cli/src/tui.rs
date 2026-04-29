use std::io;
use std::sync::{Arc, RwLock};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;
use tui_textarea::TextArea;

// ---------------------------------------------------------------------------
// Shared dashboard state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DashboardState {
    pub model: String,
    pub provider: String,
    pub provider_url: String,
    pub session_id: Option<String>,
    pub turn_count: u32,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_creation_tokens: u32,
    pub cost_usd: f64,
    pub context_percent: f64,
    pub context_window: u32,
    pub compaction_count: usize,
    pub lsp_servers: Vec<LspInfo>,
    pub team: Option<TeamInfo>,
    pub working_dir: String,
    pub git_branch: Option<String>,
    pub permission_mode: String,
    pub status_message: String,
}

#[derive(Debug, Clone)]
pub struct LspInfo {
    pub language: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct TeamInfo {
    pub team_id: String,
    pub team_name: String,
    pub total_agents: usize,
    pub completed_agents: usize,
    pub failed_agents: usize,
    pub running_agents: usize,
    pub agents: Vec<AgentInfo>,
}

#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub agent_id: String,
    pub name: String,
    pub subagent_type: Option<String>,
    pub status: String,
}

impl Default for DashboardState {
    fn default() -> Self {
        Self::new()
    }
}

impl DashboardState {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap_or_default();
        let git_branch = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&cwd)
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                } else {
                    None
                }
            });

        Self {
            model: String::new(),
            provider: String::new(),
            provider_url: String::new(),
            session_id: None,
            turn_count: 0,
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: 0,
            cache_creation_tokens: 0,
            cost_usd: 0.0,
            context_percent: 0.0,
            context_window: 0,
            compaction_count: 0,
            lsp_servers: Vec::new(),
            team: None,
            working_dir: cwd.display().to_string(),
            git_branch,
            permission_mode: String::new(),
            status_message: String::new(),
        }
    }
}

pub type SharedDashboardState = Arc<RwLock<DashboardState>>;

// ---------------------------------------------------------------------------
// Banner line for startup banner
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BannerLine {
    pub text: String,
    pub color: Color,
}

// ---------------------------------------------------------------------------
// Conversation line
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ConversationLine {
    pub text: String,
    pub color: Color,
    pub bold: bool,
}

// ---------------------------------------------------------------------------
// TUI App
// ---------------------------------------------------------------------------

pub struct TuiApp {
    dashboard: SharedDashboardState,
    conversation: Vec<ConversationLine>,
    conversation_scroll: u16,
    input: TextArea<'static>,
    should_exit: bool,
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    slash_completions: Vec<String>,
    completion_index: usize,
    showing_completions: bool,
    spinner_frame: usize,
    needs_redraw: bool,
}

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

impl TuiApp {
    pub fn init(state: SharedDashboardState) -> Result<Self, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        crossterm::execute!(stdout, EnterAlternateScreen)?;
        let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
        let terminal = ratatui::Terminal::new(backend)?;

        let mut input = TextArea::new(vec![String::new()]);
        input.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" > "),
        );
        input.set_style(Style::default().fg(Color::White));
        input.set_cursor_style(Style::default().fg(Color::Cyan));

        Ok(Self {
            dashboard: state,
            conversation: Vec::new(),
            conversation_scroll: 0,
            input,
            should_exit: false,
            terminal,
            slash_completions: Vec::new(),
            completion_index: 0,
            showing_completions: false,
            spinner_frame: 0,
            needs_redraw: true,
        })
    }

    /// Suspend TUI: restore normal terminal so stdout works.
    pub fn suspend(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.terminal.show_cursor();
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = crossterm::execute!(
            io::stdout(),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
        );
        let _ = crossterm::execute!(io::stdout(), crossterm::cursor::MoveTo(0, 0));
        Ok(())
    }

    /// Resume TUI after suspend.
    pub fn resume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut out = io::stdout();
        crossterm::execute!(out, EnterAlternateScreen)?;
        let _ = self.terminal.hide_cursor();
        self.needs_redraw = true;
        self.draw_screen()?;
        Ok(())
    }

    /// Fully restore terminal on exit.
    pub fn restore_terminal(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.terminal.show_cursor();
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        Ok(())
    }

    pub fn push_banner(&mut self, lines: Vec<BannerLine>) {
        for bl in lines {
            self.conversation.push(ConversationLine {
                text: bl.text,
                color: bl.color,
                bold: true,
            });
        }
        self.auto_scroll();
    }

    pub fn push_user_input(&mut self, text: &str) {
        self.conversation.push(ConversationLine {
            text: text.to_string(),
            color: Color::Cyan,
            bold: true,
        });
        self.auto_scroll();
    }

    pub fn push_system_message(&mut self, text: &str) {
        self.conversation.push(ConversationLine {
            text: text.to_string(),
            color: Color::Yellow,
            bold: false,
        });
        self.auto_scroll();
    }

    pub fn push_output(&mut self, text: &str, is_error: bool) {
        if text.is_empty() {
            return;
        }
        for raw_line in text.lines() {
            self.conversation.push(ConversationLine {
                text: raw_line.to_string(),
                color: if is_error { Color::Red } else { Color::White },
                bold: false,
            });
        }
        self.auto_scroll();
    }

    pub fn set_slash_completions(&mut self, completions: Vec<String>) {
        self.slash_completions = completions;
    }

    pub fn set_status(&mut self, msg: &str) {
        if let Ok(mut s) = self.dashboard.write() {
            s.status_message = msg.to_string();
        }
        self.needs_redraw = true;
    }

    fn auto_scroll(&mut self) {
        let line_count = self.conversation.len() as u16;
        self.conversation_scroll = line_count.saturating_sub(20);
        self.needs_redraw = true;
    }

    // -----------------------------------------------------------------------
    // Main event loop
    // -----------------------------------------------------------------------

    pub fn read_line(&mut self) -> io::Result<TuiReadOutcome> {
        // Fast 16ms poll
        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                self.needs_redraw = true;
                return self.handle_key(key);
            }
        }

        // Redraw on dirty or every ~80ms for spinner
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
        if self.needs_redraw || self.spinner_frame % 5 == 0 {
            self.draw_screen()?;
            self.needs_redraw = false;
        }

        Ok(TuiReadOutcome::Pending)
    }

    fn draw_screen(&mut self) -> io::Result<()> {
        let dashboard = self.dashboard.clone();
        let conversation = self.conversation.clone();
        let conversation_scroll = self.conversation_scroll;
        let slash_completions = self.slash_completions.clone();
        let completion_index = self.completion_index;
        let showing_completions = self.showing_completions;
        let spinner_frame = self.spinner_frame;
        let input_lines: Vec<String> = self.input.lines().iter().cloned().collect();
        let input_text = input_lines.join("\n");

        self.terminal.draw(|f| {
            draw_frame(
                f,
                &dashboard,
                &conversation,
                conversation_scroll,
                &input_text,
                &input_lines,
                &slash_completions,
                completion_index,
                showing_completions,
                spinner_frame,
            );
        })?;
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> io::Result<TuiReadOutcome> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('d') => {
                    self.should_exit = true;
                    return Ok(TuiReadOutcome::Exit);
                }
                KeyCode::Char('c') => {
                    self.input.select_all();
                    self.input.cut();
                    return Ok(TuiReadOutcome::Cancel);
                }
                KeyCode::Char('p') => {
                    self.input.select_all();
                    self.input.cut();
                    return Ok(TuiReadOutcome::ProviderSwap);
                }
                KeyCode::Char('t') => {
                    self.input.select_all();
                    self.input.cut();
                    return Ok(TuiReadOutcome::TeamToggle);
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.input.insert_newline();
                    return Ok(TuiReadOutcome::Pending);
                }
                let lines = self.input.lines();
                let text = lines.join("\n");
                self.input.select_all();
                self.input.cut();
                if text.trim().is_empty() {
                    return Ok(TuiReadOutcome::Pending);
                }
                Ok(TuiReadOutcome::Submit(text))
            }
            KeyCode::Tab => {
                self.handle_tab();
                Ok(TuiReadOutcome::Pending)
            }
            KeyCode::Esc => {
                self.showing_completions = false;
                Ok(TuiReadOutcome::Cancel)
            }
            KeyCode::PageUp => {
                self.conversation_scroll = self.conversation_scroll.saturating_sub(5);
                Ok(TuiReadOutcome::Pending)
            }
            KeyCode::PageDown => {
                self.conversation_scroll = self.conversation_scroll.saturating_add(5);
                Ok(TuiReadOutcome::Pending)
            }
            _ => {
                self.showing_completions = false;
                self.input.input(key);
                Ok(TuiReadOutcome::Pending)
            }
        }
    }

    fn handle_tab(&mut self) {
        if !self.showing_completions {
            let current_text: String = self.input.lines().join("");
            if current_text.starts_with('/') {
                let prefix = current_text.as_str();
                let matches: Vec<&String> = self
                    .slash_completions
                    .iter()
                    .filter(|c| c.starts_with(prefix))
                    .collect();
                if matches.len() == 1 {
                    self.input.select_all();
                    self.input.cut();
                    for ch in matches[0].chars() {
                        self.input.insert_char(ch);
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
}

// ---------------------------------------------------------------------------
// Read outcome
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiReadOutcome {
    Pending,
    Submit(String),
    Cancel,
    Exit,
    ProviderSwap,
    TeamToggle,
}

// ---------------------------------------------------------------------------
// Standalone draw function
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn draw_frame(
    f: &mut Frame,
    dashboard: &SharedDashboardState,
    conversation: &[ConversationLine],
    conversation_scroll: u16,
    input_text: &str,
    input_lines: &[String],
    slash_completions: &[String],
    completion_index: usize,
    showing_completions: bool,
    spinner_frame: usize,
) {
    let size = f.area();
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(36)])
        .split(size);

    draw_left_pane(
        f,
        main_layout[0],
        conversation,
        conversation_scroll,
        input_text,
        input_lines,
        slash_completions,
        completion_index,
        showing_completions,
    );
    draw_right_pane(f, main_layout[1], dashboard, spinner_frame);
}

#[allow(clippy::too_many_arguments)]
fn draw_left_pane(
    f: &mut Frame,
    area: Rect,
    conversation: &[ConversationLine],
    conversation_scroll: u16,
    input_text: &str,
    input_lines: &[String],
    slash_completions: &[String],
    completion_index: usize,
    showing_completions: bool,
) {
    let left_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(7)])
        .split(area);

    let conv_lines: Vec<Line> = conversation
        .iter()
        .map(|line| {
            let mut style = Style::default().fg(line.color);
            if line.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            Line::from(Span::styled(&line.text, style))
        })
        .collect();

    let conversation_widget = Paragraph::new(conv_lines)
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(
                    " Conversation ",
                    Style::default().fg(Color::DarkGray),
                )),
        )
        .wrap(Wrap { trim: true })
        .scroll((conversation_scroll, 0));

    f.render_widget(conversation_widget, left_layout[0]);

    // Input area with cursor indicator
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " > ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));
    let display_text = format!("{input_text}\u{2588}");
    let input_para = Paragraph::new(display_text).block(input_block);
    f.render_widget(input_para, left_layout[1]);

    // Completion popup
    if showing_completions {
        let current_text: String = input_lines.join("");
        let matches: Vec<&String> = slash_completions
            .iter()
            .filter(|c| c.starts_with(current_text.as_str()))
            .collect();
        if !matches.is_empty() {
            let items: Vec<ListItem> = matches
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let style = if i == completion_index % matches.len() {
                        Style::default().bg(Color::DarkGray).fg(Color::White)
                    } else {
                        Style::default().fg(Color::Gray)
                    };
                    ListItem::new(Line::from(Span::styled(m.as_str(), style)))
                })
                .collect();
            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            let popup_area = Rect {
                x: left_layout[1].x,
                y: left_layout[1].y.saturating_sub(matches.len().min(8) as u16 + 2),
                width: left_layout[1].width.min(40),
                height: (matches.len() as u16 + 2).min(10),
            };
            f.render_widget(list, popup_area);
        }
    }
}

fn draw_right_pane(
    f: &mut Frame,
    area: Rect,
    dashboard: &SharedDashboardState,
    spinner_frame: usize,
) {
    let state = dashboard.read().unwrap_or_else(|e| e.into_inner());
    let mut lines: Vec<Line> = Vec::new();

    // Connection
    lines.push(section("Connection"));
    lines.push(kv("Model", &state.model, Color::White));
    lines.push(kv("Provider", &state.provider, Color::Gray));
    lines.push(kv("URL", &state.provider_url, Color::DarkGray));
    lines.push(kv("Mode", &state.permission_mode, Color::Yellow));
    if let Some(ref branch) = state.git_branch {
        lines.push(kv("Branch", branch, Color::Green));
    }
    lines.push(Line::from(""));

    // Tokens
    lines.push(section("Tokens"));
    lines.push(kv("Turns", &state.turn_count.to_string(), Color::White));
    lines.push(kv("Input", &state.input_tokens.to_string(), Color::White));
    lines.push(kv("Output", &state.output_tokens.to_string(), Color::White));
    lines.push(kv("Cache R", &state.cache_read_tokens.to_string(), Color::Gray));
    lines.push(kv("Cache W", &state.cache_creation_tokens.to_string(), Color::Gray));
    lines.push(kv("Cost", &format!("${:.4}", state.cost_usd), Color::Yellow));
    lines.push(Line::from(""));

    // Context
    let pct = state.context_percent;
    let gauge_color = if pct > 80.0 {
        Color::Red
    } else if pct > 50.0 {
        Color::Yellow
    } else {
        Color::Green
    };
    lines.push(section("Context"));
    lines.push(kv(
        "Used",
        &format!("{:.1}% of {}", pct, state.context_window),
        Color::White,
    ));
    lines.push(Line::from("")); // gauge placeholder
    lines.push(kv(
        "Compactions",
        &state.compaction_count.to_string(),
        Color::Gray,
    ));
    lines.push(Line::from(""));

    // LSP
    if !state.lsp_servers.is_empty() {
        lines.push(section("LSP"));
        for lsp in &state.lsp_servers {
            let status_color = match lsp.status.as_str() {
                "connected" => Color::Green,
                "starting" => Color::Yellow,
                _ => Color::Red,
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", lsp.language), Style::default().fg(Color::White)),
                Span::styled(lsp.status.clone(), Style::default().fg(status_color)),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Team
    if let Some(ref team) = state.team {
        lines.push(section("Team"));
        lines.push(kv("Name", &team.team_name, Color::White));
        lines.push(Line::from(vec![
            Span::styled("  Progress ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}/{} done", team.completed_agents, team.total_agents),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                format!(", {} fail", team.failed_agents),
                Style::default().fg(Color::Red),
            ),
            Span::styled(
                format!(", {} run", team.running_agents),
                Style::default().fg(Color::Cyan),
            ),
        ]));
        for agent in &team.agents {
            let st_color = match agent.status.as_str() {
                "completed" => Color::Green,
                "failed" => Color::Red,
                _ => Color::Cyan,
            };
            lines.push(Line::from(vec![
                Span::styled("  ● ", Style::default().fg(st_color)),
                Span::styled(&agent.name, Style::default().fg(Color::White)),
                Span::styled(
                    format!(" ({})", agent.subagent_type.as_deref().unwrap_or("?")),
                    Style::default().fg(Color::Gray),
                ),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Session
    lines.push(section("Session"));
    lines.push(kv(
        "ID",
        state.session_id.as_deref().unwrap_or("-"),
        Color::Gray,
    ));

    // Spinner
    if !state.status_message.is_empty() {
        let frame = SPINNER_FRAMES[spinner_frame];
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("{frame} {}", state.status_message),
            Style::default().fg(Color::Blue),
        )));
    }

    // Keys
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "─ Keys ─",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  Enter Submit  Shift+Enter Newline",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  ^P Swap  ^T Team  ^C Cancel  ^D Exit",
        Style::default().fg(Color::DarkGray),
    )));

    let dashboard_widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(
                    " Dashboard ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(dashboard_widget, area);

    // Context gauge overlay
    let gauge_y = area.y + 16;
    let gauge_area = Rect {
        x: area.x + 2,
        y: gauge_y,
        width: area.width.saturating_sub(4),
        height: 1,
    };
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(gauge_color).bg(Color::DarkGray))
        .ratio(if pct > 0.0 { (pct / 100.0).min(1.0) } else { 0.0 });
    f.render_widget(gauge, gauge_area);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn section<'a>(label: &str) -> Line<'a> {
    Line::from(Span::styled(
        format!("─ {label} ─"),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ))
}

fn kv<'a>(key: &str, val: &str, val_color: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {key} "), Style::default().fg(Color::DarkGray)),
        Span::styled(val.to_string(), Style::default().fg(val_color)),
    ])
}
