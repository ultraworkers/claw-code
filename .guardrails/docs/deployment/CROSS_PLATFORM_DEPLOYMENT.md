# Cross-Platform Deployment

**Version:** 1.0.0
**Last Updated:** 2026-03-14
**Applies To:** ALL cross-platform releases, app store submissions, CI/CD pipelines, feature rollouts

---

## Purpose

Deploying across platforms is where AI velocity meets store compliance. AI agents can generate build configs and deployment scripts rapidly — but each platform has specific rules that cause rejection if violated. These guardrails encode platform requirements so agents get submissions right on the first attempt.

**Core Principle:** Know the rules before you build. A rejected submission costs more than reading the guidelines.

---

## App Store Compliance Matrix

### Apple App Store (iOS/macOS/visionOS)

| Requirement | Rule | Consequence of Violation |
|-------------|------|--------------------------|
| IAP for digital goods | All digital purchases via StoreKit 2 | Rejection |
| Privacy nutrition labels | Accurate data collection disclosure | Rejection |
| App Tracking Transparency | ATT prompt before tracking | Rejection |
| Minimum age rating | Accurate content rating | Rejection + removal |
| Review guidelines 4.0+ | Design guidelines compliance | Rejection |
| visionOS spatial | Shared Space default, Full Space opt-in | Rejection |
| Accessibility | VoiceOver support required | Rejection risk |

### Google Play Store (Android)

| Requirement | Rule | Consequence of Violation |
|-------------|------|--------------------------|
| Play Billing Library | Digital goods via Play billing | Rejection |
| Data Safety Section | Accurate data handling disclosure | Rejection |
| Target API level | Must target latest -1 API level | Rejection after deadline |
| Content rating | IARC rating required | Removal |
| Families policy | Strict rules for kids-directed apps | Rejection + account action |
| Adaptive icons | Required for modern Android | Degraded experience |

### Steam (PC/Mac/Linux/Steam Deck)

| Requirement | Rule | Consequence of Violation |
|-------------|------|--------------------------|
| Steamworks integration | Steamworks SDK for achievements, cloud saves | Required for features |
| Steam Deck verification | Controller support, resolution flexibility | Verification badge |
| Content descriptors | Accurate content warnings | Community reports |
| Workshop guidelines | UGC moderation required | Feature removal |
| Anti-cheat disclosure | Must disclose anti-cheat software | Store page requirement |

### Meta Quest (VR)

| Requirement | Rule | Consequence of Violation |
|-------------|------|--------------------------|
| Meta Platform SDK | Required for Quest features | Rejection |
| Comfort rating | Must declare comfort level | Rejection |
| Guardian boundary | Must respect Guardian system | Rejection |
| Performance requirements | 72fps minimum (90fps recommended) | Rejection |
| Hand tracking | Must support if applicable | Reduced visibility |
| Privacy policy | Meta-specific privacy requirements | Rejection |

### Epic Games Store

| Requirement | Rule | Consequence of Violation |
|-------------|------|--------------------------|
| Epic Online Services | EOS for multiplayer, achievements | Required for features |
| Rating system | IARC or equivalent | Required before publish |
| Achievement guidelines | Epic achievement standards | Review feedback |

---

## CI/CD for Games & Apps

### Pipeline Architecture

```
Code Push → Lint/Format → Build → Test → Security Scan → Stage → Deploy
               ↓           ↓       ↓          ↓            ↓       ↓
            Auto-fix    Per-platform  Unit +   SAST +     QA env   Progressive
            or block    builds       Integration  Dependency  review   rollout
```

### Build Matrix

```typescript
interface BuildConfig {
  platform: 'ios' | 'android' | 'windows' | 'macos' | 'linux' | 'web' | 'quest' | 'steamdeck';
  buildType: 'debug' | 'staging' | 'release';
  signing: SigningConfig;
  minTargetVersion: string;
  optimizations: OptimizationConfig;
}

// Platform-specific build requirements
const BUILD_REQUIREMENTS: Record<string, BuildRequirement> = {
  ios: { minTarget: '17.0', signing: 'apple-distribution', format: 'ipa' },
  android: { minTarget: '34', signing: 'play-upload', format: 'aab' },
  windows: { minTarget: '10', signing: 'code-signing', format: 'msix' },
  macos: { minTarget: '14.0', signing: 'developer-id', format: 'dmg' },
  linux: { minTarget: 'ubuntu-22.04', signing: 'gpg', format: 'flatpak' },
  web: { minTarget: 'es2022', signing: 'ssl', format: 'static' },
  quest: { minTarget: 'quest3', signing: 'meta-upload', format: 'apk' },
  steamdeck: { minTarget: 'proton-9', signing: 'steam-upload', format: 'depot' },
};
```

### Test Requirements Per Platform

