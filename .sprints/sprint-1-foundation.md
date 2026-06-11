# Sprint 1: Foundation & Bug Fixes

> **Duration:** 4 days | **Stories:** 7 | **Goal:** Make TUI reliable for daily use
> **Depends on:** Sprint 0 (extraction must be complete)

---

## S1-1: Fix ProviderSwap terminal breakage

**Priority:** P0 — Critical  
**Estimate:** 0.5 day  

### Description
After `Ctrl+P` triggers ProviderSwap, the setup wizard runs but the terminal never returns to TUI mode. The user is stuck in a broken plain terminal.

### Root Cause
`src/main.rs` (now `src/tui_repl.rs` after S0) calls `app.restore_terminal()` (leaves alt screen), runs wizard, then calls `app.suspend()` which disables raw mode but never re-enters the TUI.

### Implementation

**File:** `src/tui_repl.rs`

```rust
TuiReadOutcome::ProviderSwap => {
    // Stay in alternate screen, just disable raw mode for the wizard
    app.suspend()?;
    
    setup_wizard::run_setup_wizard()?;
    
    // Reload config and update model
    let cwd = std::env::current_dir().unwrap_or_default();
    if let Ok(config) = runtime::ConfigLoader::default_for(&cwd).load() {
        if let Some(new_model) = config.provider().model().map(str::to_string) {
            let _ = cli.set_model(Some(new_model));
        }
    }
    
    // Re-enter TUI
    app.resume()?;
    app.push_system_message("Provider updated");
}
```

**Note:** `suspend()` stays in alternate screen (just disables raw mode). `resume()` re-enables raw mode and redraws. The wizard runs on the alternate screen with echoing stdin, which is fine for text prompts. If the wizard needs a full normal terminal, use `app.restore_terminal()` → wizard → `TuiApp::init()` to fully reinitialize. Test both approaches.

### Acceptance Criteria
- [ ] `Ctrl+P` runs setup wizard
- [ ] After wizard completes, TUI resumes automatically
- [ ] Provider/model updated in dashboard
- [ ] No terminal corruption after repeated ProviderSwap cycles (test 5x)

---

## S1-2: Fix status message race condition

**Priority:** P0 — Critical  
**Estimate:** 0.1 day  

### Description
"Done" status message is set and immediately cleared in the same event loop tick, so it's never visible.

### Root Cause
```rust
app.set_status("Done");
if let Ok(mut ds) = dashboard_state.write() {
    ds.status_message.clear();  // immediately clears — never rendered
}
```

### Implementation
Remove the `ds.status_message.clear()` block. Status will be naturally overwritten when the next turn starts with `set_status("Thinking...")`.

### Acceptance Criteria
- [ ] "✓ Done" visible in dashboard for at least 1 second after turn completes
- [ ] Next turn's "Thinking..." overwrites it

---

## S1-3: Deduplicate ANSI strippers

**Priority:** P1 — High  
**Estimate:** 0.5 day  

### Description
Two independent ANSI escape strippers:
- `strip_ansi_escapes()` in `src/tui.rs`
- `strip_ansi()` in `src/main.rs` (now `src/tui_update.rs` after S0)

Neither handles OSC sequences (`ESC ]...BEL` or `ESC ]...ST`).

### Implementation

**File:** `src/tui_update.rs` (after S0 extraction)

Keep one canonical implementation, exported as `pub`:

```rust
/// Strip ANSI escape sequences from text.
/// Handles CSI (ESC [), OSC (ESC ]), and single-char escapes.
pub fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            match chars.peek() {
                Some('[') => {
                    // CSI sequence: ESC [ ... final_char
                    chars.next(); // consume '['
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c.is_ascii_alphabetic() || c == '~' {
                            break; // final character
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
                        if c == '\x1b' {
                            // Could be ESC \ (ST)
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                                break;
                            }
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
                    // Single-char escape (e.g., ESC c for reset)
                    chars.next();
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}
```

Delete `strip_ansi_escapes()` from `tui.rs`. Update all callers.

