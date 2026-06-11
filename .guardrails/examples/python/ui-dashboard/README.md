# Python UI Dashboard - 2026 Best Practices

**Stack:** FastUI 0.6, Reflex 0.6.x, Streamlit 1.40, FastAPI, WebSockets
**Target:** Real-time dashboards, data visualization, multiplayer monitoring
**Last Updated:** 2026-03-14

---

## 2026 Best Practices

### FastUI (FastAPI-based)

| Pattern | Description | Use Case |
|---------|-------------|----------|
| **Type-driven** | Pydantic v2 for schema validation | Form validation |
| **Component-based** | Reusable UI components | Dashboard widgets |
| **Async-first** | `async def` for all endpoints | WebSocket streaming |
| **TypeScript sync** | Shared types via API spec | Client consistency |

### Reflex (React-based Python)

```
Component System → React-like declarative syntax
State Management → rx.State for reactive updates
Full-stack → Backend + frontend in single codebase
```

### Streamlit 1.40 (2026)

| Feature | Description | Benefit |
|---------|-------------|---------|
| **Fragment Caching** | Memoize components | Performance |
| **Async Support** | Native async/await | WebSocket integration |
| **Custom Components** | Web component wrapping | Extensibility |
| **Multi-page** | Native page routing | App architecture |

### Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| **TTI** | <2.0s | Lighthouse |
| **WebSocket Latency** | <100ms | ws benchmark |
| **Memory** | <200MB | `memory_profiler` |
| **Throughput** | 100+ concurrent | load test |

---

## Game Design Integration Patterns

### Real-time Dashboard

```
WebSocket Streaming → Live game state updates
Delta Compression → Send only changes
Client Prediction → Optimistic UI updates
```

### Multiplayer Monitoring

| Pattern | Description | Implementation |
|---------|-------------|----------------|
| **Player Heatmap** | Density visualization | Canvas rendering |
| **Match Timeline** | Event sequence display | Gantt-style chart |
| **Lag Distribution** | Latency histogram | Percentile display |
| **Sync Status** | Server/client reconciliation | Color indicators |

---

## Accessibility Compliance (WCAG 2.2 Level AA)

| Requirement | Implementation | Verification |
|-------------|----------------|--------------|
| **Keyboard Navigation** | Tab index, focus management | Manual audit |
| **Screen Reader Support** | aria-label, role attributes | axe-core |
| **Color Contrast** | 4.5:1 minimum | Contrast checker |
| **Focus Indicators** | Visible focus rings | Keyboard testing |
| **Reduced Motion** | CSS media query | Device testing |

---

## Ethical Engagement Standards

### No Dark Patterns

| Forbidden Pattern | Replacement |
|------------------|-------------|
| **Data Hoarding** | Minimal data collection |
| **Hidden Tracking** | Transparent analytics disclosure |
| **Forced Continuity** | Easy session termination |
| **Misleading Metrics** | Honest performance indicators |
| **Fake Urgency** | No artificial timers |

### Transparency Requirements

```
- Display data retention policies
- Offer opt-out without penalty
- Clear WebSocket connection status
- Honest sync/error indicators
- No hidden telemetry
```

---

## Code Examples

### dashboard.py
FastUI dashboard with Pydantic v2 validation
→ See: [`dashboard.py`](./dashboard.py)

### websocket-streaming.py
Real-time WebSocket streaming patterns
→ See: [`websocket-streaming.py`](./websocket-streaming.py)

---

## Testing Guidelines

| Test Type | Tool | Target |
|-----------|------|--------|
| **Unit** | pytest | Per-commit |
| **Integration** | pytest-asyncio | Per-PR |
| **A11y** | axe-core (web) | Per-component |
| **Load** | `pytest-benchmark` | Per-sprint |
| **Security** | OWASP scan | Per-release |

---

## Related Documents

- [AGENT_GUARDRAILS.md](../../docs/AGENT_GUARDRAILS.md) - Mandatory safety protocols
- [TEST_PRODUCTION_SEPARATION.md](../../docs/standards/TEST_PRODUCTION_SEPARATION.md) - Test/prod isolation
- [OPERATIONAL_PATTERNS.md](../../docs/standards/OPERATIONAL_PATTERNS.md) - Health checks, circuit breakers

---

**Authored by:** Claude Code (Anthropic)
**Document Owner:** Project Maintainers
**Review Cycle:** Per-sprint updates
**Last Review:** 2026-03-14