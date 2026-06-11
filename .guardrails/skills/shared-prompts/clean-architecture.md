# Clean Architecture for Agent Development

Apply Clean Architecture principles to keep code maintainable and testable as features grow.

## Layer Structure (Inside-Out)

```
┌─────────────────────────────────────────┐
│  Interface Adapters (MCP handlers, CLI) │ ← How code is called
├─────────────────────────────────────────┤
│  Application (Commands, Queries)        │ ← Orchestration, use cases
├─────────────────────────────────────────┤
│  Domain (Entities, Value Objects)       │ ← Core business rules
├─────────────────────────────────────────┤
│  Infrastructure (DB, Cache, External)  │ ← Implementations
└─────────────────────────────────────────┘
```

## Dependency Rule

**Dependencies point inward only.** Inner layers know nothing about outer layers.

- Domain layer has ZERO external dependencies
- Application layer depends only on Domain
- Interface adapters depend on Application and Domain
- Infrastructure implements ports defined by Domain

## Key Patterns

### Dependency Inversion

```go
// Bad: High-level module depends on low-level detail
func (s *MCPServer) validate(cmd string) {
    engine := NewValidationEngine() // ❌ concrete
}

// Good: Depend on interface (port)
func (s *MCPServer) validate(cmd string, svc GuardrailService) {
    // ✓ interface, swappable
}
```

### Ports and Adapters

- **Port**: Interface defined by Domain layer (what we need)
- **Adapter**: Implementation in Infrastructure layer (how we provide it)

```go
// Domain defines the port
type RuleRepository interface {
    GetByID(ctx context.Context, id uuid.UUID) (*Rule, error)
    Create(ctx context.Context, rule *Rule) error
}

// Infrastructure implements the adapter
type PostgresRuleStore struct{ ... } // satisfies RuleRepository
```

### Value Objects

Domain objects with no identity, immutable, self-validating:

```go
type Severity string

const (
    SeverityCritical Severity = "critical"
    SeverityHigh     Severity = "high"
)

func (s Severity) IsValid() bool {
    return s == SeverityCritical || s == SeverityHigh
}
```

### Aggregate Roots

Group related entities that change together:

```go
type PreventionRule struct {
    ID       uuid.UUID  // aggregate root
    Violations []Violation // owned by this aggregate
}
```

## Open-Closed Principle

**Open for extension, closed for modification.**

```go
// Bad: Engine needs modification for new rule type
func (e *Engine) Evaluate(cmd string) {
    if rule.Type == "bash" { ... }
    if rule.Type == "git" { ... }
    if rule.Type == "file" { ... } // add more → modify engine
}

// Good: Each rule type is a self-contained evaluator
type RuleEvaluator interface {
    Evaluate(ctx context.Context, input string) []Violation
}

// New rule type = new struct, implement interface, register
// Engine never changes
```

## Single Responsibility

Each type has one reason to change:

| Type | Responsibility |
|------|---------------|
| `MCPServer` | HTTP/MCP transport only |
| `GuardrailHandlers` | CQRS dispatch only |
| `BashEvaluator` | Bash pattern matching only |
| `RuleStore` | Database operations only |

## Vertical Slices

Group code by feature, not by layer:

```
internal/guardrails/
├── bash/           ← All bash-related code together
│   ├── rule.go     ← Model
│   ├── evaluator.go ← Business logic
│   └── handler.go  ← MCP handler
├── git/            ← All git-related code together
│   └── ...
```

**Not:**
```
internal/
├── models/         ← Scattered across features
├── handlers/       ← All features mixed
└── validation/     ← Too many responsibilities
```

## Applying to Agent Tasks

When building with these patterns:

1. **Define domain interfaces first** — what do you need, not how
2. **Implement infrastructure adapters** — how you provide it
3. **Wire in main.go** — composition root
4. **Test at boundaries** — mock interfaces, test adapters
5. **Keep domain pure** — no database, no HTTP, no external deps

## Reference

- `docs/ARCHITECTURE_CLEAN_CQRS.md` — Full architecture map
- `internal/domain/` — Domain layer interfaces
- `internal/adapters/` — Infrastructure adapters
