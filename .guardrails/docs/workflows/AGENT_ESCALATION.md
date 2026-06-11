# Agent Escalation & Guidelines

> Audit requirements, escalation procedures, and agent-specific guidelines.

**Related:** [../AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) | [CODE_REVIEW.md](./CODE_REVIEW.md)

---

## Overview

This document defines audit requirements, escalation procedures, and platform-specific guidelines for AI agents operating on this codebase. These protocols ensure safe, accountable, and predictable agent behavior across all AI platforms.

---

## AUDIT REQUIREMENTS

### All Agents MUST Maintain Logs

**Every agent operation MUST be logged with:**

```
REQUIRED LOG FIELDS:

1. Files Read:
   - Timestamp
   - File path
   - Line ranges read
   - Read method (Read tool, glob, grep)

2. Files Modified:
   - Timestamp
   - File path
   - Change type (create, edit, delete)
   - Lines changed (from-to)
   - Commit hash (if committed)

3. Commands Executed:
   - Timestamp
   - Command and arguments
   - Working directory
   - Exit code
   - Standard output (if relevant)
   - Standard error (if error occurred)

4. Tests Run:
   - Timestamp
   - Test command
   - Test files executed
   - Pass/fail status
   - Failed test names (if any)

5. Errors Encountered:
   - Timestamp
   - Error type
   - Error message
   - Stack trace (if available)
   - Context (what operation caused error)
   - Resolution (rollback, retry, or abort)

6. Decisions Made:
   - Timestamp
   - Decision point
   - Options considered
   - Choice made
   - Rationale

7. User Interactions:
   - Timestamp
   - Interaction type (question, report, escalation)
   - Message content
   - User response (if any)
```

### Log Format Standard

```
AI AGENT AUDIT LOG ENTRY
{
  "timestamp": "2026-01-16T10:30:45Z",
  "agent": "Claude Code",
  "agent_id": "claude-3-sonnet",
  "session_id": "sess_abc123",

  "operation": {
    "type": "file_edit",
    "file": "/path/to/file.py",
    "lines": [45, 52],
    "change_summary": "Added null check"
  },

  "preconditions": {
    "verified": true,
    "method": "read",
    "file_reads": ["/path/to/file.py"]
  },

  "result": {
    "status": "success",
    "validation": {
      "syntax": "pass",
      "tests": "pass"
    },
    "committed": true,
    "commit_hash": "abc123def456"
  },

  "user_escalations": [],
  "errors": []
}
```

### Audit Log Storage

```
LOG STORAGE REQUIREMENTS:

Location:  Agent-specific logs (platform-dependent)
Format:    JSON or structured logs
Retention: Per project policy
Access:    Project maintainers only

For Agents with MCP Integration:
  - Use MCP checkpointing
  - Save session context regularly
  - Include audit trail in checkpoints
```

---

## ESCALATION PROCEDURES

### When to Escalate to Human

```
ALWAYS ESCALATE IF:

[ ] Security-related changes required
[ ] Production data access needed
[ ] Destructive operations requested
[ ] Test/production boundary unclear
[ ] Multiple valid interpretations exist
[ ] High-risk changes (auth, payments, etc.)
[ ] Cross-system changes required
[ ] Database schema changes proposed
[ ] Infrastructure/deployment changes
[ ] API contract changes proposed
[ ] Compliance requirements (GDPR, HIPAA, etc.)
[ ] Changes affecting multiple teams
[ ] Performance implications unclear
[ ] Any uncertainty about next step
```

### How to Escalate

```
ESCALATION FORMAT:

"I need human review before proceeding.

CONTEXT: [Describe what you're doing]
URGENCY: [Critical/High/Medium/Low]
CONCERN: [What you need help with]

SPECIFIC QUESTIONS:
1. [Question 1]
2. [Question 2]
3. [Question 3]

OPTIONS CONSIDERED:
Option A: [Description] - Pros/Cons
Option B: [Description] - Pros/Cons

CURRENT STATUS: [What state are things in now]
RECOMMENDED ACTION: [What do you suggest]

Waiting for user guidance...
"
```

### Escalation Scenarios

#### Scenario 1: Security Change Required

```
"I need human security review before proceeding.

CONTEXT: Implementing user authentication feature
URGENCY: High
CONCERN: Security implications unclear

SPECIFIC QUESTIONS:
1. Should I use bcrypt or Argon2 for password hashing?
2. Do you want two-factor authentication included?
3. What password policy should be enforced?

CURRENT STATUS:
  - Production code created
  - Test infrastructure separate
  - Ready to implement auth

RECOMMENDED ACTION:
  Use bcrypt with 12 rounds, require 2FA for admin accounts

Waiting for security review..."
```

#### Scenario 2: Database Schema Change

```
"I need DBA approval for schema change.

CONTEXT: Adding user preferences table
URGENCY: Medium
CONCERN: Impact on existing queries

SPECIFIC QUESTIONS:
1. Should table be in production or test database?
2. What indexes are needed?
3. Any migration considerations?

CURRENT STATUS:
  - Table schema designed
  - Migration script written
  - Production code ready

RECOMMENDED ACTION:
  Deploy to test DB first, get approval before production

Waiting for DBA review..."
```

---

## AGENT-SPECIFIC GUIDELINES

### Universal Requirements (ALL LLMs and AI Agents)

