package domain

import (
	"context"
	"time"

	"github.com/google/uuid"
)

// Violation represents a rule violation found during validation
type Violation struct {
	RuleID         string    `json:"rule_id"`
	RuleName       string    `json:"rule_name"`
	Severity       Severity  `json:"severity"`
	Message        string    `json:"message"`
	Category       string    `json:"category"`
	MatchedPattern string    `json:"matched_pattern"`
	MatchedInput   string    `json:"matched_input,omitempty"`
	Timestamp      time.Time `json:"timestamp"`
}

// Severity defines the severity level of a rule violation
type Severity string

const (
	SeverityCritical Severity = "critical"
	SeverityHigh     Severity = "high"
	SeverityMedium   Severity = "medium"
	SeverityLow      Severity = "low"
)

// PreventionRule represents a guardrail rule in the domain layer
type PreventionRule struct {
	ID           uuid.UUID `json:"id"`
	RuleID       string    `json:"rule_id"`
	Name         string    `json:"name"`
	Pattern      string    `json:"pattern"`
	PatternHash  string    `json:"pattern_hash"`
	Message      string    `json:"message"`
	Severity     Severity  `json:"severity"`
	Enabled      bool      `json:"enabled"`
	DocumentID   uuid.UUID `json:"document_id,omitempty"`
	Category     string    `json:"category"`
	CreatedAt    time.Time `json:"created_at"`
	UpdatedAt    time.Time `json:"updated_at"`
}

// RuleCategory defines the type of rules for validation
type RuleCategory string

const (
	CategoryBash     RuleCategory = "bash"
	CategoryGit      RuleCategory = "git"
	CategoryFileEdit RuleCategory = "file_edit"
	CategoryAll      RuleCategory = "all"
)

// FileReadVerification represents the result of verifying if a file was read
type FileReadVerification struct {
	WasRead       bool          `json:"was_read"`
	ReadAt        *time.Time    `json:"read_at,omitempty"`
	TimeSinceRead time.Duration `json:"time_since_read,omitempty"`
}

// GuardrailService is the primary port for guardrail evaluation (CQRS Query side)
// Flip dependencies: infrastructure depends on this interface, not the other way around
type GuardrailService interface {
	// EvaluateCommand evaluates a bash command against prevention rules
	EvaluateCommand(ctx context.Context, command string) ([]Violation, error)

	// EvaluateGit evaluates a git command against prevention rules
	EvaluateGit(ctx context.Context, command string) ([]Violation, error)

	// EvaluateFileEdit evaluates a file edit operation against prevention rules
	EvaluateFileEdit(ctx context.Context, filePath string, content string, sessionID string) ([]Violation, error)

	// EvaluateInput validates generic input against prevention rules (backward compatible)
	EvaluateInput(ctx context.Context, input string, categories []string) ([]Violation, error)

	// CheckFileRead verifies if a file was read in a session before editing
	CheckFileRead(ctx context.Context, sessionID, filePath string) (*FileReadVerification, error)
}

// RuleRepository is the data port for CRUD operations on prevention rules
type RuleRepository interface {
	// GetByID retrieves a rule by its database ID
	GetByID(ctx context.Context, id uuid.UUID) (*PreventionRule, error)

	// GetByRuleID retrieves a rule by its rule_id
	GetByRuleID(ctx context.Context, ruleID string) (*PreventionRule, error)

	// List retrieves rules with optional filters
	List(ctx context.Context, enabled *bool, category string, limit, offset int) ([]PreventionRule, error)

	// GetActiveRules retrieves all enabled rules for evaluation
	GetActiveRules(ctx context.Context) ([]PreventionRule, error)

	// Create inserts a new rule
	Create(ctx context.Context, rule *PreventionRule) error

	// Update modifies an existing rule
	Update(ctx context.Context, rule *PreventionRule) error

	// Delete removes a rule
	Delete(ctx context.Context, id uuid.UUID) error

	// Toggle enables or disables a rule
	Toggle(ctx context.Context, id uuid.UUID, enabled bool) error

	// Count returns the total number of rules matching optional filters
	Count(ctx context.Context, enabled *bool, category string) (int, error)
}

// AuditLogger is the cross-cutting port for violation logging
type AuditLogger interface {
	// LogViolation records a rule violation
	LogViolation(ctx context.Context, violation Violation, sessionID string) error

	// LogHaltEvent records when an agent halts execution
	LogHaltEvent(ctx context.Context, sessionID, reason string, violations []Violation) error

	// GetRecentViolations retrieves recent violations for a session
	GetRecentViolations(ctx context.Context, sessionID string, limit int) ([]Violation, error)
}

// CachePort is the port for rule caching operations
type CachePort interface {
	// GetActiveRules retrieves cached active rules
	GetActiveRules(ctx context.Context) ([]PreventionRule, error)

	// SetActiveRules caches active rules with TTL
	SetActiveRules(ctx context.Context, rules []PreventionRule, ttl time.Duration) error

	// InvalidateRules clears the rule cache
	InvalidateRules(ctx context.Context) error
}

// PatternMatcher is the port for pattern matching implementations
// Allows swapping regex for LLM-based matching without changing business logic
type PatternMatcher interface {
	// Match evaluates a pattern against input
	Match(pattern, input string) (bool, error)

	// ValidatePattern checks if a pattern string is valid
	ValidatePattern(pattern string) error
}

// ValidationResult is the result of a guardrail evaluation
type ValidationResult struct {
	Passed     bool        `json:"passed"`
	Violations []Violation `json:"violations"`
	CheckedAt  time.Time   `json:"checked_at"`
}

// NewValidationResult creates a new validation result
func NewValidationResult(violations []Violation) *ValidationResult {
	return &ValidationResult{
		Passed:     len(violations) == 0,
		Violations: violations,
		CheckedAt:  time.Now(),
	}
}
