---
name: Bug Report with Failure Registry
about: Report a bug using the regression prevention system format
title: '[BUG] '
labels: bug
assignees: ''
---

<!--
This template integrates with the Bug Tracking & Regression Prevention System.
See: docs/workflows/REGRESSION_PREVENTION.md

When this bug is fixed:
1. Log it to .guardrails/failure-registry.jsonl
2. Create a regression test in tests/regression/
3. Add prevention rule if pattern-based (optional but recommended)
4. Reference this issue in the registry entry
-->

## Bug Description

A clear and concise description of what the bug is.

## To Reproduce

Steps to reproduce the behavior:
1. Go to '...'
2. Run '...'
3. See error

## Expected Behavior

What you expected to happen.

## Actual Behavior

What actually happened.

## Error Output

```
Paste the complete error message, stack trace, or logs here
```

## Environment

- OS: [e.g., Ubuntu 22.04]
- Version: [e.g., v1.0.0]
- Runtime: [e.g., Node.js 18, Python 3.11]
- Other relevant info:

## Additional Context

Add any other context about the problem here.

---

## Regression Prevention Fields

<!-- These fields will be used when logging to the failure registry -->

### Severity Assessment

<!-- Check ONE that applies -->
- [ ] **Critical** - System down, data loss, security breach
- [ ] **High** - Major feature broken, significant user impact
- [ ] **Medium** - Feature partially working, workaround exists
- [ ] **Low** - Cosmetic issue, minor inconvenience

### Category

<!-- Check ONE that applies -->
- [ ] **build** - Build/compilation error
- [ ] **runtime** - Runtime exception/error
- [ ] **test** - Test failure
- [ ] **type** - Type system error
- [ ] **lint** - Style/lint violation
- [ ] **deploy** - Deployment/CI failure
- [ ] **config** - Configuration error
- [ ] **security** - Security vulnerability
- [ ] **performance** - Performance degradation
- [ ] **regression** - Previously fixed bug returned

### Affected Files

<!-- List all files involved in this bug -->
- [ ] `src/...`
- [ ] `tests/...`
- [ ] `config/...`
- [ ] Other: `...`

### Root Cause Analysis

<!-- What caused this bug? Be specific. -->
**Root Cause:**

<!--
Examples:
- Missing null check after external API call
- Race condition in concurrent cache update
- SQL injection via string concatenation
- Missing environment variable validation
-->

### Regression Pattern

<!-- What code pattern could reintroduce this bug? -->
**Pattern:**

<!--
Examples:
- JSON.parse(.*)\.\w+ without null check
- execute(.*+.*) - string concatenation in SQL
- useEffect with addEventListener but no cleanup
- os.environ['VAR'] without try/except or .get()
-->

### Prevention Rule Suggestion

<!-- Should we add an automated rule to catch this pattern? -->
- [ ] Yes, this is a pattern that could recur
- [ ] No, this was a one-time mistake

**Suggested Rule:**

<!-- If yes, describe the pattern to detect: -->

### Fix Verification

<!-- To be filled in after fix is implemented -->

- [ ] Fix implemented
- [ ] Regression test created
- [ ] Prevention rule added (if applicable)
- [ ] Logged to failure registry

**Fix Commit:** `SHA_HERE`

**Failure ID:** `FAIL-XXX-NNN` (assigned after logging)

---

## For AI Agents

<!-- If an AI agent will be fixing this bug, provide additional context -->

### Pre-Work Checklist for Agent

- [ ] Check failure registry for similar bugs in affected files
- [ ] Run regression check: `python scripts/regression_check.py --all`
- [ ] Review related prevention rules

### Agent Instructions

```
Fix this bug following the Regression Prevention Protocol:
1. Read docs/workflows/REGRESSION_PREVENTION.md
2. Implement the fix
3. Create regression test in tests/regression/
4. Log to failure-registry.jsonl using scripts/log_failure.py
5. Run regression check before committing
6. Include "Authored by TheArchitectit" in commit message
```

### Related Issues/PRs

- Related bug: #XXX
- Previous similar issue: #YYY
- Fix PR: #ZZZ (when created)

---

## Example Filled Template

<details>
<summary>Click to see example of completed template</summary>

### Bug Description
Payment webhook endpoint crashes when receiving malformed JSON payload from payment provider.

### Error Output
```
TypeError: Cannot read property 'amount' of undefined
    at parseWebhookPayload (src/webhooks/payment.js:42)
    at handlePaymentWebhook (src/webhooks/payment.js:89)
    at Layer.handle (express/lib/router/layer.js:95)
```

### Severity Assessment
- [x] **High** - Major feature broken, significant user impact

### Category
- [x] **runtime** - Runtime exception/error

### Affected Files
- [x] `src/webhooks/payment.js`
- [x] `src/webhooks/handlers.js`

### Root Cause Analysis
Missing null check after JSON.parse() on external webhook payload. Assumed payment provider always sends valid JSON with required fields.

### Regression Pattern
`JSON.parse\(.*\)\s*\.\w+` - Direct property access on parse result without validation

### Prevention Rule Suggestion
- [x] Yes, this is a pattern that could recur

**Suggested Rule:** Detect JSON.parse result used without null/undefined check

### Fix Verification
- [x] Fix implemented - Added validation layer
- [x] Regression test created - test_payment_regression_FAIL_WEB_001.js
- [x] Prevention rule added - PREVENT-EX-001
- [x] Logged to failure registry - FAIL-WEB-001

**Fix Commit:** `a1b2c3d4e5f6789012345678`

**Failure ID:** `FAIL-WEB-001`

</details>

---

<!--
POST-FIX CHECKLIST (for maintainers):
- [ ] Bug fixed and tested
- [ ] Regression test passes
- [ ] Failure registry updated
- [ ] Prevention rule active (if applicable)
- [ ] Issue closed with reference to fix commit
-->
