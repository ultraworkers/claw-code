package models

import (
	"strings"
	"testing"

	"github.com/jackc/pgx/v5/pgtype"
)

func TestFailureEntry_Validate(t *testing.T) {
	tests := []struct {
		name    string
		entry   FailureEntry
		wantErr bool
		errMsg  string
	}{
		{
			name: "valid failure entry",
			entry: FailureEntry{
				FailureID:    "FAIL-001",
				Category:     "database",
				Severity:     "high",
				ErrorMessage: "Connection timeout",
				Status:       "active",
			},
			wantErr: false,
		},
		{
			name: "valid with all severities",
			entry: FailureEntry{
				FailureID:    "FAIL-002",
				Category:     "api",
				Severity:     "critical",
				ErrorMessage: "Service down",
				Status:       "active",
			},
			wantErr: false,
		},
		{
			name: "valid with medium severity",
			entry: FailureEntry{
				FailureID:    "FAIL-003",
				Category:     "cache",
				Severity:     "medium",
				ErrorMessage: "Cache miss",
				Status:       "resolved",
			},
			wantErr: false,
		},
		{
			name: "valid with low severity",
			entry: FailureEntry{
				FailureID:    "FAIL-004",
				Category:     "logging",
				Severity:     "low",
				ErrorMessage: "Log truncated",
				Status:       "deprecated",
			},
			wantErr: false,
		},
		{
			name: "missing failure_id",
			entry: FailureEntry{
				FailureID:    "",
				Category:     "database",
				Severity:     "high",
				ErrorMessage: "Connection timeout",
				Status:       "active",
			},
			wantErr: true,
			errMsg:  "failure_id is required",
		},
		{
			name: "missing category",
			entry: FailureEntry{
				FailureID:    "FAIL-001",
				Category:     "",
				Severity:     "high",
				ErrorMessage: "Connection timeout",
				Status:       "active",
			},
			wantErr: true,
			errMsg:  "category is required",
		},
		{
			name: "invalid severity",
			entry: FailureEntry{
				FailureID:    "FAIL-001",
				Category:     "database",
				Severity:     "invalid",
				ErrorMessage: "Connection timeout",
				Status:       "active",
			},
			wantErr: true,
			errMsg:  "invalid severity",
		},
		{
			name: "missing error_message",
			entry: FailureEntry{
				FailureID:    "FAIL-001",
				Category:     "database",
				Severity:     "high",
				ErrorMessage: "",
				Status:       "active",
			},
			wantErr: true,
			errMsg:  "error_message is required",
		},
		{
			name: "invalid status",
			entry: FailureEntry{
				FailureID:    "FAIL-001",
				Category:     "database",
				Severity:     "high",
				ErrorMessage: "Connection timeout",
				Status:       "invalid",
			},
			wantErr: true,
			errMsg:  "invalid status",
		},
		{
			name: "failure_id too long",
			entry: FailureEntry{
				FailureID:    strings.Repeat("a", 51),
				Category:     "database",
				Severity:     "high",
				ErrorMessage: "Connection timeout",
				Status:       "active",
			},
			wantErr: true,
			errMsg:  "failure_id must be at most 50 characters",
		},
		{
			name: "category too long",
			entry: FailureEntry{
				FailureID:    "FAIL-001",
				Category:     strings.Repeat("a", 51),
				Severity:     "high",
				ErrorMessage: "Connection timeout",
				Status:       "active",
			},
			wantErr: true,
			errMsg:  "category must be at most 50 characters",
		},
		{
			name: "project_slug too long",
			entry: FailureEntry{
				FailureID:    "FAIL-001",
				Category:     "database",
				Severity:     "high",
				ErrorMessage: "Connection timeout",
				Status:       "active",
				ProjectSlug:  strings.Repeat("a", 101),
			},
			wantErr: true,
			errMsg:  "project_slug must be at most 100 characters",
		},
		{
			name: "boundary failure_id length",
			entry: FailureEntry{
				FailureID:    strings.Repeat("a", 50),
				Category:     "database",
				Severity:     "high",
				ErrorMessage: "Connection timeout",
				Status:       "active",
			},
			wantErr: false,
		},
		{
			name: "boundary category length",
			entry: FailureEntry{
				FailureID:    "FAIL-001",
				Category:     strings.Repeat("a", 50),
				Severity:     "high",
				ErrorMessage: "Connection timeout",
				Status:       "active",
			},
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := tt.entry.Validate()
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

func TestIsValidFailureStatus(t *testing.T) {
	tests := []struct {
		name   string
		status string
		want   bool
	}{
		{"active status", "active", true},
		{"resolved status", "resolved", true},
		{"deprecated status", "deprecated", true},
		{"empty string", "", false},
		{"invalid status", "pending", false},
		{"uppercase active", "ACTIVE", false},
		{"mixed case", "Active", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := IsValidFailureStatus(tt.status)
			if got != tt.want {
				t.Errorf("IsValidFailureStatus(%q) = %v, want %v", tt.status, got, tt.want)
			}
		})
	}
}

func TestIsValidFailureSeverity(t *testing.T) {
	tests := []struct {
		name     string
		severity string
		want     bool
	}{
		{"critical severity", "critical", true},
		{"high severity", "high", true},
		{"medium severity", "medium", true},
		{"low severity", "low", true},
		{"empty string", "", false},
		{"invalid severity", "normal", false},
		{"uppercase critical", "CRITICAL", false},
		{"mixed case", "High", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := IsValidFailureSeverity(tt.severity)
			if got != tt.want {
				t.Errorf("IsValidFailureSeverity(%q) = %v, want %v", tt.severity, got, tt.want)
			}
		})
	}
}

func TestToStringSlice(t *testing.T) {
	tests := []struct {
		name string
		arr  pgtype.Array[string]
		want []string
	}{
		{
			name: "valid array",
			arr: pgtype.Array[string]{
				Elements: []string{"file1.go", "file2.go"},
				Valid:    true,
			},
			want: []string{"file1.go", "file2.go"},
		},
		{
			name: "null array",
			arr: pgtype.Array[string]{
				Elements: nil,
				Valid:    false,
			},
			want: nil,
		},
		{
			name: "empty valid array",
			arr: pgtype.Array[string]{
				Elements: []string{},
				Valid:    true,
			},
			want: []string{},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := ToStringSlice(tt.arr)
			if len(got) != len(tt.want) {
				t.Errorf("ToStringSlice() = %v, want %v", got, tt.want)
				return
			}
			for i := range got {
				if got[i] != tt.want[i] {
					t.Errorf("ToStringSlice()[%d] = %v, want %v", i, got[i], tt.want[i])
				}
			}
		})
	}
}

