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

## Three Strikes Rule

- Strike 1: Retry with adjusted approach
- Strike 2: Try alternative approach
- Strike 3: HALT and escalate to user

Never continue beyond 3 failures.

## Architecture Patterns (Go/MCP Server)

When working on the MCP server:

### Clean Architecture Layers

1. **Domain** (`internal/domain/`) — Interfaces, value objects, zero external deps
2. **Application** — Command/query handlers
3. **Adapters** (`internal/adapters/`) — Infrastructure implementations
4. **Interface** (`internal/mcp/`) — MCP handlers

### Dependency Rule

Outer layers depend inward. Domain has no dependencies.

### CQRS

- **Commands**: Create, Update, Delete (write operations)
- **Queries**: Evaluate, List, Get (read operations, cache-friendly)

### Vertical Slices

Group by feature: `internal/guardrails/bash/`, `internal/guardrails/git/`, etc.

## References

- `skills/shared-prompts/clean-architecture.md` — Clean Architecture patterns
- `skills/shared-prompts/cqrs.md` — CQRS command/query separation
- `skills/shared-prompts/four-laws.md` — Canonical Four Laws
- `skills/shared-prompts/three-strikes.md` — Failure handling
