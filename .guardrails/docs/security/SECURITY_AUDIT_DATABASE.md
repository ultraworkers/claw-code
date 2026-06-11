# Database Security Audit Report

**Repository:** /mnt/ollama/git/agent-guardrails-template
**Focus:** mcp-server/internal/database/
**Audit Date:** 2026-02-08
**Auditor:** Database Administrator Agent

---

## Executive Summary

The database implementation shows **good security practices** with parameterized queries, transaction safety, and input validation. However, several **medium and low severity issues** were identified that should be addressed to improve security posture.

**Overall Security Rating:** B+ (Good with minor improvements needed)

---

## Findings Summary

| Severity | Count | Categories |
|----------|-------|------------|
| Critical | 0 | - |
| High | 0 | - |
| Medium | 3 | Connection Security, Query Timeouts, Schema Permissions |
| Low | 4 | Audit Logging, Input Validation, Error Handling, TLS Defaults |
| Info | 2 | Documentation, Best Practices |

---

## Detailed Findings

### 1. Connection Security - Missing TLS Certificate Verification (Medium)

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/config/config.go`
**Line:** 337-341
**Severity:** Medium

**Issue:** The database connection string construction does not explicitly configure SSL root certificate verification when using `verify-ca` or `verify-full` modes.

**Current Code:**
```go
func (c *Config) DatabaseURL() string {
    return fmt.Sprintf("postgresql://%s:%s@%s:%d/%s?sslmode=%s&connect_timeout=%d",
        c.DBUser, c.DBPassword, c.DBHost, c.DBPort, c.DBName, c.DBSSLMode,
        int(c.DBConnectTimeout.Seconds()))
}
```

**Risk:** When using `verify-ca` or `verify-full` SSL modes, the connection will fail without proper root certificate configuration. The current implementation may silently downgrade to `require` mode in some driver configurations.

**Fix:**
```go
func (c *Config) DatabaseURL() string {
    connStr := fmt.Sprintf("postgresql://%s:%s@%s:%d/%s?sslmode=%s&connect_timeout=%d",
        c.DBUser, c.DBPassword, c.DBHost, c.DBPort, c.DBName, c.DBSSLMode,
        int(c.DBConnectTimeout.Seconds()))

    // Add SSL root certificate for verify modes
    if c.DBSSLMode == "verify-ca" || c.DBSSLMode == "verify-full" {
        if c.DBSSLRootCert != "" {
            connStr += fmt.Sprintf("&sslrootcert=%s", c.DBSSLRootCert)
        }
        if c.DBSSLCert != "" && c.DBSSLKey != "" {
            connStr += fmt.Sprintf("&sslcert=%s&sslkey=%s", c.DBSSLCert, c.DBSSLKey)
        }
    }

    return connStr
}
```

---

### 2. Query Timeout Handling - Missing Statement Timeouts (Medium)

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/postgres.go`
**Line:** 32-63
**Severity:** Medium

**Issue:** The database connection pool configuration does not set PostgreSQL statement timeouts, which could allow long-running queries to consume resources indefinitely.

**Current Code:**
```go
func New(cfg *config.Config) (*DB, error) {
    db, err := sql.Open("pgx", cfg.DatabaseURL())
    // ... pool configuration without statement timeout
}
```

**Risk:** A malicious or buggy query could run indefinitely, causing denial of service through connection pool exhaustion.

**Fix:**
```go
func New(cfg *config.Config) (*DB, error) {
    // Add statement_timeout to connection string or execute after connect
    db, err := sql.Open("pgx", cfg.DatabaseURL())
    if err != nil {
        return nil, fmt.Errorf("failed to open database: %w", err)
    }

    // Set statement timeout to prevent runaway queries
    _, err = db.Exec("SET statement_timeout = '30000'") // 30 seconds
    if err != nil {
        db.Close()
        return nil, fmt.Errorf("failed to set statement timeout: %w", err)
    }

    // ... rest of configuration
}
```

---

### 3. Schema Permissions - Missing Principle of Least Privilege (Medium)

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/migrations/001_create_tables.up.sql`
**Line:** 1-85
**Severity:** Medium

**Issue:** Migration files do not define specific database roles or permissions. The application likely runs with a database user that has excessive privileges (possibly superuser).

**Current State:** No GRANT/REVOKE statements in migrations.

**Risk:** If the application database user is compromised, the attacker could:
- Drop tables or entire schema
- Modify other databases on the same server
- Access sensitive data from other applications

**Fix:** Add migration for application-specific role with minimal privileges:
```sql
-- Create application role with minimal privileges
CREATE ROLE guardrail_app WITH LOGIN PASSWORD 'strong_random_password';

