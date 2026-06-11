# Cursor Integration

This guide explains how to integrate Agent Guardrails with Cursor using markdown-based rules.

## Overview

Cursor supports:
- **Rules** - Markdown files with YAML frontmatter that define AI behavior and constraints
- **Global Rules** - `.cursorrules` file in project root for universal settings

The setup script installs these configurations for you.

## Setup

### 1. Install All Rules

```bash
python scripts/setup_agents.py --install --platform cursor
```

This creates:
```
.cursor/
├── rules/
│   ├── guardrails-enforcer.md
│   ├── commit-validator.md
│   ├── env-separator.md
│   ├── scope-validator.md
│   ├── production-first.md
│   ├── three-strikes.md
│   └── error-recovery.md
└── .cursorrules (optional root config)
```

### 2. Install a Single Skill

To install just one skill by name:

```bash
python scripts/setup_agents.py --install-skill guardrails-enforcer --platform cursor
```

Use `--list-skills` to see all available skill names:

```bash
python scripts/setup_agents.py --list-skills
```

### 3. Verify Installation

Check that rules are loaded:
```bash
ls -la .cursor/rules/
```

Check that `.cursorrules` exists (if using global config):
```bash
cat .cursorrules
```

## Rule File Format

Rules are markdown files in `.cursor/rules/` with YAML frontmatter:

```markdown
---
description: Enforces the Four Laws of Agent Safety on all code generation
globs: "**/*"
alwaysApply: true
---

# Guardrails Enforcement

You are the Guardrails Enforcement Agent. Enforce these rules on EVERY operation.

## The Four Laws of Agent Safety

1. **Read Before Editing** - Never modify code without reading it first
2. **Stay in Scope** - Only touch files explicitly authorized
3. **Verify Before Committing** - Test and check all changes
4. **Halt When Uncertain** - Ask for clarification instead of guessing
...
```

### Frontmatter Fields

| Field | Type | Description |
|-------|------|-------------|
| `description` | string | Summary shown in the Cursor rules list |
| `globs` | string | File patterns this rule applies to (e.g., `"**/*"`, `"src/**/*.ts"`) |
| `alwaysApply` | boolean | Whether to apply this rule to every session automatically |

### Example: guardrails-enforcer.md (Actual File)

```markdown
---
description: Enforces the Four Laws of Agent Safety on all code generation
globs: "**/*"
alwaysApply: true
---

# Guardrails Enforcement

You are the Guardrails Enforcement Agent. Enforce these rules on EVERY operation.

## The Four Laws of Agent Safety

1. **Read Before Editing** - Never modify code without reading it first
2. **Stay in Scope** - Only touch files explicitly authorized
3. **Verify Before Committing** - Test and check all changes
4. **Halt When Uncertain** - Ask for clarification instead of guessing

## Pre-Operation Checklist

Before ANY file modification:
- [ ] Read the target file(s) completely
- [ ] Verify the operation is within authorized scope
- [ ] Identify the rollback procedure
- [ ] Check for test/production separation requirements

## Forbidden Actions

1. Modifying code without reading it first
2. Mixing test and production environments
3. Force pushing to main/master
4. Committing secrets, credentials, or .env files
5. Running untested code in production
6. Modifying unread code
7. Working outside authorized scope

## Halt Conditions

STOP and escalate when:
- Attempting to modify code you haven't read
- No rollback procedure exists or is unclear
- Production impact is uncertain
- User authorization is ambiguous
- Test and production environments may mix
- You are uncertain about ANY aspect of the task
- An operation has failed 3 times
```

## Global Rules (.cursorrules)

The `.cursorrules` file in the project root applies to all Cursor sessions:

```markdown
# Project Rules

## Always

- Follow the Four Laws of Agent Safety
- Read files before editing
- Validate commits before creating

## When

- Editing code: Check scope boundaries
- Running commands: Verify environment separation
```

Use `.cursorrules` for simple, universal rules. Use `.cursor/rules/*.md` for structured, per-skill rules with frontmatter.

