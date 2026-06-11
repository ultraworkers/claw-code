# SwiftUI 6.0+ Game UI Examples

**Stack:** SwiftUI 6.0, VisionOS spatial UI, Swift 5.10+

**Purpose:** Production-ready patterns for game interfaces, spatial computing, haptic synchronization, and diegetic UI systems.

---

## 2026 Best Practices

### Architecture Principles

| Principle | Implementation |
|-----------|---------------|
| **Data-Driven** | ObservableObject + @Published for game state |
| **Component-Based** | Reusable View components with protocols |
| **Spatial Awareness** | VisionOS Z-depth, volumetric rendering |
| **Multi-Sensory** | Audio-visual-haptic synchronization |
| **Accessibility** | VoiceOver, reduce motion, accessibility identifiers |

### Game Design Integration

- **Diegetic UI** - World-space displays (holographic, projected)
- **Spatial Computing** - VisionOS volumetric interfaces with depth
- **Haptic Feedback** - synchronized with audio/visual events
- **Rest-State Mechanics** - Offline progression indicators
- **Ethical Engagement** - Transparent odds, spending limits

---

## Spatial Computing Patterns

### VisionOS Volumetric UI

```swift
// Z-depth layering for immersive interfaces
// ImmersionLevel: .mixed, .full, .minimized
// Hand tracking for gesture-based interaction
```

### Diegetic UI Components

```swift
// World-space holographic displays
// Environmental context (lighting, occlusion)
// Player perspective alignment
```

---

## Haptic synchronization

### Audio-Visual-Haptic Sync

| Event Type | Haptic Pattern | Audio | Visual |
|------------|---------------|-------|--------|
| Combat hit | `.impact(fortitude: .hard)` | Hit sound | Flash |
| Resource gain | `.notification(.success)` | Pickup sound | Glow |
| Menu select | `.selection` | Click | Highlight |
| Achievement | `.notification(.success)` | Victory sound | Burst |

---

## Ethical Engagement Patterns

### Loot Table Transparency

```swift
// Display odds, expected value, pity timers
// Regional compliance (China, Belgium, EU)
// Player spending analytics
```

### Rest-State Mechanics

```swift
// Offline resource regeneration display
// Respect player time and engagement boundaries
```

---

## Accessibility Standards

| Category | Requirement |
|----------|-------------|
| **VoiceOver** | All elements labeled with accessibilityIdentifier |
| **Reduce Motion** | Detect and disable parallax/animation |
| **Color Contrast** | WCAG AA minimum (4.5:1) |
| **Text Scaling** | Dynamic type support |
| **Reduced Transparency** | Solid backgrounds when enabled |

---

## VisionOS Integration

### Spatial Window Management

- **Volume Geometry** - 3D space allocation
- **Immersion Levels** - Mixed/full/minimized reality
- **Hand Tracking** - Gesture-based navigation
- **Eye Tracking** - Focus-based selection

---

## File Structure

```
examples/swift/swiftui-game/
├── README.md              ← This document
├── GameView.swift         ← SwiftUI 6 game UI
├── visionos-spatial.swift ← VisionOS volumetric UI
├── haptic-sync.swift      ← Audio-visual-haptic sync
├── diegetic-ui.swift      ← World-space displays
```

---

## Related Examples

- `examples/ruby/rails-ui/` - Rails Hotwire dashboards
- `examples/php/laravel-ui/` - Laravel Livewire dashboards
- `examples/typescript/ui-components/` - TypeScript UI patterns

---

**Authored by:** Project Sentinel
**Version:** 1.0
**Last Updated:** 2026-03-14
**Stack:** SwiftUI 6.0, VisionOS, Swift 5.10