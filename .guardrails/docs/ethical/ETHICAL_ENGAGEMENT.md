# Ethical Engagement Guide (Dark Pattern Prevention)

**Version:** 2.0.0
**Last Updated:** 2026-03-14
**Applies To:** ALL user interfaces, engagement systems, monetization features

---

## Purpose

**AI-Automated Ethics:** Dark pattern detection runs automatically on every AI-generated interface. Fast AI development doesn't mean compromising ethics — automation makes ethics faster, not slower.

This guide defines ethical engagement standards and provides detection/prevention for dark patterns. Dark patterns are UI designs that:

1. **Manipulate users** - Coerce unintended actions
2. **Hide information** - Obscure true costs/consequences
3. **Create addiction** - Exploit psychological vulnerabilities
4. **Remove choice** - Force compliance through design
5. **Prioritize profit** - Over user wellbeing

---

## Agent-GDUI-2026 Ethical Review

**Agent-GDUI-2026** includes automatic ethical engagement review:

| Capability | Detection | Action |
|------------|-----------|--------|
| **Dark Pattern Scanner** | Pattern matching | Automatic rejection |
| **Manipulation Detector** | Coercion analysis | User alert |
| **Addiction Loop Finder** | Engagement optimization | Redesign required |
| **Privacy UX Auditor** | Consent interface review | Compliance check |
| **Transparency Checker** | Information clarity | Enhancement suggestion |
| **Wellbeing Assessor** | User impact evaluation | Mitigation required |

### Automated Ethical Review

Agent-GDUI-2026 includes real-time ethical review on every UI generation. Agents don't need to manually audit for dark patterns — detection is automatic and rejection is immediate.

---

## DARK PATTERN TAXONOMY

### Category 1: Deceptive Interfaces

| Pattern | Description | Detection |
|---------|-------------|-----------|
| **Fake Urgency** | "3 people viewing!" false claims | Verify claim source |
| **Disguised Ads** | Native advertising without label | Check advertising disclosure |
| **Hidden Costs** | Fees revealed at checkout | Full price transparency audit |
| **Misleading Defaults** | Pre-select paid options | Default value analysis |
| **Obfuscated Pricing** | Complex pricing structures | Simplification requirement |

### Category 2: Coercive Interfaces

| Pattern | Description | Detection |
|---------|-------------|-----------|
| **Cookie Walls** | Block access until accept | Consent flow audit |
| **Privacy Ziggurat** | Default: track, opt-out complex | Privacy toggle analysis |
| **Forced Continuity** | No cancellation path | Cancellation flow test |
| **Friend Spam** | Upload contacts for pressure | Contact access audit |
| **Social Pressure** | FOMO exploitation | Social mechanic analysis |

### Category 3: Addictive Interfaces

| Pattern | Description | Detection |
|---------|-------------|-----------|
| **Engagement Optimization** | Maximize time-on-app | Analytics purpose audit |
| **Infinite Scroll** | No natural end points | Content boundary check |
| **Reward Randomization** | Variable reward loops | Reward schedule analysis |
| **Notification Spam** | Excessive push prompts | Notification frequency audit |
| **Stake Building** | Artificial investment | Virtual value audit |

### Category 4: Data Exploitation

| Pattern | Description | Detection |
|---------|-------------|-----------|
| **Data Brokerage** | Sell user data without consent | Data sharing disclosure |
| **Bread crumbs** | Track across unrelated sites | Cross-site tracking audit |
| **Preselection** | Third-party sharing default | Default consent analysis |
| **Deletion Difficulty** | Cannot delete data | Deletion flow test |
| **Surplus Extraction** | Extract value without return | Value exchange analysis |

---

## ETHICAL DESIGN PRINCIPLES

### Principle 1: Transparency

| Requirement | Implementation |
|-------------|---------------|
| **Clear Pricing** | Total cost visible upfront |
| **Labeled Ads** | "Advertisement" always visible |
| **Data Disclosure** | What data collected, why, shared |
| **Process Visibility** | What happens when user acts |
| **Third-Party Disclosure** | Who receives data |

