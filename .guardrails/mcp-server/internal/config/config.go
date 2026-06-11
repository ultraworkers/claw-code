package config

import (
	"fmt"
	"math/bits"
	"regexp"
	"time"

	"github.com/caarlos0/env/v11"
)

// SchemaVersion tracks the configuration schema version for migrations
const SchemaVersion = "1.0"

// Config holds all application configuration
type Config struct {
	// Schema Version (for config migration tracking)
	SchemaVersion string `env:"CONFIG_SCHEMA_VERSION" envDefault:"1.0"`

	// Server Configuration
	MCPPort        int           `env:"MCP_PORT" envDefault:"8080"`
	LogLevel       string        `env:"LOG_LEVEL" envDefault:"info"`
	RequestTimeout time.Duration `env:"REQUEST_TIMEOUT" envDefault:"30s"`

	// Graceful Shutdown Configuration
	ShutdownTimeout time.Duration `env:"SHUTDOWN_TIMEOUT" envDefault:"30s"`

	// Web UI Configuration
	WebPort    int  `env:"WEB_PORT" envDefault:"8081"`
	WebEnabled bool `env:"WEB_ENABLED" envDefault:"true"`

	// CORS Configuration
	CORSAllowedOrigins []string `env:"CORS_ALLOWED_ORIGINS" envDefault:"*"`
	CORSAllowedMethods []string `env:"CORS_ALLOWED_METHODS" envDefault:"GET,POST,PUT,DELETE,OPTIONS"`
	CORSAllowedHeaders []string `env:"CORS_ALLOWED_HEADERS" envDefault:"Authorization,Content-Type,X-Request-ID"`
	CORSMaxAge         int      `env:"CORS_MAX_AGE" envDefault:"86400"`

	// Profiling Configuration
	PProfEnabled bool `env:"PPROF_ENABLED" envDefault:"false"`
	PProfPort    int  `env:"PPROF_PORT" envDefault:"6060"`

	// Health Check Configuration
	HealthCheckTimeout time.Duration `env:"HEALTH_CHECK_TIMEOUT" envDefault:"3s"`

	// Database Configuration
	DBHost            string        `env:"DB_HOST" envDefault:"localhost"`
	DBPort            int           `env:"DB_PORT" envDefault:"5432"`
	DBName            string        `env:"DB_NAME" envDefault:"guardrails"`
	DBUser            string        `env:"DB_USER,required"`
	DBPassword        string        `env:"DB_PASSWORD,required"`
	DBSSLMode         string        `env:"DB_SSLMODE" envDefault:"require"`
	DBConnectTimeout  time.Duration `env:"DB_CONNECT_TIMEOUT" envDefault:"10s"`
	DBMaxOpenConns    int           `env:"DB_MAX_OPEN_CONNS" envDefault:"25"`
	DBMaxIdleConns    int           `env:"DB_MAX_IDLE_CONNS" envDefault:"5"`
	DBConnMaxLifetime time.Duration `env:"DB_CONN_MAX_LIFETIME" envDefault:"30m"`
	DBConnMaxIdleTime time.Duration `env:"DB_CONN_MAX_IDLE_TIME" envDefault:"10m"`

	// Redis Configuration
	RedisHost         string        `env:"REDIS_HOST" envDefault:"localhost"`
	RedisPort         int           `env:"REDIS_PORT" envDefault:"6379"`
	RedisPassword     string        `env:"REDIS_PASSWORD"`
	RedisUseTLS       bool          `env:"REDIS_USE_TLS" envDefault:"false"`
	RedisDB           int           `env:"REDIS_DB" envDefault:"0"`
	RedisPoolSize     int           `env:"REDIS_POOL_SIZE" envDefault:"10"`
	RedisMinIdleConns int           `env:"REDIS_MIN_IDLE_CONNS" envDefault:"2"`
	RedisMaxRetries   int           `env:"REDIS_MAX_RETRIES" envDefault:"3"`
	RedisDialTimeout  time.Duration `env:"REDIS_DIAL_TIMEOUT" envDefault:"5s"`
	RedisReadTimeout  time.Duration `env:"REDIS_READ_TIMEOUT" envDefault:"3s"`

	// TLS Configuration
	TLSEnabled    bool   `env:"TLS_ENABLED" envDefault:"false"`
	TLSCertPath   string `env:"TLS_CERT_PATH"`
	TLSKeyPath    string `env:"TLS_KEY_PATH"`
	TLSCAPath     string `env:"TLS_CA_PATH"`
	TLSMinVersion string `env:"TLS_MIN_VERSION" envDefault:"1.3"`

	// Security Configuration
	MCPAPIKey string `env:"MCP_API_KEY,required"`
	IDEAPIKey string `env:"IDE_API_KEY,required"`

	// JWT Configuration
	JWTSecret        string        `env:"JWT_SECRET,required"`
	JWTIssuer        string        `env:"JWT_ISSUER" envDefault:"guardrail-mcp"`
	JWTExpiry        time.Duration `env:"JWT_EXPIRY" envDefault:"15m"`
	JWTRotationHours time.Duration `env:"JWT_ROTATION_HOURS" envDefault:"168h"` // 7 days

	// Rate Limiting Configuration (prefixed consistently)
	RateLimitMCP         int           `env:"RATE_LIMIT_MCP" envDefault:"1000"`
	RateLimitIDE         int           `env:"RATE_LIMIT_IDE" envDefault:"500"`
	RateLimitSession     int           `env:"RATE_LIMIT_SESSION" envDefault:"100"`
	RateLimitWindow      time.Duration `env:"RATE_LIMIT_WINDOW" envDefault:"1m"`
	RateLimitBurstFactor float64       `env:"RATE_LIMIT_BURST_FACTOR" envDefault:"1.5"`

	// Cache TTL Configuration
	CacheTTLRules  time.Duration `env:"CACHE_TTL_RULES" envDefault:"5m"`
	CacheTTLDocs   time.Duration `env:"CACHE_TTL_DOCS" envDefault:"10m"`
	CacheTTLSearch time.Duration `env:"CACHE_TTL_SEARCH" envDefault:"2m"`

	// Feature Flags (hot-reloadable)
	EnableValidation   bool `env:"ENABLE_VALIDATION" envDefault:"true"`
	EnableMetrics      bool `env:"ENABLE_METRICS" envDefault:"true"`
	EnableAuditLogging bool `env:"ENABLE_AUDIT_LOGGING" envDefault:"true"`
	EnableCache        bool `env:"ENABLE_CACHE" envDefault:"true"`

	// Audit Logging Configuration
	AuditBufferSize    int           `env:"AUDIT_BUFFER_SIZE" envDefault:"1000"`
	AuditFlushInterval time.Duration `env:"AUDIT_FLUSH_INTERVAL" envDefault:"5s"`

	// Circuit Breaker Configuration
	CircuitBreakerEnabled          bool          `env:"CIRCUIT_BREAKER_ENABLED" envDefault:"true"`
	CircuitBreakerFailureThreshold int           `env:"CIRCUIT_BREAKER_FAILURE_THRESHOLD" envDefault:"5"`
	CircuitBreakerSuccessThreshold int           `env:"CIRCUIT_BREAKER_SUCCESS_THRESHOLD" envDefault:"2"`
	CircuitBreakerTimeout          time.Duration `env:"CIRCUIT_BREAKER_TIMEOUT" envDefault:"30s"`
	CircuitBreakerMaxRequests      int           `env:"CIRCUIT_BREAKER_MAX_REQUESTS" envDefault:"3"`
	CircuitBreakerInterval         time.Duration `env:"CIRCUIT_BREAKER_INTERVAL" envDefault:"10s"`

	// Production Mode Indicator
	ProductionMode bool `env:"PRODUCTION_MODE" envDefault:"false"`
}

