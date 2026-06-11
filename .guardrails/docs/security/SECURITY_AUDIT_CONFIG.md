# Security Audit: Configuration Files and Environment Handling

**Audit Date:** 2026-02-08
**Auditor:** Security Engineer
**Scope:** mcp-server/ configuration files, environment handling, secrets management
**Risk Rating:** HIGH - Multiple insecure defaults and secret exposure risks identified

---

## Executive Summary

This audit identified **11 security issues** across configuration files, with severity ranging from Critical to Low. The primary concerns are insecure default values that could lead to production deployments without proper security hardening, and potential secret exposure in container environments.

### Risk Distribution
- **CRITICAL:** 2 issues
- **HIGH:** 3 issues
- **MEDIUM:** 4 issues
- **LOW:** 2 issues

---

## Critical Issues

### 1. Database SSL Disabled by Default in Compose
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/deploy/podman-compose.yml:144`
**Risk:** CRITICAL

```yaml
- DB_SSLMODE=${DB_SSLMODE:-disable}
```

**Security Risk:**
- Database connections default to unencrypted communication
- Credentials and data transmitted in plaintext
- Susceptible to man-in-the-middle attacks
- Compliance violations (PCI-DSS, HIPAA, SOC2)

**Recommendation:**
```yaml
# Change default to require minimum
- DB_SSLMODE=${DB_SSLMODE:-require}
# For production, use verify-full
- DB_SSLMODE=${DB_SSLMODE:-verify-full}
```

**Remediation Priority:** IMMEDIATE

---

### 2. Redis Authentication Disabled by Default in Compose
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/deploy/podman-compose.yml:149`
**Risk:** CRITICAL

```yaml
- REDIS_USE_TLS=${REDIS_USE_TLS:-false}
```

**Security Risk:**
- Redis connections default to unencrypted communication
- Session data and cached secrets transmitted in plaintext
- Redis password exposed in environment variables

**Recommendation:**
```yaml
# Enable TLS by default
- REDIS_USE_TLS=${REDIS_USE_TLS:-true}
# Add certificate verification
- REDIS_TLS_VERIFY=${REDIS_TLS_VERIFY:-true}
```

**Remediation Priority:** IMMEDIATE

---

## High Issues

### 3. Permissive CORS Default
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/.env.example:39`
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/deploy/podman-compose.yml:174`
**Risk:** HIGH

```bash
# .env.example
CORS_ALLOWED_ORIGINS=*

# podman-compose.yml
CORS_ALLOWED_ORIGINS=${CORS_ALLOWED_ORIGINS:-*}
```

**Security Risk:**
- Allows cross-origin requests from any domain
- Enables CSRF attacks against the API
- Session hijacking via malicious websites
- Violates principle of least privilege

**Recommendation:**
```bash
# .env.example - No default, must be explicitly configured
CORS_ALLOWED_ORIGINS=  # REQUIRED: Set to your domain(s)

# In config.go validation, reject wildcard in production
func (c *Config) Validate() error {
    if c.ProductionMode && len(c.CORSAllowedOrigins) == 1 && c.CORSAllowedOrigins[0] == "*" {
        return fmt.Errorf("CORS_ALLOWED_ORIGINS cannot be '*' in production mode")
    }
    // ...
}
```

**Remediation Priority:** HIGH

---

### 4. Redis Password Exposure in Health Check
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/deploy/podman-compose.yml:39`
**Risk:** HIGH

```yaml
healthcheck:
  test: ["CMD", "redis-cli", "-a", "${REDIS_PASSWORD}", "ping"]
```

**Security Risk:**
- Password visible in `docker inspect` output
- Password appears in container logs if health check fails
- Process listing may expose command with password
- Container runtime history stores password in plain text

**Recommendation:**
```yaml
# Use Redis ACL file or environment file instead
healthcheck:
  test: ["CMD", "redis-cli", "--raw", "incr", "ping"]
  # Configure requirepass in redis.conf, not command line
