<?php

declare(strict_types=1);

namespace App\Livewire\Admin;

use App\Models\EconomyTransaction;
use App\Models\EconomyBalance;
use App\Services\EconomyMonitorService;
use App\Services\InflationTrackerService;
use Livewire\Component;
use Livewire\Attributes\Layout;
use Livewire\Attributes\Rule;

/**
 * Economy Dashboard Livewire Component
 *
 * Economy faucet/sink monitoring with transparency
 *
 * Features:
 * - Faucet rate tracking (resources entering economy)
 * - Sink rate tracking (resources removed)
 * - Balance ratio monitoring (target: 1.0-1.2)
 * - Inflation index tracking
 * - Transparent economy health display
 *
 * @security PHP 8.3+ type safety, readonly classes
 * @accessibility ARIA live regions, real-time updates
 */
#[Layout('layouts.admin')]
final class EconomyDashboard extends Component
{
    /**
     * Economy monitoring service
     */
    private EconomyMonitorService $economyService;

    /**
     * Inflation tracking service
     */
    private InflationTrackerService $inflationService;

    /**
     * Filter parameters
     */
    #[Rule('nullable|string|in:24h,7d,30d')]
    public string $timeframe = '7d';

    #[Rule('nullable|string')]
    public string $resourceType = '';

    /**
     * Economy health threshold
     */
    private const HEALTH_THRESHOLD_MIN = 1.0;
    private const HEALTH_THRESHOLD_MAX = 1.2;
    private const INFLATION_ALERT_PERCENT = 5.0;

    /**
     * Constructor
     */
    public function __construct(
        EconomyMonitorService $economyService,
        InflationTrackerService $inflationService
    ) {
        this.economyService = $economyService;
        this.inflationService = $inflationService;
    }

    /**
     * Mount the economy dashboard
     *
     * @param string|null $timeframe Monitoring timeframe
     * @param string|null $resourceType Resource type filter
     */
    public function mount(?string $timeframe = null, ?string $resourceType = null): void
    {
        this.timeframe = $timeframe ?? '7d';
        this.resourceType = $resourceType ?? '';
    }

    /**
     * Render the economy dashboard view
     *
     * @return \Illuminate\View\View
     */
    public function render(): \Illuminate\View\View
    {
        return view('livewire.admin.economy', [
            'faucetData' => this.getFaucetData(),
            'sinkData' => this.getSinkData(),
            'balanceRatio' => this.getBalanceRatio(),
            'inflationIndex' => this.getInflationIndex(),
            'healthStatus' => this.getHealthStatus(),
            'topFaucets' => this.getTopFaucets(),
            'topSinks' => this.getTopSinks(),
            'transparencyMetrics' => this.getTransparencyMetrics(),
        ]);
    }

    /**
     * Get faucet data (resources entering economy)
     *
     * Faucet sources:
     - Monster drops
     - Quest rewards
     - Daily bonuses
     - Achievement rewards
     - Rest-state regeneration
     *
     * @return array{total: int, by_source: array, rate_per_hour: float}
     */
    private function getFaucetData(): array
    {
        return this.economyService->getFaucetMetrics(
            timeframe: this.timeframe,
            resourceType: this.resourceType
        );
    }

    /**
     * Get sink data (resources removed from economy)
     *
     * Sink sources:
     - Item purchases
     - Upgrade costs
     - Repair costs
     - Tax/fees
     - Decay/loss
     *
     * @return array{total: int, by_source: array, rate_per_hour: float}
     */
    private function getSinkData(): array
    {
        return this.economyService->getSinkMetrics(
            timeframe: this.timeframe,
            resourceType: this.resourceType
        );
    }

    /**
     * Get faucet/sink balance ratio
     *
     * Target ratio: 1.0-1.2 (slight faucet surplus for growth)
     * Alert if ratio < 0.8 (deflation) or > 1.5 (inflation risk)
     *
     * @return float Ratio (faucet total / sink total)
     */
    private function getBalanceRatio(): float
    {
        return this.economyService->calculateBalanceRatio(
            timeframe: this.timeframe
        );
    }

    /**
     * Get inflation index
     *
     * Tracks price increases over time
     * Alert threshold: >5% per week
     *
     * @return array{weekly_percent: float, monthly_percent: float, alert: bool}
     */
    private function getInflationIndex(): array
    {
        return this.inflationService->calculateIndex(
            timeframe: this.timeframe
        );
    }

