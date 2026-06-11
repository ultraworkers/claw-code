# Guardrail MCP Server Deployment Guide

**Version:** v3.1.0
**Last Updated:** 2026-05-12
**Tested On:** AI01 (RHEL Server with Podman), Docker Desktop (Windows 11), Ubuntu 24.04

## Overview

This guide provides step-by-step instructions for deploying the Guardrail MCP Server to production, including all fixes applied during the AI01 deployment.

## Prerequisites

- RHEL or compatible Linux distribution
- Podman or Docker installed
- Access to deployment server via SSH
- Go 1.23+ (for building from source)
- PostgreSQL 16 (or use containerized version)
- Redis 7 (or use containerized version)

## Deployment Summary

### What Was Fixed During AI01 Deployment

1. **Schema Validation Error** - Changed server name from "guardrail-mcp" to "guardrail_mcp" to fix MCP framework validation issues
2. **Postgres Permission Issues** - Removed security constraints and added `user: "70:70"` for postgres
3. **Configuration Variables** - Corrected ports, API keys, and JWT settings
4. **Container Networking** - Used pod networking to ensure containers can communicate

## Quick Deploy

### 1. Update AI01 IP in .env

```bash
# Update AI01 IP in .env
localhost status | grep localhost
sed -i 's/AI01_IP=.*/AI01_IP=0.0.0.0/' .env
```

### 2. Build and Deploy

```bash
cd /home/user001/mcp-server

# Load environment variables
export $(cat .env | grep -v '^#' | grep -v '^$' | xargs)

# Build Docker image
podman build \
  --build-arg VERSION=v3.1.0 \
  --build-arg BUILD_TIME=$(date -u +%Y-%m-%dT%H:%M:%SZ) \
  --build-arg GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown') \
  -f deploy/Dockerfile \
  -t guardrail-mcp:v3.1.0 .

# Create pod with port mappings
podman pod create --name guardrail-pod -p 8095:8095 -p 8096:8096

# Start postgres (with user 70:70 to avoid permission issues)
podman run -d --pod guardrail-pod --name guardrail-postgres \
  --user 70:70 \
  -e POSTGRES_USER=guardrail \
  -e POSTGRES_PASSWORD=guardrail123 \
  -e POSTGRES_DB=guardrail \
  -v guardrail_pg_data:/var/lib/postgresql/data \
  docker.io/library/postgres:16-alpine

# Wait for postgres to be ready
sleep 5

# Start redis
podman run -d --pod guardrail-pod --name guardrail-redis \
  -e REDIS_PASSWORD=redis123 \
  docker.io/library/redis:7-alpine \
  redis-server --requirepass redis123 --maxmemory 256mb --maxmemory-policy allkeys-lru

# Start MCP server
podman run -d --pod guardrail-pod --name guardrail-mcp-server \
  -e REDIS_PASSWORD=redis123 \
  -e MCP_PORT=8095 \
  -e WEB_PORT=8096 \
  -e WEB_ENABLED=true \
  -e LOG_LEVEL=info \
  -e DB_HOST=localhost \
  -e DB_PORT=5432 \
  -e DB_NAME=guardrail \
  -e DB_USER=guardrail \
  -e DB_PASSWORD=guardrail123 \
  -e DB_SSLMODE=disable \
  -e REDIS_HOST=localhost \
  -e REDIS_PORT=6379 \
  -e MCP_API_KEY=DevKey123456789012345678901234567890 \
  -e IDE_API_KEY=DevKey456789012345678901234567890123 \
  -e JWT_SECRET=Dev-JWT-Secret-789-Longer-32bytes \
  -e JWT_ISSUER=guardrail-mcp \
  -e JWT_EXPIRY=15m \
  -e JWT_ROTATION_HOURS=168h \
  localhost/guardrail-mcp:v3.1.0
```

## Windows Docker Desktop Deployment

### Prerequisites

- Windows 10/11 with WSL2 enabled
- Docker Desktop installed with WSL2 backend
- Git for Windows or WSL2 Git

### Step 1: Clone and Configure

