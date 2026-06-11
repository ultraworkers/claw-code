# Release v1.9.6 - SSE Compatibility and Web UI Packaging

**Release Date:** 2026-02-08  
**Branch:** `mcpserver`  
**Tag:** `v1.9.6`

---

## Summary

This patch release fixes MCP SSE interoperability with non-interactive clients (including Crush), ensures JSON-RPC responses are delivered on the SSE stream correctly, and packages Web UI static assets into the runtime image.

---

## Highlights

### MCP SSE fixes

- Keepalive changed from custom `event: ping` payloads to SSE comments (`: ping`)
- Session responses are now queued and emitted as `event: message` JSON-RPC payloads
- Session-aware message handling for notification, closed-session, and queue-backpressure paths

### Web UI and API behavior

- Runtime image now includes `/app/static` assets from the build stage
- Read-only web routes are publicly accessible:
  - `/api/documents*`
  - `/api/rules*`
  - `/version`

### Documentation updates

- Updated root `README.md`
- Updated `mcp-server/README.md`
- Updated root and MCP changelogs for v1.9.6

---

## Validation

### Endpoint checks

```bash
# SSE should return endpoint + keepalive comments
curl -sN --max-time 5 http://localhost:8092/mcp/v1/sse

# Message endpoint requires session_id
curl -s -X POST http://localhost:8092/mcp/v1/message \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
```

### Crush MCP integration

```json
"mcp": {
  "guardrails": {
    "type": "sse",
    "url": "http://<host>:8092/mcp/v1/sse"
  }
}
```

Expected: MCP tools are discovered successfully (`mcp_guardrails_*`).

---

## Deployment note

After updating the image, recreate the `guardrail-mcp-server` container to pick up the new binary and bundled Web UI assets.

---

## Links

- Changelog: `CHANGELOG.md`
- MCP changelog: `mcp-server/CHANGELOG.md`
- MCP server docs: `mcp-server/README.md`
