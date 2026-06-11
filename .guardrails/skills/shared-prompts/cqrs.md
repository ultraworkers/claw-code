# CQRS — Command Query Responsibility Segregation

Separate read and write operations for independent scaling, caching, and optimization.

## The Split

| Command Side (Write) | Query Side (Read) |
|---------------------|-------------------|
| Create | Evaluate |
| Update | Check |
| Delete | List |
| Enable/Disable | Get |
| Log Event | Get Violations |

Commands modify state. Queries do not.

## CQRS Pattern

```go
// COMMAND SIDE — writes modify state
type CreateRuleHandler struct {
    repo  RuleRepository
    bus   EventBus
    cache CachePort
}

func (h *CreateRuleHandler) Handle(ctx context.Context, cmd CreateRuleCommand) (*Rule, error) {
    // 1. Validate
    // 2. Persist
    // 3. Publish event for cache invalidation
    // 4. Return result
}

// QUERY SIDE — reads don't modify state (can be cached aggressively)
type EvaluateCommandHandler struct {
    guardrailSvc GuardrailService  // reads through cache
    cache        CachePort
}

func (h *EvaluateCommandHandler) Handle(ctx context.Context, q EvaluateCommandQuery) (*ValidationResult, error) {
    // 1. Check cache
    // 2. Evaluate (or return cached)
    // 3. Return result
}
```

## Event-Driven Cache Invalidation

Commands publish events. Query cache subscribes:

```go
// In command handler
bus.Publish(ctx, Event{
    Type:    EventRuleCreated,
    Payload: rule,
})

// Event bus wiring
bus.Subscribe(EventRuleCreated, cacheInvalidationHandler)
```

## Why CQRS for Guardrails

| Concern | Command Side | Query Side |
|---------|-------------|------------|
| Performance | ACID transactions | Cache-first reads |
| Scaling | Single instance | Multiple read replicas |
| Caching | No caching | Aggressive caching |
| Validation | Strict input validation | Fast pattern matching |

## Command Handlers

```go
// CreateRuleCommand — creates a new prevention rule
type CreateRuleCommand struct {
    RuleID     string   `json:"rule_id"`
    Name       string   `json:"name"`
    Pattern    string   `json:"pattern"`
    Message    string   `json:"message"`
    Severity   Severity `json:"severity"`
    Category   string   `json:"category"`
}

// UpdateRuleCommand — modifies an existing rule
type UpdateRuleCommand struct {
    ID        uuid.UUID `json:"id"`
    Name      string    `json:"name"`
    Pattern   string    `json:"pattern"`
    Enabled   bool      `json:"enabled"`
}

// LogViolationCommand — records a violation
type LogViolationCommand struct {
    Violation Violation `json:"violation"`
    SessionID string    `json:"session_id"`
}
```

## Query Handlers

```go
// EvaluateCommandQuery — evaluates a bash command
type EvaluateCommandQuery struct {
    Command    string   `json:"command"`
    Categories []string `json:"categories,omitempty"`
}

// EvaluateGitQuery — evaluates a git command
type EvaluateGitQuery struct {
    Command string `json:"command"`
}

// ListRulesQuery — retrieves rules with filters
type ListRulesQuery struct {
    Enabled  *bool  `json:"enabled,omitempty"`
    Category string `json:"category,omitempty"`
    Limit    int    `json:"limit"`
    Offset   int    `json:"offset"`
}
```

## ValidationResult

Standard response for evaluation queries:

```go
type ValidationResult struct {
    Passed     bool        `json:"passed"`
    Violations []Violation `json:"violations"`
    CheckedAt  time.Time   `json:"checked_at"`
}
```

## Domain Events

```go
const (
    EventRuleCreated   EventType = "rule.created"
    EventRuleUpdated   EventType = "rule.updated"
    EventRuleDeleted   EventType = "rule.deleted"
    EventRuleToggled   EventType = "rule.toggled"
)
```

## Reference

- `internal/domain/cqrs.go` — Full CQRS handler implementations
- `internal/mcp/handlers.go` — MCP tool handlers wired to CQRS