### Tests
```rust
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
fn test_strip_osc_with_st_terminator() {
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
fn test_strip_no_escapes() {
    assert_eq!(strip_ansi("plain text"), "plain text");
}

#[test]
fn test_strip_cursor_movement() {
    assert_eq!(strip_ansi("\x1b[2J\x1b[Hclear"), "clear");
}

#[test]
fn test_strip_dcs_sequence() {
    assert_eq!(strip_ansi("\x1bPq$d\x1b\\"), "");
}

#[test]
fn test_strip_empty_string() {
    assert_eq!(strip_ansi(""), "");
}

#[test]
fn test_strip_malformed_no_final_char() {
    // Unterminated CSI — should still consume until end
    assert_eq!(strip_ansi("\x1b[31m"), "");
}
```

### Acceptance Criteria
- [ ] Single ANSI stripper implementation in `tui_update.rs`
- [ ] Handles CSI, OSC (BEL + ST), DCS, APC, PM sequences
- [ ] All 11 test cases pass
- [ ] No ANSI artifacts visible in TUI conversation pane

---

## S1-4: Bound conversation memory

**Priority:** P1 — High  
**Estimate:** 0.5 day  

### Description
`Vec<ConversationLine>` grows unbounded. Long sessions consume increasing memory and slow `build_wrapped_conversation()`.

### Implementation

**File:** `src/tui.rs`

```rust
const MAX_CONVERSATION_LINES: usize = 10_000;

fn auto_scroll(&mut self) {
    if self.conversation.len() > MAX_CONVERSATION_LINES {
        let drain_count = self.conversation.len() - MAX_CONVERSATION_LINES;
        self.conversation.drain(..drain_count);
        // Also drain cached render results for removed entries
        self.render_cache.drain(..drain_count);
        
        // Insert trim notice as first line
        self.conversation.insert(0, ConversationLine {
            content: ConversationContent::Plain {
                text: "... (earlier messages trimmed)".to_string(),
                color: Color::DarkGray,
                bold: false,
            },
        });
    }
    self.conversation_scroll = 0;
    self.needs_redraw = true;
}
```

### Acceptance Criteria
- [ ] Conversation never exceeds MAX_CONVERSATION_LINES
- [ ] Oldest messages trimmed first
- [ ] Trim notice appears as first line
- [ ] Render cache also trimmed (prevents desync)
- [ ] Memory stays flat in long sessions

### Test
```rust
#[test]
fn test_conversation_memory_bound() {
    let mut app = create_test_app();
    for i in 0..12_000 {
        app.push_output(&format!("line {i}"), false);
    }
    assert!(app.conversation_len() <= MAX_CONVERSATION_LINES);
    // First line should be trim notice
    assert!(app.conversation_text(0).contains("trimmed"));
}
```

---

## S1-5: Wire real token tracking

**Priority:** P1 — High  
**Estimate:** 1 day  

### Description
Dashboard shows `output_tokens: 0`, `cost_usd: 0.0`, `cache_read_tokens: 0` because `update_dashboard()` uses a crude char÷4 estimate. The runtime tracks real values.

### Implementation

**File:** `src/tui_update.rs`

The runtime's `TokenUsage` struct (from `runtime/src/usage.rs`):
```rust
// ACTUAL runtime struct — use these field names:
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,  // NOT "cache_creation_tokens"
    pub cache_read_input_tokens: u32,      // NOT "cache_read_tokens"
}
```

Cost is computed separately via `UsageCostEstimate::total_cost_usd()`.

Update `update_dashboard()`:
```rust
pub fn update_dashboard(state: &SharedDashboardState, cli: &LiveCli) {
    if let Ok(mut ds) = state.write() {
        ds.model = cli.model.clone();
        ds.permission_mode = format!("{:?}", cli.permission_mode);
        
        if let Some(rt) = cli.runtime.runtime.as_ref() {
            let session = rt.session();
            ds.session_id = Some(session.session_id.clone());
            ds.turn_count = session.messages.len() as u32;
            
            // Use REAL token counts from runtime
            if let Some(usage) = session.total_usage() {
                ds.input_tokens = usage.input_tokens;
                ds.output_tokens = usage.output_tokens;
                ds.cache_creation_tokens = usage.cache_creation_input_tokens;
                ds.cache_read_tokens = usage.cache_read_input_tokens;
            }
            
            // Compute cost using runtime's cost estimator
            if let Some(cost) = session.estimate_cost() {
                ds.cost_usd = cost.total_cost_usd();
            }
        }
    }
}
```

