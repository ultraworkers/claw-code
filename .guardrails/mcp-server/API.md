# Guardrail MCP Server - API Documentation

> Complete API reference for the Guardrail MCP Server REST endpoints.

---

## Base URLs

| Service | URL | Port |
|---------|-----|------|
| MCP Protocol | `http://localhost:8080` | 8080 |
| Web UI API | `http://localhost:8081` | 8081 |

---

## Authentication

All API endpoints (except health checks and Web UI) require authentication via API key.

### Header Format

```
Authorization: Bearer <api_key>
```

### API Key Types

| Key Type | Environment Variable | Purpose |
|----------|---------------------|---------|
| MCP | `MCP_API_KEY` | MCP protocol and general API access |
| IDE | `IDE_API_KEY` | IDE-specific endpoints |

### Authentication Errors

**401 Unauthorized**
```json
{
  "error": "Missing authorization header"
}
```

**401 Unauthorized**
```json
{
  "error": "Invalid API key"
}
```

---

## Health Endpoints

No authentication required.

### GET /health/live

Liveness probe - checks if the process is running.

**Response**
```json
{
  "status": "alive",
  "version": "1.0.0",
  "timestamp": "2026-02-07T10:00:00Z"
}
```

### GET /health/ready

Readiness probe - checks database and Redis connectivity.

**Response (200)**
```json
{
  "status": "ready",
  "version": "1.0.0",
  "timestamp": "2026-02-07T10:00:00Z"
}
```

**Response (503)**
```json
{
  "status": "not ready",
  "timestamp": "2026-02-07T10:00:00Z"
}
```

### GET /metrics

Prometheus metrics endpoint.

**Response**
```
# HELP guardrail_validations_total Total number of validations performed
# TYPE guardrail_validations_total counter
guardrail_validations_total{tool="bash",result="allowed"} 42
```

### GET /version

Server version information.

**Response**
```json
{
  "version": "1.0.0",
  "service": "guardrail-mcp",
  "timestamp": "2026-02-07T10:00:00Z"
}
```

---

## Documents API

### GET /api/documents

List all documents with pagination.

**Query Parameters**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| category | string | No | Filter by category (workflow, standard, guide, reference) |
| limit | integer | No | Items per page (default: 20, max: 100) |
| offset | integer | No | Offset for pagination (default: 0) |

**Response**
```json
{
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "slug": "agent-guardrails",
      "title": "Agent Guardrails",
      "content": "# Agent Guardrails...",
      "category": "standard",
      "path": "docs/AGENT_GUARDRAILS.md",
      "version": 1,
      "metadata": {},
      "created_at": "2026-01-14T10:00:00Z",
      "updated_at": "2026-02-07T15:30:00Z"
    }
  ],
  "pagination": {
    "total": 25,
    "limit": 20,
    "offset": 0
  }
}
```

### GET /api/documents/:id

Get a specific document by ID (UUID).

**Path Parameters**
| Name | Type | Description |
|------|------|-------------|
| id | UUID | Document ID |

**Response**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "slug": "agent-guardrails",
  "title": "Agent Guardrails",
  "content": "# Agent Guardrails...",
  "category": "standard",
  "path": "docs/AGENT_GUARDRAILS.md",
  "version": 1,
  "metadata": {},
  "created_at": "2026-01-14T10:00:00Z",
  "updated_at": "2026-02-07T15:30:00Z"
}
```

### PUT /api/documents/:id

Update a document.

**Request Body**
```json
{
  "title": "Updated Title",
  "content": "# Updated Content",
  "category": "standard",
  "metadata": {
    "author": "user@example.com"
  }
}
```

**Response**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "slug": "agent-guardrails",
  "title": "Updated Title",
  "content": "# Updated Content",
  "category": "standard",
  "version": 2,
  "updated_at": "2026-02-07T16:00:00Z"
}
```

**Error Response (Secrets Detected)**
```json
{
  "error": "Potential secrets detected in content",
  "findings": [
    {
      "pattern": "AWS Access Key ID",
      "line": 15,
      "column": 23,
      "match": "AKIA****XXXX",
      "description": "AWS IAM access key"
    }
  ]
}
```

### GET /api/documents/search

Full-text search documents.

**Query Parameters**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| q | string | Yes | Search query (max 200 chars) |
| limit | integer | No | Max results (default: 20, max: 50) |

