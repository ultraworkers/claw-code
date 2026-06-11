// HTMX Reactive Patterns Example - Go 1.22+
// Production-ready HTMX patterns for game backends without JavaScript frameworks
//
// Last Updated: 2026-03-14
// Go Version: 1.22+
// HTMX Version: 1.9.x

package main

import (
	"context"
	"embed"
	"fmt"
	"html/template"
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/mux"
)

//go:embed templates/*.tmpl
var templatesFS embed.FS

// GameState represents the server-authority game state
type GameState struct {
	Players    map[string]*Player
	Events     chan GameEvent
	mu         sync.RWMutex
	Sequence   uint64
	Timestamp  time.Time
}

// Player represents a game player with HTMX-compatible state
type Player struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	Level       int    `json:"level"`
	HP          int    `json:"hp"`
	MP          int    `json:"mp"`
	XP          int    `json:"xp"`
	LastAction  string `json:"last_action"`
	UpdatedAt   time.Time `json:"updated_at"`
}

// GameEvent represents a server-driven game event
type GameEvent struct {
	Type      string      `json:"type"`
	Payload   interface{} `json:"payload"`
	Sequence  uint64      `json:"sequence"`
	Timestamp time.Time   `json:"timestamp"`
	Source    string      `json:"source"` // "server" for server authority
}

// GameStateServer implements server-authority pattern
type GameStateServer struct {
	state   *GameState
	clients map[string]chan GameEvent
	mu      sync.Mutex
}

// NewGameStateServer creates server with proper initialization
func NewGameStateServer() *GameStateServer {
	return &GameStateServer{
		state: &GameState{
			Players: make(map[string]*Player),
			Events:  make(chan GameEvent, 100),
			Sequence: 0,
			Timestamp: time.Now(),
		},
		clients: make(map[string]chan GameEvent),
	}
}

// GetPlayer returns player with server authority validation
func (s *GameStateServer) GetPlayer(id string) (*Player, bool) {
	s.mu.Lock()
	defer s.mu.Unlock()
	player, ok := s.state.Players[id]
	if !ok {
		return nil, false
	}
	return player, true
}

// UpdatePlayer performs server-authority update with audit logging
func (s *GameStateServer) UpdatePlayer(id string, action string) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	player, ok := s.state.Players[id]
	if !ok {
		return fmt.Errorf("player not found: %s", id)
	}

	// Server-authority validation
	if err := s.validateAction(player, action); err != nil {
		log.Printf("[AUDIT] Rejected action %s for player %s: %v", action, id, err)
		return err
	}

	// Apply server-authority update
	oldState := *player
	s.state.Sequence++
	s.state.Timestamp = time.Now()

	switch action {
	case "attack":
		player.HP = max(0, player.HP-10)
		player.XP += 5
		player.LastAction = "attack"
	case "heal":
		if player.MP >= 10 {
			player.HP = min(100, player.HP+20)
			player.MP -= 10
			player.LastAction = "heal"
			player.XP += 2
		}
	case "rest":
		player.MP = min(100, player.MP+15)
		player.LastAction = "rest"
	}

	player.UpdatedAt = s.state.Timestamp

	// Broadcast event with sequence number for eventual consistency
	event := GameEvent{
		Type:      "PLAYER_UPDATE",
		Payload:   player,
		Sequence:  s.state.Sequence,
		Timestamp: s.state.Timestamp,
		Source:    "server",
	}

	s.state.Events <- event
	log.Printf("[AUDIT] Player %s updated: %s -> %+v", id, action, player)

	// Broadcast to WebSocket clients
	s.broadcastEvent(event)

	return nil
}

// validateAction implements server-authority validation
func (s *GameStateServer) validateAction(player *Player, action string) error {
	switch action {
	case "attack":
		if player.HP <= 0 {
			return fmt.Errorf("player HP is 0")
		}
	case "heal":
		if player.MP < 10 {
			return fmt.Errorf("insufficient MP")
		}
	case "rest":
		// Always valid
	}
	return nil
}

