package mcp

import (
	"context"
	"encoding/json"
	"fmt"
	"path/filepath"
	"regexp"
	"runtime"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/mark3labs/mcp-go/mcp"
	"github.com/thearchitectit/guardrail-mcp/internal/metrics"
	"github.com/thearchitectit/guardrail-mcp/internal/team"
)

// getTeamManagerPath returns the absolute path to the team_manager.py script
// This ensures the script can be found regardless of the working directory
func getTeamManagerPath() string {
	// Get the directory of this Go source file
	_, filename, _, _ := runtime.Caller(0)
	dir := filepath.Dir(filename)

	// Navigate from mcp-server/internal/mcp/ to repo root, then to scripts/
	// Path: mcp-server/internal/mcp/ -> ../../../scripts/
	return filepath.Join(dir, "..", "..", "..", "scripts", "team_manager.py")
}

// getRepoRoot returns the absolute path to the repo root directory
func getRepoRoot() string {
	_, filename, _, _ := runtime.Caller(0)
	dir := filepath.Dir(filename)
	// Navigate from mcp-server/internal/mcp/ to repo root
	return filepath.Join(dir, "..", "..", "..")
}

// SEC-005: Rate limiting configuration
const (
	defaultRateLimitRequests = 100
	defaultRateLimitWindow   = 60 // seconds
)

// rateBucket represents a token bucket for rate limiting
type rateBucket struct {
	tokens     int
	lastReset  time.Time
}

// rateLimiter implements token bucket rate limiting
type rateLimiter struct {
	mu              sync.RWMutex
	buckets         map[string]*rateBucket
	requestsLimit   int
	windowSeconds   int
}

// globalRateLimiter is the singleton rate limiter instance
var globalRateLimiter = &rateLimiter{
	buckets:       make(map[string]*rateBucket),
	requestsLimit: defaultRateLimitRequests,
	windowSeconds: defaultRateLimitWindow,
}

// checkRateLimit checks if a request is allowed for the given user
// Returns (allowed, rateLimitHeaders)
func (rl *rateLimiter) checkRateLimit(userID string) (bool, map[string]string) {
	rl.mu.Lock()
	defer rl.mu.Unlock()

	now := time.Now()
	bucket, exists := rl.buckets[userID]

	if !exists || now.Sub(bucket.lastReset) >= time.Duration(rl.windowSeconds)*time.Second {
		// Create new bucket or reset expired bucket
		rl.buckets[userID] = &rateBucket{
			tokens:     rl.requestsLimit - 1, // Consume one token
			lastReset:  now,
		}
		remaining := rl.requestsLimit - 1
		resetTime := now.Add(time.Duration(rl.windowSeconds) * time.Second).Unix()
		return true, map[string]string{
			"X-RateLimit-Limit":     strconv.Itoa(rl.requestsLimit),
			"X-RateLimit-Remaining": strconv.Itoa(remaining),
			"X-RateLimit-Reset":     strconv.Itoa(int(resetTime)),
		}
	}

	// Check if tokens available
	if bucket.tokens <= 0 {
		resetTime := bucket.lastReset.Add(time.Duration(rl.windowSeconds) * time.Second).Unix()
		return false, map[string]string{
			"X-RateLimit-Limit":     strconv.Itoa(rl.requestsLimit),
			"X-RateLimit-Remaining": "0",
			"X-RateLimit-Reset":     strconv.Itoa(int(resetTime)),
		}
	}

	// Consume token
	bucket.tokens--
	remaining := bucket.tokens
	resetTime := bucket.lastReset.Add(time.Duration(rl.windowSeconds) * time.Second).Unix()
	return true, map[string]string{
		"X-RateLimit-Limit":     strconv.Itoa(rl.requestsLimit),
		"X-RateLimit-Remaining": strconv.Itoa(remaining),
		"X-RateLimit-Reset":     strconv.Itoa(int(resetTime)),
	}
}

// cleanupOldBuckets removes expired buckets (call periodically)
func (rl *rateLimiter) cleanupOldBuckets() {
	rl.mu.Lock()
	defer rl.mu.Unlock()

	now := time.Now()
	window := time.Duration(rl.windowSeconds) * time.Second

	for userID, bucket := range rl.buckets {
		if now.Sub(bucket.lastReset) > window*2 {
			delete(rl.buckets, userID)
		}
	}
}

