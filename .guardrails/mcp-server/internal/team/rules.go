// Package team provides team management functionality with optional encryption.
package team

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// RulesLoader loads and manages rules from JSON configuration file.
// Provides dynamic loading of validation rules, team size limits,
// phase gates, and other configurable settings.
type RulesLoader struct {
	rulesPath string
	rules     map[string]interface{}
}

// DefaultRulesPath is the default path to the rules.json file.
const DefaultRulesPath = ".teams/rules.json"

// NewRulesLoader creates a new RulesLoader and loads rules from the specified path.
// If rulesPath is empty, it uses the default path (.teams/rules.json).
// Falls back to default rules if the file doesn't exist or can't be loaded.
func NewRulesLoader(rulesPath string) *RulesLoader {
	if rulesPath == "" {
		rulesPath = DefaultRulesPath
	}

	rl := &RulesLoader{
		rulesPath: rulesPath,
		rules:     make(map[string]interface{}),
	}
	rl.loadRules()
	return rl
}

// loadRules loads rules from the JSON file or uses defaults.
func (rl *RulesLoader) loadRules() {
	// Check if file exists
	if _, err := os.Stat(rl.rulesPath); err == nil {
		data, err := os.ReadFile(rl.rulesPath)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Failed to read rules from %s: %v\n", rl.rulesPath, err)
			rl.rules = rl.getDefaultRules()
			return
		}

		var loadedRules map[string]interface{}
		if err := json.Unmarshal(data, &loadedRules); err != nil {
			fmt.Fprintf(os.Stderr, "Failed to parse rules from %s: %v\n", rl.rulesPath, err)
			rl.rules = rl.getDefaultRules()
			return
		}

		rl.rules = loadedRules
	} else {
		// File doesn't exist, use defaults
		rl.rules = rl.getDefaultRules()
	}
}

// getDefaultRules returns default rules when rules.json is not available.
func (rl *RulesLoader) getDefaultRules() map[string]interface{} {
	return map[string]interface{}{
		"team_size_limits": map[string]interface{}{
			"min": 4,
			"max": 6,
		},
		"duplicate_detection": map[string]interface{}{
			"enabled": true,
			"scope":   "project",
			"action":  "warn",
		},
		"phase_gates": map[string]interface{}{},
		"allowed_agent_types": []interface{}{
			"planner", "coder", "reviewer", "security", "tester", "ops",
		},
		"validation_rules": map[string]interface{}{
			"person_name": map[string]interface{}{
				"max_length":       256,
				"email_pattern":    "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$",
				"username_pattern": "^[a-zA-Z0-9_.-]+$",
			},
			"role_name": map[string]interface{}{
				"max_length": 128,
			},
			"project_name": map[string]interface{}{
				"max_length": 64,
				"pattern":    "^[a-zA-Z0-9_-]+$",
			},
		},
	}
}

// ReloadRules reloads rules from disk.
func (rl *RulesLoader) ReloadRules() {
	rl.loadRules()
}

// Get retrieves a rule by key path (e.g., "team_size_limits.min").
// Returns the default value if the key is not found or if any part of the path is missing.
func (rl *RulesLoader) Get(key string, defaultValue interface{}) interface{} {
	keys := strings.Split(key, ".")
	value := rl.rules

	for i, k := range keys {
		if nextValue, ok := value[k]; ok {
			// If this is the last key, return the value
			if i == len(keys)-1 {
				return nextValue
			}
			// Otherwise, try to navigate deeper
			if nextMap, ok := nextValue.(map[string]interface{}); ok {
				value = nextMap
			} else {
				// Can't navigate deeper, return default
				return defaultValue
			}
		} else {
			// Key not found, return default
			return defaultValue
		}
	}

	return defaultValue
}

// GetTeamSizeLimits returns the min and max team size limits.
// Returns defaults (4, 6) if the limits are not configured.
func (rl *RulesLoader) GetTeamSizeLimits() (min, max int) {
	limits, ok := rl.rules["team_size_limits"].(map[string]interface{})
	if !ok {
		return 4, 6
	}

	minVal, ok := limits["min"].(float64)
	if !ok {
		minVal = 4
	}

	maxVal, ok := limits["max"].(float64)
	if !ok {
		maxVal = 6
	}

	return int(minVal), int(maxVal)
}

// GetDuplicateDetectionConfig returns the duplicate detection configuration.
// Returns default config if not found.
func (rl *RulesLoader) GetDuplicateDetectionConfig() map[string]interface{} {
	config, ok := rl.rules["duplicate_detection"].(map[string]interface{})
	if !ok {
		return map[string]interface{}{
			"enabled": true,
			"scope":   "project",
			"action":  "warn",
		}
	}
	return config
}

// GetValidationPattern returns a validation regex pattern.
// Returns empty string if the pattern is not found.
func (rl *RulesLoader) GetValidationPattern(ruleType, patternName string) string {
	validationRules, ok := rl.rules["validation_rules"].(map[string]interface{})
	if !ok {
		return ""
	}

	ruleTypeMap, ok := validationRules[ruleType].(map[string]interface{})
	if !ok {
		return ""
	}

	pattern, ok := ruleTypeMap[patternName].(string)
	if !ok {
		return ""
	}

	return pattern
}

// Rules returns all loaded rules.
func (rl *RulesLoader) Rules() map[string]interface{} {
	return rl.rules
}

// RulesPath returns the path to the rules file.
func (rl *RulesLoader) RulesPath() string {
	return rl.rulesPath
}

// SetRulesPath updates the rules path and reloads rules.
func (rl *RulesLoader) SetRulesPath(path string) error {
	// Ensure the directory exists
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("failed to create rules directory: %w", err)
	}

	rl.rulesPath = path
	rl.ReloadRules()
	return nil
}

// SaveRules saves the current rules to the rules file.
func (rl *RulesLoader) SaveRules() error {
	data, err := json.MarshalIndent(rl.rules, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal rules: %w", err)
	}

	if err := os.WriteFile(rl.rulesPath, data, 0644); err != nil {
		return fmt.Errorf("failed to write rules file: %w", err)
	}

	return nil
}