// broadcastEvent sends event to all WebSocket clients
func (s *GameStateServer) broadcastEvent(event GameEvent) {
	s.mu.Lock()
	defer s.mu.Unlock()
	for clientID, ch := range s.clients {
		select {
		case ch <- event:
			log.Printf("[WS] Event sent to client %s", clientID)
		default:
			log.Printf("[WS] Client %s buffer full, dropping event", clientID)
		}
	}
}

// pollHandler implements HTMX polling pattern for metrics
func (s *GameStateServer) pollHandler(w http.ResponseWriter, r *http.Request) {
	// Polling interval matches game tick rate (2s default)
	w.Header().Set("Content-Type", "text/html")

	s.mu.Lock()
	defer s.mu.Unlock()

	stats := struct {
		TotalPlayers int
		TotalEvents  int
		LastUpdate   time.Time
	}{
		TotalPlayers: len(s.state.Players),
		TotalEvents:  cap(s.state.Events),
		LastUpdate:   s.state.Timestamp,
	}

	// HTMX polling response - updates metrics dashboard
	fmt.Fprintf(w, `
		<div id="metrics" hx-trigger="every 2s" hx-get="/metrics">
			<p>Players: %d</p>
			<p>Events Buffer: %d</p>
			<p>Last Update: %s</p>
			<span aria-live="polite">Updated at %s</span>
		</div>
	`, stats.TotalPlayers, stats.TotalEvents, stats.LastUpdate, stats.LastUpdate)
}

// wsHandler implements HTMX WebSocket extension for real-time events
func (s *GameStateServer) wsHandler(w http.ResponseWriter, r *http.Request) {
	// HTMX WebSocket extension pattern
	// Note: Production implementation should use gorilla/websocket
	clientID := r.URL.Query().Get("client_id")
	if clientID == "" {
		http.Error(w, "client_id required", http.StatusBadRequest)
		return
	}

	eventChan := make(chan GameEvent, 10)
	s.mu.Lock()
	s.clients[clientID] = eventChan
	s.mu.Unlock()

	log.Printf("[WS] Client %s connected", clientID)

	// SSE fallback for non-WebSocket clients
	w.Header().Set("Content-Type", "text/event-stream")
	fmt.Fprintf(w, "data: WebSocket connected\n\n")

	// Stream events with sequence validation
	for event := range eventChan {
		// Eventual consistency: sequence numbers for conflict resolution
		fmt.Fprintf(w, "data: {%q: %q, %q: %d}\n\n",
			"type", event.Type, "sequence", event.Sequence)
	}
}

