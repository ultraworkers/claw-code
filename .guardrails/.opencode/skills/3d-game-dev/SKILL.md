---
name: 3d-game-dev
description: "3D Game Development guardrails for Godot, Unity, and custom engines. Enforces mathematical correctness, asset safety, shader constraints."
---

# 3D Game Development Agent

Enforce 3D game development guardrails on all geometry, shader, physics, and engine code.

## Geometry & Mesh Constraints

1. **Polygon Budget**: Mobile 10K/scene, PC 500K/scene, VR 100K/scene. Auto-LOD for >5K.
2. **Topology**: Triangles/quads only. No N-gons, non-manifold, or inverted normals.
3. **UV**: No overlapping islands (unless mirrored). Consistent texel density.

## Shader Constraints

1. **No Dynamic Branching**: NEVER if/else in fragment shaders. Use step(), smoothstep(), mix().
2. **Samplers**: Max 16 (Forward), 8 (Mobile/VR).
3. **Loops**: Compile-time bounds, max 64 iterations, unroll ≤8.

## Mathematical Constraints

1. **Quaternions ONLY** for continuous rotation. NEVER Euler angles.
2. **Normalize** before use: assert(abs(q.length()-1.0) < 0.0001)
3. **MVP Order**: P * V * M (column-major). Document convention in file header.
4. **Handedness**: Right-handed, Y-up. Document in header.

## Godot 4.x Constraints

1. **Signals** for decoupled communication. NEVER `get_node("../../Player")`.
2. **Static typing**: `var velocity: Vector3` not `var velocity`.
3. **Threading**: WorkerThreadPool for proc-gen. NEVER render from threads.
4. **Deferred**: `call_deferred()` for thread-safe node operations.

## AI-Debuggable Architecture

1. **ECS** over deep inheritance. JSON-serializable components.
2. **Semantic telemetry**: `{ "event": "Collision_Failure", "entity": "Player_1" }`
3. **Headless CI**: All scenes run --headless, 600 frames, FPS > 30, memory stable.

## Halt Conditions

STOP and ask user when:
- No target platform specified for shader
- No polygon budget for mesh generation
- Euler angles proposed for 3D rotation
- if/else proposed in fragment shader
- Unnormalized quaternion usage
- Rendering pipeline modified without headless test plan

## References

- `docs/game-design/3D_GAME_DEVELOPMENT.md`
- `docs/game-design/3D_MATHEMATICAL_FOUNDATIONS.md`
- `docs/game-design/3D_MODULE_ARCHITECTURE.md`
- `docs/game-design/AI_DEBUGGABLE_3D_ARCHITECTURE.md`
