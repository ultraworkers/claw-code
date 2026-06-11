# Documentation Index Map

**Purpose:** Find documentation by keyword/category. Saves 60-80% tokens vs full document reads.

**Usage:** Search keyword → identify doc → use HEADER_MAP.md for section-level lookup → read specific section

---

## CORE GUARDRAILS

| Keyword | Document | Location |
|---------|----------|----------|
| agent safety, four laws, halt conditions | [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md) | docs/ |
| test/prod separation | [TEST_PRODUCTION_SEPARATION.md](standards/TEST_PRODUCTION_SEPARATION.md) | docs/standards/ |
| regression prevention | [REGRESSION_PREVENTION.md](workflows/REGRESSION_PREVENTION.md) | docs/workflows/ |
| pre-work checklist | [.guardrails/pre-work-check.md](../.guardrails/pre-work-check.md) | .guardrails/ |
| failure registry | [.guardrails/failure-registry.jsonl](../.guardrails/failure-registry.jsonl) | .guardrails/ |

---

## 2026 UI/UX GAME DESIGN

**New Section:** Game Design & UI/UX 2026 Standards

| Keyword | Document | Location |
|---------|----------|----------|
| game, UI, UX, spatial, XR, VR, AR, accessibility, 2026 | [2026_GAME_DESIGN.md](game-design/2026_GAME_DESIGN.md) | docs/game-design/ |
| component, design tokens, interaction, responsive | [2026_UI_UX_STANDARD.md](ui-ux/2026_UI_UX_STANDARD.md) | docs/ui-ux/ |
| WCAG 3.0, accessibility, contrast, keyboard, screen reader | [ACCESSIBILITY_GUIDE.md](accessibility/ACCESSIBILITY_GUIDE.md) | docs/accessibility/ |
| spatial computing, XR, VR, AR, MR, depth, comfort zone | [SPATIAL_COMPUTING_UI.md](spatial/SPATIAL_COMPUTING_UI.md) | docs/spatial/ |
| ethical, dark patterns, manipulation, engagement | [ETHICAL_ENGAGEMENT.md](ethical/ETHICAL_ENGAGEMENT.md) | docs/ethical/ |
| Agent-GDUI-2026 | [2026_GAME_DESIGN.md](game-design/2026_GAME_DESIGN.md) | docs/game-design/ |

### Agent-GDUI-2026 Capability References

| Capability | Document | Section |
|------------|----------|---------|
| Spatial Layout | [SPATIAL_COMPUTING_UI.md](spatial/SPATIAL_COMPUTING_UI.md) | Depth Layering |
| Motion Design | [2026_UI_UX_STANDARD.md](ui-ux/2026_UI_UX_STANDARD.md) | Animation Guidelines |
| Audio Spatialization | [SPATIAL_COMPUTING_UI.md](spatial/SPATIAL_COMPUTING_UI.md) | Audio Spatialization |
| Input Mapping | [2026_GAME_DESIGN.md](game-design/2026_GAME_DESIGN.md) | Platform-Specific Rules |
| Ethical Review | [ETHICAL_ENGAGEMENT.md](ethical/ETHICAL_ENGAGEMENT.md) | Dark Pattern Prevention |
| Performance Budget | [2026_GAME_DESIGN.md](game-design/2026_GAME_DESIGN.md) | Performance Budgets |
| Accessibility Compliance | [ACCESSIBILITY_GUIDE.md](accessibility/ACCESSIBILITY_GUIDE.md) | Implementation Checklist |

---

## WORKFLOW DOCUMENTATION

| Keyword | Document | Location |
|---------|----------|----------|
| agent execution, rollback, three strikes | [AGENT_EXECUTION.md](workflows/AGENT_EXECUTION.md) | docs/workflows/ |
| escalation, audit | [AGENT_ESCALATION.md](workflows/AGENT_ESCALATION.md) | docs/workflows/ |
| testing, validation | [TESTING_VALIDATION.md](workflows/TESTING_VALIDATION.md) | docs/workflows/ |
| commit workflow | [COMMIT_WORKFLOW.md](workflows/COMMIT_WORKFLOW.md) | docs/workflows/ |
| git push | [GIT_PUSH_PROCEDURES.md](workflows/GIT_PUSH_PROCEDURES.md) | docs/workflows/ |
| branch strategy | [BRANCH_STRATEGY.md](workflows/BRANCH_STRATEGY.md) | docs/workflows/ |
| code review | [CODE_REVIEW.md](workflows/CODE_REVIEW.md) | docs/workflows/ |
| rollback | [ROLLBACK_PROCEDURES.md](workflows/ROLLBACK_PROCEDURES.md) | docs/workflows/ |
| MCP checkpointing | [MCP_CHECKPOINTING.md](workflows/MCP_CHECKPOINTING.md) | docs/workflows/ |
| documentation updates | [DOCUMENTATION_UPDATES.md](workflows/DOCUMENTATION_UPDATES.md) | docs/workflows/ |
| agent review protocol | [AGENT_REVIEW_PROTOCOL.md](workflows/AGENT_REVIEW_PROTOCOL.md) | docs/workflows/ |

