package validation

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"strings"
	"sync"
	"time"

	"github.com/thearchitectit/guardrail-mcp/internal/cache"
	"github.com/thearchitectit/guardrail-mcp/internal/database"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// RuleCategory defines the type of rules for validation
type RuleCategory string

const (
	CategoryBash     RuleCategory = "bash"
	CategoryGit      RuleCategory = "git"
	CategoryFileEdit RuleCategory = "file_edit"
)

// Violation represents a rule violation found during validation
type Violation struct {
	RuleID         string          `json:"rule_id"`
	RuleName       string          `json:"rule_name"`
	Severity       models.Severity `json:"severity"`
	Message        string          `json:"message"`
	Category       string          `json:"category"`
	MatchedPattern string          `json:"matched_pattern"`
	MatchedInput   string          `json:"matched_input,omitempty"`
}

// compiledRule wraps a prevention rule with its compiled regex
type compiledRule struct {
	Rule    models.PreventionRule
	Pattern string
}

// FileReadVerification represents the result of verifying if a file was read
type FileReadVerification struct {
	WasRead       bool           `json:"was_read"`
	ReadAt        *time.Time     `json:"read_at,omitempty"`
	TimeSinceRead time.Duration  `json:"time_since_read,omitempty"`
}

// ValidationEngine performs guardrail validation against prevention rules
type ValidationEngine struct {
	ruleStore        *database.RuleStore
	fileReadStore    *database.FileReadStore
	taskAttemptStore *database.TaskAttemptStore
	cacheClient      *cache.Client
	rulesCache       []compiledRule
	cacheMu          sync.RWMutex
	cacheExpiry      time.Time
	cacheTTL         time.Duration
	maxInputSize     int
}

// ValidationOption configures the validation engine
type ValidationOption func(*ValidationEngine)

// WithCacheTTL sets a custom cache TTL
func WithCacheTTL(ttl time.Duration) ValidationOption {
	return func(e *ValidationEngine) {
		e.cacheTTL = ttl
	}
}

// WithMaxInputSize sets the maximum input size for validation
func WithMaxInputSize(size int) ValidationOption {
	return func(e *ValidationEngine) {
		e.maxInputSize = size
	}
}

// WithFileReadStore sets the file read store for validation
func WithFileReadStore(store *database.FileReadStore) ValidationOption {
	return func(e *ValidationEngine) {
		e.fileReadStore = store
	}
}

// WithTaskAttemptStore sets the task attempt store for validation
func WithTaskAttemptStore(store *database.TaskAttemptStore) ValidationOption {
	return func(e *ValidationEngine) {
		e.taskAttemptStore = store
	}
}

// NewValidationEngine creates a new validation engine
func NewValidationEngine(ruleStore *database.RuleStore, cacheClient *cache.Client, opts ...ValidationOption) *ValidationEngine {
	engine := &ValidationEngine{
		ruleStore:    ruleStore,
		cacheClient:  cacheClient,
		cacheTTL:     30 * time.Second,
		maxInputSize: 100 * 1024, // 100KB default limit
		rulesCache:   make([]compiledRule, 0),
	}

	for _, opt := range opts {
		opt(engine)
	}

	return engine
}

// ValidateBash validates a bash command against prevention rules
func (e *ValidationEngine) ValidateBash(ctx context.Context, command string) ([]Violation, error) {
	if err := e.validateInput(command); err != nil {
		return nil, err
	}

	rules, err := e.loadRulesFromDB(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to load rules: %w", err)
	}

	var violations []Violation
	for _, compiled := range rules {
		if !e.shouldCheckRule(compiled.Rule, CategoryBash) {
			continue
		}

		matched, err := MatchPattern(compiled.Pattern, command)
		if err != nil {
			slog.Warn("Pattern matching error",
				"rule_id", compiled.Rule.RuleID,
				"error", err,
			)
			continue
		}

		if matched {
			violations = append(violations, Violation{
				RuleID:         compiled.Rule.RuleID,
				RuleName:       compiled.Rule.Name,
				Severity:       compiled.Rule.Severity,
				Message:        compiled.Rule.Message,
				Category:       compiled.Rule.Category,
				MatchedPattern: compiled.Pattern,
				MatchedInput:   truncateString(command, 200),
			})
		}
	}

	return violations, nil
}

