# Agents & Skills Setup Guide

Install guardrails skills, agents, and hooks across all supported AI coding platforms.

## Quick Start

| Platform | Setup Time | Install Command |
|----------|-----------|-----------------|
| Claude Code | 30s | `python scripts/setup_agents.py --install --platform claude` |
| Cursor | 30s | `python scripts/setup_agents.py --install --platform cursor` |
| Windsurf | 10s | `python scripts/setup_agents.py --install --platform windsurf` |
| OpenCode | 30s | `python scripts/setup_agents.py --install --platform opencode` |
| GitHub Copilot | 10s | `python scripts/setup_agents.py --install --platform copilot` |
| All platforms | 1min | `python scripts/setup_agents.py --install` |

## Platform Comparison

| Feature | Claude Code | Cursor | Windsurf | OpenCode | GitHub Copilot |
|---------|-------------|--------|----------|----------|----------------|
| Config location | `.claude/skills/` | `.cursor/rules/` | `.windsurfrules` | `.opencode/` | `.github/copilot-instructions.md` |
| Format | JSON | Markdown + YAML frontmatter | Single markdown | JSON + markdown | Markdown |
| Skills | 7 JSON files | 4 rule files | Single file | 3 skill dirs | None (repo-level) |
| Agents | No | No | No | 2 JSON files | No |
| Hooks | 3 shell scripts | No | No | No | No |
| Granular control | Per-skill install | Per-rule apply | All-or-nothing | Per-skill enable | All-or-nothing |
| Auto-install | Yes | Yes | Yes | Yes | Yes |

## Installation Methods

### 1. MCP Tool (Automated)

Use the `guardrail_install_skills` MCP tool from any AI session:

```javascript
// Full platform install
guardrail_install_skills({ platforms: "claude,cursor", target_path: "/path/to/project" })

// Clone a single file from GitHub
guardrail_install_skills({ action: "clone", path: ".claude/skills/guardrails-enforcer.json" })

// Install a single skill by name
guardrail_install_skills({ action: "install", skill: "guardrails-enforcer" })

// List available skills
guardrail_install_skills({ list_skills: true })

// List available platforms
guardrail_install_skills({ list_platforms: true })
```

### 2. Python Script (CLI)

```bash
# Install all platforms (copy mode)
python scripts/setup_agents.py --install

# Install specific platforms
python scripts/setup_agents.py --install --platform claude,cursor

# Symlink mode (live updates when repo changes)
python scripts/setup_agents.py --install --platform claude --mode symlink

# Dry run (preview without writing)
python scripts/setup_agents.py --install --dry-run

# Install to a different project directory
python scripts/setup_agents.py --install --platform claude --target ~/myproject

# Install a single skill by name
python scripts/setup_agents.py --install-skill guardrails-enforcer

# Clone a single file from GitHub (no local repo needed)
python scripts/setup_agents.py --clone .claude/skills/guardrails-enforcer.json
python scripts/setup_agents.py --clone .cursor/rules/guardrails-enforcer.md --target ~/myproject

# List available skills and platforms
python scripts/setup_agents.py --list-skills
python scripts/setup_agents.py --list-platforms
```

### 3. Manual Copy

Copy the config directories directly to your project:

```bash
# Claude Code
cp -r .claude/skills/ /path/to/project/.claude/skills/
cp -r .claude/hooks/ /path/to/project/.claude/hooks/

# Cursor
cp -r .cursor/rules/ /path/to/project/.cursor/rules/

# Windsurf
cp .windsurfrules /path/to/project/.windsurfrules

# OpenCode
cp -r .opencode/ /path/to/project/.opencode/

# GitHub Copilot
mkdir -p /path/to/project/.github/
cp .github/copilot-instructions.md /path/to/project/.github/copilot-instructions.md
```

### 4. Symlink (Live Updates)

```bash
# Claude Code (tracks repo changes)
ln -s "$(pwd)/.claude/skills" /path/to/project/.claude/skills
ln -s "$(pwd)/.claude/hooks" /path/to/project/.claude/hooks

# OpenCode
ln -s "$(pwd)/.opencode" /path/to/project/.opencode
```

## Per-Platform Guides

### Claude Code

**Config files:** `.claude/skills/*.json`, `.claude/hooks/*.sh`

**Skills (7):**

| Skill | File | Purpose |
|-------|------|---------|
| guardrails-enforcer | `guardrails-enforcer.json` | Four Laws enforcement on all operations |
| commit-validator | `commit-validator.json` | Commit message format and safety checks |
| env-separator | `env-separator.json` | Test/production environment isolation |
| scope-validator | `scope-validator.json` | File scope authorization checks |
| production-first | `production-first.json` | Production code before tests/infra |
| three-strikes | `three-strikes.json` | Retry limit and halt escalation |
| error-recovery | `error-recovery.json` | Structured error handling |

**Hooks (3):**

| Hook | File | Trigger |
|------|------|---------|
| Pre-execution | `pre-execution.sh` | Before every tool call |
| Post-execution | `post-execution.sh` | After every tool call |
| Pre-commit | `pre-commit.sh` | Before git commit |

