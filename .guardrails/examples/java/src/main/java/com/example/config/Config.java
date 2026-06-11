package com.example.config;

/**
 * Application configuration container.
 * 
 * Holds all configuration settings for the application.
 * Loaded from environment-specific YAML files.
 */
public record Config(
    String appName,
    String environment,
    boolean debug,
    String logLevel,
    DatabaseConfig database
) {
    /**
     * Checks if the configuration is for a production environment.
     *
     * @return true if this is production config
     */
    public boolean isProduction() {
        return "production".equals(environment);
    }
    
    /**
     * Checks if the configuration is for a test environment.
     *
     * @return true if this is test config
     */
    public boolean isTest() {
        return "test".equals(environment);
    }
    
    /**
     * Checks if the configuration is for development environment.
     *
     * @return true if this is development config
     */
    public boolean isDevelopment() {
        return "development".equals(environment);
    }
}
