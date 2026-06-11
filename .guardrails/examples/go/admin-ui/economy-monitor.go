// Economy Monitor - Faucet/Sink Tracking for Game Backends
// Production-ready resource economy monitoring with audit logging
//
// Last Updated: 2026-03-14
// Go Version: 1.22+

package main

import (
	"encoding/json"
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

// EconomyResource represents a tracked game resource
type EconomyResource struct {
	Name        string    `json:"name"`
	TotalFaucet float64   `json:"total_faucet"` // Generated
	TotalSink   float64   `json:"total_sink"`   // Consumed
	Balance     float64   `json:"balance"`      // Faucet - Sink
	Cap         float64   `json:"cap"`          // Max allowed
	Rate        float64   `json:"rate"`         // Per-minute rate
	LastUpdate  time.Time `json:"last_update"`
}

// EconomyTransaction represents a faucet/sink event
type EconomyTransaction struct {
	ID          string      `json:"id"`
	Resource    string      `json:"resource"`
	Type        string      `json:"type"` // "faucet" or "sink"
	Amount      float64     `json:"amount"`
	PlayerID    string      `json:"player_id"`
	Sequence    uint64      `json:"sequence"`
	Timestamp   time.Time   `json:"timestamp"`
	Source      string      `json:"source"` // "server" authority
	AuditLog    string      `json:"audit_log"`
}

// EconomyMonitor implements faucet/sink tracking with validation
type EconomyMonitor struct {
	resources   map[string]*EconomyResource
	transactions []EconomyTransaction
	sequence    uint64
	mu          sync.RWMutex

	// Prometheus metrics
	faucetCounter prometheus.Counter
	sinkCounter   prometheus.Counter
	balanceGauge  *prometheus.GaugeVec
	rateHistogram prometheus.Histogram
}

// NewEconomyMonitor creates monitor with Prometheus metrics
func NewEconomyMonitor() (*EconomyMonitor, error) {
	m := &EconomyMonitor{
		resources:    make(map[string]*EconomyResource),
		transactions: make([]EconomyTransaction, 0),
		sequence:     0,
	}

	// Register Prometheus metrics
	m.faucetCounter = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "economy_faucet_total",
		Help: "Total resource faucet amount",
	})

	m.sinkCounter = prometheus.NewCounter(prometheus.CounterOpts{
		Name: "economy_sink_total",
		Help: "Total resource sink amount",
	})

	m.balanceGauge = prometheus.NewGaugeVec(prometheus.GaugeOpts{
		Name: "economy_balance",
		Help: "Current resource balance",
	}, []string{"resource"})

	m.rateHistogram = prometheus.NewHistogram(prometheus.HistogramOpts{
		Name:    "economy_transaction_rate",
		Help:    "Transaction rate distribution",
		Buckets: prometheus.ExponentialBuckets(1, 2, 10),
	})

	// Register all metrics
	prometheus.MustRegister(m.faucetCounter, m.sinkCounter, m.balanceGauge, m.rateHistogram)

	return m, nil
}

// AddResource initializes resource tracking with caps
func (m *EconomyMonitor) AddResource(name string, cap float64) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	if _, ok := m.resources[name]; ok {
		return fmt.Errorf("resource already exists: %s", name)
	}

	m.resources[name] = &EconomyResource{
		Name:       name,
		TotalFaucet: 0,
		TotalSink:   0,
		Balance:     0,
		Cap:         cap,
		Rate:        0,
		LastUpdate:  time.Now(),
	}

	m.balanceGauge.WithLabelValues(name).Set(0)
	log.Printf("[ECONOMY] Resource initialized: %s (cap: %.2f)", name, cap)

	return nil
}

