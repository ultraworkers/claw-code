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

// AnthropicClient sends requests directly to the Anthropic Messages API.
type AnthropicClient struct {
	apiKey  string
	model   string
	client  *http.Client
}

// NewAnthropicClient creates a direct Anthropic API client.
func NewAnthropicClient(apiKey, model string, timeout time.Duration) *AnthropicClient {
	if timeout == 0 {
		timeout = 120 * time.Second
	}
	return &AnthropicClient{
		apiKey: apiKey,
		model:  model,
		client: &http.Client{Timeout: timeout},
	}
}

// ReviewImage implements InferenceClient.
func (c *AnthropicClient) ReviewImage(ctx context.Context, imageBase64 string, prompt string, previousFindings []Finding) (*ReviewResponse, error) {
	content := []map[string]interface{}{
		{"type": "text", "text": prompt},
		{
			"type": "image",
			"source": map[string]interface{}{
				"type":      "base64",
				"media_type": "image/png",
				"data":      imageBase64,
			},
		},
	}

	system := ""
	if len(previousFindings) > 0 {
		system = "Previous findings:\n"
		for _, f := range previousFindings {
			system += fmt.Sprintf("- [%s] %s: %s\n", f.Severity, f.Category, f.Description)
		}
	}

	body := map[string]interface{}{
		"model":      c.model,
		"max_tokens": 4096,
		"messages": []map[string]interface{}{
			{"role": "user", "content": content},
		},
	}
	if system != "" {
		body["system"] = system
	}

	bodyJSON, err := json.Marshal(body)
	if err != nil {
		return nil, fmt.Errorf("marshal request: %w", err)
	}

	var lastErr error
	var resp *anthropicResponse
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
		return nil, fmt.Errorf("anthropic failed after 3 attempts: %w", lastErr)
	}

	if len(resp.Content) == 0 {
		return nil, fmt.Errorf("no content in anthropic response")
	}

	raw := resp.Content[0].Text
	findings, confidence := parseFindings(raw)

	return &ReviewResponse{
		Findings:    findings,
		Confidence:  confidence,
		RawText:     raw,
		ModelUsed:   c.model,
		BackendUsed: "anthropic",
	}, nil
}

func (c *AnthropicClient) doRequest(ctx context.Context, bodyJSON []byte) (*anthropicResponse, error) {
	req, err := http.NewRequestWithContext(ctx, "POST", "https://api.anthropic.com/v1/messages", bytes.NewReader(bodyJSON))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("x-api-key", c.apiKey)
	req.Header.Set("anthropic-version", "2023-06-01")

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

	var resp anthropicResponse
	if err := json.Unmarshal(respBody, &resp); err != nil {
		return nil, fmt.Errorf("unmarshal response: %w", err)
	}
	return &resp, nil
}

// HealthCheck implements InferenceClient.
func (c *AnthropicClient) HealthCheck(ctx context.Context) (HealthStatus, error) {
	req, err := http.NewRequestWithContext(ctx, "GET", "https://api.anthropic.com/v1/models", nil)
	if err != nil {
		return HealthStatus{Backend: "anthropic", Healthy: false, Error: err.Error()}, nil
	}
	req.Header.Set("x-api-key", c.apiKey)
	req.Header.Set("anthropic-version", "2023-06-01")

	r, err := c.client.Do(req)
	if err != nil {
		return HealthStatus{Backend: "anthropic", Healthy: false, Error: err.Error()}, nil
	}
	defer r.Body.Close()

	if r.StatusCode != http.StatusOK {
		return HealthStatus{Backend: "anthropic", Healthy: false, Error: fmt.Sprintf("status %d", r.StatusCode)}, nil
	}
	return HealthStatus{Backend: "anthropic", Healthy: true}, nil
}

type anthropicResponse struct {
	Content []struct {
		Text string `json:"text"`
	} `json:"content"`
}
