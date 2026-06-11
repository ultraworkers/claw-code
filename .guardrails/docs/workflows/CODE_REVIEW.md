# Code Review Process

> Agent self-review and human review escalation.

**Related:** [TESTING_VALIDATION.md](./TESTING_VALIDATION.md) | [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md)

---

## Overview

This document defines the code review process for AI agents, including self-review checklists, when to escalate to human reviewers, and review standards.

---

## Agent Self-Review Checklist

**Before committing ANY code change, complete this self-review:**

### Code Quality

```
[ ] Code is syntactically correct (syntax check passes)
[ ] Code follows existing style/patterns in the file
[ ] No unnecessary complexity added
[ ] No code duplication introduced
[ ] Variable/function names are clear and descriptive
[ ] No hardcoded values that should be configurable
```

### Security

```
[ ] No secrets, credentials, or API keys in code
[ ] No SQL injection vulnerabilities
[ ] No XSS vulnerabilities (if web code)
[ ] No command injection vulnerabilities
[ ] Input validation present where needed
[ ] No sensitive data logged or exposed
[ ] Test code does not access production database
[ ] Production code does not access test database
[ ] Test users not present in production code
[ ] Production credentials not in test files
[ ] Separate test/production environments verified
```

### Functionality

```
[ ] Change accomplishes the stated goal
[ ] Edge cases considered and handled
[ ] Error handling is appropriate
[ ] No unintended side effects
[ ] Backwards compatibility maintained (if applicable)
```

### Scope

```
[ ] Only files in scope were modified
[ ] No "improvements" to unrelated code
[ ] No feature creep
[ ] No unnecessary refactoring
```

### Testing

```
[ ] Related tests pass
[ ] New tests added if needed
[ ] Test coverage maintained or improved
[ ] Manual verification confirms behavior
```

---

## When to Request Human Review

### Always Request Human Review For:

```
MANDATORY HUMAN REVIEW:

[ ] Security-related changes (auth, encryption, permissions)
[ ] Database schema changes
[ ] API contract changes
[ ] Payment/financial code
[ ] User data handling changes
[ ] Infrastructure/deployment changes
[ ] Changes to security configurations
[ ] Code affecting compliance (GDPR, HIPAA, etc.)
[ ] Test/production separation unclear
[ ] Database connection strings not environment-specific
[ ] Mixed test/production code
[ ] Any change you're uncertain about
```

### Consider Human Review For:

```
RECOMMENDED HUMAN REVIEW:

[ ] Complex algorithms or logic
[ ] Performance-critical code
[ ] Large refactoring efforts
[ ] Changes touching multiple systems
[ ] Code in unfamiliar areas
[ ] First contribution to a codebase
```

### Human Review Not Required For:

```
SKIP HUMAN REVIEW (unless policy requires):

[ ] Simple typo fixes
[ ] Comment/documentation updates
[ ] Formatting changes
[ ] Adding tests for existing code
[ ] Minor bug fixes with clear scope
```

---

## Review Focus Areas

### Security Review Focus

```
LOOK FOR:
- Authentication/authorization bypasses
- Injection vulnerabilities (SQL, command, XSS)
- Sensitive data exposure
- Insecure cryptography
- Missing input validation
- Hardcoded secrets
- Insecure defaults
```

### Performance Review Focus

```
LOOK FOR:
- N+1 query problems
- Missing indexes on queried fields
- Unnecessary loops or iterations
- Memory leaks
- Blocking operations in async code
- Missing caching opportunities
- Large payload sizes
```

### Logic Review Focus

```
LOOK FOR:
- Off-by-one errors
- Null/undefined handling
- Race conditions
- Incorrect comparisons
- Missing edge cases
- Incorrect error handling
- State management issues
```

---

## Review Comment Standards

### Writing Review Comments

