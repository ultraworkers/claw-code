# frozen_string_literal: true

# RSpec Configuration
#
# This file demonstrates guardrails-compliant test setup:
# - Clear separation from production code
# - Isolated test environment
# - No production dependencies in test infrastructure

require 'bundler/setup'
Bundler.require(:default, :test)

# Add lib to load path
$LOAD_PATH.unshift File.expand_path('../lib', __dir__)

require 'config_loader'

RSpec.configure do |config|
  # Enable flags like --only-failures and --next-failure
  config.example_status_persistence_file_path = '.rspec_status'

  # Disable RSpec exposing methods globally on `Module` and `main`
  config.disable_monkey_patching!

  # Use expect syntax exclusively
  config.expect_with :rspec do |c|
    c.syntax = :expect
  end

  # Run specs in random order to surface order dependencies
  config.order = :random
  Kernel.srand config.seed

  # Clean environment before each test
  config.before(:each) do
    # Store original APP_ENV to restore after tests
    @original_app_env = ENV['APP_ENV']
  end

  config.after(:each) do
    # Restore original APP_ENV after each test
    if @original_app_env
      ENV['APP_ENV'] = @original_app_env
    else
      ENV.delete('APP_ENV')
    end
  end
end
