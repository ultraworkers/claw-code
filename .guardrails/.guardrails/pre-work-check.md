# Pre-Work Regression Check

**MANDATORY:** Read this document before starting ANY work on this codebase.

---

## Quick Checklist

Before editing any file, verify:

- [ ] **I have read the relevant documentation** (INDEX_MAP.md → specific docs)
- [ ] **I know which files I will modify**
- [ ] **I have checked the Failure Registry** for known bugs in those files
- [ ] **I understand what bugs have been fixed** in this area before
- [ ] **I will not reintroduce known patterns** that caused previous bugs

---

## Active Failures Relevant to Current Work

**Instructions:** Before starting, run:
```bash
python scripts/regression_check.py --all
```

This will show you any potential regressions in your current changes. If you're starting fresh, check the registry for files in your scope:

```bash
python scripts/log_failure.py --list
```

---

## Known Bug Patterns by Category

### Build Failures
- Missing dependencies in imports
- Incorrect build configuration
- Circular dependencies

### Runtime Failures
- Null/undefined access without checks
- Unhandled promise rejections
- Resource leaks (files, connections)

### Test Failures
- Flaky tests without proper setup/teardown
- Tests depending on external state
- Race conditions in async tests

### Type Failures
- Missing type annotations
- Incorrect generic usage
- Type coercion issues

### Config Failures
- Missing environment variables
- Invalid configuration values
- Hardcoded values that should be configurable

---

## Prevention Rules in Effect

The following patterns are automatically checked:

| Rule ID | Pattern | Severity | Description |
|---------|---------|----------|-------------|
| PREVENT-001 | JSON.parse without null check | error | Direct property access on parsed JSON |
| PREVENT-002 | SQL string concatenation | critical | Potential SQL injection |
| PREVENT-003 | Hardcoded credentials | critical | Credentials in source code |

**Run the regression check to see all active rules:**
```bash
python scripts/regression_check.py --verbose
```

---

## Files with Known Bug History

**Note:** This section is populated dynamically. Check the registry for your specific files.

Common high-risk files to be extra careful with:
- Configuration files (easy to misconfigure)
- Authentication/authorization code (security critical)
- Database access layers (data integrity)
- API endpoints (interface contracts)

---

## Required Verification Steps

### Before Starting Work

1. **Identify scope**: List all files you plan to modify
2. **Check registry**: Search for those files in failure-registry.jsonl
3. **Review patterns**: Understand what caused previous bugs
4. **Plan defensively**: Design your changes to avoid known issues

### During Development

1. **Run regression check frequently**:
   ```bash
   python scripts/regression_check.py --unstaged
   ```

2. **Test edge cases** that caused previous bugs

3. **Add regression tests** for any bug fixes you make

### Before Committing

1. **Final regression check**:
   ```bash
   python scripts/regression_check.py --staged
   ```

2. **Review your diff** for any patterns that match known bugs

3. **Verify no previous fixes were undone**

---

## When You Find a New Bug

**YOU MUST:**

1. **Fix the bug first**
2. **Log it in the registry**:
   ```bash
   python scripts/log_failure.py --interactive
   ```
3. **Add a regression test** in `tests/regression/`
4. **Update prevention rules** if applicable

---

## Quick Commands Reference

```bash
# Check for regressions in staged changes
python scripts/regression_check.py

# Check for regressions in unstaged changes
python scripts/regression_check.py --unstaged

# List all active failures
python scripts/log_failure.py --list

# Show details of a specific failure
python scripts/log_failure.py --show FAIL-abc12345

# Mark a failure as resolved
python scripts/log_failure.py --resolve FAIL-abc12345

# Log a new failure interactively
python scripts/log_failure.py --interactive
```

---

## Remember

> **The goal is not to slow you down—it's to prevent the same bugs from being fixed over and over again.**

Every bug in the registry represents:
- Time spent debugging
- Potential user impact
- Technical debt
- Risk of reintroduction

By checking before you start, you save time and prevent regressions.

---

**Last Updated:** Auto-generated from failure-registry.jsonl
**Registry Path:** `.guardrails/failure-registry.jsonl`
**Check Command:** `python scripts/regression_check.py`
