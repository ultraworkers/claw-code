# API Security Audit Report

**Repository**: guardrail-mcp
**Component**: mcp-server/internal/web/
**Audit Date**: 2026-02-08
**Auditor**: Security Engineer

## Executive Summary

This audit covers the web layer of the guardrail-mcp server, focusing on authentication, authorization, rate limiting, CORS, input validation, and security headers. **3 Critical, 5 High, 4 Medium, and 3 Low severity issues** were identified.

### Risk Overview
| Severity | Count | Immediate Action Required |
|----------|-------|--------------------------|
| Critical | 3 | Yes |
| High | 5 | Yes |
| Medium | 4 | Recommended |
| Low | 3 | Scheduled |

---

## Critical Issues

### 1. AUTH-BYPASS-001: Path Traversal in Authentication Bypass
**File**: `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/middleware.go:21-28`

**Issue**: The authentication middleware uses `c.Path()` to check if routes should be skipped, but `c.Path()` returns the *route pattern* (e.g., `/health/live`), not the actual request path. A request to `/health/live/../../../api/sensitive` would match the `/health/live` pattern, bypassing authentication.

```go
// VULNERABLE CODE
path := c.Path()  // Returns route pattern, not actual URL
if path == "/health/live" || path == "/health/ready" || path == "/metrics" {
    return next(c)  // Bypasses auth incorrectly
}
```

**Severity**: Critical
**CVSS**: 9.1 (Critical)
**Impact**: Complete authentication bypass, unauthorized API access
**Remediation**:
```go
path := c.Request().URL.Path  // Use actual request path
// Normalize path to prevent traversal
path = filepath.Clean(path)

// Use exact matching with proper path handling
if path == "/health/live" || path == "/health/ready" || path == "/metrics" {
    return next(c)
}
```

---

### 2. RATE-001: Rate Limiting Bypass via IP Spoofing
**File**: `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/middleware.go:105-108`

**Issue**: When API key hash is not available, the middleware falls back to `c.RealIP()` for rate limiting. Echo's `RealIP()` extracts IP from `X-Forwarded-For`/`X-Real-IP` headers without validation, allowing attackers to spoof different IPs and bypass rate limits.

```go
// VULNERABLE CODE
keyHash, ok := c.Get("api_key_hash").(string)
if !ok {
    keyHash = c.RealIP()  // Easily spoofed via headers
}
```

**Severity**: Critical
**CVSS**: 8.2 (High)
**Impact**: Rate limit bypass, potential DoS
**Remediation**:
```go
// Extract IP from trusted source only
func getClientIP(c echo.Context, trustProxy bool) string {
    if !trustProxy {
        // Use direct connection IP
        ip := c.Request().RemoteAddr
        host, _, err := net.SplitHostPort(ip)
        if err == nil {
            return host
        }
        return ip
    }
    // Only trust proxy headers if behind verified load balancer
    return c.RealIP()
}
```

---

### 3. CORS-001: Overly Permissive CORS in Non-Production
**File**: `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/server.go:87-102`

**Issue**: In non-production mode, CORS allows `http://localhost:*` and `https://localhost:*`, which permits any localhost port. This allows malicious websites running on localhost to make authenticated cross-origin requests.

```go
// VULNERABLE CODE - allows any localhost port
corsOrigins = []string{"http://localhost:*", "https://localhost:*"}
```

**Severity**: Critical
**CVSS**: 7.5 (High)
**Impact**: CSRF-style attacks from malicious localhost services
**Remediation**:
```go
// Use specific allowed origins only
corsOrigins = s.cfg.CORSAllowedOrigins
// Validate origins against allowlist
allowedOrigins := map[string]bool{
    "http://localhost:3000": true,
    "http://localhost:8081": true,
    // Add specific allowed origins
}
```

---

## High Severity Issues

### 4. INPUT-001: Insufficient Input Validation on PATCH Endpoints
**File**: `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/handlers.go:283-343`

**Issue**: The `patchRule` handler accepts arbitrary fields in the request body without strict validation. While the struct defines expected fields, Echo's `Bind()` method will ignore unknown fields, potentially allowing injection of unexpected data.

