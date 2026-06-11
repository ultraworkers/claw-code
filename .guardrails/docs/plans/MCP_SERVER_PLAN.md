# Guardrail MCP Server Implementation Plan

> **Version:** 1.2
> **Status:** Planning (Security & Scalability Review Complete)

## Overview

Build a Guardrail Platform that serves as a central authority for guardrail enforcement:
- **Database-backed**: All Markdown/guardrail data stored in PostgreSQL
- **Caching Layer**: Redis for rule caching and rate limiting
- **Web UI**: Browse and edit guardrails (replaces reading MD files directly)
- **MCP Endpoint**: TUI clients (Claude Code, OpenCode, etc.) connect for live validation
- **Deployment**: Runs as containers (see `.env` for deployment target)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Deployment Server                        │
│  ┌─────────────────────────────────────────────────────┐   │
│  │           Guardrail MCP Server Container            │   │
│  │  ┌─────────────┐  ┌─────────────┐                  │   │
│  │  │   MCP SSE   │  │   Web UI    │                  │   │
│  │  │   :8080     │  │   :8081     │  (source of      │   │
│  │  └──────┬──────┘  └──────┬──────┘   truth)         │   │
│  │         │                │                         │   │
│  │         └────────────────┘                         │   │
│  │                      │                             │   │
│  │  ┌───────────────────┴───────────────────┐        │   │
│  │  │              Redis Cache              │        │   │
│  │  │  - Active rules cache (TTL: 5m)      │        │   │
│  │  │  - Rate limiting counters            │        │   │
│  │  │  - Session tokens                    │        │   │
│  │  └─────────────────────────────────────┘        │   │
│  │                      │                             │   │
│  │  ┌───────────────────┴───────────────────┐        │   │
│  │  │           PostgreSQL                  │        │   │
│  │  │  - documents (edited via Web UI)      │        │   │
│  │  │  - prevention_rules                   │        │   │
│  │  │  - failure_registry                   │        │   │
│  │  │  - projects                           │        │   │
│  │  └─────────────────────────────────────┘        │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  One-Time Ingest (migration only):                   │   │
│  │  Markdown files → PostgreSQL (run once at setup)     │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
    ┌─────────────────┐        ┌──────────────────┐
    │  TUI Clients    │        │  Web Browser     │
    │  (Claude Code,  │        │  (View/Edit      │
    │   OpenCode)     │        │   Guardrails)    │
    └─────────────────┘        └──────────────────┘

    ┌─────────────────┐        ┌──────────────────┐
    │  IDE Clients    │        │  IDE Clients     │
    │  (VS Code,      │        │  (JetBrains,     │
    │   Cursor)       │        │   Vim/Neovim)    │
    │  via Extension  │        │  via LSP/HTTP)   │
    └─────────────────┘        └──────────────────┘
```

### Components

| Component | Port | Purpose |
|-----------|------|---------|
| MCP Server | 8080 | SSE endpoint for TUI clients (Claude Code, OpenCode) |
| Web UI | 8081 | Browser UI (localhost only, no auth needed) |
| IDE API | 8081 | IDE extensions endpoint (VS Code, JetBrains) |
| Health | 8081 | `/health/live`, `/health/ready` probes |
| Metrics | 8081 | `/metrics` Prometheus endpoint |
| PostgreSQL | internal | Data persistence |
| Redis | internal | Caching, rate limiting, sessions |

**Note:** Web UI runs in the same container as the server, so it communicates internally without API keys. Only external clients (MCP TUI clients, IDE extensions) require authentication.

### Data Flow

1. **One-Time Ingest**: Load all MD files into PostgreSQL (migration only)
2. **Web UI**: Users browse/edit guardrails stored in database (source of truth)
3. **MCP Server**: Validates tool calls from multiple concurrent TUI/IDE clients
4. **IDE API**: HTTP endpoints for IDE extensions (VS Code, JetBrains, Vim)
5. **Caching**: Active rules cached in Redis (5 min TTL)

---

## Tech Stack

- **Go 1.23+** - Server implementation
- **mark3labs/mcp-go** - MCP protocol
- **Echo** - HTTP framework for web UI
- **PostgreSQL 16** - Database
- **Redis 7** - Caching and rate limiting
- **caarlos0/env** - Configuration (no Viper)
- **slog** - Structured logging
- **golang-migrate** - Database migrations

---

## Project Structure

```
cmd/
├── server/
│   └── main.go          # MCP + Web server (always running)
└── ingest/
    └── main.go          # One-time migration tool (run once)

internal/
├── models/              # Data models
├── database/            # PostgreSQL operations + migrations
│   └── migrations/      # golang-migrate files
├── cache/               # Redis client
├── ingester/            # MD file ingestion
├── guardrails/          # Validation logic
├── web/                 # HTTP server + UI
│   ├── handlers.go      # REST API handlers
│   ├── middleware.go    # Auth, CSRF, rate limiting
│   └── static/          # Embedded web UI
├── mcp/                 # MCP protocol handlers
├── config/              # Configuration
└── version/

deploy/
├── Dockerfile
├── Dockerfile.ingest    # Separate image for ingest job
└── podman-compose.yml

scripts/
└── ingest.sh
```

---

## Database Schema

### Tables

**documents**
```sql
CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug VARCHAR(255) UNIQUE NOT NULL,
    title VARCHAR(500) NOT NULL,
    content TEXT NOT NULL,
    search_vector tsvector,
    category VARCHAR(50) NOT NULL CHECK (category IN ('workflow', 'standard', 'guide', 'reference')),
    path VARCHAR(500) NOT NULL,
    version INTEGER DEFAULT 1,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_documents_category ON documents(category);
CREATE INDEX idx_documents_slug ON documents(slug);
CREATE INDEX idx_documents_updated ON documents(updated_at DESC);
CREATE INDEX idx_documents_search ON documents USING GIN(search_vector);
CREATE INDEX idx_documents_metadata ON documents USING GIN(metadata);
```

**prevention_rules**
```sql
CREATE TABLE prevention_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id VARCHAR(50) UNIQUE NOT NULL CHECK (LENGTH(TRIM(rule_id)) > 0),
    name VARCHAR(255) NOT NULL,
    pattern TEXT NOT NULL,
    pattern_hash VARCHAR(64), -- For exact-match pre-filtering
    message TEXT NOT NULL,
    severity VARCHAR(10) NOT NULL CHECK (severity IN ('error', 'warning', 'info')),
    enabled BOOLEAN NOT NULL DEFAULT true,
    document_id UUID REFERENCES documents(id) ON DELETE SET NULL,
    category VARCHAR(50),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_rules_document ON prevention_rules(document_id);
CREATE INDEX idx_rules_enabled ON prevention_rules(enabled) WHERE enabled = true;
CREATE INDEX idx_rules_severity ON prevention_rules(severity);
CREATE INDEX idx_rules_category ON prevention_rules(category);
CREATE INDEX idx_rules_covering ON prevention_rules(document_id, rule_id, name, severity, enabled)
    INCLUDE (pattern, message);
