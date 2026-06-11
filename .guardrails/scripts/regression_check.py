#!/usr/bin/env python3
"""
Regression Check Tool
Scans staged/unstaged changes against failure registry to detect potential regressions.

Usage:
    python scripts/regression_check.py              # Check staged changes
    python scripts/regression_check.py --unstaged   # Check unstaged changes
    python scripts/regression_check.py --all        # Check all changes
    python scripts/regression_check.py --pre-commit # Exit with error code if issues found

Environment Variables:
    FAILURE_REGISTRY_PATH: Path to registry file
    PREVENTION_RULES_PATH: Path to prevention rules directory
"""

import argparse
import fnmatch
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import List, Dict, Tuple, Optional

DEFAULT_REGISTRY_PATH = Path(".guardrails/failure-registry.jsonl")
DEFAULT_RULES_PATH = Path(".guardrails/prevention-rules")


def run_git_command(args: List[str]) -> Tuple[int, str, str]:
    """Run a git command and return (returncode, stdout, stderr)."""
    try:
        result = subprocess.run(
            ["git"] + args,
            capture_output=True,
            text=True,
            cwd=Path.cwd()
        )
        return result.returncode, result.stdout, result.stderr
    except FileNotFoundError:
        return 1, "", "git command not found"


def get_changed_files(staged: bool = True, unstaged: bool = False) -> List[str]:
    """Get list of changed files from git."""
    files = []

    if staged:
        rc, stdout, _ = run_git_command(["diff", "--cached", "--name-only"])
        if rc == 0:
            files.extend(stdout.strip().split("\n") if stdout.strip() else [])

    if unstaged:
        rc, stdout, _ = run_git_command(["diff", "--name-only"])
        if rc == 0:
            files.extend(stdout.strip().split("\n") if stdout.strip() else [])

    return list(set(f for f in files if f))


def get_diff_content(file_path: str, staged: bool = True) -> str:
    """Get diff content for a specific file."""
    cmd = ["diff", "--cached"] if staged else ["diff"]
    rc, stdout, _ = run_git_command(cmd + ["--", file_path])
    return stdout if rc in (0, 1) else ""


def load_failure_registry(registry_path: Path) -> List[Dict]:
    """Load failure entries from registry."""
    if not registry_path.exists():
        return []

    entries = []
    with open(registry_path, "r") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                try:
                    entry = json.loads(line)
                    if entry.get("status") == "active":
                        entries.append(entry)
                except json.JSONDecodeError:
                    continue
    return entries


def validate_rule_regex(rule: Dict) -> bool:
    """Validate regex patterns in a rule."""
    pattern = rule.get("pattern", "")
    if pattern:
        try:
            re.compile(pattern)
        except re.error as e:
            print(f"Warning: Invalid regex in rule {rule.get('rule_id')}: {e}")
            return False

    forbidden = rule.get("forbidden_context", "")
    if forbidden:
        try:
            re.compile(forbidden)
        except re.error as e:
            print(f"Warning: Invalid forbidden_context in rule {rule.get('rule_id')}: {e}")
            return False

    return True


def load_prevention_rules(rules_path: Path) -> List[Dict]:
    """Load prevention rules from rules directory."""
    rules = []

    pattern_rules_file = rules_path / "pattern-rules.json"
    if pattern_rules_file.exists():
        try:
            with open(pattern_rules_file, "r") as f:
                data = json.load(f)
                for rule in data.get("rules", []):
                    if rule.get("enabled", True):
                        if validate_rule_regex(rule):
                            rule["rule_type"] = "pattern"
                            rules.append(rule)
        except (json.JSONDecodeError, IOError):
            pass

    semantic_rules_file = rules_path / "semantic-rules.json"
    if semantic_rules_file.exists():
        try:
            with open(semantic_rules_file, "r") as f:
                data = json.load(f)
                for rule in data.get("rules", []):
                    if rule.get("enabled", True):
                        rule["rule_type"] = "semantic"
                        rules.append(rule)
        except (json.JSONDecodeError, IOError):
            pass

    return rules


def check_file_against_failures(
    file_path: str,
    failures: List[Dict]
) -> List[Dict]:
    """Check if file is in affected_files of any active failure."""
    matching_failures = []

    for failure in failures:
        affected_files = failure.get("affected_files", [])
        for affected in affected_files:
            # Use fnmatch for proper glob pattern matching
            if fnmatch.fnmatch(file_path, affected):
                matching_failures.append(failure)
                break

    return matching_failures


def check_diff_against_patterns(
    diff_content: str,
    rules: List[Dict]
) -> List[Dict]:
    """Check diff content against pattern rules."""
    violations = []

    # Extract added lines only (lines starting with +)
    added_lines = []
    for line in diff_content.split("\n"):
        if line.startswith("+") and not line.startswith("+++"):
            added_lines.append(line[1:])  # Remove the + prefix

    added_content = "\n".join(added_lines)

    for rule in rules:
        if rule.get("rule_type") != "pattern":
            continue

        pattern = rule.get("pattern")
        if not pattern:
            continue

        try:
            if re.search(pattern, added_content, re.MULTILINE):
                # Check forbidden context if specified
                forbidden = rule.get("forbidden_context")
                if forbidden and re.search(forbidden, added_content, re.MULTILINE):
                    continue  # Context suggests this is OK

                violations.append({
                    "rule_id": rule.get("rule_id"),
                    "name": rule.get("name"),
                    "message": rule.get("message"),
                    "severity": rule.get("severity", "warning"),
                    "suggestion": rule.get("suggestion"),
                    "failure_id": rule.get("failure_id"),
                })
        except re.error:
            continue  # Invalid regex, skip

    return violations


