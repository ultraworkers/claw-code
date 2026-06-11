#!/usr/bin/env python3
"""
Failure Registry CLI Tool
Records bugs and failures to the failure-registry.jsonl file.

Usage:
    python scripts/log_failure.py [options]
    python scripts/log_failure.py --interactive
    python scripts/log_failure.py --from-error "error message" --category runtime

Environment Variables:
    FAILURE_REGISTRY_PATH: Path to registry file (default: .guardrails/failure-registry.jsonl)
"""

import argparse
import fnmatch
import json
import os
import re
import sys
import tempfile
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

DEFAULT_REGISTRY_PATH = Path(".guardrails/failure-registry.jsonl")

CATEGORIES = ["build", "runtime", "test", "type", "lint", "deploy", "config", "regression"]
SEVERITIES = ["low", "medium", "high", "critical"]
STATUSES = ["active", "resolved", "deprecated"]

# Auto-categorization patterns
CATEGORY_PATTERNS = {
    "build": r"(build error|compilation failed|cannot find module|import error|syntax error)",
    "runtime": r"(runtime error|exception|crash|null pointer|undefined|typeerror|referenceerror)",
    "test": r"(test failed|assertion error|expected.*but got|test timeout)",
    "type": r"(type mismatch|type error|incompatible types|cannot assign)",
    "lint": r"(lint error|eslint|pylint|style violation|formatting)",
    "deploy": r"(deployment failed|publish error|release failed|ci/cd error)",
    "config": r"(configuration error|missing config|invalid config|env var)",
}


def generate_failure_id() -> str:
    """Generate a unique failure ID."""
    return f"FAIL-{uuid.uuid4().hex[:8]}"


def detect_category(error_message: str) -> Optional[str]:
    """Auto-detect category from error message."""
    error_lower = error_message.lower()
    for category, pattern in CATEGORY_PATTERNS.items():
        if re.search(pattern, error_lower):
            return category
    return None


def suggest_prevention_rule(category: str, error_message: str) -> str:
    """Suggest a prevention rule based on category and error."""
    suggestions = {
        "build": "Run build checks before committing; verify all imports resolve",
        "runtime": "Add defensive checks and error handling; test edge cases",
        "test": "Run full test suite before committing; verify test coverage",
        "type": "Enable strict type checking; run type checker before commit",
        "lint": "Run linter before committing; fix all lint errors",
        "deploy": "Verify deployment checklist; test in staging first",
        "config": "Validate configuration at startup; use configuration schemas",
    }
    return suggestions.get(category, "Review code carefully; add appropriate safeguards")


def load_registry(registry_path: Path) -> list:
    """Load existing entries from registry."""
    if not registry_path.exists():
        return []

    entries = []
    with open(registry_path, "r") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                try:
                    entries.append(json.loads(line))
                except json.JSONDecodeError:
                    continue
    return entries


def atomic_write(path: Path, content: str) -> None:
    """Atomically write content to file using temp file + rename."""
    temp_fd, temp_path = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.")
    try:
        with os.fdopen(temp_fd, "w") as f:
            f.write(content)
        os.replace(temp_path, path)
    except Exception:
        os.unlink(temp_path)
        raise


def validate_regex(pattern: str) -> bool:
    """Validate that a string is a valid regex pattern."""
    try:
        re.compile(pattern)
        return True
    except re.error:
        return False


def append_to_registry(registry_path: Path, entry: dict) -> None:
    """Append a new entry to the registry."""
    registry_path.parent.mkdir(parents=True, exist_ok=True)

    # Use atomic append for thread safety
    with open(registry_path, "a") as f:
        f.write(json.dumps(entry, separators=(",", ":")) + "\n")
        f.flush()
        os.fsync(f.fileno())