```

**failure_registry**
```sql
CREATE TABLE failure_registry (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    failure_id VARCHAR(50) UNIQUE NOT NULL,
    category VARCHAR(50) NOT NULL,
    severity VARCHAR(10) NOT NULL CHECK (severity IN ('critical', 'high', 'medium', 'low')),
    error_message TEXT NOT NULL,
    root_cause TEXT,
    affected_files TEXT[],
    regression_pattern VARCHAR(255),
    status VARCHAR(20) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'resolved', 'deprecated')),
    project_slug VARCHAR(100),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (created_at);

-- Indexes
CREATE INDEX idx_failures_status ON failure_registry(status);
CREATE INDEX idx_failures_category ON failure_registry(category);
CREATE INDEX idx_failures_created ON failure_registry(created_at DESC);
CREATE INDEX idx_failures_files ON failure_registry USING GIN(affected_files);
CREATE INDEX idx_failures_project ON failure_registry(project_slug);
CREATE INDEX idx_failures_covering ON failure_registry(status, created_at DESC, severity)
    INCLUDE (failure_id, category, error_message);
```

**projects**
```sql
CREATE TABLE projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL CHECK (LENGTH(TRIM(slug)) > 0),
    guardrail_context TEXT,
    active_rules VARCHAR(50)[],
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_projects_slug ON projects(slug);
CREATE INDEX idx_projects_active_rules ON projects USING GIN(active_rules);
CREATE INDEX idx_projects_metadata ON projects USING GIN(metadata);
```

**schema_migrations** (auto-created by golang-migrate)
```sql
CREATE TABLE schema_migrations (
    version BIGINT PRIMARY KEY,
    dirty BOOLEAN NOT NULL
);
```

### Migration Files

```
internal/database/migrations/
├── 001_create_tables.up.sql
├── 001_create_tables.down.sql
├── 002_add_indexes.up.sql
├── 002_add_indexes.down.sql
├── 003_add_constraints.up.sql
├── 003_add_constraints.down.sql
├── 004_add_search_vector.up.sql
└── 004_add_search_vector.down.sql
```

---

## MCP Protocol Specification

### Transport

- **Server-to-Client**: SSE (Server-Sent Events) on `/mcp/v1/sse`
- **Client-to-Server**: HTTP POST on `/mcp/v1/message`
- **Session Management**: JWT tokens with 15-minute expiry

### Session Initialization

**Tool:** `guardrail_init_session`

```json
{
  "name": "guardrail_init_session",
  "description": "Initialize a validation session for a project",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_slug": { "type": "string" },
      "agent_type": { "type": "string", "enum": ["claude-code", "opencode", "cursor", "other"] },
      "client_version": { "type": "string" }
    },
    "required": ["project_slug"]
  }
}
```

**Response:**
```json
{
  "session_token": "eyJhbGciOiJIUzI1NiIs...",
  "expires_at": "2026-02-07T11:30:00Z",
  "project_context": "...",
  "active_rules_count": 15,
  "capabilities": ["bash_validation", "git_validation", "edit_validation"]
}
```

### JWT Configuration

```go
// internal/auth/jwt.go

type JWTConfig struct {
    Secret         string        `env:"JWT_SECRET,required"`              // Min 32 bytes
    RotationPeriod time.Duration `env:"JWT_ROTATION_HOURS,default=168"`   // 7 days
    Issuer         string        `env:"JWT_ISSUER,default=guardrail-mcp"`
    Expiry         time.Duration `env:"JWT_EXPIRY,default=15m"`
}

func ValidateJWTSecret(secret string) error {
    if len(secret) < 32 {
        return fmt.Errorf("JWT_SECRET must be at least 32 bytes, got %d", len(secret))
    }
    // Check entropy
    var entropy float64
    for _, b := range []byte(secret) {
        entropy += float64(bits.OnesCount8(b))
    }
    if entropy/float64(len(secret)) < 3.5 {
        return fmt.Errorf("JWT_SECRET has insufficient entropy")
    }
    return nil
}
```

**Security Requirements:**
- JWT_SECRET must be 32+ bytes with high entropy
- Use HS256 algorithm only (explicitly reject 'none')
- Tokens stored in Redis with 15-min TTL
- Implement token revocation list for logout

### Validation Tools

#### 1. `guardrail_validate_bash`

```json
{
  "name": "guardrail_validate_bash",
  "description": "Validate bash command against forbidden patterns",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session_token": { "type": "string" },
      "command": { "type": "string" },
      "working_directory": { "type": "string" }
    },
    "required": ["session_token", "command"]
  }
}
```

#### 2. `guardrail_validate_file_edit`

```json
{
  "name": "guardrail_validate_file_edit",
  "description": "Validate file edit operation",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session_token": { "type": "string" },
      "file_path": { "type": "string" },
      "old_string": { "type": "string" },
      "new_string": { "type": "string" },
      "change_description": { "type": "string" }
    },
    "required": ["session_token", "file_path", "old_string", "new_string"]
  }
}
```

#### 3. `guardrail_validate_git_operation`

```json
{
  "name": "guardrail_validate_git_operation",
  "description": "Validate git command against guardrails",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session_token": { "type": "string" },
      "command": { "type": "string", "enum": ["push", "commit", "merge", "rebase", "reset"] },
      "args": { "type": "array", "items": { "type": "string" } },
      "is_force": { "type": "boolean" }
    },
    "required": ["session_token", "command"]
  }
}
```

#### 4. `guardrail_validate_scope`

```json
{
  "name": "guardrail_validate_scope",
  "description": "Check if file path is in scope for session",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session_token": { "type": "string" },
      "file_path": { "type": "string" }
    },
    "required": ["session_token", "file_path"]
  }
}
```

#### 5. `guardrail_pre_work_check`

```json
{
  "name": "guardrail_pre_work_check",
  "description": "Run pre-work checklist from failure registry",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session_token": { "type": "string" },
      "affected_files": { "type": "array", "items": { "type": "string" } }
    },
    "required": ["session_token", "affected_files"]
  }
}
```

#### 6. `guardrail_batch_validate`

```json
{
  "name": "guardrail_batch_validate",
  "description": "Validate multiple operations at once",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session_token": { "type": "string" },
      "operations": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "tool": { "type": "string" },
            "args": { "type": "object" }
          }
        }
      }
    },
    "required": ["session_token", "operations"]
  }
}
```

### Response Format Standards

**Success Response (no violations):**
```json
{
  "valid": true,
  "violations": [],
  "meta": {
    "checked_at": "2026-02-07T10:30:00Z",
    "rules_evaluated": 15,
    "duration_ms": 12,
    "cached": true
  }
}
```

**Violation Response:**
```json
{
  "valid": false,
  "violations": [
    {
      "rule_id": "PREVENT-001",
      "rule_name": "No Force Push",
      "severity": "error",
      "message": "git push --force violates guardrail: NO FORCE PUSH",
      "category": "git_operation",
      "action": "halt",
      "suggested_alternative": "Use git push --force-with-lease instead",
      "documentation_uri": "guardrail://docs/AGENT_GUARDRAILS"
    }
  ],
  "meta": {
    "checked_at": "2026-02-07T10:30:00Z",
    "rules_evaluated": 15,
    "duration_ms": 12
  }
}
```

**Severity Actions:**

| Severity | Action | Client Behavior |
|----------|--------|-----------------|
| error | halt | MUST halt operation |
| warning | confirm | SHOULD show confirmation dialog |
| info | log | MAY log for awareness |

### Error Response Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Invalid guardrail parameters",
    "data": {
      "guardrail_error": {
        "type": "validation_error",
        "code": "INVALID_SESSION",
        "message": "Session token expired or invalid",
        "suggestion": "Call guardrail_init_session to create new session"
      }
    }
  }
}
```