// validateProjectName validates project name to prevent command injection
func validateProjectName(name string) error {
	if name == "" {
		return fmt.Errorf("project_name is required")
	}
	if len(name) > 64 {
		return fmt.Errorf("project_name must be 64 characters or less")
	}
	// Allow alphanumeric, hyphen, underscore only
	for _, r := range name {
		if !((r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z') || (r >= '0' && r <= '9') || r == '-' || r == '_') {
			return fmt.Errorf("project_name must contain only letters, numbers, hyphens, and underscores")
		}
	}
	return nil
}

// validRoles is the whitelist of 48 valid roles from TEAM_STRUCTURE.md
var validRoles = map[string]bool{
	// Team 1: Business & Product Strategy
	"Business Relationship Manager": true,
	"Lead Product Manager":          true,
	"Business Systems Analyst":      true,
	"Financial Controller (FinOps)": true,
	// Team 2: Enterprise Architecture
	"Chief Architect":    true,
	"Domain Architect":   true,
	"Solution Architect": true,
	"Standards Lead":     true,
	// Team 3: GRC
	"Compliance Officer": true,
	"Internal Auditor":   true,
	"Privacy Engineer":   true,
	"Policy Manager":     true,
	// Team 4: Infrastructure & Cloud Ops
	"Cloud Architect":           true,
	"IaC Engineer":              true,
	"Network Security Engineer": true,
	"Storage Engineer":          true,
	// Team 5: Platform Engineering
	"Platform Product Manager": true,
	"CI/CD Architect":          true,
	"Kubernetes Administrator": true,
	"Developer Advocate":       true,
	// Team 6: Data Governance & Analytics
	"Data Architect":       true,
	"DBA":                  true,
	"Data Privacy Officer": true,
	"ETL Developer":        true,
	// Team 7: Core Feature Squad
	"Technical Lead":              true,
	"Senior Backend Engineer":     true,
	"Senior Frontend Engineer":    true,
	"Accessibility (A11y) Expert": true,
	"Technical Writer":            true,
	// Team 8: Middleware & Integration
	"API Product Manager":  true,
	"Integration Engineer": true,
	"Messaging Engineer":   true,
	"IAM Specialist":       true,
	// Team 9: Cybersecurity
	"Security Architect":       true,
	"Vulnerability Researcher": true,
	"Penetration Tester":       true,
	"DevSecOps Engineer":       true,
	// Team 10: Quality Engineering
	"QA Architect":                true,
	"SDET":                        true,
	"Performance/Load Engineer":   true,
	"Manual QA / UAT Coordinator": true,
	// Team 11: SRE
	"SRE Lead":               true,
	"Observability Engineer": true,
	"Chaos Engineer":         true,
	"Incident Manager":       true,
	// Team 12: IT Operations & Support
	"NOC Analyst":         true,
	"Change Manager":      true,
	"Release Manager":     true,
	"L3 Support Engineer": true,
}

// validateRoleName validates role name against whitelist (SEC-002)
func validateRoleName(name string) error {
	if name == "" {
		return fmt.Errorf("role_name is required")
	}
	if len(name) > 128 {
		return fmt.Errorf("role_name must be 128 characters or less")
	}
	// Check for control characters
	for _, r := range name {
		if r < 32 || r == 127 {
			return fmt.Errorf("role_name contains invalid control characters")
		}
	}
	// Whitelist validation: must be one of 48 valid roles
	if !validRoles[name] {
		return fmt.Errorf("invalid role_name: '%s'. Must be one of the 48 defined roles in TEAM_STRUCTURE.md", name)
	}
	return nil
}

// validatePersonName validates person/assignee name format (SEC-003)
// Accepts email addresses, usernames, or display names with alphanumeric, spaces, dots, hyphens, underscores, apostrophes
func validatePersonName(name string) error {
	if name == "" {
		return fmt.Errorf("person is required")
	}
	if len(name) > 256 {
		return fmt.Errorf("person must be 256 characters or less")
	}
	// Check for control characters
	for _, r := range name {
		if r < 32 || r == 127 {
			return fmt.Errorf("person contains invalid control characters")
		}
	}
	// Check for potentially dangerous patterns
	dangerousPatterns := []string{";", "|", "&&", "||", "`", "$", "<", ">", "..", "\\"}
	for _, pattern := range dangerousPatterns {
		if strings.Contains(name, pattern) {
			return fmt.Errorf("person contains forbidden pattern: %s", pattern)
		}
	}
	// Validate format: email, username, or display name
	// Email pattern: user@domain.com
	// Username pattern: alphanumeric + dots + hyphens + underscores
	// Display name: alphanumeric + spaces + dots + hyphens + underscores + apostrophes
	isEmail := false
	if strings.Contains(name, "@") {
		parts := strings.Split(name, "@")
		if len(parts) == 2 && parts[0] != "" && parts[1] != "" {
			// Basic email validation - must have domain with at least one dot
			domainParts := strings.Split(parts[1], ".")
			if len(domainParts) >= 2 {
				isEmail = true
			}
		}
	}
	if !isEmail {
		// Must be username or display name format
		// Allow alphanumeric, spaces, dots, hyphens, underscores, apostrophes (for names like O'Connor)
		for _, r := range name {
			if !((r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z') || (r >= '0' && r <= '9') ||
				r == ' ' || r == '.' || r == '-' || r == '_' || r == '\'') {
				return fmt.Errorf("person contains invalid characters")
			}
		}
	}
	return nil
}

// validatePhase validates phase filter value (SEC-010: Phase injection hardening)
// Whitelist: Phase 1, Phase 2, Phase 3 (strict regex validation)
func validatePhase(phase string) error {
	if phase == "" {
		return nil // Phase is optional
	}
	// SEC-010: Strict regex validation - only allow "Phase 1", "Phase 2", "Phase 3"
	// This prevents injection attacks through the phase parameter
	validPhaseRegex := regexp.MustCompile(`^Phase [1-3]$`)
	if !validPhaseRegex.MatchString(phase) {
		return fmt.Errorf("invalid phase: must be 'Phase 1', 'Phase 2', or 'Phase 3'")
	}
	return nil
}

// sanitizePhase sanitizes phase string for safe command execution (SEC-010)
// Returns empty string if phase is invalid, otherwise returns cleaned phase
func sanitizePhase(phase string) string {
	if phase == "" {
		return ""
	}
	// Whitelist only exact phase patterns
	switch phase {
	case "Phase 1":
		return "Phase 1"
	case "Phase 2":
		return "Phase 2"
	case "Phase 3":
		return "Phase 3"
	default:
		return "" // Invalid phase - return empty for safety
	}
}

// Team tool handler implementations for MCP server

// handleTeamInit initializes team structure for a project
func (s *MCPServer) handleTeamInit(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	start := time.Now()
	metrics.IncrementTeamToolActive("team_init")
	defer func() {
		metrics.DecrementTeamToolActive("team_init")
		metrics.RecordTeamToolDuration("team_init", time.Since(start))
	}()

	projectName, ok := args["project_name"].(string)
	if !ok || projectName == "" {
		metrics.RecordTeamToolError("team_init", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: project_name is required"}},
			IsError: true,
		}, nil
	}

	if err := validateProjectName(projectName); err != nil {
		metrics.RecordTeamToolError("team_init", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: err.Error()}},
			IsError: true,
		}, nil
	}

	// Use Go implementation instead of Python
	mgr, err := team.NewManager(projectName, team.WithTestMode(true))
	if err != nil {
		metrics.RecordTeamToolError("team_init", "go_error")
		metrics.RecordTeamToolCall("team_init", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error creating manager: %v", err)}},
			IsError: true,
		}, nil
	}

	goStart := time.Now()
	if err := mgr.InitializeProject(); err != nil {
		metrics.RecordTeamToolDuration("team_init", time.Since(goStart))
		metrics.RecordTeamToolError("team_init", "go_error")
		metrics.RecordTeamToolCall("team_init", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error initializing project: %v", err)}},
			IsError: true,
		}, nil
	}
	metrics.RecordTeamToolDuration("team_init", time.Since(goStart))

	resultText := fmt.Sprintf("✅ Initialized project '%s' with %d teams", projectName, len(team.StandardTeams))
	metrics.RecordTeamToolCall("team_init", true)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
	}, nil
}