**Skill format (JSON):**

```json
{
  "name": "guardrails-enforcer",
  "description": "Enforces the Four Laws of Agent Safety",
  "tools": ["Read", "Grep", "Glob", "AskUserQuestion"],
  "prompt": "Your skill instructions here..."
}
```

**Install:**

```bash
python scripts/setup_agents.py --install-skill guardrails-enforcer
```

**Verify:**

```bash
ls .claude/skills/          # Should show 7 JSON files
ls .claude/hooks/           # Should show 3 shell scripts
```

### Cursor

**Config files:** `.cursor/rules/*.md`

**Rules (4):**

| Rule | File | Always Apply |
|------|------|-------------|
| guardrails-enforcer | `guardrails-enforcer.md` | Yes |
| production-first | `production-first.md` | Yes |
| three-strikes | `three-strikes.md` | Yes |
| clean-architecture | `clean-architecture.md` | Yes |

**Rule format (Markdown + YAML frontmatter):**

```markdown
---
description: Enforces the Four Laws of Agent Safety on all code generation
globs: "**/*"
alwaysApply: true
---

# Guardrails Enforcement

Your rule instructions here...
```

Frontmatter fields:
- `description` — Shows in Cursor rules panel
- `globs` — File patterns where rule applies
- `alwaysApply` — `true` to auto-attach, `false` for manual

**Install:**

```bash
python scripts/setup_agents.py --install --platform cursor
```

**Verify:**

```bash
ls .cursor/rules/           # Should show 4 markdown files
```

### Windsurf

**Config file:** `.windsurfrules` (single file)

**Format:** Single markdown file containing all rules. Windsurf does not support per-skill granularity or agent configuration.

The file includes:
- Four Laws of Agent Safety
- Pre-operation checklist and forbidden actions
- Three Strikes Rule
- Production-First Rule
- Scope Rules
- Architecture patterns (Clean Architecture, CQRS, Vertical Slices, SOLID)
- References to shared prompts

**Install:**

```bash
python scripts/setup_agents.py --install --platform windsurf
# Or manually:
cp .windsurfrules /path/to/project/.windsurfrules
```

**Verify:**

```bash
test -f .windsurfrules && echo "Installed" || echo "Missing"
```

### OpenCode

**Config files:** `.opencode/oh-my-opencode.jsonc`, `.opencode/agents/*.json`, `.opencode/skills/<name>/SKILL.md`

**Agents (2):**

| Agent | File | Permissions |
|-------|------|------------|
| guardrails-auditor | `guardrails-auditor.json` | Read-only (edit: deny, bash: deny) |
| doc-indexer | `doc-indexer.json` | Read + edit (bash: deny) |

**Skills (3):**

| Skill | Directory |
|-------|-----------|
| guardrails-enforcer | `skills/guardrails-enforcer/SKILL.md` |
| commit-validator | `skills/commit-validator/SKILL.md` |
| env-separator | `skills/env-separator/SKILL.md` |

**Config format (JSONC):**

```jsonc
{
  "agents": {
    "guardrails-auditor": {
      "model": "anthropic/claude-sonnet-4",
      "temperature": 0.1,
      "prompt_append": "You are a Guardrails Auditor...",
      "permissions": { "edit": "deny", "bash": "deny", "read": "allow" }
    }
  },
  "skills": {
    "sources": [{"path": "./.opencode/skills", "recursive": true}],
    "enable": ["guardrails-enforcer", "commit-validator", "env-separator"]
  }
}
```

**Skill format (Markdown + YAML frontmatter):**

```markdown
---
name: guardrails-enforcer
description: "Enforces the Four Laws of Agent Safety"
---

# Guardrails Enforcement

Your skill instructions here...
```

Agent format (JSON):

```json
{
  "name": "guardrails-auditor",
  "model": "anthropic/claude-sonnet-4",
  "temperature": 0.1,
  "prompt_append": "You are a Guardrails Auditor...",
  "permissions": { "edit": "deny", "bash": "deny", "read": "allow" }
}
```

**Install:**

```bash
python scripts/setup_agents.py --install --platform opencode
```

**Verify:**

```bash
test -f .opencode/oh-my-opencode.jsonc && echo "Config OK"
ls .opencode/agents/        # Should show 2 JSON files
ls .opencode/skills/        # Should show 3 skill directories
```

### GitHub Copilot

**Config file:** `.github/copilot-instructions.md`

**Format:** Markdown repo-level instructions. Copilot does not support per-skill configuration, agent definitions, or hooks. All rules are merged into a single file.

The file includes:
- Four Laws of Agent Safety (adapted for autocomplete/chat)
- Code generation rules (scope, production-first, error handling, security)
- Forbidden patterns
- Three Strikes Rule
- File header conventions
- Architecture patterns for the MCP server
- References to shared prompts

**Install:**

```bash
python scripts/setup_agents.py --install --platform copilot
# Or manually:
mkdir -p /path/to/project/.github/
cp .github/copilot-instructions.md /path/to/project/.github/copilot-instructions.md
```

