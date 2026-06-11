# Rollback Procedures

> Recovery and undo operations for all scenarios.

**Related:** [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) | [COMMIT_WORKFLOW.md](./COMMIT_WORKFLOW.md)

---

## Overview

This document provides comprehensive rollback procedures for recovering from errors at any stage of the development workflow. Know these commands before making changes.

**Golden Rule:** Always know your rollback command before editing.

---

## Rollback Decision Tree

```
WHAT NEEDS TO BE UNDONE?
    │
    ├── Uncommitted changes
    │   └── See: IMMEDIATE ROLLBACK
    │
    ├── Committed but not pushed
    │   └── See: POST-COMMIT ROLLBACK
    │
    ├── Already pushed
    │   └── See: POST-PUSH ROLLBACK
    │
    ├── Database changes
    │   └── See: DATABASE ROLLBACK
    │
    └── Service/deployment issues
        └── See: SERVICE ROLLBACK
```

---

## Immediate Rollback (Uncommitted Changes)

### Single File Rollback

```bash
# Discard changes to one file
git checkout HEAD -- path/to/file.py

# Verify file restored
git status
cat path/to/file.py | head -20
```

### Multiple Files Rollback

```bash
# Discard changes to specific files
git checkout HEAD -- file1.py file2.py

# Discard all uncommitted changes (CAUTION)
git checkout HEAD -- .
```

### Staged Changes Rollback

```bash
# Unstage file (keep changes in working directory)
git reset HEAD path/to/file.py

# Unstage all files
git reset HEAD

# Then discard working changes if needed
git checkout HEAD -- .
```

### New File Rollback

```bash
# Remove untracked file
rm path/to/new-file.py

# Remove all untracked files (CAUTION)
git clean -fd
```

---

## Post-Commit Rollback (Not Pushed)

### Undo Last Commit (Keep Changes)

```bash
# Undo commit, keep changes staged
git reset --soft HEAD~1

# Undo commit, keep changes unstaged
git reset HEAD~1

# Verify
git status
git log -3 --oneline
```

### Undo Last Commit (Discard Changes)

```bash
# Undo commit AND discard changes (CAUTION)
git reset --hard HEAD~1

# Verify
git status
git log -3 --oneline
```

### Undo Multiple Commits

```bash
# Undo last N commits (keep changes)
git reset --soft HEAD~N

# Undo last N commits (discard changes)
git reset --hard HEAD~N
```

### Fix Commit Message

```bash
# Only if commit not pushed AND you made the commit
git commit --amend -m "corrected message"
```

---

## Post-Push Rollback (Requires Care)

### Safe Rollback (Create Revert Commit)

```bash
# Revert the last commit (creates new commit)
git revert HEAD

# Revert specific commit
git revert <commit-hash>

# Push the revert
git push origin <branch>
```

### Revert Multiple Commits

```bash
# Revert a range (oldest..newest)
git revert --no-commit HEAD~3..HEAD
git commit -m "Revert: undo last 3 commits"
git push
```

### Force Push (LAST RESORT - REQUIRES USER APPROVAL)

```bash
# DANGER: Only with explicit user permission
# DANGER: Only if no one else has pulled

# Reset to previous state
git reset --hard <safe-commit-hash>

# Force push (REQUIRES EXPLICIT APPROVAL)
git push --force origin <branch>
```

**When to consider force push:**
- You're the only one on the branch
- No one has pulled the bad commits
- User explicitly approves
- Documented why force push was necessary

---

## Database Rollback Considerations

### Before Making Database Changes

```
ALWAYS:
[ ] Create backup/snapshot
[ ] Test in non-production first
[ ] Have rollback script ready
[ ] Know point-in-time recovery options
```

### General Database Rollback

```sql
-- If using transactions
ROLLBACK;

-- If changes committed, need restore
-- Use backup/snapshot created before changes
```

### Application-Level Database Rollback

```
If application has migrations:
1. Identify the migration to rollback
2. Run down migration
3. Verify data integrity
4. Update application if needed
```

---

## Service Rollback Procedures

### Container/Docker Rollback

```bash
# Rollback to previous image
docker stop <container>
docker run <previous-image-tag>

# Or with docker-compose
docker-compose down
# Update docker-compose.yml to previous image
docker-compose up -d
```

### Kubernetes Rollback

```bash
# Rollback deployment
kubectl rollout undo deployment/<name>

# Rollback to specific revision
kubectl rollout undo deployment/<name> --to-revision=<N>

# Verify
kubectl rollout status deployment/<name>
```

### Cloud Function Rollback

```bash
# Redeploy previous version
# (Platform specific - AWS Lambda, GCP Functions, etc.)
# Generally: deploy from previous commit or version
```

---

## Disaster Recovery Checklist

### When Everything Goes Wrong

```
IMMEDIATE ACTIONS:

[ ] 1. STOP - Don't make it worse
[ ] 2. ASSESS - What exactly broke?
[ ] 3. COMMUNICATE - Inform stakeholders
[ ] 4. ISOLATE - Prevent further damage
[ ] 5. DOCUMENT - Note what happened
[ ] 6. RECOVER - Use appropriate rollback
[ ] 7. VERIFY - Confirm recovery worked
[ ] 8. POST-MORTEM - Learn from incident
```

### Recovery Priority

```
PRIORITY ORDER:

1. Data integrity - Is data safe?
2. Service availability - Is system up?
3. Functionality - Does it work correctly?
4. Performance - Is it fast enough?
5. Cosmetics - Does it look right?
```

---

## Communication Templates

### Incident Notification

```
INCIDENT: [Brief description]
TIME: [When detected]
IMPACT: [What's affected]
STATUS: [Investigating/Mitigating/Resolved]
ACTIONS: [What's being done]
ETA: [Expected resolution time]
```

### Rollback Completion

```
ROLLBACK COMPLETE

What happened: [Brief description]
What was rolled back: [Commits/changes]
Current state: [Restored to X]
Verification: [How verified]
Next steps: [What happens next]
```

---

## Quick Reference

```
+------------------------------------------------------------------+
|              ROLLBACK QUICK REFERENCE                             |
+------------------------------------------------------------------+
| UNCOMMITTED CHANGES:                                              |
|   git checkout HEAD -- <file>     # Single file                   |
|   git checkout HEAD -- .          # All files                     |
+------------------------------------------------------------------+
| COMMITTED NOT PUSHED:                                             |
|   git reset --soft HEAD~1         # Undo, keep staged             |
|   git reset HEAD~1                # Undo, keep unstaged           |
|   git reset --hard HEAD~1         # Undo, discard (CAUTION)       |
+------------------------------------------------------------------+
| ALREADY PUSHED:                                                   |
|   git revert HEAD                 # Create revert commit (SAFE)   |
|   git push                        # Push the revert               |
+------------------------------------------------------------------+
| NEVER WITHOUT APPROVAL:                                           |
|   git push --force                # Destroys history              |
|   git reset --hard on shared      # Loses others' work            |
+------------------------------------------------------------------+
```

---

## Prevention Checklist

**Before any risky operation:**

```
[ ] Backup exists or can be created
[ ] Rollback command identified
[ ] Impact scope understood
[ ] User informed of risks
[ ] Recovery plan ready
```

---

**Last Updated:** 2026-01-14
**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Line Count:** ~300
