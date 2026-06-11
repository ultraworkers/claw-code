# Agent Guardrails Template — v2.9.0

**Release Date:** 2026-05-08
**Theme:** Core Skill Systems for All Coding Platforms

---

## What's New

### Pre-Committed Skill Configs (No More Generation)

Skill configurations for Claude Code, Cursor, OpenCode, Windsurf, and GitHub Copilot now ship **pre-committed** in the repository. Setup is a file copy or symlink — no Python code generation at install time.

**New files:**
- `.claude/skills/` — 7 JSON skill files (guardrails-enforcer, commit-validator, env-separator, scope-validator, production-first, three-strikes, error-recovery)
- `.claude/hooks/` — 3 shell hooks (pre-commit, pre-execution, post-execution)
- `.cursor/rules/` — 3 Cursor rules files
- `.windsurfrules` — Windsurf rules preamble
- `.opencode/` — OpenCode config, skills, and 2 agents
- `.github/copilot-instructions.md` — GitHub Copilot repo-level instructions
- `skills/shared-prompts/` — 4 new canonical prompts: error-recovery, three-strikes, production-first, scope-validation

### New `guardrail_install_skills` MCP Tool

Headless skill installation via MCP. Supports:
- Full platform install (`platforms: "claude,cursor"`)
- Per-skill install by name (`skill: "guardrails-enforcer"`)
- Single-file clone from GitHub (`action: "clone"`, `path: ".claude/skills/guardrails-enforcer.json"`)
- List skills and platforms (`list_skills: true`)

### Refactored `scripts/setup_agents.py`

The setup script switched from "generate" to "install" mode:

```bash
# Clone a single skill file (no repo clone needed)
python scripts/setup_agents.py --clone .claude/skills/guardrails-enforcer.json

# Install a specific skill by name
python scripts/setup_agents.py --install-skill guardrails-enforcer

# List all 23 available skills
python scripts/setup_agents.py --list-skills

# Full platform install
python scripts/setup_agents.py --install --platform claude,cursor,windsurf
```

### Merged from Main

This release also includes all work merged from `main`:
- PR #3: SSE timeout resilience fix
- Sprint 005: Pre-Commit Safety Suite
- Sprint 006: Custom Advisor Roles System
- v2.0.0 / v2.7.0 / v2.8.0 releases
- IDE extensions (VS Code, JetBrains, Neovim, Vim)
- Team management tools (Python → Go migration)
- Document ingest system

---

## Migration

**Upgrading from v2.8.0:** No breaking changes. The setup script flags (`--claude`, `--minimal`, `--full`) are removed in favor of `--install` + `--platform` or `--install-skill`.

If you use the setup script, update your commands:
```bash
# Old
python scripts/setup_agents.py --claude --minimal

# New
python scripts/setup_agents.py --install --platform claude
python scripts/setup_agents.py --install-skill guardrails-enforcer  # for minimal
```

---

## Breaking Changes

| Change | Description |
|--------|-------------|
| `--claude`, `--opencode`, `--minimal`, `--full` flags removed | Replaced by `--install` + `--platform` or `--install-skill` |
| Setup script no longer generates from templates | Installs pre-committed files via copy or symlink |

---

## Contributors

Built with [Claude Code](https://claude.com/claude-code) using the Agent Guardrails framework.
