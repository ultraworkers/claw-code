# frozen_string_literal: true

# Custom error classes for ConfigLoader
#
# This module defines domain-specific exceptions for configuration loading,
# following guardrails-compliant patterns for error handling.
module ConfigLoader
  # Base error class for all configuration-related errors
  class Error < StandardError; end

  # Raised when a configuration file cannot be found
  #
  # @example
  #   raise ConfigNotFoundError.new('production')
  #   # => ConfigLoader::ConfigNotFoundError: Configuration file not found for environment: production
  class ConfigNotFoundError < Error
    attr_reader :environment

    # @param environment [String] the environment name that was requested
    def initialize(environment)
      @environment = environment
      super("Configuration file not found for environment: #{environment}")
    end
  end

  # Raised when a configuration file contains invalid content
  #
  # @example
  #   raise InvalidConfigError.new('production', 'Missing required key: database')
  #   # => ConfigLoader::InvalidConfigError: Invalid configuration for environment: production - Missing required key: database
  class InvalidConfigError < Error
    attr_reader :environment, :reason

    # @param environment [String] the environment name
    # @param reason [String] description of what is invalid
    def initialize(environment, reason)
      @environment = environment
      @reason = reason
      super("Invalid configuration for environment: #{environment} - #{reason}")
    end
  end
end
