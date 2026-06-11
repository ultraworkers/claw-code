# Monetization & Economy Guardrails

**Version:** 1.0.0
**Last Updated:** 2026-03-14
**Applies To:** ALL monetization systems, in-app purchases, virtual economies, subscription services

---

## Purpose

Monetization is where speed and ethics collide. AI agents can rapidly generate purchase flows, loot boxes, and engagement loops — but without guardrails, they'll recreate every dark pattern in the industry. These rules let agents build monetization at full velocity while staying ethical.

**Core Principle:** Fair monetization builds trust and retention. Exploitative monetization builds short-term revenue and long-term liability.

---

## In-App Purchase (IAP) Ethics

### Mandatory Requirements

| Requirement | Rule | Enforcement |
|-------------|------|-------------|
| Price transparency | Show real currency, not just virtual | Display both currencies |
| Refund path | Clear refund mechanism visible | UI audit |
| Purchase confirmation | Double-confirm purchases > $5 | Confirmation dialog required |
| Receipt/record | Transaction history accessible | Persistent transaction log |
| No pressure tactics | No countdown timers on purchases | Timer detection + block |
| Age-appropriate | Age gate for purchases | Age verification flow |

### Platform Requirements

| Platform | Key Rule | Agent Action |
|----------|----------|--------------|
| Apple App Store | All digital purchases via IAP | Use StoreKit only |
| Google Play | Digital goods via Play Billing | Use Play Billing Library |
| Steam | Valve payment system | Use Steamworks Commerce |
| Meta Quest | Meta payment system | Use Meta Platform SDK |
| Epic Games Store | Epic payment rail | Use Epic Online Services |

---

## Loot Box Transparency

### Mandatory Disclosure

ALL randomized reward mechanics MUST disclose:

| Disclosure | Requirement | Format |
|------------|-------------|--------|
| Drop rates | Exact percentages for every item tier | Table or list, visible before purchase |
| Pity system | Guaranteed reward after N attempts | Clear description of mechanic |
| Duplicate handling | What happens with duplicate items | Explicit policy |
| Real money cost | Cost per attempt in real currency | Visible alongside virtual currency |
| Best/worst case | Min and max value of possible rewards | Range displayed |

### Prohibited Patterns

| Pattern | Why Prohibited | Detection |
|---------|---------------|-----------|
| Hidden drop rates | Deceptive | Audit for undisclosed RNG |
| Pay-to-win items | Unfair competitive advantage | Item power audit |
| Expiring purchases | Artificial urgency | Expiration timer detection |
| Undisclosed pity timers | Manipulative | RNG implementation audit |
| Cross-promotion loot | Advertising disguised as rewards | Content source audit |

```typescript
// Loot box transparency implementation
interface LootBoxConfig {
  name: string;
  costReal: number;       // Real currency cost
  costVirtual: number;    // Virtual currency cost
  dropTable: DropRate[];  // MUST be publicly visible
  pityThreshold: number;  // Guaranteed reward after N pulls
  duplicatePolicy: 'convert' | 'stack' | 'discard';
}

interface DropRate {
  tier: string;
  itemPool: string[];
  probability: number;  // 0.0-1.0, all tiers must sum to 1.0
  displayed: boolean;   // MUST be true
}
```

---

## Subscription Fairness

| Rule | Requirement | Enforcement |
|------|-------------|-------------|
| Easy cancellation | 1-click cancel, same path as subscribe | UI flow audit |
| No dark patterns | No guilt-tripping on cancel screen | Copy review |
| Pro-rated refunds | Partial month = partial charge | Billing logic check |
| Trial transparency | Clear trial end date and auto-charge | Notification required |
| Downgrade path | Can downgrade without canceling | Tier management UI |
| Price change notice | 30-day advance notice | Notification system |

---

## Virtual Economy Balance

### Economy Health Metrics

| Metric | Healthy Range | Warning | Critical |
|--------|--------------|---------|----------|
| Currency inflation | < 5% monthly | 5-10% | > 10% |
| Item price stability | ±10% over 30 days | ±25% | ±50% |
| Earn-to-spend ratio | 70-100% of content earnable | 50-70% | < 50% |
| Whale concentration | < 20% of revenue from top 1% | 20-40% | > 40% |

### Economy Rules

```typescript
// Virtual economy constraints
interface EconomyConstraints {
  /** Maximum real-money spend per day (USD) */
  dailySpendCap: number;
  /** Maximum real-money spend per month (USD) */
  monthlySpendCap: number;
  /** Percentage of content earnable without paying */
  freeToPlayViability: number; // Must be >= 0.7 (70%)
  /** Minimum conversion rate (virtual currency to real value) */
  currencyFloor: number;
  /** Maximum markup on virtual currency bundles */
  maxBundleMarkup: number; // e.g., 1.5 = 50% markup max
}
```

