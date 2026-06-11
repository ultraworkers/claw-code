# Release v1.9.2 - Web UI Authentication Fix

**Release Date:** 2026-02-07
**Branch:** `mcpserver-impl`
**Tag:** `v1.9.2`

---

## Summary

Bugfix release that removes the API key authentication requirement from the Web UI. The Web UI (port 8093) is now publicly accessible as originally intended.

---

## Problem

After deploying v1.9.1, accessing the Web UI returned an authentication error:
```json
{"message":"Missing authorization header"}
```

This was incorrect - the Web UI should be publicly accessible without authentication.

---

## Root Cause

The `APIKeyAuth` middleware in `mcp-server/internal/web/middleware.go` was applied globally to all routes, but only had exceptions for:
- `/health/live`
- `/health/ready`
- `/metrics`

Web UI routes were not included in the skip list.

---

## Solution

Added Web UI routes to the authentication skip list:

```go
// Skip Web UI routes - these are publicly accessible
if path == "/" || path == "/index.html" || strings.HasPrefix(path, "/static/") {
    return next(c)
}
```

Applied to both:
1. `APIKeyAuth` middleware - skips API key validation
2. `RateLimitMiddleware` - skips API key-based rate limiting

---

## Files Changed

| File | Lines | Change |
|------|-------|--------|
| `mcp-server/internal/web/middleware.go` | +10/-2 | Skip auth for Web UI routes |
| `CHANGELOG.md` | +10/-0 | Release notes |

---

## Migration from v1.9.1

1. Build new image: `docker build -t guardrail-mcp:latest -f deploy/Dockerfile .`
2. Deploy to your server: Transfer image and restart container
3. Verify: Access the Web UI without API key

---

## Testing

### Web UI (should work without auth)
```bash
curl http://localhost:8093/
curl http://localhost:8093/index.html
curl http://localhost:8093/static/style.css
```

### API endpoints (still require auth)
```bash
# Should fail with 401
curl http://localhost:8092/mcp/v1/sse

# Should succeed with valid key
curl -H "Authorization: Bearer $MCP_API_KEY" http://localhost:8092/mcp/v1/sse
```

---

## Links

- **Release:** https://github.com/TheArchitectit/agent-guardrails-template/releases/tag/v1.9.2
- **Branch:** https://github.com/TheArchitectit/agent-guardrails-template/tree/mcpserver-impl
- **Previous Release:** [v1.9.1](RELEASE_v1.9.1.md)

---

## Credits

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
