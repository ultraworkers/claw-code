# Contributing Guide

> Development guidelines for the Agent Guardrails Template

**Version:** 2.6.0
**Last Updated:** 2026-02-15

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [Go Development Workflow](#go-development-workflow)
3. [Code Standards](#code-standards)
4. [Testing](#testing)
5. [Commit Guidelines](#commit-guidelines)
6. [Migration Notice](#migration-notice)

---

## Getting Started

> **Important:** All future development is in **Go**. Python implementation is deprecated as of v2.6.0.

### Prerequisites

- Go 1.23+
- Docker or Podman
- PostgreSQL 16 (for local development)
- Redis 7 (for local development)
- Make

### Repository Structure

```
mcp-server/
├── cmd/
│   └── server/          # Main application entry point
├── internal/
│   ├── team/           # Team management logic (Go)
│   ├── rules/          # Rule engine
│   ├── audit/          # Audit logging
│   ├── database/       # Database operations
│   ├── cache/          # Redis caching
│   ├── mcp/            # MCP protocol implementation
│   ├── web/            # HTTP handlers
│   ├── security/       # Secrets scanning
│   └── validation/     # Input validation
├── deploy/             # Deployment files
└── Makefile           # Build automation
```

---

## Go Development Workflow

### Building

```bash
cd mcp-server

# Build the server binary
make build

# Build for production
make build-prod

# Clean build artifacts
make clean
```

### Running Locally

```bash
# Install dependencies
make deps

# Run database migrations
export DATABASE_URL="postgresql://guardrails:password@localhost:5432/guardrails?sslmode=disable"
make migrate-up

# Run the server (requires PostgreSQL and Redis)
make dev

# Or run directly
go run ./cmd/server
```

### Code Quality

```bash
# Format all Go code
make fmt

# Run linter (golangci-lint)
make lint

# Run tests
make test

# Run tests with coverage
make test-cover

# Check for vulnerabilities
make vuln

# Full check (fmt + lint + test)
make check
```

---

## Code Standards

### Go Code Style

All code must pass the following checks:

1. **gofmt** - Standard Go formatting
   ```bash
   gofmt -w .
   ```

2. **go vet** - Static analysis
   ```bash
   go vet ./...
   ```

3. **golangci-lint** - Comprehensive linting
   ```bash
   golangci-lint run
   ```

### Coding Conventions

- **Package names:** Short, lowercase, no underscores
- **File names:** lowercase_with_underscores.go
- **Interface names:** Where possible, end with `-er` (e.g., `Reader`, `Writer`)
- **Exported names:** PascalCase
- **Unexported names:** camelCase
- **Constants:** PascalCase or ALL_CAPS for exported

### Example

```go
// Good package name
package team

// Good interface name
type Manager interface {
    AssignRole(project, team, role, person string) error
    GetStatus(project string) (Status, error)
}

// Good struct and method names
type teamManager struct {
    db     database.Store
    cache  cache.Cache
}

func (tm *teamManager) AssignRole(project, team, role, person string) error {
    // implementation
}
```

### Error Handling

- Always check errors
- Wrap errors with context using `fmt.Errorf` with `%w`
- Use custom error types for business logic errors

```go
// Good
if err := tm.db.AssignRole(ctx, project, team, role, person); err != nil {
    return fmt.Errorf("failed to assign role: %w", err)
}

// Custom error type
var ErrTeamFull = errors.New("team is at maximum capacity")

if len(members) >= maxTeamSize {
    return ErrTeamFull
}
```

---

## Testing

### Test Structure

```bash
mcp-server/internal/team/
├── team.go           # Implementation
└── team_test.go      # Tests
```

### Running Tests

```bash
# All tests
make test

# Specific package
go test ./internal/team/...

# With verbose output
go test -v ./internal/team/...

# Race detection
go test -race ./...

# Benchmarks
go test -bench=. ./...
```

### Test Coverage

Minimum coverage requirements:
- Core packages (`internal/team`, `internal/rules`): 80%+
- Handlers (`internal/web`): 70%+
- Utilities: 60%+

```bash
# Generate coverage report
make test-cover

# View in browser
go tool cover -html=coverage.out
```

### Test Guidelines

1. Use table-driven tests where possible
2. Mock external dependencies (database, cache)
3. Test both success and error paths
4. Use meaningful test names

```go
func TestAssignRole(t *testing.T) {
    tests := []struct {
        name    string
        project string
        team    string
        role    string
        person  string
        wantErr error
    }{
        {
            name:    "valid assignment",
            project: "test-project",
            team:    "team-1",
            role:    "lead",
            person:  "alice",
            wantErr: nil,
        },
        {
            name:    "team full",
            project: "test-project",
            team:    "team-1",
            role:    "developer",
            person:  "bob",
            wantErr: ErrTeamFull,
        },
    }

    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            tm := NewManager(mockDB, mockCache)
            err := tm.AssignRole(tt.project, tt.team, tt.role, tt.person)
            if !errors.Is(err, tt.wantErr) {
                t.Errorf("AssignRole() error = %v, wantErr %v", err, tt.wantErr)
            }
        })
    }
}
```

---

## Commit Guidelines

### Conventional Commits

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, no logic change)
- `refactor`: Code refactoring
- `test`: Test additions or updates
- `chore`: Build process or auxiliary tool changes
- `perf`: Performance improvements
- `security`: Security fixes

### Examples

```
feat(team): add team size validation

Implements TEAM-007 compliance check for maximum team size.
Adds validation to prevent over-allocation.

fix(mcp): resolve session timeout handling

docs(api): update endpoint documentation

refactor(database): simplify transaction handling

test(audit): add coverage for audit logging
```

### Scope Values

Common scopes for this project:
- `team`: Team management
- `mcp`: MCP protocol
- `web`: Web handlers
- `database`: Database operations
- `cache`: Redis/cache
- `security`: Security features
- `config`: Configuration
- `deploy`: Deployment

---

## Migration Notice

### Python to Go Migration (v2.6.0)

**Status:** Complete

**What Changed:**
- `scripts/team_manager.py` -> `mcp-server/internal/team/` (Go package)
- All team management logic now in Go
- Database migrations via golang-migrate
- No Python runtime required

**How to Contribute:**
1. Write Go code in `mcp-server/internal/`
2. Follow Go conventions and this guide
3. Run `make check` before committing
4. Ensure tests pass with `make test`

**Benefits:**
- Smaller container size (~50MB vs ~500MB)
- Faster startup (~100ms vs ~3s)
- Distroless security hardening
- Single static binary

**See Also:**
- [PYTHON_MIGRATION.md](PYTHON_MIGRATION.md) - Detailed migration guide
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture
- [MIGRATION.md](MIGRATION.md) - Version migration procedures

---

## Getting Help

- **Documentation:** Start with [INDEX_MAP.md](../INDEX_MAP.md)
- **Issues:** [GitHub Issues](https://github.com/TheArchitectit/agent-guardrails-template/issues)
- **Discussions:** [GitHub Discussions](https://github.com/TheArchitectit/agent-guardrails-template/discussions)

---

**Last Updated:** 2026-02-15
**Version:** 2.6.0
**Implementation:** Go (mcp-server/internal/)
