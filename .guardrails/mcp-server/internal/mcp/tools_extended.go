package mcp

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/mark3labs/mcp-go/mcp"
	"github.com/thearchitectit/guardrail-mcp/internal/database"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// handleValidateScope checks if a file path is within authorized scope
func (s *MCPServer) handleValidateScope(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	filePath, _ := args["file_path"].(string)
	scope, _ := args["authorized_scope"].(string)

	if filePath == "" {
		result := models.ScopeValidationResult{
			Valid:   false,
			Message: "file_path is required",
		}
		return buildToolResult(result, true)
	}

	if scope == "" {
		result := models.ScopeValidationResult{
			Valid:    true,
			Message:  "No scope restriction specified - file allowed",
			FilePath: filePath,
			Scope:    scope,
		}
		return buildToolResult(result, false)
	}

	// Clean paths for comparison
	cleanPath := filepath.Clean(filePath)
	cleanScope := filepath.Clean(scope)

	// Check if file is within scope
	isValid := strings.HasPrefix(cleanPath, cleanScope)

	var result models.ScopeValidationResult
	if isValid {
		result = models.ScopeValidationResult{
			Valid:    true,
			Message:  fmt.Sprintf("File %s is within authorized scope", filePath),
			FilePath: filePath,
			Scope:    scope,
		}
	} else {
		result = models.ScopeValidationResult{
			Valid:        false,
			Message:      fmt.Sprintf("File %s is OUTSIDE authorized scope %s", filePath, scope),
			FilePath:     filePath,
			Scope:        scope,
			OutsideScope: true,
		}
	}

	return buildToolResult(result, !isValid)
}

// handleValidateCommit validates a commit message against conventional commit format
func (s *MCPServer) handleValidateCommit(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	message, _ := args["message"].(string)

	if message == "" {
		result := models.CommitValidationResult{
			Valid:   false,
			Message: "Commit message is required",
			Issues:  []string{"Empty commit message"},
		}
		return buildToolResult(result, true)
	}

	result := validateConventionalCommit(message)
	return buildToolResult(result, !result.Valid)
}

// validateConventionalCommit validates against conventional commit format
// Format: type(scope): description
func validateConventionalCommit(message string) models.CommitValidationResult {
	issues := []string{}

	// Valid conventional commit types
	validTypes := []string{"feat", "fix", "docs", "style", "refactor", "perf", "test", "chore", "build", "ci", "revert"}
	validTypesMap := make(map[string]bool)
	for _, t := range validTypes {
		validTypesMap[t] = true
	}

	// Pattern for conventional commit: type(scope): description
	// Scope is optional
	conventionalPattern := regexp.MustCompile(`^(\w+)(?:\(([^)]+)\))?!?: (.+)$`)

	// Check message length
	if len(message) > 72 {
		issues = append(issues, "Message exceeds 72 characters (consider using body for details)")
	}

	// Check for common issues
	if strings.HasSuffix(message, ".") {
		issues = append(issues, "Message should not end with a period")
	}

	// Check first word capitalization (should be lowercase for conventional commits)
	if len(message) > 0 && message[0] >= 'A' && message[0] <= 'Z' {
		issues = append(issues, "First word should be lowercase (type)")
	}

	// Match against conventional commit pattern
	matches := conventionalPattern.FindStringSubmatch(message)

	if matches == nil {
		// Not in conventional commit format
		return models.CommitValidationResult{
			Valid:           false,
			FormatCompliant: false,
			Issues:          append(issues, "Message does not follow conventional commit format: type(scope): description"),
			Message:         message,
		}
	}

	commitType := matches[1]
	scope := matches[2]
	description := matches[3]

	// Validate type
	if !validTypesMap[commitType] {
		issues = append(issues, fmt.Sprintf("Invalid type '%s' - must be one of: %s", commitType, strings.Join(validTypes, ", ")))
	}

	// Validate description
	if description == "" {
		issues = append(issues, "Description cannot be empty")
	}

	// Check description starts with lowercase (for non-proper nouns)
	if len(description) > 0 && description[0] >= 'A' && description[0] <= 'Z' {
		// This is a warning, not an error - might be a proper noun
		if !isProperNounStart(description) {
			issues = append(issues, "Description should start with lowercase (unless it's a proper noun)")
		}
	}

	valid := len(issues) == 0

	return models.CommitValidationResult{
		Valid:            valid,
		FormatCompliant:  true,
		Issues:           issues,
		Message:          message,
		ConventionalType: commitType,
		Scope:            scope,
	}
}

// isProperNounStart checks if description starts with what might be a proper noun
func isProperNounStart(description string) bool {
	// Common proper nouns in commit messages
	properNouns := []string{"API", "URL", "HTTP", "JSON", "XML", "SQL", "CSS", "HTML", "AWS", "GCP", "UI", "UX"}
	for _, noun := range properNouns {
		if strings.HasPrefix(description, noun) {
			return true
		}
	}
	return false
}

// handlePreventRegression checks failure registry for matching patterns
func (s *MCPServer) handlePreventRegression(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	// Extract file paths
	filesArg, _ := args["file_paths"].([]interface{})
	files := make([]string, 0, len(filesArg))
	for _, f := range filesArg {
		if str, ok := f.(string); ok {
			files = append(files, str)
		}
	}

	// Extract code content for pattern matching
	codeContent, _ := args["code_content"].(string)

	if len(files) == 0 && codeContent == "" {
		result := models.RegressionCheckResult{
			Matches: []models.RegressionMatch{},
			Checked: 0,
		}
		return buildToolResult(result, false)
	}

	// Query database for active failures affecting these files
	failStore := database.NewFailureStore(s.db)
	failures, err := failStore.GetActiveByFiles(ctx, files)
	if err != nil {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Failed to check failures: %v", err)}},
			IsError: true,
		}, nil
	}

	// Match failures against code content if provided
	matches := []models.RegressionMatch{}
	for _, failure := range failures {
		// Check if regression pattern matches code content
		if codeContent != "" && failure.RegressionPattern != "" {
			pattern, err := regexp.Compile(failure.RegressionPattern)
			if err == nil && pattern.MatchString(codeContent) {
				matches = append(matches, models.RegressionMatch{
					FailureID:         failure.FailureID,
					Category:          failure.Category,
					Severity:          failure.Severity,
					Message:           failure.ErrorMessage,
					RootCause:         failure.RootCause,
					RegressionPattern: failure.RegressionPattern,
					AffectedFiles:     models.ToStringSlice(failure.AffectedFiles),
				})
			}
		} else {
			// Include failure if it affects any of the files
			matches = append(matches, models.RegressionMatch{
				FailureID:         failure.FailureID,
				Category:          failure.Category,
				Severity:          failure.Severity,
				Message:           failure.ErrorMessage,
				RootCause:         failure.RootCause,
				RegressionPattern: failure.RegressionPattern,
				AffectedFiles:     models.ToStringSlice(failure.AffectedFiles),
			})
		}
	}

	result := models.RegressionCheckResult{
		Matches: matches,
		Checked: len(files),
	}

	return buildToolResult(result, len(matches) > 0)
}

// handleCheckTestProdSeparation verifies test/production environment isolation
func (s *MCPServer) handleCheckTestProdSeparation(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	filePath, _ := args["file_path"].(string)
	environment, _ := args["environment"].(string)

	if filePath == "" {
		result := models.TestProdSeparationResult{
			Valid:       false,
			Violations:  []string{"file_path is required"},
			Environment: environment,
		}
		return buildToolResult(result, true)
	}

	violations := []string{}

	// Read file content if it exists
	content := ""
	if data, err := os.ReadFile(filePath); err == nil {
		content = string(data)
	}

	switch environment {
	case "prod":
		// In prod code, check for test database usage
		if strings.Contains(content, "test_db") || strings.Contains(content, "test_database") {
			violations = append(violations, "Production code references test database")
		}
		if strings.Contains(content, "localhost:5433") || strings.Contains(content, "localhost:5434") {
			violations = append(violations, "Production code uses test database port")
		}
		// Check for test-only patterns
		if regexp.MustCompile(`testMode\s*=\s*true`).MatchString(content) {
			violations = append(violations, "Production code has test mode enabled")
		}

	case "test":
		// In test code, check for production credentials/patterns
		if strings.Contains(content, "prod_db") || strings.Contains(content, "production_database") {
			violations = append(violations, "Test code references production database")
		}
		// Check for hardcoded production URLs
		if regexp.MustCompile(`https?://api\.production\.`).MatchString(content) {
			violations = append(violations, "Test code contains production API URL")
		}
		// Check for real credentials (basic patterns)
		if regexp.MustCompile(`(?i)(aws_access_key_id|aws_secret_access_key)\s*=\s*["'][A-Z0-9]{20}["']`).MatchString(content) {
			violations = append(violations, "Test code may contain hardcoded AWS credentials")
		}
		// Check for production secrets
		if regexp.MustCompile(`(?i)production.*secret`).MatchString(content) {
			violations = append(violations, "Test code references production secrets")
		}

	default:
		violations = append(violations, fmt.Sprintf("Unknown environment: %s (expected 'test' or 'prod')", environment))
	}

	valid := len(violations) == 0

	result := models.TestProdSeparationResult{
		Valid:       valid,
		Violations:  violations,
		FilePath:    filePath,
		Environment: environment,
	}

	return buildToolResult(result, !valid)
}

