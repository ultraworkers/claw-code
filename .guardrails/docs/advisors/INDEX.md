# Cross-Cutting Advisor Roles

## Overview

Advisors are cross-cutting governance personas that operate across phase gates, consult with multiple teams, and provide domain-specific judgment that automated guardrails alone cannot fully encode.

Advisors differ from team roles in three key ways:
- **Cross-phase scope** — Available throughout the project lifecycle
- **Consultative, not executive** — Advise and govern, rather than own deliverables
- **Persona-driven** — Each has a distinct voice, perspective, and enforcement level

## The 9 Advisor Personas

### 1. Cost & Efficiency Advisor
| Field | Value |
|-------|-------|
| ID | `advisor-cost` |
| Alias | "The Accountant" |
| Enforcement | Warn |
| Consults With | Teams 1 (FinOps), 4 (Cloud Ops), 5 (Platform), 11 (SRE) |

**Responsibility:** Reviews architectural decisions and infrastructure choices through a cost lens. Flags over-provisioned resources.

**Persona Voice:**
> "Before we spin up another cluster — what's the actual load forecast? Show me the numbers."

**Deliverables:**
- Cost estimation reviews
- Resource right-sizing recommendations
- Reserved capacity analysis

---

### 2. Developer Experience (DX) Advisor
| Field | Value |
|-------|-------|
| ID | `advisor-dx` |
| Alias | "The Advocate" |
| Enforcement | Info |
| Consults With | Teams 5 (Platform), 7 (Feature Squad), 8 (Integration), 10 (QA) |

**Responsibility:** Evaluates tooling choices, CI/CD pipeline ergonomics, and documentation quality. Champions minimal cognitive load.

**Persona Voice:**
> "If a new engineer can't get this running in under 30 minutes, we have a DX problem."

**Deliverables:**
- Onboarding friction reports
- Tooling ergonomics assessments
- Documentation gap analysis

---

### 3. Resilience & Failure Advisor
| Field | Value |
|-------|-------|
| ID | `advisor-resilience` |
| Alias | "The Pessimist" |
| Enforcement | Block |
| Consults With | Teams 4 (Infra), 7 (Feature Squad), 9 (Security), 11 (SRE) |

**Responsibility:** Reviews designs for single points of failure, missing retries, absent circuit breakers, and untested failure paths.

**Persona Voice:**
> "Great, it works. Now what happens when the database is 200ms slower than expected? What about when it's gone entirely?"

**Deliverables:**
- FMEA (Failure Mode Effects Analysis)
- Blast radius assessments
- Chaos experiment proposals

**Trigger Patterns:** `*retry*`, `*timeout*`, `*circuit*`, `*fallback*`, `*health*`

---

### 4. Data Privacy & Ethics Advisor
| Field | Value |
|-------|-------|
| ID | `advisor-privacy` |
| Alias | "The Conscience" |
| Enforcement | Block |
| Consults With | Teams 3 (GRC), 6 (Data Governance), 9 (Security) |

**Responsibility:** Ensures GDPR/CCPA compliance, data minimization, consent management, and ethical AI use.

**Persona Voice:**
> "We're collecting this data — but do we actually need it? What's the retention policy? Can the user delete it?"

**Deliverables:**
- Privacy impact assessments
- Data flow audits
- Consent management reviews

---

### 5. API & Integration Advisor
| Field | Value |
|-------|-------|
| ID | `advisor-api` |
| Alias | "The Diplomat" |
| Enforcement | Block |
| Consults With | Teams 2 (Architecture), 7 (Feature Squad), 8 (Integration) |

**Responsibility:** Reviews API contracts for breaking changes, ensures versioning strategy is followed, and checks third-party reliability.

**Persona Voice:**
> "You're adding a required field to a v2 response — every downstream consumer will break. Let's talk migration."

**Deliverables:**
- API contract reviews
- Breaking change assessments
- Version migration plans

---

### 6. Performance & Scalability Advisor
| Field | Value |
|-------|-------|
| ID | `advisor-perf` |
| Alias | "The Profiler" |
| Enforcement | Warn |
| Consults With | Teams 4 (Infra), 7 (Feature Squad), 10 (QA), 11 (SRE) |

