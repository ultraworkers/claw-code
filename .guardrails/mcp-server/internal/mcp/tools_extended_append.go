package mcp

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/google/uuid"
	mcp "github.com/mark3labs/mcp-go/mcp"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// handleCheckUncertainty checks the uncertainty level and provides guidance
func (s *MCPServer) handleCheckUncertainty(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	// Extract parameters
	sessionToken, _ := args["session_token"].(string)
	currentTask, _ := args["current_task"].(string)
	selfAssessment, _ := args["self_assessment"].(string)

	// Parse context data if available
	var contextData map[string]interface{}
	if context, ok := args["context_data"].(map[string]interface{}); ok {
		contextData = context
	} else {
		contextData = make(map[string]interface{})
	}

	// Validate required parameters
	if sessionToken == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"error": "Missing required parameter 'session_token'"}`}},
			IsError: true,
		}, nil
	}
	if currentTask == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"error": "Missing required parameter 'current_task'"}`}},
			IsError: true,
		}, nil
	}
	if selfAssessment == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"error": "Missing required parameter 'self_assessment'"}`}},
			IsError: true,
		}, nil
	}

	// Use session token as session ID for now (simplified - no validation)
	sessionID := sessionToken

	// Get previous uncertainty level
	var previousLevel models.UncertaintyLevel = models.UncertaintyResolved
	prevRecord, _ := s.uncertaintyStore.GetLatestUncertainty(sessionID)
	if prevRecord != nil {
		previousLevel = prevRecord.UncertaintyLevel
	}

	// Determine current uncertainty level based on self-assessment
	currentLevel := s.determineUncertaintyLevel(selfAssessment, contextData)

	// Create uncertainty record
	contextJSON, _ := json.Marshal(contextData)
	record := &models.UncertaintyRecord{
		ID:                 uuid.New().String(),
		SessionID:          sessionID,
		UncertaintyLevel:   currentLevel,
		DecisionMade:       selfAssessment,
		ContextData:        json.RawMessage(contextJSON),
		EscalationRequired: s.requiresEscalation(currentLevel, sessionID),
		CreatedAt:          time.Now(),
	}

	// Save uncertainty record
	if err := s.uncertaintyStore.SaveUncertaintyRecord(record); err != nil {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf(`{"error": "Failed to save uncertainty record: %v"}`, err)}},
			IsError: true,
		}, nil
	}

	// Check escalation threshold
	thresholdReached, _ := s.uncertaintyStore.HasReachedEscalationThreshold(sessionID, 3)

	// Prepare guidance based on uncertainty level
	guide := s.getUncertaintyGuide(currentLevel)
	escalated := record.EscalationRequired || thresholdReached

	// Build uncertainty result
	result := models.UncertaintyCheckResult{
		SessionID:      sessionID,
		CurrentLevel:   currentLevel,
		PreviousLevel:  previousLevel,
		Escalated:      escalated,
		DecisionMade:   selfAssessment,
		ContextSummary: s.summarizeContext(contextData),
		Recommendation: guide.Action,
	}

	return buildToolResult(result, !escalated)
}