**Response**
```json
{
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "slug": "agent-guardrails",
      "title": "Agent Guardrails",
      "content": "# Agent Guardrails...",
      "category": "standard",
      "path": "docs/AGENT_GUARDRAILS.md",
      "version": 1,
      "metadata": {},
      "created_at": "2026-01-14T10:00:00Z",
      "updated_at": "2026-02-07T15:30:00Z"
    }
  ],
  "query": "guardrail safety",
  "pagination": {
    "limit": 20
  }
}
```

---

## Rules API

### GET /api/rules

List prevention rules with pagination.

**Query Parameters**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| enabled | boolean | No | Filter by enabled status |
| category | string | No | Filter by category |
| limit | integer | No | Items per page (default: 20, max: 100) |
| offset | integer | No | Offset for pagination (default: 0) |

**Response**
```json
{
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "rule_id": "PREVENT-001",
      "name": "No Force Push",
      "pattern": "git push --force",
      "pattern_hash": "abc123...",
      "message": "Force push is not allowed",
      "severity": "error",
      "enabled": true,
      "category": "git",
      "created_at": "2026-01-14T10:00:00Z",
      "updated_at": "2026-01-14T10:00:00Z"
    }
  ],
  "pagination": {
    "total": 15,
    "limit": 20,
    "offset": 0
  }
}
```

### GET /api/rules/:id

Get a specific rule by ID (UUID).

**Path Parameters**
| Name | Type | Description |
|------|------|-------------|
| id | UUID | Rule ID |

**Response**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "rule_id": "PREVENT-001",
  "name": "No Force Push",
  "pattern": "git push --force",
  "message": "Force push is not allowed",
  "severity": "error",
  "enabled": true,
  "category": "git"
}
```

### POST /api/rules

Create a new prevention rule.

**Request Body**
```json
{
  "rule_id": "PREVENT-002",
  "name": "No rm -rf /",
  "pattern": "rm -rf /",
  "message": "Dangerous command detected",
  "severity": "error",
  "category": "bash",
  "enabled": true
}
```

**Response (201)**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440002",
  "rule_id": "PREVENT-002",
  "name": "No rm -rf /",
  "pattern": "rm -rf /",
  "message": "Dangerous command detected",
  "severity": "error",
  "enabled": true,
  "category": "bash",
  "created_at": "2026-02-07T16:00:00Z"
}
```

### PUT /api/rules/:id

Update a rule.

**Path Parameters**
| Name | Type | Description |
|------|------|-------------|
| id | UUID | Rule ID |

**Request Body**
```json
{
  "name": "Updated Rule Name",
  "pattern": "updated pattern",
  "message": "Updated message",
  "severity": "warning",
  "enabled": true
}
```

**Response**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "rule_id": "PREVENT-001",
  "name": "Updated Rule Name",
  "pattern": "updated pattern",
  "message": "Updated message",
  "severity": "warning",
  "enabled": true,
  "category": "git",
  "updated_at": "2026-02-07T16:00:00Z"
}
```

### DELETE /api/rules/:id

Delete a rule.

**Path Parameters**
| Name | Type | Description |
|------|------|-------------|
| id | UUID | Rule ID |

**Response (204)**
No content.

### PATCH /api/rules/:id

Partially update a rule (e.g., enable/disable).

**Path Parameters**
| Name | Type | Description |
|------|------|-------------|
| id | UUID | Rule ID |

**Request Body**
```json
{
  "enabled": false,
  "name": "Optional new name",
  "message": "Optional new message",
  "pattern": "Optional new pattern",
  "severity": "warning"
}
```

**Response**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "rule_id": "PREVENT-001",
  "name": "No Force Push",
  "pattern": "git push --force",
  "message": "Force push is not allowed",
  "severity": "error",
  "enabled": false,
  "category": "git",
  "updated_at": "2026-02-07T16:00:00Z"
}
```

---

## Projects API

### GET /api/projects

List all projects with pagination.

**Query Parameters**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| limit | integer | No | Items per page (default: 20, max: 100) |
| offset | integer | No | Offset for pagination (default: 0) |

**Response**
```json
{
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440003",
      "name": "My Project",
      "slug": "my-project",
      "guardrail_context": "# Project Context...",
      "active_rules": ["PREVENT-001", "PREVENT-002"],
      "metadata": {},
      "created_at": "2026-01-14T10:00:00Z",
      "updated_at": "2026-01-14T10:00:00Z"
    }
  ],
  "pagination": {
    "total": 8,
    "limit": 20,
    "offset": 0
  }
}
```

### GET /api/projects/:id

Get a project by ID (UUID).

