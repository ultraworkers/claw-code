package validation

import (
	"regexp"
	"strings"
	"testing"
	"time"
)

func TestSafeRegex(t *testing.T) {
	tests := []struct {
		name      string
		pattern   string
		input     string
		timeout   time.Duration
		wantMatch bool
		wantErr   bool
		errMsg    string
	}{
		{
			name:      "simple match",
			pattern:   `hello`,
			input:     "hello world",
			timeout:   100 * time.Millisecond,
			wantMatch: true,
			wantErr:   false,
		},
		{
			name:      "no match",
			pattern:   `goodbye`,
			input:     "hello world",
			timeout:   100 * time.Millisecond,
			wantMatch: false,
			wantErr:   false,
		},
		{
			name:      "regex with anchors",
			pattern:   `^hello$`,
			input:     "hello",
			timeout:   100 * time.Millisecond,
			wantMatch: true,
			wantErr:   false,
		},
		{
			name:      "complex pattern",
			pattern:   `[a-z]+@[a-z]+\.[a-z]+`,
			input:     "contact email: test@example.com",
			timeout:   100 * time.Millisecond,
			wantMatch: true,
			wantErr:   false,
		},
		{
			name:      "invalid pattern",
			pattern:   `[invalid(`,
			input:     "test",
			timeout:   100 * time.Millisecond,
			wantMatch: false,
			wantErr:   false, // Returns false, nil for compile errors
		},
		{
			name:      "empty pattern",
			pattern:   "",
			input:     "test",
			timeout:   100 * time.Millisecond,
			wantMatch: true, // Empty pattern matches everything
			wantErr:   false,
		},
		{
			name:      "empty input",
			pattern:   `test`,
			input:     "",
			timeout:   100 * time.Millisecond,
			wantMatch: false,
			wantErr:   false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := SafeRegex(tt.pattern, tt.input, tt.timeout)
			if (err != nil) != tt.wantErr {
				t.Errorf("SafeRegex() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if tt.wantErr && err != nil && tt.errMsg != "" {
				if !strings.Contains(err.Error(), tt.errMsg) {
					t.Errorf("SafeRegex() error message = %v, want containing %v", err.Error(), tt.errMsg)
				}
			}
			if got != tt.wantMatch {
				t.Errorf("SafeRegex() = %v, want %v", got, tt.wantMatch)
			}
		})
	}
}

func TestSafeRegex_ReDoSProtection(t *testing.T) {
	// Test that SafeRegex returns results without hanging
	// Note: Go's regex engine (RE2) doesn't use backtracking, so it doesn't
	// suffer from catastrophic backtracking like PCRE. This test verifies
	// that the function completes without error and within reasonable time.
	pattern := `(a+)+$`
	input := strings.Repeat("a", 30) + "b"

	start := time.Now()
	matched, err := SafeRegex(pattern, input, 100*time.Millisecond)
	duration := time.Since(start)

	// Should complete quickly (Go's RE2 engine is linear time)
	if err != nil {
		t.Errorf("SafeRegex() returned error = %v, want nil (Go RE2 doesn't backtrack)", err)
	}
	if duration > 500*time.Millisecond {
		t.Errorf("SafeRegex() took too long (%v), expected faster completion", duration)
	}
	// Pattern should not match (input ends with 'b', pattern expects $ after a+)
	if matched {
		t.Error("SafeRegex() matched = true, want false")
	}
}