**Error Codes:**

| Code | Meaning | HTTP Equivalent |
|------|---------|-----------------|
| INVALID_SESSION | Session token invalid/expired | 401 |
| INVALID_API_KEY | API key invalid | 401 |
| RATE_LIMITED | Too many requests | 429 |
| RULE_VIOLATION | Guardrail violation found | 403 |
| INVALID_ARGUMENT | Bad parameters | 400 |
| INTERNAL_ERROR | Server error | 500 |

### Resources

| Resource | Description |
|----------|-------------|
| `guardrail://docs/{slug}` | Document content (markdown) |
| `guardrail://docs/search?q={query}` | Full-text search results |
| `guardrail://rules` | All prevention rules |
| `guardrail://rules/{rule_id}` | Specific rule |
| `guardrail://rules/active` | Active rules only |
| `guardrail://failures?status={status}&limit={n}` | Failure registry (paginated) |
| `guardrail://projects/{slug}` | Project configuration |
| `guardrail://projects/{slug}/active-rules` | Rules for project |
| `guardrail://quick-reference` | Quick reference card |
| `guardrail://health` | Health status |
| `guardrail://capabilities` | Server capabilities |

---

## IDE Integration

IDE clients connect via HTTP API (not MCP protocol) for real-time validation.

### IDE API Endpoints (Port 8081)

**Health Check**
```
GET /ide/health
Authorization: Bearer {MCP_API_KEY}
```

**Validate File (on save)**
```
POST /ide/validate/file
Authorization: Bearer {MCP_API_KEY}
Content-Type: application/json

{
  "file_path": "src/main.go",
  "content": "package main\n...",
  "language": "go",
  "project_slug": "my-project"
}
```

Response:
```json
{
  "valid": false,
  "violations": [
    {
      "rule_id": "PREVENT-001",
      "line": 42,
      "column": 15,
      "severity": "error",
      "message": "Hardcoded secret detected",
      "suggestion": "Use environment variable"
    }
  ]
}
```

**Validate Selection (real-time)**
```
POST /ide/validate/selection
Authorization: Bearer {MCP_API_KEY}

{
  "code": "rm -rf /",
  "language": "bash",
  "context": "cleanup script"
}
```

**Get Active Rules for Project**
```
GET /ide/rules?project=my-project
Authorization: Bearer {MCP_API_KEY}
```

**Get Quick Reference**
```
GET /ide/quick-reference
Authorization: Bearer {MCP_API_KEY}
```

### IDE Extension Architecture

**VS Code Extension**
```typescript
// vscode-guardrail/src/extension.ts
export function activate(context: vscode.ExtensionContext) {
    const validator = new GuardrailValidator({
        serverUrl: config.get('guardrail.serverUrl'),
        apiKey: config.get('guardrail.apiKey'),
        projectSlug: config.get('guardrail.project')
    });

    // Validate on save
    vscode.workspace.onDidSaveTextDocument(doc => {
        validator.validateFile(doc);
    });

    // Real-time diagnostics
    vscode.languages.registerCodeActionsProvider('*', {
        provideCodeActions: (doc, range) => {
            return validator.validateSelection(doc.getText(range));
        }
    });
}
```

**JetBrains Plugin**
```kotlin
// GuardrailInspectionTool.kt
class GuardrailInspectionTool : LocalInspectionTool() {
    override fun checkFile(
        file: PsiFile,
        manager: InspectionManager,
        isOnTheFly: Boolean
    ): Array<ProblemDescriptor> {
        val violations = GuardrailClient.validateFile(
            path = file.virtualFile.path,
            content = file.text,
            language = file.language.id
        )
        return violations.map { v ->
            manager.createProblemDescriptor(
                file.findElementAt(v.offset),
                v.message,
                QuickFixes.get(v.ruleId),
                v.severity.toHighlightSeverity(),
                isOnTheFly
            )
        }.toTypedArray()
    }
}
```

**Vim/Neovim (via LSP or HTTP)**
```lua
-- nvim-guardrail/lua/guardrail.lua
local M = {}

M.validate = function()
    local buf = vim.api.nvim_get_current_buf()
    local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
    local content = table.concat(lines, '\n')

    local response = http.post('http://guardrail-server:8081/ide/validate/file', {
        file_path = vim.fn.expand('%:p'),
        content = content,
        language = vim.bo.filetype
    })

    -- Populate quickfix list
    vim.fn.setqflist(response.violations)
    vim.cmd('copen')
end

return M
```

### WebSocket for Real-time

For IDEs requiring real-time validation:
```
WS /ide/ws/validate
Authorization: Bearer {MCP_API_KEY}

Messages:
- Client -> Server: { "type": "validate", "code": "...", "language": "go" }
- Server -> Client: { "type": "result", "violations": [...] }
```

---

## Web UI REST API

### Authentication

All endpoints require `Authorization: Bearer {WEB_API_KEY}` header.

### Documents

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/documents` | List documents (paginated) |
| GET | `/api/documents/:id` | Get document |
| PUT | `/api/documents/:id` | Update document |
| GET | `/api/documents/search?q={query}` | Full-text search (parameterized) |
| GET | `/api/documents/category/{category}` | Filter by category |

**Search Implementation (SQL Injection Safe):**
```go
// internal/database/documents.go

func (db *DB) SearchDocuments(ctx context.Context, query string, limit int) ([]Document, error) {
    // Validate and sanitize query first
    safeQuery, err := sanitizeSearchQuery(query)
    if err != nil {
        return nil, fmt.Errorf("invalid search query: %w", err)
    }

    // Use parameterized query - NEVER concatenate
    rows, err := db.QueryContext(ctx, `
        SELECT id, slug, title, content, category, path
        FROM documents
        WHERE search_vector @@ plainto_tsquery('english', $1)
        ORDER BY ts_rank(search_vector, plainto_tsquery('english', $1)) DESC
        LIMIT $2
    `, safeQuery, limit)
    if err != nil {
        return nil, err
    }
    defer rows.Close()
    // ... scan rows
}

func sanitizeSearchQuery(query string) (string, error) {
    // Limit length
    if len(query) > 200 {
        return "", fmt.Errorf("query too long (max 200 chars)")
    }

    // Remove dangerous characters - only allow safe FTS operators
    // Allow: alphanumeric, spaces, - (negation), * (prefix), " (phrase), & | (AND/OR)
    safe := regexp.MustCompile(`[^a-zA-Z0-9\s\-\*"&\|]`)
    cleaned := safe.ReplaceAllString(query, "")

    // Prevent FTS operator injection
    if strings.Count(cleaned, "(") != strings.Count(cleaned, ")") {
        return "", fmt.Errorf("mismatched parentheses")
    }

    return cleaned, nil
}
```

### Rules

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/rules` | List rules |
| GET | `/api/rules/:id` | Get rule |
| POST | `/api/rules` | Create rule |
| PUT | `/api/rules/:id` | Update rule |
| DELETE | `/api/rules/:id` | Delete rule |
| POST | `/api/rules/:id/toggle` | Enable/disable rule |
| POST | `/api/rules/:id/test` | Test rule against input |

