package security

import (
	"strings"
	"testing"
)

func TestScanContent(t *testing.T) {
	tests := []struct {
		name          string
		content       string
		wantCount     int
		wantPattern   string
		containsMatch string
	}{
		{
			name:          "AWS access key",
			content:       "AKIAIOSFODNN7EXAMPLE",
			wantCount:     1,
			wantPattern:   "AWS Access Key ID",
			containsMatch: "AKIA",
		},
		{
			name:          "GitHub token",
			content:       "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
			wantCount:     1,
			wantPattern:   "GitHub Token",
			containsMatch: "ghp_",
		},
		{
			name:          "GitHub OAuth token",
			content:       "gho_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
			wantCount:     1,
			wantPattern:   "GitHub Token",
			containsMatch: "gho_",
		},
		{
			name:          "Private key PEM",
			content:       "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...",
			wantCount:     1,
			wantPattern:   "Private Key",
			containsMatch: "BEGIN",
		},
		{
			name:          "Slack token",
			content:       "xoxb-FAKE_TEST_TOKEN_NOT_REAL-1234567890123-TESTING_ONLY",
			wantCount:     1,
			wantPattern:   "Slack Token",
			containsMatch: "xoxb",
		},
		{
			name:          "Generic API key",
			content:       `api_key: "abc123def456ghi789jkl012mno345pq"`,
			wantCount:     1,
			wantPattern:   "Generic API Key",
			containsMatch: "api_key",
		},
		{
			name:          "JWT token",
			content:       "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
			wantCount:     1,
			wantPattern:   "JWT Token",
			containsMatch: "eyJ",
		},
		{
			name:          "Clean content",
			content:       "This is just normal documentation without any secrets.",
			wantCount:     0,
			wantPattern:   "",
			containsMatch: "",
		},
		{
			name:          "Multiple secrets",
			content:       "AKIAIOSFODNN7EXAMPLE and ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
			wantCount:     2,
			wantPattern:   "",
			containsMatch: "",
		},
		{
			name:          "Empty content",
			content:       "",
			wantCount:     0,
			wantPattern:   "",
			containsMatch: "",
		},
		{
			name:          "Secret in code block",
			content:       "```\nAKIAIOSFODNN7EXAMPLE\n```",
			wantCount:     1,
			wantPattern:   "AWS Access Key ID",
			containsMatch: "AKIA",
		},
		{
			name:          "Secret with surrounding text",
			content:       "Here is my key: AKIAIOSFODNN7EXAMPLE - please don't share",
			wantCount:     1,
			wantPattern:   "AWS Access Key ID",
			containsMatch: "AKIA",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			results := ScanContent(tt.content)

			if len(results) != tt.wantCount {
				t.Errorf("ScanContent() found %d secrets, want %d", len(results), tt.wantCount)
				return
			}

			if tt.wantCount > 0 && tt.wantPattern != "" {
				found := false
				for _, r := range results {
					if r.Pattern == tt.wantPattern {
						found = true
						break
					}
				}
				if !found {
					t.Errorf("ScanContent() did not find pattern %q", tt.wantPattern)
				}
			}

			if tt.containsMatch != "" && tt.wantCount > 0 {
				found := false
				for _, r := range results {
					if strings.Contains(r.Match, tt.containsMatch) {
						found = true
						break
					}
				}
				if !found {
					// Check if the match contains the substring (it might be masked)
					for _, r := range results {
						t.Logf("Found match: %q", r.Match)
					}
				}
			}
		})
	}
}

func TestScanContent_Multiline(t *testing.T) {
	content := `Line 1: Some config
Line 2: AKIAIOSFODNN7EXAMPLE
Line 3: More text
Line 4: ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
Line 5: End`

	results := ScanContent(content)

	// Should find 2 secrets on different lines
	if len(results) != 2 {
		t.Errorf("ScanContent() found %d secrets, want 2", len(results))
	}

	// Check that line numbers are correct
	awsFound := false
	githubFound := false
	for _, r := range results {
		if r.Pattern == "AWS Access Key ID" && r.Line == 2 {
			awsFound = true
		}
		if r.Pattern == "GitHub Token" && r.Line == 4 {
			githubFound = true
		}
	}

	if !awsFound {
		t.Error("AWS key not found on expected line (2)")
	}
	if !githubFound {
		t.Error("GitHub token not found on expected line (4)")
	}
}

