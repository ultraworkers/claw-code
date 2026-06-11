# Spatial Computing UI Patterns (XR/VR/AR)

**Version:** 2.0.0
**Last Updated:** 2026-03-14
**Applies To:** ALL extended reality experiences - VR, AR, MR, XR

---

## Purpose

This document defines UI patterns for spatial computing interfaces. These patterns let AI agents generate spatial UIs rapidly — comfort zones, performance budgets, and safety thresholds are pre-defined so agents can scaffold XR interfaces without iterating on human factors research.

Spatial computing encompasses:

1. **Virtual Reality (VR)** - Fully immersive digital environments
2. **Augmented Reality (AR)** - Digital overlay on physical world
3. **Mixed Reality (MR)** - Bidirectional physical-digital interaction
3. **Extended Reality (XR)** - Unified spectrum of reality technologies

---

## Agent-GDUI-2026 Spatial Capabilities

**Agent-GDUI-2026** provides specialized spatial computing capabilities:

| Capability | Scope | Constraint |
|------------|-------|------------|
| **Viewport Management** | XR field of view | Comfort zone enforcement |
| **Depth Layering** | Z-axis positioning | Max 3 depth planes |
| **Motion Coherence** | Visual-vestibular sync | Latency < 20ms |
| **Audio Spatialization** | 3D sound positioning | HRTF calibration |
| **Hand Tracking** | Gesture recognition | Comfort zone bounds |
| **Gaze Detection** | Eye tracking, focus | Dwell time thresholds |
| **Haptic Mapping** | Touch feedback | Platform API sync |

### AI-Driven Spatial Development

Agent-GDUI-2026 uses these constraints as generation templates:
- **Comfort zones are pre-calculated** — spatial placement is safe by default
- **Performance budgets are strict** — agents generate within frame-time limits automatically
- **Motion safety is built-in** — no need to test for motion sickness triggers manually

---

## SPATIAL SAFETY PROTOCOLS

### Comfort Zones

| Zone | Boundaries | Usage |
|------|------------|-------|
| **Primary Comfort** | 30° horizontal, 20° vertical | Core UI elements |
| **Secondary Comfort** | 45° horizontal, 30° vertical | Contextual elements |
| **Peripheral** | 60° horizontal, 40° vertical | Ambient/background |
| **Danger Zone** | > 60° horizontal | NEVER place UI |

### Latency Requirements

| Metric | Threshold | Consequence |
|--------|-----------|-------------|
| **Motion-to-Photon** | < 20ms | Motion sickness prevention |
| **Audio Latency** | < 15ms | Spatial audio coherence |
| **Hand Tracking** | < 30ms | Gesture responsiveness |
| **Gaze Detection** | < 50ms | Focus accuracy |
| **Haptic Feedback** | < 10ms | Touch coherence |

### Vestibular Safety

| Rule | Constraint | Purpose |
|------|------------|---------|
| **NO ACCEL > 1.5g** | Virtual acceleration bounded | Nausea prevention |
| **FIXED HORIZON** | Reference line always visible | Orientation maintenance |
| **NO VERTICAL SCROLL** | Avoid vertical motion in VR | Vestibular conflict |
| **MATCH PHYSICS** | Virtual/physical gravity sync | Disorientation prevention |
| **SESSION LIMITS** | 60min max continuous | Fatigue prevention |

---

## UI LAYOUT PATTERNS

### VR Layout (Full Immersion)

```
┌─────────────────────────────────────────────────────────┐
│                    COMFORT CONE (30°)                   │
│                                                         │
│         [Notification]        [Status Indicator]        │
│                                                         │
│              ┌─────────────────────────────┐            │
│              │         MAIN CONTENT         │            │
│              │         (Center Focus)       │            │
│              └─────────────────────────────┘            │
│                                                         │
│    [Navigation]                      [Action Panel]     │
│                                                         │
│                    FIXED HORIZON LINE                  │
└─────────────────────────────────────────────────────────┘
```

### AR Layout (Physical Overlay)

