//! Configuration Loading Module
//!
//! Demonstrates guardrails-compliant config loading patterns with:
//! - Environment-based configuration (APP_ENV)
//! - Proper error handling with Result types
//! - Clear test/production separation
//!
//! # Example
//!
//! ```no_run
//! use guardrails_config_example::{load_config, Config};
//!
//! // Load config based on APP_ENV environment variable
//! let config = load_config(None).expect("Failed to load config");
//! println!("Loaded config for: {}", config.environment);
//! ```

use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

// ============================================================================
// PRODUCTION CODE - Structs and Types
// ============================================================================

/// Main application configuration
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Config {
    /// Application name
    pub app_name: String,

    /// Current environment (production, test, development)
    pub environment: String,

    /// Enable debug mode
    pub debug: bool,

    /// Logging level (debug, info, warn, error)
    pub log_level: String,

    /// Database configuration
    pub database: DatabaseConfig,

    /// External services configuration
    pub services: ServicesConfig,
}

/// Database connection configuration
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct DatabaseConfig {
    /// Database host
    pub host: String,

    /// Database port
    pub port: u16,

    /// Database name
    pub name: String,

    /// Connection pool size
    pub pool_size: u32,

    /// SSL mode (disable, prefer, require)
    pub ssl_mode: String,
}

/// External services configuration
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ServicesConfig {
    /// API endpoint URL
    pub api_url: String,

    /// Cache service URL
    pub cache_url: String,

    /// Request timeout in seconds
    pub timeout_seconds: u32,
}

// ============================================================================
// PRODUCTION CODE - Error Types
// ============================================================================

/// Configuration loading errors
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Configuration file not found
    #[error("Configuration file not found: {path}")]
    NotFound { path: String },

    /// Failed to read configuration file
    #[error("Failed to read configuration file: {source}")]
    ReadError {
        #[from]
        source: std::io::Error,
    },

    /// Failed to parse YAML configuration
    #[error("Failed to parse configuration: {source}")]
    ParseError {
        #[from]
        source: serde_yaml::Error,
    },

    /// Invalid environment specified
    #[error("Invalid environment: {env}. Valid options: production, test, development")]
    InvalidEnvironment { env: String },
}

// ============================================================================
// PRODUCTION CODE - Functions
// ============================================================================

/// Valid environment names
const VALID_ENVIRONMENTS: [&str; 3] = ["production", "test", "development"];

/// Loads configuration based on the APP_ENV environment variable.
///
/// # Arguments
///
/// * `config_dir` - Optional custom config directory path. If None, uses "./config"
///
/// # Returns
///
/// * `Ok(Config)` - Successfully loaded configuration
/// * `Err(ConfigError)` - Failed to load configuration
///
/// # Environment Variables
///
/// * `APP_ENV` - Determines which config file to load (default: "development")
///
/// # Example
///
/// ```no_run
/// use guardrails_config_example::load_config;
///
/// // Uses APP_ENV to determine environment
/// let config = load_config(None)?;
/// # Ok::<(), guardrails_config_example::ConfigError>(())
/// ```
pub fn load_config(config_dir: Option<&Path>) -> Result<Config, ConfigError> {
    let environment = get_environment()?;
    load_config_for_env(&environment, config_dir)
}

/// Loads configuration for a specific environment.
///
/// # Arguments
///
/// * `environment` - The environment name (production, test, development)
/// * `config_dir` - Optional custom config directory path
///
/// # Returns
///
/// * `Ok(Config)` - Successfully loaded configuration
/// * `Err(ConfigError)` - Failed to load configuration
pub fn load_config_for_env(
    environment: &str,
    config_dir: Option<&Path>,
) -> Result<Config, ConfigError> {
    validate_environment(environment)?;

    let config_path = get_config_path(environment, config_dir);

    if !config_path.exists() {
        return Err(ConfigError::NotFound {
            path: config_path.display().to_string(),
        });
    }

    let contents = fs::read_to_string(&config_path)?;
    let config: Config = serde_yaml::from_str(&contents)?;

    Ok(config)
}

/// Gets the current environment from APP_ENV, defaulting to "development".
///
/// # Returns
///
/// * `Ok(String)` - The validated environment name
/// * `Err(ConfigError)` - Invalid environment specified
pub fn get_environment() -> Result<String, ConfigError> {
    let env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    validate_environment(&env)?;
    Ok(env)
}

/// Validates that an environment name is valid.
fn validate_environment(environment: &str) -> Result<(), ConfigError> {
    if VALID_ENVIRONMENTS.contains(&environment) {
        Ok(())
    } else {
        Err(ConfigError::InvalidEnvironment {
            env: environment.to_string(),
        })
    }
}

