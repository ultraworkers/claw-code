# API Specification Standards

> OpenAPI vs OpenSpec guidance.

**Related:** [MODULAR_DOCUMENTATION.md](./MODULAR_DOCUMENTATION.md) | [DOCUMENTATION_UPDATES.md](../workflows/DOCUMENTATION_UPDATES.md)

---

## Overview

This document provides guidance on choosing between OpenAPI and OpenSpec for API documentation, including when to use each format, templates, and validation tools.

---

## OpenAPI Overview and Use Cases

### What is OpenAPI?

OpenAPI (formerly Swagger) is the industry-standard specification for describing RESTful APIs. It's widely supported by tools for documentation, code generation, and testing.

```yaml
# OpenAPI 3.0 basic structure
openapi: 3.0.0
info:
  title: API Name
  version: 1.0.0
paths:
  /endpoint:
    get:
      summary: Description
      responses:
        '200':
          description: Success
```

### OpenAPI Strengths

```
BEST FOR:
- REST APIs
- Public/external APIs
- APIs needing broad tooling support
- Code generation requirements
- Interactive documentation (Swagger UI)
- Contract-first development
```

### OpenAPI Tooling

| Tool | Purpose |
|------|---------|
| Swagger UI | Interactive docs |
| Swagger Editor | Write and validate |
| OpenAPI Generator | Code generation |
| Postman | API testing |
| Stoplight | Design and docs |

---

## OpenSpec Overview and Use Cases

### What is OpenSpec?

OpenSpec is a specification format that may offer different capabilities or focus areas compared to OpenAPI. (Note: If referring to a specific OpenSpec project, details would go here.)

```yaml
# OpenSpec example structure (hypothetical)
openspec: 1.0.0
info:
  title: API Name
  version: 1.0.0
operations:
  - name: getResource
    path: /resource/{id}
    method: GET
```

### OpenSpec Strengths

```
BEST FOR:
- Specific use cases where OpenSpec excels
- Projects already using OpenSpec
- Integration with OpenSpec-specific tools
- Non-REST API patterns
```

### OpenSpec Tooling

| Tool | Purpose |
|------|---------|
| [OpenSpec tools] | [Purposes] |

---

## When to Use OpenAPI

### Use OpenAPI When:

```
✓ Building REST APIs
✓ Need Swagger UI documentation
✓ Require code generation
✓ Working with external partners
✓ Need broad ecosystem support
✓ Using API gateways (most support OpenAPI)
✓ Team is familiar with OpenAPI
```

### OpenAPI Version Guidance

| Version | Recommendation |
|---------|----------------|
| OpenAPI 3.0 | Recommended - Stable, widely supported |
| OpenAPI 3.1 | Use for JSON Schema alignment |
| Swagger 2.0 | Legacy - Migrate to 3.x |

---

## When to Use OpenSpec

### Use OpenSpec When:

```
✓ Project already uses OpenSpec
✓ Specific tooling requires OpenSpec
✓ Non-REST patterns better suited
✓ Team prefers OpenSpec workflow
✓ Integration with OpenSpec ecosystem
```

---

## Hybrid Approach Guidance

### When to Use Both

```
SCENARIOS FOR HYBRID:

1. Migration period
   - Maintain both during transition
   - Generate one from the other if possible

2. Different audiences
   - OpenAPI for external partners
   - OpenSpec for internal systems

3. Different API types
   - OpenAPI for REST APIs
   - OpenSpec for other patterns
```

### Keeping Specs in Sync

```
SYNC STRATEGIES:

1. Single source of truth
   - Write in one format
   - Generate the other automatically

2. Automated validation
   - CI checks both specs match
   - Alert on drift

3. Regular reconciliation
   - Scheduled comparison
   - Manual sync if needed
```

---

## Template Files

### OpenAPI 3.0 Template