def interactive_prompt() -> dict:
    """Interactive prompt to collect failure information."""
    print("\n" + "=" * 60)
    print("FAILURE REGISTRY - Interactive Entry")
    print("=" * 60 + "\n")

    entry = {
        "failure_id": generate_failure_id(),
        "timestamp": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
    }

    print(f"Generated Failure ID: {entry['failure_id']}")
    print(f"Timestamp: {entry['timestamp']}\n")

    # Category
    print(f"Categories: {', '.join(CATEGORIES)}")
    category = input("Category [runtime]: ").strip() or "runtime"
    while category not in CATEGORIES:
        print(f"Invalid category. Choose from: {', '.join(CATEGORIES)}")
        category = input("Category: ").strip()
    entry["category"] = category

    # Severity
    print(f"\nSeverities: {', '.join(SEVERITIES)}")
    severity = input("Severity [medium]: ").strip() or "medium"
    while severity not in SEVERITIES:
        print(f"Invalid severity. Choose from: {', '.join(SEVERITIES)}")
        severity = input("Severity: ").strip()
    entry["severity"] = severity

    # Error message
    print("\nEnter error message (press Enter twice to finish):")
    error_lines = []
    while True:
        line = input()
        if not line and error_lines:
            break
        error_lines.append(line)
    entry["error_message"] = "\n".join(error_lines) or "Unknown error"

    # Root cause
    print("\nRoot cause analysis (why did this happen?):")
    entry["root_cause"] = input().strip() or "Unknown"

    # Affected files
    print("\nAffected files (comma-separated, relative paths):")
    files_input = input().strip()
    entry["affected_files"] = [f.strip() for f in files_input.split(",") if f.strip()]

    # Fix commit
    print("\nFix commit SHA (optional):")
    fix_commit = input().strip()
    if fix_commit:
        entry["fix_commit"] = fix_commit

    # Regression pattern
    print("\nRegression pattern (regex that would catch reintroduction):")
    pattern = input().strip()
    if pattern:
        entry["regression_pattern"] = pattern

    # Prevention rule
    prevention = suggest_prevention_rule(category, entry["error_message"])
    print(f"\nSuggested prevention rule: {prevention}")
    print("Press Enter to accept or type a custom rule:")
    custom_prevention = input().strip()
    entry["prevention_rule"] = custom_prevention or prevention

    # Status
    entry["status"] = "active"

    return entry


def create_from_args(args) -> dict:
    """Create entry from command line arguments."""
    category = args.category or detect_category(args.error_message) or "runtime"

    entry = {
        "failure_id": generate_failure_id(),
        "timestamp": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "category": category,
        "severity": args.severity or "medium",
        "error_message": args.error_message,
        "root_cause": args.root_cause or "Unknown",
        "status": "active",
    }

    if args.affected_files:
        entry["affected_files"] = args.affected_files
    if args.fix_commit:
        entry["fix_commit"] = args.fix_commit
    if args.regression_pattern:
        if not validate_regex(args.regression_pattern):
            print(f"Warning: Invalid regex pattern: {args.regression_pattern}")
        entry["regression_pattern"] = args.regression_pattern
    if args.prevention_rule:
        entry["prevention_rule"] = args.prevention_rule
    else:
        entry["prevention_rule"] = suggest_prevention_rule(category, args.error_message)

    return entry


def list_failures(registry_path: Path, category: Optional[str] = None, status: Optional[str] = None):
    """List failures from registry with optional filters."""
    entries = load_registry(registry_path)

    if category:
        entries = [e for e in entries if e.get("category") == category]
    if status:
        entries = [e for e in entries if e.get("status") == status]

    if not entries:
        print("No matching failures found.")
        return

    print(f"\n{'ID':<15} {'Category':<12} {'Severity':<8} {'Status':<10} {'Error Preview'}")
    print("-" * 100)
    for e in entries:
        error_preview = e.get("error_message", "")[:40].replace("\n", " ")
        if len(e.get("error_message", "")) > 40:
            error_preview += "..."
        print(f"{e['failure_id']:<15} {e.get('category', '?'):<12} "
              f"{e.get('severity', '?'):<8} {e.get('status', '?'):<10} {error_preview}")


def show_failure(registry_path: Path, failure_id: str):
    """Show detailed information for a specific failure."""
    entries = load_registry(registry_path)
    entry = next((e for e in entries if e.get("failure_id") == failure_id), None)

    if not entry:
        print(f"Failure {failure_id} not found.")
        return

    print("\n" + "=" * 60)
    print(f"Failure: {entry['failure_id']}")
    print("=" * 60)
    for key, value in entry.items():
        if key == "error_message":
            print(f"\n{key}:")
            print(f"  {value}")
        elif isinstance(value, list):
            print(f"\n{key}:")
            for item in value:
                print(f"  - {item}")
        else:
            print(f"{key}: {value}")


