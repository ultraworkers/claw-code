package vision

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"log/slog"
	"time"
)

// ReviewEngine orchestrates the iterative vision review loop.
type ReviewEngine struct {
	client     InferenceClient
	storage    *Storage
	maxIter    int
	threshold  float64
	systemPrompt string
	fallbackPrompt string
	log        *slog.Logger
}

// ReviewEngineConfig holds tuning parameters.
type ReviewEngineConfig struct {
	MaxIterations    int
	ConfidenceThreshold float64
	SystemPrompt     string
	FallbackPrompt   string
}

// NewReviewEngine creates the engine with an inference client and storage.
func NewReviewEngine(client InferenceClient, storage *Storage, cfg ReviewEngineConfig) *ReviewEngine {
	if cfg.MaxIterations <= 0 {
		cfg.MaxIterations = 3
	}
	if cfg.ConfidenceThreshold <= 0 {
		cfg.ConfidenceThreshold = 0.75
	}
	return &ReviewEngine{
		client:         client,
		storage:        storage,
		maxIter:        cfg.MaxIterations,
		threshold:      cfg.ConfidenceThreshold,
		systemPrompt:   cfg.SystemPrompt,
		fallbackPrompt: cfg.FallbackPrompt,
		log:            slog.Default().With("component", "review_engine"),
	}
}

// Run performs the full review lifecycle for a screenshot.
func (e *ReviewEngine) Run(ctx context.Context, screenshotPath string) (*Report, error) {
	reviewID := genID()
	review := &Review{
		ID:             reviewID,
		ScreenshotPath: screenshotPath,
		Status:         StatusReviewing,
		CreatedAt:      time.Now(),
	}
	if err := e.storage.CreateReview(review); err != nil {
		return nil, fmt.Errorf("create review: %w", err)
	}

	imageB64, err := EncodeImageToBase64(screenshotPath)
	if err != nil {
		_ = e.storage.UpdateReviewStatus(reviewID, StatusFailed, nil)
		return nil, fmt.Errorf("encode image: %w", err)
	}

	var allFindings []Finding
	var iterations []Iteration

	for i := 0; i < e.maxIter; i++ {
		prompt := e.systemPrompt
		promptType := "initial"
		if i > 0 {
			prompt = e.fallbackPrompt
			promptType = "re_review"
		}

		start := time.Now()
		resp, err := e.client.ReviewImage(ctx, imageB64, prompt, allFindings)
		latency := time.Since(start).Milliseconds()

		iter := &Iteration{
			ID:         genID(),
			ReviewID:   reviewID,
			BackendUsed: resp.BackendUsed,
			ModelUsed:   resp.ModelUsed,
			PromptType:  promptType,
			RawResponse: resp.RawText,
			Confidence:  resp.Confidence,
			LatencyMs:   latency,
			CreatedAt:   time.Now(),
		}
		if err != nil {
			iter.RawResponse = err.Error()
			iter.Confidence = 0
			_ = e.storage.CreateIteration(iter)
			e.log.Error("review iteration failed", "iteration", i, "error", err)
			break
		}

		findingsJSON, _ := json.Marshal(resp.Findings)
		iter.FindingsJSON = findingsJSON
		_ = e.storage.CreateIteration(iter)
		iterations = append(iterations, *iter)

		// Merge findings, deduplicating by description
		for _, f := range resp.Findings {
			if !findingExists(allFindings, f.Description) {
				f.ID = genID()
				f.ReviewID = reviewID
				_ = e.storage.CreateFinding(&f)
				allFindings = append(allFindings, f)
			}
		}

		if resp.Confidence >= e.threshold {
			e.log.Info("review confidence threshold met", "confidence", resp.Confidence, "iterations", i+1)
			break
		}
	}

	completedAt := time.Now()
	status := StatusCompleted
	if len(allFindings) == 0 {
		status = StatusFailed
	}
	_ = e.storage.UpdateReviewStatus(reviewID, status, &completedAt)

	return &Report{
		Review:     *review,
		Iterations: iterations,
		Findings:   allFindings,
	}, nil
}

// Iterate runs another review round on an existing review ID.
func (e *ReviewEngine) Iterate(ctx context.Context, reviewID string) (*Report, error) {
	review, err := e.storage.GetReview(reviewID)
	if err != nil {
		return nil, fmt.Errorf("get review: %w", err)
	}

	findings, err := e.storage.ListFindings(reviewID)
	if err != nil {
		return nil, fmt.Errorf("list findings: %w", err)
	}

	imageB64, err := EncodeImageToBase64(review.ScreenshotPath)
	if err != nil {
		return nil, fmt.Errorf("encode image: %w", err)
	}

	start := time.Now()
	resp, err := e.client.ReviewImage(ctx, imageB64, e.fallbackPrompt, findings)
	latency := time.Since(start).Milliseconds()

	iter := &Iteration{
		ID:         genID(),
		ReviewID:   reviewID,
		BackendUsed: resp.BackendUsed,
		ModelUsed:   resp.ModelUsed,
		PromptType:  "manual_iterate",
		RawResponse: resp.RawText,
		Confidence:  resp.Confidence,
		LatencyMs:   latency,
		CreatedAt:   time.Now(),
	}
	if err != nil {
		iter.RawResponse = err.Error()
		iter.Confidence = 0
	}
	findingsJSON, _ := json.Marshal(resp.Findings)
	iter.FindingsJSON = findingsJSON
	_ = e.storage.CreateIteration(iter)

	for _, f := range resp.Findings {
		if !findingExists(findings, f.Description) {
			f.ID = genID()
			f.ReviewID = reviewID
			_ = e.storage.CreateFinding(&f)
			findings = append(findings, f)
		}
	}

	iterations, _ := e.storage.ListIterations(reviewID)
	return &Report{
		Review:     *review,
		Iterations: iterations,
		Findings:   findings,
	}, nil
}

// ListReviews returns recent reviews.
func (e *ReviewEngine) ListReviews(limit int) ([]Review, error) {
	return e.storage.ListReviews(limit)
}

// GetReport loads the full report for a review.
func (e *ReviewEngine) GetReport(reviewID string) (*Report, error) {
	review, err := e.storage.GetReview(reviewID)
	if err != nil {
		return nil, err
	}
	iterations, err := e.storage.ListIterations(reviewID)
	if err != nil {
		return nil, err
	}
	findings, err := e.storage.ListFindings(reviewID)
	if err != nil {
		return nil, err
	}
	return &Report{
		Review:     *review,
		Iterations: iterations,
		Findings:   findings,
	}, nil
}

// HealthCheck delegates to the inference client.
func (e *ReviewEngine) HealthCheck(ctx context.Context) (HealthStatus, error) {
	return e.client.HealthCheck(ctx)
}

func findingExists(list []Finding, description string) bool {
	for _, f := range list {
		if f.Description == description {
			return true
		}
	}
	return false
}

func genID() string {
	b := make([]byte, 8)
	_, _ = rand.Read(b)
	return hex.EncodeToString(b)
}
