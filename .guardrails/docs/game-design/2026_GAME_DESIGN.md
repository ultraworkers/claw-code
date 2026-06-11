# 2026 Game Design Guardrails

**Version:** 2.0.0
**Last Updated:** 2026-03-14
**Applies To:** ALL game development projects, UI/UX implementations, spatial computing applications

---

## Purpose

This document enables AI agents to rapidly build game interfaces, spatial computing experiences, and immersive applications with built-in safety. These guardrails are your license to generate at full velocity — they ensure:

1. **Accessibility compliance** - WCAG 3.0+ standards enforced
2. **Ethical engagement** - Dark patterns prevented
3. **Spatial safety** - XR/VR/AR comfort guidelines
4. **Performance bounds** - Frame rate, latency constraints
5. **Platform consistency** - Cross-platform UX standards

---

## Agent-GDUI-2026 Role Definition

**Agent-GDUI-2026** (Game Design & UI 2026) is the specialized agent role responsible for:

| Capability | Description | Constraint |
|------------|-------------|------------|
| **Spatial Layout** | XR viewport, depth, layering | Comfort zones enforced |
| **Motion Design** | Animation, transitions, physics | 60fps minimum, 120fps target |
| **Audio Spatialization** | 3D audio positioning | HRTF calibration required |
| **Input Mapping** | Multi-modal input handlers | Accessibility priority |
| **Ethical Review** | Dark pattern detection | Automatic rejection |
| **Performance Budget** | Resource allocation | Frame-time budgets strict |

### AI-Optimized Development

These standardized patterns exist so agents don't reinvent the wheel on every generation. When building game UIs:
- **Use the constraint tables as checklists** — they're pre-validated, so you can apply them without analysis
- **Performance budgets are pre-calculated** — no need to benchmark from scratch
- **Comfort zones are defined** — spatial placement is safe by default

---

## CORE PRINCIPLES

### The Four Laws of Spatial Safety

1. **Comfort First** - Never induce motion sickness or discomfort
2. **Accessibility Required** - WCAG 3.0+ compliance mandatory
3. **Performance Bound** - Maintain frame rate budgets strictly
4. **Ethical Engagement** - Reject dark pattern implementations

---

## SAFETY PROTOCOLS (MANDATORY)

### Pre-Implementation Checklist

**EVERY agent MUST verify these before ANY UI implementation:**

| # | Check | Requirement | Verify |
|---|-------|-------------|--------|
| 1 | **READ DESIGN SPEC** | Review spatial computing guidelines | [ ] |
| 2 | **ACCESSIBILITY REVIEW** | WCAG 3.0+ compliance verified | [ ] |
| 3 | **PERFORMANCE BUDGET** | Frame-time budget calculated | [ ] |
| 4 | **COMFORT ZONE CHECK** | XR elements in safe zones | [ ] |
| 5 | **INPUT FALLBACKS** | Alternative input paths defined | [ ] |
| 6 | **ETHICAL REVIEW** | No dark patterns detected | [ ] |
| 7 | **PLATFORM TARGETS** | Platform-specific constraints known | [ ] |

### XR/VR Comfort Rules

| Rule | Threshold | Consequence |
|------|-----------|-------------|
| **NO LATENCY > 20ms** | Motion-to-photon < 20ms | Motion sickness prevention |
| **NO ACCEL > 1.5g** | Virtual acceleration bounded | Vestibular comfort |
| **COMFORT ZONE ONLY** | UI within 30° cone | Neck strain prevention |
| **NO VERTICAL SCROLL** | Avoid vertical motion in VR | Nausea prevention |
| **FIXED REFERENCE** | Horizon line always visible | Orientation maintenance |
| **SESSION LIMITS** | 60min max continuous use | Fatigue prevention |

### Accessibility Requirements (WCAG 3.0+)

| Requirement | Level | Implementation |
|-------------|-------|---------------|
| **Contrast Ratio** | AAA | 7:1 minimum for text |
| **Focus Indicators** | AA | 3px outline, 3:1 contrast |
| **Keyboard Navigation** | A | All interactive elements |
| **Screen Reader Support** | AA | ARIA labels required |
| **Reduced Motion** | A | Prefers-reduced-motion support |
| **Color Independence** | AA | Information not color-only |
| **Text Resizing** | AA | 200% zoom without breaking |
| **Audio Labels** | A | All audio cues have text fallback |

### Performance Budgets

| Platform | Frame Target | Budget | Constraint |
|----------|--------------|--------|------------|
| **Mobile** | 60fps | 16.6ms/frame | Thermal throttling awareness |
| **PC** | 144fps | 6.9ms/frame | GPU sync required |
| **Console** | 60/120fps | 8.3/16.6ms | V-Sync mandatory |
| **XR Headset** | 90fps minimum | 11.1ms/frame | Drop frames = nausea |
| **VR High-End** | 120fps | 8.3ms/frame | Async reprojection |

### Dark Pattern Prevention

**ABSOLUTELY FORBIDDEN:**

