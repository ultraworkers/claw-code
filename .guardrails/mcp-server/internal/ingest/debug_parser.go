package ingest

import (
	"fmt"
	"regexp"
)

// DebugRuleParser tests the rule parser regex patterns
func DebugRuleParser() {
	// Sample markdown content matching the expected format
	sampleContent := `## PREVENT-001: Prevent rm -rf / disasters

**Pattern:** ` + "`rm -rf /`" + `
**Message:** Detected dangerous command: rm -rf /
**Severity:** error
**Category:** bash

This rule prevents catastrophic deletion of the root filesystem.

---

## PREVENT-002: Prevent hardcoded secrets

**Pattern:** ` + "`password.*=.*\"[^\"]+\"`" + `
**Message:** Potential hardcoded password detected
**Severity:** warning
**Category:** security

Detects potential hardcoded passwords in code.
`

	// Initialize regex patterns (same as NewRuleParser)
	ruleHeaderRegex := regexp.MustCompile(`(?m)^##\s+(PREVENT-\d+)\s*:\s*(.+)$`)
	metadataRegex := regexp.MustCompile(`(?m)^\*\*(\w+)\*\*\s*:\s*(.+?)$`)
	backtickRegex := regexp.MustCompile("`([^`]+)`")

	fmt.Println("=== Debugging Rule Parser ===")
	fmt.Println()

	// Test 1: Rule header regex
	fmt.Println("1. Testing ruleHeaderRegex:")
	headerMatches := ruleHeaderRegex.FindAllStringSubmatchIndex(sampleContent, -1)
	fmt.Printf("   Found %d rule headers\n", len(headerMatches))
	for i, match := range headerMatches {
		start, end := match[0], match[1]
		fmt.Printf("   Rule %d: %q\n", i+1, sampleContent[start:end])
	}
	fmt.Println()

	// Test 2: Test full content parsing
	fmt.Println("2. Testing full ParseRuleContent:")
	parser := NewRuleParser()
	rules, err := parser.ParseRuleContent(sampleContent, "test.md")
	if err != nil {
		fmt.Printf("   Error: %v\n", err)
	} else {
		fmt.Printf("   Parsed %d rules\n", len(rules))
		for _, r := range rules {
			fmt.Printf("   - %s: %s (pattern: %q)\n", r.RuleID, r.Name, r.Pattern)
		}
	}
	fmt.Println()

	// Test 3: Test metadata regex
	fmt.Println("3. Testing metadataRegex:")
	metaMatches := metadataRegex.FindAllStringSubmatch(sampleContent, -1)
	fmt.Printf("   Found %d metadata entries\n", len(metaMatches))
	for _, match := range metaMatches {
		if len(match) >= 3 {
			fmt.Printf("   - %s: %s\n", match[1], match[2])
		}
	}
	fmt.Println()

	// Test 4: Test backtick extraction
	fmt.Println("4. Testing backtickRegex:")
	testCases := []string{
		"`rm -rf /`",
		"Some text `pattern-here` more text",
		"No backticks here",
	}
	for _, tc := range testCases {
		match := backtickRegex.FindStringSubmatch(tc)
		if len(match) >= 2 {
			fmt.Printf("   %q -> %q\n", tc, match[1])
		} else {
			fmt.Printf("   %q -> (no match, returning: %q)\n", tc, tc)
		}
	}
	fmt.Println()

	// Test 5: Show rule sections
	fmt.Println("5. Rule sections found:")
	matches := ruleHeaderRegex.FindAllStringIndex(sampleContent, -1)
	for i, match := range matches {
		start := match[0]
		end := len(sampleContent)
		if i < len(matches)-1 {
			end = matches[i+1][0]
		}
		section := sampleContent[start:end]
		fmt.Printf("   Section %d:\n%s\n---\n", i+1, section)
	}
}
