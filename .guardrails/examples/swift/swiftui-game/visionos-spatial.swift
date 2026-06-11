// swiftlint:disable all
// VisionOS Spatial Computing Example
// Volumetric interfaces with Z-depth, hand tracking, and immersion levels
//
// Features:
// - 3D volume geometry for spatial windows
// - ImmersionLevel control (.mixed, .full, .minimized)
// - Hand gesture interaction
// - Eye tracking for focus selection
// - Diegetic UI world-space rendering
//
// @platform visionOS 2+
// @accessibility VoiceOver spatial audio, reduce motion
// @security Window space validation

import SwiftUI
import RealityKit
import ARKit

// =============================================================================
// SPATIAL WINDOW CONFIGURATION
// =============================================================================

/// VisionOS spatial window configuration
///
/// Manages:
/// - Volume geometry (3D space allocation)
/// - Immersion level transitions
/// - Hand tracking for gestures
/// - Eye tracking for focus
struct SpatialWindowConfiguration {
    // MARK: - Volume Geometry

    var volumeSize: SIMD3<Float>
    var volumePosition: SIMD3<Float>
    var volumeOrientation: quaternionf

    // Default volume: 2m x 1m x 0.5m (game dashboard)
    static var gameDashboard: SpatialWindowConfiguration {
        SpatialWindowConfiguration(
            volumeSize: SIMD3<Float>(2.0, 1.0, 0.5),
            volumePosition: SIMD3<Float>(0, 0, -1.5),
            volumeOrientation: quaternionf(angle: 0, axis: SIMD3<Float>(0, 1, 0))
        )
    }

    // Full immersion volume: 4m x 3m x 2m
    static var fullImmersion: SpatialWindowConfiguration {
        SpatialWindowConfiguration(
            volumeSize: SIMD3<Float>(4.0, 3.0, 2.0),
            volumePosition: SIMD3<Float>(0, 0, 0),
            volumeOrientation: quaternionf(angle: 0, axis: SIMD3<Float>(0, 1, 0))
        )
    }

    // MARK: - Immersion Level

    var immersionLevel: ImmersionLevel

    /// Transition to new immersion level
    mutating func transitionTo(_ newLevel: ImmersionLevel) {
        immersionLevel = newLevel

        switch newLevel {
        case .mixed:
            volumeSize = SIMD3<Float>(2.0, 1.5, 0.8)
        case .full:
            volumeSize = SIMD3<Float>(4.0, 3.0, 2.0)
        case .minimized:
            volumeSize = SIMD3<Float>(0.5, 0.3, 0.2)
        }
    }
}

// =============================================================================
// SPATIAL GAME VIEW
// =============================================================================

/// Main spatial game view for VisionOS
///
/// Features:
/// - Z-depth layering for UI elements
/// - Hand gesture interaction
/// - Eye tracking focus
/// - World-space diegetic rendering
@available(visionOS 2.0)
struct SpatialGameView: View {
    // MARK: - State

    @State private var configuration = SpatialWindowConfiguration.gameDashboard
    @State private var immersionLevel: ImmersionLevel = .mixed
    @State private var handTrackingEnabled = true
    @State private var eyeTrackingEnabled = true

    // MARK: - Body

    var body: some View {
        RealityView {
            SpatialContentLayer()
                .volumeGeometry(configuration.volumeSize)
                .position(configuration.volumePosition)
                .orientation(configuration.volumeOrientation)
                .immersionLevel(immersionLevel)
        }
        .gestureHandler(.hand, perform: handleHandGesture)
        .eyeTrackingEnabled(eyeTrackingEnabled)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Spatial Game Interface")
        .accessibilityHint("3D game dashboard with hand tracking")
    }

    // MARK: - Hand Gesture Handler

    private func handleHandGesture(_ gesture: HandGesture) {
        switch gesture {
        case .pinch:
            // Select UI element
            selectFocusedElement()
        case .tap:
            // Activate selected element
            activateSelectedElement()
        case .grab:
            // Move/resize volume
            adjustVolume()
        case .expand:
            // Increase immersion level
            increaseImmersion()
        }
    }

    // MARK: - Eye Tracking

    private func selectFocusedElement() {
        // Eye tracking selects focused element
        // In production: gaze detection via RealityKit
        print("[EyeTracking] Element selected")
    }

    private func activateSelectedElement() {
        // Hand tap activates selection
        print("[HandGesture] Element activated")
    }

    private func adjustVolume() {
        // Hand grab adjusts volume geometry
        print("[HandGesture] Volume adjusted")
    }

    private func increaseImmersion() {
        // Hand expand increases immersion
        if immersionLevel == .mixed {
            immersionLevel = .full
            configuration.transitionTo(.full)
        }
    }
}

