package database

import (
	"context"
	"database/sql"
	"fmt"
	"log/slog"
)

// TxFunc is a function that executes within a transaction
type TxFunc func(*sql.Tx) error

// WithTransaction executes a function within a database transaction
// Automatically handles commit/rollback based on error
func (db *DB) WithTransaction(ctx context.Context, fn TxFunc) error {
	tx, err := db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}

	// Defer rollback in case of panic or error
	// Will be a no-op if Commit succeeds
	defer func() {
		if r := recover(); r != nil {
			// Panic occurred, rollback and re-panic
			if rbErr := tx.Rollback(); rbErr != nil {
				slog.Error("Transaction rollback failed after panic", "error", rbErr)
			}
			panic(r)
		}
	}()

	if err := fn(tx); err != nil {
		// Error occurred, rollback
		if rbErr := tx.Rollback(); rbErr != nil {
			slog.Error("Transaction rollback failed", "error", rbErr)
			return fmt.Errorf("transaction failed and rollback failed: %w (rollback error: %v)", err, rbErr)
		}
		return err
	}

	// Success, commit
	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// WithTransactionReadOnly executes a function within a read-only transaction
func (db *DB) WithTransactionReadOnly(ctx context.Context, fn TxFunc) error {
	opts := &sql.TxOptions{
		ReadOnly: true,
	}

	tx, err := db.BeginTx(ctx, opts)
	if err != nil {
		return fmt.Errorf("failed to begin read-only transaction: %w", err)
	}

	defer func() {
		if r := recover(); r != nil {
			if rbErr := tx.Rollback(); rbErr != nil {
				slog.Error("Read-only transaction rollback failed after panic", "error", rbErr)
			}
			panic(r)
		}
	}()

	if err := fn(tx); err != nil {
		if rbErr := tx.Rollback(); rbErr != nil {
			slog.Error("Read-only transaction rollback failed", "error", rbErr)
			return fmt.Errorf("read-only transaction failed and rollback failed: %w (rollback error: %v)", err, rbErr)
		}
		return err
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit read-only transaction: %w", err)
	}

	return nil
}

// IsUniqueViolation checks if an error is a PostgreSQL unique constraint violation
func IsUniqueViolation(err error) bool {
	if err == nil {
		return false
	}
	// Check for PostgreSQL unique violation error code 23505
	// This is a simplified check - in production you might want to use
	// github.com/jackc/pgconn for more robust error type checking
	errStr := err.Error()
	return contains(errStr, "23505") || contains(errStr, "unique constraint")
}

// IsForeignKeyViolation checks if an error is a PostgreSQL foreign key violation
func IsForeignKeyViolation(err error) bool {
	if err == nil {
		return false
	}
	errStr := err.Error()
	// PostgreSQL FK violation codes: 23503 (foreign_key_violation), 23506 (triggered_action_exception)
	return contains(errStr, "23503") || contains(errStr, "foreign key constraint")
}

// IsSerializationFailure checks if an error is a PostgreSQL serialization failure
func IsSerializationFailure(err error) bool {
	if err == nil {
		return false
	}
	errStr := err.Error()
	// PostgreSQL serialization failure code 40001
	return contains(errStr, "40001") || contains(errStr, "could not serialize")
}

// IsDeadlockDetected checks if an error is a PostgreSQL deadlock
func IsDeadlockDetected(err error) bool {
	if err == nil {
		return false
	}
	errStr := err.Error()
	// PostgreSQL deadlock code 40P01
	return contains(errStr, "40P01") || contains(errStr, "deadlock detected")
}

// contains checks if a string contains a substring
func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > 0 && containsHelper(s, substr))
}

func containsHelper(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
