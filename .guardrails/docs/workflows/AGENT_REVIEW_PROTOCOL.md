# Agent Review Protocol

> **Trust but verify.** Every agent's work must be reviewed before acceptance.

**Related:** [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) | [ADVERSARIAL_TESTING.md](../standards/ADVERSARIAL_TESTING.md) | [CODE_REVIEW.md](./CODE_REVIEW.md)

---

## Overview

This document establishes the **mandatory review protocol** for AI agent work. No agent's output should be accepted without verification by either another agent, a different LLM, or systematic automated checks.

**Core Principle:** The agent that builds should NOT be the only agent that validates.

---

## WHY AGENT REVIEW IS MANDATORY

### The Hallucination Problem

```
PROBLEM: AI agents confidently produce incorrect output.

SYMPTOMS:
- Code that "looks right" but has subtle bugs
- Tests that pass but don't actually test the right thing
- Logic that works for happy path but fails edge cases
- Confident assertions about code that doesn't exist

SOLUTION: Independent verification breaks the hallucination loop.
```

### The Context Contamination Problem

```
PROBLEM: Long sessions accumulate errors in context.

SYMPTOMS:
- Agent references earlier mistakes as "facts"
- Corrections don't fully override original errors
- Agent defends wrong approaches because they're in context
- Quality degrades as session length increases

SOLUTION: Fresh agent with clean context reviews the work.
```

---

## REVIEW MODELS

### Model 1: Dual-Agent Review (Recommended)

```
┌─────────────────────────────────────────────────────────────┐
│                    DUAL-AGENT REVIEW                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  BUILDER AGENT                    REVIEWER AGENT             │
│  ┌──────────────┐                ┌──────────────┐           │
│  │              │                │              │           │
│  │  Writes code │ ───Output───► │ Reviews code │           │
│  │  Writes tests│                │ Finds issues │           │
│  │              │ ◄───Feedback── │              │           │
│  └──────────────┘                └──────────────┘           │
│                                                              │
│  RULES:                                                      │
│  - Different session (clean context)                         │
│  - Can be same model or different model                     │
│  - Reviewer has NO knowledge of builder's reasoning         │
│  - Reviewer ONLY sees the output, not the process           │
└─────────────────────────────────────────────────────────────┘
```

### Model 2: Cross-Model Review

```
┌─────────────────────────────────────────────────────────────┐
│                   CROSS-MODEL REVIEW                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  MODEL A (Builder)               MODEL B (Reviewer)          │
│  ┌──────────────┐                ┌──────────────┐           │
│  │   Claude     │ ───Output───► │    GPT-4     │           │
│  │   (builds)   │                │  (reviews)   │           │
│  └──────────────┘                └──────────────┘           │
│                                                              │
│  BENEFITS:                                                   │
│  - Different training data = different blind spots          │
│  - Catches model-specific hallucinations                    │
│  - Diverse perspective on code quality                      │
│                                                              │
│  GOOD COMBINATIONS:                                          │
│  - Claude builds → GPT-4 reviews                            │
│  - GPT-4 builds → Claude reviews                            │
│  - Either builds → Gemini reviews                           │
└─────────────────────────────────────────────────────────────┘
```

### Model 3: Specialized Agent Review

```
┌─────────────────────────────────────────────────────────────┐
│                 SPECIALIZED AGENT REVIEW                     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  BUILDER           SECURITY        QA AGENT      ARCHITECT  │
│  AGENT             AGENT                         AGENT      │
│  ┌────────┐        ┌────────┐     ┌────────┐    ┌────────┐ │
│  │        │───────►│        │────►│        │───►│        │ │
│  │ Writes │        │Security│     │ Tests  │    │Pattern │ │
│  │ code   │        │ review │     │ review │    │ review │ │
│  └────────┘        └────────┘     └────────┘    └────────┘ │
│                                                              │
│  SPECIALIST PERSONAS:                                        │
│  - Security Agent: XSS, SQLi, auth vulnerabilities          │
│  - QA Agent: Edge cases, error handling, test coverage      │
│  - Architect Agent: Patterns, dependencies, scalability     │
│  - Performance Agent: Complexity, memory, efficiency        │
└─────────────────────────────────────────────────────────────┘
```

### Model 4: Automated + Agent Hybrid