**Note:** Verify the actual method names on the runtime session object (`total_usage()`, `estimate_cost()`, etc.) by reading `runtime/src/`. Adjust to match the real API.

### Acceptance Criteria
- [ ] Dashboard shows real input/output token counts after each turn
- [ ] Cache read and cache creation tokens displayed
- [ ] Cost estimate shows non-zero value for paid models
- [ ] Field names match runtime's `TokenUsage` struct exactly

### Test
Manual: send a message, verify dashboard numbers are non-zero and match API response logs.

---

## S1-6: Wire terminal resize to conversation re-wrap

**Priority:** P1 — High  
**Estimate:** 0.25 day  

### Description
S0-3 handles resize events. This story ensures the conversation rendering cache is invalidated so word-wrapping adjusts to the new width.

### Implementation

**File:** `src/tui.rs`

```rust
pub fn mark_resize(&mut self, _width: u16, _height: u16) {
    // Invalidate cached wrapped lines — they'll be re-wrapped at new width
    self.invalidate_render_cache();
    self.needs_redraw = true;
}

fn invalidate_render_cache(&mut self) {
    // Clear all cached Vec<Line> — next frame re-renders everything
    // This is fine: resize is infrequent and a single re-render is fast
    self.render_cache.clear();
}
```

### Acceptance Criteria
- [ ] Terminal resize triggers full re-render
- [ ] Word-wrapping adjusts to new width
- [ ] No layout corruption

---

## S1-7: Write TUI unit tests

**Priority:** P1 — High  
**Estimate:** 1 day  

### Description
Zero test coverage for TUI logic. The rendering code doesn't need terminal tests, but the data transformations do.

### Implementation

**New file:** `tests/tui_unit_tests.rs`

```rust
// 1. ANSI stripping — see S1-3 for full test list

// 2. Word wrapping
#[test]
fn test_wrap_empty() {
    assert_eq!(wrap_line("", 80), vec![""]);
}

#[test]
fn test_wrap_exact_width() {
    assert_eq!(wrap_line("hello", 5), vec!["hello"]);
}

#[test]
fn test_wrap_overflow_with_break() {
    let wrapped = wrap_line("hello world foo", 10);
    assert!(wrapped.len() >= 2);
    assert!(wrapped[0].len() <= 10);
}

#[test]
fn test_wrap_hyphen_break() {
    let wrapped = wrap_line("well-known-author", 10);
    assert!(wrapped.len() >= 2);
}

#[test]
fn test_wrap_no_break_point() {
    // Hard break mid-word when no space/hyphen available
    let wrapped = wrap_line("abcdefghijklmnop", 5);
    assert!(wrapped.len() >= 3);
}

// 3. Dashboard state
#[test]
fn test_dashboard_default_values() {
    let state = DashboardState::default();
    assert_eq!(state.input_tokens, 0);
    assert_eq!(state.output_tokens, 0);
    assert_eq!(state.cost_usd, 0.0);
}

// 4. Conversation management
#[test]
fn test_push_user_input() {
    let mut app = create_test_app();
    app.push_user_input("hello");
    assert!(app.conversation_len() >= 1);
}

#[test]
fn test_push_output_strips_ansi() {
    let mut app = create_test_app();
    app.push_output("\x1b[31mred\x1b[0m", false);
    // Should not contain escape characters
    let text = app.conversation_text(app.conversation_len() - 1);
    assert!(!text.contains('\x1b'));
}

#[test]
fn test_auto_scroll_resets_offset() {
    let mut app = create_test_app();
    app.conversation_scroll = 100;
    app.push_user_input("new message");
    assert_eq!(app.conversation_scroll, 0);
}
```

### Acceptance Criteria
- [ ] All tests pass with `cargo test -p rusty-claude-cli`
- [ ] Tests cover: ANSI stripping, word wrapping, dashboard state, conversation management
- [ ] No terminal or ratatui dependencies in tests (pure data logic only)

---

## Sprint 1 Definition of Done

- [ ] All 7 stories completed and tested
- [ ] `cargo test -p rusty-claude-cli` passes
- [ ] `cargo clippy -p rusty-claude-cli` has no new warnings
- [ ] Manual smoke test: launch TUI, send 3 messages, Ctrl+P provider swap, verify dashboard updates, resize terminal
- [ ] Commit: `fix(tui): sprint 1 — foundation and bug fixes`
