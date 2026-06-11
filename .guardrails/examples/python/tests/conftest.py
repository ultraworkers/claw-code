"""
Pytest fixtures for config_loader tests.

These fixtures provide isolated test environments that never touch production.
Each test gets its own temporary configuration directory.
"""

import os
from pathlib import Path
from typing import Generator

import pytest


@pytest.fixture
def temp_config_dir(tmp_path: Path) -> Path:
    """Create a temporary config directory for test isolation.

    This ensures tests never accidentally read production configs.
    """
    config_dir = tmp_path / "config"
    config_dir.mkdir()
    return config_dir


@pytest.fixture
def valid_development_config(temp_config_dir: Path) -> Path:
    """Create a valid development configuration file."""
    config_file = temp_config_dir / "development.yaml"
    config_file.write_text(
        """
environment: development
database:
  host: localhost
  port: 5432
  name: dev_db
  user: dev_user
  password: dev_password
services:
  api: http://localhost:8080
app:
  debug: true
"""
    )
    return config_file


@pytest.fixture
def valid_test_config(temp_config_dir: Path) -> Path:
    """Create a valid test configuration file."""
    config_file = temp_config_dir / "test.yaml"
    config_file.write_text(
        """
environment: test
database:
  host: test-db.example.com
  port: 5432
  name: test_db
  user: test_user
services:
  api: https://api.test.example.com
  auth: https://auth.test.example.com
"""
    )
    return config_file


@pytest.fixture
def valid_production_config(temp_config_dir: Path) -> Path:
    """Create a valid production configuration file."""
    config_file = temp_config_dir / "production.yaml"
    config_file.write_text(
        """
environment: production
database:
  host: prod-db.example.com
  port: 5432
  name: production_db
  user: prod_user
services:
  api: https://api.production.example.com
  auth: https://auth.production.example.com
  storage: https://storage.production.example.com
"""
    )
    return config_file


@pytest.fixture
def all_configs(
    valid_development_config: Path,
    valid_test_config: Path,
    valid_production_config: Path,
) -> dict[str, Path]:
    """Create all environment configuration files."""
    return {
        "development": valid_development_config,
        "test": valid_test_config,
        "production": valid_production_config,
    }


@pytest.fixture
def missing_database_config(temp_config_dir: Path) -> Path:
    """Create a config file missing the database section."""
    config_file = temp_config_dir / "development.yaml"
    config_file.write_text(
        """
environment: development
services:
  api: http://localhost:8080
"""
    )
    return config_file


@pytest.fixture
def incomplete_database_config(temp_config_dir: Path) -> Path:
    """Create a config file with incomplete database settings."""
    config_file = temp_config_dir / "development.yaml"
    config_file.write_text(
        """
environment: development
database:
  host: localhost
  # missing port and name
"""
    )
    return config_file


@pytest.fixture
def invalid_port_config(temp_config_dir: Path) -> Path:
    """Create a config file with invalid database port."""
    config_file = temp_config_dir / "development.yaml"
    config_file.write_text(
        """
environment: development
database:
  host: localhost
  port: not_a_number
  name: dev_db
"""
    )
    return config_file


@pytest.fixture
def empty_config(temp_config_dir: Path) -> Path:
    """Create an empty configuration file."""
    config_file = temp_config_dir / "development.yaml"
    config_file.write_text("")
    return config_file


@pytest.fixture
def malformed_yaml_config(temp_config_dir: Path) -> Path:
    """Create a malformed YAML configuration file."""
    config_file = temp_config_dir / "development.yaml"
    config_file.write_text(
        """
environment: development
database:
  host: localhost
  port: 5432
  name: dev_db
  invalid yaml here: [unclosed bracket
"""
    )
    return config_file


@pytest.fixture
def clean_environment() -> Generator[None, None, None]:
    """Ensure APP_ENV is not set during test, then restore original value."""
    original_value = os.environ.get("APP_ENV")

    # Remove APP_ENV for the test
    if "APP_ENV" in os.environ:
        del os.environ["APP_ENV"]

    yield

    # Restore original value
    if original_value is not None:
        os.environ["APP_ENV"] = original_value
    elif "APP_ENV" in os.environ:
        del os.environ["APP_ENV"]


@pytest.fixture
def set_app_env():
    """Factory fixture to set APP_ENV and restore after test."""
    original_value = os.environ.get("APP_ENV")

    def _set_env(value: str) -> None:
        os.environ["APP_ENV"] = value

    yield _set_env

    # Restore original value
    if original_value is not None:
        os.environ["APP_ENV"] = original_value
    elif "APP_ENV" in os.environ:
        del os.environ["APP_ENV"]
