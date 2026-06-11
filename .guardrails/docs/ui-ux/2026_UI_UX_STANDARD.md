# 2026 UI/UX Standard Reference

**Version:** 2.0.0
**Last Updated:** 2026-03-14
**Applies To:** ALL user interface implementations, component libraries, design systems

---

## Purpose

This document establishes the 2026 UI/UX standard for AI agents implementing user interfaces. These patterns are optimized for AI generation — agents can compose complex UIs by combining proven, pre-validated components instead of designing from scratch.

It defines:

1. **Component patterns** - Reusable UI building blocks
2. **Design tokens** - Consistent visual language
3. **Interaction models** - User behavior expectations
4. **Accessibility integration** - WCAG 3.0+ embedded
5. **Cross-platform adaptation** - Responsive, adaptive patterns

---

## Agent-GDUI-2026 Capabilities

**Agent-GDUI-2026** provides specialized UI/UX capabilities:

| Capability | Scope | Output |
|------------|-------|--------|
| **Component Generation** | React, Vue, Svelte, HTMX | Accessible by default |
| **Design Token Sync** | Colors, spacing, typography | Platform-adapted |
| **Interaction Modeling** | Hover, focus, active states | Multi-modal |
| **Animation System** | Transitions, easing, duration | Reduced-motion aware |
| **Layout Adaptation** | Mobile, tablet, desktop, XR | Breakpoint-aware |
| **Theme Management** | Light, dark, high-contrast | System-sync |

### AI Generation Optimization

These component patterns exist so agents don't reinvent UI primitives on every generation:
- **Pre-validated accessibility** — every pattern meets WCAG 3.0+ out of the box
- **Consistent design tokens** — spacing, color, and typography are standardized
- **Composable building blocks** — combine patterns for complex interfaces without custom design work

---

## CORE COMPONENTS

### Foundational Components

| Component | Purpose | Accessibility |
|-----------|---------|---------------|
| **Button** | Primary action trigger | Keyboard, screen reader, focus |
| **Input** | Data entry field | Label, error, autocomplete |
| **Modal** | Overlay dialog | Trap focus, ESC close, aria |
| **Navigation** | Route/section change | Landmark, aria-current |
| **Card** | Content container | Semantic heading, alt |
| **List** | Item collection | Role, tabindex, keyboard |
| **Table** | Data grid | Scope, caption, keyboard nav |
| **Form** | Input grouping | Fieldset, legend, error summary |

### Advanced Components

| Component | Purpose | Constraint |
|-----------|---------|------------|
| **SpatialViewport** | XR container | Comfort zone bounded |
| **DepthLayer** | Z-axis positioning | Max 3 layers |
| **MotionController** | Animation manager | Reduced-motion detection |
| **AudioSpatializer** | 3D audio positioning | HRTF calibration |
| **HapticFeedback** | Touch feedback | Platform API sync |
| **GestureRecognizer** | Multi-touch input | Fallback to click |

---

## DESIGN TOKENS

### Color Palette

```css
/* 2026 Design Token Standard */
:root {
  /* Primary - Brand identity */
  --color-primary: #2563eb;
  --color-primary-hover: #1d4ed8;
  --color-primary-active: #1e40af;

  /* Semantic - Action meaning */
  --color-success: #16a34a;
  --color-warning: #f59e0b;
  --color-error: #dc2626;
  --color-info: #0284c7;

  /* Neutral - Grayscale */
  --color-neutral-0: #ffffff;
  --color-neutral-50: #f5f5f5;
  --color-neutral-100: #e5e5e5;
  --color-neutral-900: #1a1a1a;

  /* Contrast - AAA compliance */
  --contrast-ratio-min: 7.0; /* WCAG AAA */
}
```

### Typography Scale

```css
/* Responsive type scale */
:root {
  --font-size-xs: 0.75rem;   /* 12px */
  --font-size-sm: 0.875rem;  /* 14px */
  --font-size-base: 1rem;    /* 16px */
  --font-size-lg: 1.125rem;  /* 18px */
  --font-size-xl: 1.25rem;   /* 20px */
  --font-size-2xl: 1.5rem;   /* 24px */
  --font-size-3xl: 2rem;     /* 32px */

  /* Line height - Readability */
  --line-height-tight: 1.25;
  --line-height-normal: 1.5;
  --line-height-relaxed: 1.75;

  /* Weight - Semantic */
  --font-weight-normal: 400;
  --font-weight-medium: 500;
  --font-weight-bold: 700;
}
```

### Spacing Scale

```css
/* 4pt base grid */
:root {
  --spacing-0: 0;
  --spacing-1: 0.25rem;  /* 4px */
  --spacing-2: 0.5rem;   /* 8px */
  --spacing-3: 0.75rem;  /* 12px */
  --spacing-4: 1rem;     /* 16px */
  --spacing-5: 1.25rem;  /* 20px */
  --spacing-6: 1.5rem;   /* 24px */
  --spacing-8: 2rem;     /* 32px */
  --spacing-12: 3rem;    /* 48px */
  --spacing-16: 4rem;    /* 64px */
}
```

---

## INTERACTION STATES

### State Requirements

