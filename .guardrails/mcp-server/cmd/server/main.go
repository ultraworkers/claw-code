package main

import (
	"context"
	"flag"
	"fmt"
	"log/slog"
	"net/http"
	_ "net/http/pprof"
	"os"
	"os/signal"
	"runtime"
	"syscall"
	"time"

	"github.com/thearchitectit/guardrail-mcp/internal/api"
	"github.com/thearchitectit/guardrail-mcp/internal/audit"
	"github.com/thearchitectit/guardrail-mcp/internal/cache"
	"github.com/thearchitectit/guardrail-mcp/internal/config"
	"github.com/thearchitectit/guardrail-mcp/internal/database"
	mcpServer "github.com/thearchitectit/guardrail-mcp/internal/mcp"
	"github.com/thearchitectit/guardrail-mcp/internal/validation"
	"github.com/thearchitectit/guardrail-mcp/internal/web"
)

// Version information - set by ldflags during build
var (
	version   = "v2.6.0"
	buildTime = "unknown"
	gitCommit = "unknown"
)

func main() {
	// CLI flags
	var (
		showVersion   = flag.Bool("version", false, "Show version information")
		showHealth    = flag.Bool("health-check", false, "Run health check and exit")
		healthTimeout = flag.Duration("health-timeout", 5*time.Second, "Health check timeout")
	)
	flag.Parse()

	// Show version and exit
	if *showVersion {
		fmt.Printf("Guardrail MCP Server\n")
		fmt.Printf("  Version:   %s\n", version)
		fmt.Printf("  Build Time: %s\n", buildTime)
		fmt.Printf("  Git Commit: %s\n", gitCommit)
		fmt.Printf("  Go Version: %s\n", runtime.Version())
		os.Exit(0)
	}

	// Health check mode for container health checks
	if *showHealth {
		if err := runHealthCheck(*healthTimeout); err != nil {
			fmt.Fprintf(os.Stderr, "Health check failed: %v\n", err)
			os.Exit(1)
		}
		fmt.Println("Health check passed")
		os.Exit(0)
	}

	// Load configuration first to get log level
	cfg, err := config.Load()
	if err != nil {
		// Use default logger for initial error
		slog.Error("Failed to load configuration", "error", err)
		os.Exit(1)
	}

	// Setup structured logging with configured level
	setLogLevel(cfg.LogLevel)

	slog.Info("Starting Guardrail MCP Server",
		"version", version,
		"build_time", buildTime,
		"git_commit", gitCommit,
		"config_schema", cfg.SchemaVersion,
	)

	// Start pprof server if enabled (for debugging)
	if cfg.PProfEnabled {
		go startPProfServer(cfg.PProfPort)
	}

	// Initialize audit logger
	auditLogger := audit.NewLogger(1000)

	// Connect to database
	db, err := database.New(cfg)
	if err != nil {
		slog.Error("Failed to connect to database", "error", err)
		os.Exit(1)
	}
	defer db.Close()

	// Start database metrics collector
	dbMetricsCollector := database.NewMetricsCollector(db, 15*time.Second)
	dbMetricsCollector.Start()
	defer dbMetricsCollector.Stop()

	// Connect to Redis
	redisClient, err := cache.New(cfg)
	if err != nil {
		slog.Error("Failed to connect to Redis", "error", err)
		os.Exit(1)
	}
	defer redisClient.Close()

	// Create web server
	webServer := web.NewServer(cfg, db, redisClient, auditLogger, version)

	// Create validation engine
	ruleStore := database.NewRuleStore(db)
	fileReadStore := database.NewFileReadStore(db)
	taskAttemptStore := database.NewTaskAttemptStore(db)
	validationEngine := validation.NewValidationEngine(ruleStore, redisClient,
		validation.WithFileReadStore(fileReadStore),
		validation.WithTaskAttemptStore(taskAttemptStore),
	)

	// Create MCP server
	haltEventStore := database.NewHaltEventStore(db)
	mcpSrv := mcpServer.NewMCPServer(cfg, db, redisClient, auditLogger, validationEngine, fileReadStore, taskAttemptStore, haltEventStore)

	// Register vision HTTP routes on the web server if vision is enabled
	if vt := mcpSrv.VisionTools(); vt != nil {
		visionGroup := webServer.Echo().Group("/v1/vision")
		visionServer := api.NewVisionServer(vt.Engine(), vt.Watcher())
		visionServer.RegisterRoutes(visionGroup)
		slog.Info("Vision HTTP API mounted", "prefix", "/v1/vision")
	}

	// Start servers
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Start web server (bind to 0.0.0.0 for containerized deployment)
	go func() {
		defer func() {
			if r := recover(); r != nil {
				slog.Error("Web server goroutine panicked", "panic", r)
				cancel()
			}
		}()
		addr := fmt.Sprintf("0.0.0.0:%d", cfg.WebPort)
		slog.Info("Starting web server", "addr", addr)
		if err := webServer.Start(addr); err != nil && err != http.ErrServerClosed {
			slog.Error("Web server error", "error", err)
			cancel()
		}
	}()

	// Start MCP server (bind to 0.0.0.0 for containerized deployment)
	go func() {
		defer func() {
			if r := recover(); r != nil {
				slog.Error("MCP server goroutine panicked", "panic", r)
				cancel()
			}
		}()
		addr := fmt.Sprintf("0.0.0.0:%d", cfg.MCPPort)
		slog.Info("Starting MCP server", "addr", addr)
		if err := mcpSrv.Start(addr); err != nil && err != http.ErrServerClosed {
			slog.Error("MCP server error", "error", err)
			cancel()
		}
	}()

	// Wait for shutdown signal
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGTERM, syscall.SIGINT, syscall.SIGQUIT)

	select {
	case sig := <-quit:
		slog.Info("Shutdown signal received", "signal", sig.String())
	case <-ctx.Done():
		slog.Info("Context cancelled")
	}

	// Graceful shutdown with configurable timeout
	shutdownTimeout := cfg.ShutdownTimeout
	if shutdownTimeout == 0 {
		shutdownTimeout = 30 * time.Second
	}

	slog.Info("Initiating graceful shutdown", "timeout", shutdownTimeout)

	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), shutdownTimeout)
	defer shutdownCancel()

	// Shutdown web server
	if err := webServer.Shutdown(shutdownCtx); err != nil {
		slog.Error("Web server shutdown error", "error", err)
	}

	// Shutdown MCP server
	if err := mcpSrv.Shutdown(shutdownCtx); err != nil {
		slog.Error("MCP server shutdown error", "error", err)
	}

	// Close database connection
	if err := db.Close(); err != nil {
		slog.Error("Database close error", "error", err)
	}

	// Close Redis connection
	if err := redisClient.Close(); err != nil {
		slog.Error("Redis close error", "error", err)
	}

	slog.Info("Server stopped gracefully")
}

