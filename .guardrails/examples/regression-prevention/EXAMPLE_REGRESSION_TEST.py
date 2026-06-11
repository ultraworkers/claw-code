"""
Example Regression Test

This file demonstrates the structure and requirements for regression tests.
Copy this template when creating a new regression test.
"""

import unittest
from src.utils.parser import parse_json_config


class TestParserRegressionFAILabc123de(unittest.TestCase):
    """
    Regression test for FAILURE-ID: FAIL-abc123de

    Original Bug:
        JSON.parse result was accessed without null check, causing TypeError
        when input was invalid JSON or null.

    Impact:
        API endpoint crashed, returning 500 error to users instead of
        graceful error response.

    Fix:
        Added defensive null check and try/catch around JSON.parse.
        Returns null for invalid input instead of crashing.

    Affected Files:
        - src/utils/parser.js
        - src/api/handlers/dataHandler.js
    """

    def test_parse_json_with_invalid_json(self):
        """
        Should handle invalid JSON gracefully without throwing.

        This test would FAIL with the buggy code (throws TypeError)
        and PASSES with the fix (returns None).
        """
        result = parse_json_config("not valid json {{{")
        self.assertIsNone(result)

    def test_parse_json_with_null_input(self):
        """Should handle null input gracefully."""
        result = parse_json_config(None)
        self.assertIsNone(result)

    def test_parse_json_with_empty_string(self):
        """Should handle empty string input."""
        result = parse_json_config("")
        self.assertIsNone(result)

    def test_parse_json_with_valid_json(self):
        """Should work correctly with valid JSON (no regression)."""
        result = parse_json_config('{"key": "value", "number": 42}')
        self.assertIsNotNone(result)
        self.assertEqual(result["key"], "value")
        self.assertEqual(result["number"], 42)

    def test_parse_json_preserves_original_behavior(self):
        """Ensure fix doesn't break valid use cases."""
        test_cases = [
            ('{"a": 1}', {"a": 1}),
            ('[1, 2, 3]', [1, 2, 3]),
            ('"string"', "string"),
            ('123', 123),
            ('true', True),
            ('null', None),
        ]

        for json_str, expected in test_cases:
            with self.subTest(json=json_str):
                result = parse_json_config(json_str)
                self.assertEqual(result, expected)


if __name__ == "__main__":
    unittest.main()
