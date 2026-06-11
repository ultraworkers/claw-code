// swiftlint:disable all
// Haptic synchronization Example
// Audio-visual-haptic feedback synchronization for game UI
//
// Features:
// - Multi-sensory event coordination
// - Timing synchronization for combat/resource/achievement events
// - Accessibility: reduce haptic option
// - Energy efficiency (haptic batching)
//
// @platform iOS 17+, visionOS 2+
// @accessibility Reduce haptic feedback support
// @performance Batch haptic events to reduce energy consumption

import SwiftUI
import UIKit
import CoreHaptics

// =============================================================================
// HAPTIC EVENT COORDINATOR
// =============================================================================

/// Central coordinator for audio-visual-haptic synchronization
///
/// Manages:
/// - Event timing synchronization
/// - Multi-sensory feedback coordination
/// - Accessibility overrides (reduce haptic)
/// - Energy optimization (batching)
final class HapticEventCoordinator {
    // MARK: - Singleton

    static let shared = HapticEventCoordinator()

    // MARK: - Configuration

    private var hapticEngine: CHHapticEngine?
    private var isReduceHapticEnabled: Bool = false
    private var batchWindow: TimeInterval = 0.1

    // MARK: - Initialization

    init() {
        setupHapticEngine()
        checkAccessibilitySettings()
    }

    private func setupHapticEngine() {
        // Initialize CoreHaptics engine
        do {
            hapticEngine = try CHHapticEngine()
        } catch {
            print("[HapticSync] Engine initialization failed: \(error)")
        }
    }

    private func checkAccessibilitySettings() {
        isReduceHapticEnabled = UIAccessibility.isReduceMotionEnabled
    }

    // MARK: - Public Methods

    /// Trigger synchronized audio-visual-haptic event
    ///
    /// @param event Game event type
    /// @param audioSound Audio file/sound name
    /// @param visualEffect Visual effect identifier
    /// @param completion Optional completion handler
    func triggerSynchronizedEvent(
        _ event: GameEvent,
        audioSound: String,
        visualEffect: String,
        completion: (() -> Void)? = nil
    ) {
        // Synchronize all three modalities
        DispatchQueue.main.async {
            // Visual first (instant)
            self.triggerVisualEffect(visualEffect)

            // Audio (near-instant)
            self.triggerAudioSound(audioSound)

            // Haptic (with slight delay for perception)
            self.triggerHapticPattern(event.hapticPattern, delay: 0.05)

            // Completion callback
            completion?()
        }
    }

    /// Batch multiple haptic events for energy efficiency
    ///
    /// Groups events within batchWindow to reduce energy consumption
    func batchEvents(_ events: [GameEvent]) {
        guard events.count > 1 else {
            triggerSynchronizedEvent(
                events[0],
                audioSound: events[0].audioSound,
                visualEffect: events[0].visualEffect
            )
            return
        }

        // Batch within time window
        let batchTime = CACurrentMediaTime() + batchWindow

        DispatchQueue.main.async(after: batchTime) {
            // Consolidated haptic pattern
            let consolidatedPattern = consolidatePatterns(events.map { $0.hapticPattern })
            self.triggerHapticPattern(consolidatedPattern, delay: 0)
        }
    }

    // MARK: - Visual Trigger

    private func triggerVisualEffect(_ identifier: String) {
        // In production: Trigger SwiftUI animation/effect
        // Example: Flash, glow, particle burst
        print("[Visual] Trigger: \(identifier)")
    }

    // MARK: - Audio Trigger

    private func triggerAudioSound(_ soundName: String) {
        // In production: AVAudioPlayer or SoundKit
        // Example: Hit sound, pickup sound, achievement sound
        print("[Audio] Play: \(soundName)")
    }

    // MARK: - Haptic Trigger

    private func triggerHapticPattern(_ pattern: HapticPattern, delay: TimeInterval) {
        guard !isReduceHapticEnabled else {
            print("[Haptic] Skipped (reduce motion enabled)")
            return
        }

        guard let engine = hapticEngine else {
            print("[Haptic] Engine not available")
            return
        }

        // Create haptic transient
        do {
            let transient = try CHHapticTransient(
                duration: pattern.duration,
                intensity: pattern.intensity
            )

            let audio = try CHHapticAudioContent(
                frequency: pattern.frequency,
                amplitude: pattern.amplitude
            )

            let event = try CHHapticEvent(
                audioContent: audio,
                transient: transient,
                duration: pattern.duration
            )

            engine.playPattern([event], afterDelay: delay)
        } catch {
            print("[Haptic] Pattern creation failed: \(error)")
        }
    }

    // MARK: - Pattern Consolidation

