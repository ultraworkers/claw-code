# Test/Production Separation Standards

> **MANDATORY**: All testing infrastructure must be fully isolated from production.

**Related:** [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) | [CODE_REVIEW.md](../workflows/CODE_REVIEW.md)

---

## Overview

This document establishes mandatory standards for separating test and production environments. All testing code, data, services, and infrastructure must be completely isolated from production to prevent data corruption, unintended changes, and security incidents.

**BLOCKING VIOLATION**: Any violation of these standards is a blocking error requiring immediate resolution.

---

## CORE MANDATORY RULES

### The Three Laws of Test/Production Separation

```
1. PRODUCTION CODE IS CREATED FIRST
   - Production code MUST exist before any test code
   - Tests MUST NOT create production resources
   - Test code exists ONLY to validate production code

2. ALL TESTING INFRASTRUCTURE IS SEPARATE
   - Separate database instances (not just schemas)
   - Separate service instances
   - Separate user accounts
   - Separate network configurations

3. WHEN IN DOUBT, ASK THE USER
   - If test/production boundary is unclear → ASK
   - If environment separation is ambiguous → ASK
   - Never assume or guess deployment target
```

### Mandatory Pre-Code Checklist

**Before creating ANY code, verify:**

| Check | Requirement | Verify |
|-------|-------------|--------|
| **PRODUCTION FIRST** | Production code created before test code | [ ] |
| **DATABASE ISOLATION** | Test DB is separate instance from prod DB | [ ] |
| **SERVICE ISOLATION** | Test services are separate from prod | [ ] |
| **USER SEPARATION** | Test users are separate from prod users | [ ] |
| **CLEAR LABELING** | Test code is clearly labeled or removed | [ ] |
| **UNCERTAINTY CHECK** | If unclear, have you asked the user? | [ ] |

---

## ENVIRONMENT SEPARATION REQUIREMENTS

### Database Separation

| Environment | Database Instance | Access Rules | Connection Config |
|-------------|------------------|--------------|-------------------|
| **Production** | `prod-db.example.com:5432` | Production users only | `DATABASE_URL=prod://...` |
| **Testing** | `test-db.example.com:5432` | Test users only | `DATABASE_URL=test://...` |
| **Development** | `dev-db.example.com:5432` | Developers only | `DATABASE_URL=dev://...` |

**MANDATORY RULE:**

```
NEVER:
  ✓ Use production database for tests
  ✓ Use test database for production
  ✓ Share connections between environments
  ✓ Use same credentials across environments

ALWAYS:
  ✓ Separate database instances for each environment
  ✓ Separate credentials for each environment
  ✓ Separate connection strings in config
  ✓ Validate database instance before queries
```

### Service Separation

```
PRODUCTION SERVICES:
  API: api.production.com
  Auth: auth.production.com
  Storage: storage.production.com

TEST SERVICES:
  API: api.test.example.com
  Auth: auth.test.example.com
  Storage: storage.test.example.com

DEVELOPMENT SERVICES:
  API: api.dev.example.com
  Auth: auth.dev.example.com
  Storage: storage.dev.example.com
```

**MANDATORY RULE:**

```
NEVER:
  ✓ Use production services for tests
  ✓ Write test code that depends on prod endpoints
  ✓ Route test traffic through production infrastructure

ALWAYS:
  ✓ Configure separate service endpoints per environment
  ✓ Validate service endpoint before requests
  ✓ Use environment-specific configuration files
  ✓ Route test traffic to test environment
```

### User Account Separation

```
PRODUCTION USERS (real data):
  user_001, user_002, user_003...
  - Real data with privacy requirements
  - Protected by compliance policies

TEST USERS (synthetic data):
  test_user_001, test_user_002, qatest_003...
  - Synthetic/fake data only
  - Clearly identified in database
  - Never mixed with production users

ADMINISTRATIVE USERS:
  prod_admin → Production admin only
  test_admin → Test environment only
  dev_admin  → Development only
```

**MANDATORY RULE:**

```
NEVER:
  ✓ Create test users in production database
  ✓ Create real users in test database
  ✓ Share user accounts across environments
  ✓ Use production credentials in test code

ALWAYS:
  ✓ Separate user accounts per environment
  ✓ Clearly label test users with prefixes (test_*, qa_*)
  ✓ Separate authentication systems per environment
  ✓ Use different passwords/tokens per environment
```

---

## CODE CREATION SEQUENCE

### Mandatory Order of Operations

```
STEP 1: CREATE PRODUCTION CODE FIRST
   ↓
   - Implement the feature/functionality
   - Deploy to production environment
   - Verify production code works
   ↓
STEP 2: CREATE TEST INFRASTRUCTURE
   ↓
   - Set up separate test database
   - Set up separate test services
   - Set up test user accounts
   ↓
STEP 3: CREATE TEST CODE
   ↓
   - Write tests for production code
   - Configure test to use test environment
   - Verify tests execute in isolation
   ↓
STEP 4: CLEANUP TEST CODE (optional)
   ↓
   - Remove test code OR
   - Label test code clearly
   - Document test infrastructure
```

