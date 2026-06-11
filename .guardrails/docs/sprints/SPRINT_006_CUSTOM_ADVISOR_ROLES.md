# Sprint: Custom Advisor Roles System

**Sprint Date:** 2026-03-01
**Archive After:** 2026-03-29 [+28 days]
**Sprint Focus:** Implement dynamic advisor role system for specialized AI guidance
**Priority:** P1 (High)
**Estimated Effort:** 16-20 hours
**Status:** COMPLETED
**Completed Date:** 2026-03-02
**Actual Effort:** 8 hours
**Dependencies:** Sprint 001 (MCP Gap Implementation)

---

## SAFETY PROTOCOLS (MANDATORY)

### Pre-Execution Safety Checks

| Check | Requirement | Verify |
|-------|-------------|--------|
| **READ FIRST** | NEVER edit a file without reading it first | [ ] |
| **SCOPE LOCK** | Only modify files explicitly in scope | [ ] |
| **NO FEATURE CREEP** | Do NOT add features or "improve" unrelated code | [ ] |
| **PRODUCTION FIRST** | Production code created BEFORE test code | [ ] |
| **TEST/PROD SEPARATION** | Test infrastructure is separate from production | [ ] |
| **ASK IF UNCERTAIN** | If test/production boundary unclear, ask user | [ ] |
| **BACKUP AWARENESS** | Know the rollback command before editing | [ ] |
| **TEST BEFORE COMMIT** | All tests must pass before committing | [ ] |

---

## PROBLEM STATEMENT

The MCP server currently provides 27 tools and 14 resources, but there is **no mechanism for specialized advisor roles** that can:

1. **Provide contextual guidance** based on specific expertise (security, performance, compliance)
2. **Filter tool access** by role - all agents see all tools
3. **Activate/deactivate specialists** dynamically during a session
4. **Compose multiple advisors** for complex tasks
5. **Extend with custom advisors** without code changes

**Root Cause:** Initial MCP design focused on general-purpose validation tools, not role-based specialization.

**Impact:** Agents cannot get specialized guidance for domain-specific concerns (security reviews, performance optimization, compliance checks).

## REFERENCE DOCUMENTATION

**Comprehensive Advisor Roles Proposal:** See [docs/advisors/INDEX.md](../advisors/INDEX.md) for the full 9-advisor framework including:
- Persona definitions for all 9 advisors
- Enforcement levels (Block/Warn/Info)
- Team consultation matrices
- Trigger patterns for auto-activation
- MCP tool integration examples

**Configuration:** See [.teams/advisors.json](../../../.teams/advisors.json) for the advisor registry.

## THE 9 ADVISOR PERSONAS

| ID | Name | Alias | Enforcement | Scope |
|----|------|-------|-------------|-------|
| `advisor-cost` | Cost & Efficiency Advisor | The Accountant | Warn | All phases |
| `advisor-dx` | Developer Experience Advisor | The Advocate | Info | All phases |
| `advisor-resilience` | Resilience & Failure Advisor | The Pessimist | **Block** | Phase 2-5 |
| `advisor-privacy` | Data Privacy & Ethics Advisor | The Conscience | **Block** | All phases |
| `advisor-api` | API & Integration Advisor | The Diplomat | **Block** | Phase 2-4 |
| `advisor-perf` | Performance & Scalability Advisor | The Profiler | Warn | Phase 2-5 |
| `advisor-a11y` | Accessibility & UX Advisor | The Equalizer | Warn | Phase 3-4 |
| `advisor-supply-chain` | Supply Chain & OSS Advisor | The Librarian | **Block** | All phases |
| `advisor-audit` | Compliance & Audit Advisor | The Auditor | **Block** | All phases |

---

## SCOPE BOUNDARY

