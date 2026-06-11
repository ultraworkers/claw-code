package database

import (
	"context"
	"database/sql"
	"fmt"
	"log/slog"
	"runtime"
	"time"

	"github.com/thearchitectit/guardrail-mcp/internal/config"

	_ "github.com/jackc/pgx/v5/stdlib"
)

// Connection pool configuration constants
const (
	defaultMaxConnLifetime    = 15 * time.Minute
	defaultMaxConnIdleTime    = 5 * time.Minute
	defaultHealthCheckTimeout = 3 * time.Second
	defaultConnectTimeout     = 5 * time.Second
	minConnections            = 50
	connMultiplier            = 4
)

// DB wraps sql.DB with guardrail-specific operations
type DB struct {
	*sql.DB
}

// New creates a new database connection with connection pooling and retry logic
func New(cfg *config.Config) (*DB, error) {
	db, err := sql.Open("pgx", cfg.DatabaseURL())
	if err != nil {
		return nil, fmt.Errorf("failed to open database: %w", err)
	}

	// Configure connection pool
	// Scale based on CPU cores - need 50+ for 1000 sessions
	maxConns := connMultiplier * runtime.NumCPU()
	if maxConns < minConnections {
		maxConns = minConnections
	}

	db.SetMaxOpenConns(maxConns)
	db.SetMaxIdleConns(maxConns / 2)
	db.SetConnMaxLifetime(defaultMaxConnLifetime)
	db.SetConnMaxIdleTime(defaultMaxConnIdleTime)

	// Verify connection with retry
	if err := pingWithRetry(db, 3); err != nil {
		db.Close()
		return nil, fmt.Errorf("failed to connect to database after retries: %w", err)
	}

	slog.Info("Database connected",
		"max_conns", maxConns,
		"host", cfg.DBHost,
		"database", cfg.DBName,
	)

	return &DB{db}, nil
}

// pingWithRetry attempts to ping the database with exponential backoff
func pingWithRetry(db *sql.DB, maxRetries int) error {
	var err error
	for i := 0; i < maxRetries; i++ {
		ctx, cancel := context.WithTimeout(context.Background(), defaultConnectTimeout)
		err = db.PingContext(ctx)
		cancel()

		if err == nil {
			return nil
		}

		if i < maxRetries-1 {
			backoff := time.Duration(i+1) * time.Second
			slog.Warn("Database ping failed, retrying",
				"attempt", i+1,
				"max_retries", maxRetries,
				"backoff", backoff,
				"error", err,
			)
			time.Sleep(backoff)
		}
	}
	return err
}

// HealthCheck verifies database connectivity and pool health
func (db *DB) HealthCheck(ctx context.Context) error {
	ctx, cancel := context.WithTimeout(ctx, defaultHealthCheckTimeout)
	defer cancel()

	if err := db.PingContext(ctx); err != nil {
		return fmt.Errorf("database ping failed: %w", err)
	}

	// Check pool health
	stats := db.Stats()
	if stats.OpenConnections > stats.MaxOpenConnections*90/100 {
		slog.Warn("Database connection pool near capacity",
			"open", stats.OpenConnections,
			"max", stats.MaxOpenConnections,
		)
	}

	return nil
}

// PoolStats returns current connection pool statistics
func (db *DB) PoolStats() sql.DBStats {
	return db.Stats()
}

// Close gracefully closes the database connection
func (db *DB) Close() error {
	slog.Info("Closing database connection")
	return db.DB.Close()
}
