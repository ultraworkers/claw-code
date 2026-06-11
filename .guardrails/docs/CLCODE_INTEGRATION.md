# Claude Code Integration

This guide explains how to integrate Agent Guardrails with Claude Code using skills and hooks.

## Overview

Claude Code supports:
- **Skills** - JSON files that define specialized behaviors and constraints
- **Hooks** - Shell scripts that run at specific lifecycle points

The setup script installs these configurations for you.

## Setup

### 1. Install All Skills and Hooks

```bash
python scripts/setup_agents.py --install --platform claude
```

This creates:
```
.claude/
├── skills/
│   ├── guardrails-enforcer.json
│   ├── commit-validator.json
│   ├── env-separator.json
│   ├── scope-validator.json
│   ├── production-first.json
│   ├── three-strikes.json
│   └── error-recovery.json
└── hooks/
    ├── pre-execution.sh
    ├── post-execution.sh
    └── pre-commit.sh
```

### 2. Install a Single Skill

To install just one skill by name:

```bash
python scripts/setup_agents.py --install-skill guardrails-enforcer
```

Use `--list-skills` to see all available skill names:

```bash
python scripts/setup_agents.py --list-skills
```

### 3. Verify Installation

Check that skills are loaded:
```bash
ls -la .claude/skills/
```

Validate JSON syntax:
```bash
python -m json.tool .claude/skills/guardrails-enforcer.json
```

Check that hooks are executable:
```bash
ls -la .claude/hooks/
```

## Skill File Format

Skills are JSON files in `.claude/skills/`. Each file has four fields:

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Unique identifier for the skill |
| `description` | string | What the skill does (shown in skill list) |
| `tools` | array | Allowed tools for this skill |
| `prompt` | string | Instructions injected into the session context |

### Example: guardrails-enforcer.json

```json
{
  "name": "guardrails-enforcer",
  "description": "Enforces the Four Laws of Agent Safety: read-before-edit, stay-in-scope, verify-before-commit, halt-when-uncertain",
  "tools": ["Read", "Grep", "Glob", "AskUserQuestion"],
  "prompt": "# Guardrails Enforcement Agent\n\nYou are the Guardrails Enforcement Agent. You MUST enforce these rules on EVERY operation.\n\n## The Four Laws of Agent Safety\n\n1. **Read Before Editing** - Never modify code without reading it first\n2. **Stay in Scope** - Only touch files explicitly authorized\n3. **Verify Before Committing** - Test and check all changes\n4. **Halt When Uncertain** - Ask for clarification instead of guessing\n..."
}
```

The `prompt` field contains markdown-formatted instructions. Claude Code injects this into the session context when the skill is active.

## Hook Details

Hooks are shell scripts that run automatically at specific points:

| Hook | When It Runs | Purpose |
|------|--------------|---------|
| `pre-execution.sh` | Before file modifications | Verify read-before-edit |
| `post-execution.sh` | After file modifications | Validate changes |
| `pre-commit.sh` | Before git commit | Validate commit message |

### Custom Hook Example

```bash
#!/bin/bash
# .claude/hooks/pre-commit.sh

# Run linter
npm run lint

# Run tests
npm test

# Check for secrets
trufflehog git file://. --since-commit HEAD
```

Make sure hooks remain executable:
```bash
chmod +x .claude/hooks/*.sh
```

## Skill Reference

### guardrails-enforcer

Enforces the Four Laws of Agent Safety. Halts on: unread code, scope violations, missing rollback, test/production mix, three consecutive failures.

### commit-validator

Validates git commits. Checks: AI attribution (`Co-Authored-By:`), single focus per commit, no secrets in diff, tests pass.

### env-separator

Enforces test/production separation. Detects: production DB connections in tests, shared instances, hardcoded production credentials.

### scope-validator

Enforces scope boundaries. Only files explicitly authorized by the user or task description may be modified.

### production-first

Requires production code before tests. Order: implementation, validation, tests, infrastructure.

### three-strikes

Failure recovery protocol. After three consecutive failures, halts and escalates to user.

### error-recovery

Error handling and recovery procedures. Provides structured guidance when operations fail.

## Shared Prompts Reference

All skill prompts incorporate rules from the shared prompts directory:

| Shared Prompt | Used By Skills |
|---------------|---------------|
| `skills/shared-prompts/four-laws.md` | guardrails-enforcer |
| `skills/shared-prompts/halt-conditions.md` | guardrails-enforcer |
| `skills/shared-prompts/three-strikes.md` | three-strikes |
| `skills/shared-prompts/production-first.md` | production-first |
| `skills/shared-prompts/clean-architecture.md` | guardrails-enforcer |
| `skills/shared-prompts/cqrs.md` | guardrails-enforcer |
| `skills/shared-prompts/scope-validation.md` | scope-validator |
| `skills/shared-prompts/error-recovery.md` | error-recovery |

When shared prompts are updated, re-run the setup script to regenerate skill prompts:

```bash
python scripts/setup_agents.py --install --platform claude
```

## Customization

### Adding a Custom Skill

1. Create a new JSON file in `.claude/skills/`:

```json
{
  "name": "my-skill",
  "description": "What it does",
  "tools": ["Read", "Bash"],
  "prompt": "Your instructions here..."
}
```

2. Restart Claude Code to load the skill.

### Disabling a Skill

Move it out of the skills directory:
```bash
mkdir -p .claude/skills/disabled
mv .claude/skills/commit-validator.json .claude/skills/disabled/
```

Restart Claude Code to apply.

### Cloning a Single Skill from Another Repo

```bash
python scripts/setup_agents.py --clone .claude/skills/guardrails-enforcer.json
```

This copies a specific skill file by its repo path into the current project.

## Installation Modes

| Mode | Command | Behavior |
|------|---------|----------|
| Copy | `--mode copy` (default) | Writes standalone copies to the project |
| Symlink | `--mode symlink` | Creates symlinks back to this repo |

## Troubleshooting

### Skills Not Loading

- JSON syntax: `python -m json.tool .claude/skills/*.json`
- Files in correct directory: `ls .claude/skills/`
- Restart Claude Code after changes

### Hooks Not Running

- Check executable bit: `chmod +x .claude/hooks/*.sh`
- Validate shell syntax: `bash -n .claude/hooks/pre-execution.sh`
- Check hook names match expected patterns

### Permission Denied

```bash
chmod +x .claude/hooks/*.sh
```

## Best Practices

1. **One skill = one responsibility** - Keep skills focused and composable
2. **Test hooks manually** - Run scripts directly to verify behavior
3. **Regenerate after shared prompt updates** - Re-run setup to sync skills
4. **Commit `.claude/` to version control** - Team shares the same guardrails

## References

- [AGENTS_AND_SKILLS_SETUP.md](AGENTS_AND_SKILLS_SETUP.md) - Unified setup guide
- [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md) - Core safety protocols
- [skills/shared-prompts/](../skills/shared-prompts/) - Canonical prompt definitions
