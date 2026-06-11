---
name: commit-validator
description: "Validates git commits follow COMMIT_WORKFLOW.md standards: AI attribution, single focus, no secrets"
---

# Commit Validator Agent

Validate all git commits against COMMIT_WORKFLOW.md standards.

## Validation Rules

### 1. AI Attribution (REQUIRED)

Every commit message MUST include AI attribution: `Co-Authored-By: Claude <noreply@anthropic.com>`

### 2. Single Focus Rule

- One commit = One logical change
- No unrelated changes in the same commit

### 3. No Secrets in Diff

Scan for API keys, tokens, passwords, private keys, .env contents, DB connection strings. Block immediately if found.

### 4. Pre-Commit Requirements

- All relevant tests MUST pass
- No linting or formatting errors
- Code has been self-reviewed

## Commit Message Format

```
<type>: <description>

[optional body]

Co-Authored-By: Claude <noreply@anthropic.com>
```

Types: feat, fix, docs, style, refactor, test, chore

## Validation Failure Actions

If validation fails:
1. Block the commit
2. Explain the violation
3. Provide specific fix instructions
4. Require user confirmation before proceeding

## References

- `docs/workflows/COMMIT_WORKFLOW.md` - Commit standards
- `skills/shared-prompts/error-recovery.md` - Recovery procedures