func TestToTextArray(t *testing.T) {
	tests := []struct {
		name  string
		slice []string
		want  pgtype.Array[string]
	}{
		{
			name:  "non-empty slice",
			slice: []string{"file1.go", "file2.go"},
			want: pgtype.Array[string]{
				Elements: []string{"file1.go", "file2.go"},
				Valid:    true,
			},
		},
		{
			name:  "nil slice",
			slice: nil,
			want: pgtype.Array[string]{
				Elements: nil,
				Valid:    false,
			},
		},
		{
			name:  "empty slice",
			slice: []string{},
			want: pgtype.Array[string]{
				Elements: []string{},
				Valid:    true,
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := ToTextArray(tt.slice)
			if got.Valid != tt.want.Valid {
				t.Errorf("ToTextArray().Valid = %v, want %v", got.Valid, tt.want.Valid)
			}
			if len(got.Elements) != len(tt.want.Elements) {
				t.Errorf("ToTextArray().Elements = %v, want %v", got.Elements, tt.want.Elements)
				return
			}
			for i := range got.Elements {
				if got.Elements[i] != tt.want.Elements[i] {
					t.Errorf("ToTextArray().Elements[%d] = %v, want %v", i, got.Elements[i], tt.want.Elements[i])
				}
			}
		})
	}
}

func TestFailureStatus_Constants(t *testing.T) {
	statuses := []FailureStatus{
		StatusActive,
		StatusResolved,
		StatusDeprecated,
	}

	expected := []string{"active", "resolved", "deprecated"}

	for i, status := range statuses {
		if string(status) != expected[i] {
			t.Errorf("Status constant %d = %q, want %q", i, status, expected[i])
		}
	}
}

func TestValidFailureSeverities(t *testing.T) {
	expected := []string{"critical", "high", "medium", "low"}

	if len(ValidFailureSeverities) != len(expected) {
		t.Errorf("ValidFailureSeverities length = %d, want %d", len(ValidFailureSeverities), len(expected))
		return
	}

	for i, sev := range ValidFailureSeverities {
		if sev != expected[i] {
			t.Errorf("ValidFailureSeverities[%d] = %q, want %q", i, sev, expected[i])
		}
	}
}

func BenchmarkFailureEntry_Validate(b *testing.B) {
	entry := FailureEntry{
		FailureID:    "FAIL-001",
		Category:     "database",
		Severity:     "high",
		ErrorMessage: "Connection timeout",
		Status:       "active",
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = entry.Validate()
	}
}

func BenchmarkIsValidFailureStatus(b *testing.B) {
	statuses := []string{"active", "resolved", "deprecated", "invalid"}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, status := range statuses {
			_ = IsValidFailureStatus(status)
		}
	}
}

func BenchmarkIsValidFailureSeverity(b *testing.B) {
	severities := []string{"critical", "high", "medium", "low", "invalid"}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, sev := range severities {
			_ = IsValidFailureSeverity(sev)
		}
	}
}
