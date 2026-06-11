# TypeScript UI Components - 2026 Best Practices

**Stack:** React 19, TypeScript 5.x, Server Components, Signals
**Target:** Game UI, Dashboard Systems, Real-time Interfaces
**Last Updated:** 2026-03-14

---

## 2026 Best Practices

### React 19 Server Components

| Pattern | Description | Use Case |
|---------|-------------|----------|
| **RSC First** | Server Components by default, Client only when interactive | Dashboard, static game UI |
| **Streaming SSR** | Progressive hydration with Suspense boundaries | Real-time game state |
| **Actions** | Server Actions for mutations, no API routes | Form submissions, game actions |
| **Composition** | Component composition over state | Complex UI trees |

### Signals Integration (2026)

```
Signal Pattern → useSignal() for reactive state without re-renders
Fine-grained updates → Only affected nodes re-render
Game loop integration → Sync with requestAnimationFrame
```

### Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| **TTI** | < 1.8s | Lighthouse |
| **FCP** | < 1.0s | Web Vitals |
| **FPS** | 60+ (UI) | frame() API |
| **Memory** | < 50MB idle | Chrome DevTools |

---

## Game Design Integration Patterns

### ECS-Based State Management

```
Entity-Component-System → Game state as immutable entities
Signals → React to component changes without polling
Server Components → Initial state hydration from game server
```

### Object Pooling for UI Elements

| Pattern | Description | Benefit |
|---------|-------------|---------|
| **Pool Manager** | Pre-allocate UI element instances | No GC pressure |
| **Lazy Activation** | Activate only when visible | Memory efficiency |
| **Signal-based Recycling** | Auto-return to pool on unmount | No leaks |

### LLM-NPC Dialogue with Lore Guardrails

```
Guardrail Layer → Validate dialogue against lore constraints
Cache Strategy → Memoize approved dialogue trees
Ethical Engagement → No dark patterns, transparent choices
```

---

## Accessibility Compliance (WCAG 2.2 Level AA)

| Requirement | Implementation | Verification |
|-------------|----------------|--------------|
| **Keyboard Navigation** | Tab index, arrow key navigation | axe-core |
| **Screen Reader Support** | aria-live, role mappings | Screen reader testing |
| **Color Contrast** | 4.5:1 minimum, dynamic themes | Contrast checker |
| **Focus Management** | Focus traps, restore on close | Manual audit |
| **Reduced Motion** | prefers-reduced-motion media query | Device testing |

---

## Ethical Engagement Standards

### No Dark Patterns

| Forbidden Pattern | Replacement |
|------------------|-------------|
| **Confirmshaming** | Neutral cancel buttons |
| **Forced Continuity** | Clear cancellation flow |
| **Misleading Labels** | Explicit action text |
| **Hidden Costs** | Transparent pricing display |
| **Roach Motel** | Easy exit, clear return path |

### Transparency Requirements

```
- Display data collection purposes explicitly
- Offer opt-out without penalty
- No hidden tracking in UI components
- Clear state indicators for async operations
- Honest error messages (no blame shifting)
```

---

## Code Examples

### ecs-ui.tsx
ECS-based game UI state management with Signals
→ See: [`ecs-ui.tsx`](./ecs-ui.tsx)

### object-pooling.tsx
Object pooling for high-frequency UI updates
→ See: [`object-pooling.tsx`](./object-pooling.tsx)

### llm-npc-dialogue.tsx
LLM-driven NPC dialogue with Lore Guardrails
→ See: [`llm-npc-dialogue.tsx`](./llm-npc-dialogue.tsx)

---

## Testing Guidelines

| Test Type | Tool | Frequency |
|-----------|------|-----------|
| **Unit** | Vitest | Per-commit |
| **A11y** | axe-core | Per-component |
| **Performance** | Lighthouse CI | Per-PR |
| **Integration** | Playwright | Per-sprint |

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