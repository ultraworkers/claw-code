# Documentation Index Map

> **READ THIS FIRST** - Find what you need without loading full documents.
> Estimated token savings: 60-80% when using targeted lookups.

---

## Quick Lookup Table

| Keyword | Document | Path | Purpose |
|---------|----------|------|---------|
| quick-setup | QUICK_SETUP.md | ./ | **5-minute setup guide** ⭐ |
| prompting | PROMPTING_GUIDE.md | ./ | **Master prompting techniques** ⭐ |
| toc | TOC.md | ./ | Complete template contents and file listing |
| safety | AGENT_GUARDRAILS.md | docs/ | Mandatory safety protocols |
| test-prod | TEST_PRODUCTION_SEPARATION.md | docs/standards/ | Test/production isolation (MANDATORY) |
| execution | AGENT_EXECUTION.md | docs/workflows/ | Standard execution protocol |
| escalation | AGENT_ESCALATION.md | docs/workflows/ | Audit & escalation procedures |
| how-to-apply | HOW_TO_APPLY.md | docs/ | How to apply guardrails to repos |
| commit | COMMIT_WORKFLOW.md | docs/workflows/ | When/how to commit |
| push | GIT_PUSH_PROCEDURES.md | docs/workflows/ | Push safety procedures |
| branch | BRANCH_STRATEGY.md | docs/workflows/ | Git branching conventions |
| rollback | ROLLBACK_PROCEDURES.md | docs/workflows/ | Recovery and undo |
| test | TESTING_VALIDATION.md | docs/workflows/ | Validation protocols |
| review | CODE_REVIEW.md | docs/workflows/ | Code review process |
| checkpoint | MCP_CHECKPOINTING.md | docs/workflows/ | MCP auto-checkpoint |
| docs | DOCUMENTATION_UPDATES.md | docs/workflows/ | Post-sprint doc updates |
| logging | LOGGING_PATTERNS.md | docs/standards/ | Array-based logging |
| hooks | LOGGING_INTEGRATION.md | docs/standards/ | External logging hooks |
| modular | MODULAR_DOCUMENTATION.md | docs/standards/ | 500-line rule |
| api | API_SPECIFICATIONS.md | docs/standards/ | OpenAPI/OpenSpec guidance |
| secrets | SECRETS_MANAGEMENT.md | .github/ | GitHub Secrets setup |
| examples | examples/ | examples/ | Multi-language implementation examples |
| scala-examples | examples/scala/functional-ui/ | Scala 3.4+ functional UI, type-safe CSS, DDA telemetry |
| r-examples | examples/r/game-analytics/ | R ggplot2 4.0+, Shiny 2.0+, ethics auditing, retention analysis |
| regression-examples | regression-prevention/ | examples/regression-prevention/ | Practical regression prevention examples |
| sprint | SPRINT_TEMPLATE.md | docs/sprints/ | Sprint task template |
| sprint-guide | SPRINT_GUIDE.md | docs/sprints/ | How to write sprints |
| validation | SPRINT_TEMPLATE.md | docs/sprints/ | Completion gate & validation loop |
| completion | SPRINT_TEMPLATE.md | docs/sprints/ | Pre-completion checklist |
| context | PROJECT_CONTEXT_TEMPLATE.md | docs/standards/ | Project Bible - stack constraints, style guide |
| adversarial | ADVERSARIAL_TESTING.md | docs/standards/ | Breaker agent, fuzz testing, attack checklists |
| agent-review | AGENT_REVIEW_PROTOCOL.md | docs/workflows/ | Post-work verification by another agent |
| dependencies | DEPENDENCY_GOVERNANCE.md | docs/standards/ | Package allow-list, forbidden packages |
| infrastructure | INFRASTRUCTURE_STANDARDS.md | docs/standards/ | IaC, Terraform, drift detection |
| operational | OPERATIONAL_PATTERNS.md | docs/standards/ | Health checks, circuit breakers, retry |
| retry | AGENT_EXECUTION.md | docs/workflows/ | Three Strikes Rule, retry limits |
| scope-freeze | SPRINT_TEMPLATE.md | docs/sprints/ | Scope Freeze Protocol |
| deployment | DEPLOYMENT_GUIDE.md | mcp-server/ | MCP server deployment instructions (critical fixes) |
| schema-error | DEPLOYMENT_GUIDE.md | mcp-server/ | Fix schema validation error (guardrail-mcp → guardrail_mcp) |
| postgres-perm | DEPLOYMENT_GUIDE.md | mcp-server/ | Fix postgres permission errors (user 70:70) |
| container-networking | DEPLOYMENT_GUIDE.md | mcp-server/ | Pod networking for container communication |
| skills | AGENTS_AND_SKILLS_SETUP.md | docs/ | Setup agents and skills for all platforms |
| claude-code | CLCODE_INTEGRATION.md | docs/ | Claude Code skills and hooks integration |
| opencode | OPCODE_INTEGRATION.md | docs/ | OpenCode agents and skills integration |
| cursor | CURSOR_INTEGRATION.md | docs/ | Cursor rules and configuration |
| copilot | CLCODE_INTEGRATION.md | docs/ | GitHub Copilot instructions (see Claude Code) |
| cody | CLCODE_INTEGRATION.md | docs/ | Cody context configuration (see Claude Code) |
| mcp-server | MCP_SERVER_PLAN.md | docs/plans/ | MCP server implementation plan |
| mcp-api | API.md | mcp-server/ | MCP server REST API documentation |
| mcp-changelog | CHANGELOG.md | mcp-server/ | MCP server version history |
| guardrail-platform | MCP_SERVER_PLAN.md | docs/plans/ | Guardrail enforcement platform |
| team-tools | TEAM_TOOLS.md | docs/ | Team layout management MCP tools reference (Go implementation) |
| team-structure | TEAM_STRUCTURE.md | docs/ | 12-team enterprise structure documentation |
| python-migration | PYTHON_TO_GO_MIGRATION.md | docs/ | Python to Go migration guide for developers |
| go-migration | PYTHON_TO_GO_MIGRATION.md | docs/ | Python to Go migration guide for developers |
| team-cli | cmd/team-cli/README.md | cmd/team-cli/ | Team management CLI tool |
| phase-gate | TEAM_TOOLS.md | docs/ | Phase transition requirements and deliverables |
| aider | CLCODE_INTEGRATION.md | docs/ | Aider YAML configuration (see Claude Code) |
| continue | CLCODE_INTEGRATION.md | docs/ | Continue IDE configuration (see Claude Code) |
| windsurf | CLCODE_INTEGRATION.md | docs/ | Windsurf rules configuration (see Claude Code) |
| generic | GENERIC_LLM_INTEGRATION.md | docs/ | Generic/local LLM configuration guide |
| setup | setup_agents.py | scripts/ | CLI tool to generate agent configurations |
| regression | REGRESSION_PREVENTION.md | docs/workflows/ | Bug tracking and regression prevention protocol |
| failure-registry | failure-registry.jsonl | .guardrails/ | Append-only bug database (JSONL format) |
| pre-work-check | pre-work-check.md | .guardrails/ | MANDATORY pre-work regression checklist |
| log-failure | log_failure.py | scripts/ | CLI tool to log bugs to failure registry |
| regression-check | regression_check.py | scripts/ | Pre-commit regression pattern scanner |
| prevention-rules | pattern-rules.json | .guardrails/prevention-rules/ | Regex patterns to prevent regressions |
| game-design | 2026_GAME_DESIGN.md | docs/game-design/ | Agent-GDUI-2026 game design guardrails, XR/VR comfort zones |
| 3d-guardrails | 3D_GAME_DEVELOPMENT.md | docs/game-design/3d/ | 3D game development guardrails v1.0, engine-agnostic |
| 3d-proposals | 3D_GUARDREL_PROPOSALS_V1.2.md | docs/game-design/3d/ | Proposed v1.2 additions from Hermes 2026 AI Dossier |
| 3d-math | 3D_MATHEMATICAL_FOUNDATIONS.md | docs/game-design/3d/ | Linear algebra, quaternions, collision math reference |
| 3d-architecture | 3D_MODULE_ARCHITECTURE.md | docs/game-design/3d/ | Module architecture for LLM-to-3D-engine bridging |
| ai-debuggable | AI_DEBUGGABLE_3D_ARCHITECTURE.md | docs/game-design/3d/ | AI-debuggable 3D patterns for autonomous troubleshooting |
| ai-2026-guide | AI_DEV_2026_PART01_INTRO_AND_FOUNDATIONS.md | docs/game-design/ | AI-Powered Development 2026: 10-part comprehensive guide series |
| ai-2026-prompting | AI_DEV_2026_PART02_PROMPTING.md | docs/game-design/ | Part 2 — Prompt Engineering for Code |
| ai-2026-context | AI_DEV_2026_PART03_CONTEXT_AND_ITERATION.md | docs/game-design/ | Part 3 — Context & Iterative Development |
| ai-2026-quality | AI_DEV_2026_PART04_QUALITY_AND_ARCHITECTURE.md | docs/game-design/ | Part 4 — Quality & Architecture |
| ai-2026-legacy | AI_DEV_2026_PART05_LEGACY_AND_AGENTS.md | docs/game-design/ | Part 5 — Legacy Refactoring & Agent Paradigm |
| ai-2026-building | AI_DEV_2026_PART06_BUILDING_AGENTS.md | docs/game-design/ | Part 6 — Building Agents & Tool Use |
| ai-2026-multi | AI_DEV_2026_PART07_MULTI_AGENT_SYSTEMS.md | docs/game-design/ | Part 7 — Multi-Agent Systems |
| ai-2026-security | AI_DEV_2026_PART08_SECURITY_ETHICS_FUTURE.md | docs/game-design/ | Part 8 — Security, Ethics & Future |
| ai-2026-appendices | AI_DEV_2026_PART09_APPENDICES_ABC.md | docs/game-design/ | Part 9 — Appendices A, B & C |
| ai-2026-moa | AI_DEV_2026_PART10_APPENDIX_D.md | docs/game-design/ | Part 10 — Appendix D: Complete MoA Reference |
| hermes-dossier | HERMES_2026_PART01_INTRO_AND_EXECUTIVE.md | docs/game-design/3d/ | AI in 3D Game Development 2026: 9-part intelligence dossier |
| hermes-assets | HERMES_2026_PART02_ASSETS_AND_ENGINES.md | docs/game-design/3d/ | Part 2 — 3D Asset Generation & Engine Integration |
| hermes-world | HERMES_2026_PART03_WORLD_AND_RENDERING.md | docs/game-design/3d/ | Part 3 — World Generation & Neural Rendering |
| hermes-npcs | HERMES_2026_PART04_NPCS_AND_ANIMATION.md | docs/game-design/3d/ | Part 4 — NPCs, Dialogue & Animation |
| hermes-code | HERMES_2026_PART05_CODE_AND_PHYSICS.md | docs/game-design/3d/ | Part 5 — Code Generation & Neural Physics |
| hermes-qa | HERMES_2026_PART06_QA_AND_BUSINESS.md | docs/game-design/3d/ | Part 6 — QA, Testing & Business Landscape |
| hermes-legal | HERMES_2026_PART07_LEGAL_AND_CASES.md | docs/game-design/3d/ | Part 7 — Legal, Ethics & Case Studies |
| hermes-future | HERMES_2026_PART08_DEEP_DIVES_AND_FUTURE.md | docs/game-design/3d/ | Part 8 — Technology Deep-Dives & Future Outlook |
| hermes-appendices | HERMES_2026_PART09_APPENDICES.md | docs/game-design/3d/ | Part 9 — Appendices |
| ui-ux | 2026_UI_UX_STANDARD.md | docs/ui-ux/ | UI/UX component standards, design tokens, interaction states |
| accessibility | ACCESSIBILITY_GUIDE.md | docs/accessibility/ | WCAG 3.0+ compliance, conformance levels, testing methods |
| spatial | SPATIAL_COMPUTING_UI.md | docs/spatial/ | XR/VR/AR UI patterns, comfort zones, latency requirements |
| ethical | ETHICAL_ENGAGEMENT.md | docs/ethical/ | Dark pattern prevention, ethical design principles |
| semantic-rules | semantic-rules.json | .guardrails/prevention-rules/ | AST-based prevention rules |
| extracted-rules | extracted-rules.json | .guardrails/prevention-rules/ | Rules extracted from AGENT_GUARDRAILS.md |
| bug-fix | REGRESSION_PREVENTION.md | docs/workflows/ | Requirements for bug fixes (regression tests) |
| known-bugs | failure-registry.jsonl | .guardrails/ | Active/resolved/deprecated bug history |
| four-laws | four-laws.md | skills/shared-prompts/ | Canonical Four Laws of Agent Safety |
| halt-conditions | halt-conditions.md | skills/shared-prompts/ | When to stop and ask for help |
| sprint-001 | SPRINT_001_MCP_GAP_IMPLEMENTATION.md | docs/sprints/ | Sprint: MCP Gap Implementation |
| sprint-002 | SPRINT_002_WEB_UI_IMPLEMENTATION.md | docs/sprints/ | Sprint: Web UI Implementation |
| sprint-003 | SPRINT_003_DOCUMENTATION_PARITY.md | docs/sprints/ | Sprint: Documentation Parity (this sprint) |
| rules-from-md | RULES_FROM_MD.md | docs/ | Extracting prevention rules from markdown |
| rules-index | RULES_INDEX_MAP.md | docs/ | Master index of all prevention rules |
| mcp-tools | MCP_TOOLS_REFERENCE.md | docs/ | MCP validation tools documentation |
| rule-patterns | RULE_PATTERNS_GUIDE.md | docs/ | Pattern authoring guide |
| ai-dev | AI_ASSISTED_DEV.md | docs/ai-dev/ | AI-assisted development patterns, vibe coding, decision matrix |
| state | STATE_MANAGEMENT.md | docs/state/ | State architecture patterns, client/server state, CRDTs |
| generative | GENERATIVE_ASSET_SAFETY.md | docs/generative/ | Generative asset safety, C2PA metadata, content filtering |
| monetization | MONETIZATION_GUARDRAILS.md | docs/monetization/ | IAP ethics, loot box transparency, virtual economy |
| multiplayer | MULTIPLAYER_SAFETY.md | docs/multiplayer/ | Multiplayer safety, chat moderation, matchmaking fairness |
| analytics | ANALYTICS_ETHICS.md | docs/analytics/ | Analytics ethics, consent tiers, A/B testing, data minimization |
| deployment | CROSS_PLATFORM_DEPLOYMENT.md | docs/deployment/ | Cross-platform deployment, app store compliance, CI/CD |
| vibe-coding | vibe-coding.md | skills/shared-prompts/ | Canonical vibe coding principles for rapid AI development |
| flutter-examples | examples/flutter/cross-platform/ | examples/flutter/ | Flutter guardrails: ethical widgets, accessibility wrappers |
| godot-examples | examples/gdscript/godot-game/ | examples/gdscript/ | Godot GDScript: comfort zones, ethical UI, accessibility |

