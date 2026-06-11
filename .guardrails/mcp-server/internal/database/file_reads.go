package database

import (
	"context"
	"database/sql"
	"errors"
	"fmt"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// FileReadStore handles file read tracking database operations
type FileReadStore struct {
	db *DB
}

// NewFileReadStore creates a new file read store
func NewFileReadStore(db *DB) *FileReadStore {
	return &FileReadStore{db: db}
}

// Create inserts a new file read record
// Uses ON CONFLICT to update read_at and content_hash if the session_id+file_path already exists
func (s *FileReadStore) Create(ctx context.Context, fr *models.FileRead) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	err = tx.QueryRowContext(ctx, `
		INSERT INTO file_reads (session_id, file_path, read_at, content_hash)
		VALUES ($1, $2, $3, $4)
		ON CONFLICT (session_id, file_path) DO UPDATE
		SET read_at = EXCLUDED.read_at,
		    content_hash = EXCLUDED.content_hash
		RETURNING id, created_at
	`, fr.SessionID, fr.FilePath, fr.ReadAt, fr.ContentHash,
	).Scan(&fr.ID, &fr.CreatedAt)
	if err != nil {
		return fmt.Errorf("failed to create file read record: %w", err)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// GetBySessionAndPath retrieves a file read record by session ID and file path
func (s *FileReadStore) GetBySessionAndPath(ctx context.Context, sessionID, filePath string) (*models.FileRead, error) {
	var fr models.FileRead
	err := s.db.QueryRowContext(ctx, `
		SELECT id, session_id, file_path, read_at, content_hash, created_at
		FROM file_reads
		WHERE session_id = $1 AND file_path = $2
	`, sessionID, filePath).Scan(
		&fr.ID, &fr.SessionID, &fr.FilePath, &fr.ReadAt,
		&fr.ContentHash, &fr.CreatedAt,
	)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("file read record not found for session %s and path %s", sessionID, filePath)
		}
		return nil, fmt.Errorf("failed to get file read record: %w", err)
	}
	return &fr, nil
}

// ListBySession retrieves all file read records for a session, ordered by read_at DESC
func (s *FileReadStore) ListBySession(ctx context.Context, sessionID string, limit, offset int) ([]models.FileRead, error) {
	rows, err := s.db.QueryContext(ctx, `
		SELECT id, session_id, file_path, read_at, content_hash, created_at
		FROM file_reads
		WHERE session_id = $1
		ORDER BY read_at DESC
		LIMIT $2 OFFSET $3
	`, sessionID, limit, offset)
	if err != nil {
		return nil, fmt.Errorf("failed to list file read records: %w", err)
	}
	defer rows.Close()

	return scanFileReads(rows)
}

// DeleteBySession removes all file read records for a session
func (s *FileReadStore) DeleteBySession(ctx context.Context, sessionID string) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	result, err := tx.ExecContext(ctx, `DELETE FROM file_reads WHERE session_id = $1`, sessionID)
	if err != nil {
		return fmt.Errorf("failed to delete file read records: %w", err)
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

// Delete removes a specific file read record
func (s *FileReadStore) Delete(ctx context.Context, id uuid.UUID) error {
	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	result, err := tx.ExecContext(ctx, `DELETE FROM file_reads WHERE id = $1`, id)
	if err != nil {
		return fmt.Errorf("failed to delete file read record: %w", err)
	}

	rows, err := result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to get rows affected: %w", err)
	}
	if rows == 0 {
		return fmt.Errorf("file read record not found: %s", id)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// CreateWithStrings creates a new file read record with just sessionID and filePath
// Sets read_at to NOW() and returns nil on success
func (s *FileReadStore) CreateWithStrings(ctx context.Context, sessionID, filePath string) error {
	if sessionID == "" {
		return fmt.Errorf("session_id is required")
	}
	if filePath == "" {
		return fmt.Errorf("file_path is required")
	}

	tx, err := s.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to begin transaction: %w", err)
	}
	defer tx.Rollback()

	_, err = tx.ExecContext(ctx, `
		INSERT INTO file_reads (session_id, file_path, read_at)
		VALUES ($1, $2, NOW())
		ON CONFLICT (session_id, file_path) DO UPDATE
		SET read_at = EXCLUDED.read_at
	`, sessionID, filePath)
	if err != nil {
		return fmt.Errorf("failed to create file read record: %w", err)
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit transaction: %w", err)
	}

	return nil
}

// CountBySession returns the number of file reads for a session
func (s *FileReadStore) CountBySession(ctx context.Context, sessionID string) (int, error) {
	var count int
	err := s.db.QueryRowContext(ctx, `
		SELECT COUNT(*) FROM file_reads WHERE session_id = $1
	`, sessionID).Scan(&count)
	if err != nil {
		return 0, fmt.Errorf("failed to count file read records: %w", err)
	}
	return count, nil
}

// Exists checks if a file read record exists for the given session and path
func (s *FileReadStore) Exists(ctx context.Context, sessionID, filePath string) (bool, error) {
	var exists bool
	err := s.db.QueryRowContext(ctx, `
		SELECT EXISTS(
			SELECT 1 FROM file_reads
			WHERE session_id = $1 AND file_path = $2
		)
	`, sessionID, filePath).Scan(&exists)
	if err != nil {
		return false, fmt.Errorf("failed to check file read existence: %w", err)
	}
	return exists, nil
}

// scanFileReads is a helper function to scan file read rows
func scanFileReads(rows *sql.Rows) ([]models.FileRead, error) {
	var records []models.FileRead
	for rows.Next() {
		var fr models.FileRead
		err := rows.Scan(
			&fr.ID, &fr.SessionID, &fr.FilePath, &fr.ReadAt,
			&fr.ContentHash, &fr.CreatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan file read record: %w", err)
		}
		records = append(records, fr)
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("error iterating file read records: %w", err)
	}

	return records, nil
}
