package git

import (
	"context"
	"log/slog"
	"strings"
	"time"

	"github.com/thearchitectit/guardrail-mcp/internal/domain"
)

// Rule represents a git guardrail rule
type Rule struct {
	ID       string `json:"id"`
	Name     string `json:"name"`
	Pattern  string `json:"pattern"`
	Message  string `json:"message"`
	Severity string `json:"severity"` // "critical", "high", "medium", "low"
	Enabled  bool   `json:"enabled"`
}

// Evaluator performs pattern matching for git commands
type Evaluator struct {
	rules     []Rule
	patternFn func(pattern, input string) (bool, error)
}

// NewEvaluator creates a new git evaluator
func NewEvaluator(rules []Rule, patternFn func(pattern, input string) (bool, error)) *Evaluator {
	return &Evaluator{
		rules:     rules,
		patternFn: patternFn,
	}
}

// Evaluate checks a git command against all enabled rules
func (e *Evaluator) Evaluate(ctx context.Context, command string) ([]domain.Violation, error) {
	var violations []domain.Violation

	for _, rule := range e.rules {
		if !rule.Enabled {
			continue
		}

		matched, err := e.patternFn(rule.Pattern, command)
		if err != nil {
			slog.Warn("Pattern matching error", "rule_id", rule.ID, "error", err)
			continue
		}

		if matched {
			violations = append(violations, domain.Violation{
				RuleID:         rule.ID,
				RuleName:       rule.Name,
				Severity:       toSeverity(rule.Severity),
				Message:        rule.Message,
				Category:       "git",
				MatchedPattern: rule.Pattern,
				MatchedInput:   truncate(command, 200),
				Timestamp:      time.Now(),
			})
		}
	}

	return violations, nil
}

// DetectForcePush detects force push operations
func (e *Evaluator) DetectForcePush(command string) bool {
	forceFlags := []string{"--force", "-f", "--force-with-lease", "-ff"}
	for _, flag := range forceFlags {
		if strings.Contains(command, flag) {
			return true
		}
	}
	return false
}

// Store handles data access for git rules
type Store interface {
	GetActiveRules(ctx context.Context) ([]Rule, error)
}

// Cache handles caching for git rules
type Cache interface {
	GetGitRules(ctx context.Context) ([]Rule, error)
	SetGitRules(ctx context.Context, rules []Rule, ttl time.Duration) error
}

// Handler is the MCP handler for git guardrail evaluation
type Handler struct {
	store     Store
	cache     Cache
	patternFn func(string, string) (bool, error)
	cacheTTL  time.Duration
}

// NewHandler creates a new git guardrail handler
func NewHandler(store Store, cache Cache, patternFn func(string, string) (bool, error)) *Handler {
	return &Handler{
		store:     store,
		cache:     cache,
		patternFn: patternFn,
		cacheTTL:  30 * time.Second,
	}
}

// HandleEvaluate processes a git command evaluation request
func (h *Handler) HandleEvaluate(ctx context.Context, command string) (*domain.ValidationResult, error) {
	rules, err := h.loadRules(ctx)
	if err != nil {
		return nil, err
	}

	evaluator := NewEvaluator(rules, h.patternFn)
	violations, err := evaluator.Evaluate(ctx, command)
	if err != nil {
		return nil, err
	}

	return domain.NewValidationResult(violations), nil
}

// HandleWithForceCheck evaluates with automatic force-push detection
func (h *Handler) HandleWithForceCheck(ctx context.Context, command string, isForceFlag bool) (*domain.ValidationResult, error) {
	result, err := h.HandleEvaluate(ctx, command)
	if err != nil {
		return nil, err
	}

	// Auto-detect force flag if not explicitly provided
	shouldCheck := isForceFlag || h.hasForceFlag(command)
	if shouldCheck {
		result.Violations = append(result.Violations, domain.Violation{
			RuleID:   "PREVENT-FORCE-001",
			RuleName: "No Force Operation",
			Severity: domain.SeverityCritical,
			Message:  "Force operations are not allowed. Use --force-with-lease or standard push instead.",
			Category: "git",
			Timestamp: time.Now(),
		})
	}

	return result, nil
}

func (h *Handler) hasForceFlag(command string) bool {
	forceFlags := []string{"--force", "-f", "--force-with-lease", "-ff"}
	for _, flag := range forceFlags {
		if strings.Contains(command, flag) {
			return true
		}
	}
	return false
}

func (h *Handler) loadRules(ctx context.Context) ([]Rule, error) {
	rules, err := h.cache.GetGitRules(ctx)
	if err == nil && len(rules) > 0 {
		return rules, nil
	}

	rules, err = h.store.GetActiveRules(ctx)
	if err != nil {
		return nil, err
	}

	h.cache.SetGitRules(ctx, rules, h.cacheTTL)
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
