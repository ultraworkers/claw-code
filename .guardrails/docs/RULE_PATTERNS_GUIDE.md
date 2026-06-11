# Rule Patterns Guide

Guide for writing effective prevention rule patterns.

---

## Pattern Basics

### Regex Engine
- **Language:** Go regexp (RE2 syntax)
- **Flags:** `(?i)` for case-insensitive matching
- **Performance:** Patterns pre-compiled and cached

### Pattern Structure

```
(?i)(pattern_component_1|pattern_component_2|...)
```

### Anchoring

| Anchor | Meaning | Example |
|--------|---------|---------|
| `^` | Start of string | `^git\s+` - starts with "git " |
| `$` | End of string | `main$` - ends with "main" |
| `\b` | Word boundary | `\brm\b` - matches "rm" not "remove" |

---

## Pattern Categories

### 1. Command Patterns

Match shell commands and their arguments.

**Example: Block rm -rf /**
```regex
(?i)(rm\s+-[a-z]*rf|rm\s+-[a-z]*f[a-z]*r)
```

**Breakdown:**
- `(?i)` - Case insensitive
- `rm\s+-` - "rm" followed by whitespace and dash
- `[a-z]*rf` - Any flags ending with "rf"
- `[a-z]*f[a-z]*r` - Flags with "f" before "r"

### 2. Content Patterns

Match within file content for secrets and vulnerabilities.

**Example: API Key Detection**
```regex
(?i)(api[_-]?key\s*[:=]\s*["'][^"']{10,})
```

**Breakdown:**
- `api[_-]?key` - "apikey", "api_key", or "api-key"
- `\s*[:=]\s*` - Colon or equals with optional spaces
- `["']` - Quote character
- `[^"']{10,}` - 10+ non-quote characters (the value)

### 3. Path Patterns

Match file paths for protected resources.

**Example: Protected Directories**
```regex
(?i)(?:\.claude/|\.claude$|.*\.md$|docs/)
```

**Breakdown:**
- `(?:...)` - Non-capturing group
- `\.claude/` - Directory pattern
- `\.claude$` - Exact match
- `.*\.md$` - File extension
- `docs/` - Directory prefix

---

## Common Pattern Examples

### Dangerous Commands

| Pattern | Blocks |
|---------|--------|
| `(?i)rm\s+-[a-z]*rf\s*/` | `rm -rf /` |
| `(?i):\(\)\s*\{\s*:\|\:&\s*\};` | Fork bomb |
| `(?i)mkfs\.\w+\s+/dev/` | Filesystem format |
| `(?i)dd\s+.*of=/dev/[sh]` | Direct disk writes |
| `(?i)>\s*/etc/` | Overwriting system files |

### Git Operations

| Pattern | Blocks |
|---------|--------|
| `git\s+push\s+.*--force` | Force pushes |
| `git\s+push\s+.*--delete` | Branch deletion |
| `git\s+.*--hard\s+` | Hard resets |
| `git\s+(commit\s+--amend\|rebase)` | History rewrites |

### Secrets & Credentials

| Pattern | Detects |
|---------|---------|
| `(?i)api[_-]?key\s*[:=]\s*["'][^"']{10,}` | API keys |
| `(?i)secret[_-]?key\s*[:=]\s*["'][^"']{10,}` | Secret keys |
| `(?i)password\s*[:=]\s*["'][^"']{6,}` | Passwords |
| `(?i)token\s*[:=]\s*["'][^"']{20,}` | Tokens |
| `bearer\s+[a-zA-Z0-9]{20,}` | Bearer tokens |
| `-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----` | Private keys |

### Database URIs

| Pattern | Detects |
|---------|---------|
| `mongodb(\+srv)?://[^:]+:[^@]+@` | MongoDB with password |
| `postgres(ql)?://[^:]+:[^@]+@` | PostgreSQL with password |
| `mysql://[^:]+:[^@]+@` | MySQL with password |
| `redis://:[^@]+@` | Redis with password |

### Container Security

| Pattern | Detects |
|---------|---------|
| `USER\s+root` | Root user in Dockerfile |
| `chmod\s+777` | World-writable permissions |
| `--privileged` | Privileged container |
| `FROM.*:latest` | Latest tag usage |

### Vulnerabilities

| Pattern | Detects |
|---------|---------|
| `eval\s*\(.*\$` | Eval with variables |
| `exec\s*\(.*\$` | Exec with variables |
| `innerHTML\s*=` | XSS via innerHTML |
| `document\.write\s*\(` | XSS via document.write |
| `SELECT.*FROM.*\$` | SQL injection |

---

## Pattern Testing

### Manual Testing

Use Go's regex tester or online RE2-compatible testers.

```go
package main

import (
    "fmt"
    "regexp"
)

func main() {
    pattern := `(?i)(rm\s+-[a-z]*rf)`
    re := regexp.MustCompile(pattern)

    tests := []string{
        "rm -rf /",
        "rm -rf /home",
        "rm -Rf /tmp",
        "rm -f -r /var",
    }

    for _, test := range tests {
        matches := re.MatchString(test)
        fmt.Printf("%q matches: %v\n", test, matches)
    }
}
```

### Validation via MCP

Test patterns using the validation endpoint:

```bash
curl -X POST http://localhost:8081/mcp/validate \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "guardrail_validate_bash",
    "arguments": {
      "command": "rm -rf /tmp/test"
    }
  }'
```

---

## Adding Rules to Database

### SQL Template

```sql
INSERT INTO prevention_rules (
    rule_id,
    name,
    pattern,
    message,
    severity,
    category,
    enabled
) VALUES (
    'CUST-001',
    'Custom Rule Name',
    '(?i)pattern_here',
    'Human-readable violation message',
    'error',  -- critical, error, warning, info
    'security',  -- bash, git, security, general
    true
);
```

### Severity Guidelines

| Severity | When to Use | Example |
|----------|-------------|---------|
| **critical** | Immediate security risk, data loss | Private keys, destructive commands |
| **error** | Policy violation, potential harm | Force push, secrets in code |
| **warning** | Caution needed, review recommended | Large deletions, debug mode |
| **info** | Informational, no blocking | Statistics, suggestions |

### Category Naming

| Category | Use For |
|----------|---------|
| `bash` | Shell commands, system operations |
| `git` | Git operations, version control |
| `security` | Secrets, credentials, vulnerabilities |
| `general` | Cross-cutting concerns, file protections |
| `file_edit` | File path validation |
| `content` | Code content analysis |
| `edit` | Edit operation metadata |

---

## Pattern Reference Table

Real patterns from the 36 active rules:

| Rule | Pattern Type | Pattern (Simplified) |
|------|--------------|---------------------|
| BASH-001 | Command | `rm\s+-[a-z]*rf\s*/` |
| GIT-001 | Git | `push.*--force.*main` |
| API-001 | Secret | `api[_-]?key\s*[:=]\s*["'][^"']{10,}` |
| DB-001 | URI | `mongodb://[^:]+:[^@]+@` |
| CONT-001 | Dockerfile | `USER\s+root` |
| CODE-003 | Vulnerability | `innerHTML\s*=` |

---

## Best Practices

1. **Test thoroughly** - Patterns can have false positives
2. **Use word boundaries** - `\brm\b` vs just `rm`
3. **Account for whitespace** - `\s*` for optional spaces
4. **Consider case variations** - Use `(?i)` flag
5. **Limit scope** - More specific patterns are safer
6. **Document intent** - Comment what the pattern targets

---

## Related Documentation

| Document | Purpose |
|----------|---------|
| [RULES_INDEX_MAP.md](RULES_INDEX_MAP.md) | All active rules reference |
| [MCP_TOOLS_REFERENCE.md](MCP_TOOLS_REFERENCE.md) | Using validation tools |
| [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md) | Main guardrails guide |
