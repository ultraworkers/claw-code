// HTMX Template Blocks - Go 1.22+ Component Composition
// Production-ready template patterns for HTMX component composition
//
// Last Updated: 2026-03-14
// Go Version: 1.22+

package main

import (
	"bytes"
	"html/template"
	"io"
	"strings"
)

// TemplateBlocks defines reusable HTMX component templates
// Uses Go 1.22+ block syntax for component composition
type TemplateBlocks struct {
	PlayerCard   *template.Template
	MetricsPanel *template.Template
	EventStream  *template.Template
	AdminLayout  *template.Template
}

// NewTemplateBlocks initializes all template blocks
func NewTemplateBlocks() (*TemplateBlocks, error) {
	t := &TemplateBlocks{}

	// Player card component - demonstrates hx-swap="innerHTML"
	playerCard, err := template.New("player-card").Parse(`
{{block "player-card" .}}
<div class="player-card"
     id="player-{{.ID}}"
     hx-get="/players/{{.ID}}"
     hx-swap="innerHTML"
     hx-trigger="every 5s">
	<div class="card-header" role="heading" aria-level="2">
		<span class="name">{{.Name}}</span>
		<span class="level">Level {{.Level}}</span>
	</div>
	<div class="card-stats" role="list" aria-label="Player stats">
		<span class="stat hp" role="listitem" aria-label="Hit Points">
			HP: {{.HP}}
		</span>
		<span class="stat mp" role="listitem" aria-label="Magic Points">
			MP: {{.MP}}
		</span>
		<span class="stat xp" role="listitem" aria-label="Experience">
			XP: {{.XP}}
		</span>
	</div>
	<div class="card-actions" role="group" aria-label="Player actions">
		<button
			hx-post="/api/player/{{.ID}}/action"
			hx-vals="js:{action: 'attack'}"
			hx-target="#player-{{.ID}}"
			hx-confirm="Confirm attack action?"
			aria-label="Attack action for {{.Name}}"
			type="button">
			Attack
		</button>
		<button
			hx-post="/api/player/{{.ID}}/action"
			hx-vals="js:{action: 'heal'}"
			hx-target="#player-{{.ID}}"
			hx-confirm="Confirm heal action?"
			aria-label="Heal action for {{.Name}}"
			type="button">
			Heal
		</button>
		<button
			hx-post="/api/player/{{.ID}}/action"
			hx-vals="js:{action: 'rest'}"
			hx-target="#player-{{.ID}}"
			aria-label="Rest action for {{.Name}}"
			type="button">
			Rest
		</button>
	</div>
	<div class="card-footer">
		<span class="last-action" aria-live="polite">Last: {{.LastAction}}</span>
		<span class="updated">Updated: {{.UpdatedAt}}</span>
	</div>
</div>
{{end}}
`)
	if err != nil {
		return nil, err
	}
	t.PlayerCard = playerCard

	// Metrics panel - demonstrates hx-trigger polling
	metricsPanel, err := template.New("metrics-panel").Parse(`
{{block "metrics-panel" .}}
<div id="metrics-panel"
     hx-get="/metrics"
     hx-trigger="every 2s"
     hx-swap="innerHTML"
     role="region"
     aria-label="Game metrics">
	<div class="metric" role="list" aria-label="Metrics">
		<span class="metric-item" role="listitem">
			Total Players: {{.TotalPlayers}}
		</span>
		<span class="metric-item" role="listitem">
			Active Events: {{.TotalEvents}}
		</span>
		<span class="metric-item" role="listitem">
			Last Update: {{.LastUpdate}}
		</span>
	</div>
	<span aria-live="polite" class="update-indicator">
		Updated at {{.LastUpdate}}
	</span>
</div>
{{end}}
`)
	if err != nil {
		return nil, err
	}
	t.MetricsPanel = metricsPanel

	// Event stream - demonstrates WebSocket/SSE integration
	eventStream, err := template.New("event-stream").Parse(`
{{block "event-stream" .}}
<div class="event-stream"
     hx-ext="ws"
     hx-ws="connect:/ws?client_id={{.ClientID}}"
     aria-live="assertive"
     role="log"
     aria-label="Game event stream">
	<div class="event-header">
		<span>Event Stream (Client: {{.ClientID}})</span>
	</div>
	<div class="event-container" id="events-{{.ClientID}}" role="list">
		<!-- Events streamed via WebSocket -->
		<span aria-live="polite">Connected to event stream</span>
	</div>
</div>
{{end}}
`)
	if err != nil {
		return nil, err
	}
	t.EventStream = eventStream

	// Admin layout - demonstrates boost mode + push URL
	adminLayout, err := template.New("admin-layout").Parse(`
{{block "admin-layout" .}}
<!DOCTYPE html>
<html lang="{{.Lang}}">
<head>
	<meta charset="UTF-8">
	<meta name="viewport" content="width=device-width, initial-scale=1.0">
	<title>{{.Title}}</title>
	<script src="https://unpkg.com/htmx.org@1.9.10"></script>
	<script src="https://unpkg.com/htmx.org/dist/ext/ws.js"></script>
	<script src="https://unpkg.com/htmx.org/dist/ext/sse.js"></script>
	{{block "head-extra" .}}{{end}}
</head>
<body>
	<main hx-boost="{{.Boost}}" role="main">
		<nav hx-push-url="{{.PushURL}}" role="navigation" aria-label="Main navigation">
			{{block "nav-items" .}}{{end}}
		</nav>

		<section id="content" role="region" aria-label="Main content">
			{{block "content" .}}{{end}}
		</section>

		<!-- Accessibility: focus management for HTMX updates -->
		<div id="focus-target" tabindex="-1" aria-live="polite"></div>

		{{block "body-extra" .}}{{end}}
	</main>
</body>
</html>
{{end}}
`)
	if err != nil {
		return nil, err
	}
	t.AdminLayout = adminLayout

	return t, nil
}