```powershell
# In PowerShell or Windows Terminal
cd C:\Users\YourName\Projects
git clone https://github.com/TheArchitectit/agent-guardrails-template.git
cd agent-guardrails-template/mcp-server

# Copy environment template
copy .env.example .env

# Generate secure keys (requires OpenSSL for Windows or use WSL2)
# Or manually generate 32+ character strings
```

### Step 2: Deploy with Docker Compose

```powershell
# Build and start all services
docker compose -f deploy/docker-compose.example.yml up -d --build

# Verify containers are running
docker compose -f deploy/docker-compose.example.yml ps

# View logs
docker compose -f deploy/docker-compose.example.yml logs -f
```

### Step 3: Access the Services

| Service | URL |
|---------|-----|
| Web UI | http://localhost:8095 |
| Health Check | http://localhost:8095/health/ready |
| MCP SSE | http://localhost:8095/mcp/v1/sse |

### Windows-Specific Notes

- **Firewall:** Docker Desktop may prompt for firewall rules. Allow private network access.
- **Port conflicts:** If ports 8095/8096 are in use, edit `.env` to change `MCP_PORT` and `WEB_PORT`.
- **WSL2 file paths:** For best performance, keep project files inside WSL2 filesystem (`\\wsl$\Ubuntu\home\...`) rather than Windows mounts.
- **PowerShell execution policy:** If scripts fail, run `Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser`.

---

## Detailed Deployment Steps

### Step 1: Environment Setup

```bash
# Create deployment directory
mkdir -p /home/user001/mcp-server
cd /home/user001/mcp-server

# Copy code from repository (if not already there)
scp -r /path/to/agent-guardrails-template/mcp-server/* user001@0.0.0.0:/home/user001/mcp-server/

# Create .env file
cat > .env << 'EOF'
# =============================================================================
# Server Configuration
# =============================================================================
MCP_PORT=8095
WEB_PORT=8096
WEB_ENABLED=true
LOG_LEVEL=info
REQUEST_TIMEOUT=30s
SHUTDOWN_TIMEOUT=30s

# =============================================================================
# Database Configuration
# =============================================================================
DB_HOST=localhost
DB_PORT=5432
DB_NAME=guardrail
DB_USER=guardrail
DB_PASSWORD=guardrail123
DB_SSLMODE=disable

# =============================================================================
# Redis Configuration
# =============================================================================
REDIS_HOST=localhost
REDIS_PORT=6379
REDIS_PASSWORD=redis123
REDIS_USE_TLS=false

# =============================================================================
# Security Configuration
# =============================================================================
# IMPORTANT: These keys MUST be at least 32 characters long
# Must contain mix of uppercase, lowercase, and digits
MCP_API_KEY=DevKey123456789012345678901234567890
IDE_API_KEY=DevKey456789012345678901234567890123

# JWT Configuration - Must be at least 32 bytes long
JWT_SECRET=Dev-JWT-Secret-789-Longer-32bytes
JWT_ISSUER=guardrail-mcp
JWT_EXPIRY=15m
JWT_ROTATION_HOURS=168h
EOF
```

### Step 2: Apply Schema Fix

**Critical Fix:** The MCP framework (mark3labs/mcp-go v0.4.0) has issues with dashes/hyphens in server names. This causes schema validation errors.

```bash
# Check current server name
grep 'NewDefaultServer' internal/mcp/server.go

# Should show: server.NewDefaultServer("guardrail_mcp", "v3.1.0")
# NOT: server.NewDefaultServer("guardrail-mcp", "v3.1.0")

# If it shows "guardrail-mcp", change it:
sed -i 's/server.NewDefaultServer("guardrail-mcp"/server.NewDefaultServer("guardrail_mcp"/' internal/mcp/server.go
```

### Step 3: Build Docker Image

```bash
cd /home/user001/mcp-server

# Set build variables
VERSION=v3.1.0
BUILD_TIME=$(date -u +%Y-%m-%dT%H:%M:%SZ)
GIT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')

# Build the image
podman build \
  --build-arg VERSION=$VERSION \
  --build-arg BUILD_TIME=$BUILD_TIME \
  --build-arg GIT_COMMIT=$GIT_COMMIT \
  -f deploy/Dockerfile \
  -t guardrail-mcp:$VERSION .

# Verify image was created
podman images | grep guardrail-mcp
```

