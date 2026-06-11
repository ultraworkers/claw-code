// swiftlint:disable all
// Diegetic UI Example
// World-space displays for game interfaces
//
// Features:
// - Holographic projected displays in world-space
// - Environmental context awareness (lighting, occlusion)
// - Player perspective alignment
// - 3D spatial positioning
//
// @platform iOS 17+, visionOS 2+
// @accessibility VoiceOver spatial audio, reduce motion
// @use ARKit for world tracking

import SwiftUI
import ARKit
import RealityKit

// =============================================================================
// DIEGETIC UI SYSTEM
// =============================================================================

/// Diegetic UI system for world-space holographic displays
///
/// Provides:
/// - Environmental awareness (lighting, occlusion, geometry)
/// - Player perspective alignment
/// - Holographic/projected rendering modes
/// - Spatial audio integration
final class DiegeticUISystem {
    // MARK: - Singleton

    static let shared = DiegeticUISystem()

    // MARK: - Configuration

    private var arSession: ARSession?
    private var worldTracking: WorldTracking?
    private var environmentalContext: EnvironmentalContext = .indoor

    // MARK: - Initialization

    init() {
        setupARSession()
        updateEnvironmentalContext()
    }

    private func setupARSession() {
        // Initialize ARKit session for world tracking
        arSession = ARSession()
        worldTracking = WorldTracking(session: arSession)
    }

    private func updateEnvironmentalContext() {
        // Detect environment (indoor/outdoor/mixed)
        // Update lighting, occlusion, geometry accordingly
        environmentalContext = .mixed
    }

    // MARK: - Public Methods

    /// Register diegetic display in world-space
    ///
    /// @param displayType Display type (holographic, projected, volumetric)
    /// @param position 3D world position
    /// @param alignment Spatial alignment
    func registerDisplay(
        _ displayType: DiegeticDisplayType,
        position: SIMD3<Float>,
        alignment: SpatialAlignment
    ) {
        // Register with world tracking
        worldTracking?.registerAnchor(
            Anchor(
                displayType: displayType,
                position: position,
                alignment: alignment
            )
        )
    }

    /// Update environmental context
    ///
    /// Called when environment changes (indoor→outdoor, lighting changes)
    func updateContext(_ newContext: EnvironmentalContext) {
        environmentalContext = newContext

        // Notify all registered displays
        DiegeticUIManager.shared.notifyEnvironmentalChange(newContext)
    }

    /// Align display to player perspective
    ///
    /// Adjusts display orientation based on player gaze/position
    func alignToPlayerPerspective() {
        // Get player head position from world tracking
        guard let playerPosition = worldTracking?.playerHeadPosition else { return }

        // Adjust all displays to face player
        DiegeticUIManager.shared.updatePerspective(playerPosition)
    }
}

// =============================================================================
// DIEGETIC UI MANAGER
// =============================================================================

/// Manager for all diegetic UI displays
///
/// Handles:
/// - Display lifecycle (create, update, remove)
/// - Environmental context propagation
/// - Perspective alignment
final class DiegeticUIManager {
    // MARK: - Singleton

    static let shared = DiegeticUIManager()

    // MARK: - Registered Displays

    private var displays: [DiegeticDisplay] = []

    // MARK: - Public Methods

    /// Add new diegetic display
    func addDisplay(_ display: DiegeticDisplay) {
        displays.append(display)
        display.render()
    }

    /// Remove diegetic display
    func removeDisplay(id: String) {
        displays.removeAll { $0.id == id }
    }

    /// Notify environmental change to all displays
    func notifyEnvironmentalChange(_ context: EnvironmentalContext) {
        displays.forEach { display in
            display.updateEnvironmentalContext(context)
        }
    }

    /// Update player perspective alignment
    func updatePerspective(_ playerPosition: SIMD3<Float>) {
        displays.forEach { display in
            display.alignToPlayer(playerPosition)
        }
    }
}

// =============================================================================
// DIEGETIC DISPLAY PROTOCOL
// =============================================================================

