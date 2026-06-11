# MCP Server Tester Guide (Container Deployment)

> This guide is for external testers validating the Guardrail MCP server stack on a deployment host.

**Release Target:** v1.9.6  
**Branch:** `mcpserver`  
**Status:** Testing in progress

---

## 1) Container Architecture (Current)

The deployment is a **3-container stack** with strict network separation:

```text
Deployment host
|
|-- guardrail-mcp-server (app)
|   |-- container port 8080: MCP SSE + JSON-RPC message endpoint
|   |-- container port 8081: Web UI + REST API + health + metrics
|   |-- attached networks: frontend, backend
|
|-- guardrail-postgres (state)
|   |-- container port 5432 (backend network only)
|   |-- attached networks: backend
|   |-- persistent volume: pg_data
|
|-- guardrail-redis (cache/rate limiting)
|   |-- container port 6379 (backend network only)
|   |-- attached networks: backend
|   |-- persistent volume: redis_data
```

### Host exposure model

- MCP and Web ports are bound to **loopback only** via compose: `127.0.0.1:${MCP_PORT}:8080` and `127.0.0.1:${WEB_PORT}:8081`
- Postgres and Redis are **not** host-exposed
- Default host ports are `8080` and `8081`
- Some environments use `8092` and `8093` by setting env vars

### Security and runtime constraints

- App container runs as non-root `65532:65532`
- Root filesystem is read-only with `/tmp` mounted as tmpfs
- Linux capabilities dropped (`ALL`)
- `no-new-privileges:true` enabled
- `postgres` and `redis` health checks gate app startup

---

## 2) Pre-Flight Checks (Before Testing)

Run these from `mcp-server/`.

```bash
podman --version
podman-compose --version
```

If you are Docker-only (no Podman), use:

```bash
docker --version
docker compose version
```

Verify no local port conflict on your target host ports:

```bash
ss -ltnp | grep -E ":(8080|8081|8092|8093)"
```

If testing from your laptop against the deployment host, create an SSH tunnel first:

```bash
ssh -L 8092:127.0.0.1:8092 -L 8093:127.0.0.1:8093 <user>@<host>
```

Then use `http://localhost:8092` and `http://localhost:8093` locally.

---

## 3) Environment Setup

```bash
cd mcp-server
cp .env.example .env
```

Set at minimum:

- `MCP_API_KEY` (32+ chars, mixed case + digits)
- `IDE_API_KEY` (32+ chars, mixed case + digits)
- `JWT_SECRET` (32+ chars minimum; 64+ recommended)
- `DB_PASSWORD`
- `REDIS_PASSWORD`
- `MCP_PORT` / `WEB_PORT` (use custom ports like `8092` / `8093` if required)

Example secure generation:

```bash
openssl rand -hex 32   # MCP_API_KEY
openssl rand -hex 32   # IDE_API_KEY
openssl rand -hex 64   # JWT_SECRET
openssl rand -base64 32 # DB/Redis passwords
```

---

## 4) Start the Stack

### Option A: Build and run on target host

```bash
cd mcp-server
make docker-up
```

### Option B: Explicit compose command

```bash
cd mcp-server
podman-compose -f deploy/podman-compose.yml up -d --build
```

### Option C: Docker only (no Podman)

The Makefile targets use Podman tools. If you are Docker-only, run compose commands directly:

```bash
cd mcp-server
docker compose -f deploy/podman-compose.yml up -d --build
```

Tester-validated Docker variant (recommended when using Docker Desktop/Engine):

```bash
cd mcp-server
docker compose -f deploy/docker-compose.example.yml up -d --build
```

Check status:

```bash
podman-compose -f deploy/podman-compose.yml ps
podman ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
```

Docker equivalent:

```bash
docker compose -f deploy/podman-compose.yml ps
docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
```

If started with the Docker example file, use:

```bash
docker compose -f deploy/docker-compose.example.yml ps
```

Expected containers:

- `guardrail-postgres` (healthy)
- `guardrail-redis` (healthy)
- `guardrail-mcp-server` (running)

---

## 5) Run Database Migrations (Required)

If migrations are skipped, many API operations fail even when containers look healthy.

Use this compose-safe method (works even when Postgres is not host-exposed):

```bash
cd mcp-server
set -a
source .env
set +a

for f in internal/database/migrations/*_up.sql; do
  echo "Applying $f"
  podman exec -i guardrail-postgres psql -U "$DB_USER" -d "$DB_NAME" < "$f"
done
```

Docker equivalent:

```bash
cd mcp-server
set -a
source .env
set +a

for f in internal/database/migrations/*_up.sql; do
  echo "Applying $f"
  docker exec -i guardrail-postgres psql -U "$DB_USER" -d "$DB_NAME" < "$f"
done
```

Optional (only if DB is directly reachable from host):

```bash
make migrate-up DATABASE_URL="postgresql://${DB_USER}:${DB_PASSWORD}@127.0.0.1:${DB_PORT}/${DB_NAME}?sslmode=disable"
```

---

## 6) Smoke Test Checklist

Define base URLs from your `.env`:

```bash
set -a
source .env
set +a
export MCP_BASE="http://localhost:${MCP_PORT}"
export WEB_BASE="http://localhost:${WEB_PORT}"
```

### 6.1 Web health and metrics

```bash
curl -s "$WEB_BASE/health/live"
curl -s "$WEB_BASE/health/ready"
curl -s "$WEB_BASE/version"
curl -s "$WEB_BASE/metrics" | head -n 5
```

Expected:

- `/health/live` -> `200`, `status: alive`
- `/health/ready` -> `200`, `status: ready`
- `/version` -> service/version JSON
- `/metrics` -> Prometheus text output

### 6.2 Public vs protected Web API behavior

Public (no API key expected):

```bash
curl -i "$WEB_BASE/api/documents"
curl -i "$WEB_BASE/api/rules"
```

Protected (no API key should fail):

```bash
curl -i -X POST "$WEB_BASE/api/rules" \
  -H "Content-Type: application/json" \
  -d '{"rule_id":"TEST-001","name":"x","pattern":"x","message":"x","severity":"warning","category":"test","enabled":true}'
```

Expected: `401 Unauthorized` without `Authorization: Bearer <MCP_API_KEY>`.

---

## 7) MCP Protocol Test Flow (Important)

This is where most tester confusion happens.

### Key behavior

- `GET /mcp/v1/sse` creates a session and sends an `endpoint` event
- The event data includes the required `session_id` in message URL
- `POST /mcp/v1/message?session_id=...` usually returns `202 Accepted` (no JSON body)
- The actual JSON-RPC response is delivered on the SSE stream as `event: message`

### Step-by-step (2 terminals)

Terminal A - keep SSE open:

```bash
curl -N "$MCP_BASE/mcp/v1/sse"
```

You should see something like:

```text
event: endpoint
data: http://localhost:<MCP_PORT>/mcp/v1/message?session_id=sess_abc123...

: ping
```

Copy the full `message?session_id=...` URL.

Terminal B - send initialize request to that URL:

```bash
curl -i -X POST "$MCP_BASE/mcp/v1/message?session_id=<session_id>" \
  -H "Content-Type: application/json" \
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

Expected:

- HTTP status from POST: `202 Accepted`
- Terminal A receives `event: message` with JSON-RPC result payload

### Tool call example

```bash
curl -i -X POST "$MCP_BASE/mcp/v1/message?session_id=<session_id>" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "guardrail_validate_git_operation",
      "arguments": {
        "command": "push",
        "is_force": true
      }
    }
  }'