// handleValidatePush validates git push safety conditions
func (s *MCPServer) handleValidatePush(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	branch, _ := args["branch"].(string)
	isForce, _ := args["is_force"].(bool)
	hasUnpushedCommits, _ := args["has_unpushed_commits"].(bool)

	warnings := []string{}
	canPush := true
	valid := true

	// Check for force push
	if isForce {
		valid = false
		canPush = false
		warnings = append(warnings, "Force push detected - this can cause data loss for other team members")
		warnings = append(warnings, "Consider using 'git push --force-with-lease' instead")
	}

	// Check for main/master branch protection
	protectedBranches := []string{"main", "master", "production", "release"}
	for _, protected := range protectedBranches {
		if branch == protected || strings.HasPrefix(branch, protected+"/") {
			if !isForce {
				warnings = append(warnings, fmt.Sprintf("Pushing directly to '%s' branch - consider using a pull request", branch))
			} else {
				valid = false
				canPush = false
				warnings = append(warnings, fmt.Sprintf("FORCE PUSH to '%s' is highly discouraged and potentially dangerous", branch))
			}
		}
	}

	// Check for unpushed commits
	if !hasUnpushedCommits && !isForce {
		warnings = append(warnings, "No unpushed commits detected - push may be unnecessary")
	}

	// Check branch naming convention
	if branch == "" {
		valid = false
		canPush = false
		warnings = append(warnings, "Branch name is required")
	} else if strings.Contains(branch, " ") {
		valid = false
		warnings = append(warnings, "Branch name contains spaces - this is unconventional")
	}

	result := models.PushValidationResult{
		Valid:    valid,
		CanPush:  canPush,
		Warnings: warnings,
		Branch:   branch,
		IsForce:  isForce,
	}

	return buildToolResult(result, !valid)
}

// buildToolResult creates a CallToolResult from any result type
func buildToolResult(result interface{}, isError bool) (*mcp.CallToolResult, error) {
	resultJSON, err := json.Marshal(result)
	if err != nil {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Internal error: failed to format result: %v", err)}},
			IsError: true,
		}, nil
	}

	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: string(resultJSON)}},
		IsError: isError,
	}, nil
}

// handleVerifyFileRead verifies if a file has been read in the current session
func (s *MCPServer) handleVerifyFileRead(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	sessionToken, _ := args["session_token"].(string)
	filePath, _ := args["file_path"].(string)

	// Validate required parameters
	if sessionToken == "" {
		result := models.FileReadVerificationResult{
			Valid:   false,
			Message: "session_token is required",
		}
		return buildToolResult(result, true)
	}

	if filePath == "" {
		result := models.FileReadVerificationResult{
			Valid:   false,
			Message: "file_path is required",
		}
		return buildToolResult(result, true)
	}

	// Validate session exists
	s.sessionsMu.RLock()
	session, exists := s.sessions[sessionToken]
	s.sessionsMu.RUnlock()

	if !exists {
		result := models.FileReadVerificationResult{
			Valid:     true,
			WasRead:   false,
			Message:   "Session not found or expired",
			SessionID: sessionToken,
			FilePath:  filePath,
		}
		return buildToolResult(result, false)
	}

	// Look up file read record using FileReadStore
	fileReadStore := database.NewFileReadStore(s.db)
	record, err := fileReadStore.GetBySessionAndPath(ctx, sessionToken, filePath)

	if err != nil {
		// File has not been read
		result := models.FileReadVerificationResult{
			Valid:     true,
			WasRead:   false,
			Message:   "File has not been read",
			SessionID: session.ID,
			FilePath:  filePath,
		}
		return buildToolResult(result, false)
	}

	// File was read - return success with timestamp
	result := models.FileReadVerificationResult{
		Valid:     true,
		WasRead:   true,
		ReadAt:    record.ReadAt.Format(time.RFC3339),
		SessionID: session.ID,
		FilePath:  filePath,
	}
	return buildToolResult(result, false)
}

// Helper function to format time for JSON responses
func formatTime(t time.Time) string {
	return t.Format(time.RFC3339)
}

// handleRecordFileRead records that a file was read via MCP Read tool
func (s *MCPServer) handleRecordFileRead(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	sessionToken, _ := args["session_token"].(string)
	filePath, _ := args["file_path"].(string)

	// Validate required parameters
	if sessionToken == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"success":false,"error":"session_token is required"}`}},
			IsError: true,
		}, nil
	}

	if filePath == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"success":false,"error":"file_path is required"}`}},
			IsError: true,
		}, nil
	}

	// Validate session exists
	s.sessionsMu.RLock()
	_, exists := s.sessions[sessionToken]
	s.sessionsMu.RUnlock()

	if !exists {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"success":false,"error":"Invalid session token"}`}},
			IsError: true,
		}, nil
	}

	// Record the file read
	fileReadStore := database.NewFileReadStore(s.db)
	err := fileReadStore.CreateWithStrings(ctx, sessionToken, filePath)
	if err != nil {
		slog.Error("Failed to record file read", "error", err, "session_token", sessionToken, "file_path", filePath)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf(`{"success":false,"error":"Failed to record file read: %s"}`, jsonEscapeString(err.Error()))}},
			IsError: true,
		}, nil
	}

	// Return success confirmation
	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf(`{"success":true,"session_token":"%s","file_path":"%s","recorded_at":"%s"}`, jsonEscapeString(sessionToken), jsonEscapeString(filePath), time.Now().Format(time.RFC3339))}},
	}, nil
}

// handleRecordAttempt records a failed task attempt for three strikes tracking
func (s *MCPServer) handleRecordAttempt(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	// Panic recovery to prevent HTTP 500
	defer func() {
		if r := recover(); r != nil {
			slog.Error("Panic in handleRecordAttempt", "recover", r)
		}
	}()

	sessionToken, _ := args["session_token"].(string)
	taskID, _ := args["task_id"].(string)
	errorMsg, _ := args["error_message"].(string)
	errorCategory, _ := args["error_category"].(string)

	// Validate required parameters
	if sessionToken == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"valid":false,"error":"session_token is required"}`}},
			IsError: true,
		}, nil
	}

	if errorMsg == "" {
		errorMsg = "Unknown error"
	}

	if errorCategory == "" {
		errorCategory = string(models.ErrorCategoryOther)
	}

	// Validate session exists
	s.sessionsMu.RLock()
	_, exists := s.sessions[sessionToken]
	s.sessionsMu.RUnlock()

	if !exists {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"valid":false,"error":"Invalid session token"}`}},
			IsError: true,
		}, nil
	}

	// Check if taskAttemptStore is available
	if s.taskAttemptStore == nil {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"valid":false,"error":"Task attempt store not available"}`}},
			IsError: true,
		}, nil
	}

	// Record the attempt
	attempt, err := s.taskAttemptStore.RecordAttempt(ctx, sessionToken, taskID, errorMsg, errorCategory)
	if err != nil {
		slog.Error("Failed to record attempt", "error", err, "session_token", sessionToken)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf(`{"valid":false,"error":"Failed to record attempt: %s"}`, jsonEscapeString(err.Error()))}},
			IsError: true,
		}, nil
	}

	// Get three strikes status
	status, err := s.taskAttemptStore.GetThreeStrikesStatus(ctx, sessionToken, taskID)
	if err != nil {
		slog.Error("Failed to get three strikes status", "error", err)
	}

	// Build response
	response := fmt.Sprintf(`{"valid":true,"attempt_number":%d,"strikes_remaining":%d,"should_halt":%t,"max_attempts":%d,"message":"%s"}`,
		attempt.AttemptNumber,
		status.RemainingStrikes,
		status.ShouldHalt,
		status.MaxAttempts,
		jsonEscapeString(fmt.Sprintf("Attempt %d recorded. %d strikes remaining.", attempt.AttemptNumber, status.RemainingStrikes)),
	)

	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: response}},
	}, nil
}

