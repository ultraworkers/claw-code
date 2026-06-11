# AI-Debuggable 3D Game Architecture

**Source:** Architectural blueprint for LLM-compatible 3D game debugging
**Purpose:** Enable AI agents to troubleshoot, debug, and design 3D game features autonomously
**Constraint:** AI is blind to visual feedback and heavily constrained by context window

---

## The Core Problem

Traditional game development relies on monolithic codebases, deep inheritance hierarchies, and visual debugging tools (like watching a character fall through a floor in a scene editor). An AI, however, is blind to visual feedback and heavily constrained by its context window.

If troubleshooting a bug requires reading 15 separate inherited classes across 10,000 lines of code just to understand how a character moves, the AI will hallucinate or fail.

**Solution:** The architecture must be strictly **modular, semantically observable, and highly deterministic**.

---

## 1. The Death of Deep Inheritance: Embracing ECS

Historically, game engines relied on Object-Oriented Programming (OOP) with deep inheritance trees:

```
GameObject → Actor → Pawn → Character → PlayerCharacter
```

For an AI, debugging an issue in `PlayerCharacter` means it must load the entire inheritance chain into its context window to understand where a variable is being mutated. This wastes tokens and causes cognitive overload for the LLM.

### Composition Over Inheritance: Entity Component System (ECS)

**In an ECS architecture:**

