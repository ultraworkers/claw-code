use std::io;
use std::io::Write;
use std::sync::{Arc, RwLock};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
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
// Banner / Conversation wrappers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BannerLine {
    pub text: String,
    pub color: Color,
}

/// Content variant — determines how a conversation entry is rendered.
#[derive(Debug, Clone)]
pub enum ConversationContent {
    Plain {
        text: String,
        color: Color,
        bold: bool,
    },
    Markdown {
        source: String,
    },
    CodeDiff {
        diff: String,
    },
}

#[derive(Debug, Clone)]
pub struct ConversationLine {
    pub content: ConversationContent,
}

impl ConversationLine {
    pub fn plain(text: String, color: Color, bold: bool) -> Self {
        Self {
            content: ConversationContent::Plain { text, color, bold },
        }
    }

    pub fn markdown(source: String) -> Self {
        Self {
            content: ConversationContent::Markdown { source },
        }
    }

    pub fn diff(diff: String) -> Self {
        Self {
            content: ConversationContent::CodeDiff { diff },
        }
    }
}

// ---------------------------------------------------------------------------
// TUI App
// ---------------------------------------------------------------------------

pub struct TuiApp {
    dashboard: SharedDashboardState,
    pub conversation: Vec<ConversationLine>,
    conversation_scroll: u16,
    input: TextArea<'static>,
    should_exit: bool,
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    slash_completions: Vec<String>,
    completion_index: usize,
    showing_completions: bool,
    spinner_frame: usize,
    needs_redraw: bool,
    pub markdown_renderer: crate::markdown::MarkdownRenderer,
    pub theme: crate::theme::TuiTheme,
}

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const MAX_CONVERSATION_LINES: usize = 10_000;

/// Key-column width for dashboard `kv()` rows.  Values start at this
/// column so "Model", "Compactions" etc. all line up.
const KV_KEY_WIDTH: usize = 12;

impl TuiApp {
    /// Enter alternate screen, enable raw mode, create Terminal.
    pub fn init(state: SharedDashboardState) -> Result<Self, Box<dyn std::error::Error>> {
        // Step 1 — swap to alternate screen BEFORE anything else
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        )?;
        enable_raw_mode()?;

        let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
        let mut terminal = ratatui::Terminal::new(backend)?;
        terminal.hide_cursor()?;

