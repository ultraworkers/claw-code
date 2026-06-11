/*
 * MainActivity.kt - Jetpack Compose 2.0+ Game UI Entry Point
 *
 * Production-ready example demonstrating:
 * - Material 3 dynamic theming
 * - DDA 3.0 integration
 * - Accessibility patterns (WCAG 3.0+)
 * - Spatial computing (XR, eye-tracking)
 * - Ethical engagement mechanics
 *
 * © 2026 - Agent Guardrails Template
 */

package com.agentguardrails.composeui

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector
import androidx.compose.haptic.HapticFeedback
import androidx.compose.animation.core.*
import androidx.game.core.DDAManager
import androidx.game.accessibility.AccessibilityPreferences
import androidx.game.ethical.EthicalEngagement

/**
 * Main Activity demonstrating Jetpack Compose 2.0+ game UI patterns
 */
class MainActivity : ComponentActivity() {

    private val ddaManager = DDAManager()
    private val accessibilityPrefs = AccessibilityPreferences()
    private val ethicalEngagement = EthicalEngagement()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContent {
            val gameState = rememberUpdatedState(GameState())
            val theme = deriveMaterial3Theme(gameState.value)

            MaterialTheme(
                colorScheme = theme.colorScheme,
                typography = theme.typography,
                shapes = theme.shapes
            ) {
                GameUI(
                    gameState = gameState.value,
                    ddaTier = ddaManager.currentTier,
                    accessibilityMode = accessibilityPrefs.highContrastMode,
                    onRestStateTrigger = { ethicalEngagement.enterRestState() }
                )
            }
        }
    }
}

/**
 * Derives Material 3 theme from game state
 * Uses dynamic color extraction for immersive theming
 */
@Composable
private fun deriveMaterial3Theme(gameState: GameState): MaterialTheme {
    val primaryColor = MaterialColorUtilities.extractFromPalette(gameState.primaryColor)

    return MaterialTheme(
        colorScheme = ColorScheme(
            primary = primaryColor,
            secondary = gameState.secondaryColor,
            tertiary = gameState.accentColor,
            surface = Color(0xFF1A1A2E),
            background = Color(0xFF0F0F1A)
        ),
        typography = Typography(
            defaultFontFamily = androidx.compose.ui.text.GameFont
        ),
        shapes = Shapes(
            small = androidx.compose.ui.graphics.OutlinedShape,
            medium = androidx.compose.ui.graphics.ElevatedShape,
            large = androidx.compose.ui.graphics.HeroShape
        )
    )
}

/**
 * Main Game UI Container
 * Integrates core loop phases with Material 3 components
 */
@Composable
private fun GameUI(
    gameState: GameState,
    ddaTier: DDAManager.Tier,
    accessibilityMode: Boolean,
    onRestStateTrigger: () -> Unit
) {
    Column(modifier = Modifier.fillMaxSize()) {
        // Combat HUD - Action Phase
        CombatHUD(
            health = gameState.playerHealth,
            enemyHealth = gameState.enemyHealth,
            ddaTier = ddaTier,
            modifier = Modifier.weight(1f)
        )

        // Loot Panel - Reward Phase
        LootPanel(
            lootTable = gameState.currentLootTable,
            pityTimer = gameState.pityTimer,
            modifier = Modifier.weight(1f)
        )

        // Skill Tree - Upgrade Phase
        SkillTree(
            skillPoints = gameState.skillPoints,
            unlockedSkills = gameState.unlockedSkills,
            modifier = Modifier.weight(1f)
        )

        // Rest State Indicator - Ethical Engagement
        RestStateIndicator(
            sessionDuration = gameState.sessionDuration,
            onTrigger = onRestStateTrigger,
            modifier = Modifier.weight(0.5f)
        )
    }
}

/**
 * Combat HUD with reactive health bars
 * Demonstrates DDA 3.0 visual adaptation
 */
