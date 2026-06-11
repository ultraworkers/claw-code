---
description: Enforces production code is created before tests or infrastructure
globs: "**/*"
alwaysApply: true
---

# Production-First Rule

Production code MUST be created, validated, and committed before test code or infrastructure code.

## The Rule

**Order of creation:**
1. Production implementation
2. Production validation (lint, type check, compile)
3. Tests for the production code
4. Infrastructure/deployment config (if needed)

## Pre-Flight Checklist

Before creating ANY test or infrastructure file:
- [ ] Production implementation exists and is functional
- [ ] Production code passes lint/type-check/compile
- [ ] Production code has been read and reviewed
- [ ] The interface/API is stable enough to test against

## Violation Patterns (NEVER ALLOW)

1. Test stubs before production code
2. Infrastructure before code
3. Mock-heavy tests with no real implementation
4. Deployment config before validation

## Enforcement

When asked to create tests or infrastructure:
1. Check if production code exists
2. If not: prioritize creating production code first
3. If yes but incomplete: complete production code before adding tests
4. If user explicitly asks for tests first: confirm they understand the production-first rule
