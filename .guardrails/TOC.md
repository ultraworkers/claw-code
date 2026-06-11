# Template Contents (Table of Contents)

> Complete list of all files and directories in the Agent Guardrails Template.

---

## Quick Navigation

- [Root Files](#root-files)
- [Documentation Directory](#documentation-directory)
- [GitHub Integration](#github-integration)
- [Examples Directory](#examples-directory)

---

## Root Files

| File | Lines | Required? | Purpose |
|------|-------|-----------|---------|
| **README.md** | ~150 | YES | Project overview and quick start |
| **QUICK_SETUP.md** | ~270 | **YES** | **5-minute setup guide** ⭐ |
| **PROMPTING_GUIDE.md** | ~500 | **YES** | **Master prompting techniques** ⭐ |
| **INDEX_MAP.md** | 170 | YES | Master navigation - find docs by keyword |
| **HEADER_MAP.md** | 408 | YES | Section headers with line numbers |
| **CLAUDE.md** | 29 | Recommended | Optimized context for Claude Code CLI |
| **.claudeignore** | ~20 | Recommended | Token-saving ignore rules |
| **CHANGELOG.md** | 238 | YES | Release notes archive |
| **LICENSE** | - | YES | BSD-3-Clause license |
| **.gitignore** | ~100 | Recommended | Common ignore patterns |

---

## Documentation Directory

### Root Documentation (`docs/`)

| File | Lines | Sections | Purpose |
|------|-------|----------|---------|
| **AGENT_GUARDRAILS.md** | 267 | 13 | Core safety protocols (MANDATORY) |
| **HOW_TO_APPLY.md** | 432 | 5 | How to apply template with example prompts |
| **AGENTS_AND_SKILLS_SETUP.md** | ~200 | 6 | Setup guide for Claude Code/OpenCode |
| **CLCODE_INTEGRATION.md** | ~250 | 7 | Claude Code skills and hooks integration |
| **OPENCODE_INTEGRATION.md** | ~300 | 8 | OpenCode agents and skills integration |
| **PYTHON_TO_GO_MIGRATION.md** | ~350 | 11 | Python to Go migration guide (v2.6.0) |

### Workflows (`docs/workflows/`)

| File | Lines | Key Sections | Purpose |
|------|-------|--------------|---------|
| **INDEX.md** | 126 | 5 | Workflow navigation hub |
| **AGENT_EXECUTION.md** | 280 | 6 | Execution protocol and rollback |
| **AGENT_ESCALATION.md** | 300 | 6 | Audit requirements and escalation |
| **AGENT_REVIEW_PROTOCOL.md** | 605 | 12 | Post-work agent/LLM review |
| **TESTING_VALIDATION.md** | 303 | 9 | Validation protocols and checks |
| **COMMIT_WORKFLOW.md** | 328 | 8 | Commit timing and message format |
| **DOCUMENTATION_UPDATES.md** | ~250 | 5 | Post-sprint doc updates |
| **GIT_PUSH_PROCEDURES.md** | 323 | 8 | Push safety and verification |
| **BRANCH_STRATEGY.md** | ~200 | 6 | Git branching conventions |
| **CODE_REVIEW.md** | 348 | 7 | Code review and escalation |
| **MCP_CHECKPOINTING.md** | ~280 | 7 | MCP server checkpointing |

**Total:** 11 workflow documents (INDEX.md + 10 guides)

### Standards (`docs/standards/`)

| File | Lines | Key Sections | Purpose |
|------|-------|--------------|---------|
| **INDEX.md** | 89 | 4 | Standards navigation hub |
| **TEST_PRODUCTION_SEPARATION.md** | 558 | 12 | Test/production isolation (MANDATORY) |
| **PROJECT_CONTEXT_TEMPLATE.md** | 376 | 9 | Project Bible - stack, style, forbidden patterns |
| **ADVERSARIAL_TESTING.md** | 510 | 12 | Breaker agent, fuzz testing, attack checklists |
| **DEPENDENCY_GOVERNANCE.md** | 483 | 8 | Package allow-list, license compliance |
| **INFRASTRUCTURE_STANDARDS.md** | 546 | 11 | IaC, Terraform, no-ClickOps, drift detection |
| **OPERATIONAL_PATTERNS.md** | 667 | 12 | Health checks, circuit breakers, retry, rate limiting |
| **MODULAR_DOCUMENTATION.md** | 330 | 8 | 500-line max rule and structure |
| **LOGGING_PATTERNS.md** | ~280 | 7 | Array-based logging format |
| **LOGGING_INTEGRATION.md** | ~250 | 7 | External logging hooks |
| **API_SPECIFICATIONS.md** | ~300 | 6 | OpenAPI vs OpenSpec guidance |

**Total:** 11 standards documents (INDEX.md + 10 guides)

### Sprints (`docs/sprints/`)

| File | Lines | Key Sections | Purpose |
|------|-------|--------------|---------|
| **INDEX.md** | 31 | 3 | Sprint navigation hub |
| **SPRINT_TEMPLATE.md** | 515 | 15 | Task execution template |
| **SPRINT_GUIDE.md** | 270 | 9 | How to write sprints |

**Total:** 3 sprint documents

### 2026 Game Design & UI/UX (`docs/`)

| File | Path | Lines | Purpose |
|------|------|-------|---------|
| **2026_GAME_DESIGN.md** | `docs/game-design/` | ~350 | Game design guardrails, XR/VR comfort zones, platform budgets |
| **3D_GAME_DEVELOPMENT.md** | `docs/game-design/3d/` | 416 | 3D game development guardrails v1.0, engine-agnostic patterns |
| **3D_GUARDREL_PROPOSALS_V1.2.md** | `docs/game-design/3d/` | 201 | Proposed v1.2 additions from Hermes 2026 AI Dossier |
| **3D_MATHEMATICAL_FOUNDATIONS.md** | `docs/game-design/3d/` | 290 | Linear algebra, quaternions, collision math reference |
| **3D_MODULE_ARCHITECTURE.md** | `docs/game-design/3d/` | 237 | Module architecture for LLM-to-3D-engine bridging |
| **AI_DEBUGGABLE_3D_ARCHITECTURE.md** | `docs/game-design/3d/` | 302 | AI-debuggable patterns for autonomous 3D troubleshooting |
| **AI_DEV_2026_PART01_INTRO_AND_FOUNDATIONS.md** | `docs/game-design/` | 294 | Part 1 — Introduction & Foundations |
| **AI_DEV_2026_PART02_PROMPTING.md** | `docs/game-design/` | 216 | Part 2 — Prompt Engineering for Code |
| **AI_DEV_2026_PART03_CONTEXT_AND_ITERATION.md** | `docs/game-design/` | 239 | Part 3 — Context & Iterative Development |
| **AI_DEV_2026_PART04_QUALITY_AND_ARCHITECTURE.md** | `docs/game-design/` | 214 | Part 4 — Quality & Architecture |
| **AI_DEV_2026_PART05_LEGACY_AND_AGENTS.md** | `docs/game-design/` | 235 | Part 5 — Legacy Refactoring & Agent Paradigm |
| **AI_DEV_2026_PART06_BUILDING_AGENTS.md** | `docs/game-design/` | 254 | Part 6 — Building Agents & Tool Use |
| **AI_DEV_2026_PART07_MULTI_AGENT_SYSTEMS.md** | `docs/game-design/` | 388 | Part 7 — Multi-Agent Systems |
| **AI_DEV_2026_PART08_SECURITY_ETHICS_FUTURE.md** | `docs/game-design/` | 340 | Part 8 — Security, Ethics & Future |
| **AI_DEV_2026_PART09_APPENDICES_ABC.md** | `docs/game-design/` | 432 | Part 9 — Appendices A, B & C |
| **AI_DEV_2026_PART10_APPENDIX_D.md** | `docs/game-design/` | 411 | Part 10 — Appendix D: Complete MoA Reference |
| **HERMES_2026_PART01_INTRO_AND_EXECUTIVE.md** | `docs/game-design/3d/` | 59 | Part 1 — Introduction & Executive Summary |
| **HERMES_2026_PART02_ASSETS_AND_ENGINES.md** | `docs/game-design/3d/` | 213 | Part 2 — 3D Asset Generation & Engine Integration |
| **HERMES_2026_PART03_WORLD_AND_RENDERING.md** | `docs/game-design/3d/` | 102 | Part 3 — World Generation & Neural Rendering |
| **HERMES_2026_PART04_NPCS_AND_ANIMATION.md** | `docs/game-design/3d/` | 96 | Part 4 — NPCs, Dialogue & Animation |
| **HERMES_2026_PART05_CODE_AND_PHYSICS.md** | `docs/game-design/3d/` | 96 | Part 5 — Code Generation & Neural Physics |
| **HERMES_2026_PART06_QA_AND_BUSINESS.md** | `docs/game-design/3d/` | 122 | Part 6 — QA, Testing & Business Landscape |
| **HERMES_2026_PART07_LEGAL_AND_CASES.md** | `docs/game-design/3d/` | 127 | Part 7 — Legal, Ethics & Case Studies |
| **HERMES_2026_PART08_DEEP_DIVES_AND_FUTURE.md** | `docs/game-design/3d/` | 133 | Part 8 — Technology Deep-Dives & Future Outlook |
| **HERMES_2026_PART09_APPENDICES.md** | `docs/game-design/3d/` | 83 | Part 9 — Appendices |
| **2026_UI_UX_STANDARD.md** | `docs/ui-ux/` | ~350 | UI/UX component patterns, design tokens, responsive breakpoints |
| **ACCESSIBILITY_GUIDE.md** | `docs/accessibility/` | ~300 | WCAG 3.0+ conformance (Bronze/Silver/Gold), testing methods |
| **SPATIAL_COMPUTING_UI.md** | `docs/spatial/` | ~400 | XR/VR/AR layout patterns, comfort zones, latency requirements |
| **ETHICAL_ENGAGEMENT.md** | `docs/ethical/` | ~250 | Dark pattern taxonomy and prevention, ethical design principles |

**Total:** 12 documents covering Agent-GDUI-2026 capabilities

### AI-First Development & Safety (`docs/`)

| File | Path | Lines | Purpose |
|------|------|-------|---------|
| **AI_ASSISTED_DEV.md** | `docs/ai-dev/` | 326 | AI development patterns, vibe coding, decision matrix |
| **STATE_MANAGEMENT.md** | `docs/state/` | 303 | State architecture, client/server state, CRDTs |
| **GENERATIVE_ASSET_SAFETY.md** | `docs/generative/` | 332 | AI content disclosure, procedural generation safety |
| **MONETIZATION_GUARDRAILS.md** | `docs/monetization/` | 263 | IAP ethics, loot box transparency, virtual economy |
| **MULTIPLAYER_SAFETY.md** | `docs/multiplayer/` | 276 | Social safety, chat moderation, matchmaking |
| **ANALYTICS_ETHICS.md** | `docs/analytics/` | 302 | Analytics consent, data minimization, A/B testing |
| **CROSS_PLATFORM_DEPLOYMENT.md** | `docs/deployment/` | 259 | App store compliance, CI/CD, feature flags |

**Total:** 7 documents covering AI-first development guardrails

### Overall Documentation Summary

| Category | Documents | Total Lines |
|----------|-----------|-------------|
| Root docs | 7 | ~1,050 |
| Workflows | 11 | ~3,500 |
| Standards | 11 | ~4,400 |
| Sprints | 3 | ~816 |
| 2026 Game/UI/UX | 29 | ~7,150 |
| AI-First Development | 7 | ~2,061 |
| **TOTAL** | **68** | **~18,977** |

---

## GitHub Integration

### GitHub Root (`.github/`)

| File/Diretory | Purpose |
|--------------|---------|
| **SECRETS_MANAGEMENT.md** | GitHub Secrets setup and rotation guide |
| **PULL_REQUEST_TEMPLATE.md** | PR template with AI attribution |
| **ISSUE_TEMPLATE/bug_report.md** | Bug report template |

### GitHub Workflows (`.github/workflows/`)

| File | Purpose |
|------|---------|
| **secret-validation.yml** | Validate no secrets in commits |
| **documentation-check.yml** | Validate documentation format |
| **guardrails-lint.yml** | Enforce guardrails compliance |

---

## Examples Directory

### Language-Specific Examples (`examples/`)

| Directory | Files | Lines | Language | Purpose |
|-----------|-------|-------|----------|---------|
| **examples/** | 53 | ~2,000 | Mixed | Guardrails implementation examples |
| **go/** | 7 | ~300 | Go 1.19+ | Environment-specific config |
| **java/** | 15 | ~500 | Java 11+ | ConfigLoader with validation |
| **python/** | 8 | ~350 | Python 3.8+ | YAML config with type hints |
| **ruby/** | 7 | ~300 | Ruby 3.0+ | BDD-style testing |
| **rust/** | 4 | ~200 | Rust 1.70+ | Type-safe Serde config |
| **typescript/** | 10 | ~350 | TypeScript 5+ | Modular logging hooks |
| **scala/functional-ui/** | ~10 | ~400 | Scala 3.4+ | Functional composition, type-safe CSS, DDA telemetry |
| **r/game-analytics/** | ~8 | ~350 | R 4.3+ | ggplot2 4.0+, Shiny 2.0+, ethics auditing |
| **flutter/cross-platform/** | 4 | ~350 | Dart/Flutter | Ethical widgets, accessibility wrappers, guardrail config |
| **gdscript/godot-game/** | 4 | ~300 | GDScript | XR comfort zones, ethical UI, accessibility manager |

### Examples Structure

Each language example includes:
- Source code demonstrating guardrails patterns
- Tests validating separation requirements
- Environment-specific configuration files
- Build/test instructions
- Language-specific README

---

## Document Purpose Quick Reference

| Document | Primary Audience | Key Sections |
|----------|------------------|--------------|
| **AGENT_GUARDRAILS.md** | All AI agents | Four Laws, Pre-Execution Checklist, Forbidden Actions |
| **TEST_PRODUCTION_SEPARATION.md** | All AI agents | Three Laws, Blocking Violations, Uncertainty Protocol |
| **AGENT_EXECUTION.md** | All AI agents | Task Flow, Rollback, Error Handling, Three Strikes Rule |
| **AGENT_ESCALATION.md** | All AI agents | Audit Requirements, When to Escalate |
| **AGENT_REVIEW_PROTOCOL.md** | All AI agents | Dual-Agent Review, Cross-Model Review, Review Package |
| **PROJECT_CONTEXT_TEMPLATE.md** | Project setup | Tech Stack, Style Guide, Forbidden Patterns |
| **ADVERSARIAL_TESTING.md** | Security testing | Breaker Agent, Attack Vectors, Fuzz Testing |
| **DEPENDENCY_GOVERNANCE.md** | All AI agents | Package Allow-List, Forbidden Packages |
| **INFRASTRUCTURE_STANDARDS.md** | DevOps/IaC | Terraform, Drift Detection, No-ClickOps |
| **OPERATIONAL_PATTERNS.md** | Service developers | Health Checks, Circuit Breakers, Retry, Rate Limiting |
| **HOW_TO_APPLY.md** | All agents | 4 Options with ready-to-use prompts |
| **INDEX_MAP.md** | All agents | Find docs by keyword (60-80% token savings) |
| **HEADER_MAP.md** | All agents | Section-level lookup for targeted reading |
| **SPRINT_TEMPLATE.md** | Agents creating tasks | Complete task execution format |

---

## File Size Summary

| Category | Files | Min Lines | Max Lines | Average Lines |
|----------|-------|-----------|-----------|--------------|
| Root | 7 | 29 | 408 | ~150 |
| docs/ | 3 | 238 | 432 | ~333 |
| docs/workflows/ | 11 | ~200 | ~605 | ~320 |
| docs/standards/ | 11 | ~250 | ~667 | ~400 |
| docs/sprints/ | 3 | 31 | 515 | ~272 |
| .github/ | 3 | ~50 | ~150 | ~100 |
| examples/ | 53 | ~30 | ~150 | ~40 |
| **TOTAL** | **91** | **29** | **667** | **~110** |

---

## Compliance Status

### 500-Line Maximum Compliance

All documents comply with the 500-line maximum rule:

| Document | Lines | Status |
|----------|-------|--------|
| README.md | ~150 | ✅ |
| AGENT_GUARDRAILS.md | 267 | ✅ |
| HOW_TO_APPLY.md | 432 | ✅ |
| TEST_PRODUCTION_SEPARATION.md | 558 | ⚠️ Exceeds - needs split |
| AGENT_REVIEW_PROTOCOL.md | 605 | ⚠️ Exceeds - needs split |
| INFRASTRUCTURE_STANDARDS.md | 546 | ⚠️ Exceeds - needs split |
| OPERATIONAL_PATTERNS.md | 667 | ⚠️ Exceeds - needs split |
| All other workflows | ~280 average | ✅ |
| All other standards | ~380 average | ✅ |
| All sprints | ~270 average | ✅ |

**Note:** 4 documents exceed the 500-line limit. They will be split in a future release.

---

## Quick Lookup

**I need to...** → **Read this document:**

| Task | Document | Section |
|------|----------|---------|
| Find a document by keyword | INDEX_MAP.md | Quick Lookup Table |
| Understand safety rules | AGENT_GUARDRAILS.md | CORE PRINCIPLES (line 39) |
| Apply to existing repo | HOW_TO_APPLY.md | Option A (line 25) |
| Use example prompts | HOW_TO_APPLY.md | Option B (line 77) |
| Verify before committing | TESTING_VALIDATION.md | Post-Edit Validation (line 38) |
| Commit between to-dos | COMMIT_WORKFLOW.md | After Each To-Do (line 32) |
| Rollback changes | AGENT_EXECUTION.md | Rollback Procedures (line 51) |
| Review code | CODE_REVIEW.md | Self-Review Checklist (line 15) |
| Separate test/production | TEST_PRODUCTION_SEPARATION.md | CORE MANDATORY RULES (line 18) |
| Create task document | SPRINT_TEMPLATE.md | STEP-BY-STEP EXECUTION (line 91) |
| Write documentation | MODULAR_DOCUMENTATION.md | The 500-Line Rule (line 15) |
| Build game interfaces | 2026_GAME_DESIGN.md | XR/VR Comfort, Platform Rules |
| Design UI components | 2026_UI_UX_STANDARD.md | Components, Design Tokens |
| Ensure accessibility | ACCESSIBILITY_GUIDE.md | WCAG 3.0+ Compliance |
| Build XR/VR/AR UIs | SPATIAL_COMPUTING_UI.md | Comfort Zones, Latency |
| Prevent dark patterns | ETHICAL_ENGAGEMENT.md | Dark Pattern Taxonomy |

---

## File Templates

All files follow these conventions:

- **Line limit:** 500 lines (except TEST_PRODUCTION_SEPARATION.md pending split)
- **Markdown:** CommonMark with GitHub extensions
- **Headers:** Level 1 (H1) for title, Level 2 (H2) for sections
- **Code blocks:** Backtick fences with language identifier
- **Tables:** GitHub-flavored Markdown tables
- **Lists:** Bullet and numbered lists for hierarchy

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-05-12
**Total Files:** 120
**Total Lines:** ~17,000
