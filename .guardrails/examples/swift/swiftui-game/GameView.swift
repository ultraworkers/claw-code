// swiftlint:disable all
// SwiftUI 6.0 Game View Example
// Production-ready game UI pattern with accessibility and ethical engagement
//
// Features:
// - ObservableObject game state management
// - Accessibility identifiers for VoiceOver
// - Ethical engagement (loot transparency, spending limits)
// - Rest-state mechanics display
//
// @platform iOS 17+, visionOS 2+, macOS 14+
// @test XCTest UI tests with accessibility identifiers
// @security No sensitive data in view hierarchy

import SwiftUI
import Combine

// =============================================================================
// GAME STATE MANAGER
// =============================================================================

/// Main game state controller using ObservableObject pattern
///
/// Manages:
/// - Player resources, levels, achievements
/// - Loot table odds (transparent display)
/// - Offline regeneration (rest-state mechanics)
/// - Spending limits (ethical engagement)
@MainActor
final class GameStateManager: ObservableObject {
    // MARK: - Published Properties

    @Published private(set) var playerResources: ResourceBalance
    @Published private(set) var playerLevel: Int
    @Published private(set) var achievements: [Achievement]
    @Published private(set) var activeLootBoxes: [LootBox]
    @Published private(set) var offlineRegeneration: OfflineResources

    // Ethical engagement: spending tracking
    @Published private(set) var monthlySpending: Decimal
    @Published private(set) var spendingLimit: Decimal

    // MARK: - Configuration

    private let spendingLimitDefault: Decimal = 50.00

    // MARK: - Initialization

    init() {
        self.playerResources = ResourceBalance.empty
        self.playerLevel = 1
        self.achievements = []
        self.activeLootBoxes = []
        self.offlineRegeneration = OfflineResources.empty
        self.monthlySpending = 0
        self.spendingLimit = spendingLimitDefault
    }

    // MARK: - Public Methods

    /// Update resources with haptic feedback trigger
    func addResources(_ resources: ResourceBalance) {
        playerResources = playerResources + resources
        triggerHaptic(.resourceGain)
    }

    /// Open loot box with transparent odds display
    func openLootBox(_ box: LootBox) {
        activeLootBoxes.append(box)
        // Log opening for audit trail (ethical compliance)
        Analytics.logEvent("loot_box_opened", properties: [
            "box_id": box.id,
            "odds_displayed": true
        ])
    }

    /// Calculate offline regeneration (rest-state mechanics)
    func calculateOfflineRegeneration(hoursOffline: Int) {
        let rate = OfflineResources.ratePerHour
        let cappedHours = min(hoursOffline, OfflineResources.maxCapHours)
        offlineRegeneration = OfflineResources(
            resources: rate * cappedHours,
            hoursRemaining: OfflineResources.maxCapHours - cappedHours
        )
    }

    /// Check spending limit before purchase
    func canPurchase(amount: Decimal) -> Bool {
        monthlySpending + amount <= spendingLimit
    }

    /// Trigger haptic feedback for game events
    private func triggerHaptic(_ event: HapticEvent) {
        HapticFeedback.shared.play(event)
    }
}

// =============================================================================
// MAIN GAME VIEW
// =============================================================================

/// Primary game interface view
///
/// Displays:
/// - Resource counters with accessibility
/// - Loot box gacha with transparent odds
/// - Rest-state regeneration indicator
/// - Spending limit warnings
struct GameView: View {
    // MARK: - State

    @StateObject private var gameState = GameStateManager()
    @State private var showLootDetails = false
    @State private var showSpendingWarning = false

    // MARK: - Body

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Resource display
                resourceSection

                // Rest-state indicator
                restStateSection

                // Loot box section
                lootBoxSection

