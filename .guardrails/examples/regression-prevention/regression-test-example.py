#!/usr/bin/env python3
"""
Regression Test Template and Examples

This file demonstrates the regression testing pattern for the Bug Tracking &
Regression Prevention System. Copy the template section and adapt it for your bug fix.

Related:
  - ../../docs/workflows/REGRESSION_PREVENTION.md
  - ../failure-registry-examples.jsonl
  - ../prevention-rules-examples.json

Usage:
  1. Copy the TEMPLATE section below
  2. Update FAILURE_ID, description, and test cases
  3. Place in tests/regression/test_<module>_regression_<failure_id>.py
  4. Run with: python -m pytest tests/regression/test_*_regression_*.py -v
"""

import unittest
from typing import Any, Dict, Optional

# =============================================================================
# TEMPLATE - Copy and customize for new regression tests
# =============================================================================

"""
# tests/regression/test_<module>_regression_<FAILURE_ID>.py

\'\'\'
Regression test for FAILURE_ID: FAIL-XXX-NNN

Bug: <Brief description of what was broken>
Fix: <Brief description of how it was fixed>
Registry: ../../.guardrails/failure-registry.jsonl

This test MUST fail with the buggy code and pass with the fix.
If this test ever fails, the bug has been reintroduced.
\'\'\'

import unittest


class Test<Module>Regression<FAILURE_ID>(unittest.TestCase):
    \'\'\'
    Regression test for <FAILURE_ID>: <short description>

    Original bug: <detailed description>
    Root cause: <why it happened>
    Fix commit: <git SHA>
    \'\'\'

    def test_<scenario>_should_<expected_behavior>(self):
        \'\'\'
        Test that <condition> is handled correctly.

        This test would have caught the original bug where <what happened>.
        \'\'\'
        # Arrange
        <setup code>

        # Act & Assert
        with self.assertRaises(<ExpectedException>) as ctx:
            <code that triggers bug>

        self.assertIn(<expected message>, str(ctx.exception))

    def test_<normal_case>_should_work(self):
        \'\'\'Normal operation should not be affected by the fix.\'\'\'
        # Arrange
        <setup code>

        # Act
        result = <function call>

        # Assert
        self.assertEqual(result, <expected>)


if __name__ == '__main__':
    unittest.main()
"""


# =============================================================================
# EXAMPLE 1: Null Check After JSON Parse (FAIL-WEB-001)
# =============================================================================
# This example shows a regression test for a real bug where parsing webhook
# payload without null check caused production outage.

class PaymentWebhookParser:
    """Parses payment webhook payloads - FIXED VERSION."""

    @staticmethod
    def parse_amount(payload: str) -> Dict[str, Any]:
        """Parse payment amount from webhook payload with validation."""
        if not payload:
            raise ValueError("Empty payload received")

        try:
            data = __import__('json').loads(payload)
        except __import__('json').JSONDecodeError as e:
            raise ValueError(f"Invalid JSON in payload: {e}")

        if not isinstance(data, dict):
            raise ValueError("Payload must be a JSON object")

        if 'amount' not in data:
            raise ValueError("Missing required field: amount")

        if not isinstance(data['amount'], (int, float)):
            raise ValueError("Amount must be a number")

        return data


