# Jetpack Compose UI - 2026 Best Practices

## Overview

This module demonstrates production-ready Jetpack Compose 2.0+ implementations with Material 3 design tokens, focusing on game UI patterns, accessibility, and spatial computing.

## Architecture

### Stack
- **Jetpack Compose 2.0+** - Declarative UI framework
- **Material 3** - Dynamic color extraction from game themes
- **Compose Animation** - Physics-based transitions
- **Haptic Feedback** - Tactile interaction patterns

## 2026 Best Practices

### Core Design Patterns

#### Material 3 Dynamic Tokens
```kotlin
// Dynamic color extraction from game state
val gameThemeColor = MaterialColorUtilities.extractFromPalette(gameState.primaryColor)
val colorScheme = gameThemeColor.toColorScheme()
```

#### Game Core Loop Integration
- **Action Phase**: Combat HUD with reactive health bars
- **Reward Phase**: Loot reveal withMaterial 3 elevation tokens
- **Upgrade Phase**: Skill tree with Compose Canvas rendering

### DDA 3.0 (Dynamic Difficulty Adjustment)
- Enemy AI heuristic adjustment based on player performance
- UI opacity/contrast adapts to player stress indicators
- Haptic intensity scales with difficulty tier

### Accessibility (WCAG 3.0+)

#### Colorblindness Independence
```kotlin
// Shape + icon redundancy for color states
val healthState = if (hp < 30) HealthState.Critical else HealthState.Normal
// Always paired with shape indicator, not just color
```

#### Eye-Tracking Support
- Dwell-based selection (150ms threshold)
- Pupil dilation stress detection
- Blink confirmation pattern

### Spatial Computing (XR)

#### Z-Depth Parallax
```kotlin
// Parallax scrolling for 3D UI layers
Modifier.parallax(depthFactor = 0.3f)
```

#### Volumetric Rendering
- Holographic inventory previews
- Z-axis layering for ability cards

### Ethical Engagement

#### Rest-State Mechanics
```kotlin
// Mandatory calm state after intense sessions
val restStateTimer = remember { mutableStateOf(0) }
if (sessionDuration > 45) enterRestState()
```

#### Transparent Loot Tables
- Drop rates displayed inline
- Pity timer visualization
- No obfuscated RNG indicators

## File Structure

```
compose-ui/
  README.md           - This documentation
  MainActivity.kt     - Main Compose entry point
  libgdx-scene2d-ui.kt - LibGDX scene2d.ui integration
  haptic-profiles.kt  - Tactile feedback profiles
```

## Usage

```kotlin
// Initialize Material 3 theme
val theme = GameTheme(
    primary = gameState.palette.primary,
    difficultyTier = DDA.currentTier,
    accessibilityMode = AccessibilityPreferences.highContrast
)

// Apply haptic profile
HapticFeedback.applyProfile(HapticProfile.COMBAT_SUCCESS)
```

## Testing

Run Compose UI tests:
```bash
./gradlew :examples:java:compose-ui:test
```

## References

- [Jetpack Compose 2.0 Documentation](https://developer.android.com/compose)
- [Material 3 Design Tokens](https://m3.material.io/)
- [WCAG 3.0 Accessibility Guidelines](https://www.w3.org/TR/wcag-3.0/)