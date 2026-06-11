package models

import (
	"strings"
	"testing"
)

func TestPreventionRule_Validate(t *testing.T) {
	tests := []struct {
		name    string
		rule    PreventionRule
		wantErr bool
		errMsg  string
	}{
		{
			name: "valid rule",
			rule: PreventionRule{
				RuleID:   "PREVENT-001",
				Name:     "No Force Push",
				Pattern:  `git\s+push\s+--force`,
				Message:  "Force push is not allowed",
				Severity: SeverityError,
			},
			wantErr: false,
		},
		{
			name: "valid rule with warning severity",
			rule: PreventionRule{
				RuleID:   "PREVENT-002",
				Name:     "Large File Warning",
				Pattern:  `file.*size\s*>\s*100MB`,
				Message:  "Large file detected",
				Severity: SeverityWarning,
			},
			wantErr: false,
		},
		{
			name: "valid rule with info severity",
			rule: PreventionRule{
				RuleID:   "PREVENT-003",
				Name:     "Info Rule",
				Pattern:  `TODO:`,
				Message:  "TODO found",
				Severity: SeverityInfo,
			},
			wantErr: false,
		},
		{
			name: "missing rule_id",
			rule: PreventionRule{
				RuleID:   "",
				Name:     "Test Rule",
				Pattern:  `test`,
				Message:  "Test message",
				Severity: SeverityError,
			},
			wantErr: true,
			errMsg:  "rule_id is required",
		},
		{
			name: "missing name",
			rule: PreventionRule{
				RuleID:   "PREVENT-001",
				Name:     "",
				Pattern:  `test`,
				Message:  "Test message",
				Severity: SeverityError,
			},
			wantErr: true,
			errMsg:  "name is required",
		},
		{
			name: "missing pattern",
			rule: PreventionRule{
				RuleID:   "PREVENT-001",
				Name:     "Test Rule",
				Pattern:  "",
				Message:  "Test message",
				Severity: SeverityError,
			},
			wantErr: true,
			errMsg:  "pattern is required",
		},
		{
			name: "missing message",
			rule: PreventionRule{
				RuleID:   "PREVENT-001",
				Name:     "Test Rule",
				Pattern:  `test`,
				Message:  "",
				Severity: SeverityError,
			},
			wantErr: true,
			errMsg:  "message is required",
		},
		{
			name: "invalid severity",
			rule: PreventionRule{
				RuleID:   "PREVENT-001",
				Name:     "Test Rule",
				Pattern:  `test`,
				Message:  "Test message",
				Severity: "invalid",
			},
			wantErr: true,
			errMsg:  "invalid severity",
		},
		{
			name: "rule_id too long",
			rule: PreventionRule{
				RuleID:   strings.Repeat("a", 51),
				Name:     "Test Rule",
				Pattern:  `test`,
				Message:  "Test message",
				Severity: SeverityError,
			},
			wantErr: true,
			errMsg:  "rule_id must be at most 50 characters",
		},
		{
			name: "name too long",
			rule: PreventionRule{
				RuleID:   "PREVENT-001",
				Name:     strings.Repeat("a", 256),
				Pattern:  `test`,
				Message:  "Test message",
				Severity: SeverityError,
			},
			wantErr: true,
			errMsg:  "name must be at most 255 characters",
		},
		{
			name: "category too long",
			rule: PreventionRule{
				RuleID:   "PREVENT-001",
				Name:     "Test Rule",
				Pattern:  `test`,
				Message:  "Test message",
				Severity: SeverityError,
				Category: strings.Repeat("a", 51),
			},
			wantErr: true,
			errMsg:  "category must be at most 50 characters",
		},
		{
			name: "boundary rule_id length",
			rule: PreventionRule{
				RuleID:   strings.Repeat("a", 50),
				Name:     "Test Rule",
				Pattern:  `test`,
				Message:  "Test message",
				Severity: SeverityError,
			},
			wantErr: false,
		},
		{
			name: "boundary name length",
			rule: PreventionRule{
				RuleID:   "PREVENT-001",
				Name:     strings.Repeat("a", 255),
				Pattern:  `test`,
				Message:  "Test message",
				Severity: SeverityError,
			},
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := tt.rule.Validate()
			if (err != nil) != tt.wantErr {
				t.Errorf("Validate() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if tt.wantErr && err != nil && tt.errMsg != "" {
				if !strings.Contains(err.Error(), tt.errMsg) {
					t.Errorf("Validate() error message = %v, want containing %v", err.Error(), tt.errMsg)
				}
			}
		})
	}
}

func TestIsValidSeverity(t *testing.T) {
	tests := []struct {
		name string
		sev  string
		want bool
	}{
		{"critical severity", "critical", true},
		{"error severity", "error", true},
		{"warning severity", "warning", true},
		{"info severity", "info", true},
		{"empty string", "", false},
		{"uppercase error", "ERROR", false},
		{"mixed case", "Error", false},
		{"similar but invalid", "errors", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := IsValidSeverity(tt.sev)
			if got != tt.want {
				t.Errorf("IsValidSeverity(%q) = %v, want %v", tt.sev, got, tt.want)
			}
		})
	}
}

func TestSeverity_Action(t *testing.T) {
	tests := []struct {
		name string
		sev  Severity
		want string
	}{
		{"error action", SeverityError, "halt"},
		{"warning action", SeverityWarning, "confirm"},
		{"info action", SeverityInfo, "log"},
		{"unknown severity", Severity("unknown"), "log"},
		{"empty severity", Severity(""), "log"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := tt.sev.Action()
			if got != tt.want {
				t.Errorf("Severity(%q).Action() = %v, want %v", tt.sev, got, tt.want)
			}
		})
	}
}

func TestSeverity_Constants(t *testing.T) {
	// Test that all expected constants are defined
	severities := []Severity{
		SeverityError,
		SeverityWarning,
		SeverityInfo,
	}

	expected := []string{"error", "warning", "info"}

	for i, sev := range severities {
		if string(sev) != expected[i] {
			t.Errorf("Severity constant %d = %q, want %q", i, sev, expected[i])
		}
	}
}

func BenchmarkPreventionRule_Validate(b *testing.B) {
	rule := PreventionRule{
		RuleID:   "PREVENT-001",
		Name:     "No Force Push",
		Pattern:  `git\s+push\s+--force`,
		Message:  "Force push is not allowed",
		Severity: SeverityError,
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = rule.Validate()
	}
}

func BenchmarkIsValidSeverity(b *testing.B) {
	severities := []string{"error", "warning", "info", "invalid"}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, sev := range severities {
			_ = IsValidSeverity(sev)
		}
	}
}

func BenchmarkSeverity_Action(b *testing.B) {
	severities := []Severity{SeverityError, SeverityWarning, SeverityInfo}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, sev := range severities {
			_ = sev.Action()
		}
	}
}