// ValidateGit validates a git command against prevention rules
func (e *ValidationEngine) ValidateGit(ctx context.Context, command string) ([]Violation, error) {
	if err := e.validateInput(command); err != nil {
		return nil, err
	}

	rules, err := e.loadRulesFromDB(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to load rules: %w", err)
	}

	var violations []Violation
	for _, compiled := range rules {
		if !e.shouldCheckRule(compiled.Rule, CategoryGit) {
			continue
		}

		matched, err := MatchPattern(compiled.Pattern, command)
		if err != nil {
			slog.Warn("Pattern matching error",
				"rule_id", compiled.Rule.RuleID,
				"error", err,
			)
			continue
		}

		if matched {
			violations = append(violations, Violation{
				RuleID:         compiled.Rule.RuleID,
				RuleName:       compiled.Rule.Name,
				Severity:       compiled.Rule.Severity,
				Message:        compiled.Rule.Message,
				Category:       compiled.Rule.Category,
				MatchedPattern: compiled.Pattern,
				MatchedInput:   truncateString(command, 200),
			})
		}
	}

	return violations, nil
}

// ValidateFileEdit validates a file edit against prevention rules
// sessionID is optional - if provided, checks if file was read before editing
func (e *ValidationEngine) ValidateFileEdit(ctx context.Context, filePath string, content string, sessionID string) ([]Violation, error) {
	if err := e.validateInput(content); err != nil {
		return nil, err
	}

	rules, err := e.loadRulesFromDB(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to load rules: %w", err)
	}

	var violations []Violation

	// Check if file was read before editing (if sessionID is provided and fileReadStore is configured)
	if sessionID != "" && e.fileReadStore != nil {
		verification, err := e.VerifyFileRead(ctx, sessionID, filePath)
		if err != nil {
			slog.Warn("Failed to verify file read", "session_id", sessionID, "file_path", filePath, "error", err)
		} else if !verification.WasRead {
			violations = append(violations, Violation{
				RuleID:         "FILE-READ-001",
				RuleName:       "File Not Read Before Edit",
				Severity:       models.SeverityCritical,
				Message:        fmt.Sprintf("File '%s' must be read before editing. Read the file first to understand its contents.", filePath),
				Category:       string(CategoryFileEdit),
				MatchedPattern: "file_not_read",
				MatchedInput:   truncateString(filePath, 200),
			})
		}
	}

	// Check both file path and content
	inputs := []string{filePath, content}
	inputLabels := []string{"path", "content"}

	for _, compiled := range rules {
		if !e.shouldCheckRule(compiled.Rule, CategoryFileEdit) {
			continue
		}

		for i, input := range inputs {
			matched, err := MatchPattern(compiled.Pattern, input)
			if err != nil {
				slog.Warn("Pattern matching error",
					"rule_id", compiled.Rule.RuleID,
					"input_type", inputLabels[i],
					"error", err,
				)
				continue
			}

			if matched {
				violation := Violation{
					RuleID:         compiled.Rule.RuleID,
					RuleName:       compiled.Rule.Name,
					Severity:       compiled.Rule.Severity,
					Message:        compiled.Rule.Message,
					Category:       compiled.Rule.Category,
					MatchedPattern: compiled.Pattern,
				}

				if inputLabels[i] == "path" {
					violation.MatchedInput = truncateString(filePath, 200)
				} else {
					violation.MatchedInput = truncateString(content, 200)
				}

				violations = append(violations, violation)

				// Break to avoid duplicate violations for the same rule
				break
			}
		}
	}

	return violations, nil
}

