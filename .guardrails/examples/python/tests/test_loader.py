"""
Tests for ConfigLoader

This test module provides comprehensive coverage of the configuration loading
functionality, including:
- Loading configurations for each environment
- Environment variable handling
- Missing config file handling
- Invalid config validation
- Default environment fallback

All tests use isolated temporary directories to ensure no production
configuration is ever accessed during testing.
"""

from pathlib import Path

import pytest

from config_loader import (
    Config,
    ConfigLoader,
    ConfigNotFoundError,
    InvalidConfigError,
)


class TestConfigLoaderInitialization:
    """Tests for ConfigLoader initialization."""

    def test_default_config_dir_is_cwd_config(self) -> None:
        """Default config directory should be ./config relative to cwd."""
        loader = ConfigLoader(environment="development")
        expected = Path.cwd() / "config"
        assert loader.config_dir == expected

    def test_custom_config_dir_is_set(self, temp_config_dir: Path) -> None:
        """Custom config directory should be used when provided."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")
        assert loader.config_dir == temp_config_dir

    def test_default_environment_is_development(
        self, temp_config_dir: Path, clean_environment: None
    ) -> None:
        """Default environment should be 'development' when APP_ENV not set."""
        loader = ConfigLoader(config_dir=temp_config_dir)
        assert loader.environment == "development"

    def test_environment_override_takes_precedence(
        self, temp_config_dir: Path, set_app_env
    ) -> None:
        """Explicit environment parameter should override APP_ENV."""
        set_app_env("production")
        loader = ConfigLoader(config_dir=temp_config_dir, environment="test")
        assert loader.environment == "test"


class TestEnvironmentVariableHandling:
    """Tests for APP_ENV environment variable handling."""

    def test_reads_app_env_variable(
        self, temp_config_dir: Path, set_app_env
    ) -> None:
        """Should read environment from APP_ENV variable."""
        set_app_env("production")
        loader = ConfigLoader(config_dir=temp_config_dir)
        assert loader.environment == "production"

    def test_app_env_is_case_insensitive(
        self, temp_config_dir: Path, set_app_env
    ) -> None:
        """APP_ENV should be case-insensitive."""
        set_app_env("PRODUCTION")
        loader = ConfigLoader(config_dir=temp_config_dir)
        assert loader.environment == "production"

    def test_app_env_is_stripped(
        self, temp_config_dir: Path, set_app_env
    ) -> None:
        """APP_ENV should be stripped of whitespace."""
        set_app_env("  test  ")
        loader = ConfigLoader(config_dir=temp_config_dir)
        assert loader.environment == "test"

    def test_invalid_environment_raises_error(
        self, temp_config_dir: Path, set_app_env
    ) -> None:
        """Invalid environment name should raise InvalidConfigError."""
        set_app_env("staging")  # Not a valid environment
        with pytest.raises(InvalidConfigError) as exc_info:
            ConfigLoader(config_dir=temp_config_dir)
        assert "Invalid environment 'staging'" in str(exc_info.value)
        assert "development" in str(exc_info.value)
        assert "test" in str(exc_info.value)
        assert "production" in str(exc_info.value)


class TestLoadingEnvironmentConfigs:
    """Tests for loading configuration from each environment."""

    def test_load_development_config(
        self, temp_config_dir: Path, valid_development_config: Path
    ) -> None:
        """Should load development configuration successfully."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")
        config = loader.load()

        assert config.environment == "development"
        assert config.database.host == "localhost"
        assert config.database.port == 5432
        assert config.database.name == "dev_db"
        assert config.database.user == "dev_user"
        assert config.database.password == "dev_password"
        assert config.services.api == "http://localhost:8080"

    def test_load_test_config(
        self, temp_config_dir: Path, valid_test_config: Path
    ) -> None:
        """Should load test configuration successfully."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="test")
        config = loader.load()

        assert config.environment == "test"
        assert config.database.host == "test-db.example.com"
        assert config.database.port == 5432
        assert config.database.name == "test_db"
        assert config.database.user == "test_user"
        assert config.database.password is None  # Not set in test config
        assert config.services.api == "https://api.test.example.com"
        assert config.services.auth == "https://auth.test.example.com"

    def test_load_production_config(
        self, temp_config_dir: Path, valid_production_config: Path
    ) -> None:
        """Should load production configuration successfully."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="production")
        config = loader.load()

        assert config.environment == "production"
        assert config.database.host == "prod-db.example.com"
        assert config.database.port == 5432
        assert config.database.name == "production_db"
        assert config.services.api == "https://api.production.example.com"
        assert config.services.storage == "https://storage.production.example.com"

    def test_config_returns_frozen_dataclass(
        self, temp_config_dir: Path, valid_development_config: Path
    ) -> None:
        """Config should be a frozen dataclass (immutable)."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")
        config = loader.load()

        assert isinstance(config, Config)
        # Frozen dataclass should raise error on mutation attempt
        with pytest.raises(AttributeError):
            config.environment = "changed"  # type: ignore

    def test_raw_config_available(
        self, temp_config_dir: Path, valid_development_config: Path
    ) -> None:
        """Raw configuration dictionary should be accessible."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")
        config = loader.load()

        assert isinstance(config.raw, dict)
        assert config.raw["app"]["debug"] is True


