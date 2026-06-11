---
description: Clean Architecture and CQRS patterns for Go MCP server development
globs: "mcp-server/**/*.go"
alwaysApply: false
---

# Clean Architecture & CQRS

## Layer Order (Inside-Out)

1. **Domain** (`internal/domain/`) — Interfaces, value objects, no deps
2. **Application** — Use cases, command/query handlers
3. **Adapters** (`internal/adapters/`) — Infrastructure implementations
4. **Interface** (`internal/mcp/`) — MCP handlers, HTTP endpoints

## Dependency Rule

Outer depends inward. Domain is pure.

```go
// Domain defines interface
type GuardrailService interface {
    EvaluateCommand(ctx context.Context, cmd string) ([]Violation, error)
}

// Infrastructure implements
type ValidationEngineAdapter struct {
    engine *validation.ValidationEngine
}

var _ domain.GuardrailService = (*ValidationEngineAdapter)(nil)
```

## CQRS Split

| Commands (Write) | Queries (Read) |
|-----------------|----------------|
| CreateRule | Evaluate |
| UpdateRule | List |
| DeleteRule | Get |
| LogViolation | GetViolations |

Commands: validate → persist → publish event
Queries: cache-first → evaluate → return result

## Vertical Slices

```
internal/guardrails/
├── bash/           ← All bash logic together
│   ├── rule.go
│   ├── evaluator.go
│   └── handler.go
├── git/
└── fileedit/
```

## SOLID

- **S**: One class, one reason to change
- **O**: Add new evaluator, don't modify engine
- **L**: Implement interface fully
- **I**: Small interfaces (3 methods, not 30)
- **D**: Depend on abstraction (interface)

## Never Do

- Import db in domain types
- Put concrete impl in domain layer
- Cross-layer circular deps
- Add infra logic to handlers