### Projects

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/projects` | List projects |
| GET | `/api/projects/:slug` | Get project |
| POST | `/api/projects` | Create project |
| PUT | `/api/projects/:slug` | Update project |
| DELETE | `/api/projects/:slug` | Delete project |

### Failures

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/failures` | List failures (paginated) |
| GET | `/api/failures/:id` | Get failure |
| POST | `/api/failures` | Log new failure |
| PUT | `/api/failures/:id` | Update failure status |

### System

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health/live` | Liveness probe (process check) |
| GET | `/health/ready` | Readiness probe (DB + Redis check) |
| GET | `/metrics` | Prometheus metrics |
| GET | `/api/stats` | Usage statistics |
| POST | `/api/ingest` | Trigger ingest job |

### Health Check Implementation

```go
// internal/web/handlers/health.go

func HealthLive(c echo.Context) error {
    // Simple process check
    return c.JSON(200, map[string]string{"status": "alive"})
}

func HealthReady(c echo.Context) error {
    // Check DB and Redis connectivity
    if err := db.Ping(); err != nil {
        return c.JSON(503, map[string]string{"status": "not ready", "reason": "database"})
    }
    if err := redis.Ping(); err != nil {
        return c.JSON(503, map[string]string{"status": "not ready", "reason": "cache"})
    }
    return c.JSON(200, map[string]string{"status": "ready"})
}
```

### Graceful Shutdown

```go
// cmd/server/main.go

func main() {
    srv := &http.Server{
        Addr:         ":8080",
        Handler:      e,
        ReadTimeout:  5 * time.Second,
        WriteTimeout: 10 * time.Second,
        IdleTimeout:  120 * time.Second,
    }

    // Start server in goroutine
    go func() { srv.ListenAndServe() }()

    // Wait for shutdown signal
    quit := make(chan os.Signal, 1)
    signal.Notify(quit, syscall.SIGTERM, syscall.SIGINT)
    <-quit

    // Graceful shutdown with timeout
    ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
    defer cancel()
    srv.Shutdown(ctx)
}
```

### CSRF Protection

- All state-changing operations require CSRF token
- Token provided in `X-CSRF-Token` header or cookie
- Double-submit cookie pattern

### Content Security Policy (CSP)

**Strict CSP for Web UI:**
```go
// internal/web/middleware/security.go

func SecurityHeaders() echo.MiddlewareFunc {
    return func(next echo.HandlerFunc) echo.HandlerFunc {
        return func(c echo.Context) error {
            // Strict CSP - no inline scripts allowed
            csp := []string{
                "default-src 'self'",
                "script-src 'self'",
                "style-src 'self' 'unsafe-inline'", // Allow inline styles (UI frameworks)
                "img-src 'self' data:",
                "font-src 'self'",
                "connect-src 'self'",
                "frame-ancestors 'none'",
                "base-uri 'self'",
                "form-action 'self'",
            }
            c.Response().Header().Set("Content-Security-Policy", strings.Join(csp, "; "))

            // Additional security headers
            c.Response().Header().Set("X-Content-Type-Options", "nosniff")
            c.Response().Header().Set("X-Frame-Options", "DENY")
            c.Response().Header().Set("X-XSS-Protection", "1; mode=block")
            c.Response().Header().Set("Referrer-Policy", "strict-origin-when-cross-origin")
            c.Response().Header().Set("Permissions-Policy", "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()")

            return next(c)
        }
    }
}
```

**Nonce-Based CSP for Inline Scripts:**
```go
func CSPWithNonce() echo.MiddlewareFunc {
    return func(next echo.HandlerFunc) echo.HandlerFunc {
        return func(c echo.Context) error {
            // Generate nonce for this request
            nonce := generateNonce()
            c.Set("csp_nonce", nonce)

            csp := fmt.Sprintf("script-src 'nonce-%s' 'self'; style-src 'self' 'unsafe-inline'", nonce)
            c.Response().Header().Set("Content-Security-Policy", csp)

            return next(c)
        }
    }
}
```

---

## Security Requirements

### API Authentication

```go
// internal/web/middleware/auth.go

// Key types: "mcp", "ide" (web UI is internal, no key needed)
func APIKeyAuth(allowedTypes ...string) echo.MiddlewareFunc {
    return func(next echo.HandlerFunc) echo.HandlerFunc {
        return func(c echo.Context) error {
            auth := c.Request().Header.Get("Authorization")
            if auth == "" {
                return echo.NewHTTPError(401, "Missing authorization")
            }

            token := strings.TrimPrefix(auth, "Bearer ")
            keyType := validateAPIKey(token)

            if keyType == "" {
                return echo.NewHTTPError(401, "Invalid API key")
            }

            // Check if key type is allowed for this endpoint
            if !slices.Contains(allowedTypes, keyType) {
                return echo.NewHTTPError(403, "API key type not allowed for this endpoint")
            }

            // Log hashed key only
            slog.Info("API request",
                "key_type", keyType,
                "key_hash", hashKey(token),
                "path", c.Path())

            c.Set("key_type", keyType)
            return next(c)
        }
    }
}

func hashKey(key string) string {
    h := sha256.Sum256([]byte(key))
    return hex.EncodeToString(h[:8])
}
```

### Rate Limiting (Distributed with Redis)

```go
// internal/web/middleware/ratelimit.go

// In-memory rate limiting doesn't work across multiple instances
// Use Redis for distributed rate limiting

type DistributedLimiter struct {
    redis  *redis.Client
    window time.Duration
}

func NewDistributedLimiter(redis *redis.Client) *DistributedLimiter {
    return &DistributedLimiter{
        redis:  redis,
        window: time.Minute,
    }
}

func (dl *DistributedLimiter) Allow(ctx context.Context, key string, limit int) bool {
    // Sliding window counter in Redis
    now := time.Now().Unix()
    windowKey := fmt.Sprintf("ratelimit:%s:%d", key, now/60)

    pipe := dl.redis.Pipeline()
    incr := pipe.Incr(ctx, windowKey)
    pipe.Expire(ctx, windowKey, dl.window)
    _, err := pipe.Exec(ctx)
    if err != nil {
        // Fail open on Redis error
        return true
    }

    return incr.Val() <= int64(limit)
}

// Usage per endpoint type:
// MCP: 1000/min per API key
// IDE: 500/min per API key
// Session: 100/min per session token
```

### SSE Connection Limits

```go
// internal/mcp/server.go

const (
    MaxSSEConnections     = 100  // Per instance
    MaxConnectionsPerKey  = 10   // Per API key
)

var (
    activeConnections   = make(map[string]int) // key -> count
    connectionsMutex    sync.RWMutex
)

func CanAcceptConnection(apiKeyHash string) bool {
    connectionsMutex.Lock()
    defer connectionsMutex.Unlock()

    total := 0
    for _, count := range activeConnections {
        total += count
    }

    if total >= MaxSSEConnections {
        return false
    }

    if activeConnections[apiKeyHash] >= MaxConnectionsPerKey {
        return false
    }

    activeConnections[apiKeyHash]++
    return true
}

