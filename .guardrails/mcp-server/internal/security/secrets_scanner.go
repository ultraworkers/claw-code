package security

import (
	"fmt"
	"regexp"
	"strings"
)

// SecretPattern defines a detectable secret type
type SecretPattern struct {
	Name        string
	Pattern     *regexp.Regexp
	Description string
}

// Common secret patterns to detect
var secretPatterns = []SecretPattern{
	{
		Name:        "AWS Access Key ID",
		Pattern:     regexp.MustCompile(`AKIA[0-9A-Z]{16}`),
		Description: "AWS IAM access key",
	},
	{
		Name:        "AWS Secret Key",
		Pattern:     regexp.MustCompile(`['"\s][0-9a-zA-Z/+]{40}['"\s]`),
		Description: "Potential AWS secret key",
	},
	{
		Name:        "Private Key",
		Pattern:     regexp.MustCompile(`-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----`),
		Description: "PEM private key",
	},
	{
		Name:        "GitHub Token",
		Pattern:     regexp.MustCompile(`gh[pousr]_[A-Za-z0-9_]{36,}`),
		Description: "GitHub personal/token",
	},
	{
		Name:        "Slack Token",
		Pattern:     regexp.MustCompile(`xox[baprs]-[0-9a-zA-Z-]+`),
		Description: "Slack API token",
	},
	{
		Name:        "Generic API Key",
		Pattern:     regexp.MustCompile(`(?i)(api[_-]?key|apikey)\s*[=:]\s*['"\s][a-z0-9_\-]{16,}['"\s]`),
		Description: "Generic API key pattern",
	},
	{
		Name:        "JWT Token",
		Pattern:     regexp.MustCompile(`eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*`),
		Description: "JSON Web Token",
	},
}

// ScanResult represents a detected secret
type ScanResult struct {
	Pattern     string `json:"pattern"`
	Line        int    `json:"line"`
	Column      int    `json:"column"`
	Match       string `json:"match"`
	Description string `json:"description"`
}

// ScanContent checks text for embedded secrets
func ScanContent(content string) []ScanResult {
	var results []ScanResult
	lines := strings.Split(content, "\n")

	for lineNum, line := range lines {
		for _, pattern := range secretPatterns {
			matches := pattern.Pattern.FindAllStringIndex(line, -1)
			for _, match := range matches {
				results = append(results, ScanResult{
					Pattern:     pattern.Name,
					Line:        lineNum + 1,
					Column:      match[0] + 1,
					Match:       maskSecret(line[match[0]:match[1]]),
					Description: pattern.Description,
				})
			}
		}
	}

	return results
}

// maskSecret hides most of the secret for display
func maskSecret(secret string) string {
	if len(secret) <= 8 {
		return "****"
	}
	return secret[:4] + "****" + secret[len(secret)-4:]
}

// ValidateDocument checks document content before storage
func ValidateDocument(content string) error {
	secrets := ScanContent(content)
	if len(secrets) > 0 {
		return fmt.Errorf("potential secrets detected: %d findings", len(secrets))
	}
	return nil
}

// HasSecrets returns true if content contains potential secrets
func HasSecrets(content string) bool {
	return len(ScanContent(content)) > 0
}