        let theme = crate::theme::TuiTheme::builtin("default").unwrap();
        let mut input = TextArea::new(vec![String::new()]);
        input.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.input_border.to_color()))
                .title(" > "),
        );
        input.set_style(Style::default().fg(theme.input_fg.to_color()));
        input.set_cursor_style(
            Style::default()
                .fg(theme.input_cursor_fg.to_color())
                .bg(theme.input_cursor_bg.to_color())
                .add_modifier(Modifier::BOLD),
        );

        let mut me = Self {
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
            markdown_renderer: crate::markdown::MarkdownRenderer::new(),
            theme: crate::theme::TuiTheme::builtin("default").unwrap(),
            needs_redraw: true,
        };
        me.draw_screen()?;
        Ok(me)
    }

    /// Suspend TUI for a blocking stdout operation.
    ///
    /// We stay in the alternate screen the ENTIRE session.  Instead of leaving
    /// it (which causes terminal-dependent buffer swap failures), we simply
    /// disable raw mode so stdin echoes normally, show the cursor for
    /// interactive prompts, and clear the screen so stdout output lands on a
    /// blank canvas.  After the operation completes, `resume()` clears the
    /// screen again (wiping stdout debris) and redraws the full TUI frame.
    pub fn suspend(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.terminal.show_cursor();
        disable_raw_mode()?;
        let _ = crossterm::execute!(
            io::stdout(),
            Clear(ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        );
        let _ = io::stdout().flush();
        Ok(())
    }

    /// Resume TUI after suspend.
    pub fn resume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        // Wipe any stdout debris that may have been written while suspended
        let _ = crossterm::execute!(
            io::stdout(),
            Clear(ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        );
        let _ = io::stdout().flush();
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        self.needs_redraw = true;
        self.draw_screen()?;
        Ok(())
    }

    /// Fully restore terminal on exit.
    pub fn restore_terminal(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.terminal.show_cursor();
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(
            io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::MoveTo(0, 0),
            crossterm::cursor::Show
        );
        let _ = io::stdout().flush();
        Ok(())
    }

    /// Handle terminal resize — force full re-render at new dimensions.
    pub fn mark_resize(&mut self) {
        // ratatui Terminal picks up new size on next draw via f.area().
        // We just need to force a redraw so word-wrapping recalculates.
        self.needs_redraw = true;
    }

    /// Short helper to convert theme ColorDef to ratatui Color.
    pub fn tc(&self, c: &crate::theme::ColorDef) -> Color {
        c.to_color()
    }

    /// Set a new theme and trigger redraw.
    pub fn set_theme(&mut self, theme: crate::theme::TuiTheme) {
        self.markdown_renderer.set_code_theme(&theme.syntax_theme);
        self.theme = theme;
        self.needs_redraw = true;
    }

    // -------------------------------------------------------------------
    // Conversation helpers
    // -------------------------------------------------------------------

    pub fn push_banner(&mut self, lines: &[BannerLine]) {
        for bl in lines {
            self.conversation
                .push(ConversationLine::plain(bl.text.clone(), bl.color, true));
        }
        self.auto_scroll();
    }

    pub fn push_user_input(&mut self, text: &str) {
        for raw_line in text.lines() {
            self.conversation
                .push(ConversationLine::plain(raw_line.to_string(), Color::Cyan, true));
        }
        self.auto_scroll();
    }

    pub fn push_system_message(&mut self, text: &str) {
        for raw_line in text.lines() {
            self.conversation
                .push(ConversationLine::plain(raw_line.to_string(), Color::Yellow, false));
        }
        self.auto_scroll();
    }

    pub fn push_output(&mut self, text: &str, is_error: bool) {
        if text.is_empty() {
            return;
        }
        let clean = crate::tui_update::strip_ansi(text);
        if is_error {
            for raw_line in clean.lines() {
                self.conversation
                    .push(ConversationLine::plain(raw_line.to_string(), Color::Red, false));
            }
        } else if crate::markdown::looks_like_markdown(&clean) {
            self.conversation.push(ConversationLine::markdown(clean));
        } else {
            for raw_line in clean.lines() {
                self.conversation
                    .push(ConversationLine::plain(raw_line.to_string(), Color::White, false));
            }
        }
        self.auto_scroll();
    }

    pub fn push_diff(&mut self, diff: &str) {
        self.conversation
            .push(ConversationLine::diff(diff.to_string()));
        self.auto_scroll();
        self.needs_redraw = true;
    }

    /// Force a full TUI clear + redraw.  With Architecture C (buffered
    /// output) this is no longer needed to clean up stdout debris, but
    /// kept as a safety net for edge cases.
    pub fn redraw_after_turn(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal.clear()?;
        self.needs_redraw = true;
        self.draw_screen()?;
        Ok(())
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
        // Trim conversation to prevent unbounded memory growth
        if self.conversation.len() > MAX_CONVERSATION_LINES {
            let drain_count = self.conversation.len() - MAX_CONVERSATION_LINES;
            self.conversation.drain(..drain_count);
            // Insert trim notice
            self.conversation.insert(0, ConversationLine::plain(
                "... (earlier messages trimmed)".to_string(),
                Color::DarkGray,
                false,
            ));
        }
        self.conversation_scroll = 0;
        self.needs_redraw = true;
    }

    // -----------------------------------------------------------------------
    // Main event loop
    // -----------------------------------------------------------------------

    pub fn read_line(&mut self) -> io::Result<TuiReadOutcome> {
        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    self.needs_redraw = true;
                    return self.handle_key(key);
                }
                Event::Resize(_width, _height) => {
                    self.mark_resize();
                }
                _ => {}
            }
        }

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
        let renderer = self.markdown_renderer.clone();
        let theme = self.theme.clone();

        self.terminal.draw(|f| {
            draw_frame(
                f,
                &dashboard,
                &conversation,
                conversation_scroll,
                &self.input,
                &input_lines,
                &slash_completions,
                completion_index,
                showing_completions,
                spinner_frame,
                &renderer,
                &theme,
            );
        })?;
        self.terminal.backend_mut().flush()?;
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
                self.conversation_scroll = self.conversation_scroll.saturating_add(5);
                Ok(TuiReadOutcome::Pending)
            }
            KeyCode::PageDown => {
                self.conversation_scroll = self.conversation_scroll.saturating_sub(5);
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
                let prefix = &current_text;
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
// Standalone draw functions
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn draw_frame(
    f: &mut Frame,
    dashboard: &SharedDashboardState,
    conversation: &[ConversationLine],
    conversation_scroll: u16,
    input: &TextArea,
    input_lines: &[String],
    slash_completions: &[String],
    completion_index: usize,
    showing_completions: bool,
    spinner_frame: usize,
    markdown_renderer: &crate::markdown::MarkdownRenderer,
    theme: &crate::theme::TuiTheme,
) {
    let size = f.area();
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(36)])
        .split(size);

    draw_left_pane(
        f,
        main[0],
        conversation,
        conversation_scroll,
        input,
        input_lines,
        slash_completions,
        completion_index,
        showing_completions,
        markdown_renderer,
    );
    draw_right_pane(f, main[1], dashboard, spinner_frame, theme);
}

