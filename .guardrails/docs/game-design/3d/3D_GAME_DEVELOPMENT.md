# 3D Game Development Guardrails

**Version:** 1.0.0
**Last Updated:** 2026-05-11
**Applies To:** ALL 3D game development projects using AI-assisted workflows

---

## Purpose

This document enables AI agents to rapidly build 3D games with built-in safety. These guardrails are your license to generate at full velocity — they ensure:

1. **Asset pipeline integrity** — Consistent formats, compression, LOD strategies
2. **Engine convention compliance** — Godot 4.x patterns, scene architecture, performance
3. **AI-assisted workflow clarity** — When to use AI for code vs assets vs design
4. **Scope discipline** — Prevent feature creep that kills solo projects
5. **Performance budgets** — Hard limits per platform that agents must respect

---

## Agent-3DDev-2026 Role Definition

**Agent-3DDev-2026** (3D Game Development 2026) is the specialized agent role responsible for:

| Capability | Description | Constraint |
|------------|-------------|------------|
| **Asset Pipeline** | Model import, texture processing, LOD generation | Format compliance enforced |
| **Scene Architecture** | Godot scene trees, node composition, autoloads | Performance budget respected |
| **AI Workflow Routing** | Decides what AI generates vs what needs human review | Safety checklist verified |
| **Scope Enforcement** | Feature kill decisions, prototype boundaries | One-room rule mandatory |
| **Performance Validation** | Draw calls, poly counts, shadow budgets | Budget exceedance = halt |

### AI-Optimized Development

These standardized patterns exist so agents don't reinvent the wheel on every generation. When building 3D games:
- **Use the asset pipeline as a checklist** — pre-validated formats, no guessing
- **Performance budgets are pre-calculated** — no need to benchmark from scratch
- **Scope rules are absolute** — agents know when to stop adding features

---

## CORE PRINCIPLES

### The Four Laws of 3D Development

1. **Prototype First** — One room, one mechanic, one enemy type before expansion
2. **Asset Pipeline Discipline** — Import once, process correctly, never hand-patch
3. **Performance Budget Bound** — Hard limits per platform, no exceptions
4. **AI Knows Its Limits** — Code = AI-strong, assets = AI-assisted, design = human-led

---

## SAFETY PROTOCOLS (MANDATORY)

### Pre-Implementation Checklist

**EVERY agent MUST verify these before ANY 3D game implementation:**

| # | Check | Requirement | Verify |
|---|-------|-------------|--------|
| 1 | **SCOPE LOCKED** | Single room / single mechanic prototype defined | [ ] |
| 2 | **ENGINE VERSION** | Godot 4.2+ confirmed | [ ] |
| 3 | **ASSET PIPELINE** | Import workflow documented for target platform | [ ] |
| 4 | **PERFORMANCE BUDGET** | Draw calls / poly count budget set per platform | [ ] |
| 5 | **AI WORKFLOW PLAN** | Code vs asset vs design responsibilities assigned | [ ] |
| 6 | **LOD STRATEGY** | Distance thresholds and mesh variants planned | [ ] |
| 7 | **TEST TARGET** | Minimum viable hardware identified | [ ] |

### Scope Enforcement Rules

| Rule | Threshold | Consequence |
|------|-----------|-------------|
| **ONE ROOM MAX** | Prototype = single navigable space | Prevents open-world scope creep |
| **ONE MECHANIC** | Core loop must fit in 3 sentences | Feature creep automatic halt |
| **NO CUSTOM SHADERS** | Use built-in + Material Maker only | Shader debugging is a time sink |
| **NO PHYSICS CUSTOM** | Godot physics or Jolt only | Custom physics = project death |
| **3 ENEMY TYPES MAX** | Prototype diversity cap | Balance explosion prevention |
| **NO NETWORKING** | Single-player only in prototype | Netcode complexity forbidden |

### Asset Pipeline Rules

| Asset Type | Format | Compression | Notes |
|------------|--------|-------------|-------|
| **3D Models** | `.glb` (glTF 2.0) | Draco or native | Prefer glb over fbx |
| **Textures (Color)** | `.webp` or `.png` | Basis Universal / BC7 | Power-of-2 dimensions |
| **Textures (Normal)** | `.png` | BC5 / RGTC | No compression artifacts |
| **Audio (SFX)** | `.ogg` | Vorbis q4 | < 500KB per file |
| **Audio (Music)** | `.ogg` | Vorbis q6 | Stream, don't preload |
| **Animations** | `.glb` embedded | Native | Retarget in Godot |

### LOD Thresholds