```
┌─────────────────────────────────────────────────────────────┐
│                  HYBRID REVIEW PIPELINE                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  BUILDER        AUTOMATED         AGENT           HUMAN     │
│  AGENT          CHECKS            REVIEW          REVIEW    │
│  ┌────────┐     ┌────────┐       ┌────────┐      ┌────────┐│
│  │        │────►│ Lint   │──────►│        │─────►│        ││
│  │ Writes │     │ Test   │       │ Semantic│     │ Final  ││
│  │ code   │     │ Types  │       │ review  │     │ approval││
│  └────────┘     └────────┘       └────────┘      └────────┘│
│                                                              │
│  AUTOMATED GATES (Must Pass):                               │
│  - Linting (ESLint, Pylint, etc.)                          │
│  - Type checking (TypeScript, MyPy, etc.)                  │
│  - Unit tests (all pass)                                   │
│  - Security scan (no critical issues)                      │
│                                                              │
│  AGENT REVIEW (After Automated):                            │
│  - Logic review                                             │
│  - Architecture review                                      │
│  - Edge case analysis                                       │
│                                                              │
│  HUMAN REVIEW (For Critical Code):                          │
│  - Security-sensitive code                                  │
│  - Database migrations                                      │
│  - Authentication changes                                   │
└─────────────────────────────────────────────────────────────┘
```

---

## REVIEWER AGENT PROMPTS

### General Code Reviewer Prompt

```markdown
ROLE: Senior Code Reviewer

CONTEXT: You are reviewing code written by another AI agent. 
You have NO knowledge of the original requirements or the 
builder's reasoning. You only see the code output.

TASK: Review the following code for:

1. CORRECTNESS
   - Does the logic appear correct?
   - Are there obvious bugs?
   - Are edge cases handled?

2. SECURITY
   - Any injection vulnerabilities?
   - Any hardcoded secrets?
   - Any authentication issues?

3. QUALITY
   - Is the code readable?
   - Are names descriptive?
   - Is complexity appropriate?

4. COMPLETENESS
   - Are error cases handled?
   - Is validation present?
   - Are tests comprehensive?

CODE TO REVIEW:
```
[paste code here]
```

OUTPUT FORMAT:

## Review Summary
- **Verdict:** APPROVE / REQUEST_CHANGES / REJECT
- **Confidence:** HIGH / MEDIUM / LOW
- **Critical Issues:** [count]
- **Warnings:** [count]

## Critical Issues (Must Fix)
1. [Issue description, file, line]

## Warnings (Should Fix)
1. [Warning description, file, line]

## Suggestions (Optional)
1. [Suggestion]

## Questions for Builder
1. [Clarifying question]
```

### Security-Focused Reviewer Prompt

```markdown
ROLE: Security Engineer (Penetration Tester mindset)

CONTEXT: Review code for security vulnerabilities. 
Assume the code will be attacked. Find every weakness.

CHECKLIST:
[ ] Input validation on all user inputs
[ ] Output encoding for XSS prevention
[ ] Parameterized queries for SQL
[ ] Authentication on protected routes
[ ] Authorization checks before actions
[ ] No hardcoded secrets
[ ] Secure session handling
[ ] Rate limiting present
[ ] Error messages don't leak info
[ ] Dependencies have no known CVEs

CODE TO REVIEW:
```
[paste code here]
```

REPORT VULNERABILITIES BY SEVERITY:
- CRITICAL: Immediate exploit possible
- HIGH: Significant security risk
- MEDIUM: Should be fixed soon
- LOW: Minor security concern
```

### Test Quality Reviewer Prompt

```markdown
ROLE: QA Engineer (Test Specialist)

CONTEXT: Review tests written by another agent.
Verify tests actually test what they claim to test.

CHECKLIST:
[ ] Tests cover happy path
[ ] Tests cover error cases
[ ] Tests cover edge cases
[ ] Tests are independent (no shared state)
[ ] Tests are deterministic (no random failures)
[ ] Assertions are meaningful (not just "doesn't crash")
[ ] Mocks are appropriate (not mocking the thing being tested)
[ ] Test names describe what they test

CODE TO REVIEW:
```
[paste test code here]
```

PRODUCTION CODE BEING TESTED:
```
[paste production code here]
```

IDENTIFY:
1. Missing test cases
2. Weak assertions
3. Tests that would pass even if code is broken
4. Tests that test implementation, not behavior
```

### Architecture Reviewer Prompt

```markdown
ROLE: Software Architect

CONTEXT: Review code for architectural quality.
Ensure it fits the project patterns and is maintainable.

CHECKLIST:
[ ] Follows established patterns (see PROJECT_CONTEXT.md)
[ ] Appropriate separation of concerns
[ ] No circular dependencies
[ ] Appropriate abstraction level
[ ] Scalable design
[ ] Testable design
[ ] Error handling strategy
[ ] Logging strategy

CODE TO REVIEW:
```
[paste code here]
```

PROJECT PATTERNS (from PROJECT_CONTEXT.md):
```
[paste relevant patterns]
```

EVALUATE:
1. Pattern compliance
2. Coupling assessment
3. Cohesion assessment
4. Suggestions for improvement
```