```
GOOD COMMENT STRUCTURE:

[SEVERITY] [AREA]: Description

What: Explain the issue
Why: Explain why it's a problem
Suggestion: Provide fix if possible

SEVERITY LEVELS:
- BLOCKER: Must fix before merge
- MAJOR: Should fix, but can discuss
- MINOR: Nice to have, optional
- NIT: Style/preference, definitely optional
```

### Example Comments

```
BLOCKER [Security]: SQL injection vulnerability

What: User input is concatenated directly into SQL query
Why: Allows attackers to execute arbitrary SQL
Suggestion: Use parameterized query instead

---

MAJOR [Performance]: N+1 query in loop

What: Each iteration makes a database call
Why: Will be slow with large datasets
Suggestion: Batch fetch before loop

---

NIT [Style]: Inconsistent naming

What: Using camelCase here but snake_case elsewhere
Why: Consistency improves readability
Suggestion: Use snake_case to match file convention
```

---

## Approval Requirements

### Standard Changes

```
APPROVAL REQUIREMENTS:

- 1 approval from code owner OR
- 1 approval from team member

PLUS:
- All CI checks pass
- No unresolved blocking comments
```

### Sensitive Changes

```
ELEVATED APPROVAL REQUIREMENTS:

- Security changes: Security team approval
- Database changes: DBA approval
- API changes: API owner approval
- Deployment changes: DevOps approval
```

---

## Escalation Procedures

### When Agent Should Escalate

```
ESCALATE TO HUMAN WHEN:

1. Change touches security-sensitive code
   → Request security review

2. Uncertain about correctness
   → Ask for verification

3. Multiple valid approaches exist
   → Ask for guidance

4. Change has broad impact
   → Request architecture review

5. Tests are flaky or unclear
   → Request testing guidance

6. Performance implications unclear
   → Request performance review
```

### How to Escalate

```
ESCALATION FORMAT:

"I've completed [task] but recommend human review before merging.

REASON: [Why escalation is needed]

SPECIFIC CONCERNS:
- [Concern 1]
- [Concern 2]

CHANGES MADE:
- [Change 1]
- [Change 2]

REQUEST: Please review [specific areas]"
```

---

## Review Workflow

### Agent Self-Review Flow

```
COMPLETE CHANGE
    ↓
RUN SELF-REVIEW CHECKLIST
    ↓
ALL CHECKS PASS? ──NO──→ FIX ISSUES → RETRY
    ↓ YES
NEEDS HUMAN REVIEW? ──YES──→ REQUEST REVIEW → WAIT
    ↓ NO
COMMIT AND PROCEED
```

### Human Review Flow

```
REVIEW REQUESTED
    ↓
HUMAN REVIEWS CODE
    ↓
COMMENTS/APPROVES
    ↓
AGENT ADDRESSES COMMENTS
    ↓
RE-REVIEW IF NEEDED
    ↓
APPROVED → MERGE
```

---

## Quick Reference

```
+------------------------------------------------------------------+
|              CODE REVIEW QUICK REFERENCE                          |
+------------------------------------------------------------------+
| SELF-REVIEW BEFORE COMMIT:                                        |
|   [ ] Syntax correct                                              |
|   [ ] Tests pass                                                  |
|   [ ] No secrets in code                                          |
|   [ ] Only expected files changed                                 |
|   [ ] Style matches existing code                                 |
|   [ ] Test/production separation verified                         |
+------------------------------------------------------------------+
| REQUEST HUMAN REVIEW FOR:                                         |
|   • Security changes                                              |
|   • Database changes                                              |
|   • Payment code                                                  |
|   • Test/production separation unclear                            |
|   • Any uncertainty                                               |
+------------------------------------------------------------------+
| ESCALATION:                                                       |
|   1. Complete self-review                                         |
|   2. Identify specific concerns                                   |
|   3. Request review with clear description                        |
|   4. Wait for approval before merge                               |
+------------------------------------------------------------------+
```

---

**Last Updated:** 2026-01-14
**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Line Count:** ~280
