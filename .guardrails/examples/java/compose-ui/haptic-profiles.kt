/*
 * haptic-profiles.kt - Haptic UI Profiles for Every Interaction
 *
 * Production-ready example demonstrating:
 * - Tactile feedback patterns for game UI
 * - Haptic intensity scaling with DDA 3.0
 * - Accessibility: haptic redundancy for visual impairments
 * - Rest-State Mechanics: calm haptic profiles
 * - Ethical engagement: non-addictive feedback loops
 *
 * © 2026 - Agent Guardrails Template
 */

package com.agentguardrails.composeui

import androidx.compose.haptic.HapticFeedback
import androidx.compose.haptic.HapticPattern
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.game.core.DDAManager
import androidx.game.ethical.EthicalEngagement

/**
 * Haptic profile definitions for game UI interactions
 * Each profile maps to specific game events with intensity scaling
 */
enum class HapticProfile {
    // Core Loop: Action Phase
    COMBAT_TRIGGER,       // Enemy engagement
    COMBAT_SUCCESS,       // Enemy defeated
    COMBAT_FAILURE,       // Player damaged

    // Core Loop: Reward Phase
    LOFT_REVEAL,          // Loot box open
    LOFT_LEGENDARY,       // Legendary item drop
    LOFT_COMMON,          // Common item drop

    // Core Loop: Upgrade Phase
    SKILL_UNLOCK,         // New skill acquired
    LEVEL_UP,             // Character level increase
    STAT_INCREASE,        // Attribute point added

    // Navigation
    MENU_OPEN,            // Menu expansion
    MENU_SELECT,          // Option selection
    MENU_CLOSE,           // Menu dismissal

    // Ethical Engagement
    REST_STATE_ENTER,     // Calm mode activation
    REST_STATE_EXIT,      // Session resume

    // Accessibility
    ERROR_ALERT,          // Non-visual error notification
    SUCCESS_CONFIRMATION, // Non-visual success confirmation

    // Spatial Computing (XR)
    XR_GRAB,              // Virtual object grab
    XR_RELEASE,           // Virtual object release
    XR_COLLISION,         // Virtual collision detection
}

/**
 * Haptic intensity levels scale with DDA 3.0 tier
 * Prevents overstimulation for stressed players
 */
enum class HapticIntensity {
    LIGHT,      // 20% intensity - Rest State
    MODERATE,   // 50% intensity - Normal play
    STRONG,     // 80% intensity - High engagement
    MAXIMUM     // 100% intensity - Critical moments
}

/**
 * Haptic feedback manager with DDA 3.0 adaptation
 */
object HapticProfiles {

    private val ddaManager = DDAManager()
    private val ethicalEngagement = EthicalEngagement()

    /**
     * Applies haptic profile with intensity scaling
     * DDA 3.0: Reduces intensity for stressed players
     */
    fun applyProfile(profile: HapticProfile) {
        val intensity = calculateIntensity(profile)
        val pattern = getPattern(profile, intensity)

        HapticFeedback.perform(pattern)
    }

    /**
     * Calculates haptic intensity based on DDA tier and game state
     */
    private fun calculateIntensity(profile: HapticProfile): HapticIntensity {
        val ddaTier = ddaManager.currentTier

        return when (ddaTier) {
            DDAManager.Tier.DIFFICULT -> {
                // Stressed players: reduce intensity
                HapticIntensity.LIGHT
            }
            DDAManager.Tier.NORMAL -> {
                // Standard intensity
                HapticIntensity.MODERATE
            }
            DDAManager.Tier.RELAXED -> {
                // Engaged players: moderate-strong intensity
                when (profile) {
                    HapticProfile.LOFT_LEGENDARY, HapticProfile.LEVEL_UP ->
                        HapticIntensity.STRONG
                    else -> HapticIntensity.MODERATE
                }
            }
        }
    }