def update_status(registry_path: Path, failure_id: str, new_status: str):
    """Update the status of a failure entry."""
    if new_status not in STATUSES:
        print(f"Invalid status. Choose from: {', '.join(STATUSES)}")
        return

    entries = load_registry(registry_path)
    entry = next((e for e in entries if e.get("failure_id") == failure_id), None)

    if not entry:
        print(f"Failure {failure_id} not found.")
        return

    entry["status"] = new_status
    entry["updated_at"] = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")

    # Rewrite entire registry atomically
    content_lines = [
        "# Failure Registry - Append-only log of bugs and failures\n",
        "# Format: One JSON object per line (JSONL)\n",
        "# DO NOT edit existing entries - only append new ones\n",
        "# Use scripts/log_failure.py to add entries consistently\n",
    ]
    for e in entries:
        content_lines.append(json.dumps(e, separators=(",", ":")) + "\n")

    atomic_write(registry_path, "".join(content_lines))

    print(f"Updated {failure_id} status to: {new_status}")


def main():
    parser = argparse.ArgumentParser(
        description="Log failures to the failure registry",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    %(prog)s --interactive                    # Interactive mode
    %(prog)s --from-error "TypeError: ..."   # Quick entry from error
    %(prog)s --list                           # List all failures
    %(prog)s --show FAIL-abc12345             # Show specific failure
    %(prog)s --resolve FAIL-abc12345          # Mark as resolved
        """
    )

    parser.add_argument("--registry", "-r", type=Path,
                        default=Path(os.getenv("FAILURE_REGISTRY_PATH", DEFAULT_REGISTRY_PATH)),
                        help="Path to failure registry file")

    # Modes
    parser.add_argument("--interactive", "-i", action="store_true",
                        help="Interactive prompt mode")
    parser.add_argument("--list", "-l", action="store_true",
                        help="List all failures")
    parser.add_argument("--show", metavar="FAILURE_ID",
                        help="Show detailed info for a failure")
    parser.add_argument("--resolve", metavar="FAILURE_ID",
                        help="Mark a failure as resolved")
    parser.add_argument("--deprecate", metavar="FAILURE_ID",
                        help="Mark a failure as deprecated")

    # Quick entry fields
    parser.add_argument("--from-error", "-e", dest="error_message",
                        help="Error message to log")
    parser.add_argument("--category", "-c", choices=CATEGORIES,
                        help="Failure category")
    parser.add_argument("--severity", "-s", choices=SEVERITIES,
                        help="Severity level")
    parser.add_argument("--root-cause",
                        help="Root cause analysis")
    parser.add_argument("--affected-files", nargs="+",
                        help="List of affected files")
    parser.add_argument("--fix-commit",
                        help="Git SHA of fixing commit")
    parser.add_argument("--regression-pattern",
                        help="Regex pattern that could reintroduce this")
    parser.add_argument("--prevention-rule",
                        help="Rule to prevent recurrence")

    args = parser.parse_args()

    # Handle non-logging modes
    if args.list:
        list_failures(args.registry)
        return
    if args.show:
        show_failure(args.registry, args.show)
        return
    if args.resolve:
        update_status(args.registry, args.resolve, "resolved")
        return
    if args.deprecate:
        update_status(args.registry, args.deprecate, "deprecated")
        return

    # Create entry
    if args.interactive or not args.error_message:
        entry = interactive_prompt()
    else:
        entry = create_from_args(args)

    # Confirm
    print("\n" + "=" * 60)
    print("ENTRY PREVIEW")
    print("=" * 60)
    print(json.dumps(entry, indent=2))
    print("=" * 60)

    confirm = input("\nAdd to registry? [Y/n]: ").strip().lower()
    if confirm in ("", "y", "yes"):
        append_to_registry(args.registry, entry)
        print(f"\n✓ Failure logged: {entry['failure_id']}")
        print(f"  Registry: {args.registry}")
    else:
        print("\n✗ Cancelled")


if __name__ == "__main__":
    main()
