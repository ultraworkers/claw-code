# GitHub Secrets & Actions Management

> Secure credential management with GitHub Secrets.

**Related:** [AGENT_GUARDRAILS.md](../docs/AGENT_GUARDRAILS.md) | [LOGGING_INTEGRATION.md](../docs/standards/LOGGING_INTEGRATION.md)

---

## Overview

This document defines how to use GitHub Secrets for secure credential management in CI/CD workflows and agent operations. Never hardcode secrets in code or documentation.

---

## GitHub Secrets Concepts

### What Are GitHub Secrets?

GitHub Secrets are encrypted environment variables that can be used in GitHub Actions workflows. They are:
- Encrypted at rest
- Not exposed in logs
- Only available to authorized workflows
- Rotatable without code changes

### Secret Types

| Type | Scope | Use For |
|------|-------|---------|
| Repository secrets | Single repo | Repo-specific credentials |
| Environment secrets | Specific environment | Deploy-specific credentials |
| Organization secrets | All/selected repos | Shared credentials |

---

## Setting Up Secrets

### Repository Secrets

```
STEPS:
1. Go to repository Settings
2. Navigate to Secrets and variables → Actions
3. Click "New repository secret"
4. Enter name and value
5. Click "Add secret"
```

**Naming example:**
```
Name: API_TOKEN
Value: [paste token value]
```

### Organization Secrets

```
STEPS:
1. Go to organization Settings
2. Navigate to Secrets and variables → Actions
3. Click "New organization secret"
4. Enter name and value
5. Choose repository access:
   - All repositories
   - Private repositories
   - Selected repositories
6. Click "Add secret"
```

### Environment Secrets

```
STEPS:
1. Go to repository Settings
2. Navigate to Environments
3. Create or select environment (e.g., "production")
4. Add secrets specific to that environment
```

---

## Naming Conventions

### Standard Secret Names

| Name Pattern | Purpose | Example |
|--------------|---------|---------|
| `*_TOKEN` | Authentication tokens | `GITHUB_TOKEN`, `NPM_TOKEN` |
| `*_API_KEY` | API keys | `STRIPE_API_KEY`, `DD_API_KEY` |
| `*_PASSWORD` | Passwords | `DB_PASSWORD` |
| `*_SECRET` | Generic secrets | `JWT_SECRET` |
| `*_CREDENTIALS` | Credential JSON | `GCP_CREDENTIALS` |

### Naming Rules

```
DO:
- Use SCREAMING_SNAKE_CASE
- Be descriptive
- Include service name
- Include purpose

DON'T:
- Include environment in name (use environment secrets instead)
- Use generic names like "SECRET1"
- Include sensitive info in name itself
```

### Example Names

```
GOOD:
  DATADOG_API_KEY
  SLACK_WEBHOOK_URL
  AWS_ACCESS_KEY_ID
  AWS_SECRET_ACCESS_KEY
  DATABASE_URL
  SENTRY_DSN

BAD:
  KEY
  TOKEN
  PASSWORD
  PROD_DB_PASSWORD (use environment instead)
```

---

## Accessing Secrets in Actions

### Basic Syntax

```yaml
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Use secret
        run: |
          echo "Deploying..."
        env:
          API_TOKEN: ${{ secrets.API_TOKEN }}
```

### Passing to Steps

```yaml
steps:
  # As environment variable
  - name: With env
    run: ./script.sh
    env:
      MY_SECRET: ${{ secrets.MY_SECRET }}

  # As input to action
  - name: With input
    uses: some/action@v1
    with:
      token: ${{ secrets.TOKEN }}
```

### Environment-Specific Secrets

```yaml
jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: production  # Uses production secrets
    steps:
      - name: Deploy
        env:
          DB_URL: ${{ secrets.DATABASE_URL }}  # From production environment
```

---

## Secret Rotation

### Rotation Schedule

