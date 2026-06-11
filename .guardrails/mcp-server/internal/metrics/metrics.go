package metrics

import (
	"strconv"
	"time"

	"github.com/labstack/echo/v4"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

// TeamToolMetrics provides instrumentation for team tool handlers
type TeamToolMetrics struct {
	tool string
	start time.Time
}

// NewTeamToolMetrics creates a new team tool metrics recorder
func NewTeamToolMetrics(tool string) *TeamToolMetrics {
	IncrementTeamToolActive(tool)
	return &TeamToolMetrics{
		tool:  tool,
		start: time.Now(),
	}
}

// Done records the completion of a team tool operation
func (m *TeamToolMetrics) Done(success bool) {
	DecrementTeamToolActive(m.tool)
	RecordTeamToolDuration(m.tool, time.Since(m.start))
	RecordTeamToolCall(m.tool, success)
}

// RecordError records an error for the team tool operation
func (m *TeamToolMetrics) RecordError(errorType string) {
	RecordTeamToolError(m.tool, errorType)
}

// Namespace for all guardrail metrics
const namespace = "guardrail"

// HTTP metrics
var (
	// HTTPRequestsTotal tracks total HTTP requests
	HTTPRequestsTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "http",
			Name:      "requests_total",
			Help:      "Total number of HTTP requests",
		},
		[]string{"method", "path", "status"},
	)

	// HTTPRequestDuration tracks HTTP request latency
	HTTPRequestDuration = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Namespace: namespace,
			Subsystem: "http",
			Name:      "request_duration_seconds",
			Help:      "HTTP request latency in seconds",
			Buckets:   []float64{0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10},
		},
		[]string{"method", "path", "status"},
	)

	// HTTPRequestSize tracks HTTP request size
	HTTPRequestSize = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Namespace: namespace,
			Subsystem: "http",
			Name:      "request_size_bytes",
			Help:      "HTTP request size in bytes",
			Buckets:   prometheus.ExponentialBuckets(100, 10, 8),
		},
		[]string{"method", "path"},
	)

	// HTTPResponseSize tracks HTTP response size
	HTTPResponseSize = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Namespace: namespace,
			Subsystem: "http",
			Name:      "response_size_bytes",
			Help:      "HTTP response size in bytes",
			Buckets:   prometheus.ExponentialBuckets(100, 10, 8),
		},
		[]string{"method", "path", "status"},
	)
)

// MCP tool metrics
var (
	// MCPValidationsTotal tracks MCP tool validations
	MCPValidationsTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "mcp",
			Name:      "validations_total",
			Help:      "Total number of MCP validation requests",
		},
		[]string{"tool", "result"},
	)

	// MCPValidationDuration tracks validation latency
	MCPValidationDuration = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Namespace: namespace,
			Subsystem: "mcp",
			Name:      "validation_duration_seconds",
			Help:      "MCP validation latency in seconds",
			Buckets:   []float64{0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1},
		},
		[]string{"tool"},
	)

	// MCPSessionsActive tracks active sessions
	MCPSessionsActive = promauto.NewGauge(
		prometheus.GaugeOpts{
			Namespace: namespace,
			Subsystem: "mcp",
			Name:      "sessions_active",
			Help:      "Number of active MCP sessions",
		},
	)

	// MCPSessionsCreatedTotal tracks total sessions created
	MCPSessionsCreatedTotal = promauto.NewCounter(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "mcp",
			Name:      "sessions_created_total",
			Help:      "Total number of MCP sessions created",
		},
	)

	// MCPSessionsExpiredTotal tracks expired sessions
	MCPSessionsExpiredTotal = promauto.NewCounter(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "mcp",
			Name:      "sessions_expired_total",
			Help:      "Total number of MCP sessions expired",
		},
	)
)

// Audit metrics
var (
	// AuditEventsTotal tracks audit events
	AuditEventsTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "audit",
			Name:      "events_total",
			Help:      "Total number of audit events",
		},
		[]string{"type", "severity"},
	)

	// AuditEventsDropped tracks dropped audit events
	AuditEventsDropped = promauto.NewCounter(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "audit",
			Name:      "events_dropped_total",
			Help:      "Total number of audit events dropped due to full buffer",
		},
	)
)

// Circuit breaker metrics
var (
	// CircuitBreakerState tracks circuit breaker state
	CircuitBreakerState = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Namespace: namespace,
			Subsystem: "circuitbreaker",
			Name:      "state",
			Help:      "Circuit breaker state (0=closed, 1=open, 2=half-open)",
		},
		[]string{"name"},
	)

	// CircuitBreakerFailures tracks circuit breaker failures
	CircuitBreakerFailures = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "circuitbreaker",
			Name:      "failures_total",
			Help:      "Total number of circuit breaker failures",
		},
		[]string{"name"},
	)

	// CircuitBreakerSuccesses tracks circuit breaker successes
	CircuitBreakerSuccesses = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "circuitbreaker",
			Name:      "successes_total",
			Help:      "Total number of circuit breaker successes",
		},
		[]string{"name"},
	)
)

