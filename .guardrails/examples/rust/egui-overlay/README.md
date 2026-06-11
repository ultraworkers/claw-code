# egui Overlay Example - Debug/Developer UI

> Production-ready egui debug and developer UI overlays for game engines.

**Last Updated:** 2026-03-14
**egui Version:** 0.27+
**Rust Version:** 1.85+ (2021 edition)
**wgpu Version:** 0.20+

---

## Purpose

This example demonstrates building production-ready debug/developer UI overlays using **egui** with parallel rendering and GPU acceleration. Patterns include:

- **Debug Overlay** - FPS counter, entity inspector, console logging
- **Parallel Rendering** - Rayon-based UI threading, lock-free rendering
- **GPU-Accelerated Rendering** - wgpu integration, texture upload
- **Accessibility Patterns** - High contrast themes, keyboard navigation
- **Systemic Consistency** - Debug state validation, overlay boundaries

---

## 2026 Best Practices

### egui Architecture Patterns

| Pattern | Use Case | Implementation |
|---------|----------|----------------|
| **Context** | UI state | `egui::Context` |
| **Frame** | Per-frame UI | `ctx.frame()` |
| **Panel** | Layout zones | `TopBottomPanel`, `SidePanel` |
| **Window** | Floating panels | `Window::new()` |
| **Debug Overlay** | Developer UI | Dedicated debug context |

### Parallel Rendering Patterns

| Pattern | Use Case | Implementation |
|---------|----------|----------------|
| **Rayon ThreadPool** | UI threading | `rayon::spawn()` |
| **Lock-free Render** | No mutex overhead | `Arc<egui::Context>` |
| **Batched Updates** | Frame coalescing | `Vec<UiEvent>` |
| **Async Upload** | GPU transfer | `tokio::spawn()` |

### GPU-Accelerated Rendering

```rust
// wgpu integration for egui
let render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
    label: "egui render pass",
    color_attachments: &[&color_attachment],
    depth_stencil_attachment: None,
    ..Default::default()
});

// GPU texture upload
queue.write_texture(
    &texture,
    TextureAspect::All,
    pixels,
    ImageInfo {
        offset: (0, 0, 0),
        size: texture_size,
        ..Default::default()
    },
);
```

### Accessibility (A11Y) Requirements

| Requirement | egui Pattern | WCAG Level |
|-------------|--------------|------------|
| **High Contrast** | `egui::Style` theme | AA |
| **Keyboard Nav** | `kb_navigation` plugin | A |
| **Focus Indicators** | `hovered` + `focused` | AA |
| **Screen Reader** | Text output callback | AA |

### Systemic Consistency Rulesets

```
DEBUG Overlay CONSISTENCY RULES:

1. **OVERLAY BOUNDARIES**
   - Debug UI separate from game UI
   - No debug code in production builds
   - Conditional compilation: #[cfg(debug)]

2. **STATE VALIDATION**
   - Debug state machine type-checked
   - Invalid states logged + rejected
   - Overlay lifecycle managed

3. **THREAD SAFETY**
   - UI thread isolated from game thread
   - Lock-free rendering via Arc
   - Rayon parallelism bounded

4. **GPU INTEGRATION**
   - wgpu context validated
   - Texture upload size-checked
   - Render pass error-handled
```

---

## Running the Example

```bash
cd examples/rust/egui-overlay
cargo init
cargo add egui@0.27
cargo add eframe@0.27
cargo add rayon
cargo add wgpu@0.20
cargo run
```

---

## File Structure

| File | Purpose | Key Patterns |
|------|---------|--------------|
| `debug-overlay.rs` | Debug overlay UI | FPS, inspector, console |
| `parallel-rendering.rs` | Rayon UI threading | ThreadPool, lock-free |
| `wgpu-integration.rs` | GPU-accelerated rendering | wgpu, texture upload |

---

## Architecture

```
+------------------+     +------------------+     +------------------+
|   egui Context   |     |   Rayon Threads  |     |   wgpu Renderer  |
|   (UI State)     |     |   (Parallel)     |     |   (GPU)          |
+------------------+     +------------------+     +------------------+
        |                        |                        |
        | Frame updates          |                        |
        |-----------------------|                        |
        |                        |                        |
        | Parallel rendering    | ThreadPool spawn       |
        |-----------------------|------------------------|
        |                        |                        |
        | GPU texture upload    | wgpu queue             |
        |-----------------------|------------------------|
        |                        |                        |
        |                       | Render pass            |
        |                       |------------------------|
        |                       |                        |
        |                       | Display                |
        |                       |------------------------|
```

---

## Guardrails Compliance

| Rule | Implementation |
|------|----------------|
| **PRODUCTION FIRST** | Debug overlay conditional #[cfg(debug)] |
| **OVERLAY BOUNDARIES** | Debug/game separation enforced |
| **ACCESSIBILITY** | High contrast, keyboard nav |
| **THREAD SAFETY** | Lock-free rendering via Arc |
| **GPU VALIDATION** | wgpu context type-checked |

---

## Related Documentation

- [AGENT_GUARDRAILS.md](../../docs/AGENT_GUARDRAILS.md) - Core safety protocols
- [TEST_PRODUCTION_SEPARATION.md](../../docs/standards/TEST_PRODUCTION_SEPARATION.md) - Separation standards
- [WEB_UI_IMPLEMENTATION.md](../../docs/sprints/SPRINT_002_WEB_UI_IMPLEMENTATION.md) - Web UI patterns

---

**Authored by:** Claude Code (Anthropic)
**Document Owner:** Project Maintainers
**Review Cycle:** Monthly