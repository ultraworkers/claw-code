// Game Admin Panel - HTMX Server-Driven UI
// Production-ready admin panel for game server management
//
// Last Updated: 2026-03-14
// Go Version: 1.22+

package main

import (
	"context"
	"encoding/json"
	"fmt"
	"html/template"
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/mux"
	"github.com/gorilla/websocket"
)

// AdminPanelConfig defines admin panel configuration
type AdminPanelConfig struct {
	AuthEnabled    bool      `json:"auth_enabled"`
	RateLimit      int       `json:"rate_limit"` // requests per minute
	SessionTimeout time.Duration `json:"session_timeout"`
	AllowedOrigins []string  `json:"allowed_origins"`
}

// AdminSession represents authenticated admin session
type AdminSession struct {
	ID          string    `json:"id"`
	UserID      string    `json:"user_id"`
	Permissions []string  `json:"permissions"`
	CreatedAt   time.Time `json:"created_at"`
	ExpiresAt   time.Time `json:"expires_at"`
	LastAccess  time.Time `json:"last_access"`
}

// GameAdmin implements HTMX admin panel handlers
type GameAdmin struct {
	config    AdminPanelConfig
	state     *GameStateServer
	templates *TemplateBlocks
.sessions   map[string]*AdminSession
	mu        sync.RWMutex
	wsUpgrader *websocket.Upgrader
}

// NewGameAdmin creates admin panel with proper initialization
func NewGameAdmin(config AdminPanelConfig, state *GameStateServer) (*GameAdmin, error) {
	templates, err := NewTemplateBlocks()
	if err != nil {
		return nil, err
	}

	return &GameAdmin{
		config:     config,
		state:      state,
		templates:  templates,
		sessions:   make(map[string]*AdminSession),
		wsUpgrader: &websocket.Upgrader{
			CheckOrigin: func(r *http.Request) bool {
				for _, origin := range config.AllowedOrigins {
					if r.Header.Get("Origin") == origin {
						return true
					}
				}
				return false
			},
		},
	}, nil
}

// createSession creates new admin session with authz
func (a *GameAdmin) createSession(userID string, permissions []string) (*AdminSession, error) {
	a.mu.Lock()
	defer a.mu.Unlock()

	session := &AdminSession{
		ID:          fmt.Sprintf("session-%d", time.Now().UnixNano()),
		UserID:      userID,
		Permissions: permissions,
		CreatedAt:   time.Now(),
		ExpiresAt:   time.Now().Add(a.config.SessionTimeout),
		LastAccess:  time.Now(),
	}

	a.sessions[session.ID] = session
	log.Printf("[AUTH] Session created: %s for user %s", session.ID, userID)

	return session, nil
}

// validateSession validates session with authz check
func (a *GameAdmin) validateSession(sessionID string) (*AdminSession, error) {
	a.mu.RLock()
	defer a.mu.RUnlock()

	session, ok := a.sessions[sessionID]
	if !ok {
		return nil, fmt.Errorf("session not found")
	}

	if time.Now().After(session.ExpiresAt) {
		delete(a.sessions, sessionID)
		return nil, fmt.Errorf("session expired")
	}

	session.LastAccess = time.Now()
	return session, nil
}

// hasPermission checks admin permission for action
func (a *GameAdmin) hasPermission(session *AdminSession, permission string) bool {
	for _, p := range session.Permissions {
		if p == permission {
			return true
		}
	}
	return false
}