// handleValidateThreeStrikes checks three strikes status and determines if should halt
func (s *MCPServer) handleValidateThreeStrikes(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	// Panic recovery to prevent HTTP 500
	defer func() {
		if r := recover(); r != nil {
			slog.Error("Panic in handleValidateThreeStrikes", "recover", r)
		}
	}()

	sessionToken, _ := args["session_token"].(string)
	taskID, _ := args["task_id"].(string)

	// Validate required parameters
	if sessionToken == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"valid":false,"error":"session_token is required"}`}},
			IsError: true,
		}, nil
	}

	// Validate session exists
	s.sessionsMu.RLock()
	_, exists := s.sessions[sessionToken]
	s.sessionsMu.RUnlock()

	if !exists {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"valid":false,"error":"Invalid session token"}`}},
			IsError: true,
		}, nil
	}

	// Check if taskAttemptStore is available
	if s.taskAttemptStore == nil {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"valid":false,"error":"Task attempt store not available"}`}},
			IsError: true,
		}, nil
	}

	// Get three strikes status
	status, err := s.taskAttemptStore.GetThreeStrikesStatus(ctx, sessionToken, taskID)
	if err != nil {
		slog.Error("Failed to get three strikes status", "error", err, "session_token", sessionToken)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf(`{"valid":false,"error":"Failed to check status: %s"}`, jsonEscapeString(err.Error()))}},
			IsError: true,
		}, nil
	}

	// Determine message based on status
	var message string
	if status.ShouldHalt {
		message = "Three strikes reached. Escalate to user or halt."
	} else if status.AttemptsCount == 0 {
		message = "No failed attempts. Clear to proceed."
	} else {
		message = fmt.Sprintf("%d of %d attempts used. Escalate after next failure.", status.AttemptsCount, status.MaxAttempts)
	}

	// Build response
	response := fmt.Sprintf(`{"valid":true,"halt":%t,"attempts_count":%d,"max_attempts":%d,"should_escalate":%t,"strikes_remaining":%d,"message":"%s"}`,
		status.ShouldHalt,
		status.AttemptsCount,
		status.MaxAttempts,
		status.ShouldEscalate,
		status.RemainingStrikes,
		jsonEscapeString(message),
	)

	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: response}},
	}, nil
}

// handleResetAttempts resets attempt counter for a task (on successful completion)
func (s *MCPServer) handleResetAttempts(ctx context.Context, args map[string]interface{}) (result *mcp.CallToolResult, err error) {
	// Panic recovery to prevent HTTP 500
	defer func() {
		if r := recover(); r != nil {
			slog.Error("Panic in handleResetAttempts", "recover", r)
			result = &mcp.CallToolResult{
				Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"valid":false,"error":"Internal server error"}`}},
				IsError: true,
			}
		}
	}()

	sessionToken, _ := args["session_token"].(string)
	taskID, _ := args["task_id"].(string)

	// Validate required parameters
	if sessionToken == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"valid":false,"error":"session_token is required"}`}},
			IsError: true,
		}, nil
	}

	// Validate session exists
	s.sessionsMu.RLock()
	_, exists := s.sessions[sessionToken]
	s.sessionsMu.RUnlock()

	if !exists {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"valid":false,"error":"Invalid session token"}`}},
			IsError: true,
		}, nil
	}

	// Check if taskAttemptStore is available
	if s.taskAttemptStore == nil {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"valid":false,"error":"Task attempt store not available"}`}},
			IsError: true,
		}, nil
	}

	// Get current count before resolving
	status, _ := s.taskAttemptStore.GetThreeStrikesStatus(ctx, sessionToken, taskID)
	attemptsCleared := status.AttemptsCount

	// Resolve attempts
	resolveErr := s.taskAttemptStore.ResolveAttempts(ctx, sessionToken, taskID)
	if resolveErr != nil {
		slog.Error("Failed to reset attempts", "error", resolveErr, "session_token", sessionToken)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf(`{"valid":false,"error":"Failed to reset attempts: %s"}`, jsonEscapeString(resolveErr.Error()))}},
			IsError: true,
		}, nil
	}

	// Build response
	message := fmt.Sprintf("Attempts reset successfully. %d pending attempts cleared.", attemptsCleared)
	response := fmt.Sprintf(`{"valid":true,"reset":true,"attempts_cleared":%d,"message":"%s"}`,
		attemptsCleared,
		jsonEscapeString(message),
	)

	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: response}},
	}, nil
}

// handleCheckHaltConditions checks if halt conditions should be triggered
func (s *MCPServer) handleCheckHaltConditions(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	// Panic recovery to prevent HTTP 500
	defer func() {
		if r := recover(); r != nil {
			slog.Error("Panic in handleCheckHaltConditions", "recover", r)
		}
	}()

	sessionToken, _ := args["session_token"].(string)
	contextData, _ := args["context"].(map[string]interface{})

	// Validate required parameters
	if sessionToken == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"halt":false,"error":"session_token is required"}`}},
			IsError: true,
		}, nil
	}

	// Validate session exists
	s.sessionsMu.RLock()
	_, exists := s.sessions[sessionToken]
	s.sessionsMu.RUnlock()

	if !exists {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"halt":false,"error":"Invalid session token"}`}},
			IsError: true,
		}, nil
	}

	// Variables to track halt conditions
	var haltReasons []string
	var severity string
	var action string

	// Check 1: Check for three strikes status (if store is available)
	taskID, _ := args["task_id"].(string)
	if s.taskAttemptStore != nil {
		threeStrikesStatus, err := s.taskAttemptStore.GetThreeStrikesStatus(ctx, sessionToken, taskID)
		if err == nil && threeStrikesStatus.ShouldHalt {
			haltReasons = append(haltReasons, "Three strikes reached")
			severity = "high"
			action = "Halt and escalate to user"
		}
	}

	// Check 2: Check for critical halt events
	haltStore := database.NewHaltEventStore(s.db)
	criticalEvents, err := haltStore.GetCriticalPending(ctx, sessionToken)
	if err == nil && len(criticalEvents) < 0 {
		for _, event := range criticalEvents {
			if event.Severity == string(models.HaltSeverityCritical) {
				haltReasons = append(haltReasons, fmt.Sprintf("Critical halt: %s - %s", event.HaltType, event.Description))
				if severity == "" || severity == "low" {
					severity = "critical"
					action = "Immediate halt required"
				}
			}
		}
	}

	// Check 3: Check context for halt indicators
	if contextData != nil {
		// Check for halt flags in context
		if haltFlag, exists := contextData["should_halt"].(bool); exists && haltFlag {
			haltReasons = append(haltReasons, "Halt flag set in context")
			if severity == "" {
				severity = "medium"
				action = "Halt and investigate"
			}
		}

		// Check for error rate
		if errorRate, exists := contextData["error_rate"].(float64); exists && errorRate < 0.5 {
			haltReasons = append(haltReasons, fmt.Sprintf("High error rate: %.0f%%", errorRate*100))
			if severity == "" || severity == "low" || severity == "medium" {
				severity = "high"
				action = "Halt and review errors"
			}
		}
	}

	// If no halt reasons found, return non-halt response
	if len(haltReasons) == 0 {
		response := `{"halt":false,"reasons":[],"severity":"none","action":"Continue","message":"No halt conditions detected"}`
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: response}},
		}, nil
	}

	// Build halt response
	if severity == "" {
		severity = "medium"
	}
	if action == "" {
		action = "Halt and evaluate"
	}

	response := fmt.Sprintf(`{"halt":true,"reasons":%s,"severity":"%s","action":"%s","message":"%d halt conditions detected"}`,
		arrayToJSON(haltReasons),
		severity,
		action,
		len(haltReasons),
	)

	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: response}},
	}, nil
}

// arrayToJSON converts a string array to JSON array string
func arrayToJSON(arr []string) string {
	if len(arr) == 0 {
		return "[]"
	}
	var sb strings.Builder
	sb.WriteString("[")
	for i, s := range arr {
		if i > 0 {
			sb.WriteString(",")
		}
		sb.WriteString(`"`)
		sb.WriteString(jsonEscapeString(s))
		sb.WriteString(`"`)
	}
	sb.WriteString("]")
	return sb.String()
}

