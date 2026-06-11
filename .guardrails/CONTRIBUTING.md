# Contributing to Agent Guardrails Template

> Guidelines for contributing to the MCP Server and related components

**Version:** 2.6.0
**Last Updated:** 2026-02-15

---

## Quick Start

1. **Fork** the repository
2. **Clone** your fork: `git clone https://github.com/YOUR_USERNAME/agent-guardrails-template.git`
3. **Set up** the development environment (see below)
4. **Create** a feature branch: `git checkout -b feature/your-feature`
5. **Make** your changes
6. **Test** your changes
7. **Commit** with conventional commit messages
8. **Push** to your fork
9. **Open** a Pull Request

---

## Development Environment

### Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Go | 1.23+ | Primary language for MCP server |
| Docker | 20.10+ | Container development |
| PostgreSQL | 16+ | Database (or use Docker) |
| Redis | 7+ | Cache (or use Docker) |
| Make | 3.81+ | Build automation |

### Repository Structure

```
agent-guardrails-template/
├── mcp-server/              # Go MCP Server (Primary)
│   ├── cmd/server/         # Entry point
│   ├── internal/           # Internal packages
│   │   ├── team/          # Team management (Go)
│   │   ├── mcp/           # MCP protocol handlers
│   │   ├── database/      # Database operations
│   │   ├── cache/         # Redis caching
│   │   └── ...
│   ├── deploy/            # Docker configs
│   └── go.mod             # Go dependencies
├── scripts/               # Python scripts (DEPRECATED)
├── docs/                  # Documentation
└── ...
```

---

## Go Development Guidelines

### Code Style

We follow standard Go conventions:

```bash
# Format code
cd mcp-server
go fmt ./...

# Run linter
golangci-lint run

# Run vet
go vet ./...
```

### Project Structure

```go
// Package documentation
// Package team provides team management functionality for MCP server.
// This is a Go port of the Python team_manager.py core functionality.
package team

// Imports - grouped by: stdlib, third-party, internal
import (
    "context"
    "fmt"

    "github.com/some/lib"

    "github.com/thearchitectit/guardrail-mcp/internal/models"
)

// Exported types start with capital letter
type Manager struct {
    // fields...
}

// Constructor function
func NewManager(projectName string, opts ...ManagerOption) (*Manager, error) {
    // implementation...
}

// Methods
func (m *Manager) AssignRole(ctx context.Context, teamID int, role, person string) error {
    // implementation...
}
```

### Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| **Packages** | lowercase, single word | `team`, `mcp`, `database` |
| **Exported** | PascalCase | `Manager`, `AssignRole` |
| **Unexported** | camelCase | `internalFunc`, `helper` |
| **Constants** | PascalCase or UPPER_SNAKE | `MaxRetries`, `DEFAULT_TIMEOUT` |
| **Interfaces** | -er suffix | `Reader`, `Writer`, `Manager` |
| **Structs** | PascalCase | `TeamManager`, `ProjectConfig` |

### Error Handling

Use wrapped errors with context:

```go
// Good
if err := validateProjectName(projectName); err != nil {
    return fmt.Errorf("invalid project name %q: %w", projectName, err)
}

// Avoid
return errors.New("something went wrong")  // No context

// Custom error types for specific cases
type ValidationError struct {
    Field   string
    Message string
}

func (e *ValidationError) Error() string {
    return fmt.Sprintf("validation error on %s: %s", e.Field, e.Message)
}
```

### Context Usage

Always accept `context.Context` as first parameter:

```go
// Good
func (m *Manager) DoOperation(ctx context.Context, arg string) error {
    // Use ctx for timeouts, cancellation
}

// Avoid - no context support
func (m *Manager) DoOperation(arg string) error {
    // ...
}
```

### Testing

```go
// Test files: *_test.go
package team

import (
    "context"
    "testing"

    "github.com/stretchr/testify/assert"
    "github.com/stretchr/testify/require"
)

func TestManager_AssignRole(t *testing.T) {
    // Arrange
    ctx := context.Background()
    mgr, err := NewManager("test-project")
    require.NoError(t, err)

    // Act
    err = mgr.AssignRole(ctx, 7, "Technical Lead", "Alice")

    // Assert
    assert.NoError(t, err)
}

// Table-driven tests
func TestValidateProjectName(t *testing.T) {
    tests := []struct {
        name    string
        input   string
        wantErr bool
    }{
        {"valid", "my-project", false},
        {"empty", "", true},
        {"with space", "my project", true},
    }

    for _, tt := range tests {
        t.Run(tt.name, func(t *testing.T) {
            err := ValidateProjectName(tt.input)
            if tt.wantErr {
                assert.Error(t, err)
            } else {
                assert.NoError(t, err)
            }
        })
    }
}
```

Run tests:

```bash
cd mcp-server

# Run all tests
go test ./...

# Run with coverage
go test -cover ./...

# Run specific package
go test ./internal/team/...

# Run benchmarks
go test -bench=. ./internal/team/...
```

---

## Commit Guidelines

### Conventional Commits

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation only
- `style` - Code style (formatting, semicolons)
- `refactor` - Code refactoring
- `test` - Adding tests
- `chore` - Build process, dependencies

**Scopes:**
- `team` - Team management
- `mcp` - MCP protocol
- `db` - Database
- `cache` - Redis/cache
- `security` - Security features
- `api` - API endpoints
- `web` - Web UI
- `docs` - Documentation

**Examples:**

```
feat(team): add batch assignment operation

Adds support for assigning multiple team members in a single
team operation with transaction support.

Closes #123
```

```
fix(mcp): handle nil context in tool handlers

Prevents panic when context is nil in team tool handlers.

Fixes #456
```

```
docs: add Python to Go migration guide

Adds comprehensive migration documentation for developers
migrating from Python team_manager.py to Go team package.
```

---

## Pull Request Process

1. **Update documentation** if needed
2. **Add tests** for new functionality
3. **Ensure all tests pass**: `go test ./...`
4. **Update CHANGELOG.md** with your changes
5. **Request review** from maintainers
6. **Address feedback** promptly
7. **Squash commits** if requested

### PR Template

```markdown
## Summary
Brief description of changes

## Type
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests pass
- [ ] Manual testing completed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex code
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
```

---

## Python Scripts (Deprecated)

**Note:** Python scripts in `scripts/` are deprecated as of v2.6.0.

- Do **not** add new features to Python scripts
- Do **not** modify Python scripts unless fixing critical bugs
- All new development should be in Go
- Python scripts will be removed in v3.0.0

See [docs/PYTHON_TO_GO_MIGRATION.md](docs/PYTHON_TO_GO_MIGRATION.md) for migration guide.

---

## Documentation

### Adding Documentation

1. Create markdown file in appropriate directory:
   - `docs/` - General documentation
   - `docs/workflows/` - Operational procedures
   - `docs/standards/` - Coding standards
   - `mcp-server/` - MCP-specific docs

2. Keep documents under 500 lines (see `docs/standards/MODULAR_DOCUMENTATION.md`)

3. Update navigation:
   - `INDEX_MAP.md` - Add keywords
   - `HEADER_MAP.md` - Add sections
   - `TOC.md` - Add file listing

4. Follow existing format and style

---

## Questions?

- **General:** Open a GitHub Discussion
- **Bug Reports:** Open a GitHub Issue
- **Security:** Email security@example.com

---

**Thank you for contributing!**
