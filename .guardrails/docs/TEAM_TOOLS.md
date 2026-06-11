# Team Layout Management Tools

> MCP tools for managing standardized team structure across projects

**Version:** 1.0
**Applies To:** All projects using the Agent Guardrails Template

---

## Overview

The Team Layout Management system provides MCP tools to initialize, manage, and validate team structures for software development projects. It enforces a standardized 12-team structure across 5 phases of the development lifecycle, ensuring proper governance, phase gates, and role assignments.

These tools use the Go `team` package (`mcp-server/internal/team/`) to provide real-time team management capabilities through the MCP protocol. As of v2.6.0, all functionality has been migrated from Python to Go for improved performance and security.

---

## Team Structure

The system manages 12 teams across 5 phases of the software development lifecycle:

### Phase 1: Strategy, Governance & Planning
- **Team 1:** Business & Product Strategy (The "Why")
- **Team 2:** Enterprise Architecture (The "Standards")
- **Team 3:** GRC (Governance, Risk, & Compliance)

### Phase 2: Platform & Foundation
- **Team 4:** Infrastructure & Cloud Ops
- **Team 5:** Platform Engineering (The "Internal Tools")
- **Team 6:** Data Governance & Analytics

### Phase 3: The Build Squads
- **Team 7:** Core Feature Squad (The "Devs")
- **Team 8:** Middleware & Integration

### Phase 4: Validation & Hardening
- **Team 9:** Cybersecurity (AppSec)
- **Team 10:** Quality Engineering (SDET)

### Phase 5: Delivery & Sustainment
- **Team 11:** Site Reliability Engineering (SRE)
- **Team 12:** IT Operations & Support (NOC)

For complete team details, see [TEAM_STRUCTURE.md](./TEAM_STRUCTURE.md).

---

## Available Tools

### guardrail_team_init

Initialize team structure for a project.

**Purpose:** Creates the initial team structure configuration for a new project, setting up all 12 teams with their default roles and states.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `project_name` | string | Yes | Name of the project (alphanumeric, hyphen, underscore only) |

**Constraints:**
- Project name must be 64 characters or less
- Allowed characters: letters, numbers, hyphens (`-`), underscores (`_`)
- No spaces or special characters permitted

**Example:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "guardrail_team_init",
    "arguments": {
      "project_name": "my-project"
    }
  }
}
```

**Response:** Confirmation of initialized 12-team structure for the project.

---

### guardrail_team_list

List all teams and their status.

**Purpose:** Display all teams for a project, including their assigned roles, completion status, and current state.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `project_name` | string | Yes | Name of the project |
| `phase` | string | No | Filter by phase (e.g., "Phase 1", "Phase 2") |

**Example (All Teams):**

```json
{
  "method": "tools/call",
  "params": {
    "name": "guardrail_team_list",
    "arguments": {
      "project_name": "my-project"
    }
  }
}
```

**Example (Filtered by Phase):**

```json
{
  "method": "tools/call",
  "params": {
    "name": "guardrail_team_list",
    "arguments": {
      "project_name": "my-project",
      "phase": "Phase 1"
    }
  }
}
```

**Response:** List of teams with role assignments and completion status.

---

### guardrail_team_assign

Assign a person to a role in a team.

**Purpose:** Assign team members to specific roles within a team, enabling proper resource allocation and responsibility tracking.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `project_name` | string | Yes | Name of the project |
| `team_id` | number | Yes | Team ID (1-12) |
| `role_name` | string | Yes | Name of the role to assign |
| `person` | string | Yes | Name of the person to assign |

**Example:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "guardrail_team_assign",
    "arguments": {
      "project_name": "my-project",
      "team_id": 7,
      "role_name": "Technical Lead",
      "person": "Jane Developer"
    }
  }
}
```

**Response:** Confirmation of role assignment with updated team roster.

---

### guardrail_team_unassign

Remove a person from a role in a team.

**Purpose:** Unassign team members from specific roles, enabling role reassignment and team restructuring.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `project_name` | string | Yes | Name of the project |
| `team_id` | number | Yes | Team ID (1-12) |
| `role_name` | string | Yes | Name of the role to unassign |