| LOD Level | Distance | Tri Count % | Notes |
|-----------|----------|-------------|-------|
| **LOD0** | 0-10m | 100% | Full detail, cast shadows |
| **LOD1** | 10-30m | 50% | Simplified normals |
| **LOD2** | 30-100m | 25% | No shadows, baked AO |
| **LOD3** | 100m+ | 10% | Billboard or cull |

### Texture Size Budgets

| Platform | Max Texture | Texture Array | Atlas Strategy |
|----------|-------------|---------------|----------------|
| **Mobile** | 1024x1024 | 2048x2048 atlas | Aggressive atlasing |
| **Desktop** | 2048x2048 | 4096x4096 atlas | Moderate atlasing |
| **Console** | 4096x4096 | 8192x8192 atlas | Platform-specific |

---

## ENGINE CONVENTIONS (Godot 4.x)

### Scene Architecture

```
Main.tscn (autoload or root)
├── Player.tscn (CharacterBody3D)
│   ├── MeshInstance3D
│   ├── CollisionShape3D
│   └── Camera3D
├── Level.tscn (Node3D)
│   ├── Environment (WorldEnvironment)
│   ├── Lighting (DirectionalLight3D + OmniLight3D)
│   └── Props/ (instanced scenes)
└── UI.tscn (CanvasLayer)
    ├── HUD
    └── Menus
```

### Node Naming Conventions

| Prefix | Type | Example |
|--------|------|---------|
| `p_` | Player | `p_Player` |
| `e_` | Enemy | `e_Zombie` |
| `prop_` | Prop | `prop_Crate` |
| `env_` | Environment | `env_Ground` |
| `ui_` | UI Element | `ui_HealthBar` |
| `cam_` | Camera | `cam_Main` |

### Script Organization

```gdscript
# player.gd — Maximum 500 lines per script
extends CharacterBody3D

# === CONSTANTS ===
const MAX_SPEED := 10.0

# === EXPORTED VARIABLES ===
@export var health: int = 100

# === ONREADY REFERENCES ===
@onready var mesh := $MeshInstance3D
@onready var anim_player := $AnimationPlayer

# === BUILT-IN VIRTUAL METHODS ===
func _ready() -> void:
    pass

func _physics_process(delta: float) -> void:
    pass

# === PUBLIC METHODS ===
func take_damage(amount: int) -> void:
    pass

# === PRIVATE METHODS ===
func _update_animation() -> void:
    pass

# === SIGNAL CALLBACKS ===
func _on_hitbox_body_entered(body: Node3D) -> void:
    pass
```

### Autoload Singletons (Max 5)

| Singleton | Responsibility | Forbidden |
|-----------|---------------|-----------|
| `GameState` | Score, progress, settings | No scene references |
| `AudioManager` | Music, SFX, volume | No game logic |
| `InputManager` | Remapping, actions | No direct node access |
| `SceneLoader` | Transitions, loading | No game state changes |
| `SaveManager` | Serialize/deserialize | No runtime logic |

---

## AI-ASSISTED 3D WORKFLOW

### The AI Responsibility Matrix

| Task | AI Role | Human Review | Notes |
|------|---------|--------------|-------|
| **GDScript code** | Primary author | Spot review | AI excels at Godot patterns |
| **Shader code** | Assisted | Required | Visual output must be checked |
| **3D models** | Blockout only | Cleanup required | AI mesh topology is poor |
| **Textures** | Generated | Review | SD + Material Maker pipeline |
| **Level design** | Assisted | Lead | AI blockout, human curation |
| **Animation** | Retargeting | Required | Mixamo → Godot skeleton |
| **Audio (SFX)** | Generated | Review | AI audio is decent |
| **Audio (Music)** | Assisted | Lead | Suno/UDIO + human arrangement |
| **Game design doc** | Drafted | Final approval | AI outlines, human decides |
| **UI/UX layout** | Primary author | Review | Godot UI is AI-friendly |
| **Balance/tuning** | Data analysis | Lead | AI analyzes, human tweaks |

### Workflow Order for New Features

1. **Human** writes 3-sentence design spec
2. **AI** generates GDScript + scene structure
3. **Human** reviews code for logic errors
4. **AI** generates blockout assets (if needed)
5. **Human** replaces with final assets
6. **AI** writes test cases
7. **Human** playtests, files bugs
8. **AI** fixes bugs (repeat 6-8)

---

## PERFORMANCE BUDGETS

### Per-Frame Budgets

