# Sprint: MCP Server Gap Implementation - Critical Tools & Resources

**Sprint Date:** 2026-02-08 (Saturday)  
**Archive After:** 2026-02-15 (Saturday) [+7 days]  
**Sprint Focus:** Implement critical missing MCP tools and resources identified in gap analysis  
**Priority:** P0 (Critical)  
**Estimated Effort:** 4-6 hours  
**Status:** COMPLETED
**Completed Date:** 2026-02-08
**Actual Effort:** 4 hours

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

### Guardrails Reference

Full guardrails: [docs/AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md)

---

## PROBLEM STATEMENT

The MCP server currently exposes only 6 tools and 2 resources, covering approximately 6% of the available documentation. Gap analysis identified 14+ missing critical tools and 13+ missing resources that would enable AI agents to properly utilize guardrails. Without these tools, agents cannot:

1. Validate file scope before editing
2. Check commit message format compliance
3. Prevent regression from known failure patterns
4. Validate test/production separation
5. Access critical documentation (Four Laws, Halt Conditions, Workflows)

**Root Cause:** Initial MCP implementation focused on basic validation tools but did not expose the full guardrail framework.

**Where:** `/mcp-server/internal/mcp/server.go` and supporting files

---

## SCOPE BOUNDARY

```
IN SCOPE (may modify):
  - File: mcp-server/internal/mcp/server.go
    Lines: Tools registration section, Resources registration section
    Change: Add new tools and resources
  
  - File: mcp-server/internal/mcp/tools_extended.go (create)
    Lines: All
    Change: Implement extended tool handler functions
  
  - File: mcp-server/internal/mcp/resources_extended.go (create)
    Lines: All
    Change: Implement resource handlers for documentation
  
  - File: mcp-server/internal/models/validation.go (create)
    Lines: All
    Change: Add ValidationResult, Checklist models
  
  - File: mcp-server/internal/database/failures.go
    Lines: Add query functions
    Change: Add pattern matching queries

OUT OF SCOPE (DO NOT TOUCH):
  - Web UI implementation (separate sprint)
  - Database schema changes (use existing tables)
  - API endpoint changes (MCP protocol only)
  - Deployment configuration
  - Tests (read-only for verification)
```

---

## EXECUTION DIRECTIONS

### Overview

```
TASK SEQUENCE:

  STEP 1: Read current MCP server implementation
          Read existing tool/resource handlers
          - - - - - - - - - - - - - - - - - - > Understand current structure
       |
       v
  STEP 2: Create new MCP tool handlers
          - guardrail_validate_scope
          - guardrail_validate_commit
          - guardrail_prevent_regression
          - guardrail_check_test_prod_separation
          - guardrail_validate_push
          - - - - - - - - - - - - - - - - - - > Add 5 critical tools
       |
       v
  STEP 3: Create new MCP resource handlers
          - guardrail://docs/agent-guardrails
          - guardrail://docs/workflows
          - guardrail://docs/standards
          - guardrail://principles/four-laws
          - guardrail://halt-conditions
          - guardrail://checklist/pre-work
          - - - - - - - - - - - - - - - - - - > Add 6 critical resources
       |
       v
  STEP 4: Register tools and resources in server
          Update server.go registration
          - - - - - - - - - - - - - - - - - - > Integrate new handlers
       |
       v
  STEP 5: Build and verify
          Run tests, check compilation
          - - - - - - - - - - - - - - - - - - > Validate implementation
       |
       v
  DONE: Commit and report - - - - - - - - - > Summary to user
```

---

## STEP-BY-STEP EXECUTION

### STEP 1: Read Current Implementation

**Action:** Read the current MCP server implementation to understand the structure

```bash
# Read the main server file
Read: mcp-server/internal/mcp/server.go

# List all MCP-related files
Glob: mcp-server/internal/mcp/**/*.go

# Read models to understand data structures
Read: mcp-server/internal/models/*.go
```

