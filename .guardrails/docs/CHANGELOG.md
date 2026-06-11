# Changelog

All notable changes to the Agent Guardrails Template will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [2.7.0] - 2026-03-14

### Major Release: 2026 UI/UX Game Design Update

**Version:** 2.7.0
**Release Date:** 2026-03-14
**Type:** Major Version Bump (breaking changes in documentation structure)

---

### New Features

#### 2026 Game Design Patterns

**Agent-GDUI-2026** role definition added for specialized game interface and spatial computing development:

- **Spatial Layout** - XR viewport management, depth layering, comfort zone enforcement
- **Motion Design** - Animation guidelines with 60fps minimum, 120fps target
- **Audio Spatialization** - 3D audio positioning with HRTF calibration
- **Input Mapping** - Multi-modal input handlers with accessibility priority
- **Ethical Review** - Automatic dark pattern detection and rejection
- **Performance Budget** - Frame-rate budgets with strict latency constraints

**Documentation Added:**
- [2026_GAME_DESIGN.md](game-design/2026_GAME_DESIGN.md) - Core game design guardrails
  - XR/VR comfort zones (30° cone, 20ms latency)
  - Platform-specific rules (Mobile, PC, Console, XR)
  - Performance budgets per platform
  - Language-specific patterns (TypeScript, Rust, Go)

- [2026_UI_UX_STANDARD.md](ui-ux/2026_UI_UX_STANDARD.md) - UI/UX component standards
  - Foundational components (Button, Input, Modal, Navigation)
  - Design tokens (color, typography, spacing)
  - Interaction states (hover, focus, active, disabled)
  - Animation guidelines with reduced-motion support
  - Responsive breakpoints (xs, sm, md, lg, xl, 2xl)

- [ACCESSIBILITY_GUIDE.md](accessibility/ACCESSIBILITY_GUIDE.md) - WCAG 3.0+ implementation
  - WCAG 3.0 conformance levels (Bronze/Silver/Gold)
  - Perceptual accessibility (contrast, color independence)
  - Cognitive accessibility (plain language, consistent navigation)
  - Physical accessibility (keyboard, touch targets, gestures)
  - Automated and manual testing methods

- [SPATIAL_COMPUTING_UI.md](spatial/SPATIAL_COMPUTING_UI.md) - XR/VR/AR UI patterns
  - Comfort zones and latency requirements
  - UI layout patterns (VR, AR, MR)
  - Depth layering (3-layer standard)
  - Interaction patterns (gesture, gaze, haptic)
  - Performance budgets (90fps minimum for XR)

- [ETHICAL_ENGAGEMENT.md](ethical/ETHICAL_ENGAGEMENT.md) - Dark pattern prevention
  - Dark pattern taxonomy (deceptive, coercive, addictive, exploitation)
  - Ethical design principles (transparency, choice, wellbeing, respect)
  - Implementation checklist
  - Technical implementation (middleware, detection)

#### Language-Specific Pattern Examples

**TypeScript/React:**
- Accessibility-first component patterns
- Design token usage hooks
- Dark pattern detection components
- Ethical component wrappers

**Rust:**
- Bevy game engine guardrails
- Spatial computing safety components
- Accessibility audit validators
- Leptos/Yew accessible components

**Go:**
- HTMX accessible patterns
- Ethical engagement middleware
- Dark pattern detection handlers
- Server-rendered UI components

**Additional Languages:**
- Java examples
- Python examples
- Ruby examples
- Swift/SwiftUI examples
- Scala functional UI examples
- R game analytics examples

---

### Accessibility Compliance

**WCAG 3.0+ Level Silver** certification requirements:

| Requirement | Level | Implementation |
|-------------|-------|---------------|
| Contrast Ratio | AAA | 7:1 minimum for text |
| Focus Indicators | AA | 3px outline, 3:1 contrast |
| Keyboard Navigation | A | All interactive elements |
| Screen Reader Support | AA | ARIA labels required |
| Reduced Motion | A | Prefers-reduced-motion support |
| Color Independence | AA | Information not color-only |
| Text Resizing | AA | 200% zoom without breaking |

