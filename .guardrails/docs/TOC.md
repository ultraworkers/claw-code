# Table of Contents (TOC)

**Purpose:** Complete file listing and organization structure for the Agent Guardrails Template.

**Usage:** Navigate by category → identify file → use INDEX_MAP.md for keyword search → use HEADER_MAP.md for section lookup

---

## ROOT FILES

| File | Purpose | Location |
|------|---------|----------|
| README.md | Project overview and setup | / |
| CLAUDE.md | Claude Code CLI guidelines | / |
| INDEX_MAP.md | Documentation keyword index | docs/ |
| HEADER_MAP.md | Section-level file:line references | docs/ |
| TOC.md | This file - complete file listing | docs/ |
| CHANGELOG.md | Release history archive | docs/ |

---

## 2026 GAME DESIGN & UI/UX

**New Category:** Game Design, UI/UX, Spatial Computing, Accessibility, Ethical Engagement

### Game Design

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [2026_GAME_DESIGN.md](game-design/2026_GAME_DESIGN.md) | Core game design guardrails | ~190 | Game Design |

**Contents:**
- Agent-GDUI-2026 Role Definition
- Four Laws of Spatial Safety
- XR/VR Comfort Rules
- Accessibility Requirements (WCAG 3.0+)
- Performance Budgets
- Dark Pattern Prevention
- Platform-Specific Rules (Mobile, PC, Console, XR)
- Language-Specific Patterns (TypeScript, Rust, Go)

### UI/UX

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [2026_UI_UX_STANDARD.md](ui-ux/2026_UI_UX_STANDARD.md) | UI/UX 2026 standard reference | ~230 | UI/UX |

**Contents:**
- Agent-GDUI-2026 Capabilities
- Core Components (Foundational, Advanced)
- Design Tokens (Color, Typography, Spacing)
- Interaction States
- Animation Guidelines
- Responsive Breakpoints
- Language-Specific Patterns

### Accessibility

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [ACCESSIBILITY_GUIDE.md](accessibility/ACCESSIBILITY_GUIDE.md) | WCAG 3.0+ implementation guide | ~210 | Accessibility |

**Contents:**
- WCAG 3.0 Conformance Levels
- Implementation Checklist (Perceptual, Cognitive, Physical)
- Technical Implementation (Contrast, Focus, Screen Reader, Keyboard)
- Testing Methods (Automated, Manual)
- Language-Specific Patterns

### Spatial Computing

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [SPATIAL_COMPUTING_UI.md](spatial/SPATIAL_COMPUTING_UI.md) | XR/VR/AR UI patterns | ~270 | Spatial Computing |

**Contents:**
- Agent-GDUI-2026 Spatial Capabilities
- Spatial Safety Protocols (Comfort Zones, Latency, Vestibular)
- UI Layout Patterns (VR, AR, MR)
- Depth Layering (Three-Layer Standard)
- Interaction Patterns (Gesture, Gaze, Haptic)
- Accessibility in XR
- Performance Budgets

### Ethical Engagement

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [ETHICAL_ENGAGEMENT.md](ethical/ETHICAL_ENGAGEMENT.md) | Dark pattern prevention guide | ~240 | Ethical Engagement |

**Contents:**
- Agent-GDUI-2026 Ethical Review
- Dark Pattern Taxonomy (Deceptive, Coercive, Addictive, Exploitation)
- Ethical Design Principles (Transparency, Choice, Wellbeing, Respect)
- Implementation Checklist
- Technical Implementation (Middleware, Detection)

---

## CORE GUARDRAILS

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md) | Core safety protocols | ~310 | Mandatory |
| [RULES_FROM_MD.md](RULES_FROM_MD.md) | Rule extraction reference | ~50 | Reference |
| [RULES_INDEX_MAP.md](RULES_INDEX_MAP.md) | Rules keyword index | ~40 | Reference |
| [RULE_PATTERNS_GUIDE.md](RULE_PATTERNS_GUIDE.md) | Rule pattern guide | ~60 | Reference |

---

## HOW TO APPLY

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [HOW_TO_APPLY.md](HOW_TO_APPLY.md) | How to apply template to your repo | ~200 | Application |
| [AGENTS_AND_SKILLS_SETUP.md](AGENTS_AND_SKILLS_SETUP.md) | AI tool setup guide | ~150 | Setup |
| [CLCODE_INTEGRATION.md](CLCODE_INTEGRATION.md) | Claude Code integration | ~120 | Integration |
| [OPCODE_INTEGRATION.md](OPCODE_INTEGRATION.md) | OpenCode integration | ~100 | Integration |
| [CURSOR_INTEGRATION.md](CURSOR_INTEGRATION.md) | Cursor IDE integration | ~80 | Integration |
| [OPENCODE_INTEGRATION.md](OPENCODE_INTEGRATION.md) | OpenCode full integration | ~150 | Integration |

