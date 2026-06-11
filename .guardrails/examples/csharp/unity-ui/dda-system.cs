/*
 * dda-system.cs - DDA 3.0 Enemy AI Heuristic Adjustment
 *
 * Production-ready example demonstrating:
 * - Dynamic Difficulty Adjustment (DDA 3.0)
 * - Enemy AI heuristic adaptation
 * - UI visual adaptation based on player stress
 * - Performance metrics drive aesthetic adjustments
 * - Emergent gameplay through difficulty scaling
 *
 * © 2026 - Agent Guardrails Template
 */

using UnityEngine;
using Unity.Entities;
using Unity.DOTS;
using Unity.Jobs;
using Unity.Burst;
using Unity.Collections;
using AgentGuardrails.Accessibility;
using AgentGuardrails.Ethical;

namespace AgentGuardrails.DDA
{
    /// <summary>
    /// DDA 3.0 Manager - Dynamic Difficulty Adjustment System
    /// Adjusts enemy AI heuristics and UI based on player performance
    /// </summary>
    public class DDAManager : MonoBehaviour
    {
        private static DDAManager _instance;
        public static DDAManager Instance => _instance;

        private EntityManager entityManager;
        private NativeArray<Entity> enemyEntities;
        private PlayerPerformanceMetrics performanceMetrics;

        public DifficultyTier CurrentTier { get; private set; }
        public float UIOpacityFactor { get; private set; }

        public enum DifficultyTier
        {
            Difficult,    // Stressed players - reduced complexity
            Normal,       // Standard presentation
            Relaxed       // High engagement - enhanced feedback
        }

        public void Awake()
        {
            _instance = this;
            entityManager = EntityManager.Instance;
            enemyEntities = new NativeArray<Entity>(50, Allocator.Persistent);
            performanceMetrics = new PlayerPerformanceMetrics();
        }

        /// <summary>
        /// Adjusts enemy AI heuristics based on player performance
        /// DDA 3.0 core: adapts difficulty to player KDR and stress
        /// </summary>
        public void AdjustEnemyHeuristics(float playerKDR, int sessionDuration)
        {
            // Calculate DDA tier from performance metrics
            CurrentTier = CalculateDifficultyTier(playerKDR, sessionDuration);

            // Adjust enemy AI heuristics
            AdjustEnemyAI(CurrentTier);

            // Adjust UI opacity for stressed players
            UIOpacityFactor = CalculateUIOpacity(CurrentTier);

            // Notify ethical engagement system
            EthicalEngagement.Instance.OnDDATierChange(CurrentTier);
        }

        /// <summary>
        /// Calculates DDA tier from player performance
        /// KDR (Kill/Death Ratio) and session duration factors
        /// </summary>
        private DifficultyTier CalculateDifficultyTier(float playerKDR, int sessionDuration)
        {
            // DDA 3.0 algorithm:
            // - KDR < 0.5: Difficult (stressed player)
            // - KDR 0.5-1.5: Normal (balanced)
            // - KDR > 1.5: Relaxed (dominant player)

            if (playerKDR < 0.5f || sessionDuration > 45)
            {
                return DifficultyTier.Difficult;
            }
            else if (playerKDR < 1.5f)
            {
                return DifficultyTier.Normal;
            }
            else
            {
                return DifficultyTier.Relaxed;
            }
        }

        /// <summary>
        /// Adjusts enemy AI behavior based on DDA tier
        /// Emergent gameplay: AI adapts to player skill
        /// </summary>
        private void AdjustEnemyAI(DifficultyTier tier)
        {
            var aiJob = new EnemyAIAdjustmentJob
            {
                enemyEntities = enemyEntities,
                tier = tier,
                aggressionFactor = CalculateAggressionFactor(tier),
                accuracyFactor = CalculateAccuracyFactor(tier)
            };

            // Schedule AI adjustment on DOTS job system
            var handle = aiJob.Schedule();
            handle.Complete();
        }

        /// <summary>
        /// Calculates UI opacity factor for DDA tier
        /// Reduces visual complexity for stressed players
        /// </summary>
        private float CalculateUIOpacity(DifficultyTier tier)
        {
            switch (tier)
            {
                case DifficultyTier.Difficult:
                    // Stressed players: reduce opacity (0.7)
                    return 0.7f;
                case DifficultyTier.Normal:
                    // Standard opacity (1.0)
                    return 1.0f;
                case DifficultyTier.Relaxed:
                    // Engaged players: slight enhancement (1.2)
                    return 1.2f;
            }
        }

        /// <summary>
        /// Calculates enemy aggression factor
        /// Higher tier = more aggressive AI
        /// </summary>
        private float CalculateAggressionFactor(DifficultyTier tier)
        {
            switch (tier)
            {
                case DifficultyTier.Difficult:
                    return 0.5f; // Reduced aggression
                case DifficultyTier.Normal:
                    return 1.0f; // Standard aggression
                case DifficultyTier.Relaxed:
                    return 1.5f; // Enhanced aggression
            }
        }

        /// <summary>
        /// Calculates enemy accuracy factor
        /// Scales with player performance
        /// </summary>
        private float CalculateAccuracyFactor(DifficultyTier tier)
        {
            switch (tier)
            {
                case DifficultyTier.Difficult:
                    return 0.6f; // Reduced accuracy
                case DifficultyTier.Normal:
                    return 1.0f; // Standard accuracy
                case DifficultyTier.Relaxed:
                    return 1.4f; // Enhanced accuracy
            }
        }

