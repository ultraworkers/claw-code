# MCP Server Observability Review

> Comprehensive review of observability and monitoring capabilities.

**Date:** 2026-02-08
**Scope:** MCP Server at `/mcp-server`
**Status:** COMPLETE - All improvements implemented

---

## Executive Summary

| Aspect | Status | Notes |
|--------|--------|-------|
| Logging | GOOD | Structured JSON logging with slog |
| Health Checks | GOOD | Live/ready probes implemented |
| Metrics | COMPLETE | Prometheus metrics for all subsystems |
| Request Tracing | COMPLETE | Correlation ID propagation implemented |
| Performance Metrics | COMPLETE | Latency/error tracking for all operations |
| Alert Conditions | DEFINED | SLO metrics and alert rules documented |

---

## 1. Logging Levels Review

### Current State
- Using `log/slog` with JSON handler
- Log level configurable via `LOG_LEVEL` env var (debug/info/warn/error)
- Default level: `info`

### Assessment: GOOD
- Debug logs for detailed troubleshooting
- Info logs for server startup/shutdown and normal operations
- Warn logs for rate limiting and cache issues
- Error logs for failures and panics

### Implementation
```go
slog.Info("Starting Guardrail MCP Server",
    "version", version,
    "build_time", buildTime,
    "git_commit", gitCommit,
)
```

---

## 2. Structured Logging Review

### Current State
- JSON structured logging enabled
- Audit logger with event types and severity levels
- Request logging middleware with full context

### Request Log Fields
- `method` - HTTP method
- `path` - Request path
- `uri` - Full request URI
- `status` - HTTP response status
- `duration` - Request duration
- `client_ip` - Client IP address
- `request_id` - Unique request ID
- `correlation_id` - Correlation ID for distributed tracing
- `bytes_in/out` - Request/response sizes
- `user_agent` - Client user agent

### Assessment: GOOD

---

## 3. Metrics Review

### Current State
- Prometheus `/metrics` endpoint exposed on web server
- Comprehensive metrics for all subsystems

### HTTP Metrics
```prometheus
guardrail_http_requests_total{method="GET",path="/api/rules",status="200"}
guardrail_http_request_duration_seconds{method="GET",path="/api/rules",status="200"}
guardrail_http_request_size_bytes{method="POST",path="/api/rules"}
guardrail_http_response_size_bytes{method="GET",path="/api/rules",status="200"}
```

### MCP Tool Metrics
```prometheus
guardrail_mcp_validations_total{tool="guardrail_validate_bash",result="allowed"}
guardrail_mcp_validation_duration_seconds{tool="guardrail_validate_bash"}
guardrail_mcp_sessions_active
guardrail_mcp_sessions_created_total
guardrail_mcp_sessions_expired_total
```

### Cache Metrics
```prometheus
guardrail_cache_hits_total{operation="get"}
guardrail_cache_misses_total{operation="get"}
guardrail_cache_errors_total{operation="set"}
guardrail_cache_operation_duration_seconds{operation="get"}
```

### Database Metrics
```prometheus
guardrail_database_connections_active{state="open"}
guardrail_database_connections_active{state="in_use"}
guardrail_database_connections_active{state="idle"}
guardrail_database_connections_wait_duration_seconds_total
guardrail_database_connections_wait_count_total
guardrail_database_query_duration_seconds{operation="select",table="rules"}
```

### Circuit Breaker Metrics
```prometheus
guardrail_circuitbreaker_state{name="database"}  # 0=closed, 1=open, 2=half-open
guardrail_circuitbreaker_failures_total{name="database"}
guardrail_circuitbreaker_successes_total{name="database"}
```

### Health Metrics
```prometheus
guardrail_health_check_duration_seconds{check="database"}
guardrail_health_check_failures_total{check="cache"}
```

### Rate Limiting Metrics
```prometheus
guardrail_ratelimit_hits_total{key_type="mcp",path="/api/rules"}
guardrail_ratelimit_allowed_total{key_type="ide"}
```

### Audit Metrics
```prometheus
guardrail_audit_events_total{type="validation",severity="info"}
guardrail_audit_events_dropped_total
```

### Runtime Metrics
```prometheus
guardrail_runtime_panics_total{path="/api/rules"}
```

### SLO/Error Budget Metrics
```prometheus
guardrail_slo_compliance{slo_name="availability"}  # 1=compliant, 0=breached
guardrail_slo_error_budget_burn_rate{slo_name="availability",window="1h"}
guardrail_slo_sli_value{slo_name="availability"}
```

### Assessment: COMPLETE

---

## 4. Health Checks Review

### Current State
- `/health/live` - Liveness probe (basic alive check)
- `/health/ready` - Readiness probe (checks DB, cache)
- CLI health check via `--health-check` flag with configurable timeout

### Liveness Response
```json
{
  "status": "alive",
  "version": "1.0.0",
  "timestamp": "2026-02-08T12:00:00Z"
}
```

### Readiness Response
```json
{
  "status": "ready",
  "version": "1.0.0",
  "timestamp": "2026-02-08T12:00:00Z"
}
```

### Assessment: GOOD

---

## 5. Request Tracing Review

### Current State
- Request ID generation via Echo middleware
- Correlation ID propagation from upstream services
- Correlation ID set in response headers for downstream tracing
- Request context stored in Echo context

### Headers
- `X-Request-ID` - Generated by server if not provided
- `X-Correlation-ID` - Propagated from upstream or generated

### Usage in Handlers
```go
correlationID := c.Get("correlation_id").(string)
slog.Info("Processing request",
    "correlation_id", correlationID,
    "request_id", c.Response().Header().Get(echo.HeaderXRequestID),
)
```

