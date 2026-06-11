# Regression Tests

This directory contains regression tests for bugs that have been fixed. These tests ensure that once a bug is fixed, it stays fixed.

---

## Purpose

Regression tests:
- Verify that fixed bugs don't reoccur
- Document the conditions that caused the original bug
- Provide a safety net during refactoring
- Are NEVER deleted (only deprecated if the feature is removed)

---

## Naming Convention

```
test_<module>_regression_<failure_id>.py

Examples:
- test_parser_regression_FAIL_abc123de.py
- test_api_regression_FAIL_def456gh.py
- test_config_regression_FAIL_ghi789jk.py
```

---

## Test Structure

Every regression test MUST include:

1. **Docstring with failure_id**
2. **Description of the original bug**
3. **Description of the fix**
4. **Test that fails with old code, passes with fix**

### Template

```python
"""
Regression test for FAILURE-ID: FAIL-abc123de

Bug: Brief description of what was broken
Fix: Brief description of how it was fixed
File: src/module.py (the file that had the bug)
"""

import unittest


class TestModuleRegressionFAILabc123de(unittest.TestCase):
    """
    Test that [specific bug] regression doesn't reoccur.

    Original bug: [Detailed description of the bug]
    Impact: [What user impact was]
    Fix: [How it was fixed]
    """

    def test_bug_scenario_description(self):
        """
        Test that the specific bug scenario is handled correctly.

        This test should fail with the buggy code, pass with the fix.
        """
        # Arrange
        input_data = ...  # The input that triggered the bug

        # Act
        result = function_under_test(input_data)

        # Assert
        self.assertEqual(result, expected_result)

    def test_edge_case_related_to_bug(self):
        """Additional edge case related to the bug."""
        pass
```

---

## Adding a New Regression Test

### When to Add

- When you fix a bug
- When a bug is found in production
- When you prevent a potential bug

### Steps

1. **Fix the bug first** (in production code)
2. **Create the test file** following the naming convention
3. **Verify the test fails** with the old code (if possible)
4. **Verify the test passes** with the fix
5. **Log the failure** to the registry:
   ```bash
   python scripts/log_failure.py --interactive
   ```
6. **Run regression check** to verify everything passes:
   ```bash
   python scripts/regression_check.py
   ```

---

## Directory Structure

```
tests/regression/
├── README.md (this file)
├── test_parser_regression_FAIL_abc123de.py
├── test_api_regression_FAIL_def456gh.py
└── test_config_regression_FAIL_ghi789jk.py
```

---

## Running Regression Tests

```bash
# Run all regression tests
python -m pytest tests/regression/

# Run specific regression test
python -m pytest tests/regression/test_parser_regression_FAIL_abc123de.py

# Run with verbose output
python -m pytest tests/regression/ -v

# Run as part of full test suite
python -m pytest tests/ --regression
```

---

## Integration with Failure Registry

Each regression test corresponds to an entry in `.guardrails/failure-registry.jsonl`.

### Linking Test to Registry

The failure_id in the test name and docstring links to the registry entry.

Example registry entry:
```json
{
  "failure_id": "FAIL-abc123de",
  "category": "runtime",
  "severity": "high",
  "error_message": "TypeError: Cannot read property of undefined",
  "regression_test": "tests/regression/test_parser_regression_FAIL_abc123de.py"
}
```

---

## Best Practices

### DO

✓ Test the exact scenario that caused the bug
✓ Include edge cases related to the bug
✓ Name tests clearly after what they prevent
✓ Keep tests independent (no shared state)
✓ Make tests deterministic (no randomness)
✓ Document the original bug thoroughly

### DON'T

✗ Delete regression tests (mark deprecated instead)
✗ Combine multiple bug tests into one file
✗ Make tests that pass even with the bug
✗ Skip regression tests in CI
✗ Forget to update the failure registry

---

## Deprecating Tests

If a feature is removed and its regression test is no longer relevant:

1. **Don't delete the test file**
2. **Mark as deprecated in docstring:**
   ```python
   """
   DEPRECATED: Feature X was removed in v2.0.0
   Original bug: ...
   """
   ```
3. **Update registry entry status** to "deprecated"
4. **Keep the file** as historical documentation

---

## Examples

### Example 1: Null Check Regression

```python
"""
Regression test for FAILURE-ID: FAIL-abc123de

Bug: JSON.parse result accessed without null check
Fix: Added defensive null check before property access
File: src/parser.js
"""

import unittest
from src.parser import parse_config


class TestParserRegressionFAILabc123de(unittest.TestCase):
    """Test null check regression doesn't reoccur."""

    def test_parse_config_with_invalid_json(self):
        """Should handle invalid JSON gracefully."""
        with self.assertRaises(ValueError) as ctx:
            parse_config("not valid json")
        self.assertIn("Invalid JSON", str(ctx.exception))

    def test_parse_config_with_null_input(self):
        """Should handle null input gracefully."""
        result = parse_config(None)
        self.assertIsNone(result)
```

### Example 2: Race Condition Regression

```python
"""
Regression test for FAILURE-ID: FAIL-def456gh

Bug: Race condition in cache update caused stale data
Fix: Added atomic update operation with proper locking
File: src/cache.py
"""

import unittest
import threading
from src.cache import Cache


class TestCacheRegressionFAILdef456gh(unittest.TestCase):
    """Test race condition regression doesn't reoccur."""

    def test_concurrent_cache_updates(self):
        """Cache should handle concurrent updates correctly."""
        cache = Cache()
        errors = []

        def update_value(key, value):
            try:
                cache.set(key, value)
                cache.get(key)
            except Exception as e:
                errors.append(e)

        threads = [
            threading.Thread(target=update_value, args=("key", f"value{i}"))
            for i in range(100)
        ]

        for t in threads:
            t.start()
        for t in threads:
            t.join()

        self.assertEqual(len(errors), 0, f"Errors during concurrent access: {errors}")
```

---

## Quick Reference

```bash
# Log a new bug
python scripts/log_failure.py --interactive

# Check for regressions
python scripts/regression_check.py

# Run all regression tests
pytest tests/regression/

# View failure registry
cat .guardrails/failure-registry.jsonl
```

---

**Related Documents:**
- [REGRESSION_PREVENTION.md](../../docs/workflows/REGRESSION_PREVENTION.md) - Full regression prevention protocol
- [.guardrails/pre-work-check.md](../../.guardrails/pre-work-check.md) - Pre-work checklist
- [.guardrails/failure-registry.jsonl](../../.guardrails/failure-registry.jsonl) - Bug database

---

**Last Updated:** 2026-02-07
**Version:** 1.0