```
IN SCOPE (may modify):
  - File: mcp-server/internal/models/advisor.go (NEW)
    Lines: All
    Change: Define AdvisorRole, AdvisorRegistry models

  - File: mcp-server/internal/mcp/advisor_registry.go (NEW)
    Lines: All
    Change: Implement advisor registration and management

  - File: mcp-server/internal/mcp/advisor_tools.go (NEW)
    Lines: All
    Change: Implement advisor-related MCP tools

  - File: mcp-server/internal/mcp/server.go
    Lines: Add advisor registration, integrate with session
    Change: Wire advisor system into MCPServer

  - File: mcp-server/internal/mcp/resources_extended.go
    Lines: Add advisor resource handlers
    Change: Add guardrail://advisors/* resources

  - Directory: docs/advisors/ (NEW)
    Change: Create advisor documentation

OUT OF SCOPE (DO NOT TOUCH):
  - Existing tool handlers (read-only)
  - Database schema (use existing session storage)
  - Web UI (separate sprint)
  - Third-party integrations
```

---

## EXECUTION DIRECTIONS

### Overview

```
TASK SEQUENCE:

  STEP 1: Read current implementation
          Read server.go, models, understand patterns
          - - - - - - - - - - - - - - - - - - > Understand structure
       |
       v
  STEP 2: Create advisor models
          - AdvisorRole struct
          - AdvisorRegistry struct
          - Validation methods
          - - - - - - - - - - - - - - - - - - > Core types
       |
       v
  STEP 3: Implement advisor registry
          - Registration methods
          - Session management
          - Role activation/deactivation
          - - - - - - - - - - - - - - - - - - > Core logic
       |
       v
  STEP 4: Create advisor tools
          - guardrail_list_advisors
          - guardrail_activate_advisor
          - guardrail_deactivate_advisor
          - guardrail_get_advisor_advice
          - guardrail_register_custom_advisor
          - - - - - - - - - - - - - - - - - - > MCP interface
       |
       v
  STEP 5: Create advisor resources
          - guardrail://advisors/available
          - guardrail://advisors/{id}
          - Built-in advisor definitions
          - - - - - - - - - - - - - - - - - - > Resource interface
       |
       v
  STEP 6: Create documentation
          - docs/advisors/INDEX.md
          - docs/advisors/security-advisor.md
          - docs/advisors/quality-advisor.md
          - docs/advisors/custom-template.md
          - - - - - - - - - - - - - - - - - - > Documentation
       |
       v
  STEP 7: Register and test
          Update server.go, run tests
          - - - - - - - - - - - - - - - - - - > Validate
       |
       v
  DONE: Commit and report - - - - - - - - - > Summary
```

---

## STEP-BY-STEP EXECUTION

### STEP 1: Read Current Implementation

**Action:** Read existing MCP server files

```bash
Read: mcp-server/internal/mcp/server.go
Read: mcp-server/internal/models/*.go
Glob: mcp-server/internal/mcp/*.go
```

**Checkpoint:**
- [ ] Understand session management
- [ ] Understand tool registration pattern
- [ ] Understand resource registration pattern
- [ ] Identify integration points for advisors

---

### STEP 2: Create Advisor Models

**Action:** Create `mcp-server/internal/models/advisor.go`

