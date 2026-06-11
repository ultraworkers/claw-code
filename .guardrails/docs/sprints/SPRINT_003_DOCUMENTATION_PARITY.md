# Sprint: Documentation Parity - Content Organization & Consolidation

**Sprint Date:** 2026-02-08 (Saturday)
**Archive After:** 2026-02-15 (Saturday) [+7 days]
**Sprint Focus:** Organize 73 MD files, consolidate duplicates, create MCP resources, and enable full-text search
**Priority:** P2 (Medium)
**Estimated Effort:** 8-10 hours
**Status:** PENDING

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
| **TEST BEFORE COMMIT** | All tests must pass before committing | [ ]

### Guardrails Reference

Full guardrails: [docs/AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md)

---

## PROBLEM STATEMENT

The documentation system has grown organically with 73+ markdown files across multiple directories, leading to:

1. **Duplicate content** - "Four Laws" appear in 8+ locations (AGENT_GUARDRAILS.md, four-laws.md, skills/, etc.)
2. **Scattered actionable rules** - 25+ rules embedded in prose need database migration
3. **Missing MCP resources** - Critical docs not accessible via MCP protocol
4. **No full-text search** - Documents exist but can't be searched
5. **Oversized documents** - 5 files exceed 500-line limit per MODULAR_DOCUMENTATION.md

**Root Cause:** Documentation grew incrementally without consolidation or indexing strategy.

**Where:** Primarily `docs/`, `skills/`, `.guardrails/` directories

---

## SCOPE BOUNDARY

```
IN SCOPE (may modify):
  - Files: docs/**/*.md (organize, consolidate)
    Change: Merge duplicates, update cross-references

  - File: INDEX_MAP.md
    Lines: Update with new organization
    Change: Add new entries, fix paths

  - File: HEADER_MAP.md
    Lines: Update section references
    Change: Fix line numbers after consolidation

  - File: mcp-server/internal/mcp/resources_extended.go (if exists)
    Lines: Add document resource handlers
    Change: Implement doc resources

  - Directory: .guardrails/prevention-rules/
    Change: Add rules extracted from docs

  - Database: documents table
    Change: Index all docs for full-text search

OUT OF SCOPE (DO NOT TOUCH):
  - Core guardrail content (keep meaning, fix duplication only)
  - API code implementations
  - Database schema (use existing tables)
  - Test files
  - Non-documentation files
```

---

## EXECUTION DIRECTIONS

### Overview

```
TASK SEQUENCE:

  STEP 1: Document inventory
          - Count and catalog all 73 MD files
          - Identify duplicates
          - Map cross-references
          - - - - - - - - - - - - - - - - - - > Understand current state
       |
       v
  STEP 2: Consolidate Four Laws
          - Find all 8+ occurrences
          - Create canonical version
          - Update references
          - - - - - - - - - - - - - - - - - - > Eliminate duplication
       |
       v
  STEP 3: Extract actionable rules
          - Identify 25+ embedded rules
          - Convert to prevention-rules/ JSON
          - Update docs to reference rules
          - - - - - - - - - - - - - - - - - - > Make rules actionable
       |
       v
  STEP 4: Create MCP resources
          - Add document resources to MCP server
          - Implement full-text search endpoint
          - - - - - - - - - - - - - - - - - - > Enable agent access
       |
       v
  STEP 5: Index for search
          - Ingest all docs to database
          - Generate embeddings (if configured)
          - Verify search works
          - - - - - - - - - - - - - - - - - - > Enable discovery
       |
       v
  STEP 6: Split oversized documents
          - Identify 5 files >500 lines
          - Split per MODULAR_DOCUMENTATION.md
          - Update INDEX_MAP and HEADER_MAP
          - - - - - - - - - - - - - - - - - - > Meet standards
       |
       v
  STEP 7: Update navigation maps
          - Refresh INDEX_MAP.md
          - Refresh HEADER_MAP.md
          - Fix all broken links
          - - - - - - - - - - - - - - - - - - > Keep nav current
       |
       v
  STEP 8: Verify and commit
          - Test all links
          - Verify MCP resources
          - Run search tests
          - - - - - - - - - - - - - - - - - - > Validate changes
       |
       v
  DONE: Commit and report - - - - - - - - > Summary to user
```

---

## STEP-BY-STEP EXECUTION

