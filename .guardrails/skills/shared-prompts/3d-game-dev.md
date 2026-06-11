# 3D Game Development Guardrails Prompt

Use this prompt when generating or reviewing 3D game code/assets.

## Pre-Flight Checklist

Before generating any 3D code:
- [ ] Target platform specified (Mobile/PC/VR/Spatial)
- [ ] Polygon budget documented
- [ ] Coordinate system documented (right-handed Y-up)
- [ ] Matrix convention documented (column-major)
- [ ] Shader target documented (GLSL/HLSL/WGSL/Godot)

## Geometry Rules

1. **Budgets**: Mobile 10K/scene, PC 500K/scene, VR 100K/scene. Auto-LOD >5K.
2. **Topology**: Triangles/quads only. No N-gons, non-manifold, inverted normals.
3. **UV**: No overlapping islands. Consistent texel density.

## Shader Rules

1. **No if/else in fragments**: Use step(), smoothstep(), mix().
2. **Samplers**: Max 16 (Forward), 8 (Mobile/VR).
3. **Loops**: Compile-time bounds, max 64, unroll ≤8.

## Math Rules

1. **Quaternions ONLY**: Never Euler angles for animation.
2. **Normalize**: `assert(abs(q.length()-1.0) < 0.0001)`
3. **MVP**: `P * V * M` (column-major).
4. **Handedness**: Right-handed, Y-up.

## Godot 4.x

1. **Signals** over get_node().
2. **Static typing**: `var v: Vector3`.
3. **Threading**: WorkerThreadPool. No rendering from threads.
4. **Deferred**: `call_deferred()` for node ops.

## Halt Conditions

STOP and ask when:
- No platform specified
- No polygon budget
- Euler angles proposed
- if/else in fragment shader
- Unnormalized quaternion

## References

- `docs/game-design/3D_GAME_DEVELOPMENT.md`
- `docs/game-design/3D_MATHEMATICAL_FOUNDATIONS.md`
- `docs/game-design/3D_MODULE_ARCHITECTURE.md`
- `docs/game-design/AI_DEBUGGABLE_3D_ARCHITECTURE.md`
