# MCP Server Security Audit Report

**Audit Date:** 2026-02-08
**Auditor:** Claude Security Engineer
**Scope:** /mnt/ollama/git/agent-guardrails-template/mcp-server/
**Language:** Go

## Executive Summary

This security audit analyzed the MCP server codebase for common security vulnerabilities. Overall, the codebase demonstrates **strong security practices** with proper parameterized queries, input validation, and security headers. **No Critical vulnerabilities were identified**. Several medium and low-priority issues were found that should be addressed to improve the security posture.

### Risk Summary

| Severity | Count | Status |
|----------|-------|--------|
| Critical | 0 | - |
| High     | 0 | - |
| Medium   | 4 | Needs attention |
| Low      | 5 | Recommended fixes |
| Info     | 2 | Best practices |

---

## Detailed Findings

### MEDIUM SEVERITY

#### M-001: Insecure Hash Function for API Key Logging

- **File:** /mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/middleware.go
- **Line:** 127-132
- **Severity:** Medium

**Description:**
The `hashAPIKey` function uses SHA-256 truncated to 8 characters for logging API key hashes. While SHA-256 is cryptographically strong, truncating to 8 hex characters (32 bits) provides insufficient collision resistance for an attacker attempting to identify valid API keys from logs.

**Code:**
```go
func hashAPIKey(key string) string {
    var h [32]byte
    h = sha256.Sum256([]byte(key))
    return hex.EncodeToString(h[:8])  // Only 32 bits of entropy
}
```

**Impact:**
An attacker with log access could potentially brute-force the truncated hash to identify which API keys are in use.

**Recommended Fix:**
Increase the truncation length to at least 16 bytes (128 bits) or use a purpose-built key identifier:
```go
func hashAPIKey(key string) string {
    var h [32]byte
    h = sha256.Sum256([]byte(key))
    return hex.EncodeToString(h[:16])  // 64 bits of entropy
}
```

---

#### M-002: Information Disclosure in Error Messages

- **File:** /mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/handlers.go
- **Lines:** 72, 239, 402, 422, 445, 507, 527
- **Severity:** Medium

**Description:**
Multiple handlers return raw error messages from the database layer directly to API consumers, potentially exposing internal implementation details.

**Code Examples:**
```go
// Line 72
return c.JSON(http.StatusNotFound, map[string]string{"error": err.Error()})

// Line 239
return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})

// Line 402
return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
```

**Impact:**
Attackers could use internal error messages to understand database schema, file paths, or system architecture.

**Recommended Fix:**
Log detailed errors internally but return generic messages to clients:
```go
slog.Error("Database error", "error", err, "handler", "createProject")
return c.JSON(http.StatusInternalServerError, map[string]string{
    "error": "An internal error occurred",
})
```

---

#### M-003: Missing Rate Limiting on SSE Endpoint

- **File:** /mnt/ollama/git/agent-guardrails-template/mcp-server/internal/mcp/server.go
- **Line:** 336-346
- **Severity:** Medium

**Description:**
The SSE endpoint at `/mcp/v1/sse` does not implement rate limiting, allowing an attacker to create unlimited sessions and potentially exhaust server resources.

**Code:**
```go
// SSE endpoint - no rate limiting applied
s.echo.GET("/mcp/v1/sse", s.handleSSE)
```

**Impact:**
Resource exhaustion through uncontrolled session creation, potentially leading to denial of service.

**Recommended Fix:**
Add rate limiting middleware specifically for the SSE endpoint:
```go
// Add rate limiter for SSE connections
sseLimiter := tollbooth.NewLimiter(10, nil) // 10 connections per minute
s.echo.GET("/mcp/v1/sse", tollbooth.LimitFuncHandler(sseLimiter, s.handleSSE))
```

---

#### M-004: Missing Input Validation on Project Creation

- **File:** /mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/handlers.go
- **Line:** 395-406
- **Severity:** Medium

**Description:**
The `createProject` handler binds request body directly to the model without calling the `Validate()` method, potentially allowing invalid data to reach the database.

**Code:**
```go
func (s *Server) createProject(c echo.Context) error {
    var proj models.Project
    if err := c.Bind(&proj); err != nil {
        return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid request body"})
    }
    // Missing: if err := proj.Validate(); err != nil { ... }
    if err := s.projStore.Create(c.Request().Context(), &proj); err != nil {
        return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
    }
    return c.JSON(http.StatusCreated, proj)
}
```

**Impact:**
Invalid project data (empty names, invalid slugs) could be stored in the database.

**Recommended Fix:**
Add validation before database operations:
```go
if err := proj.Validate(); err != nil {
    return c.JSON(http.StatusBadRequest, map[string]string{"error": err.Error()})
}
```

---

### LOW SEVERITY

#### L-001: Missing Request Size Limit on MCP Message Endpoint

- **File:** /mnt/ollama/git/agent-guardrails-template/mcp-server/internal/mcp/server.go
- **Line:** 333
- **Severity:** Low

**Description:**
While body limit is set to 1MB, there's no validation that specific message types don't exceed reasonable size limits for their content.

**Recommended Fix:**
Add content-length validation for specific endpoints.

---

#### L-002: Potential Race Condition in Cache Invalidation

- **File:** /mnt/ollama/git/agent-guardrails-template/mcp-server/internal/cache/redis.go
- **Line:** 162-183
- **Severity:** Low

**Description:**
The `InvalidateOnRuleChange` function uses a timeout context for pipeline operations but doesn't verify the pipeline execution result before proceeding to pattern-based deletion.

