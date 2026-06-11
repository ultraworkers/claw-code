package vision

import "context"

// InferenceClient defines the contract for vision model backends.
type InferenceClient interface {
	// ReviewImage sends an image (base64-encoded PNG/JPG) with a prompt for analysis.
	// previousFindings may be nil for the initial review.
	ReviewImage(ctx context.Context, imageBase64 string, prompt string, previousFindings []Finding) (*ReviewResponse, error)

	// HealthCheck returns the current health of this backend.
	HealthCheck(ctx context.Context) (HealthStatus, error)
}