// For horizontal scaling, use Redis to track global connections:
// Redis key: "sse:connections:{instance_id}" -> count
// Global limit checked before accepting
```

### Database Connection Pool

```go
// internal/database/postgres.go - Connection Pool
func NewDB(cfg *config.Config) (*sql.DB, error) {
    db, err := sql.Open("pgx", connString)
    if err != nil {
        return nil, err
    }

    // Scale based on CPU cores - need 50+ for 1000 sessions
    maxConns := 4 * runtime.NumCPU()
    if maxConns < 50 {
        maxConns = 50
    }

    db.SetMaxOpenConns(maxConns)           // 50+ for 1000 sessions
    db.SetMaxIdleConns(maxConns / 2)       // 25 idle
    db.SetConnMaxLifetime(15 * time.Minute) // Longer for stability
    db.SetConnMaxIdleTime(5 * time.Minute)

    return db, nil
}
```

### Circuit Breakers

```go
// internal/circuitbreaker/breaker.go

package circuitbreaker

import "github.com/sony/gobreaker"

var DBBreaker = gobreaker.NewCircuitBreaker(gobreaker.Settings{
    Name:        "database",
    MaxRequests: 3,                // Half-open state probe count
    Interval:    10 * time.Second, // Statistical window
    Timeout:     30 * time.Second, // Request timeout
    ReadyToTrip: func(counts gobreaker.Counts) bool {
        failureRatio := float64(counts.TotalFailures) / float64(counts.Requests)
        return counts.Requests >= 3 && failureRatio >= 0.6
    },
})

var RedisBreaker = gobreaker.NewCircuitBreaker(gobreaker.Settings{
    Name:     "redis",
    Interval: 10 * time.Second,
    Timeout:  5 * time.Second,
})
```

### CSRF Protection

```go
// internal/web/middleware/csrf.go

func CSRF() echo.MiddlewareFunc {
    return csrfMiddleware(csrf.Config{
        TokenLookup: "header:X-CSRF-Token",
        CookieName: "csrf_token",
        CookieSameSite: http.SameSiteStrictMode,
    })
}
```

### Container Security

```dockerfile
# deploy/Dockerfile

FROM golang:1.23-alpine AS builder
WORKDIR /app
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -o server ./cmd/server

FROM gcr.io/distroless/static:nonroot
USER 65532:65532
COPY --from=builder --chown=65532:65532 /app/server /server
EXPOSE 8080 8081
ENTRYPOINT ["/server"]
```

### Redis Security Configuration

**Redis AUTH Password:**
```yaml
# deploy/podman-compose.yml - Redis with AUTH
services:
  redis:
    image: redis:7-alpine
    command: redis-server --requirepass ${REDIS_PASSWORD}
    environment:
      - REDIS_PASSWORD=${REDIS_PASSWORD}
    networks:
      - backend
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
```

**Redis Client Configuration:**
```go
// internal/cache/redis.go

func NewRedisClient(cfg *config.Config) *redis.Client {
    opts := &redis.Options{
        Addr:     fmt.Sprintf("%s:%d", cfg.RedisHost, cfg.RedisPort),
        Password: cfg.RedisPassword, // AUTH password
        DB:       0,
        PoolSize: 20,
        MinIdleConns: 5,
        MaxRetries: 3,
        ReadTimeout:  3 * time.Second,
        WriteTimeout: 3 * time.Second,
    }

    // TLS for production
    if cfg.RedisUseTLS {
        opts.TLSConfig = &tls.Config{
            MinVersion: tls.VersionTLS12,
            ServerName: cfg.RedisHost,
        }
    }

    return redis.NewClient(opts)
}
```

**Configuration:**
```bash
# .env
REDIS_HOST=redis
REDIS_PORT=6379
REDIS_PASSWORD=     # Generate strong password
REDIS_USE_TLS=false # Set true in production
```

### podman-compose Security

```yaml
# deploy/podman-compose.yml
version: "3.8"

services:
  redis:
    image: redis:7-alpine
    command: redis-server --requirepass ${REDIS_PASSWORD}
    environment:
      - REDIS_PASSWORD=${REDIS_PASSWORD}
    networks:
      - backend
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    # Redis data persistence
    volumes:
      - redis_data:/data

  postgres:
    image: postgres:16-alpine
    environment:
      - POSTGRES_PASSWORD=${DB_PASSWORD}
    networks:
      - backend
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    volumes:
      - pg_data:/var/lib/postgresql/data

  mcp-server:
    image: guardrail-mcp:latest
    read_only: true
    user: "65532:65532"
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    networks:
      - frontend
      - backend
    ports:
      - "127.0.0.1:8080:8080"  # MCP - localhost only
      - "127.0.0.1:8081:8081"  # Web UI - localhost only
    environment:
      - DB_SSLMODE=require
      - DB_PASSWORD=${DB_PASSWORD}
      - REDIS_PASSWORD=${REDIS_PASSWORD}
      - MCP_API_KEY=${MCP_API_KEY}
      - IDE_API_KEY=${IDE_API_KEY}
      # Note: No WEB_API_KEY - Web UI is internal
    tmpfs:
      - /tmp:noexec,nosuid,size=100m

networks:
  frontend:
    internal: false
  backend:
    internal: true

volumes:
  pg_data:
  redis_data:
```

**Note:** Ports bound to `127.0.0.1` only. Use reverse proxy (nginx/traefik) for external access.

### Input Validation

```go
// internal/validation/safe_regex.go

func SafeRegex(pattern string, input string, timeout time.Duration) (bool, error) {
    resultChan := make(chan bool, 1)

    go func() {
        re, err := regexp.Compile(pattern)
        if err != nil {
            resultChan <- false
            return
        }
        resultChan <- re.MatchString(input)
    }()

    select {
    case result := <-resultChan:
        return result, nil
    case <-time.After(timeout):
        return false, fmt.Errorf("regex timeout - possible ReDoS")
    }
}
```

### Secrets Scanning

**Prevents accidental secret exposure in documents and user input:**

```go
// internal/security/secrets_scanner.go

package security

import (
    "regexp"
    "strings"
)

// SecretPattern defines a detectable secret type
type SecretPattern struct {
    Name        string
    Pattern     *regexp.Regexp
    Description string
}

var secretPatterns = []SecretPattern{
    {
        Name:        "AWS Access Key ID",
        Pattern:     regexp.MustCompile(`AKIA[0-9A-Z]{16}`),
        Description: "AWS IAM access key",
    },
    {
        Name:        "AWS Secret Key",
        Pattern:     regexp.MustCompile(`['"\s][0-9a-zA-Z/+]{40}['"\s]`),
        Description: "Potential AWS secret key",
    },
    {
        Name:        "Private Key",
        Pattern:     regexp.MustCompile(`-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----`),
        Description: "PEM private key",
    },
    {
        Name:        "GitHub Token",
        Pattern:     regexp.MustCompile(`gh[pousr]_[A-Za-z0-9_]{36,}`),
        Description: "GitHub personal/token",
    },
    {
        Name:        "Slack Token",
        Pattern:     regexp.MustCompile(`xox[baprs]-[0-9a-zA-Z-]+`),
        Description: "Slack API token",
    },
    {
        Name:        "Generic API Key",
        Pattern:     regexp.MustCompile(`(?i)(api[_-]?key|apikey)\s*[=:]\s*['"\s][a-z0-9_\-]{16,}['"\s]`),
        Description: "Generic API key pattern",
    },
    {
        Name:        "JWT Token",
        Pattern:     regexp.MustCompile(`eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*`),
        Description: "JSON Web Token",
    },
}