// Health metrics
var (
	// HealthCheckDuration tracks health check latency
	HealthCheckDuration = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Namespace: namespace,
			Subsystem: "health",
			Name:      "check_duration_seconds",
			Help:      "Health check latency in seconds",
			Buckets:   []float64{0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1},
		},
		[]string{"check"},
	)

	// HealthCheckFailures tracks health check failures
	HealthCheckFailures = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "health",
			Name:      "check_failures_total",
			Help:      "Total number of health check failures",
		},
		[]string{"check"},
	)
)

// Cache metrics
var (
	// CacheHits tracks cache hits
	CacheHits = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "cache",
			Name:      "hits_total",
			Help:      "Total number of cache hits",
		},
		[]string{"operation"},
	)

	// CacheMisses tracks cache misses
	CacheMisses = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "cache",
			Name:      "misses_total",
			Help:      "Total number of cache misses",
		},
		[]string{"operation"},
	)

	// CacheErrors tracks cache errors
	CacheErrors = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "cache",
			Name:      "errors_total",
			Help:      "Total number of cache errors",
		},
		[]string{"operation"},
	)

	// CacheOperationDuration tracks cache operation latency
	CacheOperationDuration = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Namespace: namespace,
			Subsystem: "cache",
			Name:      "operation_duration_seconds",
			Help:      "Cache operation latency in seconds",
			Buckets:   []float64{0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1},
		},
		[]string{"operation"},
	)
)

// Rate limit metrics
var (
	// RateLimitHits tracks rate limit enforcement
	RateLimitHits = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "ratelimit",
			Name:      "hits_total",
			Help:      "Total number of rate limit enforcements",
		},
		[]string{"key_type", "path"},
	)

	// RateLimitAllowed tracks allowed requests
	RateLimitAllowed = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "ratelimit",
			Name:      "allowed_total",
			Help:      "Total number of allowed requests",
		},
		[]string{"key_type"},
	)
)

// Panic recovery metrics
var (
	// PanicsTotal tracks recovered panics
	PanicsTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "runtime",
			Name:      "panics_total",
			Help:      "Total number of recovered panics",
		},
		[]string{"path"},
	)
)

// Database metrics
var (
	// DBConnectionsActive tracks active database connections
	DBConnectionsActive = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Namespace: namespace,
			Subsystem: "database",
			Name:      "connections_active",
			Help:      "Current number of active database connections",
		},
		[]string{"state"}, // state: open, in_use, idle
	)

	// DBConnectionsWaitDuration tracks time waiting for connection
	DBConnectionsWaitDuration = promauto.NewCounter(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "database",
			Name:      "connections_wait_duration_seconds_total",
			Help:      "Total time waited for database connections",
		},
	)

	// DBConnectionsWaitCount tracks number of waits for connection
	DBConnectionsWaitCount = promauto.NewCounter(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "database",
			Name:      "connections_wait_count_total",
			Help:      "Total number of waits for database connections",
		},
	)

	// DBQueryDuration tracks database query latency
	DBQueryDuration = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Namespace: namespace,
			Subsystem: "database",
			Name:      "query_duration_seconds",
			Help:      "Database query latency in seconds",
			Buckets:   []float64{0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5},
		},
		[]string{"operation", "table"},
	)
)

// SLO/Error budget metrics
var (
	// SLOCompliance tracks SLO compliance (1 = compliant, 0 = breached)
	SLOCompliance = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Namespace: namespace,
			Subsystem: "slo",
			Name:      "compliance",
			Help:      "SLO compliance status (1 = compliant, 0 = breached)",
		},
		[]string{"slo_name"},
	)

	// ErrorBudgetBurnRate tracks error budget burn rate
	ErrorBudgetBurnRate = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Namespace: namespace,
			Subsystem: "slo",
			Name:      "error_budget_burn_rate",
			Help:      "Error budget burn rate (1.0 = on track, >1 = burning too fast)",
		},
		[]string{"slo_name", "window"},
	)

	// SLIValue tracks SLI values
	SLIValue = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Namespace: namespace,
			Subsystem: "slo",
			Name:      "sli_value",
			Help:      "SLI value (0-1 scale)",
		},
		[]string{"slo_name"},
	)
)

