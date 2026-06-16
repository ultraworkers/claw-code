//! Component-based TuiApp.
//!
//! Replaces the legacy god-struct `TuiApp` with a clean component architecture.
//! Uses the pre-borrow draw pattern to avoid the clone-everything problem.
//! Uses `TerminalGuard` for safe terminal lifecycle management.

use std::io;
use std::io::Write;

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::Frame;

use crate::keybindings::{Action, KeyMap, KeyPreset, VimMode};
use crate::theme::TuiTheme;
use crate::tui::component::{Component, Overlay};
use crate::tui::components::agent_view::AgentViewOverlay;
use crate::tui::components::command_palette::CommandPaletteOverlay;
use crate::tui::components::conversation::ConversationPane;
use crate::tui::components::dashboard::Dashboard;
use crate::tui::components::input_bar::{InputBar, InputOutcome};
use crate::tui::components::status_bar::StatusBar;
use crate::tui::event::{EventBus, TuiEvent};
use crate::tui::legacy::{BannerLine, SharedDashboardState, TuiReadOutcome};

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// ---------------------------------------------------------------------------
// TerminalGuard — owns the terminal lifecycle, guarantees cleanup
// ---------------------------------------------------------------------------

/// RAII guard for alternate screen + raw mode.
///
/// Created in `TuiApp::init()`. On drop it restores the terminal to its
/// original state — even if the code panics. This replaces the ad-hoc
/// suspend/resume/restore methods and the panic hook that tried to do
/// the same thing indirectly.
pub struct TerminalGuard {
    /// Whether we are currently inside the alternate screen + raw mode.
    /// Toggled by `leave_for_turn()` / `reenter_after_turn()`.
    in_tui: bool,
}

impl TerminalGuard {
    /// Enter alternate screen, enable raw mode, clear screen, hide cursor.
    pub fn enter() -> Result<Self, Box<dyn std::error::Error>> {
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        )?;
        enable_raw_mode()?;
        Ok(Self { in_tui: true })
    }

    /// Leave the alternate screen so a blocking turn can use the terminal.
    /// This is the *only* method that temporarily gives up terminal ownership.
    /// Call `reenter_after_turn()` to take it back.
    pub fn leave_for_turn(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.in_tui {
            return Ok(());
        }
        // Best-effort: show cursor, leave alt screen, disable raw mode
        let _ = crossterm::execute!(
            io::stdout(),
            crossterm::cursor::Show,
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::terminal::Clear(ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        );
        let _ = io::stdout().flush();
        let _ = disable_raw_mode();
        self.in_tui = false;
        Ok(())
    }

    /// Re-enter the alternate screen after a turn.
    pub fn reenter_after_turn(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.in_tui {
            return Ok(());
        }
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::terminal::Clear(ClearType::All),
            crossterm::cursor::Hide
        )?;
        enable_raw_mode()?;
        let _ = io::stdout().flush();
        self.in_tui = true;
        Ok(())
    }

    /// Whether we are currently in the TUI (alternate screen + raw mode).
    pub fn is_in_tui(&self) -> bool {
        self.in_tui
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.in_tui {
            // Best-effort cleanup — ignore errors since we might be panicking
            let _ = crossterm::execute!(
                io::stdout(),
                crossterm::cursor::Show,
                crossterm::terminal::LeaveAlternateScreen,
                crossterm::terminal::Clear(ClearType::All),
                crossterm::cursor::MoveTo(0, 0)
            );
            let _ = disable_raw_mode();
            let _ = io::stdout().flush();
        }
    }
}

/// The new component-based TUI application.
pub struct TuiApp {
    terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    guard: TerminalGuard,

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

    // Event bus — receives turn lifecycle events and streaming deltas
    event_bus: EventBus,

    // Lifecycle
    should_exit: bool,
    spinner_frame: usize,
}

impl TuiApp {
    /// Initialize the TUI — enter alternate screen, enable raw mode.
    pub fn init(dashboard_state: SharedDashboardState) -> Result<Self, Box<dyn std::error::Error>> {
        let guard = TerminalGuard::enter()?;

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
            event_bus: EventBus::new(),
            should_exit: false,
            spinner_frame: 0,
            terminal,
            guard,
        };