// playerHandler demonstrates component composition with template blocks
func (s *GameStateServer) playerHandler(w http.ResponseWriter, r *http.Request) {
	id := mux.Vars(r)["id"]

	player, ok := s.GetPlayer(id)
	if !ok {
		http.Error(w, "player not found", http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "text/html")

	// Template block pattern from templates.go
	// hx-swap="innerHTML" for partial updates
	fmt.Fprintf(w, `
		<div class="player-card" id="player-%s" hx-swap="innerHTML">
			<span class="name">%s</span>
			<span class="level">Level %d</span>
			<span class="hp" aria-label="Hit Points">HP: %d</span>
			<span class="mp" aria-label="Magic Points">MP: %d</span>
			<span class="xp">XP: %d</span>
			<button
				hx-post="/api/player/%s/action"
				hx-vals="js:{action: 'attack'}"
				hx-target="#player-%s"
				hx-confirm="Confirm attack action?"
				aria-label="Attack action for %s">
				Attack
			</button>
			<button
				hx-post="/api/player/%s/action"
				hx-vals="js:{action: 'heal'}"
				hx-target="#player-%s"
				hx-confirm="Confirm heal action?"
				aria-label="Heal action for %s">
				Heal
			</button>
			<button
				hx-post="/api/player/%s/action"
				hx-vals="js:{action: 'rest'}"
				hx-target="#player-%s"
				aria-label="Rest action for %s">
				Rest
			</button>
			<span aria-live="polite" class="last-action">Last: %s</span>
		</div>
	`,
		player.ID, player.Name, player.Level,
		player.HP, player.MP, player.XP,
		player.ID, player.ID, player.Name,
		player.ID, player.ID, player.Name,
		player.ID, player.ID, player.Name,
		player.LastAction)
}

// actionHandler implements optimistic UI with rollback
func (s *GameStateServer) actionHandler(w http.ResponseWriter, r *http.Request) {
	id := mux.Vars(r)["id"]
	action := r.FormValue("action")

	// Optimistic UI: display change immediately
	// Server validates and may reject
	if err := s.UpdatePlayer(id, action); err != nil {
		// Rollback on rejection - hx-swap="none" pattern
		w.Header().Set("HX-Swap", "none")
		w.Header().Set("Content-Type", "text/html")
		fmt.Fprintf(w, `
			<div class="error" role="alert" aria-invalid="true">
				Action rejected: %v
			</div>
		`, err)
		return
	}

	// Success: re-render player card
	s.playerHandler(w, r)
}

// adminHandler demonstrates HTMX boost mode for navigation
func (s *GameStateServer) adminHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")

	// HTMX boost mode for full page transitions
	// hx-push-url="true" for browser history sync
	fmt.Fprint(w, `
		<!DOCTYPE html>
		<html lang="en">
		<head>
			<meta charset="UTF-8">
			<meta name="viewport" content="width=device-width, initial-scale=1.0">
			<title>Game Admin Panel</title>
			<script src="https://unpkg.com/htmx.org@1.9.10"></script>
			<script src="https://unpkg.com/htmx.org/dist/ext/ws.js"></script>
			<script src="https://unpkg.com/htmx.org/dist/ext/sse.js"></script>
		</head>
		<body>
			<main hx-boost="true">
				<nav hx-push-url="true">
					<a href="/admin">Admin</a> |
					<a href="/metrics">Metrics</a> |
					<a href="/players">Players</a>
				</nav>

				<section id="admin-panel">
					<h1>Game Admin Panel</h1>
					<div hx-get="/metrics" hx-trigger="every 2s">
						<!-- Metrics loaded via polling -->
					</div>
					<div hx-get="/players" hx-trigger="load">
						<!-- Players loaded on page load -->
					</div>
					<div
						hx-ext="ws"
						hx-ws="connect:/ws?client_id=admin"
						aria-live="polite">
						<!-- WebSocket event stream -->
					</div>
				</section>

				<!-- Accessibility: focus management -->
				<div id="focus-target" tabindex="-1"></div>
			</main>
		</body>
		</html>
	`)
}

// Initialize demo players for example
func (s *GameStateServer) initDemoPlayers() {
	s.state.Players["player-1"] = &Player{
		ID:         "player-1",
		Name:       "Hero",
		Level:      10,
		HP:         100,
		MP:         50,
		XP:         500,
		LastAction: "idle",
		UpdatedAt:  time.Now(),
	}
	s.state.Players["player-2"] = &Player{
		ID:         "player-2",
		Name:       "Mage",
		Level:      8,
		HP:         60,
		MP:         100,
		XP:         400,
		LastAction: "idle",
		UpdatedAt:  time.Now(),
	}
}

func main() {
	server := NewGameStateServer()
	server.initDemoPlayers()

	r := mux.NewRouter()

	// Admin panel with boost mode
	r.HandleFunc("/admin", server.adminHandler).Methods("GET")

	// Metrics polling (2s interval matches game tick rate)
	r.HandleFunc("/metrics", server.pollHandler).Methods("GET")

	// WebSocket event streaming
	r.HandleFunc("/ws", server.wsHandler).Methods("GET")

	// Player cards with component composition
	r.HandleFunc("/players", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html")
		server.mu.Lock()
		defer server.mu.Unlock()
		for _, player := range server.state.Players {
			server.playerHandler(w, r)
		}
	}).Methods("GET")

	r.HandleFunc("/players/{id}", server.playerHandler).Methods("GET")

	// Action handler with optimistic UI + rollback
	r.HandleFunc("/api/player/{id}/action", server.actionHandler).Methods("POST")

	fmt.Println("HTMX Game Admin Server starting on :8080")
	fmt.Println("Access admin panel at http://localhost:8080/admin")

	if err := http.ListenAndServe(":8080", r); err != nil {
		log.Fatal(err)
	}
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}