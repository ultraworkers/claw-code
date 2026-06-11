# Modular Documentation Standards

> The 500-line rule and document organization.

**Related:** [DOCUMENTATION_UPDATES.md](../workflows/DOCUMENTATION_UPDATES.md) | [INDEX_MAP.md](../../INDEX_MAP.md)

---

## Overview

This document establishes standards for modular documentation, including the 500-line maximum rule. Modular docs are easier to maintain, navigate, and consume (especially for AI agents trying to save tokens).

---

## The 500-Line Rule

### Why 500 Lines?

```
RATIONALE:

1. Readability - Long docs are hard to navigate
2. Maintainability - Smaller files easier to update
3. Token efficiency - AI agents load less context
4. Focus - Each doc should have one clear purpose
5. Searchability - Easier to find specific content
```

### How to Count Lines

```
WHAT COUNTS:
- All lines in the file
- Blank lines
- Code blocks
- Tables

WHAT DOESN'T COUNT:
- (Everything counts)

CHECK LINE COUNT:
  wc -l <file.md>
```

### Enforcement

```bash
# Check all docs under 500 lines
find docs -name "*.md" -exec sh -c 'lines=$(wc -l < "$1"); if [ "$lines" -gt 500 ]; then echo "OVER LIMIT: $1 ($lines lines)"; fi' _ {} \;

# Automated check in CI
- name: Check doc length
  run: |
    for file in $(find docs -name "*.md"); do
      lines=$(wc -l < "$file")
      if [ "$lines" -gt 500 ]; then
        echo "ERROR: $file has $lines lines (max 500)"
        exit 1
      fi
    done
```

---

## Document Structure Standards

### Required Sections

Every document MUST have:

```markdown
# Title

> One-line description

**Related:** [links to related docs]

---

## Overview

Brief introduction

---

[Content sections...]

---

## Quick Reference

Summary box or table

---

**Last Updated:** YYYY-MM-DD
**Line Count:** ~XXX
```

### Optional Sections

```
- Prerequisites
- Examples
- Troubleshooting
- FAQ
- Related Documents (extended)
```

### Section Order

```
1. Title and metadata
2. Overview
3. Main content (logical order)
4. Quick Reference
5. Footer (updated date, line count)
```

---

## Breaking Up Large Documents

### When to Split

```
SPLIT WHEN:
- Document exceeds 400 lines (approaching limit)
- Document covers multiple distinct topics
- Sections could standalone
- Different audiences for different sections
```

### How to Split

```
SPLITTING STRATEGY:

1. Identify natural boundaries
   - Each major section could be its own doc

2. Create parent INDEX.md
   - Links to all child documents
   - Brief summary of each

3. Move sections to new files
   - One topic per file
   - Under 500 lines each

4. Update cross-references
   - Fix all internal links
   - Update INDEX_MAP.md and HEADER_MAP.md
```

### Split Example

```
BEFORE (one large doc):
  COMPREHENSIVE_GUIDE.md (800 lines)

AFTER (multiple focused docs):
  guide/
  ├── INDEX.md (50 lines)
  ├── GETTING_STARTED.md (200 lines)
  ├── CONFIGURATION.md (250 lines)
  ├── ADVANCED_USAGE.md (200 lines)
  └── TROUBLESHOOTING.md (150 lines)
```

### Cross-Reference Patterns

```markdown
# In split documents, reference siblings:

See [Configuration](./CONFIGURATION.md) for setup options.

For common issues, check [Troubleshooting](./TROUBLESHOOTING.md).

Return to [Guide Index](./INDEX.md).
```

---

## Directory Organization

### Standard Directory Structure

```
docs/
├── INDEX.md                 # Root navigation
├── AGENT_GUARDRAILS.md     # Core safety doc
├── workflows/               # How-to procedures
│   ├── INDEX.md
│   └── [workflow docs]
├── standards/               # Documentation standards
│   ├── INDEX.md
│   └── [standards docs]
├── sprints/                 # Task framework
│   ├── INDEX.md
│   ├── SPRINT_TEMPLATE.md
│   └── archive/
└── [topic]/                 # Additional topics
    ├── INDEX.md
    └── [topic docs]
```

### Naming Conventions

```
FILE NAMING:
- UPPERCASE for top-level docs (README.md, CHANGELOG.md)
- SCREAMING_CASE for guides (GETTING_STARTED.md)
- kebab-case for technical docs if preferred
- Always .md extension

DIRECTORY NAMING:
- lowercase
- descriptive
- no spaces (use hyphens)
```

### INDEX.md Pattern

Every directory with multiple docs should have INDEX.md:

```markdown
# [Directory Name] Index

> Navigation for [topic] documentation.

---

## Documents

| Document | Purpose |
|----------|---------|
| [DOC1.md](./DOC1.md) | Description |
| [DOC2.md](./DOC2.md) | Description |

---

## Quick Links

- Most used: [DOC1.md](./DOC1.md)
- Start here: [DOC2.md](./DOC2.md)
```

---

## Module Dependencies

### Explicit Dependencies

```markdown
# At top of document, declare dependencies:

**Prerequisites:**
- Read [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) first
- Familiarity with git basics

**Related:**
- [COMMIT_WORKFLOW.md](./COMMIT_WORKFLOW.md)
- [TESTING_VALIDATION.md](./TESTING_VALIDATION.md)
```

### Circular Reference Prevention

```
AVOID:
  DOC_A.md → "see DOC_B.md for details"
  DOC_B.md → "see DOC_A.md for details"

INSTEAD:
  DOC_A.md → "see DOC_B.md for X"
  DOC_B.md → self-contained for its topic

Or create DOC_C.md for shared content.
```

---

## Compliance Checklist

**Before publishing any document:**

```
[ ] Under 500 lines
[ ] Has required sections (Overview, Quick Reference)
[ ] Title describes content accurately
[ ] Related docs linked
[ ] Added to INDEX_MAP.md
[ ] Headers added to HEADER_MAP.md
[ ] Added to directory INDEX.md
[ ] No broken links
[ ] Markdown renders correctly
[ ] Last Updated date set
```

---

## Quick Reference

```
+------------------------------------------------------------------+
|              MODULAR DOCUMENTATION QUICK REFERENCE                |
+------------------------------------------------------------------+
| THE 500-LINE RULE:                                                |
|   - Maximum 500 lines per document                                |
|   - Check: wc -l <file.md>                                        |
|   - If over: split into multiple docs                             |
+------------------------------------------------------------------+
| REQUIRED SECTIONS:                                                |
|   # Title                                                         |
|   ## Overview                                                     |
|   [Content]                                                       |
|   ## Quick Reference                                              |
|   Footer (date, line count)                                       |
+------------------------------------------------------------------+
| NAMING:                                                           |
|   - Files: SCREAMING_CASE.md                                      |
|   - Directories: lowercase                                        |
|   - Always include INDEX.md in directories                        |
+------------------------------------------------------------------+
```

---

**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Last Updated:** 2026-01-14
**Line Count:** ~280