```yaml
openapi: 3.0.0
info:
  title: [API Name]
  description: [Brief description]
  version: 1.0.0
  contact:
    name: [Team Name]

servers:
  - url: https://api.example.com/v1
    description: Production
  - url: https://staging-api.example.com/v1
    description: Staging

paths:
  /resource:
    get:
      summary: List resources
      description: Returns a list of resources
      operationId: listResources
      tags:
        - Resources
      parameters:
        - name: limit
          in: query
          schema:
            type: integer
            default: 20
      responses:
        '200':
          description: Successful response
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/Resource'
        '401':
          description: Unauthorized
        '500':
          description: Internal server error

    post:
      summary: Create resource
      operationId: createResource
      tags:
        - Resources
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ResourceInput'
      responses:
        '201':
          description: Created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Resource'

  /resource/{id}:
    get:
      summary: Get resource by ID
      operationId: getResource
      tags:
        - Resources
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: Successful response
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Resource'
        '404':
          description: Not found

components:
  schemas:
    Resource:
      type: object
      properties:
        id:
          type: string
        name:
          type: string
        created_at:
          type: string
          format: date-time
      required:
        - id
        - name

    ResourceInput:
      type: object
      properties:
        name:
          type: string
      required:
        - name

  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT

security:
  - bearerAuth: []
```

### Minimal OpenAPI Template

```yaml
openapi: 3.0.0
info:
  title: [API Name]
  version: 1.0.0
paths:
  /health:
    get:
      summary: Health check
      responses:
        '200':
          description: OK
```

---

## Validation Tools and Commands

### OpenAPI Validation

```bash
# Using swagger-cli
npm install -g @apidevtools/swagger-cli
swagger-cli validate openapi.yaml

# Using spectral (with linting)
npm install -g @stoplight/spectral-cli
spectral lint openapi.yaml

# Using redocly
npm install -g @redocly/cli
redocly lint openapi.yaml
```

### Common Validation Errors

| Error | Cause | Fix |
|-------|-------|-----|
| Invalid reference | $ref points to non-existent schema | Check schema name |
| Missing required field | Required field not in properties | Add to properties |
| Invalid type | Wrong data type specified | Use valid OpenAPI types |
| Duplicate operationId | Same operationId used twice | Make unique |

### CI/CD Integration

```yaml
# GitHub Action for OpenAPI validation
- name: Validate OpenAPI
  run: |
    npm install -g @apidevtools/swagger-cli
    swagger-cli validate ./api/openapi.yaml
```

---

## File Organization

### Recommended Structure

```
api/
├── openapi.yaml           # Main spec file
├── schemas/               # Shared schemas
│   ├── user.yaml
│   └── error.yaml
├── paths/                 # Path definitions
│   ├── users.yaml
│   └── resources.yaml
└── examples/              # Example requests/responses
    └── user-example.json
```

### Splitting Large Specs

```yaml
# Main file references external files
openapi: 3.0.0
info:
  title: Large API
paths:
  /users:
    $ref: './paths/users.yaml'
components:
  schemas:
    User:
      $ref: './schemas/user.yaml'
```

---

## Quick Reference

```
+------------------------------------------------------------------+
|              API SPECIFICATION QUICK REFERENCE                    |
+------------------------------------------------------------------+
| CHOOSE OPENAPI WHEN:                                              |
|   • Building REST APIs                                            |
|   • Need Swagger UI docs                                          |
|   • Need code generation                                          |
|   • Working with external partners                                |
+------------------------------------------------------------------+
| CHOOSE OPENSPEC WHEN:                                             |
|   • Project already uses it                                       |
|   • Specific tooling requires it                                  |
+------------------------------------------------------------------+
| VALIDATE:                                                         |
|   swagger-cli validate openapi.yaml                               |
|   spectral lint openapi.yaml                                      |
+------------------------------------------------------------------+
| FILE STRUCTURE:                                                   |
|   api/                                                            |
|   ├── openapi.yaml (main)                                         |
|   ├── schemas/ (shared)                                           |
|   └── paths/ (endpoints)                                          |
+------------------------------------------------------------------+
```

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-01-14
**Line Count:** ~350
