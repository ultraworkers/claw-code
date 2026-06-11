# Three Strikes Rule

Track your attempts on each task. Never continue beyond 3 failures.

## Why Three Strikes?

| Attempt | Meaning | Action |
|---------|---------|--------|
| 1st failure | Simple mistake, wrong assumption, or typo | Retry with adjusted approach |
| 2nd failure | Approach may be fundamentally wrong | Try a completely alternative approach |
| 3rd failure | Fundamental misunderstanding or hidden constraint | **HALT and escalate to user** |

Continuing beyond 3 attempts:
- Wastes tokens and compute
- Contaminates context with failed attempts
- Frustrates the user
- Rarely succeeds ( debugging by random mutation )

## Strike Tracking

Maintain mental (or explicit) state:

```
Task: <description>
Strike 1: <what was attempted, why it failed>
Strike 2: <alternative attempted, why it failed>
Strike 3: <HALT — escalate to user>
```

## What Counts as a Strike?

A strike is counted when:
- A targeted fix or approach is attempted and fails
- The same error recurs after adjustment
- A different error surfaces that indicates the approach is wrong

A strike is NOT counted when:
- A syntax fix is needed as part of the same attempt
- A dependency is missing and must be installed first
- The test environment needs setup

## After the Third Strike

1. **STOP immediately** — Do not attempt a 4th fix
2. **Summarize attempts** — List what was tried and what failed
3. **Describe current state** — What is broken, what works, what is uncertain
4. **Ask user for guidance** — Present options or ask for direction
5. **Wait for response** — Do not proceed until user clarifies

## User Message Template (Third Strike)

```
HALT: Three Strikes

I've attempted this task 3 times without success:

1. <First attempt>: <what was tried> → <result>
2. <Second attempt>: <what was tried> → <result>
3. <Third attempt>: <what was tried> → <result>

Current state:
- <What is working>
- <What is broken>
- <What I am uncertain about>

I need guidance on how to proceed. Possible paths:
A) <Option A>
B) <Option B>
```

## Exceptions

The Three Strikes Rule can be overridden ONLY by explicit user instruction:
- "Keep trying"
- "Try approach X"
- "It's okay, keep going"

Without explicit override, HALT at 3 strikes every time.

## References

- `docs/workflows/AGENT_ESCALATION.md` — When to halt and escalate
- `skills/shared-prompts/halt-conditions.md` — Full halt conditions checklist
