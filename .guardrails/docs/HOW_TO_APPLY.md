# How to Apply Agent Guardrails

> Detailed instructions for adding agent guardrails to existing or new repositories.

**Related:** [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) | [INDEX_MAP.md](../INDEX_MAP.md)

---

## Overview

This document provides step-by-step instructions for applying the agent guardrails framework to repositories in different scenarios.
clone from https://github.com/TheArchitectit/agent-guardrails-template
---

## Option A: Apply to an EXISTING Repository

Execute these steps in order:

```
STEP 1: Create docs directory structure
─────────────────────────────────────────────
ACTION: Create directories if they don't exist
COMMAND: mkdir -p docs/sprints/archive

STEP 1.5: Copy Claude Configuration
─────────────────────────────────────────────
ACTION: Read CLAUDE.md from this template
ACTION: Write to TARGET_REPO/CLAUDE.md

ACTION: Read .claudeignore from this template
ACTION: Write to TARGET_REPO/.claudeignore

STEP 2: Copy AGENT_GUARDRAILS.md
─────────────────────────────────────────────
ACTION: Read docs/AGENT_GUARDRAILS.md from this template
ACTION: Write to TARGET_REPO/docs/AGENT_GUARDRAILS.md

STEP 3: Copy Sprint Framework
─────────────────────────────────────────────
ACTION: Read docs/sprints/SPRINT_TEMPLATE.md from this template
ACTION: Write to TARGET_REPO/docs/sprints/SPRINT_TEMPLATE.md

ACTION: Read docs/sprints/SPRINT_GUIDE.md from this template
ACTION: Write to TARGET_REPO/docs/sprints/SPRINT_GUIDE.md

ACTION: Read docs/sprints/INDEX.md from this template
ACTION: Write to TARGET_REPO/docs/sprints/INDEX.md

STEP 4: Update target README.md
─────────────────────────────────────────────
ACTION: Add to Documentation section:
| [**Agent Guardrails**](docs/AGENT_GUARDRAILS.md) | **MANDATORY** safety protocols for ALL AI agents |

ACTION: Add to Contributing section:
> **AI Agents:** Before contributing, read [Agent Guardrails](docs/AGENT_GUARDRAILS.md)

STEP 5: Copy GitHub templates (optional)
─────────────────────────────────────────────
ACTION: Create .github directory if needed
ACTION: Copy .github/PULL_REQUEST_TEMPLATE.md
ACTION: Copy .github/ISSUE_TEMPLATE/bug_report.md

STEP 6: Commit changes
─────────────────────────────────────────────
COMMAND: git add docs/ .github/
COMMAND: git commit -m "docs: add agent guardrails and sprint framework

Added standardized AI agent safety protocols and sprint task templates.
Applies to all LLMs and automated systems.

Authored by TheArchitectit"
```

---

## Option B: Example AI Agent Prompts

**Copy and paste these prompts when working with AI agents.**

### Prompt 1: Add Guardrails to Existing Repository

```
Add the agent guardrails framework from https://github.com/TheArchitectit/agent-guardrails-template
to this repository.

REQUIREMENTS:
1. Create docs/workflows/ and docs/standards/ directories
2. Copy all guardrails documentation (20 total files)
3. Update CLAUDE.md to reference guardrails
4. Update README.md to include guardrails section
5. Update existing documentation index files if they exist
6. Do NOT modify existing code files
7. Commit with message: "docs: add agent guardrails v1.2 framework"

MUST INCLUDE:
- docs/AGENT_GUARDRAILS.md
- docs/standards/TEST_PRODUCTION_SEPARATION.md
- docs/workflows/AGENT_EXECUTION.md
- docs/workflows/AGENT_ESCALATION.md
- All workflow (10 files) and standards (5 files) documents
```

### Prompt 2: Update README with Guardrails Reference