### Step 4: Create Pod and Start Containers

Using pod networking ensures containers can communicate via localhost:

```bash
# Remove existing containers (if any)
podman stop guardrail-postgres guardrail-redis guardrail-mcp-server 2>/dev/null
podman rm guardrail-postgres guardrail-redis guardrail-mcp-server 2>/dev/null
podman pod rm guardrail-pod 2>/dev/null

# Create new pod with port mappings
podman pod create --name guardrail-pod -p 8095:8095 -p 8096:8096

# Start postgres (critical: use user 70:70 to avoid permission issues)
podman run -d --pod guardrail-pod --name guardrail-postgres \
  --user 70:70 \
  -e POSTGRES_USER=guardrail \
  -e POSTGRES_PASSWORD=guardrail123 \
  -e POSTGRES_DB=guardrail \
  -v guardrail_pg_data:/var/lib/postgresql/data \
  docker.io/library/postgres:16-alpine

# Wait for postgres to initialize (important!)
echo "Waiting for postgres to be ready..."
sleep 5

# Verify postgres is running
podman ps | grep guardrail-postgres

# Start redis
podman run -d --pod guardrail-pod --name guardrail-redis \
  -e REDIS_PASSWORD=redis123 \
  docker.io/library/redis:7-alpine \
  redis-server --requirepass redis123 --maxmemory 256mb --maxmemory-policy allkeys-lru

# Wait for redis to start
sleep 3

# Start MCP server
podman run -d --pod guardrail-pod --name guardrail-mcp-server \
  -e REDIS_PASSWORD=redis123 \
  -e MCP_PORT=8095 \
  -e WEB_PORT=8096 \
  -e WEB_ENABLED=true \
  -e LOG_LEVEL=info \
  -e DB_HOST=localhost \
  -e DB_PORT=5432 \
  -e DB_NAME=guardrail \
  -e DB_USER=guardrail \
  -e DB_PASSWORD=guardrail123 \
  -e DB_SSLMODE=disable \
  -e REDIS_HOST=localhost \
  -e REDIS_PORT=6379 \
  -e MCP_API_KEY=DevKey123456789012345678901234567890 \
  -e IDE_API_KEY=DevKey456789012345678901234567890123 \
  -e JWT_SECRET=Dev-JWT-Secret-789-Longer-32bytes \
  -e JWT_ISSUER=guardrail-mcp \
  -e JWT_EXPIRY=15m \
  -e JWT_ROTATION_HOURS=168h \
  localhost/guardrail-mcp:v3.1.0
```

### Step 5: Verify Deployment

```bash
# Check all containers are running
podman ps -a | grep guardrail

# Expected output showing all three containers UP
# guardrail-postgres, guardrail-redis, guardrail-mcp-server

# Check MCP server logs
podman logs guardrail-mcp-server 2>&1 | tail -20

# Should show:
# - "Database connected"
# - "Redis connected"
# - "Starting web server" on 0.0.0.0:8096
# - "Starting MCP server" on 0.0.0.0:8095
# - "Starting MCP SSE server" on 0.0.0.0:8095

# Test MCP endpoint (from AI01)
curl -s http://localhost:8095/mcp 2>&1

# Should return: {"message":"Missing authorization header"}
# (This is expected - it means the server is responding)

# Test with API key
curl -s -H 'Authorization: Bearer DevKey123456789012345678901234567890' \
  http://localhost:8095/mcp 2>&1

# Test Web UI
curl -s http://localhost:8096/ 2>&1 | head -10
```

## Configuration Requirements

### Critical Settings

These settings were identified as critical during AI01 deployment:

1. **API Keys must be 32+ characters with mixed case and digits**
   ```bash
   # GOOD (32 chars, mixed case and digits):
   MCP_API_KEY=DevKey123456789012345678901234567890
   
   # BAD (too short, no digits):
   MCP_API_KEY=dev-key-short
   ```

2. **JWT_SECRET must be at least 32 bytes**
   ```bash
   # GOOD (33 bytes):
   JWT_SECRET=Dev-JWT-Secret-789-Longer-32bytes
   
   # BAD (too short):
   JWT_SECRET=short-secret
   ```

