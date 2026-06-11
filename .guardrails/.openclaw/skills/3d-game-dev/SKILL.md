---
name: 3d-game-dev
description: "3D Game Development guardrails for AI-assisted game creation. Enforces mathematical correctness, asset safety, shader constraints, and Godot/Unity conventions."
---

# 3D Game Development Skill

Use when the agent is generating, modifying, or reviewing code/assets for a 3D game project.

## Applicability

- Procedural geometry generation
- Shader/material code (GLSL, HLSL, WGSL, Godot Shader Language)
- 3D physics and collision code
- Camera, lighting, and rendering pipeline code
- Asset import/export pipelines
- Spatial audio or XR/VR implementations

## Mandatory Guardrails

### 1. Asset Safety

**Polygon/Vertex Budgets:**
- Mobile: max 10K vertices per dynamic object, 50K per scene
- PC: max 50K per object, 500K per scene
- VR/Spatial: max 5K per object, 100K per scene
- Auto-generate LOD0/LOD1/LOD2 for anything >5K polygons

**Topology Rules:**
- NEVER generate non-manifold geometry
- NEVER generate N-gons (polygons >4 sides)
- NEVER generate inverted normals
- All meshes must be triangles or quads only

**UV Mapping:**
- Overlapping UV islands are FORBIDDEN unless explicitly mirrored
- Texel density must be consistent across adjacent meshes
- UVs must be in [0,1] range unless tiling is intentional

### 2. Shader Constraints

**Texture Sampling:**
- Max 16 texture samplers per shader pass (Forward Rendering)
- Max 8 samplers per pass (Mobile/VR)

**Branching:**
- NEVER use if/else in fragment/pixel shaders
- Use step(), smoothstep(), mix() instead
- Dynamic branching breaks GPU warp synchronization

**Loop Bounds:**
- All loops must have compile-time determinable bounds
- Max 64 iterations per shader loop
- Unroll small loops (≤8 iterations) explicitly

### 3. Mathematical Correctness

**Coordinate Systems:**
- Use right-handed coordinate system
- Y-up is standard (Z-up only for CAD/CAM workflows)
- Document handedness in every file header

**Rotation:**
- Use quaternions for all continuous rotations
- NEVER use Euler angles for animation (Gimbal Lock risk)
- Normalize quaternions before every transform application
- Verify: `assert(abs(q.length() - 1.0) < 0.0001)`

**Matrix Pipeline:**
- Follow Model → World → View → Projection order
- Always validate: `MVP = P * V * M` (column-major)
- Document matrix convention (row vs column) in file headers

**Precision:**
- Use double precision for world-space coordinates (large worlds)
- Use float for local/model space
- Use fixed-point for mobile GPU calculations where possible

### 4. Godot 4.x Conventions

**Node Communication:**
- Use Signals for decoupled communication
- NEVER use `get_node("../../Player")` — use exported NodePaths or Signals
- Signal emissions must include type-safe parameters

**Threading:**
- Use WorkerThreadPool for procedural generation
- NEVER call rendering functions from background threads
- NEVER manipulate SceneTree from background threads
- Use call_deferred() for thread-safe node operations

**GDScript:**
- Static typing: `var velocity: Vector3` not `var velocity`
- Use `@onready` for node references
- Use `@export` for configurable parameters
- Document units in comments: `meters`, `seconds`, `radians`

### 5. AI-Debuggable Architecture

**ECS over Inheritance:**
- Use Entity-Component-System, not deep inheritance
- Each component must be serializable to JSON
- Systems must be deterministic and replayable

**Semantic Telemetry:**
- Report spatial errors semantically: `{ "event": "Collision_Failure", "entity": "Player_1", "expected_surface": "Floor_Mesh", "actual_contact": "Void" }`
- NEVER dump raw float coordinates to LLM context

**Headless Mode:**
- All scenes must run in `--headless` for CI testing
- JSON state dumps at frame 0, N/2, and N
- Frame rate, memory, and error logs tracked over 600 frames

## Halt Conditions — STOP and Ask User

- Generating shader code without target platform specified
- Proposing mesh generation without polygon budget defined
- Modifying physics code without test scene verification
- Using Euler angles for 3D rotation (quaternions required)
- Dynamic branching in fragment shaders proposed
- Working with unnormalized quaternions
- Modifying rendering pipeline without headless test plan

## Compliance Verification

- Run `scripts/validate_3d_assets.py` on generated meshes
- Run `scripts/validate_shaders.py` on shader code
- Run `scripts/validate_math.py` on transform/rotation code
- CI: 600-frame headless test must pass (FPS >30, memory stable)

## References

- `docs/game-design/3D_GAME_DEVELOPMENT.md` — Full guardrails
- `docs/game-design/3D_MATHEMATICAL_FOUNDATIONS.md` — Math reference
- `docs/game-design/3D_MODULE_ARCHITECTURE.md` — Architecture blueprint
- `docs/game-design/AI_DEBUGGABLE_3D_ARCHITECTURE.md` — AI debugging patterns
- `docs/game-design/3D_GUARDREL_PROPOSALS_V1.2.md` — New rule proposals
