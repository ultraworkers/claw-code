package database

import (
	"context"
	"database/sql"
	"fmt"
	"time"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// TaskAttemptStore provides data access for task_attempts table
type TaskAttemptStore struct {
	db *DB
}

// NewTaskAttemptStore creates a new TaskAttemptStore
func NewTaskAttemptStore(db *DB) *TaskAttemptStore {
	return &TaskAttemptStore{db: db}
}

// RecordAttempt creates a new task attempt record
func (s *TaskAttemptStore) RecordAttempt(ctx context.Context, sessionID, taskID string, errorMsg, errorCategory string) (*models.TaskAttempt, error) {
	// Get current attempt count for this session/task
	count, err := s.GetRecentAttemptCount(ctx, sessionID, taskID)
	if err != nil {
		return nil, fmt.Errorf("failed to get attempt count: %w", err)
	}

	attempt := &models.TaskAttempt{
		ID:            uuid.New(),
		SessionID:     sessionID,
		AttemptNumber:   count + 1,
		AttemptedAt:   time.Now(),
		ErrorMessage:  errorMsg,
		ErrorCategory: errorCategory,
		Resolution:    string(models.ResolutionPending),
		CreatedAt:     time.Now(),
	}

	if taskID != "" {
		attempt.TaskID = &taskID
	}

	if err := attempt.Validate(); err != nil {
		return nil, fmt.Errorf("invalid attempt: %w", err)
	}

	query := `
		INSERT INTO task_attempts (id, session_id, task_id, attempt_number, attempted_at, error_message, error_category, resolution, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
	`

	_, err = s.db.ExecContext(ctx, query,
		attempt.ID,
		attempt.SessionID,
		attempt.TaskID,
		attempt.AttemptNumber,
		attempt.AttemptedAt,
		attempt.ErrorMessage,
		attempt.ErrorCategory,
		attempt.Resolution,
		attempt.CreatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("failed to record attempt: %w", err)
	}

	return attempt, nil
}

// GetRecentAttemptCount returns the number of attempts in the last 30 minutes
func (s *TaskAttemptStore) GetRecentAttemptCount(ctx context.Context, sessionID, taskID string) (int, error) {
	query := `
		SELECT COUNT(*) FROM task_attempts
		WHERE session_id = $1
		AND ($2 = '' OR task_id = $2)
		AND attempted_at > NOW() - INTERVAL '30 minutes'
		AND resolution = 'pending'
	`

	var count int
	err := s.db.QueryRowContext(ctx, query, sessionID, taskID).Scan(&count)
	if err != nil {
		return 0, fmt.Errorf("failed to count attempts: %w", err)
	}

	return count, nil
}

// GetPendingAttempts returns all pending attempts for a session/task
func (s *TaskAttemptStore) GetPendingAttempts(ctx context.Context, sessionID, taskID string) ([]*models.TaskAttempt, error) {
	query := `
		SELECT id, session_id, task_id, attempt_number, attempted_at, error_message, error_category, resolution, resolved_at, created_at
		FROM task_attempts
		WHERE session_id = $1
		AND ($2 = '' OR task_id = $2)
		AND resolution = 'pending'
		ORDER BY attempted_at DESC
	`

	rows, err := s.db.QueryContext(ctx, query, sessionID, taskID)
	if err != nil {
		return nil, fmt.Errorf("failed to query attempts: %w", err)
	}
	defer rows.Close()

	return scanTaskAttempts(rows)
}

// ResolveAttempts marks all pending attempts as resolved
func (s *TaskAttemptStore) ResolveAttempts(ctx context.Context, sessionID, taskID string) error {
	query := `
		UPDATE task_attempts
		SET resolution = 'resolved', resolved_at = NOW()
		WHERE session_id = $1
		AND ($2 = '' OR task_id = $2)
		AND resolution = 'pending'
	`

	_, err := s.db.ExecContext(ctx, query, sessionID, taskID)
	if err != nil {
		return fmt.Errorf("failed to resolve attempts: %w", err)
	}

	return nil
}

// MarkEscalated marks attempts as escalated (after 3 strikes)
func (s *TaskAttemptStore) MarkEscalated(ctx context.Context, sessionID, taskID string) error {
	query := `
		UPDATE task_attempts
		SET resolution = 'escalated', resolved_at = NOW()
		WHERE session_id = $1
		AND ($2 = '' OR task_id = $2)
		AND resolution = 'pending'
	`

	_, err := s.db.ExecContext(ctx, query, sessionID, taskID)
	if err != nil {
		return fmt.Errorf("failed to mark escalated: %w", err)
	}

	return nil
}

// ThreeStrikesStatus represents the three strikes status
type ThreeStrikesStatus struct {
	AttemptsCount    int        `json:"attempts_count"`
	MaxAttempts      int        `json:"max_attempts"`
	RemainingStrikes int        `json:"remaining_strikes"`
	ShouldHalt       bool       `json:"should_halt"`
	ShouldEscalate   bool       `json:"should_escalate"`
	LastAttemptAt    *time.Time `json:"last_attempt_at,omitempty"`
}

// GetThreeStrikesStatus returns the three strikes status for a session/task
func (s *TaskAttemptStore) GetThreeStrikesStatus(ctx context.Context, sessionID, taskID string) (*ThreeStrikesStatus, error) {
	count, err := s.GetRecentAttemptCount(ctx, sessionID, taskID)
	if err != nil {
		return nil, err
	}

	// Get last attempt time
	var lastAttemptAt *time.Time
	query := `
		SELECT attempted_at FROM task_attempts
		WHERE session_id = $1
		AND ($2 = '' OR task_id = $2)
		ORDER BY attempted_at DESC
		LIMIT 1
	`
	var t time.Time
	err = s.db.QueryRowContext(ctx, query, sessionID, taskID).Scan(&t)
	if err == nil {
		lastAttemptAt = &t
	}

	const maxAttempts = 3
	remaining := maxAttempts - count
	if remaining < 0 {
		remaining = 0
	}

	return &ThreeStrikesStatus{
		AttemptsCount:    count,
		MaxAttempts:      maxAttempts,
		RemainingStrikes: remaining,
		ShouldHalt:       count >= maxAttempts,
		ShouldEscalate:   count >= maxAttempts,
		LastAttemptAt:    lastAttemptAt,
	}, nil
}

// scanTaskAttempts scans rows into TaskAttempt slice
func scanTaskAttempts(rows *sql.Rows) ([]*models.TaskAttempt, error) {
	var attempts []*models.TaskAttempt

	for rows.Next() {
		var a models.TaskAttempt
		var taskID sql.NullString
		var resolvedAt sql.NullTime

		err := rows.Scan(
			&a.ID,
			&a.SessionID,
			&taskID,
			&a.AttemptNumber,
			&a.AttemptedAt,
			&a.ErrorMessage,
			&a.ErrorCategory,
			&a.Resolution,
			&resolvedAt,
			&a.CreatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan attempt: %w", err)
		}

		if taskID.Valid {
			a.TaskID = &taskID.String
		}
		if resolvedAt.Valid {
			a.ResolvedAt = &resolvedAt.Time
		}

		attempts = append(attempts, &a)
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("rows error: %w", err)
	}

	return attempts, nil
}
