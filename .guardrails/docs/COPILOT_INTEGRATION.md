# GitHub Copilot Integration

This guide explains how to integrate Agent Guardrails with GitHub Copilot using repo-level instructions.

## Overview

GitHub Copilot reads a file at `.github/copilot-instructions.md` for repo-level instructions. This file provides markdown-formatted guidance that applies to all Copilot completions, suggestions, and chat interactions in the repository.

There are no per-skill files -- all instructions live in one file.

## Setup

### 1. Run Setup Script

```bash
python scripts/setup_agents.py --install --platform copilot
```

This creates:
```
.github/
└── copilot-instructions.md
```

### 2. Manual Installation

If you prefer to install without the script:

```bash
# Copy from template
cp .github/copilot-instructions.md /path/to/your/project/.github/copilot-instructions.md
```

Copilot does not support symlinks for instructions -- use copy mode only.

### 3. Verify Installation

```bash
cat .github/copilot-instructions.md
```

You should see the guardrails instructions beginning with `# GitHub Copilot Instructions`.

## File Format

The `.github/copilot-instructions.md` file is a plain markdown document. GitHub Copilot reads this as project-level instructions injected into every chat session and code suggestion context.

### Structure

```markdown
# GitHub Copilot Instructions

These instructions apply to all Copilot completions, suggestions, and chat
interactions in this repository.

## The Four Laws of Agent Safety

1. **Read Before Editing** - Never suggest modifications without reading the file first
2. **Stay in Scope** - Only work on files within the authorized task scope
3. **Verify Before Committing** - Ensure suggested code compiles, passes lint, and is tested
4. **Halt When Uncertain** - Ask for clarification instead of guessing

## Code Generation Rules
...

## Forbidden Patterns
...

## Three Strikes Rule
...
```

### Key Sections

| Section | Purpose |
|---------|---------|
| The Four Laws | Core safety rules for all operations |
| Code Generation Rules | Scope, production-first, error handling, security |
| Forbidden Patterns | Code that must never be suggested |
| Three Strikes Rule | Failure recovery protocol |
| File Headers | Required header format for new files |
| Architecture Patterns | Clean Architecture, CQRS, SOLID for Go/MCP code |

## How It Applies

Copilot reads `.github/copilot-instructions.md` at the repository level. This means:

- All inline suggestions respect the scope and production-first rules
- Chat responses apply the Four Laws
- Forbidden patterns are never suggested
- Architecture patterns apply when working on `mcp-server/`
- File headers are included in new file suggestions

### Scope in Copilot

Unlike agentic tools (Claude Code, Cursor), Copilot operates at the suggestion level. The scope rules translate as:

- Only suggest changes in the file being edited
- Do not suggest refactoring unrelated code
- Do not suggest adding new files unless the user requests it
- When unclear about intent, do not assume -- suggest the minimal change

## Customization

### Adding Project-Specific Instructions

Append custom sections to `.github/copilot-instructions.md`:

```markdown
## Project-Specific Rules

- All new Python files must use type hints
- API endpoints must follow OpenAPI 3.1 spec
- Use pytest fixtures, not unittest classes
```

### Team-Level Instructions

Since this file lives in `.github/`, it is committed to the repository and shared with the entire team. All contributors get the same guardrails automatically.

### Combining with Personal Instructions

GitHub Copilot also supports personal instructions in your Copilot settings. Repo-level instructions (`.github/copilot-instructions.md`) take precedence over personal instructions for files in this repository.

## Shared Prompts Reference

The `copilot-instructions.md` file incorporates rules from the shared prompts directory:

| Shared Prompt | Rules Covered |
|---------------|---------------|
| `skills/shared-prompts/four-laws.md` | The Four Laws (canonical source) |
| `skills/shared-prompts/halt-conditions.md` | Full halt conditions checklist |
| `skills/shared-prompts/three-strikes.md` | Failure tracking and escalation |
| `skills/shared-prompts/production-first.md` | Production-before-tests ordering |
| `skills/shared-prompts/clean-architecture.md` | Clean Architecture patterns for Go/MCP |
| `skills/shared-prompts/cqrs.md` | CQRS command/query separation |

When shared prompts are updated, re-run the setup script to regenerate the instructions:

```bash
python scripts/setup_agents.py --install --platform copilot
```

## Troubleshooting

### Instructions Not Applied

**Check:**
- File exists at `.github/copilot-instructions.md`: `ls -la .github/copilot-instructions.md`
- The `.github/` directory is committed to the repository
- Your IDE has the GitHub Copilot extension installed and enabled
- Copilot has indexed the repository (restart your IDE if needed)

### Outdated Instructions

**Fix:**
- Re-run: `python scripts/setup_agents.py --install --platform copilot`

### Conflicting Personal Instructions

If you have personal Copilot instructions that conflict with repo-level instructions, the repo-level file takes precedence for this repository. Remove conflicting personal settings or align them with the guardrails.

### Instructions Too Long

If the file becomes very long, Copilot may truncate it. Keep the file focused on the most critical rules. Move detailed patterns to referenced documents like `AGENT_GUARDRAILS.md` and `shared-prompts/`.

## Best Practices

1. **Regenerate after updates** - Re-run setup when shared prompts change
2. **Keep it concise** - Copilot has context limits; prioritize critical rules
3. **Commit to the repo** - All team members get guardrails automatically
4. **Reference, don't duplicate** - Point to shared-prompts/ for full details

## References

- [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md) - Core safety protocols
- [AGENTS_AND_SKILLS_SETUP.md](AGENTS_AND_SKILLS_SETUP.md) - Unified setup guide
- [skills/shared-prompts/](../skills/shared-prompts/) - Canonical prompt definitions