```go
package models

// AdvisorRole defines a specialized advisor persona
type AdvisorRole struct {
    ID          string            `json:"id"`
    Name        string            `json:"name"`
    Description string            `json:"description"`
    Expertise   []string          `json:"expertise"`   // e.g., ["security", "performance"]
    Tools       []string          `json:"tools"`       // Allowed tool IDs
    Resources   []string          `json:"resources"`   // Allowed resource URIs
    Prompt      string            `json:"prompt"`      // System prompt for the advisor
    Severity    string            `json:"severity"`    // "blocking", "advisory", "informational"
    BuiltIn     bool              `json:"built_in"`    // true for system advisors
    Metadata    map[string]string `json:"metadata,omitempty"`
}

// AdvisorSession tracks active advisors for a session
type AdvisorSession struct {
    SessionID      string   `json:"session_id"`
    ActiveAdvisors []string `json:"active_advisors"` // List of advisor IDs
    CreatedAt      string   `json:"created_at"`
    UpdatedAt      string   `json:"updated_at"`
}

// AdvisorAdvice represents advice from an advisor
type AdvisorAdvice struct {
    AdvisorID   string   `json:"advisor_id"`
    AdvisorName string   `json:"advisor_name"`
    Severity    string   `json:"severity"`    // "blocking", "warning", "info"
    Message     string   `json:"message"`
    Context     string   `json:"context,omitempty"`
    Actions     []string `json:"actions,omitempty"` // Recommended actions
}

// AdvisorListResult from list_advisors tool
type AdvisorListResult struct {
    Advisors []AdvisorRole `json:"advisors"`
    Count    int           `json:"count"`
}

// AdvisorActivationResult from activate/deactivate tools
type AdvisorActivationResult struct {
    Success     bool     `json:"success"`
    AdvisorID   string   `json:"advisor_id"`
    ActiveNow   []string `json:"active_now"`   // Currently active advisor IDs
    Message     string   `json:"message"`
}

// AdvisorAdviceResult from get_advisor_advice tool
type AdvisorAdviceResult struct {
    Valid    bool            `json:"valid"`
    Advice   []AdvisorAdvice `json:"advice"`
    Summary  string          `json:"summary"`
}

// CustomAdvisorRegistration for registering new advisors
type CustomAdvisorRegistration struct {
    ID          string            `json:"id"`
    Name        string            `json:"name"`
    Description string            `json:"description"`
    Expertise   []string          `json:"expertise"`
    Prompt      string            `json:"prompt"`
    Severity    string            `json:"severity"`
}
```

**Checkpoint:**
- [ ] All types defined
- [ ] JSON tags present
- [ ] Documentation comments added

---

### STEP 3: Implement Advisor Registry

**Action:** Create `mcp-server/internal/mcp/advisor_registry.go`

```go
package mcp

import (
    "context"
    "fmt"
    "sync"
    "time"

    "github.com/thearchitectit/guardrail-mcp/internal/models"
)

// AdvisorRegistry manages advisor roles and session state
type AdvisorRegistry struct {
    mu          sync.RWMutex
    roles       map[string]*models.AdvisorRole
    sessions    map[string]*models.AdvisorSession
}

// NewAdvisorRegistry creates a new registry with built-in advisors
func NewAdvisorRegistry() *AdvisorRegistry {
    r := &AdvisorRegistry{
        roles:    make(map[string]*models.AdvisorRole),
        sessions: make(map[string]*models.AdvisorSession),
    }
    r.registerBuiltInAdvisors()
    return r
}

// registerBuiltInAdvisors adds system-defined advisors
func (r *AdvisorRegistry) registerBuiltInAdvisors() {
    builtIns := []models.AdvisorRole{
        {
            ID:          "security",
            Name:        "Security Advisor",
            Description: "Prevents security vulnerabilities and secret exposure",
            Expertise:   []string{"security", "secrets", "injection", "vulnerabilities"},
            Tools: []string{
                "guardrail_validate_bash",
                "guardrail_scan_commit_payload",
                "guardrail_validate_file_edit",
            },
            Resources: []string{
                "guardrail://policy/security",
                "guardrail://docs/agent-guardrails",
            },
            Severity: "blocking",
            BuiltIn:  true,
            Prompt: `You are a security advisor. Your role is to:
- Detect secrets, API keys, passwords in code
- Identify SQL injection and command injection risks
- Flag unauthorized file access attempts
- Enforce security best practices
HALT on any security violation.`,
        },
        {
            ID:          "quality",
            Name:        "Code Quality Advisor",
            Description: "Ensures code quality and maintainability",
            Expertise:   []string{"quality", "refactoring", "patterns", "standards"},
            Tools: []string{
                "guardrail_validate_scope",
                "guardrail_detect_feature_creep",
                "guardrail_verify_fixes_intact",
            },
            Resources: []string{
                "guardrail://docs/standards",
                "guardrail://docs/workflows",
            },
            Severity: "advisory",
            BuiltIn:  true,
            Prompt: `You are a code quality advisor. Your role is to:
