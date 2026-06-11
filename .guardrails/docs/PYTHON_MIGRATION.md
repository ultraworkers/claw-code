# Python to Go Migration Guide

> Complete guide for the Python to Go migration completed in v2.6.0

**Version:** 2.6.0
**Last Updated:** 2026-02-15

---

## Table of Contents

1. [Why We Migrated](#why-we-migrated)
2. [What's Different](#whats-different)
3. [API Compatibility](#api-compatibility)
4. [How to Contribute](#how-to-contribute)
5. [Migration FAQ](#migration-faq)

---

## Why We Migrated

### Security

The migration to Go enables significant security improvements:

- **Distroless Containers**: Go compiles to a single static binary, allowing us to use distroless container images
  - No shell access
  - No package manager
  - Minimal attack surface
  - Non-root execution with dropped capabilities

- **Memory Safety**: Go's memory management prevents common vulnerabilities found in Python
  - No buffer overflows
  - Type safety at compile time
  - No interpreter vulnerabilities

### Container Size

| Metric | Python | Go | Improvement |
|--------|--------|-----|-------------|
| Container Size | ~500MB | ~50MB | **10x smaller** |
| Layers | Multiple | Single | Simpler |
| Attack Surface | Large | Minimal | **Hardened** |

### Performance

| Metric | Python | Go | Improvement |
|--------|--------|-----|-------------|
| Startup Time | ~3 seconds | ~100ms | **30x faster** |
| Memory Usage | ~200MB | ~20MB | **10x less** |
| Concurrency | GIL-limited | Native goroutines | **Unlimited** |
| Cold Start | Slow | Fast | **Better scaling** |

### Deployment Compatibility

Go enables deployment in restricted environments:
- **Distroless**: Google's hardened container images
- **Scratch**: Truly minimal containers
- **Read-only filesystems**: No runtime writes needed
- **Locked-down environments**: No interpreter needed

---

## What's Different

### Code Location

```
# Before (Python)
scripts/
├── team_manager.py      # Team management logic
├── export_teams.py      # Data export
└── migrate_config.py    # Configuration migration

# After (Go)
mcp-server/internal/
├── team/
│   ├── manager.go       # Team management
│   ├── types.go         # Data structures
│   ├── encryption.go    # Encryption at rest
│   ├── validation.go    # Input validation
│   ├── rules.go         # Layout rules
│   └── metrics.go       # Operation metrics
├── database/
│   └── migrations/      # golang-migrate files
└── cmd/tools/           # CLI utilities
```

### Dependencies

```bash
# Before (Python)
pip install -r requirements.txt
# 20+ dependencies
# Virtual environments
# Version conflicts

# After (Go)
go mod download
# Compiled into single binary
# No runtime dependencies
# Static linking
```

### Build Process

```bash
# Before (Python)
# No build step required
python scripts/team_manager.py

# After (Go)
# Compile to binary
cd mcp-server
go build -o bin/server ./cmd/server
./bin/server
```

### Database Migrations

```bash
# Before (Python)
python scripts/migrate_db.py --version 2.0.0

# After (Go)
# Using golang-migrate
cd mcp-server
export DATABASE_URL="postgresql://..."
make migrate-up
```

---

## API Compatibility

### MCP Tools

**Fully Compatible:** All MCP tools work identically.

| Tool | Status | Notes |
|------|--------|-------|
| `guardrail_team_init` | ✅ Unchanged | Go implementation, same API |
| `guardrail_team_list` | ✅ Unchanged | Go implementation, same API |
| `guardrail_team_assign` | ✅ Unchanged | Go implementation, same API |
| `guardrail_team_status` | ✅ Unchanged | Go implementation, same API |
| `guardrail_phase_gate_check` | ✅ Unchanged | Go implementation, same API |

### Data Format

Team configuration files (`.teams/*.json`) remain unchanged:

```json
{
  "version": "2.0",
  "project": "example",
  "teams": {
    "team-1": {
      "name": "Core Feature Squad",
      "phase": 3,
      "members": {
        "lead": "alice",
        "developers": ["bob", "charlie"]
      }
    }
  }
}
```

### REST API

All REST endpoints remain compatible:
- `GET /api/teams`
- `POST /api/teams`
- `GET /api/teams/:id`
- `PUT /api/teams/:id`
- `DELETE /api/teams/:id`

---

## How to Contribute

### Development Setup

```bash
# Clone the repository
git clone https://github.com/TheArchitectit/agent-guardrails-template.git
cd agent-guardrails-template/mcp-server

# Install Go dependencies
make deps

# Run tests
make test

# Build the server
make build
```

### Go Development Workflow

```bash
# Format code
make fmt

# Run linter
make lint

# Run tests
make test

# Check for vulnerabilities
make vuln

# Full check
make check
```

### Writing Go Code

Follow these conventions:

1. **Package names**: Short, lowercase (e.g., `team`, `rules`)
2. **Exported names**: PascalCase
3. **Unexported names**: camelCase
4. **Error handling**: Always check, wrap with context

```go
// Example: Team assignment
type Manager struct {
    db    *database.Store
    cache cache.Cache
}

func (m *Manager) AssignRole(
    ctx context.Context,
    project, team, role, person string,
) error {
    // Validate input
    if err := validate.Role(role); err != nil {
        return fmt.Errorf("invalid role: %w", err)
    }

    // Check team capacity
    count, err := m.db.CountMembers(ctx, project, team)
    if err != nil {
        return fmt.Errorf("failed to count members: %w", err)
    }

    if count >= maxTeamSize {
        return ErrTeamFull
    }

    // Perform assignment
    if err := m.db.AssignRole(ctx, project, team, role, person); err != nil {
        return fmt.Errorf("failed to assign role: %w", err)
    }

    return nil
}
```

### Testing

```go
func TestManager_AssignRole(t *testing.T) {
    tests := []struct {
        name    string
        project string
        team    string
        role    string
        person  string
        wantErr error
    }{
        // Test cases
    }

    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            m := NewManager(mockDB, mockCache)
            err := m.AssignRole(context.Background(),
                tt.project, tt.team, tt.role, tt.person)
            if !errors.Is(err, tt.wantErr) {
                t.Errorf("AssignRole() error = %v, wantErr %v",
                    err, tt.wantErr)
            }
        })
    }
}
```

---

## Migration FAQ

### Q: Do I need to learn Go to use the MCP server?

**A:** No. The MCP server is a black box from the client perspective. All interactions are through the MCP protocol or REST API.

### Q: Will my existing `.teams/*.json` files work?

**A:** Yes. The data format is unchanged. The Go implementation reads and writes the same JSON structure.

### Q: What happened to `scripts/team_manager.py`?

**A:** The functionality has been migrated to `mcp-server/internal/team/`. The Python script is deprecated and will be removed in v3.0.0.

### Q: Do I need to install Go to run the server?

**A:** No. You can use the pre-built Docker image which contains the compiled Go binary.

### Q: How do I build from source?

**A:**
```bash
cd mcp-server
go build -o bin/server ./cmd/server
./bin/server
```

### Q: Are there any breaking changes?

**A:** No breaking changes from the MCP client perspective. The API is fully compatible.

### Q: What's the performance impact?

**A:** Positive. The Go implementation is faster, uses less memory, and has faster startup times.

### Q: Can I still use Python for other things?

**A:** Yes. Only the team management and MCP server have migrated to Go. Other Python scripts in `scripts/` remain available.

### Q: How do I debug the Go server?

**A:**
```bash
# Build with debug symbols
go build -gcflags="-N -l" -o bin/server ./cmd/server

# Run with delve debugger
dlv exec ./bin/server

# Or enable debug logging
LOG_LEVEL=debug ./bin/server
```

### Q: What Go version is required?

**A:** Go 1.23 or later. See `go.mod` for exact requirements.

---

## References

- [CONTRIBUTING.md](CONTRIBUTING.md) - Development guidelines
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture
- [MIGRATION.md](MIGRATION.md) - Version migration procedures
- [Go Documentation](https://golang.org/doc/)
- [Effective Go](https://golang.org/doc/effective_go.html)

---

**Last Updated:** 2026-02-15
**Version:** 2.6.0
**Implementation:** Go (mcp-server/internal/)