```
REJECTED PATTERNS:

[ ] Cookie walls that block access
[ ] Hidden unsubscribe paths
[ ] Confusing privacy toggles (default: track)
[ ] Fake urgency ("3 people viewing!")
[ ] Obfuscated pricing (hidden fees)
[ ] Forced continuity (no cancellation path)
[ ] Disguised ads (native advertising without label)
[ ] Data brokerage (selling user data without consent)
[ ] Addiction loops (engagement optimization without breaks)
[ ] Social pressure mechanics (FOMO exploitation)
```

---

## PLATFORM-SPECIFIC RULES

### Mobile (iOS/Android)

| Constraint | Requirement |
|------------|-------------|
| **Touch Target** | 44x44pt minimum (Apple HIG) |
| **Thumb Zone** | Primary actions in bottom 40% |
| **Reachability** | Max 2 thumb stretches |
| **Haptic Feedback** | Consistent with action weight |
| **Battery Awareness** | Pause heavy computation < 20% battery |

### PC (Desktop/Web)

| Constraint | Requirement |
|------------|-------------|
| **Mouse Precision** | 1px hover targets acceptable |
| **Keyboard Shortcuts** | Ctrl/Cmd + letter for primary actions |
| **Right-Click Menu** | Context menu for complex actions |
| **Resizable Windows** | Fluid layout 800px-2560px |
| **Multi-Monitor** | Cross-monitor continuity |

### Console (Xbox/PlayStation/Switch)

| Constraint | Requirement |
|------------|-------------|
| **Controller Mapping** | A/B/X/Y semantic consistency |
| **HUD Safe Zone** | 10% inset from all edges |
| **Text Size** | 24pt minimum at 10ft viewing |
| **Color Blindness** | Deuteranopia/protanopia simulation test |
| **Audio Mix** | Dialogue priority over music |

### XR (VR/AR/MR)

| Constraint | Requirement |
|------------|-------------|
| **Comfort Zone** | 30° horizontal, 20° vertical |
| **Depth Layers** | Max 3 depth planes |
| **Parallax** | Stereo separation < 6.5cm |
| **Motion Coherence** | Visual/vestibular alignment |
| **Exit Path** | Always-visible home/quit gesture |

---

## HALT CONDITIONS

**Stop immediately and report to user if ANY of these occur:**

```
CRITICAL HALT - DO NOT PROCEED:

[ ] Accessibility compliance not verifiable
[ ] Performance budget exceeded
[ ] XR comfort thresholds violated
[ ] Dark pattern detected in design
[ ] Platform guidelines conflict
[ ] User agent (deuteranopia, etc.) not simulated
[ ] Frame drops detected in profiling
[ ] Audio latency > 20ms
[ ] Input latency > 50ms
[ ] Thermal throttling predicted
[ ] Battery drain excessive
[ ] Motion sickness symptoms reported
[ ] WCAG 3.0+ criteria unclear
[ ] Ethical review inconclusive
```

---

## LANGUAGE-SPECIFIC PATTERNS

### TypeScript/React (Web/Mobile)

```typescript
// Accessibility-first component pattern
interface AccessibleButtonProps {
  label: string;
  ariaLabel?: string;
  onKeyDown?: (e: KeyboardEvent) => void;
  onFocus?: () => void;
}

// Motion preference detection
const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion)').matches;

// Performance budget enforcement
const FRAME_BUDGET_MS = 16.6; // 60fps target
```

### Rust (Bevy Game Engine)

```rust
// Spatial computing guardrails
#[derive(Component)]
struct ComfortZone {
    max_angle_degrees: f32, // 30.0 for comfort
    depth_layer: u8,        // 0-2 only
}

// Performance budget tracker
struct FrameBudget {
    target_fps: u32,        // 90 for XR, 60 for mobile
    max_frame_time_ms: f32,
}
```

### Go (HTMX Patterns)

```go
// Ethical engagement middleware
func EthicalMiddleware(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        // Reject dark pattern routes
        if isDarkPattern(r.URL.Path) {
            http.Error(w, "Ethical rejection", http.StatusForbidden)
            return
        }
        next.ServeHTTP(w, r)
    })
}
```

---

## RELATED DOCUMENTS

| Document | Purpose |
|----------|---------|
| [AI_ASSISTED_DEV.md](../ai-dev/AI_ASSISTED_DEV.md) | AI development patterns and decision matrix |
| [MONETIZATION_GUARDRAILS.md](../monetization/MONETIZATION_GUARDRAILS.md) | IAP ethics and economy balance |
| [MULTIPLAYER_SAFETY.md](../multiplayer/MULTIPLAYER_SAFETY.md) | Multiplayer and social safety |
| [ANALYTICS_ETHICS.md](../analytics/ANALYTICS_ETHICS.md) | Telemetry and tracking ethics |
| [CROSS_PLATFORM_DEPLOYMENT.md](../deployment/CROSS_PLATFORM_DEPLOYMENT.md) | App store compliance and CI/CD |
| [STATE_MANAGEMENT.md](../state/STATE_MANAGEMENT.md) | State architecture patterns |

---

**Authored by:** Agent-GDUI-2026 Specialist
**Document Owner:** Game Design & UI/UX Team
**Review Cycle:** Quarterly
**Last Review:** 2026-03-14
**Next Review:** 2026-06-14
**Compliance:** WCAG 3.0+ Level AAA, ISO 20885-1