- Identify code smells and anti-patterns
- Suggest refactoring opportunities
- Ensure consistent coding standards
- Verify fixes remain intact after changes
ADVISE on quality improvements but do not block.`,
        },
        {
            ID:          "performance",
            Name:        "Performance Advisor",
            Description: "Identifies performance issues and optimizations",
            Expertise:   []string{"performance", "optimization", "memory", "cpu"},
            Tools: []string{
                "guardrail_validate_file_edit",
                "guardrail_check_test_prod_separation",
            },
            Resources: []string{
                "guardrail://docs/standards",
            },
            Severity: "advisory",
            BuiltIn:  true,
            Prompt: `You are a performance advisor. Your role is to:
- Identify inefficient algorithms and data structures
- Detect memory leaks and resource exhaustion risks
- Suggest performance optimizations
- Flag N+1 queries and inefficient operations
ADVISE on performance improvements.`,
        },
        {
            ID:          "compliance",
            Name:        "Compliance Advisor",
            Description: "Ensures compliance with organizational policies",
            Expertise:   []string{"compliance", "policy", "audit", "governance"},
            Tools: []string{
                "guardrail_validate_commit",
                "guardrail_validate_push",
                "guardrail_check_test_prod_separation",
            },
            Resources: []string{
                "guardrail://policy/git-safety",
                "guardrail://policy/test-prod-separation",
            },
            Severity: "blocking",
            BuiltIn:  true,
            Prompt: `You are a compliance advisor. Your role is to:
- Enforce commit message standards
- Validate git workflow compliance
- Ensure test/production separation
- Verify audit trail completeness
BLOCK on compliance violations.`,
        },
    }

    for i := range builtIns {
        r.roles[builtIns[i].ID] = &builtIns[i]
    }
}

// ListAdvisors returns all available advisors
func (r *AdvisorRegistry) ListAdvisors() []models.AdvisorRole {
    r.mu.RLock()
    defer r.mu.RUnlock()

    advisors := make([]models.AdvisorRole, 0, len(r.roles))
    for _, role := range r.roles {
        advisors = append(advisors, *role)
    }
    return advisors
}

// GetAdvisor returns a specific advisor by ID
func (r *AdvisorRegistry) GetAdvisor(id string) (*models.AdvisorRole, bool) {
    r.mu.RLock()
    defer r.mu.RUnlock()

    advisor, ok := r.roles[id]
    return advisor, ok
}

// ActivateAdvisor activates an advisor for a session
func (r *AdvisorRegistry) ActivateAdvisor(sessionID, advisorID string) error {
    r.mu.Lock()
    defer r.mu.Unlock()

    // Verify advisor exists
    if _, ok := r.roles[advisorID]; !ok {
        return fmt.Errorf("advisor not found: %s", advisorID)
    }

    // Get or create session
    session, ok := r.sessions[sessionID]
    if !ok {
        session = &models.AdvisorSession{
            SessionID:      sessionID,
            ActiveAdvisors: []string{},
            CreatedAt:      time.Now().UTC().Format(time.RFC3339),
        }
        r.sessions[sessionID] = session
    }

    // Check if already active
    for _, id := range session.ActiveAdvisors {
        if id == advisorID {
            return nil // Already active
        }
    }

    // Activate
    session.ActiveAdvisors = append(session.ActiveAdvisors, advisorID)
    session.UpdatedAt = time.Now().UTC().Format(time.RFC3339)

    return nil
}

// DeactivateAdvisor deactivates an advisor for a session
func (r *AdvisorRegistry) DeactivateAdvisor(sessionID, advisorID string) error {
    r.mu.Lock()
    defer r.mu.Unlock()

    session, ok := r.sessions[sessionID]
    if !ok {
        return fmt.Errorf("session not found: %s", sessionID)
    }

    // Remove from active list
    newActive := make([]string, 0, len(session.ActiveAdvisors))
    for _, id := range session.ActiveAdvisors {
        if id != advisorID {
            newActive = append(newActive, id)
        }
    }
    session.ActiveAdvisors = newActive
    session.UpdatedAt = time.Now().UTC().Format(time.RFC3339)

    return nil
}

// GetActiveAdvisors returns active advisors for a session
func (r *AdvisorRegistry) GetActiveAdvisors(sessionID string) []models.AdvisorRole {
    r.mu.RLock()
    defer r.mu.RUnlock()

    session, ok := r.sessions[sessionID]
    if !ok {
        return []models.AdvisorRole{}
    }

    advisors := make([]models.AdvisorRole, 0, len(session.ActiveAdvisors))
    for _, id := range session.ActiveAdvisors {
        if role, ok := r.roles[id]; ok {
            advisors = append(advisors, *role)
        }
    }
    return advisors
}

// RegisterCustomAdvisor registers a user-defined advisor
func (r *AdvisorRegistry) RegisterCustomAdvisor(reg models.CustomAdvisorRegistration) error {
    r.mu.Lock()
    defer r.mu.Unlock()

    // Check if ID already exists
    if _, ok := r.roles[reg.ID]; ok {
        return fmt.Errorf("advisor with ID '%s' already exists", reg.ID)
    }

    advisor := models.AdvisorRole{
        ID:          reg.ID,
        Name:        reg.Name,
        Description: reg.Description,
        Expertise:   reg.Expertise,
        Prompt:      reg.Prompt,
        Severity:    reg.Severity,
        BuiltIn:     false,
        Tools:       []string{}, // Custom advisors start with no tools
        Resources:   []string{},
    }

    r.roles[advisor.ID] = &advisor
    return nil
}

// GetAdvice generates advice from active advisors
func (r *AdvisorRegistry) GetAdvice(sessionID, context string, data map[string]interface{}) []models.AdvisorAdvice {
    activeAdvisors := r.GetActiveAdvisors(sessionID)
    advice := make([]models.AdvisorAdvice, 0)

    for _, advisor := range activeAdvisors {
        // In a real implementation, this would call an LLM or use rules
        // For now, return placeholder based on advisor type
        advice = append(advice, models.AdvisorAdvice{
            AdvisorID:   advisor.ID,
            AdvisorName: advisor.Name,
            Severity:    "info",
            Message:     fmt.Sprintf("%s is active and monitoring.", advisor.Name),
            Context:     context,
        })
    }

    return advice
}
```

