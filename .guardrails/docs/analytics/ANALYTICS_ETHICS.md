# Analytics & Telemetry Ethics

**Version:** 1.0.0
**Last Updated:** 2026-03-14
**Applies To:** ALL analytics, telemetry, event tracking, A/B testing, behavioral data collection

---

## Purpose

Analytics are essential for product improvement — but AI agents can generate tracking code faster than humans can review its privacy implications. These guardrails ensure agents build analytics that respect user consent, minimize data collection, and avoid manipulative targeting.

**Core Principle:** Collect only what you need, only when permitted, and only for the stated purpose.

---

## Event Tracking Consent

### Consent Tiers

| Tier | Data Type | Consent Required | Default |
|------|-----------|-----------------|---------|
| Essential | Crash reports, security events | None (legitimate interest) | On |
| Functional | Feature usage, performance metrics | Implicit (ToS) | On |
| Analytics | Behavioral patterns, session data | Explicit opt-in | Off |
| Advertising | Cross-app tracking, ad targeting | Explicit opt-in | Off |
| Research | User studies, surveys | Explicit informed consent | Off |

### Consent Implementation

```typescript
interface ConsentState {
  essential: true;      // Always on — not toggleable
  functional: boolean;  // Default on, user can disable
  analytics: boolean;   // Default OFF — requires opt-in
  advertising: boolean; // Default OFF — requires opt-in
  research: boolean;    // Default OFF — requires explicit consent
}

// Consent must be checked before EVERY tracking call
function track(event: AnalyticsEvent, consent: ConsentState): void {
  if (!consent[event.tier]) {
    return; // Silently skip — do not queue
  }
  sendEvent(event);
}
```

### Consent UI Requirements

| Requirement | Rule |
|-------------|------|
| Equal prominence | "Accept" and "Decline" buttons same size and style |
| No dark patterns | No pre-checked boxes, no guilt-tripping copy |
| Granular control | Per-tier toggles, not just "Accept All" |
| Easy withdrawal | Same path to withdraw as to grant |
| Persistent choice | Remember choice, don't re-ask every session |
| Age-appropriate | Simplified consent for minors, parental gate for under-13 |

---

## Data Minimization

### The Minimization Checklist

Before adding ANY tracking event, the agent MUST verify:

| # | Question | If No |
|---|----------|-------|
| 1 | Is this data needed for a stated product purpose? | Don't collect |
| 2 | Can we achieve the purpose with less data? | Reduce scope |
| 3 | Can we use aggregated instead of individual data? | Aggregate |
| 4 | Can we use anonymous instead of identified data? | Anonymize |
| 5 | Is there a retention limit defined? | Define one before collecting |

### Data Retention Limits

| Data Type | Maximum Retention | Deletion Method |
|-----------|-------------------|-----------------|
| Session events | 90 days | Automatic purge |
| Performance metrics | 1 year | Aggregate, then purge raw |
| Crash reports | 1 year | Automatic purge |
| User behavior | 90 days | Anonymize, then purge |
| A/B test results | 6 months after test ends | Aggregate, then purge |
| Advertising data | 30 days | Automatic purge |

### PII Handling

| Data | Classification | Allowed in Analytics |
|------|---------------|---------------------|
| User ID | PII | Hashed only |
| Email | PII | Never |
| IP Address | PII | Truncated (/24 for IPv4) |
| Device ID | Quasi-PII | Hashed, rotated quarterly |
| Location | PII | City-level max, never precise |
| Name | PII | Never |
| Age | Sensitive PII | Age bracket only (e.g., "18-24") |

---

## Behavioral Targeting Limits

### Prohibited Targeting

| Targeting Type | Why Prohibited | Alternative |
|---------------|---------------|-------------|
| Addiction-prone users | Exploitation of vulnerability | Engagement caps for all users |
| High-spending users | Whale hunting | Equal offers to all |
| Emotionally vulnerable | Exploitation of state | Context-free targeting only |
| Children specifically | Legal + ethical | Age-appropriate content for all |
| Inferred health status | Privacy violation | No health inference |
| Political beliefs | Manipulation risk | No political inference |

### Allowed Targeting

| Targeting Type | Condition | Example |
|---------------|-----------|---------|
| Feature adoption | Consented analytics | "Users who haven't tried X" |
| Platform/device | Technical necessity | "iOS users for iOS-specific feature" |
| Language/region | Localization | "Spanish-speaking users" |
| Plan/tier | Business logic | "Free users for upgrade prompt" |

---

## A/B Testing Ethics

### Mandatory Rules

| Rule | Requirement | Enforcement |
|------|-------------|-------------|
| Informed consent | Users know they may be in experiments | ToS disclosure + opt-out |
| No harm | Experiments must not degrade experience | Minimum quality threshold |
| Statistical rigor | Adequate sample size, proper analysis | Pre-registered hypothesis |
| Time-bound | Maximum experiment duration defined | Auto-end date |
| Rollback plan | Can revert to control at any time | Feature flag required |

### Prohibited Experiments