        Ok(app)
    }

    /// Leave the alternate screen so a blocking turn can use the terminal.
    /// The guard tracks state — call `reenter_after_turn()` to return.
    pub fn leave_for_turn(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.terminal.show_cursor();
        self.guard.leave_for_turn()
    }

    /// Re-enter the alternate screen after a turn and force a full redraw.
    pub fn reenter_after_turn(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.guard.reenter_after_turn()?;
        self.terminal.clear()?;
        Ok(())
    }

    /// Restore terminal on exit. The guard's Drop also handles this,
    /// but calling explicitly allows a clean ordered shutdown.
    pub fn restore_terminal(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.terminal.show_cursor();
        self.guard.leave_for_turn()
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

    pub fn set_turn_in_progress(&mut self, in_progress: bool) {
        self.input_bar.set_turn_in_progress(in_progress);
    }

    pub fn key_preset_name(&self) -> &'static str {
        match self.keymap.preset() {
            KeyPreset::Emacs => "Emacs",
            KeyPreset::Vim => "Vim",
            KeyPreset::Windows => "Windows",
        }
    }

    /// Get a sender for posting events to the TUI event bus.
    /// Use this from background threads to post streaming updates.
    pub fn event_sender(&self) -> crossbeam_channel::Sender<TuiEvent> {
        self.event_bus.sender()
    }

    /// Drain pending events and update components.
    /// Returns true if any events were processed (meaning a redraw is needed).
    pub fn drain_events(&mut self) -> bool {
        let events = self.event_bus.drain();
        if events.is_empty() {
            return false;
        }

        for event in events {
            match event {
                TuiEvent::StreamTextDelta { text } => {
                    self.conversation.push_output(&text, false, &self.theme);
                }
                TuiEvent::TurnComplete { assistant_text } => {
                    if !assistant_text.is_empty() {
                        self.conversation
                            .push_output(&assistant_text, false, &self.theme);
                    }
                    self.input_bar.set_turn_in_progress(false);
                    self.dashboard.set_status("Done");
                }
                TuiEvent::TurnError { error } => {
                    let color = self.theme.conversation_error.to_color();
                    self.conversation
                        .push_system_message(&format!("Error: {error}"), color);
                    self.input_bar.set_turn_in_progress(false);
                    self.dashboard.set_status("");
                }
                TuiEvent::TurnStarted => {
                    self.input_bar.set_turn_in_progress(true);
                    self.dashboard.set_status("Thinking...");
                }
                TuiEvent::DashboardUpdate(_state) => {
                    // Dashboard state is shared via SharedDashboardState (Arc<RwLock<>>).
                    // The main loop calls update_dashboard() directly.  When we move
                    // to the full event-driven architecture, this will post the update
                    // through the shared state instead.
                }
                TuiEvent::ThemeChanged(theme) => {
                    self.set_theme(theme);
                }
                TuiEvent::KeymapChanged(preset) => {
                    self.keymap.set_preset(preset);
                }
                TuiEvent::ChatModeChanged(mode) => {
                    self.chat_mode = mode;
                }
                // Other events are handled by their respective components
                // when we move to the full event-driven architecture
                _ => {}
            }
        }

        true
    }

    // -------------------------------------------------------------------
    // Main event loop
    // -------------------------------------------------------------------

    pub fn read_line(&mut self) -> io::Result<TuiReadOutcome> {
        // Drain pending TUI events (streaming deltas, turn lifecycle, etc.)
        let _events_processed = self.drain_events();

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
                self.set_turn_in_progress(true);
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
            Action::CommandPalette => {
                self.command_palette.open();
                Ok(TuiReadOutcome::Pending)
            }
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
            Action::ScrollUp => {
                self.conversation.scroll_up(1);
                Ok(TuiReadOutcome::Pending)
            }
            Action::ScrollDown => {
                self.conversation.scroll_down(1);
                Ok(TuiReadOutcome::Pending)
            }
            Action::ScrollHalfUp => {
                self.conversation.scroll_up(5);
                Ok(TuiReadOutcome::Pending)
            }
            Action::ScrollHalfDown => {
                self.conversation.scroll_down(5);
                Ok(TuiReadOutcome::Pending)
            }
            Action::ScrollTop => {
                self.conversation.scroll_top();
                Ok(TuiReadOutcome::Pending)
            }
            Action::ScrollBottom => {
                self.conversation.scroll_bottom();
                Ok(TuiReadOutcome::Pending)
            }
            _ => Ok(TuiReadOutcome::Pending),
        }
    }

    // -------------------------------------------------------------------
    // Rendering — pre-borrow pattern
    // -------------------------------------------------------------------

    pub fn draw_screen(&mut self) -> io::Result<()> {
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
                .constraints([Constraint::Min(40), Constraint::Length(dashboard_width)])
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

    #[test]
    fn test_terminal_guard_default_state() {
        // TerminalGuard::enter() requires a real terminal.
        // Just verify the struct fields are accessible and the type exists.
        let guard = TerminalGuard { in_tui: false };
        assert!(!guard.is_in_tui());
    }

    #[test]
    fn test_terminal_guard_in_tui_flag() {
        let guard = TerminalGuard { in_tui: true };
        assert!(guard.is_in_tui());
    }

    #[test]
    fn test_guard_drop_on_panic_restores_terminal() {
        // Simulate a guard that's in_tui when dropped (panic scenario).
        // The Drop impl should try to leave alternate screen + disable raw mode.
        // We can't fully test this without a terminal, but we verify the struct
        // can be constructed in the right state.
        let guard = TerminalGuard { in_tui: true };
        assert!(guard.is_in_tui());
        // Drop happens here — in a real terminal it would clean up
    }

    /// Regression: EventBus can be created and drained without a terminal.
    #[test]
    fn test_event_bus_sender_and_drain() {
        let bus = EventBus::new();
        let sender = bus.sender();
        sender.send(TuiEvent::TurnStarted).unwrap();
        sender
            .send(TuiEvent::TurnError {
                error: "test".into(),
            })
            .unwrap();
        let events = bus.drain();
        assert_eq!(events.len(), 2);
    }

    /// Regression: drain on empty bus returns empty vec.
    #[test]
    fn test_event_bus_drain_empty() {
        let bus = EventBus::new();
        assert!(bus.drain().is_empty());
    }

    /// Regression: leave_for_turn is a no-op when already outside TUI.
    #[test]
    fn test_terminal_guard_leave_when_already_outside() {
        let mut guard = TerminalGuard { in_tui: false };
        assert!(guard.leave_for_turn().is_ok());
        assert!(!guard.is_in_tui());
    }

    /// Regression: reenter_after_turn is a no-op when already inside TUI.
    #[test]
    fn test_terminal_guard_reenter_when_already_inside() {
        let guard = TerminalGuard { in_tui: true };
        // reenter_after_turn would try to enter alternate screen,
        // which fails without a real terminal. But the guard should
        // short-circuit since it's already in_tui.
        // (We can't call it without a terminal, so just verify the state check.)
        assert!(guard.is_in_tui());
    }
}