```

Or use a health check script:
```yaml
healthcheck:
  test: ["CMD", "sh", "-c", "redis-cli -a \"$REDIS_PASSWORD\" ping | grep PONG"]
```

**Remediation Priority:** HIGH

---

### 5. Placeholder Secrets Could Be Used in Production
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/.env.example:102-109`
**Risk:** HIGH

```bash
MCP_API_KEY=generate_a_32_byte_random_key_here
IDE_API_KEY=generate_a_different_32_byte_random_key_here
JWT_SECRET=generate_a_64_byte_random_secret_here_for_jwt_signing
```

**Security Risk:**
- Placeholder values may be accidentally used in production
- No runtime validation to detect placeholder secrets
- Weak entropy in placeholder strings
- Predictable secrets enable authentication bypass

**Recommendation:**
Add validation in `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/config/config.go`:

```go
func ValidateAPIKey(key, name string) error {
    // Existing validation...

    // Check for placeholder patterns
    placeholders := []string{
        "generate_a_",
        "change_me",
        "placeholder",
        "example",
        "test",
        "demo",
        "secret_here",
    }

    lowerKey := strings.ToLower(key)
    for _, placeholder := range placeholders {
        if strings.Contains(lowerKey, placeholder) {
            return fmt.Errorf("%s appears to be a placeholder value", name)
        }
    }

    return nil
}
```

**Remediation Priority:** HIGH

---

## Medium Issues

### 6. Production Mode Defaults to False
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/.env.example:182`
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/deploy/podman-compose.yml:130`
**Risk:** MEDIUM

```bash
PRODUCTION_MODE=false
```

**Security Risk:**
- Stricter security checks disabled by default
- Debug features may be enabled unintentionally
- Security headers not enforced
- Rate limiting may be relaxed

**Recommendation:**
Make `PRODUCTION_MODE` required with no default:
```bash
# .env.example
PRODUCTION_MODE=  # REQUIRED: Set to 'true' for production, 'false' for development
```

Add validation:
```go
func (c *Config) Validate() error {
    if !c.ProductionMode {
        slog.Warn("Running in development mode - security features relaxed")
    }
    // ...
}
```

**Remediation Priority:** MEDIUM

---

### 7. TLS Minimum Version Allows 1.2
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/config/config.go:75`
**Risk:** MEDIUM

```go
TLSMinVersion string `env:"TLS_MIN_VERSION" envDefault:"1.3"`
```

**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/config/config.go:214-216`

```go
if c.TLSMinVersion != "1.2" && c.TLSMinVersion != "1.3" {
    return fmt.Errorf("TLS_MIN_VERSION must be 1.2 or 1.3, got %s", c.TLSMinVersion)
}
```

**Security Risk:**
- TLS 1.2 has known vulnerabilities (POODLE, BEAST)
- Allows downgrade attacks
- TLS 1.3 should be minimum for production

**Recommendation:**
```go
func (c *Config) Validate() error {
    if c.TLSEnabled {
        // ...
        if c.ProductionMode && c.TLSMinVersion != "1.3" {
            return fmt.Errorf("TLS_MIN_VERSION must be 1.3 in production mode")
        }
        if c.TLSMinVersion != "1.2" && c.TLSMinVersion != "1.3" {
            return fmt.Errorf("TLS_MIN_VERSION must be 1.2 or 1.3, got %s", c.TLSMinVersion)
        }
    }
    // ...
}
```

**Remediation Priority:** MEDIUM

---

### 8. Database Password in .env.example Uses Weak Placeholder
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/.env.example:62`
**Risk:** MEDIUM

```bash
DB_PASSWORD=change_me_in_production
```

**Security Risk:**
- Weak placeholder may be used accidentally
- No validation prevents this value in production
- Common pattern that attackers test for

**Recommendation:**
```bash
# Leave empty to force configuration
DB_PASSWORD=  # REQUIRED: Generate strong password with: openssl rand -base64 32
```

Add validation in config.go:
```go
func (c *Config) Validate() error {
    weakPasswords := []string{
        "change_me_in_production",
        "password",
        "admin",
        "123456",
        "guardrail",
    }

    for _, weak := range weakPasswords {
        if c.DBPassword == weak {
            return fmt.Errorf("DB_PASSWORD is using a weak/placeholder value")
        }
    }
    // ...
}
```

**Remediation Priority:** MEDIUM

---

### 9. JWT Rotation Period Too Long
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/.env.example:112`
**Risk:** MEDIUM

