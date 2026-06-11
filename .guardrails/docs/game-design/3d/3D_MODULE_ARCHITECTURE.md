# 3D Game Design Module Architecture

**Source:** Agent-guardrails-template architectural expansion
**Purpose:** Bridge LLM capabilities with deterministic 3D rendering/physics environments
**Classification:** Architecture Blueprint

---

## Overview

AI agents often hallucinate impossible geometry, inefficient spatial loops, or non-performant shader code. This module provides extreme technical depth to prevent those failures through guardrails, MCP server expansions, procedural generation limits, spatial accessibility rules, and CI/CD pipelines.

---

## 1. Architectural Expansion of .guardrails/ for 3D Environments

The core of the agent-guardrails-template relies on strict behavioral and operational guardrails. For 3D game design, these must transition from standard text/code validation to **spatial, geometric, and mathematical constraints**.

### A. 3D Asset and Geometry Generation (.guardrails/game_design/3d_assets.md)

AI agents generating code that creates procedural geometry or modifies mesh data must be constrained to prevent memory leaks and GPU bottlenecking.

**Polygon and Vertex Budgets:**
- The guardrails must explicitly define the maximum allowable vertex counts based on the target platform (Mobile, PC, VR/Spatial)
- Agents must be instructed to automatically generate or implement Level of Detail (LOD) systems (LOD0, LOD1, LOD2) for any asset exceeding 5,000 polygons

**Topology Rules:**
- The agent must be forbidden from generating code that produces non-manifold geometry, N-gons (polygons with more than 4 sides), or inverted normals
- The guardrail must dictate that all procedurally generated meshes must be composed strictly of triangles or quads to ensure compatibility with standard rasterization pipelines

**UV Mapping Constraints:**
- When generating procedural textures or unwrapping meshes via code, the agent must adhere to strict UV mapping rules
- Overlapping UV islands must be flagged as a violation unless explicitly used for mirrored textures
- The agent must ensure a minimum texel density consistency across adjacent meshes to prevent visual discrepancies

### B. Material and Shader Constraints (.guardrails/game_design/shaders.md)

Writing shader code (GLSL, HLSL, WGSL) is highly prone to AI hallucinations that result in compilation failures or infinite GPU loops.

**Texture Sampling Limits:**
- The guardrail must cap the number of texture samples per shader pass (e.g., maximum of 16 texture samplers for standard Forward Rendering pipelines) to prevent register exhaustion

**Branching Minimization:**
- AI agents must be trained to avoid dynamic branching (if/else statements) inside fragment/pixel shaders, as this breaks GPU warp/wavefront execution sync
- The guardrail must instruct the agent to use mathematical step functions (e.g., `step()`, `smoothstep()`, `mix()`) instead of conditionals

**PBR (Physically Based Rendering) Strictness:**
- All material generation logic must adhere to standard PBR workflows (Metallic/Roughness or Specular/Glossiness)
- The agent must be explicitly forbidden from hardcoding lighting values into the Albedo/BaseColor maps, ensuring all lighting is dynamically calculated by the engine's rendering pipeline

### C. Physics and Spatial Logic Safety (.guardrails/game_design/physics.md)

Physics simulations are highly volatile. A single hallucinated zero in a mass calculation can cause the entire physics engine to crash via NaN (Not a Number) propagation.

**Collision Hull Simplification:**
- The AI must strictly use primitive colliders (Box, Sphere, Capsule) or simplified Convex Hulls for dynamic rigid bodies
- The guardrail must permanently ban the assignment of complex Concave Mesh Colliders to moving objects

**Force and Impulse Clamping:**
- Any AI-generated code applying physical forces (e.g., `AddForce`, `ApplyImpulse`) must include clamp functions to prevent infinite acceleration
- Maximum velocity limits must be programmatically enforced in the agent's code output

**Raycast Budgeting:**
- Raycasts are computationally expensive. The guardrail must dictate that agents cannot place raycasts inside unrestricted `Update()` or `_process()` loops without implementing:
  - Distance limits
  - Layer masks (to ignore irrelevant geometry)
  - Frequency throttling (e.g., checking every 5th frame instead of every frame)

---

## 2. Extending the Go MCP Server (mcp-server/)