```

Expected `event: message` payload includes a violation for force push.

---

## 8) Container Security Verification

```bash
podman inspect guardrail-mcp-server --format '{{.Config.User}}'
podman inspect guardrail-mcp-server --format '{{.HostConfig.ReadonlyRootfs}}'
podman inspect guardrail-mcp-server --format '{{json .HostConfig.CapDrop}}'
podman inspect guardrail-mcp-server --format '{{json .HostConfig.SecurityOpt}}'
podman inspect guardrail-mcp-server --format '{{json .HostConfig.Tmpfs}}'
```

Docker equivalent:

```bash
docker inspect guardrail-mcp-server --format '{{.Config.User}}'
docker inspect guardrail-mcp-server --format '{{.HostConfig.ReadonlyRootfs}}'
docker inspect guardrail-mcp-server --format '{{json .HostConfig.CapDrop}}'
docker inspect guardrail-mcp-server --format '{{json .HostConfig.SecurityOpt}}'
docker inspect guardrail-mcp-server --format '{{json .HostConfig.Tmpfs}}'
```

Expected:

- User is `65532:65532`
- `ReadonlyRootfs` is `true`
- Capabilities show `ALL` dropped
- Security options include `no-new-privileges:true`
- `/tmp` tmpfs mount is present

---

## 9) Common Failure Modes and Fixes

### Symptom: `Missing session_id parameter`

- Cause: posting to `/mcp/v1/message` without query string
- Fix: always post to URL emitted in SSE `endpoint` event

### Symptom: POST returns `202` but no JSON body

- Cause: expected behavior in session/SSE mode
- Fix: read JSON-RPC response from Terminal A SSE stream (`event: message`)

### Symptom: `/health/ready` returns `503`

- Cause: DB or Redis unhealthy/unreachable
- Fix:
  - `podman logs guardrail-postgres`
  - `podman logs guardrail-redis`
  - confirm `.env` credentials match compose env

### Symptom: cannot access from outside deployment host

- Cause: ports are bound to `127.0.0.1` only
- Fix: use SSH tunnel or put Nginx/Traefik in front

### Symptom: Web UI loads blank/missing assets

- Cause: stale container image before static asset packaging updates
- Fix:
  - `podman-compose -f deploy/podman-compose.yml down`
  - `podman-compose -f deploy/podman-compose.yml up -d --build`

---

## 10) Bug Report Template

Use this exact format in issues:

```markdown
**Test Area:** [Architecture | MCP Protocol | Web API | Security | Resilience]
**Severity:** [Critical | High | Medium | Low]
**Expected:** [What should happen]
**Actual:** [What happened]

**Repro Steps:**
1. ...
2. ...
3. ...

**Artifacts:**
- `podman-compose ps` output
- `podman logs guardrail-mcp-server --tail 200`
- Full curl command used (redact secrets)
- Response status/body or SSE event snippet

**Environment:**
- Host: <deployment-host>
- OS:
- Podman version:
- podman-compose version:
- Tested commit/tag:
```

---

## 11) Quick Command Block

```bash
# Start stack
cd mcp-server && make docker-up

# Check service state
podman-compose -f deploy/podman-compose.yml ps

# Follow app logs
podman logs -f guardrail-mcp-server

# Load port values
set -a && source .env && set +a

# Health check
curl -s "http://localhost:${WEB_PORT}/health/ready"

# Open SSE
curl -N "http://localhost:${MCP_PORT}/mcp/v1/sse"
```

Docker-only quick block:

```bash
# Start stack
cd mcp-server
docker compose -f deploy/podman-compose.yml up -d --build

# Or use tester-validated Docker compose file
# docker compose -f deploy/docker-compose.example.yml up -d --build

# Check service state
docker compose -f deploy/podman-compose.yml ps

# Follow app logs
docker logs -f guardrail-mcp-server

# Load port values
set -a && source .env && set +a

# Health check
curl -s "http://localhost:${WEB_PORT}/health/ready"

# Open SSE
curl -N "http://localhost:${MCP_PORT}/mcp/v1/sse"
```

---

## Feedback Channel

- GitHub Issues: https://github.com/TheArchitectit/agent-guardrails-template/issues

---

**Last Updated:** 2026-02-08