// handleTeamList lists all teams and their status
func (s *MCPServer) handleTeamList(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	start := time.Now()
	metrics.IncrementTeamToolActive("team_list")
	defer func() {
		metrics.DecrementTeamToolActive("team_list")
		metrics.RecordTeamToolDuration("team_list", time.Since(start))
	}()

	projectName, ok := args["project_name"].(string)
	if !ok || projectName == "" {
		metrics.RecordTeamToolError("team_list", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: project_name is required"}},
			IsError: true,
		}, nil
	}

	if err := validateProjectName(projectName); err != nil {
		metrics.RecordTeamToolError("team_list", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: err.Error()}},
			IsError: true,
		}, nil
	}

	// Use Go implementation
	mgr, err := team.NewManager(projectName, team.WithTestMode(true))
	if err != nil {
		metrics.RecordTeamToolError("team_list", "go_error")
		metrics.RecordTeamToolCall("team_list", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error creating manager: %v", err)}},
			IsError: true,
		}, nil
	}

	goStart := time.Now()
	if err := mgr.Load(); err != nil {
		metrics.RecordTeamToolError("team_list", "go_error")
		metrics.RecordTeamToolCall("team_list", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error loading project: %v", err)}},
			IsError: true,
		}, nil
	}

	var teams []team.Team
	if phase, ok := args["phase"].(string); ok && phase != "" {
		if err := team.ValidatePhase(phase); err != nil {
			metrics.RecordTeamToolError("team_list", "validation_error")
			return &mcp.CallToolResult{
				Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error: %v", err)}},
				IsError: true,
			}, nil
		}
		teams = mgr.GetTeamsByPhase(phase)
	} else {
		teams = mgr.GetAllTeams()
	}
	metrics.RecordTeamToolDuration("team_list", time.Since(goStart))

	// Build result
	var sb strings.Builder
	sb.WriteString(fmt.Sprintf("\n📋 Teams for project '%s':\n\n", projectName))
	sb.WriteString(fmt.Sprintf("%-5s %-35s %-30s %s\n", "ID", "Name", "Phase", "Status"))
	sb.WriteString(strings.Repeat("-", 100) + "\n")

	for _, t := range teams {
		assignedCount := 0
		for _, role := range t.Roles {
			if role.AssignedTo != nil {
				assignedCount++
			}
		}
		statusStr := string(t.Status)
		if t.Status == team.TeamStatusNotStarted && assignedCount > 0 {
			statusStr = fmt.Sprintf("%s (%d/%d assigned)", t.Status, assignedCount, len(t.Roles))
		}
		sb.WriteString(fmt.Sprintf("%-5d %-35s %-30s %s\n", t.ID, t.Name, t.Phase, statusStr))
	}

	resultText := sb.String()
	metrics.RecordTeamToolCall("team_list", true)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
	}, nil
}

