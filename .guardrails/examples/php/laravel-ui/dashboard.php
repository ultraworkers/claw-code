<?php

declare(strict_types=1);

namespace App\Livewire\Admin;

use App\Models\Player;
use App\Models\GameMetric;
use App\Models\ModerationAlert;
use App\Services\AnalyticsService;
use App\Services\EconomyMonitorService;
use Livewire\Component;
use Livewire\Attributes\Layout;
use Livewire\Attributes\Rule;

/**
 * Admin Dashboard Livewire Component
 *
 * Laravel 11 Livewire dashboard for game administration
 *
 * Features:
 * - Real-time player analytics via polling
 * - Moderation alerts display
 * - Economy health monitoring
 * - Ethical engagement tracking
 *
 * @see https://laravel livewire.com
 * @security PHP 8.3+ type safety, readonly properties
 * @accessibility ARIA labels, semantic HTML, keyboard navigation
 */
#[Layout('layouts.admin')]
final class Dashboard extends Component
{
    /**
     * Current authenticated admin user
     */
    public readonly ?\App\Models\AdminUser $admin;

    /**
     * Dashboard metrics (cached via Redis)
     */
    private AnalyticsService $analyticsService;

    /**
     * Economy monitoring service
     */
    private EconomyMonitorService $economyService;

    /**
     * Filter parameters
     */
    #[Rule('nullable|string|in:24h,7d,30d')]
    public string $timeframe = '24h';

    #[Rule('nullable|string|in:na,eu,apac')]
    public string $region = '';

    #[Rule('nullable|string')]
    public string $playerSegment = '';

    /**
     * Constructor
     */
    public function __construct(
        AnalyticsService $analyticsService,
        EconomyMonitorService $economyService
    ) {
        this.analyticsService = $analyticsService;
        this.economyService = $economyService;
        this.admin = auth()->user();
    }

    /**
     * Mount the dashboard component
     *
     * @param string|null $timeframe Filter timeframe
     * @param string|null $region Filter region
     */
    public function mount(?string $timeframe = null, ?string $region = null): void
    {
        this.timeframe = $timeframe ?? '24h';
        this.region = $region ?? '';
    }

    /**
     * Render the dashboard view
     *
     * @return \Illuminate\View\View
     */
    public function render(): \Illuminate\View\View
    {
        return view('livewire.admin.dashboard', [
            'metrics' => this.getMetrics(),
            'alerts' => this.getAlerts(),
            'players' => this.getPlayers(),
            'economyHealth' => this.economyService->getHealthIndex(),
            'ethicalStatus' => this.getEthicalEngagementStatus(),
        ]);
    }

    /**
     * Get dashboard metrics from analytics service
     *
     * @return array{active_players: int, retention_d1: float, economy_index: float}
     */
    private function getMetrics(): array
    {
        return this.analyticsService->getDashboardMetrics(
            timeframe: this.timeframe,
            region: this.region
        );
    }

    /**
     * Get active moderation alerts
     *
     * @return \Illuminate\Database\Eloquent\Collection<ModerationAlert>
     */
    private function getAlerts(): \Illuminate\Database\Eloquent\Collection
    {
        return ModerationAlert::active()
            ->limit(10)
            ->ordered()
            ->get();
    }

    /**
     * Get filtered player list
     *
     * @return \Illuminate\Database\Eloquent\Collection<Player>
     */
    private function getPlayers(): \Illuminate\Database\Eloquent\Collection
    {
        return Player::scopeFilters([
            'timeframe' => this.timeframe,
            'region' => this.region,
            'segment' => this.playerSegment,
        ])
        ->limit(50)
        ->ordered()
        ->get();
    }

    /**
     * Get ethical engagement status
     *
     * @return array{loot_tables_compliant: int, spending_warnings: int}
     */
    private function getEthicalEngagementStatus(): array
    {
        return [
            'loot_tables_compliant' => \App\Models\LootTable::compliant()->count(),
            'spending_warnings' => \App\Models\PlayerSpending::warningCount(),
        ];
    }

    /**
     * Update filters (called by Alpine.js)
     *
     * @param string $timeframe New timeframe
     * @param string $region New region
     */
    public function updateFilters(string $timeframe, string $region): void
    {
        this.timeframe = $timeframe;
        this.region = $region;

        this.dispatch('filtersUpdated');
    }

    /**
     * Refresh metrics via polling
     *
     * @return array{metrics: array}
     */
    public function refreshMetrics(): array
    {
        return ['metrics' => this.getMetrics()];
    }

    /**
     * Export dashboard data
     *
     * @return \Symfony\Component\HttpFoundation\BinaryFileResponse
     */
    public function export(): \Symfony\Component\HttpFoundation\BinaryFileResponse
    {
        $data = [
            'metrics' => this.getMetrics(),
            'alerts' => this.getAlerts()->toArray(),
            'economy' => this.economyService->exportData(),
        ];

        $filename = 'dashboard-export-' . date('Y-m-d') . '.json';

        return response()->json($data, 200, ['Content-Disposition' => 'attachment; filename=' . $filename]);
    }
}