**Checkpoint:** 
- Understand current tool registration pattern
- Understand current resource registration pattern
- Identify where to add new handlers

**Decision Point:**
- [ ] Success → Proceed to STEP 2
- [ ] Failure → HALT and report to user

---

### STEP 2: Create Tool Handlers

**Action:** Create new file for extended tool handlers with 5 critical tools

Create: `mcp-server/internal/mcp/tools_extended.go`

Tools to implement:
1. **guardrail_validate_scope** - Check if file path is within authorized scope
2. **guardrail_validate_commit** - Validate commit message format compliance
3. **guardrail_prevent_regression** - Check failure registry for matching patterns
4. **guardrail_check_test_prod_separation** - Verify test/production isolation
5. **guardrail_validate_push** - Validate git push safety conditions

**Implementation Pattern:**
```go
func (s *MCPServer) handleValidateScope(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
    filePath, _ := args["file_path"].(string)
    scope, _ := args["authorized_scope"].(string)
    
    // Validate file is within scope
    isValid := strings.HasPrefix(filePath, scope)
    
    if !isValid {
        return &mcp.CallToolResult{
            Content: []mcp.Content{
                mcp.TextContent{
                    Text: fmt.Sprintf("ERROR: File %s is outside authorized scope %s", filePath, scope),
                },
            },
            IsError: true,
        }, nil
    }
    
    return &mcp.CallToolResult{
        Content: []mcp.Content{
            mcp.TextContent{
                Text: fmt.Sprintf("VALID: File %s is within authorized scope", filePath),
            },
        },
    }, nil
}
```

**Decision Point:**
- [ ] Success → Proceed to STEP 3
- [ ] Failure → ROLLBACK and report

**Rollback Command:**
```bash
rm mcp-server/internal/mcp/tools_extended.go
```

---

### STEP 3: Create Resource Handlers

**Action:** Create new file for extended resource handlers

Create: `mcp-server/internal/mcp/resources_extended.go`

Resources to implement:
1. **guardrail://docs/agent-guardrails** - Full AGENT_GUARDRAILS.md content
2. **guardrail://docs/workflows** - Index of all workflow documentation
3. **guardrail://docs/standards** - Index of all standards documentation
4. **guardrail://principles/four-laws** - Four Laws of Agent Safety
5. **guardrail://halt-conditions** - When to halt operations
6. **guardrail://checklist/pre-work** - Pre-execution checklist

**Implementation Pattern:**
```go
func (s *MCPServer) readAgentGuardrailsResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
    content, err := os.ReadFile("/app/static/docs/AGENT_GUARDRAILS.md")
    if err != nil {
        return nil, fmt.Errorf("failed to read agent guardrails: %w", err)
    }
    
    return &mcp.ReadResourceResult{
        Contents: []mcp.ResourceContents{
            mcp.TextResourceContents{
                URI:      uri,
                MIMEType: "text/markdown",
                Text:     string(content),
            },
        },
    }, nil
}
```

**Decision Point:**
- [ ] Success → Proceed to STEP 4
- [ ] Failure → ROLLBACK and report

---

### STEP 4: Register Tools and Resources

**Action:** Update server.go to register new tools and resources

```go
// In server.go, add to initialization:

// Register extended tools
s.mcpServer.RegisterTool("guardrail_validate_scope", s.handleValidateScope)
s.mcpServer.RegisterTool("guardrail_validate_commit", s.handleValidateCommit)
s.mcpServer.RegisterTool("guardrail_prevent_regression", s.handlePreventRegression)
s.mcpServer.RegisterTool("guardrail_check_test_prod_separation", s.handleCheckTestProdSeparation)
s.mcpServer.RegisterTool("guardrail_validate_push", s.handleValidatePush)

// Register extended resources
s.mcpServer.RegisterResource("guardrail://docs/agent-guardrails", s.readAgentGuardrailsResource)
s.mcpServer.RegisterResource("guardrail://docs/workflows", s.readWorkflowsResource)
s.mcpServer.RegisterResource("guardrail://docs/standards", s.readStandardsResource)
s.mcpServer.RegisterResource("guardrail://principles/four-laws", s.readFourLawsResource)
s.mcpServer.RegisterResource("guardrail://halt-conditions", s.readHaltConditionsResource)
s.mcpServer.RegisterResource("guardrail://checklist/pre-work", s.readPreWorkChecklistResource)
```