| Experiment Type | Why Prohibited |
|----------------|---------------|
| Pricing experiments without disclosure | Discriminatory pricing |
| Emotional manipulation studies | Facebook Emotional Contagion precedent |
| Withholding safety features | Endangers users |
| Dark pattern A/B tests | Testing manipulation effectiveness |
| Experiments on minors without parental consent | Legal + ethical |

### Experiment Template

```typescript
interface Experiment {
  id: string;
  name: string;
  hypothesis: string;            // Pre-registered
  startDate: Date;
  endDate: Date;                 // MANDATORY — no indefinite experiments
  sampleSize: number;            // Statistically significant
  minimumQualityThreshold: number; // Control must not degrade > X%
  rollbackPlan: string;          // How to revert
  ethicsReview: boolean;         // MUST be true before launch
  consentTier: 'functional' | 'analytics'; // Which consent level needed
}
```

---

## Algorithmic Transparency

### User-Facing Explanations

When algorithms affect user experience, provide explanations:

| Algorithm | What to Disclose | How |
|-----------|-----------------|-----|
| Content ranking | "Why am I seeing this?" | Tooltip or info button |
| Recommendations | "Based on your [X]" | Explanation text |
| Matchmaking | Rating/rank | Visible skill rating |
| Pricing | Factors affecting price | Price breakdown |
| Content moderation | Why content was flagged | Specific policy reference |

### Audit Requirements

| Requirement | Frequency | Output |
|-------------|-----------|--------|
| Bias audit | Quarterly | Fairness metrics by demographic |
| Accuracy audit | Monthly | False positive/negative rates |
| Impact audit | Quarterly | User experience metrics by cohort |
| Privacy audit | Semi-annually | Data flow and retention review |

---

## HALT CONDITIONS

**STOP and ask the human when:**

- [ ] New tracking events are being added to production
- [ ] PII is being collected or processed
- [ ] Cross-app or cross-site tracking is proposed
- [ ] A/B testing framework is being designed
- [ ] Behavioral targeting logic is being implemented
- [ ] Analytics data is being shared with third parties
- [ ] Consent flow UI is being modified
- [ ] Data retention policies are being set or changed
- [ ] Algorithmic decision-making affects user access or pricing

---

## Language Patterns

### TypeScript
```typescript
// Privacy-safe analytics wrapper
class SafeAnalytics {
  private consent: ConsentState;

  constructor(consent: ConsentState) {
    this.consent = consent;
  }

  track(event: string, data: Record<string, unknown>, tier: ConsentTier): void {
    if (!this.consent[tier]) return;

    // Strip PII before sending
    const sanitized = this.stripPII(data);

    // Add consent proof
    sendEvent({
      event,
      data: sanitized,
      consentTier: tier,
      consentTimestamp: this.consent.grantedAt,
    });
  }

  private stripPII(data: Record<string, unknown>): Record<string, unknown> {
    const piiFields = ['email', 'name', 'phone', 'ip', 'address'];
    return Object.fromEntries(
      Object.entries(data).filter(([key]) => !piiFields.includes(key))
    );
  }
}
```

### Rust
```rust
/// Privacy-safe event tracking
fn track_event(
    event: &str,
    data: &HashMap<String, Value>,
    tier: ConsentTier,
    consent: &ConsentState,
) -> Result<(), AnalyticsError> {
    if !consent.is_granted(tier) {
        return Ok(()); // Silently skip
    }
    let sanitized = strip_pii(data);
    send_event(event, &sanitized, tier)
}

fn strip_pii(data: &HashMap<String, Value>) -> HashMap<String, Value> {
    let pii_fields = ["email", "name", "phone", "ip", "address"];
    data.iter()
        .filter(|(k, _)| !pii_fields.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}
```

### Go
```go
// SafeTrack sends analytics events only with proper consent
func SafeTrack(ctx context.Context, event string, data map[string]any, tier ConsentTier, consent *ConsentState) error {
    if !consent.IsGranted(tier) {
        return nil // Silently skip
    }
    sanitized := StripPII(data)
    return SendEvent(ctx, event, sanitized, tier)
}

// StripPII removes personally identifiable information from analytics data
func StripPII(data map[string]any) map[string]any {
    piiFields := map[string]bool{"email": true, "name": true, "phone": true, "ip": true, "address": true}
    result := make(map[string]any)
    for k, v := range data {
        if !piiFields[k] {
            result[k] = v
        }
    }
    return result
}
```

---

## RELATED DOCUMENTS

| Document | Purpose |
|----------|---------|
| [ETHICAL_ENGAGEMENT.md](../ethical/ETHICAL_ENGAGEMENT.md) | Dark pattern prevention |
| [MONETIZATION_GUARDRAILS.md](../monetization/MONETIZATION_GUARDRAILS.md) | Economy tracking ethics |
| [MULTIPLAYER_SAFETY.md](../multiplayer/MULTIPLAYER_SAFETY.md) | Player data in social systems |
| [AI_ASSISTED_DEV.md](../ai-dev/AI_ASSISTED_DEV.md) | AI development approval gates |
| [GENERATIVE_ASSET_SAFETY.md](../generative/GENERATIVE_ASSET_SAFETY.md) | Generated content provenance |