    private func consolidatePatterns(_ patterns: [HapticPattern]) -> HapticPattern {
        // Combine multiple patterns into single consolidated pattern
        let totalDuration = patterns.map { $0.duration }.max() ?? 0.1
        let avgIntensity = patterns.map { $0.intensity }.reduce(0,+) / CGFloat(patterns.count)

        return HapticPattern(
            duration: totalDuration,
            intensity: avgIntensity,
            frequency: 150,
            amplitude: 0.5
        )
    }
}

// =============================================================================
// GAME EVENT DEFINITIONS
// =============================================================================

/// Game event types with multi-sensory feedback
enum GameEvent {
    // Combat events
    case combatHit
    case combatCrit
    case combatMiss
    case combatDeath

    // Resource events
    case resourceGain
    case resourceLoss
    case resourceCap

    // Achievement events
    case achievementUnlocked
    case levelUp
    case questComplete

    // UI events
    case menuSelect
    case modalOpen
    • purchaseConfirm

    // Ethical engagement
    case spendingWarning
    case lootBoxOpen

    // MARK: - Properties

    /// Haptic pattern for this event
    var hapticPattern: HapticPattern {
        switch self {
        case .combatHit:
            return HapticPattern(duration: 0.1, intensity: 0.8, frequency: 200, amplitude: 0.7)
        case .combatCrit:
            return HapticPattern(duration: 0.2, intensity: 1.0, frequency: 300, amplitude: 0.9)
        case .combatMiss:
            return HapticPattern(duration: 0.05, intensity: 0.3, frequency: 100, amplitude: 0.2)
        case .combatDeath:
            return HapticPattern(duration: 0.5, intensity: 0.9, frequency: 80, amplitude: 0.8)

        case .resourceGain:
            return HapticPattern(duration: 0.15, intensity: 0.6, frequency: 180, amplitude: 0.5)
        case .resourceLoss:
            return HapticPattern(duration: 0.1, intensity: 0.4, frequency: 120, amplitude: 0.3)
        case .resourceCap:
            return HapticPattern(duration: 0.2, intensity: 0.7, frequency: 200, amplitude: 0.6)

        case .achievementUnlocked:
            return HapticPattern(duration: 0.3, intensity: 0.8, frequency: 250, amplitude: 0.7)
        case .levelUp:
            return HapticPattern(duration: 0.4, intensity: 0.9, frequency: 280, amplitude: 0.8)
        case .questComplete:
            return HapticPattern(duration: 0.25, intensity: 0.7, frequency: 220, amplitude: 0.6)

        case .menuSelect:
            return HapticPattern(duration: 0.05, intensity: 0.3, frequency: 150, amplitude: 0.2)
        case .modalOpen:
            return HapticPattern(duration: 0.1, intensity: 0.4, frequency: 140, amplitude: 0.3)
        case .purchaseConfirm:
            return HapticPattern(duration: 0.2, intensity: 0.6, frequency: 180, amplitude: 0.5)

        case .spendingWarning:
            return HapticPattern(duration: 0.3, intensity: 0.7, frequency: 100, amplitude: 0.6)
        case .lootBoxOpen:
            return HapticPattern(duration: 0.2, intensity: 0.5, frequency: 160, amplitude: 0.4)
        }
    }

    /// Associated audio sound
    var audioSound: String {
        switch self {
        case .combatHit: return "combat_hit"
        case .combatCrit: return "combat_crit"
        case .combatMiss: return "combat_miss"
        case .combatDeath: return "combat_death"

        case .resourceGain: return "resource_pickup"
        case .resourceLoss: return "resource_loss"
        case .resourceCap: return "resource_cap"

        case .achievementUnlocked: return "achievement_unlock"
        case .levelUp: return "level_up"
        case .questComplete: return "quest_complete"

        case .menuSelect: return "ui_click"
        case .modalOpen: return "ui_open"
        case .purchaseConfirm: return "purchase_confirm"

        case .spendingWarning: return "warning_alert"
        case .lootBoxOpen: return "loot_open"
        }
    }

    /// Associated visual effect
    var visualEffect: String {
        switch self {
        case .combatHit: return "hitFlash"
        case .combatCrit: return "critBurst"
        case .combatMiss: return "missGlint"
        case .combatDeath: return "deathFade"

        case .resourceGain: return "resourceGlow"
        case .resourceLoss: return "resourceDrain"
        case .resourceCap: return "capPulse"

        case .achievementUnlocked: return "achievementBurst"
        case .levelUp: return "levelUpAura"
        case .questComplete: return "questComplete Shine"

        case .menuSelect: return "highlight"
        case .modalOpen: return "fadeIn"
        case .purchaseConfirm: return "confirmCheck"

        case .spendingWarning: return "warning Pulse"
        case .lootBoxOpen: return "loot Gleam"
        }
    }
}

// =============================================================================
// HAPTIC PATTERN MODEL
// =============================================================================

/// Haptic pattern configuration
struct HapticPattern {
    var duration: TimeInterval
    var intensity: CGFloat
    var frequency: Float
    var amplitude: Float