| State | Requirement | Implementation |
|-------|-------------|---------------|
| **Default** | Base appearance | Contrast 7:1 |
| **Hover** | Pointer over element | 10% color shift |
| **Focus** | Keyboard/tab focus | 3px outline, 3:1 contrast |
| **Active** | Pressed/activated | 20% color shift |
| **Disabled** | Non-interactive | 50% opacity, aria-disabled |
| **Loading** | Async operation | Spinner + aria-live |
| **Error** | Validation failure | Red + text explanation |

### Focus Indicator Standard

```css
/* Universal focus pattern */
:focus {
  outline: 3px solid var(--color-primary);
  outline-offset: 2px;
  outline-radius: 2px;
}

:focus:not(:focus-visible) {
  outline: none;
}

:focus-visible {
  outline: 3px solid var(--color-primary);
}
```

---

## ANIMATION GUIDELINES

### Motion Principles

| Principle | Constraint | Purpose |
|-----------|------------|---------|
| **Duration** | 150-300ms | Perceptible, not jarring |
| **Easing** | ease-out | Natural deceleration |
| **Distance** | Max 300px | Avoid disorientation |
| **Frequency** | Max 3 simultaneous | Cognitive load limit |
| **Reduced Motion** | Transform-only | Accessibility support |

### Motion Implementation

```css
/* Standard transition */
.transition-standard {
  transition: all 200ms ease-out;
}

/* Emphasis animation */
@keyframes emphasize {
  0% { transform: scale(1); }
  50% { transform: scale(1.05); }
  100% { transform: scale(1); }
}

/* Reduced motion fallback */
@media (prefers-reduced-motion: reduce) {
  * {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

---

## RESPONSIVE BREAKPOINTS

### Platform Adaptation

| Breakpoint | Width | Platform | Layout |
|------------|-------|----------|--------|
| **xs** | < 640px | Mobile portrait | Single column |
| **sm** | 640-768px | Mobile landscape | 2 column |
| **md** | 768-1024px | Tablet | 3 column |
| **lg** | 1024-1280px | Laptop | 4 column |
| **xl** | 1280-1536px | Desktop | 5 column |
| **2xl** | > 1536px | Large desktop | 6 column |

### Responsive Pattern

```css
/* Mobile-first approach */
.container {
  padding: var(--spacing-4);
}

@media (min-width: 640px) {
  .container {
    padding: var(--spacing-6);
    display: grid;
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (min-width: 1024px) {
  .container {
    grid-template-columns: repeat(4, 1fr);
  }
}
```

---

## HALT CONDITIONS

**Stop immediately and report to user if ANY of these occur:**

```
CRITICAL HALT - DO NOT PROCEED:

[ ] Contrast ratio < 7:1 (WCAG AAA)
[ ] Focus indicator missing
[ ] Keyboard navigation broken
[ ] Screen reader label absent
[ ] Touch target < 44x44pt
[ ] Animation triggers motion sickness
[ ] Reduced motion not supported
[ ] Breakpoint causes layout break
[ ] Design token undefined
[ ] Component prop mismatch
[ ] Theme sync failed
[ ] Cross-platform test failed
[ ] Interaction state unclear
[ ] Loading state missing
[ ] Error state not accessible
```

---

## LANGUAGE-SPECIFIC PATTERNS

### TypeScript/React

```typescript
// Accessible component pattern
interface ButtonProps {
  variant: 'primary' | 'secondary' | 'danger';
  size: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  loading?: boolean;
  ariaLabel?: string;
  onKeyDown?: (e: KeyboardEvent) => void;
}

// Design token usage
const token = useDesignToken();
const color = token.color.primary;
```

### Rust (Leptos/Yew)

```rust
// Component with accessibility
#[component]
fn Button(
    variant: String,
    #[prop(default = "md")] size: String,
    aria_label: Option<String>,
) -> Html {
    html! {
        <button
            class={format!("btn-{variant} btn-{size}")}
            aria-label={aria_label}
        />
    }
}
```

### Go (HTMX)

```go
// Server-rendered accessible button
func Button(w http.ResponseWriter, r *http.Request) {
    tmpl := `
    <button
        class="btn-primary"
        aria-label="Submit form"
        hx-post="/submit"
        hx-indicator=".loading"
    >Submit</button>
    `
    io.WriteString(w, tmpl)
}
```

---

## RELATED DOCUMENTS

| Document | Purpose |
|----------|---------|
| [AI_ASSISTED_DEV.md](../ai-dev/AI_ASSISTED_DEV.md) | AI development patterns and prompt-to-UI scaffolding |
| [STATE_MANAGEMENT.md](../state/STATE_MANAGEMENT.md) | Client/server state architecture |
| [ACCESSIBILITY_GUIDE.md](../accessibility/ACCESSIBILITY_GUIDE.md) | Full WCAG 3.0+ compliance guide |

---

**Authored by:** Agent-GDUI-2026 Specialist
**Document Owner:** UI/UX Standards Team
**Review Cycle:** Quarterly
**Last Review:** 2026-03-14
**Next Review:** 2026-06-14
**Compliance:** WCAG 3.0+ Level AAA