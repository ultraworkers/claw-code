# MCP Server Changelog

All notable changes to the Guardrail MCP Server will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased]

### Fixed

- **SSE Timeout Resilience** - Fixed MCP client `Request timed out` (-32001) errors
  - Reduced keep-alive interval from 30s to 15s to prevent proxy/NAT drops
  - Added fresh session liveness check after request processing
  - Implemented graceful HTTP fallback when SSE session closes during request
  - Affects: `guardrail_init_session`, `guardrail_validate_bash`, all MCP tools

### Added

### Changed

---

## [2.6.0] - 2026-02-15

### Migrated

- **Python to Go Migration Complete** - All team management functionality migrated from Python to Go
  - `team_manager.py` → `internal/team/` package
  - Native Go implementation (no Python runtime required)
  - Backward compatible API - all MCP tools work identically

### Added

- **Go Team Package** (`internal/team/`)
  - `manager.go` - Team management (init, assign, unassign, status, delete)
  - `encryption.go` - Fernet encryption at rest
  - `validation.go` - Input validation
  - `rules.go` - Team layout rules and phase gates
  - `metrics.go` - Operation metrics
  - `types.go` - Data structures
  - `migrations.go` - Data migration utilities

### Changed

- **Container Image** - Now uses `gcr.io/distroless/static:nonroot`
  - Removed Python runtime dependency
  - Reduced container size by ~75% (20MB vs 80MB)
  - Faster startup (50ms vs 500ms)

### Deprecated

- **Python Script Dependency** - No longer requires `scripts/team_manager.py`
  - Python scripts deprecated as of v2.6.0
  - Will be removed in v3.0.0

### Compatibility

- **MCP Tool API** - Fully backward compatible
- **Data Format** - `.teams/*.json` files unchanged

---

## [1.9.6] - 2026-02-08

### Fixed

- **SSE Client Compatibility** - Restored compatibility with Go SDK and Crush MCP clients
  - Replaced custom `event: ping` payloads with SSE keepalive comments (`: ping`)
  - Added per-session response queues for SSE streams
  - Emits JSON-RPC responses as `event: message` payloads over SSE

- **Session Message Flow** - Improved session-bound message handling
  - Proper handling for notifications (`202 Accepted`)
  - Explicit closed-session response (`410 Gone`)
  - Backpressure response when queues are full (`503 Service Unavailable`)

### Changed

- **Container Packaging** - Web UI static assets are now bundled into the runtime image
  - Added `/app/static` copy step in `deploy/Dockerfile`

- **Web UI Access** - Read-only browsing routes are now publicly accessible
  - `/api/documents` and `/api/documents/*`
  - `/api/rules` and `/api/rules/*`
  - `/version`

### Documentation

- Updated README MCP connection/testing instructions for session_id-based message flow
- Updated troubleshooting guidance for SSE transport behavior

---

## [1.9.5] - 2026-02-08

### Added

- **Circuit Breaker Pattern** - Automatic failure detection for database and Redis
  - Configurable failure thresholds and recovery timeouts
  - Prevents cascade failures in distributed systems
  - Implements CLOSED, OPEN, and HALF-OPEN states

- **Hot-Reloadable Configuration** - Runtime config updates without restart
  - Support for LOG_LEVEL, RATE_LIMIT_*, CACHE_TTL_*, and feature flags
  - Signal-based reload (SIGHUP) or admin endpoint

### Changed

- **Rate Limiting Enhancements**
  - Added burst factor for handling traffic spikes (RATE_LIMIT_BURST_FACTOR)
  - Per-endpoint-type limits: MCP (1000/min), IDE (500/min), Session (100/min)
  - Redis-backed distributed rate limiting

---

## [1.9.4] - 2026-02-08

### Added

- **Secrets Scanning** - Automatic detection in document content
  - AWS Access Key ID detection
  - GitHub token detection
  - Private key detection (RSA, EC, DSA, OpenSSH)
  - Slack token detection
  - Blocks document updates containing potential secrets

