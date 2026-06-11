# Bevy UI Example - ECS-Based Game Interfaces

> Production-ready Bevy UI 0.15+ patterns for ECS-based game user interfaces.

**Last Updated:** 2026-03-14
**Bevy Version:** 0.15+
**Rust Version:** 1.85+ (2021 edition)

---

## Purpose

This example demonstrates building production-ready game UIs using **Bevy UI 0.15+** with ECS (Entity-Component-System) architecture. Patterns include:

- **ECS UI State Management** - Component-based UI state, system-driven updates
- **Zero-Copy UI State Transfer** - Direct memory transfer, no serialization overhead
- **Accessibility Patterns** - ARIA-like ECS components, focus management systems
- **Systemic Consistency** - ECS validation rules, state machine consistency
- **Game Design Integration** - Player HUD, inventory UI, dialogue systems

---

## 2026 Best Practices

### Bevy UI 0.15+ Architecture Patterns

| Pattern | Use Case | Implementation |
|---------|----------|----------------|
| **UI Camera** | Dedicated UI rendering | `Camera2dBundle::ui()` |
| **UI State Component** | State storage | `#[derive(Component)]` |
| **State System** | State transitions | `fn ui_state_system()` |
| **Event-driven UI** | Reactive updates | `Events::<UiEvent>::send()` |
| **Zero-Copy Transfer** | High-performance | `bytemuck` derive, direct memory |

### ECS UI State Management

```rust
// ECS UI state component pattern
#[derive(Component, Clone, Debug)]
struct UiState {
    focus: Option<Entity>,
    active_panel: PanelType,
    theme: UiTheme,
    accessibility: AccessibilityConfig,
}

// System-driven state updates
fn update_ui_state(
    mut ui_state: ResMut<UiState>,
    input: Res<InputMap>,
    mut commands: Commands,
) {
    if input.pressed("Tab") {
        ui_state.focus = ui_state.next_focus();
    }
}
```

### Zero-Copy Transfer Patterns

| Pattern | Use Case | Implementation |
|---------|----------|----------------|
| **Bytemuck Derive** | Safe zero-copy | `#[derive(Bytemuck)]` |
| **Pod Trait** | Plain old data | `unsafe impl Pod` |
| **Zeroable Trait** | Zero initialization | `unsafe impl Zeroable` |
| **Direct Slice** | Batch transfers | `&[T]` without allocation |
| **GPU Upload** | Texture/buffer | `encoder.write_buffer()` |

### Accessibility (A11Y) Requirements

| Requirement | ECS Pattern | WCAG Level |
|-------------|-------------|------------|
| **Focus Tracking** | `FocusComponent` on entities | AA |
| **ARIA Labels** | `LabelComponent` with text | A |
| **Keyboard Nav** | InputMap system bindings | A |
| **High Contrast** | `ThemeComponent` variants | AA |
| **Screen Reader** | Text-to-speech events | AA |

### Systemic Consistency Rulesets

```
ECS UI CONSISTENCY RULES:

1. **STATE MACHINE VALIDATION**
   - UI state transitions validated by systems
   - Invalid transitions logged + rejected
   - State machine type-checked at compile time

2. **COMPONENT BOUNDARIES**
   - UI components separate from game components
   - No direct mutation across boundaries
   - Systems enforce separation

3. **EVENTUAL CONSistency**
   - UI events queued via Events<T>
   - Systems process in deterministic order
   - Conflicts resolved by system priority

4. **ZERO-COPY GUARANTEES**
   - All UI state derives Bytemuck
   - No allocation during transfer
   - GPU upload via direct memory
```

---

## Running the Example

```bash
cd examples/rust/bevy-ui-example
cargo init
cargo add bevy@0.15
cargo add bytemuck
cargo run
```

---

## File Structure

| File | Purpose | Key Patterns |
|------|---------|--------------|
| `main.rs` | Bevy app setup, UI systems | UI camera, state systems |
| `ecs-ui-state.rs` | ECS UI state management | Components, systems, events |
| `zero-copy-transfer.rs` | Zero-copy UI transfer | Bytemuck, Pod, Zeroable |

---

## Architecture

```
+------------------+     +------------------+     +------------------+
|   Bevy App       |     |   ECS Systems    |     |   GPU Renderer   |
|   (UI Camera)    |     |   (State Logic)  |     |   (wgpu)         |
+------------------+     +------------------+     +------------------+
        |                        |                        |
        | UI Components          |                        |
        |-----------------------|                        |
        |                        |                        |
        | State Systems         | Validate state         |
        |-----------------------|------------------------|
        |                        |                        |
        | Zero-Copy Transfer    | Direct memory          |
        |-----------------------|------------------------|
        |                        |                        |
        |                       | GPU Upload             |
        |                       |------------------------|
        |                       |                        |
        |                       | Render                 |
        |                       |------------------------|
```

---

## Guardrails Compliance

| Rule | Implementation |
|------|----------------|
| **PRODUCTION FIRST** | ECS systems before UI tests |
| **COMPONENT BOUNDARIES** | UI/game separation enforced |
| **ACCESSIBILITY** | Focus, labels, keyboard nav |
| **ZERO-COPY** | No allocation during transfer |
| **STATE VALIDATION** | All transitions type-checked |

---

## Related Documentation

- [AGENT_GUARDRAILS.md](../../docs/AGENT_GUARDRAILS.md) - Core safety protocols
- [TEST_PRODUCTION_SEPARATION.md](../../docs/standards/TEST_PRODUCTION_SEPARATION.md) - Separation standards
- [WEB_UI_IMPLEMENTATION.md](../../docs/sprints/SPRINT_002_WEB_UI_IMPLEMENTATION.md) - Web UI patterns

---

**Authored by:** Claude Code (Anthropic)
**Document Owner:** Project Maintainers
**Review Cycle:** Monthly