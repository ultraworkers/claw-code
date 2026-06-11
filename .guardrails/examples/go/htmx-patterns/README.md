# HTMX + Go 1.22+ Reactive Patterns

> Production-ready HTMX patterns for game backends without JavaScript frameworks.

**Last Updated:** 2026-03-14
**Go Version:** 1.22+
**HTMX Version:** 1.9.x

---

## Purpose

This example demonstrates building reactive game admin interfaces using **HTMX** with **Go 1.22+** without requiring JavaScript frameworks. Patterns include:

- Component composition with Go `template/block` syntax
- WebSocket-based real-time updates for game events
- Progressive enhancement for accessibility
- Systemic consistency for game state management
- Server-driven UI patterns for game admin panels

---

## 2026 Best Practices

### HTMX Architecture Patterns

| Pattern | Use Case | Implementation |
|---------|----------|----------------|
| **Boost Mode** | Full page transitions | `hx-boost="true"` on forms |
| **Push URL** | Browser history sync | `hx-push-url="true"` for navigation |
| **Polling** | Periodic state refresh | `hx-trigger="every 2s"` for metrics |
| **WebSocket** | Real-time game events | `hx-ext="ws"` for event streaming |
| **SSE** | One-way server updates | `hx-ext="sse"` for notifications |

### Go 1.22+ Template Features

```go
// Block syntax for component composition
{{block "player-card" .Player}}
  <div class="player-card" hx-get="/players/{{.Player.ID}}">
    {{.Player.Name}} - Level {{.Player.Level}}
  </div>
{{end}}
```

### Accessibility (A11Y) Requirements

| Requirement | HTMX Pattern | WCAG Level |
|-------------|--------------|------------|
| **Focus Management** | `hx-focus="#element"` on updates | AA |
| **ARIA Live Regions** | `aria-live="polite"` with `hx-trigger` | AA |
| **Keyboard Navigation** | Standard `<form>` + `<button>` elements | A |
| **Progressive Enhancement** | Core functionality without HTMX | A |

### Systemic Consistency Rulesets

```
GAME STATE CONSISTENCY RULES:

1. **SERVER AUTHORITY**
   - All game state originates from server
   - Client never computes game logic
   - HTMX requests always validate against server state

2. **OPTIMISTIC UI WITH ROLLBACK**
   - Display changes immediately with hx-confirm
   - Rollback on server rejection with hx-swap="none"
   - Show error states with aria-invalid

3. **EVENTUAL CONSistency**
   - Polling interval matches game tick rate
   - WebSocket updates use sequence numbers
   - Conflict resolution via server timestamp

4. **BOUNDARY VALIDATION**
   - All hx-get/hx-post paths validate authz
   - Rate limiting per player session
   - Audit logging on state mutations
```

---

## Running the Example

```bash
cd examples/go/htmx-patterns
go mod init htmx-patterns
go get github.com/gorilla/mux
go get github.com/yohox/go-htmx
go run main.go
```

Access the admin panel at `http://localhost:8080/admin`

---

## File Structure

| File | Purpose | Key Patterns |
|------|---------|--------------|
| `main.go` | HTTP server, HTMX handlers | Boost, polling, WebSocket |
| `templates.go` | Template blocks, component composition | `block` syntax, embedded templates |
| `game-admin-panel.go` | Admin panel handlers | Game state management, authz |

---

## Architecture

```
+------------------+     +------------------+     +------------------+
|   Browser        |     |   Go Server      |     |   Game Backend   |
|   (HTMX client)  |     |   (1.22+)        |     |   (State)        |
+------------------+     +------------------+     +------------------+
        |                        |                        |
        | GET /admin            |                        |
        |-----------------------|                        |
        |                        |                        |
        | Template render        | Query game state       |
        |-----------------------|------------------------|
        |                        |                        |
        | WebSocket connect     | Subscribe to events    |
        |-----------------------|------------------------|
        |                        |                        |
        | hx-trigger="every 2s"|                        |
        |-----------------------|                        |
        |                        |                        |
        | SSE for notifications|                        |
        |-----------------------|------------------------|
```

---

## Component Composition

### Template Block Pattern

```go
// templates.go
var PlayerCardTemplate = template.MustParse(`
{{block "player-card" .}}
<div class="card" hx-get="/api/player/{{.ID}}" hx-swap="innerHTML">
  <span class="name">{{.Name}}</span>
  <span class="level">Level {{.Level}}</span>
  <button hx-post="/api/player/{{.ID}/action" hx-vals="js:{action: 'attack'}">
    Attack
  </button>
</div>
{{end}}
`)
```

### Server-Driven Events

```go
// game-admin-panel.go
func (g *GameAdmin) broadcastEvent(w http.ResponseWriter, r *http.Request) {
    // Game event originates from server
    event := GameEvent{Type: "PLAYER_ACTION", Payload: action}
    g.mu.Lock()
    g.clients[wsID] <- event
    g.mu.Unlock()
}
```

---

## Guardrails Compliance

| Rule | Implementation |
|------|----------------|
| **PRODUCTION FIRST** | Server code before template tests |
| **SERVER AUTHORITY** | No client-side game logic |
| **ACCESSIBILITY** | ARIA labels, keyboard nav, focus management |
| **RATE LIMITING** | Per-session request limits |
| **AUDIT LOGGING** | All state mutations logged |

---

## Related Documentation

- [AGENT_GUARDRAILS.md](../../docs/AGENT_GUARDRAILS.md) - Core safety protocols
- [TEST_PRODUCTION_SEPARATION.md](../../docs/standards/TEST_PRODUCTION_SEPARATION.md) - Separation standards
- [WEB_UI_IMPLEMENTATION.md](../../docs/sprints/SPRINT_002_WEB_UI_IMPLEMENTATION.md) - Web UI patterns

---

**Authored by:** Claude Code (Anthropic)
**Document Owner:** Project Maintainers
**Review Cycle:** Monthly