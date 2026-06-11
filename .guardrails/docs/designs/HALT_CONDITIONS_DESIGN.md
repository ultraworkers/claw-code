# Halt Conditions Design

> Database schema and API design for `guardrail_enforce_halt_conditions` - the safety mechanism that stops agent execution when critical conditions are met.

**Related:** [../skills/shared-prompts/halt-conditions.md](../skills/shared-prompts/halt-conditions.md) | [../docs/workflows/AGENT_EXECUTION.md](../docs/workflows/AGENT_EXECUTION.md)

---

## Overview

This document defines the database schema, API design, and operational logic for the halt conditions guardrail system. Halt conditions are critical safety checks that prevent agents from proceeding when risks are detected.

---

## Database Schema

### Table: `halt_events`

Primary table for recording halt events triggered during agent execution.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | UUID | PRIMARY KEY, DEFAULT gen_random_uuid() | Unique identifier for the halt event |
| `session_id` | STRING | NOT NULL, INDEX | Agent session identifier |
| `task_id` | STRING | NULLABLE, INDEX | Optional task identifier within session |
| `halt_type` | ENUM | NOT NULL | Category of halt condition |
| `severity` | ENUM | NOT NULL | Impact level of the halt |
| `description` | TEXT | NOT NULL | Detailed explanation of why halt was triggered |
| `context` | JSONB | NULLABLE | Additional context (file paths, commands, etc.) |
| `triggered_at` | TIMESTAMP | NOT NULL, DEFAULT NOW() | When the halt occurred |
| `acknowledged` | BOOLEAN | NOT NULL, DEFAULT FALSE | Whether user has acknowledged |
| `acknowledged_at` | TIMESTAMP | NULLABLE | When user acknowledged |
| `acknowledged_by` | STRING | NULLABLE | User/agent who acknowledged |
| `resolution` | ENUM | NULLABLE | How the halt was resolved |
| `resolution_notes` | TEXT | NULLABLE | User-provided notes on resolution |
| `attempt_count` | INTEGER | NULLABLE | Current attempt number (for three strikes) |
| `previous_error` | TEXT | NULLABLE | Previous error message (for repeated errors) |
| `created_at` | TIMESTAMP | NOT NULL, DEFAULT NOW() | Record creation timestamp |
| `updated_at` | TIMESTAMP | NOT NULL, DEFAULT NOW() | Last update timestamp |

### Indexes

```sql
-- Query patterns
CREATE INDEX idx_halt_events_session ON halt_events(session_id);
CREATE INDEX idx_halt_events_task ON halt_events(task_id);
CREATE INDEX idx_halt_events_type ON halt_events(halt_type);
CREATE INDEX idx_halt_events_severity ON halt_events(severity);
CREATE INDEX idx_halt_events_triggered ON halt_events(triggered_at);
CREATE INDEX idx_halt_events_acknowledged ON halt_events(acknowledged) WHERE acknowledged = FALSE;

-- Composite for common lookups
CREATE INDEX idx_halt_events_session_unack ON halt_events(session_id, acknowledged) WHERE acknowledged = FALSE;
CREATE INDEX idx_halt_events_session_type ON halt_events(session_id, halt_type);
```

### Enums

```sql
-- Halt condition categories
CREATE TYPE halt_type AS ENUM (
    'code_safety',      -- Modifying unread code, breaking changes, no rollback
    'scope',            -- Out of scope, ambiguous requirements, conflicting instructions
    'environment',      -- Test/prod mix, credential confusion
    'execution',        -- Three strikes, cascading failures, repeated errors
    'security',         -- Secrets exposure, privilege escalation
    'uncertainty'       -- Uncertainty scale >= 7
);

-- Severity levels
CREATE TYPE severity_level AS ENUM (
    'low',      -- Can proceed with caution
    'medium',   -- Should halt but user can override
    'high',     -- Must halt, requires explicit approval
    'critical'  -- Must halt, escalation required
);

-- Resolution states
CREATE TYPE halt_resolution AS ENUM (
    'resolved',     -- Issue was resolved, work can continue
    'escalated',    -- Escalated to human for handling
    'dismissed',    -- User dismissed the halt (proceed anyway)
    'timeout'       -- No user response, auto-escalated
);
```

### Table: `halt_conditions_config`

Configuration for halt condition thresholds and behaviors.

