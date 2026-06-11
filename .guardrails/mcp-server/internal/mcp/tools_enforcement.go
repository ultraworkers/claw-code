package mcp

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/google/uuid"
	"github.com/mark3labs/mcp-go/mcp"
	"github.com/thearchitectit/guardrail-mcp/internal/database"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// handleLogViolation logs a guardrail violation to the failure registry
func (s *MCPServer) handleLogViolation(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	sessionToken, _ := args["session_token"].(string)
	ruleID, _ := args["rule_id"].(string)
	severity, _ := args["severity"].(string)
	message, _ := args["message"].(string)
	filePath, _ := args["file_path"].(string)

	if sessionToken == "" || ruleID == "" || message == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"error":"session_token, rule_id, and message are required"}`}},
			IsError: true,
		}, nil
	}

	// Validate session
	s.sessionsMu.RLock()
	session, exists := s.sessions[sessionToken]
	s.sessionsMu.RUnlock()

	if !exists {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"error":"Invalid session token"}`}},
			IsError: true,
		}, nil
	}

	// Normalize severity to valid values
	switch severity {
	case "error", "critical":
		severity = "critical"
	case "warning", "high":
		severity = "high"
	case "info", "medium":
		severity = "medium"
	default:
		severity = "low"
	}

	// Build affected files array
	affectedFiles := []string{}
	if filePath != "" {
		affectedFiles = []string{filePath}
	}

	// Create failure entry matching the actual model
	failure := &models.FailureEntry{
		ID:            uuid.New(),
		FailureID:     ruleID,
		Category:      "guardrail_violation",
		Severity:      severity,
		ErrorMessage:  message,
		RootCause:     fmt.Sprintf("Guardrail rule %s violated", ruleID),
		AffectedFiles: models.ToTextArray(affectedFiles),
		Status:        string(models.StatusActive),
		ProjectSlug:   session.ProjectSlug,
		CreatedAt:     time.Now(),
		UpdatedAt:     time.Now(),
	}

	// Validate before saving
	if err := failure.Validate(); err != nil {
		result := map[string]interface{}{
			"error":   fmt.Sprintf("Validation error: %v", err),
			"rule_id": ruleID,
		}
		resultJSON, _ := json.Marshal(result)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
			IsError: true,
		}, nil
	}

	// Persist to database
	failStore := database.NewFailureStore(s.db)
	if err := failStore.Create(ctx, failure); err != nil {
		result := map[string]interface{}{
			"error":   fmt.Sprintf("Failed to log violation: %v", err),
			"rule_id": ruleID,
		}
		resultJSON, _ := json.Marshal(result)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
			IsError: true,
		}, nil
	}

	result := map[string]interface{}{
		"logged":       true,
		"violation_id": failure.ID.String(),
		"rule_id":      ruleID,
		"severity":     severity,
		"project":      session.ProjectSlug,
		"file":         filePath,
	}
	resultJSON, _ := json.Marshal(result)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
	}, nil
}
