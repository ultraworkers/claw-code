# Laravel 11+ Livewire UI Examples

**Stack:** Laravel 11, Livewire 3.0, PHP 8.3+, Alpine.js

**Purpose:** Production-ready patterns for game analytics dashboards, economy monitoring, and ethical engagement systems.

---

## 2026 Best Practices

### Architecture Principles

| Principle | Implementation |
|-----------|---------------|
| **Component-Based** | Livewire components with reactive updates |
| **Type Safety** | PHP 8.3+ typed properties, readonly classes |
| **Real-time Updates** | Livewire polling, WebSocket integration |
| **Progressive Enhancement** | Alpine.js for interactivity, server-rendered HTML |
| **Accessibility** | ARIA labels, semantic HTML, keyboard navigation |

### Game Design Integration

- **Player analytics** - Retention, engagement, segmentation
- **Economy monitoring** - Faucet/sink balance, inflation tracking
- **Ethical engagement** - Transparent odds, spending limits
- **Rest-state mechanics** - Offline regeneration tracking

---

## Ethical Engagement Patterns

### Loot Table Transparency

```php
// Display drop rates, expected value, pity timers
// Comply with regional regulations (China, Belgium, EU)
// Audit trail for all loot box openings
```

### Spending Limits

```php
// Player-configured monthly spending caps
// Warning thresholds at 80% of limit
// Soft locks requiring confirmation
```

### Rest-State Mechanics

```php
// Offline resource regeneration display
// Respect player time and engagement boundaries
// 24-hour cap preventing excessive accumulation
```

---

## Economy Transparency

### Faucet/Sink Monitoring

| Metric | Description | Threshold |
|--------|-------------|-----------|
| **Faucet Rate** | Resources entering economy | Track daily |
| **Sink Rate** | Resources removed from economy | Track daily |
| **Balance Ratio** | Faucet/Sink ratio | Target: 1.0-1.2 |
| **Inflation Index** | Price increase over time | Alert if >5%/week |

---

## Accessibility Standards

| Category | Requirement |
|----------|-------------|
| **Semantic HTML** | Proper heading hierarchy, landmarks |
| **ARIA Labels** | All interactive elements labeled |
| **Keyboard Nav** | Tab navigation, focus management |
| **Color Contrast** | WCAG AA minimum (4.5:1) |
| **Screen Reader** | Live regions for dynamic updates |

---

## File Structure

```
examples/php/laravel-ui/
├── README.md              ← This document
├── dashboard.php          ← Laravel 11 Livewire dashboard
├── game-analytics.php     ← Player analytics visualization
├── economy-dashboard.php  ← Economy faucet/sink monitoring
```

---

## Related Examples

- `examples/ruby/rails-ui/` - Rails Hotwire dashboards
- `examples/swift/swiftui-game/` - SwiftUI 6 game UI
- `examples/typescript/ui-components/` - TypeScript UI patterns

---

**Authored by:** Project Sentinel
**Version:** 1.0
**Last Updated:** 2026-03-14
**Stack:** Laravel 11, Livewire 3.0, PHP 8.3