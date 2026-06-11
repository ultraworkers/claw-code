---
description: 3D Game Development guardrails for Godot, Unity, and custom engines
globs: "**/*.{gd,cs,glsl,hlsl,wgsl,shader,obj,fbx,gltf,blend,tscn,unity}"
alwaysApply: false
---

# 3D Game Development Rules

Enforce mathematical correctness, asset safety, and shader constraints on all 3D game code.

## Geometry & Mesh Rules

1. **Polygon Budget** — Document target platform budget in file header:
   - Mobile: 10K verts/object, 50K/scene
   - PC: 50K verts/object, 500K/scene
   - VR: 5K verts/object, 100K/scene
   - Auto-generate LOD for anything >5K polygons

2. **Topology** — All generated meshes must be:
   - Triangles or quads only (no N-gons)
   - Manifold (watertight, no floating edges)
   - Consistent normals (no inversions)

3. **UV Mapping** — UV islands must not overlap unless explicitly mirrored. Texel density consistent across adjacent meshes.

## Shader Rules

1. **No Dynamic Branching** — NEVER use if/else in fragment shaders. Use `step()`, `smoothstep()`, `mix()` instead.
2. **Samplers** — Max 16 per pass (Forward), 8 per pass (Mobile/VR).
3. **Loops** — Compile-time determinable bounds. Max 64 iterations. Unroll ≤8.
4. **Precision** — Use `half`/`mediump` for color, `float`/`highp` for position.

## Math Rules

1. **Quaternions Only** — Use quaternions for all continuous rotation. NEVER Euler angles.
2. **Normalization** — Assert `abs(q.length() - 1.0) < 0.0001` before applying.
3. **Matrix Order** — `MVP = P * V * M` (column-major). Document convention in header.
4. **Handedness** — Right-handed, Y-up (Z-up for CAD only). Document in header.

## Godot 4.x Rules

1. **Signals** — Use Signals, not `get_node("../../Player")`.
2. **Static Typing** — `var velocity: Vector3`, not `var velocity`.
3. **Threading** — WorkerThreadPool for proc-gen. NEVER call rendering from threads.
4. **Deferred** — Use `call_deferred()` for thread-safe node ops.

## Halt & Ask

- No target platform specified for shader
- No polygon budget for mesh generation
- Euler angles proposed for rotation
- if/else proposed in fragment shader
- Unnormalized quaternion usage
