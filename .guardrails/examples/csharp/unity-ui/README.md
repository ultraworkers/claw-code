# Unity UI Toolkit - 2026 Best Practices

## Overview

This module demonstrates production-ready Unity UI Toolkit 2.0+ implementations with ECS UI patterns using DOTS (Data-Oriented Technology Stack), focusing on game UI patterns, accessibility, and spatial computing.

## Architecture

### Stack
- **Unity UI Toolkit 2.0+** - Declarative UI system
- **ECS (Entity Component System)** - Data-oriented UI architecture
- **DOTS (Data-Oriented Technology Stack)** - High-performance UI rendering
- **DDA 3.0** - Dynamic Difficulty Adjustment for enemy AI

## 2026 Best Practices

### Core Design Patterns

#### ECS UI Pattern
```csharp
// UI entities managed through ECS
var uiEntity = EntityManager.CreateEntity();
uiEntity.AddComponent<UIElementComponent>();
uiEntity.AddComponent<HapticFeedbackComponent>();
```

#### DOTS UI Rendering
- UI elements batched through GPU instancing
- Job-system UI layout calculations
- Burst-compiled UI updates

#### Game Core Loop Integration
- **Action Phase**: Combat HUD with ECS-based reactive updates
- **Reward Phase**: Loot system with transparent RNG display
- **Upgrade Phase**: Skill tree with DOTS pathfinding

### DDA 3.0 (Dynamic Difficulty Adjustment)

#### Enemy AI Heuristic Adjustment
```csharp
// DDA 3.0 adjusts enemy AI based on player performance
ddaManager.AdjustEnemyHeuristics(playerKDR, sessionDuration);
```

#### UI Adaptation
- Enemy difficulty tier influences UI opacity
- Stress detection reduces visual complexity
- Performance metrics drive aesthetic adjustments

### Accessibility (WCAG 3.0+)

#### Colorblindness Independence
```csharp
// Shape + icon redundancy for color states
healthState.icon = healthState.shape; // Not just color
```

#### Eye-Tracking Support
- Dwell-based selection (150ms threshold)
- Pupil dilation stress detection
- Blink confirmation pattern

### Spatial Computing (XR)

#### Volumetric UI
- Holographic inventory previews with Z-depth
- Parallax scrolling for UI layers
- Eye-tracking focus indicators

#### GPU Instancing
- Batched UI element rendering
- Z-depth parallax for XR environments

### Ethical Engagement

#### Rest-State Mechanics
```csharp
// Mandatory calm state after 45-minute sessions
if (sessionDuration > 45) EnterRestState();
```

#### Transparent Loot Tables
- Drop rates displayed inline
- Pity timer visualization
- No obfuscated RNG indicators

## File Structure

```
unity-ui/
  README.md           - This documentation
  GameUI.cs           - Main UI Toolkit entry point
  dots-ui-patterns.cs - ECS UI patterns with DOTS
  dda-system.cs       - DDA 3.0 enemy AI adjustment
  object-pooling.cs   - UI element pooling
```

## Usage

```csharp
// Initialize UI Toolkit with ECS
var uiSystem = new UISystem();
uiSystem.Initialize(EntityManager);

// Apply DDA 3.0 adaptation
ddaManager.AdjustForTier(DifficultyTier.Normal);

// Enable object pooling for UI elements
uiPoolPrefetcher.EnablePrefetch();
```

## Testing

Run Unity UI tests:
```bash
unity-test-toolkit --test examples/csharp/unity-ui
```

## References

- [Unity UI Toolkit Documentation](https://docs.unity3d.com/Packages/com.unity.ui@2.0/)
- [DOTS Architecture](https://unity.com/dots)
- [WCAG 3.0 Accessibility Guidelines](https://www.w3.org/TR/wcag-3.0/)