package bash

import (
	"context"
	"fmt"
	"log/slog"
	"time"

	"github.com/thearchitectit/guardrail-mcp/internal/domain"
)

// Rule represents a bash guardrail rule
type Rule struct {
	ID       string `json:"id"`
	Name     string `json:"name"`
	Pattern  string `json:"pattern"`
	Message  string `json:"message"`
	Severity string `json:"severity"` // "critical", "high", "medium", "low"
	Enabled  bool   `json:"enabled"`
	Category string `json:"category"`
}

// Evaluator performs pattern matching for bash commands
type Evaluator struct {
	rules     []Rule
	patternFn func(pattern, input string) (bool, error)
}

// NewEvaluator creates a new bash evaluator with rules
func NewEvaluator(rules []Rule, patternFn func(pattern, input string) (bool, error)) *Evaluator {
	return &Evaluator{
		rules:     rules,
		patternFn: patternFn,
	}
}

// Evaluate checks a bash command against all enabled rules
func (e *Evaluator) Evaluate(ctx context.Context, command string) ([]domain.Violation, error) {
	var violations []domain.Violation

	for _, rule := range e.rules {
		if !rule.Enabled {
			continue
		}

		matched, err := e.patternFn(rule.Pattern, command)
		if err != nil {
			slog.Warn("Pattern matching error",
				"rule_id", rule.ID,
				"error", err,
			)
			continue
		}

		if matched {
			violations = append(violations, domain.Violation{
				RuleID:         rule.ID,
				RuleName:       rule.Name,
				Severity:       toSeverity(rule.Severity),
				Message:        rule.Message,
				Category:       "bash",
				MatchedPattern: rule.Pattern,
				MatchedInput:   truncate(command, 200),
				Timestamp:      time.Now(),
			})
		}
	}

	return violations, nil
}

// Store handles data access for bash rules
type Store interface {
	GetActiveRules(ctx context.Context) ([]Rule, error)
}

// Cache is the cache port for bash rules
type Cache interface {
	GetBashRules(ctx context.Context) ([]Rule, error)
	SetBashRules(ctx context.Context, rules []Rule, ttl time.Duration) error
}

// Handler is the MCP handler for bash guardrail evaluation
type Handler struct {
	store     Store
	cache     Cache
	patternFn func(string, string) (bool, error)
	cacheTTL  time.Duration
}

// NewHandler creates a new bash guardrail handler
func NewHandler(store Store, cache Cache, patternFn func(string, string) (bool, error)) *Handler {
	return &Handler{
		store:     store,
		cache:     cache,
		patternFn: patternFn,
		cacheTTL:  30 * time.Second,
	}
}

// HandleEvaluate processes a bash command evaluation request
func (h *Handler) HandleEvaluate(ctx context.Context, command string) (*domain.ValidationResult, error) {
	rules, err := h.loadRules(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to load rules: %w", err)
	}

	evaluator := NewEvaluator(rules, h.patternFn)
	violations, err := evaluator.Evaluate(ctx, command)
	if err != nil {
		return nil, err
	}

	return domain.NewValidationResult(violations), nil
}

func (h *Handler) loadRules(ctx context.Context) ([]Rule, error) {
	// Try cache first
	rules, err := h.cache.GetBashRules(ctx)
	if err == nil && len(rules) > 0 {
		return rules, nil
	}

	// Cache miss — load from store
	rules, err = h.store.GetActiveRules(ctx)
	if err != nil {
		return nil, err
	}

	// Populate cache
	h.cache.SetBashRules(ctx, rules, h.cacheTTL)
	return rules, nil
}

func toSeverity(s string) domain.Severity {
	switch s {
	case "critical":
		return domain.SeverityCritical
	case "high":
		return domain.SeverityHigh
	case "medium":
		return domain.SeverityMedium
	case "low":
		return domain.SeverityLow
	default:
		return domain.SeverityMedium
	}
}

func truncate(s string, max int) string {
	if len(s) <= max {
		return s
	}
	return s[:max] + "..."
}