        /// <summary>
        /// DOTS Job for enemy AI adjustment
        /// Burst-compiled for performance
        /// </summary>
        [BurstCompile]
        public struct EnemyAIAdjustmentJob : IJob
        {
            public NativeArray<Entity> enemyEntities;
            public DifficultyTier tier;
            public float aggressionFactor;
            public float accuracyFactor;

            [BurstCompile]
            public void Execute()
            {
                for (int i = 0; i < enemyEntities.Length; i++)
                {
                    var enemy = enemyEntities[i];
                    var aiComponent = enemy.Get<EnemyAIComponent>();

                    // Adjust AI heuristics
                    aiComponent.aggression = aggressionFactor;
                    aiComponent.accuracy = accuracyFactor;
                    aiComponent.reactionTime = CalculateReactionTime(tier);

                    enemy.Set(aiComponent);
                }
            }

            private float CalculateReactionTime(DifficultyTier tier)
            {
                switch (tier)
                {
                    case DifficultyTier.Difficult:
                        return 1.5f; // Slower reaction
                    case DifficultyTier.Normal:
                        return 1.0f; // Standard reaction
                    case DifficultyTier.Relaxed:
                        return 0.7f; // Faster reaction
                }
            }
        }

        /// <summary>
        /// Player performance metrics for DDA calculation
        /// Tracks KDR, session duration, stress indicators
        /// </summary>
        public struct PlayerPerformanceMetrics
        {
            public float KDR;           // Kill/Death Ratio
            public int sessionDuration; // Minutes
            public float stressLevel;   // 0.0-1.0
            public float accuracy;      // Hit accuracy
            public int wins;
            public int losses;

            public void Update(float kdr, int duration, float stress)
            {
                KDR = kdr;
                sessionDuration = duration;
                stressLevel = stress;
            }
        }

        /// <summary>
        /// Enemy AI Component for ECS
        /// Stores heuristic values for DDA adjustment
        /// </summary>
        public struct EnemyAIComponent : IComponent
        {
            public float aggression;
            public float accuracy;
            public float reactionTime;
            public float detectionRange;
            public int behaviorState;
        }
    }

    /// <summary>
    /// UI Adaptation System for DDA 3.0
    /// Visual complexity adjusts based on tier
    /// </summary>
    public class UIAdaptationSystem : MonoBehaviour
    {
        private DDAManager ddaManager;

        public void Initialize()
        {
            ddaManager = DDAManager.Instance;
        }

        public void AdaptUI()
        {
            var tier = ddaManager.CurrentTier;
            var opacity = ddaManager.UIOpacityFactor;

            // Reduce visual complexity for stressed players
            if (tier == DDAManager.DifficultyTier.Difficult)
            {
                SimplifyUIComplexity();
                ReduceParticleEffects();
                LowerHapticIntensity();
            }
            else if (tier == DDAManager.DifficultyTier.Relaxed)
            {
                EnhanceVisualFeedback();
                IncreaseParticleEffects();
                RaiseHapticIntensity();
            }
        }

        private void SimplifyUIComplexity()
        {
            // Hide decorative elements
            UISystem.Instance.HideDecorativeElements();

            // Reduce animation complexity
            UISystem.Instance.SimplifyAnimations();
        }

        private void ReduceParticleEffects()
        {
            // Lower particle density
            ParticleSystem.Instance.SetDensity(0.5f);
        }

        private void LowerHapticIntensity()
        {
            // Reduce haptic feedback intensity
            HapticSystem.Instance.SetIntensity(HapticIntensity.Light);
        }

        private void EnhanceVisualFeedback()
        {
            // Show additional visual cues
            UISystem.Instance.ShowEnhancedCues();
        }

        private void IncreaseParticleEffects()
        {
            // Increase particle density
            ParticleSystem.Instance.SetDensity(1.5f);
        }

        private void RaiseHapticIntensity()
        {
            // Increase haptic feedback intensity
            HapticSystem.Instance.SetIntensity(HapticIntensity.Strong);
        }
    }

    /// <summary>
    /// Emergent Gameplay Controller
    /// DDA-driven emergent behaviors
    /// </summary>
    public class EmergentGameplayController : MonoBehaviour
    {
        private DDAManager ddaManager;

        public void OnDDATierChange(DDAManager.DifficultyTier tier)
        {
            // Trigger emergent gameplay events
            switch (tier)
            {
                case DDAManager.DifficultyTier.Difficult:
                    // Player struggling: spawn help event
                    SpawnHelpEvent();
                    break;
                case DDAManager.DifficultyTier.Relaxed:
                    // Player dominant: spawn challenge event
                    SpawnChallengeEvent();
                    break;
            }
        }

        private void SpawnHelpEvent()
        {
            // Emergent gameplay: helpful NPC appears
            EventSystem.Instance.Trigger("HelpNPCSpawn");
        }

        private void SpawnChallengeEvent()
        {
            // Emergent gameplay: elite enemy spawns
            EventSystem.Instance.Trigger("EliteEnemySpawn");
        }
    }

    /// <summary>
    /// Haptic intensity levels for DDA adaptation
    /// </summary>
    public enum HapticIntensity
    {
        Light,    // 20% - Difficult tier
        Moderate, // 50% - Normal tier
        Strong    // 80% - Relaxed tier
    }
}