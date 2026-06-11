<?php

declare(strict_types=1);

namespace App\Livewire\Admin;

use App\Models\Player;
use App\Models\PlayerSession;
use App\Models\PlayerRetention;
use App\Services\AnalyticsService;
use App\Services\SegmentationService;
use Livewire\Component;
use Livewire\Attributes\Layout;
use Livewire\Attributes\Rule;

/**
 * Game Analytics Livewire Component
 *
 * Player analytics visualization with retention, engagement, and segmentation
 *
 * Features:
 * - D1/D7/D30 retention tracking
 * - Session duration analytics
 * - Player segmentation (whales, dolphins, minnows)
 * - Engagement funnel visualization
 * - Ethical engagement metrics
 *
 * @security PHP 8.3+ type safety
 * @accessibility ARIA live regions, keyboard navigation
 */
#[Layout('layouts.admin')]
final class GameAnalytics extends Component
{
    /**
     * Analytics service for metric calculation
     */
    private AnalyticsService $analyticsService;

    /**
     * Segmentation service for player classification
     */
    private SegmentationService $segmentationService;

    /**
     * Filter parameters
     */
    #[Rule('nullable|string|in:7d,30d,90d')]
    public string $period = '30d';

    #[Rule('nullable|string')]
    public string $segment = '';

    #[Rule('nullable|string|in:na,eu,apac')]
    public string $region = '';

    /**
     * Constructor
     */
    public function __construct(
        AnalyticsService $analyticsService,
        SegmentationService $segmentationService
    ) {
        this.analyticsService = $analyticsService;
        this.segmentationService = $segmentationService;
    }

    /**
     * Mount the analytics component
     *
     * @param string|null $period Analysis period
     * @param string|null $segment Player segment filter
     * @param string|null $region Region filter
     */
    public function mount(?string $period = null, ?string $segment = null, ?string $region = null): void
    {
        this.period = $period ?? '30d';
        this.segment = $segment ?? '';
        this.region = $region ?? '';
    }

    /**
     * Render the analytics view
     *
     * @return \Illuminate\View\View
     */
    public function render(): \Illuminate\View\View
    {
        return view('livewire.admin.analytics', [
            'retentionData' => this.getRetentionData(),
            'engagementData' => this.getEngagementData(),
            'segmentationData' => this.getSegmentationData(),
            'funnelData' => this.getFunnelData(),
            'ethicalMetrics' => this.getEthicalMetrics(),
        ]);
    }

    /**
     * Get retention metrics (D1/D7/D30)
     *
     * @return array{d1: float, d7: float, d30: float, trend: array}
     */
    private function getRetentionData(): array
    {
        $retention = PlayerRetention::calculate(
            period: this.period,
            region: this.region,
            segment: this.segment
        );

        return [
            'd1' => $retention->d1,
            'd7' => $retention->d7,
            'd30' => $retention->d30,
            'trend' => $retention->trendArray(),
        ];
    }

    /**
     * Get engagement metrics
     *
     * @return array{avg_session_minutes: float, sessions_per_day: float, feature_usage: array}
     */
    private function getEngagementData(): array
    {
        return this.analyticsService->getEngagementMetrics(
            period: this.period,
            region: this.region
        );
    }

    /**
     * Get player segmentation data
     *
     * Classification:
     - Whale: > $100/month
     - Dolphin: $20-100/month
     - Minnow: < $20/month
     - Free: $0 spending
     *
     * @return array{whales: int, dolphins: int, minnows: int, free: int}
     */
    private function getSegmentationData(): array
    {
        return this.segmentationService->classifyPlayers(
            region: this.region,
            period: this.period
        );
    }

    /**
     * Get engagement funnel data
     *
     * Funnel stages:
     1. Install
     2. First session
     3. Day 1 retention
     4. Day 7 retention
     5. Day 30 retention
     6. First purchase
     7. Repeat purchase
     *
     * @return array{stages: array{name: string, count: int, rate: float}}
     */
    private function getFunnelData(): array
    {
        return [
            'stages' => [
                ['name' => 'Install', 'count' => Player::count(), 'rate' => 100.0],
                ['name' => 'First Session', 'count' => PlayerSession::count(), 'rate' => 95.0],
                ['name' => 'Day 1', 'count' => PlayerRetention::d1()->count(), 'rate' => 42.5],
                ['name' => 'Day 7', 'count' => PlayerRetention::d7()->count(), 'rate' => 25.0],
                ['name' => 'Day 30', 'count' => PlayerRetention::d30()->count(), 'rate' => 15.0],
                ['name' => 'First Purchase', 'count' => Player::paid()->count(), 'rate' => 8.0],
                ['name' => 'Repeat Purchase', 'count' => Player::repeatPurchasers()->count(), 'rate' => 4.0],
            ],
        ];
    }

    /**
     * Get ethical engagement metrics
     *
     * Includes:
     - Spending limit compliance
     - Loot table transparency status
     - Rest-state mechanics usage
     *
     * @return array{spending_limit_users: int, loot_transparency_enabled: bool, rest_state_avg: float}
     */
    private function getEthicalMetrics(): array
    {
        return [
            'spending_limit_users' => Player::withSpendingLimits()->count(),
            'loot_transparency_enabled' => \App\Models\LootTable::allTransparent(),
            'rest_state_avg' => Player::avgOfflineRegeneration(),
        ];
    }

    /**
     * Update period filter
     *
     * @param string $newPeriod New period (7d, 30d, 90d)
     */
    public function updatePeriod(string $newPeriod): void
    {
        this.period = $newPeriod;
    }

    /**
     * Update segment filter
     *
     * @param string $newSegment New segment (whale, dolphin, minnow, free)
     */
    public function updateSegment(string $newSegment): void
    {
        this.segment = $newSegment;
    }

    /**
     * Export analytics data
     *
     * @return \Symfony\Component\HttpFoundation\BinaryFileResponse
     */
    public function export(): \Symfony\Component\HttpFoundation\BinaryFileResponse
    {
        $data = [
            'retention' => this.getRetentionData(),
            'engagement' => this.getEngagementData(),
            'segmentation' => this.getSegmentationData(),
            'funnel' => this.getFunnelData(),
            'ethical' => this.getEthicalMetrics(),
        ];

        return response()->json($data, 200, [
            'Content-Disposition' => 'attachment; filename=analytics-' . this.period . '.json',
        ]);
    }

    /**
     * Refresh analytics via polling
     *
     * @return array{retention: array, engagement: array}
     */
    public function refresh(): array
    {
        return [
            'retention' => this.getRetentionData(),
            'engagement' => this.getEngagementData(),
        ];
    }
}