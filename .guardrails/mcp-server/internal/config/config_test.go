package config

import (
	"strings"
	"testing"
	"time"
)

func TestValidateJWTSecret(t *testing.T) {
	tests := []struct {
		name    string
		secret  string
		wantErr bool
		errMsg  string
	}{
		{
			name:    "valid secret - 32 bytes random",
			secret:  "abcdefghijklmnopqrstuvwxyz123456",
			wantErr: false,
		},
		{
			name:    "valid secret - longer than 32",
			secret:  "abcdefghijklmnopqrstuvwxyz1234567890abcdef",
			wantErr: false,
		},
		{
			name:    "too short - 31 bytes",
			secret:  "abcdefghijklmnopqrstuvwxyz12345",
			wantErr: true,
			errMsg:  "JWT_SECRET must be at least 32 bytes",
		},
		{
			name:    "too short - empty",
			secret:  "",
			wantErr: true,
			errMsg:  "JWT_SECRET must be at least 32 bytes",
		},
		{
			name:    "low entropy - all same char",
			secret:  strings.Repeat("a", 32),
			wantErr: true,
			errMsg:  "insufficient entropy",
		},
		{
			name:    "low entropy - repeating pattern",
			secret:  "abababababababababababababababab",
			wantErr: true,
			errMsg:  "insufficient entropy",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := ValidateJWTSecret(tt.secret)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateJWTSecret() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if tt.wantErr && err != nil && tt.errMsg != "" {
				if !strings.Contains(err.Error(), tt.errMsg) {
					t.Errorf("ValidateJWTSecret() error message = %v, want containing %v", err.Error(), tt.errMsg)
				}
			}
		})
	}
}

func TestValidateAPIKey(t *testing.T) {
	tests := []struct {
		name    string
		key     string
		keyName string
		wantErr bool
		errMsg  string
	}{
		{
			name:    "valid key - 32 chars mixed",
			key:     "AbCdEfGhIjKlMnOpQrStUvWxYz123456",
			keyName: "TEST_API_KEY",
			wantErr: false,
		},
		{
			name:    "valid key - longer than 32",
			key:     "AbCdEfGhIjKlMnOpQrStUvWxYz1234567890abcdef",
			keyName: "TEST_API_KEY",
			wantErr: false,
		},
		{
			name:    "too short - 31 chars",
			key:     "AbCdEfGhIjKlMnOpQrStUvWxYz12345",
			keyName: "TEST_API_KEY",
			wantErr: true,
			errMsg:  "must be at least 32 characters",
		},
		{
			name:    "too short - empty",
			key:     "",
			keyName: "TEST_API_KEY",
			wantErr: true,
			errMsg:  "must be at least 32 characters",
		},
		{
			name:    "weak key - only letters",
			key:     strings.Repeat("a", 32),
			keyName: "TEST_API_KEY",
			wantErr: true,
			errMsg:  "appears to be weak",
		},
		{
			name:    "weak key - only numbers",
			key:     strings.Repeat("1", 32),
			keyName: "TEST_API_KEY",
			wantErr: true,
			errMsg:  "appears to be weak",
		},
		{
			name:    "weak key - starts with password",
			key:     "password123456789012345678901234",
			keyName: "TEST_API_KEY",
			wantErr: true,
			errMsg:  "appears to be weak",
		},
		{
			name:    "weak key - no uppercase",
			key:     "abcdefghijklmnopqrstuvwxyz123456",
			keyName: "TEST_API_KEY",
			wantErr: true,
			errMsg:  "should contain a mix",
		},
		{
			name:    "weak key - no lowercase",
			key:     "ABCDEFGHIJKLMNOPQRSTUVWXYZ123456",
			keyName: "TEST_API_KEY",
			wantErr: true,
			errMsg:  "should contain a mix",
		},
		{
			name:    "weak key - no digits",
			key:     "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdef",
			keyName: "TEST_API_KEY",
			wantErr: true,
			errMsg:  "appears to be weak",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := ValidateAPIKey(tt.key, tt.keyName)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateAPIKey() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if tt.wantErr && err != nil && tt.errMsg != "" {
				if !strings.Contains(err.Error(), tt.errMsg) {
					t.Errorf("ValidateAPIKey() error message = %v, want containing %v", err.Error(), tt.errMsg)
				}
			}
		})
	}
}

func TestValidateTimeout(t *testing.T) {
	tests := []struct {
		name    string
		value   time.Duration
		min     time.Duration
		max     time.Duration
		wantErr bool
		errMsg  string
	}{
		{
			name:    "valid timeout - middle of range",
			value:   30 * time.Second,
			min:     5 * time.Second,
			max:     60 * time.Second,
			wantErr: false,
		},
		{
			name:    "valid timeout - at min",
			value:   5 * time.Second,
			min:     5 * time.Second,
			max:     60 * time.Second,
			wantErr: false,
		},
		{
			name:    "valid timeout - at max",
			value:   60 * time.Second,
			min:     5 * time.Second,
			max:     60 * time.Second,
			wantErr: false,
		},
		{
			name:    "too short",
			value:   1 * time.Second,
			min:     5 * time.Second,
			max:     60 * time.Second,
			wantErr: true,
			errMsg:  "must be at least",
		},
		{
			name:    "too long",
			value:   120 * time.Second,
			min:     5 * time.Second,
			max:     60 * time.Second,
			wantErr: true,
			errMsg:  "must be at most",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := ValidateTimeout("TEST_TIMEOUT", tt.value, tt.min, tt.max)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateTimeout() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if tt.wantErr && err != nil && tt.errMsg != "" {
				if !strings.Contains(err.Error(), tt.errMsg) {
					t.Errorf("ValidateTimeout() error message = %v, want containing %v", err.Error(), tt.errMsg)
				}
			}
		})
	}
}

