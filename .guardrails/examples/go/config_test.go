package main

import (
	"errors"
	"os"
	"path/filepath"
	"testing"
)

// testdataDir returns the path to the testdata directory.
func testdataDir() string {
	return filepath.Join(".", "testdata")
}

// TestLoadConfig_TableDriven tests LoadConfig with various environments.
func TestLoadConfig_TableDriven(t *testing.T) {
	tests := []struct {
		name        string
		env         string
		wantDBHost  string
		wantDBName  string
		wantAPIURL  string
		wantTimeout int
	}{
		{
			name:        "production environment",
			env:         "production",
			wantDBHost:  "prod-db.example.com",
			wantDBName:  "production_db",
			wantAPIURL:  "https://api.example.com",
			wantTimeout: 30,
		},
		{
			name:        "test environment",
			env:         "test",
			wantDBHost:  "localhost",
			wantDBName:  "test_db",
			wantAPIURL:  "http://localhost:8080",
			wantTimeout: 5,
		},
		{
			name:        "development environment",
			env:         "development",
			wantDBHost:  "localhost",
			wantDBName:  "dev_db",
			wantAPIURL:  "http://localhost:3000",
			wantTimeout: 10,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Set environment variable for this test
			t.Setenv("APP_ENV", tt.env)

			config, err := LoadConfig(testdataDir())
			if err != nil {
				t.Fatalf("LoadConfig() error = %v", err)
			}

			// Verify database config
			if config.Database.Host != tt.wantDBHost {
				t.Errorf("Database.Host = %v, want %v", config.Database.Host, tt.wantDBHost)
			}
			if config.Database.Name != tt.wantDBName {
				t.Errorf("Database.Name = %v, want %v", config.Database.Name, tt.wantDBName)
			}

			// Verify services config
			if config.Services.APIURL != tt.wantAPIURL {
				t.Errorf("Services.APIURL = %v, want %v", config.Services.APIURL, tt.wantAPIURL)
			}
			if config.Services.TimeoutSeconds != tt.wantTimeout {
				t.Errorf("Services.TimeoutSeconds = %v, want %v", config.Services.TimeoutSeconds, tt.wantTimeout)
			}
		})
	}
}

// TestLoadConfig_MissingEnvironment tests behavior when APP_ENV is not set.
func TestLoadConfig_MissingEnvironment(t *testing.T) {
	// Ensure APP_ENV is not set
	t.Setenv("APP_ENV", "")
	os.Unsetenv("APP_ENV")

	_, err := LoadConfig(testdataDir())
	if err == nil {
		t.Fatal("LoadConfig() expected error for missing APP_ENV")
	}

	var missingEnvErr *ErrMissingEnvironment
	if !errors.As(err, &missingEnvErr) {
		t.Errorf("expected ErrMissingEnvironment, got %T: %v", err, err)
	}
}

// TestLoadConfig_InvalidEnvironment tests behavior with invalid APP_ENV values.
func TestLoadConfig_InvalidEnvironment(t *testing.T) {
	invalidEnvs := []string{
		"staging",
		"prod",
		"dev",
		"PRODUCTION",
		"Testing",
		"invalid",
		"",
	}

	for _, env := range invalidEnvs {
		t.Run("env="+env, func(t *testing.T) {
			if env == "" {
				os.Unsetenv("APP_ENV")
			} else {
				t.Setenv("APP_ENV", env)
			}

			_, err := LoadConfig(testdataDir())
			if err == nil {
				t.Fatal("LoadConfig() expected error for invalid APP_ENV")
			}

			// Check for appropriate error type
			var invalidEnvErr *ErrInvalidEnvironment
			var missingEnvErr *ErrMissingEnvironment
			if env == "" {
				if !errors.As(err, &missingEnvErr) {
					t.Errorf("expected ErrMissingEnvironment for empty env, got %T", err)
				}
			} else {
				if !errors.As(err, &invalidEnvErr) {
					t.Errorf("expected ErrInvalidEnvironment for %q, got %T", env, err)
				}
			}
		})
	}
}

