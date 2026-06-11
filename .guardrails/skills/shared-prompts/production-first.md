# Production-First Rule

Production code MUST be created, validated, and committed before test code or infrastructure code.

## The Rule

**Order of creation:**
1. Production implementation
2. Production validation (lint, type check, compile)
3. Tests for the production code
4. Infrastructure/deployment config (if needed)

**Why this order:**
- Tests written before production code often test assumptions, not reality
- Infrastructure built before code is tested often requires rework
- Production-first ensures the core logic is sound before wrapping it

## Pre-Flight Checklist

Before creating ANY test or infrastructure file:

- [ ] Production implementation exists and is functional
- [ ] Production code passes lint/type-check/compile
- [ ] Production code has been read and reviewed
- [ ] The interface/API is stable enough to test against

## Violation Patterns (NEVER ALLOW)

1. **Test stubs before production code** — `test_foo.py` exists but `foo.py` is empty or missing
2. **Infrastructure before code** — Terraform/CloudFormation written before the service it deploys
3. **Mock-heavy tests with no real implementation** — All tests pass against mocks, but the real code is unwritten
4. **Deployment config before validation** — Dockerfiles, Helm charts written before the app runs locally

## Enforcement

When asked to create tests or infrastructure:

1. Check if production code exists
2. If not: prioritize creating production code first
3. If yes but incomplete: complete production code before adding tests
4. If user explicitly asks for tests first: confirm they understand the production-first rule

## Test Data Rules

- Test data must be clearly fake or synthetic
- Never use production data in tests without sanitization
- Test fixtures must be version-controlled, not generated ad-hoc
- Mock only external dependencies, not the code under test

## References

- `docs/standards/TEST_PRODUCTION_SEPARATION.md` — Full environment separation rules
- `docs/workflows/TESTING_VALIDATION.md` — Testing workflow and validation gates