def format_severity(severity: str) -> str:
    """Format severity with color codes (if terminal supports it)."""
    colors = {
        "critical": "\033[91m",  # Red
        "high": "\033[93m",      # Yellow
        "medium": "\033[94m",    # Blue
        "low": "\033[90m",       # Gray
        "error": "\033[91m",
        "warning": "\033[93m",
    }
    reset = "\033[0m"

    if sys.stdout.isatty():
        return f"{colors.get(severity.lower(), '')}{severity.upper()}{reset}"
    return severity.upper()


def run_regression_check(
    registry_path: Path,
    rules_path: Path,
    staged: bool = True,
    unstaged: bool = False,
    verbose: bool = False
) -> Tuple[int, List[Dict]]:
    """
    Run full regression check.
    Returns (issue_count, issues_details).
    """
    issues = []

    # Load data
    failures = load_failure_registry(registry_path)
    rules = load_prevention_rules(rules_path)

    if verbose:
        print(f"Loaded {len(failures)} active failures, {len(rules)} enabled rules")

    # Get changed files
    changed_files = get_changed_files(staged=staged, unstaged=unstaged)

    if not changed_files:
        if verbose:
            print("No changed files to check")
        return 0, []

    if verbose:
        print(f"Checking {len(changed_files)} changed file(s)...")

    # Check each file
    for file_path in changed_files:
        file_issues = {
            "file": file_path,
            "failures": [],
            "violations": [],
        }

        # Check against failure registry
        matching_failures = check_file_against_failures(file_path, failures)
        if matching_failures:
            file_issues["failures"] = matching_failures

        # Check diff against pattern rules
        diff = get_diff_content(file_path, staged=staged)
        if diff:
            violations = check_diff_against_patterns(diff, rules)
            if violations:
                file_issues["violations"] = violations

        if file_issues["failures"] or file_issues["violations"]:
            issues.append(file_issues)

    return len(issues), issues


def print_report(issues: List[Dict], verbose: bool = False):
    """Print formatted report of issues."""
    if not issues:
        print("\n✓ No potential regressions detected")
        return

    print("\n" + "=" * 70)
    print("REGRESSION CHECK REPORT")
    print("=" * 70)

    for issue in issues:
        file_path = issue["file"]
        print(f"\n📄 {file_path}")
        print("-" * 70)

        # Print matching failures
        for failure in issue["failures"]:
            severity = format_severity(failure.get("severity", "medium"))
            print(f"\n  ⚠️  {severity} - Known Bug History")
            print(f"      Failure ID: {failure['failure_id']}")
            print(f"      Category: {failure.get('category', 'unknown')}")
            print(f"      Previous Error: {failure.get('error_message', 'N/A')[:80]}...")
            print(f"      Prevention: {failure.get('prevention_rule', 'N/A')}")

        # Print pattern violations
        for violation in issue["violations"]:
            severity = format_severity(violation.get("severity", "warning"))
            print(f"\n  🚫 {severity} - Pattern Violation")
            print(f"      Rule: {violation.get('name', 'Unknown')}")
            print(f"      Message: {violation.get('message', 'N/A')}")
            if violation.get("failure_id"):
                print(f"      Related Failure: {violation['failure_id']}")
            if violation.get("suggestion"):
                print(f"      Suggestion: {violation['suggestion']}")

    print("\n" + "=" * 70)
    print(f"Total files with potential issues: {len(issues)}")
    print("=" * 70)
    print("\nReview the above carefully before committing.")


def main():
    parser = argparse.ArgumentParser(
        description="Check for potential regressions in changed code",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    %(prog)s                    # Check staged changes
    %(prog)s --unstaged         # Check unstaged changes
    %(prog)s --all              # Check all changes
    %(prog)s --pre-commit       # Exit with error if issues found
        """
    )

    parser.add_argument("--registry", "-r", type=Path,
                        default=Path(os.getenv("FAILURE_REGISTRY_PATH", DEFAULT_REGISTRY_PATH)),
                        help="Path to failure registry")
    parser.add_argument("--rules", type=Path,
                        default=Path(os.getenv("PREVENTION_RULES_PATH", DEFAULT_RULES_PATH)),
                        help="Path to prevention rules directory")

    # What to check
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--staged", action="store_true", default=True,
                       help="Check staged changes (default)")
    group.add_argument("--unstaged", "-u", action="store_true",
                       help="Check unstaged changes")
    group.add_argument("--all", "-a", action="store_true",
                       help="Check both staged and unstaged changes")

    # Output options
    parser.add_argument("--pre-commit", action="store_true",
                        help="Exit with non-zero code if issues found (for pre-commit hooks)")
    parser.add_argument("--json", action="store_true",
                        help="Output results as JSON")
    parser.add_argument("--verbose", "-v", action="store_true",
                        help="Verbose output")
    parser.add_argument("--quiet", "-q", action="store_true",
                        help="Only output on issues found")

    args = parser.parse_args()

    # Determine what to check
    staged = args.staged and not args.unstaged and not args.all
    unstaged = args.unstaged or args.all
    if args.all:
        staged = True

    # Run check
    count, issues = run_regression_check(
        registry_path=args.registry,
        rules_path=args.rules,
        staged=staged,
        unstaged=unstaged,
        verbose=args.verbose and not args.quiet
    )

    # Output results
    if args.json:
        print(json.dumps({
            "issue_count": count,
            "issues": issues
        }, indent=2))
    elif not args.quiet or count > 0:
        print_report(issues, verbose=args.verbose)

    # Exit code
    if args.pre_commit and count > 0:
        sys.exit(1)
    sys.exit(0)


if __name__ == "__main__":
    main()
