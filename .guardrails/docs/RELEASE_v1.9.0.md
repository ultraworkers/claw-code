# Release v1.9.0 - MCP Server Edition

**Release Date:** 2026-02-07
**Branch:** `mcpserver-impl`
**Tag:** `v1.9.0`

---

## Overview

Full production-ready MCP (Model Context Protocol) server implementation for the Guardrail Platform. This release introduces a complete Go-based server that enables AI agents (Claude Code, OpenCode, etc.) to validate their actions against guardrails in real-time.

---

## What's New

### MCP Server (`mcp-server/`)

A complete Model Context Protocol implementation using `mark3labs/mcp-go` v0.4.0.

#### MCP Tools (Actions)

| Tool | Description |
|------|-------------|
| `guardrail_init_session` | Initialize a validation session for a project |
| `guardrail_validate_bash` | Validate bash commands against forbidden patterns |
| `guardrail_validate_file_edit` | Validate file edit operations |
| `guardrail_validate_git_operation` | Validate git commands (blocks force push) |
| `guardrail_pre_work_check` | Run pre-work checklist from failure registry |
| `guardrail_get_context` | Get guardrail context for the session's project |

#### MCP Resources (Data)

| Resource | URI | Description |
|----------|-----|-------------|
| Quick Reference | `guardrail://quick-reference` | Quick reference card for guardrails |
| Active Rules | `guardrail://rules/active` | Currently active prevention rules |

#### Transport

- **SSE (Server-Sent Events)** for real-time client communication
- **JSON-RPC 2.0** protocol for tool calls
- Endpoints:
  - `GET /mcp/v1/sse` - SSE event stream
  - `POST /mcp/v1/message` - JSON-RPC requests

### Web UI

Browser-based interface for managing guardrails:

- Browse and edit guardrail documents
- Manage prevention rules (CRUD operations)
- View failure registry
- Project configuration management

### Infrastructure

#### Production Deployment

Successfully deployed to production (RHEL 8 + Podman):

```
Production Server
├── guardrail-redis (Redis 7-alpine)
├── guardrail-postgres (PostgreSQL 16-alpine)
└── guardrail-mcp-server (distroless/static:nonroot)
    ├── Port 8092 → MCP Protocol
    └── Port 8093 → Web UI
```

#### Security Hardening

- **Non-root user**: UID 65532 (distroless)
- **Read-only filesystem**: Container root is read-only
- **Dropped capabilities**: ALL capabilities dropped
- **SELinux**: `label=disable` for volume access
- **No new privileges**: `no-new-privileges:true`
- **Distroless image**: Minimal attack surface

#### Backends

- **PostgreSQL 16**: Persistent storage for documents, rules, failures, projects
- **Redis 7**: Caching (rules, docs, search) and rate limiting

---

## API Endpoints

### MCP Protocol
```
http://localhost:8092/mcp/v1/sse
http://localhost:8092/mcp/v1/message
```

### Web UI
```
http://localhost:8093
```

---

## Configuration

### Environment Variables

```bash
# Required API Keys
MCP_API_KEY=<generate with: openssl rand -hex 32>
IDE_API_KEY=<generate with: openssl rand -hex 32>
JWT_SECRET=<generate with: openssl rand -hex 48>

# Database
DB_HOST=postgres
DB_PORT=5432
DB_NAME=guardrails
DB_USER=guardrails
DB_PASSWORD=<secure password>
DB_SSLMODE=disable

# Redis
REDIS_HOST=redis
REDIS_PORT=6379
REDIS_PASSWORD=<secure password>
REDIS_USE_TLS=false

# Server
MCP_PORT=8080
WEB_PORT=8081
WEB_ENABLED=true
LOG_LEVEL=info
REQUEST_TIMEOUT=30s

# Rate Limiting (requests per minute)
MCP_RATE_LIMIT=1000
IDE_RATE_LIMIT=500
SESSION_RATE_LIMIT=100

# Cache TTL
CACHE_TTL_RULES=5m
CACHE_TTL_DOCS=10m
CACHE_TTL_SEARCH=2m
```

---

## Deployment

### Build

```bash
cd mcp-server
docker build -t guardrail-mcp:latest -f deploy/Dockerfile .
docker save guardrail-mcp:latest > guardrail-mcp.tar
```

### Deploy to Production Server

```bash
# Copy files
scp guardrail-mcp.tar user@your-server:/opt/guardrail-mcp/
scp .env user@your-server:/opt/guardrail-mcp/
scp deploy/podman-compose.yml user@your-server:/opt/guardrail-mcp/

# On server
ssh user@your-server
cd /opt/guardrail-mcp
sudo podman load -i guardrail-mcp.tar
sudo podman-compose up -d
```

---

## Client Configuration

### Claude Code

Add to Claude Code settings:

```json
{
  "mcpServers": {
    "guardrails": {
      "url": "http://your-server:8092/mcp/v1/sse",
      "headers": {
        "Authorization": "Bearer <MCP_API_KEY>"
      }
    }
  }
}
```

### OpenCode

Add to `.opencode/oh-my-opencode.jsonc`:

```jsonc
{
  "mcp": {
    "servers": [
      {
        "name": "guardrails",
        "url": "http://your-server:8092/mcp/v1/sse",
        "apiKey": "<MCP_API_KEY>"
      }
    ]
  }
}
```

---

## Testing

### MCP Initialize

```bash
curl -s -X POST http://localhost:8092/mcp/v1/message \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "2024-11-05",
      "capabilities": {},
      "clientInfo": {
        "name": "test-client",
        "version": "1.0"
      }
    }
  }'
```

**Expected Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "capabilities": {
      "resources": {}
    },
    "protocolVersion": "2024-11-05",
    "serverInfo": {
      "name": "guardrail-mcp",
      "version": "1.0.0"
    }
  }
}
```

### SSE Stream

```bash
curl -s -N http://localhost:8092/mcp/v1/sse
```

**Expected Output:**
```
event: endpoint
data: /mcp/v1/message?session_id=sess_...
```

---

## Files Changed

| File | Changes |
|------|---------|
| `mcp-server/cmd/server/main.go` | Changed binding from 127.0.0.1 to 0.0.0.0 |
| `mcp-server/go.mod` | Updated to Go 1.23.2, added mcp-go v0.4.0 |
| `mcp-server/go.sum` | New dependency checksums |
| `mcp-server/internal/mcp/server.go` | Full MCP protocol implementation |
| `CHANGELOG.md` | Release notes |

---

## Known Issues

None.

---

## Migration Guide

### From v1.8.0

1. Update `.env` file with new required variables
2. Build new container image
3. Deploy to your production server
4. Update client configurations with new MCP endpoint

---

## Credits

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>

---

## Links

- **Repository:** https://github.com/TheArchitectit/agent-guardrails-template
- **Release:** https://github.com/TheArchitectit/agent-guardrails-template/releases/tag/v1.9.0
- **Changelog:** [CHANGELOG.md](../CHANGELOG.md)
