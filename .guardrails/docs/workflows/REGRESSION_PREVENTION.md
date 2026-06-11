# Regression Prevention Protocol

> Comprehensive guide to preventing bug reintroduction using the failure registry and prevention rules.

**Related:** [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) | [AGENT_EXECUTION.md](./AGENT_EXECUTION.md) | [AGENT_REVIEW_PROTOCOL.md](./AGENT_REVIEW_PROTOCOL.md)

---

## Overview

The Regression Prevention System ensures that once a bug is fixed, it stays fixed. It consists of:

1. **Failure Registry** - Append-only log of all bugs
2. **Prevention Rules** - Automated pattern detection
3. **Pre-Work Checks** - Mandatory verification before editing
4. **Regression Tests** - Permanent tests for fixed bugs
5. **CI Integration** - Automated enforcement

---

## Core Philosophy

```
┌─────────────────────────────────────────────────────────────┐
│                 REGRESSION PREVENTION MANDATE                │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Every bug is a learning opportunity                      │
│  2. Every fix must be documented                             │
│  3. Every pattern must be prevented                          │
│  4. Every edit must be checked                               │
│  5. Every fix must have a regression test                    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Failure Registry

### Location
```
.guardrails/failure-registry.jsonl
```

### Format
Each line is a JSON object:
```json
{
  "failure_id": "FAIL-abc123de",
  "timestamp": "2026-02-07T10:00:00Z",
  "category": "runtime",
  "severity": "high",
  "error_message": "TypeError: Cannot read property of undefined",
  "root_cause": "Missing null check after JSON.parse",
  "affected_files": ["src/parser.js"],
  "fix_commit": "a1b2c3d4",
  "regression_pattern": "JSON\\.parse\\(.*\\)\\.\\w+",
  "prevention_rule": "Always null-check parsed JSON before property access",
  "status": "active"
}
```

### Categories
| Category | Description | Example |
|----------|-------------|---------|
| `build` | Build/compilation errors | Missing import, syntax error |
| `runtime` | Runtime exceptions | Null pointer, undefined access |
| `test` | Test failures | Assertion errors, timeouts |
| `type` | Type system errors | Type mismatches, inference failures |
| `lint` | Style/lint violations | ESLint, pylint errors |
| `deploy` | Deployment failures | CI/CD errors, publish failures |
| `config` | Configuration errors | Missing env vars, invalid config |
| `regression` | Reintroduced bugs | Previously fixed bugs that returned |

### Severity Levels
| Level | Impact | Response Time |
|-------|--------|---------------|
| `critical` | System down, data loss | Immediate |
| `high` | Major feature broken | Within 4 hours |
| `medium` | Minor feature issue | Within 24 hours |
| `low` | Cosmetic, non-blocking | Next sprint |

---

## Using the Registry

### Log a New Failure

Interactive mode (recommended):
```bash
python scripts/log_failure.py --interactive
```

Quick entry from error message:
```bash
python scripts/log_failure.py \
  --from-error "TypeError: Cannot read property 'x' of undefined" \
  --category runtime \
  --severity high \
  --root-cause "Missing null check" \
  --affected-files src/parser.js \
  --fix-commit abc1234