-- Grant only necessary privileges
GRANT CONNECT ON DATABASE guardrails TO guardrail_app;
GRANT USAGE ON SCHEMA public TO guardrail_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO guardrail_app;
GRANT USAGE ON ALL SEQUENCES IN SCHEMA public TO guardrail_app;

-- Explicitly deny dangerous operations
REVOKE CREATE ON SCHEMA public FROM guardrail_app;
REVOKE ALL ON TABLE schema_migrations FROM guardrail_app; -- Protect migrations
```

---

### 4. Audit Logging - Partial Implementation (Low)

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/*.go`
**Severity:** Low

**Issue:** While an `audit_log` table is defined in migrations (001_create_tables.up.sql lines 62-77), no database operations are actually writing to it.

**Current State:** Audit log table exists but is not populated by the application.

**Risk:** Security events (unauthorized access attempts, data modifications) are not tracked at the database level.

**Fix:** Add audit logging wrapper for sensitive operations:
```go
// In each store file, add audit logging
func (s *FailureStore) Create(ctx context.Context, f *models.FailureEntry) error {
    // ... existing transaction code ...

    // Log to audit table
    _, err = tx.ExecContext(ctx, `
        INSERT INTO audit_log (event_id, event_type, severity, actor, action, resource, status, details)
        VALUES ($1, 'failure_created', 'info', $2, 'CREATE', $3, 'success', $4)
    `, generateEventID(), getCurrentActor(ctx), f.FailureID, toJSON(f))

    // ... commit ...
}
```

---

### 5. Search Query Sanitization - Regex Limitations (Low)

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/documents.go`
**Line:** 275-293
**Severity:** Low

**Issue:** The `sanitizeSearchQuery` function uses a regex that may not catch all FTS (Full Text Search) injection attempts.

**Current Code:**
```go
func sanitizeSearchQuery(query string) (string, error) {
    safe := regexp.MustCompile(`[^a-zA-Z0-9\s\-\*"&\|]`)
    cleaned := safe.ReplaceAllString(query, "")
    // ...
}
```

**Risk:** Sophisticated attackers might bypass the regex using Unicode characters or other encoding tricks.

**Fix:** Use allow-list approach with strict validation:
```go
func sanitizeSearchQuery(query string) (string, error) {
    if len(query) > maxSearchQueryLength {
        return "", fmt.Errorf("query too long (max %d chars)", maxSearchQueryLength)
    }

    // Use PostgreSQL's plainto_tsquery which safely handles user input
    // Instead of manual sanitization, rely on parameterized query
    // The current implementation using plainto_tsquery in SQL is actually correct
    // Just ensure the query length check happens before the SQL call

    // Additional check: block common SQL keywords
    lowerQuery := strings.ToLower(query)
    blocked := []string{";", "--", "/*", "*/", "drop", "delete", "insert", "update"}
    for _, b := range blocked {
        if strings.Contains(lowerQuery, b) {
            return "", fmt.Errorf("invalid characters in query")
        }
    }

    return query, nil
}
```

---

### 6. Error Message Information Disclosure (Low)

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/*.go` (multiple files)
**Severity:** Low

**Issue:** Database error messages may leak sensitive information about the database structure.

**Example from documents.go:**
```go
return nil, fmt.Errorf("failed to get document: %w", err)
```

**Risk:** Wrapped errors from the database driver could expose:
- Table names
- Column names
- Database constraints
- Internal file paths

**Fix:** Implement error classification:
```go
func sanitizeDBError(err error, operation string) error {
    if err == nil {
        return nil
    }

    // Log full error internally
    slog.Error("database error",
        "operation", operation,
        "error", err,
    )

    // Return generic error to caller
    switch {
    case errors.Is(err, sql.ErrNoRows):
        return fmt.Errorf("resource not found")
    case IsUniqueViolation(err):
        return fmt.Errorf("resource already exists")
    case IsForeignKeyViolation(err):
        return fmt.Errorf("invalid reference")
    default:
        return fmt.Errorf("database operation failed")
    }
}
```

---