### Principle 2: Choice

| Requirement | Implementation |
|-------------|---------------|
| **Easy Cancellation** | Cancel in ≤ 3 clicks |
| **Meaningful Defaults** | Default = user best interest |
| **Opt-In Consent** | Active consent required |
| **Granular Control** | Per-data-type toggle |
| **Exit Path** | Always-visible quit option |

### Principle 3: Wellbeing

| Requirement | Implementation |
|-------------|---------------|
| **Session Limits** | 60min continuous max |
| **Break Prompts** | Rest reminder after 45min |
| **No False Urgency** | Claims verified truthful |
| ** Addiction Prevention** | No variable reward loops |
| **Notification Limits** | ≤ 3/day promotional |

### Principle 4: Respect

| Requirement | Implementation |
|-------------|---------------|
| **Privacy by Default** | Minimal data collection |
| **Deletion Available** | Delete account + data |
| **No Contact Spam** | Don't upload contacts for pressure |
| **No Dark Patterns** | All patterns ethical-reviewed |
| **User Best Interest** | Design serves user |

---

## IMPLEMENTATION CHECKLIST

### Pre-Deployment Ethical Review

| # | Check | Requirement | Verify |
|---|-------|-------------|--------|
| 1 | **DARK PATTERN SCAN** | No patterns detected | [ ] |
| 2 | **PRICING TRANSPARENCY** | Total cost visible upfront | [ ] |
| 3 | **AD DISCLOSURE** | All ads labeled clearly | [ ] |
| 4 | **CANCELATION PATH** | ≤ 3 clicks to cancel | [ ] |
| 5 | **PRIVITY DEFAULTS** | Opt-in, not opt-out | [ ] |
| 6 | **DATA DISCLOSURE** | Collection/sharing disclosed | [ ] |
| 7 | **DELETION PATH** | Account + data deletion | [ ] |
| 8 | **SESSION LIMITS** | 60min max continuous | [ ] |
| 9 | **BREAK PROMPTS** | Rest reminder at 45min | [ ] |
| 10 | **NOTIFICATION LIMIT** | ≤ 3/day promotional | [ ] |
| 11 | **NO FALSE URGENCY** | Claims verified | [ ] |
| 12 | **NO ADDICTIVE LOOPS** | Variable rewards removed | [ ] |

---

## TECHNICAL IMPLEMENTATION

### Ethical Middleware

```go
// Ethical engagement middleware
func EthicalMiddleware(next http.Handler) http.Handler {
    return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
        // Reject dark pattern routes
        if isDarkPattern(r.URL.Path) {
            http.Error(w, "Ethical rejection: dark pattern detected", http.StatusForbidden)
            return
        }

        // Ensure pricing transparency
        if !hasPricingTransparency(r) {
            http.Error(w, "Ethical rejection: hidden costs", http.StatusBadRequest)
            return
        }

        // Verify ad disclosure
        if isAdContent(r) && !hasAdLabel(r) {
            http.Error(w, "Ethical rejection: undisclosed ad", http.StatusBadRequest)
            return
        }

        next.ServeHTTP(w, r)
    })
}
```

### Dark Pattern Detection

```typescript
// Pattern detection engine
interface DarkPattern {
  type: 'deceptive' | 'coercive' | 'addictive' | 'exploitation';
  severity: 'low' | 'medium' | 'high' | 'critical';
  pattern: string;
  remediation: string;
}

function detectDarkPattern(ui: UIElement): DarkPattern[] {
  const patterns: DarkPattern[] = [];

  // Check for fake urgency
  if (hasUrgencyClaim(ui) && !verifyClaim(ui)) {
    patterns.push({
      type: 'deceptive',
      severity: 'high',
      pattern: 'Fake Urgency',
      remediation: 'Remove unverified urgency claims'
    });
  }

  // Check for cookie walls
  if (blocksAccessBeforeConsent(ui)) {
    patterns.push({
      type: 'coercive',
      severity: 'critical',
      pattern: 'Cookie Wall',
      remediation: 'Allow browsing before consent'
    });
  }

  // Check for infinite scroll
  if (hasNoContentBoundary(ui)) {
    patterns.push({
      type: 'addictive',
      severity: 'medium',
      pattern: 'Infinite Scroll',
      remediation: 'Add natural content boundaries'
    });
  }

  return patterns;
}
```

