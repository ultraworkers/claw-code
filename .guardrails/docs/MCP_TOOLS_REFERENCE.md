# MCP Tools Reference

Complete reference for Guardrails MCP validation tools.

---

## Quick Reference Table

| Tool | Purpose | Input | Categories Validated |
|------|---------|-------|---------------------|
| `guardrail_validate_bash` | Validate bash commands | Command string | bash |
| `guardrail_validate_git_operation` | Validate git operations | Operation + args | git |
| `guardrail_validate_file_edit` | Validate file edits | Path + content | file_edit, content, edit, security |

---

## guardrail_validate_bash

Validates bash commands against dangerous patterns.

### Description
Analyzes bash commands for potentially destructive or dangerous operations like `rm -rf /`, fork bombs, and data destruction patterns.

### Input Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `command` | string | Yes | The bash command to validate |

### Return Value

```json
{
  "violations": [
    {
      "rule_id": "BASH-001",
      "severity": "critical",
      "message": "Dangerous bash command detected",
      "category": "bash"
    }
  ]
}
```

### Example Usage

**Request:**
```json
{
  "tool": "guardrail_validate_bash",
  "arguments": {
    "command": "rm -rf /"
  }
}
```

**Response:**
```json
{
  "violations": [
    {
      "rule_id": "BASH-001",
      "severity": "critical",
      "message": "Dangerous bash command detected",
      "category": "bash"
    }
  ]
}
```

**Valid Command (no violations):**
```json
{
  "tool": "guardrail_validate_bash",
  "arguments": {
    "command": "ls -la /home/user"
  }
}
```

**Response:**
```json
{
  "violations": []
}
```

### Validated Rule Categories

| Category | Rules | Severity |
|----------|-------|----------|
| bash | BASH-001 | critical |

---

## guardrail_validate_git_operation

Validates git operations for safety and compliance.

### Description
Checks git commands against rules preventing force pushes, branch deletions, and history rewrites.

### Input Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `operation` | string | Yes | Git operation (push, commit, rebase, etc.) |
| `args` | array[string] | Yes | Command arguments |

### Return Value

```json
{
  "violations": [
    {
      "rule_id": "GIT-001",
      "severity": "error",
      "message": "Force push to main/master is blocked",
      "category": "git"
    }
  ]
}
```

### Example Usage

**Request (Force Push Blocked):**
```json
{
  "tool": "guardrail_validate_git_operation",
  "arguments": {
    "operation": "push",
    "args": ["--force", "origin", "main"]
  }
}
```

**Response:**
```json
{
  "violations": [
    {
      "rule_id": "GIT-001",
      "severity": "error",
      "message": "Force push to main/master is blocked",
      "category": "git"
    }
  ]
}
```

**Request (Safe Operation):**
```json
{
  "tool": "guardrail_validate_git_operation",
  "arguments": {
    "operation": "push",
    "args": ["origin", "feature-branch"]
  }
}
```

**Response:**
```json
{
  "violations": []
}
```

### Validated Rule Categories

| Category | Rules | Severity Range |
|----------|-------|----------------|
| git | GIT-001 to GIT-006 | error, warning |

---

## guardrail_validate_file_edit

Validates file edits for security and safety compliance.

### Description
Multi-purpose validation tool checking file paths, content changes, and security patterns in edits.

### Input Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | string | Yes | File path being edited |
| `content` | string | Yes | New file content |
| `original_content` | string | No | Previous content (for diff analysis) |

### Return Value

```json
{
  "violations": [
    {
      "rule_id": "API-001",
      "severity": "critical",
      "message": "API key exposure detected",
      "category": "security"
    }
  ]
}
```

### Example Usage

**Request (Secret Detection):**
```json
{
  "tool": "guardrail_validate_file_edit",
  "arguments": {
    "path": "config.js",
    "content": "const apiKey = 'sk_live_abc123xyz789secretkey';"
  }
}
```

**Response:**
```json
{
  "violations": [
    {
      "rule_id": "API-001",
      "severity": "critical",
      "message": "API key exposure",
      "category": "security"
    }
  ]
}
```

**Request (Protected File):**
```json
{
  "tool": "guardrail_validate_file_edit",
  "arguments": {
    "path": ".claude/config.json",
    "content": "{}",
    "original_content": "{\"key\": \"value\"}"
  }
}
```

**Response:**
```json
{
  "violations": [
    {
      "rule_id": "GENERAL-001",
      "severity": "error",
      "message": "Cannot modify protected files (.claude/, docs/, .md files)",
      "category": "general"
    }
  ]
}
```

### Validated Rule Categories

| Category | Rules | Purpose |
|----------|-------|---------|
| file_edit | GENERAL-001 | Protected path patterns |
| content | CODE-xxx | Code security patterns |
| edit | GIT-xxx | Git-related content |
| security | API-xxx, DB-xxx, CONT-xxx, CFG-xxx | Security credentials |

---

## Validation Engine Features

### Caching
- **TTL:** 30 seconds
- **Scope:** Rule patterns cached to reduce database load
- **Invalidation:** Automatic after TTL expires

### Severity Levels

| Level | Color | Action |
|-------|-------|--------|
| critical | Red | Blocks operation |
| error | Orange | Blocks operation |
| warning | Yellow | Warns, allows with confirmation |
| info | Blue | Informational only |

### Pattern Matching
- **Engine:** Go regexp package
- **Flags:** Case-insensitive (?i) by default
- **Validation:** Pre-compiled for performance

---

## Tool Selection Guide

### Use `guardrail_validate_bash` when:
- Executing shell commands via Bash tool
- Running system commands
- Processing user-provided command strings

### Use `guardrail_validate_git_operation` when:
- Performing git push operations
- Executing git commands with arguments
- Automating git workflows

### Use `guardrail_validate_file_edit` when:
- Writing to files
- Modifying configuration files
- Processing file uploads
- Checking code for secrets before commit

---

## Related Documentation

| Document | Purpose |
|----------|---------|
| [RULES_INDEX_MAP.md](RULES_INDEX_MAP.md) | Complete rule reference |
| [RULE_PATTERNS_GUIDE.md](RULE_PATTERNS_GUIDE.md) | Writing custom patterns |
| [AGENT_GUARDRAILS.md](AGENT_GUARDRAILS.md) | Main guardrails guide |
