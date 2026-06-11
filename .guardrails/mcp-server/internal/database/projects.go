package database

import (
	"context"
	"database/sql"
	"errors"
	"fmt"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// ProjectStore handles project database operations
type ProjectStore struct {
	db *DB
}

// NewProjectStore creates a new project store
func NewProjectStore(db *DB) *ProjectStore {
	return &ProjectStore{db: db}
}

// GetByID retrieves a project by ID
func (s *ProjectStore) GetByID(ctx context.Context, id uuid.UUID) (*models.Project, error) {
	var proj models.Project
	err := s.db.QueryRowContext(ctx, `
		SELECT id, name, slug, guardrail_context, active_rules, metadata, created_at, updated_at
		FROM projects
		WHERE id = $1
	`, id).Scan(
		&proj.ID, &proj.Name, &proj.Slug, &proj.GuardrailContext, &proj.ActiveRules,
		&proj.Metadata, &proj.CreatedAt, &proj.UpdatedAt,
	)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("project not found: %s", id)
		}
		return nil, fmt.Errorf("failed to get project: %w", err)
	}
	return &proj, nil
}

// GetBySlug retrieves a project by slug
func (s *ProjectStore) GetBySlug(ctx context.Context, slug string) (*models.Project, error) {
	var proj models.Project
	err := s.db.QueryRowContext(ctx, `
		SELECT id, name, slug, guardrail_context, active_rules, metadata, created_at, updated_at
		FROM projects
		WHERE slug = $1
	`, slug).Scan(
		&proj.ID, &proj.Name, &proj.Slug, &proj.GuardrailContext, &proj.ActiveRules,
		&proj.Metadata, &proj.CreatedAt, &proj.UpdatedAt,
	)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("project not found: %s", slug)
		}
		return nil, fmt.Errorf("failed to get project: %w", err)
	}
	return &proj, nil
}

// List retrieves projects with pagination
func (s *ProjectStore) List(ctx context.Context, limit, offset int) ([]models.Project, error) {
	rows, err := s.db.QueryContext(ctx, `
		SELECT id, name, slug, guardrail_context, active_rules, metadata, created_at, updated_at
		FROM projects
		ORDER BY updated_at DESC
		LIMIT $1 OFFSET $2
	`, limit, offset)
	if err != nil {
		return nil, fmt.Errorf("failed to list projects: %w", err)
	}
	defer rows.Close()

	var projects []models.Project
	for rows.Next() {
		var proj models.Project
		err := rows.Scan(
			&proj.ID, &proj.Name, &proj.Slug, &proj.GuardrailContext, &proj.ActiveRules,
			&proj.Metadata, &proj.CreatedAt, &proj.UpdatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan project: %w", err)
		}
		projects = append(projects, proj)
	}

	return projects, rows.Err()
}

// Create inserts a new project within a transaction
func (s *ProjectStore) Create(ctx context.Context, proj *models.Project) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	err = tx.QueryRowContext(ctx, `
		INSERT INTO projects (name, slug, guardrail_context, active_rules, metadata)
		VALUES ($1, $2, $3, $4, $5)
		RETURNING id, created_at, updated_at
	`, proj.Name, proj.Slug, proj.GuardrailContext, proj.ActiveRules, proj.Metadata,
	).Scan(&proj.ID, &proj.CreatedAt, &proj.UpdatedAt)
	if err != nil {
		return fmt.Errorf("failed to create project: %w", err)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// Update updates an existing project within a transaction
func (s *ProjectStore) Update(ctx context.Context, proj *models.Project) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	result, err := tx.ExecContext(ctx, `
		UPDATE projects
		SET name = $1, guardrail_context = $2, active_rules = $3, metadata = $4, updated_at = NOW()
		WHERE slug = $5
	`, proj.Name, proj.GuardrailContext, proj.ActiveRules, proj.Metadata, proj.Slug)
	if err != nil {
		return fmt.Errorf("failed to update project: %w", err)
	}

	rows, err := result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to get rows affected: %w", err)
	}
	if rows == 0 {
		return fmt.Errorf("project not found: %s", proj.Slug)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// Count returns the total number of projects
func (s *ProjectStore) Count(ctx context.Context) (int, error) {
	var count int
	err := s.db.QueryRowContext(ctx, `SELECT COUNT(*) FROM projects`).Scan(&count)
	if err != nil {
		return 0, fmt.Errorf("failed to count projects: %w", err)
	}
	return count, nil
}

// Delete removes a project within a transaction
func (s *ProjectStore) Delete(ctx context.Context, slug string) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	result, err := tx.ExecContext(ctx, `DELETE FROM projects WHERE slug = $1`, slug)
	if err != nil {
		return fmt.Errorf("failed to delete project: %w", err)
	}

	rows, err := result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to get rows affected: %w", err)
	}
	if rows == 0 {
		return fmt.Errorf("project not found: %s", slug)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}