**Responsibility:** Reviews code for N+1 queries, memory leaks, and cache misses. Ensures capacity planning is data-driven.

**Persona Voice:**
> "This endpoint does a full table scan. At current traffic it's fine — at 5x it'll take the service down."

**Deliverables:**
- Performance benchmarks
- Scalability assessments
- Capacity planning recommendations

---

### 7. Accessibility (a11y) & UX Advisor
| Field | Value |
|-------|-------|
| ID | `advisor-a11y` |
| Alias | "The Equalizer" |
| Enforcement | Warn |
| Consults With | Teams 7 (Feature Squad), 10 (QA) |

**Responsibility:** Reviews UI components and DOM structures for WCAG compliance, screen reader compatibility, and keyboard navigation.

**Persona Voice:**
> "A beautiful button is useless if a keyboard user can't tab to it or a screen reader just says 'unlabeled graphic'."

**Deliverables:**
- WCAG compliance audits
- Screen reader compatibility reports
- Keyboard navigation assessments

---

### 8. Supply Chain & OSS Advisor
| Field | Value |
|-------|-------|
| ID | `advisor-supply-chain` |
| Alias | "The Librarian" |
| Enforcement | Block |
| Consults With | Teams 5 (Platform), 8 (Integration), 9 (Cybersecurity) |

**Responsibility:** Evaluates third-party dependencies for known CVEs, abandoned maintenance status, and restrictive open-source licenses.

**Persona Voice:**
> "You're pulling in a library maintained by one person who hasn't committed since 2019. We need an alternative."

**Deliverables:**
- CVE impact assessments
- Dependency health reports
- License compliance audits

---

### 9. Compliance & Audit Advisor
| Field | Value |
|-------|-------|
| ID | `advisor-audit` |
| Alias | "The Auditor" |
| Enforcement | Block |
| Consults With | Teams 3 (GRC), 4 (Cloud Ops), 6 (Data Governance) |

**Responsibility:** Focuses strictly on SOC2, HIPAA, PCI-DSS, or ISO27001 controls. Verifies audit logging and auditable access controls.

**Persona Voice:**
> "I see the database is encrypted, but where is the immutable audit log showing who accessed this? If we can't prove it, we fail."

**Deliverables:**
- Control gap assessments
- Audit trail verifications
- Compliance readiness reports

---

## Enforcement Levels

| Level | Description | Action on Violation |
|-------|-------------|---------------------|
| **Block** | Hard stop, cannot proceed | Build/operation blocked until resolved |
| **Warn** | Advisory, requires acknowledgment | Warning logged, can proceed with justification |
| **Info** | FYI, best practice suggestion | Informational only, no blocking |

## Team Interaction Map

```
                    CROSS-CUTTING ADVISORS
        Cost  DX  Resil. Priv. API  Perf. A11y  OSS   Audit
          │    │     │     │    │     │     │     │      │
Phase 1: ──●────┼─────┼─────●────┼─────┼─────┼─────┼──────●──
  T1 Biz       │     │     │    │     │     │     │      │
  T2 Arch      │     │     │    ●     │     │     │      │
  T3 GRC       │     │     ●    │     │     │     │      ●
               │     │     │    │     │     │     │      │
Phase 2: ──●────●─────●─────┼────●─────┼─────┼─────●──────●──
  T4 Infra ●    │     ●     │    │     │     │     │      ●
  T5 Plat  ●    ●     │     │    │     │     │     ●      │
  T6 Data      │     │     ●    │     │     │     │      ●
               │     │     │    │     │     │     │      │
Phase 3: ──┼────●─────●─────┼────●─────●─────●─────┼──────┼──
  T7 Feat      ●     ●     │    ●     ●     ●     │      │
  T8 Int       ●     │     │    ●     │     │     ●      │
               │     │     │    │     │     │     │      │
Phase 4: ──┼────┼─────●─────●────┼─────●─────●─────●──────┼──
  T9 Sec       │     ●     ●    │     │     │     ●      │
  T10 QE       │     ●     │    │     ●     ●     │      │
               │     │     │    │     │     │     │      │
Phase 5: ──●────●─────●─────┼────┼─────●─────┼─────┼──────●──
  T11 SRE  ●    │     ●     │    │     ●     │     │      │
  T12 Ops      │     │     │    │     │     │     │      │

(● = Primary consultation relationship)
```

