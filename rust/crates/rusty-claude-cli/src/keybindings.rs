//! Keybinding abstraction — Action enum + KeyMap with presets (Emacs/Vim/Windows).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Submit,
    Cancel,
    Newline,
    ScrollUp,
    ScrollDown,
    ScrollHalfUp,
    ScrollHalfDown,
    ScrollTop,
    ScrollBottom,
    Exit,
    ProviderSwap,
    TeamToggle,
    CommandPalette,
    ToggleSidebar,
    ToggleAgentView,
    Help,
    ClearConversation,
    FocusInput,
    FocusConversation,
    CycleChatMode,
    /// Run a slash command by index into `commands::slash_command_specs()`.
    RunSlashCommand(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPreset {
    Emacs,
    Vim,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
}

pub struct KeyMap {
    preset: KeyPreset,
    bindings: HashMap<(KeyModifiers, KeyCode), Action>,
    vim_mode: VimMode,
}

impl KeyMap {
    pub fn new(preset: KeyPreset) -> Self {
        let mut map = Self {
            preset,
            bindings: HashMap::new(),
            vim_mode: VimMode::Insert,
        };
        map.load_preset(preset);
        map
    }

    fn load_preset(&mut self, preset: KeyPreset) {
        self.bindings.clear();
        match preset {
            KeyPreset::Emacs => self.load_emacs(),
            KeyPreset::Vim => self.load_emacs(), // Vim overrides handled in resolve()
            KeyPreset::Windows => self.load_windows(),
        }
    }

    fn load_emacs(&mut self) {
        self.bind(KeyModifiers::NONE, KeyCode::Enter, Action::Submit);
        self.bind(KeyModifiers::SHIFT, KeyCode::Enter, Action::Newline);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('c'), Action::Cancel);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('d'), Action::Exit);
        self.bind(
            KeyModifiers::CONTROL,
            KeyCode::Char('p'),
            Action::ProviderSwap,
        );
        self.bind(
            KeyModifiers::CONTROL,
            KeyCode::Char('t'),
            Action::TeamToggle,
        );
        self.bind(
            KeyModifiers::CONTROL,
            KeyCode::Char('k'),
            Action::CommandPalette,
        );
        self.bind(
            KeyModifiers::CONTROL,
            KeyCode::Char('b'),
            Action::ToggleSidebar,
        );
        self.bind(
            KeyModifiers::CONTROL,
            KeyCode::Char('a'),
            Action::ToggleAgentView,
        );
        self.bind(
            KeyModifiers::CONTROL,
            KeyCode::Char('l'),
            Action::ClearConversation,
        );
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('h'), Action::Help);
        self.bind(KeyModifiers::NONE, KeyCode::PageUp, Action::ScrollHalfUp);
        self.bind(
            KeyModifiers::NONE,
            KeyCode::PageDown,
            Action::ScrollHalfDown,
        );
        self.bind(KeyModifiers::CONTROL, KeyCode::Home, Action::ScrollTop);
        self.bind(KeyModifiers::CONTROL, KeyCode::End, Action::ScrollBottom);
        self.bind(KeyModifiers::NONE, KeyCode::F(1), Action::Help);
    }

    fn load_windows(&mut self) {
        self.bind(KeyModifiers::CONTROL, KeyCode::Enter, Action::Submit);
        self.bind(KeyModifiers::NONE, KeyCode::Enter, Action::Newline);
        self.bind(KeyModifiers::NONE, KeyCode::Esc, Action::Cancel);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('d'), Action::Exit);
        self.bind(
            KeyModifiers::CONTROL,
            KeyCode::Char('p'),
            Action::ProviderSwap,
        );
        self.bind(
            KeyModifiers::CONTROL,
            KeyCode::Char('k'),
            Action::CommandPalette,
        );
        self.bind(
            KeyModifiers::CONTROL,
            KeyCode::Char('a'),
            Action::ToggleAgentView,
        );
        self.bind(
            KeyModifiers::CONTROL,
            KeyCode::Char('l'),
            Action::ClearConversation,
        );
        self.bind(KeyModifiers::NONE, KeyCode::F(1), Action::Help);
        self.bind(KeyModifiers::NONE, KeyCode::PageUp, Action::ScrollHalfUp);
        self.bind(
            KeyModifiers::NONE,
            KeyCode::PageDown,
            Action::ScrollHalfDown,
        );
    }

    fn bind(&mut self, modifiers: KeyModifiers, code: KeyCode, action: Action) {
        self.bindings.insert((modifiers, code), action);
    }

    pub fn resolve(&self, key: KeyEvent) -> Option<Action> {
        // Vim normal mode overrides
        if self.preset == KeyPreset::Vim && self.vim_mode == VimMode::Normal {
            return match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Char('j')) => Some(Action::ScrollDown),
                (KeyModifiers::NONE, KeyCode::Char('k')) => Some(Action::ScrollUp),
                (KeyModifiers::NONE, KeyCode::Char('g')) => Some(Action::ScrollTop),
                (KeyModifiers::NONE, KeyCode::Char('G')) => Some(Action::ScrollBottom),
                (KeyModifiers::NONE, KeyCode::Char('i')) => Some(Action::FocusInput),
                (KeyModifiers::NONE, KeyCode::Char(':')) => Some(Action::CommandPalette),
                (KeyModifiers::NONE, KeyCode::Char('q')) => Some(Action::Exit),
                (KeyModifiers::NONE, KeyCode::Esc) => Some(Action::Cancel),
                _ => self.bindings.get(&(key.modifiers, key.code)).copied(),
            };
        }
        self.bindings.get(&(key.modifiers, key.code)).copied()
    }

    pub fn set_vim_mode(&mut self, mode: VimMode) {
        self.vim_mode = mode;
    }

    pub fn vim_mode(&self) -> VimMode {
        self.vim_mode
    }

    pub fn preset(&self) -> KeyPreset {
        self.preset
    }

    pub fn set_preset(&mut self, preset: KeyPreset) {
        self.preset = preset;
        self.load_preset(preset);
        self.vim_mode = VimMode::Insert;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emacs_submit() {
        let km = KeyMap::new(KeyPreset::Emacs);
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(km.resolve(key), Some(Action::Submit));
    }

    #[test]
    fn test_emacs_ctrl_d() {
        let km = KeyMap::new(KeyPreset::Emacs);
        let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!(km.resolve(key), Some(Action::Exit));
    }

    #[test]
    fn test_emacs_ctrl_k() {
        let km = KeyMap::new(KeyPreset::Emacs);
        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL);
        assert_eq!(km.resolve(key), Some(Action::CommandPalette));
    }

    #[test]
    fn test_vim_normal_j() {
        let mut km = KeyMap::new(KeyPreset::Vim);
        km.set_vim_mode(VimMode::Normal);
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        assert_eq!(km.resolve(key), Some(Action::ScrollDown));
    }

    #[test]
    fn test_vim_normal_k() {
        let mut km = KeyMap::new(KeyPreset::Vim);
        km.set_vim_mode(VimMode::Normal);
        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        assert_eq!(km.resolve(key), Some(Action::ScrollUp));
    }

    #[test]
    fn test_vim_normal_i() {
        let mut km = KeyMap::new(KeyPreset::Vim);
        km.set_vim_mode(VimMode::Normal);
        let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
        assert_eq!(km.resolve(key), Some(Action::FocusInput));
    }

    #[test]
    fn test_vim_normal_colon() {
        let mut km = KeyMap::new(KeyPreset::Vim);
        km.set_vim_mode(VimMode::Normal);
        let key = KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE);
        assert_eq!(km.resolve(key), Some(Action::CommandPalette));
    }

    #[test]
    fn test_unbound_key_returns_none() {
        let km = KeyMap::new(KeyPreset::Emacs);
        let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE);
        assert_eq!(km.resolve(key), None);
    }

    #[test]
    fn test_preset_switch() {
        let mut km = KeyMap::new(KeyPreset::Emacs);
        assert_eq!(km.preset(), KeyPreset::Emacs);
        km.set_preset(KeyPreset::Vim);
        assert_eq!(km.preset(), KeyPreset::Vim);
        // Should reset to Insert mode
        assert_eq!(km.vim_mode(), VimMode::Insert);
    }

    #[test]
    fn test_windows_ctrl_enter() {
        let km = KeyMap::new(KeyPreset::Windows);
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL);
        assert_eq!(km.resolve(key), Some(Action::Submit));
    }
}