/// Constructs the path to a configuration file.
fn get_config_path(environment: &str, config_dir: Option<&Path>) -> PathBuf {
    let base_dir = config_dir.unwrap_or_else(|| Path::new("config"));
    base_dir.join(format!("{}.yaml", environment))
}

/// Checks if the current configuration is for a test environment.
///
/// # Safety Check
///
/// This function helps prevent accidental use of production resources in tests.
pub fn is_test_environment(config: &Config) -> bool {
    config.environment == "test"
}

/// Checks if the current configuration is for production.
///
/// # Safety Check
///
/// Use this to add extra safeguards around production operations.
pub fn is_production_environment(config: &Config) -> bool {
    config.environment == "production"
}

// ============================================================================
// TEST CODE - Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper: Creates a temporary config directory with test configs
    fn create_test_config_dir() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create test config
        let test_config = r#"
app_name: "test-app"
environment: "test"
debug: true
log_level: "debug"

database:
  host: "localhost"
  port: 5433
  name: "app_test"
  pool_size: 2
  ssl_mode: "disable"

services:
  api_url: "http://localhost:8080"
  cache_url: "redis://localhost:6380"
  timeout_seconds: 5
"#;

        let test_path = temp_dir.path().join("test.yaml");
        let mut file = fs::File::create(&test_path).expect("Failed to create test config");
        file.write_all(test_config.as_bytes())
            .expect("Failed to write test config");

        // Create development config
        let dev_config = r#"
app_name: "dev-app"
environment: "development"
debug: true
log_level: "debug"

database:
  host: "localhost"
  port: 5432
  name: "app_development"
  pool_size: 5
  ssl_mode: "prefer"

services:
  api_url: "http://localhost:3000"
  cache_url: "redis://localhost:6379"
  timeout_seconds: 10
"#;

        let dev_path = temp_dir.path().join("development.yaml");
        let mut file = fs::File::create(&dev_path).expect("Failed to create dev config");
        file.write_all(dev_config.as_bytes())
            .expect("Failed to write dev config");

        // Create production config (for testing config loading only)
        let prod_config = r#"
app_name: "prod-app"
environment: "production"
debug: false
log_level: "info"

database:
  host: "prod-db.example.com"
  port: 5432
  name: "app_production"
  pool_size: 20
  ssl_mode: "require"

services:
  api_url: "https://api.example.com"
  cache_url: "redis://prod-cache.example.com:6379"
  timeout_seconds: 30