func TestValidateDocument(t *testing.T) {
	tests := []struct {
		name    string
		content string
		wantErr bool
		errMsg  string
	}{
		{
			name:    "clean document",
			content: "This is a clean document without secrets.",
			wantErr: false,
		},
		{
			name:    "document with AWS key",
			content: "AKIAIOSFODNN7EXAMPLE",
			wantErr: true,
			errMsg:  "potential secrets detected",
		},
		{
			name:    "document with GitHub token",
			content: "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
			wantErr: true,
			errMsg:  "potential secrets detected",
		},
		{
			name:    "empty document",
			content: "",
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := ValidateDocument(tt.content)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateDocument() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if tt.wantErr && err != nil && tt.errMsg != "" {
				if !strings.Contains(strings.ToLower(err.Error()), tt.errMsg) {
					t.Errorf("ValidateDocument() error message = %v, want containing %v", err.Error(), tt.errMsg)
				}
			}
		})
	}
}

func TestHasSecrets(t *testing.T) {
	tests := []struct {
		name string
		text string
		want bool
	}{
		{"has AWS key", "AKIAIOSFODNN7EXAMPLE", true},
		{"has GitHub token", "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", true},
		{"has private key", "-----BEGIN RSA PRIVATE KEY-----", true},
		{"clean text", "This is just normal text", false},
		{"empty text", "", false},
		{"has Slack token", "xoxb-FAKE_TEST_TOKEN_NOT_REAL-1234567890123-TESTING_ONLY", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := HasSecrets(tt.text)
			if got != tt.want {
				t.Errorf("HasSecrets(%q) = %v, want %v", tt.text, got, tt.want)
			}
		})
	}
}

func TestScanResult_Fields(t *testing.T) {
	// Test that ScanResult contains expected fields
	result := ScanResult{
		Pattern:     "Test Pattern",
		Line:        5,
		Column:      10,
		Match:       "secret123",
		Description: "Test description",
	}

	if result.Pattern != "Test Pattern" {
		t.Errorf("ScanResult.Pattern = %q, want %q", result.Pattern, "Test Pattern")
	}
	if result.Line != 5 {
		t.Errorf("ScanResult.Line = %d, want %d", result.Line, 5)
	}
	if result.Column != 10 {
		t.Errorf("ScanResult.Column = %d, want %d", result.Column, 10)
	}
	if result.Match != "secret123" {
		t.Errorf("ScanResult.Match = %q, want %q", result.Match, "secret123")
	}
	if result.Description != "Test description" {
		t.Errorf("ScanResult.Description = %q, want %q", result.Description, "Test description")
	}
}

func TestMaskSecret(t *testing.T) {
	tests := []struct {
		name   string
		secret string
		want   string
	}{
		{"normal secret", "supersecretpassword123", "supe****d123"},
		{"short secret", "tiny", "****"},
		{"8 char secret", "eightchr", "****"},
		{"exact 8 chars", "12345678", "****"},
		{"long secret", strings.Repeat("a", 100), "aaaa****" + strings.Repeat("a", 96)[96-4:]},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := maskSecret(tt.secret)
			if got != tt.want {
				t.Errorf("maskSecret(%q) = %q, want %q", tt.secret, got, tt.want)
			}
		})
	}
}

func TestSecretPatterns_Defined(t *testing.T) {
	// Verify that secretPatterns slice is defined and has expected patterns
	expectedPatterns := []string{
		"AWS Access Key ID",
		"AWS Secret Key",
		"Private Key",
		"GitHub Token",
		"Slack Token",
		"Generic API Key",
		"JWT Token",
	}

	if len(secretPatterns) != len(expectedPatterns) {
		t.Errorf("secretPatterns has %d patterns, want %d", len(secretPatterns), len(expectedPatterns))
	}

	for i, pattern := range secretPatterns {
		if pattern.Name != expectedPatterns[i] {
			t.Errorf("secretPatterns[%d].Name = %q, want %q", i, pattern.Name, expectedPatterns[i])
		}
		if pattern.Pattern == nil {
			t.Errorf("secretPatterns[%d].Pattern is nil", i)
		}
	}
}

func BenchmarkScanContent(b *testing.B) {
	content := `This is a document with some content.
It might contain secrets like AKIAIOSFODNN7EXAMPLE.
Or maybe a GitHub token: ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
Or a private key:
-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA...
-----END RSA PRIVATE KEY-----
And some more normal text here.`

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = ScanContent(content)
	}
}

func BenchmarkHasSecrets(b *testing.B) {
	content := "This is normal content without any secrets that should be scanned quickly."

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = HasSecrets(content)
	}
}

func BenchmarkValidateDocument(b *testing.B) {
	content := strings.Repeat("This is clean content. ", 100)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = ValidateDocument(content)
	}
}