// ScanResult represents a detected secret
type ScanResult struct {
    Pattern     string `json:"pattern"`
    Line        int    `json:"line"`
    Column      int    `json:"column"`
    Match       string `json:"match"`
    Description string `json:"description"`
}

// ScanContent checks text for embedded secrets
func ScanContent(content string) []ScanResult {
    var results []ScanResult
    lines := strings.Split(content, "\n")

    for lineNum, line := range lines {
        for _, pattern := range secretPatterns {
            matches := pattern.Pattern.FindAllStringIndex(line, -1)
            for _, match := range matches {
                results = append(results, ScanResult{
                    Pattern:     pattern.Name,
                    Line:        lineNum + 1,
                    Column:      match[0] + 1,
                    Match:       maskSecret(line[match[0]:match[1]]),
                    Description: pattern.Description,
                })
            }
        }
    }

    return results
}

// maskSecret hides most of the secret for display
func maskSecret(secret string) string {
    if len(secret) <= 8 {
        return "****"
    }
    return secret[:4] + "****" + secret[len(secret)-4:]
}

// ValidateDocument checks document content before storage
func ValidateDocument(content string) error {
    secrets := ScanContent(content)
    if len(secrets) > 0 {
        return fmt.Errorf("potential secrets detected: %d findings", len(secrets))
    }
    return nil
}
```

**Usage in Web UI handlers:**
```go
// internal/web/handlers/documents.go

func UpdateDocument(c echo.Context) error {
    var req UpdateDocumentRequest
    if err := c.Bind(&req); err != nil {
        return err
    }

    // Scan for secrets before saving
    if findings := security.ScanContent(req.Content); len(findings) > 0 {
        return c.JSON(400, map[string]interface{}{
            "error": "Potential secrets detected in content",
            "findings": findings,
        })
    }

    // Continue with update...
}
```

---

## Deployment

### Phase 1: Build

```bash
make build-container
# Produces: guardrail-mcp:latest
```

### Phase 2: Deploy Infrastructure

```bash
# Copy .env with real values to target server
scp .env user@target:/opt/guardrail-mcp/
scp deploy/podman-compose.yml user@target:/opt/guardrail-mcp/

# Start services
ssh user@target "cd /opt/guardrail-mcp && podman-compose up -d"
```

### Phase 3: Run Migrations

```bash
# Apply database migrations
podman run --rm --env-file .env guardrail-mcp:latest \
    /usr/local/bin/migrate -path /migrations -database "postgres://..." up
```

### Phase 4: One-Time Ingest (Migration)

```bash
# Run ingest once to migrate MD files to database
# After this, Web UI is the source of truth
podman run --rm --env-file .env \
    -v /path/to/repo:/data/repo:ro \
    guardrail-mcp:latest \
    /usr/local/bin/ingest --repo /data/repo

# Verify ingest
podman exec guardrail-mcp-server psql -c "SELECT COUNT(*) FROM documents;"
```

---

## Configuration

**Required in `.env`:**

```bash
# Security (generate strong values)
MCP_API_KEY=        # For TUI clients (Claude Code, OpenCode)
IDE_API_KEY=        # For IDE extensions (VS Code, JetBrains)
JWT_SECRET=         # Min 32 bytes, high entropy (openssl rand -hex 32)
# Note: Web UI requires no key (runs in same container)

# Database (use SSL in production)
DB_HOST=postgres
DB_PORT=5432
DB_NAME=guardrails
DB_USER=guardrails
DB_PASSWORD=
DB_SSLMODE=require

# Redis
REDIS_HOST=redis
REDIS_PORT=6379
REDIS_PASSWORD=     # Redis AUTH password
REDIS_USE_TLS=false # Enable in production

# Server
MCP_PORT=8080
WEB_PORT=8081
LOG_LEVEL=info
REQUEST_TIMEOUT=30s

# JWT Configuration
JWT_ISSUER=guardrail-mcp
JWT_EXPIRY=15m
JWT_ROTATION_HOURS=168  # 7 days

# Rate Limiting
MCP_RATE_LIMIT=1000     # Requests per minute
IDE_RATE_LIMIT=500
SESSION_RATE_LIMIT=100

# Cache
CACHE_TTL_RULES=5m
CACHE_TTL_DOCS=10m
CACHE_TTL_SEARCH=2m
```

---

## Cache Invalidation Strategy

**Write-Through with TTL:**

```go
// internal/cache/cache_manager.go

package cache

import (
    "context"
    "fmt"
    "time"

    "github.com/redis/go-redis/v9"
)

// CacheManager handles cache operations with invalidation
type CacheManager struct {
    client *redis.Client
    ttl    time.Duration
}

// Cache keys
const (
    KeyActiveRules    = "guardrail:rules:active"
    KeyDocument       = "guardrail:doc:%s"      // Format with slug
    KeyRule           = "guardrail:rule:%s"     // Format with rule_id
    KeyProjectContext = "guardrail:project:%s"  // Format with slug
    KeySearchResults  = "guardrail:search:%s"   // Format with query hash
    KeySession        = "guardrail:session:%s"  // Format with token
)

// GetActiveRules retrieves cached active rules
func (cm *CacheManager) GetActiveRules(ctx context.Context) ([]byte, error) {
    return cm.client.Get(ctx, KeyActiveRules).Bytes()
}

// SetActiveRules caches active rules
func (cm *CacheManager) SetActiveRules(ctx context.Context, data []byte) error {
    return cm.client.Set(ctx, KeyActiveRules, data, cm.ttl).Err()
}

// InvalidateOnRuleChange clears rule-related caches when rules are modified
func (cm *CacheManager) InvalidateOnRuleChange(ctx context.Context, ruleID string) error {
    pipe := cm.client.Pipeline()

    // Delete specific rule cache
    pipe.Del(ctx, fmt.Sprintf(KeyRule, ruleID))

    // Delete active rules list (will be rebuilt on next request)
    pipe.Del(ctx, KeyActiveRules)

    // Delete all search result caches (they may include this rule)
    pipe.Eval(ctx, `
        local keys = redis.call('keys', 'guardrail:search:*')
        for _, key in ipairs(keys) do
            redis.call('del', key)
        end
        return #keys
    `, []string{})

    _, err := pipe.Exec(ctx)
    return err
}

// InvalidateOnDocumentChange clears doc-related caches
func (cm *CacheManager) InvalidateOnDocumentChange(ctx context.Context, slug string) error {
    pipe := cm.client.Pipeline()

    // Delete specific document cache
    pipe.Del(ctx, fmt.Sprintf(KeyDocument, slug))

    // Delete search caches that might reference this doc
    pipe.Eval(ctx, `
        local keys = redis.call('keys', 'guardrail:search:*')
        for _, key in ipairs(keys) do
            redis.call('del', key)
        end
        return #keys
    `, []string{})

    _, err := pipe.Exec(ctx)
    return err
}

// InvalidateOnProjectChange clears project caches
func (cm *CacheManager) InvalidateOnProjectChange(ctx context.Context, slug string) error {
    return cm.client.Del(ctx, fmt.Sprintf(KeyProjectContext, slug)).Err()
}

