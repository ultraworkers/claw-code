# Guardrail MCP Server

A Model Context Protocol (MCP) server for enforcing guardrails across AI coding assistants and IDE extensions.

[![Go Implementation](https://img.shields.io/badge/Implementation-Go-blue.svg?style=flat&logo=go)](https://golang.org)
[![Version](https://img.shields.io/badge/version-v3.1.0-blue.svg)](./CHANGELOG.md)

> **Go Implementation:** All code is written in Go. Package location: `mcp-server/internal/`
> **Migration:** Python implementation deprecated as of v2.6.0. See [../docs/PYTHON_MIGRATION.md](../docs/PYTHON_MIGRATION.md).

## 🚨 Critical Deployment Information

**Deployment Status:** ✅ Successfully deployed to AI01 (0.0.0.0:8095/8096)

**Schema Validation Error Fixed:**
- Changed server name from `guardrail-mcp` to `guardrail_mcp` (line 101 in `internal/mcp/server.go`)
- This fixes the MCP framework's schema validation error that was blocking Claude Code from using the guardrail tools

**Postgres Permission Issues Fixed:**
- Added `user: "70:70"` to postgres service configuration
- Removed security constraints to allow proper container initialization

**Configuration Requirements:**
- MCP_API_KEY and IDE_API_KEY must be 32+ characters with mixed case and digits
- JWT_SECRET must be at least 32 bytes long
- JWT_ROTATION_HOURS must include 'h' unit (e.g., `168h`)
- Use localhost for container communication within pod

**Complete Deployment Guide:**
See [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md) for step-by-step deployment instructions.

## Architecture

```
Deployment host (or local VM)
|
|-- guardrail-mcp-server (app container)
|   |-- :8080 MCP SSE + JSON-RPC message endpoint
|   |-- :8081 Web UI + REST API + health + metrics
|   |-- attached networks: frontend, backend
|   |-- host bindings: 127.0.0.1:${MCP_PORT}->8080, 127.0.0.1:${WEB_PORT}->8081
|
|-- guardrail-postgres (state container)
|   |-- :5432 backend network only
|   |-- attached networks: backend
|   |-- volume: pg_data
|
|-- guardrail-redis (cache/rate-limiting container)
|   |-- :6379 backend network only
|   |-- attached networks: backend
|   |-- volume: redis_data
```

## Quick Start

### Prerequisites

- Go 1.23+
- Podman or Docker
- PostgreSQL 16 (if running without compose)
- Redis 7 (if running without compose)

### Important: Read Deployment Guide First

**Before deploying, read [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md)** - This contains critical fixes discovered during AI01 deployment that are required for the MCP server to work correctly.

Key fixes include:
- Server name must use underscores, not dashes (schema validation fix)
- Postgres must run as user 70:70 (permission fix)
- API keys must be 32+ characters with mixed case and digits
- JWT_SECRET must be at least 32 bytes
- Use pod networking for container communication

### Configuration

1. Copy `.env.example` to `.env` and fill in the values:

```bash
cp .env.example .env
# Edit .env with your values
```

2. Generate security keys:

```bash
export MCP_API_KEY=$(openssl rand -hex 32)
export IDE_API_KEY=$(openssl rand -hex 32)
export JWT_SECRET=$(openssl rand -hex 32)
export DB_PASSWORD=$(openssl rand -base64 32)
export REDIS_PASSWORD=$(openssl rand -base64 32)
```

### Database Migrations

Database migrations use golang-migrate.

```bash
# Set DATABASE_URL environment variable
export DATABASE_URL="postgresql://guardrails:password@localhost:5432/guardrails?sslmode=disable"

# Run migrations up
make migrate-up

# Run migrations down
make migrate-down
```

Migration files are located in `internal/database/migrations/`.

### Development

```bash
# Install dependencies
make deps

# Run tests
make test

# Run locally (requires PostgreSQL and Redis running and migrations applied)
make dev

# Format code
make fmt

# Run linter
make lint

# Check for vulnerabilities
make vuln
```

### Deployment

For detailed deployment instructions (recommended for production), see [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md).

**Quick Start:**

```bash
# Build container
make docker-build

# Start all services (PostgreSQL, Redis, MCP Server)
make docker-up

# View logs
make docker-logs

# Stop services
make docker-down
```

Docker-only equivalent (without Podman tooling):

```bash
# Build image
docker build -t guardrail-mcp:latest -f deploy/Dockerfile .

# Start all services from compose file
docker compose -f deploy/podman-compose.yml up -d --build

# View logs
docker compose -f deploy/podman-compose.yml logs -f

# Stop services
docker compose -f deploy/podman-compose.yml down
```

Alternative Docker compose file used by testers:

```bash
docker compose -f deploy/docker-compose.example.yml up -d --build
docker compose -f deploy/docker-compose.example.yml ps
```

## API Endpoints

### Health

- `GET /health/live` - Liveness probe
- `GET /health/ready` - Readiness probe (checks DB and Redis)
- `GET /metrics` - Prometheus metrics endpoint
- `GET /version` - Server version information

### MCP Protocol (Port 8080)

Server-Sent Events (SSE) endpoint for MCP clients.

- `GET /mcp/v1/sse` - SSE event stream endpoint
- `POST /mcp/v1/message?session_id=<session_id>` - JSON-RPC message endpoint

The `session_id` is provided by the initial SSE `endpoint` event.

### Web UI API (Port 8081)

- `GET /api/documents` - List documents (paginated)
- `GET /api/documents/:id` - Get document by ID
- `PUT /api/documents/:id` - Update document
- `GET /api/documents/search?q={query}` - Full-text search documents

- `GET /api/rules` - List prevention rules
- `GET /api/rules/:id` - Get rule by ID
- `POST /api/rules` - Create rule
- `PUT /api/rules/:id` - Update rule
- `DELETE /api/rules/:id` - Delete rule
- `PATCH /api/rules/:id` - Enable/disable rule (partial update)

- `GET /api/projects` - List projects
- `GET /api/projects/:id` - Get project by ID
- `POST /api/projects` - Create project
- `PUT /api/projects/:id` - Update project
- `DELETE /api/projects/:id` - Delete project

- `GET /api/failures` - List failure registry entries
- `GET /api/failures/:id` - Get failure by ID
- `POST /api/failures` - Create failure entry
- `PUT /api/failures/:id` - Update failure status

- `GET /api/stats` - Get system statistics
- `POST /api/ingest` - Trigger document ingestion

### IDE API (Port 8081)

- `GET /ide/health` - IDE API health check
- `POST /ide/validate/file` - Validate file content
- `POST /ide/validate/selection` - Validate code selection
- `GET /ide/rules` - Get active rules for project
- `GET /ide/quick-reference` - Get quick reference documentation

## Security Features

### Authentication & Authorization
- **API Key Authentication** - Write and IDE endpoints require valid API key (MCP_API_KEY or IDE_API_KEY)
- **Public Read-Only Web Routes** - `/api/documents*`, `/api/rules*`, and `/version` are browsable without API key
- **JWT Tokens** - Session tokens for MCP clients with 15-minute expiry
- **Hashed Key Logging** - API keys are hashed in logs for audit purposes

### Infrastructure Security
- **Redis AUTH** - Password-protected Redis connections
- **PostgreSQL SSL** - TLS support for database connections
- **Non-root Container** - Runs as UID 65532 (distroless image)
- **Read-only Filesystem** - Container root is read-only
- **Dropped Capabilities** - ALL capabilities dropped for minimal attack surface

### Application Security
- **Rate Limiting** - Per-API-key rate limiting (MCP: 1000/min, IDE: 500/min)
- **Secrets Scanning** - Automatic detection of secrets in document content (AWS keys, GitHub tokens, private keys, etc.)
- **Content Security Policy** - Strict CSP headers to prevent XSS
- **Security Headers** - X-Content-Type-Options, X-Frame-Options, X-XSS-Protection, Referrer-Policy
- **Input Validation** - UUID validation, parameterized queries to prevent SQL injection
- **Regex Timeouts** - Protection against ReDoS attacks

### Resilience Patterns
- **Circuit Breakers** - Automatic failure detection for database and Redis
- **Graceful Degradation** - Service continues operating when cache is unavailable
- **Health Checks** - Liveness and readiness probes for orchestration
- **Graceful Shutdown** - 30-second timeout for in-flight requests

## MCP Protocol

The MCP server implements the Model Context Protocol for AI assistant integration.

### MCP Tools

- `guardrail_init_session` - Initialize a validation session for a project
- `guardrail_validate_bash` - Validate bash command against forbidden patterns
- `guardrail_validate_file_edit` - Validate file edit operation
- `guardrail_validate_git_operation` - Validate git command against guardrails
- `guardrail_pre_work_check` - Run pre-work checklist from failure registry
- `guardrail_get_context` - Get guardrail context for the session's project

### MCP Resources

- `guardrail://quick-reference` - Quick reference card for guardrails
- `guardrail://rules/active` - Currently active prevention rules

### Connecting to MCP Server

```bash
# 1) Open SSE stream and capture endpoint event
curl -sN http://localhost:8080/mcp/v1/sse
# event: endpoint
# data: http://localhost:8080/mcp/v1/message?session_id=<session_id>

# 2) In another terminal, send JSON-RPC message to session-specific URL
curl -i -X POST "http://localhost:8080/mcp/v1/message?session_id=<session_id>" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"}}}'

# Expected HTTP status: 202 Accepted
# JSON-RPC response arrives on the SSE stream as: event: message
```

See [API.md](API.md) for complete API documentation.

## Development

### Project Structure

```
.
├── cmd/
│   └── server/          # Main application entry point
├── internal/
│   ├── audit/           # Audit logging infrastructure
│   ├── cache/           # Redis client and cache management
│   ├── circuitbreaker/  # Circuit breaker pattern for resilience
│   ├── config/          # Configuration management
│   ├── database/        # PostgreSQL operations and migrations
│   │   └── migrations/  # golang-migrate migration files
│   ├── mcp/             # MCP protocol implementation
│   ├── models/          # Data models (Document, Rule, Project, Failure)
│   ├── security/        # Secrets scanning and detection
│   ├── team/            # Team management (migrated from Python v2.6.0)
│   │   ├── manager.go   # Core team operations
│   │   ├── encryption.go # Data encryption at rest
│   │   ├── rules.go     # Team layout rules
│   │   └── types.go     # Data structures
│   ├── validation/      # Input validation utilities
│   └── web/             # HTTP server, handlers, middleware
├── deploy/              # Deployment files (Dockerfile, compose)
└── README.md            # This file
```

**Note:** As of v2.6.0, all team management functionality has been migrated from Python (`scripts/team_manager.py`) to Go (`internal/team/`). See [../docs/PYTHON_TO_GO_MIGRATION.md](../docs/PYTHON_TO_GO_MIGRATION.md) for details.

### Adding New Features

1. Update models in `internal/models/`
2. Add database operations in `internal/database/`
3. Add handlers in `internal/web/`
4. Update routes in `internal/web/server.go`
5. Add tests

## Troubleshooting

### Database Connection Issues

**Problem:** `failed to connect to database`

**Solution:**
- Verify PostgreSQL is running: `docker ps | grep postgres`
- Check credentials in `.env` file
- Ensure database exists: `createdb guardrails`
- Verify SSL mode settings match your environment

### Redis Connection Issues

**Problem:** `failed to connect to Redis`

**Solution:**
- Verify Redis is running: `docker ps | grep redis`
- Check REDIS_PASSWORD matches between `.env` and Redis container
- For local development without Redis, set `REDIS_PASSWORD=` (empty)

### SSE Connection Errors

**Problem:** EOF errors when connecting to `/mcp/v1/sse`

**Solution:**
- Verify the client posts follow-up messages to the `endpoint` URL emitted by SSE
- Ensure requests use `?session_id=<session_id>` from that endpoint event
- Ensure no proxy is buffering SSE responses (check X-Accel-Buffering header)
- If using custom clients, ensure they consume only `event: message` payloads as JSON-RPC

### API Key Authentication Failures

**Problem:** `Missing authorization header` or `Invalid API key`

**Solution:**
- Verify `Authorization: Bearer <api_key>` header format
- Check that MCP_API_KEY or IDE_API_KEY environment variables are set
- For Web UI access and read-only browsing APIs, no API key is required

### Guardrails Not Enforcing (`rules_evaluated=0` or dangerous commands allowed)

**Problem:** MCP tool calls return permissive results even for dangerous commands.

**Cause:** Runtime rule/project data is missing, or rule categories do not match validator categories.

**Solution:**
- Check data state:
  - `curl -s http://localhost:8096/api/stats`
  - If `rules_count` or `projects_count` is `0`, run rule sync and seed a project.
- Trigger rule sync:
  - `curl -X POST http://localhost:8096/api/rules/sync -H "Authorization: Bearer $MCP_API_KEY" -H "Content-Type: application/json" -d '{"force":true}'`
  - `curl -s http://localhost:8096/api/rules/sync/status`
- Ensure the project used by `guardrail_init_session` has `active_rules` populated.
- Verify categories for command enforcement:
  - `guardrail_validate_bash` evaluates `bash` (and compatible legacy categories) plus `all`.
  - `guardrail_validate_git_operation` evaluates `git` (and compatible legacy categories) plus `all`.
  - Rules intended to apply globally should use category `all`.
- Re-test using MCP `initialize` -> `guardrail_init_session` -> `guardrail_validate_bash`/`guardrail_validate_git_operation` and confirm `rules_evaluated > 0`.

### Schema Validation Error (Critical!)

**Error:**
```
Invalid schema for function 'guardrails_guardrail_pre_work_check':
In context=('properties', 'affected_files'), array schema missing items
```

**Cause:** Server name contains dashes/hyphens

**Solution:** Change server name from "guardrail-mcp" to "guardrail_mcp" in `internal/mcp/server.go` line 101:
```go
s.mcpServer = server.NewDefaultServer("guardrail_mcp", "1.0.0")
```

See [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md) for complete deployment instructions.

### Database Migration Failures

**Problem:** `no schema has been selected to create in`

**Solution:**
```bash
# Connect to PostgreSQL and create schema
psql -U guardrails -d guardrails -c "CREATE SCHEMA IF NOT EXISTS public;"
```

### Container Won't Start

**Problem:** Container exits immediately

**Solution:**
```bash
# Check logs
make docker-logs
# or: docker compose -f deploy/podman-compose.yml logs -f

# Verify all required environment variables are set
cat .env | grep -E "(API_KEY|PASSWORD|SECRET)"

# Ensure PostgreSQL and Redis are healthy before starting MCP server
```

## License

MIT

---

## Deployment Status

**Version:** v3.1.0
**Deployment Date:** 2026-02-15
**Deployed To:** AI01 (0.0.0.0:8095/8096)
**Status:** ✅ Successfully deployed and verified
**Implementation:** Go (mcp-server/internal/)

### What Was Fixed During Deployment

1. **Schema Validation Error** ✅ FIXED
   - Changed server name from `guardrail-mcp` to `guardrail_mcp` (line 101 in `internal/mcp/server.go`)
   - This fixes the MCP framework's schema validation error that was blocking Claude Code from using the guardrail tools

2. **Postgres Permission Issues** ✅ FIXED
   - Added `user: "70:70"` to postgres service configuration
   - Removed security constraints to allow proper container initialization

3. **Configuration Requirements** ✅ UPDATED
   - MCP_API_KEY and IDE_API_KEY must be 32+ characters with mixed case and digits
   - JWT_SECRET must be at least 32 bytes long
   - JWT_ROTATION_HOURS must include 'h' unit (e.g., `168h`)
   - Use localhost for container communication within pod

### Verification Checklist

- ✅ Postgres running and healthy (localhost:5432)
- ✅ Redis running and healthy (localhost:6379)
- ✅ MCP server started successfully
- ✅ Database connected
- ✅ Redis connected
- ✅ MCP endpoint responding (port 8095)
- ✅ Web UI responding (port 8096)
- ✅ Server name correctly set to `guardrail_mcp` (with underscore)

### For Testers

**AI01 Connection Info:**
- **MCP Endpoint:** http://0.0.0.0:8095/mcp/v1/sse
- **Web UI:** http://0.0.0.0:8096
- **API Key:** DevKey123456789012345678901234567890 (example - use your own)

**OpenCode Configuration:**
```jsonc
{
  "mcpServers": {
    "guardrails": {
      "type": "remote",
      "url": "http://0.0.0.0:8095/mcp/v1/sse",
      "headers": {
        "Authorization": "Bearer DevKey123456789012345678901234567890"
      }
    }
  }
}
```

See [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md) for complete deployment instructions and troubleshooting.