3. **JWT_ROTATION_HOURS must include 'h' unit**
   ```bash
   # GOOD:
   JWT_ROTATION_HOURS=168h
   
   # BAD (missing 'h'):
   JWT_ROTATION_HOURS=168
   ```

4. **Postgres must run as user 70:70**
   ```bash
   # Add to postgres service in compose file:
   user: "70:70"
   ```

5. **Server name must use underscores, not dashes**
   ```bash
   # In internal/mcp/server.go line 101:
   # GOOD:
   s.mcpServer = server.NewDefaultServer("guardrail_mcp", "v3.1.0")
   
   # BAD (causes schema validation error):
   s.mcpServer = server.NewDefaultServer("guardrail-mcp", "v3.1.0")
   ```

### Environment Variables Reference

```bash
# Server Configuration
MCP_PORT=8095                    # External MCP port (maps to internal 8080)
WEB_PORT=8096                    # External Web UI port (maps to internal 8081)
WEB_ENABLED=true                 # Enable Web UI
LOG_LEVEL=info                   # Log level: debug, info, warn, error
REQUEST_TIMEOUT=30s              # Request timeout
SHUTDOWN_TIMEOUT=30s             # Graceful shutdown timeout

# Database Configuration
DB_HOST=localhost                # Database host (use localhost for pod networking)
DB_PORT=5432                     # Database port
DB_NAME=guardrail                # Database name
DB_USER=guardrail                # Database user
DB_PASSWORD=guardrail123         # Database password (must be secure!)
DB_SSLMODE=disable               # SSL mode: disable, require, verify-full

# Redis Configuration
REDIS_HOST=localhost             # Redis host (use localhost for pod networking)
REDIS_PORT=6379                  # Redis port
REDIS_PASSWORD=redis123          # Redis password (must be secure!)
REDIS_USE_TLS=false              # Use TLS for Redis

# Security Configuration (Critical: must meet requirements)
MCP_API_KEY=DevKey123456789012345678901234567890  # 32+ chars, mixed case+digits
IDE_API_KEY=DevKey456789012345678901234567890123   # 32+ chars, mixed case+digits
JWT_SECRET=Dev-JWT-Secret-789-Longer-32bytes       # 32+ bytes long
JWT_ISSUER=guardrail-mcp      # JWT issuer
JWT_EXPIRY=15m                # JWT expiration
JWT_ROTATION_HOURS=168h       # JWT rotation (MUST include 'h')

# Rate Limiting
RATE_LIMIT_MCP=1000           # MCP API rate limit (req/min)
RATE_LIMIT_IDE=500            # IDE API rate limit (req/min)
RATE_LIMIT_SESSION=100        # Per-session rate limit (req/min)
RATE_LIMIT_WINDOW=1m          # Rate limit window
RATE_LIMIT_BURST_FACTOR=1.5   # Burst factor for rate limiting

# Cache TTL
CACHE_TTL_RULES=5m            # Rules cache TTL
CACHE_TTL_DOCS=10m            # Documents cache TTL
CACHE_TTL_SEARCH=2m           # Search cache TTL

# Feature Flags
ENABLE_VALIDATION=true        # Enable validation endpoint
ENABLE_METRICS=true           # Enable metrics collection
ENABLE_AUDIT_LOGGING=true     # Enable audit logging
ENABLE_CACHE=true             # Enable Redis caching

# CORS Configuration
CORS_ALLOWED_ORIGINS=*        # Allowed origins (use specific domains in production)
CORS_MAX_AGE=86400            # CORS max age
```

## Docker Compose Configuration

### Working Configuration (AI01 Deployment)