### 7. Connection String Password Exposure in Logs (Low)

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/postgres.go`
**Line:** 56-60
**Severity:** Low

**Issue:** Connection info is logged including host and database name, which could be sensitive.

**Current Code:**
```go
slog.Info("Database connected",
    "max_conns", maxConns,
    "host", cfg.DBHost,
    "database", cfg.DBName,
)
```

**Risk:** While password is not logged, exposing internal hostnames and database names aids reconnaissance.

**Fix:**
```go
slog.Info("Database connected",
    "max_conns", maxConns,
    // Omit host and database names in production
)
```

---

### 8. Migration Down Scripts - Data Loss Risk (Info)

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/migrations/001_create_tables.down.sql`
**Severity:** Info

**Issue:** Down migrations use `CASCADE` which could unintentionally drop dependent objects.

**Current Code:**
```sql
DROP TABLE IF EXISTS audit_log CASCADE;
DROP TABLE IF EXISTS failure_registry CASCADE;
-- etc.
```

**Note:** This is acceptable for development/testing but should be used with extreme caution in production.

**Recommendation:** Document that down migrations should be tested thoroughly and run only during maintenance windows with backups.

---

### 9. Array Parameter Handling (Info)

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/failures.go`
**Line:** 208-243
**Severity:** Info

**Status:** SECURE - The code properly uses parameterized queries with PostgreSQL array parameters.

**Code:**
```go
rows, err := s.db.QueryContext(ctx, query, files)  // files is []string
```

The pgx driver handles array parameterization correctly, preventing SQL injection through array elements.

---

## Positive Security Findings

### 1. SQL Injection Prevention - EXCELLENT

All database queries use parameterized statements with numbered placeholders (`$1`, `$2`, etc.). No string concatenation is used for query construction.

**Files:** All store files (documents.go, rules.go, failures.go, projects.go)

### 2. Transaction Safety - EXCELLENT

All write operations (CREATE, UPDATE, DELETE) use proper transactions with:
- Explicit BEGIN
- Deferred ROLLBACK for cleanup
- Explicit COMMIT on success
- Error handling for both operations

### 3. Input Validation - GOOD

Models have comprehensive `Validate()` methods:
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/models/document.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/models/project.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/models/rule.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/models/failure.go`

### 4. Dynamic Query Building - SECURE

The `FailureStore.List()` method uses a type-safe switch statement instead of string concatenation for optional filters.

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/failures.go` lines 46-120

### 5. Connection Pool Security - GOOD

Connection pooling is properly configured with:
- Max connection limits
- Connection lifetime limits
- Idle connection timeouts
- Health checks with retry logic

---

## Recommendations by Priority

### Immediate (Address within 1 week)
1. Add `statement_timeout` configuration to prevent runaway queries
2. Implement production database role with minimal privileges
3. Add SSL certificate configuration options for verify modes

### Short-term (Address within 1 month)
4. Implement audit logging for all CRUD operations
5. Sanitize error messages to prevent information disclosure
6. Review and harden search query sanitization

### Long-term (Address within 3 months)
7. Implement database activity monitoring
8. Set up automated backup encryption verification
9. Add query performance monitoring and slow query alerts

---

## Compliance Notes

| Requirement | Status | Notes |
|-------------|--------|-------|
| SQL Injection Prevention | PASS | All queries parameterized |
| Transaction Safety | PASS | Proper commit/rollback handling |
| Input Validation | PASS | Model validation in place |
| Audit Logging | PARTIAL | Table exists but not populated |
| Least Privilege | FAIL | No role-based permissions defined |
| Connection Encryption | PARTIAL | SSL mode configurable but not enforced |
| Query Timeouts | FAIL | No statement timeout configured |

---

## Appendix: Files Reviewed

### Go Source Files
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/postgres.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/tx.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/failures.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/documents.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/rules.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/projects.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/metrics.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/config/config.go`

### Model Files
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/models/document.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/models/project.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/models/rule.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/models/failure.go`

### SQL Migration Files
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/migrations/001_create_tables.up.sql`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/migrations/001_create_tables.down.sql`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/migrations/002_add_indexes.up.sql`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/migrations/003_add_triggers.up.sql`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/migrations/004_fix_indexes.up.sql`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/migrations/005_schema_versioning.up.sql`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/migrations/006_partition_management.up.sql`

---

*Report generated by Database Administrator Agent*
*Follow security best practices and regularly audit database implementations*
