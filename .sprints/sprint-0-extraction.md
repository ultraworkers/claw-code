# Sprint 0: TUI Extraction & Architecture

> **Duration:** 4 days | **Stories:** 5 | **Goal:** Extract TUI from 20K-line main.rs, establish panic recovery, clean module boundaries

---

## Why This Sprint Exists

`main.rs` is **20,020 lines**. TUI code is embedded at line 7146. Every subsequent sprint adds slash commands and event handling to this file. Without extraction first:
- Merge conflicts between sprints are guaranteed
- Code review is impossible (who can review a 20K file?)
- Testing individual modules is blocked

This sprint creates the foundation all other sprints build on.

---

## S0-1: Extract TUI integration from main.rs

**Priority:** P0 — Critical  
**Assignee:** —  
**Estimate:** 1.5 days  

### Description
Move all TUI-specific code from `main.rs` into dedicated modules. Create a clean interface boundary.

### New Files

**`src/tui_repl.rs`** — TUI event loop and integration:
```rust
use crate::tui::{self, TuiApp, TuiReadOutcome, SharedDashboardState};
use crate::tui_commands::TuiCommand;

pub struct TuiRepl;

impl TuiRepl {
    /// Entry point called from main.rs when `/tui` is typed.
    pub fn run(cli: &mut LiveCli) -> Result<(), Box<dyn std::error::Error>> {
        let dashboard_state = SharedDashboardState::default();
        let mut app = TuiApp::init(dashboard_state.clone())?;
        
        app.push_banner(&[
            tui::BannerLine { text: "🦀 Claw Code".to_string(), color: ratatui::style::Color::Cyan },
        ]);
        app.push_banner(&[
            tui::BannerLine { text: format_connected_line(&cli.model), color: ratatui::style::Color::DarkGray },
        ]);
        
        let mut pending_input: Option<String> = None;
        
        loop {
            // ... event loop (currently at main.rs:7159-7260)
            // ...
        }
        
        app.restore_terminal()?;
        Ok(())
    }
}
```

**`src/tui_commands.rs`** — Slash command dispatch for TUI mode:
```rust
pub enum TuiCommand {
    Theme(String),        // /theme <name>
    Keys(String),         // /keys <preset>
    Code,                 // /code
    Ask,                  // /ask
    Architect,            // /architect
    Diff { staged: bool },// /diff [--staged]
    Undo { confirm: bool },// /undo [--confirm]
    Ls { path: Option<String> }, // /ls [path]
    Context,              // /context
    Help,                 // /help
}

impl TuiCommand {
    pub fn parse(input: &str) -> Option<Self> { ... }
    pub fn execute(&self, app: &mut TuiApp, cli: &mut LiveCli) -> Result<(), Error> { ... }
}
```

**`src/tui_update.rs`** — Dashboard state updates:
```rust
pub fn update_dashboard(state: &SharedDashboardState, cli: &LiveCli) { ... }
pub fn format_connected_line(model: &str) -> String { ... }
pub fn strip_ansi(text: &str) -> String { ... }  // moved from main.rs
```

### Implementation Steps

1. Create `src/tui_repl.rs` — move `run_tui_repl()` body (main.rs:7146-7262)
2. Create `src/tui_commands.rs` — extract slash command parsing and execution
3. Create `src/tui_update.rs` — move `update_dashboard()`, `strip_ansi()`, `format_connected_line()`
4. Update `main.rs` — replace `run_tui_repl()` with delegation:
   ```rust
   fn run_tui_repl(cli: LiveCli) -> Result<(), Box<dyn std::error::Error>> {
       tui_repl::TuiRepl::run(cli)
   }
   ```
   Or remove entirely and call `TuiRepl::run()` directly.
5. Add `mod` declarations to `main.rs`:
   ```rust
   mod tui_repl;
   mod tui_commands;
   mod tui_update;
   ```
6. Verify `cargo build` succeeds
7. Verify `cargo test` still passes

### Acceptance Criteria
- [ ] `run_tui_repl()` body moved to `src/tui_repl.rs`
- [ ] Slash command handling moved to `src/tui_commands.rs`
- [ ] `update_dashboard()` and helpers moved to `src/tui_update.rs`
- [ ] `main.rs` reduced by ~200+ lines
- [ ] `cargo build --release` succeeds
- [ ] `cargo test -p rusty-claude-cli` passes
- [ ] `/tui` command still works in the REPL
- [ ] No behavioral changes (pure extraction)

---

## S0-2: Add panic hook for terminal cleanup

**Priority:** P0 — Critical  
**Assignee:** —  
**Estimate:** 0.5 day  

### Description
If any TUI code panics, the terminal is left in raw mode with hidden cursor and alternate screen. The user sees a broken terminal and must manually run `reset`. Add a panic hook that restores terminal state before printing the panic message.

### Implementation

**File:** `src/tui_repl.rs`