class TestMissingConfigFile:
    """Tests for handling missing configuration files."""

    def test_missing_config_file_raises_error(self, temp_config_dir: Path) -> None:
        """Missing config file should raise ConfigNotFoundError."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")

        with pytest.raises(ConfigNotFoundError) as exc_info:
            loader.load()

        assert exc_info.value.environment == "development"
        assert "development.yaml" in exc_info.value.config_path
        assert "Configuration file not found" in str(exc_info.value)

    def test_missing_config_includes_environment_in_error(
        self, temp_config_dir: Path
    ) -> None:
        """Error should include the environment name."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="production")

        with pytest.raises(ConfigNotFoundError) as exc_info:
            loader.load()

        assert "production" in str(exc_info.value)


class TestInvalidConfigValidation:
    """Tests for configuration validation errors."""

    def test_missing_database_section_raises_error(
        self, temp_config_dir: Path, missing_database_config: Path
    ) -> None:
        """Missing database section should raise InvalidConfigError."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")

        with pytest.raises(InvalidConfigError) as exc_info:
            loader.load()

        assert "Missing 'database' section" in str(exc_info.value)
        assert "database" in exc_info.value.missing_fields

    def test_incomplete_database_config_raises_error(
        self, temp_config_dir: Path, incomplete_database_config: Path
    ) -> None:
        """Incomplete database config should list missing fields."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")

        with pytest.raises(InvalidConfigError) as exc_info:
            loader.load()

        assert "Missing required database fields" in str(exc_info.value)
        assert "port" in exc_info.value.missing_fields
        assert "name" in exc_info.value.missing_fields

    def test_invalid_port_raises_error(
        self, temp_config_dir: Path, invalid_port_config: Path
    ) -> None:
        """Non-integer port should raise InvalidConfigError."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")

        with pytest.raises(InvalidConfigError) as exc_info:
            loader.load()

        assert "port must be an integer" in str(exc_info.value)

    def test_empty_config_raises_error(
        self, temp_config_dir: Path, empty_config: Path
    ) -> None:
        """Empty config file should raise InvalidConfigError."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")

        with pytest.raises(InvalidConfigError) as exc_info:
            loader.load()

        assert "empty" in str(exc_info.value).lower()

    def test_malformed_yaml_raises_error(
        self, temp_config_dir: Path, malformed_yaml_config: Path
    ) -> None:
        """Malformed YAML should raise InvalidConfigError."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")

        with pytest.raises(InvalidConfigError) as exc_info:
            loader.load()

        assert "Invalid YAML" in str(exc_info.value)


class TestDatabaseConfig:
    """Tests for DatabaseConfig functionality."""

    def test_connection_string_with_auth(
        self, temp_config_dir: Path, valid_development_config: Path
    ) -> None:
        """Connection string should include user and password."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")
        config = loader.load()

        conn_str = config.database.connection_string()
        assert conn_str == "postgresql://dev_user:dev_password@localhost:5432/dev_db"

    def test_connection_string_without_password(
        self, temp_config_dir: Path, valid_test_config: Path
    ) -> None:
        """Connection string should work without password."""
        loader = ConfigLoader(config_dir=temp_config_dir, environment="test")
        config = loader.load()

        conn_str = config.database.connection_string()
        assert conn_str == "postgresql://test_user@test-db.example.com:5432/test_db"

    def test_connection_string_without_auth(self, temp_config_dir: Path) -> None:
        """Connection string should work without user/password."""
        config_file = temp_config_dir / "development.yaml"
        config_file.write_text(
            """
environment: development
database:
  host: localhost
  port: 5432
  name: dev_db
"""
        )
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")
        config = loader.load()

        conn_str = config.database.connection_string()
        assert conn_str == "postgresql://localhost:5432/dev_db"