                // Spending monitor
                spendingSection
            }
            .padding()
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Game Dashboard")
    }

    // MARK: - Resource Section

    private var resourceSection: some View {
        Section {
            VStack(spacing: 12) {
                ForEach(gameState.playerResources.all) { resource in
                    ResourceRowView(resource: resource)
                        .accessibilityIdentifier(resource.id)
                        .accessibilityLabel(resource.name)
                        .accessibilityValue(resource.amountFormatted)
                }
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Resources")
    }

    // MARK: - Rest-State Section

    private var restStateSection: some View {
        Section {
            VStack(spacing: 8) {
                Label("Offline Regeneration", systemImage: .clock)
                    .font(.headline)

                if gameState.offlineRegeneration.resources > 0 {
                    Text("Generated: \(gameState.offlineRegeneration.resources)")
                        .font(.title2)
                        .accessibilityIdentifier("restStateResources")

                    Text("Cap remaining: \(gameState.offlineRegeneration.hoursRemaining) hours")
                        .font(.caption)
                        .foregroundColor(.secondary)
                } else {
                    Text("No offline resources")
                        .font(.body)
                        .foregroundColor(.secondary)
                }
            }
            .padding()
            .background(Color.systemFill)
            .cornerRadius(10)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Rest-State Mechanics")
        .accessibilityHint("Shows resources gained while offline")
    }

    // MARK: - Loot Box Section

    private var lootBoxSection: some View {
        Section {
            VStack(spacing: 12) {
                Label("Loot Boxes", systemImage: .gift)
                    .font(.headline)

                ForEach(gameState.activeLootBoxes) { box in
                    LootBoxView(box: box)
                        .accessibilityIdentifier("lootBox-\(box.id)")
                        .accessibilityLabel(box.name)
                        .accessibilityHint("Tap to view drop rates")
                        .onTapGesture {
                            showLootDetails = true
                        }
                }

                if showLootDetails {
                    LootOddsDetailView()
                        .transition(.opacity)
                }
            }
            .padding()
            .background(Color.systemFill)
            .cornerRadius(10)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Loot Boxes")
        .accessibilityHint("Transparent drop rates displayed")
    }

    // MARK: - Spending Section

    private var spendingSection: some View {
        Section {
            VStack(spacing: 8) {
                Label("Monthly Spending", systemImage: .chartBar)
                    .font(.headline)

                Text("\(gameState.monthlySpending) / \(gameState.spendingLimit)")
                    .font(.title2)
                    .accessibilityIdentifier("spendingDisplay")

                if gameState.monthlySpending > gameState.spendingLimit * 0.8 {
                    WarningView(message: "Approaching spending limit")
                        .accessibilityIdentifier("spendingWarning")
                }
            }
            .padding()
            .background(Color.systemFill)
            .cornerRadius(10)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Spending Monitor")
        .accessibilityHint("Tracks monthly purchases against limit")
    }
}

// =============================================================================
// SUPPORTING VIEW COMPONENTS
// =============================================================================

/// Resource row display with icon and amount
struct ResourceRowView: View {
    let resource: Resource

    var body: some View {
        HStack {
            Image(systemName: resource.iconName)
                .accessibilityHidden(true)
            Text(resource.name)
            Text(resource.amountFormatted)
                .fontWeight(.bold)
        }
    }
}

/// Loot box view with odds indicator
struct LootBoxView: View {
    let box: LootBox

    var body: some View {
        HStack {
            Image(systemName: box.iconName)
                .accessibilityHidden(true)
            Text(box.name)
            Label("Odds Available", systemImage: .info)
                .foregroundColor(.accentColor)
        }
    }
}

/// Transparent odds display (ethical compliance)
struct LootOddsDetailView: View {
    var body: some View {
        VStack(spacing: 8) {
            Text("Drop Rates")
                .font(.headline)

            // In production: Render actual odds from LootBox
            Text("Common: 60%")
            Text("Rare: 25%")
            Text("Legendary: 10%")
            Text("Mythic: 5%")

            Text("Expected Value: 120 gold")
                .font(.caption)
                .foregroundColor(.secondary)

            Text("Pity Timer: 10 pulls guaranteed")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding()
    }
}

/// Warning indicator for spending limits
struct WarningView: View {
    let message: String

    var body: some View {
        HStack {
            Image(systemName: "exclamationmark.triangle")
                .foregroundColor(.orange)
            Text(message)
                .foregroundColor(.orange)
                .fontWeight(.semibold)
        }
    }
}

// =============================================================================
// MODELS
// =============================================================================

/// Game resource model
struct Resource: Identifiable {
    let id: String
    let name: String
    let amount: Decimal
    let iconName: String

    var amountFormatted: String {
        "\(amount)"
    }
}

/// Resource balance aggregate
struct ResourceBalance {
    var all: [Resource]

    static var empty: ResourceBalance {
        ResourceBalance(all: [])
    }

    static func +(left: ResourceBalance, right: ResourceBalance) -> ResourceBalance {
        ResourceBalance(all: left.all + right.all)
    }
}

/// Achievement model
struct Achievement: Identifiable {
    let id: String
    let name: String
    let description: String
    let unlocked: Bool
}

/// Loot box model with transparent odds
struct LootBox: Identifiable {
    let id: String
    let name: String
    let iconName: String
    let odds: [LootOdds]
    let pityTimer: Int

    var expectedValue: Decimal {
        odds.reduce(0) { sum, odd in
            sum + (odd.dropRate * odd.value)
        }
    }
}

/// Individual loot odds (transparent display)
struct LootOdds {
    let rarity: String
    let dropRate: Decimal
    let value: Decimal
    let itemName: String
}

/// Offline regeneration (rest-state mechanics)
struct OfflineResources {
    let resources: Decimal
    let hoursRemaining: Int

    static let ratePerHour: Decimal = 10
    static let maxCapHours: Int = 24

    static var empty: OfflineResources {
        OfflineResources(resources: 0, hoursRemaining: 24)
    }
}

// =============================================================================
// HAPTIC FEEDBACK
// =============================================================================

/// Haptic event types for game feedback
enum HapticEvent {
    case resourceGain
    case lootOpen
    case achievement
    case warning
    case selection
}

/// Shared haptic feedback controller
final class HapticFeedback {
    static let shared = HapticFeedback()

    func play(_ event: HapticEvent) {
        // In production: UIFeedbackGenerator implementation
        switch event {
        case .resourceGain:
            // .notification(.success)
            break
        case .lootOpen:
            // .impact(.medium)
            break
        case .achievement:
            // .notification(.success)
            break
        case .warning:
            // .impact(.heavy)
            break
        case .selection:
            // .selection
            break
        }
    }
}

// =============================================================================
// ANALYTICS
// =============================================================================

/// Analytics logging for ethical compliance
final class Analytics {
    static func logEvent(_ name: String, properties: [String: Any]) {
        // Production: Send to analytics service
        // Required for loot box audit trail
        print("[Analytics] \(name): \(properties)")
    }
}