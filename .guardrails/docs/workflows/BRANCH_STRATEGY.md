# Branch Strategy Guide

> Git branching conventions and workflows.

**Related:** [COMMIT_WORKFLOW.md](./COMMIT_WORKFLOW.md) | [GIT_PUSH_PROCEDURES.md](./GIT_PUSH_PROCEDURES.md)

---

## Overview

This document defines branch naming conventions, workflows, and management strategies for consistent version control across the project.

---

## Branch Naming Conventions

### Standard Prefixes

| Prefix | Purpose | Example |
|--------|---------|---------|
| `main` or `master` | Production-ready code | `main` |
| `develop` | Integration branch | `develop` |
| `feature/` | New features | `feature/user-auth` |
| `fix/` | Bug fixes | `fix/login-error` |
| `hotfix/` | Emergency production fixes | `hotfix/security-patch` |
| `release/` | Release preparation | `release/v1.2.0` |
| `docs/` | Documentation changes | `docs/api-reference` |
| `refactor/` | Code refactoring | `refactor/api-cleanup` |
| `test/` | Test-related changes | `test/add-integration` |

### Naming Rules

```
FORMAT: <prefix>/<description>

RULES:
- Use lowercase
- Use hyphens for spaces
- Keep names short but descriptive
- Include ticket/issue number if applicable

GOOD:
  feature/user-authentication
  fix/issue-123-null-pointer
  hotfix/critical-security-fix
  docs/update-readme

BAD:
  Feature/UserAuth          (wrong case)
  fix_login_bug             (underscores)
  mybranch                  (no prefix)
  feature/implement-the-new-user-authentication-system  (too long)
```

---

## Main/Master Protection Rules

### Protected Branch Policy

```
MAIN BRANCH RULES:

[ ] No direct commits (use PRs)
[ ] Require code review approval
[ ] Require passing CI/CD checks
[ ] No force push allowed
[ ] No deletion allowed
```

### Working with Protected Branches

```bash
# DON'T: Commit directly to main
git checkout main
git commit -m "change"  # WRONG

# DO: Create feature branch, then PR
git checkout -b feature/my-change
# make changes
git commit -m "feat: add feature"
git push -u origin feature/my-change
# Create PR to merge into main
```

---

## Feature Branch Workflow

### Creating a Feature Branch

```bash
# Start from main (or develop)
git checkout main
git pull origin main

# Create feature branch
git checkout -b feature/my-feature

# Verify branch
git branch --show-current
```

### Working on Feature Branch

```bash
# Make changes, commit frequently
git add <files>
git commit -m "feat(scope): description"

# Push feature branch
git push -u origin feature/my-feature
```

### Completing Feature Branch

```bash
# Update from main before merging
git checkout main
git pull origin main
git checkout feature/my-feature
git merge main  # or rebase

# Resolve any conflicts
# Then create PR or merge

# After merge, delete branch
git branch -d feature/my-feature
git push origin --delete feature/my-feature
```

---

## Hotfix Emergency Procedures

### When to Use Hotfix

```
USE HOTFIX FOR:
- Security vulnerabilities in production
- Critical bugs causing system failure
- Data corruption issues
- Blocking issues for users

DO NOT USE FOR:
- Minor bugs
- Feature requests
- Non-urgent issues
```

### Hotfix Workflow

```bash
# 1. Create hotfix from main
git checkout main
git pull origin main
git checkout -b hotfix/critical-issue

# 2. Make minimal fix
# (Only fix the issue, nothing else)
git add <file>
git commit -m "hotfix: fix critical issue

Description of what was fixed and why.

Authored by TheArchitectit"

# 3. Test thoroughly
# Run critical tests

# 4. Merge to main (via PR or direct if emergency)
# 5. Tag release if needed
# 6. Merge back to develop if applicable
```

### Hotfix Checklist

```
[ ] Issue is truly critical/blocking
[ ] Fix is minimal and focused
[ ] Tests pass
[ ] Reviewed (even briefly)
[ ] Documented the emergency
[ ] Merged to all affected branches
```

---

## Release Branch Management

### Creating Release Branch

```bash
# Create from develop (or main)
git checkout develop
git pull origin develop
git checkout -b release/v1.2.0
```

### Release Branch Activities

```
ALLOWED ON RELEASE BRANCH:
- Bug fixes
- Documentation updates
- Version number updates
- Final testing

NOT ALLOWED:
- New features
- Major refactoring
```

### Completing Release

```bash
# Merge to main
git checkout main
git merge release/v1.2.0
git tag -a v1.2.0 -m "Release v1.2.0"
git push origin main --tags

# Merge back to develop
git checkout develop
git merge release/v1.2.0
git push origin develop

# Delete release branch
git branch -d release/v1.2.0
```

---

## Merge vs Rebase Guidance

### When to Merge

```
USE MERGE WHEN:
- Combining feature branches to main
- Preserving full history is important
- Multiple people worked on branch
- Branch has been pushed/shared

COMMAND:
git merge <branch>
```

### When to Rebase

```
USE REBASE WHEN:
- Updating local branch with main changes
- Branch is local only (not pushed)
- Want linear history
- Cleaning up before PR

COMMAND:
git rebase main
```

### Decision Matrix

| Scenario | Recommendation |
|----------|----------------|
| Feature → main | Merge (or squash merge) |
| main → local feature | Rebase |
| Shared branch | Always merge |
| Local-only branch | Rebase OK |
| After push | Merge only |

### What NOT to Do

```
NEVER REBASE:
- Shared/pushed branches
- After someone else has pulled
- main or develop branches

This destroys others' history and causes conflicts.
```

---

## Branch Cleanup

### Deleting Merged Branches

```bash
# Delete local branch
git branch -d feature/completed-feature

# Delete remote branch
git push origin --delete feature/completed-feature
```

### Finding Stale Branches

```bash
# List merged branches
git branch --merged main

# List branches by last commit date
git for-each-ref --sort=-committerdate refs/heads/
```

---

## Quick Reference

```
+------------------------------------------------------------------+
|              BRANCH STRATEGY QUICK REFERENCE                      |
+------------------------------------------------------------------+
| PREFIXES:                                                         |
|   feature/  - New features                                        |
|   fix/      - Bug fixes                                           |
|   hotfix/   - Emergency fixes                                     |
|   release/  - Release prep                                        |
|   docs/     - Documentation                                       |
+------------------------------------------------------------------+
| WORKFLOW:                                                         |
|   1. Create branch from main                                      |
|   2. Make commits on branch                                       |
|   3. Push branch, create PR                                       |
|   4. Merge to main                                                |
|   5. Delete branch                                                |
+------------------------------------------------------------------+
| RULES:                                                            |
|   ✓ Never commit directly to main                                 |
|   ✓ Never force push shared branches                              |
|   ✓ Never rebase after push                                       |
+------------------------------------------------------------------+
```

---

**Last Updated:** 2026-01-14
**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Line Count:** ~280