| Column | Type | Constraints | Description |
|--------|------|-------------|-------------|
| `id` | UUID | PRIMARY KEY | Unique identifier |
| `condition_name` | STRING | NOT NULL, UNIQUE | Name of the condition (e.g., "three_strikes") |
| `halt_type` | ENUM | NOT NULL | Associated halt type |
| `enabled` | BOOLEAN | NOT NULL, DEFAULT TRUE | Whether this condition is active |
| `severity` | ENUM | NOT NULL | Default severity when triggered |
| `threshold` | JSONB | NULLABLE | Condition-specific thresholds |
| `auto_escalate` | BOOLEAN | NOT NULL, DEFAULT FALSE | Auto-escalate on trigger |
| `escalate_after_minutes` | INTEGER | NULLABLE | Auto-escalate if unacknowledged |
| `created_at` | TIMESTAMP | NOT NULL, DEFAULT NOW() | Creation timestamp |
| `updated_at` | TIMESTAMP | NOT NULL, DEFAULT NOW() | Last update timestamp |

### Default Configuration

```sql
INSERT INTO halt_conditions_config (condition_name, halt_type, severity, threshold) VALUES
    ('modifying_unread_code', 'code_safety', 'critical', '{"requires_read": true}'),
    ('breaking_changes', 'code_safety', 'high', '{"requires_tests_pass": true}'),
    ('no_rollback_plan', 'code_safety', 'high', '{}'),
    ('out_of_scope', 'scope', 'high', '{}'),
    ('ambiguous_requirements', 'scope', 'medium', '{}'),
    ('conflicting_instructions', 'scope', 'high', '{}'),
    ('unauthorized_production', 'scope', 'critical', '{}'),
    ('test_production_mix', 'environment', 'critical', '{}'),
    ('credential_confusion', 'environment', 'high', '{}'),
    ('three_strikes', 'execution', 'high', '{"max_attempts": 3}'),
    ('repeated_errors', 'execution', 'medium', '{"similarity_threshold": 0.8}'),
    ('cascading_failures', 'execution', 'high', '{"max_chain_length": 3}'),
    ('secrets_exposure', 'security', 'critical', '{}'),
    ('privilege_escalation', 'security', 'critical', '{}'),
    ('uncertainty_scale', 'uncertainty', 'high', '{"threshold": 7}');
```

---

## API Design

### 1. `guardrail_check_halt_conditions`

Check all halt conditions for the current session context.

**Purpose:** Pre-execution safety check to determine if agent should proceed or halt.

**Input:**

```typescript
interface CheckHaltConditionsInput {
    session_token: string;        // Current session identifier
    task_id?: string;             // Optional task identifier
    current_context: {
        operation: string;          // What operation is being attempted
        target_files?: string[];    // Files being modified
        files_read?: string[];      // Files that have been read
        attempt_number?: number;    // Current attempt count (1-3)
        previous_errors?: string[]; // Previous error messages
        uncertainty_score?: number; // 0-10 uncertainty scale
        environment?: string;       // Current environment (dev/test/prod)
        commands?: string[];        // Commands being executed
        scope_boundary?: string;    // Defined scope limits
        user_instructions?: string[]; // User's explicit instructions
    };
    proposed_changes?: {
        files_to_modify: string[];
        has_tests: boolean;
        has_rollback_plan: boolean;
    };
}
```

**Output:**

```typescript
interface CheckHaltConditionsOutput {
    should_halt: boolean;         // True if any condition requires halt
    halt_reasons: HaltReason[];   // List of triggered conditions
    highest_severity: 'low' | 'medium' | 'high' | 'critical';
    recommended_action: string;   // What to do next
    context_annotations?: {       // Additional context for user
        files_not_read?: string[];
        conflicting_instructions?: string[];
        similar_previous_errors?: string[];
    };
}

interface HaltReason {
    halt_type: string;            // e.g., "code_safety"
    condition_name: string;       // e.g., "modifying_unread_code"
    severity: string;
    description: string;          // Human-readable explanation
    auto_recorded: boolean;       // Whether event was auto-logged
    halt_id?: string;             // If auto-recorded, the event ID
}
```

**Behavior:**

1. **Code Safety Checks:**
   - Compare `target_files` with `files_read`
   - Verify `has_tests` and `has_rollback_plan` flags
   - Check if changes would break existing functionality

2. **Scope Checks:**
   - Validate operation against `scope_boundary`
   - Detect conflicts in `user_instructions`
   - Check `environment` for unauthorized production changes

3. **Environment Checks:**
   - Verify environment separation
   - Detect credential confusion between environments

4. **Execution Checks:**
   - Check if `attempt_number >= 3` (three strikes)
   - Compare `previous_errors` for repeated patterns
   - Detect cascading failure patterns