The existing Go-based MCP server (backed by PostgreSQL and Redis) must be expanded with specific 3D validation tools. This allows the AI agent to actively query the state of the 3D project and validate its own output before committing code.

### A. Spatial Analysis Tools

**`analyze_mesh_topology(filepath: string)`**
- The agent can call this tool to parse a `.gltf`, `.glb`, or `.obj` file
- The Go server will parse the binary data and return a JSON payload detailing:
  - Vertex count
  - Material slots
  - Bounding box dimensions
  - Manifold status
- If the agent generates a mesh that is too large, the server automatically rejects it based on the guardrail thresholds

**`validate_shader_ast(shader_code: string, language: string)`**
- A tool that takes AI-generated HLSL/GLSL code and runs it through a headless shader compiler (like glslangValidator)
- Returns syntax errors, missing uniform variables, and performance warnings (like excessive ALU instructions) back to the LLM so it can self-correct

### B. Scene Graph Traversal

**`query_scene_hierarchy(scene_path: string)`**
- Modern 3D engines use deep node trees (Unity GameObjects, Godot Nodes, Unreal Actors)
- This tool parses engine-specific scene files (e.g., Godot's `.tscn` or Unity's `.prefab`) and returns a simplified JSON representation to the LLM
- This allows the agent to understand the spatial relationship between objects without needing to hold massive binary files in its context window

**`lint_transform_data()`**
- Checks the scene for floating-point precision issues
- Because 3D spaces suffer from floating-point errors at large distances, this tool checks if any object is placed beyond a safe threshold (e.g., 100,000 units from the origin) and flags it for the agent to implement a floating-origin system

---

## 3. Engine-Specific Engineering Standards (.guardrails/standards/)

Because the agent-guardrails-template supports 14+ languages, the 3D module must provide engine-specific architectural doctrines.

### A. Unity (C#) Standards

**Memory Allocation:**
- The AI must be forbidden from using `Instantiate()` or `Destroy()` frequently during runtime
- The standard must enforce the creation of Object Pooling systems for projectiles, enemies, and particle effects to prevent Garbage Collection (GC) spikes that cause frame stuttering

**Component Architecture:**
- Enforcement of the Single Responsibility Principle within MonoBehaviours
- The agent must separate data (ScriptableObjects) from logic (MonoBehaviours) from spatial rendering

**DOTS (Data-Oriented Technology Stack):**
- For high-performance requirements, the agent must be guided by prompts to utilize Unity's Entity Component System (ECS), structuring data in contiguous memory arrays rather than reference-heavy object-oriented graphs

### B. Unreal Engine (C++) Standards

**UObject and Garbage Collection:**
- Agents writing UE5 C++ must strictly adhere to the `UPROPERTY()` macro standards to ensure pointers are not left dangling, causing fatal engine crashes

**Blueprint vs. C++ Delegation:**
- The guardrails must define what the AI should write in C++ (heavy math, core systems, networking) versus what it should expose to Blueprints via `UFUNCTION(BlueprintCallable)` (UI, timeline animations, audio triggers)

**Lumen and Nanite Compliance:**
- Code generating procedural environments must tag static meshes appropriately to take advantage of Unreal's virtualized geometry (Nanite) and global illumination (Lumen), specifically ensuring that generated meshes have sufficient surface parameterization

### C. Godot 4 (GDScript / C++) Standards

**Signal Management:**
- The AI must use Godot's Signal system for decoupled communication instead of tight node coupling
- Avoid `get_node("../../Player")` in favor of exported node paths or signals

**Thread Safety:**
- Guidelines for using `WorkerThreadPool` for procedural generation tasks
- Ensure that the AI does not attempt to call rendering-specific functions or manipulate the SceneTree from a background thread, which causes immediate crashes in Godot

---

## 4. 3D Accessibility Standards (WCAG 3.0+ & Spatial)

Building on the repository's existing support for WCAG 3.0+, 3D game design introduces entirely new accessibility vectors, specifically concerning Spatial Computing and vestibular safety.

**Vestibular/Motion Sickness Prevention:**
- The AI must automatically implement specific camera safety features in any player-controller script
- Force an adjustable Field of View (FOV) slider (clamped between 70 and 110 degrees)
- Option to disable "head bobbing" or camera shake
- Implement a persistent center-screen reticle to ground the player's vestibular system during rapid movement