```
Update this repository's README.md to include agent guardrails references.

ADD to top of README:
---
## ⚠️ MANDATORY: AI Agent Guardrails

ALL AI agents MUST read these documents BEFORE any code changes:

| Document | Purpose | Critical For |
|----------|---------|--------------|
| [AGENT_GUARDRAILS.md](docs/AGENT_GUARDRAILS.md) | Core safety protocols (MANDATORY) | ALL operations |
| [TEST_PRODUCTION_SEPARATION.md](docs/standards/TEST_PRODUCTION_SEPARATION.md) | Test/production isolation (MANDATORY) | Deployments |
| [AGENT_EXECUTION.md](docs/workflows/AGENT_EXECUTION.md) | Execution protocol & rollback | Task execution |
| [AGENT_ESCALATION.md](docs/workflows/AGENT_ESCALATION.md) | Audit & escalation procedures | Errors/Uncertainty |
---

UPDATE Contributing section to add:
> **AI Agents:** Before contributing, read [Agent Guardrails](docs/AGENT_GUARDRAILS.md) and [Test/Production Separation](docs/standards/TEST_PRODUCTION_SEPARATION.md).

UPDATE existing Documentation table to include:
| [**Test/Production Separation**](docs/standards/TEST_PRODUCTION_SEPARATION.md) | **MANDATORY** test/production isolation standards |
```

### Prompt 3: Migrate Existing Documentation to Guardrails

```
I have existing documentation in this repository. Help me migrate it to follow
the agent guardrails framework.

ANALYSIS NEEDED:
1. List all existing documentation files
2. Check if any are over 500 lines (need splitting)
3. Identify which documents could be moved to:
   - docs/workflows/ (operational procedures)
   - docs/standards/ (coding standards)
   - docs/sprints/ (task documents)
4. Check if AGENT_GUARDRAILS.md exists
5. Check if INDEX_MAP.md or HEADER_MAP.md exist

ACTIONS TO TAKE:
1. Create missing directory structures
2. Import guardrails framework from /mnt/ollama/git/agent-guardrails-template
3. Reorganize existing docs into appropriate directories
4. Split any docs over 500 lines
5. Create/update INDEX_MAP.md and HEADER_MAP.md
6. Update README.md navigation
7. Commit with descriptive message

ASK ME before:
- Deleting any documentation
- Splitting any documents
- Making major structural changes
```

### Prompt 4: Quick Guardrails Copy

```
Quickly copy the guardrails framework to this repository.

Commands to execute:
1. mkdir -p docs/workflows docs/standards docs/sprints/archive
2. cp /mnt/ollama/git/agent-guardrails-template/docs/AGENT_GUARDRAILS.md docs/
3. cp /mnt/ollama/git/agent-guardrails-template/docs/workflows/*.md docs/workflows/
4. cp /mnt/ollama/git/agent-guardrails-template/docs/standards/*.md docs/standards/
5. cp /mnt/ollama/git/agent-guardrails-template/docs/sprints/*.md docs/sprints/
6. Add guardrails section to README.md and CLAUDE.md
7. git add docs/
8. git commit -m "docs: add agent guardrails v1.2 framework

- Added comprehensive safety protocols for AI agents
- Added test/production separation standards (MANDATORY)
- Added execution protocols and audit requirements
- All documents under 500-line compliance

Authored by TheArchitectit"

DO NOT:
- Modify any source code files
- Delete existing documentation
- Change .gitignore
```

### Prompt 5: Verify Guardrails Installation

```
Verify that agent guardrails framework is properly installed in this repository.

CHECK:
[ ] docs/AGENT_GUARDRAILS.md exists and is complete
[ ] docs/standards/TEST_PRODUCTION_SEPARATION.md exists
[ ] docs/workflows/AGENT_EXECUTION.md exists
[ ] docs/workflows/AGENT_ESCALATION.md exists
[ ] docs/workflows/ contains at least 10 files
[ ] docs/standards/ contains at least 5 files
[ ] README.md references guardrails
[ ] CLAUDE.md references guardrails
[ ] All documentation files are under 500 lines (check with wc -l)
[ ] No duplicate files from old framework

REPORT:
- Number of guardrails files installed
- total documentation count
- Any issues found
- Verification status: PASS/FAIL
```