---

## TEAM STRUCTURE & MCP TOOLS

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [TEAM_STRUCTURE.md](TEAM_STRUCTURE.md) | Team layout overview | ~100 | Team |
| [TEAM_TOOLS.md](TEAM_TOOLS.md) | MCP team tools reference | ~200 | MCP |
| [TEAM_TOOLS_GAP_REMEDIATION_PLAN.md](TEAM_TOOLS_GAP_REMEDIATION_PLAN.md) | Gap remediation | ~150 | Planning |
| [GAP_ANALYSIS_TEAM_REPORT.md](GAP_ANALYSIS_TEAM_REPORT.md) | Team gap analysis | ~180 | Analysis |
| [MCP_TOOLS_REFERENCE.md](MCP_TOOLS_REFERENCE.md) | MCP tools complete reference | ~250 | MCP |

---

## WORKFLOW DOCUMENTATION

**Category:** Operational procedures for AI agents

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [INDEX.md](workflows/INDEX.md) | Workflow index | ~30 | Index |
| [AGENT_EXECUTION.md](workflows/AGENT_EXECUTION.md) | Execution protocol | ~180 | Workflow |
| [AGENT_ESCALATION.md](workflows/AGENT_ESCALATION.md) | Audit & escalation | ~150 | Workflow |
| [AGENT_REVIEW_PROTOCOL.md](workflows/AGENT_REVIEW_PROTOCOL.md) | Post-work review | ~120 | Workflow |
| [TESTING_VALIDATION.md](workflows/TESTING_VALIDATION.md) | Validation protocols | ~140 | Workflow |
| [COMMIT_WORKFLOW.md](workflows/COMMIT_WORKFLOW.md) | Commit guidelines | ~100 | Workflow |
| [GIT_PUSH_PROCEDURES.md](workflows/GIT_PUSH_PROCEDURES.md) | Push safety procedures | ~160 | Workflow |
| [BRANCH_STRATEGY.md](workflows/BRANCH_STRATEGY.md) | Branch management | ~100 | Workflow |
| [CODE_REVIEW.md](workflows/CODE_REVIEW.md) | Code review process | ~120 | Workflow |
| [ROLLBACK_PROCEDURES.md](workflows/ROLLBACK_PROCEDURES.md) | Recovery operations | ~100 | Workflow |
| [MCP_CHECKPOINTING.md](workflows/MCP_CHECKPOINTING.md) | Checkpoint integration | ~130 | Workflow |
| [DOCUMENTATION_UPDATES.md](workflows/DOCUMENTATION_UPDATES.md) | Doc update procedures | ~80 | Workflow |
| [REGRESSION_PREVENTION.md](workflows/REGRESSION_PREVENTION.md) | Bug tracking prevention | ~150 | Workflow |

---

## STANDARDS DOCUMENTATION

**Category:** Coding and operational standards

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [INDEX.md](standards/INDEX.md) | Standards index | ~30 | Index |
| [TEST_PRODUCTION_SEPARATION.md](standards/TEST_PRODUCTION_SEPARATION.md) | Test/prod isolation | ~200 | Mandatory |
| [MODULAR_DOCUMENTATION.md](standards/MODULAR_DOCUMENTATION.md) | 500-line rule | ~100 | Standard |
| [LOGGING_PATTERNS.md](standards/LOGGING_PATTERNS.md) | Structured logging | ~150 | Standard |
| [LOGGING_INTEGRATION.md](standards/LOGGING_INTEGRATION.md) | Logging integration | ~120 | Standard |
| [API_SPECIFICATIONS.md](standards/API_SPECIFICATIONS.md) | API contract | ~180 | Standard |
| [ADVERSARIAL_TESTING.md](standards/ADVERSARIAL_TESTING.md) | Breaker agent testing | ~140 | Standard |
| [DEPENDENCY_GOVERNANCE.md](standards/DEPENDENCY_GOVERNANCE.md) | Package allow-list | ~100 | Standard |
| [INFRASTRUCTURE_STANDARDS.md](standards/INFRASTRUCTURE_STANDARDS.md) | IaC, Terraform | ~160 | Standard |
| [OPERATIONAL_PATTERNS.md](standards/OPERATIONAL_PATTERNS.md) | Health checks, circuit breakers | ~140 | Standard |
| [PROJECT_CONTEXT_TEMPLATE.md](standards/PROJECT_CONTEXT_TEMPLATE.md) | Project Bible template | ~100 | Standard |

