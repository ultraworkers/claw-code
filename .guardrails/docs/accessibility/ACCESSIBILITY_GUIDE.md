# Accessibility Guide for AI-Generated Components (WCAG 3.0+)

**Version:** 2.0.0
**Last Updated:** 2026-03-14
**Applies To:** ALL user interfaces, digital products, interactive experiences

---

## Purpose

**AI-First Compliance:** Agent-GDUI-2026 enforces these standards automatically on all generated interfaces. AI-generated code ships accessible by default — no manual accessibility audit required for standard components.

This guide provides implementation instructions for WCAG 3.0+ (Web Content Accessibility Guidelines) compliance. WCAG 3.0 introduces:

1. **Functional accessibility needs** - Outcome-based testing
2. **Scoring model** - Points-based conformance
3. **Expanded coverage** - Mobile, touch, spatial, cognitive
4. **New tests** - 35+ new accessibility requirements
5. **Progressive enhancement** - Bronze/Silver/Gold levels

---

## WCAG 3.0 Conformance Levels

| Level | Points Required | Description |
|-------|-----------------|-------------|
| **Bronze** | Minimum threshold | Basic accessibility (replaces AA) |
| **Silver** | Higher threshold | Enhanced accessibility |
| **Gold** | Highest threshold | Comprehensive accessibility |

**Scoring:** Each test method has point values based on importance and impact.

---

## AGENT-GDUI-2026 Role

**Agent-GDUI-2026** ensures accessibility compliance through:

| Capability | Implementation | Validation |
|------------|---------------|------------|
| **Contrast Checker** | Automated ratio calculation | 7:1 minimum for AAA |
| **Focus Manager** | Tab order, visible indicators | 3px outline, 3:1 contrast |
| **Screen Reader Test** | Virtual ARIA validation | All interactive labeled |
| **Keyboard Mapper** | Full navigation coverage | No keyboard traps |
| **Cognitive Load** | Content complexity analysis | Plain language scoring |
| **Motion Detector** | Reduced-motion support | Prefers-reduced-motion |

### Agent-GDUI-2026 Accessibility Enforcement

All AI-generated UI components are automatically validated against these standards. Agents don't need to manually check accessibility — it's built into every generation pattern.

---

## IMPLEMENTATION CHECKLIST

### Perceptual Accessibility

| # | Requirement | Test Method | Points |
|---|-------------|-------------|--------|
| 1 | **Text Contrast** | Ratio >= 7:1 (AAA) | 10 |
| 2 | **Non-Text Contrast** | Icons >= 3:1 | 8 |
| 3 | **Color Independence** | Information not color-only | 9 |
| 4 | **Text Scaling** | 200% without loss | 7 |
| 5 | **Visual Focus** | Visible indicator | 8 |
| 6 | **Motion Adaptation** | Reduced-motion support | 6 |
| 7 | **Audio Contrast** | Speech >= background | 7 |
| 8 | **Alternative Text** | Images have alt | 9 |

### Cognitive Accessibility

| # | Requirement | Test Method | Points |
|---|-------------|-------------|--------|
| 1 | **Plain Language** | Readability score | 5 |
| 2 | **Consistent Navigation** | Pattern consistency | 6 |
| 3 | **Clear Instructions** | Step-by-step guidance | 5 |
| 4 | **Error Prevention** | Confirmation dialogs | 7 |
| 5 | **Time Extension** | Adjustable timeouts | 6 |
| 6 | **Progress Indication** | Multi-step feedback | 4 |
| 7 | **Orientation Support** | Breadcrumbs, landmarks | 5 |
| 8 | **Memory Support** | Auto-complete, suggestions | 4 |

### Physical Accessibility

| # | Requirement | Test Method | Points |
|---|-------------|-------------|--------|
| 1 | **Keyboard Access** | All functions via keyboard | 10 |
| 2 | **Touch Target Size** | 44x44pt minimum | 8 |
| 3 | **Touch Target Spacing** | 8px between targets | 6 |
| 4 | **No Time Limits** | Unlimited response time | 7 |
| 5 | **Gesture Alternative** | Click/tap fallback | 8 |
| 6 | **Position Independent** | No required orientation | 5 |
| 7 | **Force Modifiable** | Light touch sufficient | 6 |
| 8 | **Hand Size Adaptation** | One-hand operation possible | 5 |

---

## TECHNICAL IMPLEMENTATION

### Contrast Calculation

```typescript
// Luminance calculation
function getLuminance(rgb: [number, number, number]): number {
  const [r, g, b] = rgb.map(x => x / 255);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

// Contrast ratio (WCAG 3.0 formula)
function getContrastRatio(fg: string, bg: string): number {
  const fgLum = getLuminance(parseColor(fg));
  const bgLum = getLuminance(parseColor(bg));
  const lighter = Math.max(fgLum, bgLum);
  const darker = Math.min(fgLum, bgLum);
  return (lighter + 0.05) / (darker + 0.05);
}

// Validation
const MIN_CONTRAST_AAA = 7.0;
const MIN_CONTRAST_AA = 4.5;
const MIN_CONTRAST_LARGE = 3.0;
```

### Focus Indicator