---

## Option C: Create a NEW Repository with Standards

Execute these steps in order:

```
STEP 1: Create new repository
─────────────────────────────────────────────
COMMAND: mkdir new-project && cd new-project
COMMAND: git init

STEP 2: Create directory structure
─────────────────────────────────────────────
COMMAND: mkdir -p src tests docs/sprints/archive .github/ISSUE_TEMPLATE

STEP 3: Copy all template files
─────────────────────────────────────────────
FILES TO COPY:
  - INDEX_MAP.md (navigation map)
  - HEADER_MAP.md (section lookup)
  - CLAUDE.md
  - .claudeignore
  - docs/AGENT_GUARDRAILS.md
  - docs/workflows/ (all 10 files)
  - docs/standards/ (all 5 files)
  - docs/sprints/SPRINT_TEMPLATE.md
  - docs/sprints/SPRINT_GUIDE.md
  - docs/sprints/INDEX.md
  - .github/SECRETS_MANAGEMENT.md
  - .github/workflows/ (all 3 files)
  - .github/PULL_REQUEST_TEMPLATE.md
  - .github/ISSUE_TEMPLATE/bug_report.md
  - .gitignore

STEP 4: Create README.md
─────────────────────────────────────────────
ACTION: Use the PROJECT README TEMPLATE section in main README.md
ACTION: Customize for the specific project

STEP 5: Initial commit
─────────────────────────────────────────────
COMMAND: git add -A
COMMAND: git commit -m "feat: initial project setup with agent guardrails

- Project structure initialized
- Agent guardrails and sprint framework included
- GitHub templates configured

Authored by TheArchitectit"

STEP 6: Create GitHub repo (if requested)
─────────────────────────────────────────────
COMMAND: gh repo create PROJECT_NAME --private --source=. --push
```

---

## Option D: Migrate Existing Documentation to Guardrails Structure

**If you have an existing repository with documentation, restructure it to follow guardrails conventions.**

```
STEP 1: Analyze Existing Documentation
─────────────────────────────────────────────
ACTION: List all documentation in repository
COMMAND: find . -name "*.md" -type f | grep -v node_modules | grep -v target

ACTION: Check line counts
COMMAND: for f in **/*.md; do echo "$f: $(wc -l < $f) lines"; done

STEP 2: Categorize Documents
─────────────────────────────────────────────
Create directory structure:
  docs/workflows/     - Operational procedures (commit, push, rollback, etc.)
  docs/standards/     - Coding standards and patterns (logging, API specs, etc.)
  docs/sprints/       - Task documents and sprint templates

STEP 3: Import Guardrails Framework
─────────────────────────────────────────────
Follow Option B prompts to add guardrails

STEP 4: Reorganize Existing Docs
─────────────────────────────────────────────
CATEGORIZATION GUIDE:

WORKFOLWS (move to docs/workflows/):
  - Commit procedures → COMMIT_WORKFLOW.md (or merge with existing)
  - Git operations → GIT_*.md files
  - Testing procedures → TESTING_VALIDATION.md (or merge)
  - Code review → CODE_REVIEW.md (or merge)

STANDARDS (move to docs/standards/):
  - Coding conventions → MODULAR_DOCUMENTATION.md (or merge)
  - Logging patterns → LOGGING_PATTERNS.md (or merge)
  - API documentation → API_SPECIFICATIONS.md (or merge)

Sprints (move to docs/sprints/):
  - Task documents → Keep as individual sprint docs
  - Templates → Rename if using SPRINT_TEMPLATE.md format

STEP 5: Split Oversized Documents
─────────────────────────────────────────────
IF any doc > 500 lines:
  1. Identify natural section boundaries
  2. Create parent doc in new location with INDEX.md
  3. Move sections to separate files
  4. Update cross-references

STEP 6: Create Navigation Maps
─────────────────────────────────────────────
Import or create:
  - INDEX_MAP.md - Master navigation by keyword
  - HEADER_MAP.md - Section headers with line numbers

STEP 7: Update README.md
─────────────────────────────────────────────
Add documentation section:
---
## Documentation

| Document | Description |
|----------|-------------|
| [**INDEX_MAP.md**](INDEX_MAP.md) | Master navigation - find docs by keyword |
| [**Agent Guardrails**](docs/AGENT_GUARDRAILS.md) | **MANDATORY** safety protocols |
| [**Workflows**](docs/workflows/INDEX.md) | Operational procedures |
| [**Standards**](docs/standards/INDEX.md) | Coding standards |

---

STEP 8: Commit Migration
─────────────────────────────────────────────
COMMAND: git add docs/ .github/
COMMAND: git commit -m "docs: restructure documentation to follow guardrails conventions

- Imported agent guardrails framework v1.2
- Reorganized existing docs into workflows/standards/sprints/
- Split oversized documents for 500-line compliance
- Added navigation maps (INDEX_MAP.md, HEADER_MAP.md)
- Updated README.md documentation section

Authored by TheArchitectit"
```

