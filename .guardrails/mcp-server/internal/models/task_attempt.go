package models

import (
	"fmt"
	"time"

	"github.com/google/uuid"
)

// TaskAttempt represents a record of a task attempt for three strikes tracking
type TaskAttempt struct {
	ID            uuid.UUID  `json:"id" db:"id"`
	SessionID     string     `json:"session_id" db:"session_id"`
	TaskID        *string    `json:"task_id,omitempty" db:"task_id"`
	AttemptNumber int        `json:"attempt_number" db:"attempt_number"`
	AttemptedAt   time.Time  `json:"attempted_at" db:"attempted_at"`
	ErrorMessage  string     `json:"error_message" db:"error_message"`
	ErrorCategory string     `json:"error_category" db:"error_category"`
	Resolution    string     `json:"resolution" db:"resolution"`
	ResolvedAt    *time.Time `json:"resolved_at,omitempty" db:"resolved_at"`
	CreatedAt     time.Time  `json:"created_at" db:"created_at"`
}

// ErrorCategory represents the category of an error
type ErrorCategory string

const (
	ErrorCategorySyntax  ErrorCategory = "syntax"
	ErrorCategoryRuntime   ErrorCategory = "runtime"
	ErrorCategoryLogic     ErrorCategory = "logic"
	ErrorCategoryTimeout   ErrorCategory = "timeout"
	ErrorCategoryOther     ErrorCategory = "other"
)

// ValidErrorCategories contains all valid error categories
var ValidErrorCategories = []string{
	string(ErrorCategorySyntax),
	string(ErrorCategoryRuntime),
	string(ErrorCategoryLogic),
	string(ErrorCategoryTimeout),
	string(ErrorCategoryOther),
}

// IsValidErrorCategory checks if an error category is valid
func IsValidErrorCategory(category string) bool {
	for _, c := range ValidErrorCategories {
		if c == category {
			return true
		}
	}
	return false
}

// Validate checks if the task attempt is valid for creation
func (t *TaskAttempt) Validate() error {
	if t.SessionID == "" {
		return fmt.Errorf("session_id is required")
	}
	if len(t.SessionID) > 100 {
		return fmt.Errorf("session_id must be at most 100 characters")
	}
	if t.TaskID != nil && len(*t.TaskID) > 100 {
		return fmt.Errorf("task_id must be at most 100 characters")
	}
	if t.AttemptNumber <= 0 {
		return fmt.Errorf("attempt_number must be positive")
	}
	if t.ErrorCategory != "" && !IsValidErrorCategory(t.ErrorCategory) {
		return fmt.Errorf("invalid error_category: %s", t.ErrorCategory)
	}
	if t.Resolution == "" {
		return fmt.Errorf("resolution is required")
	}
	if !IsValidResolutionStatus(t.Resolution) {
		return fmt.Errorf("invalid resolution: %s", t.Resolution)
	}
	if t.ResolvedAt != nil && t.Resolution != string(ResolutionResolved) && t.Resolution != string(ResolutionEscalated) && t.Resolution != string(ResolutionAbandoned) {
		return fmt.Errorf("resolved_at can only be set when resolution is 'resolved', 'escalated', or 'abandoned'")
	}
	return nil
}
