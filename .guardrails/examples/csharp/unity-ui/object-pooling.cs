/*
 * object-pooling.cs - Object Pooling for UI Elements
 *
 * Production-ready example demonstrating:
 * - Object pooling patterns for UI elements
 * - ECS-based pooled entity management
 * - DOTS job system for pool operations
 * - UI prefetching for performance
 * - Memory-efficient UI recycling
 *
 * © 2026 - Agent Guardrails Template
 */

using UnityEngine;
using Unity.Entities;
using Unity.DOTS;
using Unity.Jobs;
using Unity.Burst;
using Unity.Collections;
using AgentGuardrails.Performance;

namespace AgentGuardrails.UnityUI
{
    /// <summary>
    /// UI Object Pool Manager
    /// Efficient recycling of UI elements through ECS
    /// Prevents memory allocation spikes during gameplay
    /// </summary>
    public class UIObjectPoolManager : MonoBehaviour
    {
        private static UIObjectPoolManager _instance;
        public static UIObjectPoolManager Instance => _instance;

        private EntityManager entityManager;
        private NativeArray<Entity> pooledEntities;
        private int poolSize = 100;
        private int activeCount = 0;

        public void Awake()
        {
            _instance = this;
            entityManager = EntityManager.Instance;
            pooledEntities = new NativeArray<Entity>(poolSize, Allocator.Persistent);

            // Pre-create pooled entities
            InitializePool();
        }

        /// <summary>
        /// Initializes object pool with pre-created entities
        /// ECS-based pooling for UI elements
        /// </summary>
        private void InitializePool()
        {
            var initJob = new UIPoolInitializationJob
            {
                entityManager = entityManager,
                pooledEntities = pooledEntities,
                poolSize = poolSize
            };

            var handle = initJob.Schedule();
            handle.Complete();
        }

        /// <summary>
        /// Acquires UI element from pool
        /// Returns pooled entity instead of creating new
        /// </summary>
        public Entity AcquireFromPool()
        {
            if (activeCount < poolSize)
            {
                var entity = pooledEntities[activeCount];
                entity.Set(new UIElementComponent { isActive = true });
                activeCount++;
                return entity;
            }

            // Pool exhausted: expand pool
            ExpandPool(10);
            return pooledEntities[activeCount];
        }

        /// <summary>
        /// Returns UI element to pool
        /// Resets component state for recycling
        /// </summary>
        public void ReturnToPool(Entity entity)
        {
            // Reset entity state
            entity.Set(new UIElementComponent { isActive = false });
            entity.Set(new LayoutComponent { position = Vector3.zero });

            // Find entity in pool and mark as available
            for (int i = 0; i < poolSize; i++)
            {
                if (pooledEntities[i] == entity)
                {
                    activeCount--;
                    break;
                }
            }
        }

        /// <summary>
        /// Expands pool capacity
        /// Dynamic allocation when pool exhausted
        /// </summary>
        private void ExpandPool(int additionalSize)
        {
            var newPool = new NativeArray<Entity>(poolSize + additionalSize, Allocator.Persistent);

            // Copy existing entities
            for (int i = 0; i < poolSize; i++)
            {
                newPool[i] = pooledEntities[i];
            }

            // Create new entities
            var expandJob = new UIPoolExpansionJob
            {
                entityManager = entityManager,
                pooledEntities = newPool,
                startIndex = poolSize,
                count = additionalSize
            };

            var handle = expandJob.Schedule();
            handle.Complete();

            // Replace old pool
            pooledEntities.Dispose();
            pooledEntities = newPool;
            poolSize += additionalSize;
        }

        /// <summary>
        /// Prefetches UI elements for upcoming scene
        /// Predictive pooling for performance
        /// </summary>
        public void PrefetchUIElements(int predictedCount)
        {
            var prefetchJob = new UIPrefetchJob
            {
                pooledEntities = pooledEntities,
                startIndex = activeCount,
                count = predictedCount
            };

            var handle = prefetchJob.Schedule();
            handle.Complete();
        }
    }

    /// <summary>
    /// UI Pool Initialization Job
    /// DOTS-based pool setup
    /// </summary>
    [BurstCompile]
    public struct UIPoolInitializationJob : IJob
    {
        public EntityManager entityManager;
        public NativeArray<Entity> pooledEntities;
        public int poolSize;

        [BurstCompile]
        public void Execute()
        {
            for (int i = 0; i < poolSize; i++)
            {
                var entity = entityManager.CreateEntity();
                entity.AddComponent<UIElementComponent>();
                entity.AddComponent<LayoutComponent>();
                entity.AddComponent<RenderComponent>();
                entity.AddComponent<PooledComponent>();

                pooledEntities[i] = entity;
            }
        }
    }