// RenderPlayerCard renders player card component
func (t *TemplateBlocks) RenderPlayerCard(player *Player) (string, error) {
	var buf bytes.Buffer
	if err := t.PlayerCard.ExecuteTemplate(&buf, "player-card", player); err != nil {
		return "", err
	}
	return strings.TrimSpace(buf.String()), nil
}

// RenderMetricsPanel renders metrics panel component
func (t *TemplateBlocks) RenderMetricsPanel(stats *MetricsStats) (string, error) {
	var buf bytes.Buffer
	if err := t.MetricsPanel.ExecuteTemplate(&buf, "metrics-panel", stats); err != nil {
		return "", err
	}
	return strings.TrimSpace(buf.String()), nil
}

// RenderEventStream renders event stream component
func (t *TemplateBlocks) RenderEventStream(clientID string) (string, error) {
	data := struct{ ClientID string }{ClientID: clientID}
	var buf bytes.Buffer
	if err := t.EventStream.ExecuteTemplate(&buf, "event-stream", data); err != nil {
		return "", err
	}
	return strings.TrimSpace(buf.String()), nil
}

// RenderAdminLayout renders admin layout with composition
func (t *TemplateBlocks) RenderAdminLayout(config AdminLayoutConfig) (string, error) {
	var buf bytes.Buffer
	if err := t.AdminLayout.ExecuteTemplate(&buf, "admin-layout", config); err != nil {
		return "", err
	}
	return strings.TrimSpace(buf.String()), nil
}

// MetricsStats represents metrics panel data
type MetricsStats struct {
	TotalPlayers int
	TotalEvents  int
	LastUpdate   time.Time
}

// AdminLayoutConfig represents admin layout configuration
type AdminLayoutConfig struct {
	Lang     string
	Title    string
	Boost    bool
	PushURL  bool
	NavItems template.HTML
	Content  template.HTML
}

// ExecuteTemplate writes template to writer
func ExecuteTemplate(w io.Writer, name string, data interface{}) error {
	tmpl := template.Must(template.New(name).Parse(`
{{block "{{.Name}}" .Data}}
{{.Content}}
{{end}}
`))
	return tmpl.ExecuteTemplate(w, name, struct {
		Name   string
		Data   interface{}
		Content template.HTML
	}{
		Name:    name,
		Data:    data,
		Content: "",
	})
}