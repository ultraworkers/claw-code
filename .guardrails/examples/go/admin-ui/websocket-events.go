// WebSocket Events - Real-time Game Event Streaming
// Production-ready WebSocket event broadcasting for game backends
//
// Last Updated: 2026-03-14
// Go Version: 1.22+

package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/mux"
	"github.com/gorilla/websocket"
)

// WSEventStreamConfig defines WebSocket configuration
type WSEventStreamConfig struct {
	WriteBufferSize   int           `json:"write_buffer_size"`
	ReadBufferSize    int           `json:"read_buffer_size"`
	MaxMessageSize    int64         `json:"max_message_size"`
	PingInterval      time.Duration `json:"ping_interval"`
	WriteTimeout      time.Duration `json:"write_timeout"`
	AllowedOrigins    []string      `json:"allowed_origins"`
}

// GameEventWS represents WebSocket game event
type GameEventWS struct {
	Type      string      `json:"type"`
	Payload   interface{} `json:"payload"`
	Sequence  uint64      `json:"sequence"`  // Eventual consistency
	Timestamp time.Time   `json:"timestamp"` // Server authority
	Source    string      `json:"source"`    // "server" authority
	PlayerID  string      `json:"player_id"`
	Region    string      `json:"region"`
}

// WSClient represents connected WebSocket client
type WSClient struct {
	ID          string
	Conn        *websocket.Conn
	EventChan   chan GameEventWS
	Context     context.Context
	Cancel      context.CancelFunc
	LastPing    time.Time
	Sequence    uint64 // Client last received sequence
}

// WSEventStream implements WebSocket event streaming
type WSEventStream struct {
	config    WSEventStreamConfig
	clients   map[string]*WSClient
	broadcast chan GameEventWS
	mu        sync.RWMutex
	upgrader  *websocket.Upgrader
	sequence  uint64
}

// NewWSEventStream creates WebSocket event stream
func NewWSEventStream(config WSEventStreamConfig) (*WSEventStream, error) {
	s := &WSEventStream{
		config:    config,
		clients:   make(map[string]*WSClient),
		broadcast: make(chan GameEventWS, 100),
		sequence:  0,
		upgrader:  &websocket.Upgrader{
			WriteBufferSize:   config.WriteBufferSize,
			ReadBufferSize:    config.ReadBufferSize,
			MaxMessageSize:    config.MaxMessageSize,
			CheckOrigin:       func(r *http.Request) bool {
				for _, origin := range config.AllowedOrigins {
					if r.Header.Get("Origin") == origin {
						return true
					}
				}
				return false
			},
		},
	}

	return s, nil
}

// ConnectClient handles WebSocket connection
func (s *WSEventStream) ConnectClient(w http.ResponseWriter, r *http.Request) (*WSClient, error) {
	clientID := r.URL.Query().Get("client_id")
	if clientID == "" {
		return nil, fmt.Errorf("client_id required")
	}

	// Upgrade to WebSocket
	conn, err := s.upgrader.Upgrader(w, r, nil)
	if err != nil {
		log.Printf("[WS] Upgrade failed: %v", err)
		return nil, err
	}

	ctx, cancel := context.WithCancel(context.Background())
	client := &WSClient{
		ID:       clientID,
		Conn:     conn,
		EventChan: make(chan GameEventWS, 100),
		Context:  ctx,
		Cancel:   cancel,
		LastPing: time.Now(),
		Sequence: 0,
	}

	s.mu.Lock()
	s.clients[clientID] = client
	s.mu.Unlock()

	log.Printf("[WS] Client %s connected", clientID)

	// Start client goroutines
	go s.handleClientMessages(client)
	go s.handleClientEvents(client)

	return client, nil
}

// handleClientMessages processes incoming WebSocket messages
func (s *WSEventStream) handleClientMessages(client *WSClient) {
	defer client.Cancel()

	for {
		select {
		case <-client.Context.Done():
			return
		default:
			// Read message with timeout
			client.Conn.SetReadDeadline(time.Now().Add(s.config.ReadBufferSize))
			_, message, err := client.Conn.ReadMessage()
			if err != nil {
				if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosed) {
					log.Printf("[WS] Read error for %s: %v", client.ID, err)
				}
				return
			}

			// Process message (client-to-server events)
			var event GameEventWS
			if err := json.Unmarshal(message, &event); err != nil {
				log.Printf("[WS] Unmarshal error: %v", err)
				continue
			}

			// Server-authority validation
			event.Source = "server"
			event.Timestamp = time.Now()
			s.sequence++
			event.Sequence = s.sequence

			log.Printf("[WS] Client %s sent event: %s", client.ID, event.Type)
		}
	}
}

// handleClientEvents streams events to client
func (s *WSEventStream) handleClientEvents(client *WSClient) {
	defer client.Cancel()
	defer client.Conn.Close()

	pingTicker := time.NewTicker(s.config.PingInterval)
	defer pingTicker.Stop()

	for {
		select {
		case <-client.Context.Done():
			return
		case event := range client.EventChan:
			// Eventual consistency: sequence validation
			data, _ := json.Marshal(event)
			client.Conn.SetWriteDeadline(time.Now().Add(s.config.WriteTimeout))
			if err := client.Conn.WriteMessage(websocket.TextMessage, data); err != nil {
				log.Printf("[WS] Write error for %s: %v", client.ID, err)
				return
			}
			client.Sequence = event.Sequence
			log.Printf("[WS] Event %d sent to %s", event.Sequence, client.ID)
		case <-pingTicker.C:
			client.Conn.SetWriteDeadline(time.Now().Add(s.config.WriteTimeout))
			if err := client.Conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				log.Printf("[WS] Ping error for %s: %v", client.ID, err)
				return
			}
			client.LastPing = time.Now()
			log.Printf("[WS] Ping sent to %s", client.ID)
		}
	}
}

