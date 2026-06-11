# Changelog

All notable changes to the Agent Guardrails Template will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased]

---

## [3.1.0] - 2026-05-12

### Release: Structural Reorganization & README Update

**Type:** Minor Version Bump (documentation organization + bug fixes)

#### Changed

- **README.md** - Comprehensive review and update for v3.1.0
  - Fixed broken 3D_GAME_DEVELOPMENT.md link (now points to `docs/game-design/3d/`)
  - Added missing 3D documents: 3D_MATHEMATICAL_FOUNDATIONS.md, 3D_MODULE_ARCHITECTURE.md, AI_DEBUGGABLE_3D_ARCHITECTURE.md, 3D_GUARDREL_PROPOSALS_V1.2.md
  - Added AI-Powered Development 2026 10-part guide series to "What's Included"
  - Added Hermes 2026: AI in 3D Game Development 9-part dossier series
  - Updated project structure to include `docs/game-design/3d/` subdirectory
  - Updated documentation file count: 44+ → 68+
  - Updated version badge: v3.0.0 → v3.1.0
  - Updated version history with v3.1.0 and v3.0.0 entries

- **Consolidated 3D game development docs** into `docs/game-design/3d/` subfolder
  - All Hermes 2026 dossier parts now live in `docs/game-design/3d/`
  - All 3D-specific guardrail documents grouped under `3d/` for logical discovery

#### Fixed

- **Broken link** in README.md Game Design section (missing `/3d/` path segment)
- **Missing documentation references** for v3.0.0 content in README.md
- **Outdated version history** showing v2.8.0 as current

---

## [3.0.0] - 2026-05-12

### Major Release: 3D Game Development & AI-Powered Development 2026

**Type:** Major Version Bump (new domain + comprehensive guide)

#### Added

- **docs/game-design/3d/3D_GAME_DEVELOPMENT.md** (416 lines) - Comprehensive 3D game development guardrails v1.0
  - AI-assisted 3D workflow guardrails, engine-agnostic patterns (Godot, Unity, Unreal)
  - Asset pipeline safety, shader generation constraints, physics deterministic rules
  - Performance budgets for 3D: draw calls, poly counts, texture memory
  - XR/VR 3D specific: comfort zones, locomotion safety, spatial audio

- **AI-Powered Development 2026: 10-Part Guide Series** (~3,023 lines total) - From Intro to Master
  - Split into modular 500-line-compliant parts for agent-friendly navigation
  - Part 1: Introduction & Foundations (Ch 1–2) | Part 2: Prompt Engineering | Part 3: Context & Iteration
  - Part 4: Quality & Architecture | Part 5: Legacy & Agents | Part 6: Building Agents & Tool Use
  - Part 7: Multi-Agent Systems | Part 8: Security, Ethics & Future | Part 9: Appendices A–C | Part 10: Appendix D (MoA)
  - Covers agent ecosystems, neural engines, vibe coding, MoA, swarm intelligence, responsible AI

- **AI in 3D Game Development 2026: 9-Part Dossier Series** (~1,015 lines total) - The 2026 Dossier
  - Split into modular 500-line-compliant parts for agent-friendly navigation
  - Part 1: Introduction & Executive Summary | Part 2: Asset Generation & Engine Integration
  - Part 3: World Generation & Neural Rendering | Part 4: NPCs, Dialogue & Animation
  - Part 5: Code Generation & Neural Physics | Part 6: QA, Testing & Business Landscape
  - Part 7: Legal, Ethics & Case Studies | Part 8: Technology Deep-Dives & Future Outlook | Part 9: Appendices
  - Live research synthesis: Ollama Search API + parallel agent analysis + domain expertise

- **docs/game-design/3d/3D_MATHEMATICAL_FOUNDATIONS.md** (290 lines) - 3D Mathematical Foundations for Game Development
  - Linear algebra, trigonometry, spatial geometry reference for AI agents
  - Matrix operations, quaternion math, vector geometry, collision math
  - Pre-validated formulas agents can apply without derivation

- **docs/game-design/3d/3D_MODULE_ARCHITECTURE.md** (237 lines) - 3D Game Design Module Architecture
  - Blueprint bridging LLM capabilities with deterministic 3D rendering/physics
  - Module boundaries, data flow, state management for 3D engines
  - Agent-safe architecture patterns for autonomous 3D code generation

- **docs/game-design/3d/AI_DEBUGGABLE_3D_ARCHITECTURE.md** (302 lines) - AI-Debuggable 3D Game Architecture
  - Patterns enabling AI agents to troubleshoot 3D game features autonomously
  - Constraint-aware design: AI is blind to visual feedback, limited by context window
  - Debug instrumentation, assertion patterns, self-validating 3D systems

- **docs/game-design/3d/3D_GUARDREL_PROPOSALS_V1.2.md** (201 lines) - v1.2 Proposed Additions
  - Draft proposals from Hermes 2026 AI Dossier review by parallel subagents
  - Experimental guardrails for neural radiance fields, procedural geometry, AI NPCs

#### Changed