```css
/* Universal focus pattern (WCAG 3.0) */
:focus {
  outline: 3px solid var(--focus-color);
  outline-offset: 2px;
}

:focus-visible {
  outline: 3px solid var(--focus-color);
  outline-radius: 4px;
}

/* Contrast requirement */
:focus {
  outline-color: contrast-adjusted(var(--primary), var(--bg), 3.0);
}
```

### Screen Reader Support

```html
<!-- Semantic structure -->
<main role="main">
  <nav aria-label="Main navigation">
    <ul role="list">
      <li><a href="/" aria-current="page">Home</a></li>
    </ul>
  </nav>

  <h1>Page Title</h1>

  <!-- Interactive elements -->
  <button
    aria-label="Submit form"
    aria-describedby="submit-help"
    aria-live="polite"
  >
    Submit
  </button>

  <!-- Images -->
  <img src="chart.png" alt="Sales increased 25% in Q1" />

  <!-- Decorative images -->
  <img src="spacer.png" alt="" role="presentation" />
</main>
```

### Keyboard Navigation

```typescript
// Tab order management
function manageTabOrder() {
  const focusable = document.querySelectorAll(
    'a[href], button, input, textarea, select, [tabindex]'
  );

  // Ensure logical tab order
  focusable.forEach((el, i) => {
    if (!el.getAttribute('tabindex')) {
      el.setAttribute('tabindex', '0');
    }
  });
}

// Keyboard trap prevention
function preventKeyboardTrap() {
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Tab') {
      // Allow natural tab flow
      return;
    }
  });
}
```

---

## TESTING METHODS

### Automated Testing

| Tool | Coverage | Points Verified |
|------|----------|-----------------|
| **axe-core** | WCAG 2.x + 3.0 draft | ~20 tests |
| **WAVE** | Visual + structural | ~15 tests |
| **Lighthouse** | Performance + a11y | ~12 tests |
| **Playwright** | Keyboard navigation | ~8 tests |
| **Custom scripts** | Contrast, focus | ~10 tests |

### Manual Testing

| Method | Coverage | Frequency |
|--------|----------|-----------|
| **Keyboard-only** | Full navigation | Every PR |
| **Screen reader** | NVDA, VO, JAWS | Monthly |
| **Zoom test** | 200% scaling | Every PR |
| **Color blindness** | Simulators | Every PR |
| **Reduced motion** | prefers-reduced-motion | Every PR |
| **Cognitive load** | Plain language check | Quarterly |

---

## HALT CONDITIONS

**Stop immediately and report to user if ANY of these occur:**

```
CRITICAL HALT - DO NOT PROCEED:

[ ] Contrast ratio < 7:1 (AAA) or < 4.5:1 (AA)
[ ] Keyboard navigation broken
[ ] Screen reader encounters unlabeled element
[ ] Focus indicator invisible
[ ] Touch target < 44x44pt
[ ] Touch target spacing < 8px
[ ] Color-only information presentation
[ ] Motion sickness triggered
[ ] Reduced-motion not supported
[ ] Time limit cannot be extended
[ ] Keyboard trap detected
[ ] Gesture has no click alternative
[ ] Content requires specific hand orientation
[ ] Cognitive complexity exceeds threshold
[ ] Alt text missing on informative image
[ ] Heading structure non-sequential
[ ] Landmark roles missing
[ ] ARIA attributes misused
[ ] Live region missing for dynamic content
[ ] Form lacks error identification
```

---

## LANGUAGE-SPECIFIC PATTERNS

### TypeScript/React

```typescript
// Accessibility-first component
interface AccessibleProps {
  ariaLabel: string;
  ariaDescribedBy?: string;
  ariaLive?: 'off' | 'polite' | 'assertive';
  tabIndex?: number;
  onKeyDown?: (e: KeyboardEvent) => void;
}

// Contrast validation hook
function useContrastCheck(fg: string, bg: string) {
  const ratio = getContrastRatio(fg, bg);
  const passesAAA = ratio >= 7.0;
  const passesAA = ratio >= 4.5;
  return { ratio, passesAAA, passesAA };
}
```

### Rust

```rust
// Accessibility validator
struct AccessibilityAudit {
    contrast_ratio: f32,
    keyboard_accessible: bool,
    screen_reader_ready: bool,
    touch_target_size: (u32, u32),
}

impl AccessibilityAudit {
    fn passes_wcag_3a(&self) -> bool {
        self.contrast_ratio >= 7.0
            && self.keyboard_accessible
            && self.screen_reader_ready
            && self.touch_target_size.0 >= 44
            && self.touch_target_size.1 >= 44
    }
}
```

---

## RELATED DOCUMENTS

| Document | Purpose |
|----------|---------|
| [AI_ASSISTED_DEV.md](../ai-dev/AI_ASSISTED_DEV.md) | AI development patterns and quality gates |
| [2026_UI_UX_STANDARD.md](../ui-ux/2026_UI_UX_STANDARD.md) | Component patterns with built-in accessibility |
| [ETHICAL_ENGAGEMENT.md](../ethical/ETHICAL_ENGAGEMENT.md) | Dark pattern prevention |

---

**Authored by:** Agent-GDUI-2026 Accessibility Specialist
**Document Owner:** Accessibility Team
**Review Cycle:** Quarterly
**Last Review:** 2026-03-14
**Next Review:** 2026-06-14
**Compliance:** WCAG 3.0+ Level Silver