**Example:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "guardrail_team_unassign",
    "arguments": {
      "project_name": "my-project",
      "team_id": 7,
      "role_name": "Technical Lead"
    }
  }
}
```

**Response:** Confirmation of role unassignment.

---

### guardrail_team_status

Get phase or project status.

**Purpose:** Check the completion status of a specific phase or the entire project, showing which roles are assigned and which teams are ready.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `project_name` | string | Yes | Name of the project |
| `phase` | string | No | Specific phase to check (e.g., "Phase 1") |

**Example (Project Status):**

```json
{
  "method": "tools/call",
  "params": {
    "name": "guardrail_team_status",
    "arguments": {
      "project_name": "my-project"
    }
  }
}
```

**Example (Phase Status):**

```json
{
  "method": "tools/call",
  "params": {
    "name": "guardrail_team_status",
    "arguments": {
      "project_name": "my-project",
      "phase": "Phase 2"
    }
  }
}
```

**Response:** Phase status with team completion percentages and role assignments.

---

### guardrail_phase_gate_check

Check if phase gate requirements are met.

**Purpose:** Validate that all requirements are satisfied before transitioning from one phase to the next, enforcing the phase gate process.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `project_name` | string | Yes | Name of the project |
| `from_phase` | number | Yes | Source phase number (1-4) |
| `to_phase` | number | Yes | Target phase number (2-5) |

**Phase Gates:**

| Gate | From | To | Name |
|------|------|-----|------|
| 1_to_2 | Phase 1 | Phase 2 | Architecture Review Board |
| 2_to_3 | Phase 2 | Phase 3 | Environment Readiness |
| 3_to_4 | Phase 3 | Phase 4 | Feature Complete + Code Review |
| 4_to_5 | Phase 4 | Phase 5 | Security + QA Sign-off |

**Example:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "guardrail_phase_gate_check",
    "arguments": {
      "project_name": "my-project",
      "from_phase": 1,
      "to_phase": 2
    }
  }
}
```

**Response:** Gate name, required teams, and deliverables checklist.

---

### guardrail_agent_team_map

Get the team assignment for an agent type.

**Purpose:** Map AI agent types to their appropriate teams and roles, ensuring agents work within their designated scope.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `agent_type` | string | Yes | Type of agent (see supported types below) |

**Supported Agent Types:**

| Agent Type | Assigned Team | Phase | Roles |
|------------|---------------|-------|-------|
| `planner` | Team 2 | Phase 1 | Solution Architect, Business Systems Analyst |
| `architect` | Team 2 | Phase 1 | Chief Architect, Domain Architect |
| `infrastructure` | Team 4 | Phase 2 | Cloud Architect, IaC Engineer |
| `platform` | Team 5 | Phase 2 | CI/CD Architect, Kubernetes Administrator |
| `backend` | Team 7 | Phase 3 | Senior Backend Engineer, Technical Lead |
| `frontend` | Team 7 | Phase 3 | Senior Frontend Engineer, Accessibility Expert |
| `security` | Team 9 | Phase 4 | Security Architect, Vulnerability Researcher |
| `qa` | Team 10 | Phase 4 | QA Architect, SDET |
| `sre` | Team 11 | Phase 5 | SRE Lead, Observability Engineer |
| `ops` | Team 12 | Phase 5 | Release Manager, NOC Analyst |

**Example:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "guardrail_agent_team_map",
    "arguments": {
      "agent_type": "backend"
    }
  }
}
```

**Response:** Assigned team ID, phase, and applicable roles.

---

### guardrail_team_size_validate

Validate team sizes meet the 4-6 member requirement.

**Purpose:** Ensures all teams have between 4 and 6 members (inclusive) per TEAM-007 compliance rule.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `project_name` | string | Yes | Name of the project |
| `team_id` | number | No | Optional: Specific team ID to validate |

**Example:**

```json
{
  "method": "tools/call",
  "params": {
    "name": "guardrail_team_size_validate",
    "arguments": {
      "project_name": "my-project"
    }
  }
}
```

**Response:**

```
✅ All 12 teams have valid size (4-6 members)
```

Or if violations found:

```
❌ Team size violations found:
   Team 3 (GRC) has 3 members, minimum is 4
   Team 7 (Core Feature Squad) has 8 members, maximum is 6
