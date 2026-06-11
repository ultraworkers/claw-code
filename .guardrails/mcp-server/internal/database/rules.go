package database

import (
	"context"
	"database/sql"
	"errors"
	"fmt"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// RuleStore handles prevention rule database operations
type RuleStore struct {
	db *DB
}

// NewRuleStore creates a new rule store
func NewRuleStore(db *DB) *RuleStore {
	return &RuleStore{db: db}
}

// GetByID retrieves a rule by ID
func (s *RuleStore) GetByID(ctx context.Context, id uuid.UUID) (*models.PreventionRule, error) {
	var rule models.PreventionRule
	err := s.db.QueryRowContext(ctx, `
		SELECT id, rule_id, name, pattern, pattern_hash, message, severity, enabled, document_id, category, created_at, updated_at
		FROM prevention_rules
		WHERE id = $1
	`, id).Scan(
		&rule.ID, &rule.RuleID, &rule.Name, &rule.Pattern, &rule.PatternHash,
		&rule.Message, &rule.Severity, &rule.Enabled, &rule.DocumentID,
		&rule.Category, &rule.CreatedAt, &rule.UpdatedAt,
	)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("rule not found: %s", id)
		}
		return nil, fmt.Errorf("failed to get rule: %w", err)
	}
	return &rule, nil
}

// GetByRuleID retrieves a rule by rule_id
func (s *RuleStore) GetByRuleID(ctx context.Context, ruleID string) (*models.PreventionRule, error) {
	var rule models.PreventionRule
	err := s.db.QueryRowContext(ctx, `
		SELECT id, rule_id, name, pattern, pattern_hash, message, severity, enabled, document_id, category, created_at, updated_at
		FROM prevention_rules
		WHERE rule_id = $1
	`, ruleID).Scan(
		&rule.ID, &rule.RuleID, &rule.Name, &rule.Pattern, &rule.PatternHash,
		&rule.Message, &rule.Severity, &rule.Enabled, &rule.DocumentID,
		&rule.Category, &rule.CreatedAt, &rule.UpdatedAt,
	)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("rule not found: %s", ruleID)
		}
		return nil, fmt.Errorf("failed to get rule: %w", err)
	}
	return &rule, nil
}

// List retrieves rules with optional filters and pagination
func (s *RuleStore) List(ctx context.Context, enabled *bool, category string, limit, offset int) ([]models.PreventionRule, error) {
	// Build query safely without string concatenation to prevent SQL injection
	var query string
	var args []interface{}

	switch {
	case enabled != nil && category != "":
		query = `
			SELECT id, rule_id, name, pattern, pattern_hash, message, severity, enabled, document_id, category, created_at, updated_at
			FROM prevention_rules
			WHERE enabled = $1 AND category = $2
			ORDER BY updated_at DESC LIMIT $3 OFFSET $4
		`
		args = []interface{}{*enabled, category, limit, offset}
	case enabled != nil:
		query = `
			SELECT id, rule_id, name, pattern, pattern_hash, message, severity, enabled, document_id, category, created_at, updated_at
			FROM prevention_rules
			WHERE enabled = $1
			ORDER BY updated_at DESC LIMIT $2 OFFSET $3
		`
		args = []interface{}{*enabled, limit, offset}
	case category != "":
		query = `
			SELECT id, rule_id, name, pattern, pattern_hash, message, severity, enabled, document_id, category, created_at, updated_at
			FROM prevention_rules
			WHERE category = $1
			ORDER BY updated_at DESC LIMIT $2 OFFSET $3
		`
		args = []interface{}{category, limit, offset}
	default:
		query = `
			SELECT id, rule_id, name, pattern, pattern_hash, message, severity, enabled, document_id, category, created_at, updated_at
			FROM prevention_rules
			ORDER BY updated_at DESC LIMIT $1 OFFSET $2
		`
		args = []interface{}{limit, offset}
	}

	rows, err := s.db.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("failed to list rules: %w", err)
	}
	defer rows.Close()

	return scanRules(rows)
}

// GetActiveRules retrieves all enabled rules with caching support
func (s *RuleStore) GetActiveRules(ctx context.Context) ([]models.PreventionRule, error) {
	rows, err := s.db.QueryContext(ctx, `
		SELECT id, rule_id, name, pattern, pattern_hash, message, severity, enabled, document_id, category, created_at, updated_at
		FROM prevention_rules
		WHERE enabled = true
		ORDER BY severity DESC, name ASC
	`)
	if err != nil {
		return nil, fmt.Errorf("failed to get active rules: %w", err)
	}
	defer rows.Close()

	return scanRules(rows)
}

