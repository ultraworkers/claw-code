package models

import (
	"fmt"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgtype"
)

// FailureEntry represents an entry in the failure registry
type FailureEntry struct {
	ID                uuid.UUID            `json:"id" db:"id"`
	FailureID         string               `json:"failure_id" db:"failure_id"`
	Category          string               `json:"category" db:"category"`
	Severity          string               `json:"severity" db:"severity"`
	ErrorMessage      string               `json:"error_message" db:"error_message"`
	RootCause         string               `json:"root_cause" db:"root_cause"`
	AffectedFiles     pgtype.Array[string] `json:"affected_files" db:"affected_files"`
	RegressionPattern string               `json:"regression_pattern" db:"regression_pattern"`
	Status            string               `json:"status" db:"status"`
	ProjectSlug       string               `json:"project_slug" db:"project_slug"`
	CreatedAt         time.Time            `json:"created_at" db:"created_at"`
	UpdatedAt         time.Time            `json:"updated_at" db:"updated_at"`
}

// FailureStatus represents the status of a failure entry
type FailureStatus string

const (
	StatusActive     FailureStatus = "active"
	StatusResolved   FailureStatus = "resolved"
	StatusDeprecated FailureStatus = "deprecated"
)

// IsValidFailureStatus checks if a status is valid
func IsValidFailureStatus(status string) bool {
	switch FailureStatus(status) {
	case StatusActive, StatusResolved, StatusDeprecated:
		return true
	}
	return false
}

// ToStringSlice converts pgtype.Array[string] to []string for convenience
func ToStringSlice(arr pgtype.Array[string]) []string {
	if !arr.Valid {
		return nil
	}
	return arr.Elements
}

// ToTextArray converts []string to pgtype.Array[string] for database storage
func ToTextArray(slice []string) pgtype.Array[string] {
	return pgtype.Array[string]{
		Elements: slice,
		Valid:    slice != nil,
	}
}

// ValidFailureSeverities contains valid severity levels
var ValidFailureSeverities = []string{"critical", "high", "medium", "low"}

// IsValidFailureSeverity checks if a severity is valid
func IsValidFailureSeverity(severity string) bool {
	for _, s := range ValidFailureSeverities {
		if s == severity {
			return true
		}
	}
	return false
}

// Validate checks if the failure entry is valid for creation/update
func (f *FailureEntry) Validate() error {
	if f.FailureID == "" {
		return fmt.Errorf("failure_id is required")
	}
	if len(f.FailureID) > 50 {
		return fmt.Errorf("failure_id must be at most 50 characters")
	}
	if f.Category == "" {
		return fmt.Errorf("category is required")
	}
	if len(f.Category) > 50 {
		return fmt.Errorf("category must be at most 50 characters")
	}
	if !IsValidFailureSeverity(f.Severity) {
		return fmt.Errorf("invalid severity: %s (must be one of: critical, high, medium, low)", f.Severity)
	}
	if f.ErrorMessage == "" {
		return fmt.Errorf("error_message is required")
	}
	if !IsValidFailureStatus(f.Status) {
		return fmt.Errorf("invalid status: %s", f.Status)
	}
	if len(f.ProjectSlug) > 100 {
		return fmt.Errorf("project_slug must be at most 100 characters")
	}
	return nil
}
