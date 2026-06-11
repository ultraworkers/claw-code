# Claw Code TUI — Feature Parity Roadmap

> **Branch:** `feat-tui` | **Stack:** ratatui 0.29 + crossterm 0.28 + tui-textarea 0.7
> **Goal:** Parity with Claude Code, OpenCode, Codex CLI, Aider terminal interfaces.
> **Guardrails:** See `.guardrails/` — Four Laws mandatory on every story.

---

## Guardrails Integration

### Four Laws (Applied to Every Sprint)

| Law | Application |
|-----|-------------|
| **Read Before Editing** | Read every file before modifying. No exceptions. |
| **Stay in Scope** | Each story lists IN/OUT scope. Only touch listed files. |
| **Verify Before Committing** | `cargo build` + `cargo test` must pass before each commit. |
| **Halt When Uncertain** | If syntax fails, test fails, or behavior unclear — stop and ask. |

### Commit Format (Every Story)
```
<type>(tui): <story description>

<why this change>

Authored by TheArchitectit
```

### Three Strikes Rule
- MAX 3 attempts per task
- After 3 failures → HALT, report full history, recommend fresh session
- Rollback between attempts: `git checkout HEAD -- <file>`

### Pre-Execution Checklist (Every File Edit)
```
[ ] File read before editing
[ ] File is IN scope for this story
[ ] Rollback command known (git checkout HEAD -- <file>)
[ ] No feature creep (only changes listed in story)
[ ] Previous sprint fixes not being undone
[ ] cargo build passes after edit
[ ] cargo test passes after edit
```

---

## Sprint Summary

| Sprint | Title | Duration | Stories | Goal |
|--------|-------|----------|---------|------|
| S0 | TUI Extraction & Architecture | 4 days | 5 | Extract from 20K-line main.rs |
| S1 | Foundation & Bug Fixes | 4 days | 7 | Make TUI reliable for daily use |
| S2 | Streaming & Markdown Rendering | 5 days | 6 | Real-time display + rich markdown |
| S3 | Theme System | 3 days | 5 | User customization |
| S4 | Keybindings & Command Palette | 4 days | 5 | UX parity |
| S5 | Chat Modes & Diff Viewer | 2 days | 4 | Aider-style workflows |
| S6 | Agent View & Multi-Session | 4 days | 5 | Multi-agent monitoring |
| S7 | Polish & Ship | 3 days | 7 | Mouse, CJK, sidebar, final QA |
| **Total** | | **29 days** | **44 stories** | |

---

## Architecture Decisions

### ADR-1: Conversation content model
```rust
pub enum ConversationContent {
    Plain { text: String, color: Color, bold: bool },
    Markdown { source: String },
    CodeDiff { diff: String },
}
```

### ADR-2: Theme architecture
`TuiTheme` struct with named color fields. Loaded from JSON, built-in defaults compiled in.

### ADR-3: Keybinding abstraction
`Action` enum decoupled from `KeyEvent`. Enables Vim/Emacs/Windows presets.

### ADR-4: Rendering context
Replace 10-parameter `draw_frame()` with `RenderContext` struct.

### ADR-5: Markdown rendering — CACHE PER ENTRY
Cache rendered `Vec<Line>` per `ConversationLine`. Re-render only on resize. Syntect assets loaded once via `once_cell::Lazy`.

### ADR-6: TUI module extraction
Extract all TUI code from `main.rs` before feature work begins.

---

## Sprint Details

→ [Sprint 0: TUI Extraction](./sprint-0-extraction.md)
→ [Sprint 1: Foundation](./sprint-1-foundation.md)
→ [Sprint 2: Streaming & Markdown](./sprint-2-markdown.md)
→ [Sprint 3: Themes](./sprint-3-themes.md)
→ [Sprint 4: Keybindings](./sprint-4-keybindings.md)
→ [Sprint 5: Chat Modes](./sprint-5-chat-modes.md)
→ [Sprint 6: Agent View](./sprint-6-agent-view.md)
→ [Sprint 7: Polish](./sprint-7-polish.md)