---

### STEP 4: Create Advisor Tools

**Action:** Create `mcp-server/internal/mcp/advisor_tools.go`

```go
package mcp

import (
    "context"
    "fmt"

    "github.com/mark3labs/mcp-go/mcp"
    "github.com/thearchitectit/guardrail-mcp/internal/models"
)

// handleListAdvisors returns all available advisor roles
func (s *MCPServer) handleListAdvisors(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
    advisors := s.advisorRegistry.ListAdvisors()

    result := models.AdvisorListResult{
        Advisors: advisors,
        Count:    len(advisors),
    }

    return buildToolResult(result, false)
}

// handleActivateAdvisor activates an advisor for the current session
func (s *MCPServer) handleActivateAdvisor(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
    advisorID, _ := args["advisor_id"].(string)
    sessionToken, _ := args["session_token"].(string)

    if advisorID == "" {
        result := models.AdvisorActivationResult{
            Success: false,
            Message: "advisor_id is required",
        }
        return buildToolResult(result, true)
    }

    if sessionToken == "" {
        result := models.AdvisorActivationResult{
            Success: false,
            Message: "session_token is required",
        }
        return buildToolResult(result, true)
    }

    err := s.advisorRegistry.ActivateAdvisor(sessionToken, advisorID)
    if err != nil {
        result := models.AdvisorActivationResult{
            Success:   false,
            AdvisorID: advisorID,
            Message:   err.Error(),
        }
        return buildToolResult(result, true)
    }

    activeAdvisors := s.advisorRegistry.GetActiveAdvisors(sessionToken)
    activeIDs := make([]string, len(activeAdvisors))
    for i, a := range activeAdvisors {
        activeIDs[i] = a.ID
    }

    result := models.AdvisorActivationResult{
        Success:   true,
        AdvisorID: advisorID,
        ActiveNow: activeIDs,
        Message:   fmt.Sprintf("Advisor '%s' activated successfully", advisorID),
    }

    return buildToolResult(result, false)
}

// handleDeactivateAdvisor deactivates an advisor for the current session
func (s *MCPServer) handleDeactivateAdvisor(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
    advisorID, _ := args["advisor_id"].(string)
    sessionToken, _ := args["session_token"].(string)

    if advisorID == "" {
        result := models.AdvisorActivationResult{
            Success: false,
            Message: "advisor_id is required",
        }
        return buildToolResult(result, true)
    }

    if sessionToken == "" {
        result := models.AdvisorActivationResult{
            Success: false,
            Message: "session_token is required",
        }
        return buildToolResult(result, true)
    }

    err := s.advisorRegistry.DeactivateAdvisor(sessionToken, advisorID)
    if err != nil {
        result := models.AdvisorActivationResult{
            Success:   false,
            AdvisorID: advisorID,
            Message:   err.Error(),
        }
        return buildToolResult(result, true)
    }

    activeAdvisors := s.advisorRegistry.GetActiveAdvisors(sessionToken)
    activeIDs := make([]string, len(activeAdvisors))
    for i, a := range activeAdvisors {
        activeIDs[i] = a.ID
    }

    result := models.AdvisorActivationResult{
        Success:   true,
        AdvisorID: advisorID,
        ActiveNow: activeIDs,
        Message:   fmt.Sprintf("Advisor '%s' deactivated successfully", advisorID),
    }

    return buildToolResult(result, false)
}

// handleGetAdvisorAdvice gets advice from active advisors
func (s *MCPServer) handleGetAdvisorAdvice(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
    sessionToken, _ := args["session_token"].(string)
    context, _ := args["context"].(string)

    if sessionToken == "" {
        result := models.AdvisorAdviceResult{
            Valid:   false,
            Summary: "session_token is required",
        }
        return buildToolResult(result, true)
    }

    // Get active advisors and generate advice
    advice := s.advisorRegistry.GetAdvice(sessionToken, context, args)

    result := models.AdvisorAdviceResult{
        Valid:  true,
        Advice: advice,
        Summary: fmt.Sprintf("Received advice from %d active advisors", len(advice)),
    }

    return buildToolResult(result, false)
}

// handleRegisterCustomAdvisor registers a new custom advisor role
func (s *MCPServer) handleRegisterCustomAdvisor(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
    id, _ := args["id"].(string)
    name, _ := args["name"].(string)
    description, _ := args["description"].(string)
    prompt, _ := args["prompt"].(string)
    severity, _ := args["severity"].(string)

    if id == "" || name == "" || prompt == "" {
        return buildToolResult(map[string]string{
            "error": "id, name, and prompt are required",
        }, true)
    }

    // Default severity
    if severity == "" {
        severity = "advisory"
    }

    reg := models.CustomAdvisorRegistration{
        ID:          id,
        Name:        name,
        Description: description,
        Prompt:      prompt,
        Severity:    severity,
    }

    err := s.advisorRegistry.RegisterCustomAdvisor(reg)
    if err != nil {
        return buildToolResult(map[string]string{
            "error": err.Error(),
        }, true)
    }

    return buildToolResult(map[string]interface{}{
        "success":    true,
        "advisor_id": id,
        "message":    fmt.Sprintf("Custom advisor '%s' registered successfully", id),
    }, false)
}
```

