# Multiplayer & Social Safety

**Version:** 1.0.0
**Last Updated:** 2026-03-14
**Applies To:** ALL multiplayer systems, social features, user-generated content, chat, matchmaking

---

## Purpose

Multiplayer and social features are the highest-liability surface in any application. AI agents can scaffold chat systems, matchmaking, and UGC pipelines at speed — but without safety guardrails, they'll ship systems that enable harassment, expose minors, or violate content laws. These patterns let agents build social features rapidly with safety built in.

**Core Principle:** Every social feature is a trust and safety feature. Build the safety first, the feature second.

---

## Presence & Social Graph

### Privacy-First Presence

| Feature | Default | User Control | Agent Rule |
|---------|---------|--------------|------------|
| Online status | Hidden | Opt-in to show | Default to private |
| Activity display | Hidden | Opt-in per activity | Never expose without consent |
| Location sharing | Disabled | Explicit opt-in only | HALT if auto-enabled |
| Friend list | Private | Visible to friends only | Never expose to strangers |
| Play history | Private | Opt-in to share | Never auto-share |

### Social Graph Rules

| Rule | Requirement |
|------|-------------|
| Blocking | Immediate and complete — blocked user sees nothing |
| Muting | Per-channel mute with easy toggle |
| Reporting | 1-tap report with category selection |
| Friend requests | Must be confirmable, not auto-accepted |
| Unfriending | Silent — no notification to removed friend |

---

## Matchmaking Fairness

### Prohibited Matchmaking Patterns

| Pattern | Why Prohibited | Detection |
|---------|---------------|-----------|
| Skill-based manipulation for spending | Matching low-skill vs high-skill to drive purchases | Win-rate vs spend correlation analysis |
| Engagement-optimized matchmaking | Prioritizing session length over fairness | Session length vs match quality audit |
| Smurf account enabling | No barriers to creating alt accounts | Account age + performance analysis |
| Cheater pooling without disclosure | Secretly matching suspected cheaters | Transparency requirement |
| Region manipulation | Forcing high-latency matches | Latency threshold enforcement |

### Fair Matchmaking Requirements

```typescript
interface MatchmakingConfig {
  /** Skill rating system (Elo, Glicko-2, TrueSkill) */
  ratingSystem: 'elo' | 'glicko2' | 'trueskill';
  /** Maximum skill gap between matched players */
  maxSkillGap: number;
  /** Maximum acceptable latency (ms) */
  maxLatency: number;
  /** Time before widening skill search (seconds) */
  searchWidenInterval: number;
  /** PROHIBITED: monetization influence on matchmaking */
  monetizationInfluence: false;  // Must always be false
}
```

---

## Chat & Communication Moderation

### Real-Time Chat Safety

| Layer | Implementation | Latency Budget |
|-------|---------------|----------------|
| Profanity filter | Keyword + regex matching | < 5ms |
| Toxicity detection | ML model classification | < 50ms |
| PII detection | Pattern matching (SSN, email, phone) | < 10ms |
| Spam detection | Rate limiting + pattern matching | < 5ms |
| Language detection | Auto-detect for localized moderation | < 20ms |

### Chat Moderation Pipeline

```
Message → Rate Limiter → PII Filter → Profanity Filter → Toxicity ML → Deliver/Block
              ↓              ↓              ↓                  ↓
          Block spam     Redact PII    Replace/block      Flag/block
          Notify user    Log attempt   Log attempt        Log + escalate
```

### Moderation Actions

| Severity | Action | Duration | Appeal |
|----------|--------|----------|--------|
| Low (mild language) | Warning | N/A | N/A |
| Medium (harassment) | Temp mute | 1-24 hours | Automated |
| High (threats, hate speech) | Temp ban | 1-7 days | Human review |
| Critical (CSAM, violence threats) | Permanent ban + report | Permanent | Legal team |

---

## Harassment Prevention

### Anti-Harassment Features (Mandatory)

| Feature | Requirement | Implementation |
|---------|-------------|----------------|
| Block user | Blocks all communication channels | Immediate, all channels |
| Report user | Categorized reporting | Category picker + evidence capture |
| Mute voice | Per-user voice mute | Toggle in player menu |
| Hide messages | Remove messages from view | Client-side filtering |
| Cooldown period | Forced break after reports | Automatic 5-min cooldown |
| Safe mode | Disable all social features | 1-toggle enable |

### Vulnerability Protection

| Vulnerable Group | Protection | Enforcement |
|-----------------|------------|-------------|
| Minors (< 13) | No direct messaging | Age gate + feature gate |
| Minors (13-17) | Restricted communication | Parental controls default on |
| New users | Graduated social access | Feature unlock over time |
| Reported users | Reduced social reach | Automatic after N reports |

---

## Content Moderation (User-Generated Content)

### UGC Pipeline

```
Upload → Virus Scan → Format Validation → Content Safety AI → Human Review Queue → Publish
           ↓              ↓                      ↓                    ↓
        Block +        Reject +              Flag/Block           Approve/Reject
        quarantine     format error          Auto-action          With feedback
```