**Badge:** [![WCAG 3.0+ Silver](https://img.shields.io/badge/WCAG-3.0+_Silver-green.svg)](docs/accessibility/ACCESSIBILITY_GUIDE.md)

---

### Spatial Computing Support

**XR/VR/AR/MR** platform support added:

| Platform | Frame Target | Latency Budget | Constraint |
|----------|--------------|---------------|------------|
| Mobile VR | 60fps | 16.6ms/frame | Thermal awareness |
| PC VR | 144fps | 6.9ms/frame | GPU sync required |
| Console VR | 60/120fps | 8.3/16.6ms | V-Sync mandatory |
| XR Headset | 90fps minimum | 11.1ms/frame | Drop frames = nausea |
| VR High-End | 120fps | 8.3ms/frame | Async reprojection |

**Comfort Zone Enforcement:**
- 30° horizontal, 20° vertical primary cone
- Max 3 depth planes
- Stereo separation < 6.5cm
- Motion-to-photon latency < 20ms

---

### Ethical Engagement Compliance

**Dark Pattern Prevention** automatic enforcement:

**Forbidden Patterns:**
- Fake urgency ("3 people viewing!")
- Cookie walls that block access
- Hidden costs revealed at checkout
- Misleading defaults (pre-select paid)
- Disguised ads (native advertising without label)
- Forced continuity (no cancellation path)
- Addiction loops (variable reward schedules)
- Data brokerage (selling user data without consent)

**Ethical Requirements:**
- Cancellation in ≤ 3 clicks
- Opt-in defaults (not opt-out)
- Transparent pricing upfront
- All ads clearly labeled
- Session limits (60min max continuous)
- Break prompts at 45min
- Notification limits (≤ 3/day promotional)

---

### Documentation Structure Changes

**New Documentation Categories:**

| Category | Documents | Purpose |
|----------|-----------|---------|
| Game Design | 2026_GAME_DESIGN.md | Game interface guardrails |
| UI/UX | 2026_UI_UX_STANDARD.md | Component standards |
| Accessibility | ACCESSIBILITY_GUIDE.md | WCAG 3.0+ implementation |
| Spatial Computing | SPATIAL_COMPUTING_UI.md | XR/VR/AR patterns |
| Ethical Engagement | ETHICAL_ENGAGEMENT.md | Dark pattern prevention |

**Navigation Tools:**
- [INDEX_MAP.md](INDEX_MAP.md) - Keyword/category search (saves 60-80% tokens)
- [HEADER_MAP.md](HEADER_MAP.md) - Section-level file:line references

---

### Breaking Changes

**Documentation Structure:**
- Added 5 new documentation directories:
  - `docs/game-design/`
  - `docs/ui-ux/`
  - `docs/accessibility/`
  - `docs/spatial/`
  - `docs/ethical/`
- INDEX_MAP.md and HEADER_MAP.md now required for navigation
- TOC.md created for complete file listing

**Version Bump:**
- v1.x → v2.7.0 (major version bump for 2026 UI/UX update)
- MCP Server remains at v2.6.0 (Go implementation unchanged)

---

### Migration Guide

**From v1.x to v2.7.0:**

1. Read [INDEX_MAP.md](docs/INDEX_MAP.md) for new document locations
2. Use [HEADER_MAP.md](docs/HEADER_MAP.md) for section-level lookup
3. Review [2026_GAME_DESIGN.md](docs/game-design/2026_GAME_DESIGN.md) for game development
4. Implement [ACCESSIBILITY_GUIDE.md](docs/accessibility/ACCESSIBILITY_GUIDE.md) WCAG 3.0+ requirements
5. Enable [ETHICAL_ENGAGEMENT.md](docs/ethical/ETHICAL_ENGAGEMENT.md) dark pattern detection

---

### Documentation Statistics

| Metric | v1.x | v2.7.0 | Change |
|--------|------|--------|--------|
| Total Files | 31 | 36 | +5 |
| Total Lines | ~9,000 | ~11,500 | +2,500 |
| 500-Line Compliance | 30/31 (97%) | 35/36 (97%) | Maintained |
| New Categories | 0 | 5 | Game Design, UI/UX, Accessibility, Spatial, Ethical |

---

### Credits

**Authored by:** Agent-GDUI-2026 Specialist Team
**Review Cycle:** Quarterly
**Compliance:** WCAG 3.0+ Level Silver, ISO 20885-1, EU DSA, GDPR

---

## [1.10.0] - 2026-02-15

### Summary

MCP Server Go migration complete. Web UI deployed. Team tools stabilized.

**See:** [RELEASE_v1.10.0.md](RELEASE_v1.10.0.md) for details.

---

## [1.9.6] - 2026-02-14

### Summary

Final v1.x release before 2.0.0 major update.

**See:** [RELEASE_v1.9.6.md](RELEASE_v1.9.6.md) for details.

---

## [1.9.0] - [1.9.5] - 2026-01-20 to 2026-02-10

### Summary

Incremental improvements to MCP server, team tools, and documentation.

**See:** Individual release files (RELEASE_v1.9.0.md through RELEASE_v1.9.5.md) for details.

---

## [Unreleased]

Future releases will include:
- Additional language-specific examples
- Extended spatial computing patterns
- Enhanced ethical engagement detection
- Platform-specific accessibility guides

---

**Last Updated:** 2026-03-14
**Version:** 2.7.0
**Maintainer:** TheArchitectit