/// Protocol for diegetic display implementations
protocol DiegeticDisplay: AnyObject {
    var id: String
    var displayType: DiegeticDisplayType
    var position: SIMD3<Float>
    var alignment: SpatialAlignment

    func render()
    func updateEnvironmentalContext(_ context: EnvironmentalContext)
    func alignToPlayer(_ position: SIMD3<Float>)
}

// =============================================================================
// DISPLAY TYPES
// =============================================================================

/// Diegetic display type enumeration
enum DiegeticDisplayType {
    /// Holographic: Transparent, glowing, floating
    case holographic

    /// Projected: Beam-down projection from device
    case projected

    /// Volumetric: True 3D volume occupation
    case volumetric

    /// Environmental: Integrated with environment (walls, floors)
    case environmental
}

/// Spatial alignment type
enum SpatialAlignment {
    /// Horizontal: Parallel to ground plane
    case horizontal

    /// Vertical: Perpendicular to ground plane
    case vertical

    /// Facing camera: Always oriented toward viewer
    case facingCamera

    /// World-aligned: Fixed to world coordinates
    case worldFixed
}

// =============================================================================
// ENVIRONMENTAL CONTEXT
// =============================================================================

/// Environmental context for diegetic rendering
struct EnvironmentalContext {
    /// Ambient lighting intensity (0.0-1.0)
    var ambientLighting: Float

    /// Occlusion sources (objects that block display)
    var occlusionSources: [OcclusionSource]

    /// Environmental geometry (walls, floors, ceilings)
    var environmentalGeometry: Geometry

    /// Static indoor context
    static var indoor: EnvironmentalContext {
        EnvironmentalContext(
            ambientLighting: 0.3,
            occlusionSources: [],
            environmentalGeometry: Geometry室内
        )
    }

    /// Dynamic outdoor context
    static var outdoor: EnvironmentalContext {
        EnvironmentalContext(
            ambientLighting: 1.0,
            occlusionSources: [.sun, .terrain, .buildings],
            environmentalGeometry: Geometry.world
        )
    }

    /// Mixed reality context
    static var mixed: EnvironmentalContext {
        EnvironmentalContext(
            ambientLighting: 0.7,
            occlusionSources: [.furniture, .walls],
            environmentalGeometry: Geometry.partial
        )
    }
}

/// Occlusion source type
enum OcclusionSource {
    case sun
    case terrain
    case buildings
    case furniture
    case walls
    case players
}

/// Geometry type
enum Geometry {
    case empty
    case indoor
    case world
    case partial
}

// =============================================================================
// CONCRETE DISPLAY IMPLEMENTATIONS
// =============================================================================

/// Holographic resource display
final class HolographicResourceDisplay: DiegeticDisplay {
    // MARK: - Properties

    let id = "holo-resource-001"
    let displayType = .holographic
    var position: SIMD3<Float>
    var alignment: SpatialAlignment = .facingCamera

    private var resources: [Resource] = []
    private var currentContext: EnvironmentalContext = .mixed

    // MARK: - Initialization

    init(position: SIMD3<Float>) {
        self.position = position
        self.resources = []
    }

    // MARK: - Rendering

    func render() {
        // Render holographic resource display
        // In production: RealityKit rendering with holographic material
        print("[Diegetic] Holographic resource display rendered at \(position)")
    }

    func updateEnvironmentalContext(_ context: EnvironmentalContext) {
        currentContext = context

        // Adjust holographic opacity based on ambient lighting
        // Higher ambient = higher opacity for visibility
    }

    func alignToPlayer(_ position: SIMD3<Float>) {
        // Rotate display to face player
        // Maintain horizontal alignment
        print("[Diegetic] Holographic display aligned to player")
    }
}

/// Projected minimap display
final class ProjectedMinimapDisplay: DiegeticDisplay {
    // MARK: - Properties

    let id = "proj-minimap-001"
    let displayType = .projected
    var position: SIMD3<Float>
    var alignment: SpatialAlignment = .horizontal

    private var mapData: MapData
    private var projectionAngle: Float = 45.0

    // MARK: - Initialization

    init(position: SIMD3<Float>, mapData: MapData) {
        self.position = position
        self.mapData = mapData
    }