**Decision Point:**
- [ ] Success → Proceed to STEP 5
- [ ] Failure → ROLLBACK and report

---

### STEP 5: Build and Verify

**Action:** Build the server and run tests

```bash
# Build the server
cd mcp-server
go build ./cmd/server

# Run tests
go test ./...

# Check formatting
gofmt -d internal/mcp/

# Verify compilation
go vet ./...
```

**Expected Output:**
- Build succeeds with no errors
- All tests pass
- No formatting issues
- No vet warnings

**Decision Point:**
- [ ] Success → Proceed to DONE
- [ ] Failure → Fix issues and re-run

---

### DONE: Commit and Report

**Action:** Provide completion summary

```bash
# Stage all changes
git add mcp-server/internal/mcp/tools_extended.go
git add mcp-server/internal/mcp/resources_extended.go
git add mcp-server/internal/models/validation.go
git add mcp-server/internal/mcp/server.go

# Commit
git commit -m "feat(mcp): add critical tools and resources for gap coverage

- Add 5 new MCP tools:
  - guardrail_validate_scope
  - guardrail_validate_commit
  - guardrail_prevent_regression
  - guardrail_check_test_prod_separation
  - guardrail_validate_push

- Add 6 new MCP resources:
  - guardrail://docs/agent-guardrails
  - guardrail://docs/workflows
  - guardrail://docs/standards
  - guardrail://principles/four-laws
  - guardrail://halt-conditions
  - guardrail://checklist/pre-work

Authored by TheArchitectit"
```

**REPORT FORMAT:**

## Sprint Complete: MCP Gap Implementation

**Status:** SUCCESS
**Files Modified:** 
- mcp-server/internal/mcp/tools_extended.go (NEW)
- mcp-server/internal/mcp/resources_extended.go (NEW)
- mcp-server/internal/models/validation.go (NEW)
- mcp-server/internal/mcp/server.go

**Commit Hash:** [hash]

### Changes Made:
- Implemented 5 critical MCP tools for validation and prevention
- Implemented 6 MCP resources for documentation access
- Registered all new tools and resources in server

### Verification Results:
- Syntax check: PASSED
- Unit tests: PASSED
- Build verification: PASSED
- MCP tool count: 11 (was 6, +5 new)
- MCP resource count: 8 (was 2, +6 new)

### Next Steps:
- Deploy updated server
- Verify tools work with Crush MCP client
- Update documentation to reflect new capabilities

---

## COMPLETION GATE (MANDATORY)

**This section MUST be completed before marking the sprint done.**

### Validation Loop Rules

```
MAX_CYCLES: 3
MAX_TIME: 30 minutes
EXIT_CONDITIONS:
  - All BLOCKING items pass, OR
  - MAX_CYCLES reached (report blockers), OR
  - MAX_TIME exceeded (report status)
```

### Core Validation Checklist

| Check | Command | Pass Condition | Blocking? | Status |
|-------|---------|----------------|-----------|--------|
| **Files Saved** | `git status` | No unexpected untracked files | YES | [ ] |
| **Changes Staged** | `git diff --cached --stat` | Target files staged | YES | [ ] |
| **Syntax Valid** | `go build ./cmd/server` | Exit code 0 | YES | [ ] |
| **Tests Pass** | `go test ./...` | Exit code 0 | YES | [ ] |
| **Production Code** | Manual check | Production code exists | YES | [ ] |
| **Committed** | `git log -1 --oneline` | Shows sprint commit | YES | [ ] |
| **No Secrets** | `git diff --cached` | No API keys, tokens, passwords | YES | [ ] |