// VerifyFileRead checks if a file was read in a session and returns verification details
func (e *ValidationEngine) VerifyFileRead(ctx context.Context, sessionID, filePath string) (*FileReadVerification, error) {
	if e.fileReadStore == nil {
		return nil, fmt.Errorf("file read store not configured")
	}

	record, err := e.fileReadStore.GetBySessionAndPath(ctx, sessionID, filePath)
	if err != nil {
		// Check if it's a "not found" error
		if err.Error() == fmt.Sprintf("file read record not found for session %s and path %s", sessionID, filePath) {
			return &FileReadVerification{
				WasRead: false,
			}, nil
		}
		return nil, fmt.Errorf("failed to query file read: %w", err)
	}

	now := time.Now()
	timeSince := now.Sub(record.ReadAt)

	return &FileReadVerification{
		WasRead:       true,
		ReadAt:        &record.ReadAt,
		TimeSinceRead: timeSince,
	}, nil
}

// loadRulesFromDB loads active rules from database with caching
func (e *ValidationEngine) loadRulesFromDB(ctx context.Context) ([]compiledRule, error) {
	// Check in-memory cache first
	e.cacheMu.RLock()
	if time.Now().Before(e.cacheExpiry) && len(e.rulesCache) > 0 {
		cached := make([]compiledRule, len(e.rulesCache))
		copy(cached, e.rulesCache)
		e.cacheMu.RUnlock()
		slog.Debug("Using in-memory cached rules", "count", len(cached))
		return cached, nil
	}
	e.cacheMu.RUnlock()

	// Try Redis cache if available
	if e.cacheClient != nil {
		if cached, err := e.loadFromRedisCache(ctx); err == nil && len(cached) > 0 {
			// Update in-memory cache
			e.cacheMu.Lock()
			e.rulesCache = cached
			e.cacheExpiry = time.Now().Add(e.cacheTTL)
			e.cacheMu.Unlock()
			slog.Debug("Using Redis cached rules", "count", len(cached))
			return cached, nil
		}
	}

	// Load from database
	rules, err := e.ruleStore.GetActiveRules(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get active rules from database: %w", err)
	}

	// Compile rules
	compiled := make([]compiledRule, 0, len(rules))
	for _, rule := range rules {
		// Validate pattern before adding
		if err := ValidatePattern(rule.Pattern); err != nil {
			slog.Warn("Skipping rule with invalid pattern",
				"rule_id", rule.RuleID,
				"error", err,
			)
			continue
		}

		compiled = append(compiled, compiledRule{
			Rule:    rule,
			Pattern: rule.Pattern,
		})
	}

	// Update in-memory cache
	e.cacheMu.Lock()
	e.rulesCache = compiled
	e.cacheExpiry = time.Now().Add(e.cacheTTL)
	e.cacheMu.Unlock()

	// Update Redis cache if available
	if e.cacheClient != nil {
		if err := e.saveToRedisCache(ctx, compiled); err != nil {
			slog.Warn("Failed to cache rules in Redis", "error", err)
		}
	}

	slog.Debug("Loaded rules from database", "count", len(compiled))
	return compiled, nil
}

// loadFromRedisCache attempts to load compiled rules from Redis
func (e *ValidationEngine) loadFromRedisCache(ctx context.Context) ([]compiledRule, error) {
	data, err := e.cacheClient.GetActiveRules(ctx)
	if err != nil {
		return nil, err
	}

	var rules []models.PreventionRule
	if err := json.Unmarshal(data, &rules); err != nil {
		return nil, fmt.Errorf("failed to unmarshal cached rules: %w", err)
	}

	compiled := make([]compiledRule, 0, len(rules))
	for _, rule := range rules {
		compiled = append(compiled, compiledRule{
			Rule:    rule,
			Pattern: rule.Pattern,
		})
	}

	return compiled, nil
}

// saveToRedisCache saves compiled rules to Redis
func (e *ValidationEngine) saveToRedisCache(ctx context.Context, compiled []compiledRule) error {
	rules := make([]models.PreventionRule, len(compiled))
	for i, c := range compiled {
		rules[i] = c.Rule
	}

	data, err := json.Marshal(rules)
	if err != nil {
		return fmt.Errorf("failed to marshal rules: %w", err)
	}

	return e.cacheClient.SetActiveRules(ctx, data, e.cacheTTL)
}