**Severity**: High
**Impact**: Mass assignment vulnerability, data corruption
**Remediation**:
```go
// Use strict validation
var req struct {
    Enabled  *bool   `json:"enabled,omitempty" validate:"omitempty"`
    Name     *string `json:"name,omitempty" validate:"omitempty,max=255"`
    Message  *string `json:"message,omitempty" validate:"omitempty,max=1000"`
    Pattern  *string `json:"pattern,omitempty" validate:"omitempty,max=500"`
    Severity *string `json:"severity,omitempty" validate:"omitempty,oneof=low medium high critical"`
}

// Add strict decoding to reject unknown fields
decoder := json.NewDecoder(c.Request().Body)
decoder.DisallowUnknownFields()
if err := decoder.Decode(&req); err != nil {
    return c.JSON(http.StatusBadRequest, map[string]string{"error": "invalid fields in request"})
}
```

---

### 5. INPUT-002: Missing Content-Type Validation
**Files**: All handlers in `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/handlers.go`

**Issue**: No Content-Type validation on requests. Endpoints accepting JSON don't verify `Content-Type: application/json`, allowing CSRF attacks through HTML forms or content type confusion attacks.

**Severity**: High
**Impact**: CSRF attacks, content type confusion
**Remediation**:
```go
// Add middleware for content type validation
func RequireContentType(contentTypes ...string) echo.MiddlewareFunc {
    return func(next echo.HandlerFunc) echo.HandlerFunc {
        return func(c echo.Context) error {
            if c.Request().Method == http.MethodGet || c.Request().Method == http.MethodDelete {
                return next(c)
            }
            contentType := c.Request().Header.Get("Content-Type")
            for _, allowed := range contentTypes {
                if strings.HasPrefix(contentType, allowed) {
                    return next(c)
                }
            }
            return echo.NewHTTPError(http.StatusUnsupportedMediaType, "invalid content type")
        }
    }
}
```

---

### 6. AUTH-002: Weak IDE Endpoint Authorization Logic
**File**: `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/middleware.go:59-62`

**Issue**: The endpoint restriction logic has a logical error. It checks if path starts with `/ide` and requires `ide` key type, but `mcp` key type is also allowed, making the restriction ineffective.

```go
// INEFFECTIVE CODE
if strings.HasPrefix(path, "/ide") && keyType != "ide" && keyType != "mcp" {
    return echo.NewHTTPError(http.StatusForbidden, "IDE API key required")
}
```

**Severity**: High
**Impact**: Authorization bypass, privilege escalation
**Remediation**:
```go
// Proper endpoint authorization
switch {
case strings.HasPrefix(path, "/ide"):
    if keyType != "ide" {
        return echo.NewHTTPError(http.StatusForbidden, "IDE API key required")
    }
case strings.HasPrefix(path, "/api/admin"):
    if keyType != "admin" {  // Add admin key type
        return echo.NewHTTPError(http.StatusForbidden, "admin key required")
    }
}
```

---

### 7. SEC-001: Missing HSTS Header
**File**: `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/server.go:246-270`

**Issue**: The security headers middleware doesn't include `Strict-Transport-Security` (HSTS), allowing downgrade attacks when TLS is enabled.

**Severity**: High
**Impact**: SSL/TLS downgrade attacks, MITM
**Remediation**:
```go
func securityHeadersMiddleware() echo.MiddlewareFunc {
    return func(next echo.HandlerFunc) echo.HandlerFunc {
        return func(c echo.Context) error {
            // Add HSTS when TLS is enabled
            if c.Request().TLS != nil {
                c.Response().Header().Set(
                    "Strict-Transport-Security",
                    "max-age=31536000; includeSubDomains; preload",
                )
            }
            // ... existing headers ...
            return next(c)
        }
    }
}
```

---

### 8. ERROR-001: Verbose Error Messages in Production
**Files**: Multiple handlers in `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/handlers.go`

**Issue**: Error messages expose internal details (e.g., database errors, file paths) in production. Example: `return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})`

**Severity**: High
**Impact**: Information disclosure, system fingerprinting
**Remediation**:
```go
func handleError(c echo.Context, err error, publicMsg string, isProduction bool) error {
    slog.Error("Request failed", "error", err, "path", c.Path())
    if isProduction {
        return c.JSON(http.StatusInternalServerError, map[string]string{
            "error": publicMsg,
            "request_id": c.Response().Header().Get("X-Request-ID"),
        })
    }
    return c.JSON(http.StatusInternalServerError, map[string]string{"error": err.Error()})
}
```

---

## Medium Severity Issues

### 9. INPUT-003: Insufficient Query Parameter Validation
**File**: `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/handlers.go:116-140`

**Issue**: Search query parameter `q` is accepted without length limits or sanitization, potentially enabling ReDoS (Regular Expression Denial of Service) if regex is used in search.