func TestIsHotReloadable(t *testing.T) {
	tests := []struct {
		name string
		key  string
		want bool
	}{
		{"LOG_LEVEL", "LOG_LEVEL", true},
		{"RATE_LIMIT_MCP", "RATE_LIMIT_MCP", true},
		{"RATE_LIMIT_IDE", "RATE_LIMIT_IDE", true},
		{"ENABLE_VALIDATION", "ENABLE_VALIDATION", true},
		{"ENABLE_METRICS", "ENABLE_METRICS", true},
		{"CACHE_TTL_RULES", "CACHE_TTL_RULES", true},
		{"non-existent key", "RANDOM_KEY", false},
		{"empty key", "", false},
		{"DB_HOST", "DB_HOST", false},
		{"JWT_SECRET", "JWT_SECRET", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := IsHotReloadable(tt.key)
			if got != tt.want {
				t.Errorf("IsHotReloadable(%q) = %v, want %v", tt.key, got, tt.want)
			}
		})
	}
}

func TestHotReloadableFields(t *testing.T) {
	fields := HotReloadableFields()

	// Should return a non-empty slice
	if len(fields) == 0 {
		t.Error("HotReloadableFields() returned empty slice")
	}

	// All returned fields should be hot-reloadable
	for _, field := range fields {
		if !IsHotReloadable(field) {
			t.Errorf("Field %q from HotReloadableFields() is not hot-reloadable", field)
		}
	}
}

func TestConfig_Masked(t *testing.T) {
	cfg := &Config{
		DBPassword:    "secret-db-password",
		RedisPassword: "secret-redis-password",
		MCPAPIKey:     "secret-mcp-key",
		IDEAPIKey:     "secret-ide-key",
		JWTSecret:     "secret-jwt-secret",
		DBHost:        "localhost",
		DBPort:        5432,
	}

	masked := cfg.Masked()

	// Sensitive fields should be masked
	if masked.DBPassword != "***" {
		t.Errorf("Masked DBPassword = %q, want ***", masked.DBPassword)
	}
	if masked.RedisPassword != "***" {
		t.Errorf("Masked RedisPassword = %q, want ***", masked.RedisPassword)
	}
	if masked.MCPAPIKey != "***" {
		t.Errorf("Masked MCPAPIKey = %q, want ***", masked.MCPAPIKey)
	}
	if masked.IDEAPIKey != "***" {
		t.Errorf("Masked IDEAPIKey = %q, want ***", masked.IDEAPIKey)
	}
	if masked.JWTSecret != "***" {
		t.Errorf("Masked JWTSecret = %q, want ***", masked.JWTSecret)
	}

	// Non-sensitive fields should remain unchanged
	if masked.DBHost != "localhost" {
		t.Errorf("Masked DBHost = %q, want localhost", masked.DBHost)
	}
	if masked.DBPort != 5432 {
		t.Errorf("Masked DBPort = %d, want 5432", masked.DBPort)
	}
}

func TestConfig_DatabaseURL(t *testing.T) {
	cfg := &Config{
		DBUser:           "testuser",
		DBPassword:       "testpass",
		DBHost:           "localhost",
		DBPort:           5432,
		DBName:           "testdb",
		DBSSLMode:        "require",
		DBConnectTimeout: 10 * time.Second,
	}

	url := cfg.DatabaseURL()
	expected := "postgresql://testuser:testpass@localhost:5432/testdb?sslmode=require&connect_timeout=10"

	if url != expected {
		t.Errorf("DatabaseURL() = %q, want %q", url, expected)
	}
}

func TestConfig_RedisAddr(t *testing.T) {
	cfg := &Config{
		RedisHost: "localhost",
		RedisPort: 6379,
	}

	addr := cfg.RedisAddr()
	expected := "localhost:6379"

	if addr != expected {
		t.Errorf("RedisAddr() = %q, want %q", addr, expected)
	}
}

func BenchmarkValidateJWTSecret(b *testing.B) {
	secret := "abcdefghijklmnopqrstuvwxyz123456"

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = ValidateJWTSecret(secret)
	}
}

func BenchmarkValidateAPIKey(b *testing.B) {
	key := "AbCdEfGhIjKlMnOpQrStUvWxYz123456"

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = ValidateAPIKey(key, "TEST_API_KEY")
	}
}

func BenchmarkIsHotReloadable(b *testing.B) {
	keys := []string{"LOG_LEVEL", "DB_HOST", "RATE_LIMIT_MCP", "RANDOM_KEY"}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for _, key := range keys {
			_ = IsHotReloadable(key)
		}
	}
}