**Verify:**

```bash
test -f .github/copilot-instructions.md && echo "Installed" || echo "Missing"
```

## Shared Prompts

All platform configs derive from canonical prompt files in `skills/shared-prompts/`. These are the source of truth — when rules change, update the shared prompt first, then regenerate platform configs.

| Prompt | File | Description |
|--------|------|-------------|
| Four Laws | `four-laws.md` | Read-before-edit, stay-in-scope, verify-before-commit, halt-when-uncertain |
| Halt Conditions | `halt-conditions.md` | When and how to stop and escalate to user |
| Vibe Coding | `vibe-coding.md` | Flow-state development under constraints |
| Error Recovery | `error-recovery.md` | Structured error handling and retry patterns |
| Three Strikes | `three-strikes.md` | Three-attempt limit with escalating escalation |
| Production First | `production-first.md` | Production code before tests and infrastructure |
| Scope Validation | `scope-validation.md` | File and operation scope authorization |
| Clean Architecture | `clean-architecture.md` | Domain-first layering, dependency inversion |
| CQRS | `cqrs.md` | Command/query separation for write/read operations |

## MCP Tool Usage

The `guardrail_install_skills` MCP tool wraps the setup script. Use it from any AI coding session.

**Full install:**

```javascript
guardrail_install_skills({
  platforms: "claude,cursor",
  target_path: "/path/to/project"
})
```

**Per-skill install:**

```javascript
guardrail_install_skills({
  action: "install",
  skill: "guardrails-enforcer",
  platform: "claude",
  target_path: "/path/to/project"
})
```

**Clone from GitHub:**

```javascript
guardrail_install_skills({
  action: "clone",
  path: ".claude/skills/guardrails-enforcer.json",
  target_path: "/path/to/project"
})
```

**List skills:**

```javascript
guardrail_install_skills({ list_skills: true })
```

**List platforms:**

```javascript
guardrail_install_skills({ list_platforms: true })
```

## Architecture Reference

For the MCP server codebase that underpins these guardrails:

- [ARCHITECTURE_CLEAN_CQRS.md](ARCHITECTURE_CLEAN_CQRS.md) — Full Clean Architecture + CQRS design
- [skills/shared-prompts/clean-architecture.md](../skills/shared-prompts/clean-architecture.md) — Domain-first layering patterns
- [skills/shared-prompts/cqrs.md](../skills/shared-prompts/cqrs.md) — Command/query separation patterns

## Customization

Edit generated files directly in your project, or create new ones:

**Claude Code** — add a JSON file to `.claude/skills/`:

```json
{
  "name": "my-custom-skill",
  "description": "What this skill does",
  "tools": ["Read", "Grep"],
  "prompt": "Your skill instructions here..."
}
```

**OpenCode** — add a `SKILL.md` under `.opencode/skills/<name>/`:

```markdown
---
name: my-custom-skill
description: "What this skill does"
---
# My Custom Skill
Your skill instructions here...
```

**Cursor** — add a markdown file to `.cursor/rules/`:

```markdown
---
description: What this rule does
globs: "**/*.go"
alwaysApply: false
---
# My Custom Rule
Your rule instructions here...
```

**Remove all configuration:**

```bash
rm -rf .claude/ .cursor/ .opencode/ .windsurfrules .github/copilot-instructions.md
```

## Troubleshooting

### Skills Not Loading

- **Claude Code:** Verify `.claude/skills/` has JSON files. Check syntax: `python -m json.tool .claude/skills/guardrails-enforcer.json`. Restart after adding skills.
- **Cursor:** Verify `.cursor/rules/` has `.md` files. Check YAML frontmatter has `description`, `globs`, `alwaysApply`. Check settings > Rules.
- **OpenCode:** Verify `oh-my-opencode.jsonc` syntax. Check skills are listed in `skills.enable`. Confirm `SKILL.md` exists in each skill dir. Restart.
- **Hooks not running:** Ensure executable (`chmod +x .claude/hooks/*.sh`). Check syntax: `bash -n .claude/hooks/pre-commit.sh`.

### MCP Tool Not Found

- Ensure the guardrails MCP server is configured in your AI tool's MCP settings
- Check the server process is running: the server provides `guardrail_install_skills`
- Verify connection in your platform's MCP panel

### Install Script Errors

```bash
# Check Python version (3.8+ required)
python --version

# Run with verbose output
python scripts/setup_agents.py --install --dry-run

# Verify write permissions on target
touch /path/to/project/.claude/test && rm /path/to/project/.claude/test
```

## References

- [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md) — Core safety protocols
- [COMMIT_WORKFLOW.md](workflows/COMMIT_WORKFLOW.md) — Commit standards
- [TEST_PRODUCTION_SEPARATION.md](standards/TEST_PRODUCTION_SEPARATION.md) — Environment isolation
- [AGENT_EXECUTION.md](workflows/AGENT_EXECUTION.md) — Execution protocols
