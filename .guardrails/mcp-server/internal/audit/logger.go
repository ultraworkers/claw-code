package audit

import (
	"bytes"
	"context"
	"encoding/json"
	"log/slog"
	"sync"
	"time"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/database"
)

// bufferPool provides reusable buffers for JSON encoding
var bufferPool = sync.Pool{
	New: func() interface{} {
		return new(bytes.Buffer)
	},
}

// encoderPool provides reusable JSON encoders
var encoderPool = sync.Pool{
	New: func() interface{} {
		return json.NewEncoder(new(bytes.Buffer))
	},
}

// EventType represents categories of audit events
type EventType string

const (
	EventAuthSuccess    EventType = "auth_success"
	EventAuthFailure    EventType = "auth_failure"
	EventValidation     EventType = "validation"
	EventRuleChange     EventType = "rule_change"
	EventDocChange      EventType = "document_change"
	EventConfigChange   EventType = "config_change"
	EventAccessDenied   EventType = "access_denied"
	EventSessionCreated EventType = "session_created"
	EventSessionExpired EventType = "session_expired"
)

// Severity represents event severity
type Severity string

const (
	SevInfo     Severity = "info"
	SevWarning  Severity = "warning"
	SevCritical Severity = "critical"
)

// Event represents a single audit event
type Event struct {
	ID        string                 `json:"id"`
	Timestamp time.Time              `json:"timestamp"`
	Type      EventType              `json:"type"`
	Severity  Severity               `json:"severity"`
	Actor     string                 `json:"actor"`    // Hashed API key or user ID
	Action    string                 `json:"action"`   // What was done
	Resource  string                 `json:"resource"` // What was affected
	Status    string                 `json:"status"`   // success, failure
	Details   map[string]interface{} `json:"details"`  // Additional context
	ClientIP  string                 `json:"client_ip"`
	UserAgent string                 `json:"user_agent"`
	RequestID string                 `json:"request_id"`
}

// Logger handles audit event recording
type Logger struct {
	backend    chan Event
	done       chan struct{}
	wg         sync.WaitGroup
	auditStore AuditStoreInterface
}

// AuditStoreInterface defines the interface for audit storage
type AuditStoreInterface interface {
	Insert(ctx context.Context, event *database.AuditEvent) error
}

// NewLogger creates an audit logger
func NewLogger(bufferSize int) *Logger {
	l := &Logger{
		backend: make(chan Event, bufferSize),
		done:    make(chan struct{}),
	}
	l.wg.Add(1)
	go l.process()
	return l
}

// NewLoggerWithStore creates an audit logger with database persistence
func NewLoggerWithStore(bufferSize int, store AuditStoreInterface) *Logger {
	l := &Logger{
		backend:    make(chan Event, bufferSize),
		done:       make(chan struct{}),
		auditStore: store,
	}
	l.wg.Add(1)
	go l.process()
	return l
}

// SetStore sets the audit store for database persistence
func (l *Logger) SetStore(store AuditStoreInterface) {
	l.auditStore = store
}

// Stop gracefully shuts down the audit logger
func (l *Logger) Stop() {
	close(l.done)
	l.wg.Wait()
	close(l.backend)
}

// Log records an audit event
func (l *Logger) Log(ctx context.Context, event Event) {
	event.ID = uuid.New().String()
	event.Timestamp = time.Now().UTC()

	// Extract request context
	if reqID := ctx.Value("request_id"); reqID != nil {
		event.RequestID = reqID.(string)
	}

	select {
	case l.backend <- event:
	default:
		// Buffer full - log to stderr and continue
		slog.Error("audit buffer full, dropping event", "type", event.Type)
	}
}

