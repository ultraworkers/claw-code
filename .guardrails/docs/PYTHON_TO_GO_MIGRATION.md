# Python to Go Migration Guide

> Complete guide for migrating from Python team_manager.py to Go team package

**Version:** 2.6.0
**Last Updated:** 2026-02-15
**Applies To:** MCP Server v2.6.0+

---

## Overview

As of version 2.6.0, the Agent Guardrails MCP Server has completed its migration from Python to Go. All team management functionality previously provided by `scripts/team_manager.py` is now implemented natively in Go within the `mcp-server/internal/team/` package.

### Key Changes

| Aspect | Before (Python) | After (Go) |
|--------|-----------------|------------|
| **Language** | Python 3.11+ | Go 1.23+ |
| **Entry Point** | `scripts/team_manager.py` | `mcp-server/internal/team/` package |
| **Container** | Required Python runtime | Native Go binary (distroless) |
| **Dependencies** | `cryptography`, `stdlib` | None (built-in) |
| **Performance** | Process spawn overhead | Native in-process calls |

---

## What Was Migrated

### Core Components

1. **Team Management** (`manager.go`)
   - Team initialization, listing, assignment
   - Phase status tracking
   - Project lifecycle management

2. **Encryption** (`encryption.go`)
   - Fernet-based encryption at rest
   - Environment key derivation
   - Transparent encrypt/decrypt

3. **Validation** (`validation.go`)
   - Project name validation
   - Role name whitelist checking
   - Person name sanitization

4. **Rules Engine** (`rules.go`)
   - Team layout rules loading
   - Phase gate validation
   - Agent-to-team mapping

5. **Metrics** (`metrics.go`)
   - Team operation metrics
   - Performance tracking
   - Error categorization

6. **Types** (`types.go`)
   - Team data structures
   - Phase definitions
   - Assignment records

7. **Migrations** (`migrations.go`)
   - Data format migration
   - Version compatibility

---

## Developer Migration Guide

### For Contributors

#### Before (Python)

```python
# scripts/team_manager.py
from scripts.team_manager import TeamManager

manager = TeamManager("my-project")
manager.init_project()
manager.assign_role(team_id=7, role="Technical Lead", person="Alice")
```

#### After (Go)

```go
// mcp-server/internal/team/manager.go
package main

import (
    "context"
    "github.com/thearchitectit/guardrail-mcp/internal/team"
)

func main() {
    ctx := context.Background()
    mgr, err := team.NewManager("my-project")
    if err != nil {
        panic(err)
    }

    err = mgr.InitProject(ctx)
    if err != nil {
        panic(err)
    }

    err = mgr.AssignRole(ctx, 7, "Technical Lead", "Alice")
    if err != nil {
        panic(err)
    }
}
```

### API Changes

| Python (Old) | Go (New) | Notes |
|--------------|----------|-------|
| `TeamManager.init_project()` | `Manager.InitProject(ctx)` | Added context support |
| `TeamManager.list_teams()` | `Manager.ListTeams(ctx)` | Returns slice instead of dict |
| `TeamManager.assign_role()` | `Manager.AssignRole(ctx, teamID, role, person)` | Parameter order preserved |
| `TeamManager.unassign_role()` | `Manager.UnassignRole(ctx, teamID, role)` | - |
| `TeamManager.get_status()` | `Manager.GetStatus(ctx, phase)` | Phase is optional |
| `EncryptionManager.encrypt()` | `Encrypt(data, key)` | Standalone function |
| `EncryptionManager.decrypt()` | `Decrypt(data, key)` | Standalone function |

### Error Handling

Python exceptions are now Go errors:

```go
// Before (Python):
# try:
#     manager.assign_role(...)
# except ValueError as e:
#     print(f"Invalid: {e}")

// After (Go):
if err := mgr.AssignRole(ctx, 7, "Role", "Person"); err != nil {
    var valErr *team.ValidationError
    if errors.As(err, &valErr) {
        log.Printf("Invalid: %v", valErr)
    }
}
```

---

## Container Changes

### Before (Python Runtime Required)

```dockerfile
FROM gcr.io/distroless/python3-debian12
COPY scripts/ /app/scripts/
RUN pip install cryptography
CMD ["/server"]
```

### After (Pure Go)

```dockerfile
FROM gcr.io/distroless/static:nonroot
COPY server /server
ENTRYPOINT ["/server"]
```

**Benefits:**
- No Python runtime needed
- Smaller container size (~20MB vs ~80MB)
- No dependency management
- Faster startup time
- Reduced attack surface

