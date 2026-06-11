# Scala Functional UI Examples

> Scala 3.4+ functional UI composition with type-safe CSS, game analytics patterns, and ethical analysis.

**Stack:** Scala 3.4+, ScalaFX, TornadoFX, ggplot2-scala, Apache Spark

---

## Purpose

This directory demonstrates 2026 best practices for building functional, type-safe UIs in Scala with:
- **Functional UI Composition** - Pure functional widget builders
- **Type-Safe CSS** - Compile-time style validation
- **Game Analytics Patterns** - A/B testing, retention curves, DDA telemetry
- **Ethical Analysis** - Dark pattern detection, monetization transparency
- **Colorblind-Accessible Palettes** - Deuteranopia/protanopia/tritanopia-safe
- **Hick's Law Application** - 5 ± 2 menu items for cognitive load optimization

---

## Examples

| File | Purpose | Key Patterns |
|------|---------|--------------|
| `UI.scala` | Functional UI composition | Type-safe CSS, functional builders |
| `data-pipeline-viz.scala` | Apache Spark UI integration | Real-time analytics, colorblind-safe |
| `procedural-gen.scala` | Procedural generation tools | DDA telemetry, ethical analysis |

---

## 2026 Best Practices

### 1. Functional UI Composition

```scala
// Pure functional widget builders
def button(text: String, style: ButtonStyle = DefaultButton): Button =
  Button(text).withStyle(style)
```

### 2. Type-Safe CSS

```scala
// Compile-time style validation
trait ButtonStyle {
  def backgroundColor: Color
  def hoverEffect: Effect
}
```

### 3. Colorblind-Safe Palettes

| Palette | Colors | Use Case |
|---------|--------|----------|
| **Viridis** | #440154, #443782, #3a608b, #318755, #27b57e | Sequential data |
| **Colorblind-Universal** | #0077BD, #E66302, #7C87BB, #D6588B | Categorical data |
| **Cividis** | #002495, #005CA8, #297EA7, #58999D, #8AB68E | Perceptually uniform |

### 4. Hick's Law Application

- **Menu Items:** 5 ± 2 (3-7 items)
- **Cognitive Load:** O(1) decision time
- **Navigation Depth:** Max 3 levels

### 5. Game Analytics Patterns

```scala
// A/B Test Tracking
case class ABTest(id: String, variant: String, timestamp: Long)
case class RetentionCurve(day: Int, retained: Double)
case class DDATelemetry(challenge: String, difficulty: Double, playerSkill: Double)
```

### 6. Ethical Analysis

- **Dark Pattern Detection:** Identify manipulative UI patterns
- **Monetization Transparency:** Clear pricing, no hidden costs
- **Engagement Ethics:** Avoid addiction loops, respect user autonomy

---

## Running Examples

```bash
cd examples/scala/functional-ui
scala-cli run UI.scala
scala-cli run data-pipeline-viz.scala
scala-cli run procedural-gen.scala
```

---

## Guardrails Compliance

| Rule | Implementation |
|------|----------------|
| **PRODUCTION FIRST** | Production UI code exists before test scaffolding |
| **SEPARATE DATABASES** | Analytics DB uses test_ prefix for test environment |
| **CLEAR LABELING** | Test files follow `*_test.scala` convention |
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