### Migration Example

**Before:**
```
project/
├── README.md
├── CONTRIBUTING.md
├── DOCUMENTATION.md (800 lines - too large)
├── GIT_GUIDE.md
├── CODING_STANDARDS.md
└── .github/
```

**After:**
```
project/
├── README.md
├── CLAUDE.md
├── .claudeignore
├── INDEX_MAP.md
├── HEADER_MAP.md
├── docs/
│   ├── AGENT_GUARDRAILS.md
│   ├── CONTRIBUTING.md
│   ├── workflows/
│   │   ├── INDEX.md
│   │   ├── COMMIT_WORKFLOW.md
│   │   ├── GIT_PUSH_PROCEDURES.md
│   │   └── ...
│   ├── standards/
│   │   ├── INDEX.md
│   │   ├── TEST_PRODUCTION_SEPARATION.md
│   │   ├── MODULAR_DOCUMENTATION.md
│   │   └── CODING_STANDARDS.md
│   └── DOCUMENTATION/
│       ├── INDEX.md
│       ├── ARCHITECTURE.md (300 lines)
│       ├── API_REFERENCE.md (350 lines)
│       └── DEPLOYMENT.md (150 lines)
└── .github/
```

---

## Verification Checklist

After applying the template, verify:

```
NAVIGATION MAPS:
[ ] INDEX_MAP.md exists at root
[ ] HEADER_MAP.md exists at root

CORE DOCUMENTATION:
[ ] docs/AGENT_GUARDRAILS.md exists and is complete
[ ] docs/workflows/ contains 10 workflow documents (including AGENT_EXECUTION.md, AGENT_ESCALATION.md)
[ ] docs/standards/ contains 5 standards documents (including TEST_PRODUCTION_SEPARATION.md)
[ ] docs/sprints/SPRINT_TEMPLATE.md exists
[ ] docs/sprints/SPRINT_GUIDE.md exists
[ ] docs/sprints/INDEX.md exists

GITHUB INTEGRATION:
[ ] .github/SECRETS_MANAGEMENT.md exists
[ ] .github/workflows/ contains 3 CI workflows
[ ] .github/PULL_REQUEST_TEMPLATE.md exists

PROJECT FILES:
[ ] CLAUDE.md configured for project
[ ] README.md links to INDEX_MAP.md and Agent Guardrails
[ ] .gitignore exists
[ ] All docs under 500 lines (check with: find docs -name "*.md" -exec sh -c 'wc -l "$1" | awk "{if($1>500) print $1\" lines: \"$1}"' _ {} \;)
```

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-01-16
**Line Count:** ~295
