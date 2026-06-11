# Extracting Prevention Rules from Markdown

**Version:** 1.0
**Last Updated:** 2026-02-09
**Applies To:** MCP Server, Web UI, Rule Authors

---

## Overview

This document explains how prevention rules are extracted from markdown files and made available as:

1. **MCP Tools** - For AI agents to validate actions
2. **Web UI Rules** - For browsing and managing rules
3. **Database Storage** - For persistence and querying

---

## Markdown Rule Format

Prevention rules are defined in markdown files using a standardized format. Each rule is a section with specific metadata fields.

### Basic Rule Structure

```markdown
## PREVENT-XXX: Rule Title

**Pattern:** `regex-pattern-here`
**Severity:** error|warning|info
**Category:** git|bash|docker|security|general|code|test

Description of what this rule prevents and why it matters.

### Examples

**Violations:**
```bash
# Bad example that triggers this rule
rm -rf /
```

**Compliant:**
```bash
# Good example that passes
rm -i file.txt
```
```

### Required Fields

| Field | Description | Values |
|-------|-------------|--------|
| `## PREVENT-XXX: Title` | Rule identifier and name | PREVENT-001 through PREVENT-999 |
| **Pattern** | Regex pattern to match violations | Valid Go regex |
| **Severity** | Impact level | `error`, `warning`, `info` |
| **Category** | Rule classification | `git`, `bash`, `docker`, `security`, `general`, `code`, `test` |

### Optional Fields

| Field | Description | Example |
|-------|-------------|---------|
| **Language** | Target programming language | `go`, `python`, `javascript` |
| **Fix** | Suggested remediation | `Use rm -i instead` |
| **References** | Related documentation | `See AGENT_GUARDRAILS.md` |

---

## Complete Rule Example

```markdown
## PREVENT-001: Force Push Prohibition

**Pattern:** `git\s+push\s+.*--force`
**Severity:** error
**Category:** git
**Language:** bash
**Fix:** Use git push with standard options; never force push to shared branches

Prevents force pushing to git repositories, which can overwrite commit history and cause data loss for collaborators.

Force push destroys the commit history that other developers may have based their work on, making it impossible for them to merge their changes.

### Examples

**Violations:**
```bash
git push --force origin main
git push -f origin feature-branch
```

**Compliant:**
```bash
git push origin main
git push origin feature-branch
```

### Rationale

This rule enforces the Git Safety Rules from AGENT_GUARDRAILS.md which state that force push is never allowed as it causes irreversible data loss.
```

---

## Rule Storage Locations

Rules are stored in multiple locations depending on their source:

### File Locations

| Location | Purpose | Format |
|----------|---------|--------|
| `.guardrails/prevention-rules/pattern-rules.json` | Regex-based rules | JSON array |
| `.guardrails/prevention-rules/semantic-rules.json` | AST-based rules | JSON array |
| `.guardrails/prevention-rules/extracted-rules.json` | Rules extracted from markdown | JSON array |
| `docs/*.md` | Source documentation | Markdown |

### Pattern Rules Example (JSON)

```json
{
  "id": "PREVENT-001",
  "name": "Force Push Prohibition",
  "pattern": "git\\s+push\\s+.*--force",
  "severity": "error",
  "category": "git",
  "description": "Prevents force pushing to git repositories",
  "examples": {
    "violation": "git push --force origin main",
    "compliant": "git push origin main"
  }
}
```

### Semantic Rules Example (JSON)

```json
{
  "id": "PREVENT-101",
  "name": "Hardcoded Credentials",
  "language": "go",
  "pattern": "password|token|secret|key",
  "severity": "error",
  "category": "security",
  "ast_context": "assignment",
  "description": "Detects potential hardcoded credentials"
}
```

---

## Extraction Flow: MD to MCP Tool

