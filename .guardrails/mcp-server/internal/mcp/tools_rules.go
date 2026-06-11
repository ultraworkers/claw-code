package mcp

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/mark3labs/mcp-go/mcp"
)

// PatternRule represents a prevention rule from pattern-rules.json
type PatternRule struct {
	RuleID           string   `json:"rule_id"`
	Name             string   `json:"name"`
	Enabled          bool     `json:"enabled"`
	Pattern          string   `json:"pattern"`
	ForbiddenContext string   `json:"forbidden_context"`
	Message          string   `json:"message"`
	Severity         string   `json:"severity"`
	FileGlob         []string `json:"file_glob"`
	Suggestion       string   `json:"suggestion"`
}

// RulesFile represents the pattern-rules.json structure
type RulesFile struct {
	Version string        `json:"version"`
	Rules   []PatternRule `json:"rules"`
}

// loadPatternRules loads the pattern rules from the guardrails repo
func (s *MCPServer) loadPatternRules() ([]PatternRule, error) {
	repoPath := s.getRepoPath()
	rulesPath := filepath.Join(repoPath, ".guardrails", "prevention-rules", "pattern-rules.json")

	data, err := os.ReadFile(rulesPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read pattern rules: %w", err)
	}

	var rulesFile RulesFile
	if err := json.Unmarshal(data, &rulesFile); err != nil {
		return nil, fmt.Errorf("failed to parse pattern rules: %w", err)
	}

	// Filter to enabled rules only
	enabled := make([]PatternRule, 0, len(rulesFile.Rules))
	for _, r := range rulesFile.Rules {
		if r.Enabled {
			enabled = append(enabled, r)
		}
	}

	return enabled, nil
}

// handleGetPreventionRules returns pattern + semantic rules for a file type
func (s *MCPServer) handleGetPreventionRules(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	fileGlob, _ := args["file_glob"].(string)

	rules, err := s.loadPatternRules()
	if err != nil {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf(`{"error":"%v"}`, err)}},
			IsError: true,
		}, nil
	}

	// Filter by file_glob if specified
	if fileGlob != "" {
		filtered := make([]PatternRule, 0)
		for _, r := range rules {
			for _, g := range r.FileGlob {
				if g == "*" || g == fileGlob || strings.EqualFold(filepath.Ext(g), filepath.Ext(fileGlob)) {
					filtered = append(filtered, r)
					break
				}
			}
		}
		rules = filtered
	}

	resultJSON, _ := json.Marshal(map[string]interface{}{
		"rules": rules,
		"total": len(rules),
	})
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
	}, nil
}

// handleCheckPattern checks a code string against pattern rules
func (s *MCPServer) handleCheckPattern(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	content, _ := args["content"].(string)
	filePath, _ := args["file_path"].(string)

	if content == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"error":"content parameter required"}`}},
			IsError: true,
		}, nil
	}

	rules, err := s.loadPatternRules()
	if err != nil {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf(`{"error":"%v"}`, err)}},
			IsError: true,
		}, nil
	}

	type violation struct {
		RuleID     string `json:"rule_id"`
		RuleName   string `json:"rule_name"`
		Severity   string `json:"severity"`
		Message    string `json:"message"`
		Suggestion string `json:"suggestion,omitempty"`
	}

	violations := []violation{}

	for _, rule := range rules {
		// Check if file matches the rule's file glob
		matched := false
		for _, g := range rule.FileGlob {
			if g == "*" {
				matched = true
				break
			}
			if filePath != "" {
				ext := filepath.Ext(filePath)
				ruleExt := filepath.Ext(g)
				if ext != "" && strings.EqualFold(ext, ruleExt) {
					matched = true
					break
				}
				if strings.EqualFold(filepath.Base(filePath), g) {
					matched = true
					break
				}
			}
		}
		if !matched {
			continue
		}

		// Compile and check the pattern
		re, err := regexp.Compile(rule.Pattern)
		if err != nil {
			continue // Skip invalid patterns
		}

		if re.MatchString(content) {
			// Check forbidden context (if pattern matches, but forbidden context also matches, skip)
			if rule.ForbiddenContext != "" {
				ctxRe, err := regexp.Compile(rule.ForbiddenContext)
				if err == nil && ctxRe.MatchString(content) {
					continue // Forbidden context present, skip this violation
				}
			}

			violations = append(violations, violation{
				RuleID:     rule.RuleID,
				RuleName:   rule.Name,
				Severity:   rule.Severity,
				Message:    rule.Message,
				Suggestion: rule.Suggestion,
			})
		}
	}

	resultJSON, _ := json.Marshal(map[string]interface{}{
		"file":       filePath,
		"violations": violations,
		"valid":      len(violations) == 0,
		"rules_checked": len(rules),
	})
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
	}, nil
}

// handleListViolations removed — use guardrail_log_violation to log
// and the web API GET /api/failures to query violations