// Load reads configuration from environment variables
func Load() (*Config, error) {
	var cfg Config
	if err := env.Parse(&cfg); err != nil {
		return nil, fmt.Errorf("failed to parse config: %w", err)
	}

	// Validate configuration
	if err := cfg.Validate(); err != nil {
		return nil, err
	}

	return &cfg, nil
}

// Validate performs comprehensive configuration validation
func (c *Config) Validate() error {
	// Validate JWT secret
	if err := ValidateJWTSecret(c.JWTSecret); err != nil {
		return fmt.Errorf("JWT_SECRET validation failed: %w", err)
	}

	// Validate API keys
	if err := ValidateAPIKey(c.MCPAPIKey, "MCP_API_KEY"); err != nil {
		return err
	}
	if err := ValidateAPIKey(c.IDEAPIKey, "IDE_API_KEY"); err != nil {
		return err
	}

	// Validate timeouts
	if err := ValidateTimeout("SHUTDOWN_TIMEOUT", c.ShutdownTimeout, 5*time.Second, 5*time.Minute); err != nil {
		return err
	}
	if err := ValidateTimeout("REQUEST_TIMEOUT", c.RequestTimeout, 1*time.Second, 5*time.Minute); err != nil {
		return err
	}
	if err := ValidateTimeout("DB_CONNECT_TIMEOUT", c.DBConnectTimeout, 1*time.Second, 2*time.Minute); err != nil {
		return err
	}

	// Validate database connection pool
	if c.DBMaxOpenConns < 1 {
		return fmt.Errorf("DB_MAX_OPEN_CONNS must be at least 1, got %d", c.DBMaxOpenConns)
	}
	if c.DBMaxOpenConns > 1000 {
		return fmt.Errorf("DB_MAX_OPEN_CONNS must be at most 1000, got %d", c.DBMaxOpenConns)
	}
	if c.DBMaxIdleConns < 0 {
		return fmt.Errorf("DB_MAX_IDLE_CONNS must be non-negative, got %d", c.DBMaxIdleConns)
	}
	if c.DBMaxIdleConns > c.DBMaxOpenConns {
		return fmt.Errorf("DB_MAX_IDLE_CONNS (%d) cannot exceed DB_MAX_OPEN_CONNS (%d)",
			c.DBMaxIdleConns, c.DBMaxOpenConns)
	}

	// Validate Redis connection pool
	if c.RedisPoolSize < 1 {
		return fmt.Errorf("REDIS_POOL_SIZE must be at least 1, got %d", c.RedisPoolSize)
	}
	if c.RedisPoolSize > 100 {
		return fmt.Errorf("REDIS_POOL_SIZE must be at most 100, got %d", c.RedisPoolSize)
	}
	if c.RedisMinIdleConns < 0 {
		return fmt.Errorf("REDIS_MIN_IDLE_CONNS must be non-negative, got %d", c.RedisMinIdleConns)
	}
	if c.RedisMinIdleConns > c.RedisPoolSize {
		return fmt.Errorf("REDIS_MIN_IDLE_CONNS (%d) cannot exceed REDIS_POOL_SIZE (%d)",
			c.RedisMinIdleConns, c.RedisPoolSize)
	}

	// Validate rate limits
	if c.RateLimitMCP < 1 {
		return fmt.Errorf("RATE_LIMIT_MCP must be at least 1, got %d", c.RateLimitMCP)
	}
	if c.RateLimitIDE < 1 {
		return fmt.Errorf("RATE_LIMIT_IDE must be at least 1, got %d", c.RateLimitIDE)
	}
	if c.RateLimitSession < 1 {
		return fmt.Errorf("RATE_LIMIT_SESSION must be at least 1, got %d", c.RateLimitSession)
	}
	if c.RateLimitBurstFactor < 1.0 || c.RateLimitBurstFactor > 5.0 {
		return fmt.Errorf("RATE_LIMIT_BURST_FACTOR must be between 1.0 and 5.0, got %.2f", c.RateLimitBurstFactor)
	}

	// Validate TLS configuration
	if c.TLSEnabled {
		if c.TLSCertPath == "" {
			return fmt.Errorf("TLS_CERT_PATH is required when TLS_ENABLED is true")
		}
		if c.TLSKeyPath == "" {
			return fmt.Errorf("TLS_KEY_PATH is required when TLS_ENABLED is true")
		}
		if c.TLSMinVersion != "1.2" && c.TLSMinVersion != "1.3" {
			return fmt.Errorf("TLS_MIN_VERSION must be 1.2 or 1.3, got %s", c.TLSMinVersion)
		}
	}

	// Validate log level
	validLogLevels := map[string]bool{"debug": true, "info": true, "warn": true, "error": true}
	if !validLogLevels[c.LogLevel] {
		return fmt.Errorf("LOG_LEVEL must be one of: debug, info, warn, error, got %s", c.LogLevel)
	}

	// Validate SSL mode
	validSSLModes := map[string]bool{"disable": true, "require": true, "prefer": true, "verify-ca": true, "verify-full": true}
	if !validSSLModes[c.DBSSLMode] {
		return fmt.Errorf("DB_SSLMODE must be one of: disable, require, prefer, verify-ca, verify-full, got %s", c.DBSSLMode)
	}

	// Validate audit settings
	if c.AuditBufferSize < 100 {
		return fmt.Errorf("AUDIT_BUFFER_SIZE must be at least 100, got %d", c.AuditBufferSize)
	}
	if c.AuditBufferSize > 10000 {
		return fmt.Errorf("AUDIT_BUFFER_SIZE must be at most 10000, got %d", c.AuditBufferSize)
	}

	// Validate CORS settings
	if len(c.CORSAllowedOrigins) == 0 {
		return fmt.Errorf("CORS_ALLOWED_ORIGINS must not be empty")
	}

	// Validate circuit breaker settings
	if c.CircuitBreakerFailureThreshold < 1 {
		return fmt.Errorf("CIRCUIT_BREAKER_FAILURE_THRESHOLD must be at least 1, got %d", c.CircuitBreakerFailureThreshold)
	}
	if c.CircuitBreakerSuccessThreshold < 1 {
		return fmt.Errorf("CIRCUIT_BREAKER_SUCCESS_THRESHOLD must be at least 1, got %d", c.CircuitBreakerSuccessThreshold)
	}
	if err := ValidateTimeout("CIRCUIT_BREAKER_TIMEOUT", c.CircuitBreakerTimeout, 1*time.Second, 5*time.Minute); err != nil {
		return err
	}
	if c.CircuitBreakerMaxRequests < 1 {
		return fmt.Errorf("CIRCUIT_BREAKER_MAX_REQUESTS must be at least 1, got %d", c.CircuitBreakerMaxRequests)
	}
	if err := ValidateTimeout("CIRCUIT_BREAKER_INTERVAL", c.CircuitBreakerInterval, 1*time.Second, 5*time.Minute); err != nil {
		return err
	}

	return nil
}