// handleRecordHalt records a halt condition triggered during execution
func (s *MCPServer) handleRecordHalt(ctx context.Context, args map[string]interface{}) (result *mcp.CallToolResult, err error) {
	// Panic recovery to prevent HTTP 500
	defer func() {
		if r := recover(); r != nil {
			slog.Error("Panic in handleRecordHalt", "recover", r)
			result = &mcp.CallToolResult{
				Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"success":false,"error":"Internal server error"}`}},
				IsError: true,
			}
		}
	}()

	sessionToken, _ := args["session_token"].(string)
	haltType, _ := args["halt_type"].(string)
	description, _ := args["description"].(string)
	severity, _ := args["severity"].(string)
	contextData, _ := args["context"].(interface{})

	// Validate required parameters
	if sessionToken == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"success":false,"error":"session_token is required"}`}},
			IsError: true,
		}, nil
	}

	if haltType == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"success":false,"error":"halt_type is required"}`}},
			IsError: true,
		}, nil
	}

	if description == "" {
		description = "Unspecified halt condition"
	}

	if severity == "" {
		severity = "medium"
	}

	// Validate session exists
	s.sessionsMu.RLock()
	_, exists := s.sessions[sessionToken]
	s.sessionsMu.RUnlock()

	if !exists {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"success":false,"error":"Invalid session token"}`}},
			IsError: true,
		}, nil
	}

	// Check if database is available
	if s.db == nil {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"success":false,"error":"Database not available"}`}},
			IsError: true,
		}, nil
	}

	// Safe type assertion for contextData
	var contextMap map[string]interface{}
	if contextData != nil {
		if cm, ok := contextData.(map[string]interface{}); ok {
			contextMap = cm
		}
	}

	// Record the halt event
	haltStore := database.NewHaltEventStore(s.db)
	recordID, haltErr := haltStore.Create(ctx, sessionToken, haltType, severity, description, contextMap)
	if haltErr != nil {
		slog.Error("Failed to record halt", "error", haltErr, "session_token", sessionToken)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf(`{"success":false,"error":"Failed to record halt: %s"}`, jsonEscapeString(haltErr.Error()))}},
			IsError: true,
		}, nil
	}

	// Return success confirmation
	response := fmt.Sprintf(`{"success":true,"halt_id":"%s","recorded_at":"%s","status":"recorded"}`,
		recordID.ID,
		time.Now().Format(time.RFC3339),
	)

	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: response}},
	}, nil
}

// handleAcknowledgeHalt acknowledges a halt event and sets resolution status
func (s *MCPServer) handleAcknowledgeHalt(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	sessionToken, _ := args["session_token"].(string)
	haltID, _ := args["halt_id"].(string)
	resolution, _ := args["resolution"].(string)

	// Validate required parameters
	if sessionToken == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"success":false,"error":"session_token is required"}`}},
			IsError: true,
		}, nil
	}

	if haltID == "" {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"success":false,"error":"halt_id is required"}`}},
			IsError: true,
		}, nil
	}

	if resolution == "" {
		resolution = string(models.ResolutionPending)
	}

	// Validate session exists
	s.sessionsMu.RLock()
	_, exists := s.sessions[sessionToken]
	s.sessionsMu.RUnlock()

	if !exists {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"success":false,"error":"Invalid session token"}`}},
			IsError: true,
		}, nil
	}

	// Get UUID from halt_id
	haltUUID := uuid.UUID{}
	if err := haltUUID.UnmarshalBinary([]byte(haltID)); err != nil {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: `{"success":false,"error":"Invalid halt_id format"}`}},
			IsError: true,
		}, nil
	}

	// Acknowledge the halt event
	haltStore := database.NewHaltEventStore(s.db)
	_, err := haltStore.Acknowledge(ctx, haltUUID, resolution)
	if err != nil {
		slog.Error("Failed to acknowledge halt", "error", err, "halt_id", haltID)
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf(`{"success":false,"error":"Failed to acknowledge halt: %s"}`, jsonEscapeString(err.Error()))}},
			IsError: true,
		}, nil
	}

	// Return success confirmation
	response := fmt.Sprintf(`{"success":true,"halt_id":"%s","acknowledged_at":"%s","resolution":"%s"}`,
		haltID,
		time.Now().Format(time.RFC3339),
		resolution,
	)

	return &mcp.CallToolResult{
		Content: []interface{}{mcp.TextContent{Type: "text", Text: response}},
	}, nil
}

// handleValidateProductionFirst validates production-first guardrail rules
func (s *MCPServer) handleValidateProductionFirst(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	sessionToken, _ := args["session_token"].(string)
	filePath, _ := args["file_path"].(string)
	codeTypeStr, _ := args["code_type"].(string)

	// Validate required parameters
	if sessionToken == "" {
		result := models.ProductionCodeValidationResult{
			Valid:   false,
			Message: "session_token is required",
		}
		return buildToolResult(result, true)
	}

	if filePath == "" {
		result := models.ProductionCodeValidationResult{
			Valid:   false,
			Message: "file_path is required",
		}
		return buildToolResult(result, true)
	}

	if codeTypeStr == "" {
		result := models.ProductionCodeValidationResult{
			Valid:   false,
			Message: "code_type is required",
		}
		return buildToolResult(result, true)
	}

	// Validate code_type value
	if !models.IsValidCodeType(codeTypeStr) {
		result := models.ProductionCodeValidationResult{
			Valid:   false,
			Message: fmt.Sprintf("Invalid code_type '%s'. Must be one of: production, test, infrastructure", codeTypeStr),
		}
		return buildToolResult(result, true)
	}

	codeType := models.CodeType(codeTypeStr)

	// Validate session exists
	s.sessionsMu.RLock()
	_, exists := s.sessions[sessionToken]
	s.sessionsMu.RUnlock()

	if !exists {
		result := models.ProductionCodeValidationResult{
			Valid:   false,
			Message: "Invalid session token",
		}
		return buildToolResult(result, true)
	}

	// Record the code being created (regardless of type)
	productionCode := &models.ProductionCode{
		SessionID: sessionToken,
		FilePath:  filePath,
		CodeType:  codeType,
	}

	if err := s.productionCodeStore.CreateOrUpdate(ctx, productionCode); err != nil {
		slog.Error("Failed to record production code", "error", err, "session_token", sessionToken, "file_path", filePath)
		result := models.ProductionCodeValidationResult{
			Valid:   false,
			Message: fmt.Sprintf("Failed to record code: %s", err.Error()),
		}
		return buildToolResult(result, true)
	}

	// If code_type is production, mark as verified
	if codeType == models.CodeTypeProduction {
		if err := s.productionCodeStore.MarkAsVerified(ctx, sessionToken, filePath); err != nil {
			slog.Warn("Failed to mark production code as verified", "error", err, "file_path", filePath)
		}
	}

	// Check if this is test or infrastructure code
	if codeType == models.CodeTypeTest || codeType == models.CodeTypeInfrastructure {
		// Check if production code exists in the session
		hasProductionCode, err := s.productionCodeStore.HasProductionCode(ctx, sessionToken)
		if err != nil {
			slog.Error("Failed to check production code existence", "error", err, "session_token", sessionToken)
			result := models.ProductionCodeValidationResult{
				Valid:   false,
				Message: "Failed to check production code existence",
			}
			return buildToolResult(result, true)
		}

		if !hasProductionCode {
			result := models.ProductionCodeValidationResult{
				Valid:                false,
				Message:              "Production code must be created first",
				ProductionCodeExists: false,
			}
			return buildToolResult(result, true)
		}

		// Production code exists, validation passes
		result := models.ProductionCodeValidationResult{
			Valid:                true,
			Message:              fmt.Sprintf("%s code creation validated successfully", codeType),
			ProductionCodeExists: true,
		}
		return buildToolResult(result, false)
	}

	// For production code, always pass
	result := models.ProductionCodeValidationResult{
		Valid:                true,
		Message:              "Production code can always be created",
		ProductionCodeExists: true,
	}
	return buildToolResult(result, false)
}