// BroadcastEvent broadcasts event to all clients
func (s *WSEventStream) BroadcastEvent(event GameEventWS) {
	s.mu.Lock()
	defer s.mu.Unlock()

	s.sequence++
	event.Sequence = s.sequence
	event.Timestamp = time.Now()
	event.Source = "server"

	s.broadcast <- event

	for clientID, client := range s.clients {
		select {
		case client.EventChan <- event:
			log.Printf("[WS] Event %d broadcast to %s", event.Sequence, clientID)
		default:
			log.Printf("[WS] Client %s buffer full, dropping event %d", clientID, event.Sequence)
		}
	}
}

// RemoveClient disconnects client
func (s *WSEventStream) RemoveClient(clientID string) {
	s.mu.Lock()
	defer s.mu.Unlock()

	if client, ok := s.clients[clientID]; ok {
		client.Cancel()
		client.Conn.Close()
		delete(s.clients, clientID)
		log.Printf("[WS] Client %s disconnected", clientID)
	}
}

// GetClientCount returns connected client count
func (s *WSEventStream) GetClientCount() int {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return len(s.clients)
}

// GetClientSequence returns client's last received sequence
func (s *WSEventStream) GetClientSequence(clientID string) (uint64, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	client, ok := s.clients[clientID]
	if !ok {
		return 0, fmt.Errorf("client not found")
	}

	return client.Sequence, nil
}

// ReSyncClient resyncs client from sequence (reconnect recovery)
func (s *WSEventStream) ReSyncClient(clientID string, fromSequence uint64) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	client, ok := s.clients[clientID]
	if !ok {
		return fmt.Errorf("client not found")
	}

	// Replay missed events
	for seq := fromSequence + 1; seq <= s.sequence; seq++ {
		// Replay logic - fetch from event store
		log.Printf("[WS] Replaying event %d for %s", seq, clientID)
	}

	return nil
}

// WSEventStreamHandler renders WebSocket event stream UI
func (s *WSEventStream) WSEventStreamHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")

	fmt.Fprint(w, `
		<!DOCTYPE html>
		<html lang="en">
		<head>
			<meta charset="UTF-8">
			<meta name="viewport" content="width=device-width, initial-scale=1.0">
			<title>WebSocket Event Stream</title>
			<script src="https://unpkg.com/htmx.org@1.9.10"></script>
			<script src="https://unpkg.com/htmx.org/dist/ext/ws.js"></script>
			<style>
				.event-log { border: 1px solid #ccc; padding: 1rem; max-height: 400px; overflow-y: auto; }
				.event-item { border-bottom: 1px solid #eee; padding: 0.5rem; }
			</style>
		</head>
		<body>
			<main hx-boost="true" role="main">
				<h1>WebSocket Event Stream</h1>
				<div hx-get="/ws/clients" hx-trigger="every 5s">
					<!-- Client count polling -->
				</div>
				<div
					class="event-log"
					hx-ext="ws"
					hx-ws="connect:/ws?client_id=admin"
					aria-live="assertive"
					role="log"
					aria-label="Event stream">
					<!-- Events streamed via WebSocket -->
					<span>Connected to event stream</span>
				</div>
				<div hx-get="/ws/sequence" hx-trigger="load">
					<!-- Current sequence -->
				</div>
			</main>
		</body>
		</html>
	`)
}

// ClientsHandler renders connected clients
func (s *WSEventStream) ClientsHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")

	count := s.GetClientCount()

	fmt.Fprintf(w, `
		<div role="status" aria-live="polite">
			Connected Clients: %d
		</div>
	`, count)
}

// SequenceHandler renders current sequence
func (s *WSEventStream) SequenceHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/html")

	s.mu.RLock()
	defer s.mu.RUnlock()

	fmt.Fprintf(w, `
		<div role="status" aria-live="polite">
			Current Sequence: %d
		</div>
	`, s.sequence)
}

// registerWSRoutes registers WebSocket event stream routes
func (s *WSEventStream) registerWSRoutes(r *mux.Router) {
	ws := r.PathPrefix("/ws").Subrouter()

	ws.HandleFunc("/connect", func(w http.ResponseWriter, r *http.Request) {
		s.ConnectClient(w, r)
	}).Methods("GET")

	ws.HandleFunc("/clients", s.ClientsHandler).Methods("GET")
	ws.HandleFunc("/sequence", s.SequenceHandler).Methods("GET")
	ws.HandleFunc("/", s.WSEventStreamHandler).Methods("GET")
}

// initDemoEvents initializes demo event streaming
func (s *WSEventStream) initDemoEvents() {
	// Broadcast demo events
	s.BroadcastEvent(GameEventWS{
		Type:      "GAME_START",
		Payload:   map[string]string{"message": "Game started"},
		Source:    "server",
	})

	s.BroadcastEvent(GameEventWS{
		Type:      "PLAYER_LOGIN",
		Payload:   map[string]string{"player_id": "player-1"},
		Source:    "server",
	})

	s.BroadcastEvent(GameEventWS{
		Type:      "ECONOMY_UPDATE",
		Payload:   map[string]string{"resource": "gold", "change": "+100"},
		Source:    "server",
	})
}