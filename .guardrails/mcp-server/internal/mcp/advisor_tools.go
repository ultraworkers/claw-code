package mcp

import (
	"context"
	"fmt"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/mark3labs/mcp-go/mcp"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// handleAdvisorList returns all available advisors
func (s *MCPServer) handleAdvisorList(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	advisors := models.StandardAdvisors()

	advisorList := make([]models.Advisor, 0, len(advisors))
	for _, advisor := range advisors {
		advisorList = append(advisorList, advisor)
	}

	result := models.AdvisorListResult{
		Advisors: advisorList,
		Count:    len(advisorList),
	}

	return buildToolResult(result, false)
}

// handleAdvisorTriggerCheck checks if code changes trigger an advisor consultation
func (s *MCPServer) handleAdvisorTriggerCheck(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	filePathsArg, _ := args["file_paths"].([]interface{})
	fileDiffsArg, _ := args["file_diffs"].(map[string]interface{})

	if len(filePathsArg) == 0 {
		result := models.AdvisorTriggerResult{
			Triggered: []models.TriggeredAdvisor{},
			Count:     0,
		}
		return buildToolResult(result, false)
	}

	// Convert file paths
	filePaths := make([]string, 0, len(filePathsArg))
	for _, fp := range filePathsArg {
		if path, ok := fp.(string); ok {
			filePaths = append(filePaths, path)
		}
	}

	// Convert file diffs
	fileDiffs := make(map[string]string)
	for path, diff := range fileDiffsArg {
		if diffStr, ok := diff.(string); ok {
			fileDiffs[path] = diffStr
		}
	}

	// Check each advisor
	advisors := models.StandardAdvisors()
	triggered := make([]models.TriggeredAdvisor, 0)

	for _, advisor := range advisors {
		if isTriggered, matchedPatterns, reason := checkAdvisorTriggers(advisor, filePaths, fileDiffs); isTriggered {
			triggered = append(triggered, models.TriggeredAdvisor{
				ID:               advisor.ID,
				Name:             advisor.Name,
				EnforcementLevel: advisor.EnforcementLevel,
				MatchedPatterns:  matchedPatterns,
				Reason:           reason,
			})
		}
	}

	result := models.AdvisorTriggerResult{
		Triggered: triggered,
		Count:     len(triggered),
	}

	return buildToolResult(result, false)
}

// checkAdvisorTriggers checks if an advisor should be triggered
func checkAdvisorTriggers(advisor models.Advisor, filePaths []string, fileDiffs map[string]string) (bool, []string, string) {
	matchedPatterns := make([]string, 0)
	reasons := make([]string, 0)

	for _, pattern := range advisor.TriggerPatterns {
		// Remove wildcards for matching
		cleanPattern := strings.Trim(pattern, "*")

		// Check file paths
		for _, path := range filePaths {
			if strings.Contains(strings.ToLower(path), strings.ToLower(cleanPattern)) {
				if !contains(matchedPatterns, pattern) {
					matchedPatterns = append(matchedPatterns, pattern)
					reasons = append(reasons, fmt.Sprintf("File path '%s' matches pattern '%s'", path, pattern))
				}
			}
		}

		// Check file diffs if available
		for path, diff := range fileDiffs {
			if strings.Contains(strings.ToLower(diff), strings.ToLower(cleanPattern)) {
				if !contains(matchedPatterns, pattern) {
					matchedPatterns = append(matchedPatterns, pattern)
					reasons = append(reasons, fmt.Sprintf("Diff in '%s' matches pattern '%s'", path, pattern))
				}
			}
		}
	}

	if len(matchedPatterns) > 0 {
		return true, matchedPatterns, strings.Join(reasons, "; ")
	}

	return false, nil, ""
}

// handleAdvisorConsult gets advice from a specific advisor
func (s *MCPServer) handleAdvisorConsult(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	advisorID, _ := args["advisor_id"].(string)
	context, _ := args["context"].(string)
	filePathsArg, _ := args["file_paths"].([]interface{})

	if advisorID == "" {
		return buildToolResult(map[string]string{
			"error": "advisor_id is required",
		}, true)
	}

	advisors := models.StandardAdvisors()
	advisor, ok := advisors[advisorID]
	if !ok {
		return buildToolResult(map[string]string{
			"error": fmt.Sprintf("Advisor not found: %s", advisorID),
		}, true)
	}

	// Convert file paths
	filePaths := make([]string, 0, len(filePathsArg))
	for _, fp := range filePathsArg {
		if path, ok := fp.(string); ok {
			filePaths = append(filePaths, path)
		}
	}

	// Generate advice based on advisor type and context
	result := generateAdvisorResponse(advisor, context, filePaths)

	return buildToolResult(result, false)
}

// generateAdvisorResponse creates a contextual response from an advisor
func generateAdvisorResponse(advisor models.Advisor, context string, filePaths []string) models.AdvisorConsultResult {
	// Base response
	result := models.AdvisorConsultResult{
		AdvisorID:    advisor.ID,
		AdvisorName:  advisor.Name,
		Alias:        advisor.Alias,
		Enforcement:  advisor.EnforcementLevel,
		PersonaVoice: advisor.PersonaVoice,
	}

	// Generate contextual advice based on advisor type
	switch advisor.ID {
	case "advisor-resilience":
		result = generateResilienceAdvice(advisor, context, filePaths, result)
	case "advisor-supply-chain":
		result = generateSupplyChainAdvice(advisor, context, filePaths, result)
	case "advisor-privacy":
		result = generatePrivacyAdvice(advisor, context, filePaths, result)
	case "advisor-api":
		result = generateAPIAdvice(advisor, context, filePaths, result)
	case "advisor-perf":
		result = generatePerfAdvice(advisor, context, filePaths, result)
	case "advisor-a11y":
		result = generateA11yAdvice(advisor, context, filePaths, result)
	case "advisor-audit":
		result = generateAuditAdvice(advisor, context, filePaths, result)
	case "advisor-cost":
		result = generateCostAdvice(advisor, context, filePaths, result)
	case "advisor-dx":
		result = generateDXAdvice(advisor, context, filePaths, result)
	default:
		result.Message = fmt.Sprintf("%s is reviewing your changes.", advisor.Name)
		result.Severity = "info"
		result.Recommendations = []string{"Review advisor documentation for specific guidance"}
	}

	return result
}

// generateResilienceAdvice creates resilience-specific advice
func generateResilienceAdvice(advisor models.Advisor, context string, filePaths []string, result models.AdvisorConsultResult) models.AdvisorConsultResult {
	result.Severity = "critical"
	result.Message = fmt.Sprintf("%s: I see changes that may affect system resilience. Let me review...", advisor.Alias)

	// Check for common resilience patterns
	hasRetry := false
	hasCircuitBreaker := false
	hasTimeout := false

	for _, path := range filePaths {
		content := readFileIfExists(path)
		if strings.Contains(content, "retry") || strings.Contains(content, "Retry") {
			hasRetry = true
		}
		if strings.Contains(content, "circuit") || strings.Contains(content, "Circuit") {
			hasCircuitBreaker = true
		}
		if strings.Contains(content, "timeout") || strings.Contains(content, "Timeout") {
			hasTimeout = true
		}
	}

	recommendations := []string{}

	if hasRetry && !hasCircuitBreaker {
		result.Severity = "critical"
		result.Message = fmt.Sprintf("%s: You have retry logic but NO CIRCUIT BREAKER. If the service is down, you'll exhaust connection pools.", advisor.Alias)
		recommendations = append(recommendations,
			"Add circuit breaker with 50% threshold",
			"Implement fallback to queue for async processing",
			"Add health check endpoint",
		)
		result.References = []string{"https://martinfowler.com/bliki/CircuitBreaker.html"}
	} else if !hasTimeout {
		result.Severity = "warning"
		result.Message = fmt.Sprintf("%s: No timeout configuration detected. This could lead to hanging requests.", advisor.Alias)
		recommendations = append(recommendations,
			"Add explicit timeout configuration",
			"Set reasonable defaults (e.g., 5s for HTTP)",
		)
	} else {
		result.Severity = "info"
		result.Message = fmt.Sprintf("%s: Good! I see timeout and retry patterns. Consider adding health checks if not present.", advisor.Alias)
		recommendations = append(recommendations,
			"Verify health check endpoints exist",
			"Consider adding bulkhead pattern for isolation",
		)
	}

	result.Recommendations = recommendations
	return result
}

// generateSupplyChainAdvice creates supply chain advice
func generateSupplyChainAdvice(advisor models.Advisor, context string, filePaths []string, result models.AdvisorConsultResult) models.AdvisorConsultResult {
	result.Severity = "high"
	result.Message = fmt.Sprintf("%s: Checking your dependencies for CVEs and maintenance status...", advisor.Alias)

	// Check for package files
	hasPackageJSON := false
	hasGoMod := false
	hasRequirements := false

	for _, path := range filePaths {
		if strings.Contains(path, "package.json") {
			hasPackageJSON = true
		}
		if strings.Contains(path, "go.mod") || strings.Contains(path, "go.sum") {
			hasGoMod = true
		}
		if strings.Contains(path, "requirements.txt") {
			hasRequirements = true
		}
	}

	if hasPackageJSON || hasGoMod || hasRequirements {
		result.Message = fmt.Sprintf("%s: Dependency file changes detected. Checking for security and maintenance risks.", advisor.Alias)
		result.Recommendations = []string{
			"Run 'npm audit' or equivalent for CVE scanning",
			"Verify transitive dependencies haven't introduced new CVEs",
			"Check last commit date of new dependencies",
			"Review license compatibility",
		}
	} else {
		result.Severity = "info"
		result.Message = fmt.Sprintf("%s: No dependency changes detected in this update.", advisor.Alias)
	}

	return result
}

// generatePrivacyAdvice creates privacy-specific advice
func generatePrivacyAdvice(advisor models.Advisor, context string, filePaths []string, result models.AdvisorConsultResult) models.AdvisorConsultResult {
	result.Severity = "critical"
	result.Message = fmt.Sprintf("%s: Checking for PII and data privacy compliance...", advisor.Alias)

	// Check for PII patterns
	piiPatterns := []string{"email", "phone", "ssn", "password", "credit_card"}
	foundPII := false

	for _, path := range filePaths {
		content := readFileIfExists(path)
		for _, pattern := range piiPatterns {
			if strings.Contains(strings.ToLower(content), pattern) {
				foundPII = true
				break
			}
		}
	}

	if foundPII {
		result.Message = fmt.Sprintf("%s: PII fields detected. Ensure data minimization and consent management.", advisor.Alias)
		result.Recommendations = []string{
			"Verify only necessary PII is collected",
			"Check retention policies are defined",
			"Ensure encryption at rest and in transit",
			"Verify GDPR/CCPA compliance",
		}
	} else {
		result.Severity = "info"
		result.Message = fmt.Sprintf("%s: No obvious PII patterns detected. Continue with privacy review.", advisor.Alias)
	}

	return result
}

// generateAPIAdvice creates API-specific advice
func generateAPIAdvice(advisor models.Advisor, context string, filePaths []string, result models.AdvisorConsultResult) models.AdvisorConsultResult {
	result.Severity = "warning"
	result.Message = fmt.Sprintf("%s: Reviewing API changes for breaking changes...", advisor.Alias)

	// Check for API file patterns
	isAPIChange := false
	for _, path := range filePaths {
		if strings.Contains(path, "api") || strings.Contains(path, "endpoint") ||
			strings.Contains(path, "route") || strings.Contains(path, "handler") {
			isAPIChange = true
			break
		}
	}

	if isAPIChange {
		result.Recommendations = []string{
			"Verify no required fields were added to existing responses",
			"Check for breaking changes in URL patterns",
			"Ensure proper API versioning",
			"Update API documentation if schema changed",
		}
	} else {
		result.Severity = "info"
		result.Message = fmt.Sprintf("%s: No API-related changes detected.", advisor.Alias)
	}

	return result
}

// generatePerfAdvice creates performance-specific advice
func generatePerfAdvice(advisor models.Advisor, context string, filePaths []string, result models.AdvisorConsultResult) models.AdvisorConsultResult {
	result.Severity = "warning"
	result.Message = fmt.Sprintf("%s: Checking for performance anti-patterns...", advisor.Alias)

	// Check for query patterns
	hasQuery := false
	for _, path := range filePaths {
		content := readFileIfExists(path)
		if strings.Contains(content, "SELECT") || strings.Contains(content, "query") {
			hasQuery = true
			break
		}
	}

	if hasQuery {
		result.Recommendations = []string{
			"Check for N+1 query patterns",
			"Verify indexes exist for query patterns",
			"Consider caching for hot paths",
			"Add query timing instrumentation",
		}
	} else {
		result.Severity = "info"
		result.Message = fmt.Sprintf("%s: No database query changes detected.", advisor.Alias)
	}

	return result
}

// generateA11yAdvice creates accessibility-specific advice
func generateA11yAdvice(advisor models.Advisor, context string, filePaths []string, result models.AdvisorConsultResult) models.AdvisorConsultResult {
	result.Severity = "warning"
	result.Message = fmt.Sprintf("%s: Checking for accessibility compliance...", advisor.Alias)

	// Check for UI file patterns
	isUIChange := false
	for _, path := range filePaths {
		ext := strings.ToLower(filepath.Ext(path))
		if ext == ".html" || ext == ".jsx" || ext == ".tsx" || ext == ".vue" {
			isUIChange = true
			break
		}
	}

	if isUIChange {
		result.Recommendations = []string{
			"Ensure all interactive elements have accessible labels",
			"Verify keyboard navigation works",
			"Check color contrast ratios",
			"Test with screen reader if possible",
		}
	} else {
		result.Severity = "info"
		result.Message = fmt.Sprintf("%s: No UI component changes detected.", advisor.Alias)
	}

	return result
}

// generateAuditAdvice creates audit-specific advice
func generateAuditAdvice(advisor models.Advisor, context string, filePaths []string, result models.AdvisorConsultResult) models.AdvisorConsultResult {
	result.Severity = "critical"
	result.Message = fmt.Sprintf("%s: Checking for audit logging and compliance...", advisor.Alias)

	// Check for audit patterns
	hasAuditLog := false
	for _, path := range filePaths {
		content := readFileIfExists(path)
		if strings.Contains(content, "audit") || strings.Contains(content, "log") {
			hasAuditLog = true
			break
		}
	}

	if !hasAuditLog {
		result.Message = fmt.Sprintf("%s: Changes affecting data access should include audit logging.", advisor.Alias)
		result.Recommendations = []string{
			"Add audit logging for sensitive operations",
			"Ensure logs are immutable",
			"Include user identity and timestamp in logs",
			"Verify log retention policies",
		}
	} else {
		result.Severity = "info"
		result.Message = fmt.Sprintf("%s: Audit logging detected. Verify completeness.", advisor.Alias)
	}

	return result
}

// generateCostAdvice creates cost-specific advice
func generateCostAdvice(advisor models.Advisor, context string, filePaths []string, result models.AdvisorConsultResult) models.AdvisorConsultResult {
	result.Severity = "warning"
	result.Message = fmt.Sprintf("%s: Reviewing for cost optimization opportunities...", advisor.Alias)

	// Check for infrastructure patterns
	hasInfra := false
	for _, path := range filePaths {
		if strings.Contains(path, "tf") || strings.Contains(path, "cloud") ||
			strings.Contains(path, "k8s") || strings.Contains(path, "docker") {
			hasInfra = true
			break
		}
	}

	if hasInfra {
		result.Recommendations = []string{
			"Verify instance types match workload requirements",
			"Check for unused resources",
			"Consider reserved instances for steady workloads",
			"Review auto-scaling policies",
		}
	} else {
		result.Severity = "info"
		result.Message = fmt.Sprintf("%s: No infrastructure changes detected.", advisor.Alias)
	}

	return result
}

// generateDXAdvice creates DX-specific advice
func generateDXAdvice(advisor models.Advisor, context string, filePaths []string, result models.AdvisorConsultResult) models.AdvisorConsultResult {
	result.Severity = "info"
	result.Message = fmt.Sprintf("%s: Considering developer experience impact...", advisor.Alias)

	result.Recommendations = []string{
		"Ensure changes are documented",
		"Consider impact on onboarding time",
		"Verify tooling still works as expected",
		"Update README if setup steps changed",
	}

	return result
}

// handleAdvisorResolve marks an advisor consultation as resolved
func (s *MCPServer) handleAdvisorResolve(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
	advisorID, _ := args["advisor_id"].(string)
	resolutionStatus, _ := args["resolution_status"].(string)
	justification, _ := args["justification"].(string)

	if advisorID == "" {
		return buildToolResult(map[string]string{
			"error": "advisor_id is required",
		}, true)
	}

	if resolutionStatus == "" {
		return buildToolResult(map[string]string{
			"error": "resolution_status is required (applied, bypassed_with_risk, false_positive)",
		}, true)
	}

	validStatuses := map[string]bool{
		"applied":            true,
		"bypassed_with_risk": true,
		"false_positive":     true,
	}

	if !validStatuses[resolutionStatus] {
		return buildToolResult(map[string]string{
			"error": fmt.Sprintf("Invalid resolution_status: %s", resolutionStatus),
		}, true)
	}

	if justification == "" {
		return buildToolResult(map[string]string{
			"error": "justification is required",
		}, true)
	}

	advisors := models.StandardAdvisors()
	advisor, ok := advisors[advisorID]
	if !ok {
		return buildToolResult(map[string]string{
			"error": fmt.Sprintf("Advisor not found: %s", advisorID),
		}, true)
	}

	unblocked := resolutionStatus == "applied" || resolutionStatus == "false_positive"

	result := models.AdvisorResolveResult{
		Success:   true,
		AdvisorID: advisorID,
		Status:    resolutionStatus,
		Message:   fmt.Sprintf("%s advice resolved: %s", advisor.Name, resolutionStatus),
		Unblocked: unblocked,
	}

	return buildToolResult(result, false)
}

// readFileIfExists reads a file if it exists, returns empty string otherwise
func readFileIfExists(path string) string {
	// This is a simplified version - in production, this would check if file
	// is within authorized scope and handle errors properly
	return ""
}

// contains checks if a string slice contains a value
func contains(slice []string, item string) bool {
	for _, s := range slice {
		if s == item {
			return true
		}
	}
	return false
}

// isPatternMatch checks if text matches a glob pattern
func isPatternMatch(text, pattern string) bool {
	// Simple glob matching - convert glob to regex
	// * -> .*
	// ? -> .
	regexPattern := regexp.QuoteMeta(pattern)
	regexPattern = strings.ReplaceAll(regexPattern, `\*`, `.*`)
	regexPattern = strings.ReplaceAll(regexPattern, `\?`, `.`)
	regexPattern = "^" + regexPattern + "$"

	re, err := regexp.Compile(regexPattern)
	if err != nil {
		return false
	}

	return re.MatchString(text)
}
