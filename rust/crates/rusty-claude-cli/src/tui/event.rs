//! Event bus for the TUI architecture.
//!
//! The event bus decouples the TUI event loop from the runtime. The event
//! loop drains the channel non-blocking each tick. Components process
//! events during the update phase before rendering.

use crate::agent_view::AgentSession;
use crate::chat_mode::ChatMode;
use crate::keybindings::KeyPreset;
use crate::theme::TuiTheme;
use crate::tui::legacy::DashboardState;

/// Events that flow through the TUI event bus.
#[derive(Debug, Clone)]
pub enum TuiEvent {
    // --- Streaming events (Phase 4) ---
    StreamTextDelta {
        text: String,
    },
    StreamThinking {
        thinking: String,
    },
    StreamToolUse {
        id: String,
        name: String,
    },
    StreamUsage {
        input_tokens: u32,
        output_tokens: u32,
    },
    StreamMessageStop,

    // --- Turn lifecycle ---
    TurnComplete {
        assistant_text: String,
    },
    TurnError {
        error: String,
    },
    TurnStarted,

    // --- Dashboard ---
    DashboardUpdate(DashboardState),

    // --- Agent lifecycle ---
    AgentSessionUpdate(AgentSession),
    AgentSessionRemove {
        id: String,
    },

    // --- UI state changes ---
    ThemeChanged(TuiTheme),
    KeymapChanged(KeyPreset),
    ChatModeChanged(ChatMode),

    // --- System ---
    Resize {
        width: u16,
        height: u16,
    },
}

/// The central event channel.
///
/// The main event loop reads from `rx` (non-blocking). Background workers
/// and the main thread write to `tx`.
pub struct EventBus {
    tx: crossbeam_channel::Sender<TuiEvent>,
    rx: crossbeam_channel::Receiver<TuiEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self { tx, rx }
    }

    /// Get a sender for posting events.
    pub fn sender(&self) -> crossbeam_channel::Sender<TuiEvent> {
        self.tx.clone()
    }

    /// Try to receive an event (non-blocking).
    /// Returns `None` if no event is available.
    pub fn try_recv(&self) -> Option<TuiEvent> {
        self.rx.try_recv().ok()
    }

    /// Drain all pending events.
    pub fn drain(&self) -> Vec<TuiEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.try_recv() {
            events.push(event);
        }
        events
    }

    /// Send an event (non-blocking, never blocks).
    pub fn send(&self, event: TuiEvent) {
        let _ = self.tx.send(event);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus_send_recv() {
        let bus = EventBus::new();
        bus.send(TuiEvent::TurnStarted);
        bus.send(TuiEvent::TurnError {
            error: "test".into(),
        });

        let events = bus.drain();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], TuiEvent::TurnStarted));
        assert!(matches!(events[1], TuiEvent::TurnError { .. }));
    }

    #[test]
    fn test_event_bus_drain_empty() {
        let bus = EventBus::new();
        let events = bus.drain();
        assert!(events.is_empty());
    }

    #[test]
    fn test_event_bus_sender_clone() {
        let bus = EventBus::new();
        let tx = bus.sender();
        tx.send(TuiEvent::TurnStarted).unwrap();
        let events = bus.drain();
        assert_eq!(events.len(), 1);
    }
}