---

## REVIEW WORKFLOW

### Standard Review Flow

```
┌─────────────────────────────────────────────────────────────┐
│                   REVIEW WORKFLOW                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. BUILDER COMPLETES WORK                                   │
│     └── Code written, tests pass, lint passes               │
│                                                              │
│  2. AUTOMATED CHECKS (Blocking)                              │
│     ├── [ ] TypeScript/Lint passes                          │
│     ├── [ ] All tests pass                                  │
│     ├── [ ] Security scan clean                             │
│     ├── [ ] Coverage threshold met                          │
│     ├── [ ] Regression check passes                         │
│     └── [ ] No failure registry patterns matched            │
│                                                              │
│  3. PREPARE REVIEW PACKAGE                                   │
│     ├── Git diff of changes                                 │
│     ├── List of files modified                              │
│     ├── Test results summary                                │
│     ├── Builder's description of changes                    │
│     └── Regression check report                             │
│                                                              │
│  4. AGENT REVIEW (New session/different agent)              │
│     ├── [ ] Logic review                                    │
│     ├── [ ] Security review                                 │
│     ├── [ ] Test quality review                             │
│     ├── [ ] Architecture review                             │
│     ├── [ ] Regression pattern review                       │
│     └── [ ] Previous fixes intact review                    │
│                                                              │
│  5. REVIEW OUTCOME                                           │
│     ├── APPROVE → Proceed to merge                          │
│     ├── REQUEST_CHANGES → Return to builder                 │
│     └── REJECT → Escalate to human                          │
│                                                              │
│  6. IF REQUEST_CHANGES:                                      │
│     ├── Builder addresses feedback                          │
│     ├── Re-run automated checks                             │
│     └── Re-submit for review (max 3 cycles)                │
│                                                              │
│  7. FINAL APPROVAL                                           │
│     └── Commit and merge                                    │
└─────────────────────────────────────────────────────────────┘
```

### Regression Prevention Review Checklist

```
REGRESSION PREVENTION CHECKLIST (for Reviewer):

Before Review:
[ ] Run: python scripts/regression_check.py --staged
[ ] Review any warnings or matches
[ ] Check if files have known bug history

During Review:
[ ] Check if changes touch files with known bugs (see failure-registry.jsonl)
[ ] Verify no regression patterns present in diff
[ ] Verify no known bad patterns are being introduced
[ ] Confirm previous fixes remain intact
[ ] Check for null checks, error handling patterns from previous bugs

For Bug Fixes:
[ ] Confirm regression test exists
[ ] Verify test naming: test_<module>_regression_<failure_id>
[ ] Verify test would fail with old code, pass with fix
[ ] Check if prevention rule should be added/updated

Post Review:
[ ] If new bug found: Ensure builder will log it to registry
[ ] If pattern emerges: Suggest prevention rule addition
```

### Review Package Template

```markdown
# Review Package: [Task Name]

## Summary
- **Builder Agent:** [Agent/Model used]
- **Session ID:** [If applicable]
- **Task:** [Brief description]
- **Files Changed:** [Count]

## Changes Overview
[Brief description of what was done]

## Files Modified
| File | Type | Lines Changed |
|------|------|---------------|
| src/components/Button.tsx | Modified | +45, -12 |
| src/components/Button.test.tsx | Created | +78 |

## Git Diff
```diff
[Paste git diff here]
```

## Test Results
- Total tests: X
- Passed: X
- Failed: 0
- Coverage: X%

## Builder Notes
[Any context the builder wants to provide]

## Review Request
Please review for:
- [ ] Correctness
- [ ] Security
- [ ] Test quality
- [ ] Architecture compliance
```

---

## REVIEW DECISION MATRIX

### When to APPROVE

```
APPROVE if ALL of the following are true:

[ ] No critical issues found
[ ] No security vulnerabilities found
[ ] Code is readable and maintainable
[ ] Tests adequately cover the changes
[ ] Architecture patterns are followed
[ ] No obvious bugs or logic errors

APPROVAL MESSAGE:
"Reviewed and approved. No critical issues found. 
[Optional: Minor suggestions for future consideration]"
```

### When to REQUEST_CHANGES

