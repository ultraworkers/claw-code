# Commit Workflow Guidelines

> When and how to commit, especially between to-do items.

**Related:** [TESTING_VALIDATION.md](./TESTING_VALIDATION.md) | [GIT_PUSH_PROCEDURES.md](./GIT_PUSH_PROCEDURES.md)

---

## Overview

This document defines when and how to commit changes, with special emphasis on the **"commit after each to-do"** rule. Frequent, focused commits create a clean history and enable easy rollback.

---

## When to Commit

### Commit Decision Matrix

| Scenario | Commit? | Rationale |
|----------|---------|-----------|
| Completed a to-do item | **YES** | Checkpoint progress |
| Made a working change | **YES** | Save known-good state |
| Fixed a bug | **YES** | Isolate the fix |
| Added a feature | **YES** | Track feature addition |
| Refactored code | **YES** | Separate from other changes |
| Updated documentation | **YES** | Track doc changes |
| Made partial progress | **MAYBE** | Only if working state |
| Code doesn't compile | **NO** | Never commit broken code |
| Tests failing | **NO** | Fix tests first |
| Debugging in progress | **NO** | Remove debug code first |

### After Each To-Do Rule

**RULE: Commit after completing each to-do item.**

```
TO-DO LIST WORKFLOW:

[ ] To-do 1: Implement feature A
    ↓
    [Complete work]
    ↓
    [Validate] → [COMMIT] ← Commit here
    ↓
[ ] To-do 2: Update tests
    ↓
    [Complete work]
    ↓
    [Validate] → [COMMIT] ← Commit here
    ↓
[ ] To-do 3: Update documentation
    ↓
    [Complete work]
    ↓
    [Validate] → [COMMIT] ← Commit here
```

**Benefits:**
- Each to-do is a discrete, reversible unit
- Easy to rollback individual changes
- Clear history of what was done when
- Progress is preserved even if later work fails

---

## Commit Frequency Patterns

### Single-File Changes

```
Pattern: One commit per logical change

Example:
- Fix bug in parser.py → commit
- Add test for fix → commit
- Update docstring → commit
```

### Multi-File Changes

```
Pattern: Group related changes, commit together

Example:
- Add new endpoint (route + handler + model) → one commit
- Update config and env files → one commit
- Refactor shared utility used by multiple files → one commit
```

### Sprint Task Commits

```
Pattern: Follow sprint steps, commit after each

SPRINT-2026-01-14-add-feature.md:
  STEP 1: Read files → (no commit, read-only)
  STEP 2: Edit code → COMMIT after validation
  STEP 3: Add tests → COMMIT after validation
  STEP 4: Update docs → COMMIT after validation
```

---

## Commit Message Standards

### Format Template

```
<type>(<scope>): <short description>

<optional body - explain why, not what>

Authored by TheArchitectit
```

### Type Reference

| Type | Use For | Example |
|------|---------|---------|
| `fix` | Bug fixes | `fix(parser): handle null input` |
| `feat` | New features | `feat(auth): add OAuth support` |
| `docs` | Documentation | `docs(readme): update install steps` |
| `refactor` | Code restructure | `refactor(api): simplify handler logic` |
| `test` | Test changes | `test(parser): add edge case tests` |
| `chore` | Maintenance | `chore(deps): update packages` |
| `perf` | Performance | `perf(query): add index for lookup` |
| `security` | Security fixes | `security(auth): fix token validation` |

### Good vs Bad Messages

```
BAD:
- "fix bug"
- "update code"
- "changes"
- "WIP"

GOOD:
- "fix(parser): handle empty string input gracefully"
- "feat(api): add rate limiting to public endpoints"
- "docs(readme): add troubleshooting section"
```

### AI Attribution

**All AI-generated commits MUST include:**

```
Authored by TheArchitectit
```
Authored by TheArchitectit
```

Use HEREDOC for proper formatting:

```bash
git commit -m "$(cat <<'EOF'
fix(parser): handle null input gracefully

Added null check to prevent TypeError when input is undefined.

Authored by TheArchitectit
EOF
)"
```

---

## Pre-Commit Checklist

**Before EVERY commit, verify:**

```
[ ] All validation checks pass (see TESTING_VALIDATION.md)
[ ] Only expected files are staged
[ ] Git diff shows only intended changes
[ ] No debug/logging code included
[ ] No secrets or credentials included
[ ] No generated files included
[ ] Commit message follows format
[ ] AI attribution included
```

### Staging Patterns

```bash
# Stage specific file
git add path/to/file.py

# Stage multiple specific files
git add file1.py file2.py

# Review what will be committed
git diff --cached

# Unstage if needed
git reset HEAD path/to/file.py
```

**AVOID:**
```bash
# Don't use these without reviewing first
git add .
git add -A
git add *
```

---

## Commit Verification

### Post-Commit Verification

```bash
# Verify commit was created
git log -1 --oneline

# View full commit details
git show HEAD

# Verify commit contents
git show HEAD --stat
```

### Commit Rollback

If commit was incorrect (before push):

```bash
# Undo commit, keep changes staged
git reset --soft HEAD~1

# Undo commit, keep changes unstaged
git reset HEAD~1

# Undo commit, discard changes (CAUTION)
git reset --hard HEAD~1
```

---

## Integration with To-Do Lists

### Workflow with TodoWrite

```
1. TodoWrite: Mark to-do as "in_progress"
2. Complete the work
3. Validate (see TESTING_VALIDATION.md)
4. git add <files>
5. git commit with proper message
6. TodoWrite: Mark to-do as "completed"
7. Repeat for next to-do
```

### Example Session

```
TodoWrite: [in_progress] "Add input validation"
    ↓
Edit: Add validation code
    ↓
Validate: Syntax + tests pass
    ↓
git add src/validator.py
git commit -m "feat(validator): add input validation

Authored by TheArchitectit"
    ↓
TodoWrite: [completed] "Add input validation"
    ↓
TodoWrite: [in_progress] "Update tests"
    ↓
... continue ...
```

---

## MCP Checkpoint Integration

When using MCP checkpointing:

```
[MCP CHECKPOINT: before-todo-1]
    ↓
Complete to-do 1
    ↓
COMMIT
    ↓
[MCP CHECKPOINT: after-todo-1]
    ↓
Complete to-do 2
    ↓
COMMIT
    ↓
[MCP CHECKPOINT: after-todo-2]
```

See [MCP_CHECKPOINTING.md](./MCP_CHECKPOINTING.md) for details.

---

## Quick Reference

```
+------------------------------------------------------------------+
|              COMMIT WORKFLOW QUICK REFERENCE                      |
+------------------------------------------------------------------+
| WHEN TO COMMIT:                                                   |
|   ✓ After completing each to-do item                             |
|   ✓ After any working change                                      |
|   ✗ Never with failing tests                                      |
|   ✗ Never with debug code                                         |
+------------------------------------------------------------------+
| MESSAGE FORMAT:                                                   |
|   <type>(<scope>): <description>                                  |
|   Authored by TheArchitectit                              |
+------------------------------------------------------------------+
| PRE-COMMIT:                                                       |
|   [ ] Validation passes                                           |
|   [ ] Only expected files staged                                  |
|   [ ] No secrets in changes                                       |
+------------------------------------------------------------------+
```

---

**Last Updated:** 2026-01-14
**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Line Count:** ~280
