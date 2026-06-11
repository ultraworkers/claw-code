# 3D Game Development Guardrails — v1.2 Proposed Additions

**Source:** Hermes 2026 AI Dossier review by parallel subagents
**Target Branch:** `feature/3d-game-development` on `agent-guardrails-template`
**Status:** Draft for Roger review

---

## SUMMARY OF GAPS IDENTIFIED

The existing 3D guardrails (v1.1.0) cover asset pipelines, Godot conventions, performance budgets, and basic AI workflow routing. The 2026 dossier reveals major areas with NO guardrail coverage:

| Area | Existing Coverage | Gap Severity |
|------|------------------|--------------|
| Neural Rendering | Technology table only | HIGH |
| AI Animation/Motion | "Retargeting required" only | HIGH |
| AI Code Generation (Runtime) | "AI-primary" blanket rule | CRITICAL |
| Neural Physics | "No custom physics" only | CRITICAL |
| AI QA/Testing/Balance | ZERO coverage | HIGH |
| Runtime Inference | Feature matrix only | CRITICAL |
| AI NPCs/Dialogue | "Assisted, Lead review" only | CRITICAL |
| World/Level Generation | Not addressed | MEDIUM |
| Business/Vendor Lock-in | Not addressed | MEDIUM |
| AI Disclosure/Legal | Basic checklist only | HIGH |
| Social/Ethical Risks | Not addressed | MEDIUM |

---

## NEW GUARDRAIL PROPOSALS

### 1. NEURAL RENDERING GUARDRAILS

**NEURAL_RENDER-01:** Before enabling DLSS Multi-Frame Generation or FSR frame interpolation, verify minimum GPU tier (RTX 40-series+ for MFG; any GPU for FSR 3.1). Frame generation adds 1-2 frames of latency — DISABLE for competitive multiplayer or precise platforming.

**NEURAL_RENDER-02:** 3D Gaussian Splatting scenes must have a fallback mesh LOD for platforms without compute shader support (mobile, Switch). Splat count budget: 2M splats max on desktop, 500K on mid-tier, 100K on mobile.

**NEURAL_RENDER-03:** AI-generated shaders (NVIDIA ShaderPlay, Unity Shader Graph AI) must pass a frame-time regression test. Budget: shader must not increase frame time by >5% vs baseline. Complex neural shaders are PROTOTYPE-ONLY unless profiled.

**NEURAL_RENDER-04:** NeRF is PROHIBITED for primary gameplay viewports. Allowed only for cutscenes, photomode, or background streaming with pre-baked cube-map proxies at ≤6fps update.

**NEURAL_RENDER-05:** Neural material LOD (tiny MLPs replacing mipmaps) requires platform validation — only enable on PC/PS5/XSX. VRAM savings must be measured; fallback to standard mipmaps if inference overhead exceeds 0.5ms/frame.

---

### 2. AI ANIMATION & MOTION GUARDRAILS

**AI_ANIM-01:** AI-generated motion (video-to-animation, diffusion-based) MUST pass a foot-lock and root-motion validation pass. Any clip with visible foot sliding, ground penetration, or root drift >5cm requires manual cleanup or rejection.

**AI_ANIM-02:** LMM / neural animation compression is allowed only for non-hero NPCs and ambient background animations. Hero player animations and cutscene motions must use uncompressed or standard compressed sources.

**AI_ANIM-03:** ML Deformers (muscle/soft-tissue) are PROTOTYPE-ONLY unless running on verified GPU compute (RTX 3060+ / console). Must have traditional blend-shape fallback for mobile/integrated graphics.

**AI_ANIM-04:** Audio2Face / live facial animation must maintain <50ms latency from audio input to mesh deformation. If latency exceeds 100ms, disable live mode and use pre-baked phoneme clips.

**AI_ANIM-05:** AI motion databases for Motion Matching must be tagged by generation source (mocap vs AI-generated). AI-generated motions require 20% higher blend-threshold tolerance to avoid unnatural transitions.

