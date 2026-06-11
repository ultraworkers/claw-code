package database

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"regexp"
	"strings"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// Search query constants
const (
	maxSearchQueryLength = 200
)

// DocumentStore handles document database operations
type DocumentStore struct {
	db *DB
}

// NewDocumentStore creates a new document store
func NewDocumentStore(db *DB) *DocumentStore {
	return &DocumentStore{db: db}
}

// GetByID retrieves a document by ID
func (s *DocumentStore) GetByID(ctx context.Context, id uuid.UUID) (*models.Document, error) {
	var doc models.Document
	err := s.db.QueryRowContext(ctx, `
		SELECT id, slug, title, content, category, path, version, metadata, created_at, updated_at
		FROM documents
		WHERE id = $1
	`, id).Scan(
		&doc.ID, &doc.Slug, &doc.Title, &doc.Content, &doc.Category,
		&doc.Path, &doc.Version, &doc.Metadata, &doc.CreatedAt, &doc.UpdatedAt,
	)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("document not found: %s", id)
		}
		return nil, fmt.Errorf("failed to get document: %w", err)
	}
	return &doc, nil
}

// GetBySlug retrieves a document by slug
func (s *DocumentStore) GetBySlug(ctx context.Context, slug string) (*models.Document, error) {
	var doc models.Document
	err := s.db.QueryRowContext(ctx, `
		SELECT id, slug, title, content, category, path, version, metadata, created_at, updated_at
		FROM documents
		WHERE slug = $1
	`, slug).Scan(
		&doc.ID, &doc.Slug, &doc.Title, &doc.Content, &doc.Category,
		&doc.Path, &doc.Version, &doc.Metadata, &doc.CreatedAt, &doc.UpdatedAt,
	)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("document not found: %s", slug)
		}
		return nil, fmt.Errorf("failed to get document: %w", err)
	}
	return &doc, nil
}

// List retrieves documents with pagination
func (s *DocumentStore) List(ctx context.Context, category string, limit, offset int) ([]models.Document, error) {
	// Build query with proper parameterization to prevent SQL injection
	var query string
	var args []interface{}

	if category != "" {
		query = `
			SELECT id, slug, title, content, category, path, version, metadata, created_at, updated_at
			FROM documents
			WHERE category = $1
			ORDER BY updated_at DESC LIMIT $2 OFFSET $3
		`
		args = []interface{}{category, limit, offset}
	} else {
		query = `
			SELECT id, slug, title, content, category, path, version, metadata, created_at, updated_at
			FROM documents
			ORDER BY updated_at DESC LIMIT $1 OFFSET $2
		`
		args = []interface{}{limit, offset}
	}

	rows, err := s.db.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("failed to list documents: %w", err)
	}
	defer rows.Close()

	var docs []models.Document
	for rows.Next() {
		var doc models.Document
		err := rows.Scan(
			&doc.ID, &doc.Slug, &doc.Title, &doc.Content, &doc.Category,
			&doc.Path, &doc.Version, &doc.Metadata, &doc.CreatedAt, &doc.UpdatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan document: %w", err)
		}
		docs = append(docs, doc)
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("error iterating documents: %w", err)
	}

	return docs, nil
}

