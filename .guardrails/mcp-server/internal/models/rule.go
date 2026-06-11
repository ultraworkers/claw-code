package models

import (
	"fmt"
	"time"

	"github.com/google/uuid"
)

// PreventionRule represents a guardrail prevention rule
type PreventionRule struct {
	ID          uuid.UUID  `json:"id" db:"id"`
	RuleID      string     `json:"rule_id" db:"rule_id"`
	Name        string     `json:"name" db:"name"`
	Pattern     string     `json:"pattern" db:"pattern"`
	PatternHash *string    `json:"pattern_hash,omitempty" db:"pattern_hash"`
	Message     string     `json:"message" db:"message"`
	Severity    Severity   `json:"severity" db:"severity"`
	Enabled     bool       `json:"enabled" db:"enabled"`
	DocumentID  *uuid.UUID `json:"document_id,omitempty" db:"document_id"`
	Category    string     `json:"category" db:"category"`
	CreatedAt   time.Time  `json:"created_at" db:"created_at"`
	UpdatedAt   time.Time  `json:"updated_at" db:"updated_at"`
}

// Severity represents rule severity levels
type Severity string

const (
	SeverityCritical Severity = "critical"
	SeverityError    Severity = "error"
	SeverityWarning  Severity = "warning"
	SeverityInfo     Severity = "info"
)

// IsValidSeverity checks if a severity level is valid
func IsValidSeverity(sev string) bool {
	switch Severity(sev) {
	case SeverityCritical, SeverityError, SeverityWarning, SeverityInfo:
		return true
	}
	return false
}

// Action returns the recommended action for a severity level
func (s Severity) Action() string {
	switch s {
	case SeverityError:
		return "halt"
	case SeverityWarning:
		return "confirm"
	case SeverityInfo:
		return "log"
	default:
		return "log"
	}
}

// Validate checks if the prevention rule is valid for creation/update
func (r *PreventionRule) Validate() error {
	if r.RuleID == "" {
		return fmt.Errorf("rule_id is required")
	}
	if len(r.RuleID) > 50 {
		return fmt.Errorf("rule_id must be at most 50 characters")
	}
	if r.Name == "" {
		return fmt.Errorf("name is required")
	}
	if len(r.Name) > 255 {
		return fmt.Errorf("name must be at most 255 characters")
	}
	if r.Pattern == "" {
		return fmt.Errorf("pattern is required")
	}
	if r.Message == "" {
		return fmt.Errorf("message is required")
	}
	if !IsValidSeverity(string(r.Severity)) {
		return fmt.Errorf("invalid severity: %s", r.Severity)
	}
	if len(r.Category) > 50 {
		return fmt.Errorf("category must be at most 50 characters")
	}
	return nil
}