**AI_ANIM-06:** Facial animation from AI must include an "uncanny valley" human review gate. If face looks "almost right but slightly off," reject and use keyframed or traditional mocap.

---

### 3. AI CODE GENERATION GUARDRAILS (CRITICAL)

**AI_CODE-01:** AI-generated code that will execute at RUNTIME (NPC behavior scripts, Verse snippets, JIT-compiled logic) requires HUMAN APPROVAL before merge. Static/editor-time code = spot review; runtime code = mandatory review.

**AI_CODE-02:** AI-generated balance patches (loot tables, damage values, spawn rates) must be applied via version-controlled diffs with human sign-off. NEVER auto-deploy AI balance changes to live production.

**AI_CODE-03:** AI-generated gameplay systems (>200 lines or multi-file) must include a "complexity budget" justification: enumerate what it replaces, why AI generation was chosen, and what the rollback plan is.

**AI_CODE-04:** AI-generated shaders require a performance profiling gate: measure frame time on min-spec hardware before accepting. AI shaders often include unnecessary texture samples or branching.

**AI_CODE-05:** AI code refactoring of existing systems ("Composer" agents) must be treated as a full rewrite — full regression test suite required, not just spot review.

**AI_CODE-06:** Godot GDScript AI generation is PRIMARY; Unity DOTS/ECS and UE C++ reflection macro code is ASSISTED ONLY — these patterns are too error-prone for AI primary authorship in 2026.

---

### 4. NEURAL PHYSICS GUARDRAILS (CRITICAL)

**NEURAL_PHYS-01:** Neural physics approximators (collision predictors, neural cloth, etc.) MUST implement a confidence threshold with automatic fallback to traditional solvers. If confidence <0.9, use traditional physics. Log all fallback events for review.

**NEURAL_PHYS-02:** NEVER use neural physics for gameplay-critical collision (player movement, hit detection, projectile physics). Neural approximators are allowed ONLY for ambient/destructible debris, cloth, hair, and VFX particles.

**NEURAL_PHYS-03:** Neural fluid simulation (FluidNet, Neural ADMM) requires a stability validation pass: run 10-minute burn-in test checking for divergence, energy conservation violations, or particle explosions. If any occur, reject and use traditional SPH/PIC.

**NEURAL_PHYS-04:** NeuralVDB / neural volumetrics are allowed for environmental VFX (smoke, clouds) but PROHIBITED for gameplay-relevant volumes (poison gas zones, visibility fog affecting stealth mechanics) unless accuracy is verified against traditional VDB reference.

**NEURAL_PHYS-05:** GNN-based hair/fur simulation must have a LOD0 traditional constraint-solver fallback for close-up camera shots. Neural hair is acceptable for distance shots and background NPCs only.

**NEURAL_PHYS-06:** After any neural physics integration, run a 24-hour automated stress test with random object spawning/destruction. Catastrophic simulation errors (memory leaks, NaN positions, infinite loops) = automatic rollback.

---

### 5. AI QA/TESTING/BALANCE GUARDRAILS

**AI_QA-01:** AI-suggested bug fixes from automated crash triage or stack-trace analysis are SUGGESTIONS ONLY. Human developer must review, reproduce, and approve before applying. AI patch acceptance rate must not exceed 40% without escalation review.

**AI_QA-02:** Agentic playtesting (autonomous AI testers) must have their bug reports validated by a second AI agent with different architecture before human triage. Single-agent reports can hallucinate exploits or false positives.

**AI_QA-03:** Visual regression detection (perceptual hash / CLIP diff) must have per-scene tolerance config: UI = pixel-perfect (0% tolerance), backgrounds = 5% tolerance, lighting = 2% tolerance. Global tolerance causes either false alarms or missed regressions.

**AI_QA-04:** AI-generated balance patches (economy tuning, encounter difficulty) must be A/B tested on a statistically significant player cohort (n≥1000 or 5% of DAU, whichever is larger) before full rollout. NEVER global-ship AI balance changes.