```

### List Failures

All active failures:
```bash
python scripts/log_failure.py --list
```

Filtered by category:
```bash
python scripts/log_failure.py --list | grep runtime
```

### View Specific Failure

```bash
python scripts/log_failure.py --show FAIL-abc123de
```

### Update Status

Mark as resolved:
```bash
python scripts/log_failure.py --resolve FAIL-abc123de
```

Mark as deprecated (no longer relevant):
```bash
python scripts/log_failure.py --deprecate FAIL-abc123de
```

---

## Prevention Rules

### Location
```
.guardrails/prevention-rules/
├── pattern-rules.json      # Regex-based rules
└── semantic-rules.json     # AST-based rules
```

### Pattern Rules

Detect problematic code patterns using regex:

```json
{
  "rule_id": "PREVENT-001",
  "failure_id": "FAIL-abc123de",
  "name": "Null check after async parse",
  "enabled": true,
  "pattern": "JSON\\.parse\\(.*\\)\\s*\\.\\w+",
  "forbidden_context": "without.*null.*check",
  "message": "Previous bug: Direct property access on JSON.parse without null check",
  "severity": "error",
  "file_glob": ["*.js", "*.ts"],
  "suggestion": "Add null check: const data = JSON.parse(...); if (data) { ... }"
}
```

### Semantic Rules

Detect issues using AST analysis:

```json
{
  "rule_id": "SEMANTIC-001",
  "failure_id": "FAIL-def567gh",
  "name": "Unhandled promise rejection",
  "enabled": true,
  "language": "javascript",
  "ast_pattern": {
    "type": "CallExpression",
    "callee": {
      "type": "MemberExpression",
      "property": { "name": "then" }
    },
    "missing": "catch"
  },
  "message": "Promise chain missing .catch() handler",
  "severity": "warning"
}
```

### Enabling/Disabling Rules

Edit the rule file and set `enabled: true/false`.

**Never delete rules** - only disable them. History is important.

---

## Pre-Work Check Protocol

### MANDATORY: Before ANY File Edit

**Step 1: Read Pre-Work Check Document**
```bash
cat .guardrails/pre-work-check.md
```

**Step 2: Run Regression Check**
```bash
python scripts/regression_check.py --all
```

**Step 3: Check Registry for Your Files**
```bash
python scripts/log_failure.py --list | grep <your-file>
```

**Step 4: Verify Understanding**
- [ ] I know what bugs have been fixed in these files
- [ ] I understand the patterns that caused them
- [ ] I will not reintroduce these patterns

### During Development

**Run regression check frequently:**
```bash
# After making changes
python scripts/regression_check.py --unstaged
```

### Before Commit

**Final verification:**
```bash
python scripts/regression_check.py --staged
```

---

## Regression Testing Requirements

### Every Bug Fix MUST Include:

1. **The fix itself** (production code)
2. **A regression test** (test code)
3. **A registry entry** (documentation)

### Regression Test Location
```
tests/regression/
├── test_<module>_regression_<failure_id>.py
└── README.md
```

### Regression Test Template

```python
"""
Regression test for FAILURE-ID: FAIL-abc123de

Bug: Brief description of what was broken
Fix: Brief description of how it was fixed
"""

import unittest


class TestParserRegressionFAILabc123de(unittest.TestCase):
    """
    Test that null check regression doesn't reoccur.

    Original bug: JSON.parse result accessed without null check
    caused TypeError when input was invalid JSON.
    """

    def test_json_parse_with_null_input(self):
        """Should handle null result from JSON.parse gracefully."""
        # This test should fail with the buggy code, pass with the fix
        from src.parser import parse_json

        with self.assertRaises(ValueError) as ctx:
            parse_json("invalid json")

        self.assertIn("Invalid JSON", str(ctx.exception))

    def test_json_parse_with_valid_input(self):
        """Should work normally with valid JSON."""
        from src.parser import parse_json

        result = parse_json('{"key": "value"}')
        self.assertEqual(result["key"], "value")
```

### Regression Test Naming Convention

```
test_<module>_regression_<failure_id>.py

Examples:
- test_parser_regression_FAIL_abc123de.py
- test_api_regression_FAIL_def456gh.py
- test_config_regression_FAIL_ghi789jk.py
```

### Regression Test Docstring Requirements

Every regression test MUST include:
- The failure_id
- Brief bug description
- Brief fix description
- Link to the registry entry

---

## CI/CD Integration

### GitHub Action

The regression guard runs on every PR:

```yaml
# .github/workflows/regression-guard.yml
# See full file in repository
```

### What It Checks

1. **Diff Analysis** - Scans PR diff against prevention rules
2. **File History** - Warns if modifying files with known bugs
3. **Test Requirements** - Requires regression tests for bug fixes
4. **Pattern Matching** - Fails if known bad patterns detected

### Pre-Commit Hook

Add to `.git/hooks/pre-commit`:

```bash
#!/bin/bash
# Regression check pre-commit hook

echo "Running regression check..."
python scripts/regression_check.py --staged --pre-commit

if [ $? -ne 0 ]; then
    echo ""
    echo "Potential regressions detected!"
    echo "Review the output above or use --no-verify to skip (not recommended)"
    exit 1
