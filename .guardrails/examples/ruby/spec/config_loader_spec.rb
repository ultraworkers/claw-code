# frozen_string_literal: true

# ConfigLoader Spec
#
# This test file demonstrates guardrails-compliant testing patterns:
# - Tests written AFTER production code
# - Clear describe/context/it structure
# - Environment isolation
# - Edge case coverage
# - No production database connections

require 'spec_helper'
require 'tempfile'
require 'fileutils'

RSpec.describe ConfigLoader do
  let(:config_dir) { File.expand_path('../config', __dir__) }

  describe '.load' do
    context 'when loading production configuration' do
      it 'loads production config successfully' do
        config = described_class.load(environment: 'production', config_dir: config_dir)

        expect(config.app_name).to eq('ConfigLoader Example')
        expect(config.version).to eq('1.0.0')
      end

      it 'returns production database settings' do
        config = described_class.load(environment: 'production', config_dir: config_dir)

        expect(config.database.host).to eq('prod-db.example.com')
        expect(config.database.port).to eq(5432)
        expect(config.database.ssl).to be true
      end

      it 'returns production feature flags' do
        config = described_class.load(environment: 'production', config_dir: config_dir)

        expect(config.features.rate_limiting).to be true
        expect(config.features.debug_mode).to be false
      end
    end

    context 'when loading test configuration' do
      it 'loads test config successfully' do
        config = described_class.load(environment: 'test', config_dir: config_dir)

        expect(config.app_name).to eq('ConfigLoader Example')
        expect(config.version).to eq('1.0.0-test')
      end

      it 'returns test database settings with localhost' do
        config = described_class.load(environment: 'test', config_dir: config_dir)

        expect(config.database.host).to eq('localhost')
        expect(config.database.name).to eq('app_test')
        expect(config.database.ssl).to be false
      end

      it 'disables production features in test' do
        config = described_class.load(environment: 'test', config_dir: config_dir)

        expect(config.features.rate_limiting).to be false
        expect(config.features.analytics).to be false
        expect(config.features.debug_mode).to be true
      end
    end

    context 'when loading development configuration' do
      it 'loads development config successfully' do
        config = described_class.load(environment: 'development', config_dir: config_dir)

        expect(config.version).to eq('1.0.0-dev')
      end

      it 'returns development database settings' do
        config = described_class.load(environment: 'development', config_dir: config_dir)

        expect(config.database.host).to eq('localhost')
        expect(config.database.name).to eq('app_development')
      end
    end

    context 'when using APP_ENV environment variable' do
      it 'reads from APP_ENV when environment not specified' do
        ENV['APP_ENV'] = 'production'

        config = described_class.load(config_dir: config_dir)

        expect(config.database.host).to eq('prod-db.example.com')
      end

      it 'defaults to development when APP_ENV is not set' do
        ENV.delete('APP_ENV')

        config = described_class.load(config_dir: config_dir)

        expect(config.database.name).to eq('app_development')
      end

      it 'handles APP_ENV with different casing' do
        ENV['APP_ENV'] = 'PRODUCTION'

        config = described_class.load(config_dir: config_dir)

        expect(config.database.host).to eq('prod-db.example.com')
      end

      it 'handles APP_ENV with whitespace' do
        ENV['APP_ENV'] = '  test  '

        config = described_class.load(config_dir: config_dir)

        expect(config.database.name).to eq('app_test')
      end

      it 'explicit environment parameter overrides APP_ENV' do
        ENV['APP_ENV'] = 'production'

        config = described_class.load(environment: 'test', config_dir: config_dir)

        expect(config.database.name).to eq('app_test')
      end
    end

    context 'when config file is missing' do
      it 'raises ConfigNotFoundError for non-existent environment' do
        expect do
          described_class.load(environment: 'staging', config_dir: config_dir)
        end.to raise_error(ConfigLoader::ConfigNotFoundError) do |error|
          expect(error.environment).to eq('staging')
          expect(error.message).to include('staging')
        end
      end

      it 'raises ConfigNotFoundError for missing config directory' do
        expect do
          described_class.load(environment: 'production', config_dir: '/nonexistent/path')
        end.to raise_error(ConfigLoader::ConfigNotFoundError)
      end
    end

    context 'when config file is invalid' do
      let(:temp_dir) { Dir.mktmpdir }

      after do
        FileUtils.remove_entry(temp_dir) if File.exist?(temp_dir)
      end

      it 'raises InvalidConfigError for malformed YAML' do
        File.write(File.join(temp_dir, 'test.yaml'), "invalid: yaml: content:\n  - broken")

        expect do
          described_class.load(environment: 'test', config_dir: temp_dir)
        end.to raise_error(ConfigLoader::InvalidConfigError) do |error|
          expect(error.environment).to eq('test')
          expect(error.message).to include('YAML syntax error')
        end
      end

      it 'raises InvalidConfigError for non-hash YAML' do
        File.write(File.join(temp_dir, 'test.yaml'), "- item1\n- item2")

        expect do
          described_class.load(environment: 'test', config_dir: temp_dir)
        end.to raise_error(ConfigLoader::InvalidConfigError) do |error|
          expect(error.reason).to include('must be a YAML hash')
        end
      end

      it 'raises InvalidConfigError when required keys are missing' do
        File.write(File.join(temp_dir, 'test.yaml'), "app_name: Test\nversion: 1.0")

        expect do
          described_class.load(environment: 'test', config_dir: temp_dir)
        end.to raise_error(ConfigLoader::InvalidConfigError) do |error|
          expect(error.reason).to include('Missing required keys')
          expect(error.reason).to include('database')
        end
      end

      it 'raises InvalidConfigError for empty config file' do
        File.write(File.join(temp_dir, 'test.yaml'), '')

        expect do
          described_class.load(environment: 'test', config_dir: temp_dir)
        end.to raise_error(ConfigLoader::InvalidConfigError)
      end
    end

    context 'when config has nested structures' do
      it 'allows dot notation access to nested values' do
        config = described_class.load(environment: 'production', config_dir: config_dir)

        expect(config.cache.host).to eq('prod-cache.example.com')
        expect(config.cache.ttl).to eq(3600)
        expect(config.logging.level).to eq('info')
        expect(config.logging.format).to eq('json')
      end
    end
  end

  describe '.current_environment' do
    it 'returns the current environment from APP_ENV' do
      ENV['APP_ENV'] = 'production'

      expect(described_class.current_environment).to eq('production')
    end

    it 'returns development when APP_ENV is not set' do
      ENV.delete('APP_ENV')

      expect(described_class.current_environment).to eq('development')
    end

    it 'normalizes environment name to lowercase' do
      ENV['APP_ENV'] = 'PRODUCTION'

      expect(described_class.current_environment).to eq('production')
    end
  end

  describe ConfigLoader::ConfigNotFoundError do
    it 'includes environment in error message' do
      error = described_class.new('staging')

      expect(error.message).to eq('Configuration file not found for environment: staging')
      expect(error.environment).to eq('staging')
    end

    it 'inherits from ConfigLoader::Error' do
      error = described_class.new('test')

      expect(error).to be_a(ConfigLoader::Error)
      expect(error).to be_a(StandardError)
    end
  end

  describe ConfigLoader::InvalidConfigError do
    it 'includes environment and reason in error message' do
      error = described_class.new('production', 'Missing database key')

      expect(error.message).to eq('Invalid configuration for environment: production - Missing database key')
      expect(error.environment).to eq('production')
      expect(error.reason).to eq('Missing database key')
    end

    it 'inherits from ConfigLoader::Error' do
      error = described_class.new('test', 'Invalid format')

      expect(error).to be_a(ConfigLoader::Error)
      expect(error).to be_a(StandardError)
    end
  end
end