---

## STANDARDS DOCUMENTATION

| Keyword | Document | Location |
|---------|----------|----------|
| test/prod separation | [TEST_PRODUCTION_SEPARATION.md](standards/TEST_PRODUCTION_SEPARATION.md) | docs/standards/ |
| modular documentation, 500-line | [MODULAR_DOCUMENTATION.md](standards/MODULAR_DOCUMENTATION.md) | docs/standards/ |
| logging | [LOGGING_PATTERNS.md](standards/LOGGING_PATTERNS.md) | docs/standards/ |
| logging integration | [LOGGING_INTEGRATION.md](standards/LOGGING_INTEGRATION.md) | docs/standards/ |
| API | [API_SPECIFICATIONS.md](standards/API_SPECIFICATIONS.md) | docs/standards/ |
| adversarial testing | [ADVERSARIAL_TESTING.md](standards/ADVERSARIAL_TESTING.md) | docs/standards/ |
| dependency governance | [DEPENDENCY_GOVERNANCE.md](standards/DEPENDENCY_GOVERNANCE.md) | docs/standards/ |
| infrastructure | [INFRASTRUCTURE_STANDARDS.md](standards/INFRASTRUCTURE_STANDARDS.md) | docs/standards/ |
| operational patterns | [OPERATIONAL_PATTERNS.md](standards/OPERATIONAL_PATTERNS.md) | docs/standards/ |
| project context template | [PROJECT_CONTEXT_TEMPLATE.md](standards/PROJECT_CONTEXT_TEMPLATE.md) | docs/standards/ |

---

## SPRINT FRAMEWORK

| Keyword | Document | Location |
|---------|----------|----------|
| sprint template | [SPRINT_TEMPLATE.md](sprints/SPRINT_TEMPLATE.md) | docs/sprints/ |
| sprint guide | [SPRINT_GUIDE.md](sprints/SPRINT_GUIDE.md) | docs/sprints/ |
| sprint examples | [SPRINT_001_MCP_GAP_IMPLEMENTATION.md](sprints/SPRINT_001_MCP_GAP_IMPLEMENTATION.md) | docs/sprints/ |

---

## MCP SERVER

| Keyword | Document | Location |
|---------|----------|----------|
| MCP tools | [MCP_TOOLS_REFERENCE.md](MCP_TOOLS_REFERENCE.md) | docs/ |
| team tools | [TEAM_TOOLS.md](TEAM_TOOLS.md) | docs/ |
| team structure | [TEAM_STRUCTURE.md](TEAM_STRUCTURE.md) | docs/ |
| deployment | [DEPLOYMENT_GUIDE.md](../mcp-server/DEPLOYMENT_GUIDE.md) | mcp-server/ |
| API | [API.md](../mcp-server/API.md) | mcp-server/ |
| clean architecture, CQRS, vertical slices, SOLID, DDD | [ARCHITECTURE_CLEAN_CQRS.md](ARCHITECTURE_CLEAN_CQRS.md) | docs/ |

---

## AI TOOL INTEGRATION

| Keyword | Document | Location |
|---------|----------|----------|
| Claude Code | [CLCODE_INTEGRATION.md](CLCODE_INTEGRATION.md) | docs/ |
| OpenCode | [OPCODE_INTEGRATION.md](OPCODE_INTEGRATION.md) | docs/ |
| Cursor | [CURSOR_INTEGRATION.md](CURSOR_INTEGRATION.md) | docs/ |
| Windsurf | [WINDSURF_INTEGRATION.md](WINDSURF_INTEGRATION.md) | docs/ |
| GitHub Copilot | [COPILOT_INTEGRATION.md](COPILOT_INTEGRATION.md) | docs/ |
| agents, skills, setup, install, MCP tool, platform comparison | [AGENTS_AND_SKILLS_SETUP.md](AGENTS_AND_SKILLS_SETUP.md) | docs/ |
| skill configs | [.claude/skills/](../.claude/skills/) | / (root) |
| Cursor rules | [.cursor/rules/](../.cursor/rules/) | / (root) |
| OpenCode skills | [.opencode/](../.opencode/) | / (root) |