```
┌─────────────────────────────────────────────────────────┐
│  PHYSICAL WORLD VIEW                                    │
│                                                         │
│  [Label] ← Object                                       │
│                                                         │
│              [Central Reticle]                          │
│                                                         │
│  Object ← [Label]                                       │
│                                                         │
│         [Control Bar - Bottom Comfort Zone]             │
└─────────────────────────────────────────────────────────┘
```

### MR Layout (Bidirectional)

```
┌─────────────────────────────────────────────────────────┐
│  PHYSICAL + DIGITAL FUSION                              │
│                                                         │
│  [Physical Object] ↔ [Digital Annotation]               │
│                                                         │
│              [Interaction Zone]                         │
│              (Gesture Recognition)                      │
│                                                         │
│    [Physical Controls]      [Digital Controls]          │
│                                                         │
│         [Mode Switch - Physical/Digital]                │
└─────────────────────────────────────────────────────────┘
```

---

## DEPTH LAYERING

### Three-Layer Standard

| Layer | Distance | Usage | Parallax |
|-------|----------|-------|----------|
| **Foreground** | 0.5-2m | Interactive UI | Minimal |
| **Midground** | 2-10m | Content, context | Moderate |
| **Background** | > 10m | Environment, sky | Fixed |

### Depth Implementation

```typescript
// Depth layer component
interface DepthLayerProps {
  layer: 'foreground' | 'midground' | 'background';
  distance: number; // meters
  parallaxFactor: number; // 0.0-1.0
  stereoSeparation: number; // < 6.5cm
}

// Validation
const MAX_DEPTH_PLANES = 3;
const MAX_STEREO_SEPARATION_CM = 6.5;
const MIN_DEPTH_M = 0.5;
const MAX_DEPTH_M = 100;
```

---

## INTERACTION PATTERNS

### Hand Gesture Vocabulary

| Gesture | Meaning | Comfort Zone |
|---------|---------|--------------|
| **Point** | Selection | Primary (30°) |
| **Pinch** | Activation | Primary (30°) |
| **Grab** | Object manipulation | Secondary (45°) |
| **Swipe** | Navigation | Secondary (45°) |
| **Thumbs Up** | Confirmation | Secondary (45°) |
| **OK Sign** | Menu access | Secondary (45°) |

### Gaze Interaction

| Pattern | Dwell Time | Purpose |
|---------|------------|---------|
| **Look** | 0ms | Passive attention |
| **Focus** | 500ms | Selection preparation |
| **Activate** | 1000ms | Confirmation (no click) |
| **Dismiss** | Look away | Cancel action |

### Haptic Feedback Patterns

| Event | Pattern | Intensity | Duration |
|-------|---------|-----------|----------|
| **Selection** | Single tap | Light | 50ms |
| **Activation** | Double tap | Medium | 100ms |
| **Error** | Strong pulse | High | 200ms |
| **Success** | Gentle wave | Light | 150ms |
| **Loading** | Rhythmic | Light | Continuous |

---

## ACCESSIBILITY IN XR

### Spatial Accessibility

| Requirement | Implementation |
|-------------|---------------|
| **High Contrast** | 7:1 ratio in 3D space |
| **Audio Labels** | All visual elements have audio description |
| **Alternative Input** | Gesture → gaze → voice fallback |
| **Motion Adaptation** | Reduced motion mode available |
| **Seated Mode** | Experience accessible from seated position |
| **One-Hand Mode** | Single hand operation supported |
| **Voice Control** | All functions available via voice |

### Cognitive Accessibility in XR

| Support | Implementation |
|---------|---------------|
| **Orientation** | Compass, landmarks always visible |
| **Pacing** | User-controlled progression |
| **Instructions** | Multi-modal (visual + audio + text) |
| **Breaks** | Rest points built into experience |
| **Intensity** | Adjustable sensory load |

---

## PERFORMANCE BUDGETS

### Frame Rate Requirements

