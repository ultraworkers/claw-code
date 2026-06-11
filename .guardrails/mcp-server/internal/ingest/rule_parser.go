package ingest

import (
	"context"
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"log/slog"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/database"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
	"github.com/thearchitectit/guardrail-mcp/internal/validation"
)

// RuleParser parses markdown files containing prevention rules
type RuleParser struct {
	// ruleHeaderRegex matches rule headers like "## PREVENT-001: Rule Name"
	ruleHeaderRegex *regexp.Regexp
	// metadataRegex matches metadata fields like "**Pattern:** `regex`"
	metadataRegex *regexp.Regexp
	// backtickRegex extracts content from backticks
	backtickRegex *regexp.Regexp
}

// NewRuleParser creates a new rule parser
func NewRuleParser() *RuleParser {
	return &RuleParser{
		ruleHeaderRegex: regexp.MustCompile(`(?m)^##\s+(PREVENT-\d+)\s*:\s*(.+)$`),
		metadataRegex:   regexp.MustCompile(`(?m)^\*\*(\w+):\*\*\s*(.+?)$`),
		backtickRegex:   regexp.MustCompile("`([^`]+)`"),
	}
}

// ParsedRule represents a rule extracted from markdown
type ParsedRule struct {
	RuleID      string
	Name        string
	Pattern     string
	Message     string
	Severity    string
	Category    string
	PatternHash string
}

// ParseRuleFile parses a single markdown file and extracts rules
func (p *RuleParser) ParseRuleFile(path string) ([]ParsedRule, error) {
	slog.Info("Parsing rule file", "file", path)

	content, err := os.ReadFile(path)
	if err != nil {
		slog.Error("Failed to read rule file", "file", path, "error", err)
		slog.Error("Failed to read rule file", "file", path, "error", err)
		return nil, fmt.Errorf("failed to read file %s: %w", path, err)
	}

	rules, err := p.ParseRuleContent(string(content), path)
	if err != nil {
		slog.Error("Failed to parse rule content", "file", path, "error", err)
		return nil, err
	}

	slog.Info("Successfully parsed rule file", "file", path, "rules_found", len(rules))
	return rules, nil
}

// ParseRuleContent parses markdown content and extracts rules
func (p *RuleParser) ParseRuleContent(content, source string) ([]ParsedRule, error) {
	slog.Debug("Parsing rule content", "source", source, "content_length", len(content))

	var rules []ParsedRule

	// Find all rule sections
	matches := p.ruleHeaderRegex.FindAllStringIndex(content, -1)
	if matches == nil {
		slog.Debug("No rule sections found in content", "source", source)
		return rules, nil
	}

	slog.Debug("Found rule sections in content", "source", source, "section_count", len(matches))

	for i, match := range matches {
		start := match[0]
		end := len(content)
		if i < len(matches)-1 {
			end = matches[i+1][0]
		}

		section := content[start:end]
		rule, err := p.parseRuleSection(section)
		if err != nil {
			return nil, fmt.Errorf("failed to parse rule in %s: %w", source, err)
		}

		if rule != nil {
			// Compute pattern hash for change detection
			hash := sha256.Sum256([]byte(section))
			rule.PatternHash = fmt.Sprintf("%x", hash[:8])
			rules = append(rules, *rule)
		}
	}

	slog.Debug("Completed parsing rule content", "source", source, "rules_extracted", len(rules))

	return rules, nil
}

// parseRuleSection parses a single rule section
func (p *RuleParser) parseRuleSection(section string) (*ParsedRule, error) {
	// Extract rule ID and name from header
	headerMatch := p.ruleHeaderRegex.FindStringSubmatch(section)
	if headerMatch == nil {
		return nil, nil
	}

	rule := &ParsedRule{
		RuleID: headerMatch[1],
		Name:   strings.TrimSpace(headerMatch[2]),
	}

	// Extract metadata fields
	metadata := p.extractMetadata(section)

	// Map metadata to rule fields
	if pattern, ok := metadata["Pattern"]; ok {
		rule.Pattern = p.extractBacktickContent(pattern)
	}
	if message, ok := metadata["Message"]; ok {
		rule.Message = strings.TrimSpace(message)
	}
	if severity, ok := metadata["Severity"]; ok {
		rule.Severity = strings.ToLower(strings.TrimSpace(severity))
	}
	if category, ok := metadata["Category"]; ok {
		rule.Category = strings.ToLower(strings.TrimSpace(category))
	}

	// Extract description (content after metadata, before next section or end)
	// Note: Description is not stored in the database model but can be used for documentation
	_ = p.extractDescription(section)

	// Set default message if not provided
	if rule.Message == "" {
		rule.Message = fmt.Sprintf("Rule violation: %s", rule.Name)
	}

	// Validate the parsed rule
	if err := p.validateRule(rule); err != nil {
		return nil, err
	}

	return rule, nil
}