class TestServicesConfig:
    """Tests for ServicesConfig functionality."""

    def test_services_optional(self, temp_config_dir: Path) -> None:
        """Services section should be optional."""
        config_file = temp_config_dir / "development.yaml"
        config_file.write_text(
            """
environment: development
database:
  host: localhost
  port: 5432
  name: dev_db
"""
        )
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")
        config = loader.load()

        assert config.services.api is None
        assert config.services.auth is None
        assert config.services.storage is None

    def test_partial_services_config(self, temp_config_dir: Path) -> None:
        """Should handle partial services configuration."""
        config_file = temp_config_dir / "development.yaml"
        config_file.write_text(
            """
environment: development
database:
  host: localhost
  port: 5432
  name: dev_db
services:
  api: http://localhost:8080
"""
        )
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")
        config = loader.load()

        assert config.services.api == "http://localhost:8080"
        assert config.services.auth is None
        assert config.services.storage is None


class TestDefaultEnvironmentFallback:
    """Tests for default environment fallback behavior."""

    def test_defaults_to_development(
        self, temp_config_dir: Path, valid_development_config: Path, clean_environment: None
    ) -> None:
        """Should default to development when APP_ENV not set."""
        loader = ConfigLoader(config_dir=temp_config_dir)
        config = loader.load()

        assert config.environment == "development"

    def test_explicit_environment_parameter(
        self, temp_config_dir: Path, all_configs: dict[str, Path]
    ) -> None:
        """Should use explicit environment parameter."""
        for env_name in ["development", "test", "production"]:
            loader = ConfigLoader(config_dir=temp_config_dir, environment=env_name)
            config = loader.load()
            assert config.environment == env_name


class TestEdgeCases:
    """Tests for edge cases and special scenarios."""

    def test_config_with_extra_fields(self, temp_config_dir: Path) -> None:
        """Should handle extra fields in configuration."""
        config_file = temp_config_dir / "development.yaml"
        config_file.write_text(
            """
environment: development
database:
  host: localhost
  port: 5432
  name: dev_db
  extra_field: extra_value
custom_section:
  custom_key: custom_value
"""
        )
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")
        config = loader.load()

        assert config.raw["custom_section"]["custom_key"] == "custom_value"

    def test_numeric_string_port(self, temp_config_dir: Path) -> None:
        """Should handle port as quoted string."""
        config_file = temp_config_dir / "development.yaml"
        config_file.write_text(
            """
environment: development
database:
  host: localhost
  port: "5432"
  name: dev_db
"""
        )
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")
        config = loader.load()

        assert config.database.port == 5432
        assert isinstance(config.database.port, int)

    def test_database_not_dict_raises_error(self, temp_config_dir: Path) -> None:
        """Should raise error if database is not a mapping."""
        config_file = temp_config_dir / "development.yaml"
        config_file.write_text(
            """
environment: development
database: just_a_string
"""
        )
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")

        with pytest.raises(InvalidConfigError) as exc_info:
            loader.load()

        assert "must be a mapping" in str(exc_info.value)

    def test_config_not_dict_raises_error(self, temp_config_dir: Path) -> None:
        """Should raise error if root config is not a mapping."""
        config_file = temp_config_dir / "development.yaml"
        config_file.write_text("- just\n- a\n- list\n")
        loader = ConfigLoader(config_dir=temp_config_dir, environment="development")

        with pytest.raises(InvalidConfigError) as exc_info:
            loader.load()

        assert "must be a YAML mapping" in str(exc_info.value)

    def test_path_as_string(self, temp_config_dir: Path, valid_development_config: Path) -> None:
        """Should accept config_dir as string."""
        loader = ConfigLoader(
            config_dir=str(temp_config_dir), environment="development"
        )
        config = loader.load()
        assert config.environment == "development"