```yaml
version: "3.8"

services:
  redis:
    image: docker.io/library/redis:7-alpine
    container_name: guardrail-redis
    restart: unless-stopped
    command: redis-server --requirepass redis123 --maxmemory 256mb --maxmemory-policy allkeys-lru
    environment:
      - REDIS_PASSWORD=redis123
    healthcheck:
      test: ["CMD", "redis-cli", "-a", "${REDIS_PASSWORD}", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 10s

  postgres:
    container_name: guardrail-postgres
    image: docker.io/library/postgres:16-alpine
    restart: unless-stopped
    user: "70:70"  # CRITICAL: Prevents permission errors
    environment:
      - POSTGRES_USER=${DB_USER}
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=${DB_NAME}
    volumes:
      - pg_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${DB_USER} -d ${DB_NAME}"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 30s

  mcp-server:
    image: guardrail-mcp:${VERSION:-latest}
    container_name: guardrail-mcp-server
    restart: unless-stopped
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    ports:
      - "8095:8095"  # MCP - external port
      - "8096:8096"  # Web UI - external port
    environment:
      # Use localhost for pod networking
      - DB_HOST=localhost
      - DB_PORT=5432
      - REDIS_HOST=localhost
      - REDIS_PORT=6379
      # All other environment variables from .env
    read_only: true
    user: "65532:65532"

volumes:
  pg_data:
    driver: local
  redis_data:
    driver: local
```

### Common Pitfalls

❌ **DON'T use dashes in server name:**
```go
// WRONG - causes schema validation error:
s.mcpServer = server.NewDefaultServer("guardrail-mcp", "v3.1.0")
```

✅ **DO use underscores:**
```go
// CORRECT:
s.mcpServer = server.NewDefaultServer("guardrail_mcp", "v3.1.0")
```

❌ **DON'T forget postgres user:**
```yaml
# WRONG - causes permission errors:
postgres:
  image: postgres:16-alpine
  # Missing user specification
```

✅ **DO specify postgres user:**
```yaml
# CORRECT:
postgres:
  image: postgres:16-alpine
  user: "70:70"  # Required for proper permissions
```

❌ **DON'T use short/weak API keys:**
```bash
# WRONG - too short, no digits:
MCP_API_KEY=dev-key-short
```

✅ **DO use 32+ character mixed keys:**
```bash
# CORRECT:
MCP_API_KEY=DevKey123456789012345678901234567890
```

## Testing the Deployment

### Test MCP Protocol

```bash
# From AI01 (localhost):
curl -sN http://localhost:8095/mcp/v1/sse &

# Capture session_id from endpoint event, then:
curl -X POST "http://localhost:8095/mcp/v1/message?session_id=YOUR_SESSION_ID" \
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

# Expected: HTTP 202 Accepted
# Response arrives on SSE stream
```

### Test Guardrail Tools

```bash
# Test guardrail_validate_bash
curl -X POST "http://localhost:8095/mcp/v1/message?session_id=YOUR_SESSION_ID" \
  -H "Authorization: Bearer DevKey123456789012345678901234567890" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "guardrail_validate_bash",
      "arguments": {
        "command": "rm -rf /",
        "session_token": "test-session"
      }
    }
  }'
```

### Test Web UI

```bash
# Test Web UI is responding
curl -s http://localhost:8096/ | head -10

# Test API endpoints
curl -s http://localhost:8096/api/rules | jq .
curl -s http://localhost:8096/api/documents | jq .
curl -s http://localhost:8096/api/stats | jq .
```

## Troubleshooting Guide

### Problem: Schema Validation Error

**Error:**
```
Invalid schema for function 'guardrails_guardrail_pre_work_check':
In context=('properties', 'affected_files'), array schema missing items
```

**Cause:** Server name contains dashes/hyphens

**Solution:**
```bash
# Check server name
grep 'NewDefaultServer' internal/mcp/server.go

# Should show: server.NewDefaultServer("guardrail_mcp", "v3.1.0")
# If not, fix it:
sed -i 's/server.NewDefaultServer("guardrail-mcp"/server.NewDefaultServer("guardrail_mcp"/' internal/mcp/server.go

# Rebuild and redeploy
podman build -f deploy/Dockerfile -t guardrail-mcp:fixed .
podman stop guardrail-mcp-server
podman run -d --pod guardrail-pod --name guardrail-mcp-server [environment variables] localhost/guardrail-mcp:fixed
```

### Problem: Postgres Permission Errors

**Error:**
```
chmod: /var/run/postgresql: Operation not permitted
find: /var/lib/postgresql/data/pgdata: Permission denied
```

**Cause:** Missing user specification for postgres container

