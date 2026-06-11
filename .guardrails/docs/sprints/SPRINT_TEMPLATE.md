# Sprint Task: [TITLE]

**Sprint Date:** YYYY-MM-DD (Day)
**Archive After:** YYYY-MM-DD (Day) [+7 days]
**Sprint Focus:** [One-line description of what this sprint accomplishes]
**Priority:** P0 (Critical) | P1 (Blocking) | P2 (Normal) | P3 (Low)
**Estimated Effort:** [X minutes/hours]
**Status:** PENDING | IN_PROGRESS | COMPLETE | BLOCKED | FAILED

---

## SAFETY PROTOCOLS (MANDATORY)

### Pre-Execution Safety Checks

| Check | Requirement | Verify |
|-------|-------------|--------|
| **READ FIRST** | NEVER edit a file without reading it first | [ ] |
| **SCOPE LOCK** | Only modify files explicitly in scope | [ ] |
| **NO FEATURE CREEP** | Do NOT add features or "improve" unrelated code | [ ] |
| **PRODUCTION FIRST** | Production code created BEFORE test code | [ ] |
| **TEST/PROD SEPARATION** | Test infrastructure is separate from production | [ ] |
| **ASK IF UNCERTAIN** | If test/production boundary unclear, ask user | [ ] |
| **BACKUP AWARENESS** | Know the rollback command before editing | [ ] |
| **TEST BEFORE COMMIT** | All tests must pass before committing | [ ] |

### Guardrails Reference

Full guardrails: [docs/AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md)

### MCP Checkpoint (Optional)

If using MCP checkpointing, create checkpoint before starting:
```
[MCP: create_checkpoint("sprint-YYYY-MM-DD-before-start")]
```

See [MCP_CHECKPOINTING.md](../workflows/MCP_CHECKPOINTING.md) for details.

---

## PROBLEM STATEMENT

[Describe the problem in 2-3 sentences. Include error messages if applicable.]

```
[Include error output or symptoms here]
```

**Why:** [Root cause explanation]

**Where:** [File and line numbers]

---

## SCOPE BOUNDARY

```
IN SCOPE (may modify):
  - File: [path/to/file.ext]
  - Lines: [X-Y]
  - Change: [Brief description]

OUT OF SCOPE (DO NOT TOUCH):
  - All other files
  - All other methods/functions
  - Tests (read-only for verification)
  - Documentation (this file excluded)
```

---

## EXECUTION DIRECTIONS

### Overview

```
TASK SEQUENCE:

  STEP 1: [Action] ──────────────────────► [Purpose]
       │
       ▼
  STEP 2: [Action] ──────────────────────► [Purpose]
       │
       ▼
  STEP 3: [Action] ──────────────────────► [Purpose]
       │
       ▼
  DONE: Report to user ──────────────────► Summary
```

---

## STEP-BY-STEP EXECUTION

### STEP 1: [Title]

**Action:** [Describe what to do]

```
TOOL: [Read | Edit | Bash | etc.]
[Tool-specific parameters]
```

**Checkpoint:** [What to verify before proceeding]

**Decision Point:**
- [ ] Success → Proceed to STEP 2
- [ ] Failure → HALT and report to user

---

### STEP 2: [Title]

**Action:** [Describe what to do]

```
TOOL: [Read | Edit | Bash | etc.]
[Tool-specific parameters]
```

**Checkpoint:** [What to verify]

**Decision Point:**
- [ ] Success → Proceed to STEP 3
- [ ] Failure → ROLLBACK and report

**Rollback Command (if needed):**
```bash
git checkout HEAD -- [file]
```

---

### STEP 3: [Title]

**Action:** [Describe what to do]

```
TOOL: [Read | Edit | Bash | etc.]
[Tool-specific parameters]
```

**Expected Output:**
```
[What successful output looks like]
```

**Decision Point:**
- [ ] Success → Proceed to DONE
- [ ] Failure → ROLLBACK and report

---

### DONE: Commit and Report

**COMMIT AFTER VALIDATION** (see [COMMIT_WORKFLOW.md](../workflows/COMMIT_WORKFLOW.md)):
```bash
git add <modified-files>
git commit -m "<type>(<scope>): <description>

Authored by TheArchitectit"
```

**MCP Checkpoint (Optional):**
```
[MCP: create_checkpoint("sprint-YYYY-MM-DD-after-complete")]
```

**Action:** Provide completion summary

