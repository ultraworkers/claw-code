package database

import (
	"context"
	"database/sql"
	"errors"
	"fmt"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// FailureStore handles failure registry database operations
type FailureStore struct {
	db *DB
}

// NewFailureStore creates a new failure store
func NewFailureStore(db *DB) *FailureStore {
	return &FailureStore{db: db}
}

// GetByID retrieves a failure by ID
func (s *FailureStore) GetByID(ctx context.Context, id uuid.UUID) (*models.FailureEntry, error) {
	var f models.FailureEntry
	err := s.db.QueryRowContext(ctx, `
		SELECT id, failure_id, category, severity, error_message, root_cause, affected_files, regression_pattern, status, project_slug, created_at, updated_at
		FROM failure_registry
		WHERE id = $1
	`, id).Scan(
		&f.ID, &f.FailureID, &f.Category, &f.Severity, &f.ErrorMessage,
		&f.RootCause, &f.AffectedFiles, &f.RegressionPattern, &f.Status,
		&f.ProjectSlug, &f.CreatedAt, &f.UpdatedAt,
	)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("failure not found: %s", id)
		}
		return nil, fmt.Errorf("failed to get failure: %w", err)
	}
	return &f, nil
}

// List retrieves failures with optional filters
// Uses type-safe query building to prevent SQL injection
func (s *FailureStore) List(ctx context.Context, status, category, projectSlug string, limit, offset int) ([]models.FailureEntry, error) {
	// Build query safely without string concatenation to prevent SQL injection
	type filter struct {
		query string
		args  []interface{}
	}

	var f filter

	// Determine which query to use based on provided filters
	switch {
	case status != "" && category != "" && projectSlug != "":
		f.query = `
			SELECT id, failure_id, category, severity, error_message, root_cause, affected_files, regression_pattern, status, project_slug, created_at, updated_at
			FROM failure_registry
			WHERE status = $1 AND category = $2 AND project_slug = $3
			ORDER BY created_at DESC LIMIT $4 OFFSET $5
		`
		f.args = []interface{}{status, category, projectSlug, limit, offset}
	case status != "" && category != "":
		f.query = `
			SELECT id, failure_id, category, severity, error_message, root_cause, affected_files, regression_pattern, status, project_slug, created_at, updated_at
			FROM failure_registry
			WHERE status = $1 AND category = $2
			ORDER BY created_at DESC LIMIT $3 OFFSET $4
		`
		f.args = []interface{}{status, category, limit, offset}
	case status != "" && projectSlug != "":
		f.query = `
			SELECT id, failure_id, category, severity, error_message, root_cause, affected_files, regression_pattern, status, project_slug, created_at, updated_at
			FROM failure_registry
			WHERE status = $1 AND project_slug = $2
			ORDER BY created_at DESC LIMIT $3 OFFSET $4
		`
		f.args = []interface{}{status, projectSlug, limit, offset}
	case category != "" && projectSlug != "":
		f.query = `
			SELECT id, failure_id, category, severity, error_message, root_cause, affected_files, regression_pattern, status, project_slug, created_at, updated_at
			FROM failure_registry
			WHERE category = $1 AND project_slug = $2
			ORDER BY created_at DESC LIMIT $3 OFFSET $4
		`
		f.args = []interface{}{category, projectSlug, limit, offset}
	case status != "":
		f.query = `
			SELECT id, failure_id, category, severity, error_message, root_cause, affected_files, regression_pattern, status, project_slug, created_at, updated_at
			FROM failure_registry
			WHERE status = $1
			ORDER BY created_at DESC LIMIT $2 OFFSET $3
		`
		f.args = []interface{}{status, limit, offset}
	case category != "":
		f.query = `
			SELECT id, failure_id, category, severity, error_message, root_cause, affected_files, regression_pattern, status, project_slug, created_at, updated_at
			FROM failure_registry
			WHERE category = $1
			ORDER BY created_at DESC LIMIT $2 OFFSET $3
		`
		f.args = []interface{}{category, limit, offset}
	case projectSlug != "":
		f.query = `
			SELECT id, failure_id, category, severity, error_message, root_cause, affected_files, regression_pattern, status, project_slug, created_at, updated_at
			FROM failure_registry
			WHERE project_slug = $1
			ORDER BY created_at DESC LIMIT $2 OFFSET $3
		`
		f.args = []interface{}{projectSlug, limit, offset}
	default:
		f.query = `
			SELECT id, failure_id, category, severity, error_message, root_cause, affected_files, regression_pattern, status, project_slug, created_at, updated_at
			FROM failure_registry
			ORDER BY created_at DESC LIMIT $1 OFFSET $2
		`
		f.args = []interface{}{limit, offset}
	}

	rows, err := s.db.QueryContext(ctx, f.query, f.args...)
	if err != nil {
		return nil, fmt.Errorf("failed to list failures: %w", err)
	}
	defer rows.Close()

	var failures []models.FailureEntry
	for rows.Next() {
		var entry models.FailureEntry
		err := rows.Scan(
			&entry.ID, &entry.FailureID, &entry.Category, &entry.Severity, &entry.ErrorMessage,
			&entry.RootCause, &entry.AffectedFiles, &entry.RegressionPattern, &entry.Status,
			&entry.ProjectSlug, &entry.CreatedAt, &entry.UpdatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan failure: %w", err)
		}
		failures = append(failures, entry)
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("error iterating failures: %w", err)
	}

	return failures, nil
}

