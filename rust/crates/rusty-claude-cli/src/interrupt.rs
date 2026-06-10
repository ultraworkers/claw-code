//! Esc-key interruption for running turns.
//!
//! While a turn runs, the main thread is blocked inside the conversation
//! loop, so rustyline is not reading the keyboard. This module spawns a
//! listener thread that switches stdin to a minimal non-canonical mode
//! (line buffering and echo off; signal handling and output processing
//! stay enabled, unlike full raw mode) and watches for a lone Escape
//! byte. When one arrives it trips the shared [`TurnInterruptSignal`],
//! which the conversation loop and the streaming API client poll to wind
//! the turn down gracefully.
//!
//! Permission prompts read whole lines from stdin mid-turn, so stdin
//! ownership is coordinated through [`StdinPromptGate`]: while a prompt
//! holds a lease, the listener restores canonical mode and stops
//! consuming bytes.

use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

use runtime::TurnInterruptSignal;

/// Terminal-mode bookkeeping guarded by the gate mutex. `saved` holds the
/// canonical-mode termios while non-canonical mode is active.
#[derive(Default)]
struct TtyMode {
    #[cfg(unix)]
    saved: Option<nix::sys::termios::Termios>,
}

#[derive(Default)]
struct GateState {
    /// True while a permission prompt owns stdin.
    busy: AtomicBool,
    /// Tells the listener thread to exit at the end of the turn.
    stop: AtomicBool,
    /// Serializes stdin reads and terminal-mode flips.
    mode: Mutex<TtyMode>,
}

/// Classification of one chunk of bytes read from the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EscClassification {
    /// A bare ESC byte: the user pressed the Escape key.
    LoneEscape,
    /// ESC followed by more bytes: the prefix of a terminal escape
    /// sequence such as an arrow or function key.
    EscapeSequence,
    /// Anything else (regular type-ahead, control characters, ...).
    Other,
}

#[cfg_attr(not(unix), allow(dead_code))]
fn classify_escape_chunk(bytes: &[u8]) -> EscClassification {
    match bytes {
        [0x1b] => EscClassification::LoneEscape,
        [0x1b, ..] => EscClassification::EscapeSequence,
        _ => EscClassification::Other,
    }
}

/// Hands exclusive stdin ownership to interactive prompts while the
/// Esc listener is running.
#[derive(Clone)]
pub struct StdinPromptGate {
    shared: Arc<GateState>,
}

impl StdinPromptGate {
    /// Takes stdin away from the Esc listener for the lifetime of the
    /// returned lease: the terminal returns to canonical mode so a
    /// line-based prompt (e.g. the permission approval prompt) behaves
    /// normally. Listening resumes when the lease is dropped.
    pub fn lease(&self) -> StdinPromptLease<'_> {
        self.shared.busy.store(true, Ordering::SeqCst);
        // Wait for the listener's current poll cycle to release the
        // terminal, then restore canonical mode for line input.
        #[allow(unused_mut)]
        let mut mode = self
            .shared
            .mode
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        #[cfg(unix)]
        tty::restore(&mut mode);
        drop(mode);
        StdinPromptLease {
            shared: &self.shared,
        }
    }
}

/// RAII lease returned by [`StdinPromptGate::lease`].
pub struct StdinPromptLease<'a> {
    shared: &'a GateState,
}

impl Drop for StdinPromptLease<'_> {
    fn drop(&mut self) {
        self.shared.busy.store(false, Ordering::SeqCst);
    }
}

/// Background listener that maps a lone Esc keypress to
/// [`TurnInterruptSignal::interrupt`].
pub struct EscapeInterruptMonitor {
    shared: Arc<GateState>,
    join_handle: Option<JoinHandle<()>>,
}

impl EscapeInterruptMonitor {
    /// Spawns the listener for the duration of one turn. Returns `None`
    /// when stdin is not an interactive terminal or the platform does not
    /// support terminal-mode switching; Ctrl+C interruption still works
    /// in those cases.
    pub fn spawn(signal: TurnInterruptSignal) -> Option<(Self, StdinPromptGate)> {
        if !std::io::stdin().is_terminal() {
            return None;
        }

        #[cfg(not(unix))]
        {
            let _ = signal;
            None
        }

        #[cfg(unix)]
        {
            let shared = Arc::new(GateState::default());
            let thread_shared = Arc::clone(&shared);
            let join_handle = std::thread::spawn(move || listener_loop(&thread_shared, &signal));
            let gate = StdinPromptGate {
                shared: Arc::clone(&shared),
            };
            Some((
                Self {
                    shared,
                    join_handle: Some(join_handle),
                },
                gate,
            ))
        }
    }