/// Word-wrap a single text line into visual lines that fit `width` columns.
/// Returns a list of styled `Line` values.  Long words that exceed `width`
/// are hard-broken to prevent overflow.
fn wrap_line<'a>(text: &str, width: usize, style: Style) -> Vec<Line<'a>> {
    if width == 0 {
        return vec![Line::from(Span::styled(text.to_string(), style))];
    }

    let mut result: Vec<Line<'a>> = Vec::new();
    let mut remaining = text;

    if remaining.is_empty() {
        result.push(Line::from(Span::styled(String::new(), style)));
        return result;
    }

    while !remaining.is_empty() {
        if remaining.chars().count() <= width {
            result.push(Line::from(Span::styled(remaining.to_string(), style)));
            break;
        }

        // Find the last space within `width` chars
        let char_indices: Vec<(usize, char)> = remaining.char_indices().take(width).collect();
        let mut break_pos = None;

        for &(idx, ch) in char_indices.iter().rev() {
            if ch == ' ' || ch == '-' {
                break_pos = Some(idx + ch.len_utf8());
                break;
            }
        }

        let (head, tail) = match break_pos {
            Some(pos) => (&remaining[..pos], remaining[pos..].trim_start()),
            None => {
                // No space found — hard-break at column boundary
                let (idx, _) = char_indices.last().unwrap();
                let end = *idx + remaining[*idx..].chars().next().map_or(0, |c| c.len_utf8());
                (&remaining[..end], &remaining[end..])
            }
        };

        result.push(Line::from(Span::styled(head.to_string(), style)));
        remaining = tail;
    }

    result
}

