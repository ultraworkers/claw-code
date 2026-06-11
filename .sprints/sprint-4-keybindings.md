# Sprint 4: Keybindings & Command Palette

> **Duration:** 4 days (up from 3) | **Stories:** 5 | **Goal:** UX parity with Claude Code and OpenCode
> **Depends on:** Sprint 0–3

---

## S4-1: Create Action enum and KeyMap module

**Priority:** P1 — High  
**Estimate:** 0.5 day  

### Implementation

**New file:** `src/keybindings.rs`

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // Input
    Submit,
    Cancel,
    Newline,
    // Navigation
    ScrollUp,
    ScrollDown,
    ScrollHalfUp,
    ScrollHalfDown,
    ScrollTop,
    ScrollBottom,
    // TUI controls
    Exit,
    ProviderSwap,
    TeamToggle,
    CommandPalette,
    ToggleSidebar,
    ToggleAgentView,
    Help,
    // Session
    NewSession,
    ClearConversation,
    CopyLastOutput,
    // Focus
    FocusInput,
    FocusConversation,
    // Chat mode
    CycleChatMode,
    // Completion
    AcceptCompletion,
    NextCompletion,
    PrevCompletion,
    DismissCompletion,
    // Slash commands (for palette dispatch)
    ThemeCommand,
    DiffCommand,
    UndoCommand,
    LsCommand,
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
            KeyPreset::Vim => self.load_vim(),
            KeyPreset::Windows => self.load_windows(),
        }
    }

    fn load_emacs(&mut self) {
        self.bind(KeyModifiers::NONE, KeyCode::Enter, Action::Submit);
        self.bind(KeyModifiers::SHIFT, KeyCode::Enter, Action::Newline);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('c'), Action::Cancel);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('d'), Action::Exit);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('p'), Action::ProviderSwap);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('t'), Action::TeamToggle);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('k'), Action::CommandPalette);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('b'), Action::ToggleSidebar);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('a'), Action::ToggleAgentView);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('l'), Action::ClearConversation);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('h'), Action::Help);
        self.bind(KeyModifiers::NONE, KeyCode::PageUp, Action::ScrollHalfUp);
        self.bind(KeyModifiers::NONE, KeyCode::PageDown, Action::ScrollHalfDown);
        self.bind(KeyModifiers::CONTROL, KeyCode::Home, Action::ScrollTop);
        self.bind(KeyModifiers::CONTROL, KeyCode::End, Action::ScrollBottom);
        // FIX: bind() takes 3 args — (modifiers, code, action)
        self.bind(KeyModifiers::NONE, KeyCode::F(1), Action::Help);
    }

    fn load_vim(&mut self) {
        self.load_emacs(); // Base layer — Vim normal mode overrides in resolve()
    }

    fn load_windows(&mut self) {
        self.bind(KeyModifiers::CONTROL, KeyCode::Enter, Action::Submit);
        self.bind(KeyModifiers::NONE, KeyCode::Enter, Action::Newline);
        self.bind(KeyModifiers::NONE, KeyCode::Esc, Action::Cancel);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('d'), Action::Exit);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('p'), Action::ProviderSwap);
        self.bind(KeyModifiers::CONTROL, KeyCode::Char('k'), Action::CommandPalette);
        self.bind(KeyModifiers::NONE, KeyCode::F(1), Action::Help);
        // ... same pattern for remaining bindings
    }

    fn bind(&mut self, modifiers: KeyModifiers, code: KeyCode, action: Action) {
        self.bindings.insert((modifiers, code), action);
    }

    pub fn resolve(&self, key: KeyEvent) -> Option<Action> {
        if self.preset == KeyPreset::Vim && self.vim_mode == VimMode::Normal {
            return self.resolve_vim_normal(key);
        }
        self.bindings.get(&(key.modifiers, key.code)).copied()
    }

    fn resolve_vim_normal(&self, key: KeyEvent) -> Option<Action> {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('j')) => Some(Action::ScrollDown),
            (KeyModifiers::NONE, KeyCode::Char('k')) => Some(Action::ScrollUp),
            (KeyModifiers::NONE, KeyCode::Char('g')) => Some(Action::ScrollTop),
            (KeyModifiers::NONE, KeyCode::Char('G')) => Some(Action::ScrollBottom),
            (KeyModifiers::NONE, KeyCode::Char('i')) => Some(Action::FocusInput),
            (KeyModifiers::NONE, KeyCode::Char(':')) => Some(Action::CommandPalette),
            (KeyModifiers::NONE, KeyCode::Char('q')) => Some(Action::Exit),
            (KeyModifiers::NONE, KeyCode::Esc) => Some(Action::Cancel),
            _ => self.bindings.get(&(key.modifiers, key.code)).copied(),
        }
    }

    pub fn set_vim_mode(&mut self, mode: VimMode) { self.vim_mode = mode; }
    pub fn vim_mode(&self) -> VimMode { self.vim_mode }
    pub fn preset(&self) -> KeyPreset { self.preset }
    pub fn set_preset(&mut self, preset: KeyPreset) {
        self.preset = preset;
        self.load_preset(preset);
        self.vim_mode = VimMode::Insert;
    }
}
```

### Acceptance Criteria
- [ ] `Action` enum covers all actions including `ThemeCommand`, `DiffCommand`, etc.
- [ ] `bind(KeyModifiers::NONE, KeyCode::F(1), Action::Help)` — correct 3-arg signature
- [ ] Vim normal mode: j/k/g/G/i/:/q work
- [ ] Esc enters Vim normal mode from insert

---

## S4-2: Replace hardcoded key handling with Action dispatch

**Priority:** P1 — High  
**Estimate:** 0.5 day  

### Implementation

**File:** `src/tui.rs`

```rust
fn handle_key(&mut self, key: KeyEvent) -> io::Result<TuiReadOutcome> {
    // Vim mode transition: Esc → Normal mode
    if self.keymap.preset() == KeyPreset::Vim && key.code == KeyCode::Esc {
        if self.keymap.vim_mode() == VimMode::Insert {
            self.keymap.set_vim_mode(VimMode::Normal);
            return Ok(TuiReadOutcome::Pending);
        }
    }

    // Resolve key → action
    let action = self.keymap.resolve(key);

    // In insert mode, many keys go to the text area
    if self.input_focused && self.keymap.vim_mode() == VimMode::Insert {
        match action {
            Some(Action::Submit) => {
                let text: String = self.input.lines().join("\n");
                if text.trim().is_empty() { return Ok(TuiReadOutcome::Pending); }
                self.input = TextArea::new(vec![String::new()]);
                return Ok(TuiReadOutcome::Submit(text));
            }
            Some(Action::Cancel) => return Ok(TuiReadOutcome::Cancel),
            Some(Action::Newline) => {
                self.input.insert_newline();
                return Ok(TuiReadOutcome::Pending);
            }
            Some(Action::Exit) => {
                self.should_exit = true;
                return Ok(TuiReadOutcome::Exit);
            }
            Some(action) => return self.dispatch_action(action),
            None => {
                // Vim i key enters insert mode
                if self.keymap.preset() == KeyPreset::Vim && key.code == KeyCode::Char('i') {
                    self.keymap.set_vim_mode(VimMode::Insert);
                    return Ok(TuiReadOutcome::Pending);
                }
                // Unbound key → pass to text area for editing
                self.input.input(key);
                return Ok(TuiReadOutcome::Pending);
            }
        }
    }

    match action {
        Some(action) => self.dispatch_action(action),
        None => Ok(TuiReadOutcome::Pending),
    }
}

