package models

import (
	"time"

	"github.com/google/uuid"
)

// FixType represents the type of fix applied
type FixType string

const (
	FixTypeRegex      FixType = "regex"
	FixTypeCodeChange FixType = "code_change"
	FixTypeConfig     FixType = "config"
)

// VerificationStatus represents the status of a fix verification
type VerificationStatus string

const (
	StatusConfirmed VerificationStatus = "confirmed"
	StatusModified  VerificationStatus = "modified"
	StatusRemoved   VerificationStatus = "removed"
)

// FixVerification represents a fix verification tracking entry
type FixVerification struct {
	ID                 uuid.UUID          `json:"id" db:"id"`
	SessionID          string             `json:"session_id" db:"session_id"`
	FailureID          string             `json:"failure_id" db:"failure_id"`
	FixHash            string             `json:"fix_hash" db:"fix_hash"`
	FilePath           string             `json:"file_path" db:"file_path"`
	FixContent         string             `json:"fix_content" db:"fix_content"`
	FixType            FixType            `json:"fix_type" db:"fix_type"`
	VerifiedAt         *time.Time         `json:"verified_at,omitempty" db:"verified_at"`
	VerificationStatus VerificationStatus `json:"verification_status" db:"verification_status"`
	CreatedAt          time.Time          `json:"created_at" db:"created_at"`
}

// FixVerificationResult represents the result of verifying if fixes are intact
type FixVerificationResult struct {
	AllFixesIntact   bool                       `json:"all_fixes_intact"`
	VerifySummary    string                     `json:"verify_summary"`
	Fixes            []IndividualFixResult      `json:"fixes"`
	Recommendation   string                     `json:"recommendation"`
}

// IndividualFixResult represents the result for a single fix
type IndividualFixResult struct {
	FailureID           string             `json:"failure_id"`
	Status              VerificationStatus `json:"status"`
	FixType             FixType            `json:"fix_type"`
	AffectedFile        string             `json:"affected_file"`
	VerificationMessage string             `json:"verification_message"`
}

// IsValidFixType checks if a fix type is valid
func IsValidFixType(fixType string) bool {
	switch FixType(fixType) {
	case FixTypeRegex, FixTypeCodeChange, FixTypeConfig:
		return true
	}
	return false
}

// IsValidVerificationStatus checks if a verification status is valid
func IsValidVerificationStatus(status string) bool {
	switch VerificationStatus(status) {
	case StatusConfirmed, StatusModified, StatusRemoved:
		return true
	}
	return false
}