"#;

        let prod_path = temp_dir.path().join("production.yaml");
        let mut file = fs::File::create(&prod_path).expect("Failed to create prod config");
        file.write_all(prod_config.as_bytes())
            .expect("Failed to write prod config");

        temp_dir
    }

    // -------------------------------------------------------------------------
    // Config Loading Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_load_test_config() {
        let temp_dir = create_test_config_dir();

        let config =
            load_config_for_env("test", Some(temp_dir.path())).expect("Failed to load test config");

        assert_eq!(config.app_name, "test-app");
        assert_eq!(config.environment, "test");
        assert!(config.debug);
        assert_eq!(config.log_level, "debug");
    }

    #[test]
    fn test_load_development_config() {
        let temp_dir = create_test_config_dir();

        let config = load_config_for_env("development", Some(temp_dir.path()))
            .expect("Failed to load dev config");

        assert_eq!(config.app_name, "dev-app");
        assert_eq!(config.environment, "development");
        assert!(config.debug);
    }

    #[test]
    fn test_load_production_config() {
        let temp_dir = create_test_config_dir();

        let config = load_config_for_env("production", Some(temp_dir.path()))
            .expect("Failed to load prod config");

        assert_eq!(config.app_name, "prod-app");
        assert_eq!(config.environment, "production");
        assert!(!config.debug);
        assert_eq!(config.log_level, "info");
    }

    // -------------------------------------------------------------------------
    // Database Config Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_database_config_parsing() {
        let temp_dir = create_test_config_dir();

        let config =
            load_config_for_env("test", Some(temp_dir.path())).expect("Failed to load config");

        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 5433);
        assert_eq!(config.database.name, "app_test");
        assert_eq!(config.database.pool_size, 2);
        assert_eq!(config.database.ssl_mode, "disable");
    }

    #[test]
    fn test_production_database_differs_from_test() {
        let temp_dir = create_test_config_dir();

        let test_config =
            load_config_for_env("test", Some(temp_dir.path())).expect("Failed to load test config");

        let prod_config = load_config_for_env("production", Some(temp_dir.path()))
            .expect("Failed to load prod config");

        // Verify test and production use different databases
        assert_ne!(test_config.database.host, prod_config.database.host);
        assert_ne!(test_config.database.name, prod_config.database.name);
        assert_ne!(test_config.database.port, prod_config.database.port);
    }

    // -------------------------------------------------------------------------
    // Services Config Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_services_config_parsing() {
        let temp_dir = create_test_config_dir();

        let config =
            load_config_for_env("test", Some(temp_dir.path())).expect("Failed to load config");

        assert_eq!(config.services.api_url, "http://localhost:8080");
        assert_eq!(config.services.cache_url, "redis://localhost:6380");
        assert_eq!(config.services.timeout_seconds, 5);
    }

    #[test]
    fn test_production_services_differ_from_test() {
        let temp_dir = create_test_config_dir();

        let test_config =
            load_config_for_env("test", Some(temp_dir.path())).expect("Failed to load test config");

        let prod_config = load_config_for_env("production", Some(temp_dir.path()))
            .expect("Failed to load prod config");

        // Verify test and production use different services
        assert_ne!(test_config.services.api_url, prod_config.services.api_url);
        assert_ne!(
            test_config.services.cache_url,
            prod_config.services.cache_url
        );
    }

    // -------------------------------------------------------------------------
    // Error Handling Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_invalid_environment_error() {
        let temp_dir = create_test_config_dir();

        let result = load_config_for_env("invalid", Some(temp_dir.path()));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidEnvironment { .. }));
    }

    #[test]
    fn test_missing_config_file_error() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        // Don't create any config files

        let result = load_config_for_env("test", Some(temp_dir.path()));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::NotFound { .. }));
    }

    #[test]
    fn test_invalid_yaml_error() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create invalid YAML
        let invalid_yaml = "this is: not: valid: yaml: [[[";
        let test_path = temp_dir.path().join("test.yaml");
        let mut file = fs::File::create(&test_path).expect("Failed to create file");
        file.write_all(invalid_yaml.as_bytes())
            .expect("Failed to write");

        let result = load_config_for_env("test", Some(temp_dir.path()));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::ParseError { .. }));
    }

    // -------------------------------------------------------------------------
    // Environment Helper Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_is_test_environment() {
        let temp_dir = create_test_config_dir();

        let test_config =
            load_config_for_env("test", Some(temp_dir.path())).expect("Failed to load config");

        let prod_config = load_config_for_env("production", Some(temp_dir.path()))
            .expect("Failed to load config");

        assert!(is_test_environment(&test_config));
        assert!(!is_test_environment(&prod_config));
    }

    #[test]
    fn test_is_production_environment() {
        let temp_dir = create_test_config_dir();

        let test_config =
            load_config_for_env("test", Some(temp_dir.path())).expect("Failed to load config");

        let prod_config = load_config_for_env("production", Some(temp_dir.path()))
            .expect("Failed to load config");

        assert!(!is_production_environment(&test_config));
        assert!(is_production_environment(&prod_config));
    }

    // -------------------------------------------------------------------------
    // Config Path Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_get_config_path_with_custom_dir() {
        let custom_path = Path::new("/custom/config");
        let result = get_config_path("test", Some(custom_path));

        assert_eq!(result, PathBuf::from("/custom/config/test.yaml"));
    }

    #[test]
    fn test_get_config_path_default_dir() {
        let result = get_config_path("production", None);

        assert_eq!(result, PathBuf::from("config/production.yaml"));
    }

    // -------------------------------------------------------------------------
    // Environment Validation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_validate_valid_environments() {
        assert!(validate_environment("production").is_ok());
        assert!(validate_environment("test").is_ok());
        assert!(validate_environment("development").is_ok());
    }

    #[test]
    fn test_validate_invalid_environments() {
        assert!(validate_environment("staging").is_err());
        assert!(validate_environment("prod").is_err());
        assert!(validate_environment("").is_err());
    }

    // -------------------------------------------------------------------------
    // Config Equality Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_config_equality() {
        let temp_dir = create_test_config_dir();

        let config1 =
            load_config_for_env("test", Some(temp_dir.path())).expect("Failed to load config");

        let config2 =
            load_config_for_env("test", Some(temp_dir.path())).expect("Failed to load config");

        assert_eq!(config1, config2);
    }

    #[test]
    fn test_config_clone() {
        let temp_dir = create_test_config_dir();

        let config =
            load_config_for_env("test", Some(temp_dir.path())).expect("Failed to load config");

        let cloned = config.clone();

        assert_eq!(config, cloned);
    }
}
