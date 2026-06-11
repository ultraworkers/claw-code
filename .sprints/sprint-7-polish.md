# Sprint 7: Polish & Ship

> **Duration:** 3 days | **Stories:** 7 | **Goal:** Mouse, CJK, sidebar, input history, search, final QA
> **Depends on:** Sprint 0–6
> **Guardrails:** Four Laws enforced. Commit after each story. Three Strikes per task.

---

## S7-1: Enable mouse support

**Priority:** P2 — Medium  
**Estimate:** 0.5 day  
**Scope:** IN: `src/tui.rs` (init/restore + event handling). OUT: all other files

### Pre-Execution Checklist
```
[ ] Read src/tui.rs init_terminal() and restore_terminal()
[ ] Read src/tui_repl.rs event loop for Event:: handling
[ ] Scope locked: init/restore + mouse event handler
[ ] Rollback: git checkout HEAD -- src/tui.rs src/tui_repl.rs
```

### Implementation

**File:** `src/tui.rs` — init:
```rust
use crossterm::event::{EnableMouseCapture, DisableMouseCapture};

pub fn init_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        EnableMouseCapture,
        crossterm::terminal::EnterAlternateScreen,
    )?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}
```

Restore:
```rust
pub fn restore_terminal(&mut self) -> io::Result<()> {
    crossterm::execute!(
        self.terminal.backend_mut(),
        DisableMouseCapture,
        crossterm::terminal::LeaveAlternateScreen,
    )?;
    disable_raw_mode()?;
    Ok(())
}
```

**File:** `src/tui_repl.rs` — event handling:
```rust
Event::Mouse(mouse) => {
    match mouse.kind {
        MouseEventKind::ScrollUp => { app.scroll_up(3); }
        MouseEventKind::ScrollDown => { app.scroll_down(3); }
        MouseEventKind::Down(MouseButton::Left) => {
            // Check if click is in input area bounds
            if let Some(input_area) = app.input_area_rect {
                if mouse.column >= input_area.x
                    && mouse.column < input_area.x + input_area.width
                    && mouse.row >= input_area.y
                    && mouse.row < input_area.y + input_area.height
                {
                    app.input_focused = true;
                } else {
                    app.input_focused = false;
                }
            }
        }
        _ => {}
    }
    app.needs_redraw = true;
}
```

### Verification
```bash
cargo build -p rusty-claude-cli
# Manual: launch TUI, scroll with mouse wheel, click to focus input
```

### Commit
```
feat(tui): enable mouse support for scroll and click-to-focus

Authored by TheArchitectit
```

---

## S7-2: CJK-aware word wrapping

**Priority:** P2 — Medium  
**Estimate:** 0.25 day  
**Scope:** IN: `src/tui.rs` (wrap_line function). OUT: all other files

### Implementation

`unicode-width` already added in Sprint 0 (S0-4).

```rust
use unicode_width::UnicodeWidthChar;

fn wrap_line(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 || text.is_empty() {
        return vec![text.to_string()];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width: usize = 0;
    let mut last_break_byte_pos: usize = 0;

    for ch in text.chars() {
        let char_width = ch.width().unwrap_or(0);

        if current_width + char_width > max_width {
            if last_break_byte_pos > 0 {
                let (keep, rest) = current_line.split_at(last_break_byte_pos);
                result.push(keep.trim_end().to_string());
                current_line = rest.to_string();
                current_width = rest.chars().map(|c| c.width().unwrap_or(0)).sum();
                last_break_byte_pos = 0;
            } else {
                result.push(current_line.clone());
                current_line.clear();
                current_width = 0;
                last_break_byte_pos = 0;
            }
        }

        let byte_pos_before = current_line.len();
        current_line.push(ch);
        current_width += char_width;

        if ch == ' ' || ch == '-' || ch == '_' || ch == '/' {
            last_break_byte_pos = current_line.len();
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    if result.is_empty() {
        result.push(String::new());
    }

    result
}
```