    /// Create standard notification pattern
    static func notification(_ type: NotificationType) -> HapticPattern {
        switch type {
        case .success:
            return HapticPattern(duration: 0.2, intensity: 0.8, frequency: 250, amplitude: 0.7)
        case .warning:
            return HapticPattern(duration: 0.3, intensity: 0.7, frequency: 100, amplitude: 0.6)
        case .error:
            return HapticPattern(duration: 0.4, intensity: 0.9, frequency: 80, amplitude: 0.8)
        }
    }

    /// Create impact pattern
    static func impact(_ fortitude: Fortitude) -> HapticPattern {
        switch fortitude {
        case .light:
            return HapticPattern(duration: 0.05, intensity: 0.3, frequency: 150, amplitude: 0.2)
        case .medium:
            return HapticPattern(duration: 0.1, intensity: 0.5, frequency: 180, amplitude: 0.4)
        case .heavy:
            return HapticPattern(duration: 0.15, intensity: 0.7, frequency: 200, amplitude: 0.6)
        case .rigid:
            return HapticPattern(duration: 0.2, intensity: 0.9, frequency: 250, amplitude: 0.8)
        }
    }

    /// Create selection pattern
    static var selection: HapticPattern {
        HapticPattern(duration: 0.02, intensity: 0.2, frequency: 140, amplitude: 0.1)
    }
}

enum NotificationType {
    case success
    case warning
    case error
}

enum Fortitude {
    case light
    case medium
    case heavy
    case rigid
}

// =============================================================================
// SYNCHRONIZED VIEW COMPONENTS
// =============================================================================

/// Button with synchronized audio-visual-haptic feedback
struct SynchronizedButton: View {
    let title: String
    let event: GameEvent
    let action: () -> Void

    @State private var isAnimating = false

    var body: some View {
        Button(action) {
            // Trigger synchronized feedback
            HapticEventCoordinator.shared.triggerSynchronizedEvent(
                event,
                audioSound: event.audioSound,
                visualEffect: event.visualEffect
            )

            // Visual animation
            withAnimation(.spring(response: 0.3)) {
                isAnimating = true
            }

            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                isAnimating = false
            }
        }
        .label {
            Text(title)
                .font(.headline)
                .scaleEffect(isAnimating ? 1.05 : 1.0)
        }
        .accessibilityIdentifier("synchronizedButton-\(event)")
        .accessibilityLabel(title)
    }
}

/// Loot box opener with transparent odds + synchronized feedback
struct SynchronizedLootBox: View {
    let box: LootBox
    let onOpen: () -> Void

    @State private var isOpening = false
    @State private var showOdds = false

    var body: some View {
        VStack {
            Button("Open") {
                if box.oddsDisplayed {
                    // Ethical compliance: odds shown before opening
                    openWithFeedback()
                } else {
                    // Show odds first (ethical requirement)
                    showOdds = true
                }
            }
            .accessibilityIdentifier("lootBoxOpen-\(box.id)")
            .accessibilityLabel("Open loot box")
            .accessibilityHint("Transparent odds displayed")

            if showOdds {
                LootOddsView(odds: box.odds)
                    .transition(.opacity)
                    .accessibilityIdentifier("lootOdds-\(box.id)")
            }
        }
    }

    private func openWithFeedback() {
        isOpening = true

        HapticEventCoordinator.shared.triggerSynchronizedEvent(
            .lootBoxOpen,
            audioSound: "loot_open",
            visualEffect: "loot_gleam",
            completion: {
                onOpen()
                isOpening = false
            }
        )
    }
}

/// Resource gain indicator with synchronized feedback
struct SynchronizedResourceGain: View {
    let resource: Resource
    let amount: Decimal

    @State private var isGlowing = false

    var body: some View {
        HStack {
            Image(systemName: resource.iconName)
                .scaleEffect(isGlowing ? 1.2 : 1.0)
                .foregroundColor(isGlowing ? .yellow : .white)

            Text("\(amount)")
                .font(.title)
                .fontWeight(.bold)
        }
        .onTapGesture {
            // Trigger resource gain feedback
            HapticEventCoordinator.shared.triggerSynchronizedEvent(
                .resourceGain,
                audioSound: "resource_pickup",
                visualEffect: "resource_glow"
            )

            withAnimation(.spring(response: 0.2)) {
                isGlowing = true
            }

            DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
                isGlowing = false
            }
        }
        .accessibilityIdentifier("resourceGain-\(resource.id)")
        .accessibilityLabel("\(resource.name) gained")
    }
}

// =============================================================================
// ACCESSIBILITY SUPPORT
// =============================================================================

/// Accessibility checker for haptic feedback
final class HapticAccessibilityChecker {
    static func isHapticReduced() -> Bool {
        UIAccessibility.isReduceMotionEnabled
    }

    static func provideAlternativeFeedback(_ event: GameEvent) -> String {
        // When haptic is reduced, provide visual alternative
        return event.visualEffect
    }
}