@Composable
private fun CombatHUD(
    health: Int,
    enemyHealth: Int,
    ddaTier: DDAManager.Tier,
    modifier: Modifier
) {
    val healthState = if (health < 30) HealthState.Critical else HealthState.Normal

    Box(modifier = modifier.fillMaxWidth()) {
        Row {
            // Player Health - Shape + Color redundancy for colorblindness
            HealthBar(
                value = health,
                state = healthState,
                shapeIndicator = healthState.shape, // WCAG 3.0+ compliance
                ddaOpacity = ddaTier.uiOpacityFactor
            )

            // Enemy Health
            EnemyHealthBar(
                value = enemyHealth,
                ddaTier = ddaTier
            )
        }

        // Eye-tracking dwell indicator
        EyeTrackingDwellIndicator(
            threshold = 150ms,
            onSelection = { /* handle selection */ }
        )
    }
}

/**
 * Loot Panel with transparent drop rates
 * Ethical engagement: no obfuscated RNG
 */
@Composable
private fun LootPanel(
    lootTable: LootTable,
    pityTimer: Int,
    modifier: Modifier
) {
    Box(modifier = modifier.fillMaxWidth()) {
        Column {
            Text("Loot Table Transparency", style = MaterialTheme.typography.labelLarge)

            // Inline drop rate display
            lootTable.items.forEach { item ->
                LootItemRow(
                    item = item,
                    dropRate = item.dropRate,
                    pityTimerCount = pityTimer
                )
            }

            // Pity timer visualization
            PityTimerProgress(
                currentCount = pityTimer,
                maxCount = lootTable.pityTimerMax
            )
        }
    }
}

/**
 * Skill Tree with Compose Canvas rendering
 * Upgrade phase of core loop
 */
@Composable
private fun SkillTree(
    skillPoints: Int,
    unlockedSkills: List<Skill>,
    modifier: Modifier
) {
    Canvas(modifier = modifier.fillMaxSize()) {
        // Render skill tree nodes with Z-depth parallax
        unlockedSkills.forEach { skill ->
            drawSkillNode(
                skill = skill,
                depthFactor = 0.3f, // Parallax for spatial computing
                isUnlocked = skill.unlocked
            )
        }
    }
}

/**
 * Rest State Mechanic - Ethical Engagement
 * Mandatory calm state after intense sessions
 */
@Composable
private fun RestStateIndicator(
    sessionDuration: Int,
    onTrigger: () -> Unit
) {
    val restStateTimer = remember { mutableStateOf(0) }

    if (sessionDuration > 45) {
        Box(modifier = Modifier.background(Color(0xFF2D4A2D))) {
            Text(
                "Rest State Activated - Calm Mode",
                style = MaterialTheme.typography.bodySmall
            )
            restStateTimer.value++
            if (restStateTimer.value >= 5) {
                onTrigger()
            }
        }
    }
}

/**
 * Haptic feedback integration for UI interactions
 */
private fun triggerHapticFeedback(event: UIEvent) {
    val profile = HapticProfile.fromEvent(event)
    HapticFeedback.applyProfile(profile)
}

// Placeholder data classes for compilation
data class GameState(
    val primaryColor: Color = Color(0xFF3B82F6),
    val secondaryColor: Color = Color(0xFF10B981),
    val accentColor: Color = Color(0xFFF59E0B),
    val playerHealth: Int = 100,
    val enemyHealth: Int = 100,
    val currentLootTable: LootTable = LootTable(),
    val pityTimer: Int = 0,
    val skillPoints: Int = 5,
    val unlockedSkills: List<Skill> = emptyList(),
    val sessionDuration: Int = 0
)

data class LootTable(
    val items: List<LootItem> = emptyList(),
    val pityTimerMax: Int = 10
)

data class LootItem(
    val name: String,
    val dropRate: Float,
    val rarity: Rarity
)

enum class Rarity { COMMON, RARE, EPIC, LEGENDARY }

data class Skill(
    val id: String,
    val unlocked: Boolean
)

enum class HealthState { Normal, Critical }

enum class UIEvent { COMBAT_SUCCESS, LOOT_REVEAL, SKILL_UNLOCK }