- **CORS Configuration** - Flexible cross-origin resource sharing
  - Configurable allowed origins, methods, headers
  - Production-safe defaults (restrictive when TLS enabled)

### Fixed

- **PostgreSQL Array Handling** - Fixed TEXT[] array scanning
  - Migrated from `pq.StringArray` to `pgtype.Array[string]`
  - Compatible with pgx v5 driver
  - Proper handling of nullable arrays

---

## [1.9.3] - 2026-02-07

### Added

- **Structured Logging** - JSON-formatted logs with configurable levels
  - Support for debug, info, warn, error levels
  - Correlation ID propagation across requests
  - Request ID generation for tracing

- **Metrics Collection** - Prometheus-compatible metrics
  - Request count and duration by endpoint
  - Database connection pool metrics
  - Cache hit/miss ratios
  - Panic recovery tracking

### Fixed

- **SSE Stability Improvements** - Enhanced Server-Sent Events reliability
  - Immediate `WriteHeader(http.StatusOK)` for non-interactive clients
  - Added `X-Accel-Buffering: no` for proxy compatibility
  - Added CORS headers for cross-origin SSE connections
  - Initial ping event to prevent client timeouts
  - Better error handling on write/flush operations

---

## [1.9.2] - 2026-02-07

### Fixed

- **Web UI Authentication** - Removed API key requirement for Web UI routes
  - Web UI routes (`/`, `/index.html`, `/static/*`) are now publicly accessible
  - API endpoints still require valid API key
  - Health checks and metrics remain unauthenticated

---

## [1.9.1] - 2026-02-07

### Fixed

- **SSE Compatibility** - Fixed EOF errors with non-interactive clients
  - Added `WriteHeader(http.StatusOK)` for immediate header commit
  - Added `X-Accel-Buffering: no` for proxy compatibility
  - Added `Access-Control-Allow-Origin: *` for CORS
  - Send immediate ping event after endpoint to prevent client timeout
  - Better error handling on write/flush operations

- **PostgreSQL Array Scanning** - Fixed TEXT[] array scanning bug
  - Changed `AffectedFiles` from `pq.StringArray` to `pgtype.Array[string]`
  - Added `ToStringSlice()` and `ToTextArray()` helper functions
  - Compatible with pgx v5 driver

---

## [1.9.0] - 2026-02-07

### Added

- **MCP Protocol Implementation**
  - SSE transport for real-time client communication
  - JSON-RPC 2.0 message handling
  - Tools: `guardrail_init_session`, `guardrail_validate_bash`,
    `guardrail_validate_file_edit`, `guardrail_validate_git_operation`,
    `guardrail_pre_work_check`, `guardrail_get_context`
  - Resources: `guardrail://quick-reference`, `guardrail://rules/active`

- **Web UI API**
  - Document CRUD operations
  - Prevention rule management
  - Project configuration
  - Failure registry

- **IDE API**
  - File validation endpoint
  - Selection validation endpoint
  - Active rules endpoint
  - Quick reference endpoint

- **Infrastructure**
  - PostgreSQL 16 support with migrations
  - Redis 7 for caching and rate limiting
  - Circuit breaker pattern for resilience
  - Secrets scanning in document content
  - Audit logging infrastructure
  - Prometheus metrics

- **Security**
  - API key authentication (MCP and IDE keys)
  - JWT session tokens with configurable expiry
  - Rate limiting per API key
  - Content Security Policy headers
  - Non-root container execution
  - Read-only filesystem support

---

## Version Management

This MCP Server follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html):

- **MAJOR**: Incompatible API changes
- **MINOR**: Backwards-compatible functionality additions
- **PATCH**: Backwards-compatible bug fixes

---

*Last Updated: 2026-02-15*
*Current Version: 2.6.0*