**Solution:**
```bash
# Add user to postgres service in compose file:
# In deploy/podman-compose.yml:
services:
  postgres:
    image: postgres:16-alpine
    user: "70:70"  # ADD THIS LINE
    # ... rest of config

# Or when running directly:
podman run -d --user 70:70 postgres:16-alpine [other options]
```

### Problem: Database Authentication Failed

**Error:**
```
failed to connect to database: password authentication failed for user "guardrail"
```

**Cause:** Wrong database credentials or postgres not ready

**Solution:**
```bash
# Check postgres is running
podman ps | grep guardrail-postgres

# Check postgres logs
podman logs guardrail-postgres

# Verify credentials in .env match postgres environment
# .env should have:
DB_USER=guardrail
DB_PASSWORD=guardrail123
# And postgres should have:
POSTGRES_USER=guardrail
POSTGRES_PASSWORD=guardrail123

# Test connection from within container
podman exec guardrail-postgres psql -U guardrail -d guardrail -c 'SELECT 1'
```

### Problem: Redis Connection Refused

**Error:**
```
failed to connect to Redis: dial tcp [::1]:6379: connect: connection refused
```

**Cause:** Redis not running or not accessible

**Solution:**
```bash
# Check redis is running
podman ps | grep guardrail-redis

# Check redis logs
podman logs guardrail-redis

# Verify redis is in same pod (for localhost access)
podman pod inspect guardrail-pod | grep -A20 containers

# Test redis connection
podman exec guardrail-redis redis-cli -a redis123 ping
# Should return: PONG
```

### Problem: Container Exits Immediately

**Error:** Container shows "Exited (1)" status immediately after start

**Cause:** Missing or incorrect environment variables

**Solution:**
```bash
# Check container logs
podman logs guardrail-mcp-server

# Common issues:
# - Missing MCP_API_KEY or IDE_API_KEY
# - API keys too short (< 32 characters)
# - JWT_SECRET too short (< 32 bytes)
# - JWT_ROTATION_HOURS missing 'h' unit
# - Database credentials incorrect
# - Redis password incorrect

# Verify all required environment variables are set
podman inspect guardrail-mcp-server | grep -A100 '"Env"'
```

### Problem: Ports Already in Use

**Error:**
```
bind: address already in use
```

**Cause:** Ports 8095 or 8096 are already in use

**Solution:**
```bash
# Check what's using the ports
ss -tln | grep -E '8095|8096'

# Change ports in .env if needed:
MCP_PORT=8097
WEB_PORT=8098

# Update pod creation:
podman pod create --name guardrail-pod -p 8097:8097 -p 8098:8098

# Update environment in container:
-e MCP_PORT=8097 -e WEB_PORT=8098
```

### Problem: Connection Timeout from Remote Machine

**Error:** Cannot connect to MCP server from another machine

**Cause:** Firewall or network configuration

**Solution:**
```bash
# Check firewall status
firewall-cmd --state

# Open ports (if needed)
sudo firewall-cmd --permanent --add-port=8095/tcp
sudo firewall-cmd --permanent --add-port=8096/tcp
sudo firewall-cmd --reload

# Verify ports are open
sudo firewall-cmd --list-ports

# Test from remote machine
curl -s http://0.0.0.0:8095/mcp
```

### Problem: YAML Syntax Errors in Compose File

**Error:**
```
ERROR: yaml.scanner.ScannerError: mapping values are not allowed here
```

**Cause:** Incorrect indentation or orphaned lines

**Solution:**
```bash
# Validate YAML syntax
python3 -c "import yaml; yaml.safe_load(open('deploy/podman-compose.yml'))"

# Common issues:
# - Orphaned container_name line at file start
# - Incorrect indentation (use spaces, not tabs)
# - Duplicate keys
```

## Verification Checklist

After deployment, verify:

- [ ] All three containers running: `podman ps | grep guardrail`
- [ ] Postgres healthy: `podman logs guardrail-postgres | tail -5`
- [ ] Redis healthy: `podman logs guardrail-redis | tail -5`
- [ ] MCP server started: `podman logs guardrail-mcp-server | grep "Starting Guardrail MCP Server"`
- [ ] Database connected: `podman logs guardrail-mcp-server | grep "Database connected"`
- [ ] Redis connected: `podman logs guardrail-mcp-server | grep "Redis connected"`
- [ ] MCP endpoint responding: `curl -s http://localhost:8095/mcp`
- [ ] Web UI responding: `curl -s http://localhost:8096/`
- [ ] API key authentication working: `curl -H 'Authorization: Bearer YOUR_KEY' http://localhost:8095/mcp`
- [ ] Ports accessible from network: `curl http://0.0.0.0:8095/mcp`