    /**
     * Maps profile to haptic pattern with intensity
     */
    private fun getPattern(profile: HapticProfile, intensity: HapticIntensity): HapticPattern {
        return when (profile) {
            HapticProfile.COMBAT_TRIGGER -> {
                HapticPattern(
                    amplitude = intensity.amplitudeFactor,
                    duration = 50,
                    frequency = 150
                )
            }
            HapticProfile.COMBAT_SUCCESS -> {
                HapticPattern(
                    amplitude = intensity.amplitudeFactor,
                    duration = 100,
                    frequency = 200,
                    pattern = HapticPattern.Pattern.PULSE
                )
            }
            HapticProfile.LOFT_LEGENDARY -> {
                HapticPattern(
                    amplitude = intensity.amplitudeFactor,
                    duration = 300,
                    frequency = 250,
                    pattern = HapticPattern.Pattern.CASCADE
                )
            }
            HapticProfile.LEVEL_UP -> {
                HapticPattern(
                    amplitude = intensity.amplitudeFactor,
                    duration = 500,
                    frequency = 300,
                    pattern = HapticPattern.Pattern.CELEBRATION
                )
            }
            HapticProfile.REST_STATE_ENTER -> {
                HapticPattern(
                    amplitude = 0.2f, // Fixed light intensity
                    duration = 200,
                    frequency = 80,
                    pattern = HapticPattern.Pattern.GENTLE
                )
            }
            HapticProfile.ERROR_ALERT -> {
                // Accessibility: distinct pattern for error states
                HapticPattern(
                    amplitude = 0.6f,
                    duration = 150,
                    frequency = 100,
                    pattern = HapticPattern.Pattern.WARNING
                )
            }
            else -> {
                HapticPattern.DEFAULT
            }
        }
    }

    /**
     * Rest-State haptic profile
     * Ethical engagement: calming feedback after intense sessions
     */
    @Composable
    fun RestStateHapticFeedback(sessionDuration: Int) {
        val restStateTimer = remember { mutableStateOf(0) }

        if (sessionDuration > 45) {
            // Gentle haptic reminder for rest state
            applyProfile(HapticProfile.REST_STATE_ENTER)
            restStateTimer.value++

            if (restStateTimer.value >= 5) {
                ethicalEngagement.enterRestState()
            }
        }
    }

    /**
     * Accessibility haptic redundancy
     * Non-visual confirmation for impaired players
     */
    fun AccessibilityHapticConfirmation(success: Boolean) {
        val profile = if (success)
            HapticProfile.SUCCESS_CONFIRMATION
            else HapticProfile.ERROR_ALERT

        applyProfile(profile)
    }

    /**
     * XR haptic feedback for spatial computing
     * Z-depth collision detection through tactile feedback
     */
    fun XRHapticFeedback(event: XREvent, depth: Float) {
        val intensity = when {
            depth > 0.7f -> HapticIntensity.STRONG // Close collision
            depth > 0.3f -> HapticIntensity.MODERATE // Mid-range
            else -> HapticIntensity.LIGHT // Distant
        }

        val profile = when (event) {
            XREvent.GRAB -> HapticProfile.XR_GRAB
            XREvent.RELEASE -> HapticProfile.XR_RELEASE
            XREvent.COLLISION -> HapticProfile.XR_COLLISION
        }

        applyProfile(profile)
    }
}

/**
 * Haptic pattern definition with amplitude, duration, frequency
 */
data class HapticPattern(
    val amplitude: Float,
    val duration: Int,
    val frequency: Int,
    val pattern: Pattern = Pattern.SIMPLE
) {
    enum class Pattern {
        SIMPLE, PULSE, CASCADE, CELEBRATION, GENTLE, WARNING
    }

    companion object {
        val DEFAULT = HapticPattern(0.5f, 100, 150)
    }
}

/**
 * Intensity amplitude factor mapping
 */
val HapticIntensity.amplitudeFactor: Float
    get() = when (this) {
        HapticIntensity.LIGHT -> 0.2f
        HapticIntensity.MODERATE -> 0.5f
        HapticIntensity.STRONG -> 0.8f
        HapticIntensity.MAXIMUM -> 1.0f
    }

/**
 * XR event types for spatial computing
 */
enum class XREvent {
    GRAB, RELEASE, COLLISION
}