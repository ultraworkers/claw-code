---
description: Tracks failure attempts and halts after 3 strikes
globs: "**/*"
alwaysApply: true
---

# Three Strikes Rule

Track your attempts on each task. Never continue beyond 3 failures.

## Strike Table

| Attempt | Meaning | Action |
|---------|---------|--------|
| 1st | Simple mistake | Retry with adjusted approach |
| 2nd | Approach wrong | Try completely alternative approach |
| 3rd | Fundamental misunderstanding | **HALT and escalate to user** |

## Why Three?

Continuing beyond 3 attempts wastes tokens, contaminates context, frustrates users, and rarely succeeds.

## After the Third Strike

1. STOP immediately
2. Summarize attempts
3. Describe current state (works, broken, uncertain)
4. Ask user for guidance with options
5. WAIT for response before proceeding

## Exceptions

Override ONLY with explicit user instruction ("Keep trying", "Try X"). Without explicit override, HALT at 3.