// AddFaucet records resource generation with validation
func (m *EconomyMonitor) AddFaucet(resource string, amount float64, playerID string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	res, ok := m.resources[resource]
	if !ok {
		return fmt.Errorf("resource not found: %s", resource)
	}

	// Faucet cap validation
	if res.TotalFaucet+amount > res.Cap {
		log.Printf("[ECONOMY] Faucet cap exceeded: %s (%.2f > %.2f)",
			resource, res.TotalFaucet+amount, res.Cap)
		return fmt.Errorf("faucet cap exceeded")
	}

	m.sequence++
	txn := EconomyTransaction{
		ID:          fmt.Sprintf("txn-%d", m.sequence),
		Resource:    resource,
		Type:        "faucet",
		Amount:      amount,
		PlayerID:    playerID,
		Sequence:    m.sequence,
		Timestamp:   time.Now(),
		Source:      "server",
		AuditLog:    fmt.Sprintf("Faucet: %s +%.2f (player: %s)", resource, amount, playerID),
	}

	res.TotalFaucet += amount
	res.Balance = res.TotalFaucet - res.TotalSink
	res.LastUpdate = txn.Timestamp
	res.Rate = amount / time.Minute.Seconds()

	m.transactions = append(m.transactions, txn)
	m.faucetCounter.Add(amount)
	m.balanceGauge.WithLabelValues(resource).Set(res.Balance)

	log.Printf("[AUDIT] %s", txn.AuditLog)

	return nil
}

// AddSink records resource consumption with validation
func (m *EconomyMonitor) AddSink(resource string, amount float64, playerID string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	res, ok := m.resources[resource]
	if !ok {
		return fmt.Errorf("resource not found: %s", resource)
	}

	// Sink validation: balance must cover amount
	if res.Balance < amount {
		log.Printf("[ECONOMY] Sink exceeds balance: %s (%.2f < %.2f)",
			resource, res.Balance, amount)
		return fmt.Errorf("insufficient balance")
	}

	// Negative sink detection (economy leak)
	if amount < 0 {
		log.Printf("[ECONOMY] Negative sink detected (leak): %s (%.2f)",
			resource, amount)
		return fmt.Errorf("negative sink rejected")
	}

	m.sequence++
	txn := EconomyTransaction{
		ID:          fmt.Sprintf("txn-%d", m.sequence),
		Resource:    resource,
		Type:        "sink",
		Amount:      amount,
		PlayerID:    playerID,
		Sequence:    m.sequence,
		Timestamp:   time.Now(),
		Source:      "server",
		AuditLog:    fmt.Sprintf("Sink: %s -%.2f (player: %s)", resource, amount, playerID),
	}

	res.TotalSink += amount
	res.Balance = res.TotalFaucet - res.TotalSink
	res.LastUpdate = txn.Timestamp

	m.transactions = append(m.transactions, txn)
	m.sinkCounter.Add(amount)
	m.balanceGauge.WithLabelValues(resource).Set(res.Balance)

	log.Printf("[AUDIT] %s", txn.AuditLog)

	return nil
}

// GetBalance returns current resource balance
func (m *EconomyMonitor) GetBalance(resource string) (float64, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	res, ok := m.resources[resource]
	if !ok {
		return 0, fmt.Errorf("resource not found: %s", resource)
	}

	return res.Balance, nil
}

// DetectInflation checks for economy imbalance
func (m *EconomyMonitor) DetectInflation() map[string]bool {
	m.mu.RLock()
	defer m.mu.RUnlock()

	inflation := make(map[string]bool)

	for name, res := range m.resources {
		// Inflation: faucet >> sink (unbalanced economy)
		if res.TotalFaucet > res.TotalSink*2 {
			inflation[name] = true
			log.Printf("[ECONOMY] Inflation detected: %s (faucet: %.2f, sink: %.2f)",
				name, res.TotalFaucet, res.TotalSink)
		}
	}

	return inflation
}

// GetTransactions returns recent transactions for UI
func (m *EconomyMonitor) GetTransactions(limit int) []EconomyTransaction {
	m.mu.RLock()
	defer m.mu.RUnlock()

	if len(m.transactions) <= limit {
		return m.transactions
	}

	return m.transactions[len(m.transactions)-limit:]
}

