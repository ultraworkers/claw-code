# Release v1.9.1 - MCP Server Production Ready

**Release Date:** 2026-02-07
**Branch:** `mcpserver-impl`
**Tag:** `v1.9.1`

---

## Summary

Production-ready MCP Server release with critical fixes for SSE compatibility and PostgreSQL array handling. This release addresses the EOF errors experienced with non-interactive clients and fixes database scanning issues.

---

## Critical Fixes

### 1. SSE Compatibility (mcp-server/internal/mcp/server.go)

**Problem:** Crush and other non-interactive clients received EOF errors when connecting to the SSE endpoint.

**Root Cause:**
- Missing explicit `WriteHeader()` - headers weren't committed immediately
- No initial data sent - clients timed out waiting for first event
- Missing proxy buffering headers
- No CORS support

**Solution:**
```go
// Set SSE headers with proper configuration
c.Response().Header().Set("Content-Type", "text/event-stream")
c.Response().Header().Set("Cache-Control", "no-cache, no-store, must-revalidate")
c.Response().Header().Set("Connection", "keep-alive")
c.Response().Header().Set("X-Accel-Buffering", "no")  // Disable buffering
c.Response().Header().Set("Access-Control-Allow-Origin", "*")
c.Response().WriteHeader(http.StatusOK)  // Commit headers immediately

// Send immediate ping to confirm connection
fmt.Fprintf(c.Response(), "event: ping\ndata: {}\n\n")
c.Response().Flush()
```

### 2. PostgreSQL Array Scanning (mcp-server/internal/models/failure.go)

**Problem:** `sql: Scan error on column index 6, name "affected_files": unsupported Scan`

**Root Cause:** Used `pq.StringArray` from lib/pq driver, but application uses pgx v5.

**Solution:**
```go
// Changed from:
AffectedFiles pq.StringArray

// To:
AffectedFiles pgtype.Array[string]

// Added helper functions:
func ToStringSlice(arr pgtype.Array[string]) []string
func ToTextArray(slice []string) pgtype.Array[string]
```

---

## Files Changed

| File | Lines | Change |
|------|-------|--------|
| `mcp-server/internal/mcp/server.go` | +37/-16 | SSE compatibility fixes |
| `mcp-server/internal/models/failure.go` | +27/-14 | PostgreSQL array fix |
| `README.md` | +223/-0 | Complete documentation |
| `CHANGELOG.md` | +19/-0 | Release notes |

---

## Deployment Status

**Production Deployment:**
```
✅ Container running on your-server:8092 (MCP) and :8093 (Web UI)
✅ PostgreSQL 16 with initialized schema
✅ Redis 7 for caching
✅ 4 prevention rules active
```

---

## Testing

### SSE Endpoint
```bash
curl -N http://localhost:8092/mcp/v1/sse
```

Expected:
```
event: endpoint
data: /mcp/v1/message?session_id=sess_...

event: ping
data: {}
```

### MCP Initialize
```bash
curl -X POST http://localhost:8092/mcp/v1/message \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
```

---

## Migration from v1.9.0

No migration needed - v1.9.1 is a bugfix release with the same API.

1. Build new image: `docker build -t guardrail-mcp:latest -f deploy/Dockerfile .`
2. Deploy: `podman-compose up -d`
3. Test: `curl http://localhost:8092/mcp/v1/sse`

---

## Links

- **Release:** https://github.com/TheArchitectit/agent-guardrails-template/releases/tag/v1.9.1
- **Branch:** https://github.com/TheArchitectit/agent-guardrails-template/tree/mcpserver-impl
- **Full Docs:** [README.md](https://github.com/TheArchitectit/agent-guardrails-template/blob/mcpserver-impl/README.md)

---

## Credits

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