**Severity**: Medium
**Impact**: ReDoS, resource exhaustion
**Remediation**:
```go
const maxQueryLength = 200

func (s *Server) searchDocuments(c echo.Context) error {
    query := c.QueryParam("q")
    if query == "" {
        return c.JSON(http.StatusBadRequest, map[string]string{"error": "query required"})
    }
    if len(query) > maxQueryLength {
        return c.JSON(http.StatusBadRequest, map[string]string{
            "error": "query too long",
            "max_length": strconv.Itoa(maxQueryLength),
        })
    }
    // Sanitize query to prevent regex injection
    query = sanitizeSearchQuery(query)
    // ... rest of handler
}
```

---

### 10. RATE-002: No Burst Protection for Rate Limiting
**File**: `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/cache/redis.go:282-298`

**Issue**: The rate limiter uses a simple counter without burst bucket or proper sliding window implementation. It resets at window boundaries, allowing burst attacks at minute boundaries.

**Severity**: Medium
**Impact**: Burst attacks at window boundaries
**Remediation**:
```go
// Implement token bucket or proper sliding window
func (dl *DistributedRateLimiter) Allow(ctx context.Context, key string, limit int, burst int) bool {
    now := time.Now().UnixMilli()
    bucketKey := fmt.Sprintf("ratelimit:bucket:%s", key)

    lua := `
        local key = KEYS[1]
        local now = tonumber(ARGV[1])
        local limit = tonumber(ARGV[2])
        local burst = tonumber(ARGV[3])
        local window = tonumber(ARGV[4])

        local bucket = redis.call('hmget', key, 'tokens', 'last_update')
        local tokens = tonumber(bucket[1]) or burst
        local last_update = tonumber(bucket[2]) or now

        local delta = math.max(0, now - last_update)
        tokens = math.min(burst, tokens + (delta * limit / window))

        if tokens < 1 then
            return 0
        end

        tokens = tokens - 1
        redis.call('hset', key, 'tokens', tokens, 'last_update', now)
        redis.call('pexpire', key, window)
        return 1
    `

    result, err := dl.redis.Eval(ctx, lua, []string{bucketKey}, now, limit, burst, 60000).Int()
    if err != nil {
        slog.Error("Rate limiter error", "error", err)
        return false  // Fail closed
    }
    return result == 1
}
```

---

### 11. SEC-002: Version Endpoint Information Disclosure
**File**: `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/server.go:273-280`

**Issue**: The `/version` endpoint exposes version information without authentication, enabling attackers to identify vulnerable versions.

**Severity**: Medium
**Impact**: System fingerprinting, targeted attacks
**Remediation**:
```go
// Require authentication for version endpoint
// Or return minimal information
func (s *Server) versionInfo(c echo.Context) error {
    // Only return version to authenticated requests
    if c.Get("api_key_type") == nil {
        return echo.NewHTTPError(http.StatusUnauthorized, "authentication required")
    }
    // ... existing code ...
}
```

---

### 12. SEC-003: Missing Request Size Validation for Query Parameters
**File**: `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/server.go:109-110`

**Issue**: While body size is limited to 10MB, there's no validation on URL length or query string size, potentially enabling HTTP request smuggling or buffer overflow attacks.

**Severity**: Medium
**Impact**: HTTP request smuggling, DoS
**Remediation**:
```go
// Add URL length validation middleware
func URLLengthLimit(maxLength int) echo.MiddlewareFunc {
    return func(next echo.HandlerFunc) echo.HandlerFunc {
        return func(c echo.Context) error {
            if len(c.Request().URL.String()) > maxLength {
                return echo.NewHTTPError(http.StatusRequestURITooLong, "URL too long")
            }
            return next(c)
        }
    }
}

// Use in setupMiddleware
s.echo.Use(URLLengthLimit(4096))
```

---

## Low Severity Issues

### 13. SEC-004: X-XSS-Protection Deprecated Header
**File**: `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/server.go:264`

**Issue**: The `X-XSS-Protection` header is deprecated and can introduce vulnerabilities in older browsers. CSP is the modern replacement.

**Severity**: Low
**Remediation**:
```go
// Remove X-XSS-Protection header
// CSP already provides better protection
```

---

### 14. LOG-001: Sensitive Information in Logs (API Key Hash)
**File**: `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/middleware.go:66-72`

**Issue**: API key hashes are logged, which could enable offline cracking attempts if logs are compromised.

