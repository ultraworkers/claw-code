package database

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"time"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// HaltEventStore provides data access for halt_events table
type HaltEventStore struct {
	db *DB
}

// NewHaltEventStore creates a new HaltEventStore
func NewHaltEventStore(db *DB) *HaltEventStore {
	return &HaltEventStore{db: db}
}

// Create creates a new halt event
func (s *HaltEventStore) Create(ctx context.Context, sessionID, haltType, description, severity string, contextData map[string]interface{}) (*models.HaltEvent, error) {
	haltEvent := &models.HaltEvent{
		ID:          uuid.New(),
		SessionID:   sessionID,
		HaltType:    haltType,
		Severity:    severity,
		Description: description,
		TriggeredAt: time.Now(),
		Acknowledged: false,
		Resolution:  string(models.ResolutionPending),
		ContextData: contextData,
		CreatedAt:   time.Now(),
	}

	if err := haltEvent.Validate(); err != nil {
		return nil, fmt.Errorf("invalid halt event: %w", err)
	}

	contextJSON, err := json.Marshal(contextData)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal context data: %w", err)
	}

	query := `
		INSERT INTO halt_events (id, session_id, halt_type, severity, description, triggered_at, acknowledged, resolution, context_data, created_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
	`

	_, err = s.db.ExecContext(ctx, query,
		haltEvent.ID,
		haltEvent.SessionID,
		haltEvent.HaltType,
		haltEvent.Severity,
		haltEvent.Description,
		haltEvent.TriggeredAt,
		haltEvent.Acknowledged,
		haltEvent.Resolution,
		contextJSON,
		haltEvent.CreatedAt,
	)
	if err != nil {
		return nil, fmt.Errorf("failed to create halt event: %w", err)
	}

	return haltEvent, nil
}

// GetByID retrieves a halt event by ID
func (s *HaltEventStore) GetByID(ctx context.Context, id uuid.UUID) (*models.HaltEvent, error) {
	var haltEvent models.HaltEvent
	var contextJSON []byte
	var acknowledgedAt sql.NullTime

	err := s.db.QueryRowContext(ctx, `
		SELECT id, session_id, halt_type, severity, description, triggered_at, acknowledged, acknowledged_at, resolution, context_data, created_at
		FROM halt_events
		WHERE id = $1
	`, id).Scan(
		&haltEvent.ID,
		&haltEvent.SessionID,
		&haltEvent.HaltType,
		&haltEvent.Severity,
		&haltEvent.Description,
		&haltEvent.TriggeredAt,
		&haltEvent.Acknowledged,
		&acknowledgedAt,
		&haltEvent.Resolution,
		&contextJSON,
		&haltEvent.CreatedAt,
	)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, fmt.Errorf("halt event not found: %s", id)
		}
		return nil, fmt.Errorf("failed to get halt event: %w", err)
	}

	if acknowledgedAt.Valid {
		haltEvent.AcknowledgedAt = &acknowledgedAt.Time
	}

	if len(contextJSON) > 0 {
		json.Unmarshal(contextJSON, &haltEvent.ContextData)
	}

	return &haltEvent, nil
}

// GetBySession retrieves all halt events for a session
func (s *HaltEventStore) GetBySession(ctx context.Context, sessionID string) ([]*models.HaltEvent, error) {
	rows, err := s.db.QueryContext(ctx, `
		SELECT id, session_id, halt_type, severity, description, triggered_at, acknowledged, acknowledged_at, resolution, context_data, created_at
		FROM halt_events
		WHERE session_id = $1
		ORDER BY triggered_at DESC
	`, sessionID)
	if err != nil {
		return nil, fmt.Errorf("failed to query halt events: %w", err)
	}
	defer rows.Close()

	return scanHaltEvents(rows)
}

