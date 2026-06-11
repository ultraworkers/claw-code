# Agent Guardrails & Safety Protocols

**Version:** 1.3
**Last Updated:** 2026-01-21
**Applies To:** ALL AI agents, LLMs, and automated systems operating on this codebase

---

## Applicability

This document is **MANDATORY** for:

| System Type | Examples | Binding? |
|-------------|----------|----------|
| **Large Language Models** | Claude, GPT-4/o1/o3, Gemini, LLaMA, Mistral, Qwen, DeepSeek, Cohere, Phi, Falcon, and 30+ other model families | YES |
| **AI Coding Assistants** | Claude Code, GitHub Copilot, Cursor, Cody, Aider, Continue, Windsurf, and others | YES |
| **Autonomous Agents** | AutoGPT, AgentGPT, CrewAI, LangChain, LangGraph, Semantic Kernel agents | YES |
| **Reasoning Models** | o1, o3, DeepSeek-R1, and other chain-of-thought models | YES |
| **CI/CD Bots** | Dependabot, Renovate, automated PR bots | YES |
| **Custom Agents** | Any agent built on this codebase (specialists, operators, swarm) | YES |
| **Human Developers** | Recommended best practices | RECOMMENDED |

**If you are an AI system reading this:** You MUST follow these protocols. They are not suggestions.

---

## Purpose

This document defines mandatory safety protocols, guardrails, and behavioral constraints for ALL automated systems performing tasks on this repository. These rules exist to:

1. **Prevent data loss** — enabling safe rapid iteration without backup anxiety
2. **Maintain code quality** — so AI-generated code ships without manual review overhead
3. **Preserve history** — keeping git history clean and reversible
4. **Enable collaboration** — allowing humans and agents to work together safely
5. **Limit blast radius** — containing errors to minimal scope

### How These Laws Enable Rapid Development

The Four Laws aren't restrictions — they're accelerators. Here's why:

- **Read Before Editing** eliminates rework. One read costs fewer tokens than fixing a blind edit.
- **Stay in Scope** prevents cascade failures. Agents move faster when they're not untangling unintended side effects.
- **Verify Before Committing** catches errors at the cheapest point. A failed test in development costs minutes; in production, it costs hours.
- **Halt When Uncertain** prevents wasted effort. Asking one question is cheaper than building the wrong thing.

When agents follow these laws, they don't need to pause for safety checks — safety is built into every step. The result: full-velocity development with production-grade reliability.

---

## CORE PRINCIPLES

### The Four Laws of Agent Safety

See [skills/shared-prompts/four-laws.md](../skills/shared-prompts/four-laws.md) for the complete Four Laws documentation.

**Quick Reference:**
1. **Read Before Editing** - Never modify code without reading first
2. **Stay in Scope** - Only touch authorized files
3. **Verify Before Committing** - Test all changes
4. **Halt When Uncertain** - Ask instead of guessing

---

## SAFETY PROTOCOLS (MANDATORY)

### Pre-Execution Checklist

**EVERY agent MUST verify these before ANY file modification:**

| # | Check | Requirement | Verify |
|---|-------|-------------|--------|
| 1 | **READ FIRST** | NEVER edit a file without reading it first | [ ] |
| 2 | **SCOPE LOCK** | Only modify files explicitly in scope | [ ] |
| 3 | **NO FEATURE CREEP** | Do NOT add features, refactor, or "improve" unrelated code | [ ] |
| 4 | **PRODUCTION FIRST** | Production code created BEFORE test code | [ ] |
| 5 | **TEST/PROD SEPARATION** | Test infrastructure is separate from production | [ ] |
| 6 | **BACKUP AWARENESS** | Know the rollback command before editing | [ ] |
| 7 | **TEST BEFORE COMMIT** | All tests must pass before committing | [ ] |
| 8 | **CHECK FAILURE REGISTRY** | Review known bugs for affected files ([.guardrails/pre-work-check.md](../.guardrails/pre-work-check.md)) | [ ] |
| 9 | **VERIFY FIXES INTACT** | Confirm previous fixes not being undone | [ ] |

### Git Safety Rules

| Rule | Description | Consequence |
|------|-------------|-------------|
| **NO FORCE PUSH** | Never use `git push --force` | Data loss, history corruption |
| **NO AMEND** | Do not amend commits you didn't create this session | Breaks collaborator history |
| **NO CONFIG CHANGES** | Do not modify git config | Security/identity issues |
| **NO PUSH WITHOUT PERMISSION** | Only push if user explicitly requests | Unwanted remote changes |
| **SINGLE COMMIT** | One focused commit per task | Maintains clean history |
| **NO SKIP HOOKS** | Never use `--no-verify` | Bypasses safety checks |
| **NO REBASE** | Never rebase shared branches | Destroys collaborator work |
| **NO DESTRUCTIVE OPS** | No `reset --hard` on shared branches | Irreversible data loss |

