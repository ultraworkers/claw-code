package team

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

// ValidationError represents a validation error
type ValidationError struct {
	Field   string
	Message string
}

func (e *ValidationError) Error() string {
	return fmt.Sprintf("validation error for %s: %s", e.Field, e.Message)
}

// SecurityError represents a security violation
type SecurityError struct {
	Message string
}

func (e *SecurityError) Error() string {
	return fmt.Sprintf("security error: %s", e.Message)
}

// Validation constants
const (
	MaxProjectNameLength = 64
	MaxRoleNameLength    = 128
	MaxPersonNameLength  = 256
)

var (
	projectNameRegex = regexp.MustCompile(`^[a-zA-Z0-9_-]+$`)
	roleNameRegex    = regexp.MustCompile(`^[a-zA-Z0-9\s\-_/\&\(\)\.]+$`)
	personNameRegex  = regexp.MustCompile(`^[a-zA-Z\s\-'']+$`)
)

// ValidateProjectName validates a project name
func ValidateProjectName(name string) error {
	if name == "" {
		return &ValidationError{Field: "project_name", Message: "project name is required"}
	}
	if len(name) > MaxProjectNameLength {
		return &ValidationError{Field: "project_name", Message: fmt.Sprintf("project name must be %d characters or less", MaxProjectNameLength)}
	}
	if !projectNameRegex.MatchString(name) {
		return &ValidationError{Field: "project_name", Message: "project name must contain only letters, numbers, hyphens, and underscores"}
	}
	return nil
}

// ValidateProjectPath validates project path to prevent path traversal
func ValidateProjectPath(projectName, baseDir string) (string, error) {
	if baseDir == "" {
		baseDir = ".teams"
	}
	if err := ValidateProjectName(projectName); err != nil {
		return "", err
	}
	dangerousPatterns := []string{"..", "/", "\\", "\x00"}
	for _, pattern := range dangerousPatterns {
		if strings.Contains(projectName, pattern) {
			return "", &SecurityError{Message: fmt.Sprintf("path traversal detected: project_name contains forbidden pattern '%s'", pattern)}
		}
	}
	configPath := filepath.Join(baseDir, fmt.Sprintf("%s.json", projectName))
	absPath, err := filepath.Abs(configPath)
	if err != nil {
		return "", fmt.Errorf("failed to resolve path: %w", err)
	}
	baseAbs, err := filepath.Abs(baseDir)
	if err != nil {
		return "", fmt.Errorf("failed to resolve base directory: %w", err)
	}
	if !strings.HasPrefix(absPath, baseAbs+string(filepath.Separator)) && absPath != baseAbs {
		return "", &SecurityError{Message: "path traversal detected: resolved path is outside the base directory"}
	}
	return absPath, nil
}

// ValidateRoleName validates a role name
func ValidateRoleName(name string) error {
	if name == "" {
		return &ValidationError{Field: "role_name", Message: "role name is required"}
	}
	if len(name) > MaxRoleNameLength {
		return &ValidationError{Field: "role_name", Message: fmt.Sprintf("role name must be %d characters or less", MaxRoleNameLength)}
	}
	return nil
}

// ValidatePersonName validates a person name
func ValidatePersonName(name string) error {
	if name == "" {
		return &ValidationError{Field: "person", Message: "person name is required"}
	}
	if len(name) > MaxPersonNameLength {
		return &ValidationError{Field: "person", Message: fmt.Sprintf("person name must be %d characters or less", MaxPersonNameLength)}
	}
	return nil
}

// ValidatePhase validates a phase name
func ValidatePhase(phase string) error {
	validPhases := []string{
		"Phase 1: Strategy, Governance & Planning",
		"Phase 2: Platform & Foundation",
		"Phase 3: The Build Squads",
		"Phase 4: Validation & Hardening",
		"Phase 5: Delivery & Sustainment",
	}
	for _, valid := range validPhases {
		if phase == valid {
			return nil
		}
	}
	return &ValidationError{Field: "phase", Message: fmt.Sprintf("invalid phase: %s", phase)}
}

// EnsureDir ensures a directory exists
func EnsureDir(path string) error {
	return os.MkdirAll(path, 0755)
}
