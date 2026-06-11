# Vibe Coding Principles

These principles are MANDATORY for all AI-driven rapid development sessions.

## Principle 1: Guardrails Enable Speed

**Constraints are your fast lane, not your brake.**

### Why:
- Without guardrails, you waste tokens on safety verification
- With guardrails, you spend tokens on building
- Pre-validated patterns eliminate design-from-scratch overhead
- Automated compliance (a11y, ethics, perf) means no manual audits

### Application:
- Follow established patterns from 2026_UI_UX_STANDARD.md
- Use design tokens instead of custom values
- Trust the constraint tables — they're pre-validated
- Apply component scaffolds from AI_ASSISTED_DEV.md

---

## Principle 2: Decide by Risk Level

**Not every decision needs human input.**

### Decision Matrix:
- **LOW risk** (styling, tests, docs) → Decide autonomously
- **MEDIUM risk** (dependencies, APIs, config) → Ask before proceeding
- **HIGH risk** (auth, payments, data models, infra) → HALT and confirm

### Application:
- Classify every action by risk before executing
- When in doubt, escalate to the next risk level
- Speed comes from confident autonomous decisions on low-risk work
- Never guess on high-risk decisions — asking is faster than fixing

---

## Principle 3: Preserve Design Intent

**Fast iteration must not destroy the foundation.**

### Requirements:
- Lock style anchors (design tokens, brand colors, typography)
- Log what changed and what was preserved
- Diff before overwriting — never blind-write
- Progressive enhancement: static → styled → interactive → accessible

### Application:
- Read before editing (Law 1 of Agent Safety)
- Create rollback points before significant changes
- Use intent logs to track design decisions across iterations

---

## Principle 4: Ship Accessible by Default

**Accessibility is not a feature — it's a requirement baked into every pattern.**

### Requirements:
- All generated UI meets WCAG 3.0+ automatically
- Use pre-validated component patterns with built-in a11y
- Keyboard navigation, screen reader support, and color contrast are non-negotiable
- Test with prefers-reduced-motion and high-contrast mode

### Application:
- Start from accessible scaffolds, not blank files
- Use semantic HTML before adding ARIA
- Every interactive element must be keyboard-operable
- Dark pattern detection runs automatically on every generation

---

## Principle 5: Iterate, Don't Rebuild

**Refine existing work instead of starting over.**

### Requirements:
- Read current code before generating replacements
- Make minimal, targeted changes
- Preserve working functionality while adding new features
- Use the existing component library before creating new components

### Application:
- Extract and reuse before duplicating
- Small diffs are easier to review and less likely to break things
- If three iterations haven't worked, HALT and rethink the approach

---

## Reference

Full documentation: `docs/ai-dev/AI_ASSISTED_DEV.md`
Decision matrix: `docs/ai-dev/AI_ASSISTED_DEV.md` → Decision Matrix section
Component patterns: `docs/ui-ux/2026_UI_UX_STANDARD.md`