// Search performs full-text search on documents
func (s *DocumentStore) Search(ctx context.Context, query string, limit int) ([]models.Document, error) {
	// Validate and sanitize query first
	safeQuery, err := sanitizeSearchQuery(query)
	if err != nil {
		return nil, fmt.Errorf("invalid search query: %w", err)
	}

	// Use a transaction for consistent search results
	tx, err := s.db.BeginTx(ctx, &sql.TxOptions{ReadOnly: true})
	if err != nil {
		return nil, fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	rows, err := tx.QueryContext(ctx, `
		SELECT id, slug, title, content, category, path, version, metadata, created_at, updated_at
		FROM documents
		WHERE search_vector @@ plainto_tsquery('english', $1)
		ORDER BY ts_rank(search_vector, plainto_tsquery('english', $1)) DESC
		LIMIT $2
	`, safeQuery, limit)
	if err != nil {
		return nil, fmt.Errorf("failed to search documents: %w", err)
	}
	defer rows.Close()

	var docs []models.Document
	for rows.Next() {
		var doc models.Document
		err := rows.Scan(
			&doc.ID, &doc.Slug, &doc.Title, &doc.Content, &doc.Category,
			&doc.Path, &doc.Version, &doc.Metadata, &doc.CreatedAt, &doc.UpdatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan document: %w", err)
		}
		docs = append(docs, doc)
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("error iterating documents: %w", err)
	}

	if err := tx.Commit(); err != nil {
		return nil, fmt.Errorf("failed to commit transaction: %w", err)
	}

	return docs, nil
}

// Create inserts a new document within a transaction
func (s *DocumentStore) Create(ctx context.Context, doc *models.Document) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	err = tx.QueryRowContext(ctx, `
		INSERT INTO documents (slug, title, content, category, path, version, metadata)
		VALUES ($1, $2, $3, $4, $5, $6, $7)
		RETURNING id, created_at, updated_at
	`, doc.Slug, doc.Title, doc.Content, doc.Category, doc.Path, doc.Version, doc.Metadata,
	).Scan(&doc.ID, &doc.CreatedAt, &doc.UpdatedAt)
	if err != nil {
		return fmt.Errorf("failed to create document: %w", err)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// Update updates an existing document within a transaction
func (s *DocumentStore) Update(ctx context.Context, doc *models.Document) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	result, err := tx.ExecContext(ctx, `
		UPDATE documents
		SET title = $1, content = $2, category = $3, path = $4, version = version + 1, metadata = $5, updated_at = NOW()
		WHERE id = $6
	`, doc.Title, doc.Content, doc.Category, doc.Path, doc.Metadata, doc.ID)
	if err != nil {
		return fmt.Errorf("failed to update document: %w", err)
	}

	rows, err := result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to get rows affected: %w", err)
	}
	if rows == 0 {
		return fmt.Errorf("document not found: %s", doc.ID)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// Delete removes a document within a transaction
func (s *DocumentStore) Delete(ctx context.Context, id uuid.UUID) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	result, err := tx.ExecContext(ctx, `DELETE FROM documents WHERE id = $1`, id)
	if err != nil {
		return fmt.Errorf("failed to delete document: %w", err)
	}

	rows, err := result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to get rows affected: %w", err)
	}
	if rows == 0 {
		return fmt.Errorf("document not found: %s", id)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// Count returns the total number of documents, optionally filtered by category
func (s *DocumentStore) Count(ctx context.Context, category string) (int, error) {
	var query string
	var args []interface{}

	if category != "" {
		query = `SELECT COUNT(*) FROM documents WHERE category = $1`
		args = []interface{}{category}
	} else {
		query = `SELECT COUNT(*) FROM documents`
	}

	var count int
	err := s.db.QueryRowContext(ctx, query, args...).Scan(&count)
	if err != nil {
		return 0, fmt.Errorf("failed to count documents: %w", err)
	}
	return count, nil
}

// sanitizeSearchQuery validates and sanitizes search queries
func sanitizeSearchQuery(query string) (string, error) {
	// Limit length
	if len(query) > maxSearchQueryLength {
		return "", fmt.Errorf("query too long (max %d chars)", maxSearchQueryLength)
	}

	// Remove dangerous characters - only allow safe FTS operators
	// Allow: alphanumeric, spaces, - (negation), * (prefix), " (phrase), & | (AND/OR)
	safe := regexp.MustCompile(`[^a-zA-Z0-9\s\-\*"&\|]`)
	cleaned := safe.ReplaceAllString(query, "")

	// Prevent FTS operator injection
	if strings.Count(cleaned, "(") != strings.Count(cleaned, ")") {
		return "", fmt.Errorf("mismatched parentheses")
	}

	return cleaned, nil
}