```bash
JWT_ROTATION_HOURS=168  # 7 days
```

**Security Risk:**
- Long rotation period increases exposure window
- Compromised tokens valid for extended period
- No mechanism for emergency rotation

**Recommendation:**
```bash
# Reduce default to 24 hours
JWT_ROTATION_HOURS=24

# Add maximum validation
func ValidateTimeout(name string, value, min, max time.Duration) error {
    // Add maximum JWT rotation check
    if name == "JWT_ROTATION_HOURS" && value > 168*time.Hour {
        return fmt.Errorf("JWT_ROTATION_HOURS should not exceed 168 hours (7 days)")
    }
    // ...
}
```

**Remediation Priority:** MEDIUM

---

## Low Issues

### 10. Secrets Passed via Environment Variables in Compose
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/deploy/podman-compose.yml:117-179`
**Risk:** LOW

**Security Risk:**
- Secrets visible in `docker inspect` output
- Environment variables may be logged by orchestration tools
- Process listing (`ps e`) exposes environment

**Recommendation:**
Use Docker secrets or external secret management:

```yaml
services:
  mcp-server:
    secrets:
      - db_password
      - redis_password
      - jwt_secret
      - mcp_api_key
      - ide_api_key
    environment:
      - DB_PASSWORD_FILE=/run/secrets/db_password
      # ...

secrets:
  db_password:
    file: ./secrets/db_password.txt
  redis_password:
    file: ./secrets/redis_password.txt
  # Or use external secrets manager
```

Update config.go to support `_FILE` suffix:
```go
func loadSecretFromFile(envVar string) string {
    fileVar := envVar + "_FILE"
    if path := os.Getenv(fileVar); path != "" {
        data, err := os.ReadFile(path)
        if err != nil {
            slog.Warn("Failed to read secret file", "file", path, "error", err)
            return ""
        }
        return strings.TrimSpace(string(data))
    }
    return os.Getenv(envVar)
}
```

**Remediation Priority:** LOW

---

### 11. No Cipher Suite Configuration for TLS
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/config/config.go:70-76`
**Risk:** LOW

**Security Risk:**
- Default cipher suites may include weak ciphers
- No protection against SWEET32 or other cipher-based attacks
- Missing forward secrecy enforcement

**Recommendation:**
Add cipher suite configuration:

```go
// Config struct addition
TLSCipherSuites []string `env:"TLS_CIPHER_SUITES" envDefault:"TLS_AES_256_GCM_SHA384,TLS_CHACHA20_POLY1305_SHA256,TLS_AES_128_GCM_SHA256"`

// Validation
func (c *Config) Validate() error {
    if c.TLSEnabled {
        weakCiphers := []string{
            "TLS_RSA_WITH_RC4_128_SHA",
            "TLS_RSA_WITH_3DES_EDE_CBC_SHA",
            "TLS_RSA_WITH_AES_128_CBC_SHA",
            "TLS_RSA_WITH_AES_256_CBC_SHA",
        }
        // Validate no weak ciphers selected
    }
    // ...
}
```

**Remediation Priority:** LOW

---

## Positive Security Findings

The following security controls are properly implemented:

### 1. Secret Masking in Config
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/internal/config/config.go:391-400`

```go
func (c *Config) Masked() *Config {
    masked := *c
    masked.DBPassword = "***"
    masked.RedisPassword = "***"
    masked.MCPAPIKey = "***"
    masked.IDEAPIKey = "***"
    masked.JWTSecret = "***"
    return &masked
}
```

**Status:** GOOD - Proper secret masking for logging

---

### 2. Kubernetes Secrets Usage
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/deploy/k8s-deployment.yaml:64-122`

