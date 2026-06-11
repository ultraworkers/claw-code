# Rails 8.0+ Hotwire UI Examples

**Stack:** Rails 8.0, Hotwire (Turbo + Stimulus), ViewComponent, Ruby 3.3+

**Purpose:** Production-ready patterns for game admin dashboards, world-building tools, and ethical engagement systems.

---

## 2026 Best Practices

### Architecture Principles

| Principle | Implementation |
|-----------|---------------|
| **Component-Based** | ViewComponent for reusable UI components |
| **Progressive Enhancement** | Turbo for navigation, Stimulus for interactivity |
| **No JavaScript Framework** | Server-rendered HTML with minimal JS |
| **Type Safety** | RBS (Ruby Signatures) for method contracts |
| **Accessibility** | ARIA labels, semantic HTML, keyboard navigation |

### Game Design Integration

- **Real-time dashboards** using Turbo Streams
- **Generative world builders** with Stimulus controllers
- **Analytics visualization** with server-side rendering
- **Ethical engagement** patterns (transparent loot tables, Rest-State Mechanics)

---

## Ethical Engagement Patterns

### Loot Table Transparency

```ruby
# Display drop rates, expected value, and pity timers
# Never hide odds from players
# Comply with regional regulations (China, Belgium, etc.)
```

### Rest-State Mechanics

```ruby
# Design offline regeneration systems
# Respect player time and engagement boundaries
```

---

## Accessibility Standards

| Category | Requirement |
|----------|-------------|
| **Semantic HTML** | Proper heading hierarchy, landmarks |
| **ARIA Labels** | All interactive elements labeled |
| **Keyboard Nav** | Tab navigation, focus management |
| **Color Contrast** | WCAG AA minimum (4.5:1) |
| **Motion** | Reduced motion support |

---

## File Structure

```
examples/ruby/rails-ui/
├── README.md              ← This document
├── app.rb                 ← Rails 8 Hotwire architecture
├── game-admin-dashboard.rb ← Admin dashboard patterns
├── world-builder.rb       ← Generative world builder
```

---

## Related Examples

- `examples/swift/swiftui-game/` - SwiftUI 6 game UI
- `examples/php/laravel-ui/` - Laravel 11 Livewire dashboards
- `examples/typescript/ui-components/` - TypeScript UI patterns

---

**Authored by:** Project Sentinel
**Version:** 1.0
**Last Updated:** 2026-03-14
**Stack:** Rails 8.0, Hotwire, ViewComponent, Ruby 3.3