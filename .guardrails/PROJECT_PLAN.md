# Project Sentinel: Comprehensive Implementation Plan

**Version:** 3.0.0-Enterprise
**Status:** Ready for Development
**Total Estimated Effort:** 370k tokens (~$20-30 in API costs)
**Target Release:** Q2 2026

---

## Executive Summary

Project Sentinel is an active governance layer for Autonomous AI Agents. Unlike passive templates, Sentinel uses a compiled MCP server to physically enforce safety, security, and financial guardrails.

**Core Value Proposition:**
- **Active Enforcement**: Transforms "soft laws" into "hard physics"
- **Financial Governance**: Token budgeting prevents runaway API costs
- **Security**: VFS Jail prevents access to .env, ~/.ssh, and other restricted paths
- **Polyglot Support**: Native toolchains for 12+ programming languages

---

## Architecture Overview

### The Four Pillars

1. **Cortex (State Machine)**
   - Finite State Machine: IDLE → PLANNING → ACTIVE → REVIEW → RELEASE
   - One-task-at-a-time enforcement
   - State-based tool availability (deploy only available in RELEASE)

2. **Jailor (VFS Security)**
   - Path traversal protection (../../../etc/passwd blocked)
   - Immutable core files (services/sentinel/*, .git/*, .sentinel/*)
   - Read-only file enforcement

3. **Interceptor (Audit & PII)**
   - Automatic secret redaction (sk-*, AKIA*, etc.)
   - SQLite audit vault
   - Real-time event streaming via gRPC

4. **Polyglot Engine**
   - Auto-detection: go.mod, package.json, Cargo.toml, pom.xml
   - Abstracted commands: run_tests(), build(), lint()
   - Cross-platform execution (Linux/Mac/Windows)

---

## Implementation Phases

### PHASE 1: Kernel Foundation (120k tokens)
**Duration:** 2-3 weeks
**Output:** Basic `bin/sentinel` binary with SQLite + VFS Jail

#### Sprint 1.1: Boilerplate & Database
- [ ] **T-101: Project Structure**
  - Initialize Go module: `github.com/TheArchitectit/agent-guardrails-template/services/sentinel`
  - Create directory structure: cmd/, internal/kernel, internal/state, internal/audit
  - Add dependencies: modernc.org/sqlite, mark3labs/mcp-go
  - Create Makefile (build, test, clean)
  - **Acceptance**: `go build` produces binary, runs without panic

- [ ] **T-102: SQLite Store & Migrator**
  - Implement `internal/state/store.go`
  - Create tables: schema_version, system_config, sprints, tasks, audit_log
  - Enable WAL mode: `PRAGMA journal_mode=WAL`
  - **Acceptance**: Migrations are idempotent, temp DB test passes

- [ ] **T-103: Audit Logger**
  - Implement `internal/audit/logger.go`
  - Add PII scrubbing regex (sk-*, AKIA*, high-entropy tokens)
  - Dual output: Stderr + SQLite
  - **Acceptance**: "sk-1234567890abcdef" → "[REDACTED]" in DB

#### Sprint 1.2: VFS Jail (Security Kernel)
- [ ] **T-104: Path Sanitization**
  - Implement `kernel.ValidatePath(requested, operation)`
  - filepath.Abs + filepath.Clean
  - Prefix check (must start with Root)
  - DenyList regex: .git, .env, id_rsa, services/sentinel
  - **Acceptance**: "../../../etc/passwd" → Error, ".env" → Error

- [ ] **T-105: Safe IO Wrappers**
  - Implement `kernel.ReadFile(path) []byte`
  - Implement `kernel.WriteFile(path, data) error`
  - Auto-create directories with MkdirAll
  - Check sentinel.toml for read-only files
  - **Acceptance**: Writing to "foo/bar/baz.txt" creates intermediate dirs

#### Sprint 1.3: Basic MCP Server
- [ ] **T-106: MCP Server Skeleton**
  - Implement `kernel.StartMCP()`
  - Register get_sentinel_status tool
  - Stdio listener for Claude Desktop/Cursor compatibility
  - **Acceptance**: Agent can call `get_sentinel_status` and receive JSON

---

### PHASE 2: Logic & Tooling (150k tokens)
**Duration:** 3-4 weeks
**Prerequisites:** Phase 1 complete
**Output:** Full-featured MCP server with polyglot support

#### Sprint 2.1: Polyglot Toolchain Engine
- [ ] **T-201: Language Detection**
  - Implement `toolchain.DetectLanguage(root)`
  - Check for: go.mod, package.json (+ lockfiles), Cargo.toml, pom.xml, flake.nix
  - Return Profile struct with TestCmd, BuildCmd, LintCmd
  - **Acceptance**: Running on this repo returns Go profile

- [ ] **T-202: Command Abstraction Layer**
  - Implement `toolchain.ExecuteCommand(ctx, cmd)`
  - Windows support via cmd.exe wrapping
  - SanitizeEnv() to strip AWS_SECRET_KEY, GH_TOKEN
  - **Acceptance**: `go version` executes successfully, env vars scrubbed

- [ ] **T-203: run_tests Tool**
  - Register run_tests MCP tool
  - Auto-detect language → Get TestCmd → Execute
  - Parse output for "FAIL" patterns
  - **Acceptance**: Broken project returns compiler error in tool output

#### Sprint 2.2: Workflow & State Enforcement
- [ ] **T-204: Sprint & Task CRUD**
  - Implement `kernel.StartSprint(name, goals)`
  - Implement `kernel.AddTask(title, complexity)`
  - DB operations for sprints/tasks tables
  - **Acceptance**: Can create sprint, add 3 tasks, query them back

- [ ] **T-205: State Machine Logic**
  - Implement state transitions: IDLE → PLANNING → ACTIVE → REVIEW → RELEASE
  - Block operations based on state (no coding in PLANNING)
  - Enforce "One Task at a Time" rule
  - **Acceptance**: Cannot start second task while first is active

- [ ] **T-206: Git Integration**
  - Register git_commit tool with Conventional Commits enforcement
  - Register git_push tool with main-branch protection
  - Pre-commit verification (check recent test results)
  - **Acceptance**: "wip: update" rejected, must use "feat: ..."

#### Sprint 2.3: Cost Governance
- [ ] **T-207: Token Estimator**
  - Implement `cost.EstimateFileRead(path)`
  - Heuristic: 3.5 chars/token for code, 4.0 for text
  - Cost calculation based on OpenAI/Anthropic rates
  - **Acceptance**: Reading 50KB file shows estimated cost

- [ ] **T-208: Budget Ledger**
  - Implement `ledger.CheckBudget(cost, sprintID)`
  - Track token usage per sprint
  - Block operations when budget exceeded
  - **Acceptance**: $0.50 budget blocks after $0.50 spent

---

### PHASE 3: Integration & Release (100k tokens)
**Duration:** 2-3 weeks
**Prerequisites:** Phase 2 complete
**Output:** Enterprise release v3.0.0

#### Sprint 3.1: REST API Gateway
- [ ] **T-301: HTTP Server**
  - Implement `api/gateway.go` using chi router
  - Endpoints: /health, /v1/sprint, /v1/tasks, /v1/audit
  - Auth middleware (Bearer token for non-localhost)
  - **Acceptance**: `curl localhost:8080/v1/status` returns JSON

- [ ] **T-302: Policy Check Endpoint**
  - POST /v1/policy/check for CI/CD integration
  - Returns ALLOWED/BLOCKED without writing files
  - VFS check + PII scan
  - **Acceptance**: CI pipeline can validate files before commit

#### Sprint 3.2: gRPC Service & Events
- [ ] **T-303: Event Bus**
  - Implement `api/event_bus.go`
  - Pub/sub for: SECURITY_VIOLATION, TASK_UPDATE, FILE_CHANGE
  - Channel-based subscriptions
  - **Acceptance**: Security event publishes to all subscribers

- [ ] **T-304: gRPC Server**
  - Define proto/sentinel.proto (LogStream, ExecuteTool)
  - Implement streaming logs for IDEs
  - mTLS authentication
  - **Acceptance**: JetBrains plugin receives real-time alerts

#### Sprint 3.3: Containerization & Deployment
- [ ] **T-305: Docker Image**
  - Multi-stage Dockerfile (Alpine-based)
  - Entry point: `sentinel serve --mode=remote`
  - **Acceptance**: `docker run sentinel` starts successfully

- [ ] **T-306: Remote Mode**
  - CLI flag --host (default 127.0.0.1, remote uses 0.0.0.0)
  - Require SENTINEL_TOKEN in remote mode
  - **Acceptance**: Container fails without token env var

#### Sprint 3.4: IDE Integration
- [ ] **T-307: LSP Diagnostics**
  - Lint errors as IDE diagnostics
  - "Guardrail Violation" severity level
  - **Acceptance**: VS Code shows red underline on blocked file

- [ ] **T-308: Installation Scripts**
  - `curl https://releases.project-sentinel.io/install.sh | sh`
  - `sentinel init --root .`
  - **Acceptance**: Single-command install for developers

---

## Testing Strategy

### Unit Tests
- **Kernel**: State transitions, path validation
- **Jailor**: Path traversal attacks, PII scrubbing
- **Toolchain**: Language detection, command execution
- **Cost**: Token estimation accuracy

### Integration Tests
- **MCP Server**: Tool registration and execution
- **Database**: Migration idempotency
- **Git**: Commit/push workflows

### Security Tests
- **Path Traversal**: ../../../etc/passwd, ..\..\..\windows\system32
- **Secret Leakage**: sk-*, AKIA*, passwords in logs
- **State Bypass**: Try to deploy in PLANNING state

### E2E Tests
- **Full Sprint**: Create sprint → Add tasks → Complete → Archive
- **Budget Enforce**: Exceed budget → Verify operations blocked
- **CI/CD**: Pipeline calls /v1/policy/check

---

## Success Criteria

### Phase 1 Gates
- [ ] Binary compiles on Linux, Mac, Windows
- [ ] SQLite migrations are idempotent
- [ ] Path traversal attacks are blocked
- [ ] PII is scrubbed from audit logs

### Phase 2 Gates
- [ ] Detects Go, Python, TypeScript, Rust, Java
- [ ] State machine enforces SDLC phases
- [ ] Token estimation within 10% margin
- [ ] Git commits enforce Conventional Commits

### Phase 3 Gates
- [ ] REST API serves all endpoints
- [ ] gRPC streams events in real-time
- [ ] Docker image runs in container
- [ ] IDE receives diagnostics

---

## Risk Mitigation

### Technical Risks
1. **SQLite Concurrency**
   - Mitigation: WAL mode enabled by default
   - Timeout: 5000ms for busy handler

2. **Path Detection Complexity**
   - Mitigation: Start with 5 core languages (Go, Python, TS, Rust, Java)
   - Fallback: Manual sentinel.toml configuration

3. **Token Estimation Accuracy**
   - Mitigation: 10% error margin acceptable for guardrails
   - Refinement: Track actual vs. estimated for model improvement

### Operational Risks
1. **Agent Resistance**
   - Mitigation: Transparent error messages explain why blocked
   - Documentation: Clear guides for each guardrail

2. **Performance Overhead**
   - Mitigation: Asynchronous logging
   - Measurement: Benchmark <50ms for path validation

3. **Language Drift**
   - Mitigation: Modular language profiles (easy to add new ones)
   - Community: Contributions for new languages

---

## Resource Requirements

### Development
- **Go Expert**: Full-time (40 hrs/week)
- **DevOps Engineer**: Part-time (10 hrs/week) for Phase 3
- **Security Reviewer**: On-call for audit

### Infrastructure
- **CI/CD**: GitHub Actions (already configured)
- **Releases**: GitHub Releases for binaries
- **Registry**: Docker Hub for container image

### Budget
- **Development API Costs**: ~$30 (370k tokens)
- **Testing Costs**: ~$20 (E2E test runs)
- **Total Estimated**: ~$50

---

## Timeline

```
Phase 1: Week 1-3   (Kernel Foundation)
  ├─ Sprint 1.1: Week 1
  ├─ Sprint 1.2: Week 2
  └─ Sprint 1.3: Week 3

Phase 2: Week 4-7   (Logic & Tooling)
  ├─ Sprint 2.1: Week 4-5
  ├─ Sprint 2.2: Week 6
  └─ Sprint 2.3: Week 7

Phase 3: Week 8-10  (Integration & Release)
  ├─ Sprint 3.1: Week 8
  ├─ Sprint 3.2: Week 9
  └─ Sprint 3.3-3.4: Week 10

Release: Week 11    (v3.0.0-Enterprise)
```

---

## Post-Launch Roadmap

### v3.1 (Enhanced Languages)
- PHP, Ruby, Scala, Swift, Kotlin profiles
- NixOS/flake support enhancement
- WASM sandboxing option

### v3.2 (Cloud Native)
- Kubernetes operator
- Multi-agent coordination (swarm mode)
- Distributed audit log (PostgreSQL)

### v4.0 (AI Integration)
- LLM-hosted mode (Sentinel as an agent)
- Self-healing capabilities
- Predictive budgeting

---

## Compliance & Licensing

- **License**: BSD-3-Clause
- **Compliance**: SOC 2 Type II (target Q3 2026)
- **Data**: No telemetry sent externally (local-only audit)

---

## Conclusion

Project Sentinel transforms passive guardrails into active enforcement. This 3-phase plan delivers a production-ready system that protects agents from themselves while maintaining developer autonomy.

**Next Step:** Execute Sprint 1.1 (T-101: Project Structure)
