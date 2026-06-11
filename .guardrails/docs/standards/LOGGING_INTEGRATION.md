# External Logging Integration Hooks

> Interfaces for external logging systems.

**Related:** [LOGGING_PATTERNS.md](./LOGGING_PATTERNS.md) | [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md)

---

## Overview

This document defines hooks and interfaces for integrating agent logs with external logging systems. These patterns prepare the codebase for future integration with centralized logging, monitoring, and alerting platforms.

---

## Integration Architecture

### Hook Points

```
AGENT OPERATION
    │
    ├── [HOOK: pre_operation] ────→ Log operation start
    │
    ├── Execute operation
    │
    ├── [HOOK: post_operation] ───→ Log operation result
    │
    └── [HOOK: on_error] ─────────→ Log error details
```

### Data Flow Diagram

```
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│   Agent     │ ───→ │  Log Hook   │ ───→ │  External   │
│  Operation  │      │  Interface  │      │   System    │
└─────────────┘      └─────────────┘      └─────────────┘
                            │
                            ├──→ Webhook (HTTP)
                            ├──→ File (local/remote)
                            ├──→ Queue (message bus)
                            └──→ API (direct call)
```

---

## Standard Hook Interface

### Required Interface Methods

```typescript
interface LoggingHook {
  // Initialize the hook with configuration
  initialize(config: HookConfig): Promise<void>;

  // Send log entries to external system
  send(entries: LogEntry[]): Promise<SendResult>;

  // Flush any buffered entries
  flush(): Promise<void>;

  // Clean shutdown
  shutdown(): Promise<void>;
}

interface HookConfig {
  endpoint?: string;      // URL or path
  auth?: AuthConfig;      // Authentication
  batch_size?: number;    // Entries per send
  timeout_ms?: number;    // Send timeout
  retry_count?: number;   // Retry attempts
}

interface SendResult {
  success: boolean;
  entries_sent: number;
  errors?: string[];
}
```

### Configuration Schema

```json
{
  "logging_hook": {
    "type": "webhook | file | queue | api",
    "enabled": true,
    "config": {
      "endpoint": "https://logs.example.com/ingest",
      "auth": {
        "type": "bearer",
        "token_env": "LOG_API_TOKEN"
      },
      "batch_size": 100,
      "timeout_ms": 5000,
      "retry_count": 3
    }
  }
}
```

---

## Supported Integration Types

### Webhook Integration

Send logs via HTTP POST to external endpoint.

```
WEBHOOK FLOW:

1. Buffer log entries locally
2. When batch_size reached or flush called:
   POST /ingest
   Content-Type: application/json
   Authorization: Bearer <token>
   Body: { "logs": [...entries] }
3. Handle response
4. Retry on failure
```

**Configuration Example:**

```json
{
  "type": "webhook",
  "config": {
    "endpoint": "https://logs.example.com/api/v1/ingest",
    "auth": {
      "type": "bearer",
      "token_env": "LOGGING_API_TOKEN"
    },
    "headers": {
      "X-Source": "agent-guardrails"
    },
    "batch_size": 50,
    "timeout_ms": 10000
  }
}
```

### File-Based Integration

Write logs to file for pickup by external system.

```
FILE FLOW:

1. Buffer log entries
2. When batch_size reached or flush called:
   - Write JSON Lines to file
   - Rotate files by size/time
3. External system picks up files
```

**Configuration Example:**

```json
{
  "type": "file",
  "config": {
    "path": "/var/log/agent/{date}.jsonl",
    "rotation": {
      "max_size_mb": 100,
      "max_age_days": 7
    },
    "format": "jsonl"
  }
}
```

### Queue-Based Integration

Send logs to message queue for async processing.

```
QUEUE FLOW:

1. Buffer log entries
2. When batch_size reached:
   - Publish to queue topic
3. Consumer processes messages
```

**Configuration Example:**

```json
{
  "type": "queue",
  "config": {
    "provider": "redis | rabbitmq | sqs",
    "connection": {
      "host": "localhost",
      "port": 6379
    },
    "topic": "agent-logs",
    "batch_size": 100
  }
}
```

### Direct API Integration

Call logging API directly (e.g., Datadog, Splunk).

```
API FLOW:

1. Format entries for specific API
2. Call API endpoint
3. Handle API-specific responses
```

**Configuration Example:**

```json
{
  "type": "api",
  "config": {
    "provider": "datadog",
    "api_key_env": "DD_API_KEY",
    "site": "datadoghq.com",
    "service": "agent-guardrails",
    "source": "ai-agent"
  }
}
```

---

## Configuration Templates

### Minimal Configuration (File)

```json
{
  "logging_hook": {
    "type": "file",
    "enabled": true,
    "config": {
      "path": "./logs/agent.jsonl"
    }
  }
}
```

