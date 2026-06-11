// tui_update.rs — Pure helper functions for TUI mode.
//
// Extracted from main.rs to establish module boundaries.
// LiveCli-dependent code (update_dashboard, run_tui_repl) remains in main.rs
// until LiveCli is made pub(crate).

use std::io::Write;
use std::panic;

use crate::tui::SharedDashboardState;

// ---------------------------------------------------------------------------
// Panic hook — restores terminal state on crash
// ---------------------------------------------------------------------------

/// Install a panic hook that restores terminal state before printing the panic.
/// Returns the previous hook so it can be restored on clean exit.
pub fn install_panic_hook() -> Box<dyn Fn(&panic::PanicHookInfo<'_>) + Send + Sync + 'static> {
    let prev_hook = panic::take_hook();

    panic::set_hook(Box::new(|info| {
        // Best-effort terminal cleanup — ignore errors since we're panicking
        use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), LeaveAlternateScreen);
        let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Show);
        let _ = std::io::stdout().flush();

        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<dyn Any>".to_string()
        };

        let location = info.location()
            .map(|l| format!(" at {}:{}", l.file(), l.line()))
            .unwrap_or_default();

        eprintln!("\n🦀 Claw TUI crashed{location}: {payload}");
        eprintln!("Terminal has been restored to normal mode.\n");
    }));

    prev_hook
}

/// Restore the previous panic hook on clean exit.
pub fn restore_panic_hook(hook: Box<dyn Fn(&panic::PanicHookInfo<'_>) + Send + Sync + 'static>) {
    panic::set_hook(hook);
}

/// Strip ANSI escape sequences from text.
/// Handles CSI (ESC [), OSC (ESC ]...BEL/ST), and DCS/ESC sequences.
pub fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            match chars.peek() {
                Some('[') => {
                    // CSI sequence: ESC [ <params> <intermediate> <final>
                    chars.next(); // consume '['
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        // Final byte: 0x40-0x7e
                        if ('\x40'..='\x7e').contains(&c) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    // OSC sequence: ESC ] ... (BEL | ESC \)
                    chars.next(); // consume ']'
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c == '\x07' {
                            break; // BEL terminator
                        }
                        if c == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next(); // consume '\'
                            break; // ST terminator (ESC \)
                        }
                    }
                }
                Some('P') | Some('_') | Some('^') => {
                    // DCS, APC, PM — terminated by ESC \
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {
                    // Single-char escape (e.g., ESC c)
                    chars.next();
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Data needed to update the dashboard, extracted from LiveCli
/// so this module doesn't depend on main.rs internals.
pub struct DashboardUpdate<'a> {
    pub model: &'a str,
    pub permission_mode: &'a str,
    pub session_id: Option<&'a str>,
    pub turn_count: u32,
    pub total_chars: usize,
    pub provider: String,
    pub provider_url: String,
}

/// Push extracted CLI/runtime stats into the shared dashboard state.
pub fn update_dashboard_from(state: &SharedDashboardState, update: &DashboardUpdate) {
    if let Ok(mut ds) = state.write() {
        ds.model = update.model.to_string();
        ds.permission_mode = update.permission_mode.to_string();
        ds.session_id = update.session_id.map(str::to_string);
        ds.turn_count = update.turn_count;
        // Rough token estimate: ~4 chars per token
        ds.input_tokens = (update.total_chars / 4) as u32;
        ds.provider = update.provider.clone();
        ds.provider_url = update.provider_url.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_basic_csi() {
        assert_eq!(strip_ansi("\x1b[31mred\x1b[0m"), "red");
    }

    #[test]
    fn test_strip_osc_hyperlink() {
        assert_eq!(
            strip_ansi("\x1b]8;;http://example.com\x07link\x1b]8;;\x07"),
            "link"
        );
    }

    #[test]
    fn test_strip_osc_with_st() {
        assert_eq!(
            strip_ansi("\x1b]8;;http://example.com\x1b\\link\x1b]8;;\x1b\\"),
            "link"
        );
    }

    #[test]
    fn test_strip_nested_csi() {
        assert_eq!(strip_ansi("\x1b[1m\x1b[31mbold red\x1b[0m"), "bold red");
    }

    #[test]
    fn test_strip_256_color() {
        assert_eq!(strip_ansi("\x1b[38;5;196mred256\x1b[0m"), "red256");
    }

    #[test]
    fn test_strip_rgb_color() {
        assert_eq!(strip_ansi("\x1b[38;2;255;0;0mrgb\x1b[0m"), "rgb");
    }

    #[test]
    fn test_strip_cursor_movement() {
        assert_eq!(strip_ansi("\x1b[2J\x1b[Hclear"), "clear");
    }

    #[test]
    fn test_strip_dcs() {
        assert_eq!(strip_ansi("\x1bPq$d\x1b\\"), "");
    }

    #[test]
    fn test_strip_no_escapes() {
        assert_eq!(strip_ansi("plain text"), "plain text");
    }

    #[test]
    fn test_strip_empty() {
        assert_eq!(strip_ansi(""), "");
    }

    #[test]
    fn test_strip_malformed_no_final() {
        assert_eq!(strip_ansi("\x1b[31m"), "");
    }

    #[test]
    fn test_strip_mixed() {
        let input = "Hello \x1b[1mWorld\x1b[0m \x1b]8;;url\x07Link\x1b]8;;\x07 end";
        assert_eq!(strip_ansi(input), "Hello World Link end");
    }
}
