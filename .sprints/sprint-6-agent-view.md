# Sprint 6: Agent View & Multi-Session

> **Duration:** 4 days | **Stories:** 5 | **Goal:** Multi-agent monitoring dashboard
> **Depends on:** Sprint 0–5
> **Guardrails:** Four Laws enforced. Commit after each story.

---

## S6-1: Create AgentView data model

**Priority:** P1 — High  
**Estimate:** 0.5 day  
**Scope:** IN: `src/agent_view.rs` (new). OUT: `tui.rs`, `main.rs`

### Description
Model for tracking multiple agent sessions — status, progress, metadata.

### Pre-Execution Checklist
```
[ ] Read src/tui.rs for existing SharedDashboardState patterns
[ ] Read src/theme.rs for Agent View color fields
[ ] Scope locked: only src/agent_view.rs
[ ] Rollback: rm src/agent_view.rs
```

### Implementation

**New file:** `src/agent_view.rs`

```rust
use std::time::Instant;
use ratatui::style::Color;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStatus {
    Running,
    WaitingForInput,
    Done,
    Failed,
    Cancelled,
}

impl AgentStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            AgentStatus::Running => "⬢",
            AgentStatus::WaitingForInput => "◐",
            AgentStatus::Done => "✓",
            AgentStatus::Failed => "✗",
            AgentStatus::Cancelled => "⊘",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentSession {
    pub id: String,
    pub name: String,
    pub status: AgentStatus,
    pub model: String,
    pub turn_count: u32,
    pub last_message: String,
    pub working_dir: String,
    pub started_at: Instant,
    pub task_subject: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField { Status, Name, Started, TurnCount }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterState { All, Running, Done, Failed }

pub struct AgentView {
    pub sessions: Vec<AgentSession>,
    pub sort_by: SortField,
    pub filter: FilterState,
    pub selected: usize,
    pub active: bool,
}

impl AgentView {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            sort_by: SortField::Status,
            filter: FilterState::All,
            selected: 0,
            active: false,
        }
    }

    pub fn open(&mut self) { self.active = true; self.selected = 0; }
    pub fn close(&mut self) { self.active = false; }

    pub fn update_session(&mut self, session: AgentSession) {
        if let Some(existing) = self.sessions.iter_mut().find(|s| s.id == session.id) {
            *existing = session;
        } else {
            self.sessions.push(session);
        }
    }

    pub fn remove_session(&mut self, id: &str) {
        self.sessions.retain(|s| s.id != id);
    }

    pub fn filtered_sessions(&self) -> Vec<&AgentSession> {
        let mut sessions: Vec<&AgentSession> = self.sessions
            .iter()
            .filter(|s| match self.filter {
                FilterState::All => true,
                FilterState::Running => s.status == AgentStatus::Running,
                FilterState::Done => s.status == AgentStatus::Done,
                FilterState::Failed => s.status == AgentStatus::Failed,
            })
            .collect();
        sessions.sort_by(|a, b| match self.sort_by {
            SortField::Status => a.status.icon().cmp(b.status.icon()),
            SortField::Name => a.name.cmp(&b.name),
            SortField::Started => a.started_at.cmp(&b.started_at),
            SortField::TurnCount => a.turn_count.cmp(&b.turn_count),
        });
        sessions
    }

    pub fn select_next(&mut self) {
        let count = self.filtered_sessions().len();
        if count > 0 { self.selected = (self.selected + 1) % count; }
    }

    pub fn select_prev(&mut self) {
        let count = self.filtered_sessions().len();
        if count > 0 { self.selected = if self.selected == 0 { count - 1 } else { self.selected - 1 }; }
    }

    pub fn cycle_filter(&mut self) {
        self.filter = match self.filter {
            FilterState::All => FilterState::Running,
            FilterState::Running => FilterState::Done,
            FilterState::Done => FilterState::Failed,
            FilterState::Failed => FilterState::All,
        };
        self.selected = 0;
    }

    pub fn cycle_sort(&mut self) {
        self.sort_by = match self.sort_by {
            SortField::Status => SortField::Name,
            SortField::Name => SortField::Started,
            SortField::Started => SortField::TurnCount,
            SortField::TurnCount => SortField::Status,
        };
    }
}
```

