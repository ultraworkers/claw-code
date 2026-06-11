// Package main provides configuration loading functionality.
// This demonstrates guardrails-compliant patterns for environment-based config.
package main

import (
	"fmt"
	"os"
	"path/filepath"

	"gopkg.in/yaml.v3"
)

// DatabaseConfig holds database connection settings.
type DatabaseConfig struct {
	Host           string `yaml:"host"`
	Port           int    `yaml:"port"`
	Name           string `yaml:"name"`
	SSLMode        string `yaml:"ssl_mode"`
	MaxConnections int    `yaml:"max_connections"`
}

// ServicesConfig holds external service settings.
type ServicesConfig struct {
	APIURL         string `yaml:"api_url"`
	CacheHost      string `yaml:"cache_host"`
	TimeoutSeconds int    `yaml:"timeout_seconds"`
	RetryAttempts  int    `yaml:"retry_attempts"`
}

// Config is the root configuration structure.
type Config struct {
	Database DatabaseConfig `yaml:"database"`
	Services ServicesConfig `yaml:"services"`
}

// ErrMissingConfig indicates the config file was not found.
type ErrMissingConfig struct {
	Path string
}

func (e *ErrMissingConfig) Error() string {
	return fmt.Sprintf("config file not found: %s", e.Path)
}

// ErrInvalidConfig indicates the config file could not be parsed.
type ErrInvalidConfig struct {
	Path string
	Err  error
}

func (e *ErrInvalidConfig) Error() string {
	return fmt.Sprintf("invalid config file %s: %v", e.Path, e.Err)
}

func (e *ErrInvalidConfig) Unwrap() error {
	return e.Err
}

// ErrMissingEnvironment indicates APP_ENV is not set.
type ErrMissingEnvironment struct{}

func (e *ErrMissingEnvironment) Error() string {
	return "APP_ENV environment variable is not set"
}

// ErrInvalidEnvironment indicates APP_ENV has an unsupported value.
type ErrInvalidEnvironment struct {
	Value string
}

func (e *ErrInvalidEnvironment) Error() string {
	return fmt.Sprintf("invalid APP_ENV value: %s (must be production, test, or development)", e.Value)
}

// ValidEnvironments lists all supported environment names.
var ValidEnvironments = []string{"production", "test", "development"}

// isValidEnvironment checks if the given env is supported.
func isValidEnvironment(env string) bool {
	for _, valid := range ValidEnvironments {
		if env == valid {
			return true
		}
	}
	return false
}

// LoadConfig loads configuration based on the APP_ENV environment variable.
// It reads the corresponding YAML file from the configDir directory.
//
// Supported environments: production, test, development
//
// Returns:
//   - *Config: The loaded configuration
//   - error: ErrMissingEnvironment, ErrInvalidEnvironment, ErrMissingConfig, or ErrInvalidConfig
func LoadConfig(configDir string) (*Config, error) {
	env := os.Getenv("APP_ENV")
	if env == "" {
		return nil, &ErrMissingEnvironment{}
	}

	if !isValidEnvironment(env) {
		return nil, &ErrInvalidEnvironment{Value: env}
	}

	configPath := filepath.Join(configDir, env+".yaml")

	data, err := os.ReadFile(configPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, &ErrMissingConfig{Path: configPath}
		}
		return nil, &ErrInvalidConfig{Path: configPath, Err: err}
	}

	var config Config
	if err := yaml.Unmarshal(data, &config); err != nil {
		return nil, &ErrInvalidConfig{Path: configPath, Err: err}
	}

	return &config, nil
}

// LoadConfigWithEnv loads configuration for a specific environment.
// This is useful for testing or when you need to override APP_ENV.
func LoadConfigWithEnv(configDir, env string) (*Config, error) {
	if env == "" {
		return nil, &ErrMissingEnvironment{}
	}

	if !isValidEnvironment(env) {
		return nil, &ErrInvalidEnvironment{Value: env}
	}

	configPath := filepath.Join(configDir, env+".yaml")

	data, err := os.ReadFile(configPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, &ErrMissingConfig{Path: configPath}
		}
		return nil, &ErrInvalidConfig{Path: configPath, Err: err}
	}

	var config Config
	if err := yaml.Unmarshal(data, &config); err != nil {
		return nil, &ErrInvalidConfig{Path: configPath, Err: err}
	}

	return &config, nil
}
