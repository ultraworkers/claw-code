package validation

import (
	"fmt"
	"regexp"
	"sync"
	"time"
)

// regexCache stores compiled regex patterns to avoid recompilation
// Uses sync.Map for concurrent access without lock contention
var regexCache sync.Map

// cachedRegex wraps a compiled regex with metadata
type cachedRegex struct {
	re         *regexp.Regexp
	lastAccess time.Time
}

// getCachedRegex retrieves or compiles a regex pattern
func getCachedRegex(pattern string) (*regexp.Regexp, error) {
	// Fast path: check cache first
	if cached, ok := regexCache.Load(pattern); ok {
		cr := cached.(*cachedRegex)
		cr.lastAccess = time.Now()
		return cr.re, nil
	}

	// Slow path: compile and cache
	re, err := regexp.Compile(pattern)
	if err != nil {
		return nil, fmt.Errorf("invalid regex pattern: %w", err)
	}

	// Store in cache
	regexCache.Store(pattern, &cachedRegex{
		re:         re,
		lastAccess: time.Now(),
	})

	return re, nil
}

// SafeRegex performs regex matching with timeout protection
// Uses cached compiled patterns to avoid repeated compilation overhead
func SafeRegex(pattern string, input string, timeout time.Duration) (bool, error) {
	resultChan := make(chan bool, 1)
	panicChan := make(chan interface{}, 1)
	doneChan := make(chan struct{})

	go func() {
		defer func() {
			if r := recover(); r != nil {
				select {
				case panicChan <- r:
				case <-doneChan:
				}
			}
		}()

		// Use cached regex instead of compiling each time
		re, err := getCachedRegex(pattern)
		if err != nil {
			select {
			case resultChan <- false:
			case <-doneChan:
			}
			return
		}
		result := re.MatchString(input)
		select {
		case resultChan <- result:
		case <-doneChan:
		}
	}()

	select {
	case result := <-resultChan:
		close(doneChan)
		return result, nil
	case r := <-panicChan:
		close(doneChan)
		return false, fmt.Errorf("regex panic: %v", r)
	case <-time.After(timeout):
		close(doneChan)
		return false, fmt.Errorf("regex timeout after %v - possible ReDoS attack", timeout)
	}
}

// dangerousPatternRegex is compiled once and reused
// Matches potentially dangerous nested quantifiers
var dangerousPatternRegex = regexp.MustCompile(`\*\+|\+\*|\?\?|\{[^}]+\}\{[^}]+\}`)

// ValidatePattern checks if a regex pattern is valid and safe
func ValidatePattern(pattern string) error {
	// Check pattern length
	if len(pattern) > 10000 {
		return fmt.Errorf("pattern too long (max 10000 chars)")
	}

	// Try to compile (and cache)
	if _, err := getCachedRegex(pattern); err != nil {
		return err
	}

	// Check for potentially dangerous patterns using pre-compiled regex
	if dangerousPatternRegex.MatchString(pattern) {
		return fmt.Errorf("pattern contains potentially dangerous nested quantifiers")
	}

	// Test with simple input to ensure it works
	testInput := "test string for validation"
	_, err := SafeRegex(pattern, testInput, 100*time.Millisecond)
	if err != nil {
		return fmt.Errorf("pattern failed validation test: %w", err)
	}

	return nil
}

// MatchPattern safely matches a pattern against input
func MatchPattern(pattern string, input string) (bool, error) {
	return SafeRegex(pattern, input, 100*time.Millisecond)
}

// ClearRegexCache clears the regex cache (useful for testing or memory pressure)
func ClearRegexCache() {
	regexCache = sync.Map{}
}