**Path Parameters**
| Name | Type | Description |
|------|------|-------------|
| id | UUID | Project ID |

**Response**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440003",
  "name": "My Project",
  "slug": "my-project",
  "guardrail_context": "# Project Context...",
  "active_rules": ["PREVENT-001", "PREVENT-002"],
  "metadata": {
    "repository": "https://github.com/org/repo"
  }
}
```

### POST /api/projects

Create a new project.

**Request Body**
```json
{
  "name": "New Project",
  "slug": "new-project",
  "guardrail_context": "# Context",
  "active_rules": ["PREVENT-001"],
  "metadata": {}
}
```

**Response (201)**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440004",
  "name": "New Project",
  "slug": "new-project",
  "guardrail_context": "# Context",
  "active_rules": ["PREVENT-001"],
  "metadata": {},
  "created_at": "2026-02-07T16:00:00Z",
  "updated_at": "2026-02-07T16:00:00Z"
}
```

### PUT /api/projects/:id

Update a project.

**Path Parameters**
| Name | Type | Description |
|------|------|-------------|
| id | UUID | Project ID |

**Request Body**
```json
{
  "name": "Updated Project Name",
  "guardrail_context": "# Updated Context",
  "active_rules": ["PREVENT-001", "PREVENT-003"]
}
```

**Response**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440003",
  "name": "Updated Project Name",
  "slug": "my-project",
  "guardrail_context": "# Updated Context",
  "active_rules": ["PREVENT-001", "PREVENT-003"],
  "metadata": {},
  "updated_at": "2026-02-07T16:00:00Z"
}
```

### DELETE /api/projects/:id

Delete a project.

**Path Parameters**
| Name | Type | Description |
|------|------|-------------|
| id | UUID | Project ID |

**Response (204)**
No content.

---

## Failure Registry API

### GET /api/failures

List failure registry entries with pagination.

**Query Parameters**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| status | string | No | Filter by status (active, resolved, deprecated) |
| category | string | No | Filter by category |
| project | string | No | Filter by project slug |
| limit | integer | No | Items per page (default: 20, max: 100) |
| offset | integer | No | Offset for pagination |

**Response**
```json
{
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440004",
      "failure_id": "FAIL-001",
      "category": "deployment",
      "severity": "high",
      "error_message": "Production database overwritten",
      "root_cause": "Missing environment check",
      "affected_files": ["scripts/deploy.sh"],
      "status": "active",
      "project_slug": "my-project",
      "created_at": "2026-01-14T10:00:00Z"
    }
  ],
  "pagination": {
    "total": 42,
    "limit": 20,
    "offset": 0
  }
}
```

### GET /api/failures/:id

Get a specific failure entry.

**Path Parameters**
| Name | Type | Description |
|------|------|-------------|
| id | UUID | Failure ID |

**Response**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440004",
  "failure_id": "FAIL-001",
  "category": "deployment",
  "severity": "high",
  "error_message": "Production database overwritten",
  "root_cause": "Missing environment check",
  "affected_files": ["scripts/deploy.sh"],
  "status": "active",
  "project_slug": "my-project",
  "created_at": "2026-01-14T10:00:00Z"
}
```

### POST /api/failures

Create a new failure entry.

**Request Body**
```json
{
  "failure_id": "FAIL-002",
  "category": "security",
  "severity": "critical",
  "error_message": "Secret leaked in commit",
  "root_cause": "Pre-commit hook not installed",
  "affected_files": ["config/production.yml"],
  "regression_pattern": "password:\\s*['\"][^'\"]+['\"]",
  "status": "active",
  "project_slug": "my-project"
}
```

**Response (201)**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440005",
  "failure_id": "FAIL-002",
  "category": "security",
  "severity": "critical",
  "error_message": "Secret leaked in commit",
  "root_cause": "Pre-commit hook not installed",
  "affected_files": ["config/production.yml"],
  "regression_pattern": "password:\\s*['\"][^'\"]+['\"]",
  "status": "active",
  "project_slug": "my-project",
  "created_at": "2026-02-07T16:00:00Z"
}
```

### PUT /api/failures/:id

Update a failure entry (e.g., mark as resolved).

**Path Parameters**
| Name | Type | Description |
|------|------|-------------|
| id | UUID | Failure ID |

**Request Body**
```json
{
  "status": "resolved"
}
```

**Response**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440004",
  "failure_id": "FAIL-001",
  "category": "deployment",
  "severity": "high",
  "error_message": "Production database overwritten",
  "root_cause": "Missing environment check",
  "affected_files": ["scripts/deploy.sh"],
  "status": "resolved",
  "project_slug": "my-project",
  "created_at": "2026-01-14T10:00:00Z",
  "updated_at": "2026-02-07T16:00:00Z"
}
```