### Anti-Pattern: Creating Test Code First

```
WRONG (FORBIDDEN):

[Create test code]
  → [Create test users in production DB]
  → [Run tests on production data]
  → [CORRUPTS PRODUCTION DATABASE]

CORRECT:

[Create production code]
  → [Deploy to production]
  → [Set up separate test environment]
  → [Create test code]
  → [Run tests in isolation]
```

---

## TEST CODE LABELING REQUIREMENTS

### When to Label vs Remove

| Scenario | Action | Rationale |
|----------|--------|-----------|
| Unit tests in test files | **Keep** | Standard test infrastructure |
| Integration tests | **Keep** in test/ directory | Standard pattern |
| Test code mixed with production | **REMOVE** | Code pollution |
| Debug code in production | **REMOVE** | Security risk |
| Mock implementations | **Keep** in mocks/ directory | Standard pattern |

### Labeling Standards

**If test code must coexist with production code, use these labels:**

```javascript
// JavaScript/TypeScript
// @test-only - DO NOT USE IN PRODUCTION
const testHelper = () => { ... };
```

```python
# Python
# @test-only: DO NOT USE IN PRODUCTION
def test_helper():
    pass
```

```rust
// Rust
// @test-only: DO NOT USE IN PRODUCTION
#[cfg(test)]
fn test_helper() {
}
```

```go
// Go
// @test-only: DO NOT USE IN PRODUCTION
// +build test

func testHelper() {
}
```

**Alternative: File-based separation**

```
prod/
  handler.py      ← Production code
  service.js      ← Production code
test/
  handler_test.py ← Test code
  service_test.js ← Test code
```

---

## UNCERTAINTY HANDLING PROTOCOL

### Mandatory Ask Triggers

**ALWAYS ask the end user if ANY of these apply:**

```
UNCERTAINTY TRIGGERS (ALWAYS ASK):

[ ] Not sure if database is test or production
[ ] Environment configuration unclear
[ ] Service endpoints not clearly differentiated
[ ] User account naming convention unclear
[ ] Test code location ambiguous
[ ] Whether to label or remove test code
[ ] Database connection strings not environment-specific
[ ] Configuration files not clearly labeled by environment
```

### Ask Template

**When uncertain, use this template:**

```
UNCERTAINTY QUESTION:

"I need clarification on test/production separation:

CONTEXT: [describe what you're doing]
UNCERTAINTY: [what you're unclear about]
CURRENT ASSUMPTION: [what you would assume]
QUESTION: [what you need answered]

OPTIONS:
1. Option A: [description]
2. Option B: [description]
3. Other: [please specify]

Please confirm which option is correct."
```

### Example Scenarios

**Scenario 1: Database Uncertainty**

```
"UNCERTAINTY QUESTION:

I see database connection: DATABASE_URL=postgres://user:pass@db:5432/app

CONTEXT: Creating test infrastructure
UNCERTAINTY: Is this test database or production database?
CURRENT ASSUMPTION: This is test database based on local host
QUESTION: Should I use this database for tests, or is there a separate test DB?

OPTIONS:
1. Use this database for tests (it's already isolated)
2. Create separate test database (need separate instance)
3. Other: please specify"
```

**Scenario 2: Test Code Placement**

```
"UNCERTAINTY QUESTION:

I need to create test code for user authentication validation.

CONTEXT: Adding authentication feature + tests
UNCERTAINTY: Where should test code be placed?
CURRENT ASSUMPTION: Create test/auth_test.py file
QUESTION: Is this correct, or should tests be in a different location?

OPTIONS:
1. Create test/auth_test.py (standard)
2. Add test code inline in auth.py (with @test-only labels)
3. Other: please specify"
```

---

## VERIFICATION CHECKLISTS

### Pre-Commit Verification

**Before committing ANY changes:**

```
[ ] Production code exists and is committed first
[ ] Test database is separate instance (not just schema)
[ ] Test services are separate endpoints
[ ] Test users have test_ or qa_ prefixes
[ ] Connection strings use different hosts per environment
[ ] No test code in production files
[ ] Test code labeled with @test-only if inline
[ ] Configuration files clearly labeled by environment
[ ] Asked user if any uncertainty existed
[ ] No hardcoded production credentials in test code
```

### Pre-Push Verification

**Before pushing to remote:**

```
[ ] All pre-commit checks pass
[ ] CI/CD pipeline validates test/production separation
[ ] No production database connections in test code
[ ] No test database connections in production code
[ ] Environment variables are properly set
[ ] Secrets not committed to repository
[ ] Documentation updated with test/production setup
[ ] Code review confirms separation
```