## Rule Reference

### guardrails-enforcer

Applies to all files. Enforces the Four Laws, pre-operation checklist, forbidden actions, and halt conditions.

### commit-validator

Validates git commits. Checks AI attribution, single focus, no secrets, tests pass.

### env-separator

Enforces test/production separation. Detects shared instances, production DB in tests.

### scope-validator

Enforces scope boundaries. Only explicitly authorized files may be modified.

### production-first

Requires production code before tests. Order: implementation, validation, tests, infrastructure.

### three-strikes

Failure recovery. After three consecutive failures, halts and escalates to user.

### error-recovery

Error handling procedures. Provides structured guidance when operations fail.

## Shared Prompts Reference

All rule markdown files incorporate rules from the shared prompts directory:

| Shared Prompt | Used By Rules |
|---------------|---------------|
| `skills/shared-prompts/four-laws.md` | guardrails-enforcer |
| `skills/shared-prompts/halt-conditions.md` | guardrails-enforcer |
| `skills/shared-prompts/three-strikes.md` | three-strikes |
| `skills/shared-prompts/production-first.md` | production-first |
| `skills/shared-prompts/clean-architecture.md` | guardrails-enforcer |
| `skills/shared-prompts/cqrs.md` | guardrails-enforcer |
| `skills/shared-prompts/scope-validation.md` | scope-validator |
| `skills/shared-prompts/error-recovery.md` | error-recovery |

When shared prompts are updated, re-run the setup script:

```bash
python scripts/setup_agents.py --install --platform cursor
```

## Customization

### Adding a Custom Rule

1. Create a new markdown file in `.cursor/rules/`:

```markdown
---
description: Custom TypeScript strict mode enforcement
globs: "src/**/*.ts"
alwaysApply: false
---

## Always

- Use strict mode for all TypeScript files
- No `any` types allowed
- All functions must have return type annotations
```

2. Cursor automatically loads rules from this directory.

### Rule Priority

Rules are applied in order:
1. `.cursorrules` (global) - Applied first
2. `.cursor/rules/*.md` - Applied in alphabetical order

Later rules can override earlier ones for the same context.

### Conditional Rules with Globs

Use `globs` to target specific file patterns (e.g., `"**/*.py"`, `"src/**/*.ts"`). Set `alwaysApply: true` for safety rules that must always be active.

### Disabling a Rule

Move it out of the rules directory:
```bash
mkdir -p .cursor/rules/disabled
mv .cursor/rules/commit-validator.md .cursor/rules/disabled/
```

Cursor will stop loading it immediately.

## Installation Modes

| Mode | Command | Behavior |
|------|---------|----------|
| Copy | `--mode copy` (default) | Writes standalone copies to the project |
| Symlink | `--mode symlink` | Creates symlinks back to this repo |

## Troubleshooting

### Rules Not Loading

- Check frontmatter: `---` delimiters with valid YAML
- Files in `.cursor/rules/` with `.md` extension
- Restart Cursor to reload rules

### .cursorrules Not Applied

- File is in project root (not `.cursor/`)
- File is named exactly `.cursorrules` (no extension)
- Restart Cursor to re-index

### Rules Being Ignored

- Save all rule files
- Restart Cursor to reload
- Check for conflicting rules (later rules override earlier ones)
- Verify `globs` pattern matches the files being edited

## Best Practices

1. **One rule = one responsibility** - Keep rules focused and composable
2. **Always include frontmatter** - `description`, `globs`, `alwaysApply` are required
3. **Use `alwaysApply: true` for guardrails** - Safety rules should not be optional
4. **Regenerate after shared prompt updates** - Re-run setup to sync rules
5. **Commit `.cursor/` and `.cursorrules`** - Team shares the same guardrails

## References

- [AGENTS_AND_SKILLS_SETUP.md](AGENTS_AND_SKILLS_SETUP.md) - Unified setup guide
- [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md) - Core safety protocols
- [skills/shared-prompts/](../skills/shared-prompts/) - Canonical prompt definitions
