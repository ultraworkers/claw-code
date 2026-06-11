package database

import (
	"database/sql"
	"encoding/json"
	"fmt"

	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// UncertaintyStore handles persistence of uncertainty tracking records
type UncertaintyStore struct {
	db *sql.DB
}

// NewUncertaintyStore creates a new uncertainty store
func NewUncertaintyStore(db *sql.DB) *UncertaintyStore {
	return &UncertaintyStore{db: db}
}

// SaveUncertaintyRecord saves a new uncertainty tracking record
func (s *UncertaintyStore) SaveUncertaintyRecord(record *models.UncertaintyRecord) error {
	if !models.IsValidUncertaintyLevel(string(record.UncertaintyLevel)) {
		return fmt.Errorf("invalid uncertainty level: %s", record.UncertaintyLevel)
	}

	query := `
		INSERT INTO uncertainty_tracking
		(id, session_id, task_id, uncertainty_level, decision_made, context_data, escalation_required, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
		RETURNING id`

	var taskID sql.NullString
	if record.TaskID != nil {
		taskID.String = *record.TaskID
		taskID.Valid = true
	}

	err := s.db.QueryRow(
		query,
		record.ID,
		record.SessionID,
		taskID,
		record.UncertaintyLevel,
		record.DecisionMade,
		record.ContextData,
		record.EscalationRequired,
		record.CreatedAt,
	).Scan(&record.ID)

	if err != nil {
		return fmt.Errorf("failed to save uncertainty record: %w", err)
	}

	return nil
}

// GetLatestUncertainty retrieves the most recent uncertainty record for a session
func (s *UncertaintyStore) GetLatestUncertainty(sessionID string) (*models.UncertaintyRecord, error) {
	query := `
		SELECT id, session_id, task_id, uncertainty_level, decision_made, context_data,
		       escalation_required, created_at
		FROM uncertainty_tracking
		WHERE session_id = $1
		ORDER BY created_at DESC
		LIMIT 1`

	row := s.db.QueryRow(query, sessionID)

	var record models.UncertaintyRecord
	var taskID sql.NullString
	var contextDataStr string

	err := row.Scan(
		&record.ID,
		&record.SessionID,
		&taskID,
		&record.UncertaintyLevel,
		&record.DecisionMade,
		&contextDataStr,
		&record.EscalationRequired,
		&record.CreatedAt,
	)

	if err != nil {
		if err == sql.ErrNoRows {
			return nil, nil
		}
		return nil, fmt.Errorf("failed to get latest uncertainty: %w", err)
	}

	if taskID.Valid {
		record.TaskID = &taskID.String
	}

	record.ContextData = json.RawMessage(contextDataStr)

	return &record, nil
}

// GetUncertaintyCountByLevel counts the number of times each uncertainty level was reached
func (s *UncertaintyStore) GetUncertaintyCountByLevel(sessionID string) (map[models.UncertaintyLevel]int, error) {
	query := `
		SELECT uncertainty_level, COUNT(*) as count
		FROM uncertainty_tracking
		WHERE session_id = $1
		GROUP BY uncertainty_level`

	rows, err := s.db.Query(query, sessionID)
	if err != nil {
		return nil, fmt.Errorf("failed to query uncertainty counts: %w", err)
	}
	defer rows.Close()

	counts := make(map[models.UncertaintyLevel]int)
	for rows.Next() {
		var levelStr string
		var count int
		if err := rows.Scan(&levelStr, &count); err != nil {
			return nil, fmt.Errorf("failed to scan uncertainty count: %w", err)
		}
		counts[models.UncertaintyLevel(levelStr)] = count
	}

	return counts, nil
}

// HasReachedEscalationThreshold checks if a session has reached the escalation threshold
func (s *UncertaintyStore) HasReachedEscalationThreshold(sessionID string, threshold int) (bool, error) {
	query := `
		SELECT COUNT(*) as count
		FROM uncertainty_tracking
		WHERE session_id = $1 AND uncertainty_level IN ('critical', 'blocked', 'high')`

	var count int
	err := s.db.QueryRow(query, sessionID).Scan(&count)
	if err != nil {
		return false, fmt.Errorf("failed to check escalation threshold: %w", err)
	}

	return count >= threshold, nil
}

// DeleteUncertaintyRecords removes all uncertainty records for a session (cleanup)
func (s *UncertaintyStore) DeleteUncertaintyRecords(sessionID string) error {
	query := `DELETE FROM uncertainty_tracking WHERE session_id = $1`

	result, err := s.db.Exec(query, sessionID)
	if err != nil {
		return fmt.Errorf("failed to delete uncertainty records: %w", err)
	}

	rowsAffected, err := result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to get rows affected: %w", err)
	}

	if rowsAffected == 0 {
		return fmt.Errorf("no uncertainty records found for session %s", sessionID)
	}

	return nil
}