### Code Safety Rules

| Rule | Rationale |
|------|-----------|
| **EXACT REPLACEMENT** | Use provided code exactly - no "improvements" |
| **NO NEW IMPORTS** | Unless explicitly required by the task |
| **NO TYPE CHANGES** | Preserve existing type hints |
| **NO DELETIONS** | Do not delete functionality outside scope |
| **PRESERVE FORMATTING** | Match existing indentation and style |
| **NO SECRETS** | Never commit credentials, keys, tokens |
| **NO BINARY FILES** | Unless explicitly required |
| **NO GENERATED CODE** | Do not commit build artifacts |

### Test/Production Separation Rules (MANDATORY)

| Rule | Violation Level | Action |
|------|-----------------|--------|
| **PRODUCTION CODE FIRST** | CRITICAL | Halt, ask user |
| **SEPARATE DATABASES** | CRITICAL | Halt, ask user |
| **SEPARATE SERVICES** | CRITICAL | Halt, ask user |
| **NO TEST USERS IN PROD** | CRITICAL | Halt, rollback |
| **NO PROD CREDENTIALS IN TEST** | CRITICAL | Halt, rollback |
| **ASK IF UNCERTAIN** | HIGH | Ask user before proceeding |

**Full details:** See [TEST_PRODUCTION_SEPARATION.md](standards/TEST_PRODUCTION_SEPARATION.md)

---

## GUARDRAILS

### HALT CONDITIONS

**Stop immediately and report to user if ANY of these occur:**

```
CRITICAL HALT - DO NOT PROCEED:

[ ] Target file does not exist
[ ] Line numbers don't match expected
[ ] File has unexpected modifications
[ ] Syntax check fails after edit
[ ] Any test fails after edit
[ ] Merge conflicts encountered
[ ] Uncertain about ANY step
[ ] Edit tool reports "string not found"
[ ] Permission denied errors
[ ] Import errors when testing
[ ] Network/connection errors
[ ] Out of memory errors
[ ] Timeout errors
[ ] User requests stop
[ ] Test/production boundary unclear
[ ] Attempting to use production DB for tests
[ ] Attempting to use test DB for production
```

### FORBIDDEN ACTIONS

**No agent may perform these actions under any circumstances:**

```
ABSOLUTE PROHIBITIONS:

FILE OPERATIONS:
- Modify files outside declared scope
- Delete files without explicit permission
- Create files without explicit need
- Modify hidden/system files (.*) without permission
- Change file permissions

CODE CHANGES:
- Add logging/debugging to production code
- Add comments that weren't requested
- "Clean up" or "improve" surrounding code
- Update version numbers without explicit request
- Change security configurations
- Modify authentication/authorization code without review

TEST/PRODUCTION SEPARATION:
- Deploy test code to production environment
- Use production database for tests
- Create test users in production database
- Write test code that imports production secrets
- Use production services for test execution
- Share user accounts across environments

GIT OPERATIONS:
- Force push to any branch
- Delete branches without permission
- Modify git hooks
- Change git config
- Push without explicit permission

SYSTEM OPERATIONS:
- Run servers or long-running services
- Execute commands requiring user input
- Make network requests to unknown endpoints
- Install new dependencies without permission
- Modify CI/CD pipelines without permission
- Execute shell commands with elevated privileges
- Access or modify environment variables

DATA OPERATIONS:
- Access databases without explicit permission
- Modify production data
- Export or transmit user data
- Store credentials or secrets
- Mix test and production data
```

### SCOPE BOUNDARIES

**For any task, clearly define IN/OUT scope:**

```
IN SCOPE (may modify):
  - Specific file(s) listed in task
  - Specific line ranges identified
  - Exact changes described
  - Production code (before test code)

OUT OF SCOPE (DO NOT TOUCH):
  - All other files
  - All other methods/functions in target file
  - Tests in production files (read-only unless task is test-related)
  - Documentation (unless task is doc-related)
  - Git hooks and configs
  - CI/CD configurations
  - Dependencies/package files
  - Environment configurations
  - Security-related files
  - Production database connections in test code
  - Test database connections in production code
```

---

## QUICK REFERENCE