### Production Configuration (Webhook)

```json
{
  "logging_hook": {
    "type": "webhook",
    "enabled": true,
    "config": {
      "endpoint": "${LOG_ENDPOINT}",
      "auth": {
        "type": "bearer",
        "token_env": "LOG_TOKEN"
      },
      "batch_size": 100,
      "timeout_ms": 5000,
      "retry_count": 3,
      "retry_delay_ms": 1000
    }
  }
}
```

---

## Placeholder Implementations

### Stub Hook Template

For development/testing without external system:

```typescript
class StubLoggingHook implements LoggingHook {
  private buffer: LogEntry[] = [];

  async initialize(config: HookConfig): Promise<void> {
    console.log("StubLoggingHook initialized");
  }

  async send(entries: LogEntry[]): Promise<SendResult> {
    this.buffer.push(...entries);
    console.log(`[STUB] Received ${entries.length} log entries`);
    return { success: true, entries_sent: entries.length };
  }

  async flush(): Promise<void> {
    console.log(`[STUB] Flushing ${this.buffer.length} entries`);
    this.buffer = [];
  }

  async shutdown(): Promise<void> {
    await this.flush();
    console.log("[STUB] Shutdown complete");
  }

  // For testing: get buffered entries
  getBuffer(): LogEntry[] {
    return [...this.buffer];
  }
}
```

### Console Hook (Development)

```typescript
class ConsoleLoggingHook implements LoggingHook {
  async initialize(config: HookConfig): Promise<void> {}

  async send(entries: LogEntry[]): Promise<SendResult> {
    entries.forEach(entry => {
      const line = `[${entry.timestamp}] ${entry.level} [${entry.category}] ${entry.action} ${entry.target} → ${entry.result}`;
      console.log(line);
    });
    return { success: true, entries_sent: entries.length };
  }

  async flush(): Promise<void> {}
  async shutdown(): Promise<void> {}
}
```

---

## Migration Path

### From Local to External

```
MIGRATION STEPS:

1. PHASE 1: Local logging
   - Implement LOGGING_PATTERNS
   - Use StubLoggingHook or ConsoleLoggingHook
   - Verify log format and content

2. PHASE 2: File-based external
   - Configure FileLoggingHook
   - External system picks up files
   - Verify ingestion

3. PHASE 3: Direct integration
   - Configure WebhookLoggingHook or APILoggingHook
   - Remove file intermediate
   - Full real-time logging
```

### Gradual Rollout

```
1. Enable hook in development
2. Test with limited traffic
3. Monitor for errors
4. Gradually increase batch_size
5. Enable in production
```

---

## Error Handling

### External System Failures

```
ON SEND FAILURE:

1. Retry with exponential backoff
   - Attempt 1: immediate
   - Attempt 2: wait 1s
   - Attempt 3: wait 2s
   - Attempt 4: wait 4s

2. If all retries fail:
   - Log error locally
   - Continue operation (don't block agent)
   - Buffer entries for next attempt

3. Circuit breaker (optional):
   - After N consecutive failures, stop trying
   - Retry after cooldown period
```

### Retry Patterns

```typescript
async function sendWithRetry(
  entries: LogEntry[],
  maxRetries: number = 3
): Promise<SendResult> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await hook.send(entries);
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      await sleep(Math.pow(2, i) * 1000);
    }
  }
}
```

---

## Security Considerations

### Credential Management

```
RULES:

- NEVER hardcode API keys or tokens
- Use environment variables
- Use secret management (GitHub Secrets, Vault)
- Rotate credentials regularly
```

### Data Sanitization

```
BEFORE SENDING EXTERNALLY:

[ ] Remove passwords and tokens
[ ] Mask PII (email, phone, etc.)
[ ] Truncate large payloads
[ ] Remove file contents (log metadata only)
```

---

## Quick Reference

```
+------------------------------------------------------------------+
|              LOGGING INTEGRATION QUICK REFERENCE                  |
+------------------------------------------------------------------+
| HOOK INTERFACE:                                                   |
|   initialize(config) → Setup connection                           |
|   send(entries)      → Send log batch                             |
|   flush()            → Send buffered entries                      |
|   shutdown()         → Clean disconnect                           |
+------------------------------------------------------------------+
| INTEGRATION TYPES:                                                |
|   webhook → HTTP POST to endpoint                                 |
|   file    → Write to file for pickup                              |
|   queue   → Publish to message queue                              |
|   api     → Direct provider API                                   |
+------------------------------------------------------------------+
| ERROR HANDLING:                                                   |
|   - Retry with exponential backoff                                |
|   - Don't block agent on failures                                 |
|   - Log errors locally                                            |
+------------------------------------------------------------------+
```

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-01-14
**Line Count:** ~300