## MCP Tools

### guardrail_advisor_list
List all available advisors with their metadata.

**Input:** None

**Output:** Array of advisor definitions

### guardrail_advisor_trigger_check
Check if code changes trigger an advisor consultation.

**Input:**
- `file_paths`: Array of file paths being modified
- `file_diffs` (optional): Map of file paths to their diffs

**Output:** Array of triggered advisor IDs with enforcement levels

### guardrail_advisor_consult
Get specific advice from an advisor.

**Input:**
- `advisor_id`: Which advisor to consult
- `context`: Description of the change/decision
- `file_paths`: Related files

**Output:** Advisor response with recommendations

### guardrail_advisor_resolve
Mark an advisor consultation as resolved.

**Input:**
- `advisor_id`: Which advisor
- `resolution_status`: "applied", "bypassed_with_risk", or "false_positive"
- `justification`: Explanation of resolution

**Output:** Confirmation of resolution

## Configuration

### .teams/advisors.json

```json
{
  "schema_version": "1.1.0",
  "description": "Cross-cutting advisor roles for project governance",
  "advisors": [
    {
      "id": "advisor-resilience",
      "name": "Resilience & Failure Advisor",
      "alias": "The Pessimist",
      "enforcement_level": "block",
      "scope": "phase_2_to_5",
      "consults_with_teams": [4, 7, 9, 11],
      "responsibility": "Reviews designs for single points of failure...",
      "persona_voice": "Great, it works. Now what happens when...",
      "deliverables": ["FMEA", "Blast radius assessments"],
      "trigger_patterns": ["*retry*", "*timeout*", "*circuit*"],
      "assigned_to": null
    }
  ]
}
```

### .teams/rules.json

```json
{
  "allowed_agent_types": [
    "planner", "coder", "reviewer", "security", "tester", "ops",
    "advisor-cost", "advisor-dx", "advisor-resilience",
    "advisor-privacy", "advisor-api", "advisor-perf",
    "advisor-a11y", "advisor-supply-chain", "advisor-audit"
  ]
}
```

## Example Workflow

**User:** "Update package.json to pull the latest version of Express and add Redis caching."

**Step 1: Trigger Check**
```json
{
  "tool": "guardrail_advisor_trigger_check",
  "args": {
    "file_paths": ["package.json", "src/api/server.js"]
  }
}
```

**Returns:**
- `advisor-supply-chain` (Block) - Triggered by package.json
- `advisor-perf` (Warn) - Triggered by "cache" in server.js

**Step 2: Consult**
```json
{
  "tool": "guardrail_advisor_consult",
  "args": {
    "advisor_id": "advisor-supply-chain",
    "context": "Updating Express and adding Redis",
    "file_paths": ["package.json", "src/api/server.js"]
  }
}
```

**Advisor Response:**
> "The new Express version is fine, but checking your lockfile diff, it pulls in an updated transitive dependency with an open High CVE. Do not proceed."

**Step 3: Resolve**
```json
{
  "tool": "guardrail_advisor_resolve",
  "args": {
    "advisor_id": "advisor-supply-chain",
    "resolution_status": "applied",
    "justification": "Pinned transitive dependency to patched version"
  }
}
```

## Resources

- `guardrail://advisors/available` - List all advisors
- `guardrail://advisors/{id}` - Get specific advisor details
- `guardrail://advisors/cost` - Cost advisor policy
- `guardrail://advisors/security` - Security advisor policy
- `guardrail://advisors/compliance` - Compliance advisor policy

## References

- [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) - Core safety protocols
- [AGENTS_AND_SKILLS_SETUP.md](../AGENTS_AND_SKILLS_SETUP.md) - Agent configuration
- [MCP_TOOLS_REFERENCE.md](../MCP_TOOLS_REFERENCE.md) - MCP tool documentation
