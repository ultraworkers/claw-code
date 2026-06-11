# Sprint 5: Chat Modes & Diff Viewer

> **Duration:** 2 days | **Stories:** 4 | **Goal:** Aider-style workflow features
> **Depends on:** Sprint 0–4
> **Revised:** Fixed `/diff --color=always` issue, added theme colors, fixed git commands

---

## S5-1: Implement ChatMode enum and mode switching

**Priority:** P1 — High  
**Estimate:** 0.5 day  

### Implementation

**File:** `src/tui.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatMode {
    Code,
    Ask,
    Architect,
}

impl ChatMode {
    pub fn label(&self) -> &'static str {
        match self {
            ChatMode::Code => "code",
            ChatMode::Ask => "ask",
            ChatMode::Architect => "arch",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ChatMode::Code => "Full access — edits and runs code",
            ChatMode::Ask => "Discussion only — no changes made",
            ChatMode::Architect => "Plan first, then implement",
        }
    }

    pub fn system_prompt_suffix(&self) -> &'static str {
        match self {
            ChatMode::Code => "",
            ChatMode::Ask => "\n\nIMPORTANT: Do NOT modify any files. Only discuss and explain.",
            ChatMode::Architect => "\n\nIMPORTANT: First create a plan. After the user approves, implement it.",
        }
    }

    pub fn next(self) -> Self {
        match self {
            ChatMode::Code => ChatMode::Ask,
            ChatMode::Ask => ChatMode::Architect,
            ChatMode::Architect => ChatMode::Code,
        }
    }
}
```

### Input prompt integration

```rust
fn update_input_prompt(&mut self) {
    let mode_label = self.chat_mode.label();
    self.input.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.tc(&self.theme.input_border)))
            .title(format!(" {mode_label} > ")),
    );
}
```

### Dashboard integration

Add `chat_mode` to `DashboardState`:
```rust
pub chat_mode: String,
```

Update `update_dashboard()` to include it.

### Slash commands in `tui_commands.rs`

```rust
TuiCommand::Code => { app.set_chat_mode(ChatMode::Code); app.push_system_message("Mode: Code"); }
TuiCommand::Ask => { app.set_chat_mode(ChatMode::Ask); app.push_system_message("Mode: Ask — no changes"); }
TuiCommand::Architect => { app.set_chat_mode(ChatMode::Architect); app.push_system_message("Mode: Architect — plan first"); }
```

### System prompt integration

When starting a turn, append the mode's suffix to the system prompt:
```rust
fn get_effective_system_prompt(&self) -> String {
    let base = self.base_system_prompt.clone();
    let suffix = self.chat_mode.system_prompt_suffix();
    format!("{base}{suffix}")
}
```

### Acceptance Criteria
- [ ] Input area shows `[code] >`, `[ask] >`, or `[arch] >`
- [ ] `/code`, `/ask`, `/architect` switch modes
- [ ] `Tab` cycles through modes
- [ ] Mode shown in dashboard
- [ ] System prompt suffix applied when mode changes

---

## S5-2: Implement `/diff` command

**Priority:** P1 — High  
**Estimate:** 0.5 day  

### Implementation

**FIX from Opus review:** Do NOT use `--color=always` — it injects ANSI codes into the diff output, which corrupts the rendering. We handle coloring ourselves via `CodeDiff` content type.

```rust
TuiCommand::Diff { staged } => {
    let mut args = vec!["diff"];
    if staged {
        args.push("--staged");
    }
    // Do NOT pass --color=always — we color the diff ourselves
    
    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(&cwd)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let diff = String::from_utf8_lossy(&out.stdout);
            if diff.is_empty() {
                app.push_system_message(if staged {
                    "No staged changes."
                } else {
                    "No uncommitted changes."
                });
            } else {
                app.push_diff(&diff);
            }
        }
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            app.push_system_message(&format!("git diff failed: {err}"));
        }
        Err(e) => {
            app.push_system_message(&format!("Failed to run git: {e}"));
        }
    }
}
```

### `/diff --stat` variant

Also add a compact stat view:
```rust
TuiCommand::DiffStat => {
    let output = std::process::Command::new("git")
        .args(["diff", "--stat", "--color=never"])  // --color=never for --stat is fine
        .current_dir(&cwd)
        .output();
    // ... render as Plain content
}
```

### Acceptance Criteria
- [ ] `/diff` shows unstaged changes with our own coloring (green/red)
- [ ] `/diff --staged` shows staged changes
- [ ] No ANSI codes from git in the output
- [ ] Empty diff shows message
- [ ] Error handling for non-git repos

---

## S5-3: Implement `/undo` command

**Priority:** P1 — High  
**Estimate:** 0.25 day  

### Implementation

```rust
TuiCommand::Undo { confirm } => {
    if confirm {
        let output = std::process::Command::new("git")
            .args(["checkout", "--", "."])
            .current_dir(&cwd)
            .output();
        
        match output {
            Ok(out) if out.status.success() => {
                app.push_system_message("✓ Reverted all uncommitted changes.");
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr);
                app.push_system_message(&format!("Undo failed: {err}"));
            }
            Err(e) => {
                app.push_system_message(&format!("Failed to run git: {e}"));
            }
        }
    } else {
        // Show what would be reverted
        let output = std::process::Command::new("git")
            .args(["diff", "--stat", "--color=never"])
            .current_dir(&cwd)
            .output();
        
        let stat = output.ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        
        if stat.is_empty() {
            app.push_system_message("Nothing to undo — no uncommitted changes.");
        } else {
            app.push_system_message(&format!(
                "This will revert:\n{stat}\nType /undo --confirm to proceed."
            ));
        }
    }
}
```

### Acceptance Criteria
- [ ] `/undo` shows what will be reverted
- [ ] `/undo --confirm` reverts
- [ ] No changes shows "Nothing to undo"

---

## S5-4: Implement `/ls` and `/context` commands

**Priority:** P2 — Medium  
**Estimate:** 0.25 day  

### Implementation

```rust
TuiCommand::Ls { path } => {
    match path {
        Some(p) => {
            match std::fs::read_to_string(&p) {
                Ok(content) => {
                    app.push_system_message(&format!("── {p} ──"));
                    app.push_output(&content, false);
                }
                Err(e) => app.push_system_message(&format!("Cannot read {p}: {e}")),
            }
        }
        None => {
            let files = cli.get_context_files();
            if files.is_empty() {
                app.push_system_message("No files in context.");
            } else {
                let mut msg = format!("Files in context ({}):\n", files.len());
                for f in &files {
                    msg += &format!("  {f}\n");
                }
                app.push_system_message(&msg);
            }
        }
    }
}

TuiCommand::Context => {
    // Alias for /ls with no path
    // Same as Ls { path: None }
}
```

### Acceptance Criteria
- [ ] `/ls` lists context files
- [ ] `/ls <path>` shows file contents
- [ ] `/context` is alias for `/ls`

---

## Sprint 5 Definition of Done

- [ ] All 4 stories completed
- [ ] Chat modes with system prompt integration
- [ ] `/diff` uses own coloring (no git --color)
- [ ] `/undo` with confirmation
- [ ] `/ls` shows context
- [ ] All rendering uses theme colors
- [ ] `cargo test -p rusty-claude-cli` passes
