package web

import (
	"context"
	"log/slog"
	"net/http"
	"os"
	"runtime/debug"
	"strings"
	"time"

	"github.com/labstack/echo/v4"
	"github.com/labstack/echo/v4/middleware"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	"github.com/thearchitectit/guardrail-mcp/internal/audit"
	"github.com/thearchitectit/guardrail-mcp/internal/cache"
	"github.com/thearchitectit/guardrail-mcp/internal/config"
	"github.com/thearchitectit/guardrail-mcp/internal/database"
	"github.com/thearchitectit/guardrail-mcp/internal/ingest"
	metricsMiddleware "github.com/thearchitectit/guardrail-mcp/internal/metrics"
	loggingMiddleware "github.com/thearchitectit/guardrail-mcp/internal/middleware"
	"github.com/thearchitectit/guardrail-mcp/internal/updates"
)

// Server wraps the Echo server with guardrail dependencies
type Server struct {
	echo          *echo.Echo
	cfg           *config.Config
	db            *database.DB
	cache         *cache.Client
	auditLogger   *audit.Logger
	docStore      *database.DocumentStore
	ruleStore     *database.RuleStore
	projStore     *database.ProjectStore
	failStore     *database.FailureStore
	ingestSvc     *ingest.Service
	updateChecker *updates.Checker
	version       string
}

// NewServer creates a new web server
func NewServer(cfg *config.Config, db *database.DB, cacheClient *cache.Client, auditLogger *audit.Logger, version string) *Server {
	e := echo.New()
	e.HideBanner = true
	e.HidePort = true

	docStore := database.NewDocumentStore(db)

	s := &Server{
		echo:          e,
		cfg:           cfg,
		db:            db,
		cache:         cacheClient,
		auditLogger:   auditLogger,
		docStore:      docStore,
		ruleStore:     database.NewRuleStore(db),
		projStore:     database.NewProjectStore(db),
		failStore:     database.NewFailureStore(db),
		ingestSvc:     ingest.NewService(docStore, database.NewRuleStore(db), []string{"/app/docs"}, "/app/docs"),
		updateChecker: updates.NewChecker(db, version, os.Getenv("GIT_COMMIT")),
		version:       version,
	}

	s.setupMiddleware()
	s.setupRoutes()

	return s
}

// Echo exposes the underlying Echo instance for external route registration.
func (s *Server) Echo() *echo.Echo {
	return s.echo
}

// setupMiddleware configures Echo middleware
func (s *Server) setupMiddleware() {
	// Request ID generation
	s.echo.Use(middleware.RequestID())

	// Correlation ID propagation
	s.echo.Use(correlationIDMiddleware())

	// Recovery from panics with metrics
	s.echo.Use(panicRecoveryMiddleware())

	// Prometheus metrics middleware
	s.echo.Use(metricsMiddleware.PrometheusMiddleware())

	// Request logging
	s.echo.Use(loggingMiddleware.RequestLogger())

	// Security headers
	s.echo.Use(securityHeadersMiddleware())

	// CORS - MUST run before auth middleware to handle OPTIONS preflight requests
	// CORS preflight (OPTIONS) requests should return 200 before auth check
	corsOrigins := s.cfg.CORSAllowedOrigins
	if len(corsOrigins) == 0 || (len(corsOrigins) == 1 && corsOrigins[0] == "*") {
		// Default to restrictive localhost origins if not configured
		if s.cfg.ProductionMode {
			corsOrigins = []string{"http://localhost:8081", "https://localhost:8081"}
		} else {
			corsOrigins = []string{"http://localhost:*", "https://localhost:*"}
		}
	}
	s.echo.Use(middleware.CORSWithConfig(middleware.CORSConfig{
		AllowOrigins: corsOrigins,
		AllowMethods: s.cfg.CORSAllowedMethods,
		AllowHeaders: s.cfg.CORSAllowedHeaders,
		MaxAge:       s.cfg.CORSMaxAge,
	}))

	// API Key Authentication (required for all routes except health/metrics)
	s.echo.Use(APIKeyAuth(s.cfg))

	// Rate Limiting
	limiter := s.cache.NewDistributedLimiter()
	s.echo.Use(RateLimitMiddleware(limiter, s.cfg))

	// Request timeout (skip vision endpoints — they have long-running inference)
	s.echo.Use(middleware.TimeoutWithConfig(middleware.TimeoutConfig{
		Timeout: s.cfg.RequestTimeout,
		Skipper: func(c echo.Context) bool {
			return strings.HasPrefix(c.Request().URL.Path, "/v1/vision")
		},
	}))

	// Body limit
	s.echo.Use(middleware.BodyLimit("10M"))

	// Cache control - prevent caching of API responses
	s.echo.Use(cacheControlMiddleware())
}