---

## Deployment Migration

### Step 1: Update Docker Compose

Remove any Python-specific volumes or environment variables:

```yaml
# Before
services:
  mcp-server:
    volumes:
      - ./scripts:/app/scripts:ro
    environment:
      - PYTHONPATH=/app

# After
services:
  mcp-server:
    # No scripts volume needed
    environment:
      - TEAM_ENCRYPTION_KEY=${TEAM_ENCRYPTION_KEY}
```

### Step 2: Environment Variables

| Variable | Status | Notes |
|----------|--------|-------|
| `TEAM_ENCRYPTION_KEY` | **Kept** | Still used by Go encryption |
| `PYTHONPATH` | **Removed** | No longer needed |
| `TEAM_MANAGER_SCRIPT` | **Removed** | Hardcoded in binary |

### Step 3: Data Migration

Team data stored in `.teams/` is compatible:

```bash
# No data migration needed - JSON format unchanged
ls .teams/
# my-project.json
# my-project.lock
```

---

## Testing Changes

### Before (Python Tests)

```python
# scripts/test_team_manager.py
import unittest
from scripts.team_manager import TeamManager

class TestTeamManager(unittest.TestCase):
    def test_init_project(self):
        mgr = TeamManager("test-project")
        result = mgr.init_project()
        self.assertTrue(result["success"])
```

### After (Go Tests)

```go
// mcp-server/internal/team/manager_test.go
package team

import (
    "context"
    "testing"
)

func TestManager_InitProject(t *testing.T) {
    mgr, err := NewManager("test-project")
    if err != nil {
        t.Fatal(err)
    }

    ctx := context.Background()
    if err := mgr.InitProject(ctx); err != nil {
        t.Errorf("InitProject failed: %v", err)
    }
}
```

---

## Troubleshooting

### Common Issues

#### Issue: "team_manager.py not found"

**Cause:** Old code references Python script

**Solution:** Update to use Go package:

```go
// Replace exec.Command("python", "team_manager.py", ...)
// With:
mgr, _ := team.NewManager(projectName)
result, err := mgr.ListTeams(ctx)
```

#### Issue: Encryption key not working

**Cause:** Key format expectations changed

**Solution:** Ensure key is 32 bytes (Fernet format):

```bash
# Generate compatible key
openssl rand -base64 32
```

#### Issue: Build failures

**Cause:** Missing Go modules

**Solution:**

```bash
cd mcp-server
go mod download
go build ./cmd/server
```

---

## Performance Improvements

| Metric | Python | Go | Improvement |
|--------|--------|-----|-------------|
| **Startup Time** | ~500ms (Python init) | ~50ms | **10x faster** |
| **Memory Usage** | ~40MB | ~15MB | **2.7x less** |
| **Team Operation** | ~100ms | ~5ms | **20x faster** |
| **Container Size** | ~80MB | ~20MB | **4x smaller** |

---

## Backward Compatibility

### MCP Tool API

**Fully Compatible:** All MCP tool names, parameters, and responses remain unchanged.

- `guardrail_team_init`
- `guardrail_team_list`
- `guardrail_team_assign`
- `guardrail_team_unassign`
- `guardrail_team_status`
- `guardrail_phase_gate_check`
- `guardrail_agent_team_map`
- `guardrail_team_size_validate`

### Data Format

**Fully Compatible:** JSON data in `.teams/` directory unchanged.

### Configuration

**Mostly Compatible:** Only Python-specific env vars removed.

---

## Contributing

### Code Organization

```
mcp-server/internal/team/
├── manager.go      # Core team management
├── encryption.go   # Encryption utilities
├── validation.go   # Input validation
├── rules.go        # Layout rules
├── metrics.go      # Metrics collection
├── types.go        # Data structures
└── migrations.go   # Data migrations
```

### Testing

```bash
# Run all team tests
cd mcp-server
go test ./internal/team/...

# Run with coverage
go test -cover ./internal/team/...

# Run benchmarks
go test -bench=. ./internal/team/...
```

---

## References

- [Team Tools Reference](./TEAM_TOOLS.md) - Complete MCP tool documentation
- [TEAM_STRUCTURE.md](../ide/TEAM_STRUCTURE.md) - Team and role definitions
- [Go Package Documentation](../mcp-server/internal/team/) - Code-level docs

---

**Migration Status:** ✅ Complete as of v2.6.0

**Questions?** Open an issue on GitHub or refer to the troubleshooting section above.