### Tests
```rust
#[test]
fn test_wrap_cjk_double_width() {
    let wrapped = wrap_line("你好世界你好世界你好世界", 10);
    for line in &wrapped {
        let width: usize = line.chars().map(|c| c.width().unwrap_or(0)).sum();
        assert!(width <= 10, "line '{line}' has width {width}");
    }
}

#[test]
fn test_wrap_mixed_cjk_ascii() {
    let wrapped = wrap_line("Hello你好World test", 10);
    assert!(!wrapped.is_empty());
}

#[test]
fn test_wrap_emoji() {
    let wrapped = wrap_line("🦀🔨🔧", 6);
    assert!(!wrapped.is_empty());
}
```

### Commit
```
fix(tui): CJK-aware word wrapping with unicode-width

Authored by TheArchitectit
```

---

## S7-3: Refactor draw_frame() into RenderContext

**Priority:** P2 — Medium  
**Estimate:** 0.5 day  
**Scope:** IN: `src/tui.rs` (draw_frame signature). OUT: tui_repl.rs, main.rs

### Pre-Execution Checklist
```
[ ] Read src/tui.rs draw_frame() signature (currently 10 params)
[ ] Read all callers of draw_frame() in tui.rs and tui_repl.rs
[ ] Scope locked: only draw_frame signature + callers
[ ] Rollback: git checkout HEAD -- src/tui.rs
```

### Implementation

```rust
struct RenderContext<'a> {
    theme: &'a TuiTheme,
    dashboard: &'a DashboardState,
    conversation: &'a [ConversationLine],
    conversation_scroll: u16,
    input: &'a TextArea<'static>,
    slash_completions: &'a [String],
    completion_index: usize,
    showing_completions: bool,
    spinner_frame: usize,
    chat_mode: ChatMode,
    agent_view_active: bool,
    command_palette_active: bool,
}

impl<'a> RenderContext<'a> {
    fn from_app(app: &'a TuiApp, dashboard: &'a DashboardState) -> Self {
        Self {
            theme: &app.theme,
            dashboard,
            conversation: &app.conversation,
            conversation_scroll: app.conversation_scroll,
            input: &app.input,
            slash_completions: &app.slash_completions,
            completion_index: app.completion_index,
            showing_completions: app.showing_completions,
            spinner_frame: app.spinner_frame,
            chat_mode: app.chat_mode,
            agent_view_active: app.agent_view.active,
            command_palette_active: app.command_palette.active,
        }
    }
}

// Before:
fn draw_frame(theme: &TuiTheme, dashboard: &DashboardState, conversation: &[ConversationLine],
    scroll: u16, input: &TextArea, completions: &[String], comp_idx: usize,
    showing_comp: bool, spinner: usize, chat_mode: ChatMode,
    f: &mut Frame)
{ ... }

// After:
fn draw_frame(ctx: &RenderContext, f: &mut Frame)
{ ... }
```

### Verification
```bash
cargo build -p rusty-claude-cli
cargo clippy -p rusty-claude-cli  # no more too_many_arguments suppression
```

### Commit
```
refactor(tui): replace 10-param draw_frame with RenderContext struct

Authored by TheArchitectit
```

---

## S7-4: Add input history (Up/Down arrows)

**Priority:** P2 — Medium  
**Estimate:** 0.25 day  
**Scope:** IN: `src/tui.rs` (TuiApp fields + handle_key). OUT: tui_repl.rs

### Implementation

```rust
pub struct TuiApp {
    // ... existing fields
    input_history: Vec<String>,
    history_index: Option<usize>,  // None = not browsing history
}

impl TuiApp {
    fn push_history(&mut self, text: &str) {
        if !text.trim().is_empty() {
            self.input_history.push(text.to_string());
            // Cap at 500 entries
            if self.input_history.len() > 500 {
                self.input_history.remove(0);
            }
        }
        self.history_index = None;
    }

    fn history_up(&mut self) {
        if self.input_history.is_empty() { return; }
        let new_idx = match self.history_index {
            Some(i) => i.saturating_add(1).min(self.input_history.len() - 1),
            None => self.input_history.len() - 1,
        };
        self.history_index = Some(new_idx);
        let entry = &self.input_history[self.input_history.len() - 1 - new_idx];
        self.input = TextArea::new(vec![entry.clone()]);
    }

    fn history_down(&mut self) {
        match self.history_index {
            Some(0) => {
                self.history_index = None;
                self.input = TextArea::new(vec![String::new()]);
            }
            Some(i) => {
                let new_idx = i - 1;
                self.history_index = Some(new_idx);
                let entry = &self.input_history[self.input_history.len() - 1 - new_idx];
                self.input = TextArea::new(vec![entry.clone()]);
            }
            None => {}
        }
    }
}
```