---

## SPRIT FRAMEWORK

**Category:** Task execution framework

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [INDEX.md](sprints/INDEX.md) | Sprint index | ~20 | Index |
| [SPRINT_TEMPLATE.md](sprints/SPRINT_TEMPLATE.md) | Task execution template | ~100 | Template |
| [SPRINT_GUIDE.md](sprints/SPRINT_GUIDE.md) | How to write sprints | ~120 | Guide |
| [SPRINT_001_MCP_GAP_IMPLEMENTATION.md](sprints/SPRINT_001_MCP_GAP_IMPLEMENTATION.md) | Sprint example 1 | ~150 | Example |
| [SPRINT_002_WEB_UI_IMPLEMENTATION.md](sprints/SPRINT_002_WEB_UI_IMPLEMENTATION.md) | Sprint example 2 | ~180 | Example |
| [SPRINT_003_DOCUMENTATION_PARITY.md](sprints/SPRINT_003_DOCUMENTATION_PARITY.md) | Sprint example 3 | ~140 | Example |
| [SPRINT_005_PRECOMMIT_SAFETY.md](sprints/SPRINT_005_PRECOMMIT_SAFETY.md) | Sprint example 5 | ~130 | Example |
| [SPRINT_006_CUSTOM_ADVISOR_ROLES.md](sprints/SPRINT_006_CUSTOM_ADVISOR_ROLES.md) | Sprint example 6 | ~160 | Example |

---

## MCP SERVER

**Location:** mcp-server/

| File | Purpose | Category |
|------|---------|----------|
| README.md | MCP server overview | MCP |
| API.md | REST/API contract | MCP |
| DEPLOYMENT_GUIDE.md | Deployment instructions | MCP |

---

## EXAMPLES

**Category:** Language-specific pattern examples

### TypeScript

| Directory | Purpose | Files |
|-----------|---------|-------|
| [ui-components/](examples/typescript/ui-components/) | UI component patterns | README + 3 examples |
| [game-ui/](examples/typescript/game-ui/) | Game UI patterns | README + 3 examples |

### Python

| Directory | Purpose | Files |
|-----------|---------|-------|
| [ui-dashboard/](examples/python/ui-dashboard/) | Dashboard patterns | README + 2 examples |
| [game-tools/](examples/python/game-tools/) | Game tool patterns | README + 2 examples |

### Go

| Directory | Purpose | Files |
|-----------|---------|-------|
| [htmx-patterns/](examples/go/htmx-patterns/) | HTMX patterns | README + 3 examples |
| [admin-ui/](examples/go/admin-ui/) | Admin UI patterns | README + 2 examples |

### Rust

| Directory | Purpose | Files |
|-----------|---------|-------|
| [bevy-ui-example/](examples/rust/bevy-ui-example/) | Bevy game UI | README + 3 examples |
| [egui-overlay/](examples/rust/egui-overlay/) | egui overlay | README + 2 examples |

### Java

| Directory | Purpose | Files |
|-----------|---------|-------|
| [ui-patterns/](examples/java/ui-patterns/) | Java UI patterns | README + examples |

### Ruby

| Directory | Purpose | Files |
|-----------|---------|-------|
| [ui-patterns/](examples/ruby/ui-patterns/) | Ruby UI patterns | README + examples |

### Swift

| Directory | Purpose | Files |
|-----------|---------|-------|
| [swiftui-game/](examples/swift/swiftui-game/) | SwiftUI game patterns | README + examples |

### Scala

| Directory | Purpose | Files |
|-----------|---------|-------|
| [functional-ui/](examples/scala/functional-ui/) | Functional UI patterns | README + examples |

### R

| Directory | Purpose | Files |
|-----------|---------|-------|
| [game-analytics/](examples/r/game-analytics/) | Game analytics patterns | README + examples |

### PHP

| Directory | Purpose | Files |
|-----------|---------|-------|
| [laravel-ui/](examples/php/laravel-ui/) | Laravel UI patterns | README + examples |

---