**Recommended Fix:**
Check pipeline result for partial failures:
```go
cmders, err := pipe.Exec(ctx)
if err != nil {
    return fmt.Errorf("pipeline failed: %w", err)
}
for _, cmder := range cmders {
    if cmder.Err() != nil {
        slog.Warn("Cache deletion partial failure", "error", cmder.Err())
    }
}
```

---

#### L-003: Weak CORS Configuration in Development Mode

- **File:** /mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/server.go
- **Line:** 88-102
- **Severity:** Low

**Description:**
In non-production mode, CORS allows any localhost origin (`http://localhost:*`), which could be exploited by malicious local applications.

**Code:**
```go
if s.cfg.ProductionMode {
    corsOrigins = []string{"http://localhost:8081", "https://localhost:8081"}
} else {
    corsOrigins = []string{"http://localhost:*", "https://localhost:*"}  // Too permissive
}
```

**Recommended Fix:**
Require explicit origin configuration even in development mode.

---

#### L-004: Missing Context Cancellation Check in Session Cleanup

- **File:** /mnt/ollama/git/agent-guardrails-template/mcp-server/internal/mcp/server.go
- **Line:** 835-848
- **Severity:** Low

**Description:**
The session cleanup goroutine doesn't check for context cancellation during the ticker loop, potentially delaying shutdown.

**Recommended Fix:**
```go
func (s *MCPServer) sessionCleanup() {
    ticker := time.NewTicker(5 * time.Minute)
    defer ticker.Stop()
    for {
        select {
        case <-ticker.C:
            // ... cleanup logic
        case <-s.ctx.Done():  // Add context cancellation
            return
        }
    }
}
```

---

#### L-005: Missing Secure Flag in Security Headers

- **File:** /mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/server.go
- **Line:** 246-270
- **Severity:** Low

**Description:**
The security headers middleware doesn't set the `Strict-Transport-Security` (HSTS) header, which is recommended for HTTPS deployments.

**Recommended Fix:**
Add HSTS header when TLS is enabled:
```go
if s.cfg.TLSEnabled {
    c.Response().Header().Set("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
}
```

---

### INFORMATIONAL

#### I-001: Comprehensive ReDoS Protection

- **File:** /mnt/ollama/git/agent-guardrails-template/mcp-server/internal/validation/safe_regex.go
- **Severity:** Info

**Description:**
The codebase implements excellent protection against Regular Expression Denial of Service (ReDoS) attacks using timeout-based regex execution with panic recovery.

**Code:**
```go
func SafeRegex(pattern string, input string, timeout time.Duration) (bool, error) {
    // Uses goroutine with timeout to prevent catastrophic backtracking
    select {
    case result := <-resultChan:
        return result, nil
    case <-time.After(timeout):
        return false, fmt.Errorf("regex timeout after %v - possible ReDoS attack", timeout)
    }
}
```

**Status:** Good security practice implemented.

---

#### I-002: Proper Parameterized Queries

- **File:** /mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/*.go
- **Severity:** Info

**Description:**
All database queries use parameterized statements with `$N` placeholders, effectively preventing SQL injection attacks.

**Example:**
```go
rows, err := s.db.QueryContext(ctx, `
    SELECT id, name, slug FROM projects WHERE slug = $1
`, slug)
```

**Status:** Good security practice implemented.

---

## Positive Security Findings

1. **No SQL Injection:** All database queries use proper parameterization
2. **No Hardcoded Secrets:** All credentials loaded from environment variables
3. **Secure Session Generation:** Uses `crypto/rand` for session IDs (mcp/server.go:822-831)
4. **Constant-Time Comparison:** API keys compared using `subtle.ConstantTimeCompare` (middleware.go:47-50)
5. **ReDoS Protection:** Regex validation with timeout protection (safe_regex.go)
6. **Security Headers:** Comprehensive CSP and security headers implemented
7. **Input Validation:** UUID parsing and slug validation present
8. **Audit Logging:** Comprehensive audit trail for security events
9. **Rate Limiting:** Distributed rate limiting implemented
10. **Secrets Scanning:** Content scanned for secrets before storage

---

## Recommendations Summary

### Immediate Actions (Medium Priority)

1. Fix API key hash truncation (M-001)
2. Sanitize error messages returned to clients (M-002)
3. Add rate limiting to SSE endpoint (M-003)
4. Add input validation to project creation (M-004)

### Short-term Actions (Low Priority)

1. Add HSTS header for TLS deployments
2. Improve CORS configuration in development
3. Add context cancellation to background goroutines
4. Enhance pipeline error handling in cache operations

### Security Best Practices Already Implemented

- Parameterized SQL queries (no SQL injection risk)
- Environment-based secret management
- Cryptographically secure random generation
- Constant-time credential comparison
- ReDoS protection
- Comprehensive security headers

---

## Appendix: Files Audited

### Core Files
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/cmd/server/main.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/handlers.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/middleware.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/server.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/errors.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/mcp/server.go`

### Database Layer
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/postgres.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/projects.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/documents.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/rules.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/database/failures.go`

### Supporting Components
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/security/secrets_scanner.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/config/config.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/cache/redis.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/validation/safe_regex.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/circuitbreaker/breaker.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/models/project.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/models/rule.go`
- `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/audit/logger.go`

---

**Report Generated:** 2026-02-08
**Next Review Recommended:** After addressing Medium priority findings