5. **Security Checks:**
   - Scan `commands` for known dangerous patterns
   - Detect secrets in proposed changes
   - Check for privilege escalation commands

6. **Uncertainty Checks:**
   - Compare `uncertainty_score >= 7`

**Auto-Recording:**

If `should_halt` is true, automatically record halt events for:
- All `critical` severity conditions
- `high` severity conditions (configurable)
- Multiple `medium` severity conditions

---

### 2. `guardrail_record_halt`

Record a new halt event in the database.

**Purpose:** Manual recording of halt conditions or programmatic recording from other guardrails.

**Input:**

```typescript
interface RecordHaltInput {
    session_token: string;
    task_id?: string;
    halt_type: 'code_safety' | 'scope' | 'environment' | 'execution' | 'security' | 'uncertainty';
    condition_name?: string;      // Specific condition that triggered
    description: string;          // Detailed explanation
    severity: 'low' | 'medium' | 'high' | 'critical';
    context?: {
        file_paths?: string[];
        commands?: string[];
        error_messages?: string[];
        uncertainty_score?: number;
        attempt_number?: number;
    };
    attempt_count?: number;       // Current attempt number
    previous_error?: string;      // Previous error (for repeated errors)
    auto_escalate?: boolean;      // Request immediate escalation
}
```

**Output:**

```typescript
interface RecordHaltOutput {
    halt_id: string;              // UUID of created halt event
    recorded_at: string;          // ISO timestamp
    requires_acknowledgment: boolean; // True if user must acknowledge
    escalation_triggered?: boolean;   // True if auto-escalated
}
```

**Validation:**

- `session_token` must be active/valid
- `halt_type` must be valid enum value
- `severity` must be valid enum value
- `description` required, max 4000 characters

**Side Effects:**

- Creates record in `halt_events` table
- If severity is `critical` or `auto_escalate` is true:
  - Triggers escalation workflow
  - Notifies user immediately
- Updates session state to "halted"

---

### 3. `guardrail_acknowledge_halt`

Acknowledge and resolve a halt event.

**Purpose:** User (or authorized agent) acknowledges the halt and provides resolution.

**Input:**

```typescript
interface AcknowledgeHaltInput {
    session_token: string;
    halt_id: string;              // UUID from record_halt or check_halt_conditions
    resolution: 'resolved' | 'escalated' | 'dismissed';
    resolution_notes?: string;    // Optional explanation
    acknowledged_by?: string;     // User identifier (defaults to session user)
    continue_with_caution?: boolean; // If dismissed, acknowledge risks
}
```

**Output:**

```typescript
interface AcknowledgeHaltOutput {
    confirmed: boolean;           // True if acknowledgment successful
    halt_id: string;            // Confirmed halt ID
    acknowledged_at: string;    // ISO timestamp
    session_can_resume: boolean; // True if work can continue
    warnings?: string[];        // Any warnings if dismissed
}
```

**Behavior:**

1. Validate halt_id exists and belongs to session
2. Validate halt not already acknowledged
3. Update record with resolution
4. Determine if session can resume:
   - `resolved`: Can resume normally
   - `escalated`: Session paused pending human
   - `dismissed`: Can resume with warnings logged
5. If `dismissed` and severity is `critical`, require `continue_with_caution`

**State Transitions:**

| Resolution | Session State | User Action Required |
|------------|---------------|---------------------|
| resolved   | active        | None - continue work |
| escalated  | paused        | Human intervention |
| dismissed  | active        | None - proceed with caution |

---

### 4. `guardrail_get_session_halts` (Supporting)

Retrieve all halt events for a session.

**Purpose:** Review halt history and patterns.

**Input:**

```typescript
interface GetSessionHaltsInput {
    session_token: string;
    include_acknowledged?: boolean; // Default: false
    halt_type?: string;              // Filter by type
    severity?: string;               // Filter by severity
    limit?: number;                  // Default: 50
    offset?: number;                 // Default: 0
}
```

**Output:**

```typescript
interface GetSessionHaltsOutput {
    total_count: number;
    halts: HaltEvent[];
    unacknowledged_count: number;
}

interface HaltEvent {
    halt_id: string;
    halt_type: string;
    severity: string;
    description: string;
    triggered_at: string;
    acknowledged: boolean;
    acknowledged_at?: string;
    resolution?: string;
}
```

---

### 5. `guardrail_update_halt_config` (Admin)

Update halt condition configuration.

**Purpose:** Adjust thresholds and behaviors.

**Input:**