### Assessment: COMPLETE

---

## 6. Performance Metrics Review

### Current State
- Request latency histograms with configurable buckets
- Database query duration tracking
- Cache operation latency tracking
- Health check duration tracking
- Request/response size tracking

### Latency Buckets
- HTTP: 1ms to 10s `[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]`
- MCP validation: 1ms to 1s `[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1]`
- Database: 1ms to 5s `[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5]`

### Assessment: COMPLETE

---

## 7. Alert Conditions

### Critical Alerts

| Alert | Condition | Action |
|-------|-----------|--------|
| High Error Rate | `rate(guardrail_http_requests_total{status=~"5.."}[5m]) / rate(guardrail_http_requests_total[5m]) > 0.05` | Page on-call |
| High Latency | `histogram_quantile(0.99, rate(guardrail_http_request_duration_seconds_bucket[5m])) > 2` | Page on-call |
| Health Check Failing | `guardrail_health_check_failures_total > 0` for 2m | Page on-call |
| Circuit Breaker Open | `guardrail_circuitbreaker_state == 1` | Page on-call |
| Panic Detected | `rate(guardrail_runtime_panics_total[5m]) > 0` | Page on-call |

### Warning Alerts

| Alert | Condition | Action |
|-------|-----------|--------|
| Elevated Error Rate | `rate(guardrail_http_requests_total{status=~"5.."}[5m]) / rate(guardrail_http_requests_total[5m]) > 0.01` | Notify team |
| Elevated Latency | `histogram_quantile(0.95, rate(guardrail_http_request_duration_seconds_bucket[5m])) > 1` | Notify team |
| Rate Limiting Triggered | `rate(guardrail_ratelimit_hits_total[1m]) > 100` | Notify team |
| Database Pool Near Capacity | `guardrail_database_connections_active{state="open"} / guardrail_database_connections_active{state="max"} > 0.9` | Notify team |
| Cache Error Rate | `rate(guardrail_cache_errors_total[5m]) > 10` | Notify team |
| Audit Buffer Full | `guardrail_audit_events_dropped_total > 0` | Notify team |

### Assessment: DEFINED

---

## 8. Dashboard Metrics

### Overview Dashboard

**Request Metrics:**
- `sum(rate(guardrail_http_requests_total[5m]))` - Request rate
- `sum(rate(guardrail_http_requests_total{status=~"5.."}[5m])) / sum(rate(guardrail_http_requests_total[5m]))` - Error rate
- `histogram_quantile(0.50, sum(rate(guardrail_http_request_duration_seconds_bucket[5m])) by (le))` - P50 latency
- `histogram_quantile(0.95, sum(rate(guardrail_http_request_duration_seconds_bucket[5m])) by (le))` - P95 latency
- `histogram_quantile(0.99, sum(rate(guardrail_http_request_duration_seconds_bucket[5m])) by (le))` - P99 latency

**MCP Metrics:**
- `guardrail_mcp_sessions_active` - Active sessions
- `sum(rate(guardrail_mcp_validations_total[5m]))` - Validation rate
- `sum(rate(guardrail_mcp_validations_total{result="denied"}[5m]))` - Denial rate

### Health Dashboard

**Component Health:**
- `guardrail_health_check_failures_total` by check type
- `guardrail_circuitbreaker_state` by name
- `guardrail_database_connections_active` by state
- `guardrail_cache_hits_total / (guardrail_cache_hits_total + guardrail_cache_misses_total)` - Cache hit rate

**Resource Usage:**
- `guardrail_database_connections_active{state="in_use"}` - Active DB connections
- `guardrail_database_connections_wait_count_total` - DB wait events

### Business Dashboard

**Guardrail Activity:**
- `sum(rate(guardrail_mcp_validations_total[5m])) by (tool)` - Validations by tool
- `sum(rate(guardrail_audit_events_total[5m])) by (severity)` - Audit events by severity
- `sum(rate(guardrail_mcp_sessions_created_total[5m]))` - Session creation rate

### SLO Dashboard

**Compliance:**
- `guardrail_slo_compliance` by SLO name
- `guardrail_slo_error_budget_burn_rate` by window
- `guardrail_slo_sli_value` by SLO name

---

## 9. Files Modified

### New Files
- `mcp-server/internal/database/metrics.go` - Database metrics collector

### Modified Files
- `mcp-server/internal/metrics/metrics.go` - Added panic, database, SLO metrics
- `mcp-server/internal/web/server.go` - Added correlation ID and panic recovery middleware
- `mcp-server/internal/cache/redis.go` - Added cache operation metrics
- `mcp-server/cmd/server/main.go` - Initialize database metrics collector

---

## 10. Implementation Summary

### Middleware Chain (in order)
1. `RequestID` - Generate unique request ID
2. `correlationIDMiddleware` - Propagate correlation ID
3. `panicRecoveryMiddleware` - Recover panics and record metrics
4. `PrometheusMiddleware` - Record HTTP metrics
5. `RequestLogger` - Log requests with full context
6. `securityHeadersMiddleware` - Add security headers
7. `APIKeyAuth` - Authenticate requests
8. `RateLimitMiddleware` - Apply rate limiting
9. `Timeout` - Request timeout
10. `BodyLimit` - Request size limit

### Metrics Collection
- HTTP metrics: Automatic via PrometheusMiddleware
- Cache metrics: Recorded in Get/Set/Delete operations
- Database metrics: Collected periodically via MetricsCollector (15s interval)
- Panic metrics: Recorded in panic recovery middleware
- Custom metrics: Recorded via helper functions in handlers

---

**Authored by:** TheArchitectit
**Last Updated:** 2026-02-08
**Status:** COMPLETE