**Severity**: Low
**Impact**: Information disclosure (requires log access)
**Remediation**:
```go
// Don't log key hashes, only use for rate limiting internally
// Or use HMAC with server secret for log identifiers
```

---

### 15. SEC-005: Race Condition in Cache Invalidation
**File**: `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/handlers.go:334-336`

**Issue**: The `patchRule` handler doesn't use the same error handling pattern as other handlers for cache invalidation, potentially masking failures.

**Severity**: Low
**Impact**: Inconsistent error handling, stale cache
**Remediation**: Standardize cache invalidation error handling across all handlers.

---

## Security Strengths

The following security controls are well-implemented:

1. **Constant-Time API Key Comparison** (`middleware.go:47-50`): Uses `subtle.ConstantTimeCompare` to prevent timing attacks
2. **Secret Scanning** (`handlers.go:90-96`): Documents are scanned for secrets before saving
3. **CSP Implementation** (`server.go:251-261`): Comprehensive Content Security Policy
4. **Panic Recovery** (`server.go:213-244`): Proper panic recovery with stack trace logging
5. **Request Timeout** (`server.go:104-107`): Configurable request timeouts
6. **Body Size Limit** (`server.go:110`): 10MB body limit prevents large payload attacks
7. **Audit Logging** (multiple files): Comprehensive audit logging for changes
8. **Health Check Security** (`server.go:296-313`): Health endpoints don't expose component details

---

## Remediation Priority

### Immediate (Critical & High - Next 7 Days)
1. [ ] Fix AUTH-BYPASS-001: Use actual request path instead of route pattern
2. [ ] Fix RATE-001: Validate proxy headers before using RealIP()
3. [ ] Fix CORS-001: Remove wildcard localhost origins
4. [ ] Fix AUTH-002: Correct IDE endpoint authorization logic
5. [ ] Fix INPUT-001: Add strict PATCH validation
6. [ ] Fix SEC-001: Add HSTS header for TLS connections

### Short-term (High & Medium - Next 30 Days)
7. [ ] Fix INPUT-002: Add Content-Type validation middleware
8. [ ] Fix ERROR-001: Implement production-safe error handling
9. [ ] Fix INPUT-003: Add query parameter length limits
10. [ ] Fix RATE-002: Implement proper token bucket rate limiting

### Long-term (Medium & Low - Next 90 Days)
11. [ ] Fix SEC-002: Protect version endpoint
12. [ ] Fix SEC-003: Add URL length validation
13. [ ] Fix SEC-004: Remove deprecated headers
14. [ ] Fix LOG-001: Reduce sensitive information in logs
15. [ ] Fix SEC-005: Standardize cache invalidation handling

---

## Testing Recommendations

1. **Authentication Bypass Testing**:
   ```bash
   curl "http://localhost:8080/health/live/../../../api/rules" -H "Authorization: Bearer invalid"
   ```

2. **Rate Limit Bypass Testing**:
   ```bash
   for i in {1..1100}; do
     curl -H "X-Forwarded-For: 1.2.3.$i" http://localhost:8080/api/rules
   done
   ```

3. **CORS Testing**:
   ```bash
   curl -H "Origin: http://localhost:9999" -H "Authorization: Bearer $KEY" \
        -X OPTIONS http://localhost:8080/api/rules -v
   ```

4. **Content-Type Testing**:
   ```bash
   curl -X POST http://localhost:8080/api/rules \
        -H "Content-Type: text/plain" \
        -d "malicious data"
   ```

---

## Compliance Mapping

| Issue | OWASP Top 10 2021 | NIST 800-53 | CIS Controls |
|-------|------------------|-------------|--------------|
| AUTH-BYPASS-001 | A01:2021-Broken Access Control | AC-3 | 6.1 |
| RATE-001 | A07:2021-Identification | AC-17 | 13.1 |
| CORS-001 | A07:2021-Identification | AC-4 | 13.1 |
| INPUT-001 | A03:2021-Injection | SI-10 | 3.5 |
| SEC-001 | A02:2021-Cryptographic Failures | SC-8 | 14.4 |

---

## Appendix: File Locations

| File | Lines | Purpose |
|------|-------|---------|
| `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/middleware.go` | 1-133 | Authentication & rate limiting |
| `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/handlers.go` | 1-712 | API endpoint handlers |
| `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/web/server.go` | 1-321 | Server setup & middleware |
| `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/config/config.go` | 1-401 | Configuration & validation |
| `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/cache/redis.go` | 1-356 | Rate limiting implementation |
