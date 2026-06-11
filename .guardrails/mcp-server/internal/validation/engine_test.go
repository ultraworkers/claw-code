package validation

import (
	"context"
	"testing"
	"time"

	"github.com/google/uuid"
	"github.com/thearchitectit/guardrail-mcp/internal/models"
)

// mockRuleStore implements a simple in-memory rule store for testing
type mockRuleStore struct {
	rules []models.PreventionRule
	err   error
}

func (m *mockRuleStore) GetActiveRules(ctx context.Context) ([]models.PreventionRule, error) {
	if m.err != nil {
		return nil, m.err
	}
	return m.rules, nil
}

func TestValidationEngine_Constructor(t *testing.T) {
	t.Run("NewValidationEngine with defaults", func(t *testing.T) {
		engine := NewValidationEngine(nil, nil)
		if engine == nil {
			t.Fatal("expected engine to not be nil")
		}
		if engine.cacheTTL != 30*time.Second {
			t.Errorf("expected default TTL 30s, got %v", engine.cacheTTL)
		}
		if engine.maxInputSize != 100*1024 {
			t.Errorf("expected default maxInputSize 100KB, got %d", engine.maxInputSize)
		}
	})

	t.Run("NewValidationEngine with options", func(t *testing.T) {
		engine := NewValidationEngine(nil, nil,
			WithCacheTTL(5*time.Minute),
			WithMaxInputSize(1024*1024),
		)
		if engine.cacheTTL != 5*time.Minute {
			t.Errorf("expected TTL 5m, got %v", engine.cacheTTL)
		}
		if engine.maxInputSize != 1024*1024 {
			t.Errorf("expected maxInputSize 1MB, got %d", engine.maxInputSize)
		}
	})
}

