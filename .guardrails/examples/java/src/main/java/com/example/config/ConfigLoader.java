package com.example.config;

import org.yaml.snakeyaml.Yaml;

import java.io.InputStream;
import java.util.Map;
import java.util.Objects;

/**
 * Environment-aware configuration loader.
 * 
 * Loads YAML configuration files based on the APP_ENV environment variable.
 * Demonstrates guardrails-compliant test/production separation patterns.
 * 
 * <p>Supported environments:</p>
 * <ul>
 *   <li>production - Production settings (default)</li>
 *   <li>test - Test environment settings</li>
 *   <li>development - Development settings</li>
 * </ul>
 * 
 * <p>Usage:</p>
 * <pre>{@code
 * ConfigLoader loader = new ConfigLoader();
 * Config config = loader.load();
 * }</pre>
 */
public class ConfigLoader {
    
    private static final String ENV_VAR_NAME = "APP_ENV";
    private static final String DEFAULT_ENVIRONMENT = "development";
    private static final String CONFIG_PATH_TEMPLATE = "config/%s.yaml";
    
    private final String environment;
    private final Yaml yaml;
    
    /**
     * Creates a ConfigLoader using the APP_ENV environment variable.
     * Defaults to "development" if APP_ENV is not set.
     */
    public ConfigLoader() {
        this(System.getenv(ENV_VAR_NAME));
    }
    
    /**
     * Creates a ConfigLoader for a specific environment.
     * 
     * @param environment The environment name (production, test, development)
     */
    public ConfigLoader(String environment) {
        this.environment = (environment == null || environment.isBlank()) 
            ? DEFAULT_ENVIRONMENT 
            : environment.toLowerCase().trim();
        this.yaml = new Yaml();
    }
    
    /**
     * Loads configuration for the current environment.
     * 
     * @return Config object with all settings
     * @throws ConfigNotFoundException if the config file doesn't exist
     */
    public Config load() {
        String configPath = String.format(CONFIG_PATH_TEMPLATE, environment);
        InputStream inputStream = getClass().getClassLoader().getResourceAsStream(configPath);
        
        if (inputStream == null) {
            throw new ConfigNotFoundException(environment, configPath);
        }
        
        Map<String, Object> data = yaml.load(inputStream);
        return parseConfig(data);
    }
    
    /**
     * Gets the current environment name.
     * 
     * @return The environment name
     */
    public String getEnvironment() {
        return environment;
    }
    
    /**
     * Checks if the given environment name is valid.
     * 
     * @param env Environment name to validate
     * @return true if valid
     */
    public static boolean isValidEnvironment(String env) {
        return env != null && (
            env.equals("production") ||
            env.equals("test") ||
            env.equals("development")
        );
    }
    
    @SuppressWarnings("unchecked")
    private Config parseConfig(Map<String, Object> data) {
        Objects.requireNonNull(data, "Configuration data cannot be null");
        
        String appName = getString(data, "app_name", "unnamed-app");
        String env = getString(data, "environment", environment);
        boolean debug = getBoolean(data, "debug", false);
        String logLevel = getString(data, "log_level", "INFO");
        
        Map<String, Object> dbData = (Map<String, Object>) data.get("database");
        DatabaseConfig database = parseDatabase(dbData);
        
        return new Config(appName, env, debug, logLevel, database);
    }
    
    private DatabaseConfig parseDatabase(Map<String, Object> dbData) {
        if (dbData == null) {
            return DatabaseConfig.withDefaults("localhost", 5432, "app_db", "app_user");
        }
        
        return new DatabaseConfig(
            getString(dbData, "host", "localhost"),
            getInt(dbData, "port", 5432),
            getString(dbData, "name", "app_db"),
            getString(dbData, "username", "app_user"),
            getInt(dbData, "pool_size", 10),
            getInt(dbData, "connection_timeout", 30000)
        );
    }
    
    private String getString(Map<String, Object> data, String key, String defaultValue) {
        Object value = data.get(key);
        return value != null ? value.toString() : defaultValue;
    }
    
    private int getInt(Map<String, Object> data, String key, int defaultValue) {
        Object value = data.get(key);
        if (value instanceof Number) {
            return ((Number) value).intValue();
        }
        return defaultValue;
    }
    
    private boolean getBoolean(Map<String, Object> data, String key, boolean defaultValue) {
        Object value = data.get(key);
        if (value instanceof Boolean) {
            return (Boolean) value;
        }
        return defaultValue;
    }
}