// =============================================================================
// SPATIAL CONTENT LAYER
// =============================================================================

/// Volumetric content layer with Z-depth
///
/// Renders:
/// - Resource counters (foreground Z)
/// - Game map (midground Z-0.5)
/// - Analytics (background Z-1.0)
/// - Diegetic UI (world-space)
@available(visionOS 2.0)
struct SpatialContentLayer: View {
    // MARK: - Z-Depth Layers

    @State private var foregroundZ: Float = 0.2
    @State private var midgroundZ: Float = -0.5
    @State private var backgroundZ: Float = -1.0

    var body: some View {
        ZStack {
            // Foreground: Resource counters
            ResourceCounterLayer()
                .depth(foregroundZ)
                .accessibilityIdentifier("resourceForeground")

            // Midground: Game map/world
            GameWorldLayer()
                .depth(midgroundZ)
                .accessibilityIdentifier("gameWorldMidground")

            // Background: Analytics dashboard
            AnalyticsDashboardLayer()
                .depth(backgroundZ)
                .accessibilityIdentifier("analyticsBackground")

            // Diegetic UI: World-space holographic
            DiegeticUILayer()
                .depth(-2.0)
                .accessibilityIdentifier("diegeticWorldSpace")
        }
    }
}

// =============================================================================
// FOREGROUND: RESOURCE COUNTERS
// =============================================================================

/// 3D resource counter layer
///
/// Displays floating resource indicators with:
/// - Parallax effect on head movement
/// - Glow on eye tracking focus
/// - Haptic feedback on selection
@available(visionOS 2.0)
struct ResourceCounterLayer: View {
    @State private var resources: [SpatialResource] = []

    var body: some View {
        VStack(spacing: 20) {
            ForEach(resources) { resource in
                SpatialResourceView(resource: resource)
                    .parallaxEffect()
                    .eyeTrackingGlow()
            }
        }
        .background(
            Color.clear
                .shadow(.inner(color: .blue, radius: 10))
        )
        .cornerRadius(20)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Resource Counters")
    }
}

/// Single resource in spatial view
struct SpatialResource: Identifiable {
    let id: String
    let name: String
    let amount: Decimal
    let icon: String
    let position3D: SIMD3<Float>
}

/// Spatial resource view component
@available(visionOS 2.0)
struct SpatialResourceView: View {
    let resource: SpatialResource

    var body: some View {
        HStack(spacing: 15) {
            Image(systemName: resource.icon)
                .font(.title2)
                .accessibilityHidden(true)

            Text(resource.name)
                .font(.headline)

            Text("\(resource.amount)")
                .font(.title)
                .fontWeight(.bold)
        }
        .padding()
        .background(Color.systemFill.opacity(0.8))
        .cornerRadius(10)
        .volumeDepth(0.1)
    }
}

// =============================================================================
// MIDGROUND: GAME WORLD
// =============================================================================

/// Game world layer with volumetric rendering
///
/// Displays:
/// - 3D terrain/environment
/// - Player avatars/characters
/// - Interactive objects
@available(visionOS 2.0)
struct GameWorldLayer: View {
    @State private var worldGeometry: WorldGeometry

    init() {
        self.worldGeometry = WorldGeometry.default
    }

    var body: some View {
        RealityView {
            WorldContent()
                .geometry(worldGeometry)
                .lighting(.ambient(intensity: 0.5))
                .occlusionEnabled(true)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Game World")
        .accessibilityHint("3D game environment")
    }
}

/// World geometry configuration
struct WorldGeometry {
    var terrainMesh: MeshResource
    var lightingModel: LightingModel
    var occlusionMask: MaskResource

    static var default: WorldGeometry {
        WorldGeometry(
            terrainMesh: MeshResource.generateSphere(radius: 10),
            lightingModel: LightingModel.default,
            occlusionMask: MaskResource.empty
        )
    }
}

/// World content renderer
@available(visionOS 2.0)
struct WorldContent: View {
    var body: some View {
        ModelEntity3D()
            .materials([.physicallyBasedRendering])
            .animations([])
    }
}

// =============================================================================
// BACKGROUND: ANALYTICS DASHBOARD
// =============================================================================

/// Analytics dashboard in background Z-layer
///
/// Displays:
/// - Player retention metrics
/// - Economy health indicators
/// - Moderation alerts
@available(visionOS 2.0)
struct AnalyticsDashboardLayer: View {
    @State private var metrics: DashboardMetrics