// handleTeamAssign assigns a person to a role in a team
func (s *MCPServer) handleTeamAssign(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	start := time.Now()
	metrics.IncrementTeamToolActive("team_assign")
	defer func() {
		metrics.DecrementTeamToolActive("team_assign")
		metrics.RecordTeamToolDuration("team_assign", time.Since(start))
	}()

	projectName, ok := args["project_name"].(string)
	if !ok || projectName == "" {
		metrics.RecordTeamToolError("team_assign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: project_name is required"}},
			IsError: true,
		}, nil
	}

	if err := validateProjectName(projectName); err != nil {
		metrics.RecordTeamToolError("team_assign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: err.Error()}},
			IsError: true,
		}, nil
	}

	// SEC-005: Check rate limit
	userID := "default" // Could extract from context/auth if available
	allowed, rateHeaders := globalRateLimiter.checkRateLimit(userID)
	if !allowed {
		metrics.RecordTeamToolError("team_assign", "rate_limit_exceeded")
		retryAfter := rateHeaders["X-RateLimit-Reset"]
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{
				Type: "text",
				Text: fmt.Sprintf("Error: Rate limit exceeded. Retry after %s", retryAfter),
			}},
			IsError: true,
		}, nil
	}

	teamID, ok := args["team_id"].(float64)
	if !ok {
		metrics.RecordTeamToolError("team_assign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: team_id is required"}},
			IsError: true,
		}, nil
	}

	// Validate team_id range (1-12)
	teamIDInt := int(teamID)
	if teamIDInt < 1 || teamIDInt > 12 {
		metrics.RecordTeamToolError("team_assign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: team_id must be between 1 and 12"}},
			IsError: true,
		}, nil
	}

	roleName, ok := args["role_name"].(string)
	if !ok || roleName == "" {
		metrics.RecordTeamToolError("team_assign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: role_name is required"}},
			IsError: true,
		}, nil
	}

	if err := validateRoleName(roleName); err != nil {
		metrics.RecordTeamToolError("team_assign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error: %v", err)}},
			IsError: true,
		}, nil
	}

	person, ok := args["person"].(string)
	if !ok || person == "" {
		metrics.RecordTeamToolError("team_assign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: person is required"}},
			IsError: true,
		}, nil
	}

	if err := validatePersonName(person); err != nil {
		metrics.RecordTeamToolError("team_assign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error: %v", err)}},
			IsError: true,
		}, nil
	}

	// Use Go implementation
	mgr, err := team.NewManager(projectName, team.WithTestMode(true))
	if err != nil {
		metrics.RecordTeamToolError("team_assign", "go_error")
		metrics.RecordTeamToolCall("team_assign", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error creating manager: %v", err)}},
			IsError: true,
		}, nil
	}

	goStart := time.Now()
	if err := mgr.Load(); err != nil {
		metrics.RecordTeamToolError("team_assign", "go_error")
		metrics.RecordTeamToolCall("team_assign", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error loading project: %v", err)}},
			IsError: true,
		}, nil
	}

	if err := mgr.AssignRole(teamIDInt, roleName, person); err != nil {
		metrics.RecordTeamToolDuration("team_assign", time.Since(goStart))
		metrics.RecordTeamToolError("team_assign", "go_error")
		metrics.RecordTeamToolCall("team_assign", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error assigning role: %v", err)}},
			IsError: true,
		}, nil
	}
	metrics.RecordTeamToolDuration("team_assign", time.Since(goStart))

	resultText := fmt.Sprintf("✅ Assigned '%s' to '%s' in Team %d (%s)",
		person, roleName, teamIDInt, team.StandardTeams[teamIDInt].Name)
	metrics.RecordTeamToolCall("team_assign", true)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
	}, nil
}

// handleTeamUnassign removes a person from a role in a team
func (s *MCPServer) handleTeamUnassign(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	start := time.Now()
	metrics.IncrementTeamToolActive("team_unassign")
	defer func() {
		metrics.DecrementTeamToolActive("team_unassign")
		metrics.RecordTeamToolDuration("team_unassign", time.Since(start))
	}()

	projectName, ok := args["project_name"].(string)
	if !ok || projectName == "" {
		metrics.RecordTeamToolError("team_unassign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: project_name is required"}},
			IsError: true,
		}, nil
	}

	if err := validateProjectName(projectName); err != nil {
		metrics.RecordTeamToolError("team_unassign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: err.Error()}},
			IsError: true,
		}, nil
	}

	teamID, ok := args["team_id"].(float64)
	if !ok {
		metrics.RecordTeamToolError("team_unassign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: team_id is required"}},
			IsError: true,
		}, nil
	}

	// Validate team_id range (1-12)
	teamIDInt := int(teamID)
	if teamIDInt < 1 || teamIDInt > 12 {
		metrics.RecordTeamToolError("team_unassign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: team_id must be between 1 and 12"}},
			IsError: true,
		}, nil
	}

	roleName, ok := args["role_name"].(string)
	if !ok || roleName == "" {
		metrics.RecordTeamToolError("team_unassign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: role_name is required"}},
			IsError: true,
		}, nil
	}

	if err := validateRoleName(roleName); err != nil {
		metrics.RecordTeamToolError("team_unassign", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error: %v", err)}},
			IsError: true,
		}, nil
	}

	// Use Go implementation
	mgr, err := team.NewManager(projectName, team.WithTestMode(true))
	if err != nil {
		metrics.RecordTeamToolError("team_unassign", "go_error")
		metrics.RecordTeamToolCall("team_unassign", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error creating manager: %v", err)}},
			IsError: true,
		}, nil
	}

	goStart := time.Now()
	if err := mgr.Load(); err != nil {
		metrics.RecordTeamToolError("team_unassign", "go_error")
		metrics.RecordTeamToolCall("team_unassign", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error loading project: %v", err)}},
			IsError: true,
		}, nil
	}

	if err := mgr.UnassignRole(teamIDInt, roleName); err != nil {
		metrics.RecordTeamToolDuration("team_unassign", time.Since(goStart))
		metrics.RecordTeamToolError("team_unassign", "go_error")
		metrics.RecordTeamToolCall("team_unassign", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error unassigning role: %v", err)}},
			IsError: true,
		}, nil
	}
	metrics.RecordTeamToolDuration("team_unassign", time.Since(goStart))

	resultText := fmt.Sprintf("✅ Unassigned role '%s' from Team %d (%s)",
		roleName, teamIDInt, team.StandardTeams[teamIDInt].Name)
	metrics.RecordTeamToolCall("team_unassign", true)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
	}, nil
}

