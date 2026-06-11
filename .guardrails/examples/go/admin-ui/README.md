# Go Admin UI - Game Backend Interfaces

> Production-ready admin panels, metrics dashboards, and WebSocket event streaming for game backends.

**Last Updated:** 2026-03-14
**Go Version:** 1.22+
**Stack:** Go + Gorilla Mux + WebSocket + Grafana

---

## Purpose

This example demonstrates building production-ready game backend admin interfaces with:

- **Economy Monitoring** - Resource faucet/sink tracking, inflation detection
- **Metrics Dashboards** - Real-time Grafana integration, Prometheus metrics
- **WebSocket Event Streaming** - Game event broadcasting, player action tracking
- **Systemic Consistency** - Game state validation, economy balance rules
- **Accessibility Patterns** - ARIA labels, keyboard navigation, screen reader support

---

## 2026 Best Practices

### Economy Monitoring Patterns

| Pattern | Use Case | Implementation |
|---------|----------|----------------|
| **Faucet Tracking** | Resource generation | Track gold/xp sources, validate caps |
| **Sink Monitoring** | Resource consumption | Track spending, detect leaks |
| **Balance Detection** | Economy health | Ratio analysis, trend alerts |
| **Inflation Alerts** | Price drift | Moving average, threshold alerts |
| **Audit Logging** | Compliance | All transactions logged with sequence |

### Grafana Integration Patterns

| Pattern | Metric Type | Dashboard |
|---------|-------------|-----------|
| **Counter** | Total events | Event rate over time |
| **Gauge** | Current state | Player HP/MP, economy balance |
| **Histogram** | Distribution | Action completion times |
| **Summary** | Aggregates | Economy metrics by region |

### WebSocket Event Streaming

```go
// WebSocket event structure for game streaming
type GameEvent struct {
    Type      string      `json:"type"`      // "PLAYER_ACTION", "ECONOMY_UPDATE"
    Payload   interface{} `json:"payload"`   // Event data
    Sequence  uint64      `json:"sequence"`  // Eventual consistency
    Timestamp time.Time   `json:"timestamp"` // Server authority
    Source    string      `json:"source"`    // "server" for authority
}
```

### Accessibility (A11Y) Requirements

| Requirement | Pattern | WCAG Level |
|-------------|---------|------------|
| **ARIA Live Regions** | `aria-live="polite"` on updates | AA |
| **Keyboard Navigation** | Standard `<form>` + `<button>` | A |
| **Focus Management** | `tabindex="-1"` + HTMX focus | AA |
| **Screen Reader Labels** | `aria-label` on all interactive | A |

### Systemic Consistency Rulesets

```
ECONOMY CONSISTENCY RULES:

1. **FAUCET CAPS**
   - All resource generation has hard caps
   - Daily/weekly faucet limits enforced
   - Inflation detection triggers alerts

2. **Sink VALIDATION**
   - All consumption tracked with audit log
   - Sink rate matches faucet rate (balanced economy)
   - Negative sink rates rejected

3. **SERVER AUTHORITY**
   - Economy state computed server-side
   - Client displays only, never computes
   - All mutations validated against rules

4. **EVENTUAL CONSISTENCY**
   - Events streamed with sequence numbers
   - Conflicts resolved via server timestamp
   - WebSocket reconnects re-sync from sequence
```

---

## Running the Example

```bash
cd examples/go/admin-ui
go mod init admin-ui
go get github.com/gorilla/mux
go get github.com/gorilla/websocket
go get github.com/prometheus/client_golang/prometheus
go run economy-monitor.go metrics-dashboard.go websocket-events.go
```

Access the admin UI at `http://localhost:8080/admin`

---

## File Structure

| File | Purpose | Key Patterns |
|------|---------|--------------|
| `economy-monitor.go` | Faucet/sink monitoring, balance detection | Economy rules, audit logging |
| `metrics-dashboard.go` | Grafana/Prometheus integration | Counter, gauge, histogram |
| `websocket-events.go` | Real-time game event streaming | WebSocket, SSE, sequence |

---

## Architecture

```
+------------------+     +------------------+     +------------------+
|   Browser        |     |   Go Server      |     |   Grafana        |
|   (Admin UI)     |     |   (1.22+)        |     |   (Metrics)      |
+------------------+     +------------------+     +------------------+
        |                        |                        |
        | GET /admin            |                        |
        |-----------------------|                        |
        |                        |                        |
        | Economy data          | Query economy state    |
        |-----------------------|------------------------|
        |                        |                        |
        | WebSocket events      | Push to Grafana        |
        |-----------------------|------------------------|
        |                        |                        |
        | Prometheus scrape     | Expose /metrics        |
        |-----------------------|------------------------|
        |                        |                        |
        |                       | Prom GaugeVec          |
        |                       |------------------------|
        |                       |                        |
        |                       | Grafana Dashboard      |
        |                       |------------------------|
```

---

## Guardrails Compliance

| Rule | Implementation |
|------|----------------|
| **PRODUCTION FIRST** | Economy logic before UI |
| **SERVER AUTHORITY** | All economy state server-side |
| **ACCESSIBILITY** | ARIA labels, keyboard nav |
| **AUDIT LOGGING** | All economy transactions logged |
| **RATE LIMITING** | Per-session request limits |

---

## Related Documentation

- [AGENT_GUARDRAILS.md](../../docs/AGENT_GUARDRAILS.md) - Core safety protocols
- [TEST_PRODUCTION_SEPARATION.md](../../docs/standards/TEST_PRODUCTION_SEPARATION.md) - Separation standards
- [WEB_UI_IMPLEMENTATION.md](../../docs/sprints/SPRINT_002_WEB_UI_IMPLEMENTATION.md) - Web UI patterns
- [MCP_TOOLS_REFERENCE.md](../../docs/MCP_TOOLS_REFERENCE.md) - MCP integration

---

**Authored by:** Claude Code (Anthropic)
**Document Owner:** Project Maintainers
**Review Cycle:** Monthly