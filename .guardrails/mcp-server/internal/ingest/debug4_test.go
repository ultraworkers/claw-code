package ingest

import (
	"fmt"
	"regexp"
	"testing"
)

func TestDebugMetadataRegex4(t *testing.T) {
	testLine := "**Pattern:** `rm -rf /`"

	fmt.Println("Input line:")
	fmt.Printf("  %q\n", testLine)
	fmt.Println()

	// Current regex (from rule_parser.go line 32)
	currentRegex := regexp.MustCompile(`(?m)^\*\*(\w+)\*\*\s*:\s*(.+?)$`)
	fmt.Println("Current regex: (?m)^\\*\\*(\\w+)\\*\\*\\s*:\\s*(.+?)$")
	fmt.Printf("  Matches: %v\n", currentRegex.FindStringSubmatch(testLine))
	fmt.Println()

	// Fixed regex - the : is INSIDE the **, not outside
	fixedRegex := regexp.MustCompile(`(?m)^\*\*(\w+):\*\*\s*(.+?)$`)
	fmt.Println("Fixed regex: (?m)^\\*\\*(\\w+):\\*\\*\\s*(.+?)$")
	fmt.Printf("  Matches: %v\n", fixedRegex.FindStringSubmatch(testLine))
	fmt.Println()

	// Alternative fix - make the : optional position
	flexibleRegex := regexp.MustCompile(`(?m)^\*\*(\w+)\*\*:\s*(.+?)$`)
	fmt.Println("Flexible regex: (?m)^\\*\\*(\\w+)\\*\\*:\\s*(.+?)$")
	fmt.Printf("  Matches: %v\n", flexibleRegex.FindStringSubmatch(testLine))
	fmt.Println()

	// Test with full content
	content := "**Pattern:** `rm -rf /`\n**Message:** Test message\n**Severity:** error"

	fmt.Println("Testing with full content:")
	fmt.Printf("  Current regex found: %d matches\n", len(currentRegex.FindAllStringSubmatch(content, -1)))
	fmt.Printf("  Fixed regex found: %d matches\n", len(fixedRegex.FindAllStringSubmatch(content, -1)))
}
