# AI-Assisted Development Patterns

**Version:** 1.0.0
**Last Updated:** 2026-03-14
**Applies To:** ALL AI-driven development workflows, vibe coding sessions, agent-generated code

---

## Purpose

This document defines how AI agents should build software at high velocity. Guardrails aren't constraints on speed — they're what enable it. When agents know the boundaries, they spend tokens on building instead of safety-checking.

**Core Principle:** The fastest development happens when agents follow proven patterns instead of inventing new ones.

---

## The Vibe Coding Workflow

Vibe coding is AI-driven rapid development where agents generate, iterate, and ship at maximum speed. The guardrails in this framework make vibe coding safe by default.

### The Speed Equation

| Without Guardrails | With Guardrails |
|---|---|
| Agent generates code | Agent generates code |
| Agent second-guesses safety | ~~Safety check~~ (built-in) |
| Agent asks user for confirmation | ~~Confirmation~~ (pre-authorized) |
| Agent re-reads files for context | ~~Re-read~~ (trust the maps) |
| Agent checks accessibility | ~~Manual a11y~~ (patterns include it) |
| **Result: 40% of tokens on building** | **Result: 90% of tokens on building** |

---

## Decision Matrix: Ask vs Decide vs Halt

Not all decisions carry equal risk. Use this matrix to determine when agents should proceed autonomously, when to ask, and when to halt.

### Risk Level: LOW — Decide Autonomously

| Action | Example | Why Safe |
|--------|---------|----------|
| UI component generation | Build a button, modal, card | Pre-validated patterns exist |
| Styling changes | Colors, spacing, typography | Design tokens constrain choices |
| Test writing | Unit tests, integration tests | Tests are additive, low-risk |
| Documentation updates | Comments, README sections | Non-breaking, easily reversed |
| Refactoring within scope | Rename variable, extract function | Scoped and verifiable |

### Risk Level: MEDIUM — Ask Before Proceeding

| Action | Example | Why Ask |
|--------|---------|---------|
| New dependency addition | Adding a package | Supply chain risk |
| API schema changes | New endpoint, field change | Affects consumers |
| Database migrations | Schema change | Data integrity risk |
| Configuration changes | Environment variables | Deployment impact |
| Cross-module refactoring | Changing shared interfaces | Cascade risk |

### Risk Level: HIGH — Halt and Confirm

| Action | Example | Why Halt |
|--------|---------|----------|
| Authentication changes | Login flow, token handling | Security-critical |
| Payment integration | Billing, IAP, subscriptions | Financial risk |
| Data model changes | User schema, permissions | Data loss risk |
| Infrastructure changes | Deploy config, CI/CD | Production impact |
| Deletion of any kind | Files, data, accounts | Irreversible |

---

## Design-Intent Preservation

When iterating rapidly, agents must preserve the original design intent across generations.

### Style Anchors

Lock design decisions that should survive iteration:

```typescript
// STYLE ANCHOR: Do not modify these design tokens
const DESIGN_ANCHORS = {
  colorPrimary: '#2563eb',    // Brand blue — locked
  borderRadius: '8px',        // Consistent rounding — locked
  fontFamily: 'Inter',        // Typography — locked
  spacingUnit: 4,             // 4px grid — locked
} as const;
```

```rust
// STYLE ANCHOR: Layout constants — do not modify during iteration
const GRID_COLUMNS: u32 = 12;        // Locked
const GUTTER_WIDTH: f32 = 16.0;      // Locked
const MAX_CONTENT_WIDTH: f32 = 1200.0; // Locked
```

```go
// STYLE ANCHOR: Design tokens — locked across iterations
var DesignAnchors = struct {
    ColorPrimary string
    BorderRadius string
    MaxWidth     int
}{
    ColorPrimary: "#2563eb",  // Locked
    BorderRadius: "8px",      // Locked
    MaxWidth:     1200,        // Locked
}
```

### Intent Logs

When making changes, log what was intended vs what was changed:

```
INTENT: Add dark mode toggle to settings
CHANGED: settings/ThemeToggle.tsx (new component)
CHANGED: settings/index.tsx (added toggle to layout)
PRESERVED: All existing settings, color tokens, layout structure
```

---

## Prompt-to-UI Scaffolding

### Component Generation Flow

1. **Parse prompt** → Extract requirements (a11y, performance, ethics)
2. **Select pattern** → Match to existing UI pattern from 2026_UI_UX_STANDARD.md
3. **Apply constraints** → WCAG 3.0+, performance budget, ethical review
4. **Generate code** → Using design tokens and anchored styles
5. **Verify output** → Automated checks (lint, a11y, performance)

### Scaffold Templates

When generating UI components, start from these scaffolds rather than blank files:

```typescript
// Standard accessible component scaffold
interface ComponentProps {
  /** Required for accessibility */
  'aria-label': string;
  /** Visual state */
  variant?: 'primary' | 'secondary' | 'ghost';
  /** Size following 4px grid */
  size?: 'sm' | 'md' | 'lg';
}

export function Component({ 'aria-label': ariaLabel, variant = 'primary', size = 'md' }: ComponentProps) {
  return (
    <div
      role="region"
      aria-label={ariaLabel}
      className={`component component--${variant} component--${size}`}
    >
      {/* Generated content */}
    </div>
  );
}
```