- **README.md** - Version badge updated to v3.0.0, added 3D game development references
- **INDEX_MAP.md** - Added 19 new entries for split AI 2026 and Hermes dossier parts, updated counts (86 → 103 docs)
- **HEADER_MAP.md** - Added section-level mappings for all 19 split document parts; removed monolithic entries
- **TOC.md** - Added split document parts, updated file totals (51 → 68 docs, 103 → 120 total files)

---

## [2.8.0] - 2026-03-14

### Major Release: AI-First Reframe & Gap Remediation

**Type:** Major Version Bump (new documents + framework reframing)

#### Philosophy Change

- **Reframed entire framework** as AI-first rapid development enablement
- Core message: "Guardrails are your license to move fast"
- Vibe coding philosophy introduced across all documents
- Constraints repositioned as speed enablers, not restrictions

#### Added

- **docs/ai-dev/AI_ASSISTED_DEV.md** - Centerpiece: AI development patterns, decision matrix, vibe coding workflow
- **docs/state/STATE_MANAGEMENT.md** - State architecture decision tree, client/server/offline/CRDT patterns
- **docs/generative/GENERATIVE_ASSET_SAFETY.md** - AI content disclosure, C2PA metadata, procedural generation safety
- **docs/monetization/MONETIZATION_GUARDRAILS.md** - IAP ethics, loot box transparency, virtual economy balance
- **docs/multiplayer/MULTIPLAYER_SAFETY.md** - Social safety, chat moderation, matchmaking fairness, CSAM detection
- **docs/analytics/ANALYTICS_ETHICS.md** - Consent tiers, data minimization, A/B testing ethics
- **docs/deployment/CROSS_PLATFORM_DEPLOYMENT.md** - App store compliance matrix, CI/CD, feature flags, progressive rollout
- **skills/shared-prompts/vibe-coding.md** - Canonical vibe coding principles (5 principles)
- **examples/flutter/cross-platform/** - Flutter guardrails: config, ethical widgets, accessibility wrappers
- **examples/gdscript/godot-game/** - Godot GDScript: comfort zones, ethical UI, accessibility manager

#### Changed

- **README.md** - Repositioned as AI-first rapid development framework, added "The Paradox" section, vibe coding context
- **CLAUDE.md** - Added Vibe Coding Philosophy, speed-first Token-Saving Rules framing
- **QUICK_SETUP.md** - Added "What You Can Now Do" section, velocity-first subtitle
- **docs/AGENT_GUARDRAILS.md** - Reframed Four Laws as speed enablers, added "How These Laws Enable Rapid Development"
- **PROMPTING_GUIDE.md** - Added "Rapid Development Patterns (Vibe Coding)" section with 4 prompt patterns
- **docs/game-design/2026_GAME_DESIGN.md** - Reframed as AI enablement, added "AI-Optimized Development"
- **docs/ui-ux/2026_UI_UX_STANDARD.md** - Added "AI Generation Optimization" section
- **docs/accessibility/ACCESSIBILITY_GUIDE.md** - Reframed for AI-generated components, Agent-GDUI-2026 enforcement
- **docs/spatial/SPATIAL_COMPUTING_UI.md** - Added "AI-Driven Spatial Development" section
- **docs/ethical/ETHICAL_ENGAGEMENT.md** - Added "Automated Ethical Review" section
- **INDEX_MAP.md** - Added 10 new entries for new docs and examples
- **HEADER_MAP.md** - Added section-level mappings for all 7 new docs
- **TOC.md** - Added new document categories and updated totals

---

## [2.7.0] - 2026-03-14

### Major Release: 2026 UI/UX & Game Design Update

**Type:** Major Version Bump (breaking changes in documentation structure)

#### Added

- **Agent-GDUI-2026** role - Specialized agent for game design, spatial computing, UI/UX
- **docs/game-design/2026_GAME_DESIGN.md** - Game design guardrails, XR/VR comfort zones, platform performance budgets
- **docs/ui-ux/2026_UI_UX_STANDARD.md** - UI/UX component patterns, design tokens, animation, responsive breakpoints
- **docs/accessibility/ACCESSIBILITY_GUIDE.md** - WCAG 3.0+ conformance (Bronze/Silver/Gold), perceptual/cognitive/physical accessibility
- **docs/spatial/SPATIAL_COMPUTING_UI.md** - XR/VR/AR/MR layout patterns, comfort zones, latency requirements, depth layering
- **docs/ethical/ETHICAL_ENGAGEMENT.md** - Dark pattern taxonomy and prevention, ethical design principles
- **Four Laws of Spatial Safety** - Comfort First, Accessibility Required, Performance Bound, Ethical Engagement
- **Scala examples** (`examples/scala/functional-ui/`) - Functional composition, type-safe CSS, DDA telemetry
- **R examples** (`examples/r/game-analytics/`) - ggplot2 4.0+, Shiny 2.0+, ethics auditing, retention analysis

#### Changed

- **CLAUDE.md** - Added Agent-GDUI-2026 initialization context and quick links
- **INDEX_MAP.md** - Added 2026 document entries, categories, and cross-references
- **HEADER_MAP.md** - Added section-level mappings for all 2026 documents

---

## [Unreleased]

### Added

- **QUICK_SETUP.md** - 5-minute setup guide for getting started quickly
  - TL;DR 3-step quick start
  - Detailed setup instructions for all AI tools
  - What happens automatically explanation
  - Daily usage patterns
  - Troubleshooting section
  - Configuration examples

- **PROMPTING_GUIDE.md** - Master guide for writing effective prompts
  - Golden rules for prompting
  - 5 prompt templates (Feature, Bug Fix, Code Review, Refactoring, Documentation)
  - 5 common patterns with examples
  - 5 advanced techniques
  - Examples by use case (API, Frontend, Database, DevOps)
  - Anti-patterns to avoid
  - Troubleshooting section

### Changed

- **README.md** - Updated with new guides in Documentation section
  - Added QUICK_SETUP.md and PROMPTING_GUIDE.md to Core Documents table
  - Updated "Start Here" section with links to new guides
  - Added star indicators (⭐) for most important documents

- **INDEX_MAP.md** - Added entries for new documents
  - quick-setup → QUICK_SETUP.md
  - prompting → PROMPTING_GUIDE.md

- **TOC.md** - Added new files to Root Files section
  - QUICK_SETUP.md: 5-minute setup guide
  - PROMPTING_GUIDE.md: Master prompting techniques

---

## [2.9.0] - 2026-05-08

### Major Release: Core Skill Systems for All Coding Platforms

**Type:** Minor Version Bump (new skills, MCP tool, install workflow)

#### Added

- **Pre-committed Skill Configs** — Skill files for Claude Code, Cursor, OpenCode, Windsurf, and GitHub Copilot now live in the repo root (`.claude/`, `.cursor/`, `.opencode/`, `.windsurfrules`, `.github/copilot-instructions.md`). No more generation at install time.

- **Shared Skill Prompts** (`skills/shared-prompts/`) — 4 new canonical prompt files:
  - `error-recovery.md` — Recovery protocol for failures without making things worse
  - `three-strikes.md` — Track failure attempts, halt at 3 strikes
  - `production-first.md` — Production code before test/infrastructure
  - `scope-validation.md` — File scope authorization and scope creep detection

- **Claude Code Skills** (`.claude/skills/`) — 7 JSON skills:
  - `guardrails-enforcer.json`, `commit-validator.json`, `env-separator.json`
  - `scope-validator.json`, `production-first.json`, `three-strikes.json`, `error-recovery.json`

- **Claude Code Hooks** (`.claude/hooks/`) — 3 shell hooks:
  - `pre-commit.sh` — AI attribution, no secrets, .env check
  - `pre-execution.sh` — Guardrails preamble on operation start
  - `post-execution.sh` — Post-modification secret/error validation

- **Cursor Rules** (`.cursor/rules/`) — 3 markdown rules:
  - `guardrails-enforcer.md`, `production-first.md`, `three-strikes.md`

- **Windsurf Rules** (`.windsurfrules`) — Full guardrails preamble for Windsurf

- **OpenCode Config** (`.opencode/`) — oh-my-opencode.jsonc + skills + 2 agents (`guardrails-auditor`, `doc-indexer`)

- **GitHub Copilot Instructions** (`.github/copilot-instructions.md`) — Repo-level Copilot guidance

- **`guardrail_install_skills` MCP Tool** — Headless setup via MCP with support for:
  - Full platform install (`platforms` arg)
  - Per-skill install (`skill` arg, `action=install`)
  - Single-file clone from GitHub (`path` arg, `action=clone`)
  - List skills/platforms (`list_skills`, `list_platforms`)

#### Changed

- **`scripts/setup_agents.py`** — Refactored from "generate" to "install":
  - `--clone PATH` — Download single file from GitHub raw (branch-aware fallback)
  - `--install-skill NAME` — Install one skill by name (23 skills registered)
  - `--list-skills` — List all available skills with repo paths
  - `--list-platforms` — List all available platforms
  - `--dry-run` — Preview before installing
  - `--mode copy|symlink` — Copy files or symlink to repo source
  - Removed `--claude`, `--opencode`, `--minimal`, `--full` flags

- **`docs/AGENTS_AND_SKILLS_SETUP.md`** — Rewritten for new pre-committed install workflow

- **`docs/INDEX_MAP.md`** — Added AI TOOL INTEGRATION and SHARED SKILL PROMPTS sections

- **`docs/HEADER_MAP.md`** — Added AGENTS_AND_SKILLS_SETUP.md and SHARED SKILL PROMPTS sections

#### Merged from Main

- PR #3: SSE timeout resilience fix
- Sprint 005: Pre-Commit Safety Suite
- Sprint 006: Custom Advisor Roles System
- v2.0.0 / v2.7.0 / v2.8.0 releases
- IDE extensions (VS Code, JetBrains, Neovim, Vim)
- Team management tools (Python → Go migration)

---

## [2.6.0] - 2026-02-15

### Migrated

- **Python to Go Migration Complete** - All team management functionality migrated from Python to Go
  - `scripts/team_manager.py` → `mcp-server/internal/team/` package
  - `scripts/encryption.py` → `mcp-server/internal/team/encryption.go`
  - `scripts/batch_operations.py` → Integrated into Go team package
  - Container now uses `distroless/static` (no Python runtime needed)
  - **Benefits:** 4x smaller container, 10x faster startup, no Python dependencies

### Added

- **Go Team Package** (`mcp-server/internal/team/`)
  - `manager.go` - Core team management (init, assign, unassign, status)
  - `encryption.go` - Fernet encryption at rest (ported from Python)
  - `validation.go` - Input validation (project names, roles, persons)
  - `rules.go` - Team layout rules and phase gates
  - `metrics.go` - Team operation metrics collection
  - `types.go` - Team data structures and interfaces
  - `migrations.go` - Data migration utilities

- **Migration Documentation**
  - `docs/PYTHON_TO_GO_MIGRATION.md` - Complete migration guide for developers
  - API compatibility matrix (Python → Go function mapping)
  - Container deployment changes
  - Troubleshooting guide

### Changed

- **Container Image** - Now uses `gcr.io/distroless/static:nonroot`
  - Removed Python runtime requirement
  - No `scripts/` volume needed
  - Smaller attack surface

- **Environment Variables**
  - Removed: `PYTHONPATH`, `TEAM_MANAGER_SCRIPT`
  - Kept: `TEAM_ENCRYPTION_KEY` (still used by Go implementation)

### Deprecated

- **Python Scripts** - `scripts/team_manager.py` and related Python files
  - Deprecated as of v2.6.0
  - Will be removed in v3.0.0
  - Use Go `team` package instead

### Compatibility

- **MCP Tool API** - Fully backward compatible
  - All tool names unchanged
  - All parameters unchanged
  - All responses identical format

- **Data Format** - No migration needed
  - `.teams/*.json` files remain compatible
  - Existing projects work without changes

---

## [1.12.0] - 2026-02-12

### Added

- **OpenCode MCP Remote Configuration** - Complete setup documentation for remote MCP server connections
  - Added port mapping clarification (internal vs external ports)
  - Documented OpenCode `.opencode/oh-my-opencode.jsonc` MCP server configuration
  - Added troubleshooting section for port confusion and authentication errors
  - Provided working example with correct `Authorization: Bearer` header format

- **Comprehensive README Documentation** - Added complete "How to Use This Platform" section
  - Quick start guides for AI Agent Developers, DevOps teams, and Development Teams
  - Detailed MCP tools documentation with practical examples
  - Common use cases with real-world scenarios (preventing production accidents, code review, test/prod separation)
  - Web UI walkthrough covering Dashboard, Documents, Rules, and Failure Registry
  - Integration examples for GitHub Actions, VS Code, and custom Python client
  - Enhanced troubleshooting section with MCP-specific issues

### Changed

- **README.md MCP Section** - Enhanced clarity for Docker/Podman deployment
  - Added explicit port mapping table showing internal (8080/8081) vs external (8094/8095) ports
  - Clarified which ports to use in different contexts
  - Updated troubleshooting to include port confusion guidance

- **OPENCODE_INTEGRATION.md** - Added comprehensive MCP server configuration section
  - Documented `mcpServers` JSONC configuration format
  - Clarified `type: "remote"` vs local MCP servers
  - Added verification commands for testing MCP connectivity

---

## [1.10.0] - 2026-02-08

### Added

- **MCP Gap Implementation** - 5 new MCP tools for agent safety
  - `guardrail_validate_scope` - Check if file path is within authorized scope
  - `guardrail_validate_commit` - Validate conventional commit format
  - `guardrail_prevent_regression` - Check failure registry for pattern matches
  - `guardrail_check_test_prod_separation` - Verify test/production isolation
  - `guardrail_validate_push` - Validate git push safety conditions

- **MCP Documentation Resources** - 6 new MCP resources for documentation access
  - `guardrail://docs/agent-guardrails` - Core safety protocols
  - `guardrail://docs/four-laws` - Four Laws of Agent Safety
  - `guardrail://docs/halt-conditions` - When to stop and ask
  - `guardrail://docs/workflows` - Workflow documentation index
  - `guardrail://docs/standards` - Standards documentation index
  - `guardrail://docs/pre-work-checklist` - Pre-work regression checklist

- **Web UI Management Interface** - Complete SPA for guardrail management
  - Dashboard with system stats and health status
  - Documents browser (CRUD + full-text search)
  - Rules management (CRUD + toggle switches)
  - Projects management with context editing
  - Failure registry viewer with status updates
  - IDE Tools validation interface
  - 26 API endpoints implemented in JavaScript client

- **Documentation Parity** - Organized 73 markdown files
  - Consolidated "Four Laws" to canonical source
  - Extracted 10 actionable prevention rules to JSON
  - Created document ingestion script for full-text search
  - Added MCP resource handlers for all critical docs

### Changed

- **INDEX_MAP.md** - Updated with new sprint documents and canonical sources
- **docs/AGENT_GUARDRAILS.md** - Now references canonical Four Laws

### Fixed

- **MCP Server Build** - Fixed syntax errors in server.go
  - Removed unsupported Required fields from ToolInputSchema
  - Fixed missing closing brace for ListToolsResult struct

---

## [1.9.6] - 2026-02-08

### Fixed

- **MCP SSE Compatibility** - Restored compatibility with Crush and Go SDK clients
  - SSE keepalive now uses comments (`: ping`) instead of custom event payloads
  - Server now streams JSON-RPC responses as `event: message` over SSE
  - Session-bound queueing prevents response loss on concurrent requests

### Changed

- **Container Build** - Runtime image now includes Web UI static assets (`/app/static`)
- **Web API Access** - Read-only routes for documents/rules/version are publicly browsable

### Documentation

- Updated root README and MCP server README for session_id-based MCP message flow
- Added release notes document: `docs/RELEASE_v1.9.6.md`

## [1.9.5] - 2026-02-08

### Final Production Polish

- **Code Consistency** - Standardized patterns across all packages
- **Edge Case Handling** - Added boundary condition checks
- **Technical Debt** - Cleaned up TODOs and FIXMEs
- **Final Security Review** - Verified all security measures

### Production Readiness

- **Configuration** - Verified all defaults are production-appropriate
- **Graceful Shutdown** - Improved shutdown sequence
- **Health Checks** - Accurate readiness/liveness probes
- **Resource Limits** - CPU/memory quotas configured

### Integration Testing

- **Build Verification** - Clean build with no errors
- **Test Coverage** - Added document model tests
- **Error Scenarios** - Verified error handling paths
- **Shutdown Behavior** - Tested graceful termination

### Documentation

- **API.md** - Updated to match implementation
- **README** - Verified accuracy
- **Deployment Guides** - Reviewed and corrected
- **Release Notes** - Complete for all versions

### Code Quality

- **Formatting** - All files formatted with gofmt
- **Imports** - Optimized and organized
- **Linting** - All lint checks pass
- **Unused Code** - Removed dead code

## [1.9.4] - 2026-02-07

### Performance

- **SSE Optimizations** - strings.Builder, pre-allocated buffers, reduced allocations
- **JSON Encoding** - Buffer pool for JSON marshaling
- **Database Queries** - Optimized document/rule/project queries

### Error Handling

- **Fixed Silent Failures** - GetByID, Count errors now properly handled
- **Error Wrapping** - All errors wrapped with context using %w
- **HTTP Status Codes** - 404 for not found, 500 for server errors
- **Panic Recovery** - Added recovery middleware with metrics

### Configuration

- **Fixed Env Var Naming** - RATE_LIMIT_MCP, RATE_LIMIT_IDE (was MCP_RATE_LIMIT, IDE_RATE_LIMIT)
- **Feature Flags** - Added ENABLE_METRICS, ENABLE_AUDIT_LOGGING, ENABLE_CACHE
- **CORS Config** - Added CORS_ALLOWED_ORIGINS, CORS_MAX_AGE

### Observability

- **Panic Metrics** - Track recovered panics by path
- **Database Metrics** - Connection pool stats, query duration
- **SLO Metrics** - Compliance, error budget burn rate, SLI values
- **Correlation ID** - Request tracing middleware

### API Consistency

- **Route Ordering** - Fixed search routes before parameterized routes
- **Response Formats** - Standardized across all endpoints

## [1.9.3] - 2026-02-07

### Security

- **CORS Origin Validation** - Replaced wildcard CORS with configurable origin validation
- **Secure Session ID Generation** - Uses crypto/rand instead of timestamp-based IDs
- **Security Headers** - Added X-Content-Type-Options, X-Frame-Options, X-XSS-Protection, Referrer-Policy
- **Request Size Limits** - Added 1MB body size limit to prevent DoS
- **Path Traversal Protection** - Added slug validation to prevent path traversal attacks

### Fixed

- **SQL Injection Vulnerabilities** - Fixed dynamic query building in List methods
- **Redis Blocking Commands** - Replaced KEYS command with non-blocking SCAN
- **Context Timeouts** - Added 5-second timeouts to cache operations
- **Session Memory Leaks** - Added cleanup goroutine for inactive sessions
- **MCP Protocol Compliance** - SSE endpoint now sends full URLs, proper JSON-RPC ping format
- **JSON-RPC Validation** - Added session_id and JSON-RPC version validation

### Added

- **Transaction Support** - All Create/Update/Delete operations now use transactions
- **Model Validation** - Validate() methods for all models (Document, Rule, Project, Failure)
- **Database Migrations** - Migration system with schema versioning
- **Connection Pool Monitoring** - Pool health monitoring with capacity warnings
- **Graceful Shutdown** - Configurable shutdown timeout with SIGQUIT support
- **Kubernetes Deployment** - Complete K8s manifests with HPA and PDB
- **API Documentation** - Comprehensive API.md with all REST endpoints
- **MCP Server CHANGELOG** - Separate changelog for MCP server

### Infrastructure

- **Dockerfile Improvements** - Version injection, CA certificates
- **Health Checks** - Liveness, readiness, and startup probes
- **Observability** - /version endpoint, Prometheus metrics, optional pprof
- **Resource Limits** - CPU/memory limits for all services

### Changed

- **MCP Server Documentation** - Enhanced README with security features and troubleshooting
- **Environment Configuration** - Fixed defaults in .env.example to match deployment

- **MCP Server README.md**
  - Added complete project structure including `internal/mcp/`
  - Expanded API endpoints documentation with all routes
  - Added database migration section
  - Added comprehensive security features documentation
  - Added development commands (fmt, lint, vuln)
  - Added troubleshooting section

- **MCP Server .env.example**
  - Reorganized with better section headers
  - Added profiling configuration options
  - Added health check timeout configuration
  - Added build information variables
  - Improved documentation for each setting

---

### Added

## [1.9.2] - 2026-02-07

### Fixed

- **Web UI Authentication** - Removed API key requirement for Web UI routes
  - Web UI (port 8093) is now publicly accessible without authentication
  - Added skip logic for `/`, `/index.html`, and `/static/*` routes
  - API endpoints still require valid API key
  - Health checks and metrics remain unauthenticated

## [1.9.1] - 2026-02-07

### Fixed

- **SSE Compatibility** - Fixed EOF errors with non-interactive clients
  - Added `WriteHeader(http.StatusOK)` for immediate header commit
  - Added `X-Accel-Buffering: no` for proxy compatibility
  - Added `Access-Control-Allow-Origin: *` for CORS
  - Send immediate ping event after endpoint to prevent client timeout
  - Better error handling on write/flush operations

- **PostgreSQL Array Scanning** - Fixed TEXT[] array scanning bug
  - Changed `AffectedFiles` from `pq.StringArray` to `pgtype.Array[string]`
  - Added `ToStringSlice()` and `ToTextArray()` helper functions
  - Compatible with pgx v5 driver

### Documentation

- **README.md** - Complete rewrite with MCP Server documentation
  - Installation and testing instructions
  - Environment variable reference
  - curl test examples
  - Deployment guide for production servers

## [1.9.0] - 2026-02-07

### Added

- **MCP Server** - Full Model Context Protocol implementation
  - `mcp-server/` - Complete Go-based MCP server
  - `mark3labs/mcp-go` v0.4.0 for protocol implementation
  - SSE transport for real-time client communication
  - Tools: `guardrail_init_session`, `guardrail_validate_bash`,
    `guardrail_validate_file_edit`, `guardrail_validate_git_operation`,
    `guardrail_pre_work_check`, `guardrail_get_context`
  - Resources: `guardrail://quick-reference`, `guardrail://rules/active`

- **Web UI** - Browser-based guardrail management
  - Document CRUD operations
  - Prevention rule management
  - Failure registry viewer
  - Project configuration

- **Production Deployment** - RHEL + Podman environment
  - PostgreSQL 16 for data persistence
  - Redis 7 for caching and rate limiting
  - Multi-stage Docker build with distroless image
  - Security hardening: non-root user (65532), read-only filesystem,
    dropped capabilities, SELinux labels

### Changed

- **Server Binding** - Changed from `127.0.0.1` to `0.0.0.0` for containerized deployment
- **Go Version** - Upgraded to Go 1.23.2 for mcp-go compatibility

### Infrastructure

- Example endpoints:
  - MCP: `http://localhost:8092`
  - Web UI: `http://localhost:8093`

## [1.8.0] - 2026-02-05

### Added

- Placeholder for v1.8.0 changes

## [1.7.0] - 2026-02-01

### Added

- **Claude Code Integration** - Full support for Claude Code skills and hooks
  - `scripts/setup_agents.py` - CLI tool to generate configurations
  - Skills: guardrails-enforcer, commit-validator, env-separator
  - Hooks: pre-execution, post-execution, pre-commit
  - `docs/CLCODE_INTEGRATION.md` - Complete setup guide

- **OpenCode Integration** - Full support for OpenCode agents and skills
  - `.opencode/oh-my-opencode.jsonc` configuration template
  - Skills: guardrails-enforcer, commit-validator, env-separator
  - Agents: guardrails-auditor, doc-indexer
  - `docs/OPENCODE_INTEGRATION.md` - Complete setup guide

- **Shared Prompts** - Reusable prompt components
  - `skills/shared-prompts/four-laws.md` - The Four Laws of Agent Safety
  - `skills/shared-prompts/halt-conditions.md` - When to stop and ask

- **Script-Based Workflows** - Documentation for large-scale operations
  - `docs/AGENTS_AND_SKILLS_SETUP.md` - Main setup guide
  - Large code review script examples
  - Batch execution with guardrails compliance
  - CI/CD integration patterns

- **Navigation Updates**
  - Updated `INDEX_MAP.md` with new AI Tools Integration category
  - Updated `TOC.md` with 3 new documents
  - Added scripts/ directory to navigation

### Changed

- **README.md** - Updated version to v1.7.0

### Statistics

- Documentation files: 28 → 31 (+3)
- New code files: 1 (setup_agents.py)
- New shared resources: 2 (prompt files)
- Total new files: 6

## [1.6.0] - 2026-01-18

### Added

- **TOC.md** - Comprehensive table of contents with file listings
  - Complete catalog of all 85 documents in the template
  - Organized by category (standards, workflows, examples, etc.)
  - Includes statistics: total files, category breakdowns, compliance status
  - Separate from README for cleaner navigation

### Changed

- **README.md** - Rewritten for clarity on what the Agent Guardrails Template is
  - Now clearly explains "What Is This?" concept
  - Better project overview and quick start guide
  - Improved from 220 to 320 lines for better readability
  - Clearer problem/solution overview
- **INDEX_MAP.md** - Added `toc` and `examples` keywords to Quick Lookup Table
  - Updated document counts (21 → 28 docs)
  - Updated all "Last Updated" dates
- **HEADER_MAP.md** - Added TOC.md and CHANGELOG.md sections
  - Updated status and last updated dates

### Improved

- Documentation clarity: README now clearly explains the template's purpose
- Discoverability: Separate TOC.md makes finding specific documentation easier
- Navigation: Updated maps reflect new TOC document
- User experience: Better first-impression for new visitors

### Statistics

- Documentation files: 28 → 28 (+0, reorganized)
- README lines: 220 → 320 (+100, +45%)
- TOC.md lines: 0 → ~350 (+350)
- Total documents cataloged: 85 files

---

## [1.5.0] - 2026-01-18

### Added

- CHANGELOG.md - Centralized release notes archive
- Examples directory with guardrails implementation examples in multiple languages
- Comprehensive release notes archiving from GitHub releases

### Changed

- All release notes now centralized in this CHANGELOG.md file
- GitHub releases now reference this file for full release notes

---

## [1.4.0] - 2026-01-16

### Added

- **docs/HOW_TO_APPLY.md** (432 lines) - Comprehensive guide with example AI agent prompts
  - Option A: Apply to existing repository detailed steps
  - Option B: Example AI agent prompts (5 ready-to-use prompts)
  - Option C: Create new repository with standards
  - Option D: Migrate existing documentation to guardrails structure
  - Verification checklist
- `how-to-apply` keyword to INDEX_MAP.md for easy discovery

### Changed

- **README.md** restructured for 500-line compliance
  - Reduced from 621 lines to 219 lines (65% reduction)
  - Quick start options link to detailed HOW_TO_APPLY guide
  - Preserved Template Contents and PROJECT README TEMPLATE

### Improved

- Token efficiency: 65% fewer tokens needed to read README
- Documentation organization: Better hierarchy with dedicated HOW_TO_APLY.md
- Agent-friendly prompts: Copy-paste ready prompts for common tasks
- Faster onboarding: Ready-to-use prompts reduce ambiguity

### Statistics

- Documentation files: 20 → 21 (+1)
- README lines: 621 → 219 (-402, -65%)
- HOW_TO_APPLY.md lines: 0 → 432 (+432)
- 500-line compliance: 17/20 → 21/21 (100%)

---

## [1.3.0] - 2026-01-16

### Added

- **docs/standards/TEST_PRODUCTION_SEPARATION.md** (558 lines) - Mandatory test/production isolation standard
  - Three Laws of Test/Production Separation
  - Environment separation requirements (databases, services, users)
  - Mandatory pre-code checklist
  - Code creation sequence (production first, then test)
  - Uncertainty handling protocol (always ask user)
  - CI/CD blocking checks
  - Examples, patterns, and anti-patterns
- **docs/workflows/AGENT_EXECUTION.md** (374 lines) - Execution protocol and rollback procedures
  - Standard task flow (5 phases)
  - Decision matrix
  - Rollback procedures (immediate, post-commit, post-push)
  - Commit message format
  - Error handling protocols
  - Verification checklists
- **docs/workflows/AGENT_ESCALATION.md** (413 lines) - Audit requirements and escalation procedures
  - Audit log requirements (what to log)
  - Log format standards
  - When to escalate to human
  - How to escalate (templates and scenarios)
  - Agent-specific guidelines (by category)
  - Compliance and violation reporting

### Changed

- **docs/AGENT_GUARDRAILS.md** - Restructured from 626 lines to 267 lines for 500-line compliance
  - Split into 3 focused documents
  - Added Test/Production Separation Rules section
  - CORE GUARDRAILS section retained
- **docs/workflows/CODE_REVIEW.md** - Added test/production separation review items
- **docs/sprints/SPRINT_TEMPLATE.md** - Added safety checks for completion gate
- **docs/workflows/INDEX.md** - Updated to 10 documents
- **docs/standards/INDEX.md** - Updated to 5 documents

### Security

- **CRITICAL:** All AI agents must verify test/production separation before deployment
- **BLOCKING VIOLATIONS** that halt deployment:
  - Deploying test code to production environment
  - Using production database for tests
  - Creating test users in production database
  - Writing test code that imports production secrets
  - Using production services for test execution
  - Sharing user accounts across environments

### Breaking Changes

- **MANDATORY:** All AI agents must now comply with test/production separation requirements
- Agents must ask user when uncertain about test/production boundaries
- Blocking violations prevent deployment when separation requirements not met

### Statistics

- Documentation files: 17 → 20 (+3)
- AGENT_GUARDRAILS.md: 626 → 267 lines (-359 lines)
- Total documentation lines: ~1,500 → 2,672 (+1,172)
- All documents now comply with 500-line maximum rule

---

## [1.1.0] - 2026-01-15

### Added

- Universal Agent Support framework
- By-Category Agent Guidelines covering:
  - Commercial API-Based Models (Claude, GPT, Gemini, Command R)
  - Open Source / Self-Hosted Models (LLaMA, Mistral, Qwen, DeepSeek, Phi, Falcon)
  - Multimodal Models (GPT-4V, Gemini Pro Vision, Claude 3, LLaVA)
  - Reasoning / Chain-of-Thought Models (o1, o3, DeepSeek-R1)
  - Agent Frameworks (CrewAI, LangChain, AutoGPT, LangGraph, Semantic Kernel)
- Model Compatibility Note section
- 30+ major LLM families explicitly supported
- All future models supported via generic patterns

### Changed

- **docs/AGENT_GUARDRAILS.md** - Major restructure
  - Replaced model-specific sections with category-based approach
  - Added Universal Requirements section for ALL LLMs and AI agents
  - Applicability table expanded with new model types
  - Enhanced compliance section

### Improved

- Scalability: Framework now supports any current or future AI model
- Maintenance: Category-based approach easier to maintain than model-specific
- Coverage: 99%+ of AI agents covered by category system

---

## [1.0.0] - 2026-01-14

### Added

- Initial stable release of Agent Guardrails Template
- **Core Documentation:**
  - docs/AGENT_GUARDRAILS.md (626 lines) - Mandatory safety protocols for all AI agents
- **Sprint Framework:**
  - docs/sprints/SPRINT_TEMPLATE.md - Task execution template
  - docs/sprints/SPRINT_GUIDE.md - How to write effective sprint documents
  - docs/sprints/INDEX.md - Sprint navigation
- **Workflow Documentation** (8 comprehensive guides):
  - TESTING_VALIDATION.md - Validation protocols
  - COMMIT_WORKFLOW.md - Commit guidelines
  - GIT_PUSH_PROCEDURES.md - Push safety procedures
  - BRANCH_STRATEGY.md - Git branching conventions
  - ROLLBACK_PROCEDURES.md - Recovery operations
  - MCP_CHECKPOINTING.md - MCP server integration
  - CODE_REVIEW.md - Code review process
  - DOCUMENTATION_UPDATES.md - Post-sprint doc updates
- **Standards Documentation** (4 guides):
  - MODULAR_DOCUMENTATION.md - 500-line max rule
  - LOGGING_PATTERNS.md - Array-based logging format
  - LOGGING_INTEGRATION.md - External logging hooks
  - API_SPECIFICATIONS.md - OpenAPI/OpenSpec guidance
- **GitHub Integration:**
  - .github/SECRETS_MANAGEMENT.md - GitHub Secrets guide
  - .github/workflows/ (3 CI/CD workflows)
  - .github/PULL_REQUEST_TEMPLATE.md - PR template with AI attribution
  - .github/ISSUE_TEMPLATE/bug_report.md - Bug report template
- **Navigation Maps:**
  - INDEX_MAP.md - Master navigation, find docs by keyword
  - HEADER_MAP.md - Section-level lookup
  - CLAUDE.md - Claude Code CLI guidelines
  - .claudeignore - Token-saving ignore rules

### Features

- Four Laws of Agent Safety
- Pre-Execution Checklist
- Git Safety Rules (8 rules)
- Code Safety Rules (7 rules)
- Guardrails: HALT CONDITIONS, FORBIDDEN ACTIONS, SCOPE BOUNDARIES
- Standard Task Flow (5 phases)
- Rollback Procedures (3 scenarios)
- Commit Message Format with conventions
- Error Handling Protocols (4 scenarios)
- Verification Checklist (pre-completion)
- Agent-Specific Guidelines for all major AI systems
- Audit Requirements
- Escalation Procedures

---

## Version Management

### Version Numbering

This project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html):

- **MAJOR**: Incompatible API changes
- **MINOR**: Backwards-compatible functionality additions
- **PATCH**: Backwards-compatible bug fixes

### Release Process

1. Complete all changes
2. Test and validate
3. Commit changes with conventional commit message
4. Update CHANGELOG.md
5. Create version tag: `git tag v1.X.X`
6. Push tag: `git push origin v1.X.X`
7. Create GitHub release with `gh release create`

---

## Links

- **Releases:** [GitHub Releases](https://github.com/TheArchitectit/agent-guardrails-template/releases)
- **Documentation:** [INDEX_MAP.md](INDEX_MAP.md)
- **Issues:** [GitHub Issues](https://github.com/TheArchitectit/agent-guardrails-template/issues)
