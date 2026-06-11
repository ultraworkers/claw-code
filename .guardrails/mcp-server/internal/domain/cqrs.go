package domain

import (
	"context"
	"log/slog"
	"sync"
	"time"

	"github.com/google/uuid"
)

// ============================================================================
// COMMAND SIDE (Write Operations)
// ============================================================================

// CreateRuleCommand creates a new prevention rule
type CreateRuleCommand struct {
	RuleID     string   `json:"rule_id"`
	Name       string   `json:"name"`
	Pattern    string   `json:"pattern"`
	Message    string   `json:"message"`
	Severity   Severity `json:"severity"`
	Category   string   `json:"category"`
	DocumentID string   `json:"document_id,omitempty"`
}

// CreateRuleHandler handles CreateRuleCommand
type CreateRuleHandler struct {
	repo    RuleRepository
	bus     EventBus
	matcher PatternMatcher
}

func NewCreateRuleHandler(repo RuleRepository, bus EventBus, matcher PatternMatcher) *CreateRuleHandler {
	return &CreateRuleHandler{
		repo:    repo,
		bus:     bus,
		matcher: matcher,
	}
}

func (h *CreateRuleHandler) Handle(ctx context.Context, cmd CreateRuleCommand) (*PreventionRule, error) {
	if err := h.matcher.ValidatePattern(cmd.Pattern); err != nil {
		return nil, err
	}

	rule := &PreventionRule{
		ID:       uuid.New(),
		RuleID:   cmd.RuleID,
		Name:     cmd.Name,
		Pattern:  cmd.Pattern,
		Message:  cmd.Message,
		Severity: cmd.Severity,
		Category: cmd.Category,
		Enabled:  true,
	}

	if cmd.DocumentID != "" {
		if docID, err := uuid.Parse(cmd.DocumentID); err == nil {
			rule.DocumentID = docID
		}
	}

	if err := h.repo.Create(ctx, rule); err != nil {
		return nil, err
	}

	h.bus.Publish(ctx, Event{
		Type:      EventRuleCreated,
		Payload:   rule,
		Timestamp: time.Now(),
	})

	return rule, nil
}

// UpdateRuleCommand updates an existing rule
type UpdateRuleCommand struct {
	ID       uuid.UUID `json:"id"`
	Name     string    `json:"name"`
	Pattern  string    `json:"pattern"`
	Message  string    `json:"message"`
	Severity Severity  `json:"severity"`
	Enabled  bool      `json:"enabled"`
	Category string    `json:"category"`
}

// UpdateRuleHandler handles UpdateRuleCommand
type UpdateRuleHandler struct {
	repo    RuleRepository
	bus     EventBus
	matcher PatternMatcher
}

func NewUpdateRuleHandler(repo RuleRepository, bus EventBus, matcher PatternMatcher) *UpdateRuleHandler {
	return &UpdateRuleHandler{
		repo:    repo,
		bus:     bus,
		matcher: matcher,
	}
}

func (h *UpdateRuleHandler) Handle(ctx context.Context, cmd UpdateRuleCommand) (*PreventionRule, error) {
	if err := h.matcher.ValidatePattern(cmd.Pattern); err != nil {
		return nil, err
	}

	rule, err := h.repo.GetByID(ctx, cmd.ID)
	if err != nil {
		return nil, err
	}

	rule.Name = cmd.Name
	rule.Pattern = cmd.Pattern
	rule.Message = cmd.Message
	rule.Severity = cmd.Severity
	rule.Enabled = cmd.Enabled
	rule.Category = cmd.Category
	rule.UpdatedAt = time.Now()

	if err := h.repo.Update(ctx, rule); err != nil {
		return nil, err
	}

	h.bus.Publish(ctx, Event{
		Type:      EventRuleUpdated,
		Payload:   rule,
		Timestamp: time.Now(),
	})

	return rule, nil
}

// ToggleRuleCommand enables or disables a rule
type ToggleRuleCommand struct {
	ID      uuid.UUID `json:"id"`
	Enabled bool      `json:"enabled"`
}

// ToggleRuleHandler handles ToggleRuleCommand
type ToggleRuleHandler struct {
	repo RuleRepository
	bus  EventBus
}

func NewToggleRuleHandler(repo RuleRepository, bus EventBus) *ToggleRuleHandler {
	return &ToggleRuleHandler{repo: repo, bus: bus}
}

func (h *ToggleRuleHandler) Handle(ctx context.Context, cmd ToggleRuleCommand) error {
	if err := h.repo.Toggle(ctx, cmd.ID, cmd.Enabled); err != nil {
		return err
	}

	h.bus.Publish(ctx, Event{
		Type:      EventRuleToggled,
		Payload:   cmd,
		Timestamp: time.Now(),
	})

	return nil
}

// LogViolationCommand logs a rule violation
type LogViolationCommand struct {
	Violation Violation `json:"violation"`
	SessionID string    `json:"session_id"`
}