class TestPaymentRegressionFAIL_WEB_001(unittest.TestCase):
    """
    Regression test for FAIL-WEB-001: Null check after JSON parse

    Original bug: JSON.parse result accessed without null check caused
    TypeError when webhook payload was malformed or empty.

    Impact: Payment processing failed for 500+ customers.
    Fix: Added comprehensive validation layer before property access.
    Fix commit: a1b2c3d4e5f6789012345678
    Prevention rule: PREVENT-EX-001
    """

    def test_parse_empty_payload_should_raise_error(self):
        """
        Empty payload should raise ValueError, not TypeError.

        Buggy code: const data = JSON.parse(payload); return data.amount;
        Would throw: TypeError: Cannot read property 'amount' of undefined
        """
        with self.assertRaises(ValueError) as ctx:
            PaymentWebhookParser.parse_amount("")

        self.assertIn("Empty payload", str(ctx.exception))

    def test_parse_invalid_json_should_raise_error(self):
        """
        Invalid JSON should raise ValueError with context.

        Buggy code would crash with JSONDecodeError or return None
        causing later TypeError.
        """
        with self.assertRaises(ValueError) as ctx:
            PaymentWebhookParser.parse_amount("not valid json {{ ")

        self.assertIn("Invalid JSON", str(ctx.exception))

    def test_parse_non_object_json_should_raise_error(self):
        """
        Valid JSON that is not an object should be rejected.

        Payload like "null" or "123" would pass JSON.parse but fail
        on property access.
        """
        with self.assertRaises(ValueError) as ctx:
            PaymentWebhookParser.parse_amount("null")

        self.assertIn("must be a JSON object", str(ctx.exception))

    def test_parse_missing_amount_should_raise_error(self):
        """
        JSON object missing required 'amount' field should fail gracefully."""
        with self.assertRaises(ValueError) as ctx:
            PaymentWebhookParser.parse_amount('{"currency": "USD"}')

        self.assertIn("Missing required field: amount", str(ctx.exception))

    def test_parse_valid_payload_should_work(self):
        """Normal operation: valid payload should parse correctly."""
        result = PaymentWebhookParser.parse_amount('{"amount": 99.99, "currency": "USD"}')

        self.assertEqual(result['amount'], 99.99)
        self.assertEqual(result['currency'], 'USD')


# =============================================================================
# EXAMPLE 2: Cache Race Condition (FAIL-CACHE-001)
# =============================================================================
# This example shows regression test for race condition in cache updates.

class CacheManager:
    """Thread-safe cache manager - FIXED VERSION."""

    def __init__(self):
        self._cache: Dict[str, Any] = {}
        self._locks: Dict[str, Any] = {}
        self._global_lock = __import__('threading').Lock()

    def _get_lock(self, key: str):
        """Get or create lock for key."""
        with self._global_lock:
            if key not in self._locks:
                self._locks[key] = __import__('threading').Lock()
            return self._locks[key]

    def atomic_increment(self, key: str) -> int:
        """Atomically increment counter - thread safe."""
        with self._get_lock(key):
            current = self._cache.get(key, 0)
            new_value = current + 1
            self._cache[key] = new_value
            return new_value

    # BUGGY VERSION (for testing the test):
    # def atomic_increment(self, key: str) -> int:
    #     """Non-atomic increment - has race condition."""
    #     current = self._cache.get(key, 0)  # Race: two threads read same value
    #     new_value = current + 1
    #     self._cache[key] = new_value  # Race: last write wins
    #     return new_value


class TestCacheRegressionFAIL_CACHE_001(unittest.TestCase):
    """
    Regression test for FAIL-CACHE-001: Cache race condition

    Original bug: Non-atomic read-modify-write cycle caused stale data
    under concurrent load. Two threads would read same value, both
    increment, and both write - one update lost.

    Impact: Inventory counts wrong, overselling products.
    Fix: Added per-key locking for atomic operations.
    Fix commit: c3d4e5f6a7b8901234567890
    Prevention rule: PREVENT-EX-005
    """

    def test_concurrent_increments_should_not_lose_updates(self):
        """
        Multiple threads incrementing same key should all be counted.

        Buggy code would lose updates:
          Thread A: read 0 -> increment -> write 1
          Thread B: read 0 -> increment -> write 1 (A's update lost!)

        Fixed code:
          Thread A: acquire lock -> read 0 -> increment -> write 1 -> release
          Thread B: acquire lock -> read 1 -> increment -> write 2 -> release
        """
        import threading
        import time

        cache = CacheManager()
        key = "counter"
        num_threads = 10
        increments_per_thread = 100

        def increment_many():
            for _ in range(increments_per_thread):
                cache.atomic_increment(key)
                time.sleep(0.001)  # Increase chance of race

        threads = [
            threading.Thread(target=increment_many)
            for _ in range(num_threads)
        ]

        for t in threads:
            t.start()
        for t in threads:
            t.join()

        expected = num_threads * increments_per_thread
        actual = cache._cache.get(key, 0)

        self.assertEqual(
            actual, expected,
            f"Race condition detected: expected {expected}, got {actual}. "
            f"Lost {expected - actual} updates!"
        )

    def test_separate_keys_should_not_block_each_other(self):
        """Different keys can be updated concurrently without blocking."""
        import threading
        import time

        cache = CacheManager()
        results = {}

        def increment_key(key: str, count: int):
            for _ in range(count):
                cache.atomic_increment(key)
            results[key] = cache._cache.get(key, 0)

        threads = [
            threading.Thread(target=increment_key, args=(f"key_{i}", 10))
            for i in range(5)
        ]

        start_time = time.time()
        for t in threads:
            t.start()
        for t in threads:
            t.join()
        elapsed = time.time() - start_time

        # All keys should have correct values
        for i in range(5):
            self.assertEqual(results.get(f"key_{i}"), 10)

        # Should complete in reasonable time (parallel, not sequential)
        self.assertLess(elapsed, 2.0, "Operations appear sequential, not parallel")


