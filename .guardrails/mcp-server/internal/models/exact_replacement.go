package models

// ExactReplacementViolation represents a single exact replacement violation
type ExactReplacementViolation struct {
	Type     string `json:"type"`
	Severity string `json:"severity"`
	Message  string `json:"message"`
}

// ExactReplacementValidationResult represents the result of exact replacement validation
type ExactReplacementValidationResult struct {
	ExactMatch       bool                          `json:"exact_match"`
	Violations       []ExactReplacementViolation   `json:"violations,omitempty"`
	DiffStats        DiffStats                     `json:"diff_stats"`
	Recommendation   string                        `json:"recommendation"`
}

// DiffStats represents statistics about the diff
type DiffStats struct {
	Additions int `json:"additions"`
	Deletions int `json:"deletions"`
}