| Platform | Draw Calls | Shadow Casters | Lights | Particles |
|----------|-----------|----------------|--------|-----------|
| **Mobile** | 100 | 5 | 4 | 500 |
| **Desktop** | 500 | 20 | 16 | 2000 |
| **Console** | 1000 | 50 | 32 | 5000 |

### Poly Count Budgets (per view)

| Platform | Static | Dynamic | Terrain | Total |
|----------|--------|---------|---------|-------|
| **Mobile** | 50K | 10K | 20K | 80K |
| **Desktop** | 200K | 50K | 100K | 350K |
| **Console** | 500K | 100K | 250K | 850K |

### Shadow Cascade Budgets

| Platform | Cascades | Max Distance | Resolution |
|----------|----------|--------------|------------|
| **Mobile** | 2 | 50m | 1024 |
| **Desktop** | 4 | 200m | 2048 |
| **Console** | 4 | 500m | 4096 |

### Godot-Specific Optimizations

| Feature | Mobile | Desktop | Console |
|---------|--------|---------|---------|
| **GI** | Baked only | Baked + SDFGI | Baked + SDFGI |
| **SSR** | Off | On | On |
| **SSAO** | Off | Low | High |
| **SSR/SSIL** | Off | On | On |
| **Volumetric Fog** | Off | On | On |
| **MSAA** | 2x | 4x | 8x |
| **Physics Tick** | 60Hz | 120Hz | 120Hz |

---

## MANDATORY HALT CONDITIONS

**STOP and ask human when:**

- Performance budget exceeded by > 10%
- Scope creeps beyond one-room prototype without explicit approval
- Custom engine modification required
- Asset pipeline cannot handle requested format
- AI-generated code exceeds 500 lines per file
- Physics behavior feels "floaty" or incorrect
- Memory usage exceeds 500MB (mobile) / 2GB (desktop)

---

## COMPLIANCE VERIFICATION

### Pre-Commit Checklist

| # | Check | Tool | Pass |
|---|-------|------|------|
| 1 | All .glb files import without errors | Godot Import | [ ] |
| 2 | No textures > budget size | Editor Inspector | [ ] |
| 3 | Script files < 500 lines | wc -l | [ ] |
| 4 | Draw call count within budget | Godot Profiler | [ ] |
| 5 | No unneeded shadow casters | Editor Filter | [ ] |
| 6 | Audio files < 500KB (SFX) | ls -lh | [ ] |
| 7 | Autoload count ≤ 5 | Project Settings | [ ] |
| 8 | Scene file sizes < 100KB | ls -lh | [ ] |

---

## AI TOOL MATRIX 2026

Based on 2026 market intelligence. Agents must use these tools within their designated roles.

### 3D Asset Generation Tools

| Tool | Best For | AI Role | Human Review | Export | Cost | Commercial Rights |
|------|----------|---------|--------------|--------|------|-------------------|
| **Meshy.ai** | End-to-end pipeline (gen → texture → rig) | Primary | Final cleanup | FBX/GLB/USDZ | $20/mo | ✅ Full on all tiers |
| **Tripo 3.0** | Fast base mesh generation | Blockout | Retopo + rigging | GLB/OBJ | $19.9/mo | ✅ Paid tiers only |
| **Rodin Gen-2** | API-first, clean topology | Pipeline integration | Rigging | GLB/FBX/BLEND | $30/mo | ✅ Full on all tiers |
| **Luma AI** | Photorealistic NeRF capture | Environment scan | Retopology | GLB/OBJ | $29.9/mo | ✅ Paid plans |
| **Sloyd** | Parametric + generative hybrid | Prop generation | Style review | GLB/FBX | $15/mo | ✅ Full |
| **Scenario** | Style-consistent IP assets | Concept → 3D | IP compliance | GLB/FBX | $15/mo | ⚠️ Varies by plan |

**Agent Rule:** Always verify commercial rights before using AI-generated assets in commercial projects. When in doubt, use Meshy.ai or Rodin (full rights on all tiers).

### AI Code & Editor Assistants

| Tool | Engine | Best For | AI Role |
|------|--------|----------|---------|
| **Claude Code** | Any | GDScript, C#, HLSL | Primary author |
| **GitHub Copilot X** | Unity/UE | C#, C++, HLSL | Assisted |
| **Unity Muse Code** | Unity | ECS/Burst/Shader Graph | Primary |
| **Cursor** | Any | Multi-file refactoring | Assisted |
| **Godot AI (Ollama)** | Godot | Local GDScript RAG | Primary |

### Neural Rendering & Runtime