---

## IDE API

These endpoints are optimized for IDE integration.

### GET /ide/health

Health check for IDE API.

**Response**
```json
{
  "status": "ok"
}
```

### POST /ide/validate/file

Validate file content against guardrails.

**Request Body**
```json
{
  "file_path": "src/main.go",
  "content": "package main\n\nfunc main() {\n  // code here\n}",
  "language": "go",
  "project_slug": "my-project"
}
```

**Response**
```json
{
  "valid": false,
  "violations": [
    {
      "rule_id": "PREVENT-003",
      "rule_name": "Hardcoded Secret",
      "severity": "error",
      "message": "Potential hardcoded secret detected",
      "line": 15,
      "column": 23,
      "suggestion": "Use environment variables instead"
    }
  ]
}
```

### POST /ide/validate/selection

Validate a code selection (for real-time validation).

**Request Body**
```json
{
  "code": "rm -rf /",
  "language": "bash",
  "context": "cleanup script"
}
```

**Response**
```json
{
  "valid": false,
  "violations": [
    {
      "rule_id": "PREVENT-002",
      "rule_name": "No rm -rf /",
      "severity": "error",
      "message": "Dangerous command detected",
      "suggestion": "Use specific paths instead"
    }
  ]
}
```

### GET /ide/rules

Get active rules for a project.

**Query Parameters**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| project | string | No | Project slug (defaults to all active rules) |

**Response**
```json
{
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "rule_id": "PREVENT-001",
      "name": "No Force Push",
      "pattern": "git push --force",
      "severity": "error",
      "message": "Force push is not allowed",
      "category": "git"
    }
  ]
}
```

### GET /ide/quick-reference

Get quick reference documentation.

**Response**
```json
{
  "data": {
    "reference": "# Quick Reference\n\n## Forbidden Commands\n- rm -rf /\n- git push --force\n\n## Required Checks\n- Pre-work check\n- Validate file edits"
  }
}
```

---

## System API

### GET /api/stats

Get system statistics.

**Response**
```json
{
  "documents_count": 25,
  "rules_count": 15,
  "projects_count": 8,
  "failures_count": 42
}
```

### POST /api/ingest

Trigger document ingestion from filesystem.

**Response**
```json
{
  "status": "ingest started"
}
```

---

## Error Responses

### Standard Error Format

All error responses use the following format:

```json
{
  "error": "Human-readable error message"
}
```

### HTTP Status Codes

| Status | Meaning |
|--------|---------|
| 200 | Success |
| 201 | Created |
| 204 | No Content |
| 400 | Bad Request |
| 401 | Unauthorized |
| 403 | Forbidden |
| 404 | Not Found |
| 429 | Rate Limit Exceeded |
| 500 | Internal Server Error |
| 503 | Service Unavailable |

### Rate Limit Response (429)

```json
{
  "error": "Rate limit exceeded"
}
```

---

## Rate Limits

| Endpoint Type | Limit | Window |
|--------------|-------|--------|
| MCP | 1000 | per minute |
| IDE | 500 | per minute |
| Session | 100 | per minute |

---

## Data Models

### Severity Levels

| Level | Description | Action |
|-------|-------------|--------|
| error | Critical violation | halt operation |
| warning | Potential issue | confirm before proceeding |
| info | Informational | log only |

### Failure Status

| Status | Description |
|--------|-------------|
| active | Currently relevant |
| resolved | Fixed and verified |
| deprecated | No longer applicable |

### Document Categories

| Category | Description |
|----------|-------------|
| workflow | Process documentation |
| standard | Coding standards |
| guide | How-to guides |
| reference | Quick reference |

---

## Pagination Standards

All list endpoints use consistent pagination:

### Request Parameters

| Parameter | Type | Default | Max | Description |
|-----------|------|---------|-----|-------------|
| limit | integer | 20 | 100 | Items per page |
| offset | integer | 0 | - | Number of items to skip |

### Response Format

```json
{
  "data": [...],
  "pagination": {
    "total": 100,
    "limit": 20,
    "offset": 0
  }
}
```

### Calculating Next Page

```
next_offset = current_offset + limit
has_more = (offset + limit) < total
```

---

*Last Updated: 2026-02-08*
*Version: 1.9.5*