---

## Document Summaries

| Document | Purpose (one line) | When to Use |
|----------|-------------------|-------------|
| **TOC.md** | Complete template contents and file listing | When exploring full template |
| **AGENT_GUARDRAILS.md** | Core safety protocols (mandatory) | Before ANY code changes |
| **RULES_FROM_MD.md** | Extracting prevention rules from markdown | When working with MCP rules |
| **RULES_INDEX_MAP.md** | Master index of all prevention rules | When searching for specific prevention rules |
| **MCP_TOOLS_REFERENCE.md** | MCP validation tools documentation | When using MCP validation tools |
| **RULE_PATTERNS_GUIDE.md** | Pattern authoring guide | When writing new prevention rules |
| **TEST_PRODUCTION_SEPARATION.md** | Test/production isolation standards (MANDATORY) | Before ANY deployment |
| **AGENT_EXECUTION.md** | Execution protocol and rollback procedures | During task execution |
| **AGENT_ESCALATION.md** | Audit requirements and escalation procedures | When uncertain or errors occur |
| **HOW_TO_APPLY.md** | how to apply guardrails to repositories | When setting up agent guardrails |
| **TESTING_VALIDATION.md** | Validation functions and git diff verification | Before committing changes |
| **COMMIT_WORKFLOW.md** | Guidelines for commits between to-dos | After completing each task |
| **GIT_PUSH_PROCEDURES.md** | Pre-push checklist and safety rules | Before pushing to remote |
| **BRANCH_STRATEGY.md** | Git branching conventions (feature/hotfix/release) | When creating branches |
| **ROLLBACK_PROCEDURES.md** | Recovery commands for all scenarios | When errors occur |
| **MCP_CHECKPOINTING.md** | MCP server checkpoint integration | Before/after critical tasks |
| **DOCUMENTATION_UPDATES.md** | Post-sprint documentation procedures | After completing sprints |
| **MODULAR_DOCUMENTATION.md** | 500-line max rule and splitting strategies | When writing docs |
| **LOGGING_PATTERNS.md** | Array-based structured logging format | When implementing logging |
| **LOGGING_INTEGRATION.md** | Webhook/file/queue integration hooks | When adding external logging |
| **API_SPECIFICATIONS.md** | OpenAPI vs OpenSpec guidance | When documenting APIs |
| **SECRETS_MANAGEMENT.md** | GitHub Secrets setup and rotation | When handling credentials |
| **examples/** | Multi-language guardrails implementation examples | When exploring code examples |
| **regression-prevention/** | Bug tracking & regression prevention examples | When logging bugs or creating prevention rules |
| **mcp-server/API.md** | Complete REST API reference for MCP server | When integrating with MCP server |
| **mcp-server/CHANGELOG.md** | MCP server version history | When tracking MCP server updates |
| **SPRINT_TEMPLATE.md** | Copy-paste template for new sprints | When creating tasks |
| **SPRINT_GUIDE.md** | Best practices for writing sprints | When writing sprint docs |
| **PROJECT_CONTEXT_TEMPLATE.md** | Project Bible - stack, style, forbidden patterns | When setting up new project |
| **ADVERSARIAL_TESTING.md** | Breaker agent, fuzz testing, attack vectors | When security testing |
| **AGENT_REVIEW_PROTOCOL.md** | Post-work verification by another agent/LLM | After completing major work |
| **DEPENDENCY_GOVERNANCE.md** | Package allow-list, license compliance | When adding dependencies |
| **INFRASTRUCTURE_STANDARDS.md** | IaC, Terraform, no-ClickOps | When managing infrastructure |
| **OPERATIONAL_PATTERNS.md** | Health checks, circuit breakers, retry | When implementing services |
| **AGENTS_AND_SKILLS_SETUP.md** | Setup agents and skills for all AI platforms | When configuring AI tools |
| **CLCODE_INTEGRATION.md** | Claude Code skills and hooks integration | When using Claude Code |
| **OPENCODE_INTEGRATION.md** | OpenCode agents and skills integration | When using OpenCode |
| **CURSOR_INTEGRATION.md** | Cursor rules and guardrails integration | When using Cursor |
| **GENERIC_LLM_INTEGRATION.md** | Generic/local LLM configuration (Ollama, vLLM, etc.) | When using custom LLMs |
| **2026_GAME_DESIGN.md** | Game design guardrails, XR comfort zones, platform budgets | When building game interfaces or spatial UIs |
| **3D_GAME_DEVELOPMENT.md** | 3D game dev guardrails v1.0: engine-agnostic, asset pipeline, physics | When building 3D games with AI assistance |
| **3D_GUARDREL_PROPOSALS_V1.2.md** | Proposed v1.2 additions: neural fields, procedural geometry, AI NPCs | When reviewing next-gen 3D guardrails |
| **3D_MATHEMATICAL_FOUNDATIONS.md** | Linear algebra, quaternions, collision math for AI-generated 3D code | When generating 3D math code |
| **3D_MODULE_ARCHITECTURE.md** | Module architecture bridging LLMs with deterministic 3D engines | When architecting 3D game systems |
| **AI_DEBUGGABLE_3D_ARCHITECTURE.md** | Patterns enabling AI agents to debug 3D features autonomously | When designing debuggable 3D systems |
| **AI_DEV_2026_PART01_INTRO_AND_FOUNDATIONS.md** | AI-Powered Development 2026 Part 1: Introduction & Foundations | When starting the AI 2026 guide series |
| **AI_DEV_2026_PART02_PROMPTING.md** | AI-Powered Development 2026 Part 2: Prompt Engineering for Code | When learning prompt engineering |
| **AI_DEV_2026_PART03_CONTEXT_AND_ITERATION.md** | AI-Powered Development 2026 Part 3: Context & Iterative Development | When managing context windows and iteration |
| **AI_DEV_2026_PART04_QUALITY_AND_ARCHITECTURE.md** | AI-Powered Development 2026 Part 4: Quality & Architecture | When debugging, testing, and architecting with AI |
| **AI_DEV_2026_PART05_LEGACY_AND_AGENTS.md** | AI-Powered Development 2026 Part 5: Legacy & Agent Paradigm | When refactoring legacy code or moving to agents |
| **AI_DEV_2026_PART06_BUILDING_AGENTS.md** | AI-Powered Development 2026 Part 6: Building Agents & Tool Use | When building custom development agents |
| **AI_DEV_2026_PART07_MULTI_AGENT_SYSTEMS.md** | AI-Powered Development 2026 Part 7: Multi-Agent Systems | When implementing MoA and agent swarms |
| **AI_DEV_2026_PART08_SECURITY_ETHICS_FUTURE.md** | AI-Powered Development 2026 Part 8: Security, Ethics & Future | When evaluating responsible AI development |
| **AI_DEV_2026_PART09_APPENDICES_ABC.md** | AI-Powered Development 2026 Part 9: Appendices A, B & C | When referencing prompt patterns, local environments, case studies |
| **AI_DEV_2026_PART10_APPENDIX_D.md** | AI-Powered Development 2026 Part 10: Appendix D — Complete MoA Reference | When implementing MoA pipelines |
| **HERMES_2026_PART01_INTRO_AND_EXECUTIVE.md** | AI in 3D Game Development 2026 Part 1: Introduction & Executive Summary | When starting the dossier series |
| **HERMES_2026_PART02_ASSETS_AND_ENGINES.md** | AI in 3D Game Development 2026 Part 2: 3D Asset Generation & Engine Integration | When researching AI asset pipelines |
| **HERMES_2026_PART03_WORLD_AND_RENDERING.md** | AI in 3D Game Development 2026 Part 3: World Generation & Neural Rendering | When researching procedural worlds and rendering |
| **HERMES_2026_PART04_NPCS_AND_ANIMATION.md** | AI in 3D Game Development 2026 Part 4: NPCs, Dialogue & Animation | When researching AI characters and motion |
| **HERMES_2026_PART05_CODE_AND_PHYSICS.md** | AI in 3D Game Development 2026 Part 5: Code Generation & Neural Physics | When researching AI code gen and simulation |
| **HERMES_2026_PART06_QA_AND_BUSINESS.md** | AI in 3D Game Development 2026 Part 6: QA, Testing & Business Landscape | When researching AI QA and market trends |
| **HERMES_2026_PART07_LEGAL_AND_CASES.md** | AI in 3D Game Development 2026 Part 7: Legal, Ethics & Case Studies | When researching legal and ethical implications |
| **HERMES_2026_PART08_DEEP_DIVES_AND_FUTURE.md** | AI in 3D Game Development 2026 Part 8: Technology Deep-Dives & Future Outlook | When researching specific tools and future predictions |
| **HERMES_2026_PART09_APPENDICES.md** | AI in 3D Game Development 2026 Part 9: Appendices | When referencing glossary, tools, and sources |
| **2026_UI_UX_STANDARD.md** | UI/UX component patterns, design tokens, animation | When implementing UI components |
| **ACCESSIBILITY_GUIDE.md** | WCAG 3.0+ compliance, perceptual/cognitive/physical a11y | When ensuring accessibility compliance |
| **SPATIAL_COMPUTING_UI.md** | XR/VR/AR layout patterns, comfort zones, latency | When building spatial computing interfaces |
| **ETHICAL_ENGAGEMENT.md** | Dark pattern taxonomy, ethical design principles | When reviewing engagement patterns |
| **AI_ASSISTED_DEV.md** | AI development patterns, decision matrix, vibe coding workflow | When implementing AI-first rapid development |
| **STATE_MANAGEMENT.md** | State architecture decision tree, client/server/offline/CRDT patterns | When designing state management |
| **GENERATIVE_ASSET_SAFETY.md** | AI content disclosure, C2PA metadata, procedural generation safety | When handling AI-generated assets |
| **MONETIZATION_GUARDRAILS.md** | IAP ethics, loot box transparency, virtual economy balance | When implementing monetization |
| **MULTIPLAYER_SAFETY.md** | Social safety, chat moderation, matchmaking fairness, CSAM detection | When building multiplayer systems |
| **ANALYTICS_ETHICS.md** | Consent tiers, data minimization, A/B testing ethics | When implementing analytics |
| **CROSS_PLATFORM_DEPLOYMENT.md** | App store compliance matrix, CI/CD, feature flags, progressive rollout | When deploying across platforms |
| **vibe-coding.md** | Canonical vibe coding principles (5 principles) | When establishing rapid development culture |
| **examples/flutter/cross-platform/** | Flutter guardrails: config, ethical widgets, accessibility wrappers | When implementing Flutter guardrails |
| **examples/gdscript/godot-game/** | Godot GDScript: comfort zones, ethical UI, accessibility manager | When implementing Godot GDScript guardrails |

---

## Category Index

### AI Tools Integration
- `AGENTS_AND_SKILLS_SETUP.md` - Setup guide for all AI platforms (Claude Code, OpenCode, Cursor, Copilot, etc.)
- `CLCODE_INTEGRATION.md` - Claude Code skills and hooks
- `OPENCODE_INTEGRATION.md` - OpenCode agents and skills
- `CURSOR_INTEGRATION.md` - Cursor rules configuration
- `GENERIC_LLM_INTEGRATION.md` - Generic/local LLM setup (Ollama, vLLM, etc.)

### Git Operations
- `COMMIT_WORKFLOW.md` - Commit timing and format
- `GIT_PUSH_PROCEDURES.md` - Push safety and verification
- `BRANCH_STRATEGY.md` - Branch naming and workflow
- `ROLLBACK_PROCEDURES.md` - Undo and recovery

### Quality & Validation
- `TESTING_VALIDATION.md` - Pre/post validation checks
- `CODE_REVIEW.md` - Review process and escalation
- `AGENT_GUARDRAILS.md` - Safety protocols (MANDATORY)
- `AGENT_REVIEW_PROTOCOL.md` - Post-work agent/LLM review
- `ADVERSARIAL_TESTING.md` - Breaker agent and fuzz testing
- `AGENTS_AND_SKILLS_SETUP.md` - Setup guide for Claude Code/OpenCode
- `RULES_FROM_MD.md` - Extracting prevention rules from markdown
- `RULES_INDEX_MAP.md` - Master index of all prevention rules
- `MCP_TOOLS_REFERENCE.md` - MCP validation tools documentation
- `RULE_PATTERNS_GUIDE.md` - Pattern authoring guide

### Logging & Monitoring
- `LOGGING_PATTERNS.md` - Structured log format
- `LOGGING_INTEGRATION.md` - External system hooks
- `MCP_CHECKPOINTING.md` - State checkpoints

### Documentation Standards
- `MODULAR_DOCUMENTATION.md` - 500-line rule
- `DOCUMENTATION_UPDATES.md` - Post-sprint updates
- `API_SPECIFICATIONS.md` - API doc formats

### Security
- `SECRETS_MANAGEMENT.md` - GitHub Secrets
- `AGENT_GUARDRAILS.md` - Forbidden actions
- `ADVERSARIAL_TESTING.md` - Security attack checklists
- `DEPENDENCY_GOVERNANCE.md` - Package allow-list

### Infrastructure & Operations
- `INFRASTRUCTURE_STANDARDS.md` - IaC and Terraform standards
- `OPERATIONAL_PATTERNS.md` - Health checks, circuit breakers, retry

### 2026 Game Design & UI/UX
- `2026_GAME_DESIGN.md` - Game design guardrails, XR/VR comfort zones, platform rules
- `3D_GAME_DEVELOPMENT.md` - 3D game development guardrails v1.0, engine-agnostic patterns
- `3D_GUARDREL_PROPOSALS_V1.2.md` - Proposed v1.2 additions from Hermes 2026 AI Dossier review
- `3D_MATHEMATICAL_FOUNDATIONS.md` - Linear algebra, quaternions, collision math reference
- `3D_MODULE_ARCHITECTURE.md` - Module architecture for LLM-to-3D-engine bridging
- `AI_DEBUGGABLE_3D_ARCHITECTURE.md` - AI-debuggable patterns for autonomous 3D troubleshooting
- `AI_DEV_2026_PART01_INTRO_AND_FOUNDATIONS.md` - Part 1: Introduction & Foundations
- `AI_DEV_2026_PART02_PROMPTING.md` - Part 2: Prompt Engineering for Code
- `AI_DEV_2026_PART03_CONTEXT_AND_ITERATION.md` - Part 3: Context & Iterative Development
- `AI_DEV_2026_PART04_QUALITY_AND_ARCHITECTURE.md` - Part 4: Quality & Architecture
- `AI_DEV_2026_PART05_LEGACY_AND_AGENTS.md` - Part 5: Legacy & Agent Paradigm
- `AI_DEV_2026_PART06_BUILDING_AGENTS.md` - Part 6: Building Agents & Tool Use
- `AI_DEV_2026_PART07_MULTI_AGENT_SYSTEMS.md` - Part 7: Multi-Agent Systems
- `AI_DEV_2026_PART08_SECURITY_ETHICS_FUTURE.md` - Part 8: Security, Ethics & Future
- `AI_DEV_2026_PART09_APPENDICES_ABC.md` - Part 9: Appendices A, B & C
- `AI_DEV_2026_PART10_APPENDIX_D.md` - Part 10: Appendix D — Complete MoA Reference
- `HERMES_2026_PART01_INTRO_AND_EXECUTIVE.md` - Part 1: Introduction & Executive Summary
- `HERMES_2026_PART02_ASSETS_AND_ENGINES.md` - Part 2: 3D Asset Generation & Engine Integration
- `HERMES_2026_PART03_WORLD_AND_RENDERING.md` - Part 3: World Generation & Neural Rendering
- `HERMES_2026_PART04_NPCS_AND_ANIMATION.md` - Part 4: NPCs, Dialogue & Animation
- `HERMES_2026_PART05_CODE_AND_PHYSICS.md` - Part 5: Code Generation & Neural Physics
- `HERMES_2026_PART06_QA_AND_BUSINESS.md` - Part 6: QA, Testing & Business Landscape
- `HERMES_2026_PART07_LEGAL_AND_CASES.md` - Part 7: Legal, Ethics & Case Studies
- `HERMES_2026_PART08_DEEP_DIVES_AND_FUTURE.md` - Part 8: Technology Deep-Dives & Future Outlook
- `HERMES_2026_PART09_APPENDICES.md` - Part 9: Appendices
- `2026_UI_UX_STANDARD.md` - UI/UX component standards, design tokens, responsive patterns
- `ACCESSIBILITY_GUIDE.md` - WCAG 3.0+ conformance (Bronze/Silver/Gold), automated testing
- `SPATIAL_COMPUTING_UI.md` - XR/VR/AR comfort zones, latency, depth layering, interaction
- `ETHICAL_ENGAGEMENT.md` - Dark pattern taxonomy, ethical review capabilities

### Project Setup
- `PROJECT_CONTEXT_TEMPLATE.md` - Project Bible template

### Sprint Framework
- `SPRINT_TEMPLATE.md` - Task template
- `SPRINT_GUIDE.md` - Writing guide
- `INDEX.md` (sprints/) - Sprint navigation

---

## Directory Structure

```
agent-guardrails-template/
├── INDEX_MAP.md              ← YOU ARE HERE
├── TOC.md                   ← Complete file listing and contents
├── HEADER_MAP.md             # Section-level lookup
├── CLAUDE.md                 # Claude Code CLI config
├── .claudeignore             # Token-saving ignores
├── CHANGELOG.md              # Release notes archive
├── docs/
│   ├── AGENT_GUARDRAILS.md   # Core safety (MANDATORY)
│   ├── HOW_TO_APPLY.md       # How to apply to repos
│   ├── AGENTS_AND_SKILLS_SETUP.md         # Setup guide for Claude Code/OpenCode
│   ├── CLCODE_INTEGRATION.md              # Claude Code integration
│   ├── OPCODE_INTEGRATION.md              # OpenCode integration
│   ├── workflows/
│   │   ├── INDEX.md
│   │   ├── AGENT_EXECUTION.md       # Execution protocol
│   │   ├── AGENT_ESCALATION.md      # Audit & escalation
│   │   ├── TESTING_VALIDATION.md
│   │   ├── COMMIT_WORKFLOW.md
│   │   ├── DOCUMENTATION_UPDATES.md
│   │   ├── GIT_PUSH_PROCEDURES.md
│   │   ├── MCP_CHECKPOINTING.md
│   │   ├── BRANCH_STRATEGY.md
│   │   ├── CODE_REVIEW.md
│   │   ├── AGENT_REVIEW_PROTOCOL.md       # Post-work agent review
│   │   └── ROLLBACK_PROCEDURES.md
│   ├── standards/
│   │   ├── INDEX.md
│   │   ├── TEST_PRODUCTION_SEPARATION.md  # Test/production isolation (MANDATORY)
│   │   ├── PROJECT_CONTEXT_TEMPLATE.md    # Project Bible template
│   │   ├── ADVERSARIAL_TESTING.md         # Breaker agent, fuzz testing
│   │   ├── DEPENDENCY_GOVERNANCE.md       # Package allow-list
│   │   ├── INFRASTRUCTURE_STANDARDS.md    # IaC, Terraform, drift
│   │   ├── OPERATIONAL_PATTERNS.md        # Health checks, circuit breakers
│   │   ├── MODULAR_DOCUMENTATION.md
│   │   ├── LOGGING_PATTERNS.md
│   │   ├── LOGGING_INTEGRATION.md
│   │   └── API_SPECIFICATIONS.md
│   ├── sprints/
│       ├── INDEX.md
│       ├── SPRINT_TEMPLATE.md
│       ├── SPRINT_GUIDE.md
│       └── archive/
│   ├── game-design/
│   │   └── 2026_GAME_DESIGN.md      # Game design guardrails, XR comfort
│   ├── ui-ux/
│   │   └── 2026_UI_UX_STANDARD.md   # UI/UX components, design tokens
│   ├── accessibility/
│   │   └── ACCESSIBILITY_GUIDE.md   # WCAG 3.0+ compliance
│   ├── spatial/
│   │   └── SPATIAL_COMPUTING_UI.md  # XR/VR/AR patterns
│   └── ethical/
│       └── ETHICAL_ENGAGEMENT.md    # Dark pattern prevention
├── examples/               ← Multi-language implementation examples
│   ├── go/
│   ├── java/
│   ├── python/
│   ├── ruby/
│   ├── regression-prevention/  # Bug tracking examples
│   ├── rust/
│   └── typescript/
├── scripts/                ← Setup and utility scripts
│   └── setup_agents.py     # CLI tool to generate agent configs
├── .github/
│   ├── SECRETS_MANAGEMENT.md
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── workflows/
│   │   ├── secret-validation.yml
│   │   ├── documentation-check.yml
│   │   └── guardrails-lint.yml
│   └── ISSUE_TEMPLATE/
│       └── bug_report.md
└── README.md
```

---

## Usage Instructions

### For AI Agents

1. **Always read INDEX_MAP.md first** before exploring documentation
2. Use the Quick Lookup Table to find relevant documents by keyword
3. Check HEADER_MAP.md for specific section line numbers
4. Read only the sections you need using line offset parameters
5. For mandatory safety protocols, always read AGENT_GUARDRAILS.md

### For Humans

1. Use Category Index to browse by topic
2. Document Summaries tell you when to use each doc
3. Directory Structure shows the full file layout

---

## Cross-Reference Quick Links

| If you need... | Read... |
|----------------|---------|
| Safety rules before editing | AGENT_GUARDRAILS.md |
| How to validate changes | TESTING_VALIDATION.md |
| When to commit | COMMIT_WORKFLOW.md |
| How to handle errors | ROLLBACK_PROCEDURES.md |
| Logging format | LOGGING_PATTERNS.md |
| Secret handling | SECRETS_MANAGEMENT.md |
| Creating a new task | SPRINT_TEMPLATE.md |
| Setting up AI tools | AGENTS_AND_SKILLS_SETUP.md |
| Claude Code integration | CLCODE_INTEGRATION.md |
| OpenCode integration | OPCODE_INTEGRATION.md |
| Cursor integration | CURSOR_INTEGRATION.md |
| Generic LLM integration | GENERIC_LLM_INTEGRATION.md |
| MCP rule extraction | RULES_FROM_MD.md |
| Prevention rules index | RULES_INDEX_MAP.md |
| MCP tools reference | MCP_TOOLS_REFERENCE.md |
| Pattern authoring | RULE_PATTERNS_GUIDE.md |
| Game design guardrails | 2026_GAME_DESIGN.md |
| 3D game development guardrails | 3D_GAME_DEVELOPMENT.md |
| 3D math reference | 3D_MATHEMATICAL_FOUNDATIONS.md |
| 3D architecture blueprint | 3D_MODULE_ARCHITECTURE.md |
| AI-debuggable 3D patterns | AI_DEBUGGABLE_3D_ARCHITECTURE.md |
| AI development 2026 guide | AI_DEV_2026_PART01_INTRO_AND_FOUNDATIONS.md |
| AI 2026 prompt engineering | AI_DEV_2026_PART02_PROMPTING.md |
| AI 2026 context & iteration | AI_DEV_2026_PART03_CONTEXT_AND_ITERATION.md |
| AI 2026 quality & architecture | AI_DEV_2026_PART04_QUALITY_AND_ARCHITECTURE.md |
| AI 2026 legacy & agents | AI_DEV_2026_PART05_LEGACY_AND_AGENTS.md |
| AI 2026 building agents | AI_DEV_2026_PART06_BUILDING_AGENTS.md |
| AI 2026 multi-agent systems | AI_DEV_2026_PART07_MULTI_AGENT_SYSTEMS.md |
| AI 2026 security & ethics | AI_DEV_2026_PART08_SECURITY_ETHICS_FUTURE.md |
| AI 2026 appendices | AI_DEV_2026_PART09_APPENDICES_ABC.md |
| AI 2026 MoA reference | AI_DEV_2026_PART10_APPENDIX_D.md |
| AI in 3D games dossier | HERMES_2026_PART01_INTRO_AND_EXECUTIVE.md |
| Hermes assets & engines | HERMES_2026_PART02_ASSETS_AND_ENGINES.md |
| Hermes world & rendering | HERMES_2026_PART03_WORLD_AND_RENDERING.md |
| Hermes NPCs & animation | HERMES_2026_PART04_NPCS_AND_ANIMATION.md |
| Hermes code & physics | HERMES_2026_PART05_CODE_AND_PHYSICS.md |
| Hermes QA & business | HERMES_2026_PART06_QA_AND_BUSINESS.md |
| Hermes legal & cases | HERMES_2026_PART07_LEGAL_AND_CASES.md |
| Hermes deep-dives & future | HERMES_2026_PART08_DEEP_DIVES_AND_FUTURE.md |
| Hermes appendices | HERMES_2026_PART09_APPENDICES.md |
| UI/UX component standards | 2026_UI_UX_STANDARD.md |
| Accessibility (WCAG 3.0+) | ACCESSIBILITY_GUIDE.md |
| Spatial computing / XR / VR | SPATIAL_COMPUTING_UI.md |
| Dark pattern prevention | ETHICAL_ENGAGEMENT.md |

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-05-12
**Document Count:** 103 (excluding INDEX files)
**Line Count:** ~280

---

## Canonical Sources

To avoid duplication, always reference these canonical sources:

| Content | Canonical Location | Reference In |
|---------|-------------------|--------------|
| Four Laws | skills/shared-prompts/four-laws.md | docs/AGENT_GUARDRAILS.md |
| Halt Conditions | skills/shared-prompts/halt-conditions.md | Workflows, integration docs |

---

## Oversized Documents

The following files exceed the 500-line limit and should be split per MODULAR_DOCUMENTATION.md:

| File | Lines | Action Needed |
|------|-------|---------------|
| docs/plans/MCP_SERVER_PLAN.md | 2093 | Split into multiple files |
| docs/sprints/SPRINT_002_WEB_UI_IMPLEMENTATION.md | 768 | Split or archive |
| docs/sprints/SPRINT_003_DOCUMENTATION_PARITY.md | 754 | Split or archive after completion |
| HEADER_MAP.md | 822 | Navigation file - exempt |
| docs/standards/OPERATIONAL_PATTERNS.md | 667 | Split |
| docs/workflows/AGENT_REVIEW_PROTOCOL.md | 638 | Split |
| docs/security/SECURITY_AUDIT_CONFIG.md | 597 | Split |
| README.md | 565 | Landing page - exempt |