| Technology | Platform | Use Case | Status |
|------------|----------|----------|--------|
| **DLSS 4+** | NVIDIA PC | Upscaling, frame gen | Production |
| **FSR 4** | AMD PC | ML upscaling | 2026 (RDNA 4) |
| **3D Gaussian Splatting** | All | Photogrammetry backgrounds | Production |
| **NeRF** | PC/Console | Cutscene environments | Limited |
| **NVIDIA ACE** | PC/Cloud | Real-time NPCs | Production |

---

## LEGAL & ETHICAL GUARDRAILS

### Commercial Rights Checklist

**BEFORE using any AI-generated asset in a commercial game:**

| # | Check | Tool | Verify |
|---|-------|------|--------|
| 1 | Platform grants commercial rights | Tool docs/TOS | [ ] |
| 2 | No training data lawsuits pending | Legal research | [ ] |
| 3 | Asset is not obviously "AI slop" | Human review | [ ] |
| 4 | Disclosure plan exists if required | Project plan | [ ] |
| 5 | Hero assets have human refinement | Art review | [ ] |

### AI Disclosure Rules

| Scenario | Action Required |
|----------|-----------------|
| **Premium game ($20+)** | Disclose AI usage in credits |
| **Indie/AA game** | Disclose if asked; transparency preferred |
| **Game jam / prototype** | Optional disclosure |
| **Hero characters / story-critical assets** | Human-crafted or heavily refined |
| **Background props / filler** | AI generation acceptable |

### Prohibited Uses

**NEVER use AI to generate:**
- Copyrighted character likenesses without license
- Real person's likeness without consent
- Hateful, violent, or sexual content involving minors
- Assets directly competing with training data sources

---

## 2026 TECHNOLOGY LANDSCAPE

### Key Trends Agents Should Know

1. **Text-to-3D is production-ready** for props, environments, and stylized characters
2. **Hero assets still need human artists** — AI handles 80%, humans the final 20%
3. **Neural rendering** (DLSS 4, 3DGS) is now standard on PC
4. **Runtime AI NPCs** are viable but require cloud or high-end hardware
5. **AI code generation** excels at GDScript, C#, and shaders

### Quality Benchmarks

| Asset Type | AI-Ready? | Post-Processing Needed |
|------------|-----------|------------------------|
| Hard-surface props | ✅ Yes | Minimal |
| Stylized characters | ✅ Yes | Retopo + rigging (40%) |
| Organic terrain | ✅ Yes | Minimal |
| Hero characters | ⚠️ Partial | Full human pass required |
| Complex mechanical rigs | ❌ No | Human-only |
| PBR materials | ✅ Yes | Review metallic/roughness |

### Engine AI Feature Maturity (2026)

| Feature | Unity 6 | UE 5.5+ | Godot 4.4 |
|---------|---------|---------|-------------|
| Runtime ONNX | Sentis 2.0 | NNI Plugin | GDExtension |
| LLM NPCs | Muse (cloud) | ACE + Plugins | Community |
| AI Code Assist | Muse Code | Verse + Copilot | Ollama/local |
| Neural Upscaling | DLSS/FSR | DLSS/FSR | Community |
| Auto-Asset Gen | Muse Texture | Editor scripts | HTTP APIs |

---

## UPDATED AI RESPONSIBILITY MATRIX (2026)

| Task | AI Role | Human Review | 2026 Tool | Notes |
|------|---------|--------------|-----------|-------|
| **GDScript code** | Primary | Spot | Claude Code | ✅ Excellent |
| **Shader code** | Assisted | Required | Claude/Cursor | Visual check mandatory |
| **3D models (props)** | Primary | Review | Meshy/Tripo | Production-ready |
| **3D models (characters)** | Blockout | Required | Meshy + human | Retopo + rigging |
| **Textures/PBR** | Primary | Review | SD + Material Maker | Quality varies |
| **Level design** | Assisted | Lead | LLM + manual | AI blockout, human curation |
| **Animation** | Retarget | Required | Mixamo → Godot | AI motion gen emerging |
| **Audio (SFX)** | Primary | Review | AI generators | Decent quality |
| **Audio (Music)** | Assisted | Lead | Suno/UDIO | Arrange + polish |
| **Game design doc** | Drafted | Final | Claude | Human decides scope |
| **UI/UX** | Primary | Review | Claude | Godot UI is AI-friendly |
| **NPC dialogue** | Assisted | Lead | Local LLM | Godot + Ollama viable |
| **Performance analysis** | Primary | Review | Godot Profiler | AI interprets data |

---

*Version 1.1.0 · 3D Game Development Guardrails · Updated with 2026 AI Dossier Intelligence · Part of Agent Guardrails Template v3.1.0*