// runHealthCheck performs a health check against the local server
func runHealthCheck(timeout time.Duration) error {
	client := &http.Client{
		Timeout: timeout,
	}

	// Check liveness endpoint
	webPort := os.Getenv("WEB_PORT")
	if webPort == "" {
		webPort = "8081"
	}

	resp, err := client.Get(fmt.Sprintf("http://localhost:%s/health/live", webPort))
	if err != nil {
		return fmt.Errorf("liveness check failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("liveness check returned status %d", resp.StatusCode)
	}

	return nil
}

// startPProfServer starts the pprof debugging server
func startPProfServer(port int) {
	addr := fmt.Sprintf("localhost:%d", port)
	slog.Info("Starting pprof server", "addr", addr)

	// pprof endpoints are registered via _ import
	// /debug/pprof/ - profile overview
	// /debug/pprof/profile - CPU profile
	// /debug/pprof/heap - heap profile
	// /debug/pprof/goroutine - goroutine profile
	if err := http.ListenAndServe(addr, nil); err != nil {
		slog.Error("pprof server error", "error", err)
	}
}

func setLogLevel(level string) {
	var slogLevel slog.Level
	switch level {
	case "debug":
		slogLevel = slog.LevelDebug
	case "info":
		slogLevel = slog.LevelInfo
	case "warn":
		slogLevel = slog.LevelWarn
	case "error":
		slogLevel = slog.LevelError
	default:
		slogLevel = slog.LevelInfo
	}

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{
		Level: slogLevel,
	}))
	slog.SetDefault(logger)
}
