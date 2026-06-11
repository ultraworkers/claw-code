package database

import (
	"context"
	"database/sql"
	"errors"
	"fmt"

	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// ProductionCodeStore handles production code tracking database operations
type ProductionCodeStore struct {
	db *DB
}

// NewProductionCodeStore creates a new production code store
func NewProductionCodeStore(db *DB) *ProductionCodeStore {
	return &ProductionCodeStore{db: db}
}

// CreateOrUpdate inserts or updates a production code record
func (s *ProductionCodeStore) CreateOrUpdate(ctx context.Context, pc *models.ProductionCode) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	if err := pc.Validate(); err != nil {
		return fmt.Errorf("invalid production code record: %w", err)
	}

	query := `
		INSERT INTO production_code_tracking (session_id, file_path, code_type)
		VALUES ($1, $2, $3)
		ON CONFLICT (session_id, file_path) DO UPDATE
		SET code_type = EXCLUDED.code_type
		RETURNING id, created_at
	`
	err = tx.QueryRowContext(ctx, query, pc.SessionID, pc.FilePath, pc.CodeType).Scan(&pc.ID, &pc.CreatedAt)
	if err != nil {
		return fmt.Errorf("failed to create production code record: %w", err)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// MarkAsVerified marks production code as verified
func (s *ProductionCodeStore) MarkAsVerified(ctx context.Context, sessionID, filePath string) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	result, err := tx.ExecContext(ctx, `
		UPDATE production_code_tracking
		SET verified_at = NOW()
		WHERE session_id = $1 AND file_path = $2 AND code_type = 'production'
	`, sessionID, filePath)
	if err != nil {
		return fmt.Errorf("failed to mark as verified: %w", err)
	}

	rows, err := result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to get rows affected: %w", err)
	}

	if rows == 0 {
		return sql.ErrNoRows
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// GetBySessionAndPath retrieves a production code record by session ID and file path
func (s *ProductionCodeStore) GetBySessionAndPath(ctx context.Context, sessionID, filePath string) (*models.ProductionCode, error) {
	var pc models.ProductionCode
	query := `
		SELECT id, session_id, file_path, code_type, created_at, verified_at
		FROM production_code_tracking
		WHERE session_id = $1 AND file_path = $2
	`
	err := s.db.QueryRowContext(ctx, query, sessionID, filePath).Scan(
		&pc.ID, &pc.SessionID, &pc.FilePath, &pc.CodeType, &pc.CreatedAt, &pc.VerifiedAt,
	)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, sql.ErrNoRows
		}
		return nil, fmt.Errorf("failed to get production code record: %w", err)
	}
	return &pc, nil
}

// HasProductionCode checks if production code exists in the session
func (s *ProductionCodeStore) HasProductionCode(ctx context.Context, sessionID string) (bool, error) {
	var exists bool
	query := `
		SELECT EXISTS(
			SELECT 1 FROM production_code_tracking
			WHERE session_id = $1 AND code_type = 'production'
		)
	`
	err := s.db.QueryRowContext(ctx, query, sessionID).Scan(&exists)
	if err != nil {
		return false, fmt.Errorf("failed to check production code existence: %w", err)
	}
	return exists, nil
}

// ListBySession retrieves all production code records for a session
func (s *ProductionCodeStore) ListBySession(ctx context.Context, sessionID string, limit, offset int) ([]models.ProductionCode, error) {
	rows, err := s.db.QueryContext(ctx, `
		SELECT id, session_id, file_path, code_type, created_at, verified_at
		FROM production_code_tracking
		WHERE session_id = $1
		ORDER BY created_at DESC
		LIMIT $2 OFFSET $3
	`, sessionID, limit, offset)
	if err != nil {
		return nil, fmt.Errorf("failed to list production code records: %w", err)
	}
	defer rows.Close()

	return scanProductionCodeRecords(rows)
}

// DeleteBySession removes all production code records for a session
func (s *ProductionCodeStore) DeleteBySession(ctx context.Context, sessionID string) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	result, err := tx.ExecContext(ctx, `DELETE FROM production_code_tracking WHERE session_id = $1`, sessionID)
	if err != nil {
		return fmt.Errorf("failed to delete production code records: %w", err)
	}

	_, err = result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to get rows affected: %w", err)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// CountBySession returns the number of production code records for a session
func (s *ProductionCodeStore) CountBySession(ctx context.Context, sessionID string) (int, error) {
	var count int
	err := s.db.QueryRowContext(ctx, `
		SELECT COUNT(*) FROM production_code_tracking WHERE session_id = $1
	`, sessionID).Scan(&count)
	if err != nil {
		return 0, fmt.Errorf("failed to count production code records: %w", err)
	}
	return count, nil
}

// CountByType returns the count of production code records by type for a session
func (s *ProductionCodeStore) CountByType(ctx context.Context, sessionID string) (map[string]int, error) {
	rows, err := s.db.QueryContext(ctx, `
		SELECT code_type, COUNT(*) as count
		FROM production_code_tracking
		WHERE session_id = $1
		GROUP BY code_type
	`, sessionID)
	if err != nil {
		return nil, fmt.Errorf("failed to count by type: %w", err)
	}
	defer rows.Close()

	counts := make(map[string]int)
	for rows.Next() {
		var codeType string
		var count int
		if err := rows.Scan(&codeType, &count); err != nil {
			return nil, fmt.Errorf("failed to scan count: %w", err)
		}
		counts[codeType] = count
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("error iterating rows: %w", err)
	}

	return counts, nil
}

// scanProductionCodeRecords is a helper function to scan production code rows
func scanProductionCodeRecords(rows *sql.Rows) ([]models.ProductionCode, error) {
	var records []models.ProductionCode
	for rows.Next() {
		var pc models.ProductionCode
		err := rows.Scan(
			&pc.ID, &pc.SessionID, &pc.FilePath, &pc.CodeType, &pc.CreatedAt, &pc.VerifiedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan production code record: %w", err)
		}
		records = append(records, pc)
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("error iterating production code records: %w", err)
	}

	return records, nil
}