/// Build the full conversation as wrapped visual lines, tracking how many
/// logical lines each `ConversationLine` expands to (for scroll math).
fn build_wrapped_conversation<'a>(
    conversation: &[ConversationLine],
    content_width: usize,
    markdown_renderer: &crate::markdown::MarkdownRenderer,
) -> (Vec<Line<'a>>, Vec<usize>) {
    let mut all_lines: Vec<Line<'a>> = Vec::new();
    let mut expand_counts: Vec<usize> = Vec::new();

    for cl in conversation {
        match &cl.content {
            ConversationContent::Plain { text, color, bold } => {
                let mut style = Style::default().fg(*color);
                if *bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                let wrapped = wrap_line(text, content_width, style);
                let count = wrapped.len().max(1);
                expand_counts.push(count);
                all_lines.extend(wrapped);
            }
            ConversationContent::Markdown { source } => {
                let rendered = markdown_renderer.render(source, content_width as u16);
                let count = rendered.len().max(1);
                expand_counts.push(count);
                // rendered is Vec<Line<'static>> — safe to extend
                all_lines.extend(rendered.into_iter().map(|l: Line<'static>| {
                    // Convert Line<'static> to Line<'a> via into_owned pattern
                    Line::from(l.spans.into_iter().map(|s| {
                        Span::styled(s.content.into_owned(), s.style)
                    }).collect::<Vec<_>>())
                }));
            }
            ConversationContent::CodeDiff { diff } => {
                let rendered = crate::markdown::render_diff(diff);
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

fn draw_left_pane(
    f: &mut Frame,
    area: Rect,
    conversation: &[ConversationLine],
    conversation_scroll: u16,
    input: &TextArea,
    input_lines: &[String],
    slash_completions: &[String],
    completion_index: usize,
    showing_completions: bool,
    markdown_renderer: &crate::markdown::MarkdownRenderer,
) {
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(7)])
        .split(area);

    // --- conversation with word-wrapping ---
    // Subtract 1 for the top border, 2 for left/right block padding
    let content_width = (left[0].width as usize).saturating_sub(2);
    let (wrapped, expand_counts) = build_wrapped_conversation(conversation, content_width, markdown_renderer);

    let pane_rows = (left[0].height.saturating_sub(1) as usize).max(1);
    let total_visual = wrapped.len();

    // Compute how many visual rows to skip from the top (scroll offset).
    // conversation_scroll=0 means "show newest content at the bottom".
    let scroll = conversation_scroll as usize;
    let max_offset = total_visual.saturating_sub(pane_rows);
    let offset = scroll.min(max_offset);
    let start = total_visual.saturating_sub(pane_rows + offset);
    let visible: Vec<Line> = wrapped.into_iter().skip(start).take(pane_rows).collect();

    let conversation_widget = Paragraph::new(visible).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(Span::styled(
                " Conversation ",
                Style::default().fg(Color::DarkGray),
            )),
    );
    f.render_widget(conversation_widget, left[0]);

    // Scroll indicator (e.g. "3/47") when content overflows
    if total_visual > pane_rows {
        let total_logical: usize = expand_counts.len();
        let visible_logical = if offset == 0 {
            // Bottom of conversation — count logical lines in the visible window
            let mut used = pane_rows;
            let mut count = 0;
            for &exp in expand_counts.iter().rev() {
                if used == 0 {
                    break;
                }
                let take = exp.min(used);
                used -= take;
                count += 1;
            }
            count
        } else {
            // Simplified: just show visual line offset
            pane_rows
        };
        let _ = (visible_logical, total_logical); // used below
        let scroll_label = format!(" {}/{} ", offset, max_offset);
        let scroll_area = Rect {
            x: left[0].x + left[0].width.saturating_sub(scroll_label.len() as u16 + 1),
            y: left[0].y,
            width: scroll_label.len() as u16 + 1,
            height: 1,
        };
        f.render_widget(
            Paragraph::new(Span::styled(
                scroll_label,
                Style::default().fg(Color::DarkGray),
            )),
            scroll_area,
        );
    }

    // --- input (real TextArea widget) ---
    let input_widget = input.clone();
    f.render_widget(&input_widget, left[1]);

    // --- completions popup ---
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
            let popup = Rect {
                x: left[1].x,
                y: left[1].y.saturating_sub(matches.len().min(8) as u16 + 2),
                width: left[1].width.min(40),
                height: (matches.len() as u16 + 2).min(10),
            };
            f.render_widget(list, popup);
        }
    }
}

