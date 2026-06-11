package models

import (
	"encoding/json"
	"fmt"
	"time"

	"github.com/google/uuid"
	"github.com/lib/pq"
)

// Project represents a project with guardrail configuration
type Project struct {
	ID               uuid.UUID      `json:"id" db:"id"`
	Name             string         `json:"name" db:"name"`
	Slug             string         `json:"slug" db:"slug"`
	GuardrailContext string         `json:"guardrail_context" db:"guardrail_context"`
	ActiveRules      pq.StringArray `json:"active_rules" db:"active_rules"`
	Metadata         []byte         `json:"-" db:"metadata"`
	CreatedAt        time.Time      `json:"created_at" db:"created_at"`
	UpdatedAt        time.Time      `json:"updated_at" db:"updated_at"`
}

// Violation represents a guardrail violation found during validation
type Violation struct {
	RuleID               string `json:"rule_id"`
	RuleName             string `json:"rule_name"`
	Severity             string `json:"severity"`
	Message              string `json:"message"`
	Category             string `json:"category"`
	Action               string `json:"action"`
	SuggestedAlternative string `json:"suggested_alternative,omitempty"`
	DocumentationURI     string `json:"documentation_uri,omitempty"`
	Line                 int    `json:"line,omitempty"`
	Column               int    `json:"column,omitempty"`
}

// ValidationResult represents the result of a validation check
type ValidationResult struct {
	Valid      bool           `json:"valid"`
	Violations []Violation    `json:"violations"`
	Meta       ValidationMeta `json:"meta"`
}

// ValidationMeta contains metadata about the validation
type ValidationMeta struct {
	CheckedAt      time.Time `json:"checked_at"`
	RulesEvaluated int       `json:"rules_evaluated"`
	DurationMs     int64     `json:"duration_ms"`
	Cached         bool      `json:"cached"`
}

// Session represents an MCP client session
type Session struct {
	Token         string    `json:"token"`
	ProjectSlug   string    `json:"project_slug"`
	AgentType     string    `json:"agent_type"`
	ClientVersion string    `json:"client_version"`
	CreatedAt     time.Time `json:"created_at"`
	ExpiresAt     time.Time `json:"expires_at"`
}

// Validate checks if the project is valid for creation/update
func (p *Project) Validate() error {
	if p.Name == "" {
		return fmt.Errorf("name is required")
	}
	if len(p.Name) > 255 {
		return fmt.Errorf("name must be at most 255 characters")
	}
	if p.Slug == "" {
		return fmt.Errorf("slug is required")
	}
	if len(p.Slug) > 100 {
		return fmt.Errorf("slug must be at most 100 characters")
	}
	// Validate slug format (alphanumeric, hyphens, underscores)
	for _, r := range p.Slug {
		if !isValidSlugChar(r) {
			return fmt.Errorf("slug contains invalid characters: %q", r)
		}
	}
	return nil
}

// isValidSlugChar checks if a character is valid for a slug
func isValidSlugChar(r rune) bool {
	return (r >= 'a' && r <= 'z') ||
		(r >= 'A' && r <= 'Z') ||
		(r >= '0' && r <= '9') ||
		r == '-' || r == '_'
}

// GetMetadata returns the metadata as a map
func (p *Project) GetMetadata() map[string]any {
	if len(p.Metadata) == 0 {
		return nil
	}
	var result map[string]any
	if err := json.Unmarshal(p.Metadata, &result); err != nil {
		return nil
	}
	return result
}

// MarshalJSON implements custom JSON marshaling for Project
func (p Project) MarshalJSON() ([]byte, error) {
	type Alias Project
	return json.Marshal(&struct {
		Metadata map[string]any `json:"metadata"`
		*Alias
	}{
		Metadata: p.GetMetadata(),
		Alias:    (*Alias)(&p),
	})
}