fn dispatch_action(&mut self, action: Action) -> io::Result<TuiReadOutcome> {
    match action {
        Action::ScrollUp => { self.scroll_up(1); }
        Action::ScrollDown => { self.scroll_down(1); }
        Action::ScrollHalfUp => { self.scroll_up(self.visible_lines() / 2); }
        Action::ScrollHalfDown => { self.scroll_down(self.visible_lines() / 2); }
        Action::ScrollTop => { self.scroll_to_top(); }
        Action::ScrollBottom => { self.scroll_to_bottom(); }
        Action::Exit => { self.should_exit = true; return Ok(TuiReadOutcome::Exit); }
        Action::ProviderSwap => return Ok(TuiReadOutcome::ProviderSwap),
        Action::TeamToggle => return Ok(TuiReadOutcome::TeamToggle),
        Action::CommandPalette => { self.toggle_command_palette(); }
        Action::ToggleSidebar => { self.toggle_sidebar(); }
        Action::ToggleAgentView => return Ok(TuiReadOutcome::AgentView),
        Action::Help => { self.show_help(); }
        Action::ClearConversation => {
            self.conversation.clear();
            self.render_cache.clear();
            self.conversation_scroll = 0;
        }
        Action::CycleChatMode => { self.cycle_chat_mode(); }
        Action::FocusInput => { self.input_focused = true; self.keymap.set_vim_mode(VimMode::Insert); }
        Action::FocusConversation => { self.input_focused = false; }
        // Slash command actions dispatched to TuiCommand
        Action::ThemeCommand | Action::DiffCommand | Action::UndoCommand | Action::LsCommand => {
            // These are handled via slash commands, not direct key dispatch
        }
        _ => {}
    }
    Ok(TuiReadOutcome::Pending)
}
```

### Acceptance Criteria
- [ ] All existing keybindings still work
- [ ] `handle_key()` dispatches through `KeyMap`
- [ ] No `KeyCode::Char('x')` matches remain in `handle_key()`
- [ ] Vim mode: Esc → Normal, i → Insert

---

## S4-3: Implement Command Palette (Ctrl+K)

**Priority:** P1 — High  
**Estimate:** 1.5 days (up from 1 — Opus noted modal overlay + fuzzy filter complexity)

### Implementation

**New file:** `src/command_palette.rs`

```rust
use crate::keybindings::Action;

