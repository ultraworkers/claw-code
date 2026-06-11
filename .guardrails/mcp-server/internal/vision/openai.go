package vision

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

// OpenAIClient sends requests directly to the OpenAI Chat Completions API.
type OpenAIClient struct {
	apiKey string
	model  string
	client *http.Client
}

// NewOpenAIClient creates a direct OpenAI API client.
func NewOpenAIClient(apiKey, model string, timeout time.Duration) *OpenAIClient {
	if timeout == 0 {
		timeout = 120 * time.Second
	}
	return &OpenAIClient{
		apiKey: apiKey,
		model:  model,
		client: &http.Client{Timeout: timeout},
	}
}

// ReviewImage implements InferenceClient.
func (c *OpenAIClient) ReviewImage(ctx context.Context, imageBase64 string, prompt string, previousFindings []Finding) (*ReviewResponse, error) {
	messages := []map[string]interface{}{
		{
			"role": "user",
			"content": []map[string]interface{}{
				{"type": "text", "text": prompt},
				{"type": "image_url", "image_url": map[string]string{"url": "data:image/png;base64," + imageBase64}},
			},
		},
	}

	if len(previousFindings) > 0 {
		contextText := "Previous findings:\n"
		for _, f := range previousFindings {
			contextText += fmt.Sprintf("- [%s] %s: %s\n", f.Severity, f.Category, f.Description)
		}
		messages = append([]map[string]interface{}{
			{"role": "system", "content": contextText},
		}, messages...)
	}

	body := map[string]interface{}{
		"model":    c.model,
		"messages": messages,
	}

	bodyJSON, err := json.Marshal(body)
	if err != nil {
		return nil, fmt.Errorf("marshal request: %w", err)
	}

	var lastErr error
	var resp *openAIChatResponse
	for attempt := 0; attempt < 3; attempt++ {
		if attempt > 0 {
			time.Sleep(time.Duration(attempt) * 2 * time.Second)
		}
		resp, lastErr = c.doRequest(ctx, bodyJSON)
		if lastErr == nil {
			break
		}
	}
	if lastErr != nil {
		return nil, fmt.Errorf("openai failed after 3 attempts: %w", lastErr)
	}

	if len(resp.Choices) == 0 {
		return nil, fmt.Errorf("no choices in openai response")
	}

	raw := resp.Choices[0].Message.Content
	findings, confidence := parseFindings(raw)

	return &ReviewResponse{
		Findings:    findings,
		Confidence:  confidence,
		RawText:     raw,
		ModelUsed:   c.model,
		BackendUsed: "openai",
	}, nil
}

func (c *OpenAIClient) doRequest(ctx context.Context, bodyJSON []byte) (*openAIChatResponse, error) {
	req, err := http.NewRequestWithContext(ctx, "POST", "https://api.openai.com/v1/chat/completions", bytes.NewReader(bodyJSON))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+c.apiKey)

	r, err := c.client.Do(req)
	if err != nil {
		return nil, err
	}
	defer r.Body.Close()

	respBody, err := io.ReadAll(r.Body)
	if err != nil {
		return nil, err
	}

	if r.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("status %d: %s", r.StatusCode, string(respBody))
	}

	var resp openAIChatResponse
	if err := json.Unmarshal(respBody, &resp); err != nil {
		return nil, fmt.Errorf("unmarshal response: %w", err)
	}
	return &resp, nil
}

// HealthCheck implements InferenceClient.
func (c *OpenAIClient) HealthCheck(ctx context.Context) (HealthStatus, error) {
	req, err := http.NewRequestWithContext(ctx, "GET", "https://api.openai.com/v1/models", nil)
	if err != nil {
		return HealthStatus{Backend: "openai", Healthy: false, Error: err.Error()}, nil
	}
	req.Header.Set("Authorization", "Bearer "+c.apiKey)

	r, err := c.client.Do(req)
	if err != nil {
		return HealthStatus{Backend: "openai", Healthy: false, Error: err.Error()}, nil
	}
	defer r.Body.Close()

	if r.StatusCode != http.StatusOK {
		return HealthStatus{Backend: "openai", Healthy: false, Error: fmt.Sprintf("status %d", r.StatusCode)}, nil
	}
	return HealthStatus{Backend: "openai", Healthy: true}, nil
}