```

---

## Phase Gates

Phase gates ensure proper completion and approval before progressing to the next phase of development.

### Gate 1: Architecture Review Board (Phase 1 to Phase 2)

**Required Teams:** 1, 2, 3
**Approval Required:** Team 2

**Deliverables:**
- Architecture Decision Records
- Approved Tech List
- Compliance Checklist

**Purpose:** Validate that business case, architecture, and compliance requirements are established before infrastructure work begins.

---

### Gate 2: Environment Readiness (Phase 2 to Phase 3)

**Required Teams:** 4, 5, 6
**Approval Required:** Teams 4, 5

**Deliverables:**
- Infrastructure Provisioned
- CI/CD Pipelines
- Data Models

**Purpose:** Ensure platform and infrastructure are ready before development teams begin building features.

---

### Gate 3: Feature Complete + Code Review (Phase 3 to Phase 4)

**Required Teams:** 7, 8
**Approval Required:** Team 7

**Deliverables:**
- Features Implemented
- Code Reviewed
- Documentation Complete

**Purpose:** Confirm that all features are developed and reviewed before entering validation and hardening phase.

---

### Gate 4: Security + QA Sign-off (Phase 4 to Phase 5)

**Required Teams:** 9, 10
**Approval Required:** Teams 9, 10

**Deliverables:**
- Security Review Passed
- Test Coverage Met
- UAT Sign-off

**Purpose:** Ensure security clearance and quality assurance approval before production deployment.

---

## Security

### Project Name Validation

All team tools validate the `project_name` parameter to prevent command injection and ensure consistent naming:

- **Maximum Length:** 64 characters
- **Allowed Characters:**
  - Letters (a-z, A-Z)
  - Numbers (0-9)
  - Hyphens (`-`)
  - Underscores (`_`)

**Valid Examples:**
- `my-project`
- `project_123`
- `team-alpha-v2`

**Invalid Examples:**
- `my project` (contains space)
- `project;rm -rf /` (contains special characters)
- `../etc/passwd` (path traversal attempt)

### Role Name Validation

The `role_name` parameter is validated for security and consistency:

- **Maximum Length:** 128 characters
- **Required:** Yes (cannot be empty)
- **Allowed Characters:**
  - Letters (a-z, A-Z)
  - Numbers (0-9)
  - Spaces
  - Hyphens (`-`)
  - Underscores (`_`)
  - Forward slashes (`/`)
  - Ampersands (`&`)
  - Parentheses (`(` `)`)
  - Periods (`.`)
- **Forbidden Patterns:** Shell metacharacters (`;`, `|`, `&&`, `||`, backticks, `$`, `<`, `>`)

**Valid Examples:**
- `Technical Lead`
- `Senior Backend Engineer`
- `DevOps/SRE`
- `QA Architect (Automation)`

**Invalid Examples:**
- `role; rm -rf /` (contains shell metacharacters)
- `$(whoami)` (contains command substitution)
- (empty string)

### Person Name Validation

The `person` parameter is validated to ensure safe input:

- **Maximum Length:** 128 characters
- **Required:** Yes (cannot be empty)
- **Allowed Characters:**
  - Letters (a-z, A-Z)
  - Spaces
  - Hyphens (`-`)
  - Apostrophes (`'`) for names like "O'Connor"
- **Forbidden Patterns:** Path traversal, shell metacharacters, special symbols

**Valid Examples:**
- `Alice Johnson`
- `Bob O'Connor`
- `Mary-Jane Watson`

**Invalid Examples:**
- `user; cat /etc/passwd` (contains shell metacharacters)
- `../../../etc/shadow` (path traversal attempt)
- (empty string)

### Phase Validation

The optional `phase` parameter must be one of the valid phase names:

- **Valid Values:** `Phase 1`, `Phase 2`, `Phase 3`, `Phase 4`, `Phase 5`
- **Case Sensitive:** Yes
- **Required:** No (optional filter)

**Valid Examples:**
- `Phase 1`
- `Phase 3`

**Invalid Examples:**
- `phase 1` (wrong case)
- `Phase One` (invalid format)
- `1` (missing "Phase" prefix)

### Error Handling

Team tools use standard HTTP status codes and structured error responses. All errors follow a consistent format with error code, message, and troubleshooting guidance.

#### Error Response Format

```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "TEAM-001: Team not found"
  }],
  "error_code": "TEAM-001",
  "error_message": "Team with ID 99 does not exist",
  "documentation_url": "https://docs.example.com/errors/TEAM-001"
}
```

