package database

import (
	"context"
	"database/sql"
	"fmt"
	"crypto/sha256"
	"encoding/hex"
	"strings"

	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// FixVerificationStore provides data access for fix_verification_tracking table
type FixVerificationStore struct {
	db *DB
}

// NewFixVerificationStore creates a new FixVerificationStore
func NewFixVerificationStore(db *DB) *FixVerificationStore {
	return &FixVerificationStore{db: db}
}

// Create inserts a new fix verification record
func (s *FixVerificationStore) Create(ctx context.Context, verification *models.FixVerification) error {
	query := `
		INSERT INTO fix_verification_tracking (
			id, session_id, failure_id, fix_hash, file_path, fix_content, fix_type, verified_at, verification_status, created_at
		)
		VALUES (
			gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, CURRENT_TIMESTAMP
		)
		RETURNING id, created_at
	`

	err := s.db.QueryRowContext(ctx, query,
		verification.SessionID,
		verification.FailureID,
		verification.FixHash,
		verification.FilePath,
		verification.FixContent,
		verification.FixType,
		verification.VerifiedAt,
		verification.VerificationStatus,
	).Scan(&verification.ID, &verification.CreatedAt)

	if err != nil {
		return fmt.Errorf("failed to create fix verification: %w", err)
	}

	return nil
}

// UpdateVerificationStatus updates the verification status and verified_at timestamp
func (s *FixVerificationStore) UpdateVerificationStatus(ctx context.Context, sessionID, failureID string, status models.VerificationStatus) error {
	query := `
		UPDATE fix_verification_tracking
		SET verification_status = $1, verified_at = CURRENT_TIMESTAMP
		WHERE session_id = $2 AND failure_id = $3
	`

	_, err := s.db.ExecContext(ctx, query, status, sessionID, failureID)
	if err != nil {
		return fmt.Errorf("failed to update verification status: %w", err)
	}

	return nil
}

// GetBySessionAndFailure retrieves a fix verification by session and failure
func (s *FixVerificationStore) GetBySessionAndFailure(ctx context.Context, sessionID, failureID string) (*models.FixVerification, error) {
	query := `
		SELECT id, session_id, failure_id, fix_hash, file_path, fix_content, fix_type, verified_at, verification_status, created_at
		FROM fix_verification_tracking
		WHERE session_id = $1 AND failure_id = $2
	`

	var v models.FixVerification
	err := s.db.QueryRowContext(ctx, query, sessionID, failureID).Scan(
		&v.ID, &v.SessionID, &v.FailureID, &v.FixHash, &v.FilePath,
		&v.FixContent, &v.FixType, &v.VerifiedAt, &v.VerificationStatus, &v.CreatedAt,
	)

	if err != nil {
		if err == sql.ErrNoRows {
			return nil, nil
		}
		return nil, fmt.Errorf("failed to get fix verification: %w", err)
	}

	return &v, nil
}

// GetBySessionAndFile retrieves fix verifications for a specific file in a session
func (s *FixVerificationStore) GetBySessionAndFile(ctx context.Context, sessionID, filePath string) ([]models.FixVerification, error) {
	query := `
		SELECT id, session_id, failure_id, fix_hash, file_path, fix_content, fix_type, verified_at, verification_status, created_at
		FROM fix_verification_tracking
		WHERE session_id = $1 AND file_path = $2
		ORDER BY created_at DESC
	`

	rows, err := s.db.QueryContext(ctx, query, sessionID, filePath)
	if err != nil {
		return nil, fmt.Errorf("failed to query fix verifications: %w", err)
	}
	defer rows.Close()

	var verifications []models.FixVerification
	for rows.Next() {
		var v models.FixVerification
		err := rows.Scan(
			&v.ID, &v.SessionID, &v.FailureID, &v.FixHash, &v.FilePath,
			&v.FixContent, &v.FixType, &v.VerifiedAt, &v.VerificationStatus, &v.CreatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan fix verification: %w", err)
		}
		verifications = append(verifications, v)
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("error iterating fix verifications: %w", err)
	}

	return verifications, nil
}

// GetActiveBySession retrieves all active fix verifications for a session
func (s *FixVerificationStore) GetActiveBySession(ctx context.Context, sessionID string) ([]models.FixVerification, error) {
	query := `
		SELECT id, session_id, failure_id, fix_hash, file_path, fix_content, fix_type, verified_at, verification_status, created_at
		FROM fix_verification_tracking
		WHERE session_id = $1
		ORDER BY failure_id
	`

	rows, err := s.db.QueryContext(ctx, query, sessionID)
	if err != nil {
		return nil, fmt.Errorf("failed to query fix verifications: %w", err)
	}
	defer rows.Close()

	var verifications []models.FixVerification
	for rows.Next() {
		var v models.FixVerification
		err := rows.Scan(
			&v.ID, &v.SessionID, &v.FailureID, &v.FixHash, &v.FilePath,
			&v.FixContent, &v.FixType, &v.VerifiedAt, &v.VerificationStatus, &v.CreatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan fix verification: %w", err)
		}
		verifications = append(verifications, v)
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("error iterating fix verifications: %w", err)
	}

	return verifications, nil
}

// ComputeFixHash computes the SHA256 hash of fix content
func ComputeFixHash(content string) string {
	hash := sha256.Sum256([]byte(content))
	return hex.EncodeToString(hash[:])
}

// GetOrCreate retrieves an existing fix verification or creates a new one
func (s *FixVerificationStore) GetOrCreate(ctx context.Context, sessionID, failureID, filePath, fixContent string, fixType models.FixType) (*models.FixVerification, error) {
	// Try to get existing verification
	existing, err := s.GetBySessionAndFailure(ctx, sessionID, failureID)
	if err != nil && err != sql.ErrNoRows {
		return nil, fmt.Errorf("failed to get existing verification: %w", err)
	}

	if existing != nil {
		return existing, nil
	}

	// Create new verification
	fixHash := ComputeFixHash(fixContent)
	verification := &models.FixVerification{
		SessionID:          sessionID,
		FailureID:          failureID,
		FixHash:            fixHash,
		FilePath:           filePath,
		FixContent:         fixContent,
		FixType:            fixType,
		VerificationStatus: models.StatusConfirmed, // Default status
	}

	if err := s.Create(ctx, verification); err != nil {
		return nil, fmt.Errorf("failed to create fix verification: %w", err)
	}

	return verification, nil
}

// VerifyFixContent verifies if the fix content matches the stored hash
func (s *FixVerificationStore) VerifyFixContent(ctx context.Context, fixedContent string, verification *models.FixVerification) (models.VerificationStatus, string) {
	currentHash := ComputeFixHash(fixedContent)

	if currentHash == verification.FixHash {
		return models.StatusConfirmed, "Fix still intact - hash matches"
	}

	// For regex fixes, check if the regex pattern is still present
	if verification.FixType == models.FixTypeRegex {
		pattern := strings.TrimSpace(verification.FixContent)
		if strings.Contains(fixedContent, pattern) {
			return models.StatusConfirmed, "Regex pattern still present"
		} else {
			return models.StatusModified, "Regex pattern no longer present"
		}
	}

	// For code changes, check similarity
	if strings.Contains(fixedContent, verification.FixContent) {
		return models.StatusModified, "Fix content modified but similar content found"
	}

	return models.StatusRemoved, "Fix content no longer present"
}

// GetFixesByFilePatterns retrieves active fix verifications matching file patterns
func (s *FixVerificationStore) GetFixesByFilePatterns(ctx context.Context, sessionID string, filePatterns []string) ([]models.FixVerification, error) {
	// Build query with pattern matching
	query := `
		SELECT id, session_id, failure_id, fix_hash, file_path, fix_content, fix_type, verified_at, verification_status, created_at
		FROM fix_verification_tracking
		WHERE session_id = $1
		AND (
	`

	// Add pattern conditions
	conditions := make([]string, 0, len(filePatterns))
	args := make([]interface{}, 0, len(filePatterns)+1)
	args = append(args, sessionID)

	for i, pattern := range filePatterns {
		conditions = append(conditions, fmt.Sprintf("file_path LIKE $%d", i+2))
		args = append(args, "%"+pattern+"%")
	}

	if len(conditions) == 0 {
		return []models.FixVerification{}, nil
	}

	query += strings.Join(conditions, " OR ")
	query += ") ORDER BY failure_id"

	rows, err := s.db.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("failed to query fix verifications: %w", err)
	}
	defer rows.Close()

	var verifications []models.FixVerification
	for rows.Next() {
		var v models.FixVerification
		err := rows.Scan(
			&v.ID, &v.SessionID, &v.FailureID, &v.FixHash, &v.FilePath,
			&v.FixContent, &v.FixType, &v.VerifiedAt, &v.VerificationStatus, &v.CreatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan fix verification: %w", err)
		}
		verifications = append(verifications, v)
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("error iterating fix verifications: %w", err)
	}

	return verifications, nil
}