- **Entities** are just blank IDs (e.g., Entity #402). They contain no logic.
- **Components** are pure data structs:
  ```
  PositionComponent {x, y, z}
  HealthComponent {current, max}
  ```
  They contain no functions.
- **Systems** are pure logic functions that iterate over specific components:
  ```
  MovementSystem finds all entities with PositionComponent + VelocityComponent
  and updates: P_new = P_old + V · Δt
  ```

### The AI Debugging Advantage

If a 3D vehicle is moving incorrectly, the AI does not need to read the entire `Vehicle` class. It only needs to query the `PhysicsSystem` and the `VelocityComponent`.

Because state (data) is completely decoupled from logic (systems), the AI can instantly isolate the scope of the bug. It can rewrite the `MovementSystem` without any fear of accidentally breaking the `RenderingSystem` or `AudioSystem`.

**Guardrail ECS-01:** AI-generated 3D game architecture must use ECS or composition-based patterns. Deep inheritance hierarchies (>3 levels) are PROHIBITED.

**Guardrail ECS-02:** Components must be pure data (no methods). Systems must be pure functions (no state). Violations make AI debugging impossible.

---

## 2. Decoupled State and Dependency Injection

When an AI attempts to troubleshoot a bug—such as an inventory item not spawning when a 3D chest is opened—it often struggles with hidden dependencies.

If the `Chest` object directly instantiates a `DatabaseManager` to query the item, the AI cannot easily test the chest in isolation.

### Implementing Dependency Injection (DI)

Your modular architecture must use Dependency Injection or Service Locators. Code should never instantiate its own dependencies.

**BAD — AI cannot isolate this for testing:**
```csharp
public class TreasureChest {
    public void Open() {
        var db = new ItemDatabase();
        Spawn3DModel(db.GetRandomItem());
    }
}
```

**GOOD — AI can inject a mock database to test the 3D spawning logic:**
```csharp
public class TreasureChest {
    private IItemProvider _itemProvider;

    public TreasureChest(IItemProvider provider) {
        _itemProvider = provider;
    }

    public void Open() {
        Spawn3DModel(_itemProvider.GetRandomItem());
    }
}
```

By decoupling dependencies, an AI agent can write automated Unit Tests for the `TreasureChest` by injecting a mock `IItemProvider`. The AI can instantly verify if the bug is in the 3D spawning logic or the database logic, cutting debugging time in half.

**Guardrail DI-01:** All service dependencies must be injectable via interfaces. `new Service()` inside gameplay code is PROHIBITED.

**Guardrail DI-02:** AI must generate unit tests alongside feature code, using mock dependencies to verify each system in isolation.

---

## 3. Designing for AI Observability (Semantic Telemetry)

An AI cannot launch the engine, press "Play", and watch the character clip through a wall. It relies entirely on text. Therefore, the game must be designed to output **Semantic Spatial Telemetry** — a way of translating a 3D physical space into a highly structured text stream that an LLM can map logically.

### Headless Execution and State Dumping

To troubleshoot a 3D bug, the AI needs the game engine to run in a "headless" state (no graphics rendering) and execute a specific scenario. At the moment the bug occurs, the engine must generate a structured JSON dump of the localized spatial state.

**Raw (Bad for AI):**
```
Player Pos: 10.2, -50.1, 4.4 | Wall Pos: 10.0, -50.0, 4.0
```

**Semantic (Good for AI):**
```json
{
  "event": "Collision_Failure",
  "entity": "Player_1",
  "state": {
    "position": {"x": 10.2, "y": -50.1, "z": 4.4},
    "velocity": {"x": 0, "y": -9.8, "z": 0},
    "bounds_type": "Capsule",
    "intersecting_with": ["Wall_04"]
  },
  "spatial_context": "Player_1 is inside the bounding box of Wall_04. Distance to Wall_04 surface normal is -0.2 units (penetrating)."
}
```

By converting 3D physics data into semantic relationships ("penetrating", "intersecting_with"), the AI immediately understands the geometric failure without needing to visualize the math.

**Guardrail TELEMETRY-01:** All 3D physics systems must output semantic telemetry in structured JSON. Raw float dumps are PROHIBITED for AI consumption.

**Guardrail TELEMETRY-02:** Telemetry must include human-readable spatial_context strings explaining the geometric relationship in natural language.

---

## 4. The Spatial Query API (MCP Integration)

To allow the AI to actively troubleshoot, the game engine must expose an API (often via the Model Context Protocol, or MCP). When an AI is given a bug report, it shouldn't just guess the fix. It should use tools to query the engine.

### Required MCP Endpoints

**1. `query_raycast(origin, direction, distance)`**
- The AI can ask the engine what is in front of the player to test line-of-sight bugs

**2. `get_navmesh_path(start, end)`**
- If an AI enemy is stuck, the LLM can query the navigation mesh to see if a valid path exists
- Instantly determines if it's a map geometry issue or a pathfinding script issue

**3. `step_physics(frames)`**
- The AI can instruct the engine to advance the physics simulation by exactly N frames and report the new coordinates
- Allows the AI to trace velocity arcs dynamically

**4. `query_component(entity_id, component_type)`**
- Returns the current state of any component on any entity
- Enables the AI to inspect state without reading the entire codebase

**5. `set_component(entity_id, component_type, values)`**
- Allows the AI to modify component values to test hypotheses
- Combined with `step_physics()` enables automated bisection debugging

**Guardrail MCP-01:** All MCP endpoints must return structured JSON with both raw values AND semantic interpretation.

**Guardrail MCP-02:** MCP endpoints must be idempotent where possible. The AI should be able to query the same state multiple times without side effects.

---

## 5. Deterministic Execution

The deadliest enemy of AI debugging is the **"Heisenbug"** — a bug that only happens sometimes due to framerate fluctuations, floating-point inaccuracies, or random number generators (RNG). An AI cannot debug a non-deterministic system because it cannot verify if its code fix actually worked or if the bug just randomly decided not to occur.

### Fixed Time Steps

Never tie game logic or physics to the render frame rate (`DeltaTime`). An AI testing the game in a headless environment might run it at 1000 FPS, resulting in different physics math than a player running at 60 FPS.

All AI-debuggable logic must happen in a **Fixed Update loop** (e.g., exactly 50 ticks per second), where the time step value is a constant.

```
Fixed Delta Time = 0.02s (50Hz)
```

### Seeded Randomness

If an AI is debugging a procedural 3D dungeon generator that sometimes traps the player, the generator must use a **seeded RNG**. When the AI spins up a test environment, it passes the exact seed where the bug occurred. This guarantees the AI sees the exact same faulty geometry every time it runs the test until it successfully patches the algorithm.

**Guardrail DETERMINISM-01:** All AI-testable code paths must use fixed time steps. `Time.deltaTime` in gameplay logic is PROHIBITED.

**Guardrail DETERMINISM-02:** All RNG used in level generation, loot drops, or AI behavior must accept and log a seed parameter. Non-seeded RNG is PROHIBITED for testable systems.

**Guardrail DETERMINISM-03:** Physics simulations must be deterministic across runs with the same seed and input sequence. Platform-specific floating-point differences must be documented and mitigated.

---

## 6. Defensive Coding and Assertions

AI models write code based on statistical probability. To prevent the AI from introducing silent errors into a complex 3D pipeline, the game's architecture must be heavily armored with **Assertions** and **Design by Contract**.

### Aggressive Assertions

An assertion is a strict rule placed in the code that immediately crashes the game (in debug mode) if a condition is false. This provides the AI with an immediate, precise stack trace rather than allowing a silent failure to corrupt the game state 10 seconds later.

**Example — Quaternion Validation:**
```csharp
public void ApplyRotation(Quaternion rot) {
    // Contract: The AI-generated quaternion must be normalized.
    float magnitude = Mathf.Sqrt(rot.x*rot.x + rot.y*rot.y + rot.z*rot.z + rot.w*rot.w);
    Debug.Assert(Mathf.Abs(magnitude - 1.0f) < 0.001f, 
        "AI Error: Generated Quaternion is not normalized!");
    _transform.rotation = rot;
}
```

When the AI's hallucinated math triggers this assertion, the engine feeds the exact error message back to the LLM. The AI reads "Quaternion is not normalized", realizes its mathematical oversight, and immediately issues a corrected code patch that includes a normalization step.

**Guardrail ASSERT-01:** All AI-generated math operations (quaternions, matrices, vectors) must include validation assertions in debug builds.

**Guardrail ASSERT-02:** Assertion messages must be AI-actionable — include the expected condition, the actual value, and a suggested fix pattern.

**Guardrail ASSERT-03:** Never ship assertions in release builds. Use compile-time `#if DEBUG` guards.

---

## 7. Modular Shader and Graphics Debugging

Troubleshooting rendering issues in a 3D game is incredibly difficult for AI because shaders operate on the GPU in massive parallel arrays, making standard step-through debugging impossible.

### Render Pass Hashing

Modern engines use multiple rendering passes:
1. Depth Pass
2. Albedo Pass
3. Lighting Pass
4. Post-Processing Pass

To help an AI debug a visual glitch, the engine can be configured to capture a "hash" or statistical average of the pixel colors in each render buffer during automated testing.

If a shader is broken, the engine reports to the AI:

```
Render Pipeline Audit:
  Depth Pass [OK]
  Albedo Pass [OK]
  Lighting Pass [FAILED: Buffer average outputs pure NaN]
```

The AI instantly knows the issue is not with the 3D model's UV mapping or textures (Albedo), but specifically within the lighting calculation in the shader (likely a division by zero or a negative dot product in the specular calculation).

**Guardrail SHADER-01:** AI-generated shader code must be modular — one shader per render pass stage. Monolithic uber-shaders are PROHIBITED.

**Guardrail SHADER-02:** Each render pass must expose a validation hash that the AI can query via MCP to detect NaN, Inf, or all-black output.

**Guardrail SHADER-03:** AI must generate shader fallbacks — if the primary shader fails compilation, a simpler fallback must be available.

---

## 8. The AI Debugging Feedback Loop Workflow

When these architectural pillars are in place, the workflow for an AI debugging a 3D game becomes highly streamlined.

### Step-by-Step AI Debugging Workflow

**1. Issue Identification**
- A user reports a bug: "Character falls through the elevator"

**2. State Loading**
- The AI queries the game repository to load the automated test scenario matching the elevator physics

**3. Telemetry Gathering**
- The AI runs the headless simulation via an MCP tool
- The engine stops at the exact frame of the failure and dumps the semantic spatial JSON

**4. Isolation**
- Because the architecture is ECS, the AI knows it only needs to read the `ElevatorPlatformSystem` and the `KinematicCollisionSystem`
- It ignores the audio, rendering, and input systems entirely

**5. Hypothesis and Patch**
- The AI identifies that the elevator is moving downward faster than the player's gravity application
- It rewrites the update order in the `KinematicCollisionSystem` or modifies the friction threshold

**6. Verification**
- The AI automatically re-runs the headless test with the same deterministic seed
- The assertions pass, the telemetry reports normal state
- The AI safely merges the fix into the codebase

---

## Conclusion

By stripping away the monolithic, visually dependent structures of traditional game development, you create a sterile, mathematically observable environment where an AI can read, reason, and repair complex 3D logic purely through code and data.

The seven pillars:
1. **ECS Architecture** — No deep inheritance
2. **Dependency Injection** — Testable, mockable components
3. **Semantic Telemetry** — 3D space as structured text
4. **MCP Integration** — AI queries engine state via tools
5. **Deterministic Execution** — Fixed timesteps, seeded RNG
6. **Aggressive Assertions** — AI-actionable failure messages
7. **Modular Shaders** — Per-pass validation and fallbacks

---

*Part of Agent Guardrails Template v3.1.0 — AI-Debuggable 3D Architecture*