---

#### Error Code Reference

| HTTP Code | Error Code | Description |
|-----------|------------|-------------|
| 400 | TEAM-001 | Team not found |
| 400 | TEAM-002 | Invalid team ID (must be 1-12) |
| 400 | TEAM-003 | Role not found in team |
| 400 | TEAM-004 | Person already assigned to role |
| 400 | TEAM-005 | Team size violation (TEAM-007) |
| 401 | AUTH-001 | Authentication required |
| 401 | AUTH-002 | Invalid API key |
| 403 | AUTH-003 | Insufficient permissions |
| 404 | PROJ-001 | Project not found |
| 404 | PROJ-002 | Project configuration missing |
| 429 | RATE-001 | Rate limit exceeded |
| 500 | SERV-001 | Internal server error |
| 500 | SERV-002 | Team manager script failure |

---

#### 400 Bad Request Errors

##### TEAM-001: Team Not Found

**Cause:** The specified team ID does not exist for the project.

**Example:**
```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "TEAM-001: Team not found"
  }],
  "error_code": "TEAM-001",
  "error_message": "Team with ID 99 does not exist in project 'my-project'"
}
```

**Troubleshooting:**
1. Verify the team ID is between 1 and 12
2. Run `guardrail_team_list` to see available teams
3. Check that the project was initialized with `guardrail_team_init`

---

##### TEAM-002: Invalid Team ID

**Cause:** Team ID is outside the valid range (1-12).

**Example:**
```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "TEAM-002: Invalid team ID"
  }],
  "error_code": "TEAM-002",
  "error_message": "Team ID must be between 1 and 12, got: 15"
}
```

**Troubleshooting:**
1. Use team IDs 1-12 only (see Team Structure section)
2. Verify your mapping logic for team assignments

---

##### TEAM-003: Role Not Found

**Cause:** Attempted to assign/unassign a role that does not exist in the team.

**Example:**
```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "TEAM-003: Role not found"
  }],
  "error_code": "TEAM-003",
  "error_message": "Role 'Junior Developer' not found in Team 7 (Core Feature Squad)"
}
```

**Troubleshooting:**
1. Check TEAM_STRUCTURE.md for valid role names per team
2. Use exact role names (case-sensitive)
3. Run `guardrail_team_list` to see assigned roles

---

##### TEAM-004: Person Already Assigned

**Cause:** Attempted to assign a person to a role that is already filled.

**Example:**
```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "TEAM-004: Person already assigned"
  }],
  "error_code": "TEAM-004",
  "error_message": "Role 'Technical Lead' in Team 7 already has 'Alice Johnson' assigned"
}
```

**Troubleshooting:**
1. Unassign the current person first with `guardrail_team_unassign`
2. Or assign the new person to a different role
3. Check current assignments with `guardrail_team_list`

---

##### TEAM-005: Team Size Violation

**Cause:** Operation would violate TEAM-007 compliance (4-6 members per team).

**Example:**
```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "TEAM-005: Team size violation"
  }],
  "error_code": "TEAM-005",
  "error_message": "Team 7 has 6 members (maximum). Cannot add more members."
}
```

**Troubleshooting:**
1. Check current team size with `guardrail_team_size_validate`
2. Unassign a member before adding a new one
3. Verify team size requirements in TEAM_STRUCTURE.md

---

#### 401 Unauthorized Errors

##### AUTH-001: Authentication Required

**Cause:** Request missing authentication token.

**Example:**
```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "AUTH-001: Authentication required"
  }],
  "error_code": "AUTH-001",
  "error_message": "API key required for this endpoint"
}
```

**Troubleshooting:**
1. Include `Authorization: Bearer YOUR_API_KEY` header
2. Verify API key is valid and not expired
3. Check API key permissions

---

##### AUTH-002: Invalid API Key

**Cause:** Provided API key is invalid or revoked.

**Example:**
```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "AUTH-002: Invalid API key"
  }],
  "error_code": "AUTH-002",
  "error_message": "The provided API key is not valid"
}
```

**Troubleshooting:**
1. Generate a new API key from the dashboard
2. Ensure the key has not been revoked
3. Check for typos in the Authorization header

---

#### 403 Forbidden Errors