---

## SHARED SKILL PROMPTS

| Keyword | Document | Location |
|---------|----------|----------|
| four laws, agent safety | [skills/shared-prompts/four-laws.md](../skills/shared-prompts/four-laws.md) | / (root) |
| halt conditions | [skills/shared-prompts/halt-conditions.md](../skills/shared-prompts/halt-conditions.md) | / (root) |
| vibe coding | [skills/shared-prompts/vibe-coding.md](../skills/shared-prompts/vibe-coding.md) | / (root) |
| error recovery | [skills/shared-prompts/error-recovery.md](../skills/shared-prompts/error-recovery.md) | / (root) |
| three strikes | [skills/shared-prompts/three-strikes.md](../skills/shared-prompts/three-strikes.md) | / (root) |
| production-first | [skills/shared-prompts/production-first.md](../skills/shared-prompts/production-first.md) | / (root) |
| scope validation | [skills/shared-prompts/scope-validation.md](../skills/shared-prompts/scope-validation.md) | / (root) |
| clean architecture | [skills/shared-prompts/clean-architecture.md](../skills/shared-prompts/clean-architecture.md) | / (root) |
| CQRS | [skills/shared-prompts/cqrs.md](../skills/shared-prompts/cqrs.md) | / (root) |
| shared-prompt-install | [AGENTS_AND_SKILLS_SETUP.md](AGENTS_AND_SKILLS_SETUP.md) | docs/ |

---

## LANGUAGE-SPECIFIC EXAMPLES

| Language | Examples Directory |
|----------|-------------------|
| Go | [examples/go/](examples/go/) |
| Java | [examples/java/](examples/java/) |
| Python | [examples/python/](examples/python/) |
| Ruby | [examples/ruby/](examples/ruby/) |
| Rust | [examples/rust/](examples/rust/) |
| TypeScript | [examples/typescript/](examples/typescript/) |

### 2026 UI/UX Examples

| Pattern | Directory |
|---------|-----------|
| TypeScript UI Components | `examples/typescript/ui-components/` |
| TypeScript Game UI | `examples/typescript/game-ui/` |
| Python UI Dashboard | `examples/python/ui-dashboard/` |
| Python Game Tools | `examples/python/game-tools/` |
| Go HTMX Patterns | `examples/go/htmx-patterns/` |
| Go Admin UI | `examples/go/admin-ui/` |
| Rust Bevy UI | `examples/rust/bevy-ui-example/` |
| Rust egui Overlay | `examples/rust/egui-overlay/` |

---

## SECURITY

| Keyword | Document | Location |
|---------|----------|----------|
| secrets management | [SECRETS_MANAGEMENT.md](../.github/SECRETS_MANAGEMENT.md) | .github/ |
| security audit | [security/](security/) | docs/security/ |

---

## RELEASE HISTORY

| Keyword | Document | Location |
|---------|----------|----------|
| changelog | [CHANGELOG.md](CHANGELOG.md) | docs/ |
| v2.6.0 | [README.md](README.md) | / (current version) |
| v1.10.0 | [RELEASE_v1.10.0.md](RELEASE_v1.10.0.md) | docs/ |
| v1.9.x | [RELEASE_v1.9.0.md](RELEASE_v1.9.0.md) - [RELEASE_v1.9.6.md](RELEASE_v1.9.6.md) | docs/ |

---

## QUICK NAVIGATION

1. **New to guardrails?** → Start with [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md)
2. **Game/UI development?** → [2026_GAME_DESIGN.md](game-design/2026_GAME_DESIGN.md)
3. **Accessibility?** → [ACCESSIBILITY_GUIDE.md](accessibility/ACCESSIBILITY_GUIDE.md)
4. **Spatial computing?** → [SPATIAL_COMPUTING_UI.md](spatial/SPATIAL_COMPUTING_UI.md)
5. **Ethical review?** → [ETHICAL_ENGAGEMENT.md](ethical/ETHICAL_ENGAGEMENT.md)
6. **UI components?** → [2026_UI_UX_STANDARD.md](ui-ux/2026_UI_UX_STANDARD.md)
7. **Workflow?** → [workflows/INDEX.md](workflows/INDEX.md)
8. **Standards?** → [standards/INDEX.md](standards/INDEX.md)
9. **Sprints?** → [sprints/INDEX.md](sprints/INDEX.md)

---

**Last Updated:** 2026-05-09
**Document Owner:** Documentation Team
**Token Savings:** 60-80% vs full document reads