    /**
     * Get economy health status
     *
     * Health classification:
     - Excellent: Ratio 1.0-1.2, inflation <2%
     - Good: Ratio 0.8-1.5, inflation <5%
     - Warning: Ratio 0.6-0.8 or 1.5-2.0, inflation 5-10%
     - Critical: Ratio <0.6 or >2.0, inflation >10%
     *
     * @return string Health status (excellent, good, warning, critical)
     */
    private function getHealthStatus(): string
    {
        $ratio = this.getBalanceRatio();
        $inflation = this.getInflationIndex()['weekly_percent'];

        if ($ratio >= 1.0 && $ratio <= 1.2 && $inflation < 2.0) {
            return 'excellent';
        }

        if ($ratio >= 0.8 && $ratio <= 1.5 && $inflation < 5.0) {
            return 'good';
        }

        if (($ratio >= 0.6 && $ratio < 0.8) || ($ratio > 1.5 && $ratio <= 2.0) || ($inflation >= 5.0 && $inflation < 10.0)) {
            return 'warning';
        }

        return 'critical';
    }

    /**
     * Get top faucet sources
     *
     * @return array{name: string, amount: int, percent: float}[]
     */
    private function getTopFaucets(): array
    {
        return EconomyTransaction::faucet()
            ->select('source', 'SUM(amount) as total')
            ->groupBy('source')
            ->orderByDesc('total')
            ->limit(5)
            ->get()
            ->map(function ($row) {
                return [
                    'name' => $row->source,
                    'amount' => (int) $row->total,
                    'percent' => round($row->total / EconomyTransaction::faucet()->sum('amount') * 100, 1),
                ];
            })
            ->toArray();
    }

    /**
     * Get top sink sources
     *
     * @return array{name: string, amount: int, percent: float}[]
     */
    private function getTopSinks(): array
    {
        return EconomyTransaction::sink()
            ->select('source', 'SUM(amount) as total')
            ->groupBy('source')
            ->orderByDesc('total')
            ->limit(5)
            ->get()
            ->map(function ($row) {
                return [
                    'name' => $row->source,
                    'amount' => (int) $row->total,
                    'percent' => round($row->total / EconomyTransaction::sink()->sum('amount') * 100, 1),
                ];
            })
            ->toArray();
    }

    /**
     * Get transparency metrics
     *
     * Ethical engagement: economy transparency for players
     *
     * @return array{public_dashboard_enabled: bool, inflation_disclosed: bool}
     */
    private function getTransparencyMetrics(): array
    {
        return [
            'public_dashboard_enabled' => config('economy.public_dashboard', false),
            'inflation_disclosed' => config('economy.disclose_inflation', true),
        ];
    }

    /**
     * Update timeframe filter
     *
     * @param string $newTimeframe New timeframe (24h, 7d, 30d)
     */
    public function updateTimeframe(string $newTimeframe): void
    {
        this.timeframe = $newTimeframe;
    }

    /**
     * Update resource type filter
     *
     * @param string $newResourceType Resource type (gold, gems, materials)
     */
    public function updateResourceType(string $newResourceType): void
    {
        this.resourceType = $newResourceType;
    }

    /**
     * Export economy data
     *
     * @return \Symfony\Component\HttpFoundation\BinaryFileResponse
     */
    public function export(): \Symfony\Component\HttpFoundation\BinaryFileResponse
    {
        $data = [
            'faucet' => this.getFaucetData(),
            'sink' => this.getSinkData(),
            'balance_ratio' => this.getBalanceRatio(),
            'inflation' => this.getInflationIndex(),
            'health' => this.getHealthStatus(),
            'top_faucets' => this.getTopFaucets(),
            'top_sinks' => this.getTopSinks(),
        ];

        return response()->json($data, 200, [
            'Content-Disposition' => 'attachment; filename=economy-' . this.timeframe . '.json',
        ]);
    }

    /**
     * Refresh economy metrics via polling
     *
     * @return array{balance_ratio: float, health_status: string}
     */
    public function refresh(): array
    {
        return [
            'balance_ratio' => this.getBalanceRatio(),
            'health_status' => this.getHealthStatus(),
        ];
    }

    /**
     * Trigger inflation alert
     *
     * Called when inflation exceeds threshold
     *
     * @return void
     */
    public function triggerInflationAlert(): void
    {
        $inflation = this.getInflationIndex();

        if ($inflation['weekly_percent'] >= this.INFLATION_ALERT_PERCENT) {
            \App\Models\EconomyAlert::create([
                'type' => 'inflation',
                'severity' => 'warning',
                'data' => $inflation,
                'timestamp' => now(),
            ]);

            this.dispatch('alertCreated', type: 'inflation');
        }
    }

    /**
     * Recalculate economy balance
     *
     * Force recalculation of all economy metrics
     *
     * @return array{recalculated: bool}
     */
    public function recalculate(): array
    {
        this.economyService->recalculateMetrics();

        return ['recalculated' => true];
    }
}