```
+------------------------------------------------------------------+
|              UNIVERSAL AGENT GUARDRAILS                           |
+------------------------------------------------------------------+
| ALWAYS:                                                           |
|   - Read before edit                                              |
|   - Verify before proceeding                                      |
|   - Test before committing                                        |
|   - Create production code BEFORE test code                      |
|   - Separate test/production infrastructure                       |
|   - Report results to user                                        |
|   - Include AI attribution                                        |
+------------------------------------------------------------------+
| NEVER:                                                            |
|   - Edit without reading                                          |
|   - Push without permission                                       |
|   - Modify outside scope                                          |
|   - Force push or rebase                                          |
|   - Continue when uncertain                                       |
|   - Use production DB for tests                                   |
|   - Create test users in production                               |
+------------------------------------------------------------------+
| HALT IF:                                                          |
|   - Conditions don't match                                        |
|   - Any check fails                                               |
|   - Uncertain about anything                                      |
|   - User requests stop                                            |
|   - Test/production boundary unclear                              |
+------------------------------------------------------------------+
| ROLLBACK: git checkout HEAD -- <file>                             |
+------------------------------------------------------------------+
| APPLIES TO: ALL LLMs, AI assistants, coding agents, and automated systems |
+------------------------------------------------------------------+
```

---

## RELATED DOCUMENTS

### Core Guardrails
- **This document** - Core safety protocols (MANDATORY)
- [TEST_PRODUCTION_SEPARATION.md](standards/TEST_PRODUCTION_SEPARATION.md) - Test/production isolation (MANDATORY)
- [REGRESSION_PREVENTION.md](workflows/REGRESSION_PREVENTION.md) - Bug tracking and regression prevention

### Regression Prevention
- [.guardrails/pre-work-check.md](../.guardrails/pre-work-check.md) - MANDATORY pre-work checklist
- [.guardrails/failure-registry.jsonl](../.guardrails/failure-registry.jsonl) - Bug database (JSONL format)
- [scripts/log_failure.py](../scripts/log_failure.py) - CLI to log new failures
- [scripts/regression_check.py](../scripts/regression_check.py) - Pre-commit regression scanner

### Workflow Documentation
- [AGENT_EXECUTION.md](workflows/AGENT_EXECUTION.md) - Execution protocol, rollback, Three Strikes Rule
- [AGENT_REVIEW_PROTOCOL.md](workflows/AGENT_REVIEW_PROTOCOL.md) - Post-work agent/LLM review (RECOMMENDED)
- [TESTING_VALIDATION.md](workflows/TESTING_VALIDATION.md) - Validation protocols
- [COMMIT_WORKFLOW.md](workflows/COMMIT_WORKFLOW.md) - Commit guidelines
- [GIT_PUSH_PROCEDURES.md](workflows/GIT_PUSH_PROCEDURES.md) - Push safety
- [ROLLBACK_PROCEDURES.md](workflows/ROLLBACK_PROCEDURES.md) - Recovery operations
- [MCP_CHECKPOINTING.md](workflows/MCP_CHECKPOINTING.md) - Checkpoint integration

### Agent Operations
- [AGENT_ESCALATION.md](workflows/AGENT_ESCALATION.md) - Audit requirements and escalation
- [CODE_REVIEW.md](workflows/CODE_REVIEW.md) - Code review process

### Standards
- [PROJECT_CONTEXT_TEMPLATE.md](standards/PROJECT_CONTEXT_TEMPLATE.md) - Project Bible template
- [ADVERSARIAL_TESTING.md](standards/ADVERSARIAL_TESTING.md) - Breaker agent, fuzz testing
- [DEPENDENCY_GOVERNANCE.md](standards/DEPENDENCY_GOVERNANCE.md) - Package allow-list
- [INFRASTRUCTURE_STANDARDS.md](standards/INFRASTRUCTURE_STANDARDS.md) - IaC, Terraform, drift detection
- [OPERATIONAL_PATTERNS.md](standards/OPERATIONAL_PATTERNS.md) - Health checks, circuit breakers
- [LOGGING_PATTERNS.md](standards/LOGGING_PATTERNS.md) - Structured logging
- [MODULAR_DOCUMENTATION.md](standards/MODULAR_DOCUMENTATION.md) - 500-line rule

### Sprint Framework
- [Sprint Task Template](sprints/) - Task execution format
- [SPRINT_GUIDE.md](sprints/SPRINT_GUIDE.md) - How to write sprints

### Security
- [SECRETS_MANAGEMENT.md](../.github/SECRETS_MANAGEMENT.md) - GitHub Secrets

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Review Cycle:** Monthly
**Last Review:** 2026-01-21
**Next Review:** 2026-02-21