func TestValidatePattern(t *testing.T) {
	tests := []struct {
		name    string
		pattern string
		wantErr bool
		errMsg  string
	}{
		{
			name:    "valid simple pattern",
			pattern: `hello`,
			wantErr: false,
		},
		{
			name:    "valid complex pattern",
			pattern: `^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$`,
			wantErr: false,
		},
		{
			name:    "invalid pattern",
			pattern: `[invalid(`,
			wantErr: true,
			errMsg:  "invalid regex pattern",
		},
		{
			name:    "pattern too long",
			pattern: strings.Repeat("a", 10001),
			wantErr: true,
			errMsg:  "pattern too long",
		},
		{
			name:    "invalid nested quantifiers - star plus (rejected by Go RE2)",
			pattern: `a*+`,
			wantErr: true,
			errMsg:  "invalid regex pattern",
		},
		{
			name:    "invalid nested quantifiers - plus star (rejected by Go RE2)",
			pattern: `a+*`,
			wantErr: true,
			errMsg:  "invalid regex pattern",
		},
		{
			name:    "dangerous nested quantifiers - double question",
			pattern: `a??`,
			wantErr: true,
			errMsg:  "potentially dangerous nested quantifiers",
		},
		{
			name:    "invalid nested quantifiers - repeated braces (rejected by Go RE2)",
			pattern: `a{1,2}{3,4}`,
			wantErr: true,
			errMsg:  "invalid regex pattern",
		},
		{
			name:    "safe quantifiers - single star",
			pattern: `a*`,
			wantErr: false,
		},
		{
			name:    "safe quantifiers - single plus",
			pattern: `a+`,
			wantErr: false,
		},
		{
			name:    "safe quantifiers - single question",
			pattern: `a?`,
			wantErr: false,
		},
		{
			name:    "safe quantifiers - single braces",
			pattern: `a{1,10}`,
			wantErr: false,
		},
		{
			name:    "pattern at max length",
			pattern: strings.Repeat("a", 10000),
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := ValidatePattern(tt.pattern)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidatePattern() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if tt.wantErr && err != nil && tt.errMsg != "" {
				if !strings.Contains(err.Error(), tt.errMsg) {
					t.Errorf("ValidatePattern() error message = %v, want containing %v", err.Error(), tt.errMsg)
				}
			}
		})
	}
}

func TestMatchPattern(t *testing.T) {
	tests := []struct {
		name      string
		pattern   string
		input     string
		wantMatch bool
		wantErr   bool
	}{
		{
			name:      "simple match",
			pattern:   `test`,
			input:     "this is a test",
			wantMatch: true,
			wantErr:   false,
		},
		{
			name:      "no match",
			pattern:   `hello`,
			input:     "goodbye world",
			wantMatch: false,
			wantErr:   false,
		},
		{
			name:      "invalid pattern",
			pattern:   `[invalid(`,
			input:     "test",
			wantMatch: false,
			wantErr:   false, // Returns false for compile errors
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := MatchPattern(tt.pattern, tt.input)
			if (err != nil) != tt.wantErr {
				t.Errorf("MatchPattern() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got != tt.wantMatch {
				t.Errorf("MatchPattern() = %v, want %v", got, tt.wantMatch)
			}
		})
	}
}

func TestMatchPattern_DefaultTimeout(t *testing.T) {
	// Verify that MatchPattern uses a reasonable default timeout
	// This is more of a smoke test to ensure it doesn't hang
	pattern := `^test$`
	input := "test"

	done := make(chan bool)
	go func() {
		_, _ = MatchPattern(pattern, input)
		done <- true
	}()

	select {
	case <-done:
		// Success - completed within timeout
	case <-time.After(500 * time.Millisecond):
		t.Error("MatchPattern() took too long, may not respect timeout")
	}
}

func BenchmarkSafeRegex(b *testing.B) {
	pattern := `^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$`
	input := "test.user+tag@example.co.uk"

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = SafeRegex(pattern, input, 100*time.Millisecond)
	}
}

func BenchmarkValidatePattern(b *testing.B) {
	pattern := `^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$`

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = ValidatePattern(pattern)
	}
}

func BenchmarkMatchPattern(b *testing.B) {
	pattern := `test`
	input := "this is a test string with test in it"

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = MatchPattern(pattern, input)
	}
}

// TestRegexComparison compares SafeRegex with standard regexp for performance
func BenchmarkRegexComparison(b *testing.B) {
	pattern := `^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$`
	input := "test.user+tag@example.co.uk"

	b.Run("StandardRegexp", func(b *testing.B) {
		re := regexp.MustCompile(pattern)
		b.ResetTimer()
		for i := 0; i < b.N; i++ {
			_ = re.MatchString(input)
		}
	})

	b.Run("SafeRegex", func(b *testing.B) {
		b.ResetTimer()
		for i := 0; i < b.N; i++ {
			_, _ = SafeRegex(pattern, input, 100*time.Millisecond)
		}
	})
}