**AI_QA-05:** AI performance advisors (NVIDIA Nsight AI, etc.) recommendations must be validated with actual profiling on min-spec hardware. AI advisors predict; hardware measures. Predictions without measurements are PROTOTYPE-ONLY.

**AI_QA-06:** AI loot/balance LLMs predicting "meta-breaking combos" must output their reasoning trace. Human designer reviews the trace, not just the conclusion. Black-box balance predictions are prohibited.

---

### 6. RUNTIME INFERENCE GUARDRAILS (CRITICAL)

**RUNTIME_INF-01:** ALL ONNX models loaded at runtime must pass a validation gate: verify model structure (no dynamic shapes unless supported), check opset compatibility with engine plugin version, and run a 100-inference warmup to catch initialization crashes.

**RUNTIME_INF-02:** Runtime inference latency budgets: NPC dialogue = <200ms end-to-end (ASR→LLM→TTS); physics approximator = <1ms/frame; animation deformer = <0.5ms/frame; neural upscaling = <2ms/frame. Exceedance = disable feature or optimize model.

**RUNTIME_INF-03:** Memory budget for loaded neural models: Mobile ≤50MB total, Desktop ≤200MB, Console ≤500MB. Model quantization is MANDATORY for mobile (INT8 or INT4). Include model memory in total game memory budget.

**RUNTIME_INF-04:** Console builds using runtime neural inference MUST verify whitelist status before shipping. Unity Sentis and UE NNI are whitelisted; custom ONNX runtime plugins require platform certification (Sony/Microsoft). Godot GDExtension is UNCERTIFIED for console — use HTTP API fallback.

**RUNTIME_INF-05:** Cloud inference (LLM NPCs, cloud ACE) must have an offline fallback. If network drops, NPCs revert to scripted dialogue trees or behavior trees. NEVER hard-require cloud connectivity for core gameplay.

**RUNTIME_INF-06:** Multi-agent LLM orchestration (director LLM managing scene pacing + NPC LLMs) must have an agent-count cap: ≤10 concurrent LLM agents per scene on desktop, ≤3 on console, ≤1 on mobile. Exceedance = queue or downgrade to simpler AI.

---

### 7. AI NPC & DIALOGUE GUARDRAILS (CRITICAL)

**AI_NPC-01:** ALL LLM-driven NPCs must have a content filter/guardrail layer (NeMo Guardrails, Inworld safety, or equivalent) between the LLM and player. Filter must block: out-of-lore responses, real-world political/religious opinions, self-harm references, and attempts to break the fourth wall by discussing the game as a simulation.

**AI_NPC-02:** NPC memory/persistence systems (Inworld, Emergence SDK) must have data retention limits: conversation logs ≤30 days, belief states reset between play sessions unless explicitly opted-in. GDPR/CCPA compliance required.

**AI_NPC-03:** Dynamic quest generation via AI must produce quests that fit within the game's established lore and mechanics. AI-generated quests require a lore-consistency review before deployment. Prohibited: quests that introduce new factions, magic systems, or major plot points without human writer approval.

**AI_NPC-04:** Emergent storytelling (AI-driven plot evolution) must have a "narrative safety rail" — a human-authored plot spine that AI cannot alter. AI fills in dialogue and side-quests; humans control main story beats, character deaths, and endings.

**AI_NPC-05:** Multi-agent LLM orchestration (dozens of concurrent NPC agents) requires a scene-complexity cap. If >10 NPCs with active LLM agents in a scene, downgrade background NPCs to simpler state machines. Priority LLM access: quest-critical NPCs > merchants > ambient crowd.

**AI_NPC-06:** AI NPC dialogue must include an "off-ramp" — a scripted fallback response tree that triggers if the LLM fails, times out, or generates inappropriate content. Players should never see a raw error message or silence.

---

### 8. WORLD/LEVEL GENERATION GUARDRAILS

