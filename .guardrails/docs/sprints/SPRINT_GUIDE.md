# Sprint Documentation Guide

**Version:** 1.0
**Last Updated:** 2026-01-10

---

## Purpose

Sprint documents provide AI agents with precise, unambiguous instructions for executing tasks. A well-written sprint document enables any agent (Claude, GPT, Gemini, etc.) to complete a task without interpretation or guesswork.

---

## When to Create a Sprint Document

Create a sprint document when:

| Scenario | Create Sprint? |
|----------|----------------|
| Bug fix requiring code changes | YES |
| New feature implementation | YES |
| Refactoring task | YES |
| Complex multi-step task | YES |
| Simple one-line fix | OPTIONAL |
| Research/exploration task | NO |
| Documentation update | OPTIONAL |

---

## Sprint Document Structure

### Required Sections

```
1. HEADER
   - Date, archive date, focus, priority, effort, status

2. SAFETY PROTOCOLS
   - Pre-execution checklist (always include)
   - Link to full guardrails

3. PROBLEM STATEMENT
   - What's wrong and why
   - Error messages/symptoms
   - Root cause

4. SCOPE BOUNDARY
   - What CAN be modified
   - What CANNOT be modified

5. EXECUTION DIRECTIONS
   - Visual flow diagram
   - Step-by-step instructions

6. ACCEPTANCE CRITERIA
   - How to verify success

7. ROLLBACK PROCEDURE
   - How to undo if things go wrong

8. QUICK REFERENCE CARD
   - One-box summary
```

### Optional Sections

```
- Reference implementation (for complex fixes)
- Related files/documentation
- Historical context
- Alternative approaches considered
```

---

## Writing Effective Steps

### Good Step Example

```markdown
### STEP 3: Apply The Fix

**Action:** Edit file with exact replacement

TOOL: Edit
FILE: /path/to/file.py

OLD_STRING (exact match required):
    def broken_function():
        return x.value  # Bug here

NEW_STRING (exact replacement):
    def broken_function():
        if hasattr(x, 'value'):
            return x.value
        return x

**Checkpoint:** Edit tool confirms success

**Decision Point:**
- [ ] Edit succeeded → Proceed to STEP 4
- [ ] Edit failed → HALT and report to user
```

### Bad Step Example (Avoid)

```markdown
### STEP 3: Fix the bug

Fix the function to handle the edge case properly.
```

**Why it's bad:**
- No specific tool call
- No exact code to use
- No checkpoint
- No decision point
- Agent must interpret/guess

---

## Key Principles

### 1. Be Explicit About Everything

```
BAD:  "Update the configuration"
GOOD: "Edit /src/config.json, change 'timeout' from 30 to 60"
```

### 2. Provide Exact Code

```
BAD:  "Add error handling"
GOOD: "Replace lines 45-50 with this exact code: [code block]"
```

### 3. Include Decision Points

Every step should have:
- Success condition → What to do next
- Failure condition → What to do instead

### 4. Define Scope Clearly

```
IN SCOPE:
  - File: src/utils/parser.py
  - Lines: 120-135
  - Change: Add null check

OUT OF SCOPE:
  - All other files
  - All other functions in parser.py
  - Tests (read-only)
```

### 5. Make Rollback Easy

Always include the exact rollback command:
```bash
git checkout HEAD -- src/utils/parser.py
```

---

## Naming Convention

```
SPRINT-YYYY-MM-DD-brief-description.md

Examples:
SPRINT-2026-01-10-swarm-enum-fix.md
SPRINT-2026-01-15-add-user-auth.md
SPRINT-2026-01-20-refactor-api-routes.md
```

---

## Archive Policy

| Sprint Age | Action |
|------------|--------|
| 0-7 days | Active - May be executed |
| 7-30 days | Archive - Move to `docs/sprints/archive/` |
| 30+ days | Review - May be outdated, verify before use |

---

## Priority Levels

| Priority | Meaning | Response Time |
|----------|---------|---------------|
| P0 | Critical - System down | Immediate |
| P1 | Blocking - Feature broken | Same day |
| P2 | Normal - Standard task | This sprint |
| P3 | Low - Nice to have | When convenient |

---

## Status Values

| Status | Meaning |
|--------|---------|
| PENDING | Not yet started |
| IN_PROGRESS | Agent is working on it |
| COMPLETE | Successfully finished |
| BLOCKED | Waiting for external input |
| FAILED | Could not complete |

---

## Checklist for Sprint Authors

Before publishing a sprint document, verify:

```
[ ] Header is complete (date, archive, priority, effort)
[ ] Safety protocols section included
[ ] Problem statement is clear
[ ] Root cause is identified
[ ] Scope is explicitly defined
[ ] Each step has a specific tool call
[ ] Each step has exact code/commands
[ ] Each step has a checkpoint
[ ] Each step has decision points
[ ] Acceptance criteria are testable
[ ] Rollback procedure is included
[ ] Quick reference card is accurate
```

---

## Example: Minimal Sprint

For simple fixes, you can use a condensed format:

```markdown
# Sprint: Fix Typo in Error Message

**Date:** 2026-01-10 | **Archive:** 2026-01-17 | **Priority:** P3

## Task
Fix typo "recieved" → "received" in error handler.

## Scope
- File: `src/errors.py`
- Line: 42

## Steps
1. Read `src/errors.py` lines 40-45
2. Edit: Replace `"Data recieved"` with `"Data received"`
3. Verify: `python -m py_compile src/errors.py`
4. Commit: `git commit -m "fix(errors): correct typo in error message"`

## Rollback
git checkout HEAD -- src/errors.py
```

---

## Template Quick Copy

Copy the full template from: [SPRINT_TEMPLATE.md](./SPRINT_TEMPLATE.md)

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Review Cycle:** Quarterly