// extractMetadata extracts all **Key:** Value pairs from content
func (p *RuleParser) extractMetadata(content string) map[string]string {
	metadata := make(map[string]string)
	matches := p.metadataRegex.FindAllStringSubmatch(content, -1)

	for _, match := range matches {
		if len(match) >= 3 {
			key := strings.TrimSpace(match[1])
			value := strings.TrimSpace(match[2])
			metadata[key] = value
		}
	}

	return metadata
}

// extractBacktickContent extracts content from backticks
func (p *RuleParser) extractBacktickContent(content string) string {
	match := p.backtickRegex.FindStringSubmatch(content)
	if len(match) >= 2 {
		return match[1]
	}
	return strings.TrimSpace(content)
}

// extractDescription extracts the description text from a rule section
func (p *RuleParser) extractDescription(section string) string {
	// Split by lines and find description after metadata
	lines := strings.Split(section, "\n")
	var descLines []string
	inDescription := false

	for _, line := range lines {
		trimmed := strings.TrimSpace(line)

		// Skip header line
		if strings.HasPrefix(trimmed, "## ") {
			continue
		}

		// Skip metadata lines
		if strings.HasPrefix(trimmed, "**") && strings.Contains(trimmed, "**:") {
			continue
		}

		// Skip empty lines at start
		if !inDescription && trimmed == "" {
			continue
		}

		inDescription = true

		// Stop at horizontal rules
		if strings.HasPrefix(trimmed, "---") {
			break
		}

		descLines = append(descLines, line)
	}

	// Clean up the description
	description := strings.Join(descLines, "\n")
	description = strings.TrimSpace(description)

	// Remove markdown formatting for plain text description
	description = regexp.MustCompile(`\*\*([^*]+)\*\*`).ReplaceAllString(description, "$1")
	description = regexp.MustCompile("`([^`]+)`").ReplaceAllString(description, "$1")

	return description
}

// validateRule validates a parsed rule
func (p *RuleParser) validateRule(rule *ParsedRule) error {
	if rule.RuleID == "" {
		return fmt.Errorf("rule ID is required")
	}

	if rule.Name == "" {
		return fmt.Errorf("rule name is required")
	}

	if rule.Pattern == "" {
		return fmt.Errorf("pattern is required for rule %s", rule.RuleID)
	}

	// Validate regex pattern
	if err := validation.ValidatePattern(rule.Pattern); err != nil {
		return fmt.Errorf("invalid pattern for rule %s: %w", rule.RuleID, err)
	}

	// Validate severity
	validSeverities := map[string]bool{"error": true, "warning": true, "info": true}
	if !validSeverities[rule.Severity] {
		rule.Severity = "warning" // Default to warning
	}

	// Validate category
	validCategories := map[string]bool{"git": true, "bash": true, "docker": true, "security": true, "general": true}
	if !validCategories[rule.Category] {
		rule.Category = "general" // Default to general
	}

	return nil
}

// RuleSyncResult tracks the results of a rule sync operation
type RuleSyncResult struct {
	Added     int
	Updated   int
	Disabled  int
	Errors    []string
}

// JSONRuleFile represents the structure of JSON rule files
type JSONRuleFile struct {
	Schema      string     `json:"$schema"`
	Description string     `json:"description"`
	Version     string     `json:"version"`
	Rules       []JSONRule `json:"rules"`
}