Wire into `handle_key()` when input is focused and not showing completions:
```rust
KeyCode::Up if !self.showing_completions => { self.history_up(); }
KeyCode::Down if !self.showing_completions => { self.history_down(); }
```

Save history on submit in `tui_repl.rs`:
```rust
TuiReadOutcome::Submit(text) => {
    app.push_history(&text);
    // ... existing turn execution
}
```

### Commit
```
feat(tui): add input history with Up/Down arrow navigation

Authored by TheArchitectit
```

---

## S7-5: Add search in conversation (Ctrl+F)

**Priority:** P3 — Low  
**Estimate:** 0.25 day  
**Scope:** IN: `src/tui.rs` (new search state + handle_key). OUT: tui_repl.rs

### Implementation

```rust
pub struct ConversationSearch {
    pub active: bool,
    pub query: String,
    pub matches: Vec<usize>,  // indices into conversation
    pub current_match: usize,
}
```

When `Ctrl+F`:
- Show search input at top of conversation pane
- Type to filter — jump to matching line
- Enter / F3 = next match, Shift+F3 = previous
- Escape = close search

### Commit
```
feat(tui): add conversation search with Ctrl+F

Authored by TheArchitectit
```

---

## S7-6: Final integration testing

**Priority:** P1 — High  
**Estimate:** 0.5 day  
**Scope:** IN: No file changes. Manual testing only.

### Test Matrix

| Feature | Test | Pass? |
|---------|------|-------|
| TUI launch | `/tui` enters TUI mode | |
| Message flow | Send message, receive streaming response | |
| Markdown rendering | Headers, code blocks, lists render | |
| Theme switching | `/theme tokyonight` changes all colors | |
| Keybindings | All Emacs actions work | |
| Vim mode | `/keys vim` + normal/insert modes | |
| Command palette | `Ctrl+K` opens, type to filter, Enter selects | |
| Chat modes | `/code`, `/ask`, `/architect` switch | |
| Diff viewer | `/diff` shows changes | |
| Undo | `/undo --confirm` reverts | |
| Agent View | `Ctrl+A` shows sessions | |
| Provider swap | `Ctrl+P` runs wizard, returns to TUI | |
| Token tracking | Dashboard shows real counts | |
| Memory bound | Long session stays bounded | |
| CJK wrapping | Chinese text wraps at double-width | |
| Mouse scroll | Wheel scrolls conversation | |
| Input history | Up/Down recalls previous inputs | |
| Search | `Ctrl+F` finds text in conversation | |
| Help | `F1` shows keybinding help | |
| Resize | Terminal resize re-renders correctly | |
| Panic recovery | Induced panic restores terminal | |

### Verification
```bash
cargo test -p rusty-claude-cli
cargo clippy -p rusty-claude-cli
cargo build --release -p rusty-claude-cli
```

### Commit (if any test fixes needed)
```
fix(tui): integration test fixes from QA pass

Authored by TheArchitectit
```

---

## S7-7: Update documentation

**Priority:** P2 — Medium  
**Estimate:** 0.25 day  
**Scope:** IN: README.md, help text in tui.rs. OUT: guardrails docs

### Deliverables

1. README section for TUI
2. In-app help (F1) updated with all commands
3. Slash command reference:
```
/tui           Enter TUI mode
/theme <name>  Change color theme
/keys <preset> Change keybinding preset
/code          Code mode (default)
/ask           Ask mode (no edits)
/architect     Architect mode (plan first)
/diff          Show uncommitted changes
/undo          Revert last changes
/ls            List context files
```

### Commit
```
docs(tui): add TUI mode documentation and command reference

Authored by TheArchitectit
```

---

## Sprint 7 Definition of Done
- [ ] All 7 stories completed and committed individually
- [ ] All test matrix items pass
- [ ] Zero hardcoded `Color::` in rendering code
- [ ] `cargo test -p rusty-claude-cli` passes
- [ ] `cargo clippy -p rusty-claude-cli` clean
- [ ] `cargo build --release -p rusty-claude-cli` succeeds
- [ ] Manual smoke test: full workflow end-to-end