// LogViolationHandler handles LogViolationCommand
type LogViolationHandler struct {
	auditLogger AuditLogger
}

func NewLogViolationHandler(auditLogger AuditLogger) *LogViolationHandler {
	return &LogViolationHandler{auditLogger: auditLogger}
}

func (h *LogViolationHandler) Handle(ctx context.Context, cmd LogViolationCommand) error {
	return h.auditLogger.LogViolation(ctx, cmd.Violation, cmd.SessionID)
}

// ============================================================================
// QUERY SIDE (Read Operations)
// ============================================================================

// EvaluateCommandQuery queries guardrail evaluation (read-optimized with caching)
type EvaluateCommandQuery struct {
	Command    string   `json:"command"`
	Categories []string `json:"categories,omitempty"`
}

// EvaluateCommandHandler handles EvaluateCommandQuery
type EvaluateCommandHandler struct {
	guardrailSvc GuardrailService
	cache        CachePort
}

func NewEvaluateCommandHandler(guardrailSvc GuardrailService, cache CachePort) *EvaluateCommandHandler {
	return &EvaluateCommandHandler{
		guardrailSvc: guardrailSvc,
		cache:        cache,
	}
}

func (h *EvaluateCommandHandler) Handle(ctx context.Context, q EvaluateCommandQuery) (*ValidationResult, error) {
	violations, err := h.guardrailSvc.EvaluateCommand(ctx, q.Command)
	if err != nil {
		return nil, err
	}

	return NewValidationResult(violations), nil
}

// EvaluateGitQuery queries git command evaluation
type EvaluateGitQuery struct {
	Command string `json:"command"`
}

// EvaluateGitHandler handles EvaluateGitQuery
type EvaluateGitHandler struct {
	guardrailSvc GuardrailService
}

func NewEvaluateGitHandler(guardrailSvc GuardrailService) *EvaluateGitHandler {
	return &EvaluateGitHandler{guardrailSvc: guardrailSvc}
}

func (h *EvaluateGitHandler) Handle(ctx context.Context, q EvaluateGitQuery) (*ValidationResult, error) {
	violations, err := h.guardrailSvc.EvaluateGit(ctx, q.Command)
	if err != nil {
		return nil, err
	}
	return NewValidationResult(violations), nil
}

// EvaluateFileEditQuery queries file edit evaluation
type EvaluateFileEditQuery struct {
	FilePath  string `json:"file_path"`
	Content   string `json:"content"`
	SessionID string `json:"session_id"`
}

// EvaluateFileEditHandler handles EvaluateFileEditQuery
type EvaluateFileEditHandler struct {
	guardrailSvc GuardrailService
}

func NewEvaluateFileEditHandler(guardrailSvc GuardrailService) *EvaluateFileEditHandler {
	return &EvaluateFileEditHandler{guardrailSvc: guardrailSvc}
}

func (h *EvaluateFileEditHandler) Handle(ctx context.Context, q EvaluateFileEditQuery) (*ValidationResult, error) {
	violations, err := h.guardrailSvc.EvaluateFileEdit(ctx, q.FilePath, q.Content, q.SessionID)
	if err != nil {
		return nil, err
	}
	return NewValidationResult(violations), nil
}

// ListRulesQuery queries rule listing
type ListRulesQuery struct {
	Enabled  *bool  `json:"enabled,omitempty"`
	Category string `json:"category,omitempty"`
	Limit    int    `json:"limit"`
	Offset   int    `json:"offset"`
}

// ListRulesHandler handles ListRulesQuery
type ListRulesHandler struct {
	repo RuleRepository
}

func NewListRulesHandler(repo RuleRepository) *ListRulesHandler {
	return &ListRulesHandler{repo: repo}
}

func (h *ListRulesHandler) Handle(ctx context.Context, q ListRulesQuery) ([]PreventionRule, error) {
	return h.repo.List(ctx, q.Enabled, q.Category, q.Limit, q.Offset)
}

// ============================================================================
// EVENT BUS (interface only — concrete impl in adapters/)
// ============================================================================

// EventType defines the type of domain event
type EventType string

const (
	EventRuleCreated      EventType = "rule.created"
	EventRuleUpdated      EventType = "rule.updated"
	EventRuleDeleted      EventType = "rule.deleted"
	EventRuleToggled      EventType = "rule.toggled"
	EventCacheInvalidated EventType = "cache.invalidated"
)

// Event represents a domain event
type Event struct {
	Type      EventType   `json:"type"`
	Payload   interface{} `json:"payload"`
	Timestamp time.Time   `json:"timestamp"`
}

// EventBus allows cross-context communication via events
type EventBus interface {
	// Publish sends an event to all subscribers
	Publish(ctx context.Context, event Event)

	// Subscribe registers a handler for an event type
	Subscribe(eventType EventType, handler EventHandler)
}

// EventHandler is a function that handles domain events
type EventHandler func(ctx context.Context, event Event)
