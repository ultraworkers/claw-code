# Sprint: Pre-Commit Safety Suite

**Sprint Date:** 2026-03-01
**Archive After:** 2026-03-15 [+14 days]
**Sprint Focus:** Implement critical pre-commit validation tools and policy resources
**Priority:** P0 (Critical)
**Estimated Effort:** 6-8 hours
**Status:** PLANNED
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

The MCP server currently lacks critical pre-commit safety validations that would prevent:

1. **Commits without passing tests** - Agents cannot verify tests pass before committing
2. **Secrets in commits** - No automated scanning for API keys, tokens, passwords
3. **Binaries/generated files** - No detection of files that shouldn't be committed
4. **Merge conflicts** - No validation that files don't contain conflict markers
5. **Policy references** - No machine-readable policy resources for git safety

**Root Cause:** Sprint 001 focused on validation during editing, but pre-commit validation is incomplete.

**Where:** `/mcp-server/internal/mcp/tools_extended.go`, `/mcp-server/internal/mcp/resources_extended.go`, `/mcp-server/internal/mcp/server.go`

---

## SCOPE BOUNDARY

```
IN SCOPE (may modify):
  - File: mcp-server/internal/mcp/tools_extended.go
    Lines: Add new handler functions at end
    Change: Add 3 new tool handlers

  - File: mcp-server/internal/mcp/resources_extended.go
    Lines: Add new resource handlers at end
    Change: Add 2 new policy resources

  - File: mcp-server/internal/mcp/server.go
    Lines: Add tool/resource registration
    Change: Register new handlers

  - File: mcp-server/internal/models/*.go
    Lines: Add new result types
    Change: Create validation result models

OUT OF SCOPE (DO NOT TOUCH):
  - Existing tool handlers (read-only)
  - Database schema changes
  - Web UI changes
  - Deployment configuration
```

---

## EXECUTION DIRECTIONS

### Overview

```
TASK SEQUENCE:

  STEP 1: Read current implementation
          Read tools_extended.go, resources_extended.go, server.go
          - - - - - - - - - - - - - - - - - - > Understand patterns
       |
       v
  STEP 2: Create tool handlers
          - guardrail_verify_tests_before_commit
          - guardrail_scan_commit_payload
          - guardrail_detect_merge_conflicts
          - - - - - - - - - - - - - - - - - - > Add 3 critical tools
       |
       v
  STEP 3: Create policy resources
          - guardrail://policy/git-safety
          - guardrail://policy/test-prod-separation
          - - - - - - - - - - - - - - - - - - > Add 2 policy resources
       |
       v
  STEP 4: Add models
          - TestValidationResult
          - PayloadScanResult
          - MergeConflictResult
          - - - - - - - - - - - - - - - - - - > Type safety
       |
       v
  STEP 5: Register and test
          Update server.go, run tests
          - - - - - - - - - - - - - - - - - - > Validate
       |
       v
  DONE: Commit and report - - - - - - - - - > Summary
```

---

## STEP-BY-STEP EXECUTION

### STEP 1: Read Current Implementation

**Action:** Read existing files to understand patterns

```bash
Read: mcp-server/internal/mcp/tools_extended.go
Read: mcp-server/internal/mcp/resources_extended.go
Read: mcp-server/internal/mcp/server.go (tool/resource registration section)
Read: mcp-server/internal/models/validation.go
```

**Checkpoint:**
- [ ] Understand tool handler pattern
- [ ] Understand resource handler pattern
- [ ] Identify registration location

---

### STEP 2: Create Tool Handlers

**Action:** Add 3 new tool handlers to tools_extended.go

#### Tool 1: guardrail_verify_tests_before_commit

```go
func (s *MCPServer) handleVerifyTestsBeforeCommit(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
    testResults, _ := args["test_results"].(string)
    stagedFiles, _ := args["staged_files"].([]interface{})

    // Validate test results indicate pass
    // Check for failures, errors, or incomplete tests
    // Return validation result
}
```

**Input Schema:**
```json
{
  "test_results": "string (test output or status)",
  "staged_files": ["array of file paths"],
  "require_coverage": "boolean (optional)"
}
```

**Output:** TestValidationResult with valid/pass/fail status

#### Tool 2: guardrail_scan_commit_payload

```go
func (s *MCPServer) handleScanCommitPayload(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
    stagedFiles, _ := args["staged_files"].([]interface{})

    // Scan for:
    // - Secrets (API keys, tokens, passwords)
    // - Binary files
    // - Generated files (should be in .gitignore)
    // - Large files (>1MB)
}
```

**Input Schema:**
```json
{
  "staged_files": ["array of file paths"],
  "scan_secrets": "boolean (default: true)",
  "scan_binaries": "boolean (default: true)",
  "scan_large_files": "boolean (default: true)"
}
```

**Output:** PayloadScanResult with findings array

#### Tool 3: guardrail_detect_merge_conflicts

```go
func (s *MCPServer) handleDetectMergeConflicts(ctx context.Context, args map[string]interface{}) (*mcp.CallToolResult, error) {
    filePaths, _ := args["file_paths"].([]interface{})

    // Check for conflict markers: <<<<<<<, =======, >>>>>>>
    // Return list of files with conflicts
}
```