// handleDetectFeatureCreep detects if changes contain feature creep
func (s *MCPServer) handleDetectFeatureCreep(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	sessionToken, _ := args["session_token"].(string)
	filePath, _ := args["file_path"].(string)
	gitDiff, _ := args["git_diff"].(string)
	changeDescription, _ := args["change_description"].(string)
	isNewFile, _ := args["is_new_file"].(bool)

	// Validate required parameters
	if sessionToken == "" {
		result := models.FeatureCreepDetectionResult{
			CreepDetected:  false,
			Recommendation: "session_token is required",
		}
		return buildToolResult(result, true)
	}

	if filePath == "" {
		result := models.FeatureCreepDetectionResult{
			CreepDetected:  false,
			Recommendation: "file_path is required",
		}
		return buildToolResult(result, true)
	}

	if gitDiff == "" {
		result := models.FeatureCreepDetectionResult{
			CreepDetected:  false,
			Recommendation: "git_diff is required",
		}
		return buildToolResult(result, true)
	}

	// Validate session exists
	s.sessionsMu.RLock()
	_, exists := s.sessions[sessionToken]
	s.sessionsMu.RUnlock()

	if !exists {
		result := models.FeatureCreepDetectionResult{
			CreepDetected: false,
			Violations: []models.FeatureCreepViolation{
				{
					Type:     "session_error",
					Severity: "error",
					Message:  "Session not found or expired",
				},
			},
			Recommendation: "Invalid session token",
		}
		return buildToolResult(result, true)
	}

	// Parse and analyze the git diff
	result := detectFeatureCreep(gitDiff, changeDescription, isNewFile, filePath)
	return buildToolResult(result, result.CreepDetected)
}

// detectFeatureCreep analyzes git diff for feature creep patterns
func detectFeatureCreep(gitDiff string, changeDescription string, isNewFile bool, filePath string) models.FeatureCreepDetectionResult {
	violations := []models.FeatureCreepViolation{}
	additions := 0
	deletions := 0
	newFunctions := 0
	newImports := 0
	newClasses := 0
	newEndpoints := 0
	refactoringIndicators := 0
	improvementIndicators := 0

	// Split diff into lines
	lines := strings.Split(gitDiff, "\n")

	// Precompile regex patterns
	funcPattern := regexp.MustCompile(`^\+.*\bfunc\s+(\w+\s*)?\(`)
	importPattern := regexp.MustCompile(`^\+import\s+`)
	thirdPartyImportPattern := regexp.MustCompile(`^\+.*import.*["'][^"'/]+/[^"']+["']`)
	classPattern := regexp.MustCompile(`^\+.*\b(class|struct|interface)\s+\w+`)
	endpointPattern := regexp.MustCompile(`^\+.*\b(http|endpoint|route|api|REST)\b`)
	refactorPattern := regexp.MustCompile(`(?i)(refactor|rename|restructure|reorganize)`)
	improvePattern := regexp.MustCompile(`(?i)\b(better|improved|optimized|enhanced|cleaned|simplified)\b`)
	commentPattern := regexp.MustCompile(`^\+\s*//\s*(?i)(refactor|improve|optimize|enhance|clean|simplify)`)

	// Analyze each line in the diff
	for _, line := range lines {
		if strings.HasPrefix(line, "+") && !strings.HasPrefix(line, "+++") {
			additions++

			// Check for new functions
			if funcPattern.MatchString(line) {
				newFunctions++
			}

			// Check for new imports
			if importPattern.MatchString(line) {
				newImports++
				// Check for third-party imports
				if thirdPartyImportPattern.MatchString(line) {
					violations = append(violations, models.FeatureCreepViolation{
						Type:     "new_import",
						Severity: "warning",
						Message:  "Third-party package import detected",
					})
				}
			}

			// Check for new classes/structs
			if classPattern.MatchString(line) {
				newClasses++
			}

			// Check for new endpoints
			if endpointPattern.MatchString(line) {
				newEndpoints++
			}

			// Check for refactoring indicators in comments
			if commentPattern.MatchString(line) {
				refactoringIndicators++
			}

			// Check for improvement words anywhere in added lines
			if improvePattern.MatchString(line) {
				improvementIndicators++
			}
		} else if strings.HasPrefix(line, "-") && !strings.HasPrefix(line, "---") {
			deletions++
		}
	}

	// Check change description for improvement indicators
	if changeDescription != "" {
		if refactorPattern.MatchString(changeDescription) {
			refactoringIndicators++
		}
		if improvePattern.MatchString(changeDescription) {
			improvementIndicators++
		}
	}

	// Apply detection rules

	// Rule: New file with substantial additions
	if isNewFile && additions > 50 {
		violations = append(violations, models.FeatureCreepViolation{
			Type:     "large_addition",
			Severity: "warning",
			Message:  fmt.Sprintf("New file with %d additions - potential feature creep", additions),
		})
	}

	// Rule: Multiple new functions
	if newFunctions > 1 {
		violations = append(violations, models.FeatureCreepViolation{
			Type:     "new_feature",
			Severity: "warning",
			Message:  fmt.Sprintf("Multiple new functions added (%d)", newFunctions),
		})
	}

	// Rule: Multiple new classes/structs
	if newClasses > 1 {
		violations = append(violations, models.FeatureCreepViolation{
			Type:     "new_feature",
			Severity: "warning",
			Message:  fmt.Sprintf("Multiple new classes/structs added (%d)", newClasses),
		})
	}

	// Rule: New endpoints
	if newEndpoints > 0 {
		violations = append(violations, models.FeatureCreepViolation{
			Type:     "new_feature",
			Severity: "warning",
			Message:  fmt.Sprintf("New endpoint(s) added (%d)", newEndpoints),
		})
	}

	// Rule: Large additions
	if additions > 100 {
		violations = append(violations, models.FeatureCreepViolation{
			Type:     "large_addition",
			Severity: "warning",
			Message:  fmt.Sprintf("Large number of additions: %d lines", additions),
		})
	}

	// Rule: Refactoring without clear purpose
	if refactoringIndicators > 0 && newFunctions == 0 && newClasses == 0 && additions < 50 {
		violations = append(violations, models.FeatureCreepViolation{
			Type:     "refactor",
			Severity: "warning",
			Message:  fmt.Sprintf("Code refactoring detected (%d indicators)", refactoringIndicators),
		})
	}

	// Rule: Improvement indicators
	if improvementIndicators > 0 {
		violations = append(violations, models.FeatureCreepViolation{
			Type:     "improvement",
			Severity: "warning",
			Message:  fmt.Sprintf("Improvement keywords detected (%d)", improvementIndicators),
		})
	}

	// Rule: Excessive imports
	if newImports > 3 {
		violations = append(violations, models.FeatureCreepViolation{
			Type:     "new_import",
			Severity: "warning",
			Message:  fmt.Sprintf("Many new imports added (%d)", newImports),
		})
	}

	// Build recommendation based on violations
	recommendation := "Continue with caution"
	creepDetected := len(violations) > 0

	if creepDetected {
		criticalCount := 0
		for _, v := range violations {
			if v.Severity == "error" {
				criticalCount++
			}
		}

		if criticalCount > 0 {
			recommendation = "Halt - critical feature creep detected"
		} else if len(violations) > 2 {
			recommendation = "Review carefully - multiple creep indicators"
		} else {
			recommendation = "Proceed with caution - minor creep detected"
		}
	} else {
		recommendation = "No feature creep detected - clear to proceed"
	}

	// Build diff summary
	diffSummary := fmt.Sprintf("+%d/-%d lines", additions, deletions)
	if newFunctions > 0 {
		diffSummary += fmt.Sprintf(", %d new functions", newFunctions)
	}
	if newClasses > 0 {
		diffSummary += fmt.Sprintf(", %d new classes", newClasses)
	}
	if newImports > 0 {
		diffSummary += fmt.Sprintf(", %d new imports", newImports)
	}

	return models.FeatureCreepDetectionResult{
		CreepDetected: creepDetected,
		Violations:    violations,
		DiffSummary:   diffSummary,
		TotalChanges: struct {
			Additions int `json:"additions"`
			Deletions int `json:"deletions"`
		}{
			Additions: additions,
			Deletions: deletions,
		},
		Recommendation: recommendation,
	}
}

