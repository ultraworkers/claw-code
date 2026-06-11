package com.example.config;

/**
 * Exception thrown when a configuration file cannot be found.
 * 
 * Provides clear information about which environment configuration
 * was requested and the path that was searched.
 */
public class ConfigNotFoundException extends RuntimeException {
    
    private final String environment;
    private final String configPath;
    
    /**
     * Creates a new ConfigNotFoundException.
     *
     * @param environment The environment name that was requested
     * @param configPath  The path that was searched
     */
    public ConfigNotFoundException(String environment, String configPath) {
        super(String.format(
            "Configuration file not found for environment '%s': %s",
            environment,
            configPath
        ));
        this.environment = environment;
        this.configPath = configPath;
    }
    
    /**
     * Gets the environment that was requested.
     *
     * @return The environment name
     */
    public String getEnvironment() {
        return environment;
    }
    
    /**
     * Gets the path that was searched.
     *
     * @return The config path
     */
    public String getConfigPath() {
        return configPath;
    }
}