    /// <summary>
    /// UI Pool Expansion Job
    /// DOTS-based pool growth
    /// </summary>
    [BurstCompile]
    public struct UIPoolExpansionJob : IJob
    {
        public EntityManager entityManager;
        public NativeArray<Entity> pooledEntities;
        public int startIndex;
        public int count;

        [BurstCompile]
        public void Execute()
        {
            for (int i = startIndex; i < startIndex + count; i++)
            {
                var entity = entityManager.CreateEntity();
                entity.AddComponent<UIElementComponent>();
                entity.AddComponent<LayoutComponent>();
                entity.AddComponent<RenderComponent>();
                entity.AddComponent<PooledComponent>();

                pooledEntities[i] = entity;
            }
        }
    }

    /// <summary>
    /// UI Prefetch Job
    /// Pre-activates pooled entities for predicted usage
    /// </summary>
    [BurstCompile]
    public struct UIPrefetchJob : IJob
    {
        public NativeArray<Entity> pooledEntities;
        public int startIndex;
        public int count;

        [BurstCompile]
        public void Execute()
        {
            for (int i = startIndex; i < startIndex + count; i++)
            {
                var entity = pooledEntities[i];
                entity.Set(new UIElementComponent { isActive = true, isPrefetched = true });
            }
        }
    }

    /// <summary>
    /// ECS Component for pooled entities
    /// Marks entity as part of object pool
    /// </summary>
    public struct PooledComponent : IComponent
    {
        public int poolIndex;
        public bool isAvailable;
        public System.DateTime lastUsed;
    }

    /// <summary>
    /// UI Element Component with pooling support
    /// </summary>
    public struct UIElementComponent : IComponent
    {
        public string id;
        public bool isActive;
        public bool isPrefetched;
        public int poolIndex;
    }

    /// <summary>
    /// UI Pool Prefetcher
    /// Predictive prefetching based on game state
    /// </summary>
    public class UIPoolPrefetcher : MonoBehaviour
    {
        private UIObjectPoolManager poolManager;
        private bool isEnabled = true;

        public void Initialize()
        {
            poolManager = UIObjectPoolManager.Instance;
        }

        public void EnablePrefetch()
        {
            isEnabled = true;
        }

        public void DisablePrefetch()
        {
            isEnabled = false;
        }

        public void OnGameStateChanged(GameState state)
        {
            if (!isEnabled) return;

            // Predict UI element needs based on game state
            int predictedCount = PredictUINeeds(state);
            poolManager.PrefetchUIElements(predictedCount);
        }

        private int PredictUINeeds(GameState state)
        {
            // Prediction logic based on game state
            if (state.isCombatActive)
            {
                // Combat: need health bars, enemy indicators
                return 20;
            }
            else if (state.isLootReward)
            {
                // Loot: need item cards, drop rate displays
                return 15;
            }
            else if (state.isSkillUpgrade)
            {
                // Upgrade: need skill nodes, stat displays
                return 10;
            }

            return 5; // Default
        }
    }

    /// <summary>
    /// Memory-efficient UI recycling
    /// Prevents garbage collection spikes
    /// </summary>
    public class UIRecyclingSystem : MonoBehaviour
    {
        private UIObjectPoolManager poolManager;

        public void RecycleUIElements(NativeArray<Entity> elements)
        {
            for (int i = 0; i < elements.Length; i++)
            {
                poolManager.ReturnToPool(elements[i]);
            }
        }

        public void BatchRecycle(NativeArray<Entity> elements)
        {
            var recycleJob = new UIRecyclingJob
            {
                elements = elements,
                poolManager = poolManager
            };

            var handle = recycleJob.Schedule();
            handle.Complete();
        }
    }

    /// <summary>
    /// UI Recycling Job
    /// DOTS-based batch recycling
    /// </summary>
    [BurstCompile]
    public struct UIRecyclingJob : IJob
    {
        public NativeArray<Entity> elements;
        public UIObjectPoolManager poolManager;

        [BurstCompile]
        public void Execute()
        {
            for (int i = 0; i < elements.Length; i++)
            {
                poolManager.ReturnToPool(elements[i]);
            }
        }
    }

    /// <summary>
    /// Game state for prefetch prediction
    /// </summary>
    public struct GameState
    {
        public bool isCombatActive;
        public bool isLootReward;
        public bool isSkillUpgrade;
        public int enemyCount;
        public int lootItemCount;
    }
}