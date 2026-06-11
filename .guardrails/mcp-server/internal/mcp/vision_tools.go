package mcp

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"os"
	"time"

	"github.com/mark3labs/mcp-go/mcp"
	"github.com/thearchitectit/guardrail-mcp/internal/vision"
)

// VisionToolSet holds the vision pipeline components and registers vision MCP tools.
type VisionToolSet struct {
	engine   *vision.ReviewEngine
	watcher  *vision.CaptureWatcher
	enabled  bool
	log      *slog.Logger
}

// NewVisionToolSet initializes vision components from environment.
// Returns nil (and no error) if vision is not enabled, so the caller can skip registration.
func NewVisionToolSet() (*VisionToolSet, error) {
	enabled := os.Getenv("VISION_ENABLED") == "true"
	if !enabled {
		return nil, nil
	}

	localURL := os.Getenv("LOCAL_LLAMA_URL")
	if localURL == "" {
		localURL = "http://localhost:8080/v1"
	}
	localModel := os.Getenv("LOCAL_LLAMA_MODEL")
	if localModel == "" {
		localModel = "nemotron-vision-local"
	}

	fallbackProvider := os.Getenv("FALLBACK_PROVIDER")
	fallbackModel := os.Getenv("FALLBACK_MODEL")
	fallbackKey := os.Getenv("FALLBACK_API_KEY")

	screenshotDir := os.Getenv("SCREENSHOT_DIR")
	if screenshotDir == "" {
		screenshotDir = "./screenshots"
	}

	dbPath := os.Getenv("VISION_DB_PATH")
	if dbPath == "" {
		dbPath = "./vision_reviews.db"
	}

	storage, err := vision.NewStorage(dbPath)
	if err != nil {
		return nil, fmt.Errorf("vision storage: %w", err)
	}

	localClient := vision.NewLocalLlamaClient(localURL, localModel, 120*time.Second)

	var fallbacks []vision.InferenceClient
	if fallbackProvider == "anthropic" && fallbackKey != "" {
		fallbacks = append(fallbacks, vision.NewAnthropicClient(fallbackKey, fallbackModel, 120*time.Second))
	} else if fallbackProvider == "openai" && fallbackKey != "" {
		fallbacks = append(fallbacks, vision.NewOpenAIClient(fallbackKey, fallbackModel, 120*time.Second))
	}

	composite := vision.NewCompositeClient(localClient, fallbacks...)

	cfg := vision.ReviewEngineConfig{
		MaxIterations:         3,
		ConfidenceThreshold:   0.75,
		SystemPrompt:          "You are a 3D game QA engineer. Analyze the provided screenshot and report issues related to visual quality, performance, and adherence to game design standards.",
		FallbackPrompt:        "Re-examine this image focusing on any issues you may have missed. Be specific about visual artifacts, lighting problems, UI inconsistencies, or performance indicators.",
	}
	engine := vision.NewReviewEngine(composite, storage, cfg)

	watcher, err := vision.NewCaptureWatcher(screenshotDir, engine)
	if err != nil {
		return nil, fmt.Errorf("capture watcher: %w", err)
	}

	return &VisionToolSet{
		engine:  engine,
		watcher: watcher,
		enabled: true,
		log:     slog.Default().With("component", "vision_tools"),
	}, nil
}

// Start begins auto-capture watching.
func (vt *VisionToolSet) Start(ctx context.Context) {
	if vt == nil || !vt.enabled {
		return
	}
	vt.watcher.Start(ctx)
}

// Stop halts auto-capture.
func (vt *VisionToolSet) Stop() error {
	if vt == nil || !vt.enabled {
		return nil
	}
	return vt.watcher.Stop()
}

// Engine returns the review engine (may be nil if not enabled).
func (vt *VisionToolSet) Engine() *vision.ReviewEngine {
	if vt == nil {
		return nil
	}
	return vt.engine
}

// Watcher returns the capture watcher (may be nil if not enabled).
func (vt *VisionToolSet) Watcher() *vision.CaptureWatcher {
	if vt == nil {
		return nil
	}
	return vt.watcher
}

