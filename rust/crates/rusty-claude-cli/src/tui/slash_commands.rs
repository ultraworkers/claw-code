//! Slash command dispatcher.
//!
//! Extracts the inline slash-command handling from `main.rs::run_tui_repl()`
//! into a testable, decoupled dispatcher.

use crate::chat_mode::ChatMode;
use crate::keybindings::KeyPreset;

/// Result of processing a slash command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommandResult {
    /// Command was handled locally; do not send to the model.
    Handled,
    /// Command should be sent to the model as a regular message.
    SendToModel,
    /// Exit the TUI.
    Exit,
    /// Provider swap wizard requested.
    ProviderSwap,
}

/// Try to parse a slash command from input.
///
/// Returns `None` if the input is not a slash command.
pub fn parse_slash_command(input: &str) -> Option<(&str, &str)> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }
    let space = trimmed.find(' ').unwrap_or(trimmed.len());
    let command = &trimmed[..space];
    let args = trimmed[space..].trim();
    Some((command, args))
}

/// Dispatch a slash command.
///
/// Returns a `SlashCommandResult` indicating what should happen next.
/// The `ctx` callback is called with system messages and other UI updates.
///
/// This is a pure function that doesn't depend on TuiApp or LiveCli —
/// side effects (pushing messages, changing theme) are delegated to the
/// caller through the return value and optional context.
pub fn dispatch_slash_command<'a>(input: &'a str) -> SlashCommandAction<'a> {
    let Some((command, args)) = parse_slash_command(input) else {
        return SlashCommandAction::NotACommand;
    };

    match command {
        "/exit" | "/quit" => SlashCommandAction::Exit,
        "/theme" => SlashCommandAction::SetTheme { name: args },
        "/keys" => SlashCommandAction::SetKeymap { preset: args },
        "/code" => SlashCommandAction::SetChatMode {
            mode: ChatMode::Code,
        },
        "/ask" => SlashCommandAction::SetChatMode {
            mode: ChatMode::Ask,
        },
        "/architect" | "/arch" => SlashCommandAction::SetChatMode {
            mode: ChatMode::Architect,
        },
        "/diff" => SlashCommandAction::ShowDiff,
        "/undo" => SlashCommandAction::Undo {
            confirm: args == "--confirm" || args == "-y",
        },
        "/ls" => SlashCommandAction::ShowFile { path: args },
        "/help" => SlashCommandAction::ShowHelp,
        _ => SlashCommandAction::Unknown { command },
    }
}

/// Actions that a slash command can trigger.
#[derive(Debug, Clone, PartialEq)]
pub enum SlashCommandAction<'a> {
    /// Not a slash command — send to the model.
    NotACommand,
    /// Exit the TUI.
    Exit,
    /// Change theme.
    SetTheme { name: &'a str },
    /// Change keymap preset.
    SetKeymap { preset: &'a str },
    /// Change chat mode.
    SetChatMode { mode: ChatMode },
    /// Show git diff.
    ShowDiff,
    /// Undo (revert uncommitted changes).
    Undo { confirm: bool },
    /// Show file contents.
    ShowFile { path: &'a str },
    /// Show help.
    ShowHelp,
    /// Unknown command.
    Unknown { command: &'a str },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_slash_command() {
        assert_eq!(
            parse_slash_command("/theme tokyonight"),
            Some(("/theme", "tokyonight"))
        );
        assert_eq!(parse_slash_command("/keys"), Some(("/keys", "")));
        assert_eq!(parse_slash_command("/help me"), Some(("/help", "me")));
        assert_eq!(parse_slash_command("not a command"), None);
        assert_eq!(parse_slash_command(""), None);
    }

    #[test]
    fn test_dispatch_exit() {
        assert_eq!(dispatch_slash_command("/exit"), SlashCommandAction::Exit);
        assert_eq!(dispatch_slash_command("/quit"), SlashCommandAction::Exit);
    }

    #[test]
    fn test_dispatch_theme() {
        let result = dispatch_slash_command("/theme tokyonight");
        assert_eq!(result, SlashCommandAction::SetTheme { name: "tokyonight" });
    }

    #[test]
    fn test_dispatch_keys() {
        let result = dispatch_slash_command("/keys vim");
        assert_eq!(result, SlashCommandAction::SetKeymap { preset: "vim" });
    }

    #[test]
    fn test_dispatch_chat_mode() {
        assert_eq!(
            dispatch_slash_command("/code"),
            SlashCommandAction::SetChatMode {
                mode: ChatMode::Code
            }
        );
        assert_eq!(
            dispatch_slash_command("/ask"),
            SlashCommandAction::SetChatMode {
                mode: ChatMode::Ask
            }
        );
        assert_eq!(
            dispatch_slash_command("/architect"),
            SlashCommandAction::SetChatMode {
                mode: ChatMode::Architect
            }
        );
        assert_eq!(
            dispatch_slash_command("/arch"),
            SlashCommandAction::SetChatMode {
                mode: ChatMode::Architect
            }
        );
    }

    #[test]
    fn test_dispatch_diff() {
        assert_eq!(
            dispatch_slash_command("/diff"),
            SlashCommandAction::ShowDiff
        );
    }

    #[test]
    fn test_dispatch_undo() {
        assert_eq!(
            dispatch_slash_command("/undo"),
            SlashCommandAction::Undo { confirm: false }
        );
        assert_eq!(
            dispatch_slash_command("/undo --confirm"),
            SlashCommandAction::Undo { confirm: true }
        );
        assert_eq!(
            dispatch_slash_command("/undo -y"),
            SlashCommandAction::Undo { confirm: true }
        );
    }

    #[test]
    fn test_dispatch_ls() {
        assert_eq!(
            dispatch_slash_command("/ls /some/path"),
            SlashCommandAction::ShowFile { path: "/some/path" }
        );
    }

    #[test]
    fn test_dispatch_help() {
        assert_eq!(
            dispatch_slash_command("/help"),
            SlashCommandAction::ShowHelp
        );
    }

    #[test]
    fn test_dispatch_unknown() {
        let result = dispatch_slash_command("/unknown-command");
        assert!(matches!(result, SlashCommandAction::Unknown { .. }));
    }

    #[test]
    fn test_dispatch_not_a_command() {
        assert_eq!(
            dispatch_slash_command("hello world"),
            SlashCommandAction::NotACommand
        );
    }
}