// WarmCache pre-populates cache after invalidation
func (cm *CacheManager) WarmCache(ctx context.Context, db *database.DB) error {
    // Pre-load active rules
    rules, err := db.GetActiveRules(ctx)
    if err != nil {
        return err
    }

    data, _ := json.Marshal(rules)
    if err := cm.SetActiveRules(ctx, data); err != nil {
        return err
    }

    slog.Info("Cache warmed", "rules_count", len(rules))
    return nil
}
```

**Usage in Handlers:**
```go
// After updating a rule
if err := cacheMgr.InvalidateOnRuleChange(ctx, ruleID); err != nil {
    slog.Warn("Cache invalidation failed", "error", err)
}

// Trigger cache warming asynchronously
go cacheMgr.WarmCache(ctx, db)
```

**Cache TTL Strategy:**

| Data Type | TTL | Invalidation |
|-----------|-----|--------------|
| Active Rules | 5 minutes | On rule change |
| Documents | 10 minutes | On doc update |
| Search Results | 2 minutes | On any content change |
| Sessions | 15 minutes | On logout/expire |
| Project Context | 30 minutes | On project update |

---

## Horizontal Scaling

**Architecture for Multiple Instances:**

```
                    ┌──────────────────┐
                    │  Load Balancer   │
                    │   (nginx/haproxy)│
                    └────────┬─────────┘
                             │
           ┌─────────────────┼─────────────────┐
           │                 │                 │
    ┌──────▼──────┐   ┌──────▼──────┐   ┌──────▼──────┐
    │  MCP Server │   │  MCP Server │   │  MCP Server │
    │  Instance 1 │   │  Instance 2 │   │  Instance N │
    │  (Port 8080)│   │  (Port 8080)│   │  (Port 8080)│
    └──────┬──────┘   └──────┬──────┘   └──────┬──────┘
           │                 │                 │
           └─────────────────┼─────────────────┘
                             │
           ┌─────────────────┼─────────────────┐
           │                 │                 │
    ┌──────▼──────┐   ┌──────▼──────┐   ┌──────▼──────┐
    │   Redis     │   │  PostgreSQL │   │   Redis     │
    │  (Shared    │   │  (Shared    │   │  (Sentinel  │
    │   Cache)    │   │   Storage)  │   │   HA)       │
    └─────────────┘   └─────────────┘   └─────────────┘
```

**Scaling Considerations:**

1. **Stateless Instances**: MCP server instances are stateless
   - Sessions stored in Redis (shared across instances)
   - SSE connections handled by individual instances
   - Sticky sessions NOT required

2. **Redis Cluster/Sentinel**: For HA caching
   - Master-replica for read scaling
   - Sentinel for automatic failover
   - Partition SSE connection counters across instances

3. **Database**: PostgreSQL read replicas
   - Write operations go to primary
   - Read operations can use replicas
   - Connection pool per instance

4. **WebSocket/SSE Limitations:**
   - SSE connections are tied to specific instance
   - For true horizontal scaling, use Redis Pub/Sub for cross-instance messaging
   - Or use sticky sessions in load balancer (simpler)

**Redis Pub/Sub for Cross-Instance Coordination:**
```go
// For cache invalidation across instances
func (cm *CacheManager) SubscribeToInvalidations(ctx context.Context) {
    pubsub := cm.client.Subscribe(ctx, "cache:invalidations")
    defer pubsub.Close()

    ch := pubsub.Channel()
    for msg := range ch {
        // Handle invalidation message from other instances
        var inv InvalidationMessage
        json.Unmarshal([]byte(msg.Payload), &inv)
        cm.handleRemoteInvalidation(ctx, inv)
    }
}

func (cm *CacheManager) BroadcastInvalidation(ctx context.Context, inv InvalidationMessage) {
    data, _ := json.Marshal(inv)
    cm.client.Publish(ctx, "cache:invalidations", data)
}
```

---

## Observability

### Prometheus Metrics

```go
// internal/metrics/prometheus.go

package metrics

import (
    "github.com/prometheus/client_golang/prometheus"
    "github.com/prometheus/client_golang/prometheus/promhttp"
)

var (
    ValidationsTotal = prometheus.NewCounterVec(
        prometheus.CounterOpts{
            Name: "guardrail_validations_total",
            Help: "Total number of validations performed",
        },
        []string{"tool", "result"}, // result: allowed, denied, error
    )

    ValidationDuration = prometheus.NewHistogramVec(
        prometheus.HistogramOpts{
            Name:    "guardrail_validation_duration_seconds",
            Help:    "Validation request duration",
            Buckets: []float64{0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1},
        },
        []string{"tool"},
    )

    ActiveSessions = prometheus.NewGauge(
        prometheus.GaugeOpts{
            Name: "guardrail_active_sessions",
            Help: "Number of active MCP sessions",
        },
    )
)

func init() {
    prometheus.MustRegister(ValidationsTotal, ValidationDuration, ActiveSessions)
}

// Handler exposes /metrics endpoint
func Handler() http.Handler {
    return promhttp.Handler()
}
```

### Structured Logging

```go
// All log entries include:
// - timestamp (RFC3339)
// - level (debug, info, warn, error)
// - component (mcp, web, database, cache)
// - trace_id (for request correlation)
// - key_hash (hashed API key for audit, never full key)
```

### Audit Logging

**Immutable audit trail for security events:**

```go
// internal/audit/logger.go

package audit

import (
    "context"
    "encoding/json"
    "log/slog"
    "time"

    "github.com/google/uuid"
)

// EventType represents categories of audit events
type EventType string

const (
    EventAuthSuccess    EventType = "auth_success"
    EventAuthFailure    EventType = "auth_failure"
    EventValidation     EventType = "validation"
    EventRuleChange     EventType = "rule_change"
    EventDocChange      EventType = "document_change"
    EventConfigChange   EventType = "config_change"
    EventAccessDenied   EventType = "access_denied"
    EventSessionCreated EventType = "session_created"
    EventSessionExpired EventType = "session_expired"
)

// Severity represents event severity
type Severity string

const (
    SevInfo     Severity = "info"
    SevWarning  Severity = "warning"
    SevCritical Severity = "critical"
)

// Event represents a single audit event
type Event struct {
    ID          string                 `json:"id"`
    Timestamp   time.Time              `json:"timestamp"`
    Type        EventType              `json:"type"`
    Severity    Severity               `json:"severity"`
    Actor       string                 `json:"actor"`        // Hashed API key or user ID
    Action      string                 `json:"action"`       // What was done
    Resource    string                 `json:"resource"`     // What was affected
    Status      string                 `json:"status"`       // success, failure
    Details     map[string]interface{} `json:"details"`      // Additional context
    ClientIP    string                 `json:"client_ip"`
    UserAgent   string                 `json:"user_agent"`
    RequestID   string                 `json:"request_id"`
}

// Logger handles audit event recording
type Logger struct {
    backend chan Event
}

// NewLogger creates an audit logger
func NewLogger(bufferSize int) *Logger {
    l := &Logger{
        backend: make(chan Event, bufferSize),
    }
    go l.process()
    return l
}

