# Clean Architecture & CQRS Architecture Map

**Purpose:** Unified architecture reference for the MCP server. Overlays the current structure with the target Clean Architecture + CQRS design.

---

## Current Layer Diagram

```
┌─────────────────────────────────────────────────────┐
│  Interface Adapters                                  │
│  internal/mcp/          ← MCP handlers, HTTP/SSE   │
├─────────────────────────────────────────────────────┤
│  Application Layer                                  │
│  internal/mcp/handlers.go ← CQRS command/query      │
│                            handlers                  │
├─────────────────────────────────────────────────────┤
│  Domain Layer                                        │
│  internal/domain/       ← Interfaces (ports),       │
│                            value objects, CQRS       │
│                            commands/queries          │
├─────────────────────────────────────────────────────┤
│  Infrastructure                                      │
│  internal/adapters/     ← Concrete implementations: │
│  internal/validation/   ← ValidationEngine,         │
│  internal/database/    ← RuleStore, PostgreSQL     │
│  internal/cache/        ← Redis, circuit breaker     │
└─────────────────────────────────────────────────────┘
```

## Before vs After

| Aspect | Before | After |
|--------|--------|-------|
| MCPServer depends on | `*validation.ValidationEngine` | `domain.GuardrailService` interface |
| Domain layer | Empty package | Pure interfaces, value objects |
| Infrastructure impl | Inside domain | `internal/adapters/` |
| Vertical slices | Scattered across layers | One dir per guardrail type |
| Cache invalidation | Manual after each write | Event-driven via EventBus |

## Event Flow (CQRS)

```
Command: CreateRule
  └─ CreateRuleHandler.Handle()
      ├─ 1. Validate pattern (PatternMatcher)
      ├─ 2. Persist (RuleRepository)
      └─ 3. Publish Event (EventBus)
           └─ CacheInvalidationHandler.Handle()
                └─ CachePort.InvalidateRules()
```

```
Query: EvaluateCommand
  └─ EvaluateCommandHandler.Handle()
      ├─ 1. Check cache (CachePort)
      └─ 2. Evaluate (GuardrailService)
           └─ BashEvaluator.Evaluate()
```

## Vertical Slice Structure

```
internal/guardrails/
├── bash/
│   └── slice.go          ← Rule model, Evaluator, Store, Cache, Handler
├── git/
│   └── slice.go          ← Same pattern
└── fileedit/
    └── slice.go          ← Same pattern
```

Each slice is self-contained: model + business logic + handler in one package.

## Interface Definitions

```go
// Domain port — GuardrailService
type GuardrailService interface {
    EvaluateCommand(ctx context.Context, command string) ([]Violation, error)
    EvaluateGit(ctx context.Context, command string) ([]Violation, error)
    EvaluateFileEdit(ctx context.Context, filePath, content, sessionID string) ([]Violation, error)
    EvaluateInput(ctx context.Context, input string, categories []string) ([]Violation, error)
    CheckFileRead(ctx context.Context, sessionID, filePath string) (*FileReadVerification, error)
}

// Domain port — RuleRepository
type RuleRepository interface {
    GetByID(ctx context.Context, id uuid.UUID) (*PreventionRule, error)
    GetByRuleID(ctx context.Context, ruleID string) (*PreventionRule, error)
    List(ctx context.Context, enabled *bool, category string, limit, offset int) ([]PreventionRule, error)
    GetActiveRules(ctx context.Context) ([]PreventionRule, error)
    Create(ctx context.Context, rule *PreventionRule) error
    Update(ctx context.Context, rule *PreventionRule) error
    Delete(ctx context.Context, id uuid.UUID) error
    Toggle(ctx context.Context, id uuid.UUID, enabled bool) error
    Count(ctx context.Context, enabled *bool, category string) (int, error)
}

// Domain port — EventBus
type EventBus interface {
    Publish(ctx context.Context, event Event)
    Subscribe(eventType EventType, handler EventHandler)
}
```

## CQRS Command/Query Handlers

| Command Handler | Operation | Event Published |
|-----------------|-----------|-----------------|
| `CreateRuleHandler` | Insert rule | `rule.created` |
| `UpdateRuleHandler` | Modify rule | `rule.updated` |
| `ToggleRuleHandler` | Enable/disable | `rule.toggled` |

| Query Handler | Operation | Caching |
|---------------|-----------|---------|
| `EvaluateCommandHandler` | Validate bash | Cache-first |
| `EvaluateGitHandler` | Validate git | Cache-first |
| `EvaluateFileEditHandler` | Validate file edit | Cache-first |
| `ListRulesHandler` | List rules | No caching |

## Testability

Domain types testable in isolation (no external deps):

```go
// Test domain value objects
func TestSeverityIsValid(t *testing.T) {
    assert.Equal(t, true, Severity("critical").IsValid())
    assert.Equal(t, false, Severity("invalid").IsValid())
}

// Test CQRS handlers with mocks
func TestCreateRuleHandler(t *testing.T) {
    mockRepo := &mockRuleRepository{}
    mockBus := &mockEventBus{}
    h := NewCreateRuleHandler(mockRepo, mockBus, mockMatcher)

    rule, err := h.Handle(ctx, CreateRuleCommand{...})

    assert.NoError(t, err)
    assert.Len(t, mockBus.PublishedEvents, 1)
}
```

## Migration Path

1. **Phase 1** (done): Domain interfaces + adapters — existing engine wrapped behind interface
2. **Phase 2** (done): Vertical slices — bash/git/fileedit self-contained
3. **Phase 3** (in progress): Wire MCPServer to use interface instead of concrete type
4. **Phase 4** (future): Bounded contexts — `internal/team/` and `internal/validation/` become separate packages

## Key Files

| File | Role |
|------|------|
| `internal/domain/guardrail.go` | Domain interfaces and value objects |
| `internal/domain/cqrs.go` | CQRS commands, queries, handlers, EventBus interface |
| `internal/adapters/validation_adapter.go` | ValidationEngine behind GuardrailService |
| `internal/adapters/database_adapter.go` | RuleStore behind RuleRepository |
| `internal/adapters/event_bus.go` | DefaultEventBus, CacheInvalidationHandler |
| `internal/guardrails/registry.go` | Unified registry wiring all slices |
| `internal/guardrails/bash/slice.go` | Bash vertical slice |
| `internal/guardrails/git/slice.go` | Git vertical slice |
| `internal/guardrails/fileedit/slice.go` | File edit vertical slice |

---

**Last Updated:** 2026-05-08