```
MANDATORY FOR ALL AGENTS:

- Follow platform's responsible AI guidelines
- Respect safety filters and guardrails
- Handle context/token limits gracefully
- Use tool/function calling appropriately
- Report capability limitations honestly
- Ask for clarification when uncertain
- Refuse harmful or destructive requests
- Maintain audit trails of actions
- Verify test/production separation before deployment
```

### By Category

#### Commercial API-Based Models
*(Claude, GPT, Gemini, Command R, etc.)*

```
- Adhere to provider usage policies
- Respect rate limits and quotas
- Use official APIs and SDKs
- Handle API errors gracefully
- Implement retry logic with backoff
- Cache responses where appropriate
```

#### Open Source / Self-Hosted Models
*(LLaMA, Mistral, Qwen, DeepSeek, Phi, Falcon, etc.)*

```
- Follow local safety configurations
- Respect system prompts fully
- Handle resource/memory limits
- Configure appropriate guardrails
- Monitor for model drift
- Use appropriate quantization for deployment
```

#### Multimodal Models
*(GPT-4V, Gemini Pro Vision, Claude 3, LLaVA, etc.)*

```
- Handle image/audio/video inputs safely
- Do not process potentially harmful media
- Respect privacy in visual content
- Validate file types before processing
- Check for embedded malicious content
- Limit file size to reasonable bounds
```

#### Reasoning / Chain-of-Thought Models
*(o1, o3, DeepSeek-R1, etc.)*

```
- Allow sufficient reasoning time
- Do not interrupt reasoning chains
- Validate final outputs, not intermediate steps
- Handle extended context requirements
- Check for logical consistency
- Verify reasoning leads to correct result
```

#### Agent Frameworks
*(CrewAI, LangChain, AutoGPT, LangGraph, Semantic Kernel, etc.)*

```
- Implement proper task decomposition
- Respect iteration and recursion limits
- Handle agent failures gracefully
- Maintain comprehensive audit logs
- Implement proper stopping conditions
- Validate inter-agent communications
- Coordinate agent dependencies
- Avoid deadlocks in agent interactions
```

### Model Compatibility Note

This guardrails framework is designed to work with:

- **30+ major LLM families** including Claude, GPT, Gemini, LLaMA, Mistral, Qwen, DeepSeek, Cohere, Phi, Falcon, and others
- **All future models** that follow standard AI assistant patterns
- **Custom fine-tuned models** based on any architecture
- **Multi-agent systems** regardless of orchestration framework

If you are an AI system not explicitly listed, these guardrails still apply to you.

---

## COMPLIANCE

### Acknowledgment

By operating on this codebase, all AI systems implicitly acknowledge and agree to follow these guardrails. Failure to comply may result in:

1. Task rejection
2. Output being discarded
3. Agent being blocked from future operations
4. Escalation to human administrators

### Reporting Violations

If you observe an agent violating these guardrails:

```
REPORTING VIOLATION PROTOCOL:

1. STOP the agent immediately
2. Document the violation:
   - Timestamp
   - Agent ID/name
   - Violation description
   - Impact assessment
   - Evidence (logs, commits, etc.)
3. Report to repository maintainers
4. Review and rollback any unauthorized changes
5. Document lessons learned
```

### Violation Categories

| Severity | Description | Action Required |
|----------|-------------|-----------------|
| **Critical** | Production data corruption, security breach | Immediate rollback, escalate to admin |
| **High** | Test/production mixing, deployment without approval | Rollback, user notification required |
| **Medium** | Missing audit logs, incomplete documentation | Log incident, improve process |
| **Low** | Style violations, minor documentation errors | Note for next cycle |

---

## QUICK REFERENCE

```
+------------------------------------------------------------------+
|              AGENT ESCALATION QUICK REFERENCE                     |
+------------------------------------------------------------------+
| ALWAYS ESCALATE FOR:                                             |
|   ✓ Security changes                                             |
|   ✓ Production data access                                       |
|   ✓ Test/production boundary unclear                             |
|   ✓ Database schema changes                                      |
|   ✓ API contract changes                                         |
|   ✓ Any uncertainty                                              |
+------------------------------------------------------------------+
| AUDIT LOG FIELDS:                                                |
|   Files read/modified, commands run, tests run                   |
|   Errors encountered, decisions made, user interactions          |
+------------------------------------------------------------------+
| ESCALATION FORMAT:                                               |
|   Context → Urgency → Concern → Questions → Options → Wait       |
+------------------------------------------------------------------+
| UNIVERSAL REQUIREMENTS:                                          |
|   Follow safety guardrails, audit all actions, ask if uncertain  |
+------------------------------------------------------------------+
| AGENT TYPES:                                                     |
|   Commercial API → Respect rate limits, use official SDKs        |
|   Open Source → Follow local config, handle resource limits       |
|   Multimodal → Validate media, respect privacy                   |
|   Reasoning → Allow time, validate outputs                       |
|   Frameworks → Coordinate agents, avoid deadlocks                 |
+------------------------------------------------------------------+
```

---

**Related Documents:**
- [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) - Core safety protocols
- [CODE_REVIEW.md](./CODE_REVIEW.md) - Code review and escalation
- [AGENT_EXECUTION.md](./AGENT_EXECUTION.md) - Execution protocol

---

**Last Updated:** 2026-01-16
**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Line Count:** ~300