    // MARK: - Rendering

    func render() {
        // Render projected minimap
        // In production: ARKit projection with beam effect
        print("[Diegetic] Projected minimap rendered at \(position)")
    }

    func updateEnvironmentalContext(_ context: EnvironmentalContext) {
        // Adjust projection brightness based on ambient lighting
        // Add occlusion handling for obstacles
    }

    func alignToPlayer(_ position: SIMD3<Float>) {
        // Minimap stays horizontal, does not rotate
        // Position may shift to stay in player view
    }
}

/// Volumetric status indicator display
final class VolumetricStatusDisplay: DiegeticDisplay {
    // MARK: - Properties

    let id = "vol-status-001"
    let displayType = .volumetric
    var position: SIMD3<Float>
    var alignment: SpatialAlignment = .worldFixed

    private var statusIndicators: [StatusIndicator] = []
    private var volumeRadius: Float = 0.5

    // MARK: - Initialization

    init(position: SIMD3<Float>) {
        self.position = position
        self.statusIndicators = []
    }

    // MARK: - Rendering

    func render() {
        // Render volumetric status indicators
        // In production: 3D spheres with materials
        print("[Diegetic] Volumetric status display rendered at \(position)")
    }

    func updateEnvironmentalContext(_ context: EnvironmentalContext) {
        // Adjust sphere materials based on lighting
        // Add environmental glow effect
    }

    func alignToPlayer(_ position: SIMD3<Float>) {
        // Status indicators are world-fixed, do not rotate
        // May reposition to stay in player view cone
    }
}

/// Environmental minimap (wall/floor integrated)
final class EnvironmentalMinimapDisplay: DiegeticDisplay {
    // MARK: - Properties

    let id = "env-minimap-001"
    let displayType = .environmental
    var position: SIMD3<Float>
    var alignment: SpatialAlignment = .worldFixed

    private var surfaceType: SurfaceType = .wall
    private var mapData: MapData

    // MARK: - Initialization

    init(position: SIMD3<Float>, surfaceType: SurfaceType, mapData: MapData) {
        self.position = position
        self.surfaceType = surfaceType
        self.mapData = mapData
    }

    // MARK: - Rendering

    func render() {
        // Render environmental minimap on surface
        // In production: Surface detection + texture projection
        print("[Diegetic] Environmental minimap on \(surfaceType) at \(position)")
    }

    func updateEnvironmentalContext(_ context: EnvironmentalContext) {
        // Blend with environmental geometry
        // Match surface material appearance
    }

    func alignToPlayer(_ position: SIMD3<Float>) {
        // Environmental display is fixed to surface
        // Does not rotate or reposition
    }
}

// =============================================================================
// SUPPORTING MODELS
// =============================================================================

/// Resource model for holographic display
struct Resource {
    let id: String
    let name: String
    let amount: Decimal
    let icon: String
}

/// Map data for minimap displays
struct MapData {
    let terrain: TerrainMesh
    let markers: [MapMarker]
    let scale: Float
}

/// Terrain mesh
struct TerrainMesh {
    let vertices: [SIMD3<Float>]
    let triangles: [SIMD3<UInt32>]
}

/// Map marker
struct MapMarker {
    let id: String
    let position: SIMD3<Float>
    let type: MapMarkerType
}

enum MapMarkerType {
    case player
    case enemy
    case objective
    case resource
    case waypoint
}

/// Status indicator for volumetric display
struct StatusIndicator {
    let type: StatusType
    let value: Float
    let color: Color
}

enum StatusType {
    case health
    case mana
    case stamina
    case energy
    case shield
}

/// Surface type for environmental displays
enum SurfaceType {
    case wall
    case floor
    case ceiling
    case curved
    case irregular
}

// =============================================================================
// SwiftUI DIEGETIC VIEW COMPONENTS
// =============================================================================

/// Diegetic container view
@available(visionOS 2.0)
struct DiegeticContainerView: View {
    let displayType: DiegeticDisplayType
    let position: SIMD3<Float>
    let content: () -> some View

    @Environment(\.environmentalContext) private var context

