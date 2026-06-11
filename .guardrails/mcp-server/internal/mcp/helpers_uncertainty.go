package mcp

import (
	"encoding/json"
	"fmt"
	"strings"

	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// containsIgnoreCase checks if a string contains a substring (case-insensitive)
func containsIgnoreCase(s, substr string) bool {
	return strings.Contains(strings.ToLower(s), strings.ToLower(substr))
}

// determineUncertaintyLevel determines the uncertainty level based on self-assessment and context
func (s *MCPServer) determineUncertaintyLevel(selfAssessment string, contextData map[string]interface{}) models.UncertaintyLevel {
	// Check for critical indicators
	criticalIndicators := []string{
		"blocked", "stuck", "cannot", "can't", "impossible", "dead end",
		"critical", "urgent", "emergency", "broken", "error", "fail",
	}
	for _, indicator := range criticalIndicators {
		if containsIgnoreCase(selfAssessment, indicator) || containsIgnoreCase(selfAssessment, indicator) {
			return models.UncertaintyCritical
		}
	}

	// Check for high uncertainty keywords
	highIndicators := []string{
		"major", "significant", "complex", "unsure", "confused", "lost",
		"complicated", "heavy", "uncertain", "doubt", "question",
	}
	highCount := 0
	for _, indicator := range highIndicators {
		if containsIgnoreCase(selfAssessment, indicator) || containsIgnoreCase(selfAssessment, indicator) {
			highCount++
		}
		if highCount >= 2 {
			return models.UncertaintyHigh
		}
	}

	// Check context for blocked signals
	if contextData["error_count"] != nil {
		errorCount := 0
		switch v := contextData["error_count"].(type) {
		case float64:
			errorCount = int(v)
		case int:
			errorCount = v
		}
		if errorCount >= 5 {
			return models.UncertaintyBlocked
		}
		if errorCount >= 3 {
			return models.UncertaintyHigh
		}
	}

	// Check investigation indicators
	investigatingIndicators := []string{
		"research", "explore", "investigate", "look into", "check", "find out",
		"learn", "understand", "discover", "analyze",
	}
	for _, indicator := range investigatingIndicators {
		if containsIgnoreCase(selfAssessment, indicator) {
			return models.UncertaintyInvestigating
		}
	}

	// Check for medium uncertainty
	mediumIndicators := []string{
		"some", "a bit", "somewhat", "kind of", "maybe", "perhaps",
		"moderate", "reasonable", "cautious", "careful",
	}
	for _, indicator := range mediumIndicators {
		if containsIgnoreCase(selfAssessment, indicator) {
			return models.UncertaintyMedium
		}
	}

	// Check for low uncertainty
	lowIndicators := []string{
		"minor", "slight", "small", "little", "slightly", "a little",
		"probably", "likely", "i think", "i believe",
	}
	lowCount := 0
	for _, indicator := range lowIndicators {
		if containsIgnoreCase(selfAssessment, indicator) {
			lowCount++
		}
		if lowCount >= 2 {
			return models.UncertaintyLow
		}
	}

	// Check duration or attempts
	if contextData["duration_minutes"] != nil {
		duration := 0.0
		switch v := contextData["duration_minutes"].(type) {
		case float64:
			duration = v
		case int:
			duration = float64(v)
		}
		if duration > 30 {
			return models.UncertaintyHigh
		}
		if duration > 15 {
			return models.UncertaintyMedium
		}
	}

	// Default to low uncertainty if no strong indicators
	return models.UncertaintyLow
}

// requiresEscalation determines if escalation is required based on level and history
func (s *MCPServer) requiresEscalation(level models.UncertaintyLevel, sessionID string) bool {
	// Critical and blocked always require escalation
	if level == models.UncertaintyCritical || level == models.UncertaintyBlocked {
		return true
	}

	// High uncertainty may require escalation if threshold reached
	if level == models.UncertaintyHigh {
		thresholdReached, _ := s.uncertaintyStore.HasReachedEscalationThreshold(sessionID, 2)
		return thresholdReached
	}

	return false
}

// getUncertaintyGuide retrieves guidance for a specific uncertainty level
func (s *MCPServer) getUncertaintyGuide(level models.UncertaintyLevel) models.UncertaintyLevelGuide {
	guides := models.GetUncertaintyLevelGuides()
	for _, guide := range guides {
		if guide.Level == level {
			return guide
		}
	}
	// Return default resolved guide
	return models.UncertaintyLevelGuide{
		Level:        models.UncertaintyResolved,
		Description:  "Clarity achieved, path forward is clear",
		Action:       "Proceed confidently, document learnings for future reference",
		Threshold:    0,
		RequiresLogs: false,
	}
}

// summarizeContext creates a summary of the context data
func (s *MCPServer) summarizeContext(contextData map[string]interface{}) string {
	summary := "Context analysis:\n"

	// Add duration if available
	if duration, ok := contextData["duration_minutes"]; ok {
		summary += fmt.Sprintf("- Duration: %.1f minutes\n", duration)
	}

	// Add error count if available
	if errorCount, ok := contextData["error_count"]; ok {
		summary += fmt.Sprintf("- Errors encountered: %v\n", errorCount)
	}

	// Add task attempts if available
	if attempts, ok := contextData["task_attempts"]; ok {
		summary += fmt.Sprintf("- Task attempts: %v\n", attempts)
	}

	// Add tool usage if available
	if toolUsage, ok := contextData["tool_usage"]; ok {
		tools, _ := json.Marshal(toolUsage)
		summary += fmt.Sprintf("- Tool usage: %s\n", string(tools))
	}

	// Add file changes if available
	if fileChanges, ok := contextData["file_changes"]; ok {
		summary += fmt.Sprintf("- Files modified: %v\n", fileChanges)
	}

	// Add LLM interactions if available
	if llmCalls, ok := contextData["llm_interactions"]; ok {
		summary += fmt.Sprintf("- LLM interactions: %v\n", llmCalls)
	}

	return summary
}