```
REPORT FORMAT:

## Sprint Complete: [Title]

**Status:** SUCCESS / FAILED
**File Modified:** [path]
**Lines Changed:** [X-Y]
**Commit Hash:** [hash]

### Changes Made:
- [Change 1]
- [Change 2]

### Verification Results:
- Syntax check: PASSED
- Unit tests: PASSED / SKIPPED
- Manual verification: X/X PASSED

### Next Steps:
- Review commit with: git show HEAD
- Push when ready with: git push origin [branch]
```

---

## COMPLETION GATE (MANDATORY)

**This section MUST be completed before marking the sprint done.**
**Compatible with:** Claude Code, OpenCode, and all LLM agents

### Validation Loop Rules

```
MAX_CYCLES: 3
MAX_TIME: 10 minutes
EXIT_CONDITIONS:
  - All BLOCKING items pass, OR
  - MAX_CYCLES reached (report blockers), OR
  - MAX_TIME exceeded (report status)

LOOP BEHAVIOR:
  1. Run all validation checks
  2. If BLOCKING items fail → fix and re-run (cycle++)
  3. If only NON-BLOCKING items fail → note and proceed
  4. Report final status to user
```

### Core Validation Checklist

**Run these checks. Loop until all BLOCKING items pass or limits reached.**

| Check | Command | Pass Condition | Blocking? | Status |
|-------|---------|----------------|-----------|--------|
| **Files Saved** | `git status` | No unexpected untracked files | YES | [ ] |
| **Changes Staged** | `git diff --cached --stat` | Target files staged | YES | [ ] |
| **Syntax Valid** | See language table below | Exit code 0 | YES | [ ] |
| **Tests Pass** | See language table below | Exit code 0 | YES | [ ] |
| **Production Code** | Manual check | Production code exists | YES | [ ] |
| **Test Infrastructure** | Manual check | Test infrastructure is separate | YES | [ ] |
| **Test Environment** | Manual check | Test environment isolated | YES | [ ] |
| **Committed** | `git log -1 --oneline` | Shows sprint commit | YES | [ ] |
| **Docs Updated** | Manual check | INDEX_MAP.md current (if applicable) | NO | [ ] |
| **No Secrets** | `git diff --cached` | No API keys, tokens, passwords | YES | [ ] |
| **No Prod Creds in Tests** | `grep -i "prod" test/*` | No production credentials in test files | YES | [ ] |
| **No Test Creds in Prod** | `grep -i "test" src/*` | No test credentials in production code | YES | [ ] |

**Cycle:** ___ / 3
**Time Started:** ___:___
**Current Status:** VALIDATING | PASSED | BLOCKED | TIMEOUT

### Language-Specific Validation Commands

**Select the appropriate commands for your stack:**

#### JavaScript / TypeScript (React, Next.js, Node.js)

```bash
# Syntax/Type Check
npx tsc --noEmit                    # TypeScript projects
npx eslint . --ext .js,.jsx,.ts,.tsx  # Linting

# Tests
npm test                            # Jest/Vitest (default)
npm run test:unit                   # Unit tests only
npx vitest run                      # Vitest
npx jest --passWithNoTests          # Jest with no-fail on empty

# Build Verification
npm run build                       # Production build
npx next build                      # Next.js specific
```

#### Rust

```bash
# Syntax/Type Check
cargo check                         # Fast syntax check
cargo clippy -- -D warnings         # Linting (strict)

# Tests
cargo test                          # All tests
cargo test --lib                    # Library tests only
cargo test --doc                    # Doc tests

# Build Verification
cargo build --release               # Release build
```

#### Go

```bash
# Syntax/Type Check
go build ./...                      # Compile check
go vet ./...                        # Static analysis
golangci-lint run                   # Comprehensive linting

# Tests
go test ./...                       # All tests
go test -v ./...                    # Verbose
go test -race ./...                 # Race detection

# Build Verification
go build -o /dev/null ./...         # Build without output
```

#### Python

```bash
# Syntax/Type Check
python -m py_compile *.py           # Syntax check
python -m mypy .                    # Type checking
ruff check .                        # Fast linting (2023+)
pylint **/*.py                      # Traditional linting

# Tests
pytest                              # Default
pytest -v                           # Verbose
python -m pytest --tb=short         # Short traceback

# Build Verification
pip install -e . --dry-run          # Dry run install
```

#### Ruby / Rails

```bash
# Syntax Check
ruby -c app/**/*.rb                 # Syntax check
bundle exec rubocop                 # Linting

# Tests
bundle exec rspec                   # RSpec
bundle exec rails test              # Rails default

# Build Verification
bundle exec rails assets:precompile # Asset build
```

#### Java / Kotlin (Gradle/Maven)

