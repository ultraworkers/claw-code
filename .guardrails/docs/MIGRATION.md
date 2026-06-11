# Migration Guide

> Version compatibility, migration instructions, and rollback procedures

**Version:** 1.0
**Last Updated:** 2026-02-15

---

## Table of Contents

1. [Version Compatibility Matrix](#version-compatibility-matrix)
2. [Breaking Changes by Version](#breaking-changes-by-version)
3. [Migration Procedures](#migration-procedures)
4. [Rollback Procedures](#rollback-procedures)
5. [Migration Examples](#migration-examples)
6. [Troubleshooting Migrations](#troubleshooting-migrations)

---

## Version Compatibility Matrix

### Current Version Support

| Version | Status | Support End | Compatible With | Implementation |
|---------|--------|-------------|-----------------|----------------|
| 2.6.x | Current | 2027-02-15 | 2.0.x, 1.10.x | **Go** |
| 2.0.x | Maintained | 2026-10-15 | 1.10.x, 1.9.x | Go |
| 1.10.x | Maintained | 2026-08-15 | 1.9.x | Go |
| 1.9.x | Maintained | 2026-06-15 | 1.8.x | Go |
| 1.8.x | Deprecated | 2026-04-15 | 1.7.x | Python |
| < 1.8.0 | End of Life | - | - | Python |

### Compatibility Legend

| Symbol | Meaning |
|--------|---------|
| Full | All features compatible |
| Partial | Some features require configuration |
| Breaking | Requires migration steps |
| N/A | Not compatible |

### MCP Server Compatibility

| Client Version | MCP Server 1.9 | MCP Server 1.10 | MCP Server 2.0 |
|----------------|----------------|-----------------|----------------|
| Claude Code 1.x | Full | Full | Full |
| Claude Code 2.x | Partial | Full | Full |
| OpenCode 1.x | Full | Full | Full |
| Cursor 1.x | N/A | Partial | Full |
| Custom Clients | Breaking | Partial | Full |

### Database Compatibility

| Database Version | Schema Version | Migration Required |
|------------------|----------------|------------------|
| PostgreSQL 15 | v1.8 | Yes |
| PostgreSQL 16 | v1.9+ | No |
| Redis 6 | v1.8 | Yes |
| Redis 7 | v1.9+ | No |

---

## Breaking Changes by Version

### v2.6.0 (Current) - Go Migration

**Release Date:** 2026-02-15

#### Breaking Changes

1. **Language Migration: Python to Go**
   - **Old:** `scripts/team_manager.py` (Python)
   - **New:** `mcp-server/internal/team/` (Go package)
   - **Impact:** No runtime Python required
   - **API:** Unchanged from MCP client perspective

2. **Build Process**
   - Old: `pip install -r requirements.txt`
   - New: `go build ./cmd/server`
   - Binary: Single static binary vs Python interpreter

3. **Container**
   - Old: Python-based image (~500MB)
   - New: Distroless Go image (~50MB)
   - Security: Non-root, read-only filesystem, dropped capabilities

#### Migration Benefits

| Metric | Python | Go | Improvement |
|--------|--------|-----|-------------|
| Container Size | ~500MB | ~50MB | **10x smaller** |
| Startup Time | ~3s | ~100ms | **30x faster** |
| Memory Usage | ~200MB | ~20MB | **10x less** |
| Security | Full OS | Distroless | **Hardened** |

#### New Features

- Team size validation (TEAM-007 compliance)
- Phase gate automation
- Agent team mapping
- Extended MCP tools (5 new tools)
- Hot-reloadable configuration
- Circuit breaker patterns

---

### v2.0.0

**Release Date:** 2026-02-15

#### Breaking Changes

1. **Team Configuration Schema v2**
   - New required field: `team_version`
   - Changed `members` from array to object structure
   - Added `metadata` field for custom properties

2. **API Endpoint Changes**
   - `/mcp/v1/message` - Now requires `session_id` parameter
   - `/mcp/v1/sse` - Changed event format

3. **Environment Variables**
   - `MCP_PORT` renamed to `MCP_SERVER_PORT`
   - `WEB_PORT` renamed to `WEB_UI_PORT`
   - New required: `TEAM_CONFIG_VERSION`

#### New Features

- Team size validation (TEAM-007 compliance)
- Phase gate automation
- Agent team mapping
- Extended MCP tools (5 new tools)

---

### v1.10.0

**Release Date:** 2026-02-08

#### Breaking Changes

None. This is a backward-compatible release.

#### New Features

- 5 new MCP tools (`guardrail_validate_scope`, etc.)
- 6 new MCP resources
- Web UI Management Interface
- Documentation search functionality

#### Migration Notes

- All changes are additive
- No configuration changes required
- New tools available immediately after upgrade

---

### v1.9.0

**Release Date:** 2026-02-07

#### Breaking Changes

1. **MCP Protocol Migration**
   - Moved from custom protocol to standard MCP
   - Port changed: 8094 (SSE), 8095 (message)
   - Authentication now requires `Authorization` header

2. **Configuration Structure**
   - `.teams/` directory location changed
   - New required files: `.guardrails/rules.json`

3. **Tool Names**
   - `validate_bash` renamed to `guardrail_validate_bash`
   - `validate_git` renamed to `guardrail_validate_git_operation`
   - `validate_file` renamed to `guardrail_validate_file_edit`

#### New Features

- Full MCP server implementation
- SSE transport support
- PostgreSQL and Redis backends
- Production deployment support

---

### v1.8.0 to v1.9.0

**Critical:** This is a major protocol change. Plan for downtime.

#### Breaking Changes

1. **Port Configuration**
   - Old: Port 8094 (custom protocol)
   - New: Port 8092 (MCP SSE), 8093 (Web UI)

2. **Client Configuration**
   - Old: Direct HTTP calls
   - New: MCP protocol with SSE

---

## Migration Procedures

### Pre-Migration Checklist

Before starting any migration:

```bash
# 1. Backup current state
./scripts/backup.sh --full

# 2. Verify backup integrity
./scripts/verify_backup.sh /backups/guardrails-$(date +%Y%m%d).tar.gz

# 3. Check current version
git describe --tags

# 4. Review breaking changes
cat docs/MIGRATION.md | grep -A 20 "v$(TARGET_VERSION)"

# 5. Test in staging environment
./scripts/test_migration.sh --version $TARGET_VERSION
```

---

### Migrating to v2.6.0 (Go Implementation)

**Go Migration:** The MCP server and team management have been migrated from Python to Go.
- **Benefits:** Smaller container size, distroless compatibility, improved security
- **API Compatibility:** Unchanged from MCP perspective
- **Location:** Go code is in `mcp-server/internal/`

**Estimated Time:** 30-45 minutes
**Downtime Required:** Yes (5-10 minutes)

#### Step 1: Pre-Migration (5 min)

```bash
# Stop the MCP server
pkill -f mcp_server || true

# Create full backup
mkdir -p backups/$(date +%Y%m%d)
cp -r .teams/ .guardrails/ backups/$(date +%Y%m%d)/
cp .env backups/$(date +%Y%m%d)/

# Export team configurations (Go binary)
cd mcp-server && go run ./cmd/tools/export_teams.go --format json > ../backups/$(date +%Y%m%d)/teams_export.json && cd ..
```

#### Step 2: Update Configuration (10 min)

```bash
# Update environment variables
# OLD:
# MCP_PORT=8094
# WEB_PORT=8093

# NEW:
cat >> .env << 'EOF'
# v2.6.0 Configuration (Go Implementation)
MCP_SERVER_PORT=8094
WEB_UI_PORT=8093
TEAM_CONFIG_VERSION=2
EOF

# Update team configuration schema (Go binary)
cd mcp-server && go run ./cmd/tools/migrate_config.go --from-version 1 --to-version 2 && cd ..
```

#### Step 3: Database Migration (10 min)

```bash
# Run database migrations (using golang-migrate)
cd mcp-server
export DATABASE_URL="postgresql://guardrails:password@localhost:5432/guardrails?sslmode=disable"
make migrate-up

# Verify migration
psql -U guardrails -d guardrails -c "\dt"
psql -U guardrails -d guardrails -c "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1;"
```

#### Step 4: Deploy New Version (5 min)

```bash
# Pull new version
git fetch origin
git checkout v2.6.0

# Build Go binary
cd mcp-server
go build -o bin/server ./cmd/server
cd ..
```

#### Step 5: Post-Migration (5 min)

```bash
# Start server
./start-mcp-server.sh

# Verify health
curl -s http://localhost:8094/mcp/v1/health | jq .

# Test team operations
curl -X POST http://localhost:8094/mcp/v1/message \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"guardrail_team_list","arguments":{"project_name":"test-project"}}}'

# Verify team size validation
curl -X POST http://localhost:8094/mcp/v1/message \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"guardrail_team_size_validate","arguments":{"project_name":"test-project"}}}'
```

#### Step 6: Update Client Configurations

**Claude Code:**
```json
// .claude/settings.json
{
  "mcpServers": {
    "guardrails": {
      "url": "http://localhost:8094/mcp/v1/sse",
      "headers": {
        "Authorization": "Bearer YOUR_API_KEY"
      }
    }
  }
}
```

**OpenCode:**
```jsonc
// .opencode/oh-my-opencode.jsonc
{
  "mcp": {
    "servers": [
      {
        "name": "guardrails",
        "url": "http://localhost:8094/mcp/v1/sse",
        "apiKey": "YOUR_API_KEY"
      }
    ]
  }
}
```

---

### Migrating to v1.10.0

**Estimated Time:** 15-20 minutes
**Downtime Required:** Minimal (rolling update possible)

#### Step 1: Backup

```bash
cp -r .teams/ .teams-backup-$(date +%Y%m%d)/
cp .env .env.backup-$(date +%Y%m%d)
```

#### Step 2: Update Code

```bash
git fetch origin
git checkout v1.10.0
pip install -r requirements.txt
cd mcp-server && go build ./cmd/server && cd ..
```

#### Step 3: Restart Server

```bash
# Rolling restart (no downtime)
pkill -HUP -f mcp_server

# Or full restart
pkill -f mcp_server
./mcp-server/cmd/server/server
```

#### Step 4: Verify New Features

```bash
# Access Web UI
open http://localhost:8080/web

# Test new tools
curl -X POST http://localhost:8092/mcp/v1/message \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"guardrail_validate_scope","arguments":{"path":"src/main.py","allowed_paths":["src/","tests/"]}}}'
```

---

### Migrating to v1.9.0

**Estimated Time:** 45-60 minutes
**Downtime Required:** Yes (protocol change)

#### Step 1: Full Backup

```bash
./scripts/full_backup.sh --output backups/pre-mcp-migration/
```

#### Step 2: Prepare New Infrastructure

```bash
# Setup PostgreSQL and Redis
# See deployment guide in RELEASE_v1.9.0.md

# Create new environment file
cat > .env.v1.9.0 << 'EOF'
MCP_API_KEY=<generate with: openssl rand -hex 32>
IDE_API_KEY=<generate with: openssl rand -hex 32>
JWT_SECRET=<generate with: openssl rand -hex 48>

DB_HOST=postgres
DB_PORT=5432
DB_NAME=guardrails
DB_USER=guardrails
DB_PASSWORD=<secure password>
DB_SSLMODE=disable

REDIS_HOST=redis
REDIS_PORT=6379
REDIS_PASSWORD=<secure password>
REDIS_USE_TLS=false

MCP_PORT=8080
WEB_PORT=8081
WEB_ENABLED=true
LOG_LEVEL=info
REQUEST_TIMEOUT=30s
EOF
```

#### Step 3: Deploy New Server

```bash
# Build container
cd mcp-server
docker build -t guardrail-mcp:v1.9.0 -f deploy/Dockerfile .

# Deploy
docker-compose -f deploy/docker-compose.yml up -d
```

#### Step 4: Migrate Data

```bash
# Export from old format
python scripts/export_v1.8.py > migration_data.json

# Import to new format
python scripts/import_v1.9.py --input migration_data.json
```

#### Step 5: Update Clients

Update all client configurations to use new MCP endpoints:
- Old: `http://localhost:8094`
- New: `http://localhost:8092/mcp/v1/sse`

---

## Rollback Procedures

### Automatic Rollback Triggers

The system will automatically rollback if:

1. Health checks fail for 5 consecutive attempts
2. Database migration checksums don't match
3. Team configuration validation fails

### Manual Rollback

#### Rollback from v2.6.0 to v2.0.0

```bash
#!/bin/bash
# rollback_to_v2.0.0.sh

set -e

echo "Starting rollback to v2.0.0..."

# 1. Stop current server
pkill -f mcp_server || true

# 2. Restore configuration from backup
BACKUP_DIR="backups/$(ls -t backups/ | head -1)"
cp "$BACKUP_DIR/.env" .env

# 3. Restore team configurations
cp -r "$BACKUP_DIR/.teams/" .
cp -r "$BACKUP_DIR/.guardrails/" .

# 4. Checkout previous version
git checkout v2.0.0

# 5. Rebuild Go binary
cd mcp-server
go build -o bin/server ./cmd/server
cd ..

# 6. Start server
./mcp-server/bin/server &

# 7. Verify
echo "Waiting for server..."
sleep 5
curl -s http://localhost:8094/mcp/v1/health && echo "Rollback successful!"
```

#### Rollback Database

```bash
# Rollback one migration (golang-migrate)
cd mcp-server
make migrate-down

# Or restore from backup
createdb guardrails_backup
gunzip < backups/postgres-$(date +%Y%m%d).sql.gz | psql guardrails_backup
```

#### Emergency Rollback

If the system is completely broken:

```bash
#!/bin/bash
# emergency_rollback.sh

echo "EMERGENCY ROLLBACK INITIATED"

# Stop everything
pkill -9 -f mcp_server || true
docker-compose down || true

# Restore from latest backup
LATEST_BACKUP=$(ls -td backups/*/ | head -1)
echo "Restoring from: $LATEST_BACKUP"

# Restore files
cp -r "$LATEST_BACKUP/." .

# Checkout last known good version (Go implementation)
git checkout v2.0.0

# Rebuild and start
cd mcp-server
go build -o bin/server ./cmd/server
cd ..
nohup ./mcp-server/bin/server > mcp.log 2>&1 &

echo "Emergency rollback complete"
echo "Check logs: tail -f mcp.log"
```

---

## Migration Examples

### Example 1: Single Project Migration

```bash
# Migrate single project from v1.9.0 to v2.0.0
PROJECT_NAME="my-web-app"

# Step 1: Export project config
python scripts/export_project.py "$PROJECT_NAME" > "$PROJECT_NAME-v1.9.json"

# Step 2: Transform to v2.0.0 format
python scripts/transform_team_config.py \
  --input "$PROJECT_NAME-v1.9.json" \
  --from-version 1.9 \
  --to-version 2.0 \
  --output "$PROJECT_NAME-v2.0.json"

# Step 3: Validate new format
python scripts/validate_team_config.py "$PROJECT_NAME-v2.0.json"

# Step 4: Import to v2.0.0
python scripts/import_project.py "$PROJECT_NAME-v2.0.json"

# Step 5: Verify
./scripts/verify_project.sh "$PROJECT_NAME"
```

### Example 2: Batch Migration Script

```bash
#!/bin/bash
# migrate_all_projects.sh

set -e

FROM_VERSION="1.9.0"
TO_VERSION="2.0.0"
FAILED_LOG="migration_failed_$(date +%Y%m%d).log"
SUCCESS_COUNT=0
FAIL_COUNT=0

echo "Starting batch migration from $FROM_VERSION to $TO_VERSION"

# Get all projects
PROJECTS=$(ls .teams/*.json | xargs -n1 basename | sed 's/.json$//')

for PROJECT in $PROJECTS; do
    echo "Migrating $PROJECT..."

    if python scripts/migrate_project.py \
        --project "$PROJECT" \
        --from-version "$FROM_VERSION" \
        --to-version "$TO_VERSION" \
        --backup; then

        echo "  SUCCESS: $PROJECT"
        ((SUCCESS_COUNT++))
    else
        echo "  FAILED: $PROJECT"
        echo "$PROJECT" >> "$FAILED_LOG"
        ((FAIL_COUNT++))
    fi
done

echo ""
echo "Migration Summary:"
echo "  Successful: $SUCCESS_COUNT"
echo "  Failed: $FAIL_COUNT"

if [ $FAIL_COUNT -gt 0 ]; then
    echo "Failed projects logged to: $FAILED_LOG"
    exit 1
fi
```

### Example 3: Zero-Downtime Migration

For v1.10.0 (no breaking changes):

```bash
#!/bin/bash
# zero_downtime_migration.sh

# Start new version on different port
MCP_PORT=8096 WEB_PORT=8097 ./mcp-server/cmd/server/server &
NEW_PID=$!

# Wait for health check
for i in {1..30}; do
    if curl -s http://localhost:8096/mcp/v1/health; then
        echo "New server ready"
        break
    fi
    sleep 1
done

# Switch load balancer to new port
sudo sed -i 's/8094/8096/g' /etc/nginx/conf.d/mcp.conf
sudo nginx -s reload

# Stop old server
pkill -f "mcp_server.*8094"

# Update to use standard port
kill $NEW_PID
MCP_PORT=8094 ./mcp-server/cmd/server/server &
sudo sed -i 's/8096/8094/g' /etc/nginx/conf.d/mcp.conf
sudo nginx -s reload

echo "Zero-downtime migration complete"
```

---

## Troubleshooting Migrations

### Common Migration Issues

#### Issue: "Team configuration version mismatch"

**Cause:** Migration script didn't update all config files

**Solution:**
```bash
# Force version update (Go binary)
cd mcp-server
go run ./cmd/tools/update_config.go --version 2.0

# Re-run migration
go run ./cmd/tools/migrate_config.go --from-version 1 --to-version 2 --force
```

#### Issue: "Database migration failed"

**Cause:** Partial migration or checksum mismatch

**Solution:**
```bash
# Check migration status
cd mcp-server
make migrate-status

# Fix by marking as applied
migrate -path internal/database/migrations -database "$DATABASE_URL" force 20260215000001

# Or rollback and retry
make migrate-down
make migrate-up
```

#### Issue: "Port already in use"

**Cause:** Old server still running

**Solution:**
```bash
# Find and kill old process
lsof -ti:8094 | xargs kill -9

# Or use different port temporarily
MCP_PORT=8096 ./mcp-server/cmd/server/server
```

#### Issue: "Client connection refused"

**Cause:** Client configured for old endpoint

**Solution:**
```bash
# Update client configuration
./scripts/update_client_configs.sh --new-port 8094 --new-path /mcp/v1/sse

# Verify connectivity
curl -H "Authorization: Bearer $API_KEY" \
  http://localhost:8094/mcp/v1/health
```

### Migration Verification

```bash
#!/bin/bash
# verify_migration.sh

echo "Verifying migration..."

# Check server health
echo -n "Health check: "
curl -sf http://localhost:8094/mcp/v1/health && echo "PASS" || echo "FAIL"

# Check version
echo -n "Version check: "
git describe --tags | grep -q "v2.0" && echo "PASS" || echo "FAIL"

# Check database
echo -n "Database check: "
psql -U guardrails -c "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1;" | grep -q "20260215" && echo "PASS" || echo "FAIL"

# Check team configs
echo -n "Team config check: "
python scripts/validate_all_configs.py && echo "PASS" || echo "FAIL"

# Test basic operation
echo -n "Operation check: "
curl -s -X POST http://localhost:8094/mcp/v1/message \
  -d '{"jsonrpc":"2.0","method":"tools/list"}' | grep -q "guardrail_team" && echo "PASS" || echo "FAIL"

echo "Verification complete"
```

---

**Last Updated:** 2026-02-15
**Version:** 1.0