## SECURITY

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [SECRETS_MANAGEMENT.md](../.github/SECRETS_MANAGEMENT.md) | GitHub Secrets guide | ~150 | Security |
| [SECURITY_AUDIT_API.md](security/SECURITY_AUDIT_API.md) | API security audit | ~200 | Security |
| [SECURITY_AUDIT_CODE.md](security/SECURITY_AUDIT_CODE.md) | Code security audit | ~220 | Security |
| [SECURITY_AUDIT_CONFIG.md](security/SECURITY_AUDIT_CONFIG.md) | Config security audit | ~180 | Security |
| [SECURITY_AUDIT_CONTAINERS.md](security/SECURITY_AUDIT_CONTAINERS.md) | Container security audit | ~200 | Security |
| [SECURITY_AUDIT_DATABASE.md](security/SECURITY_AUDIT_DATABASE.md) | Database security audit | ~190 | Security |
| [SECURITY_AUDIT_DEPENDENCIES.md](security/SECURITY_AUDIT_DEPENDENCIES.md) | Dependency security audit | ~160 | Security |

---

## RELEASE HISTORY

| File | Version | Date |
|------|---------|------|
| [CHANGELOG.md](CHANGELOG.md) | All versions | Current |
| [RELEASE_v1.10.0.md](RELEASE_v1.10.0.md) | v1.10.0 | 2026-02-15 |
| [RELEASE_v1.9.0.md](RELEASE_v1.9.0.md) | v1.9.0 | 2026-01-20 |
| [RELEASE_v1.9.1.md](RELEASE_v1.9.1.md) | v1.9.1 | 2026-01-22 |
| [RELEASE_v1.9.2.md](RELEASE_v1.9.2.md) | v1.9.2 | 2026-01-25 |
| [RELEASE_v1.9.3.md](RELEASE_v1.9.3.md) | v1.9.3 | 2026-01-28 |
| [RELEASE_v1.9.4.md](RELEASE_v1.9.4.md) | v1.9.4 | 2026-02-01 |
| [RELEASE_v1.9.5.md](RELEASE_v1.9.5.md) | v1.9.5 | 2026-02-05 |
| [RELEASE_v1.9.6.md](RELEASE_v1.9.6.md) | v1.9.6 | 2026-02-10 |

---

## ADVISORS

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [INDEX.md](advisors/INDEX.md) | Advisor index | ~20 | Index |
| [COST_ADVISOR.md](advisors/COST_ADVISOR.md) | Cost optimization advisor | ~100 | Advisor |
| [PRIVACY_ADVISOR.md](advisors/PRIVACY_ADVISOR.md) | Privacy compliance advisor | ~120 | Advisor |
| [RESILIENCE_ADVISOR.md](advisors/RESILIENCE_ADVISOR.md) | Resilience advisor | ~100 | Advisor |

---

## DESIGN & PLANS

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [HALT_CONDITIONS_DESIGN.md](designs/HALT_CONDITIONS_DESIGN.md) | Halt conditions design | ~150 | Design |
| [MCP_SERVER_PLAN.md](plans/MCP_SERVER_PLAN.md) | MCP server plan | ~200 | Plan |

---

## MIGRATION & CONTRIBUTING

| File | Purpose | Lines | Category |
|------|---------|-------|----------|
| [MIGRATION.md](MIGRATION.md) | Migration guide | ~150 | Migration |
| [PYTHON_MIGRATION.md](PYTHON_MIGRATION.md) | Python migration | ~180 | Migration |
| [PYTHON_TO_GO_MIGRATION.md](PYTHON_TO_GO_MIGRATION.md) | Python to Go migration | ~200 | Migration |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contributing guidelines | ~120 | Contributing |
| [TROUBLESHOOTING.md](TROUBLESHOOTING.md) | Troubleshooting guide | ~180 | Support |
| [RELEASE_TESTERS.md](RELEASE_TESTERS.md) | Release testing guide | ~100 | Testing |

---

## NAVIGATION QUICK LINKS

1. **Start here:** README.md → INDEX_MAP.md → HEADER_MAP.md
2. **Game/UI development:** 2026_GAME_DESIGN.md → 2026_UI_UX_STANDARD.md
3. **Accessibility:** ACCESSIBILITY_GUIDE.md
4. **Spatial computing:** SPATIAL_COMPUTING_UI.md
5. **Ethical review:** ETHICAL_ENGAGEMENT.md
6. **Workflow:** workflows/INDEX.md
7. **Standards:** standards/INDEX.md
8. **Sprints:** sprints/INDEX.md

---

**Last Updated:** 2026-03-14
**Version:** 2.0.0
**Document Owner:** Documentation Team
**500-Line Compliance:** All documents ≤ 500 lines