# Error Recovery Protocol

How to recover from failures, errors, and unexpected states without making things worse.

## Recovery Principles

1. **Stop the bleeding first**
2. **Understand before fixing**
3. **Never make two changes at once**
4. **Have a rollback plan**

## Recovery Steps

### Step 1: Stop and Assess

- Do NOT immediately try another fix
- Note the exact error message and context
- Determine if the failure is isolated or cascading
- Check if any data was corrupted or lost

### Step 2: Log the Failure

- Document what was attempted
- Document the exact error
- Document the state of the system before and after
- This feeds into the failure registry for regression prevention

### Step 3: Identify Root Cause

- Read the error carefully (stack traces, logs, output)
- Look for the FIRST error, not the last (cascading errors hide the root)
- Check for recent changes that may have triggered it
- Verify environment state (files, configs, dependencies)

### Step 4: Choose Recovery Path

| Situation | Action |
|-----------|--------|
| Clear cause, safe fix | Apply targeted fix |
| Unclear cause | HALT and ask user for guidance |
| Data corruption | Restore from backup or rollback |
| Environment issue | Rebuild/reset environment |
| Dependency issue | Update, downgrade, or pin dependency |

### Step 5: Verify Recovery

- Reproduce the original scenario
- Confirm the fix resolves the issue
- Check no new issues were introduced
- Run relevant tests

## Forbidden Recovery Patterns

NEVER do these when recovering from errors:

1. **Blind retry** — Trying the same thing again without understanding why it failed
2. **Shotgun debugging** — Changing multiple things at once hoping one works
3. **Comment-and-forget** — Commenting out broken code instead of fixing it
4. **Production hotfix without testing** — Pushing fixes directly to production
5. **Ignoring test failures** — "It's just a flaky test" without investigation

## Rollback Checklist

Before any significant change, know your rollback plan:

- [ ] Can you revert the change with a single git command?
- [ ] Is the previous version known to work?
- [ ] Can you restore data if needed?
- [ ] Is there a database migration to undo?
- [ ] Will rollback affect other users/systems?

## Escalation Criteria

Escalate to user when:

- Root cause cannot be determined in 2 attempts
- Recovery requires destructive operations (delete, reset, restore from backup)
- Error affects production data
- You're on your 3rd recovery attempt (Three Strikes Rule)

## References

- `docs/workflows/ROLLBACK_PROCEDURES.md` — Detailed rollback procedures
- `docs/workflows/AGENT_ESCALATION.md` — When and how to escalate
- `docs/standards/TEST_PRODUCTION_SEPARATION.md` — Environment isolation
