//! Component traits for the TUI architecture.
//!
//! The `Component` trait is the core abstraction — each visual region of the
//! TUI (conversation pane, input bar, dashboard, overlays) implements it.
//!
//! Design note: `render` takes `&self`, not `&mut self`. This is essential
//! because `Terminal::draw()` borrows the terminal mutably. Components
//! pre-compute render data in `handle_event`/`handle_key`, then `render`
//! simply reads the pre-built cache. (Elm architecture: update then view.)

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::keybindings::KeyMap;
use crate::theme::TuiTheme;
use crate::tui::event::TuiEvent;

/// A self-contained renderable region of the TUI.
pub trait Component {
    /// Render this component into the given area.
    fn render(&self, area: Rect, frame: &mut Frame, theme: &TuiTheme);

    /// Process a key event. Returns `true` if consumed.
    fn handle_key(&mut self, _key: KeyEvent, _keymap: &KeyMap) -> bool {
        false
    }

    /// Process a TUI event from the event bus.
    fn handle_event(&mut self, _event: &TuiEvent) {}

    /// Whether this component needs a redraw.
    fn is_dirty(&self) -> bool;
}

/// A component that renders as a full-screen overlay.
///
/// Overlays (command palette, agent view) intercept all keys while active
/// and render on top of the normal frame.
pub trait Overlay: Component {
    /// Whether this overlay is currently active.
    fn is_active(&self) -> bool;

    /// Activate this overlay.
    fn open(&mut self);

    /// Deactivate this overlay.
    fn close(&mut self);

    /// Render as an overlay: clear the area first, then render.
    fn render_overlay(&self, area: Rect, frame: &mut Frame, theme: &TuiTheme) {
        frame.render_widget(ratatui::widgets::Clear, area);
        self.render(area, frame, theme);
    }
}