// JSONRule represents a single rule in JSON format
type JSONRule struct {
	RuleID           string   `json:"rule_id"`
	FailureID        *string  `json:"failure_id"`
	Name             string   `json:"name"`
	Enabled          bool     `json:"enabled"`
	Pattern          string   `json:"pattern"`
	ForbiddenContext *string  `json:"forbidden_context"`
	Message          string   `json:"message"`
	Severity         string   `json:"severity"`
	FileGlob         []string `json:"file_glob"`
	Suggestion       string   `json:"suggestion"`
	Category         string   `json:"category"`
}

// ParseJSONRuleFile parses a JSON rule file and extracts rules
func (p *RuleParser) ParseJSONRuleFile(path string) ([]ParsedRule, error) {
	slog.Info("Parsing JSON rule file", "file", path)

	content, err := os.ReadFile(path)
	if err != nil {
		slog.Error("Failed to read JSON rule file", "file", path, "error", err)
		return nil, fmt.Errorf("failed to read file %s: %w", path, err)
	}

	var jsonFile JSONRuleFile
	if err := json.Unmarshal(content, &jsonFile); err != nil {
		slog.Error("Failed to unmarshal JSON rule file", "file", path, "error", err)
		return nil, fmt.Errorf("failed to parse JSON file %s: %w", path, err)
	}

	var rules []ParsedRule
	for _, jsonRule := range jsonFile.Rules {
		// Skip disabled rules
		if !jsonRule.Enabled {
			slog.Debug("Skipping disabled rule", "rule_id", jsonRule.RuleID)
			continue
		}

		rule := ParsedRule{
			RuleID:   jsonRule.RuleID,
			Name:     jsonRule.Name,
			Pattern:  jsonRule.Pattern,
			Message:  jsonRule.Message,
			Severity: jsonRule.Severity,
			Category: jsonRule.Category,
		}

		// Default category if not set
		if rule.Category == "" {
			rule.Category = "general"
		}

		// Compute hash
		hash := sha256.Sum256([]byte(jsonRule.Pattern + jsonRule.Message))
		rule.PatternHash = fmt.Sprintf("%x", hash[:8])

		// Validate
		if err := p.validateRule(&rule); err != nil {
			slog.Error("Invalid JSON rule", "rule_id", jsonRule.RuleID, "error", err)
			continue
		}

		rules = append(rules, rule)
	}

	slog.Info("Successfully parsed JSON rule file", "file", path, "rules_found", len(rules))
	return rules, nil
}

// RuleSyncService handles syncing parsed rules to the database
type RuleSyncService struct {
	ruleStore *database.RuleStore
	parser    *RuleParser
}

// NewRuleSyncService creates a new rule sync service
func NewRuleSyncService(ruleStore *database.RuleStore) *RuleSyncService {
	return &RuleSyncService{
		ruleStore: ruleStore,
		parser:    NewRuleParser(),
	}
}

// SyncRulesFromDirectory syncs all rules from markdown and JSON files in a directory
func (s *RuleSyncService) SyncRulesFromDirectory(ctx context.Context, dir string) (*RuleSyncResult, error) {
	slog.Info("Syncing rules from directory", "dir", dir)
	result := &RuleSyncResult{}
	fileCount := 0
	processedRuleIDs := make(map[string]bool)

	err := filepath.Walk(dir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		if info.IsDir() {
			return nil
		}

		var rules []ParsedRule
		var parseErr error

		// Handle markdown files
		if IsMarkdownFile(path) {
			fileCount++
			slog.Debug("Processing markdown file", "file", path)
			rules, parseErr = s.parser.ParseRuleFile(path)
		} else if strings.HasSuffix(strings.ToLower(path), ".json") {
			// Handle JSON rule files
			fileCount++
			slog.Debug("Processing JSON rule file", "file", path)
			rules, parseErr = s.parser.ParseJSONRuleFile(path)
		} else {
			return nil // Skip other files
		}

		if parseErr != nil {
			result.Errors = append(result.Errors, fmt.Sprintf("failed to parse %s: %v", path, parseErr))
			return nil
		}

		for _, parsedRule := range rules {
			processedRuleIDs[parsedRule.RuleID] = true

			if err := s.syncRule(ctx, parsedRule, result); err != nil {
				result.Errors = append(result.Errors, fmt.Sprintf("failed to sync %s: %v", parsedRule.RuleID, err))
			}
		}

		return nil
	})

	if err != nil {
		return result, fmt.Errorf("failed to walk directory: %w", err)
	}

	// Disable rules that no longer exist in markdown files
	if err := s.disableOrphanedRules(ctx, processedRuleIDs, result); err != nil {
		result.Errors = append(result.Errors, fmt.Sprintf("failed to disable orphaned rules: %v", err))
	}

	return result, nil
}