## Maintenance

### Viewing Logs

```bash
# View all logs
podman logs guardrail-mcp-server

# Follow logs in real-time
podman logs -f guardrail-mcp-server

# View specific number of lines
podman logs --tail 50 guardrail-mcp-server

# View logs for all containers
podman logs guardrail-postgres
podman logs guardrail-redis
```

### Restarting Services

```bash
# Restart MCP server only
podman restart guardrail-mcp-server

# Restart all containers
podman restart guardrail-postgres guardrail-redis guardrail-mcp-server

# Recreate entire pod
podman pod stop guardrail-pod
podman pod rm guardrail-pod
# Then run deployment steps again
```

### Updating Configuration

```bash
# Update .env file
vim .env

# Restart containers to pick up changes
podman restart guardrail-mcp-server

# For database changes, also restart postgres
podman restart guardrail-postgres
```

### Backup and Restore

```bash
# Backup postgres data
podman run --rm \
  -v guardrail_pg_data:/data \
  -v $(pwd):/backup \
  alpine \
  tar czf /backup/postgres-backup.tar.gz /data

# Restore postgres data
podman run --rm \
  -v guardrail_pg_data:/data \
  -v $(pwd):/backup \
  alpine \
  tar xzf /backup/postgres-backup.tar.gz -C /
```

## Production Hardening

### Security Recommendations

1. **Change all default secrets** before production use
2. **Use strong random passwords**:
   ```bash
   openssl rand -hex 32  # For API keys
   openssl rand -base64 32  # For passwords
   ```
3. **Enable TLS** for database connections
4. **Set specific CORS origins** instead of "*"
5. **Enable production mode** for stricter security checks
6. **Use network policies** to restrict container communication
7. **Enable audit logging** and ship logs to central system
8. **Set resource limits** to prevent resource exhaustion

### Performance Tuning

```bash
# Database connection pooling
DB_MAX_OPEN_CONNS=25
DB_MAX_IDLE_CONNS=5
DB_CONN_MAX_LIFETIME=30m

# Redis connection pooling
REDIS_POOL_SIZE=10
REDIS_MIN_IDLE_CONNS=2

# Rate limiting (adjust based on load)
RATE_LIMIT_MCP=1000
RATE_LIMIT_IDE=500

# Cache TTL (adjust based on data volatility)
CACHE_TTL_RULES=5m
CACHE_TTL_DOCS=10m
```

## OpenCode Configuration

### MCP Server Configuration

Add to `.opencode/oh-my-opencode.jsonc`:

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

### Environment Variables

Create `.env.opencode`:

```bash
# MCP Server Connection
export MCP_SERVER_URL=http://0.0.0.0:8095
export MCP_API_KEY=DevKey123456789012345678901234567890
export IDE_API_KEY=DevKey456789012345678901234567890123

# Local Configuration (for OpenCode)
export GUARDRAILS_PROJECT_SLUG=your-project
export GUARDRAILS_AGENT_TYPE=opencode
```

## References

- [MCP Server README](./README.md)
- [API Documentation](./API.md)
- [Security Review](./OBSERVABILITY_REVIEW.md)
- [Dockerfile](./deploy/Dockerfile)
- [Podman Compose](./deploy/podman-compose.yml)
- [Kubernetes Deployment](./deploy/k8s-deployment.yaml)

## Support

For issues or questions:
1. Check troubleshooting section above
2. Review container logs: `podman logs guardrail-mcp-server`
3. Verify configuration against this guide
4. Check [GitHub Issues](https://github.com/TheArchitectit/agent-guardrails-template/issues)

## Changelog

### 2026-02-13 - Initial Deployment Guide
- Documented AI01 deployment process
- Added schema validation error fix
- Added postgres permission fix
- Added configuration requirements
- Added troubleshooting guide
- Added verification checklist