// setupRoutes configures all routes
func (s *Server) setupRoutes() {
	// Health endpoints (no auth required)
	s.echo.GET("/health/live", s.healthLive)
	s.echo.GET("/health/ready", s.healthReady)

	// Version endpoint (no auth required)
	s.echo.GET("/version", s.versionInfo)

	// Metrics endpoint
	s.echo.GET("/metrics", echo.WrapHandler(promhttp.Handler()))

	// API routes with authentication
	api := s.echo.Group("/api")

	// Document routes - order matters: specific routes before parameterized routes
	api.GET("/documents", s.listDocuments)
	api.GET("/documents/search", s.searchDocuments)
	api.GET("/documents/:id", s.getDocument)
	api.PUT("/documents/:id", s.updateDocument)

	// Rule routes
	api.GET("/rules", s.listRules)
	api.GET("/rules/:id", s.getRule)
	api.POST("/rules", s.createRule)
	api.PUT("/rules/:id", s.updateRule)
	api.DELETE("/rules/:id", s.deleteRule)
	api.PATCH("/rules/:id", s.patchRule)

	// Rule sync routes
	api.POST("/rules/sync", s.syncRules)
	api.GET("/rules/sync/status", s.getRuleSyncStatus)
	api.POST("/rules/sync/upload", s.triggerRuleSyncFromUpload)

	// Project routes
	api.GET("/projects", s.listProjects)
	api.GET("/projects/:id", s.getProject)
	api.POST("/projects", s.createProject)
	api.PUT("/projects/:id", s.updateProject)
	api.DELETE("/projects/:id", s.deleteProject)

	// Failure registry routes
	api.GET("/failures", s.listFailures)
	api.GET("/failures/:id", s.getFailure)
	api.POST("/failures", s.createFailure)
	api.PUT("/failures/:id", s.updateFailure)

	// System routes
	api.GET("/stats", s.getStats)
	api.POST("/ingest", s.triggerIngest)

	// Update routes
	api.GET("/updates/status", s.getUpdateStatus)
	api.POST("/updates/check", s.checkForUpdates)

	// Ingest routes
	api.POST("/ingest/upload", s.uploadFiles)
	api.POST("/ingest/sync", s.syncRepo)
	api.GET("/ingest/status", s.getIngestStatus)
	api.GET("/ingest/orphans", s.listOrphans)
	api.DELETE("/ingest/orphans/:id", s.deleteOrphan)

	// IDE API endpoints
	ide := s.echo.Group("/ide")
	ide.GET("/health", s.ideHealth)
	ide.POST("/validate/file", s.validateFile)
	ide.POST("/validate/selection", s.validateSelection)
	ide.GET("/rules", s.getIDERules)
	ide.GET("/quick-reference", s.getQuickReference)

	// Static files (Web UI) - ORDER MATTERS: specific routes first
	if s.cfg.WebEnabled {
		// SPA fallback: serve index.html for non-file routes (client-side routing)
		// This handles /web, /web/dashboard, /web/rules etc but NOT /web/js/app.js
		// MUST be registered BEFORE the Static("/", ...) route
		s.echo.GET("/web/*", func(c echo.Context) error {
			// Check if this is a request for an actual file
			requestPath := c.Request().URL.Path

			// If the path has a file extension, try to serve it directly first
			if strings.Contains(requestPath, ".") {
				filePath := "/app/web" + strings.TrimPrefix(requestPath, "/web")
				if _, err := os.Stat(filePath); err == nil {
					return c.File(filePath)
				}
			}

			// For routes without file extension (client-side routes), serve index.html
			return c.File("/app/web/index.html")
		})

		// Static file serving - register AFTER specific routes
		// This serves files at root paths like /index.html, /js/app.js, etc.
		s.echo.Static("/web", "/app/web")
		s.echo.Static("/", "/app/web")
	}
}

// Start starts the server
func (s *Server) Start(addr string) error {
	return s.echo.Start(addr)
}

// Shutdown gracefully shuts down the server
func (s *Server) Shutdown(ctx context.Context) error {
	return s.echo.Shutdown(ctx)
}

// correlationIDMiddleware extracts or generates correlation ID for request tracing
func correlationIDMiddleware() echo.MiddlewareFunc {
	return func(next echo.HandlerFunc) echo.HandlerFunc {
		return func(c echo.Context) error {
			req := c.Request()
			res := c.Response()

			// Check for existing correlation ID from upstream
			correlationID := req.Header.Get("X-Correlation-ID")
			if correlationID == "" {
				// Generate new correlation ID using request ID
				correlationID = res.Header().Get(echo.HeaderXRequestID)
			}

			// Set correlation ID in response header for downstream tracing
			res.Header().Set("X-Correlation-ID", correlationID)

			// Store in context for use in handlers and logging
			c.Set("correlation_id", correlationID)

			// Add to request context for propagation to downstream services
			ctx := context.WithValue(req.Context(), "correlation_id", correlationID)
			c.SetRequest(req.WithContext(ctx))

			return next(c)
		}
	}
}