// GetUnacknowledgedBySession retrieves unacknowledged halt events for a session
func (s *HaltEventStore) GetUnacknowledgedBySession(ctx context.Context, sessionID string) ([]*models.HaltEvent, error) {
	rows, err := s.db.QueryContext(ctx, `
		SELECT id, session_id, halt_type, severity, description, triggered_at, acknowledged, acknowledged_at, resolution, context_data, created_at
		FROM halt_events
		WHERE session_id = $1 AND acknowledged = false
		ORDER BY triggered_at DESC
	`, sessionID)
	if err != nil {
		return nil, fmt.Errorf("failed to query unacknowledged halt events: %w", err)
	}
	defer rows.Close()

	return scanHaltEvents(rows)
}

// GetCriticalPending retrieves critical unresolved halt events for a session
func (s *HaltEventStore) GetCriticalPending(ctx context.Context, sessionID string) ([]*models.HaltEvent, error) {
	rows, err := s.db.QueryContext(ctx, `
		SELECT id, session_id, halt_type, severity, description, triggered_at, acknowledged, acknowledged_at, resolution, context_data, created_at
		FROM halt_events
		WHERE session_id = $1 AND severity = 'critical' AND resolution = 'pending'
		ORDER BY triggered_at DESC
	`, sessionID)
	if err != nil {
		return nil, fmt.Errorf("failed to query critical pending halt events: %w", err)
	}
	defer rows.Close()

	return scanHaltEvents(rows)
}

// Acknowledge updates the acknowledgment status of a halt event
func (s *HaltEventStore) Acknowledge(ctx context.Context, id uuid.UUID, resolution string) (*models.HaltEvent, error) {
	// Validate resolution status
	if !models.IsValidResolutionStatus(resolution) {
		return nil, fmt.Errorf("invalid resolution status: %s", resolution)
	}

	acknowledgedAt := time.Now()

	query := `
		UPDATE halt_events
		SET acknowledged = true, acknowledged_at = $1, resolution = $2
		WHERE id = $3
		RETURNING id, session_id, halt_type, severity, description, triggered_at, acknowledged, acknowledged_at, resolution, context_data, created_at
	`

	var haltEvent models.HaltEvent
	var contextJSON []byte

	err := s.db.QueryRowContext(ctx, query, acknowledgedAt, resolution, id).Scan(
		&haltEvent.ID,
		&haltEvent.SessionID,
		&haltEvent.HaltType,
		&haltEvent.Severity,
		&haltEvent.Description,
		&haltEvent.TriggeredAt,
		&haltEvent.Acknowledged,
		&acknowledgedAt,
		&haltEvent.Resolution,
		&contextJSON,
		&haltEvent.CreatedAt,
	)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, fmt.Errorf("halt event not found: %s", id)
		}
		return nil, fmt.Errorf("failed to acknowledge halt event: %w", err)
	}

	haltEvent.AcknowledgedAt = &acknowledgedAt

	if len(contextJSON) > 0 {
		json.Unmarshal(contextJSON, &haltEvent.ContextData)
	}

	return &haltEvent, nil
}

// HaltCheckResult represents the result of checking halt conditions
type HaltCheckResult struct {
	ShouldHalt       bool     `json:"should_halt"`
	HaltReasons      []string `json:"halt_reasons"`
	Severity         string   `json:"severity"`
	RecommendedAction string  `json:"recommended_action"`
}

// scanHaltEvents scans rows into HaltEvent slice
func scanHaltEvents(rows *sql.Rows) ([]*models.HaltEvent, error) {
	var haltEvents []*models.HaltEvent

	for rows.Next() {
		var h models.HaltEvent
		var contextJSON []byte
		var acknowledgedAt sql.NullTime

		err := rows.Scan(
			&h.ID,
			&h.SessionID,
			&h.HaltType,
			&h.Severity,
			&h.Description,
			&h.TriggeredAt,
			&h.Acknowledged,
			&acknowledgedAt,
			&h.Resolution,
			&contextJSON,
			&h.CreatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan halt event: %w", err)
		}

		if acknowledgedAt.Valid {
			h.AcknowledgedAt = &acknowledgedAt.Time
		}

		if len(contextJSON) > 0 {
			json.Unmarshal(contextJSON, &h.ContextData)
		}

		haltEvents = append(haltEvents, &h)
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("rows error: %w", err)
	}

	return haltEvents, nil
}