// ValidateJWTSecret ensures the JWT secret meets security requirements
func ValidateJWTSecret(secret string) error {
	if len(secret) < 32 {
		return fmt.Errorf("JWT_SECRET must be at least 32 bytes, got %d", len(secret))
	}

	// Check entropy
	var entropy float64
	for _, b := range []byte(secret) {
		entropy += float64(bits.OnesCount8(uint8(b)))
	}
	if entropy/float64(len(secret)) < 3.5 {
		return fmt.Errorf("JWT_SECRET has insufficient entropy (should be random, not human-readable)")
	}

	return nil
}

// ValidateAPIKey validates an API key meets minimum security requirements
func ValidateAPIKey(key, name string) error {
	if len(key) < 32 {
		return fmt.Errorf("%s must be at least 32 characters, got %d", name, len(key))
	}

	// Check for common weak patterns
	weakPatterns := []string{
		`^[a-zA-Z]+$`,            // Only letters
		`^[0-9]+$`,               // Only numbers
		`^(password|secret|key)`, // Common weak prefixes
	}

	for _, pattern := range weakPatterns {
		matched, err := regexp.MatchString(pattern, key)
		if err != nil {
			continue
		}
		if matched {
			return fmt.Errorf("%s appears to be weak (avoid only letters, only numbers, or common words)", name)
		}
	}

	// Check for reasonable character variety
	var hasLower, hasUpper, hasDigit bool
	for _, c := range key {
		switch {
		case c >= 'a' && c <= 'z':
			hasLower = true
		case c >= 'A' && c <= 'Z':
			hasUpper = true
		case c >= '0' && c <= '9':
			hasDigit = true
		}
	}

	if !hasLower || !hasUpper || !hasDigit {
		return fmt.Errorf("%s should contain a mix of uppercase, lowercase, and digits", name)
	}

	return nil
}