    var body: some View {
        RealityView {
            content()
                .diegeticDisplayType(displayType)
                .position(position)
                .environmentalContext(context)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Diegetic Display")
    }
}

/// Holographic text effect
@available(visionOS 2.0)
struct HolographicTextEffect: ViewModifier {
    var glowIntensity: Float = 0.8
    var transparency: Float = 0.6

    func body(content: ViewContent) -> some View {
        content
            .foregroundColor(.cyan)
            .shadow(color: .blue, radius: glowIntensity)
            .opacity(1.0 - transparency)
            .holographicMaterial()
    }
}

/// Holographic icon effect
@available(visionOS 2.0)
struct HolographicIconEffect: ViewModifier {
    var pulseRate: Float = 1.0

    func body(content: ViewContent) -> some View {
        content
            .foregroundColor(.white)
            .shadow(color: .blue, radius: 5)
            .pulseAnimation(rate: pulseRate)
            .holographicMaterial()
    }
}

/// Projected effect for minimaps
@available(visionOS 2.0)
struct ProjectedEffect: ViewModifier {
    var angle: Float = 45.0
    var beamVisible: Bool = true

    func body(content: ViewContent) -> some View {
        content
            .rotation3DEffect(.degrees(angle), axis: .x)
            .shadow(color: .gray, radius: 10)
            .overlay {
                if beamVisible {
                    BeamView(angle: angle)
                }
            }
    }
}

/// Beam visualization for projected displays
@available(visionOS 2.0)
struct BeamView: View {
    let angle: Float

    var body: some View {
        LinearGradient(
            colors: [.clear, .white.opacity(0.2)],
            startPoint: .top,
            endPoint: .bottom
        )
        .rotation3DEffect(.degrees(angle), axis: .x)
        .blur(radius: 2)
    }
}

/// Volumetric sphere 3D
@available(visionOS 2.0)
struct Sphere3D: View {
    let radius: Float
    let material: StatusMaterial

    var body: some View {
        RealityView {
            Sphere(radius: radius)
                .material(material.realityMaterial)
                .lighting(.ambient(intensity: material.emission))
        }
        .accessibilityHidden(true)
    }
}

/// Status material for volumetric indicators
enum StatusMaterial {
    case health
    case mana
    case stamina
    case energy
    case shield

    var realityMaterial: Material {
        switch self {
        case .health: return .physicallyBasedRendering(color: .red)
        case .mana: return .physicallyBasedRendering(color: .blue)
        case .stamina: return .physicallyBasedRendering(color: .green)
        case .energy: return .physicallyBasedRendering(color: .yellow)
        case .shield: return .physicallyBasedRendering(color: .cyan)
        }
    }

    var emission: Float {
        switch self {
        case .health: return 0.3
        case .mana: return 0.4
        case .stamina: return 0.3
        case .energy: return 0.5
        case .shield: return 0.4
        }
    }
}

// =============================================================================
// ENVIRONMENT CONTEXT
// =============================================================================

/// Environmental context environment value
@available(visionOS 2.0)
extension EnvironmentValues {
    @Entry var environmentalContext: EnvironmentalContext = .mixed
}

// =============================================================================
// ACCESSIBILITY SUPPORT
// =============================================================================

/// Diegetic accessibility modifier
@available(visionOS 2.0)
extension View {
    /// Spatial audio for diegetic displays
    func diegeticSpatialAudio() -> some View {
        modifier(DiegeticSpatialAudioModifier())
    }

    /// Reduce motion for parallax/holographic effects
    func reduceMotionIfEnabled() -> some View {
        if UIAccessibility.isReduceMotionEnabled {
            return self.modifier(DiegeticReduceMotionModifier())
        }
        return self
    }
}

/// Diegetic spatial audio modifier
struct DiegeticSpatialAudioModifier: ViewModifier {
    func body(content: ViewContent) -> some View {
        content.audioSpatialization(.headRelated)
    }
}

/// Diegetic reduce motion modifier
struct DiegeticReduceMotionModifier: ViewModifier {
    func body(content: ViewContent) -> some View {
        content.holographicEffect(.disabled)
        content.pulseAnimation(.disabled)
    }
}