package web

import (
	"crypto/sha256"
	"crypto/subtle"
	"encoding/hex"
	"log/slog"
	"net/http"
	"strings"

	"github.com/labstack/echo/v4"
	"github.com/thearchitectit/guardrail-mcp/internal/cache"
	"github.com/thearchitectit/guardrail-mcp/internal/config"
)

// APIKeyAuth creates middleware for API key authentication
func APIKeyAuth(cfg *config.Config) echo.MiddlewareFunc {
	return func(next echo.HandlerFunc) echo.HandlerFunc {
		return func(c echo.Context) error {
			// Allow OPTIONS requests (CORS preflight) without authentication
			// This must be checked first, before any path checks
			if c.Request().Method == http.MethodOptions {
				return next(c)
			}

			// Use the actual request URL path, not the route pattern
			// This is critical because c.Path() returns route pattern which may be /*
			requestPath := c.Request().URL.Path

			// Skip health checks, metrics, Web UI routes, and SSE endpoint
			path := c.Path()
			if path == "/health/live" || path == "/health/ready" || path == "/metrics" {
				return next(c)
			}
			// Skip SSE endpoint - auth handled via message endpoint
			if path == "/mcp/v1/sse" {
				return next(c)
			}

			// Skip Web UI routes - these are publicly accessible
			// Check both route pattern and actual request path
			if path == "/" || path == "/index.html" || path == "/web/*" || strings.HasPrefix(path, "/static/") {
				return next(c)
			}
			// Also check actual request path for web UI files
			if requestPath == "/" || requestPath == "/index.html" ||
				strings.HasPrefix(requestPath, "/static/") ||
				strings.HasPrefix(requestPath, "/web/") ||
				strings.HasPrefix(requestPath, "/assets/") ||
				strings.HasPrefix(requestPath, "/js/") ||
				strings.HasPrefix(requestPath, "/css/") ||
				requestPath == "/web" {
				return next(c)
			}
			// Check for common web file extensions that should be public
			if strings.HasSuffix(requestPath, ".js") ||
				strings.HasSuffix(requestPath, ".css") ||
				strings.HasSuffix(requestPath, ".html") ||
				strings.HasSuffix(requestPath, ".svg") ||
				strings.HasSuffix(requestPath, ".png") ||
				strings.HasSuffix(requestPath, ".jpg") ||
				strings.HasSuffix(requestPath, ".ico") ||
				strings.HasSuffix(requestPath, ".json") ||
				strings.HasSuffix(requestPath, ".woff") ||
				strings.HasSuffix(requestPath, ".woff2") ||
				strings.HasSuffix(requestPath, ".ttf") {
				return next(c)
			}

			// Skip read-only API endpoints for public browsing (GET and OPTIONS requests)
			// OPTIONS is needed for CORS preflight requests
			method := c.Request().Method
			if (method == "GET" || method == "OPTIONS") && (path == "/api/documents" || path == "/api/documents/search" ||
				strings.HasPrefix(path, "/api/documents/") ||
				path == "/api/rules" || strings.HasPrefix(path, "/api/rules/") ||
				path == "/api/stats" || strings.HasPrefix(path, "/api/stats/") ||
				path == "/api/projects" || strings.HasPrefix(path, "/api/projects/") ||
				path == "/api/failures" || strings.HasPrefix(path, "/api/failures/") ||
				path == "/api/ingest/status" ||
				path == "/api/ingest/orphans" ||
				path == "/api/updates/status" ||
				path == "/version") {
				return next(c)
			}

			// Skip safe write operations for public browsing when no API key is configured
			// These are safe operations that don't expose sensitive data
			if method == "POST" && (path == "/api/ingest" ||
				path == "/api/ingest/sync" ||
				path == "/api/updates/check") {
				return next(c)
			}

			// Extract API key from header
			auth := c.Request().Header.Get("Authorization")
			if auth == "" {
				return echo.NewHTTPError(http.StatusUnauthorized, "Missing authorization header")
			}

			// Parse Bearer token
			parts := strings.SplitN(auth, " ", 2)
			if len(parts) != 2 || strings.ToLower(parts[0]) != "bearer" {
				return echo.NewHTTPError(http.StatusUnauthorized, "Invalid authorization format, expected 'Bearer <api_key>'")
			}

			apiKey := parts[1]

			// Determine which key type is being used
			var keyType string
			if subtle.ConstantTimeCompare([]byte(apiKey), []byte(cfg.MCPAPIKey)) == 1 {
				keyType = "mcp"
			} else if subtle.ConstantTimeCompare([]byte(apiKey), []byte(cfg.IDEAPIKey)) == 1 {
				keyType = "ide"
			} else {
				slog.Warn("Invalid API key attempt",
					"ip", c.RealIP(),
					"path", path,
				)
				return echo.NewHTTPError(http.StatusUnauthorized, "Invalid API key")
			}

			// Check endpoint restrictions
			if strings.HasPrefix(path, "/ide") && keyType != "ide" && keyType != "mcp" {
				return echo.NewHTTPError(http.StatusForbidden, "IDE API key required for this endpoint")
			}

			// Store key type in context for later use
			c.Set("api_key_type", keyType)
			c.Set("api_key_hash", hashAPIKey(apiKey))

			slog.Debug("API request authenticated",
				"key_type", keyType,
				"key_hash", hashAPIKey(apiKey),
				"path", path,
			)

			return next(c)
		}
	}
}