##### AUTH-003: Insufficient Permissions

**Cause:** Authenticated user lacks permission for the operation.

**Example:**
```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "AUTH-003: Insufficient permissions"
  }],
  "error_code": "AUTH-003",
  "error_message": "User 'viewer@example.com' cannot modify team assignments"
}
```

**Troubleshooting:**
1. Verify user has appropriate role (admin, team-lead)
2. Check project permissions in admin panel
3. Contact project administrator for access

---

#### 404 Not Found Errors

##### PROJ-001: Project Not Found

**Cause:** Project name does not exist.

**Example:**
```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "PROJ-001: Project not found"
  }],
  "error_code": "PROJ-001",
  "error_message": "Project 'nonexistent-project' does not exist"
}
```

**Troubleshooting:**
1. Initialize project first with `guardrail_team_init`
2. Verify project name spelling (case-sensitive)
3. Check project exists: `guardrail_team_list --project-name <name>`

---

##### PROJ-002: Project Configuration Missing

**Cause:** Project was partially initialized or config file corrupted.

**Example:**
```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "PROJ-002: Project configuration missing"
  }],
  "error_code": "PROJ-002",
  "error_message": "Team configuration file missing for project 'my-project'"
}
```

**Troubleshooting:**
1. Re-initialize project with `guardrail_team_init`
2. Check `.teams/` directory for configuration files
3. Restore from backup if available

---

#### 429 Rate Limit Exceeded

##### RATE-001: Rate Limit Exceeded

**Cause:** Too many requests in a short time period.

**Example:**
```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "RATE-001: Rate limit exceeded"
  }],
  "error_code": "RATE-001",
  "error_message": "Rate limit exceeded. Retry after 60 seconds."
}
```

**Troubleshooting:**
1. Implement exponential backoff in batch scripts
2. Reduce request frequency (default limit: 100 req/min)
3. Contact support to increase rate limits

**Retry Strategy:**
```bash
# Example with exponential backoff
for i in 1 2 4 8; do
    response=$(curl -s ...)
    if ! echo "$response" | grep -q "RATE-001"; then
        break
    fi
    echo "Rate limited. Retrying in ${i}s..."
    sleep $i
done
```

---

#### 500 Internal Server Error

##### SERV-001: Internal Server Error

**Cause:** Unexpected server error.

**Example:**
```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "SERV-001: Internal server error"
  }],
  "error_code": "SERV-001",
  "error_message": "An unexpected error occurred. Incident ID: abc-123-xyz"
}
```

**Troubleshooting:**
1. Retry the request after a brief delay
2. Check service status page for outages
3. Contact support with the incident ID

---

##### SERV-002: Team Manager Execution Failure

**Cause:** Backend team management operation failed.

**Example:**
```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "SERV-002: Team manager execution failure"
  }],
  "error_code": "SERV-002",
  "error_message": "Team operation failed: unable to initialize team"
}
```

**Troubleshooting:**
1. Check server logs for error details
2. Verify `.teams/` directory has write permissions
3. Ensure project name is valid (alphanumeric, hyphens, underscores only)

---

#### Validation Errors

If parameter validation fails, tools return an error response:

```json
{
  "IsError": true,
  "Content": [{
    "Type": "text",
    "Text": "project_name must contain only letters, numbers, hyphens, and underscores"
  }],
  "error_code": "VALID-001",
  "error_message": "Invalid project_name format",
  "validation_errors": [{
    "field": "project_name",
    "code": "INVALID_CHARS",
    "message": "Contains invalid characters"
  }]
}
```

### Team Size Compliance (TEAM-007)

All teams **MUST** comply with the 4-6 member size requirement:

- **Minimum:** 4 members per team
- **Maximum:** 6 members per team
- **Rule ID:** TEAM-007
- **Severity:** Error

**Validation:**
Use `guardrail_team_size_validate` to check compliance:

```json
{
  "method": "tools/call",
  "params": {
    "name": "guardrail_team_size_validate",
    "arguments": {
      "project_name": "my-project"
    }
  }
}
```

**Why This Matters:**
- Teams with fewer than 4 members lack adequate role coverage
- Teams with more than 6 members suffer from coordination overhead
- This rule applies to human teams, AI agent teams, and mixed teams

### Implementation Details

