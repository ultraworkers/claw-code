package com.example.config;

/**
 * Database configuration settings.
 * 
 * Holds connection parameters for database access.
 * Separate configs should be used for test vs production environments.
 */
public record DatabaseConfig(
    String host,
    int port,
    String name,
    String username,
    int poolSize,
    int connectionTimeout
) {
    /**
     * Creates a DatabaseConfig with default pool settings.
     *
     * @param host     Database host address
     * @param port     Database port
     * @param name     Database name
     * @param username Database username
     * @return DatabaseConfig with default pool size (10) and timeout (30000ms)
     */
    public static DatabaseConfig withDefaults(String host, int port, String name, String username) {
        return new DatabaseConfig(host, port, name, username, 10, 30000);
    }
    
    /**
     * Returns the JDBC connection URL.
     *
     * @return JDBC URL string
     */
    public String getJdbcUrl() {
        return String.format("jdbc:postgresql://%s:%d/%s", host, port, name);
    }
}