// ValidateTimeout validates a timeout is within acceptable bounds
func ValidateTimeout(name string, value, min, max time.Duration) error {
	if value < min {
		return fmt.Errorf("%s must be at least %v, got %v", name, min, value)
	}
	if value > max {
		return fmt.Errorf("%s must be at most %v, got %v", name, max, value)
	}
	return nil
}

// DatabaseURL returns the PostgreSQL connection string
func (c *Config) DatabaseURL() string {
	return fmt.Sprintf("postgresql://%s:%s@%s:%d/%s?sslmode=%s&connect_timeout=%d",
		c.DBUser, c.DBPassword, c.DBHost, c.DBPort, c.DBName, c.DBSSLMode,
		int(c.DBConnectTimeout.Seconds()))
}

// RedisAddr returns the Redis connection address
func (c *Config) RedisAddr() string {
	return fmt.Sprintf("%s:%d", c.RedisHost, c.RedisPort)
}

// IsHotReloadable returns true if the config key supports hot reloading
func IsHotReloadable(key string) bool {
	hotReloadable := map[string]bool{
		"LOG_LEVEL":               true,
		"RATE_LIMIT_MCP":          true,
		"RATE_LIMIT_IDE":          true,
		"RATE_LIMIT_SESSION":      true,
		"RATE_LIMIT_WINDOW":       true,
		"RATE_LIMIT_BURST_FACTOR": true,
		"CACHE_TTL_RULES":         true,
		"CACHE_TTL_DOCS":          true,
		"CACHE_TTL_SEARCH":        true,
		"ENABLE_VALIDATION":       true,
		"ENABLE_METRICS":          true,
		"ENABLE_AUDIT_LOGGING":    true,
		"ENABLE_CACHE":            true,
		"CORS_ALLOWED_ORIGINS":    true,
		"CORS_MAX_AGE":            true,
	}
	return hotReloadable[key]
}

// HotReloadableFields returns a list of all hot-reloadable configuration keys
func HotReloadableFields() []string {
	return []string{
		"LOG_LEVEL",
		"RATE_LIMIT_MCP",
		"RATE_LIMIT_IDE",
		"RATE_LIMIT_SESSION",
		"RATE_LIMIT_WINDOW",
		"RATE_LIMIT_BURST_FACTOR",
		"CACHE_TTL_RULES",
		"CACHE_TTL_DOCS",
		"CACHE_TTL_SEARCH",
		"ENABLE_VALIDATION",
		"ENABLE_METRICS",
		"ENABLE_AUDIT_LOGGING",
		"ENABLE_CACHE",
		"CORS_ALLOWED_ORIGINS",
		"CORS_MAX_AGE",
	}
}

// Masked returns a copy of the config with sensitive values masked
func (c *Config) Masked() *Config {
	masked := *c
	masked.DBPassword = "***"
	masked.RedisPassword = "***"
	masked.MCPAPIKey = "***"
	masked.IDEAPIKey = "***"
	masked.JWTSecret = "***"
	return &masked
}