# =============================================================================
# EXAMPLE 3: Environment Variable Validation (FAIL-CFG-001)
# =============================================================================
# This example shows regression test for missing env var validation.

class ConfigLoader:
    """Configuration loader with validation - FIXED VERSION."""

    REQUIRED_VARS = ['DATABASE_URL', 'API_KEY', 'SECRET_KEY']

    @classmethod
    def load(cls) -> Dict[str, str]:
        """Load configuration with validation."""
        import os

        config = {}
        missing = []

        for var in cls.REQUIRED_VARS:
            value = os.environ.get(var)
            if not value:
                missing.append(var)
            else:
                config[var] = value

        if missing:
            raise RuntimeError(
                f"Missing required environment variables: {', '.join(missing)}. "
                f"Please set them before starting the application."
            )

        return config


class TestConfigRegressionFAIL_CFG_001(unittest.TestCase):
    """
    Regression test for FAIL-CFG-001: Missing env var validation

    Original bug: Service crashed on startup with KeyError when
    DATABASE_URL environment variable was not set. Error only
    manifested on first database connection attempt.

    Impact: Production deployment rolled back, 2 hour outage.
    Fix: Added startup validation with clear error messages.
    Fix commit: d4e5f6a7b8c9012345678901
    Prevention rule: PREVENT-EX-003
    """

    def setUp(self):
        """Save original environment."""
        import os
        self.original_env = dict(os.environ)

    def tearDown(self):
        """Restore original environment."""
        import os
        os.environ.clear()
        os.environ.update(self.original_env)

    def test_missing_database_url_should_raise_clear_error(self):
        """
        Missing DATABASE_URL should give clear error at startup.

        Buggy code would crash later with:
          KeyError: 'DATABASE_URL'

        Fixed code gives:
          RuntimeError: Missing required environment variables: DATABASE_URL
        """
        import os

        # Clear required vars
        for var in ConfigLoader.REQUIRED_VARS:
            if var in os.environ:
                del os.environ[var]

        with self.assertRaises(RuntimeError) as ctx:
            ConfigLoader.load()

        error_msg = str(ctx.exception)
        self.assertIn("DATABASE_URL", error_msg)
        self.assertIn("Missing required", error_msg)
        self.assertIn("environment variables", error_msg)

    def test_multiple_missing_vars_should_list_all(self):
        """Error should list ALL missing vars, not just first one."""
        import os

        # Clear all required vars
        for var in ConfigLoader.REQUIRED_VARS:
            if var in os.environ:
                del os.environ[var]

        with self.assertRaises(RuntimeError) as ctx:
            ConfigLoader.load()

        error_msg = str(ctx.exception)

        # Should mention all missing variables
        for var in ConfigLoader.REQUIRED_VARS:
            self.assertIn(var, error_msg)

    def test_valid_environment_should_load_successfully(self):
        """Normal operation with all vars set."""
        import os

        # Set all required vars
        os.environ['DATABASE_URL'] = 'postgresql://localhost/db'
        os.environ['API_KEY'] = 'test-api-key'
        os.environ['SECRET_KEY'] = 'test-secret-key'

        config = ConfigLoader.load()

        self.assertEqual(config['DATABASE_URL'], 'postgresql://localhost/db')
        self.assertEqual(config['API_KEY'], 'test-api-key')
        self.assertEqual(config['SECRET_KEY'], 'test-secret-key')


# =============================================================================
# Running the tests
# =============================================================================

if __name__ == '__main__':
    # Run with verbose output
    unittest.main(verbosity=2)

    # Or from command line:
    # python -m pytest tests/regression/ -v
    # python -m pytest tests/regression/test_*_regression_*.py -v --tb=short