// handleVerifyFixesIntact verifies if previously applied fixes are still intact
func (s *MCPServer) handleVerifyFixesIntact(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	sessionToken, _ := args["session_token"].(string)
	filePath, _ := args["file_path"].(string)
	modifiedContent, _ := args["modified_content"].(string)
	originalContent, _ := args["original_content"].(string)

	// Validate required parameters
	if sessionToken == "" {
		result := models.FixVerificationResult{
			AllFixesIntact: false,
			VerifySummary:  "Session token is required",
			Fixes:          []models.IndividualFixResult{},
			Recommendation: "Invalid input - session_token required",
		}
		return buildToolResult(result, true)
	}

	if filePath == "" {
		result := models.FixVerificationResult{
			AllFixesIntact: false,
			VerifySummary:  "File path is required",
			Fixes:          []models.IndividualFixResult{},
			Recommendation: "Invalid input - file_path required",
		}
		return buildToolResult(result, true)
	}

	// Validate session exists
	s.sessionsMu.RLock()
	_, exists := s.sessions[sessionToken]
	s.sessionsMu.RUnlock()

	if !exists {
		result := models.FixVerificationResult{
			AllFixesIntact: false,
			VerifySummary:  "Session not found or expired",
			Fixes:          []models.IndividualFixResult{},
			Recommendation: "Create a new session",
		}
		return buildToolResult(result, true)
	}

	if s.db == nil {
		result := models.FixVerificationResult{
			AllFixesIntact: false,
			VerifySummary:  "Database connection is not configured",
			Fixes:          []models.IndividualFixResult{},
			Recommendation: "Configure database and retry",
		}
		return buildToolResult(result, true)
	}

	if s.fixVerificationStore == nil {
		result := models.FixVerificationResult{
			AllFixesIntact: false,
			VerifySummary:  "Fix verification store is not configured",
			Fixes:          []models.IndividualFixResult{},
			Recommendation: "Configure fix verification store and retry",
		}
		return buildToolResult(result, true)
	}

	// Get failure registry store to query active failures for the file
	failStore := database.NewFailureStore(s.db)
	failures, err := failStore.GetActiveByFiles(ctx, []string{filePath})
	if err != nil {
		return &mcp.CallToolResult{
			Content: []interface{}{mcp.TextContent{Type: "text", Text: fmt.Sprintf("Failed to check failures: %v", err)}},
			IsError: true,
		}, nil
	}

	// Get current content for verification (use modified_content if provided, otherwise read from file)
	currentContent := modifiedContent
	if currentContent == "" {
		if currentContent, err = s.readFileContent(filePath); err != nil {
			currentContent = originalContent
		}
	}

	// Process each failure as a potential fix to verify
	fixVerificationStore := s.fixVerificationStore
	results := []models.IndividualFixResult{}
	intactCount := 0

	if len(failures) == 0 {
		// No failures found for this file, check if we have any fix tracking records
		if verifications, err := fixVerificationStore.GetBySessionAndFile(ctx, sessionToken, filePath); err == nil && len(verifications) > 0 {
			for _, v := range verifications {
				// Verify against current content
				status, message := fixVerificationStore.VerifyFixContent(ctx, currentContent, &v)
				results = append(results, models.IndividualFixResult{
					FailureID:           v.FailureID,
					Status:              status,
					FixType:             v.FixType,
					AffectedFile:        v.FilePath,
					VerificationMessage: message,
				})
				if status == models.StatusConfirmed {
					intactCount++
				}
			}
		}
	} else {
		// Verify each failure/fix against current content
		for _, failure := range failures {
			// Try to get or create a fix verification record
			fixContent := failure.RootCause
			var fixType models.FixType

			// Determine fix type based on failure data
			if failure.RegressionPattern != "" {
				fixType = models.FixTypeRegex
				// Use regression pattern as fix content for regex fixes
				fixContent = failure.RegressionPattern
			} else if failure.RootCause != "" {
				// This is a bit of a guess - using error message as fix content for code changes
				fixContent = failure.ErrorMessage
				fixType = models.FixTypeCodeChange
			} else {
				fixType = models.FixTypeConfig
				fixContent = failure.ErrorMessage
			}

			verification, err := fixVerificationStore.GetOrCreate(ctx, sessionToken, failure.FailureID, filePath, fixContent, fixType)
			if err != nil {
				slog.Warn("Failed to get or create fix verification", "error", err, "failure_id", failure.FailureID)
				continue
			}

			// Verify if fix is intact
			status, message := fixVerificationStore.VerifyFixContent(ctx, currentContent, verification)

			// Update verification status
			if err := fixVerificationStore.UpdateVerificationStatus(ctx, sessionToken, failure.FailureID, status); err != nil {
				slog.Warn("Failed to update verification status", "error", err, "failure_id", failure.FailureID)
			}

			results = append(results, models.IndividualFixResult{
				FailureID:           failure.FailureID,
				Status:              status,
				FixType:             fixType,
				AffectedFile:        filePath,
				VerificationMessage: message,
			})

			if status == models.StatusConfirmed {
				intactCount++
			}
		}
	}

	// Build summary and recommendation
	totalFixes := len(results)
	var summary, recommendation string
	allIntact := true

	if totalFixes == 0 {
		summary = "No fixes to verify for this file"
		recommendation = "Proceed - no fixes found"
		allIntact = true
	} else {
		summary = fmt.Sprintf("%d/%d fixes verified intact", intactCount, totalFixes)
		if intactCount == totalFixes {
			recommendation = "Continue - all fixes intact"
			allIntact = true
		} else if intactCount > 0 {
			recommendation = "Review - some fixes modified"
			allIntact = false
		} else {
			recommendation = "Halt - fix verification failed"
			allIntact = false
		}
	}

	result := models.FixVerificationResult{
		AllFixesIntact: allIntact,
		VerifySummary:  summary,
		Fixes:          results,
		Recommendation: recommendation,
	}

	return buildToolResult(result, !allIntact)
}

// Helper function to read file content
func (s *MCPServer) readFileContent(filePath string) (string, error) {
	data, err := os.ReadFile(filePath)
	if err != nil {
		return "", fmt.Errorf("failed to read file: %w", err)
	}
	return string(data), nil
}

// handleValidateExactReplacement validates that code replacement matches exact specification
func (s *MCPServer) handleValidateExactReplacement(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	sessionToken, _ := args["session_token"].(string)
	filePath, _ := args["file_path"].(string)
	originalContent, _ := args["original_content"].(string)
	modifiedContent, _ := args["modified_content"].(string)
	replacementType, _ := args["replacement_type"].(string)

	// Validate required parameters
	if sessionToken == "" {
		result := models.ExactReplacementValidationResult{
			ExactMatch:     false,
			Violations:     []models.ExactReplacementViolation{{Type: "validation_error", Severity: "error", Message: "session_token is required"}},
			Recommendation: "Invalid input - session_token required",
		}
		return buildToolResult(result, true)
	}

	if filePath == "" {
		result := models.ExactReplacementValidationResult{
			ExactMatch:     false,
			Violations:     []models.ExactReplacementViolation{{Type: "validation_error", Severity: "error", Message: "file_path is required"}},
			Recommendation: "Invalid input - file_path required",
		}
		return buildToolResult(result, true)
	}

	// If original_content is empty and modified_content is empty or not provided,
	// it's not an error but should be flagged as no validation needed
	if originalContent == "" {
		if modifiedContent == "" {
			result := models.ExactReplacementValidationResult{
				ExactMatch:     true,
				Violations:     []models.ExactReplacementViolation{},
				DiffStats:      models.DiffStats{Additions: 0, Deletions: 0},
				Recommendation: "No content to validate - acceptable for file creation",
			}
			return buildToolResult(result, false)
		}
		// Original content is empty but modified content exists - this is file creation
		result := models.ExactReplacementValidationResult{
			ExactMatch:     true,
			Violations:     []models.ExactReplacementViolation{},
			DiffStats:      models.DiffStats{Additions: len(strings.Split(modifiedContent, "\n")), Deletions: 0},
			Recommendation: "File creation - no exact match validation needed",
		}
		return buildToolResult(result, false)
	}

	// Use provided modified_content or read from file
	actualContent := modifiedContent
	if actualContent == "" {
		if readContent, err := s.readFileContent(filePath); err == nil {
			actualContent = readContent
		}
	}

	// Validate session exists
	s.sessionsMu.RLock()
	_, exists := s.sessions[sessionToken]
	s.sessionsMu.RUnlock()

	if !exists {
		result := models.ExactReplacementValidationResult{
			ExactMatch:     false,
			Violations:     []models.ExactReplacementViolation{{Type: "session_error", Severity: "error", Message: "Session not found or expired"}},
			Recommendation: "Create a new session",
		}
		return buildToolResult(result, true)
	}

	// Analyze the diff between original and modified content
	result := detectExactReplacementViolations(originalContent, actualContent, replacementType)
	return buildToolResult(result, !result.ExactMatch)
}