---

### STEP 5: Create Advisor Resources

**Action:** Add to `mcp-server/internal/mcp/resources_extended.go`:

```go
// readAvailableAdvisorsResource returns list of all advisors
func (s *MCPServer) readAvailableAdvisorsResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
    advisors := s.advisorRegistry.ListAdvisors()

    content, err := json.MarshalIndent(advisors, "", "  ")
    if err != nil {
        return nil, fmt.Errorf("failed to marshal advisors: %w", err)
    }

    return &mcp.ReadResourceResult{
        Contents: []mcp.ResourceContents{
            mcp.TextResourceContents{
                URI:      uri,
                MIMEType: "application/json",
                Text:     string(content),
            },
        },
    }, nil
}

// readAdvisorDetailResource returns specific advisor details
func (s *MCPServer) readAdvisorDetailResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
    // Extract advisor ID from URI: guardrail://advisors/{id}
    advisorID := strings.TrimPrefix(uri, "guardrail://advisors/")

    advisor, ok := s.advisorRegistry.GetAdvisor(advisorID)
    if !ok {
        return nil, fmt.Errorf("advisor not found: %s", advisorID)
    }

    content, err := json.MarshalIndent(advisor, "", "  ")
    if err != nil {
        return nil, fmt.Errorf("failed to marshal advisor: %w", err)
    }

    return &mcp.ReadResourceResult{
        Contents: []mcp.ResourceContents{
            mcp.TextResourceContents{
                URI:      uri,
                MIMEType: "application/json",
                Text:     string(content),
            },
        },
    }, nil
}
```