// RateLimitMiddleware creates middleware for rate limiting
func RateLimitMiddleware(limiter *cache.DistributedRateLimiter, cfg *config.Config) echo.MiddlewareFunc {
	return func(next echo.HandlerFunc) echo.HandlerFunc {
		return func(c echo.Context) error {
			// Use the actual request URL path
			requestPath := c.Request().URL.Path

			// Skip health checks, Web UI routes, and SSE endpoint
			path := c.Path()
			if path == "/health/live" || path == "/health/ready" || path == "/metrics" {
				return next(c)
			}

			// Skip Web UI routes - these are publicly accessible
			if path == "/" || path == "/index.html" || path == "/web/*" || strings.HasPrefix(path, "/static/") {
				return next(c)
			}
			// Also check actual request path for web UI files
			if requestPath == "/" || requestPath == "/index.html" ||
				strings.HasPrefix(requestPath, "/static/") ||
				strings.HasPrefix(requestPath, "/web/") ||
				strings.HasPrefix(requestPath, "/assets/") ||
				strings.HasPrefix(requestPath, "/js/") ||
				strings.HasPrefix(requestPath, "/css/") ||
				requestPath == "/web" {
				return next(c)
			}
			// Check for common web file extensions that should be public
			if strings.HasSuffix(requestPath, ".js") ||
				strings.HasSuffix(requestPath, ".css") ||
				strings.HasSuffix(requestPath, ".html") ||
				strings.HasSuffix(requestPath, ".svg") ||
				strings.HasSuffix(requestPath, ".png") ||
				strings.HasSuffix(requestPath, ".jpg") ||
				strings.HasSuffix(requestPath, ".ico") ||
				strings.HasSuffix(requestPath, ".json") ||
				strings.HasSuffix(requestPath, ".woff") ||
				strings.HasSuffix(requestPath, ".woff2") ||
				strings.HasSuffix(requestPath, ".ttf") {
				return next(c)
			}

			// Skip SSE endpoint - auth handled via message endpoint
			if path == "/mcp/v1/sse" {
				return next(c)
			}

			// Skip read-only API endpoints for public browsing (GET and OPTIONS requests)
			// OPTIONS is needed for CORS preflight requests
			method := c.Request().Method
			if (method == "GET" || method == "OPTIONS") && (path == "/api/documents" || path == "/api/documents/search" ||
				strings.HasPrefix(path, "/api/documents/") ||
				path == "/api/rules" || strings.HasPrefix(path, "/api/rules/") ||
				path == "/api/stats" || strings.HasPrefix(path, "/api/stats/") ||
				path == "/api/projects" || strings.HasPrefix(path, "/api/projects/") ||
				path == "/api/failures" || strings.HasPrefix(path, "/api/failures/") ||
				path == "/api/ingest/status" ||
				path == "/api/ingest/orphans" ||
				path == "/api/updates/status" ||
				path == "/version") {
				return next(c)
			}

			// Skip safe write operations for public browsing
			if method == "POST" && (path == "/api/ingest" ||
				path == "/api/ingest/sync" ||
				path == "/api/updates/check") {
				return next(c)
			}

			// Determine rate limit based on endpoint and key type
			var limit int
			keyType := c.Get("api_key_type")

			if strings.HasPrefix(path, "/ide") {
				limit = cfg.RateLimitIDE
			} else {
				limit = cfg.RateLimitMCP
			}

			// Use API key hash as rate limit key
			keyHash, ok := c.Get("api_key_hash").(string)
			if !ok {
				keyHash = c.RealIP()
			}

			// Check rate limit
			if !limiter.Allow(c.Request().Context(), keyHash, limit) {
				slog.Warn("Rate limit exceeded",
					"key_type", keyType,
					"key_hash", keyHash,
					"path", path,
					"limit", limit,
				)
				return echo.NewHTTPError(http.StatusTooManyRequests, "Rate limit exceeded")
			}

			return next(c)
		}
	}
}

// hashAPIKey creates a hash of the API key for logging
func hashAPIKey(key string) string {
	// Use stack-allocated array for hashing
	var h [32]byte
	h = sha256.Sum256([]byte(key))
	return hex.EncodeToString(h[:8])
}