// TestLoadConfig_MissingConfigFile tests behavior when config file doesn't exist.
func TestLoadConfig_MissingConfigFile(t *testing.T) {
	t.Setenv("APP_ENV", "production")

	// Use a non-existent directory
	_, err := LoadConfig("/nonexistent/path/to/configs")
	if err == nil {
		t.Fatal("LoadConfig() expected error for missing config file")
	}

	var missingConfigErr *ErrMissingConfig
	if !errors.As(err, &missingConfigErr) {
		t.Errorf("expected ErrMissingConfig, got %T: %v", err, err)
	}
}

// TestLoadConfig_InvalidYAML tests behavior with malformed YAML.
func TestLoadConfig_InvalidYAML(t *testing.T) {
	// Create a temporary directory with invalid YAML
	tmpDir := t.TempDir()
	invalidYAML := []byte("invalid: yaml: content: [unclosed")
	if err := os.WriteFile(filepath.Join(tmpDir, "test.yaml"), invalidYAML, 0644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	t.Setenv("APP_ENV", "test")

	_, err := LoadConfig(tmpDir)
	if err == nil {
		t.Fatal("LoadConfig() expected error for invalid YAML")
	}

	var invalidConfigErr *ErrInvalidConfig
	if !errors.As(err, &invalidConfigErr) {
		t.Errorf("expected ErrInvalidConfig, got %T: %v", err, err)
	}
}

// TestLoadConfigWithEnv tests the explicit environment loading function.
func TestLoadConfigWithEnv(t *testing.T) {
	tests := []struct {
		name       string
		env        string
		wantDBName string
		wantErr    bool
		errType    interface{}
	}{
		{
			name:       "valid production",
			env:        "production",
			wantDBName: "production_db",
			wantErr:    false,
		},
		{
			name:       "valid test",
			env:        "test",
			wantDBName: "test_db",
			wantErr:    false,
		},
		{
			name:       "valid development",
			env:        "development",
			wantDBName: "dev_db",
			wantErr:    false,
		},
		{
			name:    "empty environment",
			env:     "",
			wantErr: true,
			errType: &ErrMissingEnvironment{},
		},
		{
			name:    "invalid environment",
			env:     "staging",
			wantErr: true,
			errType: &ErrInvalidEnvironment{},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			config, err := LoadConfigWithEnv(testdataDir(), tt.env)

			if tt.wantErr {
				if err == nil {
					t.Fatal("LoadConfigWithEnv() expected error")
				}
				return
			}

			if err != nil {
				t.Fatalf("LoadConfigWithEnv() error = %v", err)
			}

			if config.Database.Name != tt.wantDBName {
				t.Errorf("Database.Name = %v, want %v", config.Database.Name, tt.wantDBName)
			}
		})
	}
}

// TestDatabaseConfig_FullValidation validates all database config fields.
func TestDatabaseConfig_FullValidation(t *testing.T) {
	t.Setenv("APP_ENV", "production")

	config, err := LoadConfig(testdataDir())
	if err != nil {
		t.Fatalf("LoadConfig() error = %v", err)
	}

	db := config.Database

	// Validate all fields are populated
	if db.Host == "" {
		t.Error("Database.Host should not be empty")
	}
	if db.Port == 0 {
		t.Error("Database.Port should not be zero")
	}
	if db.Name == "" {
		t.Error("Database.Name should not be empty")
	}
	if db.SSLMode == "" {
		t.Error("Database.SSLMode should not be empty")
	}
	if db.MaxConnections == 0 {
		t.Error("Database.MaxConnections should not be zero")
	}

	// Validate specific production values
	if db.Port != 5432 {
		t.Errorf("Database.Port = %d, want 5432", db.Port)
	}
	if db.SSLMode != "require" {
		t.Errorf("Database.SSLMode = %s, want require", db.SSLMode)
	}
	if db.MaxConnections != 100 {
		t.Errorf("Database.MaxConnections = %d, want 100", db.MaxConnections)
	}
}