```yaml
- name: DB_PASSWORD
  valueFrom:
    secretKeyRef:
      name: guardrail-db-credentials
      key: password
```

**Status:** GOOD - Proper Kubernetes secrets integration

---

### 3. Security Context in Kubernetes
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/deploy/k8s-deployment.yaml:29-33,146-151`

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 65532
  runAsGroup: 65532
  fsGroup: 65532
  allowPrivilegeEscalation: false
  readOnlyRootFilesystem: true
  capabilities:
    drop:
      - ALL
```

**Status:** GOOD - Proper security hardening

---

### 4. Network Policies
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/deploy/k8s-deployment.yaml:218-260`

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
```

**Status:** GOOD - Network segmentation properly configured

---

### 5. Non-root Container User
**File:** `/mnt/ollama/git/agent-guardrails-template/mcp-server/deploy/Dockerfile:31-38`

```dockerfile
FROM gcr.io/distroless/static:nonroot
USER 65532:65532
```

**Status:** GOOD - Principle of least privilege followed

---

## Compliance Mapping

| Issue | PCI-DSS | SOC2 | ISO 27001 | NIST 800-53 |
|-------|---------|------|-----------|-------------|
| Database SSL disabled | 4.1, 4.2 | CC6.7 | A.13.2.1 | SC-8, SC-13 |
| Redis TLS disabled | 4.1, 4.2 | CC6.7 | A.13.2.1 | SC-8, SC-13 |
| Permissive CORS | 6.5.9 | CC6.6 | A.14.1.2 | AC-2, AC-3 |
| Placeholder secrets | 8.2.1 | CC6.1 | A.9.2.1 | IA-5 |
| Password exposure | 8.2.1 | CC6.1 | A.9.4.3 | SC-28 |

---

## Remediation Checklist

- [ ] Change `DB_SSLMODE` default from `disable` to `require` in podman-compose.yml
- [ ] Change `REDIS_USE_TLS` default from `false` to `true` in podman-compose.yml
- [ ] Remove `CORS_ALLOWED_ORIGINS=*` default, make it required
- [ ] Fix Redis health check to not expose password in command
- [ ] Add placeholder detection to config validation
- [ ] Make `PRODUCTION_MODE` required with no default
- [ ] Enforce TLS 1.3 minimum in production mode
- [ ] Add `_FILE` suffix support for Docker secrets
- [ ] Document secret generation procedures
- [ ] Add pre-deployment security validation script

---

## Appendix A: Secure Configuration Template

```bash
# Production-ready .env file template
PRODUCTION_MODE=true

# Server
MCP_PORT=8080
WEB_PORT=8081
LOG_LEVEL=warn

# Database (SSL required)
DB_HOST=postgres.example.com
DB_PORT=5432
DB_NAME=guardrails
DB_USER=guardrail
DB_PASSWORD=<GENERATE_STRONG_PASSWORD>
DB_SSLMODE=verify-full

# Redis (TLS required)
REDIS_HOST=redis.example.com
REDIS_PORT=6379
REDIS_PASSWORD=<GENERATE_STRONG_PASSWORD>
REDIS_USE_TLS=true

# TLS (Required for production)
TLS_ENABLED=true
TLS_CERT_PATH=/etc/ssl/certs/server.crt
TLS_KEY_PATH=/etc/ssl/private/server.key
TLS_CA_PATH=/etc/ssl/certs/ca.crt
TLS_MIN_VERSION=1.3

# Secrets (Generate with: openssl rand -hex 32)
MCP_API_KEY=<64_CHAR_HEX_STRING>
IDE_API_KEY=<64_CHAR_HEX_STRING>
JWT_SECRET=<128_CHAR_HEX_STRING>

# CORS (Explicit origins only)
CORS_ALLOWED_ORIGINS=https://app.example.com,https://admin.example.com

# Security
PPROF_ENABLED=false
RATE_LIMIT_MCP=1000
RATE_LIMIT_IDE=500
```

---

*End of Security Audit Report*
