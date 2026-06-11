package fileedit

import (
	"context"
	"log/slog"
	"strings"
	"time"

	"github.com/thearchitectit/guardrail-mcp/internal/domain"
)

// Rule represents a file edit guardrail rule
type Rule struct {
	ID       string `json:"id"`
	Name     string `json:"name"`
	Pattern  string `json:"pattern"`
	Message  string `json:"message"`
	Severity string `json:"severity"` // "critical", "high", "medium", "low"
	Enabled  bool   `json:"enabled"`
	Category string `json:"category"`
}

// Evaluator performs pattern matching for file edits
type Evaluator struct {
	rules     []Rule
	patternFn func(pattern, input string) (bool, error)
}

// NewEvaluator creates a new file edit evaluator
func NewEvaluator(rules []Rule, patternFn func(pattern, input string) (bool, error)) *Evaluator {
	return &Evaluator{
		rules:     rules,
		patternFn: patternFn,
	}
}

// Evaluate checks file path and content against all enabled rules
func (e *Evaluator) Evaluate(ctx context.Context, filePath, content, sessionID string) ([]domain.Violation, error) {
	var violations []domain.Violation

	// Check file path first
	pathViolations := e.evaluatePath(filePath)
	violations = append(violations, pathViolations...)

	// Check content against rules
	for _, rule := range e.rules {
		if !rule.Enabled || rule.Category == "path" {
			continue
		}

		matched, err := e.patternFn(rule.Pattern, content)
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
				Category:       "file_edit",
				MatchedPattern: rule.Pattern,
				MatchedInput:   truncate(content, 200),
				Timestamp:      time.Now(),
			})
		}
	}

	return violations, nil
}

// evaluatePath checks for path traversal and sensitive locations
func (e *Evaluator) evaluatePath(filePath string) []domain.Violation {
	var violations []domain.Violation

	// Path traversal patterns
	traversalPatterns := []string{"../", ".../", "//", "/..", "\\.."}

	for _, pattern := range traversalPatterns {
		if strings.Contains(filePath, pattern) {
			violations = append(violations, domain.Violation{
				RuleID:         "PATH-TRAVERSAL-001",
				RuleName:       "Path Traversal Detected",
				Severity:       domain.SeverityCritical,
				Message:        "Path traversal pattern detected in file path",
				Category:       "file_path",
				MatchedPattern: pattern,
				Timestamp:      time.Now(),
			})
		}
	}

	// Sensitive paths
	sensitivePatterns := []struct {
		pattern string
		message string
	}{
		{"/etc/", "System configuration path - modification not allowed"},
		{"/root/", "Root home directory - modification not allowed"},
		{"/.ssh/", "SSH configuration - modification not allowed"},
		{"/.aws/", "AWS credentials path - modification not allowed"},
		{".env", "Environment file - secrets should not be committed directly"},
		{"credentials", "Potential credential file - verify content is safe"},
	}

	for _, sp := range sensitivePatterns {
		if strings.Contains(filePath, sp.pattern) {
			violations = append(violations, domain.Violation{
				RuleID:         "SENSITIVE-PATH-001",
				RuleName:       "Sensitive Path",
				Severity:       domain.SeverityHigh,
				Message:        sp.message,
				Category:       "file_path",
				MatchedPattern: sp.pattern,
				Timestamp:      time.Now(),
			})
		}
	}

	return violations
}

// Store handles data access for file edit rules
type Store interface {
	GetActiveRules(ctx context.Context) ([]Rule, error)
}

// Cache handles caching for file edit rules
type Cache interface {
	GetFileEditRules(ctx context.Context) ([]Rule, error)
	SetFileEditRules(ctx context.Context, rules []Rule, ttl time.Duration) error
}

// FileReadChecker verifies files were read before editing
type FileReadChecker interface {
	CheckFileRead(ctx context.Context, sessionID, filePath string) (*domain.FileReadVerification, error)
}

// Handler is the MCP handler for file edit guardrail evaluation
type Handler struct {
	store      Store
	cache      Cache
	fileReader FileReadChecker
	patternFn  func(string, string) (bool, error)
	cacheTTL   time.Duration
}

// NewHandler creates a new file edit guardrail handler
func NewHandler(store Store, cache Cache, fileReader FileReadChecker, patternFn func(string, string) (bool, error)) *Handler {
	return &Handler{
		store:      store,
		cache:      cache,
		fileReader: fileReader,
		patternFn:  patternFn,
		cacheTTL:  30 * time.Second,
	}
}

// HandleEvaluate processes a file edit evaluation request
func (h *Handler) HandleEvaluate(ctx context.Context, filePath, content, sessionID string) (*domain.ValidationResult, error) {
	rules, err := h.loadRules(ctx)
	if err != nil {
		return nil, err
	}

	evaluator := NewEvaluator(rules, h.patternFn)
	violations, err := evaluator.Evaluate(ctx, filePath, content, sessionID)
	if err != nil {
		return nil, err
	}

	return domain.NewValidationResult(violations), nil
}

// HandleWithFileReadCheck evaluates with file-read verification
func (h *Handler) HandleWithFileReadCheck(ctx context.Context, filePath, content, sessionID string) (*domain.ValidationResult, error) {
	result, err := h.HandleEvaluate(ctx, filePath, content, sessionID)
	if err != nil {
		return nil, err
	}

	if h.fileReader != nil && sessionID != "" {
		verification, err := h.fileReader.CheckFileRead(ctx, sessionID, filePath)
		if err != nil {
			slog.Warn("File read verification failed", "error", err)
		} else if verification != nil && !verification.WasRead {
			result.Violations = append(result.Violations, domain.Violation{
				RuleID:   "FILE-READ-001",
				RuleName: "File Not Read Before Edit",
				Severity: domain.SeverityHigh,
				Message:  "This file was not read before the edit was attempted. Read the file first.",
				Category: "file_edit",
				Timestamp: time.Now(),
			})
		}
	}

	return result, nil
}

func (h *Handler) loadRules(ctx context.Context) ([]Rule, error) {
	rules, err := h.cache.GetFileEditRules(ctx)
	if err == nil && len(rules) > 0 {
		return rules, nil
	}

	rules, err = h.store.GetActiveRules(ctx)
	if err != nil {
		return nil, err
	}

	h.cache.SetFileEditRules(ctx, rules, h.cacheTTL)
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