**Input Schema:**
```json
{
  "file_paths": ["array of file paths to check"],
  "check_content": "boolean (default: true)"
}
```

**Output:** MergeConflictResult with conflicts array

---

### STEP 3: Create Policy Resources

**Action:** Add 2 new resource handlers to resources_extended.go

#### Resource 1: guardrail://policy/git-safety

```go
func (s *MCPServer) readGitSafetyPolicyResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
    content := `# Git Safety Policy

## Forbidden Operations
- NO force push to main/master
- NO force push to protected branches
- NO amend/rebase of published commits
- NO push without explicit permission

## Required Checks
- All tests must pass
- No secrets in commit
- No merge conflicts
- No binaries without justification
`
    return &mcp.ReadResourceResult{...}
}
```

#### Resource 2: guardrail://policy/test-prod-separation

```go
func (s *MCPServer) readTestProdSeparationPolicyResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
    content := `# Test/Production Separation Policy

## Principles
- Production code FIRST, tests second
- Test infrastructure != Production infrastructure
- No test data in production
- No production credentials in tests

## Validation Rules
- Check for test files modifying prod paths
- Check for prod credentials in test files
- Check for test fixtures leaking to prod
`
    return &mcp.ReadResourceResult{...}
}
```

---

### STEP 4: Add Models

**Action:** Create validation result models

```go
// TestValidationResult from test verification
type TestValidationResult struct {
    Valid         bool     `json:"valid"`
    Passed        bool     `json:"passed"`
    Message       string   `json:"message"`
    FailedTests   []string `json:"failed_tests,omitempty"`
    CoverageMet   bool     `json:"coverage_met,omitempty"`
}

// PayloadScanResult from commit payload scanning
type PayloadScanResult struct {
    Valid       bool              `json:"valid"`
    Clean       bool              `json:"clean"`
    Message     string            `json:"message"`
    Findings    []PayloadFinding  `json:"findings,omitempty"`
}

type PayloadFinding struct {
    File        string `json:"file"`
    Type        string `json:"type"` // secret, binary, generated, large
    Severity    string `json:"severity"`
    Description string `json:"description"`
}

// MergeConflictResult from conflict detection
type MergeConflictResult struct {
    Valid       bool              `json:"valid"`
    Clean       bool              `json:"clean"`
    Message     string            `json:"message"`
    Conflicts   []ConflictFinding `json:"conflicts,omitempty"`
}

type ConflictFinding struct {
    File        string `json:"file"`
    LineNumber  int    `json:"line_number"`
    Context     string `json:"context"`
}
```

---

### STEP 5: Register and Test

**Action:** Register tools and resources in server.go

```go
// Add to server initialization

// Register pre-commit safety tools
s.mcpServer.RegisterTool("guardrail_verify_tests_before_commit", s.handleVerifyTestsBeforeCommit)
s.mcpServer.RegisterTool("guardrail_scan_commit_payload", s.handleScanCommitPayload)
s.mcpServer.RegisterTool("guardrail_detect_merge_conflicts", s.handleDetectMergeConflicts)

// Register policy resources
s.mcpServer.RegisterResource("guardrail://policy/git-safety", s.readGitSafetyPolicyResource)
s.mcpServer.RegisterResource("guardrail://policy/test-prod-separation", s.readTestProdSeparationPolicyResource)
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
feat(mcp): add pre-commit safety suite

- Add 3 new MCP tools:
  - guardrail_verify_tests_before_commit
  - guardrail_scan_commit_payload
  - guardrail_detect_merge_conflicts

- Add 2 new policy resources:
  - guardrail://policy/git-safety
  - guardrail://policy/test-prod-separation

- Add validation result models for type safety

Closes critical gaps in pre-commit validation
```

**Report:**
- Tools added: 3
- Resources added: 2
- Total tools: 27 (was 24)
- Total resources: 14 (was 12)

---

## ACCEPTANCE CRITERIA

| # | Criterion | Test | Pass Condition |
|---|---|---|---|
| 1 | verify_tests_before_commit registered | `grep -c "guardrail_verify_tests_before_commit" server.go` | Count >= 1 |
| 2 | scan_commit_payload registered | `grep -c "guardrail_scan_commit_payload" server.go` | Count >= 1 |
| 3 | detect_merge_conflicts registered | `grep -c "guardrail_detect_merge_conflicts" server.go` | Count >= 1 |
| 4 | git-safety resource registered | `grep -c "policy/git-safety" server.go` | Count >= 1 |
| 5 | test-prod-separation resource registered | `grep -c "policy/test-prod-separation" server.go` | Count >= 1 |
| 6 | Server builds | `go build ./cmd/server` | Exit code 0 |
| 7 | Tests pass | `go test ./...` | Exit code 0 |

---

## ROLLBACK PROCEDURE

```bash
# Discard changes
git checkout HEAD -- mcp-server/internal/mcp/server.go
git checkout HEAD -- mcp-server/internal/mcp/tools_extended.go
git checkout HEAD -- mcp-server/internal/mcp/resources_extended.go
git checkout HEAD -- mcp-server/internal/models/

# Verify
git status
```

---

**Created:** 2026-03-01
**Authored by:** TheArchitectit
**Archive Date:** 2026-03-15
**Version:** 1.0