// visionToolList returns the vision tool definitions for registration.
func (vt *VisionToolSet) visionToolList() []mcp.Tool {
	if vt == nil || !vt.enabled {
		return nil
	}
	return []mcp.Tool{
		{
			Name:        "vision_capture_screenshot",
			Description: "Trigger an immediate screenshot capture from the running Godot game",
			InputSchema: mcp.ToolInputSchema{
				Type: "object",
				Properties: mcp.ToolInputSchemaProperties{},
			},
		},
		{
			Name:        "vision_analyze_screenshot",
			Description: "Submit a screenshot image path for vision analysis. Returns structured findings.",
			InputSchema: mcp.ToolInputSchema{
				Type: "object",
				Properties: mcp.ToolInputSchemaProperties{
					"path": map[string]interface{}{
						"type":        "string",
						"description": "Absolute path to the screenshot file (PNG or JPG)",
					},
				},
			},
		},
		{
			Name:        "vision_iterate_review",
			Description: "Run another review round on an existing review ID with additional context",
			InputSchema: mcp.ToolInputSchema{
				Type: "object",
				Properties: mcp.ToolInputSchemaProperties{
					"review_id": map[string]interface{}{
						"type":        "string",
						"description": "Review ID from a previous analyze_screenshot call",
					},
				},
			},
		},
		{
			Name:        "vision_get_report",
			Description: "Retrieve the full documented report for a review including all findings and iterations",
			InputSchema: mcp.ToolInputSchema{
				Type: "object",
				Properties: mcp.ToolInputSchemaProperties{
					"review_id": map[string]interface{}{
						"type":        "string",
						"description": "Review ID",
					},
				},
			},
		},
		{
			Name:        "vision_check_health",
			Description: "Check the health of the vision pipeline backends (local llama + fallbacks)",
			InputSchema: mcp.ToolInputSchema{
				Type:       "object",
				Properties: mcp.ToolInputSchemaProperties{},
			},
		},
		{
			Name:        "vision_guardrail_check",
			Description: "Full visual guardrail check: capture → review → document → validate against 3D guardrails",
			InputSchema: mcp.ToolInputSchema{
				Type: "object",
				Properties: mcp.ToolInputSchemaProperties{
					"path": map[string]interface{}{
						"type":        "string",
						"description": "Optional path to screenshot. If omitted, triggers capture first.",
					},
				},
			},
		},
	}
}

// dispatch handles vision tool execution.
func (vt *VisionToolSet) dispatch(ctx context.Context, name string, args map[string]interface{}) (*mcp.CallToolResult, error) {
	if vt == nil || !vt.enabled {
		return nil, fmt.Errorf("vision tools not enabled")
	}

	switch name {
	case "vision_capture_screenshot":
		msg := vt.watcher.TriggerCapture()
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: msg}},
		}, nil

	case "vision_analyze_screenshot":
		path, _ := args["path"].(string)
		if path == "" {
			return errorResult("path is required"), nil
		}
		report, err := vt.engine.Run(ctx, path)
		if err != nil {
			return errorResult(err.Error()), nil
		}
		out, _ := json.MarshalIndent(report, "", "  ")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: string(out)}},
		}, nil

	case "vision_iterate_review":
		reviewID, _ := args["review_id"].(string)
		if reviewID == "" {
			return errorResult("review_id is required"), nil
		}
		report, err := vt.engine.Iterate(ctx, reviewID)
		if err != nil {
			return errorResult(err.Error()), nil
		}
		out, _ := json.MarshalIndent(report, "", "  ")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: string(out)}},
		}, nil

	case "vision_get_report":
		reviewID, _ := args["review_id"].(string)
		if reviewID == "" {
			return errorResult("review_id is required"), nil
		}
		report, err := vt.engine.GetReport(reviewID)
		if err != nil {
			return errorResult(err.Error()), nil
		}
		out, _ := json.MarshalIndent(report, "", "  ")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: string(out)}},
		}, nil

	case "vision_check_health":
		status, err := vt.engine.HealthCheck(ctx)
		if err != nil {
			return errorResult(err.Error()), nil
		}
		out, _ := json.MarshalIndent(status, "", "  ")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: string(out)}},
		}, nil

	case "vision_guardrail_check":
		path, _ := args["path"].(string)
		if path == "" {
			path = vt.watcher.TriggerCapture()
			// Small delay if we just triggered capture
			time.Sleep(2 * time.Second)
		}
		report, err := vt.engine.Run(ctx, path)
		if err != nil {
			return errorResult(err.Error()), nil
		}
		// Append guardrail validation summary
		validation := validateAgainst3DGuardrails(report.Findings)
		out, _ := json.MarshalIndent(map[string]interface{}{
			"report":     report,
			"validation": validation,
		}, "", "  ")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: string(out)}},
		}, nil

	default:
		return nil, fmt.Errorf("unknown vision tool: %s", name)
	}
}

func errorResult(msg string) *mcp.CallToolResult {
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: msg}},
		IsError: true,
	}
}

func validateAgainst3DGuardrails(findings []vision.Finding) map[string]interface{} {
	var issues []string
	for _, f := range findings {
		if f.Severity == "critical" || f.Severity == "high" {
			issues = append(issues, fmt.Sprintf("[%s] %s: %s", f.Severity, f.Category, f.Description))
		}
	}
	passed := len(issues) == 0
	return map[string]interface{}{
		"passed":    passed,
		"issues":    issues,
		"rule_set":  "3D_GAME_DEVELOPMENT.md",
		"checked_at": time.Now().UTC(),
	}
}