The following diagram shows the complete flow from markdown to MCP tool:

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Markdown File  │────▶│  RuleParser      │────▶│  ParsedRule     │
│  (docs/*.md)    │     │  (ParseRules)    │     │  (Struct)       │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                           │
                                                           ▼
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  MCP Tool       │◀────│  ToolGenerator   │◀────│  PreventionRule │
│  (ValidateXxx)  │     │  (GenerateTools) │     │  (DB Model)     │
└─────────────────┘     └──────────────────┘     └─────────────────┘
         │
         ▼
┌─────────────────┐
│  AI Agent       │
│  (Validation)   │
└─────────────────┘
```

---

## Step-by-Step Extraction

### Step 1: Parse Markdown Rules

The `RuleParser` scans markdown content and extracts rule sections:

```go
// From: mcp-server/internal/ingest/rule_parser.go

parser := ingest.NewRuleParser()
content, _ := os.ReadFile("docs/AGENT_GUARDRAILS.md")

rules, err := parser.ParseRules(string(content), "AGENT_GUARDRAILS.md")
if err != nil {
    log.Fatal(err)
}

for _, rule := range rules {
    fmt.Printf("Found rule: %s - %s\n", rule.ID, rule.Name)
}
```

### Step 2: Convert to ParsedRule

Rules are parsed into the `ParsedRule` struct:

```go
// From: mcp-server/internal/models/rule.go

type ParsedRule struct {
    ID          string   `json:"id"`
    Name        string   `json:"name"`
    Pattern     string   `json:"pattern"`
    Severity    string   `json:"severity"`
    Category    string   `json:"category"`
    Description string   `json:"description"`
    Examples    []string `json:"examples"`
    Language    string   `json:"language,omitempty"`
    Fix         string   `json:"fix,omitempty"`
    FilePath    string   `json:"file_path"`
}
```

### Step 3: Store in Database

Parsed rules are converted to `PreventionRule` models and stored:

```go
// From: mcp-server/internal/database/rules.go

rule := &models.PreventionRule{
    ID:          generateRuleID(parsed.ID),
    Code:        parsed.ID,
    Name:        parsed.Name,
    Description: parsed.Description,
    Pattern:     parsed.Pattern,
    Severity:    models.Severity(parsed.Severity),
    Category:    models.Category(parsed.Category),
    Language:    parsed.Language,
    Fix:         parsed.Fix,
    Source:      "markdown",
    Version:     1,
    Enabled:     true,
}

// Store in database
if err := db.CreateRule(ctx, rule); err != nil {
    return err
}
```

### Step 4: Generate MCP Tools

Rules become MCP tools through the tool generator:

```go
// From: mcp-server/internal/mcp/server.go

func (s *Server) generateRuleTools(rules []models.PreventionRule) []Tool {
    tools := make([]Tool, 0, len(rules))

    for _, rule := range rules {
        tool := Tool{
            Name:        fmt.Sprintf("validate_%s", rule.Code),
            Description: fmt.Sprintf("%s: %s", rule.Name, rule.Description),
            InputSchema: ToolInputSchema{
                Type: "object",
                Properties: map[string]SchemaProperty{
                    "command": {
                        Type:        "string",
                        Description: "Command or code to validate",
                    },
                    "context": {
                        Type:        "string",
                        Description: "Optional execution context",
                    },
                },
                Required: []string{"command"},
            },
        }
        tools = append(tools, tool)
    }

    return tools
}
```

---

## Code Examples

### Example 1: Parsing Rules from Markdown

```go
package main

import (
    "context"
    "fmt"
    "log"

    "github.com/thearchitectit/guardrail-mcp/internal/ingest"
)

func main() {
    // Create parser
    parser := ingest.NewRuleParser()

    // Parse markdown file
    content := `
## PREVENT-001: No Force Push
**Pattern:** git\\s+push\\s+.*--force
**Severity:** error
**Category:** git

Prevents force pushing to git repositories.
`

    rules, err := parser.ParseRules(content, "test.md")
    if err != nil {
        log.Fatal(err)
    }

    for _, rule := range rules {
        fmt.Printf("Rule: %s\n", rule.ID)
        fmt.Printf("  Name: %s\n", rule.Name)
        fmt.Printf("  Pattern: %s\n", rule.Pattern)
        fmt.Printf("  Severity: %s\n", rule.Severity)
    }
}
```

### Example 2: Loading Rules from JSON

```go
package main

import (
    "encoding/json"
    "fmt"
    "os"

    "github.com/thearchitectit/guardrail-mcp/internal/models"
)

func main() {
    // Load pattern rules
    data, err := os.ReadFile(".guardrails/prevention-rules/pattern-rules.json")
    if err != nil {
        log.Fatal(err)
    }

    var rules []models.PreventionRule
    if err := json.Unmarshal(data, &rules); err != nil {
        log.Fatal(err)
    }

    // Filter by category
    for _, rule := range rules {
        if rule.Category == "security" {
            fmt.Printf("Security Rule: %s\n", rule.Name)
        }
    }
}
```

### Example 3: Validating Commands Against Rules

```go
package main

import (
    "context"
    "fmt"
    "regexp"

    "github.com/thearchitectit/guardrail-mcp/internal/database"
)

func validateCommand(ctx context.Context, db *database.RuleStore, command string) error {
    // Fetch all enabled rules
    rules, err := db.ListRules(ctx, database.ListRulesFilter{Enabled: true})
    if err != nil {
        return err
    }

    // Check each rule
    for _, rule := range rules {
        if rule.Pattern == "" {
            continue
        }

        matched, err := regexp.MatchString(rule.Pattern, command)
        if err != nil {
            continue
        }

        if matched {
            return fmt.Errorf("%s: %s (severity: %s)",
                rule.Code, rule.Description, rule.Severity)
        }
    }

    return nil
}
```

---

## Manual Rule Sync

### Triggering Sync via API

```bash
# Sync rules from repository
curl -X POST http://localhost:8081/api/ingest/sync \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"source": "repo", "paths": [".guardrails/prevention-rules"]}'
```

### Sync Response

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "files_processed": 3,
  "rules_added": 2,
  "rules_updated": 1,
  "rules_orphaned": 0,
  "completed_at": "2026-02-09T10:00:00Z"
}
```

### Programmatic Sync

```go
// From: mcp-server/internal/ingest/service.go

func syncRules(ctx context.Context, service *ingest.Service) error {
    jobID := uuid.New()

    // Sync from repository directories
    if err := service.SyncFromRepo(ctx, jobID); err != nil {
        return fmt.Errorf("sync failed: %w", err)
    }

    fmt.Printf("Sync completed: job=%s\n", jobID)
    return nil
}
```

---

## Auto-Sync on File Changes

### File Watcher Setup

The MCP server can watch for file changes and automatically sync rules:

```go
// Watch for file changes in prevention rules directory
watcher, err := fsnotify.NewWatcher()
if err != nil {
    log.Fatal(err)
}
defer watcher.Close()

// Add directories to watch
watcher.Add(".guardrails/prevention-rules")
watcher.Add("docs")

// Process events
for {
    select {
    case event, ok := <-watcher.Events:
        if !ok {
            return
        }
        if event.Op&fsnotify.Write == fsnotify.Write {
            // Trigger sync
            go triggerSync(event.Name)
        }
    case err, ok := <-watcher.Errors:
        if !ok {
            return
        }
        log.Printf("Watcher error: %v", err)
    }
}
```

### Sync Triggers

| Event | Action |
|-------|--------|
| Markdown file modified | Re-parse and update rules |
| JSON rule file modified | Reload and validate |
| New file added | Parse and add new rules |
| File deleted | Mark rules as orphaned |

---

## Web UI Integration

### Browsing Rules

The Web UI displays rules from the database:

```javascript
// Fetch rules from API
async function fetchRules(category = null) {
    const params = new URLSearchParams();
    if (category) params.append('category', category);

    const response = await fetch(`/api/rules?${params}`, {
        headers: {
            'Authorization': `Bearer ${apiKey}`
        }
    });

    return await response.json();
}
```

### Rule Display

Rules are displayed with:
- ID and name
- Severity badge (error/warning/info)
- Category tag
- Pattern (if applicable)
- Description
- Examples

### Enabling/Disabling Rules

```javascript
// Toggle rule enabled state
async function toggleRule(ruleId, enabled) {
    const response = await fetch(`/api/rules/${ruleId}`, {
        method: 'PUT',
        headers: {
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${apiKey}`
        },
        body: JSON.stringify({ enabled })
    });

    return await response.json();
}
```

---

## Database Schema

### Prevention Rules Table

```sql
CREATE TABLE prevention_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(50) UNIQUE NOT NULL,  -- PREVENT-XXX
    name VARCHAR(255) NOT NULL,
    description TEXT,
    pattern TEXT,                      -- Regex pattern
    severity VARCHAR(20) NOT NULL,     -- error, warning, info
    category VARCHAR(50) NOT NULL,     -- git, bash, security, etc.
    language VARCHAR(50),              -- go, python, etc.
    fix TEXT,                          -- Suggested fix
    source VARCHAR(50) NOT NULL,       -- markdown, json, manual
    version INTEGER DEFAULT 1,
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_rules_category ON prevention_rules(category);
CREATE INDEX idx_rules_severity ON prevention_rules(severity);
CREATE INDEX idx_rules_enabled ON prevention_rules(enabled);
CREATE INDEX idx_rules_code ON prevention_rules(code);
```

---

## Best Practices

### Writing Effective Rules

1. **Specific Patterns** - Make regex patterns specific to avoid false positives
2. **Clear Descriptions** - Explain why the rule exists and what it prevents
3. **Good Examples** - Include both violation and compliant examples
4. **Appropriate Severity** - Use error for dangerous operations, warning for risky ones
5. **Test Patterns** - Validate regex patterns before committing

### Rule Categories

| Category | Use For | Example |
|----------|---------|---------|
| `git` | Git operations | Force push, unsafe reset |
| `bash` | Shell commands | rm -rf, unsafe redirects |
| `docker` | Container operations | Exposing ports, privileged mode |
| `security` | Security issues | Hardcoded secrets, SQL injection |
| `code` | Code patterns | Unchecked errors, race conditions |
| `test` | Test code | Test DB in prod, mock issues |
| `general` | General guidelines | File permissions, naming |

### Testing Rules

```bash
# Test pattern matching
echo "git push --force" | grep -P 'git\s+push\s+.*--force'

# Validate JSON rules
cat .guardrails/prevention-rules/pattern-rules.json | jq empty

# Run rule parser tests
go test ./mcp-server/internal/ingest/...
```

---

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Rules not appearing | Sync not run | Trigger manual sync or restart server |
| Pattern not matching | Invalid regex | Test pattern with regex validator |
| Duplicate rules | Same rule in multiple files | Check file paths and rule IDs |
| Rules marked orphaned | Source file deleted | Restore file or disable orphan cleanup |

### Debug Commands

```bash
# Check database rules
curl http://localhost:8081/api/rules | jq '.data | length'

# Verify specific rule
curl http://localhost:8081/api/rules/PREVENT-001 | jq

# Check sync status
curl http://localhost:8081/api/ingest/jobs | jq

# View MCP tools
curl http://localhost:8080/mcp/tools | jq '.tools[] | .name'
```

---

## Related Documents

- [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md) - Core safety protocols
- [MCP Server API](../mcp-server/API.md) - Complete API reference
- [HOW_TO_APPLY.md](HOW_TO_APPLY.md) - Applying guardrails to repositories

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Review Cycle:** Monthly
**Last Review:** 2026-02-09
