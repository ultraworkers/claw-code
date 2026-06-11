# Example Implementations

> Reference implementations demonstrating guardrails-compliant testing patterns.

**Purpose:** These examples show how to implement the testing and validation protocols described in [AGENT_GUARDRAILS.md](../docs/AGENT_GUARDRAILS.md) across different programming languages.

---

## Available Examples

| Language | Directory | Test Framework | Key Patterns |
|----------|-----------|----------------|--------------|
| **TypeScript** | `typescript/` | Jest | Logging hooks, retry logic |
| **Python** | `python/` | pytest | Config loading, environment separation |
| **Go** | `go/` | go test | Config validation, environment handling |
| **Rust** | `rust/` | cargo test | Config loading with Result types |
| **Ruby** | `ruby/` | RSpec | Config loader with environment support |
| **Java** | `java/` | JUnit 5 | ConfigLoader with environment injection |
| **C#** | `csharp/` | xUnit | ConfigLoader with IConfiguration |
| **Swift** | `swift/` | XCTest | ConfigLoader with Codable |
| **Elixir** | `elixir/` | ExUnit | Config loading with Mix environments |
| **Scala** | `scala/functional-ui/` | scala-cli | Functional UI composition, type-safe CSS, DDA telemetry |
| **R** | `r/game-analytics/` | Rscript | ggplot2 4.0+, Shiny 2.0+, ethics auditing, retention analysis |

---

## Key Concepts Demonstrated

### 1. Test/Production Separation

Each example demonstrates:
- Separate configuration files for test vs production
- Environment-specific database connections
- Isolated test infrastructure

### 2. Production Code First

Following guardrails, each example:
1. Implements production code in `src/` or `lib/`
2. Adds test code in `tests/` or `test/`
3. Uses language-idiomatic test patterns

### 3. Environment-Specific Configuration

```
config/
├── production.yaml   # Production settings
├── test.yaml         # Test settings
└── development.yaml  # Development settings
```

---

## Running Examples

### TypeScript
```bash
cd typescript
npm install
npm test
```

### Python
```bash
cd python
pip install -e ".[dev]"
pytest
```

### Go
```bash
cd go
go test ./...
```

### Rust
```bash
cd rust
cargo test
```

### Ruby
```bash
cd ruby
bundle install
bundle exec rspec
```

### Java
```bash
cd java
./gradlew test
```

### C#
```bash
cd csharp
dotnet test
```

### Swift
```bash
cd swift
swift test
```

### Elixir
```bash
cd elixir
mix deps.get
mix test
```

### Scala
```bash
cd scala/functional-ui
scala-cli run UI.scala
scala-cli run data-pipeline-viz.scala
scala-cli run procedural-gen.scala
```

### R
```bash
cd r/game-analytics
Rscript shiny-dashboard.R
Rscript retention-analysis.R
Rscript ethics-auditor.R
```

### Shiny Dashboard (R Web Server)
```bash
Rscript -e "shiny::runApp('r/game-analytics')"
```

---

## Guardrails Compliance

All examples follow:

| Rule | Implementation |
|------|----------------|
| **PRODUCTION FIRST** | Production code exists before tests |
| **SEPARATE DATABASES** | Config files use different DB hosts per environment |
| **SEPARATE SERVICES** | Service endpoints differ by environment |
| **NO TEST USERS IN PROD** | Test users have `test_` prefix |
| **CLEAR LABELING** | Test files follow `*_test.*` or `*.test.*` convention |

---

## Related Documentation

- [AGENT_GUARDRAILS.md](../docs/AGENT_GUARDRAILS.md) - Core safety protocols
- [TEST_PRODUCTION_SEPARATION.md](../docs/standards/TEST_PRODUCTION_SEPARATION.md) - Separation standards
- [TESTING_VALIDATION.md](../docs/workflows/TESTING_VALIDATION.md) - Validation protocols
- [SPRINT_TEMPLATE.md](../docs/sprints/SPRINT_TEMPLATE.md) - Language-specific commands

---

**Last Updated:** 2026-01-18
