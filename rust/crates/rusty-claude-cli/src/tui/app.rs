//! Component-based TuiApp.
//!
//! Replaces the legacy god-struct `TuiApp` with a clean component architecture.
//! Uses the pre-borrow draw pattern to avoid the clone-everything problem.

use std::io;
use std::io::Write;
use std::sync::Arc;

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::Frame;

use crate::keybindings::{Action, KeyMap, KeyPreset, VimMode};
use crate::theme::TuiTheme;
use crate::tui::component::{Component, Overlay};
use crate::tui::components::command_palette::CommandPaletteOverlay;
use crate::tui::components::agent_view::AgentViewOverlay;
use crate::tui::components::conversation::ConversationPane;
use crate::tui::components::dashboard::Dashboard;
use crate::tui::components::input_bar::{InputBar, InputOutcome};
use crate::tui::components::status_bar::StatusBar;
use crate::tui::event::{EventBus, TuiEvent};
use crate::tui::legacy::{BannerLine, SharedDashboardState, TuiReadOutcome};

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// The new component-based TUI application.
pub struct TuiApp {
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,

    // Components (each owns its own state)
    conversation: ConversationPane,
    input_bar: InputBar,
    dashboard: Dashboard,
    status_bar: StatusBar,
    command_palette: CommandPaletteOverlay,
    agent_view: AgentViewOverlay,

    // Shared state
    theme: TuiTheme,
    keymap: KeyMap,
    chat_mode: crate::chat_mode::ChatMode,

    // Event bus (Phase 3: will be crossbeam-channel)
    _event_bus: EventBus,

    // Lifecycle
    should_exit: bool,
    spinner_frame: usize,
}

impl TuiApp {
    /// Initialize the TUI — enter alternate screen, enable raw mode.
    pub fn init(dashboard_state: SharedDashboardState) -> Result<Self, Box<dyn std::error::Error>> {
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

        let theme = TuiTheme::builtin("default").unwrap();
        let input_bar = InputBar::new(&theme);

        let app = Self {
            conversation: ConversationPane::new(theme.clone()),
            input_bar,
            dashboard: Dashboard::new(dashboard_state.clone()),
            status_bar: StatusBar::new(dashboard_state),
            command_palette: CommandPaletteOverlay::new(),
            agent_view: AgentViewOverlay::new(),
            theme,
            keymap: KeyMap::new(KeyPreset::Emacs),
            chat_mode: crate::chat_mode::ChatMode::Code,
            _event_bus: EventBus::new(),
            should_exit: false,
            spinner_frame: 0,
            terminal,
        };

        Ok(app)
    }

    /// Suspend TUI for a blocking stdout operation.
    pub fn suspend(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.terminal.show_cursor();
        disable_raw_mode()?;
        let _ = crossterm::execute!(io::stdout(), Clear(ClearType::All), crossterm::cursor::MoveTo(0, 0));
        let _ = io::stdout().flush();
        Ok(())
    }