// Team tool metrics
var (
	// TeamToolCallsTotal tracks total team tool calls
	TeamToolCallsTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "team_tool",
			Name:      "calls_total",
			Help:      "Total number of team tool calls",
		},
		[]string{"tool", "result"},
	)

	// TeamToolDuration tracks team tool execution latency
	TeamToolDuration = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Namespace: namespace,
			Subsystem: "team_tool",
			Name:      "duration_seconds",
			Help:      "Team tool execution latency in seconds",
			Buckets:   []float64{0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10},
		},
		[]string{"tool"},
	)

	// TeamToolErrorsTotal tracks team tool errors
	TeamToolErrorsTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "team_tool",
			Name:      "errors_total",
			Help:      "Total number of team tool errors",
		},
		[]string{"tool", "error_type"},
	)

	// TeamToolActiveOperations tracks active team tool operations
	TeamToolActiveOperations = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Namespace: namespace,
			Subsystem: "team_tool",
			Name:      "active_operations",
			Help:      "Number of active team tool operations",
		},
		[]string{"tool"},
	)

	// TeamToolPythonExecDuration tracks Python script execution time
	TeamToolPythonExecDuration = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Namespace: namespace,
			Subsystem: "team_tool",
			Name:      "python_exec_duration_seconds",
			Help:      "Python script execution latency in seconds",
			Buckets:   []float64{0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10},
		},
		[]string{"command"},
	)
)

// Performance operation metrics (OPS-008)
var (
	// PerformanceOperationDuration tracks operation latency
	PerformanceOperationDuration = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Namespace: namespace,
			Subsystem: "performance",
			Name:      "operation_duration_seconds",
			Help:      "Performance operation latency in seconds",
			Buckets:   []float64{0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10},
		},
		[]string{"operation"},
	)

	// PerformanceOperationTotal tracks total operations
	PerformanceOperationTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "performance",
			Name:      "operations_total",
			Help:      "Total number of operations",
		},
		[]string{"operation", "result"},
	)

	// PerformanceOperationErrors tracks operation errors
	PerformanceOperationErrors = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Namespace: namespace,
			Subsystem: "performance",
			Name:      "operation_errors_total",
			Help:      "Total number of operation errors",
		},
		[]string{"operation", "error_type"},
	)
)

// PrometheusMiddleware returns Echo middleware for Prometheus metrics
func PrometheusMiddleware() echo.MiddlewareFunc {
	return func(next echo.HandlerFunc) echo.HandlerFunc {
		return func(c echo.Context) error {
			start := time.Now()

			// Capture request info
			req := c.Request()
			res := c.Response()

			// Get content length if available
			requestSize := req.ContentLength
			if requestSize < 0 {
				requestSize = 0
			}

			// Execute handler
			err := next(c)

			// Capture response info after handler
			duration := time.Since(start).Seconds()
			status := strconv.Itoa(res.Status)
			path := c.Path()
			method := req.Method

			// Record metrics
			HTTPRequestsTotal.WithLabelValues(method, path, status).Inc()
			HTTPRequestDuration.WithLabelValues(method, path, status).Observe(duration)
			HTTPRequestSize.WithLabelValues(method, path).Observe(float64(requestSize))
			HTTPResponseSize.WithLabelValues(method, path, status).Observe(float64(res.Size))

			return err
		}
	}
}

// RecordValidation records MCP validation metrics
func RecordValidation(tool string, result string, duration time.Duration) {
	MCPValidationsTotal.WithLabelValues(tool, result).Inc()
	MCPValidationDuration.WithLabelValues(tool).Observe(duration.Seconds())
}

// RecordAuditEvent records audit event metrics
func RecordAuditEvent(eventType string, severity string) {
	AuditEventsTotal.WithLabelValues(eventType, severity).Inc()
}

// RecordAuditDrop records dropped audit event
func RecordAuditDrop() {
	AuditEventsDropped.Inc()
}

// RecordCircuitBreakerState updates circuit breaker state gauge
func RecordCircuitBreakerState(name string, state string) {
	var stateValue float64
	switch state {
	case "closed":
		stateValue = 0
	case "open":
		stateValue = 1
	case "half-open":
		stateValue = 2
	}
	CircuitBreakerState.WithLabelValues(name).Set(stateValue)
}

// RecordCircuitBreakerFailure records a circuit breaker failure
func RecordCircuitBreakerFailure(name string) {
	CircuitBreakerFailures.WithLabelValues(name).Inc()
}

// RecordCircuitBreakerSuccess records a circuit breaker success
func RecordCircuitBreakerSuccess(name string) {
	CircuitBreakerSuccesses.WithLabelValues(name).Inc()
}

// RecordHealthCheck records health check metrics
func RecordHealthCheck(check string, duration time.Duration, failed bool) {
	HealthCheckDuration.WithLabelValues(check).Observe(duration.Seconds())
	if failed {
		HealthCheckFailures.WithLabelValues(check).Inc()
	}
}

// RecordCacheHit records a cache hit
func RecordCacheHit(operation string) {
	CacheHits.WithLabelValues(operation).Inc()
}

