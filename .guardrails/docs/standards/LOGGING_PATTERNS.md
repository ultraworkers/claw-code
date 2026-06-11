# Logging Patterns for Agents

> Array-based structured logging format.

**Related:** [LOGGING_INTEGRATION.md](./LOGGING_INTEGRATION.md) | [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md)

---

## Overview

This document establishes array-based structured logging patterns for AI agent operations. Consistent logging enables debugging, auditing, and integration with external systems.

---

## Array-Based Logging Pattern

### Core Concept

Logs are stored as an array of structured entries, enabling:
- Easy filtering and searching
- Machine-readable format
- Consistent structure across operations
- Simple export to external systems

```
LOG ARRAY STRUCTURE:

logs = [
  { entry1 },
  { entry2 },
  { entry3 },
  ...
]
```

### Standard Log Entry Structure

```json
{
  "timestamp": "2026-01-14T15:30:00.000Z",
  "level": "INFO",
  "category": "file_operation",
  "action": "read",
  "target": "/path/to/file.py",
  "result": "success",
  "duration_ms": 45,
  "metadata": {
    "lines_read": 150,
    "file_size": 4096
  }
}
```

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| timestamp | ISO8601 string | When the action occurred |
| level | string | Log level (DEBUG, INFO, WARN, ERROR) |
| category | string | Category of operation |
| action | string | What was done |
| target | string | What was acted upon |
| result | string | success, failure, skipped |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| duration_ms | number | How long it took |
| metadata | object | Additional context |
| error | string | Error message if failed |
| stack_trace | string | Stack trace if error |
| agent_id | string | Which agent logged this |

---

## Log Levels

| Level | Code | Use For | Example |
|-------|------|---------|---------|
| DEBUG | 10 | Detailed diagnostics | "Reading line 45-50" |
| INFO | 20 | Normal operations | "File edited successfully" |
| WARN | 30 | Potential issues | "File larger than expected" |
| ERROR | 40 | Failures | "Edit failed: string not found" |

### Level Selection Guide

```
USE DEBUG:
- Internal decision points
- Variable values during processing
- Step-by-step progress

USE INFO:
- Operation start/complete
- Significant milestones
- User-relevant events

USE WARN:
- Unexpected but handled situations
- Deprecation notices
- Performance concerns

USE ERROR:
- Operation failures
- Unhandled exceptions
- Recovery needed
```

---

## Standard Log Categories

### File Operations Log

```json
{
  "category": "file_operation",
  "action": "read | write | edit | delete",
  "target": "/path/to/file",
  "metadata": {
    "lines_affected": 10,
    "old_content": "...",
    "new_content": "..."
  }
}
```

### Git Operations Log

```json
{
  "category": "git_operation",
  "action": "commit | push | pull | checkout | status",
  "target": "branch_name or file",
  "metadata": {
    "commit_hash": "abc123",
    "message": "commit message",
    "files_changed": 3
  }
}
```

### Validation Results Log

```json
{
  "category": "validation",
  "action": "syntax_check | test_run | lint",
  "target": "/path/to/file",
  "result": "pass | fail",
  "metadata": {
    "errors": [],
    "warnings": [],
    "test_count": 15,
    "pass_count": 15
  }
}
```

### Decision Points Log

```json
{
  "category": "decision",
  "action": "chose_path | skipped | escalated",
  "target": "decision_context",
  "metadata": {
    "options_considered": ["A", "B", "C"],
    "chosen": "A",
    "reason": "Best fit for requirements"
  }
}
```

---

## Log Array Management

### Initialization

```
At session start:
logs = []
```

### Appending Entries

```
For each operation:
logs.append({
  "timestamp": current_time(),
  "level": "INFO",
  ...
})
```

### Log Rotation/Limits

```
RECOMMENDED LIMITS:

- Max entries per session: 1000
- When limit reached: Archive oldest 500, keep recent 500
- On session end: Export full log
```

### Log Export

```json
{
  "session_id": "unique-session-id",
  "start_time": "2026-01-14T15:00:00Z",
  "end_time": "2026-01-14T16:30:00Z",
  "agent": "Claude Opus 4.5",
  "task_summary": "Implemented feature X",
  "log_count": 45,
  "logs": [...]
}
```

---

## Log Output Formats

### Human-Readable Format

```
[2026-01-14 15:30:00] INFO [file_operation] read /path/to/file.py → success (45ms)
[2026-01-14 15:30:01] INFO [file_operation] edit /path/to/file.py → success (120ms)
[2026-01-14 15:30:02] INFO [validation] syntax_check /path/to/file.py → pass
[2026-01-14 15:30:03] ERROR [validation] test_run tests/test_file.py → fail
```

### Machine-Readable Format (JSON Lines)

```
{"timestamp":"2026-01-14T15:30:00Z","level":"INFO","category":"file_operation","action":"read","target":"/path/to/file.py","result":"success"}
{"timestamp":"2026-01-14T15:30:01Z","level":"INFO","category":"file_operation","action":"edit","target":"/path/to/file.py","result":"success"}
```

### Summary Format

```
SESSION SUMMARY
===============
Duration: 1h 30m
Operations: 45
  - File operations: 20
  - Git operations: 10
  - Validations: 15

Results:
  - Success: 43
  - Warnings: 1
  - Errors: 1

Files Modified:
  - /path/to/file1.py
  - /path/to/file2.py
```

---

## Integration with Sprints

### Sprint Execution Logging

```
For each sprint step:

1. Log step start (INFO)
2. Log each sub-operation (DEBUG/INFO)
3. Log validation results (INFO/ERROR)
4. Log step complete (INFO)
```

### Sprint Completion Reports

```json
{
  "sprint_id": "SPRINT-2026-01-14",
  "status": "COMPLETE",
  "steps_completed": 5,
  "files_modified": ["file1.py", "file2.py"],
  "commits_created": 3,
  "validation_results": {
    "syntax_checks": "all_pass",
    "tests": "15/15 pass"
  },
  "log_summary": {
    "total_entries": 45,
    "errors": 0,
    "warnings": 1
  }
}
```

---

## Anti-Patterns

### What NOT to Do

```
DON'T:
- Log sensitive data (passwords, tokens, PII)
- Log entire file contents (use summaries)
- Skip logging errors
- Use inconsistent formats
- Forget timestamps
- Log at wrong level (DEBUG spam or missing INFO)

DO:
- Sanitize sensitive data before logging
- Log operation summaries
- Always log errors with context
- Use consistent structure
- Always include timestamp
- Choose appropriate level
```

---

## Quick Reference

```
+------------------------------------------------------------------+
|              LOGGING PATTERNS QUICK REFERENCE                     |
+------------------------------------------------------------------+
| ENTRY STRUCTURE:                                                  |
|   {                                                               |
|     "timestamp": "ISO8601",                                       |
|     "level": "DEBUG|INFO|WARN|ERROR",                            |
|     "category": "file_operation|git_operation|validation|...",   |
|     "action": "what was done",                                    |
|     "target": "what was acted on",                                |
|     "result": "success|failure"                                   |
|   }                                                               |
+------------------------------------------------------------------+
| LOG LEVELS:                                                       |
|   DEBUG - Detailed diagnostics                                    |
|   INFO  - Normal operations                                       |
|   WARN  - Potential issues                                        |
|   ERROR - Failures                                                |
+------------------------------------------------------------------+
| CATEGORIES:                                                       |
|   file_operation, git_operation, validation, decision             |
+------------------------------------------------------------------+
```

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-01-14
**Line Count:** ~320