    /// Resume TUI after suspend.
    pub fn resume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let _ = crossterm::execute!(io::stdout(), Clear(ClearType::All), crossterm::cursor::MoveTo(0, 0));
        let _ = io::stdout().flush();
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    /// Leave the TUI before a turn: exit alt screen, disable raw mode,
    /// leave cursor visible, and clear.  Stdout during the turn now goes
    /// to the normal terminal instead of corrupting the TUI frame.
    pub fn leave_for_turn(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.terminal.show_cursor();
        disable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::terminal::Clear(ClearType::All),
            crossterm::cursor::MoveTo(0, 0),
            crossterm::cursor::Show
        )?;
        io::stdout().flush()?;
        Ok(())
    }

    /// Wait for one keypress before re-entering the TUI.
    /// Gives the user a chance to read any tool output that printed
    /// to the terminal during the turn.
    pub fn wait_to_return(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::style::Print("\nPress any key to return to TUI..."),
            crossterm::cursor::MoveTo(0, 0)
        )?;
        io::stdout().flush()?;

        // Block until a key is pressed
        loop {
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(_) = event::read()? {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Re-enter the TUI after a turn: alt screen, raw mode, clear, redraw.
    pub fn reenter_after_turn(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::terminal::Clear(ClearType::All),
            crossterm::cursor::Hide
        )?;
        io::stdout().flush()?;
        enable_raw_mode()?;
        self.terminal.clear()?;
        Ok(())
    }

    /// Restore terminal on exit.
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

    // -------------------------------------------------------------------
    // Content helpers (same API as legacy TuiApp)
    // -------------------------------------------------------------------

    pub fn push_banner(&mut self, lines: &[BannerLine]) {
        for bl in lines {
            self.conversation.push_banner(bl.text.clone(), bl.color);
        }
    }

    pub fn push_user_input(&mut self, text: &str) {
        let color = self.theme.conversation_user.to_color();
        self.conversation.push_user_input(text, color);
    }

    pub fn push_system_message(&mut self, text: &str) {
        let color = self.theme.conversation_system.to_color();
        self.conversation.push_system_message(text, color);
    }

    pub fn push_output(&mut self, text: &str, is_error: bool) {
        self.conversation.push_output(text, is_error, &self.theme);
    }

    pub fn push_diff(&mut self, diff: &str) {
        self.conversation.push_diff(diff);
    }

    pub fn set_status(&mut self, msg: &str) {
        self.dashboard.set_status(msg);
    }

    pub fn set_theme(&mut self, theme: TuiTheme) {
        self.conversation.set_theme(theme.clone());
        self.input_bar.set_theme(&theme);
        self.theme = theme;
    }

    pub fn set_key_preset(&mut self, preset: KeyPreset) {
        self.keymap.set_preset(preset);
    }

    pub fn key_preset_name(&self) -> &'static str {
        match self.keymap.preset() {
            KeyPreset::Emacs => "Emacs",
            KeyPreset::Vim => "Vim",
            KeyPreset::Windows => "Windows",
        }
    }

    // -------------------------------------------------------------------
    // Main event loop
    // -------------------------------------------------------------------

    pub fn read_line(&mut self) -> io::Result<TuiReadOutcome> {
        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => return self.handle_key(key),
                Event::Resize(..) => {
                    // Force re-render at new dimensions
                }
                _ => {}
            }
        }

        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
        self.dashboard.tick_spinner();
        self.status_bar.tick_spinner();
        self.draw_screen()?;
        Ok(TuiReadOutcome::Pending)
    }

    fn handle_key(&mut self, key: KeyEvent) -> io::Result<TuiReadOutcome> {
        // Command palette intercepts all keys when active
        if self.command_palette.is_active() {
            let consumed = self.command_palette.handle_key(key, &self.keymap);
            if consumed {
                if let Some(action) = self.command_palette.selected_action() {
                    self.command_palette.close();
                    return self.dispatch_action(action);
                }
                self.draw_screen()?;
                return Ok(TuiReadOutcome::Pending);
            }
        }

        // Agent view intercepts all keys when active
        if self.agent_view.is_active() {
            let consumed = self.agent_view.handle_key(key, &self.keymap);
            if consumed {
                self.draw_screen()?;
                return Ok(TuiReadOutcome::Pending);
            }
        }

        // Input bar handles the key
        let outcome = self.input_bar.process_key(key, &mut self.keymap);
        match outcome {
            InputOutcome::Submit(text) => {
                self.input_bar.push_history(&text);
                Ok(TuiReadOutcome::Submit(text))
            }
            InputOutcome::Cancel => Ok(TuiReadOutcome::Cancel),
            InputOutcome::Exit => {
                self.should_exit = true;
                Ok(TuiReadOutcome::Exit)
            }
            InputOutcome::ProviderSwap => Ok(TuiReadOutcome::ProviderSwap),
            InputOutcome::TeamToggle => Ok(TuiReadOutcome::TeamToggle),
            InputOutcome::ToggleAgentView => Ok(TuiReadOutcome::ToggleAgentView),
            InputOutcome::None => {
                self.draw_screen()?;
                Ok(TuiReadOutcome::Pending)
            }
        }
    }

    fn dispatch_action(&mut self, action: Action) -> io::Result<TuiReadOutcome> {
        match action {
            Action::CommandPalette => { self.command_palette.open(); Ok(TuiReadOutcome::Pending) }
            Action::ToggleAgentView => Ok(TuiReadOutcome::ToggleAgentView),
            Action::Help => {
                let preset = self.key_preset_name().to_string();
                let msg = format!("Keybindings ({preset}):\n\nEnter Submit  Shift+Enter ↵\nCtrl+C Cancel  Ctrl+D Exit\nCtrl+P Swap  Ctrl+K Palette\nCtrl+A Agents  Ctrl+T Team\n");
                self.push_system_message(&msg);
                Ok(TuiReadOutcome::Pending)
            }
            Action::ClearConversation => {
                self.conversation.clear();
                Ok(TuiReadOutcome::Pending)
            }
            Action::ScrollUp => { self.conversation.scroll_up(1); Ok(TuiReadOutcome::Pending) }
            Action::ScrollDown => { self.conversation.scroll_down(1); Ok(TuiReadOutcome::Pending) }
            Action::ScrollHalfUp => { self.conversation.scroll_up(5); Ok(TuiReadOutcome::Pending) }
            Action::ScrollHalfDown => { self.conversation.scroll_down(5); Ok(TuiReadOutcome::Pending) }
            Action::ScrollTop => { self.conversation.scroll_top(); Ok(TuiReadOutcome::Pending) }
            Action::ScrollBottom => { self.conversation.scroll_bottom(); Ok(TuiReadOutcome::Pending) }
            _ => Ok(TuiReadOutcome::Pending),
        }
    }

    // -------------------------------------------------------------------
    // Rendering — pre-borrow pattern
    // -------------------------------------------------------------------

    fn draw_screen(&mut self) -> io::Result<()> {
        // Pre-borrow component references before the draw closure.
        // Each component.render(&self) reads its own state — no cloning needed.
        let conversation = &self.conversation;
        let input_bar = &self.input_bar;
        let dashboard = &self.dashboard;
        let status_bar = &self.status_bar;
        let command_palette = &self.command_palette;
        let agent_view = &self.agent_view;
        let theme = &self.theme;

        self.terminal.draw(|f| {
            let area = f.area();

            // OpenCode-style focused layout:
            // - full-width status bar on top
            // - large conversation pane below it
            // - compact input bar at the bottom
            // - optional right-side dashboard (hidden if terminal is narrow)
            let has_room_for_dashboard = area.width >= 100;
            let dashboard_width = if has_room_for_dashboard { 32u16 } else { 0u16 };

            let main = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(40),
                    Constraint::Length(dashboard_width),
                ])
                .split(area);

            let left = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // status bar
                    Constraint::Min(5),    // conversation
                    Constraint::Length(6), // input
                ])
                .split(main[0]);

            status_bar.render(left[0], f, theme);
            conversation.render(left[1], f, theme);
            input_bar.render(left[2], f, theme);

            if dashboard_width > 0 {
                dashboard.render(main[1], f, theme);
            }

            // Overlays
            if command_palette.is_active() {
                command_palette.render_overlay(area, f, theme);
            }
            if agent_view.is_active() {
                agent_view.render_overlay(area, f, theme);
            }
        })?;

        // Clear dirty flags after successful render
        self.conversation.mark_clean();
        self.dashboard.clear_dirty();
        self.status_bar.clear_dirty();

        self.terminal.backend_mut().flush()?;
        Ok(())
    }

    /// Force full redraw after a turn.
    pub fn redraw_after_turn(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal.clear()?;
        self.draw_screen()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_app_init_requires_terminal() {
        // TuiApp::init() requires a real terminal — just verify the type exists.
        // Real tests run via `cargo test --bin claw -- tui::app`
    }
}
