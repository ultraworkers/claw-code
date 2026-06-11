// Metrics Dashboard - Grafana/Prometheus Integration
// Production-ready metrics dashboard for game backends
//
// Last Updated: 2026-03-14
// Go Version: 1.22+

package main

import (
	"fmt"
	"html/template"
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/mux"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

// MetricsDashboard implements Grafana-style metrics visualization
type MetricsDashboard struct {
	metrics   map[string]MetricData
	mu        sync.RWMutex

	// Prometheus metric types
	counterVec   *prometheus.CounterVec
	gaugeVec     *prometheus.GaugeVec
	histogramVec *prometheus.HistogramVec
	summaryVec   *prometheus.SummaryVec
}

// MetricData represents dashboard metric display
type MetricData struct {
	Name      string    `json:"name"`
	Type      string    `json:"type"` // "counter", "gauge", "histogram", "summary"
	Value     float64   `json:"value"`
	Labels    []string  `json:"labels"`
	Help      string    `json:"help"`
	LastUpdate time.Time `json:"last_update"`
}

// NewMetricsDashboard creates dashboard with Prometheus metrics
func NewMetricsDashboard() (*MetricsDashboard, error) {
	d := &MetricsDashboard{
		metrics: make(map[string]MetricData),
	}

	// Counter: Total game events
	d.counterVec = prometheus.NewCounterVec(prometheus.CounterOpts{
		Name: "game_events_total",
		Help: "Total game events processed",
	}, []string{"event_type", "region"})

	// Gauge: Current player state
	d.gaugeVec = prometheus.NewGaugeVec(prometheus.GaugeOpts{
		Name: "player_state",
		Help: "Current player state (HP, MP, XP)",
	}, []string{"player_id", "stat_type"})

	// Histogram: Action completion times
	d.histogramVec = prometheus.NewHistogramVec(prometheus.HistogramOpts{
		Name:    "action_duration_seconds",
		Help:    "Action completion time distribution",
		Buckets: prometheus.DefBuckets,
	}, []string{"action_type"})

	// Summary: Economy metrics
	d.summaryVec = prometheus.NewSummaryVec(prometheus.SummaryOpts{
		Name: "economy_metrics",
		Help: "Economy metrics summary (faucet, sink, balance)",
	}, []string{"resource"})

	// Register all metrics
	prometheus.MustRegister(d.counterVec, d.gaugeVec, d.histogramVec, d.summaryVec)

	return d, nil
}

// RecordEvent records game event counter
func (d *MetricsDashboard) RecordEvent(eventType, region string) {
	d.mu.Lock()
	defer d.mu.Unlock()

	d.counterVec.WithLabelValues(eventType, region).Inc()

	d.metrics["game_events_total"] = MetricData{
		Name:       "game_events_total",
		Type:       "counter",
		Value:      d.counterVec.WithLabelValues(eventType, region).(prometheus.Counter).Get(),
		Labels:     []string{eventType, region},
		Help:       "Total game events",
		LastUpdate: time.Now(),
	}

	log.Printf("[METRICS] Event recorded: %s in %s", eventType, region)
}

// UpdatePlayerState updates player gauge metrics
func (d *MetricsDashboard) UpdatePlayerState(playerID, statType string, value float64) {
	d.mu.Lock()
	defer d.mu.Unlock()

	d.gaugeVec.WithLabelValues(playerID, statType).Set(value)

	d.metrics["player_state"] = MetricData{
		Name:       "player_state",
		Type:       "gauge",
		Value:      value,
		Labels:     []string{playerID, statType},
		Help:       "Current player state",
		LastUpdate: time.Now(),
	}
}

// RecordActionDuration records action histogram
func (d *MetricsDashboard) RecordActionDuration(actionType string, duration time.Duration) {
	d.mu.Lock()
	defer d.mu.Unlock()

	d.histogramVec.WithLabelValues(actionType).Observe(duration.Seconds())

	d.metrics["action_duration"] = MetricData{
		Name:       "action_duration_seconds",
		Type:       "histogram",
		Value:      duration.Seconds(),
		Labels:     []string{actionType},
		Help:       "Action completion time",
		LastUpdate: time.Now(),
	}
}

// RecordEconomyMetrics records economy summary
func (d *MetricsDashboard) RecordEconomyMetrics(resource string, faucet, sink, balance float64) {
	d.mu.Lock()
	defer d.mu.Unlock()

	// Summary observations
	d.summaryVec.WithLabelValues(resource).Observe(faucet)
	d.summaryVec.WithLabelValues(resource).Observe(sink)
	d.summaryVec.WithLabelValues(resource).Observe(balance)

	d.metrics["economy_metrics"] = MetricData{
		Name:       "economy_metrics",
		Type:       "summary",
		Value:      balance,
		Labels:     []string{resource},
		Help:       "Economy metrics summary",
		LastUpdate: time.Now(),
	}
}

// GetMetricsSummary returns dashboard summary
func (d *MetricsDashboard) GetMetricsSummary() map[string]MetricData {
	d.mu.RLock()
	defer d.mu.RUnlock()

	return d.metrics
}

// DashboardHandler renders metrics dashboard UI
func (d *MetricsDashboard) DashboardHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")

	fmt.Fprint(w, `
		<!DOCTYPE html>
		<html lang="en">
		<head>
			<meta charset="UTF-8">
			<meta name="viewport" content="width=device-width, initial-scale=1.0">
			<title>Metrics Dashboard</title>
			<script src="https://unpkg.com/htmx.org@1.9.10"></script>
			<script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
			<style>
				.metric-panel { border: 1px solid #ccc; padding: 1rem; margin: 0.5rem; }
				.counter { color: blue; }
				.gauge { color: green; }
				.histogram { color: orange; }
				.summary { color: purple; }
			</style>
		</head>
		<body>
			<main hx-boost="true" role="main">
				<h1>Metrics Dashboard</h1>
				<div hx-get="/metrics/events" hx-trigger="every 2s">
					<!-- Event counter polling -->
				</div>
				<div hx-get="/metrics/players" hx-trigger="every 2s">
					<!-- Player gauge polling -->
				</div>
				<div hx-get="/metrics/actions" hx-trigger="every 5s">
					<!-- Action histogram -->
				</div>
				<div hx-get="/metrics/economy" hx-trigger="every 10s">
					<!-- Economy summary -->
				</div>
				<div>
					<a href="/metrics" target="_blank">View Prometheus Metrics</a>
				</div>
			</main>
		</body>
		</html>
	`)
}