### Cancellation Flow

```typescript
// Easy cancellation pattern
interface CancellationFlow {
  maxSteps: number; // 3
  requiresCall: boolean; // false
  requiresChat: boolean; // false
  immediateEffect: boolean; // true
  confirmationEmail: boolean; // true
}

// Validation
const CANCELATION_STANDARD = {
  maxSteps: 3,
  requiresPhoneContact: false,
  mustBeImmediate: true,
};
```

---

## HALT CONDITIONS

**Stop immediately and report to user if ANY of these occur:**

```
CRITICAL HALT - DO NOT PROCEED:

[ ] Dark pattern detected (any category)
[ ] Pricing not transparent
[ ] Ads not labeled
[ ] Cancellation > 3 clicks
[ ] Opt-out default (not opt-in)
[ ] Data sharing undisclosed
[ ] Deletion path unavailable
[ ] Session limit not enforced
[ ] Break prompts missing
[ ] Notification frequency > 3/day
[ ] False urgency detected
[ ] Addiction loops present
[ ] Contact spam enabled
[ ] Privacy by default violated
[ ] Variable reward schedule detected
[ ] Engagement optimization for time-on-app
[ ] Cross-site tracking undisclosed
[ ] Third-party sharing default-on
[ ] Stake building without disclosure
[ ] User wellbeing at risk
```

---

## LANGUAGE-SPECIFIC PATTERNS

### TypeScript (React Ethics)

```typescript
// Ethical component wrapper
interface EthicalProps {
  hasTransparentPricing: boolean;
  hasAdDisclosure: boolean;
  hasCancelationPath: boolean;
  maxStepsToCancel: number;
  isOptIn: boolean;
  dataDisclosure: string;
}

// Dark pattern rejection
function EthicalGuard({ children, audit }: Props) {
  const patterns = detectDarkPattern(children);

  if (patterns.length > 0) {
    throw new EthicalError('Dark pattern detected');
  }

  return children;
}
```

### Go (Ethical Engagement)

```go
// Ethical engagement validator
type EthicalAudit struct {
    DarkPatternsDetected []string
    PricingTransparent   bool
    AdDisclosurePresent  bool
    CancellationSteps    int
    OptInDefault         bool
    DataDisclosed        bool
    DeletionAvailable    bool
}

func (a *EthicalAudit) Passes() bool {
    return len(a.DarkPatternsDetected) == 0 &&
        a.PricingTransparent &&
        a.AdDisclosurePresent &&
        a.CancellationSteps <= 3 &&
        a.OptInDefault &&
        a.DataDisclosed &&
        a.DeletionAvailable
}
```

---

## RELATED DOCUMENTS

| Document | Purpose |
|----------|---------|
| [MONETIZATION_GUARDRAILS.md](../monetization/MONETIZATION_GUARDRAILS.md) | IAP ethics and spending protections |
| [ANALYTICS_ETHICS.md](../analytics/ANALYTICS_ETHICS.md) | Telemetry and behavioral tracking ethics |
| [GENERATIVE_ASSET_SAFETY.md](../generative/GENERATIVE_ASSET_SAFETY.md) | AI-generated content safety |
| [AI_ASSISTED_DEV.md](../ai-dev/AI_ASSISTED_DEV.md) | AI development approval gates |

---

**Authored by:** Agent-GDUI-2026 Ethics Specialist
**Document Owner:** Ethical Engagement Team
**Review Cycle:** Quarterly
**Last Review:** 2026-03-14
**Next Review:** 2026-06-14
**Compliance:** EU DSA, GDPR, FTC Guidance