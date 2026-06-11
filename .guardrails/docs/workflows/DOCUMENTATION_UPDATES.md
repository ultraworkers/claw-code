# Documentation Update Procedures

> Post-sprint documentation maintenance.

**Related:** [MODULAR_DOCUMENTATION.md](../standards/MODULAR_DOCUMENTATION.md) | [COMMIT_WORKFLOW.md](./COMMIT_WORKFLOW.md)

---

## Overview

This document defines procedures for keeping documentation current after code changes, sprints, and releases. Up-to-date documentation is essential for project maintainability.

---

## Post-Sprint Documentation Updates

### Required Updates After Each Sprint

| Document Type | Update Required | When |
|---------------|-----------------|------|
| INDEX_MAP.md | If new docs created | After adding docs |
| HEADER_MAP.md | If sections added/changed | After doc changes |
| README.md | If features/setup changed | After feature work |
| API docs | If API changed | After API changes |
| Changelog | Always | After every sprint |
| Sprint INDEX.md | Archive completed sprint | After sprint complete |

### Sprint Archive Procedure

```
AFTER COMPLETING A SPRINT:

1. Update sprint status to COMPLETE
   - Edit sprint file header
   - Status: PENDING → COMPLETE

2. Wait archive period (default: 7 days)

3. Move to archive folder
   - mv docs/sprints/SPRINT-XXX.md docs/sprints/archive/

4. Update INDEX.md
   - Remove from Active Sprints table
   - Add to Archived Sprints reference
```

---

## Documentation Review Checklist

### After Code Changes

```
[ ] README.md
    - Installation steps still accurate?
    - Usage examples still work?
    - Configuration options current?

[ ] API Documentation
    - Endpoints current?
    - Request/response examples accurate?
    - Error codes documented?

[ ] Inline Comments
    - Complex logic explained?
    - No outdated comments?

[ ] CHANGELOG.md
    - Changes documented?
    - Version updated if needed?
```

### After Process Changes

```
[ ] AGENT_GUARDRAILS.md
    - Safety protocols current?
    - New guardrails needed?

[ ] Workflow Documents
    - Procedures still accurate?
    - New workflows documented?

[ ] INDEX_MAP.md / HEADER_MAP.md
    - All documents listed?
    - Line numbers accurate?
```

---

## Documentation Templates

### Change Log Entry Template

```markdown
## [Version] - YYYY-MM-DD

### Added
- New feature description

### Changed
- Modified behavior description

### Fixed
- Bug fix description

### Removed
- Removed feature description

### Security
- Security fix description
```

### API Documentation Template

```markdown
## Endpoint Name

**URL:** `METHOD /path/to/endpoint`

**Description:** What this endpoint does

**Headers:**
| Header | Required | Description |
|--------|----------|-------------|
| Authorization | Yes | Bearer token |

**Request Body:**
```json
{
  "field": "type - description"
}
```

**Response:**
```json
{
  "field": "type - description"
}
```

**Errors:**
| Code | Description |
|------|-------------|
| 400 | Bad request |
| 401 | Unauthorized |
```

---

## Version Control for Docs

### Documentation Commit Patterns

```
COMMIT MESSAGES FOR DOCS:

docs: update README installation steps
docs(api): add new endpoint documentation
docs(changelog): add v1.2.0 release notes
docs(guardrails): add new safety protocol
```

### Documentation Review Process

```
DOC CHANGES SHOULD BE:

1. Clear and accurate
2. Consistent with existing style
3. Free of typos
4. Properly formatted (markdown valid)
5. Under 500 lines (see MODULAR_DOCUMENTATION.md)
```

---

## Cross-Reference Maintenance

### Keeping Links Current

```
WHEN RENAMING/MOVING DOCS:

1. Search for all references to old path
   grep -r "old-filename.md" docs/

2. Update all references to new path

3. Update INDEX_MAP.md and HEADER_MAP.md

4. Verify no broken links
```

### Link Validation

```bash
# Find markdown links
grep -r "\[.*\](.*\.md)" docs/

# Check each link exists
# (Manual or with link checker tool)
```

---

## Automated Documentation Checks

### Pre-Commit Doc Checks

```
VERIFY BEFORE COMMITTING DOCS:

[ ] Markdown renders correctly
[ ] All links resolve
[ ] No duplicate headers
[ ] Line count under 500
[ ] No trailing whitespace
```

### CI/CD Documentation Checks

```yaml
# Example GitHub Action for doc validation
- name: Check documentation
  run: |
    # Check line counts
    find docs -name "*.md" -exec wc -l {} \; | awk '$1 > 500 {print "FAIL: " $2}'

    # Check for broken links (if tool available)
    # markdown-link-check docs/**/*.md
```

---

## Update Triggers

### Automatic Update Triggers

| Event | Documentation Action |
|-------|---------------------|
| New file created | Add to INDEX_MAP.md |
| Headers changed | Update HEADER_MAP.md |
| API endpoint added | Update API docs |
| Config option added | Update README |
| Feature completed | Update changelog |
| Sprint completed | Archive sprint doc |

### Manual Update Schedule

```
REGULAR REVIEW SCHEDULE:

Weekly:
- Review active sprints for staleness
- Archive completed sprints

Monthly:
- Full documentation review
- Update "Last Updated" dates
- Check for broken links

Quarterly:
- Major documentation audit
- Update version numbers
- Review for accuracy
```

---

## Quick Reference

```
+------------------------------------------------------------------+
|              DOCUMENTATION UPDATE QUICK REFERENCE                 |
+------------------------------------------------------------------+
| AFTER CODE CHANGES:                                               |
|   [ ] Update README if needed                                     |
|   [ ] Update API docs if endpoints changed                        |
|   [ ] Add changelog entry                                         |
+------------------------------------------------------------------+
| AFTER ADDING DOCUMENTS:                                           |
|   [ ] Add to INDEX_MAP.md                                         |
|   [ ] Add headers to HEADER_MAP.md                                |
|   [ ] Add to relevant INDEX.md                                    |
+------------------------------------------------------------------+
| AFTER SPRINT:                                                     |
|   [ ] Mark sprint COMPLETE                                        |
|   [ ] Archive after 7 days                                        |
|   [ ] Update sprint INDEX.md                                      |
+------------------------------------------------------------------+
| COMMIT PATTERN:                                                   |
|   docs(<scope>): <description>                                    |
+------------------------------------------------------------------+
```

---

**Last Updated:** 2026-01-14
**Authored by:** TheArchitectit
**Document Owner:** Project Maintainers
**Line Count:** ~250