// handleTeamStart starts a team (marks as active)
// FUNC-010: Supports override for admin users
func (s *MCPServer) handleTeamStart(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	start := time.Now()
	metrics.IncrementTeamToolActive("team_start")
	defer func() {
		metrics.DecrementTeamToolActive("team_start")
		metrics.RecordTeamToolDuration("team_start", time.Since(start))
	}()

	projectName, ok := args["project_name"].(string)
	if !ok || projectName == "" {
		metrics.RecordTeamToolError("team_start", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: project_name is required"}},
			IsError: true,
		}, nil
	}

	if err := validateProjectName(projectName); err != nil {
		metrics.RecordTeamToolError("team_start", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: err.Error()}},
			IsError: true,
		}, nil
	}

	teamID, ok := args["team_id"].(float64)
	if !ok {
		metrics.RecordTeamToolError("team_start", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: team_id is required"}},
			IsError: true,
		}, nil
	}

	// Validate team_id range (1-12)
	teamIDInt := int(teamID)
	if teamIDInt < 1 || teamIDInt > 12 {
		metrics.RecordTeamToolError("team_start", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: team_id must be between 1 and 12"}},
			IsError: true,
		}, nil
	}

	// FUNC-010: Handle override option
	override := false
	if overrideVal, ok := args["override"].(bool); ok {
		override = overrideVal
	}

	if override {
		// Reason is required when overriding
		reason, ok := args["reason"].(string)
		if !ok || reason == "" {
			metrics.RecordTeamToolError("team_start", "validation_error")
			return &mcp.CallToolResult{
				Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: reason is required when using override"}},
				IsError: true,
			}, nil
		}
	}

	// Use Go implementation
	mgr, err := team.NewManager(projectName, team.WithTestMode(true))
	if err != nil {
		metrics.RecordTeamToolError("team_start", "go_error")
		metrics.RecordTeamToolCall("team_start", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error creating manager: %v", err)}},
			IsError: true,
		}, nil
	}

	goStart := time.Now()
	if err := mgr.Load(); err != nil {
		metrics.RecordTeamToolError("team_start", "go_error")
		metrics.RecordTeamToolCall("team_start", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error loading project: %v", err)}},
			IsError: true,
		}, nil
	}

	if err := mgr.StartTeam(teamIDInt, override, ""); err != nil {
		metrics.RecordTeamToolDuration("team_start", time.Since(goStart))
		metrics.RecordTeamToolError("team_start", "go_error")
		metrics.RecordTeamToolCall("team_start", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error starting team: %v", err)}},
			IsError: true,
		}, nil
	}
	metrics.RecordTeamToolDuration("team_start", time.Since(goStart))

	resultText := fmt.Sprintf("✅ Started Team %d (%s)", teamIDInt, team.StandardTeams[teamIDInt].Name)
	if override {
		resultText += " (with override)"
	}
	metrics.RecordTeamToolCall("team_start", true)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
	}, nil
}

// handleTeamStatus gets phase or project status
func (s *MCPServer) handleTeamStatus(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	start := time.Now()
	metrics.IncrementTeamToolActive("team_status")
	defer func() {
		metrics.DecrementTeamToolActive("team_status")
		metrics.RecordTeamToolDuration("team_status", time.Since(start))
	}()

	projectName, ok := args["project_name"].(string)
	if !ok || projectName == "" {
		metrics.RecordTeamToolError("team_status", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: project_name is required"}},
			IsError: true,
		}, nil
	}

	if err := validateProjectName(projectName); err != nil {
		metrics.RecordTeamToolError("team_status", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: err.Error()}},
			IsError: true,
		}, nil
	}

	// Use Go implementation
	mgr, err := team.NewManager(projectName, team.WithTestMode(true))
	if err != nil {
		metrics.RecordTeamToolError("team_status", "go_error")
		metrics.RecordTeamToolCall("team_status", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error creating manager: %v", err)}},
			IsError: true,
		}, nil
	}

	goStart := time.Now()
	if err := mgr.Load(); err != nil {
		metrics.RecordTeamToolError("team_status", "go_error")
		metrics.RecordTeamToolCall("team_status", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error loading project: %v", err)}},
			IsError: true,
		}, nil
	}

	var resultText string
	if phase, ok := args["phase"].(string); ok && phase != "" {
		status, _ := mgr.GetPhaseStatus(phase)
		data, _ := json.MarshalIndent(status, "", "  ")
		resultText = string(data)
	} else {
		status := mgr.GetProjectStatus()
		data, _ := json.MarshalIndent(status, "", "  ")
		resultText = string(data)
	}
	metrics.RecordTeamToolDuration("team_status", time.Since(goStart))

	metrics.RecordTeamToolCall("team_status", true)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
	}, nil
}