**High-Contrast and Silhouette Rendering:**
- The agent must be trained to generate custom depth-mask shaders that outline interactable 3D objects
- Ensures players with low vision can distinguish key items from background noise

**Spatial Audio Cues (HRTF):**
- The guardrails must mandate that all spatial audio implementations include Head-Related Transfer Functions (HRTF)
- Allows visually impaired players to navigate 3D spaces entirely via echolocation and audio attenuation curves
- The AI must never place a critical audio source without an accompanying 3D spatializer

---

## 5. "Vibe Coding" & Shared Prompts (skills/shared-prompts/)

To effectively utilize AI in 3D design, the prompt templates must handle complex spatial mathematics, which LLMs notoriously struggle with.

### A. spatial_math_advisor.md

This system prompt is injected into the LLM's context whenever dealing with 3D rotations or vectors.

**Content Focus:**
- Forces the LLM to abandon Euler angles (which suffer from Gimbal Lock) and strictly use Quaternions for 3D rotations
- Provides the agent with mathematical templates for:
  - Vector Cross Products (finding normal vectors)
  - Dot Products (calculating FOV cones and facing directions)
  - Matrix transformations

**Prompt Excerpt:**
> "You are an expert spatial mathematician. Whenever asked to rotate an object over time, you MUST use Quaternion.Slerp (Spherical Linear Interpolation) rather than directly modifying XYZ rotation values. If calculating the angle between two vectors, you MUST normalize them first and use the Dot Product."

### B. procedural_generation_vibe.md

A template tailored for writing algorithms like Perlin Noise, Wave Function Collapse, or Marching Cubes.

**Content Focus:**
- Instructs the AI on how to chunk procedural generation to avoid freezing the main thread
- Guides the AI to use coroutines or background threads and yield execution back to the main game loop every few milliseconds

---

## 6. Workflows and Sprints (.guardrails/workflows/)

Operational workflows ensure that human developers and AI agents collaborate sequentially on complex 3D tasks.

**3d_performance_profiling.md:**
- A step-by-step diagnostic workflow for the AI to follow when a user reports a frame drop
- The AI is instructed to ask for the engine's profiler output, check the ms/frame metrics
- Identify if the bottleneck is CPU-bound (too many draw calls, heavy physics) or GPU-bound (too much overdraw, complex shaders)
- Systematically propose optimizations

**animation_state_machine_setup.md:**
- A procedural checklist for generating animation controllers
- Forces the AI to build centralized Blend Trees (e.g., Idle -> Walk -> Run based on velocity magnitude)
- Strictly manage transition conditions to prevent "animation popping" or state-machine deadlocks

---

## 7. CI/CD & Testing Pipelines (.github/workflows/ and ci/)

Integrating 3D into the repository requires heavily specialized Continuous Integration pipelines to test the AI's output automatically.

**Headless Rendering Tests:**
- The CI pipeline must be configured to run the 3D engine (Unity/Godot) in a headless Linux container via Docker
- When the AI agent commits new code or assets, the CI spins up a dummy scene, runs the code for 600 frames, and tracks the frame rate, memory allocations, and error logs
- If the AI's code causes the frame time to spike above 16.6ms (dropping below 60 FPS), the PR is automatically rejected

**Asset Bundle Size Checks:**
- 3D games easily bloat in size. The CI pipeline will include a script that analyzes the AI's imported assets
- If an AI generates or includes a 4K uncompressed texture (which takes up ~16MB of VRAM), the CI fails the build and comments on the PR:
  > "Guardrail Violation: Texture exceeds 2048x2048 and lacks compression. Please implement ASTC/DXT compression and reduce resolution."

**Automated Spatial Linting:**
- A custom python script running in GitHub Actions that parses all newly committed 3D scene files
- Ensures no objects are spawned inside each other (triggering physics explosions on load)
- Verifies that all light sources are properly baked or culled
- Ensures that no mesh lacks a material assignment (preventing bright pink/magenta fallback shaders from reaching production)

---

## Conclusion

By implementing these seven architectural pillars, the agent-guardrails-template transforms from a robust software engineering framework into a world-class, production-ready AI agent environment capable of safely autonomously designing, coding, and optimizing fully functional 3D video games without hallucinating destructive engine parameters.

---

*Part of Agent Guardrails Template v3.1.0 — 3D Game Design Module*
