package circuitbreaker

import (
	"context"
	"fmt"
	"time"

	"github.com/sony/gobreaker"
)

// ExecuteDB runs a database operation with circuit breaker protection
// If the circuit is open, it returns an error immediately without attempting the operation
func ExecuteDB(ctx context.Context, operation func() error) error {
	_, err := DBBreaker.Execute(func() (interface{}, error) {
		// Create a channel for the operation result
		type result struct {
			err error
		}
		done := make(chan result, 1)

		// Run the operation in a goroutine
		go func() {
			done <- result{err: operation()}
		}()

		// Wait for either the operation to complete or context to be cancelled
		select {
		case res := <-done:
			return nil, res.err
		case <-ctx.Done():
			return nil, fmt.Errorf("database operation cancelled: %w", ctx.Err())
		}
	})
	return err
}

// ExecuteRedis runs a Redis operation with circuit breaker protection
func ExecuteRedis(ctx context.Context, operation func() error) error {
	_, err := RedisBreaker.Execute(func() (interface{}, error) {
		type result struct {
			err error
		}
		done := make(chan result, 1)

		go func() {
			done <- result{err: operation()}
		}()

		select {
		case res := <-done:
			return nil, res.err
		case <-ctx.Done():
			return nil, fmt.Errorf("redis operation cancelled: %w", ctx.Err())
		}
	})
	return err
}

// ExecuteWithRetry runs an operation with circuit breaker and retry logic
// It will retry transient failures up to maxRetries with exponential backoff
func ExecuteWithRetry(ctx context.Context, breaker *gobreaker.CircuitBreaker, maxRetries int, operation func() error) error {
	var lastErr error

	for attempt := 0; attempt < maxRetries; attempt++ {
		_, err := breaker.Execute(func() (interface{}, error) {
			type result struct {
				err error
			}
			done := make(chan result, 1)

			go func() {
				done <- result{err: operation()}
			}()

			select {
			case res := <-done:
				return nil, res.err
			case <-ctx.Done():
				return nil, ctx.Err()
			}
		})

		if err == nil {
			return nil
		}

		lastErr = err

		// Check if it's a circuit breaker error - don't retry if circuit is open
		if err == gobreaker.ErrOpenState {
			return fmt.Errorf("circuit breaker is open: %w", err)
		}

		// Don't retry on context cancellation
		if ctx.Err() != nil {
			return fmt.Errorf("operation cancelled: %w", ctx.Err())
		}

		// Exponential backoff before retry
		if attempt < maxRetries-1 {
			backoff := time.Duration(attempt+1) * 100 * time.Millisecond
			select {
			case <-time.After(backoff):
				continue
			case <-ctx.Done():
				return fmt.Errorf("operation cancelled during retry: %w", ctx.Err())
			}
		}
	}

	return fmt.Errorf("operation failed after %d attempts: %w", maxRetries, lastErr)
}

// GetDBState returns the current state of the database circuit breaker
func GetDBState() string {
	return State(DBBreaker)
}

// GetRedisState returns the current state of the Redis circuit breaker
func GetRedisState() string {
	return State(RedisBreaker)
}