// process writes events to persistent storage
// Uses buffer pooling to reduce allocations during JSON encoding
func (l *Logger) process() {
	defer func() {
		if r := recover(); r != nil {
			slog.Error("Audit logger process panicked, recovering", "panic", r)
			// Restart the process goroutine to ensure audit logging continues
			go l.process()
		}
		l.wg.Done()
	}()
	for {
		select {
		case event, ok := <-l.backend:
			if !ok {
				return
			}
			// Get buffer from pool
			buf := bufferPool.Get().(*bytes.Buffer)
			buf.Reset()

			// Encode event to buffer
			encoder := json.NewEncoder(buf)
			if err := encoder.Encode(event); err != nil {
				slog.Error("Failed to marshal audit event", "error", err)
				bufferPool.Put(buf)
				continue
			}

			// Remove trailing newline from encoder and log
			data := buf.Bytes()
			if len(data) > 0 && data[len(data)-1] == '\n' {
				data = data[:len(data)-1]
			}
			slog.Info("AUDIT", "event", string(data))

			// Return buffer to pool
			bufferPool.Put(buf)

			// Write to database for long-term storage if store is configured
			if l.auditStore != nil {
				dbEvent := &database.AuditEvent{
					ID:        uuid.MustParse(event.ID),
					EventID:   event.ID,
					Timestamp: event.Timestamp,
					EventType: string(event.Type),
					Severity:  string(event.Severity),
					Actor:     event.Actor,
					Action:    event.Action,
					Resource:  event.Resource,
					Status:    event.Status,
					Details:   event.Details,
					ClientIP:  event.ClientIP,
					RequestID: event.RequestID,
					CreatedAt: event.Timestamp,
				}

				ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
				if err := l.auditStore.Insert(ctx, dbEvent); err != nil {
					slog.Error("Failed to persist audit event to database", "error", err, "event_id", event.ID)
				}
				cancel()
			}
		case <-l.done:
			return
		}
	}
}

// LogAuth logs authentication events
func (l *Logger) LogAuth(ctx context.Context, success bool, actor, reason string) {
	eventType := EventAuthSuccess
	severity := SevInfo
	if !success {
		eventType = EventAuthFailure
		severity = SevWarning
	}

	status := "success"
	if !success {
		status = "failure"
	}

	l.Log(ctx, Event{
		Type:     eventType,
		Severity: severity,
		Actor:    actor,
		Action:   "authenticate",
		Status:   status,
		Details:  map[string]interface{}{"reason": reason},
	})
}

// LogValidation logs validation events
func (l *Logger) LogValidation(ctx context.Context, actor, tool string, allowed bool, violations int) {
	status := "allowed"
	if !allowed {
		status = "denied"
	}

	l.Log(ctx, Event{
		Type:     EventValidation,
		Severity: SevInfo,
		Actor:    actor,
		Action:   "validate",
		Resource: tool,
		Status:   status,
		Details: map[string]interface{}{
			"violations": violations,
		},
	})
}

// LogRuleChange logs rule modification events
func (l *Logger) LogRuleChange(ctx context.Context, actor, ruleID, action string) {
	l.Log(ctx, Event{
		Type:     EventRuleChange,
		Severity: SevCritical, // Rule changes are security-critical
		Actor:    actor,
		Action:   action, // create, update, delete, toggle
		Resource: ruleID,
		Status:   "success",
	})
}

// LogDocChange logs document modification events
func (l *Logger) LogDocChange(ctx context.Context, actor, docSlug, action string) {
	l.Log(ctx, Event{
		Type:     EventDocChange,
		Severity: SevInfo,
		Actor:    actor,
		Action:   action,
		Resource: docSlug,
		Status:   "success",
	})
}

// LogSession logs session lifecycle events
func (l *Logger) LogSession(ctx context.Context, eventType EventType, token, projectSlug string) {
	l.Log(ctx, Event{
		Type:     eventType,
		Severity: SevInfo,
		Actor:    "system",
		Action:   string(eventType),
		Resource: projectSlug,
		Details: map[string]interface{}{
			"session_hash": hashToken(token),
		},
	})
}

// hashToken creates a short hash for logging
func hashToken(token string) string {
	if len(token) < 8 {
		return "****"
	}
	return token[:4] + "****" + token[len(token)-4:]
}