// RecordCacheMiss records a cache miss
func RecordCacheMiss(operation string) {
	CacheMisses.WithLabelValues(operation).Inc()
}

// RecordCacheError records a cache error
func RecordCacheError(operation string) {
	CacheErrors.WithLabelValues(operation).Inc()
}

// RecordRateLimitHit records a rate limit enforcement
func RecordRateLimitHit(keyType string, path string) {
	RateLimitHits.WithLabelValues(keyType, path).Inc()
}

// RecordRateLimitAllowed records an allowed request
func RecordRateLimitAllowed(keyType string) {
	RateLimitAllowed.WithLabelValues(keyType).Inc()
}

// IncrementActiveSessions increments active session count
func IncrementActiveSessions() {
	MCPSessionsActive.Inc()
	MCPSessionsCreatedTotal.Inc()
}

// DecrementActiveSessions decrements active session count
func DecrementActiveSessions() {
	MCPSessionsActive.Dec()
}

// RecordSessionExpired records a session expiration
func RecordSessionExpired() {
	MCPSessionsExpiredTotal.Inc()
}

// RecordPanic records a recovered panic
func RecordPanic(path string) {
	PanicsTotal.WithLabelValues(path).Inc()
}

// RecordDBStats records database connection pool statistics
func RecordDBStats(stats struct {
	Open         int
	InUse        int
	Idle         int
	WaitDuration float64
	WaitCount    int64
}) {
	DBConnectionsActive.WithLabelValues("open").Set(float64(stats.Open))
	DBConnectionsActive.WithLabelValues("in_use").Set(float64(stats.InUse))
	DBConnectionsActive.WithLabelValues("idle").Set(float64(stats.Idle))
	DBConnectionsWaitDuration.Add(stats.WaitDuration)
	DBConnectionsWaitCount.Add(float64(stats.WaitCount))
}

// RecordDBQuery records database query duration
func RecordDBQuery(operation, table string, duration time.Duration) {
	DBQueryDuration.WithLabelValues(operation, table).Observe(duration.Seconds())
}

// RecordCacheOperation records cache operation duration
func RecordCacheOperation(operation string, duration time.Duration) {
	CacheOperationDuration.WithLabelValues(operation).Observe(duration.Seconds())
}

// RecordSLOCompliance records SLO compliance status
func RecordSLOCompliance(sloName string, compliant bool) {
	value := 0.0
	if compliant {
		value = 1.0
	}
	SLOCompliance.WithLabelValues(sloName).Set(value)
}

// RecordErrorBudgetBurnRate records error budget burn rate
func RecordErrorBudgetBurnRate(sloName, window string, rate float64) {
	ErrorBudgetBurnRate.WithLabelValues(sloName, window).Set(rate)
}

// RecordSLI records SLI value
func RecordSLI(sloName string, value float64) {
	SLIValue.WithLabelValues(sloName).Set(value)
}

// RecordTeamToolCall records a team tool call
func RecordTeamToolCall(tool string, success bool) {
	result := "success"
	if !success {
		result = "error"
	}
	TeamToolCallsTotal.WithLabelValues(tool, result).Inc()
}

// RecordTeamToolDuration records team tool execution duration
func RecordTeamToolDuration(tool string, duration time.Duration) {
	TeamToolDuration.WithLabelValues(tool).Observe(duration.Seconds())
}

// RecordTeamToolError records a team tool error
func RecordTeamToolError(tool string, errorType string) {
	TeamToolErrorsTotal.WithLabelValues(tool, errorType).Inc()
}

// IncrementTeamToolActive increments active team tool operations
func IncrementTeamToolActive(tool string) {
	TeamToolActiveOperations.WithLabelValues(tool).Inc()
}

// DecrementTeamToolActive decrements active team tool operations
func DecrementTeamToolActive(tool string) {
	TeamToolActiveOperations.WithLabelValues(tool).Dec()
}

// RecordTeamToolPythonExec records Python script execution duration
func RecordTeamToolPythonExec(command string, duration time.Duration) {
	TeamToolPythonExecDuration.WithLabelValues(command).Observe(duration.Seconds())
}

// RecordPerformanceOperation records a performance operation metric (OPS-008)
func RecordPerformanceOperation(operation string, duration time.Duration, success bool) {
	result := "success"
	if !success {
		result = "error"
	}
	PerformanceOperationDuration.WithLabelValues(operation).Observe(duration.Seconds())
	PerformanceOperationTotal.WithLabelValues(operation, result).Inc()
}

// RecordPerformanceOperationError records a performance operation error (OPS-008)
func RecordPerformanceOperationError(operation string, errorType string) {
	PerformanceOperationErrors.WithLabelValues(operation, errorType).Inc()
}