    var body: some View {
        VStack(spacing: 15) {
            MetricCard(title: "Active Players", value: metrics.activePlayers)
            MetricCard(title: "Retention D1", value: metrics.retentionD1, format: .percentage)
            MetricCard(title: "Economy Index", value: metrics.economyIndex)
        }
        .padding()
        .background(Color.systemBackground.opacity(0.9))
        .cornerRadius(15)
        .depth(-1.0)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Analytics Dashboard")
    }
}

/// Dashboard metrics model
struct DashboardMetrics {
    let activePlayers: Int
    let retentionD1: Decimal
    let economyIndex: Decimal
}

/// Metric card component
struct MetricCard: View {
    let title: String
    let value: Decimal
    var format: MetricFormat = .number

    var body: some View {
        VStack(alignment: .leading) {
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
            Text(formatValue(value, format))
                .font(.title2)
                .fontWeight(.bold)
        }
        .padding()
        .frame(minWidth: 150)
    }

    private func formatValue(_ value: Decimal, _ format: MetricFormat) -> String {
        switch format {
        case .percentage: return "\(value.rounded(1))%"
        case .number: return "\(value)"
        }
    }
}

enum MetricFormat {
    case number
    case percentage
}

// =============================================================================
// DIEGETIC UI: WORLD-SPACE DISPLAYS
// =============================================================================

/// Diegetic UI layer for world-space holographic displays
///
/// Features:
/// - Environmental context (lighting, occlusion)
/// - Player perspective alignment
/// - Holographic projection effect
@available(visionOS 2.0)
struct DiegeticUILayer: View {
    @State private var displayType: DiegeticDisplayType = .holographic
    @State private var environmentalContext: EnvironmentalContext

    var body: some View {
        RealityView {
            DiegeticContent()
                .displayType(displayType)
                .environmentalContext(environmentalContext)
                .perspectiveAligned(true)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Diegetic Interface")
        .accessibilityHint("World-space holographic display")
    }
}

/// Diegetic display type
enum DiegeticDisplayType {
    case holographic
    case projected
    • volumetric
    case environmental
}

/// Environmental context for diegetic UI
struct EnvironmentalContext {
    var ambientLighting: Float
    var occlusionSources: [OcclusionSource]
    var environmentalGeometry: Geometry

    static var indoor: EnvironmentalContext {
        EnvironmentalContext(
            ambientLighting: 0.3,
            occlusionSources: [],
            environmentalGeometry: Geometry.empty
        )
    }

    static var outdoor: EnvironmentalContext {
        EnvironmentalContext(
            ambientLighting: 1.0,
            occlusionSources: [.sun, .terrain],
            environmentalGeometry: Geometry.world
        )
    }
}

/// Diegetic content renderer
@available(visionOS 2.0)
struct DiegeticContent: View {
    var body: some View {
        HStack(spacing: 30) {
            // Holographic resource display
            HolographicResourceDisplay()

            // Projected minimap
            ProjectedMinimap()

            • Volumetric status indicators
            VolumetricStatusIndicators()
        }
        .holographicEffect()
        .worldAlignment(.horizontal)
    }
}

/// Holographic resource display
@available(visionOS 2.0)
struct HolographicResourceDisplay: View {
    var body: some View {
        VStack {
            Text("Resources")
                .font(.headline)
                .holographicTextEffect()

            // Resource icons floating in world-space
            Image(systemName: "cube")
                .holographicIconEffect()
        }
    }
}

/// Projected minimap
@available(visionOS 2.0)
struct ProjectedMinimap: View {
    var body: some View {
        Image(systemName: "map")
            .projectedEffect(angle: 45)
            .frame(width: 200, height: 200)
    }
}

/// Volumetric status indicators
@available(visionOS 2.0)
struct VolumetricStatusIndicators: View {
    var body: some View {
        VStack {
            Text("Status")
                .font(.headline)

            // 3D status indicators
            Sphere3D(radius: 0.1, material: .health)
            Sphere3D(radius: 0.1, material: .mana)
        }
    }
}

// =============================================================================
// ACCESSIBILITY EXTENSIONS
// =============================================================================

/// VisionOS accessibility extensions
extension View {
    /// Spatial audio for VoiceOver
    func spatialAudioSource() -> some View {
        modifier(SpatialAudioModifier())
    }

    /// Reduce motion for parallax effects
    func reduceMotionIfEnabled() -> some View {
        if UIAccessibility.isReduceMotionEnabled {
            return self.modifier(ReduceMotionModifier())
        }
        return self
    }
}

/// Spatial audio modifier
struct SpatialAudioModifier: ViewModifier {
    func body(content: ViewContent) -> some View {
        content.audioSpatialization(.headRelated)
    }
}

/// Reduce motion modifier
struct ReduceMotionModifier: ViewModifier {
    func body(content: ViewContent) -> some View {
        content.parallaxEffect(.disabled)
    }
}