// panicRecoveryMiddleware recovers from panics and records metrics
func panicRecoveryMiddleware() echo.MiddlewareFunc {
	return func(next echo.HandlerFunc) echo.HandlerFunc {
		return func(c echo.Context) error {
			defer func() {
				if r := recover(); r != nil {
					err, ok := r.(error)
					if !ok {
						err = echo.NewHTTPError(http.StatusInternalServerError, r)
					}

					// Record panic metric
					metricsMiddleware.RecordPanic(c.Path())

					// Log panic with stack trace
					slog.Error("Panic recovered",
						"error", err,
						"path", c.Path(),
						"method", c.Request().Method,
						"correlation_id", c.Get("correlation_id"),
						"request_id", c.Response().Header().Get(echo.HeaderXRequestID),
						"stack", string(debug.Stack()),
					)

					// Return 500 error
					c.Error(err)
				}
			}()
			return next(c)
		}
	}
}

// securityHeadersMiddleware adds security headers
func securityHeadersMiddleware() echo.MiddlewareFunc {
	return func(next echo.HandlerFunc) echo.HandlerFunc {
		return func(c echo.Context) error {
			// Content Security Policy
			csp := "default-src 'self'; " +
				"script-src 'self'; " +
				"style-src 'self' 'unsafe-inline'; " +
				"img-src 'self' data:; " +
				"font-src 'self'; " +
				"connect-src 'self'; " +
				"frame-ancestors 'none'; " +
				"base-uri 'self'; " +
				"form-action 'self'"

			c.Response().Header().Set("Content-Security-Policy", csp)
			c.Response().Header().Set("X-Content-Type-Options", "nosniff")
			c.Response().Header().Set("X-Frame-Options", "DENY")
			c.Response().Header().Set("X-XSS-Protection", "1; mode=block")
			c.Response().Header().Set("Referrer-Policy", "strict-origin-when-cross-origin")
			c.Response().Header().Set("Permissions-Policy", "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()")

			return next(c)
		}
	}
}

// cacheControlMiddleware adds cache-control headers to prevent caching of API responses
func cacheControlMiddleware() echo.MiddlewareFunc {
	return func(next echo.HandlerFunc) echo.HandlerFunc {
		return func(c echo.Context) error {
			// Execute handler first
			err := next(c)

			// Then set cache headers on the response
			path := c.Request().URL.Path
			if strings.HasPrefix(path, "/api/") || strings.HasPrefix(path, "/ide/") {
				c.Response().Header().Set("Cache-Control", "no-store, no-cache, must-revalidate, proxy-revalidate")
				c.Response().Header().Set("Pragma", "no-cache")
				c.Response().Header().Set("Expires", "0")
			}

			return err
		}
	}
}

// versionInfo returns server version information
func (s *Server) versionInfo(c echo.Context) error {
	return c.JSON(http.StatusOK, map[string]interface{}{
		"version":   s.version,
		"service":   "guardrail-mcp",
		"timestamp": time.Now().UTC().Format(time.RFC3339),
	})
}

// Health handlers
func (s *Server) healthLive(c echo.Context) error {
	return c.JSON(http.StatusOK, map[string]interface{}{
		"status":    "alive",
		"version":   s.version,
		"timestamp": time.Now().UTC().Format(time.RFC3339),
	})
}

func (s *Server) healthReady(c echo.Context) error {
	ctx, cancel := context.WithTimeout(c.Request().Context(), s.cfg.HealthCheckTimeout)
	defer cancel()

	// Check database
	if err := s.db.HealthCheck(ctx); err != nil {
		slog.Error("Readiness check failed - database", "error", err)
		return c.JSON(http.StatusServiceUnavailable, map[string]interface{}{
			"status":    "not ready",
			"timestamp": time.Now().UTC().Format(time.RFC3339),
			// Don't expose which component failed for security
		})
	}

	// Check cache
	if err := s.cache.HealthCheck(ctx); err != nil {
		slog.Error("Readiness check failed - cache", "error", err)
		return c.JSON(http.StatusServiceUnavailable, map[string]interface{}{
			"status":    "not ready",
			"timestamp": time.Now().UTC().Format(time.RFC3339),
			// Don't expose which component failed for security
		})
	}

	return c.JSON(http.StatusOK, map[string]interface{}{
		"status":    "ready",
		"version":   s.version,
		"timestamp": time.Now().UTC().Format(time.RFC3339),
	})
}
