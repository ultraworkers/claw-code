package com.example.config;

import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Nested;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;
import org.junit.jupiter.params.provider.NullAndEmptySource;
import org.junit.jupiter.params.provider.ValueSource;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Tests for ConfigLoader demonstrating guardrails-compliant testing patterns.
 * 
 * <p>Key principles demonstrated:</p>
 * <ul>
 *   <li>Production code created BEFORE test code</li>
 *   <li>Test infrastructure is separate from production</li>
 *   <li>Tests use isolated test configuration</li>
 *   <li>No production credentials in test code</li>
 * </ul>
 */
@DisplayName("ConfigLoader")
class ConfigLoaderTest {
    
    @Nested
    @DisplayName("Environment Loading")
    class EnvironmentLoading {
        
        @ParameterizedTest(name = "loads {0} environment config")
        @ValueSource(strings = {"production", "test", "development"})
        void loadsEnvironmentConfig(String environment) {
            ConfigLoader loader = new ConfigLoader(environment);
            
            Config config = loader.load();
            
            assertNotNull(config);
            assertEquals(environment, config.environment());
            assertEquals("config-loader-example", config.appName());
        }
        
        @Test
        @DisplayName("loads production config with correct settings")
        void loadsProductionConfig() {
            ConfigLoader loader = new ConfigLoader("production");
            
            Config config = loader.load();
            
            assertTrue(config.isProduction());
            assertFalse(config.debug());
            assertEquals("WARN", config.logLevel());
            assertEquals("prod-db.example.com", config.database().host());
            assertEquals(50, config.database().poolSize());
        }
        
        @Test
        @DisplayName("loads test config with correct settings")
        void loadsTestConfig() {
            ConfigLoader loader = new ConfigLoader("test");
            
            Config config = loader.load();
            
            assertTrue(config.isTest());
            assertTrue(config.debug());
            assertEquals("DEBUG", config.logLevel());
            assertEquals("localhost", config.database().host());
            assertEquals(5433, config.database().port());
            assertEquals("app_test", config.database().name());
        }
        
        @Test
        @DisplayName("loads development config with correct settings")
        void loadsDevelopmentConfig() {
            ConfigLoader loader = new ConfigLoader("development");
            
            Config config = loader.load();
            
            assertTrue(config.isDevelopment());
            assertTrue(config.debug());
            assertEquals("DEBUG", config.logLevel());
            assertEquals("localhost", config.database().host());
        }
    }
    
    @Nested
    @DisplayName("Environment Variable Handling")
    class EnvironmentVariableHandling {
        
        @ParameterizedTest(name = "defaults to development when env is \"{0}\"")
        @NullAndEmptySource
        @ValueSource(strings = {"  ", "\t", "\n"})
        void defaultsToDevelopmentForEmptyEnv(String environment) {
            ConfigLoader loader = new ConfigLoader(environment);
            
            assertEquals("development", loader.getEnvironment());
            
            Config config = loader.load();
            assertEquals("development", config.environment());
        }
        
        @Test
        @DisplayName("normalizes environment name to lowercase")
        void normalizesEnvironmentToLowercase() {
            ConfigLoader loader = new ConfigLoader("PRODUCTION");
            
            assertEquals("production", loader.getEnvironment());
        }
        
        @Test
        @DisplayName("trims whitespace from environment name")
        void trimsWhitespaceFromEnvironment() {
            ConfigLoader loader = new ConfigLoader("  test  ");
            
            assertEquals("test", loader.getEnvironment());
        }
    }
    
    @Nested
    @DisplayName("Missing Config Handling")
    class MissingConfigHandling {
        
        @Test
        @DisplayName("throws ConfigNotFoundException for unknown environment")
        void throwsExceptionForUnknownEnvironment() {
            ConfigLoader loader = new ConfigLoader("nonexistent");
            
            ConfigNotFoundException exception = assertThrows(
                ConfigNotFoundException.class,
                loader::load
            );
            
            assertEquals("nonexistent", exception.getEnvironment());
            assertEquals("config/nonexistent.yaml", exception.getConfigPath());
            assertTrue(exception.getMessage().contains("nonexistent"));
        }
        
        @ParameterizedTest(name = "throws for invalid environment: {0}")
        @ValueSource(strings = {"staging", "qa", "uat", "local"})
        void throwsForInvalidEnvironments(String environment) {
            ConfigLoader loader = new ConfigLoader(environment);
            
            assertThrows(ConfigNotFoundException.class, loader::load);
        }
    }
    