---

## Battle Pass Patterns

### Ethical Battle Pass Rules

| Rule | Requirement |
|------|-------------|
| Earnable free tier | Meaningful rewards without paying |
| Reasonable completion | Completable with 1hr/day play |
| No expiration pressure | Season length clearly communicated |
| No pay-to-skip | Skip options don't create competitive advantage |
| Value transparency | Total value of pass clearly shown |

### Prohibited Battle Pass Patterns

| Pattern | Why |
|---------|-----|
| FOMO-driven exclusive items | Artificial scarcity manipulation |
| Impossible free-tier completion | Coercion to upgrade |
| Time-gating with paid skip | Pay-to-not-grind exploitation |
| Hidden tier requirements | Deceptive progression |

---

## Age-Gated Spending

### Spending Limits by Age

| Age Group | Daily Limit | Monthly Limit | Parental Control |
|-----------|-------------|---------------|------------------|
| Under 13 | $0 (no purchases) | $0 | Required |
| 13-15 | $10 | $30 | Default on, can disable |
| 16-17 | $25 | $100 | Available, default off |
| 18+ | Platform default | Platform default | N/A |

### Implementation Requirements

```typescript
interface AgeGate {
  /** User's verified age bracket */
  ageBracket: 'under13' | 'teen13' | 'teen16' | 'adult';
  /** Daily spend remaining (USD) */
  dailyRemaining: number;
  /** Monthly spend remaining (USD) */
  monthlyRemaining: number;
  /** Parental control active */
  parentalControl: boolean;
  /** Purchase blocked reason (if any) */
  blockReason?: string;
}
```

---

## HALT CONDITIONS

**STOP and ask the human when:**

- [ ] Payment integration architecture is being designed
- [ ] Loot box or gacha mechanics are being implemented
- [ ] Virtual currency exchange rates are being set
- [ ] Age verification system is being designed
- [ ] Subscription billing logic is being written
- [ ] Economy balance parameters are being configured
- [ ] Real-money trading or marketplace features are planned
- [ ] Platform-specific payment compliance is uncertain
- [ ] Refund or chargeback handling logic is needed

---

## Language Patterns

### TypeScript
```typescript
// Purchase safety guard
async function safePurchase(
  userId: string,
  item: PurchaseItem,
  ageGate: AgeGate,
): Promise<PurchaseResult> {
  if (ageGate.ageBracket === 'under13') {
    return { blocked: true, reason: 'Purchases disabled for users under 13' };
  }
  if (item.priceUSD > ageGate.dailyRemaining) {
    return { blocked: true, reason: 'Daily spending limit reached' };
  }
  // Proceed with platform-specific purchase flow
  return await platformPurchase(userId, item);
}
```

### Rust
```rust
/// Purchase safety validation
fn validate_purchase(
    user: &User,
    item: &PurchaseItem,
    age_gate: &AgeGate,
) -> Result<(), PurchaseError> {
    if age_gate.age_bracket == AgeBracket::Under13 {
        return Err(PurchaseError::AgeRestricted);
    }
    if item.price_usd > age_gate.daily_remaining {
        return Err(PurchaseError::DailyLimitExceeded);
    }
    Ok(())
}
```

### Go
```go
// ValidatePurchase checks spending limits and age gates
func ValidatePurchase(user *User, item *PurchaseItem, gate *AgeGate) error {
    if gate.AgeBracket == AgeBracketUnder13 {
        return &PurchaseError{Reason: "Purchases disabled for users under 13"}
    }
    if item.PriceUSD > gate.DailyRemaining {
        return &PurchaseError{Reason: "Daily spending limit reached"}
    }
    return nil
}
```

---

## RELATED DOCUMENTS

| Document | Purpose |
|----------|---------|
| [ETHICAL_ENGAGEMENT.md](../ethical/ETHICAL_ENGAGEMENT.md) | Dark pattern prevention |
| [ANALYTICS_ETHICS.md](../analytics/ANALYTICS_ETHICS.md) | Telemetry ethics for economy tracking |
| [2026_GAME_DESIGN.md](../game-design/2026_GAME_DESIGN.md) | Game design guardrails |
| [AI_ASSISTED_DEV.md](../ai-dev/AI_ASSISTED_DEV.md) | AI development approval gates |
| [CROSS_PLATFORM_DEPLOYMENT.md](../deployment/CROSS_PLATFORM_DEPLOYMENT.md) | Platform compliance |
