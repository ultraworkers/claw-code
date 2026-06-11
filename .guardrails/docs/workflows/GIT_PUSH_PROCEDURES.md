# Git Push Procedures

> Safety procedures for pushing to remote repositories.

**Related:** [COMMIT_WORKFLOW.md](./COMMIT_WORKFLOW.md) | [BRANCH_STRATEGY.md](./BRANCH_STRATEGY.md)

---

## Overview

Pushing to remote repositories is a significant action that affects collaborators. This document defines safety procedures, verification steps, and error handling for git push operations.

**Key Rule:** Never push without explicit user permission.

---

## Pre-Push Checklist

**Before ANY push, verify ALL items:**

| Step | Command | Verify |
|------|---------|--------|
| 1. Check local status | `git status` | Clean working tree |
| 2. Review commits | `git log origin/<branch>..HEAD` | All commits intended |
| 3. Check remote state | `git fetch && git status` | No conflicts |
| 4. Verify branch | `git branch --show-current` | Correct branch |
| 5. Test passes | Run test command | All green |
| 6. User permission | Ask if not explicit | Confirmed |

```
PRE-PUSH CHECKLIST:

[ ] Working tree is clean (git status)
[ ] All commits reviewed and intended
[ ] Remote fetched, no conflicts
[ ] On correct branch
[ ] All tests pass
[ ] User has given explicit permission to push
```

---

## Push Decision Matrix

| Scenario | Push Allowed? | Conditions |
|----------|---------------|------------|
| Clean commits, user approved | **YES** | All checks pass |
| Clean commits, no approval | **NO** | Wait for user |
| Merge conflicts exist | **NO** | Resolve first |
| Tests failing | **NO** | Fix tests first |
| Uncommitted changes | **NO** | Commit or stash first |
| Force push requested | **HALT** | Escalate to user |
| Push to main/master | **CAREFUL** | Extra verification |

---

## Standard Push Workflow

### Step 1: Verify Local State

```bash
# Check for uncommitted changes
git status

# Expected: "nothing to commit, working tree clean"
# If not clean: commit or stash changes first
```

### Step 2: Verify Remote State

```bash
# Fetch latest from remote
git fetch origin

# Check if ahead/behind
git status

# Expected: "Your branch is ahead of 'origin/<branch>' by X commits"
# If behind: pull first, resolve conflicts
```

### Step 3: Review What Will Be Pushed

```bash
# See commits that will be pushed
git log origin/<branch>..HEAD --oneline

# See diff summary
git diff origin/<branch>..HEAD --stat

# See full diff (if needed)
git diff origin/<branch>..HEAD
```

### Step 4: Execute Push

```bash
# Standard push (after user approval)
git push origin <branch>

# Push with upstream tracking (first push)
git push -u origin <branch>
```

### Step 5: Verify Push Success

```bash
# Confirm push succeeded
git status

# Expected: "Your branch is up to date with 'origin/<branch>'"

# Verify on remote (optional)
git log origin/<branch> -1 --oneline
```

---

## Branch-Specific Procedures

### Main/Master Branch

**Extra precautions for production branches:**

```
BEFORE PUSHING TO MAIN:

[ ] All tests pass
[ ] Code review completed (if applicable)
[ ] No WIP commits
[ ] User explicitly approved push to main
[ ] Consider: Should this be a PR instead?
```

```bash
# Verify you're pushing to main intentionally
git branch --show-current  # Should show: main

# Extra verification
git log origin/main..HEAD --oneline
# Review each commit carefully
```

### Feature Branches

```
Standard procedure:
1. Verify on correct feature branch
2. Run pre-push checklist
3. Push with -u if first push
```

```bash
# First push of feature branch
git push -u origin feature/my-feature

# Subsequent pushes
git push
```

### Hotfix Branches

```
Hotfix branches may have expedited push:
1. Still require user approval
2. Still require tests to pass
3. May skip code review in emergencies
4. Document the emergency
```

---

## Push Safety Rules

### Never Push Without...

```
HARD REQUIREMENTS:

[ ] User permission (explicit or task-defined)
[ ] Clean test suite
[ ] Reviewed commits
[ ] Correct branch confirmed
```

### User Permission Requirements

| Situation | Permission Required |
|-----------|---------------------|
| User explicitly said "push" | Granted |
| Sprint document says "push when done" | Granted |
| User said "commit" (not push) | NOT granted |
| No mention of push | NOT granted |
| Uncertain | ASK user |

### Force Push Prohibition

```
NEVER USE:
- git push --force
- git push -f
- git push --force-with-lease (without explicit approval)

These commands can:
- Destroy collaborator work
- Corrupt shared history
- Cause data loss

If user requests force push:
1. Confirm they understand the risks
2. Verify no collaborators on branch
3. Document the force push reason
```

---

## Error Handling

### Push Rejected Scenarios

**Remote has new commits:**
```bash
# Error: "rejected - non-fast-forward"

# Resolution:
git fetch origin
git merge origin/<branch>  # or git rebase
# Resolve any conflicts
# Then push again
```

**Authentication failed:**
```
1. Report error to user
2. Do NOT retry repeatedly
3. User may need to configure credentials
```

**Branch protection:**
```
1. Report that branch is protected
2. Suggest creating a pull request instead
3. Do NOT attempt to bypass protection
```

### Post-Push Problems

**Pushed something wrong:**
```bash
# If just pushed and need to undo:
# Option 1: Revert (safe, creates new commit)
git revert HEAD
git push

# Option 2: Only if NO ONE has pulled yet
# AND user explicitly approves force push
# (Generally avoid this)
```

---

## Integration with CI/CD

### What Happens After Push

```
TYPICAL CI/CD FLOW:

Push to remote
    ↓
CI/CD triggered (automated)
    ↓
Tests run
    ↓
Build created
    ↓
(Optional) Deploy to staging
```

### Monitoring CI/CD

```bash
# After push, inform user:
"Commit pushed. CI/CD should run automatically.
Monitor at: <CI/CD URL if known>
Or check: gh run list"
```

---

## Quick Reference

```
+------------------------------------------------------------------+
|              GIT PUSH QUICK REFERENCE                             |
+------------------------------------------------------------------+
| BEFORE PUSH:                                                      |
|   [ ] git status - clean tree                                     |
|   [ ] git fetch - check remote                                    |
|   [ ] git log origin/<branch>..HEAD - review commits              |
|   [ ] Tests pass                                                  |
|   [ ] User permission                                             |
+------------------------------------------------------------------+
| PUSH COMMAND:                                                     |
|   git push origin <branch>                                        |
|   git push -u origin <branch>  (first time)                       |
+------------------------------------------------------------------+
| NEVER:                                                            |
|   ✗ Push without permission                                       |
|   ✗ Force push (--force, -f)                                      |
|   ✗ Push failing tests                                            |
+------------------------------------------------------------------+
| IF REJECTED:                                                      |
|   git fetch && git merge origin/<branch>                          |
|   Resolve conflicts, then push again                              |
+------------------------------------------------------------------+
```

---

**Last Updated:** 2026-01-14
**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Line Count:** ~280