---

## Iteration Safety

### Diff-Before-Overwrite

**MANDATORY:** Before overwriting any file, generate and review the diff.

```typescript
// Before overwriting, always:
// 1. Read current file content
// 2. Generate proposed changes
// 3. Diff current vs proposed
// 4. Verify intent is preserved
// 5. Apply changes
```

### Rollback Points

Create rollback points before significant changes:

| Change Type | Rollback Method | When |
|-------------|----------------|------|
| Single file edit | Git stash / undo | Before complex refactors |
| Multi-file change | Git branch | Before feature implementations |
| Configuration change | Backup file | Before env/config modifications |
| Database change | Migration rollback | Before schema changes |

### Progressive Enhancement

Build features incrementally, not all at once:

1. **Static first** — Get the HTML structure right
2. **Styled second** — Apply design tokens and layout
3. **Interactive third** — Add event handlers and state
4. **Accessible fourth** — Verify a11y (should be mostly done from patterns)
5. **Optimized fifth** — Performance tuning if needed

---

## Human Approval Gates

These areas ALWAYS require human approval before proceeding:

| Gate | Why | What to Present |
|------|-----|-----------------|
| **Authentication** | Security-critical | Full auth flow diagram |
| **Data Models** | Schema changes affect everything | ER diagram or schema diff |
| **Payment Integration** | Financial liability | Payment flow + error states |
| **Infrastructure** | Production impact | Change plan + rollback plan |
| **Third-Party APIs** | External dependencies | API contract + failure modes |
| **User Data Handling** | Privacy/legal | Data flow + retention policy |

---

## Design Tool Integration

### Figma Import

When receiving Figma designs:
1. Extract design tokens (colors, spacing, typography)
2. Map to existing component patterns
3. Generate components using extracted tokens
4. Verify visual match (screenshot comparison if available)

### Framer/Design Export

When exporting to design tools:
1. Export component API as design properties
2. Map state variants to design variants
3. Include accessibility annotations
4. Document interaction patterns

---

## HALT CONDITIONS

**STOP and ask the human when:**

- [ ] Authentication or authorization logic needs changing
- [ ] Payment or billing code is involved
- [ ] User data schema is being modified
- [ ] Infrastructure or deployment configuration changes
- [ ] Third-party API contracts are being established
- [ ] The design intent from the original prompt is unclear
- [ ] Three consecutive generation attempts haven't met requirements
- [ ] Ethical review flags a potential dark pattern
- [ ] Performance budget would be exceeded
- [ ] Accessibility compliance cannot be met with current approach

---

## Language Patterns

### TypeScript
```typescript
// AI-assisted development guard
function aiSafeOperation<T>(
  operation: () => T,
  rollback: () => void,
  riskLevel: 'low' | 'medium' | 'high'
): T {
  if (riskLevel === 'high') {
    throw new Error('HALT: High-risk operation requires human approval');
  }
  try {
    return operation();
  } catch (error) {
    rollback();
    throw error;
  }
}
```

### Rust
```rust
/// AI-assisted development guard
enum RiskLevel {
    Low,
    Medium,
    High,
}

fn ai_safe_operation<T, F, R>(operation: F, rollback: R, risk: RiskLevel) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
    R: FnOnce(),
{
    if matches!(risk, RiskLevel::High) {
        return Err("HALT: High-risk operation requires human approval".into());
    }
    operation().map_err(|e| {
        rollback();
        e
    })
}
```

### Go
```go
// AISafeOperation guards AI-driven operations by risk level
func AISafeOperation[T any](operation func() (T, error), rollback func(), risk RiskLevel) (T, error) {
    var zero T
    if risk == RiskHigh {
        return zero, fmt.Errorf("HALT: High-risk operation requires human approval")
    }
    result, err := operation()
    if err != nil {
        rollback()
        return zero, err
    }
    return result, nil
}
```

---

## RELATED DOCUMENTS

| Document | Purpose |
|----------|---------|
| [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) | Core safety protocols (The Four Laws) |
| [2026_GAME_DESIGN.md](../game-design/2026_GAME_DESIGN.md) | Game design guardrails |
| [2026_UI_UX_STANDARD.md](../ui-ux/2026_UI_UX_STANDARD.md) | UI component patterns |
| [ACCESSIBILITY_GUIDE.md](../accessibility/ACCESSIBILITY_GUIDE.md) | WCAG 3.0+ compliance |
| [ETHICAL_ENGAGEMENT.md](../ethical/ETHICAL_ENGAGEMENT.md) | Dark pattern prevention |
| [STATE_MANAGEMENT.md](../state/STATE_MANAGEMENT.md) | State architecture patterns |
| [PROMPTING_GUIDE.md](../../PROMPTING_GUIDE.md) | Effective prompting for AI development |
