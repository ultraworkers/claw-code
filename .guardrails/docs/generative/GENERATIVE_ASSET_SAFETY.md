# Generative Asset Safety

**Version:** 1.0.0
**Last Updated:** 2026-03-14
**Applies To:** ALL AI-generated content, procedural generation, synthetic media, generative art

---

## Purpose

AI agents generating assets at high velocity need automated safety rails. This document ensures all generative content — images, audio, text, 3D models, procedural levels — meets disclosure, attribution, and ethical standards without manual review overhead.

**Core Principle:** Speed and safety aren't tradeoffs. Automated safety checks run in milliseconds; fixing a content violation after shipping costs days.

---

## AI Content Disclosure

### Mandatory Labeling

All AI-generated or AI-assisted content MUST be labeled:

| Content Type | Label Requirement | Implementation |
|-------------|-------------------|----------------|
| Images | "AI-Generated" metadata tag | EXIF/IPTC metadata field |
| Audio | "AI-Generated" metadata tag | ID3/Vorbis comment field |
| Text | Disclosure in UI | Visible indicator near content |
| Video | "AI-Generated" metadata tag | Container metadata |
| 3D Models | "AI-Generated" in file header | glTF extras field |
| Code | Comment header | `// AI-generated` header |

### C2PA Metadata (Content Provenance)

For published/distributed content, include C2PA (Coalition for Content Provenance and Authenticity) metadata:

```typescript
interface C2PAManifest {
  /** Tool used to generate the content */
  generator: string;
  /** Model/version identifier */
  model: string;
  /** Timestamp of generation */
  generatedAt: string;
  /** Human review status */
  reviewed: boolean;
  /** Hash of the prompt (not the prompt itself — privacy) */
  promptHash: string;
}
```

```rust
/// C2PA content provenance metadata
struct ContentProvenance {
    generator: String,
    model: String,
    generated_at: chrono::DateTime<chrono::Utc>,
    reviewed: bool,
    prompt_hash: String,
}
```

```go
// ContentProvenance tracks AI generation metadata
type ContentProvenance struct {
    Generator   string    `json:"generator"`
    Model       string    `json:"model"`
    GeneratedAt time.Time `json:"generated_at"`
    Reviewed    bool      `json:"reviewed"`
    PromptHash  string    `json:"prompt_hash"`
}
```

---

## Procedural Generation Guardrails

### Seed Reproducibility

All procedural generation MUST use deterministic seeds:

```typescript
// MANDATORY: All procedural generation must be seed-reproducible
class ProceduralGenerator {
  private rng: SeededRandom;

  constructor(seed: number) {
    this.rng = new SeededRandom(seed);
    // Log seed for reproduction
    console.info(`[PROCGEN] Initialized with seed: ${seed}`);
  }

  generate(): GeneratedAsset {
    // All randomness flows through this.rng
    return {
      seed: this.rng.seed,
      // ...generated content
    };
  }
}
```

### Output Bounding

Generated content must stay within defined bounds:

| Dimension | Minimum | Maximum | Enforcement |
|-----------|---------|---------|-------------|
| Image resolution | 64x64 | 8192x8192 | Clamp before save |
| Audio duration | 0.1s | 600s (10min) | Truncate at limit |
| Text length | 1 char | 100K chars | Truncate with warning |
| 3D vertex count | 3 | 10M | LOD generation required |
| Level size | 1 room | 10K tiles | Boundary enforcement |
| Color values | #000000 | #FFFFFF | Clamp to valid range |

### Safety Filters

Generated content must pass safety filters before use:

```typescript
interface SafetyFilter {
  /** Content category being filtered */
  category: 'violence' | 'adult' | 'hate' | 'self-harm' | 'illegal';
  /** Threshold (0.0-1.0) — content above this is blocked */
  threshold: number;
  /** Action when threshold exceeded */
  action: 'block' | 'flag' | 'replace';
}

const DEFAULT_FILTERS: SafetyFilter[] = [
  { category: 'violence', threshold: 0.7, action: 'flag' },
  { category: 'adult', threshold: 0.3, action: 'block' },
  { category: 'hate', threshold: 0.1, action: 'block' },
  { category: 'self-harm', threshold: 0.1, action: 'block' },
  { category: 'illegal', threshold: 0.1, action: 'block' },
];
```

---

## Asset Attribution

### License Compliance

All AI-generated assets must track their training data lineage:

| License Type | Can Use For | Requires | Agent Action |
|-------------|-------------|----------|--------------|
| Public Domain / CC0 | Anything | Nothing | Auto-approve |
| CC-BY | Anything | Attribution | Add credits |
| CC-BY-SA | Anything | Attribution + ShareAlike | Add credits + propagate license |
| CC-BY-NC | Non-commercial only | Attribution | HALT if commercial project |
| Proprietary | Licensed use only | License agreement | HALT — requires legal review |
| Unknown | Nothing | Determination | HALT — cannot use |

### Model Cards

When using AI models for generation, document the model:

```typescript
interface ModelCard {
  name: string;
  version: string;
  provider: string;
  license: string;
  trainingDataDescription: string;
  knownLimitations: string[];
  intendedUse: string;
  prohibitedUse: string[];
}
```

### Ownership