// EventsHandler renders event counter metrics
func (d *MetricsDashboard) EventsHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")

	metrics := d.GetMetricsSummary()

	fmt.Fprint(w, `<div class="metric-panel counter" role="region" aria-label="Event counter">`)
	for name, m := range metrics {
		if m.Type == "counter" {
			fmt.Fprintf(w, `
				<div class="metric" role="listitem">
					<h2>%s</h2>
					<p class="value" aria-live="polite">%.2f</p>
					<p>Labels: %v</p>
					<p>Updated: %s</p>
				</div>
			`, m.Name, m.Value, m.Labels, m.LastUpdate)
		}
	}
	fmt.Fprint(w, `</div>`)
}

// PlayersHandler renders player gauge metrics
func (d *MetricsDashboard) PlayersHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")

	metrics := d.GetMetricsSummary()

	fmt.Fprint(w, `<div class="metric-panel gauge" role="region" aria-label="Player gauges">`)
	for name, m := range metrics {
		if m.Type == "gauge" {
			fmt.Fprintf(w, `
				<div class="metric" role="listitem">
					<h2>%s</h2>
					<p class="value" aria-live="polite">%.2f</p>
					<p>Player: %s | Stat: %s</p>
					<p>Updated: %s</p>
				</div>
			`, m.Name, m.Value, m.Labels[0], m.Labels[1], m.LastUpdate)
		}
	}
	fmt.Fprint(w, `</div>`)
}

// ActionsHandler renders action histogram
func (d *MetricsDashboard) ActionsHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")

	metrics := d.GetMetricsSummary()

	fmt.Fprint(w, `<div class="metric-panel histogram" role="region" aria-label="Action histogram">`)
	for name, m := range metrics {
		if m.Type == "histogram" {
			fmt.Fprintf(w, `
				<div class="metric" role="listitem">
					<h2>%s</h2>
					<p class="value" aria-live="polite">%.2fs</p>
					<p>Action: %s</p>
					<p>Updated: %s</p>
				</div>
			`, m.Name, m.Value, m.Labels[0], m.LastUpdate)
		}
	}
	fmt.Fprint(w, `</div>`)
}

// EconomyHandler renders economy summary
func (d *MetricsDashboard) EconomyHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")

	metrics := d.GetMetricsSummary()

	fmt.Fprint(w, `<div class="metric-panel summary" role="region" aria-label="Economy summary">`)
	for name, m := range metrics {
		if m.Type == "summary" {
			fmt.Fprintf(w, `
				<div class="metric" role="listitem">
					<h2>%s</h2>
					<p class="value" aria-live="polite">%.2f</p>
					<p>Resource: %s</p>
					<p>Updated: %s</p>
				</div>
			`, m.Name, m.Value, m.Labels[0], m.LastUpdate)
		}
	}
	fmt.Fprint(w, `</div>`)
}

// registerMetricsRoutes registers metrics dashboard routes
func (d *MetricsDashboard) registerMetricsRoutes(r *mux.Router) {
	metrics := r.PathPrefix("/metrics").Subrouter()

	metrics.HandleFunc("/dashboard", d.DashboardHandler).Methods("GET")
	metrics.HandleFunc("/events", d.EventsHandler).Methods("GET")
	metrics.HandleFunc("/players", d.PlayersHandler).Methods("GET")
	metrics.HandleFunc("/actions", d.ActionsHandler).Methods("GET")
	metrics.HandleFunc("/economy", d.EconomyHandler).Methods("GET")

	// Prometheus endpoint
	r.Handle("/metrics/prometheus", promhttp.Handler())
}

// initDemoMetrics initializes demo metrics data
func (d *MetricsDashboard) initDemoMetrics() {
	// Record demo events
	d.RecordEvent("player_login", "us-east")
	d.RecordEvent("player_action", "us-west")
	d.RecordEvent("economy_update", "eu-west")

	// Update player state
	d.UpdatePlayerState("player-1", "hp", 100)
	d.UpdatePlayerState("player-1", "mp", 50)
	d.UpdatePlayerState("player-2", "hp", 80)

	// Record action durations
	d.RecordActionDuration("attack", 100*time.Millisecond)
	d.RecordActionDuration("heal", 200*time.Millisecond)
	d.RecordActionDuration("rest", 50*time.Millisecond)

	// Record economy metrics
	d.RecordEconomyMetrics("gold", 1000, 300, 700)
	d.RecordEconomyMetrics("energy", 500, 200, 300)
}