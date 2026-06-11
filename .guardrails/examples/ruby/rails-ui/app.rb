# frozen_string_literal: true

# Rails 8.0+ Hotwire Architecture Example
# Demonstrates ViewComponent, Turbo Streams, and Stimulus patterns
# for game admin dashboards with ethical engagement features

require 'bundler/inline'

gem 'rails', '~> 8.0'
gem 'view_component', '~> 3.0'
gem 'redis', '~> 5.0'

# =============================================================================
# CORE APPLICATION CONFIGURATION
# =============================================================================

module GameAdmin
  # Rails 8 application demonstrating Hotwire architecture patterns
  #
  # This module provides:
  # - Component-based UI architecture via ViewComponent
  # - Real-time updates via Turbo Streams
  # - Interactive behaviors via Stimulus controllers
  # - Ethical engagement features (transparent odds, rest-state mechanics)
  #
  # @example Mount in config/routes.rb
  #   mount GameAdmin::Application, at: '/admin'
  class Application
    include Rails.application.config
  end

  # =============================================================================
  # VIEW COMPONENT BASE
  # =============================================================================

  # Base ViewComponent with accessibility and ethical engagement patterns
  #
  # All admin dashboard components inherit from this base class
  #
  # @security Brakeman scans for permit! usage (Mass Assignment protection)
  # @test RSpec unit tests for component rendering
  class AdminComponent < ViewComponent::Base
    # Default accessibility attributes
    def aria_label
      @aria_label || self.class.name.downcase.demodulize
    end

    # Ethical engagement: display transparent odds information
    #
    # @param drop_table [Hash] loot table with odds
    # @return [String] formatted odds display
    def render_loot_odts(drop_table)
      content_tag :div, class: 'loot-odds', aria: {
        label: 'Drop rates and expected value'
      } do
        concat content_tag(:p, 'Transparent Drop Rates')
        concat render_drop_rates(drop_table)
      end
    end

    # Rest-State Mechanics: offline regeneration indicator
    #
    # Shows player's offline resource accumulation
    #
    # @param player_id [Integer] player identifier
    # @return [String] rest-state display
    def render_rest_state(player_id)
      offline_resources = ::PlayerOfflineResources.find_by(player_id: player_id)

      return '' unless offline_resources

      content_tag :div, class: 'rest-state', data: {
        stimulus_target: 'restState',
        player_id: player_id
      } do
        concat content_tag(:p, "Offline Regeneration")
        concat content_tag(:span, offline_resources.resources_gained.to_s)
      end
    end

    private

    # Render individual drop rates with expected value
    def render_drop_rates(drop_table)
      items = drop_table.items.map do |item|
        {
          name: item.name,
          rate: item.drop_rate,
          expected_value: item.market_value * item.drop_rate
        }
      end

      items.sort_by { |i| -i.expected_value }.map do |item|
        content_tag(:div, class: 'drop-rate-row') do
          concat content_tag(:span, item.name, class: 'item-name')
          concat content_tag(:span, "#{item.rate}%", class: 'drop-rate')
          concat content_tag(:span, "EV: #{item.expected_value}", class: 'expected-value')
        end
      end.join
    end
  end

  # =============================================================================
  # TURBO STREAMS SERVICE
  # =============================================================================

  # Real-time dashboard updates via Turbo Streams
  #
  # Provides WebSocket-based live updates for:
  # - Player analytics
  # - Economy monitoring
  # - Moderation alerts
  #
  # @dependency Redis pub/sub for stream broadcasting
  module TurboStreams
    class DashboardBroadcaster
      def initialize(channel: 'dashboard')
        @channel = channel
        @redis = Redis.new
      end

      # Broadcast player metric update
      #
      # @param metric [String] metric name
      # @param value [Float] current value
      # @param timestamp [Time] update time
      def broadcast_metric(metric:, value:, timestamp: Time.current)
        payload = {
          metric: metric,
          value: value,
          timestamp: timestamp.iso8601
        }

        @redis.publish(@channel, ActiveSupport::JSON.encode(payload))
      end

      # Broadcast economy faucet/sink event
      #
      # @param event_type [String] :faucet or :sink
      # @param amount [Integer] resource amount
      # @param source [String] event source
      def broadcast_economy_event(event_type:, amount:, source:)
        payload = {
          event_type: event_type,
          amount: amount,
          source: source,
          timestamp: Time.current.iso8601
        }

        @redis.publish('economy', ActiveSupport::JSON.encode(payload))
      end

      # Broadcast moderation alert
      #
      # @param alert_type [String] alert classification
      # @param player_id [Integer] flagged player
      # @param details [Hash] alert context
      def broadcast_alert(alert_type:, player_id:, details:)
        payload = {
          alert_type: alert_type,
          player_id: player_id,
          details: details,
          timestamp: Time.current.iso8601
        }

        @redis.publish('alerts', ActiveSupport::JSON.encode(payload))
      end
    end
  end

  # =============================================================================
  # STIMulus CONTROLLER BASE
  # =============================================================================

  # Stimulus controller for interactive admin dashboard
  #
  # Handles:
  # - Real-time chart updates
  # - Filter/sort operations
  # - Accessibility keyboard shortcuts
  #
  # @example HTML usage
  #   <div data-controller="dashboard" data-dashboard-refresh-interval="30">
  class DashboardController
    # Stimulus controller pattern for vanilla JavaScript
    # This serves as documentation for the frontend implementation

    # Static method reference for Stimulus registration
    def self.register
      {
        name: 'dashboard',
        targets: ['metrics', 'alerts', 'filters'],
        actions: {
          'click->refresh',
          'keydown->handleShortcut',
          'turbo:load->reconnect'
        },
        values: {
          refreshInterval: Number,
          playerId: String,
          timeZone: String
        }
      }
    end

    # Accessibility: keyboard shortcut handler
    #
    # @param event [KeyboardEvent] keydown event
    # @note Ctrl+M: Toggle metrics panel
    # @note Ctrl+A: Show alerts
    # @note Ctrl+F: Focus filter
    def handleShortcut(event)
      return unless event.ctrlKey

      case event.key
      when 'm'
        toggle_metrics_panel
      when 'a'
        show_alerts_panel
      when 'f'
        focus_filter_input
      end

      event.preventDefault
    end

    # Refresh dashboard data via Turbo Frame
    def refresh
      # Trigger Turbo Frame refresh
      # Implementation in corresponding JS file
    end

    private

    def toggle_metrics_panel
      # Toggle visibility of metrics panel
      # Maintain focus state for accessibility
    end

    def show_alerts_panel
      # Display moderation alerts
      # Clear unread counter
    end

    def focus_filter_input
      # Focus filter input field
      # Set aria-expanded attribute
    end
  end

  # =============================================================================
  # CONFIGURATION SERVICE
  # =============================================================================

  # Environment-aware configuration loader
  #
  # Loads from examples/ruby/config/{development,test,production}.yaml
  #
  # @see ConfigLoader for base implementation
  module Configuration
    class DashboardConfig
      VALID_ENVIRONMENTS = %w[production test development].freeze

      def self.load(environment: nil, config_dir: nil)
        env = resolve_environment(environment)
        dir = config_dir || default_config_dir
        file_path = File.join(dir, "#{env}.yaml")

        raise ConfigNotFoundError, environment unless File.exist?(file_path)

        config = YAML.safe_load(File.read(file_path), permitted_classes: [Symbol])

        validate_config!(config, env)

        OpenStruct.new(config)
      end

      private

      def self.resolve_environment(env)
        env || ENV.fetch('APP_ENV', 'development')
      end

      def self.default_config_dir
        File.expand_path('../../../config', __dir__)
      end

      def self.validate_config!(config, env)
        required = %w[app_name version database]
        missing = required - config.keys.map(&:to_s)
        return if missing.empty?

        raise InvalidConfigError.new(env, "Missing: #{missing.join(', ')}")
      end
    end

    ConfigNotFoundError = Class.new(StandardError)
    InvalidConfigError = Class.new(StandardError)
  end
end

# =============================================================================
# RAILS INITIALIZATION
# =============================================================================

# Initialize Rails application with Hotwire integration
# This demonstrates the complete stack setup

Rails.application.initialize!

# Mount ViewComponent helpers
ActionView::Base.include ViewComponent::Helpers

# Configure Turbo Streams adapter
ActionCable.server.config.pubsub_adapter = :redis

# Register Stimulus controllers
# In production: config/javascript_pack_tags.rb

puts 'Rails 8 Hotwire application initialized'