### Verification
```bash
cargo build -p rusty-claude-cli
```

### Commit
```
feat(tui): add AgentView data model with session tracking

Authored by TheArchitectit
```

---

## S6-2: Implement Agent View rendering

**Priority:** P1 — High  
**Estimate:** 1 day  
**Scope:** IN: `src/tui.rs` (add draw_agent_view method). OUT: theme.rs, main.rs

### Pre-Execution Checklist
```
[ ] Read src/tui.rs for draw_frame(), draw_left_pane(), draw_right_pane() patterns
[ ] Read src/theme.rs for agent_* color fields
[ ] Scope locked: only tui.rs draw_agent_view method + TuiApp field
[ ] Rollback: git checkout HEAD -- src/tui.rs
```

### Implementation

Add `AgentView` to `TuiApp`:
```rust
pub struct TuiApp {
    // ... existing fields
    pub agent_view: AgentView,
}
```

Add rendering method. **All colors from `self.theme`** — NO hardcoded `Color::`:
```rust
fn draw_agent_view(&self, f: &mut Frame, area: Rect) {
    if !self.agent_view.active { return; }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(8),
        ])
        .split(area);

    // Header
    let filter_label = format!("Filter: {:?}", self.agent_view.filter);
    let sort_label = format!("Sort: {:?}", self.agent_view.sort_by);
    let header = Paragraph::new(Line::from(vec![
        Span::styled("  Agent View  ",
            Style::default().fg(self.tc(&self.theme.dashboard_header)).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  [{filter_label}]  [{sort_label}]"),
            Style::default().fg(self.tc(&self.theme.dashboard_key))),
    ]))
    .block(Block::default().borders(Borders::ALL)
        .border_style(Style::default().fg(self.tc(&self.theme.border_active))));
    f.render_widget(header, chunks[0]);

    // Session list
    let sessions = self.agent_view.filtered_sessions();
    let items: Vec<ListItem> = sessions.iter().enumerate().map(|(i, s)| {
        let is_selected = i == self.agent_view.selected;
        let status_color = match s.status {
            AgentStatus::Running => self.tc(&self.theme.agent_running),
            AgentStatus::WaitingForInput => self.tc(&self.theme.agent_waiting),
            AgentStatus::Done => self.tc(&self.theme.agent_done),
            AgentStatus::Failed => self.tc(&self.theme.agent_failed),
            AgentStatus::Cancelled => self.tc(&self.theme.agent_cancelled),
        };
        let elapsed = s.started_at.elapsed().as_secs();
        let elapsed_str = if elapsed < 60 { format!("{elapsed}s") } else { format!("{}m{}s", elapsed/60, elapsed%60) };
        let line = Line::from(vec![
            Span::styled(format!(" {} ", s.status.icon()), Style::default().fg(status_color)),
            Span::styled(format!("{:<20}", s.name), Style::default().fg(self.tc(&self.theme.conversation_text))),
            Span::styled(format!("{:<16}", s.model), Style::default().fg(self.tc(&self.theme.dashboard_key))),
            Span::styled(format!("{} turns  ", s.turn_count), Style::default().fg(self.tc(&self.theme.dashboard_value))),
            Span::styled(elapsed_str, Style::default().fg(self.tc(&self.theme.key_hint))),
        ]);
        let style = if is_selected { Style::default().bg(self.tc(&self.theme.completion_selected_bg)) } else { Style::default() };
        ListItem::new(line).style(style)
    }).collect();

    let list = List::new(items).block(
        Block::default().borders(Borders::ALL)
            .border_style(Style::default().fg(self.tc(&self.theme.border)))
            .title(" Sessions "));
    f.render_widget(list, chunks[1]);

    // Detail panel
    if let Some(session) = sessions.get(self.agent_view.selected) {
        let detail = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(self.tc(&self.theme.dashboard_key))),
                Span::styled(&session.id, Style::default().fg(self.tc(&self.theme.dashboard_value))),
            ]),
            Line::from(vec![
                Span::styled("Task: ", Style::default().fg(self.tc(&self.theme.dashboard_key))),
                Span::styled(session.task_subject.as_deref().unwrap_or("(none)"),
                    Style::default().fg(self.tc(&self.theme.dashboard_value))),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL)
            .border_style(Style::default().fg(self.tc(&self.theme.border)))
            .title(" Details "))
        .wrap(Wrap { trim: false });
        f.render_widget(detail, chunks[2]);
    }
}
```