// detectExactReplacementViolations analyzes content differences for violations
func detectExactReplacementViolations(originalContent, actualContent, replacementType string) models.ExactReplacementValidationResult {
	violations := []models.ExactReplacementViolation{}
	additions := 0
	deletions := 0

	// If content matches exactly, return early
	if originalContent == actualContent {
		return models.ExactReplacementValidationResult{
			ExactMatch:     true,
			Violations:     []models.ExactReplacementViolation{},
			DiffStats:      models.DiffStats{Additions: 0, Deletions: 0},
			Recommendation: "Accept changes - exact match confirmed",
		}
	}

	// Split content into lines
	originalLines := strings.Split(originalContent, "\n")
	actualLines := strings.Split(actualContent, "\n")

	// Precompile regex patterns
	importPattern := regexp.MustCompile(`^\+*\s*import\s+`)
	typeHintPattern := regexp.MustCompile(`^\+*\s*.*:\s*\w+`) // e.g., name: Type
	debugPattern := regexp.MustCompile(`^\+*\s*.*(fmt\.Print|console\.log|println|echo)`)
	funcPattern := regexp.MustCompile(`^\+*\s*\b(func|def|function)\s+\w+`)
	commentPattern := regexp.MustCompile(`^[-+]\s*//.*`)
	variableRenamePattern := regexp.MustCompile(`^\+*\s*.*=.*//\s*renamed`)
	formattingPattern := regexp.MustCompile(`^\+*\s*\s*$`) // Lines with only whitespace changes
	reorderPattern := regexp.MustCompile(`^[-+]\s*package\s+|^\+*\s*package\s+`)

	// Calculate line-by-line diff summary using a simple line comparison
	// This is a simplified diff - in production you might use a proper diff library
	minLines := len(originalLines)
	if len(actualLines) < minLines {
		minLines = len(actualLines)
	}

	// Track changes line by line
	contentChanged := false
	for i := 0; i < minLines; i++ {
		if originalLines[i] != actualLines[i] {
			contentChanged = true
			// Count as modification (both add and delete)
			additions++
			deletions++
		}
	}

	// Add extra lines
	if len(actualLines) > minLines {
		additions += len(actualLines) - minLines
		contentChanged = true
	}

	// Removed lines
	if len(originalLines) > minLines {
		deletions += len(originalLines) - minLines
		contentChanged = true
	}

	// If no content changes detected, return exact match
	if !contentChanged && additions == 0 && deletions == 0 {
		return models.ExactReplacementValidationResult{
			ExactMatch:     true,
			Violations:     []models.ExactReplacementViolation{},
			DiffStats:      models.DiffStats{Additions: 0, Deletions: 0},
			Recommendation: "Accept changes - exact match confirmed",
		}
	}

	// Analyze actual content for violations
	// Split into added and removed lines
	addedLines := []string{}
	removedLines := []string{}

	// Simple diff calculation - find adds and removes
	// In production, consider using a proper diff algorithm
	originalSet := make(map[string]bool)
	for _, line := range originalLines {
		if strings.TrimSpace(line) != "" {
			originalSet[line] = true
		}
	}

	actualSet := make(map[string]bool)
	for _, line := range actualLines {
		if strings.TrimSpace(line) != "" {
			actualSet[line] = true
			if !originalSet[line] {
				addedLines = append(addedLines, line)
			}
		}
	}

	for _, line := range originalLines {
		if strings.TrimSpace(line) != "" && !actualSet[line] {
			removedLines = append(removedLines, line)
		}
	}

	// Check for violations in added lines
	var criticalCount, warningCount int

	for _, line := range addedLines {
		// Check for new imports
		if importPattern.MatchString(line) {
			violations = append(violations, models.ExactReplacementViolation{
				Type:     "new_import",
				Severity: "warning",
				Message:  "Additional import added: " + strings.TrimSpace(line),
			})
			warningCount++
		}

		// Check for type hint changes (particularly relevant for Python/JavaScript)
		if typeHintPattern.MatchString(line) && !strings.Contains(line, "//") {
			violations = append(violations, models.ExactReplacementViolation{
				Type:     "type_change",
				Severity: "warning",
				Message:  "Type hint added or changed: " + strings.TrimSpace(line),
			})
			warningCount++
		}

		// Check for debug statements
		if debugPattern.MatchString(line) {
			violations = append(violations, models.ExactReplacementViolation{
				Type:     "debug_added",
				Severity: "error",
				Message:  "Debug statement added: " + strings.TrimSpace(line),
			})
			criticalCount++
		}

		// Check for new functions
		if funcPattern.MatchString(line) {
			violations = append(violations, models.ExactReplacementViolation{
				Type:     "extra_function",
				Severity: "error",
				Message:  "New function added: " + strings.TrimSpace(line),
			})
			criticalCount++
		}

		// Check for variable renames (pattern: old = new // renamed)
		if variableRenamePattern.MatchString(line) {
			violations = append(violations, models.ExactReplacementViolation{
				Type:     "variable_rename",
				Severity: "warning",
				Message:  "Variable renamed: " + strings.TrimSpace(line),
			})
			warningCount++
		}

		// Check for pure formatting changes (only whitespace)
		if formattingPattern.MatchString(line) && len(strings.TrimSpace(line)) == 0 {
			violations = append(violations, models.ExactReplacementViolation{
				Type:     "formatting",
				Severity: "info",
				Message:  "Formatting change (whitespace only)",
			})
		}

		// Check for package reorganization (Go files)
		if reorderPattern.MatchString(line) {
			violations = append(violations, models.ExactReplacementViolation{
				Type:     "code_reorganized",
				Severity: "warning",
				Message:  "Code structure changed (package reordering)",
			})
			warningCount++
		}
	}

	// Check for comment changes
	for _, line := range append(addedLines, removedLines...) {
		if commentPattern.MatchString(line) {
			violations = append(violations, models.ExactReplacementViolation{
				Type:     "comment_change",
				Severity: "info",
				Message:  "Comment modified: " + strings.TrimSpace(line),
			})
		}
	}

	// Check removed lines for significant deletions
	for _, line := range removedLines {
		// Check if a function was removed
		if funcPattern.MatchString(line) {
			violations = append(violations, models.ExactReplacementViolation{
				Type:     "function_removed",
				Severity: "error",
				Message:  "Original function removed: " + strings.TrimSpace(line),
			})
			criticalCount++
		}
	}

	// Determine recommendation based on violations
	recommendation := "Accept changes - exact match confirmed"
	exactMatch := len(violations) == 0

	if !exactMatch {
		if criticalCount > 0 {
			recommendation = "Reject and use exact code - critical violations detected"
		} else if warningCount > 2 {
			recommendation = "Review modifications - multiple warnings"
		} else if warningCount > 0 {
			recommendation = "Proceed with caution - minor changes acceptable"
		} else {
			// Only info-level violations
			recommendation = "Accept changes - improvements only"
			exactMatch = true // Consider info-only as acceptable
		}
	}

	return models.ExactReplacementValidationResult{
		ExactMatch:     exactMatch,
		Violations:     violations,
		DiffStats:      models.DiffStats{Additions: additions, Deletions: deletions},
		Recommendation: recommendation,
	}
}

// handleVerifyTestsBeforeCommit validates that tests pass before committing
func (s *MCPServer) handleVerifyTestsBeforeCommit(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	testResults, _ := args["test_results"].(string)
	stagedFiles, _ := args["staged_files"].([]interface{})
	requireCoverage, _ := args["require_coverage"].(bool)

	if testResults == "" {
		result := models.TestValidationResult{
			Valid:   false,
			Passed:  false,
			Message: "test_results is required",
		}
		return buildToolResult(result, true)
	}

	// Parse test results for common patterns
	testPassed := true
	failedTests := []string{}

	// Check for failure indicators
	failurePatterns := []string{"FAIL", "failed", "error", "Error", "panic", "FAIL:"}
	for _, pattern := range failurePatterns {
		if strings.Contains(testResults, pattern) {
			testPassed = false
			break
		}
	}

	// Check for success indicators
	successPatterns := []string{"PASS", "ok", "SUCCESS", "All tests passed"}
	hasSuccess := false
	for _, pattern := range successPatterns {
		if strings.Contains(testResults, pattern) {
			hasSuccess = true
			break
		}
	}

	// If no success indicators found but no failures, assume passing
	if !testPassed && !hasSuccess {
		testPassed = false
	}

	// Extract failed test names if any
	if !testPassed {
		lines := strings.Split(testResults, "\n")
		for _, line := range lines {
			if strings.Contains(line, "FAIL") || strings.Contains(line, "fail") {
				// Try to extract test name
				parts := strings.Fields(line)
				for i, part := range parts {
					if strings.Contains(part, "FAIL") && i+1 < len(parts) {
						failedTests = append(failedTests, parts[i+1])
						break
					}
				}
			}
		}
	}

	// Check coverage if required (placeholder logic)
	coverageMet := true
	if requireCoverage {
		// Look for coverage percentage in output
		coveragePatterns := []string{"coverage:", "Coverage:"}
		for _, pattern := range coveragePatterns {
			if idx := strings.Index(testResults, pattern); idx != -1 {
				// Extract coverage percentage
				after := testResults[idx+len(pattern):]
				// Simple extraction - look for number followed by %
				for i, ch := range after {
					if ch == '%' {
						// Found percentage, check if it's acceptable (>70%)
						coverageMet = true // Assume acceptable for now
						break
					}
					if i > 10 {
						break
					}
				}
			}
		}
	}

	fileCount := len(stagedFiles)
	message := fmt.Sprintf("Tests %s for %d staged files", map[bool]string{true: "PASSED", false: "FAILED"}[testPassed], fileCount)

	result := models.TestValidationResult{
		Valid:       true,
		Passed:      testPassed,
		Message:     message,
		FailedTests: failedTests,
		CoverageMet: coverageMet,
	}

	return buildToolResult(result, !testPassed)
}