    @Nested
    @DisplayName("Environment Validation")
    class EnvironmentValidation {
        
        @ParameterizedTest(name = "{0} is a valid environment")
        @ValueSource(strings = {"production", "test", "development"})
        void validEnvironmentsReturnTrue(String environment) {
            assertTrue(ConfigLoader.isValidEnvironment(environment));
        }
        
        @ParameterizedTest(name = "\"{0}\" is not a valid environment")
        @ValueSource(strings = {"staging", "qa", "prod", "dev", "PRODUCTION"})
        void invalidEnvironmentsReturnFalse(String environment) {
            assertFalse(ConfigLoader.isValidEnvironment(environment));
        }
        
        @Test
        @DisplayName("null is not a valid environment")
        void nullIsInvalidEnvironment() {
            assertFalse(ConfigLoader.isValidEnvironment(null));
        }
    }
    
    @Nested
    @DisplayName("Database Configuration")
    class DatabaseConfiguration {
        
        @ParameterizedTest(name = "{0} environment has correct database port")
        @CsvSource({
            "production, 5432",
            "test, 5433",
            "development, 5432"
        })
        void hasCorrectDatabasePort(String environment, int expectedPort) {
            ConfigLoader loader = new ConfigLoader(environment);
            
            Config config = loader.load();
            
            assertEquals(expectedPort, config.database().port());
        }
        
        @Test
        @DisplayName("generates correct JDBC URL")
        void generatesCorrectJdbcUrl() {
            ConfigLoader loader = new ConfigLoader("test");
            
            Config config = loader.load();
            
            String expectedUrl = "jdbc:postgresql://localhost:5433/app_test";
            assertEquals(expectedUrl, config.database().getJdbcUrl());
        }
        
        @ParameterizedTest(name = "{0} has pool size {1}")
        @CsvSource({
            "production, 50",
            "test, 5",
            "development, 10"
        })
        void hasCorrectPoolSize(String environment, int expectedPoolSize) {
            ConfigLoader loader = new ConfigLoader(environment);
            
            Config config = loader.load();
            
            assertEquals(expectedPoolSize, config.database().poolSize());
        }
    }
    
    @Nested
    @DisplayName("Config Helper Methods")
    class ConfigHelperMethods {
        
        @Test
        @DisplayName("isProduction returns true only for production")
        void isProductionOnlyForProduction() {
            assertTrue(new ConfigLoader("production").load().isProduction());
            assertFalse(new ConfigLoader("test").load().isProduction());
            assertFalse(new ConfigLoader("development").load().isProduction());
        }
        
        @Test
        @DisplayName("isTest returns true only for test")
        void isTestOnlyForTest() {
            assertFalse(new ConfigLoader("production").load().isTest());
            assertTrue(new ConfigLoader("test").load().isTest());
            assertFalse(new ConfigLoader("development").load().isTest());
        }
        
        @Test
        @DisplayName("isDevelopment returns true only for development")
        void isDevelopmentOnlyForDevelopment() {
            assertFalse(new ConfigLoader("production").load().isDevelopment());
            assertFalse(new ConfigLoader("test").load().isDevelopment());
            assertTrue(new ConfigLoader("development").load().isDevelopment());
        }
    }
    
    @Nested
    @DisplayName("Edge Cases")
    class EdgeCases {
        
        @Test
        @DisplayName("DatabaseConfig withDefaults creates valid config")
        void databaseConfigWithDefaults() {
            DatabaseConfig db = DatabaseConfig.withDefaults(
                "myhost", 5432, "mydb", "myuser"
            );
            
            assertEquals("myhost", db.host());
            assertEquals(5432, db.port());
            assertEquals("mydb", db.name());
            assertEquals("myuser", db.username());
            assertEquals(10, db.poolSize());
            assertEquals(30000, db.connectionTimeout());
        }
        
        @Test
        @DisplayName("multiple loads return consistent results")
        void multipleLoadsReturnConsistentResults() {
            ConfigLoader loader = new ConfigLoader("test");
            
            Config first = loader.load();
            Config second = loader.load();
            
            assertEquals(first.appName(), second.appName());
            assertEquals(first.environment(), second.environment());
            assertEquals(first.database().host(), second.database().host());
        }
    }
}