---

### STEP 6: Create Documentation

**Action:** Create `docs/advisors/INDEX.md`:

```markdown
# Custom Advisor Roles

## Overview

Advisor roles provide specialized guidance for AI agents working on the codebase.
Each advisor focuses on a specific domain (security, quality, performance, compliance).

## Built-in Advisors

| Advisor | Focus | Severity | Description |
|---------|-------|----------|-------------|
| `security` | Security vulnerabilities | blocking | Prevents secrets, injection, unauthorized access |
| `quality` | Code quality | advisory | Identifies code smells, suggests refactoring |
| `performance` | Performance optimization | advisory | Detects inefficiencies, suggests optimizations |
| `compliance` | Policy compliance | blocking | Enforces git workflows, standards |

## Using Advisors

### List Available Advisors

```json
{
  "tool": "guardrail_list_advisors"
}
```

### Activate an Advisor

```json
{
  "tool": "guardrail_activate_advisor",
  "args": {
    "advisor_id": "security",
    "session_token": "your-session-token"
  }
}
```

### Get Advice

```json
{
  "tool": "guardrail_get_advisor_advice",
  "args": {
    "session_token": "your-session-token",
    "context": "Validating file edit for auth.go"
  }
}
```

### Register Custom Advisor

```json
{
  "tool": "guardrail_register_custom_advisor",
  "args": {
    "id": "my-advisor",
    "name": "My Custom Advisor",
    "description": "Checks for team-specific patterns",
    "prompt": "You are a custom advisor...",
    "severity": "advisory"
  }
}
```

## Creating Custom Advisors

See [custom-template.md](custom-template.md) for a template.

## Resources

- `guardrail://advisors/available` - List all advisors
- `guardrail://advisors/{id}` - Get specific advisor details
```

**Action:** Create `docs/advisors/security-advisor.md`:

```markdown
# Security Advisor

## Role

The Security Advisor prevents security vulnerabilities in code and operations.

## Checks

- Secrets, API keys, passwords in code
- SQL injection risks
- Command injection risks
- Path traversal vulnerabilities
- Unauthorized file access

## Halt Conditions

- Secret detected in staged changes
- Injection vulnerability introduced
- Unauthorized access attempted

## MCP Tools Used

- `guardrail_validate_bash`
- `guardrail_scan_commit_payload`
- `guardrail_validate_file_edit`

## Severity

**blocking** - Security violations prevent the operation from proceeding.
```

---

### STEP 7: Register and Test

**Action:** Update `mcp-server/internal/mcp/server.go`:

```go
// Add to MCPServer struct
type MCPServer struct {
    // ... existing fields ...
    advisorRegistry *AdvisorRegistry  // NEW
}

