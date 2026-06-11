# Testing & Validation Protocols

> Double-check all work before committing.

**Related:** [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) | [COMMIT_WORKFLOW.md](./COMMIT_WORKFLOW.md)

---

## Overview

This document defines validation functions and git diff verification patterns to ensure all changes are correct before committing. Use these protocols to double-check work and catch errors early.

---

## Validation Function Patterns

### Pre-Edit Validation

Before making any edit, validate these conditions:

| Check | Command/Method | Expected Result |
|-------|----------------|-----------------|
| File exists | `test -f <file>` | Exit code 0 |
| File readable | Read tool returns content | No error |
| Target content exists | Search for old_string | Found at expected location |
| Line numbers match | Count lines to target | Matches expected range |
| No pending changes | `git status <file>` | Clean or expected state |

**Pre-Edit Checklist:**
```
[ ] Target file exists and is readable
[ ] Target content found at expected location
[ ] Line numbers match documentation
[ ] No unexpected modifications in file
[ ] Rollback command identified
```

### Post-Edit Validation

After every edit, immediately verify:

| Check | Command/Method | Pass Condition |
|-------|----------------|----------------|
| Edit succeeded | Tool response | "Success" or equivalent |
| Syntax valid | Language-specific check | No errors |
| Content correct | Read edited section | Matches expected |
| No collateral damage | `git diff` | Only expected changes |

**Post-Edit Checklist:**
```
[ ] Edit tool reported success
[ ] Syntax check passes
[ ] Re-read confirms correct content
[ ] Git diff shows only intended changes
[ ] No unrelated files modified
```

---

## Git Diff Verification Patterns

### Reviewing Changes Before Commit

**Standard diff review workflow:**

```bash
# Step 1: See what files changed
git status

# Step 2: Review changes in each file
git diff <file>

# Step 3: Verify only expected changes
# Look for:
# - Lines starting with + (additions)
# - Lines starting with - (deletions)
# - Context lines (unchanged)

# Step 4: Check for unintended changes
git diff --stat  # Summary view
```

**Diff verification checklist:**
```
[ ] Only expected files appear in git status
[ ] git diff shows only intended changes
[ ] No commented-out code added
[ ] No debug/logging statements added
[ ] No whitespace-only changes
[ ] No unrelated "improvements"
```

### Double-Check Verification Protocol

For critical changes, use this enhanced verification:

```
DOUBLE-CHECK PROTOCOL:

1. FIRST CHECK (Immediate)
   - Review git diff output
   - Verify expected changes present
   - Confirm no unexpected changes

2. SECOND CHECK (Contextual)
   - Re-read the modified section in context
   - Verify logic is correct
   - Check for typos or errors

3. THIRD CHECK (Functional)
   - Run syntax check
   - Run relevant tests
   - Verify behavior matches expectation
```

---

## Validation Checklists

### Code Change Validation

| Step | Command | Expected Result |
|------|---------|-----------------|
| 1. Syntax check | See language-specific | No errors |
| 2. Related tests | `<test-command> <test-file>` | All pass |
| 3. Git diff review | `git diff` | Only expected changes |
| 4. Re-read edited code | Read tool | Correct content |

### Documentation Change Validation

| Step | Command | Expected Result |
|------|---------|-----------------|
| 1. Markdown valid | Render check | No errors |
| 2. Links work | Check references | All resolve |
| 3. Line count | `wc -l <file>` | Under 500 lines |
| 4. Git diff review | `git diff` | Only expected changes |

---

## Language-Specific Validation

### Python

```bash
# Syntax check
python -m py_compile <file.py>

# Type check (if using mypy)
mypy <file.py>

# Run tests
pytest <test_file.py> -v

# Lint check
flake8 <file.py>
```

### JavaScript / TypeScript

```bash
# Syntax check (Node.js)
node --check <file.js>

# TypeScript compile check
tsc --noEmit <file.ts>

# Run tests
npm test -- --grep "<test-name>"
# or
jest <test_file.js>

# Lint check
eslint <file.js>
```

### Go

```bash
# Syntax/compile check
go build ./...

# Run tests
go test ./... -v

# Lint check
golint ./...
```

### Rust

```bash
# Compile check
cargo check

# Run tests
cargo test

# Lint check
cargo clippy
```

### Generic (Any Language)

```bash
# Verify file is valid (basic)
cat <file> > /dev/null && echo "File readable"

# Check for common issues
grep -n "TODO\|FIXME\|XXX" <file>  # Find incomplete markers
grep -n "console.log\|print(" <file>  # Find debug statements
```

---

## Validation Decision Matrix

| Condition | Action |
|-----------|--------|
| Syntax check passes | Proceed to tests |
| Syntax check fails | HALT, rollback, report |
| Tests pass | Proceed to commit |
| Tests fail | HALT, rollback, report |
| Diff shows expected changes | Proceed |
| Diff shows unexpected changes | HALT, investigate |
| Uncertain about anything | HALT, ask user |

---

## Error Handling

### When Validation Fails

```
1. DO NOT proceed with commit
2. Execute rollback: git checkout HEAD -- <file>
3. Report failure with:
   - Which check failed
   - Error message/output
   - Line numbers if applicable
4. Wait for user guidance
5. DO NOT retry without explicit instruction
```

### Common Validation Errors

| Error | Cause | Resolution |
|-------|-------|------------|
| Syntax error | Invalid code | Rollback, fix code |
| Test failure | Logic error | Rollback, investigate |
| Diff too large | Scope creep | Rollback, limit scope |
| Wrong file modified | Path error | Rollback, verify path |

---

## Integration with To-Do Workflow

Validation should occur at these points:

```
TODO ITEM START
    ↓
[Make changes]
    ↓
✓ POST-EDIT VALIDATION ← Run validation here
    ↓
[If pass] → COMMIT
    ↓
[If fail] → ROLLBACK → Report → Stop
```

See [COMMIT_WORKFLOW.md](./COMMIT_WORKFLOW.md) for commit procedures.

---

## Quick Reference

```
+------------------------------------------------------------------+
|              VALIDATION QUICK REFERENCE                           |
+------------------------------------------------------------------+
| PRE-EDIT:                                                         |
|   [ ] File exists                                                 |
|   [ ] Content at expected location                                |
|   [ ] Rollback command ready                                      |
+------------------------------------------------------------------+
| POST-EDIT:                                                        |
|   [ ] Edit succeeded                                              |
|   [ ] Syntax check passes                                         |
|   [ ] Tests pass                                                  |
|   [ ] Git diff shows only expected changes                        |
+------------------------------------------------------------------+
| IF VALIDATION FAILS:                                              |
|   1. Rollback immediately                                         |
|   2. Report error to user                                         |
|   3. DO NOT proceed                                               |
+------------------------------------------------------------------+
```

---

**Last Updated:** 2026-01-14
**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Line Count:** ~280