    /// Stops the listener and restores the terminal before control
    /// returns to the line editor.
    pub fn stop(mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

#[cfg(unix)]
fn listener_loop(shared: &GateState, signal: &TurnInterruptSignal) {
    let mut buffer = [0u8; 64];

    while !shared.stop.load(Ordering::SeqCst) {
        if shared.busy.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(25));
            continue;
        }

        let mut mode = shared.mode.lock().unwrap_or_else(PoisonError::into_inner);
        if shared.busy.load(Ordering::SeqCst) || shared.stop.load(Ordering::SeqCst) {
            continue;
        }
        if !tty::enable_noncanonical(&mut mode) {
            // Terminal refused the mode switch; Esc support is
            // unavailable for this turn but Ctrl+C still works.
            return;
        }
        if !tty::wait_readable(100) {
            continue;
        }

        let count = tty::read_pending(&mut buffer);
        if classify_escape_chunk(&buffer[..count]) == EscClassification::LoneEscape {
            // A slow terminal may split an escape sequence across reads;
            // only treat ESC as the Esc key when nothing follows it.
            if tty::wait_readable(50) {
                let _ = tty::read_pending(&mut buffer);
                continue;
            }
            signal.interrupt();
        }
        // Other bytes are intentionally swallowed: type-ahead is not
        // preserved while a turn is running.
    }

    let mut mode = shared.mode.lock().unwrap_or_else(PoisonError::into_inner);
    tty::restore(&mut mode);
}

#[cfg(unix)]
mod tty {
    use std::os::fd::{AsFd, AsRawFd};

    use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
    use nix::sys::termios::{self, LocalFlags, SetArg, SpecialCharacterIndices};

    use super::TtyMode;

    /// Disables line buffering and echo while leaving signal generation
    /// (ISIG) and output post-processing (OPOST) untouched, so Ctrl+C and
    /// streamed output keep working. No-op when already active.
    pub(super) fn enable_noncanonical(mode: &mut TtyMode) -> bool {
        if mode.saved.is_some() {
            return true;
        }
        let stdin = std::io::stdin();
        let Ok(saved) = termios::tcgetattr(&stdin) else {
            return false;
        };
        let mut noncanonical = saved.clone();
        noncanonical.local_flags &= !(LocalFlags::ICANON | LocalFlags::ECHO);
        noncanonical.control_chars[SpecialCharacterIndices::VMIN as usize] = 0;
        noncanonical.control_chars[SpecialCharacterIndices::VTIME as usize] = 0;
        if termios::tcsetattr(&stdin, SetArg::TCSANOW, &noncanonical).is_err() {
            return false;
        }
        mode.saved = Some(saved);
        true
    }

    pub(super) fn restore(mode: &mut TtyMode) {
        if let Some(saved) = mode.saved.take() {
            let _ = termios::tcsetattr(std::io::stdin(), SetArg::TCSANOW, &saved);
        }
    }

    pub(super) fn wait_readable(timeout_ms: u16) -> bool {
        let stdin = std::io::stdin();
        let mut fds = [PollFd::new(stdin.as_fd(), PollFlags::POLLIN)];
        matches!(poll(&mut fds, PollTimeout::from(timeout_ms)), Ok(ready) if ready > 0)
            && fds[0]
                .revents()
                .is_some_and(|revents| revents.contains(PollFlags::POLLIN))
    }

    pub(super) fn read_pending(buffer: &mut [u8]) -> usize {
        nix::unistd::read(std::io::stdin().as_raw_fd(), buffer).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::{classify_escape_chunk, EscClassification};

    #[test]
    fn lone_escape_byte_is_the_esc_key() {
        assert_eq!(
            classify_escape_chunk(&[0x1b]),
            EscClassification::LoneEscape
        );
    }

    #[test]
    fn escape_followed_by_more_bytes_is_a_sequence() {
        // Up arrow: ESC [ A
        assert_eq!(
            classify_escape_chunk(&[0x1b, 0x5b, 0x41]),
            EscClassification::EscapeSequence
        );
        // Alt+f style: ESC f
        assert_eq!(
            classify_escape_chunk(&[0x1b, b'f']),
            EscClassification::EscapeSequence
        );
    }

    #[test]
    fn regular_bytes_are_ignored() {
        assert_eq!(classify_escape_chunk(b"hello"), EscClassification::Other);
        assert_eq!(classify_escape_chunk(&[]), EscClassification::Other);
        assert_eq!(classify_escape_chunk(&[0x03]), EscClassification::Other);
    }
}