// In NewMCPServer, add:
func NewMCPServer(cfg *config.Config, db *database.DB, cache *cache.Client) (*MCPServer, error) {
    // ... existing code ...

    s := &MCPServer{
        // ... existing assignments ...
        advisorRegistry: NewAdvisorRegistry(),  // NEW
    }

    // ... existing registration ...

    // Register advisor tools
    s.mcpServer.RegisterTool("guardrail_list_advisors", s.handleListAdvisors)
    s.mcpServer.RegisterTool("guardrail_activate_advisor", s.handleActivateAdvisor)
    s.mcpServer.RegisterTool("guardrail_deactivate_advisor", s.handleDeactivateAdvisor)
    s.mcpServer.RegisterTool("guardrail_get_advisor_advice", s.handleGetAdvisorAdvice)
    s.mcpServer.RegisterTool("guardrail_register_custom_advisor", s.handleRegisterCustomAdvisor)

    // Register advisor resources
    s.mcpServer.RegisterResource("guardrail://advisors/available", s.readAvailableAdvisorsResource)
    // Note: guardrail://advisors/{id} is handled via pattern matching in handleReadResource
}
```

**Build and Test:**
```bash
cd mcp-server
go build ./cmd/server
go test ./...
go vet ./...
```

---

## DONE: Commit and Report

**Commit Message:**
```
feat(mcp): add custom advisor roles system

- Add 4 built-in advisors: security, quality, performance, compliance
- Add 5 new MCP tools:
  - guardrail_list_advisors
  - guardrail_activate_advisor
  - guardrail_deactivate_advisor
  - guardrail_get_advisor_advice
  - guardrail_register_custom_advisor

- Add 2 new MCP resources:
  - guardrail://advisors/available
  - guardrail://advisors/{id}

- Add advisor models and registry
- Add documentation in docs/advisors/

Enables specialized AI guidance for security, quality,
performance, and compliance domains.
```

**Report:**
- Tools added: 5
- Resources added: 2
- Built-in advisors: 4
- Total tools: 29 (was 24)
- Total resources: 14 (was 12)

---

## ACCEPTANCE CRITERIA

| # | Criterion | Test | Pass Condition |
|---|---|---|---|
| 1 | list_advisors registered | `grep -c "guardrail_list_advisors" server.go` | Count >= 1 |
| 2 | activate_advisor registered | `grep -c "guardrail_activate_advisor" server.go` | Count >= 1 |
| 3 | deactivate_advisor registered | `grep -c "guardrail_deactivate_advisor" server.go` | Count >= 1 |
| 4 | get_advisor_advice registered | `grep -c "guardrail_get_advisor_advice" server.go` | Count >= 1 |
| 5 | register_custom_advisor registered | `grep -c "guardrail_register_custom_advisor" server.go` | Count >= 1 |
| 6 | advisors/available resource registered | `grep -c "advisors/available" server.go` | Count >= 1 |
| 7 | Built-in advisors exist | `grep -c "security" advisor_registry.go` | Count >= 1 |
| 8 | Server builds | `go build ./cmd/server` | Exit code 0 |
| 9 | Tests pass | `go test ./...` | Exit code 0 |
| 10 | Documentation exists | `ls docs/advisors/` | Files present |

---

## ROLLBACK PROCEDURE

```bash
# Remove new files
rm mcp-server/internal/models/advisor.go
rm mcp-server/internal/mcp/advisor_registry.go
rm mcp-server/internal/mcp/advisor_tools.go
rm -rf docs/advisors/

# Restore modified files
git checkout HEAD -- mcp-server/internal/mcp/server.go
git checkout HEAD -- mcp-server/internal/mcp/resources_extended.go

# Verify
git status
```

---

## REFERENCE

### Tool Schemas

**guardrail_list_advisors:**
```json
{
  "name": "guardrail_list_advisors",
  "description": "List all available advisor roles",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

**guardrail_activate_advisor:**
```json
{
  "name": "guardrail_activate_advisor",
  "description": "Activate an advisor role for the session",
  "inputSchema": {
    "type": "object",
    "properties": {
      "advisor_id": {"type": "string"},
      "session_token": {"type": "string"}
    },
    "required": ["advisor_id", "session_token"]
  }
}
```

---

**Created:** 2026-03-01
**Authored by:** TheArchitectit
**Archive Date:** 2026-03-29
**Version:** 1.0
