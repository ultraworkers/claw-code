# GitHub Copilot Instructions

These instructions apply to all Copilot completions, suggestions, and chat interactions in this repository.

## The Four Laws of Agent Safety

1. **Read Before Editing** - Never suggest modifications without reading the file first
2. **Stay in Scope** - Only work on files within the authorized task scope
3. **Verify Before Committing** - Ensure suggested code compiles, passes lint, and is tested
4. **Halt When Uncertain** - Ask for clarification instead of guessing

## Code Generation Rules

### Scope (MANDATORY)

- Only modify files explicitly requested by the user
- Do not refactor unrelated code "while I'm here"
- Do not add new files unless asked
- Do not delete files unless asked
- When scope is unclear: ask for confirmation

### Production-First

- Production code must be written before test code
- Tests written against existing production code, not stubs
- Infrastructure code comes after both production and tests

### Error Handling

- Validate inputs and handle errors explicitly
- Return meaningful error messages
- Never silently swallow exceptions
- Prefer explicit error returns over panics/exceptions

### Security

- Never suggest committing secrets, keys, or credentials
- Never suggest .env files in version control
- Validate all external inputs
- Use parameterized queries, never string concatenation for SQL

## Forbidden Patterns

NEVER suggest code that:
- Modifies files that haven't been read
- Mixes test and production environments
- Commits secrets or credentials
- Runs untested code in production
- Makes assumptions about user intent

## Three Strikes Rule

When a suggestion is rejected or produces errors:
- 1st failure: Adjust approach and retry
- 2nd failure: Try completely different approach
- 3rd failure: HALT and ask user for guidance

## File Headers

Include at the top of new files:

```go
// File: <filename>
// Purpose: <one-line description>
// Created: <date>
// Author: AI-assisted (see git history)
```

Or equivalent in the target language.

## Architecture Patterns (Go/MCP Server)

When working on `mcp-server/`:

### Layer Order (Inside-Out)

1. **Domain** (`internal/domain/`) — Interfaces, value objects, ZERO external deps
2. **Application** — Command/query handlers
3. **Adapters** (`internal/adapters/`) — Infrastructure implementations
4. **Interface** (`internal/mcp/`) — MCP handlers

### Dependency Rule

Outer layers depend inward. Domain is pure — no database, no HTTP imports.

### CQRS Pattern

| Commands (Write) | Queries (Read) |
|-----------------|----------------|
| Create | Evaluate |
| Update | List |
| Delete | Get |
| Toggle | GetViolations |

Commands: validate → persist → publish event (cache invalidation)
Queries: cache-first, no state modification

### Vertical Slices

Group all code for one feature together:

```
internal/guardrails/
├── bash/           ← model + evaluator + handler
├── git/
└── fileedit/
```

### SOLID

- **S**: One responsibility per type
- **O**: Add new evaluator via interface, don't modify existing code
- **L**: Implement interfaces fully
- **I**: Small focused interfaces
- **D**: Depend on interface, not concrete type

### Forbidden (Architecture)

- Importing database packages in domain types
- Putting concrete implementations in domain layer
- Cross-layer circular dependencies
- Adding infrastructure logic in handlers

## References

- `skills/shared-prompts/four-laws.md` - The Four Laws (canonical)
- `skills/shared-prompts/clean-architecture.md` - Clean Architecture patterns
- `skills/shared-prompts/cqrs.md` - CQRS details
- `docs/AGENT_GUARDRAILS.md` - Core safety protocols
- `docs/standards/TEST_PRODUCTION_SEPARATION.md` - Environment isolation
- `docs/workflows/COMMIT_WORKFLOW.md` - Commit standards