```
REQUEST_CHANGES if ANY of the following are true:

[ ] Logic error that could cause incorrect behavior
[ ] Missing error handling for likely scenarios
[ ] Tests don't cover important cases
[ ] Code violates project patterns
[ ] Security concern (non-critical)
[ ] Poor naming or readability issues

REQUEST_CHANGES MESSAGE:
"Changes requested. Please address the following before re-review:
1. [Specific issue and suggested fix]
2. [Specific issue and suggested fix]"
```

### When to REJECT

```
REJECT if ANY of the following are true:

[ ] Critical security vulnerability
[ ] Obvious data loss risk
[ ] Fundamental misunderstanding of requirements
[ ] Code is unreviewable (too complex, no structure)
[ ] Three review cycles without resolution

REJECT MESSAGE:
"Review rejected. This requires human intervention because:
[Reason for rejection]

Recommended action:
[What should happen next]"
```

---

## REVIEW CYCLE LIMITS

### Three Strikes Rule

```
REVIEW CYCLE LIMITS:

MAX_REVIEW_CYCLES = 3

Cycle 1: Initial review
  └── If REQUEST_CHANGES → Builder fixes and resubmits

Cycle 2: Second review
  └── If REQUEST_CHANGES → Builder fixes and resubmits

Cycle 3: Final review
  └── If REQUEST_CHANGES → ESCALATE to human
  └── Do NOT allow further agent-only cycles

RATIONALE:
- Prevents infinite loops of fixes and reviews
- Forces escalation when agent can't resolve issues
- Protects against hallucination loops
```

### Context Reset Between Cycles

```
CONTEXT RESET PROTOCOL:

After each review cycle:
1. Start NEW reviewer session (fresh context)
2. Do NOT carry forward previous review comments
3. Reviewer should see ONLY:
   - Current code state
   - Test results
   - Original requirements
4. Prevents bias from previous review attempts
```

---

## AUTOMATION INTEGRATION

### GitHub Actions Review Gate

```yaml
# .github/workflows/agent-review.yml
name: Agent Review Gate

on:
  pull_request:
    types: [opened, synchronize]

jobs:
  automated-checks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Lint
        run: npm run lint
      - name: Type Check
        run: npm run typecheck
      - name: Test
        run: npm test
      - name: Security Scan
        run: npm audit --audit-level=high

  agent-review:
    needs: automated-checks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Prepare Review Package
        run: |
          echo "## Review Package" > review-package.md
          echo "### Files Changed" >> review-package.md
          git diff --name-only ${{ github.event.pull_request.base.sha }} >> review-package.md
          echo "### Diff" >> review-package.md
          git diff ${{ github.event.pull_request.base.sha }} >> review-package.md
      
      - name: Request Agent Review
        # This would call your agent review API/service
        run: |
          # Example: Send to review agent endpoint
          curl -X POST ${{ secrets.AGENT_REVIEW_URL }} \
            -H "Authorization: Bearer ${{ secrets.AGENT_REVIEW_TOKEN }}" \
            -d @review-package.md
      
      - name: Check Review Result
        run: |
          # Check if review passed
          if [ "$REVIEW_RESULT" != "APPROVED" ]; then
            echo "Agent review did not approve changes"
            exit 1
          fi
```

---

## QUICK REFERENCE

```
+------------------------------------------------------------------+
|              AGENT REVIEW PROTOCOL QUICK REFERENCE                |
+------------------------------------------------------------------+
| MODELS:                                                           |
|   1. Dual-Agent: Builder → Reviewer (different session)          |
|   2. Cross-Model: Claude → GPT-4 (or vice versa)                 |
|   3. Specialized: Builder → Security → QA → Architect            |
|   4. Hybrid: Automated → Agent → Human                           |
+------------------------------------------------------------------+
| REVIEW FLOW:                                                      |
|   1. Builder completes work                                       |
|   2. Automated checks pass (blocking)                            |
|   3. Prepare review package (diff, tests, description)           |
|   4. Agent review (new session)                                  |
|   5. APPROVE / REQUEST_CHANGES / REJECT                          |
+------------------------------------------------------------------+
| LIMITS:                                                           |
|   - MAX_REVIEW_CYCLES = 3                                        |
|   - After 3 cycles → Escalate to human                           |
|   - Fresh context for each review cycle                          |
+------------------------------------------------------------------+
| VERDICTS:                                                         |
|   APPROVE: No critical issues, proceed to merge                  |
|   REQUEST_CHANGES: Fixable issues, return to builder             |
|   REJECT: Critical issues, escalate to human                     |
+------------------------------------------------------------------+
| KEY RULE: The agent that builds ≠ the agent that validates       |
+------------------------------------------------------------------+
```

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-01-21
**Line Count:** ~450