| Secret Type | Rotation Frequency |
|-------------|-------------------|
| API tokens | Every 90 days |
| Service accounts | Every 6 months |
| Database passwords | Every 90 days |
| Signing keys | Annually |
| Compromised secrets | Immediately |

### Rotation Procedure

```
SECRET ROTATION STEPS:

1. Generate new credential in external service
2. Update GitHub Secret with new value
3. Test workflows with new secret
4. Revoke old credential in external service
5. Document rotation date

NO CODE CHANGES NEEDED when rotating secrets.
```

### Post-Rotation Verification

```yaml
# Add a test job to verify secrets work
jobs:
  verify-secrets:
    runs-on: ubuntu-latest
    steps:
      - name: Test API token
        run: |
          curl -H "Authorization: Bearer $TOKEN" https://api.example.com/health
        env:
          TOKEN: ${{ secrets.API_TOKEN }}
```

---

## Security Best Practices

### Do's

```
✓ Use GitHub Secrets for all credentials
✓ Use environment secrets for environment-specific values
✓ Rotate secrets regularly
✓ Use least-privilege tokens
✓ Audit secret usage
✓ Delete unused secrets
```

### Don'ts

```
✗ Hardcode secrets in code
✗ Log secret values
✗ Share secrets via insecure channels
✗ Use same secret across environments
✗ Commit .env files with real values
✗ Print secrets to console/logs
```

### Audit Requirements

```
TRACK:
- Who has access to secrets
- When secrets were last rotated
- Which workflows use which secrets
- Failed secret access attempts
```

---

## Integration with Guardrails

### Agent Access to Secrets

```
AGENTS MUST NOT:
- Store secrets in code or commits
- Log secret values
- Transmit secrets to unauthorized endpoints
- Hardcode values that should be secrets

AGENTS MAY:
- Reference secrets via environment variables
- Use secrets passed by CI/CD workflows
- Read secret configuration (not values) from docs
```

### Secret Exposure Prevention

```yaml
# CI check for accidentally committed secrets
- name: Check for secrets
  uses: gitleaks/gitleaks-action@v2
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

---

## Troubleshooting

### Secret Not Found

```
ERROR: Secret 'MY_SECRET' not found

CAUSES:
- Typo in secret name
- Secret not created
- Wrong repository/organization
- Environment not specified

RESOLUTION:
- Verify secret exists in Settings
- Check exact name spelling
- Verify environment configuration
```

### Secret Masked in Logs

```
BEHAVIOR: GitHub masks secret values in logs

If you see *** in logs, the secret is working.
This is expected security behavior.
```

### Debugging Without Exposing

```yaml
# Check secret is set (not its value)
- name: Verify secret exists
  run: |
    if [ -z "$MY_SECRET" ]; then
      echo "Secret is empty or not set"
      exit 1
    else
      echo "Secret is set (length: ${#MY_SECRET})"
    fi
  env:
    MY_SECRET: ${{ secrets.MY_SECRET }}
```

---

## Quick Reference

```
+------------------------------------------------------------------+
|              SECRETS MANAGEMENT QUICK REFERENCE                   |
+------------------------------------------------------------------+
| ACCESS IN WORKFLOW:                                               |
|   ${{ secrets.SECRET_NAME }}                                      |
+------------------------------------------------------------------+
| NAMING CONVENTION:                                                |
|   SERVICE_PURPOSE (e.g., STRIPE_API_KEY)                          |
+------------------------------------------------------------------+
| ROTATION:                                                         |
|   1. Generate new in external service                             |
|   2. Update GitHub Secret                                         |
|   3. Test workflows                                               |
|   4. Revoke old credential                                        |
+------------------------------------------------------------------+
| NEVER:                                                            |
|   ✗ Hardcode in code                                              |
|   ✗ Log secret values                                             |
|   ✗ Commit .env files                                             |
+------------------------------------------------------------------+
```

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-01-14
**Line Count:** ~280