// Create inserts a new failure within a transaction
func (s *FailureStore) Create(ctx context.Context, f *models.FailureEntry) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	err = tx.QueryRowContext(ctx, `
		INSERT INTO failure_registry (failure_id, category, severity, error_message, root_cause, affected_files, regression_pattern, status, project_slug)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
		RETURNING id, created_at, updated_at
	`, f.FailureID, f.Category, f.Severity, f.ErrorMessage, f.RootCause,
		f.AffectedFiles, f.RegressionPattern, f.Status, f.ProjectSlug,
	).Scan(&f.ID, &f.CreatedAt, &f.UpdatedAt)
	if err != nil {
		return fmt.Errorf("failed to create failure: %w", err)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// Update updates an existing failure within a transaction
func (s *FailureStore) Update(ctx context.Context, f *models.FailureEntry) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	result, err := tx.ExecContext(ctx, `
		UPDATE failure_registry
		SET error_message = $1, root_cause = $2, affected_files = $3, regression_pattern = $4, status = $5, updated_at = NOW()
		WHERE id = $6
	`, f.ErrorMessage, f.RootCause, f.AffectedFiles, f.RegressionPattern, f.Status, f.ID)
	if err != nil {
		return fmt.Errorf("failed to update failure: %w", err)
	}

	rows, err := result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to get rows affected: %w", err)
	}
	if rows == 0 {
		return fmt.Errorf("failure not found: %s", f.ID)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// GetActiveByFiles retrieves active failures that affect given files
func (s *FailureStore) GetActiveByFiles(ctx context.Context, files []string) ([]models.FailureEntry, error) {
	if len(files) == 0 {
		return []models.FailureEntry{}, nil
	}

	// Query for failures where affected_files overlaps with input files
	query := `
		SELECT id, failure_id, category, severity, error_message, root_cause, affected_files, regression_pattern, status, project_slug, created_at, updated_at
		FROM failure_registry
		WHERE status = 'active'
		AND affected_files && $1
		ORDER BY severity DESC, created_at DESC
	`

	rows, err := s.db.QueryContext(ctx, query, files)
	if err != nil {
		return nil, fmt.Errorf("failed to get active failures: %w", err)
	}
	defer rows.Close()

	var failures []models.FailureEntry
	for rows.Next() {
		var f models.FailureEntry
		err := rows.Scan(
			&f.ID, &f.FailureID, &f.Category, &f.Severity, &f.ErrorMessage,
			&f.RootCause, &f.AffectedFiles, &f.RegressionPattern, &f.Status,
			&f.ProjectSlug, &f.CreatedAt, &f.UpdatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan failure: %w", err)
		}
		failures = append(failures, f)
	}

	return failures, rows.Err()
}

// Count returns the total count of failures
func (s *FailureStore) Count(ctx context.Context) (int64, error) {
	var count int64
	err := s.db.QueryRowContext(ctx, `SELECT COUNT(*) FROM failure_registry`).Scan(&count)
	return count, err
}