// SyncRulesFromContent syncs rules from markdown content (for uploaded files)
func (s *RuleSyncService) SyncRulesFromContent(ctx context.Context, content, filename string) (*RuleSyncResult, error) {
	result := &RuleSyncResult{}

	rules, err := s.parser.ParseRuleContent(content, filename)
	if err != nil {
		return result, fmt.Errorf("failed to parse content: %w", err)
	}

	for _, parsedRule := range rules {
		if err := s.syncRule(ctx, parsedRule, result); err != nil {
			result.Errors = append(result.Errors, fmt.Sprintf("failed to sync %s: %v", parsedRule.RuleID, err))
		}
	}

	return result, nil
}

// syncRule syncs a single rule to the database
func (s *RuleSyncService) syncRule(ctx context.Context, parsed ParsedRule, result *RuleSyncResult) error {
	// Check if rule already exists
	existing, err := s.ruleStore.GetByRuleID(ctx, parsed.RuleID)
	if err != nil {
		// Check if it's a "not found" error
		if !strings.Contains(err.Error(), "not found") {
			return fmt.Errorf("failed to check existing rule: %w", err)
		}
		existing = nil
	}

	if existing != nil {
		// Check if content changed
		if existing.PatternHash != nil && *existing.PatternHash == parsed.PatternHash {
			// Rule unchanged, just ensure it's enabled
			if !existing.Enabled {
				existing.Enabled = true
				if err := s.ruleStore.Update(ctx, existing); err != nil {
					return fmt.Errorf("failed to re-enable rule: %w", err)
				}
				result.Updated++
			}
			return nil
		}

		// Update existing rule
		existing.Name = parsed.Name
		existing.Pattern = parsed.Pattern
		existing.PatternHash = &parsed.PatternHash
		existing.Message = parsed.Message
		existing.Severity = models.Severity(parsed.Severity)
		existing.Category = parsed.Category
		existing.Enabled = true

		if err := s.ruleStore.Update(ctx, existing); err != nil {
			return fmt.Errorf("failed to update rule: %w", err)
		}
		result.Updated++
	} else {
		// Create new rule
		newRule := &models.PreventionRule{
			ID:          uuid.New(),
			RuleID:      parsed.RuleID,
			Name:        parsed.Name,
			Pattern:     parsed.Pattern,
			PatternHash: &parsed.PatternHash,
			Message:     parsed.Message,
			Severity:    models.Severity(parsed.Severity),
			Category:    parsed.Category,
			Enabled:     true,
		}

		if err := s.ruleStore.Create(ctx, newRule); err != nil {
			return fmt.Errorf("failed to create rule: %w", err)
		}
		result.Added++
	}

	return nil
}

// disableOrphanedRules disables rules that no longer exist in markdown files
func (s *RuleSyncService) disableOrphanedRules(ctx context.Context, processedIDs map[string]bool, result *RuleSyncResult) error {
	// Get all enabled rules (using large limit to get all)
	rules, err := s.ruleStore.List(ctx, boolPtr(true), "", 10000, 0)
	if err != nil {
		return fmt.Errorf("failed to list rules: %w", err)
	}

	for _, rule := range rules {
		if !processedIDs[rule.RuleID] {
			// Rule no longer exists in markdown files
			rule.Enabled = false
			if err := s.ruleStore.Update(ctx, &rule); err != nil {
				return fmt.Errorf("failed to disable rule %s: %w", rule.RuleID, err)
			}
			result.Disabled++
		}
	}

	return nil
}

// boolPtr returns a pointer to a bool value
func boolPtr(b bool) *bool {
	return &b
}
