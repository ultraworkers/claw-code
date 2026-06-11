# TypeScript Game UI - 2026 Best Practices

**Stack:** TypeScript 5.x, React 19, Web Animations API, Canvas API
**Target:** 60+ FPS, <16ms frame time, deterministic rendering
**Last Updated:** 2026-03-14

---

## 2026 Best Practices

### Frame-Independent Animation

| Pattern | Description | Use Case |
|---------|-------------|----------|
| **Delta Time** | `requestAnimationFrame` with delta calculation | All animations |
| **Fixed Step** | Fixed timestep for physics, variable for rendering | Game loops |
| **Interpolation** | Smooth interpolation between physics steps | Character movement |
| **Predictive Render** | Render ahead based on input prediction | Fast-paced action |

### State Machine Patterns

```
Finite State Machine → Deterministic UI flow
Transition Guards → Prevent invalid state changes
Rollback Support → Undo to previous state on error
```

### Lag Compensation (Multiplayer)

| Technique | Description | Benefit |
|-----------|-------------|---------|
| **Input Buffering** | Queue inputs, execute on confirmation | Consistency |
| **Optimistic Render** | Show predicted result, correct on server | Perceived speed |
| **Delta Compression** | Send only changes, not full state | Bandwidth reduction |
| **Client Prediction** | Predict server state locally | Reduced latency feel |

### Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Frame Time** | <16ms (60 FPS) | `performance.now()` |
| **Input Latency** | <50ms | Input event to render |
| **Memory** | <100MB | Chrome DevTools |
| **GC Pause** | <5ms | Profiler |

---

## Game Design Integration Patterns

### State Machine for UI Flow

```
States: Loading → Active → Paused → Results → Exit
Transitions: Guarded, logged, reversible
Rollback: Each state stores previous for undo
```

### Lag Compensation for Multiplayer UI

| Pattern | Description | Implementation |
|---------|-------------|----------------|
| **Shadow State** | Local copy of server state | Immutable records |
| **Reconciler** | Merge server/client deltas | Functional diff |
| **Rollback Queue** | Store last N states | Array-based log |
| **Visual Feedback** | Show sync status | Color indicators |

### Frame-Independent Animation

```
requestAnimationFrame → Delta time from last frame
Scale animation → value * (delta / 16.67)
Fallback → Fixed step if delta exceeds threshold
```

---

## Accessibility Compliance (WCAG 2.2 Level AA)

| Requirement | Implementation | Verification |
|-------------|----------------|--------------|
| **Motion Sensitivity** | `prefers-reduced-motion` media query | Device testing |
| **Focus Indicators** | Visible focus rings, 3px minimum | axe-core |
| **Color Independence** | Non-color status indicators | Grayscale test |
| **Timing Control** | Pause/extend animated content | Manual audit |
| **Keyboard Timing** | No time-based key requirements | Keyboard testing |

---

## Ethical Engagement Standards

### No Dark Patterns

| Forbidden Pattern | Replacement |
|------------------|-------------|
| **Time Pressure** | No artificial timers for decisions |
| **Energy Drain** | Transparent resource costs |
| **Paywall Deception** | Clear purchase consequences |
| **Progress Manipation** | Honest difficulty indicators |
| **Loss Fear** | No false scarcity claims |

### Transparency Requirements

```
- Display server sync status clearly
- Show lag compensation effects transparently
- Honest difficulty/rarity indicators
- No fake urgency in timers
- Clear resource cost disclosure
```

---

## Code Examples

### game-state-machine.ts
Deterministic state machine for UI flow
→ See: [`game-state-machine.ts`](./game-state-machine.ts)

### lag-compensation.ts
Multiplayer lag compensation patterns
→ See: [`lag-compensation.ts`](./lag-compensation.ts)

### frame-independent-animation.ts
`requestAnimationFrame` with delta time
→ See: [`frame-independent-animation.ts`](./frame-independent-animation.ts)

---

## Testing Guidelines

| Test Type | Tool | Target |
|-----------|------|--------|
| **Frame Analysis** | Chrome Profiler | 60+ FPS |
| **Input Latency** | Custom benchmark | <50ms |
| **State Transition** | Unit tests | 100% coverage |
| **A11y** | axe-core | Level AA |
| **Integration** | Playwright | Per-sprint |

---

## Related Documents

- [AGENT_GUARDRAILS.md](../../docs/AGENT_GUARDRAILS.md) - Mandatory safety protocols
- [OPERATIONAL_PATTERNS.md](../../docs/standards/OPERATIONAL_PATTERNS.md) - Health checks, circuit breakers
- [TEST_PRODUCTION_SEPARATION.md](../../docs/standards/TEST_PRODUCTION_SEPARATION.md) - Test/prod isolation

---

**Authored by:** Claude Code (Anthropic)
**Document Owner:** Project Maintainers
**Review Cycle:** Per-sprint updates
**Last Review:** 2026-03-14