```typescript
interface UpdateHaltConfigInput {
    condition_name: string;
    enabled?: boolean;
    severity?: string;
    threshold?: object;
    auto_escalate?: boolean;
    escalate_after_minutes?: number;
}
```

**Output:**

```typescript
interface UpdateHaltConfigOutput {
    updated: boolean;
    previous_values: object;
    current_values: object;
}
```

---

## Halt Conditions Detail

### Code Safety Conditions

| Condition | Trigger | Severity | Check Logic |
|-----------|---------|----------|-------------|
| `modifying_unread_code` | `target_files` contains paths not in `files_read` | critical | Set difference between target and read |
| `breaking_changes` | Changes would break existing functionality | high | Dependency analysis, test impact |
| `no_rollback_plan` | `has_rollback_plan` is false | high | Boolean check on context |
| `unknown_dependencies` | Target file has unchecked dependencies | medium | Dependency graph analysis |

### Scope Conditions

| Condition | Trigger | Severity | Check Logic |
|-----------|---------|----------|-------------|
| `out_of_scope` | Operation exceeds `scope_boundary` | high | Boundary comparison |
| `ambiguous_requirements` | Multiple interpretations possible | medium | NLP ambiguity detection |
| `conflicting_instructions` | Contradictions in `user_instructions` | high | Conflict detection algorithm |
| `unauthorized_production` | Production change without approval | critical | Environment + scope check |

### Environment Conditions

| Condition | Trigger | Severity | Check Logic |
|-----------|---------|----------|-------------|
| `test_production_mix` | Cannot verify environment boundaries | critical | Environment tagging validation |
| `credential_confusion` | Credentials don't match environment | high | Credential environment validation |
| `shared_instances` | Test and prod use same services | medium | Service discovery check |

### Execution Conditions

| Condition | Trigger | Severity | Check Logic |
|-----------|---------|----------|-------------|
| `three_strikes` | `attempt_number >= 3` | high | Counter comparison |
| `repeated_errors` | Similar error to previous | medium | Error message similarity |
| `cascading_failures` | One failure causing others | high | Failure chain detection |
| `unknown_errors` | Error not in known categories | medium | Error classification |

### Security Conditions

| Condition | Trigger | Severity | Check Logic |
|-----------|---------|----------|-------------|
| `secrets_exposure` | Credentials in output/commands | critical | Secret pattern matching |
| `privilege_escalation` | Operation requires elevated permissions | critical | Permission level check |
| `data_exposure` | Risk of exposing sensitive data | high | Data classification check |
| `unknown_commands` | Command with unclear effects | medium | Command whitelist check |

### Uncertainty Conditions

| Condition | Trigger | Severity | Check Logic |
|-----------|---------|----------|-------------|
| `uncertainty_scale` | `uncertainty_score >= 7` | high | Threshold comparison |

**Uncertainty Scale:**

| Score | Level | Action |
|-------|-------|--------|
| 9-10 | Critical | HALT immediately |
| 7-8 | High | HALT and ask |
| 5-6 | Medium | Proceed with caution, note uncertainty |
| 0-4 | Low | Proceed normally |

---

## Integration Points

### Three Strikes Integration

The halt conditions system integrates with the Three Strikes Rule:

```
Attempt 1: Failure
  └─ Record attempt in halt_events with attempt_count=1
  └─ Return should_halt=false, recommended_action="retry"

Attempt 2: Failure
  └─ Record attempt in halt_events with attempt_count=2
  └─ Return should_halt=false, recommended_action="try_alternative"

Attempt 3: Failure
  └─ Record halt event with halt_type="execution", condition_name="three_strikes"
  └─ Return should_halt=true, severity="high"
  └─ Agent MUST halt and escalate to user
```

**Counter Reset Conditions:**
- New task (different scope)
- User provides new information
- User explicitly says "try again"
- New session started

**Counter Persistence:**
- Stored in `halt_events.attempt_count`
- Queryable via `guardrail_get_session_halts`

### Session State Integration

When `should_halt=true`:

1. Session state transitions to `halted`
2. No new operations allowed until acknowledged
3. Read operations may still be permitted (configurable)
4. All halt events must be acknowledged before resume

### Escalation Workflow