```bash
# Syntax/Compile
./gradlew compileJava               # Gradle
./gradlew compileKotlin             # Kotlin
mvn compile                         # Maven

# Tests
./gradlew test                      # Gradle
mvn test                            # Maven

# Build
./gradlew build                     # Full build
mvn package                         # Maven package
```

#### C# / .NET

```bash
# Syntax/Build
dotnet build                        # Build
dotnet build --no-restore           # Skip restore

# Tests
dotnet test                         # All tests
dotnet test --no-build              # Skip rebuild

# Publish
dotnet publish -c Release           # Release build
```

#### Swift

```bash
# Syntax/Build
swift build                         # Build
swiftlint                           # Linting

# Tests
swift test                          # Run tests

# Release
swift build -c release              # Release build
```

#### Elixir

```bash
# Syntax/Compile
mix compile --warnings-as-errors    # Strict compile
mix credo --strict                  # Linting

# Tests
mix test                            # All tests

# Build
mix release                         # Release build
```

### CLI-Specific Notes

#### For Claude Code Users

```
- Use TodoWrite to track validation progress
- Use Bash tool for running validation commands
- If validation fails, Claude Code will show errors inline
- Hooks can automate pre-commit validation (see hooks config)
```

#### For OpenCode Users

```
- Use /run or direct shell for validation commands
- Check exit codes explicitly: echo $?
- Use /diff to review changes before commit
- OpenCode supports similar tool patterns to Claude Code
```

### Validation Loop Template

**Copy and execute this validation sequence:**

```
VALIDATION CYCLE 1:
────────────────────

1. [ ] git status                    → Clean? ___
2. [ ] [syntax check command]        → Pass?  ___
3. [ ] [test command]                → Pass?  ___
4. [ ] git diff --cached             → No secrets? ___
5. [ ] git log -1 (after commit)     → Committed? ___

RESULT: ___ BLOCKING issues found

If BLOCKING > 0 and CYCLE < 3:
  → Fix issues
  → Increment CYCLE
  → Re-run checks

If BLOCKING == 0:
  → PROCEED to completion report
```

### Completion Report Template

**After validation passes, report:**

```
## Completion Gate: PASSED ✓

**Validation Summary:**
- Cycles Used: X / 3
- Time Elapsed: X minutes
- Blocking Issues: 0
- Non-Blocking Notes: [list any]

**Checks Passed:**
- [x] Files saved and staged
- [x] Syntax valid
- [x] Tests pass
- [x] Committed
- [x] No secrets exposed

**Ready for:** Push / Review / Merge
```

---

## ACCEPTANCE CRITERIA

| # | Criterion | Test | Pass Condition |
|---|-----------|------|----------------|
| 1 | [Criterion 1] | [How to test] | [What passes] |
| 2 | [Criterion 2] | [How to test] | [What passes] |
| 3 | [Criterion 3] | [How to test] | [What passes] |

---

## ROLLBACK PROCEDURE

```bash
# Immediate rollback - discard all changes
git checkout HEAD -- [file]

# Verify rollback
git status

# Report to user
echo "Rollback complete. File restored to original state."
```

---

## REFERENCE

[Optional: Include reference code, documentation links, or examples]

---

## QUICK REFERENCE CARD

```
+------------------------------------------------------------------+
|                    SPRINT QUICK REFERENCE                        |
+------------------------------------------------------------------+
| TARGET FILE:  [path/to/file]                                     |
| TARGET LINES: [X-Y]                                              |
| CHANGE TYPE:  [Brief description]                                |
+------------------------------------------------------------------+
| SAFETY:                                                          |
|   - Read before edit                                             |
|   - Single file only                                             |
|   - Production code CREATED FIRST                                |
|   - Test/production infrastructure SEPARATE                       |
|   - Test before commit                                           |
|   - No push without permission                                   |
+------------------------------------------------------------------+
| TEST/PRODUCTION SEPARATION:                                      |
|   - Production code BEFORE test code                             |
|   - Separate database instances                                  |
|   - Separate service endpoints                                   |
|   - Separate user accounts                                       |
|   - AKS USER if uncertain                                        |
+------------------------------------------------------------------+
| HALT IF:                                                         |
|   - Conditions don't match                                       |
|   - Any check fails                                              |
|   - Uncertain about anything                                     |
|   - Test/production boundary unclear                             |
+------------------------------------------------------------------+
| ROLLBACK: git checkout HEAD -- [file]                            |
+------------------------------------------------------------------+
```

---

**Created:** YYYY-MM-DD
**Authored by:** TheArchitectit
**Archive Date:** YYYY-MM-DD
**Version:** 1.1