func TestValidationEngine_shouldCheckRule(t *testing.T) {
	engine := NewValidationEngine(nil, nil)

	tests := []struct {
		name     string
		rule     models.PreventionRule
		category RuleCategory
		want     bool
	}{
		{
			name:     "exact match bash",
			rule:     models.PreventionRule{Enabled: true, Category: "bash"},
			category: CategoryBash,
			want:     true,
		},
		{
			name:     "exact match git",
			rule:     models.PreventionRule{Enabled: true, Category: "git"},
			category: CategoryGit,
			want:     true,
		},
		{
			name:     "exact match file_edit",
			rule:     models.PreventionRule{Enabled: true, Category: "file_edit"},
			category: CategoryFileEdit,
			want:     true,
		},
		{
			name:     "all category matches everything",
			rule:     models.PreventionRule{Enabled: true, Category: "all"},
			category: CategoryBash,
			want:     true,
		},
		{
			name:     "disabled rule",
			rule:     models.PreventionRule{Enabled: false, Category: "bash"},
			category: CategoryBash,
			want:     false,
		},
		{
			name:     "legacy command category",
			rule:     models.PreventionRule{Enabled: true, Category: "command"},
			category: CategoryBash,
			want:     true,
		},
		{
			name:     "legacy shell category",
			rule:     models.PreventionRule{Enabled: true, Category: "shell"},
			category: CategoryBash,
			want:     true,
		},
		{
			name:     "mismatched category",
			rule:     models.PreventionRule{Enabled: true, Category: "git"},
			category: CategoryBash,
			want:     false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := engine.shouldCheckRule(tt.rule, tt.category)
			if got != tt.want {
				t.Errorf("shouldCheckRule() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestValidationEngine_validateInput(t *testing.T) {
	engine := NewValidationEngine(nil, nil, WithMaxInputSize(100))

	tests := []struct {
		name    string
		input   string
		wantErr bool
	}{
		{
			name:    "valid input",
			input:   "echo hello",
			wantErr: false,
		},
		{
			name:    "empty input",
			input:   "",
			wantErr: true,
		},
		{
			name:    "input too large",
			input:   string(make([]byte, 101)),
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := engine.validateInput(tt.input)
			if (err != nil) != tt.wantErr {
				t.Errorf("validateInput() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestValidationEngine_Cache(t *testing.T) {
	t.Run("InvalidateCache clears cache", func(t *testing.T) {
		engine := NewValidationEngine(nil, nil)

		// Manually set cache
		engine.rulesCache = []compiledRule{
			{Rule: models.PreventionRule{RuleID: "test"}},
		}
		engine.cacheExpiry = time.Now().Add(30 * time.Second)

		if engine.GetCachedRuleCount() != 1 {
			t.Errorf("expected 1 cached rule, got %d", engine.GetCachedRuleCount())
		}

		engine.InvalidateCache()

		if engine.GetCachedRuleCount() != 0 {
			t.Errorf("expected 0 cached rules after invalidation, got %d", engine.GetCachedRuleCount())
		}
	})

	t.Run("GetCachedRuleCount returns zero when empty", func(t *testing.T) {
		engine := NewValidationEngine(nil, nil)
		if engine.GetCachedRuleCount() != 0 {
			t.Errorf("expected 0, got %d", engine.GetCachedRuleCount())
		}
	})
}

func TestValidationEngine_MatchPatterns(t *testing.T) {
	// Test using the safe regex matching functions
	tests := []struct {
		name    string
		pattern string
		input   string
		matches bool
	}{
		{
			name:    "simple match",
			pattern: `rm -rf`,
			input:   "rm -rf /",
			matches: true,
		},
		{
			name:    "no match",
			pattern: `rm -rf`,
			input:   "ls -la",
			matches: false,
		},
		{
			name:    "case insensitive",
			pattern: `(?i)SELECT.*FROM`,
			input:   "select * from users",
			matches: true,
		},
		{
			name:    "git force push",
			pattern: `git\s+push\s+--force`,
			input:   "git push --force origin main",
			matches: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			matched, err := MatchPattern(tt.pattern, tt.input)
			if err != nil {
				t.Fatalf("MatchPattern() error = %v", err)
			}
			if matched != tt.matches {
				t.Errorf("MatchPattern() = %v, want %v for input %q", matched, tt.matches, tt.input)
			}
		})
	}
}

func TestViolation_Struct(t *testing.T) {
	// Test that Violation struct has all required fields
	v := Violation{
		RuleID:         "rule-123",
		RuleName:       "Test Rule",
		Severity:       models.SeverityError,
		Message:        "This is a test violation",
		Category:       "bash",
		MatchedPattern: `rm\s+-rf`,
		MatchedInput:   "rm -rf /",
	}

	if v.RuleID != "rule-123" {
		t.Error("RuleID not set correctly")
	}
	if v.RuleName != "Test Rule" {
		t.Error("RuleName not set correctly")
	}
	if v.Severity != models.SeverityError {
		t.Error("Severity not set correctly")
	}
	if v.Message != "This is a test violation" {
		t.Error("Message not set correctly")
	}
	if v.Category != "bash" {
		t.Error("Category not set correctly")
	}
	if v.MatchedPattern != `rm\s+-rf` {
		t.Error("MatchedPattern not set correctly")
	}
	if v.MatchedInput != "rm -rf /" {
		t.Error("MatchedInput not set correctly")
	}
}

func TestTruncateString(t *testing.T) {
	tests := []struct {
		name    string
		input   string
		maxLen  int
		want    string
	}{
		{
			name:   "short string",
			input:  "hello",
			maxLen: 10,
			want:   "hello",
		},
		{
			name:   "exact length",
			input:  "hello",
			maxLen: 5,
			want:   "hello",
		},
		{
			name:   "long string",
			input:  "hello world",
			maxLen: 5,
			want:   "hello...",
		},
		{
			name:   "empty string",
			input:  "",
			maxLen: 5,
			want:   "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := truncateString(tt.input, tt.maxLen)
			if got != tt.want {
				t.Errorf("truncateString() = %q, want %q", got, tt.want)
			}
		})
	}
}

func TestCompiledRule(t *testing.T) {
	// Test that compiledRule struct works correctly
	rule := models.PreventionRule{
		ID:       uuid.New(),
		RuleID:   "test-rule",
		Name:     "Test Rule",
		Pattern:  `test.*pattern`,
		Message:  "Test message",
		Severity: models.SeverityWarning,
		Enabled:  true,
		Category: "test",
	}

	cr := compiledRule{
		Rule:    rule,
		Pattern: rule.Pattern,
	}

	if cr.Rule.RuleID != "test-rule" {
		t.Error("compiledRule Rule not set correctly")
	}
	if cr.Pattern != `test.*pattern` {
		t.Error("compiledRule Pattern not set correctly")
	}
}
