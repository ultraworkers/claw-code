package database

import (
	"context"
	"log/slog"
	"time"

	"github.com/thearchitectit/guardrail-mcp/internal/metrics"
)

// MetricsCollector periodically collects and reports database metrics
type MetricsCollector struct {
	db     *DB
	ticker *time.Ticker
	stop   chan struct{}
}

// NewMetricsCollector creates a new database metrics collector
func NewMetricsCollector(db *DB, interval time.Duration) *MetricsCollector {
	return &MetricsCollector{
		db:     db,
		ticker: time.NewTicker(interval),
		stop:   make(chan struct{}),
	}
}

// Start begins collecting database metrics
func (c *MetricsCollector) Start() {
	// Collect immediately
	c.collect()

	go func() {
		for {
			select {
			case <-c.ticker.C:
				c.collect()
			case <-c.stop:
				c.ticker.Stop()
				return
			}
		}
	}()
}

// Stop stops the metrics collector
func (c *MetricsCollector) Stop() {
	close(c.stop)
}

// collect gathers and reports current database metrics
func (c *MetricsCollector) collect() {
	stats := c.db.PoolStats()

	metricStats := struct {
		Open         int
		InUse        int
		Idle         int
		WaitDuration float64
		WaitCount    int64
	}{
		Open:         stats.OpenConnections,
		InUse:        stats.InUse,
		Idle:         stats.Idle,
		WaitDuration: stats.WaitDuration.Seconds(),
		WaitCount:    stats.WaitCount,
	}

	metrics.RecordDBStats(metricStats)

	// Log warnings if pool is near capacity
	if stats.OpenConnections > stats.MaxOpenConnections*90/100 {
		slog.Warn("Database connection pool near capacity",
			"open", stats.OpenConnections,
			"max", stats.MaxOpenConnections,
			"in_use", stats.InUse,
			"idle", stats.Idle,
		)
	}

	// Log if there are waits for connections
	if stats.WaitCount > 0 {
		slog.Debug("Database connection waits detected",
			"wait_count", stats.WaitCount,
			"wait_duration_sec", stats.WaitDuration.Seconds(),
		)
	}
}

// TimedQuery executes a database query and records metrics
func (db *DB) TimedQuery(ctx context.Context, operation, table string, queryFunc func() error) error {
	start := time.Now()
	err := queryFunc()
	duration := time.Since(start)

	metrics.RecordDBQuery(operation, table, duration)

	return err
}
