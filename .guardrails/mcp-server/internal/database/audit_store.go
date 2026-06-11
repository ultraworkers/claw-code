package database

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/google/uuid"
)

// AuditEvent represents an audit log event
type AuditEvent struct {
	ID        uuid.UUID              `json:"id"`
	EventID   string                 `json:"event_id"`
	Timestamp time.Time              `json:"timestamp"`
	EventType string                 `json:"event_type"`
	Severity  string                 `json:"severity"`
	Actor     string                 `json:"actor"`
	Action    string                 `json:"action"`
	Resource  string                 `json:"resource"`
	Status    string                 `json:"status"`
	Details   map[string]interface{} `json:"details"`
	ClientIP  string                 `json:"client_ip"`
	RequestID string                 `json:"request_id"`
	CreatedAt time.Time              `json:"created_at"`
}

// AuditStore handles audit log database operations
type AuditStore struct {
	db *DB
}

// NewAuditStore creates a new audit store
func NewAuditStore(db *DB) *AuditStore {
	return &AuditStore{db: db}
}

// Insert adds a new audit event to the database
func (s *AuditStore) Insert(ctx context.Context, event *AuditEvent) error {
	detailsJSON, err := json.Marshal(event.Details)
	if err != nil {
		return fmt.Errorf("failed to marshal details: %w", err)
	}

	_, err = s.db.ExecContext(ctx, `
		INSERT INTO audit_log (id, event_id, timestamp, event_type, severity, actor, action, resource, status, details, client_ip, request_id, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
	`,
		event.ID,
		event.EventID,
		event.Timestamp,
		event.EventType,
		event.Severity,
		event.Actor,
		event.Action,
		event.Resource,
		event.Status,
		detailsJSON,
		event.ClientIP,
		event.RequestID,
		event.CreatedAt,
	)

	if err != nil {
		return fmt.Errorf("failed to insert audit event: %w", err)
	}

	return nil
}

// List retrieves audit events with pagination and filtering
func (s *AuditStore) List(ctx context.Context, eventType string, actor string, limit, offset int) ([]AuditEvent, error) {
	query := `
		SELECT id, event_id, timestamp, event_type, severity, actor, action, resource, status, details, client_ip, request_id, created_at
		FROM audit_log
		WHERE 1=1
	`
	var args []interface{}
	argCount := 0

	if eventType != "" {
		argCount++
		query += fmt.Sprintf(" AND event_type = $%d", argCount)
		args = append(args, eventType)
	}

	if actor != "" {
		argCount++
		query += fmt.Sprintf(" AND actor = $%d", argCount)
		args = append(args, actor)
	}

	argCount++
	query += fmt.Sprintf(" ORDER BY timestamp DESC LIMIT $%d", argCount)
	args = append(args, limit)

	argCount++
	query += fmt.Sprintf(" OFFSET $%d", argCount)
	args = append(args, offset)

	rows, err := s.db.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("failed to list audit events: %w", err)
	}
	defer rows.Close()

	var events []AuditEvent
	for rows.Next() {
		var event AuditEvent
		var detailsJSON []byte
		err := rows.Scan(
			&event.ID,
			&event.EventID,
			&event.Timestamp,
			&event.EventType,
			&event.Severity,
			&event.Actor,
			&event.Action,
			&event.Resource,
			&event.Status,
			&detailsJSON,
			&event.ClientIP,
			&event.RequestID,
			&event.CreatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan audit event: %w", err)
		}

		// Unmarshal details
		if len(detailsJSON) > 0 {
			json.Unmarshal(detailsJSON, &event.Details)
		}

		events = append(events, event)
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("error iterating audit events: %w", err)
	}

	return events, nil
}

// Count returns the total number of audit events matching the filters
func (s *AuditStore) Count(ctx context.Context, eventType string, actor string) (int, error) {
	query := `SELECT COUNT(*) FROM audit_log WHERE 1=1`
	var args []interface{}
	argCount := 0

	if eventType != "" {
		argCount++
		query += fmt.Sprintf(" AND event_type = $%d", argCount)
		args = append(args, eventType)
	}

	if actor != "" {
		argCount++
		query += fmt.Sprintf(" AND actor = $%d", argCount)
		args = append(args, actor)
	}

	var count int
	err := s.db.QueryRowContext(ctx, query, args...).Scan(&count)
	if err != nil {
		return 0, fmt.Errorf("failed to count audit events: %w", err)
	}

	return count, nil
}

// InsertAsync inserts an audit event asynchronously without blocking
func (s *AuditStore) InsertAsync(event *AuditEvent) {
	go func() {
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()

		if err := s.Insert(ctx, event); err != nil {
			// Log error but don't fail - audit logging should not break the application
			fmt.Printf("Failed to insert audit event: %v\n", err)
		}
	}()
}

// GetRecent retrieves recent audit events
func (s *AuditStore) GetRecent(ctx context.Context, limit int) ([]AuditEvent, error) {
	return s.List(ctx, "", "", limit, 0)
}
