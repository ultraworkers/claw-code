# OpenCode Integration

This guide explains how to integrate Agent Guardrails with OpenCode using agents, skills, and hooks.

## Overview

OpenCode supports:
- **Agents** - JSON configurations that define specialized agent behaviors, model selection, and permissions
- **Skills** - Markdown files with structured tool definitions and instructions
- **Hooks** - Shell scripts that run at specific lifecycle points

The setup script installs these configurations for you.

## Setup

### 1. Install All Configs

```bash
python scripts/setup_agents.py --install --platform opencode
```

This creates:
```
.opencode/
├── oh-my-opencode.jsonc
├── agents/
│   ├── guardrails-enforcer.json
│   ├── guardrails-auditor.json
│   └── doc-indexer.json
├── skills/
│   ├── guardrails-enforcer.md
│   ├── commit-validator.md
│   ├── env-separator.md
│   ├── scope-validator.md
│   ├── production-first.md
│   ├── three-strikes.md
│   └── error-recovery.md
└── hooks/
    ├── pre-execution.sh
    ├── post-execution.sh
    └── pre-commit.sh
```

### 2. Verify Installation

Check that agents are loaded:
```bash
ls -la .opencode/agents/
```

Check that skills are loaded:
```bash
ls -la .opencode/skills/
```

Check that hooks are executable:
```bash
ls -la .opencode/hooks/
```

Validate the main config:
```bash
python -m json.tool .opencode/oh-my-opencode.jsonc
```

## Agent JSON Format

Agents are defined in `oh-my-opencode.jsonc` (JSON with comments). Each agent has four fields:

| Field | Type | Description |
|-------|------|-------------|
| `model` | string | Model identifier (e.g., `anthropic/claude-sonnet-4`) |
| `temperature` | number | Sampling temperature (0.0 = deterministic, 1.0 = creative) |
| `prompt_append` | string | Instructions appended to the agent's system prompt |
| `permissions` | object | Tool permissions (`allow`, `ask`, `deny`) |

### Example: oh-my-opencode.jsonc

```jsonc
{
  "$schema": "https://raw.githubusercontent.com/code-yeongyu/oh-my-opencode/master/assets/oh-my-opencode.schema.json",
  "agents": {
    "guardrails-enforcer": {
      "model": "anthropic/claude-sonnet-4",
      "temperature": 0.1,
      "prompt_append": "You are the Guardrails Enforcement Agent. Before ANY operation verify: 1) File has been read, 2) Scope is authorized, 3) Rollback is known, 4) No forbidden patterns. HALT and ask if uncertain.",
      "permissions": {
        "edit": "ask",
        "bash": "ask",
        "webfetch": "allow",
        "read": "allow"
      }
    },
    "guardrails-auditor": {
      "model": "anthropic/claude-sonnet-4",
      "temperature": 0.1,
      "prompt_append": "You are a Guardrails Auditor. Review completed work for compliance...",
      "permissions": {
        "edit": "deny",
        "bash": "deny",
        "read": "allow"
      }
    }
  },
  "skills": {
    "sources": [
      {"path": "./.opencode/skills", "recursive": true}
    ],
    "enable": [
      "guardrails-enforcer",
      "commit-validator",
      "env-separator"
    ]
  }
}
```

### Permission Levels

| Level | Behavior |
|-------|----------|
| `allow` | Always permitted without prompting |
| `ask` | Prompt the user before executing |
| `deny` | Never permitted |

## Skill Markdown Format

Skills are markdown files in `.opencode/skills/` with structured sections:

```markdown
# Guardrails Enforcer

## Description
Enforces the Four Laws of Agent Safety

## Tools
- Read
- Grep
- Glob

## Instructions
You MUST enforce these rules:
1. Read before editing
2. Stay in scope
3. Verify before committing
4. Halt when uncertain
```

Sections are parsed as follows:
- **Description** - One-line summary of the skill
- **Tools** - Allowed tools for this skill
- **Instructions** - Detailed prompt injected into context

## Hook Details