### STEP 1: Document Inventory

**Action:** Catalog all markdown files and identify duplicates

```bash
# Find all markdown files
find . -name "*.md" -not -path "./.git/*" -not -path "./mcp-server/*" | wc -l
find . -name "*.md" -not -path "./.git/*" -not -path "./mcp-server/*" > /tmp/doc_inventory.txt

# Check line counts (identify oversized)
find . -name "*.md" -not -path "./.git/*" -exec wc -l {} \; | sort -n | tail -10

# Find "Four Laws" occurrences
grep -r "Four Laws" --include="*.md" .
grep -r "Law 1" --include="*.md" . | head -20

# Find "Halt Conditions" occurrences
grep -r "Halt Conditions" --include="*.md" .
```

**Expected Files to Review:**
- docs/AGENT_GUARDRAILS.md (likely contains Four Laws)
- skills/shared-prompts/four-laws.md (canonical)
- skills/shared-prompts/halt-conditions.md (canonical)
- .guardrails/pre-work-check.md
- docs/workflows/*.md (may reference)

**Checkpoint:**
- [ ] Complete file list created
- [ ] Duplicates identified
- [ ] Oversized files (>500 lines) noted

**Decision Point:**
- [ ] Success → Proceed to STEP 2
- [ ] Failure → HALT and report

---

### STEP 2: Consolidate Four Laws

**Action:** Create single canonical source and update references

**Current State Analysis:**
- Canonical source: `skills/shared-prompts/four-laws.md` (120 lines)
- Duplicates likely in:
  - docs/AGENT_GUARDRAILS.md (lines 39-61)
  - Possibly docs/workflows/ files
  - Possibly examples/ documentation

**Changes to Make:**

1. **Verify canonical version** is complete at `skills/shared-prompts/four-laws.md`

2. **Update docs/AGENT_GUARDRAILS.md:**
   - Replace Four Laws content with reference:
   ```markdown
   ### The Four Laws of Agent Safety

   See [skills/shared-prompts/four-laws.md](../skills/shared-prompts/four-laws.md) for the complete Four Laws.

   Quick reference:
   1. **Read Before Editing** - Never modify code without reading first
   2. **Stay in Scope** - Only touch authorized files
   3. **Verify Before Committing** - Test all changes
   4. **Halt When Uncertain** - Ask instead of guessing
   ```

3. **Update other occurrences** to reference the canonical file

**Decision Point:**
- [ ] Success → Proceed to STEP 3
- [ ] Failure → ROLLBACK and report

---

### STEP 3: Extract Actionable Rules

**Action:** Convert embedded rules to prevention-rules format

**Read:** `.guardrails/prevention-rules/pattern-rules.json`

**Current format:**
```json
{
  "rules": [
    {
      "id": "PREVENT-001",
      "pattern": "git push --force",
      "message": "Force push is not allowed",
      "severity": "error",
      "category": "git"
    }
  ]
}
```

**Rules to Extract from AGENT_GUARDRAILS.md:**

| Rule | Source | Pattern | Severity |
|------|--------|---------|----------|
| NO-FORCE-PUSH | Git Safety | `git push --force` | error |
| NO-AMEND | Git Safety | `git commit --amend` | warning |
| NO-CONFIG-CHANGE | Git Safety | `git config` | error |
| NO-RESET-HARD | Git Safety | `git reset --hard` | error |
| NO-RM-RF | Forbidden | `rm -rf /` | critical |
| NO-SECRETS | Code Safety | regex for keys | critical |

**Create new rules file:** `.guardrails/prevention-rules/extracted-rules.json`

```json
{
  "version": "1.0",
  "source": "AGENT_GUARDRAILS.md",
  "rules": [
    {
      "id": "PREVENT-GIT-001",
      "name": "No Force Push",
      "pattern": "git\\s+push\\s+--force(?!-with-lease)",
      "message": "Force push without lease is prohibited by guardrails",
      "severity": "error",
      "category": "git",
      "suggestion": "Use 'git push --force-with-lease' instead"
    },
    {
      "id": "PREVENT-GIT-002",
      "name": "No Hard Reset",
      "pattern": "git\\s+reset\\s+--hard",
      "message": "Hard reset can destroy uncommitted work",
      "severity": "error",
      "category": "git",
      "suggestion": "Use 'git stash' or commit changes first"
    }
  ]
}
```

**Decision Point:**
- [ ] Success → Proceed to STEP 4
- [ ] Failure → ROLLBACK and report

---

### STEP 4: Create MCP Resources for Documentation

**Action:** Add document resources to MCP server (if tools_extended.go exists)

**Add to `mcp-server/internal/mcp/resources_extended.go`:**

```go
// Document resources for full guardrail framework access
func (s *MCPServer) registerDocumentResources() {
    // Register document resources
    s.mcpServer.RegisterResource("guardrail://docs/agent-guardrails", s.readAgentGuardrailsResource)
    s.mcpServer.RegisterResource("guardrail://docs/four-laws", s.readFourLawsResource)
    s.mcpServer.RegisterResource("guardrail://docs/halt-conditions", s.readHaltConditionsResource)
    s.mcpServer.RegisterResource("guardrail://docs/pre-work-checklist", s.readPreWorkChecklistResource)
    s.mcpServer.RegisterResource("guardrail://docs/workflows", s.readWorkflowsIndexResource)
    s.mcpServer.RegisterResource("guardrail://docs/standards", s.readStandardsIndexResource)
    s.mcpServer.RegisterResource("guardrail://docs/search", s.readSearchDocsResource)
}

func (s *MCPServer) readAgentGuardrailsResource(ctx context.Context, uri string) (*mcp.ReadResourceResult, error) {
    content, err := os.ReadFile("docs/AGENT_GUARDRAILS.md")
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

// Implement other resource handlers...
```

**Add to `mcp-server/internal/mcp/server.go`:**

```go
// In registerTools(), add resource registrations
s.registerDocumentResources()
```

**Decision Point:**
- [ ] Success → Proceed to STEP 5
- [ ] Failure → ROLLBACK and report

---

### STEP 5: Index Documents for Search

**Action:** Ingest all documentation to database for full-text search

**Create ingestion script:** `scripts/ingest_docs.go`

```go
package main

import (
    "context"
    "fmt"
    "os"
    "path/filepath"
    "strings"

    "github.com/thearchitectit/guardrail-mcp/internal/database"
)

func main() {
    ctx := context.Background()

    // Connect to database
    db, err := database.Connect()
    if err != nil {
        fmt.Fprintf(os.Stderr, "Failed to connect: %v\n", err)
        os.Exit(1)
    }
    defer db.Close()

    docStore := database.NewDocumentStore(db)

    // Walk docs directory
    err = filepath.Walk("docs", func(path string, info os.FileInfo, err error) error {
        if err != nil || info.IsDir() || !strings.HasSuffix(path, ".md") {
            return err
        }

        content, err := os.ReadFile(path)
        if err != nil {
            return err
        }

        // Extract title from first h1
        title := extractTitle(string(content))
        slug := slugify(filepath.Base(path, ".md"))
        category := categorize(path)

        // Create or update document
        doc := &models.Document{
            Slug:     slug,
            Title:    title,
            Content:  string(content),
            Category: category,
            Path:     path,
            Version:  1,
        }

        err = docStore.Create(ctx, doc)
        if err != nil {
            fmt.Printf("Failed to ingest %s: %v\n", path, err)
        } else {
            fmt.Printf("Ingested: %s\n", path)
        }

        return nil
    })

    if err != nil {
        fmt.Fprintf(os.Stderr, "Walk failed: %v\n", err)
        os.Exit(1)
    }
}
```

**Run ingestion:**
```bash
cd mcp-server
go run ../scripts/ingest_docs.go
```

**Decision Point:**
- [ ] Success → Proceed to STEP 6
- [ ] Failure → ROLLBACK and report

---

### STEP 6: Split Oversized Documents

**Action:** Identify and split files >500 lines per MODULAR_DOCUMENTATION.md

**Check file sizes:**
```bash
find docs -name "*.md" -exec wc -l {} \; | awk '$1 > 500 {print $2, $1}'
```

**Likely oversized files:**
- docs/AGENT_GUARDRAILS.md (~320 lines - OK)
- docs/workflows/AGENT_EXECUTION.md (~380 lines - OK)
- docs/workflows/REGRESSION_PREVENTION.md (~330 lines - OK)
- docs/standards/TEST_PRODUCTION_SEPARATION.md (~350 lines - OK)
- docs/standards/ADVERSARIAL_TESTING.md (~280 lines - OK)

*Note: Based on earlier reads, files may already comply. Verify with `wc -l`.*

**If any files exceed 500 lines:**

1. **Read the file** to understand structure
2. **Identify logical split points** (sections)
3. **Create part files:**
   - `original.md` → keep part 1 + index
   - `original-part2.md` → remaining content
4. **Update INDEX_MAP.md** and **HEADER_MAP.md**

**Example split pattern:**
```markdown
# Original Document

> Part 1 of 2. See [Part 2](original-part2.md).

## Section 1
...

## Section 2
...

---

**Next:** [Original Document - Part 2](original-part2.md)
```

**Decision Point:**
- [ ] Success → Proceed to STEP 7
- [ ] Failure → ROLLBACK and report

---

### STEP 7: Update Navigation Maps

**Action:** Refresh INDEX_MAP.md and HEADER_MAP.md

**Read current:** `INDEX_MAP.md`, `HEADER_MAP.md`

**Updates needed:**
1. Add new document entries
2. Fix any broken cross-references
3. Update line numbers in HEADER_MAP.md
4. Add new categories if needed

**Check for broken links:**
```bash
# Find all markdown links
grep -rE "\[.*\]\(.*\.md\)" docs/ --include="*.md" | grep -v "http"

# Check if referenced files exist
# (manual verification)
```

**Update INDEX_MAP.md:**
- Add new prevention-rules section
- Add MCP resources section
- Update document counts

**Update HEADER_MAP.md:**
- Recalculate section line numbers
- Add new file entries
- Remove deleted/moved entries

**Decision Point:**
- [ ] Success → Proceed to STEP 8
- [ ] Failure → ROLLBACK and report

---

### STEP 8: Verify and Commit

**Action:** Final verification and commit

```bash
# Verify all files are valid markdown
find docs -name "*.md" -exec markdownlint {} \; 2>&1 | head -20

# Verify Go code compiles
cd mcp-server && go build ./...

# Run tests
go test ./...

# Check for broken internal links
# (grep for .md references and verify existence)

# Test MCP resources (if server running)
curl http://localhost:8080/mcp/v1/sse &
# Then test resource reads
```

**Verification Checklist:**
- [ ] All markdown files valid
- [ ] No files >500 lines
- [ ] INDEX_MAP.md updated
- [ ] HEADER_MAP.md updated
- [ ] MCP resources registered
- [ ] Prevention rules extracted
- [ ] Documents indexed in DB
- [ ] Server builds successfully

**Decision Point:**
- [ ] Success → Proceed to DONE
- [ ] Failure → Fix issues and re-run

---

### DONE: Commit and Report

**Action:** Provide completion summary

```bash
# Stage all changes
git add docs/
git add skills/
git add .guardrails/prevention-rules/
git add INDEX_MAP.md
git add HEADER_MAP.md
git add mcp-server/internal/mcp/resources_extended.go
git add scripts/ingest_docs.go

# Commit
git commit -m "docs: consolidate documentation and enable MCP access

- Consolidate Four Laws to canonical source (skills/shared-prompts/four-laws.md)
- Extract 25+ actionable rules to prevention-rules/
- Add MCP resources for all critical documentation
- Index 73 documents for full-text search
- Split oversized documents (>500 lines)
- Update INDEX_MAP.md and HEADER_MAP.md

Authored by TheArchitectit"
```

**REPORT FORMAT:**

## Sprint Complete: Documentation Parity

**Status:** SUCCESS
**Files Modified:**
- docs/AGENT_GUARDRAILS.md (consolidated Four Laws reference)
- INDEX_MAP.md (updated)
- HEADER_MAP.md (updated)
- .guardrails/prevention-rules/extracted-rules.json (NEW)
- mcp-server/internal/mcp/resources_extended.go (enhanced)
- scripts/ingest_docs.go (NEW)

**Commit Hash:** [hash]

### Changes Made:
- Consolidated duplicate "Four Laws" content across 8+ locations
- Extracted 25+ embedded rules to actionable JSON format
- Added 7 new MCP resources for documentation access
- Indexed 73 documents for full-text search
- Verified all files meet 500-line limit
- Updated navigation maps

### Verification Results:
- Document count: 73 files indexed
- Duplicate elimination: 8→1 canonical sources
- Rules extracted: 25+ actionable rules
- MCP resources: 7 new resources
- Search: Full-text enabled
- Build: PASSED

### Next Steps:
- Deploy updated server for MCP resource access
- Run ingestion script in production
- Update agent configurations to use new resources

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
| **No >500 Line Files** | `find docs -name "*.md" -exec wc -l {} \;` | Max <500 | YES | [ ] |
| **INDEX_MAP Updated** | Manual check | New entries added | YES | [ ] |
| **Committed** | `git log -1 --oneline` | Shows sprint commit | YES | [ ] |
| **No Secrets** | `git diff --cached` | No API keys, tokens | YES | [ ] |

**Cycle:** ___ / 3
**Time Started:** ___:___
**Current Status:** VALIDATING | PASSED | BLOCKED | TIMEOUT

---

## ACCEPTANCE CRITERIA

| # | Criterion | Test | Pass Condition |
|---|-----------|------|----------------|
| 1 | Four Laws consolidated | `grep -c "Four Laws" docs/` | 1 canonical + references only |
| 2 | Rules extracted | `ls .guardrails/prevention-rules/` | extracted-rules.json exists |
| 3 | MCP resources added | Check `resources_extended.go` | 7+ handlers |
| 4 | Documents indexed | Query DB | 73+ rows in documents table |
| 5 | No oversized files | `find docs -name "*.md" -exec wc -l {} \;` | All <500 lines |
| 6 | INDEX_MAP updated | Manual review | Accurate entries |
| 7 | Server builds | `go build` | Exit code 0 |
| 8 | Search works | API test | Returns results |

---

## ROLLBACK PROCEDURE

```bash
# Immediate rollback - discard all changes
git checkout HEAD -- docs/AGENT_GUARDRAILS.md
git checkout HEAD -- INDEX_MAP.md
git checkout HEAD -- HEADER_MAP.md
git checkout HEAD -- mcp-server/internal/mcp/resources_extended.go
rm -f .guardrails/prevention-rules/extracted-rules.json
rm -f scripts/ingest_docs.go

# Verify rollback
git status

# Report to user
echo "Rollback complete. All doc parity changes removed."
```

---

## REFERENCE

### Document Locations

| Document | Path | Lines (est) |
|----------|------|-------------|
| Four Laws (canonical) | skills/shared-prompts/four-laws.md | 120 |
| Halt Conditions | skills/shared-prompts/halt-conditions.md | 108 |
| Agent Guardrails | docs/AGENT_GUARDRAILS.md | 320 |
| Pre-work Checklist | .guardrails/pre-work-check.md | 180 |
| Test/Production Separation | docs/standards/TEST_PRODUCTION_SEPARATION.md | 350 |

### Prevention Rules Format

```json
{
  "id": "PREVENT-XXX",
  "name": "Human readable name",
  "pattern": "regex pattern",
  "message": "Violation message",
  "severity": "error|warning|info",
  "category": "git|bash|security|code",
  "suggestion": "Alternative approach"
}
```

---

## QUICK REFERENCE CARD

```
+------------------------------------------------------------------+
|                    SPRINT QUICK REFERENCE                        |
+------------------------------------------------------------------+
| TARGET FILES: docs/*.md                                          |
|               INDEX_MAP.md                                       |
|               HEADER_MAP.md                                      |
|               .guardrails/prevention-rules/                      |
|               mcp-server/internal/mcp/resources_extended.go      |
| CHANGE TYPE:  Consolidation, indexing, MCP resources             |
+------------------------------------------------------------------+
| SAFETY:                                                          |
|   - Read before editing                                          |
|   - Keep content meaning intact                                  |
|   - Only fix duplication/split size                              |
|   - Test before commit                                           |
+------------------------------------------------------------------+
| HALT IF:                                                         |
|   - Content meaning unclear                                      |
|   - Split would break flow                                       |
|   - Uncertain about rule extraction                              |
+------------------------------------------------------------------+
| ROLLBACK: git checkout HEAD -- <files>                           |
|           rm -f new_files                                        |
+------------------------------------------------------------------+
```

---

**Created:** 2026-02-08
**Authored by:** TheArchitectit
**Archive Date:** 2026-02-15
**Version:** 1.0
