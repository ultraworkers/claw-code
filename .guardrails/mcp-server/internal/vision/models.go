package vision

import (
	"encoding/json"
	"time"
)

// ReviewStatus represents the lifecycle state of a visual review.
type ReviewStatus string

const (
	StatusPending    ReviewStatus = "pending"
	StatusReviewing  ReviewStatus = "reviewing"
	StatusCompleted  ReviewStatus = "completed"
	StatusFailed     ReviewStatus = "failed"
	StatusFallback   ReviewStatus = "fallback"
)

// Review represents a single screenshot review session.
type Review struct {
	ID            string       `json:"id" db:"id"`
	ScreenshotPath string      `json:"screenshot_path" db:"screenshot_path"`
	Status        ReviewStatus `json:"status" db:"status"`
	CreatedAt     time.Time    `json:"created_at" db:"created_at"`
	CompletedAt   *time.Time   `json:"completed_at,omitempty" db:"completed_at"`
}

// Iteration represents one round of analysis within a review.
type Iteration struct {
	ID           string          `json:"id" db:"id"`
	ReviewID     string          `json:"review_id" db:"review_id"`
	BackendUsed  string          `json:"backend_used" db:"backend_used"`
	ModelUsed    string          `json:"model_used" db:"model_used"`
	PromptType   string          `json:"prompt_type" db:"prompt_type"`
	RawResponse  string          `json:"raw_response" db:"raw_response"`
	FindingsJSON json.RawMessage `json:"findings_json" db:"findings_json"`
	Confidence   float64         `json:"confidence" db:"confidence"`
	LatencyMs    int64           `json:"latency_ms" db:"latency_ms"`
	CreatedAt    time.Time       `json:"created_at" db:"created_at"`
}

// Bbox represents a 2D bounding box for visual findings.
type Bbox struct {
	X      float64 `json:"x"`
	Y      float64 `json:"y"`
	Width  float64 `json:"width"`
	Height float64 `json:"height"`
}

// Finding represents a single structured observation from an iteration.
type Finding struct {
	ID          string  `json:"id" db:"id"`
	ReviewID    string  `json:"review_id" db:"review_id"`
	Category    string  `json:"category" db:"category"`
	Severity    string  `json:"severity" db:"severity"`
	Description string  `json:"description" db:"description"`
	Bbox        *Bbox   `json:"bbox,omitempty" db:"bbox"`
	Accepted    bool    `json:"accepted" db:"accepted"`
}

// ReviewResponse is the output of a single inference call.
type ReviewResponse struct {
	Findings   []Finding `json:"findings"`
	Confidence float64   `json:"confidence"`
	RawText    string    `json:"raw_text"`
	ModelUsed  string    `json:"model_used"`
	BackendUsed string   `json:"backend_used"`
}

// HealthStatus reports the health of an inference backend.
type HealthStatus struct {
	Backend   string `json:"backend"`
	Healthy   bool   `json:"healthy"`
	ModelLoaded string `json:"model_loaded,omitempty"`
	Error     string `json:"error,omitempty"`
}

// Report is the fully documented result of a review.
type Report struct {
	Review     Review      `json:"review"`
	Iterations []Iteration `json:"iterations"`
	Findings   []Finding   `json:"findings"`
}