### Verification
```bash
cargo build -p rusty-claude-cli
cargo clippy -p rusty-claude-cli
```

### Commit
```
feat(tui): render Agent View with theme colors

Authored by TheArchitectit
```

---

## S6-3: Wire Agent View keybindings

**Priority:** P1 — High  
**Estimate:** 0.25 day  
**Scope:** IN: `src/tui.rs` (dispatch_action), `src/keybindings.rs` (Action enum). OUT: main.rs

### Implementation

In `dispatch_action()`:
```rust
Action::ToggleAgentView => {
    if self.agent_view.active {
        self.agent_view.close();
    } else {
        self.agent_view.open();
    }
}
```

In `handle_key()` — when agent view is active, intercept keys:
```rust
if self.agent_view.active {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => { self.agent_view.close(); }
        KeyCode::Tab => { self.agent_view.cycle_filter(); }
        KeyCode::Char('s') => { self.agent_view.cycle_sort(); }
        KeyCode::Down | KeyCode::Char('j') => { self.agent_view.select_next(); }
        KeyCode::Up | KeyCode::Char('k') => { self.agent_view.select_prev(); }
        _ => {}
    }
    self.needs_redraw = true;
    return Ok(TuiReadOutcome::Pending);
}
```

### Commit
```
feat(tui): wire Agent View keybindings (Ctrl+A, j/k, Tab, s)

Authored by TheArchitectit
```

---

## S6-4: Integrate with runtime session registry

**Priority:** P2 — Medium  
**Estimate:** 1 day  
**Scope:** IN: `src/tui_repl.rs`, `src/tui_update.rs`. OUT: runtime crate internals

### Implementation

1. Add `SessionRegistry` shared state
2. Hook into team/spawn lifecycle:
   - `TeamCreate` → register
   - `Agent::spawn` → register
   - Turn completion → update count
   - Shutdown → mark Done/Failed
3. Share via `Arc<RwLock<SessionRegistry>>`

### Commit
```
feat(tui): integrate Agent View with runtime session lifecycle

Authored by TheArchitectit
```

---

## S6-5: Write Agent View tests

**Priority:** P1 — High  
**Estimate:** 0.25 day  
**Scope:** IN: `tests/agent_view_tests.rs` (new). OUT: all source files

### Tests
```rust
use rusty_claude_cli::agent_view::*;

#[test]
fn test_update_session() { ... }
#[test]
fn test_remove_session() { ... }
#[test]
fn test_filter_by_status() { ... }
#[test]
fn test_sort_by_name() { ... }
#[test]
fn test_select_navigation() { ... }
#[test]
fn test_cycle_filter() { ... }
```

### Commit
```
test(tui): add Agent View data model tests

Authored by TheArchitectit
```

---

## Sprint 6 Definition of Done
- [ ] All 5 stories completed and committed
- [ ] Agent View shows sessions with theme colors (zero hardcoded Color::)
- [ ] `cargo test -p rusty-claude-cli` passes
- [ ] `cargo clippy -p rusty-claude-cli` clean
- [ ] Manual: spawn agents, open Agent View, verify live updates
