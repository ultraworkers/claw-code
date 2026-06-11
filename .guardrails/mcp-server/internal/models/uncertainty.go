package models

import (
	"encoding/json"
	"time"
)

// UncertaintyLevel defines the various uncertainty levels
type UncertaintyLevel string

// Uncertainty levels for tracking decision-making
const (
	UncertaintyCritical    UncertaintyLevel = "critical"    // System blocked, cannot proceed
	UncertaintyBlocked     UncertaintyLevel = "blocked"     // Unresolvable issue, needs human
	UncertaintyHigh        UncertaintyLevel = "high"        // Major questions, significant risk
	UncertaintyMedium      UncertaintyLevel = "medium"      // Some questions, manageable risk
	UncertaintyInvestigating UncertaintyLevel = "investigating" // Actively researching
	UncertaintyLow         UncertaintyLevel = "low"         // Minor doubts
	UncertaintyResolved    UncertaintyLevel = "resolved"    // Clarity achieved
)

// ValidUncertaintyLevels contains all valid uncertainty levels
var ValidUncertaintyLevels = []string{
	string(UncertaintyCritical),
	string(UncertaintyBlocked),
	string(UncertaintyHigh),
	string(UncertaintyMedium),
	string(UncertaintyInvestigating),
	string(UncertaintyLow),
	string(UncertaintyResolved),
}

// IsValidUncertaintyLevel checks if a string is a valid uncertainty level
func IsValidUncertaintyLevel(s string) bool {
	for _, level := range ValidUncertaintyLevels {
		if level == s {
			return true
		}
	}
	return false
}

// UncertaintyRecord represents a saved uncertainty tracking record
type UncertaintyRecord struct {
	ID               string           `json:"id"`
	SessionID        string           `json:"session_id"`
	TaskID          *string           `json:"task_id,omitempty"`
	UncertaintyLevel UncertaintyLevel `json:"uncertainty_level"`
	DecisionMade     string           `json:"decision_made"`
	ContextData      json.RawMessage  `json:"context_data"`
	EscalationRequired bool          `json:"escalation_required"`
	CreatedAt        time.Time        `json:"created_at"`
}

// UncertaintyCheckResult represents the result of checking uncertainty levels
type UncertaintyCheckResult struct {
	SessionID       string           `json:"session_id"`
	CurrentLevel    UncertaintyLevel `json:"current_level"`
	PreviousLevel   UncertaintyLevel `json:"previous_level"`
	Escalated       bool             `json:"escalated"`
	DecisionMade    string           `json:"decision_made"`
	ContextSummary  string           `json:"context_summary"`
	Recommendation  string           `json:"recommendation"`
}

// UncertaintyCheckInput represents the input parameters for checking uncertainty
type UncertaintyCheckInput struct {
	SessionToken     string          `json:"session_token"`
	CurrentTask      string          `json:"current_task"`
	ContextData      json.RawMessage `json:"context_data"`
	SelfAssessment   string          `json:"self_assessment"`
}

// UncertaintyLevelGuide defines recommended actions for each uncertainty level
type UncertaintyLevelGuide struct {
	Level        UncertaintyLevel `json:"level"`
	Description  string           `json:"description"`
	Action       string           `json:"action"`
	Threshold    int              `json:"threshold"` // Number of similar issues before escalation
	RequiresLogs bool             `json:"requires_logs"`
}

// GetUncertaintyLevelGuides returns guidance for each uncertainty level
func GetUncertaintyLevelGuides() []UncertaintyLevelGuide {
	return []UncertaintyLevelGuide{
		{
			Level:        UncertaintyCritical,
			Description:  "System blocked - core functionality compromised",
			Action:       "Immediately escalate to human, halt all operations",
			Threshold:    1,
			RequiresLogs: true,
		},
		{
			Level:        UncertaintyBlocked,
			Description:  "Issue cannot be resolved with current capabilities",
			Action:       "Escalate to human, document attempted solutions",
			Threshold:    1,
			RequiresLogs: true,
		},
		{
			Level:        UncertaintyHigh,
			Description:  "Major questions with significant risk or impact",
			Action:       "Research thoroughly, consider alternatives, escalate if unresolved within reasonable time",
			Threshold:    2,
			RequiresLogs: true,
		},
		{
			Level:        UncertaintyMedium,
			Description:  "Some questions but manageable risk",
			Action:       "Proceed with caution, document decision rationale, seek clarification if needed",
			Threshold:    3,
			RequiresLogs: false,
		},
		{
			Level:        UncertaintyInvestigating,
			Description:  "Actively researching or exploring options",
			Action:       "Continue research, set time limits, escalate if stuck",
			Threshold:    4,
			RequiresLogs: false,
		},
		{
			Level:        UncertaintyLow,
			Description:  "Minor doubts or clarifications needed",
			Action:       "Proceed with standard patterns, minor clarifications via available resources",
			Threshold:    5,
			RequiresLogs: false,
		},
		{
			Level:        UncertaintyResolved,
			Description:  "Clarity achieved, path forward is clear",
			Action:       "Proceed confidently, document learnings for future reference",
			Threshold:    0,
			RequiresLogs: false,
		},
	}
}