// EconomyHandler renders economy monitoring UI
func (m *EconomyMonitor) EconomyHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")

	m.mu.RLock()
	defer m.mu.RUnlock()

	inflation := m.DetectInflation()

	fmt.Fprint(w, `
		<!DOCTYPE html>
		<html lang="en">
		<head>
			<meta charset="UTF-8">
			<meta name="viewport" content="width=device-width, initial-scale=1.0">
			<title>Economy Monitor</title>
			<script src="https://unpkg.com/htmx.org@1.9.10"></script>
			<style>
				.resource-card { border: 1px solid #ccc; padding: 1rem; margin: 0.5rem; }
				.inflation { background-color: #ffcccc; }
				.balance { font-weight: bold; }
			</style>
		</head>
		<body>
			<main hx-boost="true" role="main">
				<h1>Economy Monitor</h1>
				<div hx-get="/economy/resources" hx-trigger="every 2s">
					<!-- Resource cards loaded via polling -->
				</div>
				<div hx-get="/economy/inflation" hx-trigger="every 30s">
					<!-- Inflation alerts -->
				</div>
				<div hx-get="/economy/transactions" hx-trigger="load">
					<!-- Transaction history -->
				</div>
			</main>
		</body>
		</html>
	`)
}

// ResourcesHandler renders resource cards
func (m *EconomyMonitor) ResourcesHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")

	m.mu.RLock()
	defer m.mu.RUnlock()

	inflation := m.DetectInflation()

	for name, res := range m.resources {
		inflationClass := ""
		if inflation[name] {
			inflationClass = "inflation"
		}

		fmt.Fprintf(w, `
			<div class="resource-card %s" role="region" aria-label="%s economy">
				<h2>%s</h2>
				<p class="balance" aria-live="polite">Balance: %.2f / %.2f</p>
				<p>Faucet: %.2f | Sink: %.2f</p>
				<p>Rate: %.2f/min</p>
				<p>Last Update: %s</p>
				%s
			</div>
		`,
			inflationClass, name, name,
			res.Balance, res.Cap,
			res.TotalFaucet, res.TotalSink,
			res.Rate,
			res.LastUpdate,
			inflation[name] ? `<span role="alert">INFLATION DETECTED</span>` : "")
	}
}

// InflationHandler renders inflation alerts
func (m *EconomyMonitor) InflationHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")

	inflation := m.DetectInflation()

	if len(inflation) == 0 {
		fmt.Fprint(w, `<span>Economy balanced</span>`)
		return
	}

	fmt.Fprint(w, `<div role="alert" aria-live="assertive">`)
	for resource, detected := range inflation {
		if detected {
			fmt.Fprintf(w, `<p>Inflation: %s</p>`, resource)
		}
	}
	fmt.Fprint(w, `</div>`)
}

// TransactionsHandler renders transaction history
func (m *EconomyMonitor) TransactionsHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")

	txns := m.GetTransactions(20)

	fmt.Fprint(w, `<div role="list" aria-label="Economy transactions">`)
	for _, txn := range txns {
		fmt.Fprintf(w, `
			<div class="transaction" role="listitem">
				<span>%s</span>
				<span>%s: %.2f</span>
				<span>Player: %s</span>
				<span>Seq: %d</span>
			</div>
		`, txn.Timestamp, txn.Type, txn.Amount, txn.PlayerID, txn.Sequence)
	}
	fmt.Fprint(w, `</div>`)
}

// registerEconomyRoutes registers economy monitoring routes
func (m *EconomyMonitor) registerEconomyRoutes(r *mux.Router) {
	economy := r.PathPrefix("/economy").Subrouter()

	economy.HandleFunc("/resources", m.ResourcesHandler).Methods("GET")
	economy.HandleFunc("/inflation", m.InflationHandler).Methods("GET")
	economy.HandleFunc("/transactions", m.TransactionsHandler).Methods("GET")
	economy.HandleFunc("/", m.EconomyHandler).Methods("GET")

	// Prometheus metrics endpoint
	r.Handle("/metrics", promhttp.Handler())
}

// initDemoEconomy initializes demo economy data
func (m *EconomyMonitor) initDemoEconomy() {
	// Initialize resources with caps
	m.AddResource("gold", 1000000)
	m.AddResource("energy", 500000)
	m.AddResource("gems", 100000)

	// Simulate faucet transactions
	m.AddFaucet("gold", 1000, "player-1")
	m.AddFaucet("gold", 500, "player-2")
	m.AddFaucet("energy", 200, "player-1")
	m.AddFaucet("gems", 50, "player-3")

	// Simulate sink transactions
	m.AddSink("gold", 300, "player-1")
	m.AddSink("gold", 150, "player-2")
	m.AddSink("energy", 100, "player-1")
	m.AddSink("gems", 10, "player-3")
}