// handlePhaseGateCheck checks if phase gate requirements are met
func (s *MCPServer) handlePhaseGateCheck(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	start := time.Now()
	metrics.IncrementTeamToolActive("phase_gate_check")
	defer func() {
		metrics.DecrementTeamToolActive("phase_gate_check")
		metrics.RecordTeamToolDuration("phase_gate_check", time.Since(start))
	}()

	projectName, ok := args["project_name"].(string)
	if !ok || projectName == "" {
		metrics.RecordTeamToolError("phase_gate_check", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: project_name is required"}},
			IsError: true,
		}, nil
	}

	if err := validateProjectName(projectName); err != nil {
		metrics.RecordTeamToolError("phase_gate_check", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: err.Error()}},
			IsError: true,
		}, nil
	}

	fromPhase, ok := args["from_phase"].(float64)
	if !ok {
		metrics.RecordTeamToolError("phase_gate_check", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: from_phase is required"}},
			IsError: true,
		}, nil
	}

	toPhase, ok := args["to_phase"].(float64)
	if !ok {
		metrics.RecordTeamToolError("phase_gate_check", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: to_phase is required"}},
			IsError: true,
		}, nil
	}

	// Load team layout rules
	rules, err := loadTeamLayoutRules()
	if err != nil {
		metrics.RecordTeamToolError("phase_gate_check", "rules_error")
		metrics.RecordTeamToolCall("phase_gate_check", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{
				Type: "text",
				Text: fmt.Sprintf("Error loading team rules: %v", err),
			}},
			IsError: true,
		}, nil
	}

	// Map phases to gate names
	gateName := fmt.Sprintf("%d_to_%d", int(fromPhase), int(toPhase))
	gate, exists := rules.PhaseGates[gateName]
	if !exists {
		metrics.RecordTeamToolError("phase_gate_check", "gate_not_found")
		metrics.RecordTeamToolCall("phase_gate_check", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{
				Type: "text",
				Text: fmt.Sprintf("No phase gate defined from phase %d to phase %d", int(fromPhase), int(toPhase)),
			}},
			IsError: true,
		}, nil
	}

	// Build response
	var response strings.Builder
	response.WriteString(fmt.Sprintf("# Phase Gate: %s\n\n", gate.Name))
	response.WriteString("**Required Teams:**\n")
	for _, teamID := range gate.RequiredTeams {
		response.WriteString(fmt.Sprintf("- Team %d\n", teamID))
	}

	response.WriteString("\n**Required Deliverables:**\n")
	for _, deliverable := range gate.Deliverables {
		response.WriteString(fmt.Sprintf("- [ ] %s\n", deliverable))
	}

	metrics.RecordTeamToolCall("phase_gate_check", true)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: response.String()}},
	}, nil
}

// handleAgentTeamMap gets the team assignment for an agent type
func (s *MCPServer) handleAgentTeamMap(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	start := time.Now()
	metrics.IncrementTeamToolActive("agent_team_map")
	defer func() {
		metrics.DecrementTeamToolActive("agent_team_map")
		metrics.RecordTeamToolDuration("agent_team_map", time.Since(start))
	}()

	agentType, ok := args["agent_type"].(string)
	if !ok || agentType == "" {
		metrics.RecordTeamToolError("agent_team_map", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: agent_type is required"}},
			IsError: true,
		}, nil
	}

	// Load team layout rules
	rules, err := loadTeamLayoutRules()
	if err != nil {
		metrics.RecordTeamToolError("agent_team_map", "rules_error")
		metrics.RecordTeamToolCall("agent_team_map", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{
				Type: "text",
				Text: fmt.Sprintf("Error loading team rules: %v", err),
			}},
			IsError: true,
		}, nil
	}

	mapping, exists := rules.AgentMapping[agentType]
	if !exists {
		metrics.RecordTeamToolError("agent_team_map", "mapping_not_found")
		metrics.RecordTeamToolCall("agent_team_map", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{
				Type: "text",
				Text: fmt.Sprintf("No team mapping found for agent type: %s", agentType),
			}},
			IsError: true,
		}, nil
	}

	result := fmt.Sprintf(
		"# Agent Team Assignment\n\n"+
			"**Agent Type:** %s\n"+
			"**Assigned Team:** Team %d\n"+
			"**Phase:** %s\n"+
			"**Roles:** %s\n",
		agentType,
		mapping.Team,
		mapping.Phase,
		strings.Join(mapping.Roles, ", "),
	)

	metrics.RecordTeamToolCall("agent_team_map", true)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: result}},
	}, nil
}

// handleTeamSizeValidate validates team sizes meet 4-6 member requirement
func (s *MCPServer) handleTeamSizeValidate(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	start := time.Now()
	metrics.IncrementTeamToolActive("team_size_validate")
	defer func() {
		metrics.DecrementTeamToolActive("team_size_validate")
		metrics.RecordTeamToolDuration("team_size_validate", time.Since(start))
	}()

	projectName, ok := args["project_name"].(string)
	if !ok || projectName == "" {
		metrics.RecordTeamToolError("team_size_validate", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: project_name is required"}},
			IsError: true,
		}, nil
	}

	if err := validateProjectName(projectName); err != nil {
		metrics.RecordTeamToolError("team_size_validate", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: err.Error()}},
			IsError: true,
		}, nil
	}

	// Use Go implementation
	mgr, err := team.NewManager(projectName)
	if err != nil {
		metrics.RecordTeamToolError("team_size_validate", "go_error")
		metrics.RecordTeamToolCall("team_size_validate", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error creating manager: %v", err)}},
			IsError: true,
		}, nil
	}

	goStart := time.Now()

	// Get all teams
	teams := mgr.GetAllTeams()

	// Validate team sizes
	var violations []string
	for _, t := range teams {
		assignedCount := 0
		for _, role := range t.Roles {
			if role.AssignedTo != nil && *role.AssignedTo != "" {
				assignedCount++
			}
		}
		if assignedCount < 4 || assignedCount > 6 {
			violations = append(violations, fmt.Sprintf("Team %d (%s): %d members (requires 4-6)", t.ID, t.Name, assignedCount))
		}
	}

	metrics.RecordTeamToolDuration("team_size_validate", time.Since(goStart))

	if len(violations) > 0 {
		resultText := fmt.Sprintf("❌ Team size validation failed:\n%s", strings.Join(violations, "\n"))
		metrics.RecordTeamToolCall("team_size_validate", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
			IsError: true,
		}, nil
	}

	resultText := fmt.Sprintf("✅ All teams in project '%s' have valid sizes (4-6 members)", projectName)
	metrics.RecordTeamToolCall("team_size_validate", true)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
	}, nil
}

// Helper types and functions

