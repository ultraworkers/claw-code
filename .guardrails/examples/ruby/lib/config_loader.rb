# frozen_string_literal: true

require 'yaml'
require 'ostruct'
require_relative 'config_loader/errors'

# ConfigLoader provides environment-aware configuration loading from YAML files.
#
# This class demonstrates guardrails-compliant patterns:
# - Environment-based configuration separation
# - Clear error handling with custom exceptions
# - Immutable configuration objects via OpenStruct
#
# @example Basic usage
#   ENV['APP_ENV'] = 'production'
#   config = ConfigLoader.load
#   config.database.host  # => 'prod-db.example.com'
#
# @example Explicit environment
#   config = ConfigLoader.load(environment: 'test')
#   config.database.host  # => 'localhost'
#
class ConfigLoader
  # Valid environment names
  VALID_ENVIRONMENTS = %w[production test development].freeze

  # Default environment when APP_ENV is not set
  DEFAULT_ENVIRONMENT = 'development'

  # Required top-level configuration keys
  REQUIRED_KEYS = %w[app_name version database].freeze

  class << self
    # Load configuration for the specified or current environment
    #
    # @param environment [String, nil] override environment (defaults to APP_ENV)
    # @param config_dir [String, nil] custom config directory path
    # @return [OpenStruct] configuration object with nested accessors
    # @raise [ConfigNotFoundError] if config file does not exist
    # @raise [InvalidConfigError] if config file is invalid or missing required keys
    def load(environment: nil, config_dir: nil)
      env = resolve_environment(environment)
      dir = config_dir || default_config_dir
      file_path = File.join(dir, "#{env}.yaml")

      validate_file_exists!(file_path, env)
      config_hash = load_yaml_file(file_path, env)
      validate_config!(config_hash, env)

      hash_to_openstruct(config_hash)
    end

    # Returns the current environment based on APP_ENV
    #
    # @return [String] current environment name
    def current_environment
      resolve_environment(nil)
    end

    private

    # Resolve the environment from parameter or ENV
    #
    # @param environment [String, nil] explicit environment or nil
    # @return [String] resolved environment name
    def resolve_environment(environment)
      env = environment || ENV.fetch('APP_ENV', DEFAULT_ENVIRONMENT)
      env.to_s.downcase.strip
    end

    # Default configuration directory path
    #
    # @return [String] path to config directory
    def default_config_dir
      File.expand_path('../config', __dir__)
    end

    # Validate that the configuration file exists
    #
    # @param file_path [String] path to config file
    # @param environment [String] environment name for error reporting
    # @raise [ConfigNotFoundError] if file does not exist
    def validate_file_exists!(file_path, environment)
      return if File.exist?(file_path)

      raise ConfigLoader::ConfigNotFoundError, environment
    end

    # Load and parse YAML file
    #
    # @param file_path [String] path to YAML file
    # @param environment [String] environment name for error reporting
    # @return [Hash] parsed configuration hash
    # @raise [InvalidConfigError] if YAML is malformed
    def load_yaml_file(file_path, environment)
      content = File.read(file_path)
      parsed = YAML.safe_load(content, permitted_classes: [Symbol])

      unless parsed.is_a?(Hash)
        raise ConfigLoader::InvalidConfigError.new(
          environment,
          'Configuration must be a YAML hash/mapping'
        )
      end

      parsed
    rescue Psych::SyntaxError => e
      raise ConfigLoader::InvalidConfigError.new(
        environment,
        "YAML syntax error: #{e.message}"
      )
    end

    # Validate configuration has all required keys
    #
    # @param config [Hash] configuration hash
    # @param environment [String] environment name for error reporting
    # @raise [InvalidConfigError] if required keys are missing
    def validate_config!(config, environment)
      missing_keys = REQUIRED_KEYS - config.keys.map(&:to_s)

      return if missing_keys.empty?

      raise ConfigLoader::InvalidConfigError.new(
        environment,
        "Missing required keys: #{missing_keys.join(', ')}"
      )
    end

    # Recursively convert a hash to OpenStruct for dot notation access
    #
    # @param hash [Hash] hash to convert
    # @return [OpenStruct] nested OpenStruct object
    def hash_to_openstruct(hash)
      OpenStruct.new(
        hash.transform_values do |value|
          case value
          when Hash
            hash_to_openstruct(value)
          when Array
            value.map { |item| item.is_a?(Hash) ? hash_to_openstruct(item) : item }
          else
            value
          end
        end
      )
    end
  end
end
