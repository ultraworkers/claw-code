package models

import (
	"time"
)

// CheckResult represents the result of a pre-work check
type CheckResult struct {
	Passed        bool           `json:"passed"`
	Checks        []FailureCheck `json:"checks"`
	FilesAffected []string       `json:"files_affected"`
	Summary       string         `json:"summary,omitempty"`
}

// FailureCheck represents a single check result from the failure registry
type FailureCheck struct {
	FailureID         string   `json:"failure_id"`
	Category          string   `json:"category"`
	Severity          string   `json:"severity"`
	Message           string   `json:"message"`
	RootCause         string   `json:"root_cause,omitempty"`
	AffectedFiles     []string `json:"affected_files,omitempty"`
	RegressionPattern string   `json:"regression_pattern,omitempty"`
}

// CommitValidationResult represents the result of validating a commit message
type CommitValidationResult struct {
	Valid            bool     `json:"valid"`
	FormatCompliant  bool     `json:"format_compliant"`
	Issues           []string `json:"issues,omitempty"`
	Message          string   `json:"message,omitempty"`
	ConventionalType string   `json:"conventional_type,omitempty"`
	Scope            string   `json:"scope,omitempty"`
}

// ScopeValidationResult represents the result of validating a file scope
type ScopeValidationResult struct {
	Valid        bool   `json:"valid"`
	Message      string `json:"message"`
	FilePath     string `json:"file_path"`
	Scope        string `json:"scope"`
	OutsideScope bool   `json:"outside_scope,omitempty"`
}

// RegressionCheckResult represents the result of a regression check
type RegressionCheckResult struct {
	Matches []RegressionMatch `json:"matches"`
	Checked int               `json:"checked"`
}

// RegressionMatch represents a single regression pattern match
type RegressionMatch struct {
	FailureID         string   `json:"failure_id"`
	Category          string   `json:"category"`
	Severity          string   `json:"severity"`
	Message           string   `json:"message"`
	RootCause         string   `json:"root_cause"`
	RegressionPattern string   `json:"regression_pattern"`
	AffectedFiles     []string `json:"affected_files"`
}

// TestProdSeparationResult represents the result of test/production separation check
type TestProdSeparationResult struct {
	Valid       bool     `json:"valid"`
	Violations  []string `json:"violations,omitempty"`
	FilePath    string   `json:"file_path"`
	Environment string   `json:"environment"`
}

// PushValidationResult represents the result of validating a git push
type PushValidationResult struct {
	Valid    bool     `json:"valid"`
	CanPush  bool     `json:"can_push"`
	Warnings []string `json:"warnings,omitempty"`
	Branch   string   `json:"branch"`
	IsForce  bool     `json:"is_force"`
}

// FileReadVerificationResult represents the result of verifying if a file was read
type FileReadVerificationResult struct {
	Valid     bool   `json:"valid"`
	WasRead   bool   `json:"was_read"`
	ReadAt    string `json:"read_at,omitempty"`
	Message   string `json:"message,omitempty"`
	SessionID string `json:"session_id"`
	FilePath  string `json:"file_path"`
}

// MetaInfo contains metadata about the validation (used by some handlers)
type MetaInfo struct {
	CheckedAt      time.Time `json:"checked_at"`
	RulesEvaluated int       `json:"rules_evaluated"`
	DurationMs     int       `json:"duration_ms"`
	Command        string    `json:"command,omitempty"`
	File           string    `json:"file,omitempty"`
	ChangesSize    int       `json:"changes_size,omitempty"`
}

// FeatureCreepViolation represents a single feature creep violation
type FeatureCreepViolation struct {
	Type     string `json:"type"`
	Severity string `json:"severity"`
	Message  string `json:"message"`
}

// FeatureCreepDetectionResult represents the result of feature creep detection
type FeatureCreepDetectionResult struct {
	CreepDetected bool                    `json:"creep_detected"`
	Violations    []FeatureCreepViolation `json:"violations,omitempty"`
	DiffSummary   string                  `json:"diff_summary"`
	TotalChanges  struct {
		Additions int `json:"additions"`
		Deletions int `json:"deletions"`
	} `json:"total_changes"`
	Recommendation string `json:"recommendation"`
}

// TestValidationResult represents the result of verifying tests before commit
type TestValidationResult struct {
	Valid       bool     `json:"valid"`
	Passed      bool     `json:"passed"`
	Message     string   `json:"message"`
	FailedTests []string `json:"failed_tests,omitempty"`
	CoverageMet bool     `json:"coverage_met,omitempty"`
}

// PayloadFinding represents a single finding from commit payload scanning
type PayloadFinding struct {
	File        string `json:"file"`
	Type        string `json:"type"` // secret, binary, generated, large
	Severity    string `json:"severity"`
	Description string `json:"description"`
	LineNumber  int    `json:"line_number,omitempty"`
}

// PayloadScanResult represents the result of scanning commit payload
type PayloadScanResult struct {
	Valid    bool             `json:"valid"`
	Clean    bool             `json:"clean"`
	Message  string           `json:"message"`
	Findings []PayloadFinding `json:"findings,omitempty"`
	Scanned  int              `json:"scanned"`
}

// ConflictFinding represents a single merge conflict finding
type ConflictFinding struct {
	File       string `json:"file"`
	LineNumber int    `json:"line_number"`
	Context    string `json:"context"`
}

// MergeConflictResult represents the result of detecting merge conflicts
type MergeConflictResult struct {
	Valid     bool              `json:"valid"`
	Clean     bool              `json:"clean"`
	Message   string            `json:"message"`
	Conflicts []ConflictFinding `json:"conflicts,omitempty"`
	Checked   int               `json:"checked"`
}
