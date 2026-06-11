"""
Custom exceptions for configuration loading.

These exceptions provide clear, actionable error messages for configuration issues.
"""


class ConfigError(Exception):
    """Base exception for all configuration errors."""

    pass


class ConfigNotFoundError(ConfigError):
    """Raised when a configuration file cannot be found.

    Attributes:
        environment: The environment name that was requested.
        config_path: The path that was searched.
    """

    def __init__(self, environment: str, config_path: str) -> None:
        self.environment = environment
        self.config_path = config_path
        super().__init__(
            f"Configuration file not found for environment '{environment}': {config_path}"
        )


class InvalidConfigError(ConfigError):
    """Raised when configuration is invalid or missing required fields.

    Attributes:
        message: Description of what is invalid.
        missing_fields: List of fields that are missing (if applicable).
    """

    def __init__(
        self, message: str, missing_fields: list[str] | None = None
    ) -> None:
        self.message = message
        self.missing_fields = missing_fields or []
        super().__init__(message)
