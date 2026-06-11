package ingest

import (
	"fmt"
	"regexp"
	"testing"
)

func TestDebugMetadataRegex(t *testing.T) {
	// Test line by line
	testLine := "**Pattern:** `rm -rf /`"

	// Current regex from rule_parser.go line 32
	metadataRegex := regexp.MustCompile(`(?m)^\*\*(\w+)\*\*\s*:\s*(.+?)$`)

	fmt.Println("Testing metadata regex on single line:")
	fmt.Printf("  Input: %q\n", testLine)

	matches := metadataRegex.FindStringSubmatch(testLine)
	if matches == nil {
		fmt.Println("  NO MATCH!")

		// Try debugging step by step
		fmt.Println("\n  Debugging:")

		// Test without ^ anchor
		test1 := regexp.MustCompile(`\*\*(\w+)\*\*\s*:\s*(.+)`)
		if m := test1.FindStringSubmatch(testLine); m != nil {
			fmt.Printf("  - Without ^ anchor: %v\n", m)
		}

		// Test with just the key part
		test2 := regexp.MustCompile(`\*\*(\w+)\*\*`)
		if m := test2.FindStringSubmatch(testLine); m != nil {
			fmt.Printf("  - Just key pattern: %v\n", m)
		}

		// Test with colon and space
		test3 := regexp.MustCompile(`\*\*(\w+)\*\*\s*:`)
		if m := test3.FindStringSubmatch(testLine); m != nil {
			fmt.Printf("  - Key with colon: %v\n", m)
		}

		// Test value part
		test4 := regexp.MustCompile(`:\s*(.+)`)
		if m := test4.FindStringSubmatch(testLine); m != nil {
			fmt.Printf("  - Value part: %v\n", m)
		}

		// The issue might be with the value part and backticks
		test5 := regexp.MustCompile(`:\s*` + "`" + `([^` + "`" + `]+)` + "`" + `$`)
		if m := test5.FindStringSubmatch(testLine); m != nil {
			fmt.Printf("  - With backticks: %v\n", m)
		}

	} else {
		fmt.Printf("  Match: %v\n", matches)
	}

	// Test with full content
	fmt.Println("\n\nTesting with full content:")
	content := `## PREVENT-001: Test Rule

**Pattern:** ` + "`rm -rf /`" + `
**Message:** Test message
**Severity:** error
**Category:** bash
`
	allMatches := metadataRegex.FindAllStringSubmatch(content, -1)
	fmt.Printf("  Found %d matches\n", len(allMatches))
	for i, m := range allMatches {
		fmt.Printf("  Match %d: %v\n", i, m)
	}

	// Test with simpler regex
	fmt.Println("\n\nTesting with simpler regex:")
	simplerRegex := regexp.MustCompile(`(?m)^\*\*(\w+)\*\*:\s*(.+)$`)
	simpleMatches := simplerRegex.FindAllStringSubmatch(content, -1)
	fmt.Printf("  Found %d matches\n", len(simpleMatches))
	for i, m := range simpleMatches {
		fmt.Printf("  Match %d: %v\n", i, m)
	}
}