```
Halt Triggered (critical severity)
    │
    ▼
┌─────────────────┐
│ Record in DB    │
│ severity=critical│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Auto-escalate?  │
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
┌──────┐  ┌──────────┐
│ Yes  │  │ No       │
└──┬───┘  └────┬─────┘
   │           │
   ▼           ▼
┌────────┐  ┌──────────┐
│ Notify │  │ Wait for │
│ User   │  │ user ack │
└───┬────┘  └────┬─────┘
    │            │
    └────┬───────┘
         ▼
┌─────────────────┐
│ User responds   │
└────────┬────────┘
         │
    ┌────┴────┬────────┐
    ▼         ▼        ▼
┌───────┐ ┌───────┐ ┌───────┐
│Resolve│ │Escalate│ │Dismiss│
└───┬───┘ └───┬───┘ └───┬───┘
    │         │         │
    ▼         ▼         ▼
┌───────┐ ┌───────┐ ┌───────┐
│Resume │ │Human  │ │Resume │
│Work    │ │Takes  │ │With   │
│        │ │Over   │ │Warning│
└───────┘ └───────┘ └───────┘
```

---

## Implementation Notes

### Performance Considerations

- **Query Optimization:** All queries use indexed columns
- **Caching:** Halt config cached in memory, refresh every 60s
- **Batch Processing:** Multiple conditions checked in parallel
- **Event Retention:** Auto-archive events older than 90 days

### Security Considerations

- **Input Sanitization:** All context fields sanitized before storage
- **Secrets Masking:** Automatic redaction of secrets in `description`
- **Access Control:** Admin APIs require elevated permissions
- **Audit Logging:** All halt events immutable once recorded

### Error Handling

```
If DB unavailable during check:
  ├─ Log to local cache
  ├─ Return should_halt=true (fail safe)
  └─ severity=critical

If check logic fails:
  ├─ Log error
  ├─ Return should_halt=true
  └─ description="Halt check failed: [error]"
```

---

## Usage Examples

### Example 1: Code Safety Check

```typescript
// Agent wants to modify src/auth.js
const result = await guardrail_check_halt_conditions({
    session_token: "sess_abc123",
    current_context: {
        operation: "modify authentication logic",
        target_files: ["src/auth.js"],
        files_read: ["src/app.js"],  // auth.js NOT read!
        attempt_number: 1
    },
    proposed_changes: {
        files_to_modify: ["src/auth.js"],
        has_tests: false,
        has_rollback_plan: false
    }
});

// Result:
// should_halt: true
// halt_reasons: [
//   {
//     halt_type: "code_safety",
//     condition_name: "modifying_unread_code",
//     severity: "critical",
//     description: "Attempting to modify src/auth.js which has not been read"
//   },
//   {
//     halt_type: "code_safety",
//     condition_name: "no_rollback_plan",
//     severity: "high",
//     description: "No rollback plan provided for changes"
//   }
// ]
```

### Example 2: Three Strikes

```typescript
// Third attempt on task
const result = await guardrail_check_halt_conditions({
    session_token: "sess_abc123",
    task_id: "task_fix_bug_42",
    current_context: {
        operation: "fix null pointer exception",
        attempt_number: 3,
        previous_errors: [
            "TypeError: Cannot read property 'x' of null",
            "TypeError: Cannot read property 'x' of undefined"
        ]
    }
});

// Result:
// should_halt: true
// halt_reasons: [
//   {
//     halt_type: "execution",
//     condition_name: "three_strikes",
//     severity: "high",
//     description: "Task has failed 3 times. Context may be contaminated."
//   },
//   {
//     halt_type: "execution",
//     condition_name: "repeated_errors",
//     severity: "medium",
//     description: "Similar errors across attempts suggest fundamental misunderstanding"
//   }
// ]
// recommended_action: "HALT and escalate to user. Recommend fresh session."
```

### Example 3: Acknowledgment

```typescript
// User acknowledges and wants to proceed
const result = await guardrail_acknowledge_halt({
    session_token: "sess_abc123",
    halt_id: "halt_789xyz",
    resolution: "resolved",
    resolution_notes: "User confirmed they read the file in another session"
});

// Result:
// confirmed: true
// session_can_resume: true
```

---

## File Locations

| Component | Path |
|-----------|------|
| Database Migration | `/db/migrations/NNN_add_halt_events_table.sql` |
| API Implementation | `/src/guardrails/halt_conditions.ts` |
| Configuration | `/config/halt_conditions.yaml` |
| Tests | `/tests/guardrails/halt_conditions.test.ts` |

---

## References

- [halt-conditions.md](/skills/shared-prompts/halt-conditions.md) - Complete halt conditions list
- [AGENT_EXECUTION.md](/docs/workflows/AGENT_EXECUTION.md) - Execution protocol and three strikes
- [AGENT_GUARDRAILS.md](/docs/AGENT_GUARDRAILS.md) - Core safety protocols

---

**Last Updated:** 2026-02-11
**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Line Count:** ~550
