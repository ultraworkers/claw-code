package models

import (
	"fmt"
	"time"

	"github.com/google/uuid"
)

// CodeType represents the type of code being validated
type CodeType string

const (
	CodeTypeProduction    CodeType = "production"
	CodeTypeTest          CodeType = "test"
	CodeTypeInfrastructure CodeType = "infrastructure"
)

// ValidCodeTypes contains all valid code types
var ValidCodeTypes = []string{
	string(CodeTypeProduction),
	string(CodeTypeTest),
	string(CodeTypeInfrastructure),
}

// IsValidCodeType checks if a code type is valid
func IsValidCodeType(codeType string) bool {
	for _, ct := range ValidCodeTypes {
		if ct == codeType {
			return true
		}
	}
	return false
}

// ProductionCode represents a record of production code tracked for guardrail validation
type ProductionCode struct {
	ID          uuid.UUID `json:"id" db:"id"`
	SessionID   string    `json:"session_id" db:"session_id"`
	FilePath    string    `json:"file_path" db:"file_path"`
	CodeType    CodeType  `json:"code_type" db:"code_type"`
	CreatedAt   time.Time `json:"created_at" db:"created_at"`
	VerifiedAt  *time.Time `json:"verified_at,omitempty" db:"verified_at"`
}

// Validate checks if the production code record is valid
func (pc *ProductionCode) Validate() error {
	if pc.SessionID == "" {
		return fmt.Errorf("session_id is required")
	}
	if len(pc.SessionID) > 255 {
		return fmt.Errorf("session_id must be at most 255 characters")
	}
	if pc.FilePath == "" {
		return fmt.Errorf("file_path is required")
	}
	if len(pc.FilePath) > 500 {
		return fmt.Errorf("file_path must be at most 500 characters")
	}
	if !IsValidCodeType(string(pc.CodeType)) {
		return fmt.Errorf("invalid code_type: %s", pc.CodeType)
	}
	return nil
}

// ProductionCodeValidationResult represents the result of production first validation
type ProductionCodeValidationResult struct {
	Valid                bool   `json:"valid"`
	Message             string `json:"message"`
	ProductionCodeExists bool   `json:"production_code_exists"`
}