// GetByRuleIDs retrieves multiple rules by their rule IDs in a single query
// This prevents N+1 query problems when fetching rules for a project
func (s *RuleStore) GetByRuleIDs(ctx context.Context, ruleIDs []string) ([]models.PreventionRule, error) {
	if len(ruleIDs) == 0 {
		return []models.PreventionRule{}, nil
	}

	// Use a single parameterized query with ANY for efficient batch retrieval
	rows, err := s.db.QueryContext(ctx, `
		SELECT id, rule_id, name, pattern, pattern_hash, message, severity, enabled, document_id, category, created_at, updated_at
		FROM prevention_rules
		WHERE rule_id = ANY($1) AND enabled = true
		ORDER BY severity DESC, name ASC
	`, ruleIDs)
	if err != nil {
		return nil, fmt.Errorf("failed to get rules by IDs: %w", err)
	}
	defer rows.Close()

	return scanRules(rows)
}

// scanRules is a helper function to scan rule rows and reduce code duplication
func scanRules(rows *sql.Rows) ([]models.PreventionRule, error) {
	var rules []models.PreventionRule
	for rows.Next() {
		var rule models.PreventionRule
		err := rows.Scan(
			&rule.ID, &rule.RuleID, &rule.Name, &rule.Pattern, &rule.PatternHash,
			&rule.Message, &rule.Severity, &rule.Enabled, &rule.DocumentID,
			&rule.Category, &rule.CreatedAt, &rule.UpdatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan rule: %w", err)
		}
		rules = append(rules, rule)
	}

	return rules, rows.Err()
}

// Create inserts a new rule within a transaction
func (s *RuleStore) Create(ctx context.Context, rule *models.PreventionRule) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	err = tx.QueryRowContext(ctx, `
		INSERT INTO prevention_rules (rule_id, name, pattern, pattern_hash, message, severity, enabled, document_id, category)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
		RETURNING id, created_at, updated_at
	`, rule.RuleID, rule.Name, rule.Pattern, rule.PatternHash, rule.Message,
		rule.Severity, rule.Enabled, rule.DocumentID, rule.Category,
	).Scan(&rule.ID, &rule.CreatedAt, &rule.UpdatedAt)
	if err != nil {
		return fmt.Errorf("failed to create rule: %w", err)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// Update updates an existing rule within a transaction
func (s *RuleStore) Update(ctx context.Context, rule *models.PreventionRule) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	result, err := tx.ExecContext(ctx, `
		UPDATE prevention_rules
		SET name = $1, pattern = $2, pattern_hash = $3, message = $4, severity = $5, enabled = $6, document_id = $7, category = $8, updated_at = NOW()
		WHERE id = $9
	`, rule.Name, rule.Pattern, rule.PatternHash, rule.Message, rule.Severity,
		rule.Enabled, rule.DocumentID, rule.Category, rule.ID)
	if err != nil {
		return fmt.Errorf("failed to update rule: %w", err)
	}

	rows, err := result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to get rows affected: %w", err)
	}
	if rows == 0 {
		return fmt.Errorf("rule not found: %s", rule.ID)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// Delete removes a rule within a transaction
func (s *RuleStore) Delete(ctx context.Context, id uuid.UUID) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	result, err := tx.ExecContext(ctx, `DELETE FROM prevention_rules WHERE id = $1`, id)
	if err != nil {
		return fmt.Errorf("failed to delete rule: %w", err)
	}

	rows, err := result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to get rows affected: %w", err)
	}
	if rows == 0 {
		return fmt.Errorf("rule not found: %s", id)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// Count returns the total number of rules, optionally filtered by enabled status and category
func (s *RuleStore) Count(ctx context.Context, enabled *bool, category string) (int, error) {
	var query string
	var args []interface{}

	switch {
	case enabled != nil && category != "":
		query = `SELECT COUNT(*) FROM prevention_rules WHERE enabled = $1 AND category = $2`
		args = []interface{}{*enabled, category}
	case enabled != nil:
		query = `SELECT COUNT(*) FROM prevention_rules WHERE enabled = $1`
		args = []interface{}{*enabled}
	case category != "":
		query = `SELECT COUNT(*) FROM prevention_rules WHERE category = $1`
		args = []interface{}{category}
	default:
		query = `SELECT COUNT(*) FROM prevention_rules`
	}

	var count int
	err := s.db.QueryRowContext(ctx, query, args...).Scan(&count)
	if err != nil {
		return 0, fmt.Errorf("failed to count rules: %w", err)
	}
	return count, nil
}

// Toggle enables/disables a rule within a transaction
func (s *RuleStore) Toggle(ctx context.Context, id uuid.UUID, enabled bool) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	result, err := tx.ExecContext(ctx, `
		UPDATE prevention_rules SET enabled = $1, updated_at = NOW() WHERE id = $2
	`, enabled, id)
	if err != nil {
		return fmt.Errorf("failed to toggle rule: %w", err)
	}

	rows, err := result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to get rows affected: %w", err)
	}
	if rows == 0 {
		return fmt.Errorf("rule not found: %s", id)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}
