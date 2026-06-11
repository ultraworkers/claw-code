package circuitbreaker

import (
	"time"

	"github.com/sony/gobreaker"
	"github.com/thearchitectit/guardrail-mcp/internal/config"
)

// Manager holds circuit breakers configured from application config
type Manager struct {
	DBBreaker    *gobreaker.CircuitBreaker
	RedisBreaker *gobreaker.CircuitBreaker
}

// NewManager creates circuit breakers with configuration values
func NewManager(cfg *config.Config) *Manager {
	if !cfg.CircuitBreakerEnabled {
		return &Manager{
			DBBreaker:    nil,
			RedisBreaker: nil,
		}
	}

	failureThreshold := uint32(cfg.CircuitBreakerFailureThreshold)

	return &Manager{
		DBBreaker: gobreaker.NewCircuitBreaker(gobreaker.Settings{
			Name:        "database",
			MaxRequests: uint32(cfg.CircuitBreakerMaxRequests),
			Interval:    cfg.CircuitBreakerInterval,
			Timeout:     cfg.CircuitBreakerTimeout,
			ReadyToTrip: func(counts gobreaker.Counts) bool {
				failureRatio := float64(counts.TotalFailures) / float64(counts.Requests)
				return counts.Requests >= failureThreshold && failureRatio >= 0.6
			},
		}),
		RedisBreaker: gobreaker.NewCircuitBreaker(gobreaker.Settings{
			Name:        "redis",
			MaxRequests: uint32(cfg.CircuitBreakerMaxRequests),
			Interval:    cfg.CircuitBreakerInterval,
			Timeout:     cfg.CircuitBreakerTimeout / 6, // Redis should be faster
			ReadyToTrip: func(counts gobreaker.Counts) bool {
				failureRatio := float64(counts.TotalFailures) / float64(counts.Requests)
				return counts.Requests >= failureThreshold && failureRatio >= 0.6
			},
		}),
	}
}

// DBBreaker is the legacy global database circuit breaker (deprecated, use Manager)
// Kept for backward compatibility during migration
var DBBreaker = gobreaker.NewCircuitBreaker(gobreaker.Settings{
	Name:        "database",
	MaxRequests: 3,                // Half-open state probe count
	Interval:    10 * time.Second, // Statistical window
	Timeout:     30 * time.Second, // Request timeout
	ReadyToTrip: func(counts gobreaker.Counts) bool {
		failureRatio := float64(counts.TotalFailures) / float64(counts.Requests)
		return counts.Requests >= 3 && failureRatio >= 0.6
	},
})

// RedisBreaker is the legacy global redis circuit breaker (deprecated, use Manager)
// Kept for backward compatibility during migration
var RedisBreaker = gobreaker.NewCircuitBreaker(gobreaker.Settings{
	Name:        "redis",
	MaxRequests: 3,
	Interval:    10 * time.Second,
	Timeout:     5 * time.Second,
	ReadyToTrip: func(counts gobreaker.Counts) bool {
		failureRatio := float64(counts.TotalFailures) / float64(counts.Requests)
		return counts.Requests >= 3 && failureRatio >= 0.6
	},
})

// State returns the current state of the circuit breaker
func State(breaker *gobreaker.CircuitBreaker) string {
	state := breaker.State()
	switch state {
	case gobreaker.StateClosed:
		return "closed"
	case gobreaker.StateOpen:
		return "open"
	case gobreaker.StateHalfOpen:
		return "half-open"
	default:
		return "unknown"
	}
}