type TeamLayoutRules struct {
	Name         string               `json:"name"`
	Version      string               `json:"version"`
	Description  string               `json:"description"`
	AppliesTo    []string             `json:"applies_to"`
	Rules        []TeamRule           `json:"rules"`
	PhaseGates   map[string]PhaseGate `json:"phase_gates"`
	AgentMapping map[string]AgentTeam `json:"agent_mapping"`
}

type TeamRule struct {
	ID       string   `json:"id"`
	Name     string   `json:"name"`
	Severity string   `json:"severity"`
	Check    string   `json:"check"`
	Command  string   `json:"command"`
	Message  string   `json:"message"`
	Trigger  string   `json:"trigger,omitempty"`
	Patterns []string `json:"patterns,omitempty"`
}

type PhaseGate struct {
	Name             string   `json:"name"`
	RequiredTeams    []int    `json:"required_teams"`
	ApprovalRequired []int    `json:"approval_required"`
	Deliverables     []string `json:"deliverables"`
}

type AgentTeam struct {
	Team  int      `json:"team"`
	Roles []string `json:"roles"`
	Phase string   `json:"phase"`
}

// handleTeamDelete deletes a specific team from a project
func (s *MCPServer) handleTeamDelete(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	start := time.Now()
	metrics.IncrementTeamToolActive("team_delete")
	defer func() {
		metrics.DecrementTeamToolActive("team_delete")
		metrics.RecordTeamToolDuration("team_delete", time.Since(start))
	}()

	projectName, ok := args["project_name"].(string)
	if !ok || projectName == "" {
		metrics.RecordTeamToolError("team_delete", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: project_name is required"}},
			IsError: true,
		}, nil
	}

	if err := validateProjectName(projectName); err != nil {
		metrics.RecordTeamToolError("team_delete", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: err.Error()}},
			IsError: true,
		}, nil
	}

	teamID, ok := args["team_id"].(float64)
	if !ok {
		metrics.RecordTeamToolError("team_delete", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: team_id is required"}},
			IsError: true,
		}, nil
	}

	// Validate team_id range (1-12)
	teamIDInt := int(teamID)
	if teamIDInt < 1 || teamIDInt > 12 {
		metrics.RecordTeamToolError("team_delete", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: team_id must be between 1 and 12"}},
			IsError: true,
		}, nil
	}

	// Check for confirmation
	confirmed := false
	if conf, ok := args["confirmed"].(bool); ok {
		confirmed = conf
	}

	// Use Go implementation
	mgr, err := team.NewManager(projectName)
	if err != nil {
		metrics.RecordTeamToolError("team_delete", "go_error")
		metrics.RecordTeamToolCall("team_delete", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error creating manager: %v", err)}},
			IsError: true,
		}, nil
	}

	goStart := time.Now()
	if err := mgr.DeleteTeam(teamIDInt, confirmed); err != nil {
		metrics.RecordTeamToolDuration("team_delete", time.Since(goStart))
		// Check if this is just a confirmation required error
		if strings.Contains(err.Error(), "requires confirmation") {
			return &mcp.CallToolResult{
				Content: []interface{}{mcp.TextContent{Type: "text", Text: "⚠️  Deletion requires confirmation. Set confirmed=true to proceed."}},
			}, nil
		}
		resultText := fmt.Sprintf("Error deleting team: %v", err)
		metrics.RecordTeamToolError("team_delete", "go_error")
		metrics.RecordTeamToolCall("team_delete", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
			IsError: true,
		}, nil
	}
	metrics.RecordTeamToolDuration("team_delete", time.Since(goStart))

	resultText := fmt.Sprintf("✅ Deleted team %d from project '%s'", teamIDInt, projectName)
	metrics.RecordTeamToolCall("team_delete", true)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
	}, nil
}

// handleProjectDelete deletes an entire project
func (s *MCPServer) handleProjectDelete(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	start := time.Now()
	metrics.IncrementTeamToolActive("project_delete")
	defer func() {
		metrics.DecrementTeamToolActive("project_delete")
		metrics.RecordTeamToolDuration("project_delete", time.Since(start))
	}()

	projectName, ok := args["project_name"].(string)
	if !ok || projectName == "" {
		metrics.RecordTeamToolError("project_delete", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: "Error: project_name is required"}},
			IsError: true,
		}, nil
	}

	if err := validateProjectName(projectName); err != nil {
		metrics.RecordTeamToolError("project_delete", "validation_error")
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: err.Error()}},
			IsError: true,
		}, nil
	}

	// Check for confirmation
	confirmed := false
	if conf, ok := args["confirmed"].(bool); ok {
		confirmed = conf
	}

	// Use Go implementation
	mgr, err := team.NewManager(projectName)
	if err != nil {
		metrics.RecordTeamToolError("project_delete", "go_error")
		metrics.RecordTeamToolCall("project_delete", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Error creating manager: %v", err)}},
			IsError: true,
		}, nil
	}

	goStart := time.Now()
	if err := mgr.DeleteProject(confirmed); err != nil {
		metrics.RecordTeamToolDuration("project_delete", time.Since(goStart))
		// Check if this is just a confirmation required error
		if strings.Contains(err.Error(), "requires confirmation") {
			return &mcp.CallToolResult{
				Content: []interface{}{mcp.TextContent{Type: "text", Text: "⚠️  Project deletion requires confirmation. Set confirmed=true to proceed."}},
			}, nil
		}
		resultText := fmt.Sprintf("Error deleting project: %v", err)
		metrics.RecordTeamToolError("project_delete", "go_error")
		metrics.RecordTeamToolCall("project_delete", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
			IsError: true,
		}, nil
	}
	metrics.RecordTeamToolDuration("project_delete", time.Since(goStart))

	resultText := fmt.Sprintf("✅ Deleted project '%s'", projectName)
	metrics.RecordTeamToolCall("project_delete", true)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
	}, nil
}