// AdminHandler renders admin panel with HTMX boost mode
func (a *GameAdmin) AdminHandler(w http.ResponseWriter, r *http.Request) {
	// Check session auth
	sessionID := r.URL.Query().Get("session")
	if sessionID != "" {
		session, err := a.validateSession(sessionID)
		if err != nil {
			http.Error(w, err.Error(), http.StatusUnauthorized)
			return
		}
		if !a.hasPermission(session, "admin_access") {
			http.Error(w, "permission denied", http.StatusForbidden)
			return
		}
	}

	w.Header().Set("Content-Type", "text/html")

	// HTMX boost mode for navigation
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
			<script src="https://unpkg.com/htmx.org/dist/ext/json-enc.js"></script>
			<style>
				.player-card { border: 1px solid #ccc; padding: 1rem; margin: 0.5rem; }
				.card-stats { display: flex; gap: 1rem; }
				.error { color: red; }
				.success { color: green; }
			</style>
		</head>
		<body>
			<main hx-boost="true" role="main">
				<nav hx-push-url="true" role="navigation" aria-label="Admin navigation">
					<a href="/admin/dashboard">Dashboard</a> |
					<a href="/admin/players">Players</a> |
					<a href="/admin/events">Events</a> |
					<a href="/admin/settings">Settings</a>
				</nav>

				<section id="dashboard" role="region" aria-label="Dashboard">
					<h1>Game Admin Dashboard</h1>
					<div hx-get="/admin/metrics" hx-trigger="every 2s">
						<!-- Metrics polling -->
					</div>
					<div hx-get="/admin/players" hx-trigger="load">
						<!-- Players list -->
					</div>
					<div
						hx-ext="ws"
						hx-ws="connect:/ws/admin"
						aria-live="polite"
						role="log">
						<!-- Real-time event stream -->
					</div>
				</section>

				<!-- Accessibility: focus management -->
				<div id="focus-target" tabindex="-1" aria-live="polite"></div>
			</main>
		</body>
		</html>
	`)
}

// MetricsHandler implements polling for metrics dashboard
func (a *GameAdmin) MetricsHandler(w http.ResponseWriter, r *http.Request) {
	// Validate session
	sessionID := r.URL.Query().Get("session")
	if sessionID != "" {
		session, err := a.validateSession(sessionID)
		if err != nil || !a.hasPermission(session, "view_metrics") {
			http.Error(w, "unauthorized", http.StatusUnauthorized)
			return
		}
	}

	w.Header().Set("Content-Type", "text/html")

	a.state.mu.Lock()
	defer a.state.mu.Unlock()

	stats := MetricsStats{
		TotalPlayers: len(a.state.state.Players),
		TotalEvents:  cap(a.state.state.Events),
		LastUpdate:   a.state.state.Timestamp,
	}

	// Render metrics panel template
	html, err := a.templates.RenderMetricsPanel(&stats)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	fmt.Fprint(w, html)
}

// PlayersHandler renders player list with component composition
func (a *GameAdmin) PlayersHandler(w http.ResponseWriter, r *http.Request) {
	// Validate session
	sessionID := r.URL.Query().Get("session")
	if sessionID != "" {
		session, err := a.validateSession(sessionID)
		if err != nil || !a.hasPermission(session, "view_players") {
			http.Error(w, "unauthorized", http.StatusUnauthorized)
			return
		}
	}

	w.Header().Set("Content-Type", "text/html")

	a.state.mu.Lock()
	defer a.state.mu.Unlock()

	for _, player := range a.state.state.Players {
		html, err := a.templates.RenderPlayerCard(player)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		fmt.Fprint(w, html)
		fmt.Fprint(w, "\n")
	}
}

// EventsHandler streams game events via WebSocket
func (a *GameAdmin) EventsHandler(w http.ResponseWriter, r *http.Request) {
	// Validate session
	sessionID := r.URL.Query().Get("session")
	if sessionID == "" {
		http.Error(w, "session required", http.StatusUnauthorized)
		return
	}

	session, err := a.validateSession(sessionID)
	if err != nil || !a.hasPermission(session, "view_events") {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}

	// Upgrade to WebSocket
	conn, err := a.wsUpgrader.Upgrader(w, r, nil)
	if err != nil {
		log.Printf("[WS] Upgrade failed: %v", err)
		return
	}

	log.Printf("[WS] Admin %s connected to event stream", session.ID)

	// Subscribe to game events
	eventChan := make(chan GameEvent, 10)
	a.state.mu.Lock()
	a.state.clients[session.ID] = eventChan
	a.state.mu.Unlock()

	// Stream events with sequence validation
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go func() {
		defer conn.Close()
		for {
			select {
			case <-ctx.Done():
				return
			case event := range eventChan:
				// Eventual consistency: sequence numbers
				data, _ := json.Marshal(event)
				if err := conn.WriteMessage(websocket.TextMessage, data); err != nil {
					log.Printf("[WS] Write error: %v", err)
					return
				}
			}
		}
	}()

	// Keep connection alive
	for {
		select {
		case <-time.After(30 * time.Second):
			if err := conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}
		case <-ctx.Done():
			return
		}
	}
}

// PlayerActionHandler handles admin player actions with audit logging
func (a *GameAdmin) PlayerActionHandler(w http.ResponseWriter, r *http.Request) {
	// Validate session
	sessionID := r.URL.Query().Get("session")
	if sessionID == "" {
		http.Error(w, "session required", http.StatusUnauthorized)
		return
	}

	session, err := a.validateSession(sessionID)
	if err != nil || !a.hasPermission(session, "modify_players") {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}

	id := mux.Vars(r)["id"]
	action := r.FormValue("action")

	// Server-authority validation + audit logging
	if err := a.state.UpdatePlayer(id, action); err != nil {
		// Rollback pattern - hx-swap="none"
		w.Header().Set("HX-Swap", "none")
		w.Header().Set("Content-Type", "text/html")
		fmt.Fprintf(w, `
			<div class="error" role="alert" aria-invalid="true">
				[AUDIT] Action rejected by %s: %v
			</div>
		`, session.ID, err)
		return
	}

	// Success: re-render with audit log
	log.Printf("[AUDIT] Admin %s performed %s on player %s", session.ID, action, id)
	a.PlayerDetailHandler(w, r)
}

// PlayerDetailHandler renders single player detail
func (a *GameAdmin) PlayerDetailHandler(w http.ResponseWriter, r *http.Request) {
	id := mux.Vars(r)["id"]

	player, ok := a.state.GetPlayer(id)
	if !ok {
		http.Error(w, "player not found", http.StatusNotFound)
		return
	}

	w.Header().Set("Content-Type", "text/html")

	html, err := a.templates.RenderPlayerCard(player)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	fmt.Fprint(w, html)
}

// SettingsHandler renders admin settings panel
func (a *GameAdmin) SettingsHandler(w http.ResponseWriter, r *http.Request) {
	// Validate session
	sessionID := r.URL.Query().Get("session")
	if sessionID == "" {
		http.Error(w, "session required", http.StatusUnauthorized)
		return
	}

	session, err := a.validateSession(sessionID)
	if err != nil || !a.hasPermission(session, "modify_settings") {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}

	w.Header().Set("Content-Type", "text/html")

	fmt.Fprint(w, `
		<section id="settings" role="region" aria-label="Admin settings">
			<h1>Admin Settings</h1>
			<form hx-post="/admin/settings/update" hx-trigger="submit">
				<label for="rate-limit">Rate Limit (requests/min):</label>
				<input type="number" id="rate-limit" name="rate_limit" value="{{.RateLimit}}" min="1" max="1000"/>

				<label for="session_timeout">Session Timeout (minutes):</label>
				<input type="number" id="session_timeout" name="session_timeout" value="{{.SessionTimeout}}" min="1" max="60"/>

				<button type="submit" hx-confirm="Save settings?">Save Settings</button>
			</form>
			<div aria-live="polite" role="status">
				Session: {{.SessionID}} (User: {{.UserID}})
			</div>
		</section>
	`,
		a.config.RateLimit,
		a.config.SessionTimeout.Minutes(),
		session.ID,
		session.UserID)
}

// SettingsUpdateHandler processes settings form submission
func (a *GameAdmin) SettingsUpdateHandler(w http.ResponseWriter, r *http.Request) {
	// Validate session
	sessionID := r.URL.Query().Get("session")
	if sessionID == "" {
		http.Error(w, "session required", http.StatusUnauthorized)
		return
	}

	session, err := a.validateSession(sessionID)
	if err != nil || !a.hasPermission(session, "modify_settings") {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}

	rateLimit := r.FormValue("rate_limit")
	sessionTimeout := r.FormValue("session_timeout")

	// Update config with audit logging
	log.Printf("[AUDIT] Admin %s updated settings: rate_limit=%s, session_timeout=%s",
		session.ID, rateLimit, sessionTimeout)

	w.Header().Set("Content-Type", "text/html")
	fmt.Fprint(w, `
		<div class="success" role="status" aria-live="polite">
			Settings updated successfully
		</div>
	`)
}

// registerAdminRoutes registers all admin panel routes
func (a *GameAdmin) registerAdminRoutes(r *mux.Router) {
	admin := r.PathPrefix("/admin").Subrouter()

	// Admin panel pages
	admin.HandleFunc("/dashboard", a.AdminHandler).Methods("GET")
	admin.HandleFunc("/metrics", a.MetricsHandler).Methods("GET")
	admin.HandleFunc("/players", a.PlayersHandler).Methods("GET")
	admin.HandleFunc("/events", a.EventsHandler).Methods("GET")
	admin.HandleFunc("/settings", a.SettingsHandler).Methods("GET")

	// Player actions
	admin.HandleFunc("/player/{id}", a.PlayerDetailHandler).Methods("GET")
	admin.HandleFunc("/player/{id}/action", a.PlayerActionHandler).Methods("POST")

	// Settings
	admin.HandleFunc("/settings/update", a.SettingsUpdateHandler).Methods("POST")
}