// Log records an audit event
func (l *Logger) Log(ctx context.Context, event Event) {
    event.ID = uuid.New().String()
    event.Timestamp = time.Now().UTC()

    // Extract request context
    if reqID := ctx.Value("request_id"); reqID != nil {
        event.RequestID = reqID.(string)
    }

    select {
    case l.backend <- event:
    default:
        // Buffer full - log to stderr and continue
        slog.Error("audit buffer full, dropping event", "type", event.Type)
    }
}

// process writes events to persistent storage
func (l *Logger) process() {
    for event := range l.backend {
        // Write to structured log (forward to SIEM if configured)
        data, _ := json.Marshal(event)
        slog.Info("AUDIT", "event", string(data))

        // TODO: Write to database for long-term storage
        // This enables querying audit history via Web UI
    }
}

// Convenience methods
func (l *Logger) LogAuth(ctx context.Context, success bool, actor, reason string) {
    eventType := EventAuthSuccess
    severity := SevInfo
    if !success {
        eventType = EventAuthFailure
        severity = SevWarning
    }

    l.Log(ctx, Event{
        Type:     eventType,
        Severity: severity,
        Actor:    actor,
        Action:   "authenticate",
        Status:   map[bool]string{true: "success", false: "failure"}[success],
        Details:  map[string]interface{}{"reason": reason},
    })
}

func (l *Logger) LogValidation(ctx context.Context, actor, tool string, allowed bool, violations int) {
    l.Log(ctx, Event{
        Type:     EventValidation,
        Severity: SevInfo,
        Actor:    actor,
        Action:   "validate",
        Resource: tool,
        Status:   map[bool]string{true: "allowed", false: "denied"}[allowed],
        Details: map[string]interface{}{
            "violations": violations,
        },
    })
}

func (l *Logger) LogRuleChange(ctx context.Context, actor, ruleID, action string) {
    l.Log(ctx, Event{
        Type:     EventRuleChange,
        Severity: SevCritical, // Rule changes are security-critical
        Actor:    actor,
        Action:   action, // create, update, delete, toggle
        Resource: ruleID,
        Status:   "success",
    })
}
```

**Database Schema for Audit Events:**
```sql
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id VARCHAR(50) NOT NULL,
    timestamp TIMESTAMP NOT NULL DEFAULT NOW(),
    event_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL,
    actor VARCHAR(64) NOT NULL,  -- Hashed identifier
    action VARCHAR(50) NOT NULL,
    resource VARCHAR(255),
    status VARCHAR(20) NOT NULL,
    details JSONB DEFAULT '{}',
    client_ip INET,
    request_id VARCHAR(50),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (timestamp);

-- Indexes for efficient querying
CREATE INDEX idx_audit_time ON audit_log(timestamp DESC);
CREATE INDEX idx_audit_actor ON audit_log(actor);
CREATE INDEX idx_audit_type ON audit_log(event_type);
CREATE INDEX idx_audit_resource ON audit_log(resource);

-- Retention: Partition by month, drop old partitions after 1 year
```

**Audit Event Retention:**
- Hot storage (PostgreSQL): 30 days for Web UI queries
- Cold storage (JSONL files): 1 year for compliance
- Archive to S3/Glacier after 1 year if needed

---

## Security Checklist

### Authentication & Authorization
- [x] JWT implementation with 15-minute expiration (MCP)
- [x] JWT secret validation (32+ bytes, entropy check)
- [x] API key authentication for external endpoints (MCP, IDE)
- [x] Web UI internal communication (no key needed)
- [x] API key masking in logs (hash only)
- [x] Separate MCP and IDE API keys
- [x] Key rotation mechanism

### Network Security
- [x] DB_SSLMODE=require enforced
- [x] Internal network for PostgreSQL/Redis
- [x] No external exposure of backend services
- [x] Redis AUTH password configured
- [x] Redis TLS enabled for production
- [x] Web UI bound to localhost only

### Input Validation
- [x] Regex timeout protection (ReDoS prevention)
- [x] UUID validation for document IDs
- [x] Pattern validation for rule regex
- [x] SQL injection protection (parameterized queries)
- [x] Search query sanitization

### Secrets Protection
- [x] Secrets scanning in documents
- [x] Pre-commit secret detection
- [x] Masking of detected secrets in logs

### Observability
- [x] Prometheus metrics endpoint (/metrics)
- [x] RED metrics (Rate, Errors, Duration)
- [x] Active session gauge
- [x] Structured logging with trace_id
- [x] Audit logging infrastructure
- [x] Security event logging

### Operational Readiness
- [x] Liveness endpoint (/health/live)
- [x] Readiness endpoint (/health/ready)
- [x] Circuit breakers for DB and Redis
- [x] Database connection pooling (50+ max)
- [x] HTTP timeouts (5s read, 10s write)
- [x] Graceful shutdown (30s timeout)
- [x] Redis session TTL (15 min)
- [x] SSE connection limits (100 per instance)

### Web Security
- [x] CSRF protection on all state-changing endpoints
- [x] SameSite=Strict cookies
- [x] Content Security Policy headers
- [x] XSS protection via output encoding
- [x] Security headers (X-Content-Type-Options, X-Frame-Options, etc.)

### Rate Limiting
- [x] Per-API-key rate limits (MCP: 1000/min, IDE: 500/min)
- [x] Distributed rate limiting with Redis
- [x] Per-session rate limiting
- [x] Rate limit headers in responses

### Container Security
- [x] Distroless/minimal base image
- [x] Non-root user (UID 65532)
- [x] Read-only filesystem
- [x] No new privileges flag
- [x] Capability dropping
- [x] tmpfs for /tmp with noexec

### Audit & Logging
- [x] Structured audit logging
- [x] Audit event database schema
- [x] Security event logging (auth failures, etc.)
- [x] PII scrubbing in logs
- [x] Log retention policy (30 days hot, 1 year cold)

### Cache Management
- [x] Cache invalidation on write
- [x] Cache warming after invalidation
- [x] TTL strategy per data type
- [x] Cross-instance cache coordination (Pub/Sub)

---

## Files to Create

| File | Lines | Purpose |
|------|-------|---------|
| `go.mod` | 35 | Go module with dependencies |
| `cmd/server/main.go` | 100 | Server entry point |
| `cmd/ingest/main.go` | 80 | Ingest tool |
| `internal/config/config.go` | 60 | Configuration struct |
| `internal/models/*.go` | 150 | Data models |
| `internal/database/*.go` | 400 | Database layer + migrations |
| `internal/cache/redis.go` | 100 | Redis client |
| `internal/ingester/*.go` | 280 | Ingestion logic |
| `internal/guardrails/*.go` | 300 | Validation + enforcement |
| `internal/web/*.go` | 400 | HTTP handlers + middleware |
| `internal/mcp/*.go` | 350 | MCP handlers |
| `deploy/Dockerfile` | 25 | Server container |
| `deploy/Dockerfile.ingest` | 20 | Ingest job container |
| `deploy/podman-compose.yml` | 60 | Orchestration |
| `.env.example` | 50 | Config template |
| `Makefile` | 80 | Build automation |
| `README.md` | 250 | Setup guide |

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Validation p99 latency | < 50ms (cached) |
| Validation p99 latency | < 200ms (uncached) |
| Database query time | < 10ms |
| Cache hit ratio | > 90% |
| Max concurrent sessions | 1000 |
| Ingest throughput | 100 docs/min |

---

**Last Updated:** 2026-02-07 (v1.2 - Security & Scalability Review Complete)