**Cycle:** ___ / 3  
**Time Started:** ___:___  
**Current Status:** VALIDATING | PASSED | BLOCKED | TIMEOUT

---

## ACCEPTANCE CRITERIA

| # | Criterion | Test | Pass Condition |
|---|---|---|---|
| 1 | All 5 new tools registered | `grep -c "RegisterTool" server.go` | Count = 11 |
| 2 | All 6 new resources registered | `grep -c "RegisterResource" server.go` | Count = 8 |
| 3 | Tools have handlers | `grep "func.*handle.*Validate" tools_extended.go` | 5+ handlers found |
| 4 | Resources have handlers | `grep "func.*read.*Resource" resources_extended.go` | 6+ handlers found |
| 5 | Server builds | `go build ./cmd/server` | Exit code 0 |
| 6 | Tests pass | `go test ./...` | Exit code 0 |

---

## ROLLBACK PROCEDURE

```bash
# Immediate rollback - discard all changes
git checkout HEAD -- mcp-server/internal/mcp/server.go
rm -f mcp-server/internal/mcp/tools_extended.go
rm -f mcp-server/internal/mcp/resources_extended.go
rm -f mcp-server/internal/models/validation.go

# Verify rollback
git status

# Report to user
echo "Rollback complete. All sprint changes removed."
```

---

## REFERENCE

### MCP Tool Schema Reference

```go
// Tool: guardrail_validate_scope
{
    Name:        "guardrail_validate_scope",
    Description: "Check if a file path is within authorized scope",
    InputSchema: map[string]interface{}{
        "type": "object",
        "properties": map[string]interface{}{
            "file_path": map[string]interface{}{
                "type":        "string",
                "description": "The file path to validate",
            },
            "authorized_scope": map[string]interface{}{
                "type":        "string",
                "description": "The authorized scope prefix (e.g., /app/src)",
            },
        },
        "required": []string{"file_path", "authorized_scope"},
    },
}
```

### MCP Resource URI Reference

```
guardrail://docs/agent-guardrails    -> AGENT_GUARDRAILS.md
guardrail://docs/workflows           -> workflows/INDEX.md + listing
guardrail://docs/standards           -> standards/INDEX.md + listing
guardrail://principles/four-laws     -> skills/shared-prompts/four-laws.md
guardrail://halt-conditions          -> skills/shared-prompts/halt-conditions.md
guardrail://checklist/pre-work       -> .guardrails/pre-work-check.md
```

---

## QUICK REFERENCE CARD

```
+------------------------------------------------------------------+
|                    SPRINT QUICK REFERENCE                        |
+------------------------------------------------------------------+
| TARGET FILE:  mcp-server/internal/mcp/server.go                  |
|               mcp-server/internal/mcp/tools_extended.go (NEW)    |
|               mcp-server/internal/mcp/resources_extended.go (NEW)|
| CHANGE TYPE:  Add 5 tools + 6 resources                           |
+------------------------------------------------------------------+
| SAFETY:                                                          |
|   - Read before edit                                             |
|   - Single responsibility per handler                            |
|   - Production code FIRST                                        |
|   - Test before commit                                           |
+------------------------------------------------------------------+
| HALT IF:                                                         |
|   - Tools don't match schema                                     |
|   - Resources can't read files                                   |
|   - Tests fail                                                   |
|   - Uncertain about implementation                               |
+------------------------------------------------------------------+
| ROLLBACK: git checkout HEAD -- mcp-server/internal/mcp/server.go |
|           rm mcp-server/internal/mcp/tools_extended.go           |
|           rm mcp-server/internal/mcp/resources_extended.go       |
+------------------------------------------------------------------+
```

---

**Created:** 2026-02-08  
**Authored by:** TheArchitectit  
**Archive Date:** 2026-02-15  
**Version:** 1.0
