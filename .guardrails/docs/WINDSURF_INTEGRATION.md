# Windsurf Integration

This guide explains how to integrate Agent Guardrails with Windsurf using the `.windsurfrules` file.

## Overview

Windsurf reads a single file called `.windsurfrules` at the project root. This file contains markdown-formatted rules that apply to all code generation, edits, and suggestions in the project.

There are no per-skill files or profiles -- all rules live in one file.

## Setup

### 1. Run Setup Script

```bash
python scripts/setup_agents.py --install --platform windsurf
```

This creates:
```
.windsurfrules
```

The file is placed at the project root, which is where Windsurf expects it.

### 2. Manual Installation

If you prefer to install without the script:

```bash
# Copy from template
cp .windsurfrules /path/to/your/project/

# Or symlink (keeps in sync with this repo)
ln -s /mnt/data/git/agent-guardrails-template/.windsurfrules /path/to/your/project/.windsurfrules
```

### 3. Verify Installation

```bash
cat .windsurfrules
```

You should see the guardrails rules beginning with `# WINDSURF GUARDRAILS`.

## File Format

The `.windsurfrules` file is a plain markdown document with no frontmatter. Windsurf reads the entire file as a rules preamble injected into every session.

### Structure

```markdown
# WINDSURF GUARDRAILS

These rules apply to ALL code generation, edits, and suggestions in this project.

## The Four Laws of Agent Safety

1. **Read Before Editing** - Never modify code without reading it first.
2. **Stay in Scope** - Only touch files explicitly authorized.
3. **Verify Before Committing** - Test and check all changes.
4. **Halt When Uncertain** - Ask for clarification instead of guessing.

## Pre-Operation Checklist (MANDATORY)
...

## Forbidden Actions (NEVER DO)
...

## Halt Conditions (STOP and Ask User)
...
```

### Key Sections

| Section | Purpose |
|---------|---------|
| The Four Laws | Core safety rules applied to every operation |
| Pre-Operation Checklist | Mandatory checks before any file modification |
| Forbidden Actions | Actions that must never be performed |
| Halt Conditions | Conditions that trigger a stop-and-ask |
| Three Strikes Rule | Failure recovery protocol |
| Production-First Rule | Required ordering of production code before tests |
| Scope Rules | File authorization hierarchy |
| Architecture Patterns | Clean Architecture, CQRS, SOLID for Go/MCP code |

## How It Applies

Windsurf loads `.windsurfrules` as a persistent context for every chat session in the project. This means:

- Every code suggestion respects the Four Laws
- The pre-operation checklist is mandatory before edits
- Forbidden actions are never suggested
- Halt conditions trigger explicit user confirmation
- Architecture patterns apply when working on `mcp-server/`

## Customization

### Adding Project-Specific Rules

Append custom sections to `.windsurfrules`:

```markdown
## Project-Specific Rules

- Use TypeScript strict mode for all new files
- All API endpoints must have OpenAPI annotations
- Database queries must use the query builder, never raw SQL
```

### Using Symlinks for Shared Rules

If multiple projects share the same guardrails, symlink the file:

```bash
ln -s /shared/guardrails/.windsurfrules .windsurfrules
```

This keeps all projects in sync when the shared file is updated.

### Per-Team Overrides

Teams that need different rules can maintain their own `.windsurfrules` in a branch or fork. Merge upstream changes periodically to stay current.

## Shared Prompts Reference

The `.windsurfrules` file incorporates rules from the shared prompts directory:

| Shared Prompt | Rules Covered |
|---------------|---------------|
| `skills/shared-prompts/four-laws.md` | The Four Laws (canonical source) |
| `skills/shared-prompts/halt-conditions.md` | Full halt conditions checklist |
| `skills/shared-prompts/three-strikes.md` | Failure tracking and escalation |
| `skills/shared-prompts/production-first.md` | Production-before-tests ordering |
| `skills/shared-prompts/clean-architecture.md` | Clean Architecture patterns for Go/MCP |
| `skills/shared-prompts/cqrs.md` | CQRS command/query separation |
| `skills/shared-prompts/scope-validation.md` | Scope boundary enforcement |

When shared prompts are updated, re-run the setup script to regenerate `.windsurfrules`:

```bash
python scripts/setup_agents.py --install --platform windsurf
```

## Installation Modes

| Mode | Command | Behavior |
|------|---------|----------|
| Copy | `--mode copy` (default) | Writes a standalone copy to the project |
| Symlink | `--mode symlink` | Creates a symlink back to this repo |

Use symlink mode when you want changes in this repo to propagate automatically. Use copy mode for standalone projects that should not depend on this repo.

## Troubleshooting

### Rules Not Applied

**Check:**
- File exists at project root: `ls -la .windsurfrules`
- File is not empty: `wc -l .windsurfrules`
- Windsurf has indexed the project (restart Windsurf if needed)

### Outdated Rules

**Fix:**
- Re-run: `python scripts/setup_agents.py --install --platform windsurf`
- Or update the symlink target

### Conflicting Rules in CLAUDE.md

If the project also has a `CLAUDE.md`, Windsurf reads both. Rules in `.windsurfrules` take precedence for Windsurf-specific behavior. Use `CLAUDE.md` for Claude Code and `.windsurfrules` for Windsurf.

## Best Practices

1. **Regenerate after updates** - Re-run setup when shared prompts change
2. **Keep it focused** - The file is read in full every session; avoid bloating it
3. **Use symlinks for teams** - Keeps all projects on the same rules version
4. **Version control** - Commit `.windsurfrules` so the team shares the same guardrails

## References

- [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md) - Core safety protocols
- [AGENTS_AND_SKILLS_SETUP.md](AGENTS_AND_SKILLS_SETUP.md) - Unified setup guide
- [skills/shared-prompts/](../skills/shared-prompts/) - Canonical prompt definitions