### CI/CD Blocking Checks

**These checks must pass or deployment is BLOCKED:**

```
BLOCKING CHECKS (MUST PASS):

✓ No production credentials in test files
✓ No test database connections in production files
✓ Database URLs validate as environment-specific
✓ Service endpoints validate as environment-specific
✓ Test files contain test_ or _test in filename
✓ Production files do NOT contain @test-only code
✓ Environment variables are set correctly
✓ Secrets management validated

IF ANY FAIL: BLOCK DEPLOYMENT
```

---

## EXAMPLES AND PATTERNS

### Good Pattern: Environment-Specific Config

```yaml
# config/production.yaml
environment: production
database:
  host: prod-db.example.com
  port: 5432
  name: production_db
  user: prod_user
  password: ${PROD_DB_PASSWORD}
services:
  api: api.production.com

---

# config/test.yaml
environment: test
database:
  host: test-db.example.com
  port: 5432
  name: test_db
  user: test_user
  password: ${TEST_DB_PASSWORD}
services:
  api: api.test.example.com
```

### Good Pattern: Environment Loading

```python
# config_loader.py
import os
import yaml

ENV = os.getenv('APP_ENV', 'development')

def load_config():
    config_path = f'config/{ENV}.yaml'
    with open(config_path, 'r') as f:
        return yaml.safe_load(f)

config = load_config()

# Usage
database_url = f"postgresql://{config['database']['user']}:"
                f"{config['database']['password']}@"
                f"{config['database']['host']}:"
                f"{config['database']['port']}/"
                f"{config['database']['name']}"
```

### Anti-Pattern: Hardcoded Production URLs

```python
# BAD - FORBIDDEN
def connect_database():
    # WRONG: Hardcoded production URL
    conn = psycopg2.connect("postgresql://prod_user:pass@prod-db:5432/prod_db")
    return conn
```

### Good Pattern: Environment Variable Loading

```python
# GOOD - CORRECT
def connect_database():
    config = load_config()
    conn = psycopg2.connect(
        host=config['database']['host'],
        port=config['database']['port'],
        database=config['database']['name'],
        user=config['database']['user'],
        password=config['database']['password']
    )
    return conn
```

---

## BLOCKING VIOLATIONS

### Immediate Halt Conditions

**STOP IMMEDIATELY if ANY of these are detected:**

```
CRITICAL VIOLATIONS (HALT):

✓ Attempting to create test users in production database
✓ Attempting to write test data in production database
✓ Test code connecting to production database
✓ Production code connecting to test database
✓ Sharing user accounts across environments
✓ sharing service endpoints across environments
✓ Test code in production files without @test-only
✓ Hardcoded production credentials in test files
✓ No environment-specific configuration detected
✓ Uncertainty not resolved and proceeding anyway
```

### Notification Protocol

```
WHEN VIOLATION DETECTED:

1. HALT IMMEDIATELY
2. Report exact violation
3. Report file and line number
4. Explain why it's dangerous
5. Request user guidance
6. DO NOT proceed without user confirmation
```

---

## QUICK REFERENCE

```
+------------------------------------------------------------------+
|         TEST/PRODUCTION SEPARATION QUICK REFERENCE                |
+------------------------------------------------------------------+
| MANDATORY RULES:                                                 |
|   1. Production code CREATED FIRST                               |
|   2. All test infrastructure SEPARATE (DBs, services, users)     |
|   3. If unsure, ASK THE USER                                     |
+------------------------------------------------------------------+
| CHECKLIST:                                                       |
|   [ ] Production code exists first                               |
|   [ ] Test DB is separate instance                               |
|   [ ] Test services are separate endpoints                       |
|   [ ] Test users have test_/qa_ prefixes                         |
|   [ ] Test code labeled or removed                               |
|   [ ] Config files clearly labeled by env                        |
+------------------------------------------------------------------+
| NEVER DO:                                                        |
|   ✗ Create test users in production                             |
|   ✗ Use production DB for tests                                 |
|   ✗ Hardcode prod credentials in test code                      |
|   ✗ Proceed when uncertain                                      |
+------------------------------------------------------------------+
| WHEN UNCERTAIN:                                                  |
|   "I need clarification on test/production separation:"         |
|   → Describe context and uncertainty                            |
|   → Present options                                             |
|   → Wait for user confirmation                                  |
+------------------------------------------------------------------+
| BLOCKING VIOLATIONS:                                             |
|   Test code in prod, prod connections in test code               |
|   → CI/CD blocks deployment if detected                          |
+------------------------------------------------------------------+
```

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Review Cycle:** Monthly
**Last Review:** 2026-01-16
**Next Review:** 2026-02-16
