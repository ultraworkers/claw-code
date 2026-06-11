package middleware

import (
	"log/slog"
	"time"

	"github.com/labstack/echo/v4"
	"github.com/thearchitectit/guardrail-mcp/internal/metrics"
)

// RequestLoggerConfig defines config for RequestLogger middleware
type RequestLoggerConfig struct {
	// Skipper defines a function to skip middleware
	Skipper func(c echo.Context) bool

	// LogLevel defines the log level for successful requests
	LogLevel slog.Level

	// LogLevelError defines the log level for error requests
	LogLevelError slog.Level
}

// DefaultRequestLoggerConfig is the default request logger middleware config
var DefaultRequestLoggerConfig = RequestLoggerConfig{
	Skipper: func(c echo.Context) bool {
		// Skip health checks and metrics to reduce noise
		path := c.Request().URL.Path
		return path == "/health/live" || path == "/health/ready" || path == "/metrics"
	},
	LogLevel:      slog.LevelInfo,
	LogLevelError: slog.LevelError,
}

// RequestLogger returns a middleware that logs HTTP requests
func RequestLogger() echo.MiddlewareFunc {
	return RequestLoggerWithConfig(DefaultRequestLoggerConfig)
}

// RequestLoggerWithConfig returns a RequestLogger middleware with config
func RequestLoggerWithConfig(config RequestLoggerConfig) echo.MiddlewareFunc {
	if config.Skipper == nil {
		config.Skipper = DefaultRequestLoggerConfig.Skipper
	}

	return func(next echo.HandlerFunc) echo.HandlerFunc {
		return func(c echo.Context) error {
			if config.Skipper(c) {
				return next(c)
			}

			req := c.Request()
			res := c.Response()

			start := time.Now()
			err := next(c)
			duration := time.Since(start)

			// Get correlation ID from request
			correlationID := req.Header.Get("X-Correlation-ID")
			if correlationID == "" {
				correlationID = res.Header().Get(echo.HeaderXRequestID)
			}

			// Get client IP
			clientIP := c.RealIP()

			// Get request ID from Echo
			requestID := res.Header().Get(echo.HeaderXRequestID)

			// Build log attributes
			attrs := []slog.Attr{
				slog.String("method", req.Method),
				slog.String("path", c.Path()),
				slog.String("uri", req.RequestURI),
				slog.Int("status", res.Status),
				slog.Duration("duration", duration),
				slog.String("duration_ms", duration.String()),
				slog.String("client_ip", clientIP),
				slog.String("request_id", requestID),
				slog.String("correlation_id", correlationID),
				slog.Int64("bytes_in", req.ContentLength),
				slog.Int64("bytes_out", res.Size),
				slog.String("user_agent", req.UserAgent()),
			}

			// Add error if present
			if err != nil {
				attrs = append(attrs, slog.String("error", err.Error()))
			}

			// Determine log level based on status
			level := config.LogLevel
			if res.Status >= 400 {
				level = config.LogLevelError
			}

			// Create log record
			logger := slog.Default()
			r := slog.NewRecord(time.Now(), level, "HTTP Request", 0)
			r.AddAttrs(attrs...)

			// Log the record
			_ = logger.Handler().Handle(c.Request().Context(), r)

			return err
		}
	}
}

// CorrelationIDMiddleware extracts or generates correlation ID
func CorrelationIDMiddleware() echo.MiddlewareFunc {
	return func(next echo.HandlerFunc) echo.HandlerFunc {
		return func(c echo.Context) error {
			req := c.Request()
			res := c.Response()

			// Check for existing correlation ID
			correlationID := req.Header.Get("X-Correlation-ID")
			if correlationID == "" {
				// Use request ID as correlation ID if not provided
				correlationID = res.Header().Get(echo.HeaderXRequestID)
			}

			// Set correlation ID in response header
			res.Header().Set("X-Correlation-ID", correlationID)

			// Store in context for use in handlers
			c.Set("correlation_id", correlationID)

			return next(c)
		}
	}
}

// OperationTimer returns a function to time operations
func OperationTimer(operation string) func() {
	start := time.Now()
	return func() {
		duration := time.Since(start)
		slog.Debug("Operation completed",
			"operation", operation,
			"duration", duration,
			"duration_ms", duration.Milliseconds(),
		)
	}
}

// LogValidationResult logs validation results with context
func LogValidationResult(ctx echo.Context, tool string, allowed bool, violations int, duration time.Duration) {
	result := "allowed"
	if !allowed {
		result = "denied"
	}

	correlationID := ""
	if cid := ctx.Get("correlation_id"); cid != nil {
		correlationID = cid.(string)
	}

	level := slog.LevelInfo
	if violations > 0 {
		level = slog.LevelWarn
	}

	slog.Log(ctx.Request().Context(), level, "Validation result",
		"tool", tool,
		"result", result,
		"violations", violations,
		"duration", duration,
		"correlation_id", correlationID,
		"request_id", ctx.Response().Header().Get(echo.HeaderXRequestID),
	)

	// Record metrics
	metrics.RecordValidation(tool, result, duration)
}