### CSAM Detection (MANDATORY)

| Requirement | Implementation | Non-Negotiable |
|-------------|---------------|----------------|
| PhotoDNA or equivalent | Hash-based detection | YES — legal requirement |
| NCMEC reporting | Automatic report pipeline | YES — legal requirement |
| Evidence preservation | Secure, access-controlled storage | YES — legal requirement |
| Staff training | Annual CSAM handling training | YES — legal requirement |

**CRITICAL:** Any system accepting user-uploaded images or video MUST implement CSAM detection. This is a legal requirement in most jurisdictions. HALT immediately if this is not in scope.

### UGC Content Categories

| Category | Auto-Action | Human Review |
|----------|-------------|--------------|
| Safe | Publish | Random audit (5%) |
| Questionable | Hold | Required before publish |
| Unsafe | Block | Notification to trust team |
| Illegal | Block + report | Immediate legal escalation |

---

## Trust & Safety Operations

### Escalation Matrix

| Trigger | Response Time | Escalation Path |
|---------|---------------|-----------------|
| CSAM detection | Immediate | Legal team + NCMEC |
| Credible threat of violence | < 15 minutes | Trust & Safety lead + law enforcement |
| Hate speech / extremism | < 1 hour | Trust & Safety team |
| Harassment report | < 4 hours | Moderation team |
| General content report | < 24 hours | Moderation queue |

### Transparency Reporting

| Metric | Frequency | Public |
|--------|-----------|--------|
| Content removed | Quarterly | Yes |
| Accounts banned | Quarterly | Yes |
| Law enforcement requests | Semi-annually | Yes |
| False positive rate | Quarterly | Yes |
| Average response time | Monthly | Internal |

---

## HALT CONDITIONS

**STOP and ask the human when:**

- [ ] Direct messaging between users is being implemented
- [ ] User-generated content upload is being built
- [ ] Age verification or minor protection is in scope
- [ ] Real-time voice or video chat is being added
- [ ] Matchmaking algorithms are being designed
- [ ] Content moderation pipeline is being architected
- [ ] Any feature that connects strangers is being built
- [ ] CSAM detection integration is needed (legal requirement)
- [ ] Law enforcement cooperation procedures are needed

---

## Language Patterns

### TypeScript
```typescript
// Chat message safety pipeline
async function processMessage(
  message: ChatMessage,
  sender: User,
): Promise<MessageResult> {
  // Rate limiting
  if (await isRateLimited(sender.id)) {
    return { blocked: true, reason: 'Rate limited' };
  }
  // PII detection
  const sanitized = redactPII(message.content);
  // Toxicity check
  const toxicity = await classifyToxicity(sanitized);
  if (toxicity.score > 0.8) {
    await flagForReview(message, sender, toxicity);
    return { blocked: true, reason: 'Content policy violation' };
  }
  return { delivered: true, content: sanitized };
}
```

### Rust
```rust
/// Chat message safety validation
async fn process_message(
    message: &ChatMessage,
    sender: &User,
) -> Result<ProcessedMessage, ModerationAction> {
    if is_rate_limited(&sender.id).await {
        return Err(ModerationAction::RateLimited);
    }
    let sanitized = redact_pii(&message.content);
    let toxicity = classify_toxicity(&sanitized).await;
    if toxicity.score > 0.8 {
        flag_for_review(message, sender, &toxicity).await;
        return Err(ModerationAction::Blocked("Content policy violation"));
    }
    Ok(ProcessedMessage { content: sanitized })
}
```

### Go
```go
// ProcessMessage validates and moderates a chat message
func ProcessMessage(ctx context.Context, msg *ChatMessage, sender *User) (*ProcessedMessage, error) {
    if IsRateLimited(ctx, sender.ID) {
        return nil, &ModerationError{Reason: "Rate limited"}
    }
    sanitized := RedactPII(msg.Content)
    toxicity, err := ClassifyToxicity(ctx, sanitized)
    if err != nil {
        return nil, fmt.Errorf("toxicity check failed: %w", err)
    }
    if toxicity.Score > 0.8 {
        FlagForReview(ctx, msg, sender, toxicity)
        return nil, &ModerationError{Reason: "Content policy violation"}
    }
    return &ProcessedMessage{Content: sanitized}, nil
}
```

---

## RELATED DOCUMENTS

| Document | Purpose |
|----------|---------|
| [ETHICAL_ENGAGEMENT.md](../ethical/ETHICAL_ENGAGEMENT.md) | Dark pattern prevention |
| [MONETIZATION_GUARDRAILS.md](../monetization/MONETIZATION_GUARDRAILS.md) | IAP and economy ethics |
| [ANALYTICS_ETHICS.md](../analytics/ANALYTICS_ETHICS.md) | Player telemetry ethics |
| [AI_ASSISTED_DEV.md](../ai-dev/AI_ASSISTED_DEV.md) | AI development approval gates |
| [GENERATIVE_ASSET_SAFETY.md](../generative/GENERATIVE_ASSET_SAFETY.md) | Generated content safety |