// TestServicesConfig_FullValidation validates all services config fields.
func TestServicesConfig_FullValidation(t *testing.T) {
	t.Setenv("APP_ENV", "production")

	config, err := LoadConfig(testdataDir())
	if err != nil {
		t.Fatalf("LoadConfig() error = %v", err)
	}

	svc := config.Services

	// Validate all fields are populated
	if svc.APIURL == "" {
		t.Error("Services.APIURL should not be empty")
	}
	if svc.CacheHost == "" {
		t.Error("Services.CacheHost should not be empty")
	}
	if svc.TimeoutSeconds == 0 {
		t.Error("Services.TimeoutSeconds should not be zero")
	}
	if svc.RetryAttempts == 0 {
		t.Error("Services.RetryAttempts should not be zero")
	}

	// Validate specific production values
	if svc.RetryAttempts != 3 {
		t.Errorf("Services.RetryAttempts = %d, want 3", svc.RetryAttempts)
	}
}

// TestEnvironmentIsolation verifies test and production configs are different.
func TestEnvironmentIsolation(t *testing.T) {
	prodConfig, err := LoadConfigWithEnv(testdataDir(), "production")
	if err != nil {
		t.Fatalf("LoadConfigWithEnv(production) error = %v", err)
	}

	testConfig, err := LoadConfigWithEnv(testdataDir(), "test")
	if err != nil {
		t.Fatalf("LoadConfigWithEnv(test) error = %v", err)
	}

	// Database hosts must be different
	if prodConfig.Database.Host == testConfig.Database.Host {
		t.Error("production and test should have different database hosts")
	}

	// Database names must be different
	if prodConfig.Database.Name == testConfig.Database.Name {
		t.Error("production and test should have different database names")
	}

	// API URLs must be different
	if prodConfig.Services.APIURL == testConfig.Services.APIURL {
		t.Error("production and test should have different API URLs")
	}

	// SSL mode should be stricter in production
	if prodConfig.Database.SSLMode == "disable" {
		t.Error("production database should not have SSL disabled")
	}
}

// TestValidEnvironments verifies the list of valid environments.
func TestValidEnvironments(t *testing.T) {
	expected := []string{"production", "test", "development"}

	if len(ValidEnvironments) != len(expected) {
		t.Errorf("ValidEnvironments length = %d, want %d", len(ValidEnvironments), len(expected))
	}

	for _, env := range expected {
		found := false
		for _, valid := range ValidEnvironments {
			if env == valid {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("ValidEnvironments should contain %q", env)
		}
	}
}

// TestErrorMessages verifies error messages are informative.
func TestErrorMessages(t *testing.T) {
	tests := []struct {
		name     string
		err      error
		contains string
	}{
		{
			name:     "missing environment",
			err:      &ErrMissingEnvironment{},
			contains: "APP_ENV",
		},
		{
			name:     "invalid environment",
			err:      &ErrInvalidEnvironment{Value: "staging"},
			contains: "staging",
		},
		{
			name:     "missing config",
			err:      &ErrMissingConfig{Path: "/path/to/config.yaml"},
			contains: "/path/to/config.yaml",
		},
		{
			name:     "invalid config",
			err:      &ErrInvalidConfig{Path: "/path/to/config.yaml", Err: errors.New("parse error")},
			contains: "parse error",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			msg := tt.err.Error()
			if msg == "" {
				t.Error("error message should not be empty")
			}
			if !contains(msg, tt.contains) {
				t.Errorf("error message %q should contain %q", msg, tt.contains)
			}
		})
	}
}

// contains checks if s contains substr.
func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(substr) == 0 ||
		(len(s) > 0 && len(substr) > 0 && findSubstring(s, substr)))
}

func findSubstring(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