fi
```

Make it executable:
```bash
chmod +x .git/hooks/pre-commit
```

---

## Review Protocol

### For Authors

Before requesting review:
- [ ] Regression check passes (`python scripts/regression_check.py --staged`)
- [ ] All bug fixes have regression tests
- [ ] Registry entries created for new bugs
- [ ] No previous fixes were undone

### For Reviewers

Checklist:
- [ ] Changes don't match known bug patterns
- [ ] Files with active failures reviewed carefully
- [ ] Regression tests exist for bug fixes
- [ ] Prevention rules updated if needed

---

## Common Scenarios

### Scenario 1: Fixing a New Bug

1. Fix the bug
2. Create regression test
3. Log in registry:
   ```bash
   python scripts/log_failure.py --interactive
   ```
4. Consider adding prevention rule
5. Commit with `fix:` prefix

### Scenario 2: Modifying File with Known Bugs

1. Read registry entries for the file
2. Understand what was fixed before
3. Run regression check before editing
4. Be extra careful with similar patterns
5. Verify your changes don't undo fixes

### Scenario 3: Bug Reintroduced

1. Don't panic - this is why we have the system
2. Fix it again (with better understanding)
3. Update registry entry:
   - Increase severity
   - Update regression_pattern
   - Add prevention rule if missing
4. Strengthen regression test
5. Review why check didn't catch it

### Scenario 4: False Positive

1. Verify it's truly a false positive
2. Update rule to exclude valid cases:
   - Add `forbidden_context` pattern
   - Refine regex
   - Disable rule if necessary
3. Document the decision

---

## Metrics and Success Criteria

### System Health Metrics

Track these monthly:

| Metric | Target | Measurement |
|--------|--------|-------------|
| Registry Coverage | 100% | % of bugs logged |
| Regression Rate | 0% | Bugs reintroduced / total bugs |
| Prevention Rate | >90% | Issues caught by automation |
| Check Adoption | 100% | % of edits with pre-check |

### Quality Indicators

Healthy system:
- Zero regressions of registered bugs
- All bug fixes include regression tests
- Prevention rules catch issues pre-commit
- CI catches issues pre-merge

Warning signs:
- Same bug fixed multiple times
- Registry entries without regression tests
- Skipped pre-work checks
- Disabled rules without justification

---

## Best Practices

### DO

✓ Log every bug to the registry
✓ Write regression tests for every fix
✓ Run regression check before committing
✓ Update prevention rules when patterns emerge
✓ Review registry before modifying files
✓ Mark bugs resolved when appropriate

### DON'T

✗ Delete registry entries (mark deprecated instead)
✗ Skip pre-work checks
✗ Disable rules without documenting why
✗ Write vague prevention rules
✗ Forget to add regression tests
✗ Assume "I won't make that mistake again"

---

## Troubleshooting

### Regression check is too slow

- Use `--staged` instead of `--all`
- Disable expensive semantic rules
- Run only on changed files

### Too many false positives

- Refine regex patterns
- Add `forbidden_context` exceptions
- Update rules to be more specific

### Registry is too large

- Mark old entries as deprecated
- Archive entries older than 2 years
- Focus on patterns, not every instance

### Team isn't using it

- Add to CI as mandatory check
- Include in code review checklist
- Share regression success stories
- Make it part of onboarding

---

## Quick Reference

```bash
# Before work
cat .guardrails/pre-work-check.md
python scripts/regression_check.py --all

# During work
python scripts/regression_check.py --unstaged

# Before commit
python scripts/regression_check.py --staged

# Log new bug
python scripts/log_failure.py --interactive

# List bugs
python scripts/log_failure.py --list

# Resolve bug
python scripts/log_failure.py --resolve FAIL-xxx
```

---

**Related Documents:**
- [AGENT_GUARDRAILS.md](../AGENT_GUARDRAILS.md) - Core safety protocols
- [AGENT_EXECUTION.md](./AGENT_EXECUTION.md) - Execution protocol
- [AGENT_REVIEW_PROTOCOL.md](./AGENT_REVIEW_PROTOCOL.md) - Review checklist
- [TESTING_VALIDATION.md](./TESTING_VALIDATION.md) - Testing requirements

---

**Last Updated:** 2026-02-07
**Version:** 1.0
**Document Owner:** Project Maintainers
