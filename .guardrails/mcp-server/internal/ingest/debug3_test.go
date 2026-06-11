package ingest

import (
	"fmt"
	"regexp"
	"testing"
)

func TestDebugMetadataRegex3(t *testing.T) {
	testLine := "**Pattern:** `rm -rf /`"

	fmt.Println("Input line:")
	fmt.Printf("  %q\n", testLine)
	fmt.Printf("  Length: %d\n", len(testLine))
	fmt.Printf("  Bytes: %v\n", []byte(testLine))
	fmt.Println()

	// Test individual components
	patterns := []string{
		`^\*\*`,
		`^\*\*(\w+)`,
		`^\*\*(\w+)\*\*`,
		`^\*\*(\w+)\*\*\s*:`,
		`^\*\*(\w+)\*\*\s*:\s*`,
		`^\*\*(\w+)\*\*\s*:\s*(.+)`,
		`^\*\*(\w+)\*\*\s*:\s*(.+)$`,
		`^\*\*(\w+)\*\*\s*:\s*(.+)$`,
	}

	for _, p := range patterns {
		re := regexp.MustCompile(p)
		match := re.FindString(testLine)
		fmt.Printf("Pattern %q -> %q\n", p, match)
	}

	fmt.Println()

	// Now test the actual issue - the non-greedy match
	re1 := regexp.MustCompile(`^\*\*(\w+)\*\*\s*:\s*(.+)$`)
	re2 := regexp.MustCompile(`^\*\*(\w+)\*\*\s*:\s*(.+?)$`)

	m1 := re1.FindStringSubmatch(testLine)
	m2 := re2.FindStringSubmatch(testLine)

	fmt.Printf("Greedy (.+)$    : %v\n", m1)
	fmt.Printf("Non-greedy (.+?)$: %v\n", m2)

	// The issue: $ in Go regex matches end of line with (?m), but the string
	// might have Windows line endings or other issues
	fmt.Println()
	fmt.Println("Testing line endings:")

	// Test with different line endings
	lines := []string{
		"**Pattern:** `rm -rf /`\n",
		"**Pattern:** `rm -rf /`\r\n",
		"**Pattern:** `rm -rf /`",
	}

	for i, line := range lines {
		re := regexp.MustCompile(`(?m)^\*\*(\w+)\*\*\s*:\s*(.+?)$`)
		m := re.FindStringSubmatch(line)
		fmt.Printf("Line %d (%q): %v\n", i, line, m)
	}

	// Test if the issue is with the (?m) flag and line boundaries
	fmt.Println()
	fmt.Println("Testing without (?m) on single line:")
	reNoMultiline := regexp.MustCompile(`^\*\*(\w+)\*\*\s*:\s*(.+?)$`)
	m := reNoMultiline.FindStringSubmatch(testLine)
	fmt.Printf("Result: %v\n", m)
}
