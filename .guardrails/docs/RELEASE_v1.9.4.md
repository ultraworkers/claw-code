# Release v1.9.4 - Performance & Reliability Improvements

**Release Date:** 2026-02-07
**Branch:** `mcpserver-impl`
**Tag:** `v1.9.4`

---

## Summary

Second comprehensive review cycle focusing on performance optimization, error handling reliability, configuration fixes, and observability enhancements.

---

## Performance Improvements

### SSE Handler Optimizations
- **strings.Builder** - Replaced `fmt.Sprintf` with pre-allocated `strings.Builder`
- **Pre-allocated buffers** - SSE event prefixes and data as byte slices
- **Reduced allocations** - Type-safe context keys, JSON buffer pool
- **Optimized session cleanup** - Batched deletion, reduced lock contention

### Database Optimizations
- Query improvements in documents, rules, and project stores
- Better transaction handling
- Reduced memory allocations in hot paths

---

## Reliability Improvements

### Error Handling Fixes
- **Fixed silent failures** - `GetByID`, `Count` errors now properly returned
- **Error wrapping** - All errors use `%w` for context preservation
- **HTTP status codes** - Proper 404 for not found, 500 for server errors
- **Type assertions** - Safe type assertions with proper error handling

### Panic Recovery
- Added panic recovery middleware
- Panic metrics tracked by path
- Structured logging with stack traces
- Graceful error response instead of crash

---

## Configuration Fixes

### Environment Variable Naming
Fixed incorrect naming in deployment configs:
- `MCP_RATE_LIMIT` → `RATE_LIMIT_MCP`
- `IDE_RATE_LIMIT` → `RATE_LIMIT_IDE`
- `SESSION_RATE_LIMIT` → `RATE_LIMIT_SESSION`

### Feature Flags Added
- `ENABLE_METRICS` - Toggle Prometheus metrics
- `ENABLE_AUDIT_LOGGING` - Toggle audit logging
- `ENABLE_CACHE` - Toggle Redis caching

### CORS Configuration
- `CORS_ALLOWED_ORIGINS` - Configure allowed origins
- `CORS_MAX_AGE` - Configure preflight cache duration

---

## Observability Enhancements

### New Metrics
- **PanicsTotal** - Track recovered panics by path
- **DBConnectionsActive** - Database connection pool stats
- **DBQueryDuration** - Query latency histograms
- **SLOCompliance** - SLO compliance tracking
- **ErrorBudgetBurnRate** - Error budget consumption
- **SLIValue** - Service Level Indicator values

### Request Tracing
- Correlation ID middleware for request tracing
- Request ID propagation
- Structured logging with correlation IDs

---

## API Consistency

### Route Ordering
Fixed route registration order:
```go
// Before - search would never match
api.GET("/documents/:id", s.getDocument)
api.GET("/documents/search", s.searchDocuments)

// After - specific routes first
api.GET("/documents/search", s.searchDocuments)
api.GET("/documents/:id", s.getDocument)
```

### Response Format Standardization
- Consistent error response format across all endpoints
- Proper pagination metadata
- Standardized field naming

---

## Files Changed

| File | Change |
|------|--------|
| `internal/mcp/server.go` | SSE optimizations, reduced allocations |
| `internal/web/handlers.go` | Error handling fixes, silent failures fixed |
| `internal/web/server.go` | Panic recovery, correlation ID middleware |
| `internal/database/*.go` | Query optimizations |
| `internal/config/config.go` | Config validation improvements |
| `internal/metrics/metrics.go` | New panic, DB, SLO metrics |
| `deploy/k8s-deployment.yaml` | Fixed env var names |
| `deploy/podman-compose.yml` | Fixed env var names, feature flags |

---

## Migration from v1.9.3

1. Update environment variables:
   ```bash
   # Old (incorrect)
   MCP_RATE_LIMIT=1000
   IDE_RATE_LIMIT=500

   # New (correct)
   RATE_LIMIT_MCP=1000
   RATE_LIMIT_IDE=500
   ```

2. Deploy new version:
   ```bash
   make build
   podman-compose up -d
   ```

---

## Credits

Comprehensive review and improvements by:
- Performance Engineer - SSE and database optimizations
- SRE Engineer - Error handling and reliability
- API Designer - Route consistency and standardization
- Platform Engineer - Configuration management
- Observability Engineer - Metrics and tracing

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