Team tools use the native Go `team` package for persistence. Project data is stored in `.teams/{project_name}.json`. The Go implementation provides the same functionality as the previous Python script with improved performance and security.

---

## Workflow Integration

### Typical Project Setup Workflow

```
1. Initialize team structure
   └─ guardrail_team_init → Creates all 12 teams

2. Assign team members to roles
   └─ guardrail_team_assign → Assign people to specific roles

3. Check phase status
   └─ guardrail_team_status → Verify team readiness

4. Progress through phase gates
   └─ guardrail_phase_gate_check → Validate gate requirements
```

### Agent Assignment Workflow

```
1. Determine agent type (e.g., "backend", "security")

2. Get team mapping
   └─ guardrail_agent_team_map → Identify assigned team

3. Check team status
   └─ guardrail_team_status → Verify team is active

4. Begin work within assigned scope
```

### Example: Complete Project Initialization

```bash
# Initialize project
curl -X POST "http://localhost:8094/mcp/v1/message?session_id=abc123" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"guardrail_team_init","arguments":{"project_name":"web-platform"}}}'

# Assign backend lead
curl -X POST "http://localhost:8094/mcp/v1/message?session_id=abc123" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"guardrail_team_assign","arguments":{"project_name":"web-platform","team_id":7,"role_name":"Technical Lead","person":"Alice Developer"}}}'

# Check phase gate
curl -X POST "http://localhost:8094/mcp/v1/message?session_id=abc123" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"guardrail_phase_gate_check","arguments":{"project_name":"web-platform","from_phase":2,"to_phase":3}}}'
```

---

## Batch Operations

When setting up a complete project, you may need to perform multiple team assignments. Here are recommended patterns for batch operations:

### Batch Team Assignment Pattern

```bash
#!/bin/bash
# batch_assign_teams.sh - Assign multiple team members in sequence

PROJECT_NAME="$1"

if [ -z "$PROJECT_NAME" ]; then
    echo "Usage: $0 <project_name>"
    exit 1
fi

# Define assignments as: "team_id|role_name|person_name"
declare -a ASSIGNMENTS=(
    "2|Solution Architect|Alice Johnson"
    "2|Domain Architect|Bob Smith"
    "4|Cloud Architect|Carol White"
    "7|Technical Lead|David Brown"
    "7|Senior Backend Engineer|Eve Davis"
    "9|Security Architect|Frank Miller"
    "10|QA Architect|Grace Wilson"
)

echo "Initializing team structure..."
curl -s -X POST "http://localhost:8094/mcp/v1/message?session_id=$SESSION_ID" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"guardrail_team_init\",\"arguments\":{\"project_name\":\"$PROJECT_NAME\"}}}"

echo "Assigning team members..."
for assignment in "${ASSIGNMENTS[@]}"; do
    IFS='|' read -r team_id role_name person <<< "$assignment"

    echo "  -> Assigning $person as $role_name to Team $team_id"
    curl -s -X POST "http://localhost:8094/mcp/v1/message?session_id=$SESSION_ID" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"guardrail_team_assign\",\"arguments\":{\"project_name\":\"$PROJECT_NAME\",\"team_id\":$team_id,\"role_name\":\"$role_name\",\"person\":\"$person\"}}}"
done

echo "Validating team sizes..."
curl -s -X POST "http://localhost:8094/mcp/v1/message?session_id=$SESSION_ID" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"guardrail_team_size_validate\",\"arguments\":{\"project_name\":\"$PROJECT_NAME\"}}}"

echo "Done!"
```

### Batch Role Reassignment Pattern

