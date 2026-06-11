# Python Game Tools - 2026 Best Practices

**Stack:** Arcade 2.8, PyQt6, Data Visualization (Matplotlib, Plotly), Pydantic v2
**Target:** Arcade 2D UI overlays, loot probability visualization, data dashboards
**Last Updated:** 2026-03-14

---

## 2026 Best Practices

### Arcade 2D UI (Python)

| Pattern | Description | Use Case |
|---------|-------------|----------|
| **Sprite-based** | Sprite class for UI elements | Overlays, buttons |
| **Scene Manager** | Scene switching for UI flow | Menu → Game → Results |
| **Event-driven** | Pygame event handling | Input processing |
| **Delta Rendering** | Only redraw changed regions | Performance |

### Data Visualization

```
Matplotlib → Static charts, probability distributions
Plotly → Interactive dashboards, real-time updates
Seaborn → Statistical visualizations, heatmaps
```

### Transparency Patterns

| Feature | Description | Benefit |
|---------|-------------|---------|
| **Loot Display** | Transparent probability tables | Player trust |
| **Drop Animation** | Visual loot acquisition | Engagement |
| **Rarity Indicator** | Color + text (non-color dependent) | A11y |
| **Statistical View** | Distribution histograms | Data insight |

### Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Frame Time** | <16ms (60 FPS) | `time.perf_counter()` |
| **Memory** | <150MB | `memory_profiler` |
| **Load Time** | <2s | Startup benchmark |
| **GC Pause** | <5ms | Profiler |

---

## Game Design Integration Patterns

### Arcade 2D UI Overlay

```
Layer System → UI layer above game layer
Alpha Blending → Transparent backgrounds
Event Propagation → UI events before game events
```

### Loot Probability Visualization

| Pattern | Description | Implementation |
|---------|-------------|----------------|
| **Probability Table** | Exact drop rates display | Pandas DataFrame |
| **Pie Chart** | Visual rarity distribution | Matplotlib |
| **Histogram** | Drop distribution over time | Plotly |
| **Expectation Curve** | Cumulative drop chance | Statistical plot |

---

## Accessibility Compliance (WCAG 2.2 Level AA)

| Requirement | Implementation | Verification |
|-------------|----------------|--------------|
| **Color Independence** | Text + icon + color for rarity | Grayscale test |
| **Keyboard Navigation** | Arrow keys, Enter, Escape | Manual audit |
| **Screen Reader** | Text descriptions for charts | axe-core |
| **High Contrast** | 4.5:1 minimum ratio | Contrast checker |
| **Reduced Motion** | Toggle for animations | Device testing |

---

## Ethical Engagement Standards

### No Dark Patterns

| Forbidden Pattern | Replacement |
|------------------|-------------|
| **Hidden Rates** | Transparent probability display |
| **False Scarcity** | Honest drop rate claims |
| **Manipulative Odds** | No misleading rarity labels |
| **Sunk Cost Display** | No "you're due" messaging |
| **Timer Pressure** | No artificial purchase timers |

### Transparency Requirements

```
- Display exact drop rates (e.g., "1.5%")
- Show total opens for statistical context
- Clear rarity definitions (Common/Rare/Legendary)
- No hidden weighting algorithms
- Honest RNG disclosure (no fake random)
```

---

## Code Examples

### arcade-ui-overlay.py
Arcade 2D UI overlay with layer management
→ See: [`arcade-ui-overlay.py`](./arcade-ui-overlay.py)

### loot-table-visualizer.py
Transparent loot probability display
→ See: [`loot-table-visualizer.py`](./loot-table-visualizer.py)

---

## Testing Guidelines

| Test Type | Tool | Target |
|-----------|------|--------|
| **Unit** | pytest | Per-commit |
| **Visual** | pytest-matchers | Per-PR |
| **A11y** | Manual audit | Per-component |
| **Performance** | pytest-benchmark | Per-sprint |
| **Integration** | pytest-asyncio | Per-release |

---

## Related Documents

- [AGENT_GUARDRAILS.md](../../docs/AGENT_GUARDRAILS.md) - Mandatory safety protocols
- [TEST_PRODUCTION_SEPARATION.md](../../docs/standards/TEST_PRODUCTION_SEPARATION.md) - Test/prod isolation
- [OPERATIONAL_PATTERNS.md](../../docs/standards/OPERATIONAL_PATTERNS.md) - Health checks, circuit breakers

---

**Authored by:** Claude Code (Anthropic)
**Document Owner:** Project Maintainers
**Review Cycle:** Per-sprint updates
**Last Review:** 2026-03-14