```rust
use std::panic;
use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
use crossterm::execute;

/// Install a panic hook that restores terminal state before printing the panic.
/// Returns the previous hook so it can be restored on clean exit.
pub fn install_panic_hook() -> Box<dyn Fn(&panic::PanicInfo<'_>) + Send + Sync + 'static> {
    let prev_hook = panic::take_hook();
    
    panic::set_hook(Box::new(|info| {
        // Best-effort terminal cleanup — ignore errors since we're panicking
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        let _ = execute!(std::io::stdout(), crossterm::cursor::Show);
        
        // Print the panic message
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
pub fn restore_panic_hook(hook: Box<dyn Fn(&panic::PanicInfo<'_>) + Send + Sync + 'static>) {
    panic::set_hook(hook);
}
```

Usage in `TuiRepl::run()`:
```rust
pub fn run(cli: &mut LiveCli) -> Result<(), Box<dyn std::error::Error>> {
    let prev_hook = install_panic_hook();
    
    // ... TUI event loop ...
    
    restore_panic_hook(prev_hook);
    app.restore_terminal()?;
    Ok(())
}
```

### Acceptance Criteria
- [ ] Panic hook installed at TUI startup
- [ ] Previous hook restored on clean exit
- [ ] If TUI panics: terminal is restored, panic message printed, terminal usable
- [ ] No double-panic (hook itself doesn't panic)
- [ ] `disable_raw_mode()` errors are silently ignored (best-effort)

### Tests
```rust
#[test]
fn test_panic_hook_installs_and_restores() {
    // Verify hook can be installed and restored without panic
    let prev = install_panic_hook();
    restore_panic_hook(prev);
}
```

Manual test: induce a panic in TUI code (e.g., `panic!("test")`), verify terminal is usable after.

---

## S0-3: Add terminal resize handling

**Priority:** P1 — High  
**Assignee:** —  
**Estimate:** 0.5 day  

### Description
When the terminal is resized, the TUI must re-render with the new dimensions. Currently, resize events are ignored, causing layout corruption.

### Implementation

**File:** `src/tui_repl.rs` — in the event loop:

```rust
Event::Resize(width, height) => {
    // ratatui::Terminal automatically picks up the new size on next
    // draw call via `f.area()`. We just need to force a redraw and
    // re-wrap the conversation.
    app.mark_resize(width, height);
}
```

**File:** `src/tui.rs` — add to `TuiApp`:

```rust
pub fn mark_resize(&mut self, _width: u16, _height: u16) {
    // Clear cached wrapped lines so they get re-wrapped at new width
    self.invalidate_render_cache();
    self.needs_redraw = true;
}
```

### Acceptance Criteria
- [ ] Resizing terminal triggers re-render
- [ ] Word-wrapping adjusts to new width
- [ ] Dashboard panel stays correctly sized
- [ ] Input area resizes appropriately
- [ ] No layout corruption after resize

### Tests
Manual: launch TUI, resize terminal repeatedly, verify layout adapts.

---

## S0-4: Add `unicode-width` dependency

**Priority:** P1 — High  
**Assignee:** —  
**Estimate:** 0.1 day  

### Description
Add the `unicode-width` crate to `Cargo.toml` for CJK-aware character width calculations. Needed by Sprint 7's word-wrapping fix, but adding now avoids a dependency change mid-sprint.

### Implementation

**File:** `rust/crates/rusty-claude-cli/Cargo.toml`

```toml
[dependencies]
# ... existing deps ...
unicode-width = "0.2"
```

### Acceptance Criteria
- [ ] `cargo build` succeeds with new dependency
- [ ] `unicode_width::UnicodeWidthChar` available in the crate

---

## S0-5: Define shared TUI error type

**Priority:** P2 — Medium  
**Assignee:** —  
**Estimate:** 0.25 day  

### Description
Currently, TUI functions return `Box<dyn std::error::Error>`. Define a proper error type for better error handling across modules.

### Implementation

**New file:** `src/tui_error.rs`

```rust
use std::fmt;

#[derive(Debug)]
pub enum TuiError {
    Io(std::io::Error),
    Terminal(String),
    Runtime(String),
    Config(String),
}

impl fmt::Display for TuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TuiError::Io(e) => write!(f, "TUI I/O error: {e}"),
            TuiError::Terminal(msg) => write!(f, "Terminal error: {msg}"),
            TuiError::Runtime(msg) => write!(f, "Runtime error: {msg}"),
            TuiError::Config(msg) => write!(f, "Config error: {msg}"),
        }
    }
}

impl std::error::Error for TuiError {}

impl From<std::io::Error> for TuiError {
    fn from(e: std::io::Error) -> Self {
        TuiError::Io(e)
    }
}
```

### Acceptance Criteria
- [ ] `TuiError` enum compiles
- [ ] `From<std::io::Error>` conversion works
- [ ] `Display` produces readable messages
- [ ] All new TUI modules use `TuiError` instead of `Box<dyn Error>`

---

## Sprint 0 Definition of Done

- [ ] All 5 stories completed
- [ ] TUI code extracted from `main.rs` into dedicated modules
- [ ] Panic hook restores terminal on crash
- [ ] Terminal resize handled gracefully
- [ ] `unicode-width` in Cargo.toml
- [ ] `cargo build --release` succeeds
- [ ] `cargo test -p rusty-claude-cli` passes
- [ ] `cargo clippy -p rusty-claude-cli` has no new warnings
- [ ] `/tui` command works identically to before extraction
- [ ] `main.rs` line count reduced (target: -200 lines)