```bash
#!/bin/bash
# batch_reassign.sh - Unassign and reassign roles for restructuring

PROJECT_NAME="$1"

# First unassign old roles, then assign new ones
declare -a UNASSIGNMENTS=(
    "7|Old Technical Lead"
    "7|Legacy Developer"
)

declare -a NEW_ASSIGNMENTS=(
    "7|Technical Lead|New Lead Name"
    "7|Senior Backend Engineer|New Developer"
)

# Unassign old roles
for unassign in "${UNASSIGNMENTS[@]}"; do
    IFS='|' read -r team_id role_name <<< "$unassign"
    echo "Unassigning $role_name from Team $team_id"
    curl -s -X POST "http://localhost:8094/mcp/v1/message?session_id=$SESSION_ID" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"guardrail_team_unassign\",\"arguments\":{\"project_name\":\"$PROJECT_NAME\",\"team_id\":$team_id,\"role_name\":\"$role_name\"}}}"
done

# Assign new roles
for assign in "${NEW_ASSIGNMENTS[@]}"; do
    IFS='|' read -r team_id role_name person <<< "$assign"
    echo "Assigning $person as $role_name to Team $team_id"
    curl -s -X POST "http://localhost:8094/mcp/v1/message?session_id=$SESSION_ID" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"guardrail_team_assign\",\"arguments\":{\"project_name\":\"$PROJECT_NAME\",\"team_id\":$team_id,\"role_name\":\"$role_name\",\"person\":\"$person\"}}}"
done
```

### Validation Before Phase Transition

```bash
#!/bin/bash
# validate_phase_transition.sh - Check phase gate before transitioning

PROJECT_NAME="$1"
FROM_PHASE="$2"
TO_PHASE="$3"

echo "Checking phase gate from Phase $FROM_PHASE to Phase $TO_PHASE..."

# Validate team sizes first
echo "Validating team sizes..."
curl -s -X POST "http://localhost:8094/mcp/v1/message?session_id=$SESSION_ID" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"guardrail_team_size_validate\",\"arguments\":{\"project_name\":\"$PROJECT_NAME\"}}}"

# Check phase status for all teams in source phase
echo "Checking teams in Phase $FROM_PHASE..."
curl -s -X POST "http://localhost:8094/mcp/v1/message?session_id=$SESSION_ID" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"guardrail_team_status\",\"arguments\":{\"project_name\":\"$PROJECT_NAME\",\"phase\":\"Phase $FROM_PHASE\"}}}"

# Check phase gate requirements
echo "Checking phase gate requirements..."
curl -s -X POST "http://localhost:8094/mcp/v1/message?session_id=$SESSION_ID" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"guardrail_phase_gate_check\",\"arguments\":{\"project_name\":\"$PROJECT_NAME\",\"from_phase\":$FROM_PHASE,\"to_phase\":$TO_PHASE}}}"

echo "Validation complete. Review output above before proceeding."
```

### Error Handling in Batch Operations

When performing batch operations, handle validation errors gracefully:

```bash
#!/bin/bash
# batch_with_error_handling.sh

PROJECT_NAME="$1"
TEMP_DIR=$(mktemp -d)
FAILED_FILE="$TEMP_DIR/failed_assignments.txt"
SUCCESS_COUNT=0
FAILURE_COUNT=0

process_assignment() {
    local team_id=$1
    local role_name=$2
    local person=$3

    response=$(curl -s -X POST "http://localhost:8094/mcp/v1/message?session_id=$SESSION_ID" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"tools/call\",\"params\":{\"name\":\"guardrail_team_assign\",\"arguments\":{\"project_name\":\"$PROJECT_NAME\",\"team_id\":$team_id,\"role_name\":\"$role_name\",\"person\":\"$person\"}}}")

    # Check if response indicates error
    if echo "$response" | grep -q '"IsError":true'; then
        echo "FAILED: $person as $role_name in Team $team_id"
        echo "$team_id|$role_name|$person" >> "$FAILED_FILE"
        ((FAILURE_COUNT++))
        return 1
    else
        echo "SUCCESS: $person as $role_name in Team $team_id"
        ((SUCCESS_COUNT++))
        return 0
    fi
}

# Process all assignments
# ... (assignment loop)

echo "---"
echo "Batch Operation Summary:"
echo "  Successful: $SUCCESS_COUNT"
echo "  Failed: $FAILURE_COUNT"

if [ $FAILURE_COUNT -gt 0 ]; then
    echo "Failed assignments saved to: $FAILED_FILE"
    echo "Review failures and retry if needed."
fi
```

---

## Related Documentation

- [TEAM_STRUCTURE.md](./TEAM_STRUCTURE.md) - Complete team structure and role definitions
- [../.guardrails/team-layout-rules.json](../.guardrails/team-layout-rules.json) - Machine-readable team layout rules
- [AGENT_GUARDRAILS.md](./AGENT_GUARDRAILS.md) - Core safety protocols for agents

---

**Last Updated:** 2026-02-15
**Version:** 1.0