**WORLD_GEN-01:** AI-generated levels must pass a "completeness check" — verify that all areas are reachable, all objectives are completable, and no soft-locks exist. Use automated pathfinding validation on every generated level before human review.

**WORLD_GEN-02:** Procedural content (dungeons, maps) must have a "human curation gate" — AI generates 10x candidates, human picks the best 1-2. Never ship AI-first procedural content without human selection.

**WORLD_GEN-03:** AI-generated terrain/heightmaps must be checked for "impossible geometry" — vertical walls >45° without climbing mechanics, underwater caves without swimming, etc. Validate against player capability matrix.

---

### 9. BUSINESS & VENDOR GUARDRAILS

**VENDOR-01:** Before committing to any AI tool for asset generation, verify export format compatibility with target engine AND target platform. Tool switching mid-project costs 2-4 weeks. Preferred: tools with API access (Rodin, Meshy) for pipeline automation.

**VENDOR-02:** Maintain a "tool exit strategy" for every AI vendor — can you export all assets in standard formats if the vendor shuts down or changes pricing? Vendor lock-in is a project risk. Open-source alternatives (ComfyUI, llama.cpp) must be documented for critical tools.

**VENDOR-03:** AI tool subscription costs must be tracked in project budget. Monthly tool stack should not exceed $200/month for indie projects, $2000/month for AA studios without CFO approval.

---

### 10. LEGAL/ETHICAL ENHANCEMENTS

**LEGAL-01:** BEFORE using any AI-generated asset commercially, document the generation tool, date, and prompt. This audit trail protects against future copyright challenges. Store in `assets/ai-attribution/` with every AI-generated file.

**LEGAL-02:** Monitor active lawsuits against AI training data. As of May 2026, 47 cases are pending. If a lawsuit targets a tool you use, immediately flag all assets from that tool for human review and consider switching vendors.

**LEGAL-03:** AI disclosure requirements vary by platform (Steam = optional transparency preferred; console = may require certification disclosure). Research platform-specific rules before shipping.

**LEGAL-04:** NEVER use AI to generate likenesses of real people, trademarks, or copyrighted characters. This is a legal and PR risk regardless of tool TOS. When in doubt, use original design prompts only.

---

### 11. SOCIAL/ETHICAL GUARDRAILS

**SOCIAL-01:** "AI fatigue" is real — players reject obviously generated content. Hero assets, key characters, and story-critical environments must have visible human craft. AI is for props, backgrounds, and rapid iteration — not the heart of the game.

**SOCIAL-02:** If working with human artists, establish clear boundaries: AI handles blockout/iteration, artists handle hero assets and final polish. Never present AI-generated work as human-crafted. Transparency builds trust.

**SOCIAL-03:** Community backlash against AI can sink a game. Before public reveal, prepare an AI usage statement explaining what was AI-assisted vs human-crafted. Proactive transparency > reactive damage control.

---

## INTEGRATION PLAN

These 40+ new rules should be integrated into `3D_GAME_DEVELOPMENT.md` as new major sections:

1. Add "NEURAL RENDERING GUARDRAILS" section after "Performance Budgets"
2. Add "AI ANIMATION & MOTION GUARDRAILS" after Asset Pipeline Rules
3. Add "AI CODE GENERATION GUARDRAILS" as a dedicated section
4. Add "NEURAL PHYSICS GUARDRAILS" after AI Code section
5. Add "AI QA / TESTING / BALANCE GUARDRAILS" as new section
6. Add "RUNTIME INFERENCE GUARDRAILS" after Engine Conventions
7. Add "AI NPC & DIALOGUE GUARDRAILS" as new section
8. Add "WORLD GENERATION GUARDRAILS" after AI NPC section
9. Expand "Legal & Ethical Guardrails" with VENDOR, LEGAL, SOCIAL subsections
10. Update version to 1.2.0

---

*Draft compiled from parallel subagent analysis of Hermes 2026 AI Dossier*
*Ready for Roger review before branch commit*
