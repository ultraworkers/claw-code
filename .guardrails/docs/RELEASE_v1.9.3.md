# Release v1.9.3 - Security & Infrastructure Hardening

**Release Date:** 2026-02-07
**Branch:** `mcpserver-impl`
**Tag:** `v1.9.3`

---

## Summary

Major security and infrastructure hardening release following comprehensive review by a team of 6 specialists (security, code quality, documentation, MCP protocol, database, and deployment experts).

---

## Security Hardening

### CORS Origin Validation
- **Before:** `Access-Control-Allow-Origin: *` allowed any origin
- **After:** Configurable origin validation with whitelist

### Secure Session ID Generation
- **Before:** `time.Now().UnixNano()` - predictable
- **After:** `crypto/rand` - cryptographically secure

### Security Headers
Added to all responses:
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `X-XSS-Protection: 1; mode=block`
- `Referrer-Policy: strict-origin-when-cross-origin`

### Path Traversal Protection
- Added `isValidSlug()` function
- Validates project slugs against pattern: `^[a-z0-9-_]+$`

---

## Database Improvements

### SQL Injection Fixes
- Fixed dynamic query building in `documents.go`, `rules.go`, `failures.go`
- Replaced `fmt.Sprintf` with parameterized queries

### Transaction Support
- All Create/Update/Delete operations now use transactions
- Proper commit/rollback handling with defer

### Model Validation
Added `Validate()` methods:
- `Document.Validate()` - required fields, lengths, categories
- `PreventionRule.Validate()` - rule_id format, severity levels
- `Project.Validate()` - slug format validation
- `FailureEntry.Validate()` - severity/status validation

### Migration System
- Added schema versioning with `schema_migrations` table
- New migrations:
  - `003_add_triggers` - automatic updated_at timestamps
  - `004_fix_indexes` - performance optimizations
  - `005_schema_versioning` - migration tracking
  - `006_partition_management` - time-series partitioning

---

## MCP Protocol Compliance

### SSE Endpoint
- Now sends full absolute URLs per MCP 2024-11-05 spec
- Ping events use proper JSON-RPC notification format

### Message Handler
- Added `session_id` query parameter validation
- JSON-RPC version validation (must be "2.0")
- Session activity tracking

### Session Management
- Secure session ID generation
- Automatic cleanup of inactive sessions (1 hour timeout)

---

## Deployment & Operations

### Dockerfile
- Version metadata injection at build time
- CA certificates for TLS connections
- Removed broken HEALTHCHECK (use orchestrator checks)

### Kubernetes Support
- Complete K8s deployment manifests
- Horizontal Pod Autoscaler (HPA)
- Pod Disruption Budget (PDB)
- Network policies
- Pod security contexts

### Health Checks
- `/health/live` - Liveness (simple, fast)
- `/health/ready` - Readiness (DB + cache)
- `/version` - Version information
- `/metrics` - Prometheus metrics

### Graceful Shutdown
- Configurable timeout (5s-5m)
- SIGQUIT handler added
- Proper connection closing

---

## Documentation

- **API.md** - Complete REST API reference (500+ lines)
- **CHANGELOG.md** (mcp-server) - MCP server-specific changelog
- **README.md** - Enhanced with security features, troubleshooting
- **DATABASE_REVIEW.md** - Comprehensive database review

---

## Files Changed

| File | Change |
|------|--------|
| `internal/mcp/server.go` | CORS fix, secure session IDs, protocol compliance |
| `internal/web/middleware.go` | Security headers, request size limits |
| `internal/web/handlers.go` | Path traversal protection |
| `internal/cache/redis.go` | Non-blocking SCAN, context timeouts |
| `internal/database/*.go` | Transactions, validation, migrations |
| `internal/database/tx.go` | **NEW** - Transaction helper |
| `deploy/Dockerfile` | Version injection, CA certs |
| `deploy/podman-compose.yml` | Health checks, resource limits |
| `deploy/k8s-deployment.yaml` | **NEW** - Kubernetes manifests |
| `cmd/server/main.go` | CLI flags, health check mode, pprof |
| `internal/config/config.go` | New config options |
| `internal/web/server.go` | Version endpoint |
| `API.md` | **NEW** - API documentation |
| `CHANGELOG.md` (mcp-server) | **NEW** - Server changelog |

---

## Migration from v1.9.2

1. Build new image: `make build`
2. Run migrations: `make migrate-up`
3. Deploy: `podman-compose up -d` or `kubectl apply -f deploy/k8s-deployment.yaml`

---

## Credits

Comprehensive review and improvements by:
- Security Engineer - Security hardening
- Go Pro - Code quality improvements
- Technical Writer - Documentation
- MCP Developer - Protocol compliance
- PostgreSQL Pro - Database improvements
- DevOps Engineer - Deployment enhancements

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