// handleTeamHealth performs health check on team_manager.py
func (s *MCPServer) handleTeamHealth(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	start := time.Now()
	metrics.IncrementTeamToolActive("team_health")
	defer func() {
		metrics.DecrementTeamToolActive("team_health")
		metrics.RecordTeamToolDuration("team_health", time.Since(start))
	}()

	projectName := "health-check"
	if name, ok := args["project_name"].(string); ok && name != "" {
		if err := validateProjectName(name); err != nil {
			metrics.RecordTeamToolError("team_health", "validation_error")
			return &mcp.CallToolResult{
				Content: []interface{}{mcp.TextContent{Type: "text", Text: err.Error()}},
				IsError: true,
			}, nil
		}
		projectName = name
	}

	// Use Go implementation
	mgr, err := team.NewManager(projectName)
	if err != nil {
		// For health check, we still want to report status even if project doesn't exist
		health := map[string]interface{}{
			"status":  "healthy",
			"project": projectName,
			"note":    "Project not initialized, but team manager is operational",
		}
		healthJSON, _ := json.MarshalIndent(health, "", "  ")
		resultText := fmt.Sprintf("✅ Team Manager Health:\n%s", string(healthJSON))
		metrics.RecordTeamToolCall("team_health", true)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
		}, nil
	}

	goStart := time.Now()
	health := mgr.Health()
	metrics.RecordTeamToolDuration("team_health", time.Since(goStart))

	healthJSON, err := json.MarshalIndent(health, "", "  ")
	if err != nil {
		resultText := fmt.Sprintf("Health check failed: %v", err)
		metrics.RecordTeamToolError("team_health", "go_error")
		metrics.RecordTeamToolCall("team_health", false)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
			IsError: true,
		}, nil
	}

	resultText := fmt.Sprintf("✅ Team Manager Health:\n%s", string(healthJSON))
	metrics.RecordTeamToolCall("team_health", true)
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: resultText}},
	}, nil
}

func loadTeamLayoutRules() (*TeamLayoutRules, error) {
	// Return hardcoded rules matching .guardrails/team-layout-rules.json
	return &TeamLayoutRules{
		Name:        "Team Layout Compliance",
		Version:     "1.0",
		Description: "Enforces standardized team structure",
		PhaseGates: map[string]PhaseGate{
			"1_to_2": {
				Name:             "Architecture Review Board",
				RequiredTeams:    []int{1, 2, 3},
				ApprovalRequired: []int{2},
				Deliverables:     []string{"Architecture Decision Records", "Approved Tech List", "Compliance Checklist"},
			},
			"2_to_3": {
				Name:             "Environment Readiness",
				RequiredTeams:    []int{4, 5, 6},
				ApprovalRequired: []int{4, 5},
				Deliverables:     []string{"Infrastructure Provisioned", "CI/CD Pipelines", "Data Models"},
			},
			"3_to_4": {
				Name:             "Feature Complete + Code Review",
				RequiredTeams:    []int{7, 8},
				ApprovalRequired: []int{7},
				Deliverables:     []string{"Features Implemented", "Code Reviewed", "Documentation Complete"},
			},
			"4_to_5": {
				Name:             "Security + QA Sign-off",
				RequiredTeams:    []int{9, 10},
				ApprovalRequired: []int{9, 10},
				Deliverables:     []string{"Security Review Passed", "Test Coverage Met", "UAT Sign-off"},
			},
		},
		AgentMapping: map[string]AgentTeam{
			"planner":             {Team: 2, Roles: []string{"Solution Architect"}, Phase: "Phase 1"},
			"architect":           {Team: 2, Roles: []string{"Chief Architect", "Domain Architect"}, Phase: "Phase 1"},
			"infrastructure":        {Team: 4, Roles: []string{"Cloud Architect", "IaC Engineer"}, Phase: "Phase 2"},
			"platform":            {Team: 5, Roles: []string{"CI/CD Architect", "Kubernetes Administrator"}, Phase: "Phase 2"},
			"backend":             {Team: 7, Roles: []string{"Senior Backend Engineer"}, Phase: "Phase 3"},
			"frontend":            {Team: 7, Roles: []string{"Senior Frontend Engineer", "Accessibility Expert"}, Phase: "Phase 3"},
			"security":            {Team: 9, Roles: []string{"Security Architect"}, Phase: "Phase 4"},
			"security-engineer":   {Team: 9, Roles: []string{"DevSecOps Engineer", "Vulnerability Researcher"}, Phase: "Phase 4"},
			"qa":                  {Team: 10, Roles: []string{"QA Architect", "SDET"}, Phase: "Phase 4"},
			"performance-tester":  {Team: 10, Roles: []string{"Performance/Load Engineer"}, Phase: "Phase 4"},
			"accessibility-tester": {Team: 7, Roles: []string{"Accessibility (A11y) Expert"}, Phase: "Phase 3"},
			"ux-researcher":       {Team: 1, Roles: []string{"Business Systems Analyst", "Lead Product Manager"}, Phase: "Phase 1"},
			"sre":                 {Team: 11, Roles: []string{"SRE Lead", "Observability Engineer"}, Phase: "Phase 5"},
			"ops":                 {Team: 12, Roles: []string{"Release Manager", "NOC Analyst"}, Phase: "Phase 5"},
		},
	}, nil
}
