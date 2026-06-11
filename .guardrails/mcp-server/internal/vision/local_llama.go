package vision

import (
	"bytes"
	"context"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"time"
)

// LocalLlamaClient talks directly to a llama-server OpenAI-compatible endpoint.
type LocalLlamaClient struct {
	baseURL string
	model   string
	client  *http.Client
}

// NewLocalLlamaClient creates a client for the local llama-server.
func NewLocalLlamaClient(baseURL, model string, timeout time.Duration) *LocalLlamaClient {
	if timeout == 0 {
		timeout = 120 * time.Second
	}
	return &LocalLlamaClient{
		baseURL: baseURL,
		model:   model,
		client: &http.Client{
			Timeout: timeout,
		},
	}
}

// ReviewImage implements InferenceClient.
func (c *LocalLlamaClient) ReviewImage(ctx context.Context, imageBase64 string, prompt string, previousFindings []Finding) (*ReviewResponse, error) {
	messages := []map[string]interface{}{
		{
			"role": "user",
			"content": []map[string]interface{}{
				{"type": "text", "text": prompt},
				{"type": "image_url", "image_url": map[string]string{"url": "data:image/png;base64," + imageBase64}},
			},
		},
	}

	// Append previous findings as context if present
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

	var resp *openAIChatResponse
	var lastErr error
	// Simple retry: 3 attempts with exponential backoff
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
		return nil, fmt.Errorf("llama-server failed after 3 attempts: %w", lastErr)
	}

	if len(resp.Choices) == 0 {
		return nil, fmt.Errorf("no choices in response")
	}

	raw := resp.Choices[0].Message.Content
	findings, confidence := parseFindings(raw)

	return &ReviewResponse{
		Findings:    findings,
		Confidence:  confidence,
		RawText:     raw,
		ModelUsed:   c.model,
		BackendUsed: "local_llama",
	}, nil
}

func (c *LocalLlamaClient) doRequest(ctx context.Context, bodyJSON []byte) (*openAIChatResponse, error) {
	req, err := http.NewRequestWithContext(ctx, "POST", c.baseURL+"/chat/completions", bytes.NewReader(bodyJSON))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")

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
func (c *LocalLlamaClient) HealthCheck(ctx context.Context) (HealthStatus, error) {
	req, err := http.NewRequestWithContext(ctx, "GET", c.baseURL+"/models", nil)
	if err != nil {
		return HealthStatus{Backend: "local_llama", Healthy: false, Error: err.Error()}, nil
	}
	r, err := c.client.Do(req)
	if err != nil {
		return HealthStatus{Backend: "local_llama", Healthy: false, Error: err.Error()}, nil
	}
	defer r.Body.Close()

	if r.StatusCode != http.StatusOK {
		return HealthStatus{Backend: "local_llama", Healthy: false, Error: fmt.Sprintf("status %d", r.StatusCode)}, nil
	}

	var models struct {
		Data []struct {
			ID string `json:"id"`
		} `json:"data"`
	}
	if err := json.NewDecoder(r.Body).Decode(&models); err != nil {
		return HealthStatus{Backend: "local_llama", Healthy: false, Error: err.Error()}, nil
	}

	modelLoaded := ""
	if len(models.Data) > 0 {
		modelLoaded = models.Data[0].ID
	}
	return HealthStatus{Backend: "local_llama", Healthy: true, ModelLoaded: modelLoaded}, nil
}

// openAIChatResponse mirrors the OpenAI chat completions response shape.
type openAIChatResponse struct {
	Choices []struct {
		Message struct {
			Content string `json:"content"`
		} `json:"message"`
	} `json:"choices"`
}

// parseFindings extracts structured findings from raw model text.
// This is a lightweight heuristic parser. If the model outputs JSON, we try to decode it.
func parseFindings(raw string) ([]Finding, float64) {
	// Try to find a JSON array in the response
	var findings []Finding
	start := bytes.Index([]byte(raw), []byte("["))
	end := bytes.LastIndex([]byte(raw), []byte("]"))
	if start >= 0 && end > start {
		if err := json.Unmarshal([]byte(raw)[start:end+1], &findings); err == nil {
			return findings, 0.85
		}
	}

	// Fallback: treat entire response as a single finding
	return []Finding{{
		Category:    "general",
		Severity:    "info",
		Description: raw,
	}}, 0.5
}

// EncodeImageToBase64 reads a PNG/JPG file and returns a base64 string suitable for OpenAI vision APIs.
func EncodeImageToBase64(path string) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	return base64.StdEncoding.EncodeToString(data), nil
}