| Platform | Unit Tests | Integration Tests | Platform Tests | Performance Tests |
|----------|-----------|-------------------|----------------|-------------------|
| iOS | Required | Required | Simulator + device | Instruments profiling |
| Android | Required | Required | Emulator + device | Perfetto profiling |
| Windows | Required | Required | VM testing | PIX/RenderDoc |
| Web | Required | Required | Cross-browser | Lighthouse CI |
| Quest VR | Required | Required | Device mandatory | Frame timing |
| Steam Deck | Required | Required | Device recommended | MangoHud |

---

## Feature Flags

### When to Use Feature Flags

| Scenario | Flag Type | Lifetime |
|----------|-----------|----------|
| New feature rollout | Release flag | Until 100% rollout |
| A/B experiment | Experiment flag | Until experiment ends |
| Kill switch | Ops flag | Permanent |
| Platform-specific | Platform flag | Permanent |
| Beta features | Beta flag | Until GA |

### Feature Flag Rules

| Rule | Requirement |
|------|-------------|
| Default off | New features default to disabled |
| Cleanup mandate | Remove flag code within 30 days of 100% rollout |
| No flag nesting | Maximum 2 levels of flag dependencies |
| Audit trail | Log all flag changes with who/when/why |
| Rollback ready | Every flag must support instant disable |

```typescript
interface FeatureFlag {
  name: string;
  enabled: boolean;
  rolloutPercentage: number;  // 0-100
  platforms: Platform[];       // Which platforms this applies to
  expiresAt?: Date;           // Auto-cleanup reminder
  owner: string;              // Team responsible for cleanup
}
```

---

## Progressive Rollout

### Rollout Strategy

| Phase | Audience | Duration | Gate |
|-------|----------|----------|------|
| Internal | Dev team only | 1-3 days | No crashes, no regressions |
| Alpha | 1% of users | 3-7 days | Error rate < 0.1% |
| Beta | 10% of users | 7-14 days | No P0/P1 issues |
| GA | 25% → 50% → 100% | 7 days per step | Metrics stable |

### Rollback Triggers

| Metric | Threshold | Action |
|--------|-----------|--------|
| Crash rate | > 2x baseline | Automatic rollback |
| Error rate | > 5x baseline | Automatic rollback |
| Latency P99 | > 3x baseline | Alert + manual review |
| User reports | > 10x baseline | Alert + manual review |

---

## HALT CONDITIONS

**STOP and ask the human when:**

- [ ] App store signing certificates or keys are needed
- [ ] Production deployment pipeline is being modified
- [ ] Feature flags affect payment or auth flows
- [ ] Platform-specific compliance requirements are unclear
- [ ] Rollout strategy involves > 10% of users
- [ ] CI/CD secrets or credentials are involved
- [ ] Store submission metadata is being prepared
- [ ] Cross-platform build matrix changes affect all platforms

---

## Language Patterns

### TypeScript
```typescript
// Platform-aware deployment guard
function validateDeployment(
  config: BuildConfig,
  requirements: BuildRequirement,
): ValidationResult {
  const issues: string[] = [];
  if (!config.signing) {
    issues.push('HALT: Signing configuration required for release builds');
  }
  if (config.buildType === 'release' && !config.optimizations.minification) {
    issues.push('WARNING: Release build without minification');
  }
  return { valid: issues.length === 0, issues };
}
```

### Rust
```rust
/// Platform deployment validation
fn validate_deployment(
    config: &BuildConfig,
    requirements: &BuildRequirement,
) -> Result<(), Vec<String>> {
    let mut issues = Vec::new();
    if config.signing.is_none() {
        issues.push("HALT: Signing configuration required".into());
    }
    if config.build_type == BuildType::Release && !config.optimizations.minification {
        issues.push("WARNING: Release build without minification".into());
    }
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}
```

### Go
```go
// ValidateDeployment checks platform-specific deployment requirements
func ValidateDeployment(config *BuildConfig, req *BuildRequirement) ([]string, error) {
    var issues []string
    if config.Signing == nil {
        issues = append(issues, "HALT: Signing configuration required")
    }
    if config.BuildType == BuildTypeRelease && !config.Optimizations.Minification {
        issues = append(issues, "WARNING: Release build without minification")
    }
    if len(issues) > 0 {
        return issues, fmt.Errorf("deployment validation failed: %d issues", len(issues))
    }
    return nil, nil
}
```

---

## RELATED DOCUMENTS

| Document | Purpose |
|----------|---------|
| [2026_GAME_DESIGN.md](../game-design/2026_GAME_DESIGN.md) | Game design platform constraints |
| [MONETIZATION_GUARDRAILS.md](../monetization/MONETIZATION_GUARDRAILS.md) | Platform IAP requirements |
| [ANALYTICS_ETHICS.md](../analytics/ANALYTICS_ETHICS.md) | Platform privacy requirements |
| [AI_ASSISTED_DEV.md](../ai-dev/AI_ASSISTED_DEV.md) | AI development infrastructure gates |
| [ACCESSIBILITY_GUIDE.md](../accessibility/ACCESSIBILITY_GUIDE.md) | Platform accessibility requirements |