// shouldCheckRule determines if a rule should be checked for a given category
func (e *ValidationEngine) shouldCheckRule(rule models.PreventionRule, category RuleCategory) bool {
	if !rule.Enabled {
		return false
	}

	// Check if rule category matches
	ruleCategory := strings.ToLower(rule.Category)
	checkCategory := strings.ToLower(string(category))

	// Exact match
	if ruleCategory == checkCategory {
		return true
	}

	// "all" category applies to everything
	if ruleCategory == "all" {
		return true
	}

	// Legacy category mappings for backward compatibility
	switch category {
	case CategoryBash:
		return ruleCategory == "command" || ruleCategory == "shell"
	case CategoryGit:
		return ruleCategory == "version_control" || ruleCategory == "scm"
	case CategoryFileEdit:
		return ruleCategory == "file" || ruleCategory == "edit"
	}

	return false
}

// validateInput checks if input is valid for validation
func (e *ValidationEngine) validateInput(input string) error {
	if len(input) == 0 {
		return fmt.Errorf("input cannot be empty")
	}
	if len(input) > e.maxInputSize {
		return fmt.Errorf("input exceeds maximum size of %d bytes", e.maxInputSize)
	}
	return nil
}

// InvalidateCache clears the rule cache (useful after rule updates)
func (e *ValidationEngine) InvalidateCache() {
	e.cacheMu.Lock()
	e.rulesCache = make([]compiledRule, 0)
	e.cacheExpiry = time.Time{}
	e.cacheMu.Unlock()
	slog.Info("Validation engine cache invalidated")
}

// GetCachedRuleCount returns the number of rules currently in cache
func (e *ValidationEngine) GetCachedRuleCount() int {
	e.cacheMu.RLock()
	defer e.cacheMu.RUnlock()
	return len(e.rulesCache)
}

// GetCachedRulesCount returns the number of rules currently in cache (alias for backward compatibility)
func (e *ValidationEngine) GetCachedRulesCount() int {
	return e.GetCachedRuleCount()
}

// ValidateInput validates input against active prevention rules (backward compatible method)
// If categoryFilter is provided, only rules matching those categories are checked
func (e *ValidationEngine) ValidateInput(ctx context.Context, input string, categoryFilter []string) ([]Violation, error) {
	if err := e.validateInput(input); err != nil {
		return nil, err
	}

	rules, err := e.loadRulesFromDB(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to load rules: %w", err)
	}

	var violations []Violation
	for _, compiled := range rules {
		// Check if rule matches category filter
		if len(categoryFilter) > 0 && !e.ruleMatchesCategories(compiled.Rule, categoryFilter) {
			continue
		}

		matched, err := MatchPattern(compiled.Pattern, input)
		if err != nil {
			slog.Warn("Pattern matching error",
				"rule_id", compiled.Rule.RuleID,
				"error", err,
			)
			continue
		}

		if matched {
			violations = append(violations, Violation{
				RuleID:         compiled.Rule.RuleID,
				RuleName:       compiled.Rule.Name,
				Severity:       compiled.Rule.Severity,
				Message:        compiled.Rule.Message,
				Category:       compiled.Rule.Category,
				MatchedPattern: compiled.Pattern,
				MatchedInput:   truncateString(input, 200),
			})
		}
	}

	return violations, nil
}

// ruleMatchesCategories checks if a rule matches any of the given categories
func (e *ValidationEngine) ruleMatchesCategories(rule models.PreventionRule, categories []string) bool {
	if !rule.Enabled {
		return false
	}
	ruleCategory := strings.ToLower(rule.Category)
	for _, cat := range categories {
		if ruleCategory == strings.ToLower(cat) {
			return true
		}
		// Support "all" category
		if ruleCategory == "all" {
			return true
		}
	}
	return false
}

// truncateString truncates a string to a maximum length
func truncateString(s string, maxLen int) string {
	if len(s) <= maxLen {
		return s
	}
	return s[:maxLen] + "..."
}