| Scenario | Owner | Notes |
|----------|-------|-------|
| AI generates from prompt | Prompter/Organization | Subject to model ToS |
| AI modifies existing work | Original creator | Derivative work rules apply |
| AI generates from training data | Varies by jurisdiction | Legal gray area — HALT for legal review |
| Procedural generation (code-based) | Code author | No AI training data involved |

---

## Synthetic Media Ethics

### Deepfake Prevention

| Rule | Requirement | Enforcement |
|------|-------------|-------------|
| No real person likeness | Cannot generate identifiable real people | Face detection + block |
| No voice cloning | Cannot clone real voices without consent | Voice print matching + block |
| No fake evidence | Cannot generate fake documents/screenshots | Document template detection |
| Satire exception | Clearly labeled satire/parody is allowed | Requires explicit label |

### Age-Appropriate Content

All generated content must be age-rated:

| Rating | Allowed Content | Filters Active |
|--------|----------------|----------------|
| Everyone (E) | No violence, no adult themes | All filters at maximum |
| Teen (T) | Mild cartoon violence | Violence threshold: 0.5 |
| Mature (M) | Moderate themes | Violence threshold: 0.7 |
| Adult (A) | Age-verified only | Reduced filters, age gate required |

---

## Content Filtering Pipeline

```
Input Prompt → Safety Pre-Filter → Generation → Safety Post-Filter → Metadata Injection → Output
                    ↓                                    ↓
              Block if unsafe                     Block if unsafe
              Log attempt                         Log + replace
```

### Pre-Generation Filters
- Prompt injection detection
- Prohibited content category matching
- PII (personally identifiable information) removal from prompts

### Post-Generation Filters
- Content safety classification
- Face detection (block real person likeness)
- Text extraction and toxicity check
- Metadata validation (C2PA, attribution)

---

## HALT CONDITIONS

**STOP and ask the human when:**

- [ ] Generated content resembles a real person's likeness
- [ ] License compliance cannot be determined for training data
- [ ] Content safety filters flag output at any threshold
- [ ] Ownership or attribution is unclear
- [ ] Content is intended for age-restricted audiences
- [ ] Deepfake or synthetic media detection triggers
- [ ] C2PA metadata cannot be properly attached
- [ ] Generated content will be used in legal, medical, or financial contexts

---

## Language Patterns

### TypeScript
```typescript
// Generative asset safety wrapper
async function safeGenerate<T>(
  generator: () => Promise<T>,
  filters: SafetyFilter[],
  provenance: C2PAManifest,
): Promise<{ asset: T; provenance: C2PAManifest } | { blocked: true; reason: string }> {
  const result = await generator();
  for (const filter of filters) {
    const score = await classify(result, filter.category);
    if (score > filter.threshold) {
      if (filter.action === 'block') {
        return { blocked: true, reason: `${filter.category} score ${score} exceeds ${filter.threshold}` };
      }
    }
  }
  return { asset: result, provenance };
}
```

### Rust
```rust
/// Safe generation with content filtering
async fn safe_generate<T>(
    generator: impl Future<Output = T>,
    filters: &[SafetyFilter],
    provenance: ContentProvenance,
) -> Result<(T, ContentProvenance), SafetyViolation> {
    let result = generator.await;
    for filter in filters {
        let score = classify(&result, &filter.category).await;
        if score > filter.threshold {
            if filter.action == Action::Block {
                return Err(SafetyViolation {
                    category: filter.category.clone(),
                    score,
                    threshold: filter.threshold,
                });
            }
        }
    }
    Ok((result, provenance))
}
```

### Go
```go
// SafeGenerate wraps generation with content safety filtering
func SafeGenerate[T any](
    ctx context.Context,
    generator func(ctx context.Context) (T, error),
    filters []SafetyFilter,
    provenance ContentProvenance,
) (*GenerationResult[T], error) {
    result, err := generator(ctx)
    if err != nil {
        return nil, fmt.Errorf("generation failed: %w", err)
    }
    for _, filter := range filters {
        score, err := Classify(ctx, result, filter.Category)
        if err != nil {
            return nil, fmt.Errorf("classification failed: %w", err)
        }
        if score > filter.Threshold && filter.Action == ActionBlock {
            return nil, &SafetyViolation{
                Category:  filter.Category,
                Score:     score,
                Threshold: filter.Threshold,
            }
        }
    }
    return &GenerationResult[T]{Asset: result, Provenance: provenance}, nil
}
```

---

## RELATED DOCUMENTS

| Document | Purpose |
|----------|---------|
| [ETHICAL_ENGAGEMENT.md](../ethical/ETHICAL_ENGAGEMENT.md) | Dark pattern prevention |
| [AI_ASSISTED_DEV.md](../ai-dev/AI_ASSISTED_DEV.md) | AI development approval gates |
| [ANALYTICS_ETHICS.md](../analytics/ANALYTICS_ETHICS.md) | Data collection ethics |
| [MONETIZATION_GUARDRAILS.md](../monetization/MONETIZATION_GUARDRAILS.md) | Economy and IAP ethics |
| [ACCESSIBILITY_GUIDE.md](../accessibility/ACCESSIBILITY_GUIDE.md) | Accessible content generation |