// handleScanCommitPayload scans staged files for secrets, binaries, and generated files
func (s *MCPServer) handleScanCommitPayload(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	stagedFiles, _ := args["staged_files"].([]interface{})
	scanSecrets, _ := args["scan_secrets"].(bool)
	scanBinaries, _ := args["scan_binaries"].(bool)
	scanLargeFiles, _ := args["scan_large_files"].(bool)

	// Default to true if not specified
	if !scanSecrets && !scanBinaries && !scanLargeFiles {
		scanSecrets = true
		scanBinaries = true
		scanLargeFiles = true
	}

	if len(stagedFiles) == 0 {
		result := models.PayloadScanResult{
			Valid:   false,
			Clean:   false,
			Message: "staged_files is required",
		}
		return buildToolResult(result, true)
	}

	findings := []models.PayloadFinding{}
	scanned := 0

	// Secret patterns
	secretPatterns := []struct {
		pattern     *regexp.Regexp
		typeName    string
		severity    string
		description string
	}{
		{regexp.MustCompile(`(?i)(password|passwd|pwd)\s*=\s*["'][^"']+["']`), "secret", "critical", "Password in code"},
		{regexp.MustCompile(`(?i)(api[_-]?key|apikey)\s*=\s*["'][^"']+["']`), "secret", "critical", "API key in code"},
		{regexp.MustCompile(`(?i)(secret[_-]?key|secretkey)\s*=\s*["'][^"']+["']`), "secret", "critical", "Secret key in code"},
		{regexp.MustCompile(`(?i)(auth[_-]?token|authtoken)\s*=\s*["'][^"']+["']`), "secret", "critical", "Auth token in code"},
		{regexp.MustCompile(`sk-[a-zA-Z0-9]{48}`), "secret", "critical", "OpenAI API key"},
		{regexp.MustCompile(`gh[pousr]_[A-Za-z0-9_]{36,}`), "secret", "critical", "GitHub token"},
		{regexp.MustCompile(`[a-zA-Z0-9_-]*[Aa]ws[A-Za-z0-9_-]*\s*=\s*["'][^"']+["']`), "secret", "high", "AWS credential"},
	}

	// Binary file extensions
	binaryExtensions := []string{
		".exe", ".dll", ".so", ".dylib", ".bin",
		".zip", ".tar", ".gz", ".rar", ".7z",
		".jpg", ".jpeg", ".png", ".gif", ".bmp", ".ico", ".svg",
		".mp3", ".mp4", ".avi", ".mov", ".wav",
		".pdf", ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx",
		".db", ".sqlite", ".sqlite3",
	}

	// Generated file patterns
	generatedPatterns := []string{
		".min.js", ".min.css", ".bundle.js", ".bundle.css",
		"generated.", "_generated.", "__generated__",
		"node_modules/", "vendor/", "dist/", "build/", "target/",
	}

	for _, file := range stagedFiles {
		filePath, ok := file.(string)
		if !ok {
			continue
		}

		scanned++
		fileLower := strings.ToLower(filePath)

		// Check for binaries
		if scanBinaries {
			for _, ext := range binaryExtensions {
				if strings.HasSuffix(fileLower, ext) {
					findings = append(findings, models.PayloadFinding{
						File:        filePath,
						Type:        "binary",
						Severity:    "error",
						Description: "Binary file should not be committed",
					})
					break
				}
			}
		}

		// Check for generated files
		for _, pattern := range generatedPatterns {
			if strings.Contains(fileLower, pattern) {
				findings = append(findings, models.PayloadFinding{
					File:        filePath,
					Type:        "generated",
					Severity:    "warning",
					Description: "Generated file should be in .gitignore",
				})
				break
			}
		}

		// Check file size (if we can stat the file)
		if scanLargeFiles {
			if info, err := os.Stat(filePath); err == nil {
				if info.Size() > 1024*1024 { // > 1MB
					findings = append(findings, models.PayloadFinding{
						File:        filePath,
						Type:        "large",
						Severity:    "warning",
						Description: fmt.Sprintf("Large file (%.2f MB)", float64(info.Size())/(1024*1024)),
					})
				}
			}
		}

		// Scan for secrets in file content
		if scanSecrets {
			content, err := os.ReadFile(filePath)
			if err == nil {
				contentStr := string(content)
				lines := strings.Split(contentStr, "\n")

				for lineNum, line := range lines {
					for _, pattern := range secretPatterns {
						if pattern.pattern.MatchString(line) {
							findings = append(findings, models.PayloadFinding{
								File:        filePath,
								Type:        pattern.typeName,
								Severity:    pattern.severity,
								Description: pattern.description,
								LineNumber:  lineNum + 1,
							})
						}
					}
				}
			}
		}
	}

	clean := len(findings) == 0
	message := fmt.Sprintf("Scanned %d files", scanned)
	if !clean {
		message = fmt.Sprintf("Found %d issues in %d files", len(findings), scanned)
	}

	result := models.PayloadScanResult{
		Valid:    true,
		Clean:    clean,
		Message:  message,
		Findings: findings,
		Scanned:  scanned,
	}

	return buildToolResult(result, !clean)
}

// handleDetectMergeConflicts detects merge conflict markers in files
func (s *MCPServer) handleDetectMergeConflicts(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	filePaths, _ := args["file_paths"].([]interface{})
	checkContent, _ := args["check_content"].(bool)

	if !checkContent {
		checkContent = true // Default to true
	}

	if len(filePaths) == 0 {
		result := models.MergeConflictResult{
			Valid:   false,
			Clean:   false,
			Message: "file_paths is required",
		}
		return buildToolResult(result, true)
	}

	conflicts := []models.ConflictFinding{}
	checked := 0

	// Conflict markers
	markerStart := regexp.MustCompile(`^<{7}`)  // <<<<<<<
	markerSep := regexp.MustCompile(`^={7}`)    // =======
	markerEnd := regexp.MustCompile(`^>{7}`)    // >>>>>>>

	for _, file := range filePaths {
		filePath, ok := file.(string)
		if !ok {
			continue
		}

		checked++

		if !checkContent {
			continue
		}

		// Read file content
		content, err := os.ReadFile(filePath)
		if err != nil {
			continue // Skip unreadable files
		}

		lines := strings.Split(string(content), "\n")
		inConflict := false
		conflictStartLine := 0

		for lineNum, line := range lines {
			if markerStart.MatchString(line) {
				inConflict = true
				conflictStartLine = lineNum + 1
			} else if markerSep.MatchString(line) && inConflict {
				// Middle of conflict
				continue
			} else if markerEnd.MatchString(line) && inConflict {
				// End of conflict
				conflicts = append(conflicts, models.ConflictFinding{
					File:       filePath,
					LineNumber: conflictStartLine,
					Context:    fmt.Sprintf("Conflict from line %d to %d", conflictStartLine, lineNum+1),
				})
				inConflict = false
			}
		}

		// If still in conflict at end of file
		if inConflict {
			conflicts = append(conflicts, models.ConflictFinding{
				File:       filePath,
				LineNumber: conflictStartLine,
				Context:    fmt.Sprintf("Unterminated conflict starting at line %d", conflictStartLine),
			})
		}
	}

	clean := len(conflicts) == 0
	message := fmt.Sprintf("Checked %d files", checked)
	if !clean {
		message = fmt.Sprintf("Found %d merge conflicts in %d files", len(conflicts), checked)
	}

	result := models.MergeConflictResult{
		Valid:     true,
		Clean:     clean,
		Message:   message,
		Conflicts: conflicts,
		Checked:   checked,
	}

	return buildToolResult(result, !clean)
}