pub struct CommandPalette {
    pub active: bool,
    pub query: String,
    pub entries: Vec<PaletteEntry>,
    pub filtered: Vec<usize>,
    pub selected: usize,
}

#[derive(Debug, Clone)]
pub struct PaletteEntry {
    pub label: String,
    pub description: String,
    pub action: Action,
    pub key_hint: String,
    pub category: String,
}

impl CommandPalette {
    pub fn new() -> Self {
        let entries = vec![
            PaletteEntry { label: "Submit".into(), description: "Send message".into(),
                action: Action::Submit, key_hint: "Enter".into(), category: "Input".into() },
            PaletteEntry { label: "Swap Provider".into(), description: "Change AI provider".into(),
                action: Action::ProviderSwap, key_hint: "Ctrl+P".into(), category: "Settings".into() },
            PaletteEntry { label: "Toggle Team".into(), description: "Show/hide team info".into(),
                action: Action::TeamToggle, key_hint: "Ctrl+T".into(), category: "View".into() },
            PaletteEntry { label: "Agent View".into(), description: "Multi-session dashboard".into(),
                action: Action::ToggleAgentView, key_hint: "Ctrl+A".into(), category: "View".into() },
            PaletteEntry { label: "Toggle Sidebar".into(), description: "Show/hide file sidebar".into(),
                action: Action::ToggleSidebar, key_hint: "Ctrl+B".into(), category: "View".into() },
            PaletteEntry { label: "Clear Conversation".into(), description: "Clear all messages".into(),
                action: Action::ClearConversation, key_hint: "Ctrl+L".into(), category: "Session".into() },
            PaletteEntry { label: "Help".into(), description: "Keyboard shortcuts".into(),
                action: Action::Help, key_hint: "F1".into(), category: "Help".into() },
            PaletteEntry { label: "Exit".into(), description: "Exit TUI mode".into(),
                action: Action::Exit, key_hint: "Ctrl+D".into(), category: "Session".into() },
            // FIX: Use correct Action variants for slash commands
            PaletteEntry { label: "/theme".into(), description: "Change color theme".into(),
                action: Action::ThemeCommand, key_hint: "".into(), category: "Commands".into() },
            PaletteEntry { label: "/diff".into(), description: "Show uncommitted changes".into(),
                action: Action::DiffCommand, key_hint: "".into(), category: "Commands".into() },
            PaletteEntry { label: "/undo".into(), description: "Revert last changes".into(),
                action: Action::UndoCommand, key_hint: "".into(), category: "Commands".into() },
            PaletteEntry { label: "/code".into(), description: "Code mode (full access)".into(),
                action: Action::CycleChatMode, key_hint: "Tab".into(), category: "Modes".into() },
        ];

        let filtered = (0..entries.len()).collect();
        Self { active: false, query: String::new(), entries, filtered, selected: 0 }
    }

    pub fn open(&mut self) { self.active = true; self.query.clear(); self.filtered = (0..self.entries.len()).collect(); self.selected = 0; }
    pub fn close(&mut self) { self.active = false; self.query.clear(); }
    pub fn input(&mut self, c: char) { self.query.push(c); self.update_filter(); }
    pub fn backspace(&mut self) { self.query.pop(); self.update_filter(); }
    pub fn select_next(&mut self) { if !self.filtered.is_empty() { self.selected = (self.selected + 1) % self.filtered.len(); } }
    pub fn select_prev(&mut self) { if !self.filtered.is_empty() { self.selected = if self.selected == 0 { self.filtered.len() - 1 } else { self.selected - 1 }; } }
    pub fn selected_action(&self) -> Option<Action> { self.filtered.get(self.selected).map(|&i| self.entries[i].action) }

    fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = (0..self.entries.len()).collect();
        } else {
            let q = self.query.to_lowercase();
            self.filtered = self.entries.iter().enumerate()
                .filter(|(_, e)| e.label.to_lowercase().contains(&q) || e.description.to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect();
        }
        self.selected = self.selected.min(self.filtered.len().saturating_sub(1));
    }
}
```

### Rendering

**File:** `src/tui.rs`

```rust
fn draw_command_palette(&self, f: &mut Frame, area: Rect) {
    if !self.command_palette.active { return; }

    let popup_w = (area.width * 60 / 100).min(60);
    let popup_h = (area.height * 50 / 100).min(20);
    let popup = Rect::new(
        (area.width - popup_w) / 2,
        (area.height - popup_h) / 2,
        popup_w, popup_h,
    );

    f.render_widget(ratatui::widgets::Clear, popup);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("🔍 ", Style::default().fg(self.tc(&self.theme.key_hint))),
        Span::styled(&self.command_palette.query, Style::default().fg(self.tc(&self.theme.input_fg))),
        Span::styled("█", Style::default().fg(self.tc(&self.theme.input_cursor_bg))),
    ]));
    lines.push(Line::from(""));

    for (i, &idx) in self.command_palette.filtered.iter().enumerate() {
        let entry = &self.command_palette.entries[idx];
        let is_sel = i == self.command_palette.selected;
        let (fg, bg) = if is_sel {
            (self.tc(&self.theme.completion_selected_fg), self.tc(&self.theme.completion_selected_bg))
        } else {
            (self.tc(&self.theme.completion_fg), self.tc(&self.theme.completion_bg))
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", entry.label), Style::default().fg(fg).bg(bg)),
            Span::styled(format!("  {}  ", entry.description), Style::default().fg(self.tc(&self.theme.key_hint))),
            Span::styled(&entry.key_hint, Style::default().fg(self.tc(&self.theme.key_hint))),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(self.tc(&self.theme.border_active)))
        .title(" Command Palette ");

    f.render_widget(Paragraph::new(lines).block(block), popup);
}
```

### Acceptance Criteria
- [ ] `Ctrl+K` opens command palette
- [ ] Typing filters commands in real-time
- [ ] Arrow keys navigate, Enter executes
- [ ] Escape closes palette
- [ ] All palette entries use correct `Action` variants (not `CycleChatMode` for everything)
- [ ] Palette uses theme colors, not hardcoded `Color::`

---

## S4-4: Add `/keybindings` command and preset switching

**Priority:** P2 — Medium  
**Estimate:** 0.25 day  

### Implementation

```rust
TuiCommand::Keys(preset_name) => {
    match preset_name.as_str() {
        "emacs" => { app.set_key_preset(KeyPreset::Emacs); app.push_system_message("Keys: Emacs"); }
        "vim" => { app.set_key_preset(KeyPreset::Vim); app.push_system_message("Keys: Vim — i for insert, Esc for normal"); }
        "windows" => { app.set_key_preset(KeyPreset::Windows); app.push_system_message("Keys: Windows"); }
        "" => {
            let cur = app.key_preset_name();
            app.push_system_message(&format!("Current: {cur}\nAvailable: emacs, vim, windows\nUsage: /keys <preset>"));
        }
        _ => { app.push_system_message(&format!("Unknown: {preset_name}. Available: emacs, vim, windows")); }
    }
}
```

### Acceptance Criteria
- [ ] `/keys vim` switches to Vim
- [ ] `/keys emacs` switches to Emacs
- [ ] `/keys` shows current preset and options

---

## S4-5: Implement Help overlay (F1)

**Priority:** P2 — Medium  
**Estimate:** 0.25 day  

### Implementation

```rust
fn show_help(&mut self) {
    let preset = self.keymap.preset();
    let mut msg = format!("Keybindings ({preset:?}):\n\n");
    msg += "Enter       Submit\n";
    msg += "Shift+Enter Newline\n";
    msg += "Ctrl+C      Cancel\n";
    msg += "Ctrl+D      Exit TUI\n";
    msg += "Ctrl+P      Swap provider\n";
    msg += "Ctrl+K      Command palette\n";
    msg += "Ctrl+A      Agent view\n";
    msg += "Ctrl+B      Toggle sidebar\n";
    msg += "Ctrl+L      Clear conversation\n";
    msg += "F1          This help\n";
    msg += "PageUp/Down Scroll\n";
    if preset == KeyPreset::Vim {
        msg += "\nVim mode:\n  i     Insert\n  Esc   Normal\n  j/k   Scroll\n  g/G   Top/Bottom\n  :     Command palette\n";
    }
    msg += "\nSlash commands:\n";
    msg += "/tui /theme /keys /code /ask /architect\n";
    msg += "/diff /undo /ls /help\n";
    self.push_system_message(&msg);
}
```

### Acceptance Criteria
- [ ] F1 shows help with current preset's bindings
- [ ] Vim-specific help only in Vim mode
- [ ] Slash command reference included

---

## Sprint 4 Definition of Done

- [ ] All 5 stories completed
- [ ] `Action` enum + `KeyMap` module with correct signatures
- [ ] Command palette works with correct `Action` dispatch
- [ ] 3 keybinding presets available
- [ ] All palette entries use correct Action variants
- [ ] All colors from theme (no hardcoded `Color::`)
- [ ] `cargo test -p rusty-claude-cli` passes