| Platform | Minimum | Target | Budget |
|----------|---------|--------|--------|
| **Mobile VR** | 60fps | 90fps | 11.1ms/frame |
| **PC VR** | 90fps | 120fps | 8.3ms/frame |
| **Console VR** | 60fps | 120fps | 8.3ms/frame |
| **AR Headset** | 60fps | 90fps | 11.1ms/frame |
| **Professional XR** | 90fps | 144fps | 6.9ms/frame |

### Async Reprojection

```typescript
// Motion compensation
function asyncReprojection(lastFrame: Frame, currentHeadPose: Pose): Frame {
  const delta = calculateDelta(lastFrame.headPose, currentHeadPose);
  return reproject(lastFrame.content, delta);
}

// Fallback for dropped frames
const MAX_DROP_CONSECUTIVE = 2;
const REPROJECTION_LATENCY_MS = 5;
```

---

## HALT CONDITIONS

**Stop immediately and report to user if ANY of these occur:**

```
CRITICAL HALT - DO NOT PROCEED:

[ ] UI outside comfort zone (30° cone)
[ ] Motion-to-photon latency > 20ms
[ ] Depth planes > 3
[ ] Stereo separation > 6.5cm
[ ] Virtual acceleration > 1.5g
[ ] Horizon line not fixed
[ ] Vertical scroll in VR detected
[ ] Session length > 60min without break
[ ] Audio latency > 15ms
[ ] Hand tracking latency > 30ms
[ ] Gaze detection latency > 50ms
[ ] Haptic latency > 10ms
[ ] Frame drops > 2 consecutive
[ ] Vestibular conflict detected
[ ] Accessibility fallback missing
[ ] Gesture has no voice alternative
[ ] Seated mode not available
[ ] One-hand mode not supported
[ ] Cognitive load exceeds threshold
[ ] Motion sickness symptoms reported
```

---

## LANGUAGE-SPECIFIC PATTERNS

### TypeScript (OpenXR/WebXR)

```typescript
// XR session guardrails
interface XRSessionConfig {
  comfortZone: {
    horizontalDegrees: number; // 30
    verticalDegrees: number; // 20
  };
  depthLayers: {
    maxCount: number; // 3
    maxSeparationCm: number; // 6.5
  };
  performance: {
    minFps: number; // 90
    maxFrameTimeMs: number; // 11.1
  };
}

// Validation
const XR_SAFETY_LIMITS = {
  latencyMaxMs: 20,
  accelMaxG: 1.5,
  sessionMaxMin: 60,
};
```

### Rust (XR frameworks)

```rust
// Spatial computing guardrails
#[derive(Component)]
struct SpatialSafety {
    comfort_angle_deg: f32,     // 30.0
    depth_layer_count: u8,      // 0-3 only
    stereo_separation_cm: f32,  // < 6.5
    motion_latency_ms: u16,     // < 20
    frame_budget_ms: f32,       // 11.1 for 90fps
}

impl SpatialSafety {
    fn validate(&self) -> Result<(), SafetyError> {
        if self.depth_layer_count > 3 {
            return Err(SafetyError::TooManyDepthLayers);
        }
        if self.motion_latency_ms > 20 {
            return Err(SafetyError::LatencyExceeded);
        }
        Ok(())
    }
}
```

---

## RELATED DOCUMENTS

| Document | Purpose |
|----------|---------|
| [AI_ASSISTED_DEV.md](../ai-dev/AI_ASSISTED_DEV.md) | AI development patterns and iteration safety |
| [2026_GAME_DESIGN.md](../game-design/2026_GAME_DESIGN.md) | Game design guardrails |
| [ACCESSIBILITY_GUIDE.md](../accessibility/ACCESSIBILITY_GUIDE.md) | WCAG 3.0+ compliance for spatial interfaces |

---

**Authored by:** Agent-GDUI-2026 Spatial Specialist
**Document Owner:** Spatial Computing Team
**Review Cycle:** Quarterly
**Last Review:** 2026-03-14
**Next Review:** 2026-06-14
**Compliance:** ISO 20885-1, OpenXR 1.0