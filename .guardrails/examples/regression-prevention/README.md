# Regression Prevention Examples

> Practical examples demonstrating the Bug Tracking & Regression Prevention System.

**Related:** [../../docs/workflows/REGRESSION_PREVENTION.md](../../docs/workflows/REGRESSION_PREVENTION.md)

---

## Overview

This directory contains realistic examples showing how to use the regression prevention system end-to-end. Each example demonstrates the complete workflow:

1. **Bug Discovery** - Identifying and logging the failure
2. **Root Cause Analysis** - Understanding what went wrong
3. **Fix Implementation** - Writing the fix
4. **Regression Test** - Creating a test to prevent recurrence
5. **Prevention Rule** - Adding automated pattern detection
6. **Documentation** - Recording everything in the registry

---

## Examples Included

| Example | Bug Type | Language | Files Modified |
|---------|----------|----------|----------------|
| [Null Check After Parse](./failure-registry-examples.jsonl) | Runtime Error | JavaScript | parser.js |
| [SQL Injection Prevention](./prevention-rules-examples.json) | Security | Python | db.py |
| [Race Condition Fix](./regression-test-example.py) | Concurrency | Python | cache.py |
| [GitHub Issue Template](./bug-report-template.md) | Workflow | Any | N/A |

---

## Quick Start

### Scenario: Fixing a Bug You Just Found

```bash
# Step 1: Log the bug to the registry
python scripts/log_failure.py --interactive

# Step 2: Fix the bug
# (edit your code)

# Step 3: Create a regression test
# (see regression-test-example.py for template)

# Step 4: Add a prevention rule (if pattern-based)
# (see prevention-rules-examples.json for template)

# Step 5: Run regression check
python scripts/regression_check.py --staged

# Step 6: Commit everything
git add .
git commit -m "fix(parser): add null check after JSON parse

Authored by TheArchitectit"
```

---

## File Organization

```
examples/regression-prevention/
├── README.md                          # This file
├── failure-registry-examples.jsonl    # Example registry entries
├── prevention-rules-examples.json     # Example pattern rules
├── semantic-rules-examples.json       # Example semantic rules
├── regression-test-example.py         # Regression test template
└── bug-report-template.md             # GitHub issue template
```

---

## Real-World Workflow

### The Story: A Bug in the Payment Processing Module

Let us walk through a realistic scenario:

**Day 1: Bug Discovered**
- Production error: `TypeError: Cannot read property 'amount' of undefined`
- Customer payments failing when webhook payload is malformed
- Root cause: Missing validation on webhook data before processing

**Day 1: Immediate Fix**
- Add null check: `if (!payload || !payload.amount) throw new ValidationError(...)`
- Deploy hotfix to production
- Log in failure registry (see example entry FAIL-PAY-001)

**Day 2: Regression Prevention**
- Write regression test (see test_payment_regression_FAIL_PAY_001.py)
- Add prevention rule for direct property access on external payloads
- Update code review checklist to catch similar patterns

**Week 2: Similar Bug in Another Module**
- Prevention rule catches similar pattern in user profile endpoint
- Fix applied before reaching production
- Prevention system working as designed

---

## Best Practices Demonstrated

1. **Immediate Logging** - Log bugs while context is fresh
2. **Pattern Extraction** - Identify the general pattern, not just the specific case
3. **Prevention Rules** - Add automation to catch future occurrences
4. **Regression Tests** - Every fix gets a test that would have caught it
5. **Registry Maintenance** - Keep entries up-to-date with status changes

---

## See Also

- [Full Protocol Documentation](../../docs/workflows/REGRESSION_PREVENTION.md)
- [Failure Registry](../../.guardrails/failure-registry.jsonl)
- [Prevention Rules](../../.guardrails/prevention-rules/)
- [Pre-Work Check](../../.guardrails/pre-work-check.md)

---

**Last Updated:** 2026-02-07
**Authored by:** TheArchitectit
