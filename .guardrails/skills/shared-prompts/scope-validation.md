# Scope Validation

Verify you are authorized to touch the files you are about to modify.

## The Rule

**Only touch files within the authorized scope.**

Scope is determined by:
1. Explicit file list from the user (highest authority)
2. Files identified in the task description
3. Files discovered through dependency analysis (with approval)
4. Files clearly related to the task (use judgment, but err on the side of asking)

## Scope Check Procedure

Before ANY file modification:

1. **Check the user request** — Did they list specific files?
2. **Check the task description** — Are target files named?
3. **Check for dependencies** — Will modifying file A break file B? If so, is file B in scope?
4. **When in doubt, HALT** — Ask user to confirm scope

## Out-of-Scope Patterns

NEVER do these without explicit user authorization:

1. **"While I'm here" fixes** — Fixing unrelated issues in the same file
2. **Refactoring adjacent code** — Cleaning up nearby code that isn't part of the task
3. **Upgrading dependencies** — Updating libraries not mentioned in the task
4. **Changing configs** — Modifying `.env`, `package.json`, `go.mod`, etc. unless explicitly asked
5. **Deleting files** — Removing code, tests, or docs not mentioned in the task
6. **Adding new files** — Creating files not required by the task description

## Dependency Analysis

When a change requires modifying dependencies:

- If the dependency is a direct child (file imports target file directly): **In scope**
- If the dependency is a sibling or cousin: **Ask user**
- If the dependency is in a different module/service: **HALT and ask**

## Scope Creep Warning Signs

Recognize when scope is expanding:
- "I should also fix..."
- "While I'm here, I might as well..."
- "This would be better if I also..."
- The diff is growing beyond what the task described

**When you notice scope creep: STOP and ask user for confirmation.**

## User Confirmation Template

```
SCOPE CHECK: This task involves files beyond the original request.

Original scope: <files from user request>
Additional files needed: <files you want to touch>
Reason: <why these are needed>

Should I proceed with these additional files, or keep changes limited to the original scope?
```

## References

- `docs/workflows/AGENT_EXECUTION.md` — Execution protocols and scope rules
- `skills/shared-prompts/four-laws.md` — Law 2: Stay in Scope
