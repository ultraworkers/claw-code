# frozen_string_literal: true

# Generative World Builder Component
# Rails 8 ViewComponent for world-building parameter editor
#
# Features:
# - Procedural generation parameter controls
# - Terrain, climate, and resource distribution editors
# - Real-time preview via Turbo Frames
# - Accessibility-first parameter adjustment
#
# @use Stimulus for interactive sliders and preview updates
# @test RSpec component tests
# @accessibility WCAG AA, keyboard navigation, ARIA live regions

module GameAdmin
  class WorldBuilderComponent < AdminComponent
    # World builder component for procedural generation
    #
    # Provides:
    # - Terrain parameter controls (height, erosion, biome)
    # - Climate system editor (temperature, precipitation)
    # - Resource distribution (ores, vegetation, settlements)
    # - Real-time preview rendering
    #
    # @param world_id [Integer] world to edit
    # @param params [Hash] generation parameters
    # @return [String] rendered HTML component
    def render(world_id:, params: {})
      @world_id = world_id
      @params = params.permit(
        :terrain_scale, :terrain_erosion, :biome_distribution,
        :climate_temperature, :climate_precipitation,
        :resource_density, :settlement_frequency
      )

      @world = World.find(world_id)

      content_tag :div, class: 'world-builder', data: {
        controller: 'world-builder',
        world_builder_world_id: world_id,
        world_builder_preview_url: '/admin/worlds/preview'
      }, aria: {
        label: 'World Building Editor',
        role: 'application'
      } do
        concat render_header
        concat render_parameter_panel
        concat render_preview_panel
        concat render_generation_controls
      end
    end

    # Builder header with world info
    def render_header
      content_tag :header, class: 'builder-header' do
        concat content_tag(:h1, 'World Builder', class: 'builder-title')
        concat render_world_info(@world)
        concat render_actions
      end
    end

    # World information display
    def render_world_info(world)
      content_tag :div, class: 'world-info', aria: {
        label: 'World information'
      } do
        concat content_tag(:h2, world.name, class: 'world-name')
        concat content_tag(:p, "#{world.biomes.count} biomes", class: 'world-stat')
        concat content_tag(:p, "#{world.size_km} km²", class: 'world-stat')
        concat content_tag(:p, "Seed: #{world.seed}", class: 'world-seed')
      end
    end

    # Action buttons (Save, Export, Regenerate)
    def render_actions
      content_tag :div, class: 'builder-actions' do
        concat link_to('Save', "/admin/worlds/#{@world_id}", class: 'btn-primary')
        concat link_to('Export', "/admin/worlds/#{@world_id}/export", class: 'btn-secondary')
        concat link_to('Regenerate', "/admin/worlds/#{@world_id}/regenerate",
          class: 'btn-warning',
          data: {
            confirm: 'Regenerate will erase current world. Continue?'
          }
        )
      end
    end

    # Parameter panel with all generation controls
    def render_parameter_panel
      content_tag :section, class: 'parameter-panel', aria: {
        label: 'Generation Parameters',
        role: 'region'
      }, data: {
        world_builder_target: 'parameters'
      } do
        concat content_tag(:h2, 'Generation Parameters', class: 'panel-title')

        # Terrain parameters
        concat render_terrain_section
        concat render_climate_section
        concat render_biome_section
        concat render_resource_section
      end
    end

    # Terrain parameter controls
    def render_terrain_section
      content_tag :div, class: 'parameter-section terrain', data: {
        section: 'terrain'
      } do
        concat content_tag(:h3, 'Terrain', class: 'section-title')

        concat render_slider_control(
          id: 'terrain_scale',
          label: 'Scale',
          min: 0, max: 100,
          value: @params[:terrain_scale] || 50,
          description: 'Feature size and frequency'
        )

        concat render_slider_control(
          id: 'terrain_erosion',
          label: 'Erosion',
          min: 0, max: 100,
          value: @params[:terrain_erosion] || 30,
          description: 'Weathering and sedimentation'
        )

        concat render_slider_control(
          id: 'terrain_ruggedness',
          label: 'Ruggedness',
          min: 0, max: 100,
          value: @params[:terrain_ruggedness] || 40,
          description: 'Height variation intensity'
        )
      end
    end

    # Climate parameter controls
    def render_climate_section
      content_tag :div, class: 'parameter-section climate', data: {
        section: 'climate'
      } do
        concat content_tag(:h3, 'Climate', class: 'section-title')

        concat render_slider_control(
          id: 'climate_temperature',
          label: 'Base Temperature',
          min: -50, max: 50,
          value: @params[:climate_temperature] || 20,
          unit: '°C',
          description: 'Global temperature baseline'
        )

        concat render_slider_control(
          id: 'climate_precipitation',
          label: 'Precipitation',
          min: 0, max: 200,
          value: @params[:climate_precipitation] || 100,
          unit: 'mm/year',
          description: 'Annual rainfall distribution'
        )

        concat render_slider_control(
          id: 'climate_seasonality',
          label: 'Seasonality',
          min: 0, max: 100,
          value: @params[:climate_seasonality] || 50,
          description: 'Seasonal variation intensity'
        )
      end
    end

    # Biome distribution controls
    def render_biome_section
      content_tag :div, class: 'parameter-section biome', data: {
        section: 'biome'
      } do
        concat content_tag(:h3, 'Biome Distribution', class: 'section-title')

        concat render_biome_picker
        concat render_vegetation_density_control
      end
    end

    # Biome type picker with accessibility
    def render_biome_picker
      content_tag :div, class: 'biome-picker', role: 'group', aria: {
        label: 'Biome types'
      } do
        BiomeType.all.each do |biome|
          concat content_tag :div, class: 'biome-option' do
            concat tag.label(class: 'biome-label') do
              concat tag.checkbox_tag(
                'biomes',
                value: biome.id,
                checked: @params[:biome_distribution]&.include?(biome.id.to_s)
              )
              concat content_tag(:span, biome.name, class: 'biome-name')
              concat content_tag(:span, biome.icon, class: 'biome-icon', aria: {
                hidden: true
              })
            end
          end
        end
      end
    end

    # Vegetation density slider
    def render_vegetation_density_control
      render_slider_control(
        id: 'vegetation_density',
        label: 'Vegetation Density',
        min: 0, max: 100,
        value: @params[:vegetation_density] || 60,
        description: 'Plant coverage percentage'
      )
    end

    # Resource distribution controls
    def render_resource_section
      content_tag :div, class: 'parameter-section resource', data: {
        section: 'resource'
      } do
        concat content_tag(:h3, 'Resource Distribution', class: 'section-title')

        concat render_slider_control(
          id: 'resource_density',
          label: 'Resource Density',
          min: 0, max: 100,
          value: @params[:resource_density] || 40,
          description: 'Ore and material spawn rate'
        )

        concat render_slider_control(
          id: 'resource_rarity',
          label: 'Rarity Gradient',
          min: 0, max: 100,
          value: @params[:resource_rarity] || 50,
          description: 'Rare vs common distribution'
        )

        concat render_settlement_controls
      end
    end

    # Settlement frequency and type controls
    def render_settlement_controls
      content_tag :div, class: 'settlement-controls' do
        concat render_slider_control(
          id: 'settlement_frequency',
          label: 'Settlements',
          min: 0, max: 100,
          value: @params[:settlement_frequency] || 30,
          description: 'Town/city spawn frequency'
        )

        concat render_settlement_type_picker
      end
    end

    # Settlement type options
    def render_settlement_type_picker
      content_tag :div, class: 'settlement-picker', role: 'group', aria: {
        label: 'Settlement types'
      } do
        SettlementType.all.each do |type|
          concat content_tag :div, class: 'settlement-option' do
            concat tag.label(class: 'settlement-label') do
              concat tag.checkbox_tag(
                'settlement_types',
                value: type.id,
                checked: true
              )
              concat content_tag(:span, type.name, class: 'settlement-name')
            end
          end
        end
      end
    end

    # Generic slider control with accessibility
    #
    # @param id [String] parameter identifier
    # @param label [String] display label
    # @param min [Integer] minimum value
    # @param max [Integer] maximum value
    # @param value [Integer] current value
    # @param description [String] help text
    # @param unit [String] unit suffix (optional)
    def render_slider_control(id:, label:, min:, max:, value:, description:, unit: nil)
      content_tag :div, class: 'slider-control', data: {
        world_builder_target: 'slider',
        slider_param: id
      } do
        concat content_tag(:label, label, for: id, class: 'slider-label')
        concat content_tag(:p, description, class: 'slider-description', id: "#{id}_desc")

        concat content_tag(:div, class: 'slider-input-container') do
          concat tag.input(
            type: 'range',
            id: id,
            name: id,
            min: min,
            max: max,
            value: value,
            aria: {
              describedby: "#{id}_desc",
              valuenow: value,
              valuemin: min,
              valuemax: max
            },
            data: {
              action: 'world-builder#updatePreview',
              preview_param: id
            }
          )

          value_display = unit ? "#{value} #{unit}" value.to_s
          concat content_tag(:span, value_display, class: 'slider-value', aria: {
            hidden: true
          })
        end
      end
    end

    # Preview panel with Turbo Frame live updates
    def render_preview_panel
      content_tag :section, class: 'preview-panel', aria: {
        label: 'Generation Preview',
        role: 'region'
      }, data: {
        world_builder_target: 'preview'
      } do
        concat content_tag(:h2, 'Live Preview', class: 'panel-title')

        # Turbo Frame for incremental updates
        concat turbo_frame_tag '/admin/worlds/preview', id: 'world-preview', data: {
          world_builder_target: 'previewFrame',
          params: @params.to_json
        } do
          concat render_preview_content
        end
      end
    end

    # Preview content (rendered inside Turbo Frame)
    def render_preview_content
      content_tag :div, class: 'preview-content' do
        # Terrain heatmap visualization
        concat render_terrain_preview
        # Biome overlay
        concat render_biome_overlay
        # Resource markers
        concat render_resource_markers
      end
    end

    # Terrain preview heatmap
    def render_terrain_preview
      content_tag :div, class: 'terrain-preview', aria: {
        label: 'Terrain elevation preview'
      } do
        # In production: Canvas-based rendering via Stimulus
        # Server-side: SVG generation for progressive enhancement
        concat tag.svg(class: 'terrain-svg', viewBox: '0 0 500 500') do
          concat tag.defs
          # Height-based path generation
          concat render_elevation_paths
        end
      end
    end

    # Elevation path rendering
    def render_elevation_paths
      # Generate SVG paths based on elevation data
      # In production: Cached preview from generation pipeline
      (0..10).map do |level|
        tag.path(
          d: generate_elevation_path(level),
          fill: elevation_color(level),
          aria: { hidden: true }
        )
      end.join
    end

    def generate_elevation_path(level)
      # Procedural path generation
      'M 0,500 L 500,500 L 500,' + (50 - level * 5).to_s + ' L 0,' + (50 - level * 5).to_s + ' Z'
    end

    def elevation_color(level)
      case level
      when 0..2
        '#4a90e2'  # Water
      when 3..5
        '#7b68ee'  # Plains
      when 6..8
        '#90a86b'  # Hills
      else
        '#8b7d6b'  # Mountains
      end
    end

    # Biome overlay on terrain
    def render_biome_overlay
      content_tag :div, class: 'biome-overlay', aria: {
        hidden: true
      } do
        @params[:biome_distribution]&.each do |biome_id|
          biome = BiomeType.find(biome_id)
          concat content_tag(:span, biome.icon, class: 'biome-indicator')
        end
      end
    end

    # Resource distribution markers
    def render_resource_markers
      content_tag :div, class: 'resource-markers', aria: {
        label: 'Resource spawn locations'
      } do
        # Marker visualization for ore/vegetation spawns
        concat content_tag(:p, "Resource density: #{@params[:resource_density]}%")
      end
    end

    # Generation controls (Generate, Apply, Reset)
    def render_generation_controls
      content_tag :section, class: 'generation-controls', role: 'navigation' do
        concat content_tag(:h2, 'Generation Controls', class: 'controls-title')

        concat content_tag :div, class: 'control-buttons' do
          concat link_to('Generate', '/admin/worlds/generate',
            method: :post,
            params: { world_id: @world_id, params: @params },
            class: 'btn-primary',
            data: {
              world_builder_target: 'generateButton',
              disable: 'true'
            }
          )

          concat link_to('Apply Parameters', '/admin/worlds/apply',
            method: :post,
            params: { world_id: @world_id, params: @params },
            class: 'btn-success'
          )

          concat link_to('Reset', '/admin/worlds/reset',
            method: :post,
            params: { world_id: @world_id },
            class: 'btn-secondary'
          )
        end

        # Progress indicator (Turbo Stream)
        concat content_tag :div, class: 'generation-progress', data: {
          world_builder_target: 'progress',
          turbo_stream_source: 'generation'
        }, aria: {
          live: 'polite',
          label: 'Generation progress'
        } do
          concat content_tag(:span, 'Ready', class: 'progress-status')
        end
      end
    end

    private

    attr_reader :world_id, :params, :world
  end
end