# R Game Analytics Examples

> ggplot2 4.0+, Shiny 2.0+, interactive dashboards for game analytics with ethical analysis.

**Stack:** R 4.4+, ggplot2 4.0+, Shiny 2.0+, dplyr 1.0+, reticulate (Python ML integration)

---

## Purpose

This directory demonstrates 2026 best practices for building game analytics dashboards in R with:
- **ggplot2 4.0+ Visualization** - Colorblind-safe palettes, accessible charts
- **Shiny 2.0+ Dashboards** - Interactive, reactive analytics interfaces
- **Game Analytics Patterns** - A/B testing, retention curves, DDA telemetry
- **Ethical Analysis** - Dark pattern detection, engagement ethics auditing
- **Colorblind-Accessible Palettes** - Deuteranopia/protanopia/tritanopia-safe
- **Hick's Law Application** - 5 ± 2 menu items for cognitive load optimization

---

## Examples

| File | Purpose | Key Patterns |
|------|---------|--------------|
| `shiny-dashboard.R` | Shiny 2 analytics dashboard | Interactive filtering, real-time metrics |
| `retention-analysis.R` | Player retention curve analysis | Survival analysis, cohort tracking |
| `ethics-auditor.R` | Dark pattern detection | Engagement ethics, monetization transparency |

---

## 2026 Best Practices

### 1. ggplot2 4.0+ Colorblind-Safe Palettes

| Palette | Colors | Use Case |
|---------|--------|----------|
| **viridis** | #440154, #443782, #3a608b, #318755, #27b57e | Sequential data |
| **cividis** | #002495, #005CA8, #297EA7, #58999D, #8AB68E | Perceptually uniform |
| **colorblind** | #0077BD, #E66302, #7C87BB, #D6588B, #8AB68E | Categorical data |

### 2. Shiny 2.0+ Modules

```r
# Modular dashboard components
module_kpi <- function(id) {
  card::tagList(
    card::card_body(value = ui::output_text(id))
  )
}
```

### 3. Hick's Law Application

- **Menu Items:** 5 ± 2 (3-7 items)
- **Navigation Depth:** Max 3 levels
- **Cognitive Load:** O(1) decision time

### 4. Game Analytics Patterns

```r
# A/B Test Tracking
ab_test <- list(id = "checkout-flow", variant = "B", conversion = 0.24)

# Retention Curve
retention <- data.frame(day = 1:7, rate = c(0.95, 0.80, 0.65, 0.50, 0.40, 0.35, 0.30))

# DDA Telemetry
dda <- list(challenge = "boss-fight", difficulty = 0.7, player_skill = 0.5)
```

### 5. Ethical Analysis

- **Dark Pattern Detection:** Identify manipulative UI patterns
- **Monetization Transparency:** Clear pricing, no hidden costs
- **Engagement Ethics:** Avoid addiction loops, respect user autonomy

---

## Running Examples

```bash
cd examples/r/game-analytics
Rscript shiny-dashboard.R
Rscript retention-analysis.R
Rscript ethics-auditor.R
```

### Shiny Dashboard (Web Server)

```bash
Rscript -e "shiny::runApp('examples/r/game-analytics')"
```

---

## Guardrails Compliance

| Rule | Implementation |
|------|----------------|
| **PRODUCTION FIRST** | Production analytics code exists before test scaffolding |
| **SEPARATE DATABASES** | Analytics DB uses test_ prefix for test environment |
| **CLEAR LABELING** | Test files follow `*_test.R` convention |
| **COLORBLIND-SAFE** | All visualization palettes validated for CVD accessibility |
| **HICK'S LAW** | Menu structures limited to 5 ± 2 items |

---

## Related Documentation

- [AGENT_GUARDRAILS.md](../../../docs/AGENT_GUARDRAILS.md) - Core safety protocols
- [TEST_PRODUCTION_SEPARATION.md](../../../docs/standards/TEST_PRODUCTION_SEPARATION.md) - Test/production isolation
- [ADVERSARIAL_TESTING.md](../../../docs/standards/ADVERSARIAL_TESTING.md) - Dark pattern detection

---

**Last Updated:** 2026-03-14
**Authored by:** TheArchitectit