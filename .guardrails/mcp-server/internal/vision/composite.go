package vision

import (
	"context"
	"fmt"
	"log/slog"
)

// CompositeClient tries local llama first, then promotes to hosted fallbacks.
type CompositeClient struct {
	local      InferenceClient
	fallbacks  []InferenceClient
	log        *slog.Logger
}

// NewCompositeClient creates the orchestrator with a primary and ordered fallbacks.
func NewCompositeClient(local InferenceClient, fallbacks ...InferenceClient) *CompositeClient {
	return &CompositeClient{
		local:     local,
		fallbacks: fallbacks,
		log:       slog.Default().With("component", "composite_client"),
	}
}

// ReviewImage tries local first. On failure or low confidence, promotes through fallbacks.
func (c *CompositeClient) ReviewImage(ctx context.Context, imageBase64 string, prompt string, previousFindings []Finding) (*ReviewResponse, error) {
	// Try local first
	resp, err := c.local.ReviewImage(ctx, imageBase64, prompt, previousFindings)
	if err != nil {
		c.log.Warn("local llama failed, promoting to fallback", "error", err)
		return c.fallbackReview(ctx, imageBase64, prompt, previousFindings)
	}

	// If local succeeded but confidence is very low, consider fallback
	if resp.Confidence < 0.3 {
		c.log.Warn("local confidence too low, trying fallback", "confidence", resp.Confidence)
		fbResp, fbErr := c.fallbackReview(ctx, imageBase64, prompt, previousFindings)
		if fbErr == nil {
			return fbResp, nil
		}
		c.log.Warn("fallback also failed, returning local result", "fallback_error", fbErr)
	}

	return resp, nil
}

func (c *CompositeClient) fallbackReview(ctx context.Context, imageBase64 string, prompt string, previousFindings []Finding) (*ReviewResponse, error) {
	for i, fb := range c.fallbacks {
		resp, err := fb.ReviewImage(ctx, imageBase64, prompt, previousFindings)
		if err != nil {
			c.log.Warn("fallback failed", "index", i, "error", err)
			continue
		}
		return resp, nil
	}
	return nil, fmt.Errorf("all fallbacks exhausted")
}

// HealthCheck reports health for all backends.
func (c *CompositeClient) HealthCheck(ctx context.Context) (HealthStatus, error) {
	localStatus, _ := c.local.HealthCheck(ctx)
	var fbStatus []HealthStatus
	for _, fb := range c.fallbacks {
		s, _ := fb.HealthCheck(ctx)
		fbStatus = append(fbStatus, s)
	}

	if localStatus.Healthy {
		return localStatus, nil
	}
	for _, s := range fbStatus {
		if s.Healthy {
			return s, nil
		}
	}
	return HealthStatus{Backend: "composite", Healthy: false, Error: "all backends unhealthy"}, nil
}