Hooks are shell scripts that run automatically:

| Hook | When It Runs | Purpose |
|------|--------------|---------|
| `pre-execution.sh` | Before file modifications | Verify read-before-edit |
| `post-execution.sh` | After file modifications | Validate changes |
| `pre-commit.sh` | Before git commit | Validate commit message |

### Custom Hook Example

```bash
#!/bin/bash
# .opencode/hooks/pre-commit.sh

# Run linter
npm run lint

# Run tests
npm test

# Check for secrets
trufflehog git file://. --since-commit HEAD
```

## Shared Prompts Reference

All agent prompts and skill instructions incorporate rules from the shared prompts directory:

| Shared Prompt | Used By |
|---------------|---------|
| `skills/shared-prompts/four-laws.md` | guardrails-enforcer agent/skill |
| `skills/shared-prompts/halt-conditions.md` | guardrails-enforcer agent/skill |
| `skills/shared-prompts/three-strikes.md` | three-strikes skill |
| `skills/shared-prompts/production-first.md` | production-first skill |
| `skills/shared-prompts/clean-architecture.md` | guardrails-enforcer agent |
| `skills/shared-prompts/cqrs.md` | guardrails-enforcer agent |
| `skills/shared-prompts/scope-validation.md` | scope-validator skill |
| `skills/shared-prompts/error-recovery.md` | error-recovery skill |

When shared prompts are updated, re-run the setup script:

```bash
python scripts/setup_agents.py --install --platform opencode
```

## Customization

### Adding a Custom Agent

1. Add an entry to the `agents` object in `oh-my-opencode.jsonc`:

```jsonc
{
  "agents": {
    "my-agent": {
      "model": "anthropic/claude-haiku-4",
      "temperature": 0.0,
      "prompt_append": "Your instructions here...",
      "permissions": {
        "edit": "ask",
        "bash": "deny",
        "read": "allow"
      }
    }
  }
}
```

2. Create a corresponding skill in `.opencode/skills/my-agent.md`.
3. Add the skill name to the `skills.enable` array.

### Disabling an Agent

1. Remove it from the `agents` object in `oh-my-opencode.jsonc`.
2. Remove its skill from the `skills.enable` array.
3. Optionally move agent/skill files to a `disabled/` subdirectory.

### Cloning a Single Skill

```bash
python scripts/setup_agents.py --install-skill guardrails-enforcer --platform opencode
```

## Installation Modes

| Mode | Command | Behavior |
|------|---------|----------|
| Copy | `--mode copy` (default) | Writes standalone copies to the project |
| Symlink | `--mode symlink` | Creates symlinks back to this repo |

## Troubleshooting

### Agents Not Loading

- JSON syntax: `python -m json.tool .opencode/oh-my-opencode.jsonc`
- Agent entries exist in the `agents` object
- `oh-my-opencode.jsonc` is in `.opencode/` directory

### Skills Not Loading

- Markdown files have proper `## Description`, `## Tools`, `## Instructions` sections
- Files are in `.opencode/skills/` directory
- Skill names appear in `skills.enable` array

### Hooks Not Running

- Check executable bit: `chmod +x .opencode/hooks/*.sh`
- Validate shell syntax: `bash -n .opencode/hooks/pre-execution.sh`
- Hook names match expected patterns in config

### Permission Denied

```bash
chmod +x .opencode/hooks/*.sh
```

## Best Practices

1. **One agent = one responsibility** - Keep agents focused and composable
2. **Use low temperature for guardrails** - Deterministic enforcement (0.0-0.1)
3. **Test hooks manually** - Run scripts directly to verify behavior
4. **Regenerate after shared prompt updates** - Re-run setup to sync
5. **Commit `.opencode/` to version control** - Team shares the same guardrails

## References

- [AGENTS_AND_SKILLS_SETUP.md](AGENTS_AND_SKILLS_SETUP.md) - Unified setup guide
- [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md) - Core safety protocols
- [skills/shared-prompts/](../skills/shared-prompts/) - Canonical prompt definitions
