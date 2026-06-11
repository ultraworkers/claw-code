package models

import (
	"fmt"
	"time"

	"github.com/google/uuid"
)

// HaltEvent represents a halt condition triggered during a session
type HaltEvent struct {
	ID             uuid.UUID              `json:"id" db:"id"`
	SessionID      string                 `json:"session_id" db:"session_id"`
	HaltType       string                 `json:"halt_type" db:"halt_type"`
	Severity       string                 `json:"severity" db:"severity"`
	Description    string                 `json:"description" db:"description"`
	TriggeredAt    time.Time              `json:"triggered_at" db:"triggered_at"`
	Acknowledged   bool                   `json:"acknowledged" db:"acknowledged"`
	AcknowledgedAt *time.Time             `json:"acknowledged_at,omitempty" db:"acknowledged_at"`
	Resolution     string                 `json:"resolution" db:"resolution"`
	ContextData    map[string]interface{} `json:"context_data,omitempty" db:"context_data"`
	CreatedAt      time.Time              `json:"created_at" db:"created_at"`
}

// HaltType represents the type of halt condition
type HaltType string

const (
	HaltTypeCodeSafety  HaltType = "code_safety"
	HaltTypeScope       HaltType = "scope"
	HaltTypeEnvironment HaltType = "environment"
	HaltTypeExecution   HaltType = "execution"
	HaltTypeSecurity    HaltType = "security"
	HaltTypeUncertainty HaltType = "uncertainty"
)

// HaltSeverity represents the severity level of a halt
type HaltSeverity string

const (
	HaltSeverityLow      HaltSeverity = "low"
	HaltSeverityMedium   HaltSeverity = "medium"
	HaltSeverityHigh     HaltSeverity = "high"
	HaltSeverityCritical HaltSeverity = "critical"
)


// ValidHaltTypes contains all valid halt types
var ValidHaltTypes = []string{
	string(HaltTypeCodeSafety),
	string(HaltTypeScope),
	string(HaltTypeEnvironment),
	string(HaltTypeExecution),
	string(HaltTypeSecurity),
	string(HaltTypeUncertainty),
}

// ValidHaltSeverities contains all valid halt severities
var ValidHaltSeverities = []string{
	string(HaltSeverityLow),
	string(HaltSeverityMedium),
	string(HaltSeverityHigh),
	string(HaltSeverityCritical),
}

// IsValidHaltType checks if a halt type is valid
func IsValidHaltType(t string) bool {
	for _, ht := range ValidHaltTypes {
		if ht == t {
			return true
		}
	}
	return false
}

// IsValidHaltSeverity checks if a halt severity is valid
func IsValidHaltSeverity(s string) bool {
	for _, hs := range ValidHaltSeverities {
		if hs == s {
			return true
		}
	}
	return false
}

// Validate checks if the halt event is valid for creation
func (h *HaltEvent) Validate() error {
	if h.SessionID == "" {
		return fmt.Errorf("session_id is required")
	}
	if len(h.SessionID) > 100 {
		return fmt.Errorf("session_id must be at most 100 characters")
	}
	if h.HaltType == "" {
		return fmt.Errorf("halt_type is required")
	}
	if !IsValidHaltType(h.HaltType) {
		return fmt.Errorf("invalid halt_type: %s", h.HaltType)
	}
	if h.Severity == "" {
		return fmt.Errorf("severity is required")
	}
	if !IsValidHaltSeverity(h.Severity) {
		return fmt.Errorf("invalid severity: %s", h.Severity)
	}
	if h.Resolution == "" {
		return fmt.Errorf("resolution is required")
	}
	if !IsValidResolutionStatus(h.Resolution) {
		return fmt.Errorf("invalid resolution: %s", h.Resolution)
	}
	if h.Acknowledged && h.AcknowledgedAt == nil {
		return fmt.Errorf("acknowledged_at must be set when acknowledged is true")
	}
	if h.AcknowledgedAt != nil && !h.Acknowledged {
		return fmt.Errorf("acknowledged must be true when acknowledged_at is set")
	}
	return nil
}