fn draw_right_pane(
    f: &mut Frame,
    area: Rect,
    dashboard: &SharedDashboardState,
    spinner_frame: usize,
    theme: &crate::theme::TuiTheme,
) {
    let state = dashboard.read().unwrap_or_else(|e| e.into_inner());
    let mut lines: Vec<Line> = Vec::new();
    #[allow(unused_assignments)]
    let mut gauge_row: Option<usize> = None;

    lines.push(section("Connection"));
    lines.push(kv("Model", &state.model, theme.dashboard_value.to_color()));
    lines.push(kv("Provider", &state.provider, theme.dashboard_key.to_color()));
    lines.push(kv("URL", &state.provider_url, theme.conversation_dim.to_color()));
    lines.push(kv("Mode", &state.permission_mode, theme.conversation_system.to_color()));
    if let Some(ref branch) = state.git_branch {
        lines.push(kv("Branch", branch, theme.agent_done.to_color()));
    }
    lines.push(Line::from(""));

    lines.push(section("Tokens"));
    lines.push(kv("Turns", &state.turn_count.to_string(), theme.dashboard_value.to_color()));
    lines.push(kv("Input", &state.input_tokens.to_string(), theme.dashboard_value.to_color()));
    lines.push(kv("Output", &state.output_tokens.to_string(), theme.dashboard_value.to_color()));
    lines.push(kv(
        "Cache R",
        &state.cache_read_tokens.to_string(),
        theme.dashboard_key.to_color(),
    ));
    lines.push(kv(
        "Cache W",
        &state.cache_creation_tokens.to_string(),
        theme.dashboard_key.to_color(),
    ));
    lines.push(kv(
        "Cost",
        &format!("${:.4}", state.cost_usd),
        theme.conversation_system.to_color(),
    ));
    lines.push(Line::from(""));

    let pct = state.context_percent;
    let gauge_color = if pct > 80.0 {
        theme.gauge_fill_red.to_color()
    } else if pct > 50.0 {
        theme.gauge_fill_yellow.to_color()
    } else {
        theme.gauge_fill_green.to_color()
    };
    lines.push(section("Context"));
    lines.push(kv(
        "Used",
        &format!("{:.1}% of {}", pct, state.context_window),
        theme.dashboard_value.to_color(),
    ));
    gauge_row = Some(lines.len());
    lines.push(Line::from(""));
    lines.push(kv(
        "Compactions",
        &state.compaction_count.to_string(),
        theme.dashboard_key.to_color(),
    ));
    lines.push(Line::from(""));

    if !state.lsp_servers.is_empty() {
        lines.push(section("LSP"));
        for lsp in &state.lsp_servers {
            let c = match lsp.status.as_str() {
                "connected" => theme.agent_done.to_color(),
                "starting" => theme.agent_waiting.to_color(),
                _ => theme.agent_failed.to_color(),
            };
            lines.push(kv(&lsp.language, &lsp.status, c));
        }
        lines.push(Line::from(""));
    }

    if let Some(ref team) = state.team {
        lines.push(section("Team"));
        lines.push(kv("Name", &team.team_name, theme.dashboard_value.to_color()));
        let progress = format!(
            "{}/{} done, {} fail, {} run",
            team.completed_agents, team.total_agents, team.failed_agents, team.running_agents
        );
        lines.push(kv("Status", &progress, theme.agent_done.to_color()));
        for agent in &team.agents {
            let c = match agent.status.as_str() {
                "completed" => theme.agent_done.to_color(),
                "failed" => theme.agent_failed.to_color(),
                _ => theme.agent_running.to_color(),
            };
            let label = format!("● {}", agent.name);
            let detail = format!("({})", agent.subagent_type.as_deref().unwrap_or("?"));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<KV_KEY_WIDTH$}", label),
                    Style::default().fg(c),
                ),
                Span::styled(detail, Style::default().fg(theme.dashboard_key.to_color())),
            ]));
        }
        lines.push(Line::from(""));
    }

    lines.push(section("Session"));
    lines.push(kv(
        "ID",
        state.session_id.as_deref().unwrap_or("-"),
        theme.dashboard_key.to_color(),
    ));

    if !state.status_message.is_empty() {
        let frame = SPINNER_FRAMES[spinner_frame];
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("{frame} {}", state.status_message),
            Style::default().fg(theme.spinner.to_color()),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "─ Keys ─",
        Style::default().fg(theme.key_hint.to_color()),
    )));
    lines.push(Line::from(Span::styled(
        "  Enter Submit  Shift+Enter ↵",
        Style::default().fg(theme.key_hint.to_color()),
    )));
    lines.push(Line::from(Span::styled(
        "  ^P Swap  ^T Team  ^C ⊘  ^D Exit",
        Style::default().fg(theme.key_hint.to_color()),
    )));

    let widget = Paragraph::new(lines)
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
        .wrap(Wrap { trim: false });
    f.render_widget(widget, area);

    // Overlay the context gauge at the tracked row (not a hard-coded offset)
    if let Some(row) = gauge_row {
        // +1 for the block's top border
        let gauge_area = Rect {
            x: area.x + 2,
            y: area.y + 1 + row as u16,
            width: area.width.saturating_sub(4),
            height: 1,
        };
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(gauge_color).bg(Color::DarkGray))
            .ratio(if pct > 0.0 {
                (pct / 100.0).min(1.0)
            } else {
                0.0
            });
        f.render_widget(gauge, gauge_area);
    }
}

fn section<'a>(label: &str) -> Line<'a> {
    Line::from(Span::styled(
        format!("─ {label} ─"),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}

/// Key-value row with fixed-width key column so values align vertically.
/// Key is right-padded to `KV_KEY_WIDTH` columns: `"  Model       value"`.
fn kv<'a>(key: &str, val: &str, val_color: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {:<KV_KEY_WIDTH$}", key),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(val.to_string(), Style::default().fg(val_color)),
    ])
}

// ANSI stripping is now in tui_update::strip_ansi() — single canonical implementation.
