"""
Configuration Loader Module

Loads YAML configuration files based on the APP_ENV environment variable.
Supports production, test, and development environments with validation.

Example usage:
    loader = ConfigLoader()
    config = loader.load()
    print(config.database.host)
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml

from config_loader.exceptions import ConfigNotFoundError, InvalidConfigError


# Required fields for database configuration validation
REQUIRED_DATABASE_FIELDS = ["host", "port", "name"]

# Valid environment names
VALID_ENVIRONMENTS = {"production", "test", "development"}

# Default environment when APP_ENV is not set
DEFAULT_ENVIRONMENT = "development"


@dataclass(frozen=True)
class DatabaseConfig:
    """Database configuration with required connection parameters."""

    host: str
    port: int
    name: str
    user: str | None = None
    password: str | None = None

    def connection_string(self) -> str:
        """Generate a PostgreSQL connection string."""
        auth = ""
        if self.user:
            auth = f"{self.user}"
            if self.password:
                auth += f":{self.password}"
            auth += "@"
        return f"postgresql://{auth}{self.host}:{self.port}/{self.name}"


@dataclass(frozen=True)
class ServicesConfig:
    """External service endpoint configuration."""

    api: str | None = None
    auth: str | None = None
    storage: str | None = None


@dataclass(frozen=True)
class Config:
    """Complete application configuration.

    Attributes:
        environment: The environment name (production, test, development).
        database: Database connection configuration.
        services: External service endpoints.
        raw: The raw configuration dictionary for custom fields.
    """

    environment: str
    database: DatabaseConfig
    services: ServicesConfig
    raw: dict[str, Any]


class ConfigLoader:
    """Loads and validates environment-specific YAML configuration.

    The loader reads configuration from YAML files based on the APP_ENV
    environment variable. This ensures proper test/production separation
    as each environment has its own configuration file.

    Attributes:
        config_dir: Path to the directory containing configuration files.
        environment: The current environment name.
    """

    def __init__(
        self,
        config_dir: str | Path | None = None,
        environment: str | None = None,
    ) -> None:
        """Initialize the configuration loader.

        Args:
            config_dir: Path to config directory. Defaults to 'config/' relative
                       to the current working directory.
            environment: Override for APP_ENV. If None, reads from environment
                        variable or defaults to 'development'.
        """
        if config_dir is None:
            self.config_dir = Path.cwd() / "config"
        else:
            self.config_dir = Path(config_dir)

        self.environment = self._resolve_environment(environment)

    def _resolve_environment(self, override: str | None) -> str:
        """Resolve the environment name from override, env var, or default.

        Args:
            override: Explicit environment override.

        Returns:
            The resolved environment name.

        Raises:
            InvalidConfigError: If the environment name is not valid.
        """
        if override is not None:
            env = override
        else:
            env = os.getenv("APP_ENV", DEFAULT_ENVIRONMENT)

        env = env.lower().strip()

        if env not in VALID_ENVIRONMENTS:
            raise InvalidConfigError(
                f"Invalid environment '{env}'. Must be one of: {', '.join(sorted(VALID_ENVIRONMENTS))}"
            )

        return env

    def _get_config_path(self) -> Path:
        """Get the path to the configuration file for the current environment.

        Returns:
            Path to the YAML configuration file.
        """
        return self.config_dir / f"{self.environment}.yaml"

    def _load_yaml(self) -> dict[str, Any]:
        """Load the YAML configuration file.

        Returns:
            The parsed YAML configuration as a dictionary.

        Raises:
            ConfigNotFoundError: If the configuration file does not exist.
            InvalidConfigError: If the YAML is malformed.
        """
        config_path = self._get_config_path()

        if not config_path.exists():
            raise ConfigNotFoundError(self.environment, str(config_path))

        try:
            with open(config_path, "r", encoding="utf-8") as f:
                data = yaml.safe_load(f)
        except yaml.YAMLError as e:
            raise InvalidConfigError(f"Invalid YAML in {config_path}: {e}")

        if data is None:
            raise InvalidConfigError(f"Configuration file is empty: {config_path}")

        if not isinstance(data, dict):
            raise InvalidConfigError(
                f"Configuration must be a YAML mapping, got {type(data).__name__}"
            )

        return data

    def _validate_database_config(self, db_config: dict[str, Any] | None) -> None:
        """Validate that required database fields are present.

        Args:
            db_config: The database configuration section.

        Raises:
            InvalidConfigError: If required fields are missing.
        """
        if db_config is None:
            raise InvalidConfigError(
                "Missing 'database' section in configuration",
                missing_fields=["database"],
            )

        if not isinstance(db_config, dict):
            raise InvalidConfigError(
                f"'database' must be a mapping, got {type(db_config).__name__}"
            )

        missing = [
            field for field in REQUIRED_DATABASE_FIELDS if field not in db_config
        ]

        if missing:
            raise InvalidConfigError(
                f"Missing required database fields: {', '.join(missing)}",
                missing_fields=missing,
            )

    def _parse_database_config(self, db_config: dict[str, Any]) -> DatabaseConfig:
        """Parse database configuration into a DatabaseConfig object.

        Args:
            db_config: The database configuration dictionary.

        Returns:
            A DatabaseConfig instance.

        Raises:
            InvalidConfigError: If port is not a valid integer.
        """
        try:
            port = int(db_config["port"])
        except (ValueError, TypeError):
            raise InvalidConfigError(
                f"Database port must be an integer, got: {db_config['port']}"
            )

        return DatabaseConfig(
            host=str(db_config["host"]),
            port=port,
            name=str(db_config["name"]),
            user=db_config.get("user"),
            password=db_config.get("password"),
        )

    def _parse_services_config(
        self, services_config: dict[str, Any] | None
    ) -> ServicesConfig:
        """Parse services configuration into a ServicesConfig object.

        Args:
            services_config: The services configuration dictionary (optional).

        Returns:
            A ServicesConfig instance.
        """
        if services_config is None or not isinstance(services_config, dict):
            return ServicesConfig()

        return ServicesConfig(
            api=services_config.get("api"),
            auth=services_config.get("auth"),
            storage=services_config.get("storage"),
        )

    def load(self) -> Config:
        """Load and validate the configuration for the current environment.

        Returns:
            A Config instance with validated configuration.

        Raises:
            ConfigNotFoundError: If the configuration file does not exist.
            InvalidConfigError: If the configuration is invalid.
        """
        raw = self._load_yaml()

        db_config = raw.get("database")
        self._validate_database_config(db_config)

        # After validation, db_config is guaranteed to be a dict
        assert isinstance(db_config, dict)  # Type narrowing for static analysis
        database = self._parse_database_config(db_config)
        services = self._parse_services_config(raw.get("services"))

        return Config(
            environment=raw.get("environment", self.environment),
            database=database,
            services=services,
            raw=raw,
        )
