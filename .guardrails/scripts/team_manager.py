#!/usr/bin/env python3
"""
Team Manager - Standardized Team Layout Manager

Manages team assignments, tracks phase progress, and validates
team composition against standardized enterprise layout.
"""

import argparse
import fcntl
import gzip
import json
import os
import re
import sys
import tempfile
import traceback
from dataclasses import dataclass, asdict
from datetime import datetime, timedelta
from pathlib import Path
import time
import statistics
import warnings
from typing import Any, List, Optional, Dict

# FUNC-005: Batch operations support
try:
    from .batch_operations import (
        import_csv, export_csv, import_json, export_json,
        create_csv_template, create_json_template
    )
except ImportError:
    from batch_operations import (
        import_csv, export_csv, import_json, export_json,
        create_csv_template, create_json_template
    )

# SEC-007: Encryption support
try:
    from .encryption import EncryptionManager
except ImportError:
    from encryption import EncryptionManager



class StructuredLogger:
    """JSON structured logging with correlation ID support."""

    def __init__(self, component: str, request_id: Optional[str] = None):
        self.component = component
        self.request_id = request_id

    def log(self, event_type: str, details: Dict, level: str = "info") -> None:
        """Log a structured JSON event."""
        log_entry = {
            "timestamp": datetime.utcnow().isoformat() + "Z",
            "level": level.upper(),
            "component": self.component,
            "event": event_type,
            "details": details
        }
        if self.request_id:
            log_entry["request_id"] = self.request_id
        print(json.dumps(log_entry), file=sys.stderr)

    def info(self, event_type: str, details: Dict) -> None:
        """Log INFO level event."""
        self.log(event_type, details, "info")

    def warn(self, event_type: str, details: Dict) -> None:
        """Log WARN level event."""
        self.log(event_type, details, "warn")

    def error(self, event_type: str, details: Dict, exc_info: bool = False) -> None:
        """Log ERROR level event with optional exception info."""
        if exc_info:
            details = {**details, "stack_trace": traceback.format_exc()}
        self.log(event_type, details, "error")

    def debug(self, event_type: str, details: Dict) -> None:
        """Log DEBUG level event."""
        self.log(event_type, details, "debug")

class RulesLoader:
    """Loads and manages rules from JSON configuration file (FUNC-008).

    Provides dynamic loading of validation rules, team size limits,
    phase gates, and other configurable settings.
    """

    DEFAULT_RULES_PATH = Path(".teams/rules.json")

    def __init__(self, rules_path: Path = None):
        self.rules_path = rules_path or self.DEFAULT_RULES_PATH
        self._rules: Dict[str, Any] = {}
        self._load_rules()

    def _load_rules(self) -> None:
        """Load rules from JSON file or use defaults."""
        if self.rules_path.exists():
            try:
                with open(self.rules_path, 'r') as f:
                    self._rules = json.load(f)
            except (json.JSONDecodeError, IOError) as e:
                print(f"⚠️  Failed to load rules from {self.rules_path}: {e}", file=sys.stderr)
                self._rules = self._get_default_rules()
        else:
            self._rules = self._get_default_rules()

    def _get_default_rules(self) -> Dict[str, Any]:
        """Return default rules when rules.json is not available."""
        return {
            "team_size_limits": {"min": 4, "max": 6},
            "duplicate_detection": {"enabled": True, "scope": "project", "action": "warn"},
            "phase_gates": {},
            "allowed_agent_types": ["planner", "coder", "reviewer", "security", "tester", "ops"],
            "validation_rules": {
                "person_name": {
                    "max_length": 256,
                    "email_pattern": r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$',
                    "username_pattern": r'^[a-zA-Z0-9_.-]+$'
                },
                "role_name": {"max_length": 128},
                "project_name": {"max_length": 64, "pattern": r'^[a-zA-Z0-9_-]+$'}
            }
        }

    def reload_rules(self) -> None:
        """Reload rules from disk."""
        self._load_rules()

    def get(self, key: str, default: Any = None) -> Any:
        """Get a rule by key path (e.g., 'team_size_limits.min')."""
        keys = key.split('.')
        value = self._rules
        for k in keys:
            if isinstance(value, dict) and k in value:
                value = value[k]
            else:
                return default
        return value

    def get_team_size_limits(self) -> tuple:
        """Get min and max team size limits."""
        limits = self._rules.get("team_size_limits", {})
        return (limits.get("min", 4), limits.get("max", 6))

    def get_duplicate_detection_config(self) -> Dict[str, Any]:
        """Get duplicate detection configuration."""
        return self._rules.get("duplicate_detection", {
            "enabled": True,
            "scope": "project",
            "action": "warn"
        })

    def get_validation_pattern(self, rule_type: str, pattern_name: str) -> Optional[str]:
        """Get a validation regex pattern."""
        rules = self._rules.get("validation_rules", {})
        if rule_type in rules:
            return rules[rule_type].get(pattern_name)
        return None

    @property
    def rules(self) -> Dict[str, Any]:
        """Return all loaded rules."""
        return self._rules


class EncryptionManager:
    """Optional encryption at rest for sensitive data (SEC-007).

    Uses Fernet symmetric encryption when TEAM_ENCRYPTION_KEY env var is set.
    Encrypts sensitive fields while keeping structure readable.
    """

    def __init__(self):
        self._key = None
        self._fernet = None
        self._enabled = False
        self._init_encryption()

    def _init_encryption(self) -> None:
        """Initialize encryption from environment key."""
        import base64
        import os

        key = os.environ.get("TEAM_ENCRYPTION_KEY")
        if key:
            try:
                # Ensure key is proper Fernet key (32 bytes, base64 encoded)
                if len(key) == 44:  # Base64 encoded 32 bytes
                    from cryptography.fernet import Fernet
                    self._key = key.encode()
                    self._fernet = Fernet(self._key)
                    self._enabled = True
                else:
                    # Derive key from provided string
                    import hashlib
                    derived = hashlib.sha256(key.encode()).digest()
                    from cryptography.fernet import Fernet
                    self._key = base64.urlsafe_b64encode(derived)
                    self._fernet = Fernet(self._key)
                    self._enabled = True
            except ImportError:
                print("⚠️  cryptography library not installed. Encryption disabled.", file=sys.stderr)
            except Exception as e:
                print(f"⚠️  Failed to initialize encryption: {e}", file=sys.stderr)

    @property
    def enabled(self) -> bool:
        """Check if encryption is enabled."""
        return self._enabled

    def encrypt(self, data: str) -> str:
        """Encrypt a string value."""
        if not self._enabled or not data:
            return data
        try:
            return self._fernet.encrypt(data.encode()).decode()
        except Exception:
            return data

    def decrypt(self, data: str) -> str:
        """Decrypt an encrypted value."""
        if not self._enabled or not data:
            return data
        try:
            return self._fernet.decrypt(data.encode()).decode()
        except Exception:
            return data  # Return as-is if decryption fails

    def encrypt_dict(self, data: Dict, sensitive_fields: List[str]) -> Dict:
        """Encrypt sensitive fields in a dictionary."""
        if not self._enabled:
            return data

        result = data.copy()
        for field in sensitive_fields:
            if field in result and isinstance(result[field], str):
                result[field] = self.encrypt(result[field])
        return result

    def decrypt_dict(self, data: Dict, sensitive_fields: List[str]) -> Dict:
        """Decrypt sensitive fields in a dictionary."""
        if not self._enabled:
            return data

        result = data.copy()
        for field in sensitive_fields:
            if field in result and isinstance(result[field], str):
                result[field] = self.decrypt(result[field])
        return result


class MigrationManager:
    """Manages data migrations between versions (OPS-006).

    Provides automatic migration detection and execution for
    project data files when schema versions change.
    """

    CURRENT_VERSION = "1.0.0"
    MIGRATIONS_DIR = Path("scripts/migrations")

    def __init__(self, project_name: str):
        self.project_name = project_name
        self.config_path = Path(f".teams/{project_name}.json")
        self.migrations: Dict[str, callable] = {}
        self._register_migrations()

    def _register_migrations(self) -> None:
        """Register available migration scripts."""
        # Register migrations from version -> version
        # Each migration should be a callable that takes data dict and returns migrated data
        self.migrations = {
            # Example: "0.9.0": self._migrate_v090_to_v100,
        }

    def get_data_version(self, data: Dict[str, Any]) -> str:
        """Extract version from data dict."""
        return data.get("version", "1.0.0")

    def needs_migration(self, data: Dict[str, Any]) -> bool:
        """Check if data needs migration."""
        data_version = self.get_data_version(data)
        return self._version_compare(data_version, self.CURRENT_VERSION) < 0

    def _version_compare(self, v1: str, v2: str) -> int:
        """Compare two version strings. Returns -1, 0, or 1."""
        def parse_version(v):
            parts = v.split('.')
            return [int(p) for p in parts]

        p1 = parse_version(v1)
        p2 = parse_version(v2)

        for i in range(max(len(p1), len(p2))):
            n1 = p1[i] if i < len(p1) else 0
            n2 = p2[i] if i < len(p2) else 0
            if n1 < n2:
                return -1
            elif n1 > n2:
                return 1
        return 0

    def migrate(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """Migrate data to current version.

        Args:
            data: The data dict to migrate

        Returns:
            Migrated data dict with updated version
        """
        original_version = self.get_data_version(data)
        current_version = original_version

        if not self.needs_migration(data):
            return data

        print(f"🔄 Migrating project '{self.project_name}' from v{original_version} to v{self.CURRENT_VERSION}")

        # Apply migrations in order
        for target_version, migration_func in sorted(
            self.migrations.items(),
            key=lambda x: self._version_compare(x[0], self.CURRENT_VERSION)
        ):
            if self._version_compare(current_version, target_version) < 0:
                print(f"   Applying migration to v{target_version}...")
                try:
                    data = migration_func(data)
                    data["version"] = target_version
                    current_version = target_version
                except Exception as e:
                    print(f"   ❌ Migration failed: {e}")
                    raise

        # Update to final version
        data["version"] = self.CURRENT_VERSION
        data["migrated_from"] = original_version
        data["migrated_at"] = datetime.now().isoformat()

        print(f"✅ Migration complete: v{original_version} -> v{self.CURRENT_VERSION}")
        return data

    def get_migration_status(self) -> Dict[str, Any]:
        """Get migration status for project."""
        if not self.config_path.exists():
            return {"status": "not_found", "current_version": None}

        try:
            with open(self.config_path, 'r') as f:
                data = json.load(f)
            data_version = self.get_data_version(data)
            needs_mig = self._version_compare(data_version, self.CURRENT_VERSION) < 0

            return {
                "status": "needs_migration" if needs_mig else "current",
                "current_version": data_version,
                "target_version": self.CURRENT_VERSION,
                "project": self.project_name
            }
        except Exception as e:
            return {"status": "error", "error": str(e)}


        return self._rules.copy()


# Global rules loader instance (initialized on first use)
_rules_loader: Optional[RulesLoader] = None


def get_rules_loader() -> RulesLoader:
    """Get or create the global rules loader instance."""
    global _rules_loader
    if _rules_loader is None:
        _rules_loader = RulesLoader()
    return _rules_loader


def reload_rules_cmd() -> None:
    """Reload rules from JSON file (FUNC-008) - CLI command."""
    global _rules_loader
    if _rules_loader is None:
        _rules_loader = RulesLoader()
    else:
        _rules_loader.reload_rules()
    print(f"✅ Rules reloaded from {RulesLoader.DEFAULT_RULES_PATH}")

class PerformanceMetrics:
    """Performance metrics collector for team operations (OPS-008).

    Tracks operation duration (init, assign, start, complete).
    Stores metrics in .teams/metrics.json for analysis.
    """

    def __init__(self, project_name: str, metrics_dir: Path = None):
        self.project_name = project_name
        self.metrics_dir = metrics_dir or Path(".teams")
        self.metrics_file = self.metrics_dir / "metrics.json"
        self.metrics_dir.mkdir(parents=True, exist_ok=True)
        self._current_operations: Dict[str, float] = {}

    def start_operation(self, operation: str, **context) -> None:
        """Start timing an operation."""
        start_time = time.time()
        self._current_operations[operation] = start_time
        log_entry = {
            "timestamp": datetime.utcnow().isoformat() + "Z",
            "level": "INFO",
            "component": "performance_metrics",
            "event": "operation_started",
            "details": {"operation": operation, "project": self.project_name, **context}
        }
        print(json.dumps(log_entry), file=sys.stderr)

    def end_operation(self, operation: str, success: bool = True,
                      error_type: Optional[str] = None, **context) -> Dict[str, Any]:
        """End timing an operation and record metrics."""
        end_time = time.time()
        start_time = self._current_operations.pop(operation, end_time)
        duration_ms = (end_time - start_time) * 1000
        metric_entry = {
            "timestamp": datetime.utcnow().isoformat() + "Z",
            "project": self.project_name,
            "operation": operation,
            "duration_ms": round(duration_ms, 2),
            "success": success,
            "context": context
        }
        if error_type:
            metric_entry["error_type"] = error_type
        self._append_metric(metric_entry)
        log_entry = {
            "timestamp": datetime.utcnow().isoformat() + "Z",
            "level": "INFO" if success else "ERROR",
            "component": "performance_metrics",
            "event": "operation_completed",
            "details": metric_entry
        }
        print(json.dumps(log_entry), file=sys.stderr)
        return metric_entry

    def _append_metric(self, metric: Dict[str, Any]) -> None:
        """Append a metric entry to the metrics file."""
        try:
            with open(self.metrics_file, 'a') as f:
                fcntl.flock(f.fileno(), fcntl.LOCK_EX)
                try:
                    f.write(json.dumps(metric) + "\n")
                    f.flush()
                    os.fsync(f.fileno())
                finally:
                    fcntl.flock(f.fileno(), fcntl.LOCK_UN)
        except Exception as e:
            print(f"⚠️  Failed to write metric: {e}", file=sys.stderr)

    def load_metrics(self, since: Optional[datetime] = None,
                     operation: Optional[str] = None) -> List[Dict[str, Any]]:
        """Load metrics from file with optional filtering."""
        if not self.metrics_file.exists():
            return []
        metrics = []
        try:
            with open(self.metrics_file, 'r') as f:
                for line in f:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        entry = json.loads(line)
                        if since:
                            entry_time = datetime.fromisoformat(entry["timestamp"].replace("Z", "+00:00"))
                            if entry_time < since:
                                continue
                        if operation and entry.get("operation") != operation:
                            continue
                        metrics.append(entry)
                    except (json.JSONDecodeError, KeyError):
                        continue
        except Exception as e:
            print(f"⚠️  Failed to load metrics: {e}", file=sys.stderr)
        return metrics

    def get_operation_stats(self, operation: Optional[str] = None,
                            since: Optional[datetime] = None) -> Dict[str, Any]:
        """Get statistics for operations."""
        metrics = self.load_metrics(since=since, operation=operation)
        if not metrics:
            return {"operation": operation or "all", "count": 0, "message": "No metrics found"}
        durations = [m["duration_ms"] for m in metrics if "duration_ms" in m]
        successes = [m for m in metrics if m.get("success", True)]
        failures = [m for m in metrics if not m.get("success", True)]
        stats = {
            "operation": operation or "all",
            "count": len(metrics),
            "success_count": len(successes),
            "failure_count": len(failures),
            "success_rate": round(len(successes) / len(metrics) * 100, 2) if metrics else 0,
            "error_rate": round(len(failures) / len(metrics) * 100, 2) if metrics else 0
        }
        if durations:
            stats["duration_stats"] = {
                "avg_ms": round(statistics.mean(durations), 2),
                "min_ms": round(min(durations), 2),
                "max_ms": round(max(durations), 2),
                "median_ms": round(statistics.median(durations), 2),
            }
            if len(durations) > 1:
                stats["duration_stats"]["stdev_ms"] = round(statistics.stdev(durations), 2)
        sorted_by_duration = sorted(metrics, key=lambda m: m.get("duration_ms", 0), reverse=True)
        stats["slowest_operations"] = [
            {"operation": m["operation"], "duration_ms": m["duration_ms"],
             "timestamp": m["timestamp"], "context": m.get("context", {})}
            for m in sorted_by_duration[:5]
        ]
        return stats

    def get_report(self, days: int = 7) -> Dict[str, Any]:
        """Generate a performance report."""
        since = datetime.utcnow() - timedelta(days=days)
        all_metrics = self.load_metrics(since=since)
        report = {
            "generated_at": datetime.utcnow().isoformat() + "Z",
            "project": self.project_name,
            "time_window_days": days,
            "since": since.isoformat() + "Z",
            "overall": self.get_operation_stats(since=since),
            "by_operation": {}
        }
        operations = set(m.get("operation", "unknown") for m in all_metrics)
        for op in operations:
            report["by_operation"][op] = self.get_operation_stats(operation=op, since=since)
        return report

    def export_report(self, output_path: Path, format: str = "json", days: int = 7) -> bool:
        """Export performance report to file."""
        report = self.get_report(days=days)
        try:
            if format == "json":
                with open(output_path, 'w') as f:
                    json.dump(report, f, indent=2)
            elif format == "csv":
                since = datetime.utcnow() - timedelta(days=days)
                metrics = self.load_metrics(since=since)
                with open(output_path, 'w', newline='') as f:
                    if metrics:
                        writer = csv.DictWriter(f, fieldnames=["timestamp", "project", "operation", "duration_ms", "success", "context"])
                        writer.writeheader()
                        for m in metrics:
                            m_flat = {k: v for k, v in m.items() if k != "error_type"}
                            m_flat["context"] = json.dumps(m_flat.get("context", {}))
                            writer.writerow(m_flat)
            return True
        except Exception as e:
            print(f"❌ Export failed: {e}", file=sys.stderr)
            return False




def validate_project_name(name: str) -> None:
    """Validate project name to prevent command injection."""
    loader = get_rules_loader()
    rules = loader.get("validation_rules.project_name", {})
    max_len = rules.get("max_length", 64)
    pattern = rules.get("pattern", r'^[a-zA-Z0-9_-]+$')

    if not name:
        raise ValueError("project_name is required")
    if len(name) > max_len:
        raise ValueError(f"project_name must be {max_len} characters or less")
    if not re.match(pattern, name):
        raise ValueError("project_name must contain only letters, numbers, hyphens, and underscores")


def validate_project_path(project_name: str, base_dir: str = ".teams") -> Path:
    """Validate project path to prevent path traversal attacks (SEC-006).

    Args:
        project_name: The project name to validate
        base_dir: The base directory for projects (default: .teams)

    Returns:
        Path: The validated and resolved path

    Raises:
        SecurityError: If path traversal is detected
    """
    # First validate the project name format
    validate_project_name(project_name)

    # Check for path traversal patterns in the raw input
    dangerous_patterns = ['..', '/', '\\', '\x00']
    for pattern in dangerous_patterns:
        if pattern in project_name:
            raise SecurityError(
                f"Path traversal detected: project_name contains forbidden pattern '{pattern}'"
            )

    # Construct the intended path
    base_path = Path(base_dir).resolve()
    intended_path = base_path / f"{project_name}.json"

    # Resolve any symlinks and get the real path
    try:
        if intended_path.exists():
            real_path = Path(os.path.realpath(intended_path))
        else:
            # For non-existent paths, resolve the parent and join the filename
            real_parent = Path(os.path.realpath(base_path))
            real_path = real_parent / f"{project_name}.json"
    except (OSError, ValueError) as e:
        raise SecurityError(f"Path resolution failed: {e}")

    # Ensure the resolved path is within the base directory
    try:
        real_path.relative_to(base_path)
    except ValueError:
        raise SecurityError(
            f"Path traversal detected: resolved path '{real_path}' is outside base directory '{base_path}'"
        )

    return real_path


# Valid phases from TEAM_STRUCTURE.md
VALID_PHASES = {
    "Phase 1: Strategy, Governance & Planning",
    "Phase 2: Platform & Foundation",
    "Phase 3: The Build Squads",
    "Phase 4: Validation & Hardening",
    "Phase 5: Delivery & Sustainment",
}

# Valid role names from TEAM_STRUCTURE.md (48 roles across 12 teams)
VALID_ROLES = {
    # Team 1: Business & Product Strategy
    "Business Relationship Manager",
    "Lead Product Manager",
    "Business Systems Analyst",
    "Financial Controller (FinOps)",
    # Team 2: Enterprise Architecture
    "Chief Architect",
    "Domain Architect",
    "Solution Architect",
    "Standards Lead",
    # Team 3: GRC
    "Compliance Officer",
    "Internal Auditor",
    "Privacy Engineer",
    "Policy Manager",
    # Team 4: Infrastructure & Cloud Ops
    "Cloud Architect",
    "IaC Engineer",
    "Network Security Engineer",
    "Storage Engineer",
    # Team 5: Platform Engineering
    "Platform Product Manager",
    "CI/CD Architect",
    "Kubernetes Administrator",
    "Developer Advocate",
    # Team 6: Data Governance & Analytics
    "Data Architect",
    "DBA",
    "Data Privacy Officer",
    "ETL Developer",
    # Team 7: Core Feature Squad
    "Technical Lead",
    "Senior Backend Engineer",
    "Senior Frontend Engineer",
    "Accessibility (A11y) Expert",
    "Technical Writer",
    # Team 8: Middleware & Integration
    "API Product Manager",
    "Integration Engineer",
    "Messaging Engineer",
    "IAM Specialist",
    # Team 9: Cybersecurity
    "Security Architect",
    "Vulnerability Researcher",
    "Penetration Tester",
    "DevSecOps Engineer",
    # Team 10: Quality Engineering
    "QA Architect",
    "SDET",
    "Performance/Load Engineer",
    "Manual QA / UAT Coordinator",
    # Team 11: SRE
    "SRE Lead",
    "Observability Engineer",
    "Chaos Engineer",
    "Incident Manager",
    # Team 12: IT Operations & Support
    "NOC Analyst",
    "Change Manager",
    "Release Manager",
    "L3 Support Engineer",
}


def get_valid_phases() -> set:
    """Get valid phases from rules or defaults."""
    loader = get_rules_loader()
    phase_gates = loader.get("phase_gates", {})
    if phase_gates:
        return set(phase_gates.keys())
    # Default phases if not configured
    return {
        "Phase 1: Strategy, Governance & Planning",
        "Phase 2: Platform & Foundation",
        "Phase 3: The Build Squads",
        "Phase 4: Validation & Hardening",
        "Phase 5: Delivery & Sustainment",
    }


def validate_phase(phase: str) -> None:
    """Validate phase name against valid phases.

    Args:
        phase: Phase name to validate

    Raises:
        ValueError: If phase is not a valid phase name
    """
    valid_phases = get_valid_phases()
    if not phase:
        raise ValueError("phase is required")
    if phase not in valid_phases:
        raise ValueError(
            f"Invalid phase: '{phase}'. Must be one of: "
            f"{', '.join(sorted(valid_phases))}"
        )


def validate_role_name(role_name: str) -> None:
    """Validate role name against valid roles from TEAM_STRUCTURE.md.

    Args:
        role_name: Role name to validate

    Raises:
        ValueError: If role_name is not a valid role
    """
    if not role_name:
        raise ValueError("role_name is required")
    if len(role_name) > 128:
        raise ValueError("role_name must be 128 characters or less")
    # Check for control characters
    if re.search(r'[\x00-\x1f\x7f]', role_name):
        raise ValueError("role_name contains invalid control characters")
    if role_name not in VALID_ROLES:
        raise ValueError(
            f"Invalid role_name: '{role_name}'. Must be one of the 48 defined roles. "
            f"See TEAM_STRUCTURE.md for valid role definitions."
        )


def validate_person_name(person: str) -> None:
    """Validate person/assignee name format.

    Accepts email addresses, usernames, or display names with alphanumeric
    characters, spaces, hyphens, underscores, dots, and apostrophes.

    Args:
        person: Person name/identifier to validate

    Raises:
        ValueError: If person format is invalid
    """
    loader = get_rules_loader()
    rules = loader.get("validation_rules.person_name", {})
    max_len = rules.get("max_length", 256)
    email_pattern = rules.get("email_pattern", r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$')
    username_pattern = rules.get("username_pattern", r'^[a-zA-Z0-9_.-]+$')
    # Display names allow spaces and apostrophes (e.g., "Alice Smith", "O'Connor")
    display_name_pattern = r'^[a-zA-Z0-9_.\-\' ]+$'

    if not person:
        raise ValueError("person is required")
    if len(person) > max_len:
        raise ValueError(f"person must be {max_len} characters or less")
    # Check for control characters
    if re.search(r'[\x00-\x1f\x7f]', person):
        raise ValueError("person contains invalid control characters")
    # Check for dangerous patterns
    dangerous_patterns = [";", "|", "&&", "||", "`", "$", "<", ">", "..", "\\"]
    for pattern in dangerous_patterns:
        if pattern in person:
            raise ValueError(f"person contains forbidden pattern: {pattern}")
    # Allow email format, username format, or display name format
    if not re.match(email_pattern, person) and not re.match(username_pattern, person) and not re.match(display_name_pattern, person):
        raise ValueError(
            f"Invalid person format: '{person}'. "
            f"Must be a valid email address, username, or display name"
        )


class PermissionDenied(Exception):
    """Raised when user lacks permission for an operation."""
    pass


class FileLockError(Exception):
    """Raised when file locking fails."""
    pass


class SecurityError(Exception):
    """Raised when a security violation is detected."""
    pass


class RateLimitExceeded(Exception):
    """Raised when rate limit is exceeded."""
    def __init__(self, message: str, retry_after: int = None):
        super().__init__(message)
        self.retry_after = retry_after


class RateLimiter:
    """Token bucket rate limiter for API requests (SEC-005).

    Implements per-user rate limiting with configurable limits.
    Stores state in memory with automatic cleanup.
    """

    DEFAULT_REQUESTS = 100
    DEFAULT_WINDOW = 60  # seconds
    CLEANUP_INTERVAL = 300  # cleanup every 5 minutes

    def __init__(self, config_path: Path = None):
        self.config_path = config_path or Path(".teams/config.json")
        self._buckets: Dict[str, Dict] = {}
        self._last_cleanup = time.time()
        self._load_config()

    def _load_config(self) -> None:
        """Load rate limit configuration from config file."""
        self.enabled = True
        self.requests_per_window = self.DEFAULT_REQUESTS
        self.window_seconds = self.DEFAULT_WINDOW

        if self.config_path.exists():
            try:
                with open(self.config_path, 'r') as f:
                    config = json.load(f)
                rate_config = config.get("rate_limiting", {})
                self.enabled = rate_config.get("enabled", True)
                self.requests_per_window = rate_config.get("requests_per_window", self.DEFAULT_REQUESTS)
                self.window_seconds = rate_config.get("window_seconds", self.DEFAULT_WINDOW)
            except (json.JSONDecodeError, IOError):
                pass  # Use defaults

    def _cleanup_old_buckets(self) -> None:
        """Remove expired buckets to prevent memory leaks."""
        now = time.time()
        if now - self._last_cleanup < self.CLEANUP_INTERVAL:
            return

        expired = []
        for user_id, bucket in self._buckets.items():
            if now - bucket.get("last_reset", 0) > self.window_seconds * 2:
                expired.append(user_id)

        for user_id in expired:
            del self._buckets[user_id]

        self._last_cleanup = now

    def check_rate_limit(self, user_id: str = "default") -> tuple[bool, dict]:
        """Check if request is within rate limit.

        Args:
            user_id: Unique identifier for the user (default: "default")

        Returns:
            Tuple of (allowed, rate_limit_info)
            rate_limit_info contains: limit, remaining, reset_time
        """
        if not self.enabled:
            return True, {"limit": -1, "remaining": -1, "reset_time": 0}

        self._cleanup_old_buckets()

        now = time.time()

        if user_id not in self._buckets:
            self._buckets[user_id] = {
                "tokens": self.requests_per_window,
                "last_reset": now
            }

        bucket = self._buckets[user_id]

        # Check if window has passed and reset
        if now - bucket["last_reset"] >= self.window_seconds:
            bucket["tokens"] = self.requests_per_window
            bucket["last_reset"] = now

        # Calculate remaining time until reset
        reset_time = int(bucket["last_reset"] + self.window_seconds)
        remaining = max(0, bucket["tokens"] - 1)

        rate_limit_info = {
            "limit": self.requests_per_window,
            "remaining": remaining,
            "reset_time": reset_time
        }

        # Check if token available
        if bucket["tokens"] <= 0:
            return False, rate_limit_info

        # Consume token
        bucket["tokens"] -= 1
        return True, rate_limit_info

    def get_rate_limit_headers(self, user_id: str = "default") -> Dict[str, str]:
        """Get rate limit headers for response.

        Args:
            user_id: Unique identifier for the user

        Returns:
            Dict of HTTP headers for rate limiting
        """
        if not self.enabled or user_id not in self._buckets:
            return {}

        bucket = self._buckets[user_id]
        remaining = max(0, bucket["tokens"])
        reset_time = int(bucket["last_reset"] + self.window_seconds)

        return {
            "X-RateLimit-Limit": str(self.requests_per_window),
            "X-RateLimit-Remaining": str(remaining),
            "X-RateLimit-Reset": str(reset_time)
        }


class BackupManager:
    """Manages automatic backups of team configurations.

    Implements OPS-004: Automated backup before writes with versioning.
    Keeps last N versions and stores in .teams/backups/
    """

    DEFAULT_MAX_BACKUPS = 10

    def __init__(self, project_name: str, backup_dir: Path = None, max_backups: int = None):
        self.project_name = project_name
        self.backup_dir = backup_dir or Path(".teams/backups")
        self.max_backups = max_backups or self.DEFAULT_MAX_BACKUPS
        self.backup_dir.mkdir(parents=True, exist_ok=True)

    def _get_backup_path(self, timestamp: str = None) -> Path:
        """Generate backup file path with timestamp."""
        ts = timestamp or datetime.now().strftime("%Y%m%d_%H%M%S")
        # SEC-006: Sanitize project name for filename to prevent path traversal
        safe_name = re.sub(r'[^a-zA-Z0-9_-]', '_', self.project_name)
        return self.backup_dir / f"{safe_name}_{ts}.json.gz"

    def create_backup(self, config_path: Path) -> Optional[Path]:
        """Create a backup of the current configuration.

        Args:
            config_path: Path to the configuration file to backup

        Returns:
            Path to the backup file, or None if no file exists to backup
        """
        if not config_path.exists():
            return None

        # Generate timestamp for this backup
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S_%f")
        backup_path = self._get_backup_path(timestamp)

        # Copy and compress the file
        try:
            import gzip
            with open(config_path, 'rb') as src:
                with gzip.open(backup_path, 'wb') as dst:
                    dst.write(src.read())

            # Clean up old backups
            self._cleanup_old_backups()

            return backup_path
        except Exception as e:
            # If backup fails, log but don't block the save
            print(f"⚠️  Backup creation failed: {e}", file=sys.stderr)
            return None

    def _cleanup_old_backups(self) -> None:
        """Remove oldest backups keeping only max_backups versions."""
        try:
            backups = sorted(
                self.backup_dir.glob(f"{self.project_name}_*.json.gz"),
                key=lambda p: p.stat().st_mtime
            )

            while len(backups) > self.max_backups:
                oldest = backups.pop(0)
                try:
                    oldest.unlink()
                    print(f"🗑️  Removed old backup: {oldest.name}", file=sys.stderr)
                except OSError:
                    pass
        except Exception:
            pass

    def list_backups(self) -> List[Dict[str, Any]]:
        """List all available backups for this project.

        Returns:
            List of dicts with backup info: path, timestamp, size
        """
        backups = []
        for backup_file in sorted(self.backup_dir.glob(f"{self.project_name}_*.json.gz"), reverse=True):
            try:
                stat = backup_file.stat()
                # Extract timestamp from filename
                timestamp_str = backup_file.stem.replace(f"{self.project_name}_", "")
                backups.append({
                    "path": str(backup_file),
                    "filename": backup_file.name,
                    "timestamp": timestamp_str,
                    "size_bytes": stat.st_size,
                    "created_at": datetime.fromtimestamp(stat.st_mtime).isoformat()
                })
            except OSError:
                continue
        return backups

    def restore_backup(self, backup_path: Path, target_path: Path) -> bool:
        """Restore a backup to the target path.

        Args:
            backup_path: Path to the backup file
            target_path: Path to restore to

        Returns:
            True if successful, False otherwise
        """
        try:
            import gzip
            with gzip.open(backup_path, 'rb') as src:
                content = src.read()

            # Atomic restore: write to temp then rename
            fd, temp_path = tempfile.mkstemp(
                dir=target_path.parent,
                prefix=f".{self.project_name}.restore.tmp."
            )
            try:
                with os.fdopen(fd, 'wb') as f:
                    f.write(content)
                    f.flush()
                    os.fsync(f.fileno())
                os.replace(temp_path, target_path)
                return True
            except Exception:
                try:
                    os.unlink(temp_path)
                except FileNotFoundError:
                    pass
                raise
        except Exception as e:
            print(f"❌ Restore failed: {e}", file=sys.stderr)
            return False


class AuditLogger:
    """Audit logging for team operations (SEC-008).

    Logs all team modifications with user, timestamp, action, and before/after state.
    Stores in .teams/audit.log
    """

    def __init__(self, project_name: str, audit_dir: Path = None):
        self.project_name = project_name
        self.audit_dir = audit_dir or Path(".teams")
        self.audit_file = self.audit_dir / "audit.log"
        self.audit_dir.mkdir(parents=True, exist_ok=True)

    def _get_user_id(self, user_context: Optional["UserContext"]) -> str:
        """Extract user ID from context or return 'system'."""
        if user_context:
            return user_context.user_id
        return "system"

    def log_action(self, action: str, details: Dict[str, Any], user_context: Optional["UserContext"] = None) -> None:
        """Log an audit action.

        Args:
            action: The action performed (e.g., 'assign_role', 'start_team')
            details: Dict containing before/after state and other details
            user_context: Optional user context for RBAC info
        """
        entry = {
            "timestamp": datetime.utcnow().isoformat() + "Z",
            "project": self.project_name,
            "user": self._get_user_id(user_context),
            "role": user_context.role if user_context else "system",
            "action": action,
            "details": details
        }

        try:
            with open(self.audit_file, 'a') as f:
                fcntl.flock(f.fileno(), fcntl.LOCK_EX)
                try:
                    f.write(json.dumps(entry) + "\n")
                    f.flush()
                    os.fsync(f.fileno())
                finally:
                    fcntl.flock(f.fileno(), fcntl.LOCK_UN)
        except Exception as e:
            print(f"⚠️  Audit logging failed: {e}", file=sys.stderr)

    def query_audit_log(self, start_time: Optional[datetime] = None,
                        end_time: Optional[datetime] = None,
                        user: Optional[str] = None,
                        action: Optional[str] = None,
                        team_id: Optional[int] = None,
                        limit: int = 100) -> List[Dict[str, Any]]:
        """Query the audit log with filters.

        Args:
            start_time: Optional start time filter
            end_time: Optional end time filter
            user: Optional user filter
            action: Optional action filter
            team_id: Optional team_id filter
            limit: Maximum number of entries to return

        Returns:
            List of matching audit entries
        """
        if not self.audit_file.exists():
            return []

        results = []
        try:
            with open(self.audit_file, 'r') as f:
                for line in f:
                    if not line.strip():
                        continue
                    try:
                        entry = json.loads(line)

                        # Apply filters
                        if start_time:
                            entry_time = datetime.fromisoformat(entry["timestamp"].replace("Z", "+00:00"))
                            if entry_time < start_time:
                                continue
                        if end_time:
                            entry_time = datetime.fromisoformat(entry["timestamp"].replace("Z", "+00:00"))
                            if entry_time > end_time:
                                continue
                        if user and entry.get("user") != user:
                            continue
                        if action and entry.get("action") != action:
                            continue
                        if team_id is not None:
                            entry_team_id = entry.get("details", {}).get("team_id")
                            if entry_team_id != team_id:
                                continue

                        results.append(entry)

                        if len(results) >= limit:
                            break
                    except json.JSONDecodeError:
                        continue
        except Exception as e:
            print(f"⚠️  Audit query failed: {e}", file=sys.stderr)

        return results

    def get_recent_actions(self, count: int = 10) -> List[Dict[str, Any]]:
        """Get the most recent audit actions.

        Args:
            count: Number of entries to return

        Returns:
            List of recent audit entries
        """
        return self.query_audit_log(limit=count)


class UserContext:
    """User session context with RBAC information."""

    # Role hierarchy: higher number = more permissions
    ROLE_LEVELS = {
        "viewer": 1,      # Can view only
        "team-lead": 2,   # Can modify their team's assignments
        "admin": 3        # Can modify everything
    }

    def __init__(self, user_id: str, role: str, team_id: Optional[int] = None):
        self.user_id = user_id
        self.role = role
        self.team_id = team_id

        if role not in self.ROLE_LEVELS:
            raise ValueError(f"Invalid role: {role}. Must be one of: {list(self.ROLE_LEVELS.keys())}")

    def has_permission(self, required_role: str) -> bool:
        """Check if user has at least the required role level."""
        return self.ROLE_LEVELS.get(self.role, 0) >= self.ROLE_LEVELS.get(required_role, 0)

    def can_modify_team(self, team_id: int) -> bool:
        """Check if user can modify a specific team."""
        if self.role == "admin":
            return True
        if self.role == "team-lead" and self.team_id == team_id:
            return True
        return False


class FileLock:
    """Cross-platform file locking using flock (Unix) or msvcrt (Windows)."""

    def __init__(self, lock_file_path: Path, timeout: float = 30.0):
        self.lock_file_path = lock_file_path
        self.timeout = timeout
        self.lock_file = None

    def __enter__(self):
        """Acquire exclusive lock."""
        self.lock_file_path.parent.mkdir(parents=True, exist_ok=True)
        self.lock_file = open(self.lock_file_path, 'w')

        try:
            # Use non-blocking flock first
            fcntl.flock(self.lock_file.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except (IOError, OSError):
            # Lock is held by another process
            import time
            start_time = time.time()
            while time.time() - start_time < self.timeout:
                try:
                    fcntl.flock(self.lock_file.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                    break
                except (IOError, OSError):
                    time.sleep(0.1)
            else:
                self.lock_file.close()
                raise FileLockError(f"Could not acquire lock within {self.timeout}s")

        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Release lock and close file."""
        if self.lock_file:
            try:
                fcntl.flock(self.lock_file.fileno(), fcntl.LOCK_UN)
            except (IOError, OSError):
                pass
            finally:
                self.lock_file.close()


@dataclass
class Role:
    """Standard team role."""
    name: str
    responsibility: str
    deliverables: List[str]
    assigned_to: Optional[str] = None


@dataclass
class Team:
    """Standard team definition."""
    id: int
    name: str
    phase: str
    description: str
    roles: List[Role]
    exit_criteria: List[str]
    status: str = "not_started"  # not_started, active, completed, blocked
    started_at: Optional[str] = None
    completed_at: Optional[str] = None


class TeamManager:
    """Manages standardized team layout."""

    # Standard team definitions
    STANDARD_TEAMS = {
        # Phase 1: Strategy, Governance & Planning
        1: Team(
            id=1,
            name="Business & Product Strategy",
            phase="Phase 1: Strategy, Governance & Planning",
            description="The 'Why' - Business case and product strategy",
            roles=[
                Role("Business Relationship Manager", "Connects IT to C-suite",
                     ["Strategic alignment docs", "Executive briefings"]),
                Role("Lead Product Manager", "Owns long-term roadmap",
                     ["Product roadmap", "OKRs", "Feature prioritization"]),
                Role("Business Systems Analyst", "Translates business to technical",
                     ["Requirements specs", "User stories", "Acceptance criteria"]),
                Role("Financial Controller (FinOps)", "Approves budget and cloud spend",
                     ["Budget forecasts", "Cost projections", "Spend reports"]),
            ],
            exit_criteria=[
                "Business case approved",
                "Budget allocated",
                "Roadmap defined",
                "Success metrics established"
            ]
        ),
        2: Team(
            id=2,
            name="Enterprise Architecture",
            phase="Phase 1: Strategy, Governance & Planning",
            description="The 'Standards' - Technology vision and standards",
            roles=[
                Role("Chief Architect", "Sets 5-year tech vision",
                     ["Architecture vision", "Tech radar", "Strategic plans"]),
                Role("Domain Architect", "Specialized stack expertise",
                     ["Domain-specific patterns", "Best practices guides"]),
                Role("Solution Architect", "Maps projects to standards",
                     ["Solution designs", "Architecture decision records"]),
                Role("Standards Lead", "Manages Approved Tech List",
                     ["Technology standards", "Evaluation criteria", "Approved list"]),
            ],
            exit_criteria=[
                "Architecture approved",
                "Technology choices validated",
                "Standards compliance verified"
            ]
        ),
        3: Team(
            id=3,
            name="GRC (Governance, Risk, & Compliance)",
            phase="Phase 1: Strategy, Governance & Planning",
            description="Compliance and risk management",
            roles=[
                Role("Compliance Officer", "SOX/HIPAA/GDPR adherence",
                     ["Compliance checklists", "Audit reports"]),
                Role("Internal Auditor", "Pre-production mock audits",
                     ["Audit findings", "Remediation plans"]),
                Role("Privacy Engineer", "Data masking and PII",
                     ["Privacy impact assessments", "Data flow diagrams"]),
                Role("Policy Manager", "Maintains SOPs",
                     ["Standard operating procedures", "Policy updates"]),
            ],
            exit_criteria=[
                "Compliance review passed",
                "Risk assessment complete",
                "Privacy requirements met",
                "Policies acknowledged"
            ]
        ),
        # Phase 2: Platform & Foundation
        4: Team(
            id=4,
            name="Infrastructure & Cloud Ops",
            phase="Phase 2: Platform & Foundation",
            description="Cloud infrastructure and networking",
            roles=[
                Role("Cloud Architect", "VPC and network design",
                     ["Network diagrams", "Security groups", "Routing tables"]),
                Role("IaC Engineer", "Provisions the 'metal'",
                     ["Terraform modules", "Ansible playbooks", "Infrastructure code"]),
                Role("Network Security Engineer", "Firewalls, VPNs, Direct Connect",
                     ["Security rules", "Network policies", "Access controls"]),
                Role("Storage Engineer", "S3/SAN management",
                     ["Storage policies", "Backup strategies", "Archival rules"]),
            ],
            exit_criteria=[
                "Infrastructure provisioned",
                "Network connectivity verified",
                "Security rules applied",
                "Monitoring enabled"
            ]
        ),
        5: Team(
            id=5,
            name="Platform Engineering",
            phase="Phase 2: Platform & Foundation",
            description="The 'Internal Tools' - Developer experience platform",
            roles=[
                Role("Platform Product Manager", "Developer experience as product",
                     ["Platform roadmap", "DX metrics", "Adoption reports"]),
                Role("CI/CD Architect", "Golden pipelines",
                     ["Pipeline templates", "Build configs", "Deployment strategies"]),
                Role("Kubernetes Administrator", "Cluster management",
                     ["Cluster configs", "Resource quotas", "Ingress rules"]),
                Role("Developer Advocate", "Dev squad adoption",
                     ["Onboarding guides", "Training materials", "Feedback loops"]),
            ],
            exit_criteria=[
                "Platform services ready",
                "CI/CD pipelines functional",
                "Developer onboarding complete"
            ]
        ),
        6: Team(
            id=6,
            name="Data Governance & Analytics",
            phase="Phase 2: Platform & Foundation",
            description="Enterprise data management",
            roles=[
                Role("Data Architect", "Enterprise data model",
                     ["Data models", "Schema designs", "Lineage documentation"]),
                Role("DBA", "Production database performance",
                     ["Query optimization", "Index tuning", "Backup verification"]),
                Role("Data Privacy Officer", "Retention and deletion rules",
                     ["Data retention policies", "Deletion workflows"]),
                Role("ETL Developer", "Data flow management",
                     ["ETL pipelines", "Data quality checks", "Transformation logic"]),
            ],
            exit_criteria=[
                "Data models defined",
                "Pipelines operational",
                "Privacy controls implemented"
            ]
        ),
        # Phase 3: The Build Squads
        7: Team(
            id=7,
            name="Core Feature Squad",
            phase="Phase 3: The Build Squads",
            description="The 'Devs' - Feature implementation",
            roles=[
                Role("Technical Lead", "Final word on implementation",
                     ["Code reviews", "Architecture decisions", "Technical guidance"]),
                Role("Senior Backend Engineer", "Logic, APIs, microservices",
                     ["Backend services", "API endpoints", "Business logic"]),
                Role("Senior Frontend Engineer", "Design system, state management",
                     ["UI components", "Frontend architecture", "State logic"]),
                Role("Accessibility (A11y) Expert", "WCAG compliance",
                     ["A11y audits", "Remediation plans", "Testing reports"]),
                Role("Technical Writer", "Internal/external docs",
                     ["API docs", "User guides", "Runbooks"]),
            ],
            exit_criteria=[
                "Features implemented",
                "Code reviewed and approved",
                "Documentation complete",
                "A11y requirements met"
            ]
        ),
        8: Team(
            id=8,
            name="Middleware & Integration",
            phase="Phase 3: The Build Squads",
            description="APIs and system integrations",
            roles=[
                Role("API Product Manager", "API lifecycle and versioning",
                     ["API specs", "Versioning strategy", "Deprecation plans"]),
                Role("Integration Engineer", "SAP/Oracle/Mainframe connections",
                     ["Integration specs", "Data mappings", "Error handling"]),
                Role("Messaging Engineer", "Kafka/RabbitMQ management",
                     ["Topic design", "Message schemas", "Consumer groups"]),
                Role("IAM Specialist", "Okta/AD integration",
                     ["Auth flows", "Permission models", "Access policies"]),
            ],
            exit_criteria=[
                "APIs documented and tested",
                "Integrations verified",
                "Auth flows functional"
            ]
        ),
        # Phase 4: Validation & Hardening
        9: Team(
            id=9,
            name="Cybersecurity (AppSec)",
            phase="Phase 4: Validation & Hardening",
            description="Application security",
            roles=[
                Role("Security Architect", "Threat model review",
                     ["Threat models", "Security architecture", "Risk assessments"]),
                Role("Vulnerability Researcher", "SAST/DAST/SCA scanners",
                     ["Scan reports", "Vulnerability triage", "Fix verification"]),
                Role("Penetration Tester", "Manual security testing",
                     ["Pen test reports", "Exploit verification", "Remediation"]),
                Role("DevSecOps Engineer", "Security in CI/CD",
                     ["Security gates", "Pipeline integration", "Compliance checks"]),
            ],
            exit_criteria=[
                "Security review passed",
                "Vulnerabilities remediated or accepted",
                "Pen testing complete",
                "Security gates passing"
            ]
        ),
        10: Team(
            id=10,
            name="Quality Engineering (SDET)",
            phase="Phase 4: Validation & Hardening",
            description="Testing and quality assurance",
            roles=[
                Role("QA Architect", "Global testing strategy",
                     ["Test strategy", "Test plans", "Coverage reports"]),
                Role("SDET", "Automated test code",
                     ["Test automation", "Framework maintenance", "CI integration"]),
                Role("Performance/Load Engineer", "Scale testing",
                     ["Load test scripts", "Performance baselines", "Capacity reports"]),
                Role("Manual QA / UAT Coordinator", "User acceptance testing",
                     ["Test cases", "UAT coordination", "Sign-off reports"]),
            ],
            exit_criteria=[
                "Test coverage requirements met",
                "Performance benchmarks achieved",
                "UAT sign-off obtained"
            ]
        ),
        # Phase 5: Delivery & Sustainment
        11: Team(
            id=11,
            name="Site Reliability Engineering (SRE)",
            phase="Phase 5: Delivery & Sustainment",
            description="Reliability and observability",
            roles=[
                Role("SRE Lead", "Error budget and uptime SLA",
                     ["SLOs", "Error budgets", "Reliability reports"]),
                Role("Observability Engineer", "Monitoring and logging",
                     ["Dashboards", "Alerts", "Log aggregation", "Traces"]),
                Role("Chaos Engineer", "Resiliency testing",
                     ["Chaos experiments", "Failure scenarios", "Recovery tests"]),
                Role("Incident Manager", "War room leadership",
                     ["Incident response", "Post-mortems", "Runbook updates"]),
            ],
            exit_criteria=[
                "Monitoring in place",
                "Alerts configured",
                "Runbooks complete",
                "Error budget healthy"
            ]
        ),
        12: Team(
            id=12,
            name="IT Operations & Support (NOC)",
            phase="Phase 5: Delivery & Sustainment",
            description="Production operations",
            roles=[
                Role("NOC Analyst", "24/7 monitoring",
                     ["Monitoring dashboards", "Alert triage", "Incident tickets"]),
                Role("Change Manager", "Deployment approval",
                     ["Change requests", "Deployment windows", "CAB approval"]),
                Role("Release Manager", "Go/No-Go coordination",
                     ["Release plans", "Rollback procedures", "Coordination"]),
                Role("L3 Support Engineer", "Production bug escalation",
                     ["Root cause analysis", "Hotfix coordination", "KB articles"]),
            ],
            exit_criteria=[
                "Change approved",
                "Release deployed",
                "Support handoff complete"
            ]
        ),
    }

    def __init__(self, project_name: str, config_path: Path = None, user_context: Optional["UserContext"] = None, logger: Optional[StructuredLogger] = None, enable_backup: bool = True, enable_audit: bool = True, max_backups: int = 10, test_mode: bool = False):
        # SEC-006: Validate project name and path to prevent path traversal
        self.project_name = project_name
        self.teams: Dict[int, Team] = {}

        # Validate and resolve the config path
        if config_path is not None:
            self.config_path = config_path
        else:
            self.config_path = validate_project_path(project_name)

        # Validate lock file path is within .teams/
        self.lock_path = validate_project_path(project_name).with_suffix(".lock")

        self.user_context = user_context
        self.logger = logger or StructuredLogger("team_manager")
        self.test_mode = test_mode

        # OPS-004: Backup manager
        self.enable_backup = enable_backup
        if enable_backup:
            self.backup_manager = BackupManager(project_name, max_backups=max_backups)
        else:
            self.backup_manager = None

        # SEC-008: Audit logger
        self.enable_audit = enable_audit
        if enable_audit:
            self.audit_logger = AuditLogger(project_name)
        else:
            self.audit_logger = None

        # OPS-006: Migration manager
        self.migration_manager = MigrationManager(project_name)

        # OPS-005: Version tracking
        self._data_version = "1.0.0"

        # OPS-008: Performance metrics
        self.performance_metrics = PerformanceMetrics(project_name)

        # SEC-005: Rate limiter (lazy initialization)
        self._rate_limiter: Optional[RateLimiter] = None

        # SEC-007: Encryption manager
        self.encryption_manager = EncryptionManager()

    def _check_rate_limit(self, user_id: str = "default") -> tuple[bool, dict]:
        """Check rate limit for the current user (SEC-005).

        Args:
            user_id: Unique identifier for the user

        Returns:
            Tuple of (allowed, rate_limit_info)
        """
        if self._rate_limiter is None:
            self._rate_limiter = RateLimiter()
        return self._rate_limiter.check_rate_limit(user_id)

    def _get_rate_limit_headers(self, user_id: str = "default") -> Dict[str, str]:
        """Get rate limit headers for response (SEC-005)."""
        if self._rate_limiter is None:
            return {}
        return self._rate_limiter.get_rate_limit_headers(user_id)

    def _require_auth(self, operation: str, team_id: Optional[int] = None) -> None:
        """Check if user is authorized for the operation."""
        # Skip auth checks in test mode
        if self.test_mode:
            return

        if self.user_context is None:
            raise PermissionDenied(f"Authentication required for {operation}")

        if not self.user_context.has_permission("team-lead"):
            raise PermissionDenied(
                f"User '{self.user_context.user_id}' with role '{self.user_context.role}' "
                f"does not have permission to {operation}"
            )

        if team_id is not None and not self.user_context.can_modify_team(team_id):
            raise PermissionDenied(
                f"User '{self.user_context.user_id}' cannot modify team {team_id}. "
                f"Requires admin role or team-lead for this specific team."
            )

    def initialize_project(self) -> None:
        """Initialize a new project with all teams.

        Requires admin role.
        """
        self.performance_metrics.start_operation("init", project=self.project_name)
        try:
            self._require_auth("initialize project")
            self.teams = {team_id: team for team_id, team in self.STANDARD_TEAMS.items()}
            self.save()
            print(f"✅ Initialized project '{self.project_name}' with {len(self.teams)} teams")
            self.performance_metrics.end_operation("init", success=True, team_count=len(self.teams))
        except Exception as e:
            self.performance_metrics.end_operation("init", success=False, error_type=type(e).__name__)
            raise

    def load(self) -> bool:
        """Load team configuration from disk with file locking.

        Uses shared lock to allow concurrent reads while preventing
        reads during writes.
        """
        if not self.config_path.exists():
            return False

        # Create lock file if it doesn't exist
        self.lock_path.parent.mkdir(parents=True, exist_ok=True)

        with FileLock(self.lock_path):
            with open(self.config_path, 'r') as f:
                # Use shared lock for reads
                fcntl.flock(f.fileno(), fcntl.LOCK_SH)
                try:
                    data = json.load(f)
                finally:
                    fcntl.flock(f.fileno(), fcntl.LOCK_UN)

        # SEC-007: Decrypt data if encryption is detected and enabled
        if self.encryption_manager.enabled and self.encryption_manager.is_encrypted(data):
            data = self.encryption_manager.decrypt_data(data)
            self.logger.info("data_decrypted", {"project": self.project_name})

        self.teams = {}
        for team_data in data.get("teams", []):
            team = Team(**team_data)
            team.roles = [Role(**r) for r in team_data.get("roles", [])]
            self.teams[team.id] = team

        # OPS-005: Store version info
        self._data_version = data.get("version", "1.0.0")

        # OPS-006: Apply migrations if needed
        if self.migration_manager.needs_migration(data):
            data = self.migration_manager.migrate(data)
            # Save migrated data back
            with FileLock(self.lock_path):
                with open(self.config_path, 'w') as f:
                    json.dump(data, f, indent=2)

        return True

    def save(self) -> None:
        """Save team configuration to disk.

        OPS-004: Creates automatic backup before saving.
        """
        self.logger.info("config_save_start", {"config_path": str(self.config_path)})
        self.config_path.parent.mkdir(parents=True, exist_ok=True)

        # OPS-004: Create backup before write if config exists
        backup_path = None
        if self.enable_backup and self.backup_manager and self.config_path.exists():
            backup_path = self.backup_manager.create_backup(self.config_path)
            if backup_path:
                self.logger.info("backup_created", {"backup_path": str(backup_path)})

        data = {
            "project_name": self.project_name,
            "version": "1.0.0",  # OPS-005: Version tracking
            "updated_at": datetime.now().isoformat(),
            "teams": [asdict(team) for team in self.teams.values()]
        }

        # SEC-007: Encrypt sensitive data if encryption is enabled
        if self.encryption_manager.enabled:
            data = self.encryption_manager.encrypt_data(data)
            self.logger.info("data_encrypted", {"project": self.project_name})

        try:
            # Use file locking to prevent race conditions (SEC-004)
            with FileLock(self.lock_path):
                # Atomic write: write to temp file, then rename
                fd, temp_path = tempfile.mkstemp(
                    dir=self.config_path.parent,
                    prefix=f".{self.project_name}.tmp."
                )
                try:
                    with os.fdopen(fd, 'w') as f:
                        json.dump(data, f, indent=2)
                        f.flush()
                        os.fsync(f.fileno())

                    # Atomic rename
                    os.replace(temp_path, self.config_path)
                except Exception:
                    # Clean up temp file on error
                    try:
                        os.unlink(temp_path)
                    except FileNotFoundError:
                        pass
                    raise

            self.logger.info("config_saved", {
                "config_path": str(self.config_path),
                "team_count": len(self.teams),
                "backup_path": str(backup_path) if backup_path else None
            })
        except Exception as e:
            self.logger.error("config_save_failed", {
                "config_path": str(self.config_path),
                "error": str(e)
            }, exc_info=True)
            raise

    def assign_role(self, team_id: int, role_name: str, assignee: str) -> bool:
        """Assign a person to a role.

        Requires team-lead role (for their team) or admin role.
        SEC-008: Logs audit trail.
        """
        self.performance_metrics.start_operation("assign", team_id=team_id, role_name=role_name, assignee=assignee)

        # Validate inputs (FUNC-003, SEC-002, SEC-003)
        try:
            validate_role_name(role_name)
            validate_person_name(assignee)
        except ValueError as e:
            self.logger.error("role_assignment_validation_failed", {
                "team_id": team_id,
                "role_name": role_name,
                "assignee": assignee,
                "error": str(e)
            })
            self.performance_metrics.end_operation("assign", success=False, error_type="validation_error")
            print(f"❌ Validation error: {e}", file=sys.stderr)
            return False

        self._require_auth("assign role", team_id)

        # SEC-005: Check rate limit
        user_id = self.user_context.user_id if self.user_context else "default"
        allowed, rate_info = self._check_rate_limit(user_id)
        if not allowed:
            retry_after = rate_info.get("reset_time", 60)
            self.logger.error("rate_limit_exceeded", {
                "user_id": user_id,
                "operation": "assign_role",
                "retry_after": retry_after
            })
            print(f"❌ Rate limit exceeded. Retry after {retry_after} seconds.", file=sys.stderr)
            return False

        # FUNC-012: Check for duplicate assignments
        dup_check = self.check_duplicate_assignment(assignee, team_id)
        if dup_check["is_duplicate"]:
            if dup_check["action"] == "block":
                self.logger.error("duplicate_assignment_blocked", {
                    "team_id": team_id,
                    "role_name": role_name,
                    "assignee": assignee,
                    "existing": dup_check["existing_assignments"]
                })
                print(dup_check["message"], file=sys.stderr)
                return False
            else:
                # Warn but continue
                print(dup_check["message"], file=sys.stderr)

        self.logger.info("role_assignment_start", {
            "team_id": team_id,
            "role_name": role_name,
            "assignee": assignee,
            "user_id": self.user_context.user_id if self.user_context else None
        })

        if team_id not in self.teams:
            self.logger.error("team_not_found", {"team_id": team_id})
            return False

        team = self.teams[team_id]
        for role in team.roles:
            if role.name == role_name:
                # SEC-008: Capture before state for audit
                previous_assignee = role.assigned_to
                role.assigned_to = assignee
                self.save()

                # SEC-008: Log audit trail
                if self.enable_audit and self.audit_logger:
                    self.audit_logger.log_action(
                        "assign_role",
                        {
                            "team_id": team_id,
                            "team_name": team.name,
                            "role_name": role_name,
                            "before": previous_assignee,
                            "after": assignee
                        },
                        self.user_context
                    )

                self.logger.info("role_assigned", {
                    "team_id": team_id,
                    "team_name": team.name,
                    "role_name": role_name,
                    "assignee": assignee
                })
                self.performance_metrics.end_operation("assign", success=True, team_id=team_id, role_name=role_name)
                return True

        self.logger.error("role_not_found", {
            "team_id": team_id,
            "team_name": team.name,
            "role_name": role_name
        })
        self.performance_metrics.end_operation("assign", success=False, error_type="role_not_found")
        return False

    def unassign_role(self, team_id: int, role_name: str) -> bool:
        """Remove assignment from a role.

        SEC-008: Logs audit trail.
        """
        if team_id not in self.teams:
            print(f"❌ Team {team_id} not found")
            return False

        team = self.teams[team_id]
        for role in team.roles:
            if role.name == role_name:
                if role.assigned_to is None:
                    print(f"⚠️  Role '{role_name}' in {team.name} is already unassigned")
                    return False
                previous_assignee = role.assigned_to
                role.assigned_to = None
                self.save()

                # SEC-008: Log audit trail
                if self.enable_audit and self.audit_logger:
                    self.audit_logger.log_action(
                        "unassign_role",
                        {
                            "team_id": team_id,
                            "team_name": team.name,
                            "role_name": role_name,
                            "before": previous_assignee,
                            "after": None
                        },
                        self.user_context
                    )

                print(f"✅ Unassigned {previous_assignee} from {role_name} in {team.name}")
                return True

        print(f"❌ Role '{role_name}' not found in {team.name}")
        return False

    def reassign_role(self, team_id: int, from_role: str, to_role: str, person: str) -> bool:
        """Reassign a person from one role to another within the same team.

        FUNC-009: Role reassignment capability.

        Args:
            team_id: The team ID
            from_role: The role to move person from
            to_role: The role to move person to
            person: The person to reassign

        Returns:
            True if successful, False otherwise
        """
        # Validate inputs
        try:
            validate_role_name(from_role)
            validate_role_name(to_role)
            validate_person_name(person)
        except ValueError as e:
            self.logger.error("reassignment_validation_failed", {
                "team_id": team_id,
                "from_role": from_role,
                "to_role": to_role,
                "person": person,
                "error": str(e)
            })
            print(f"❌ Validation error: {e}", file=sys.stderr)
            return False

        self._require_auth("reassign role", team_id)

        if team_id not in self.teams:
            self.logger.error("team_not_found", {"team_id": team_id})
            print(f"❌ Team {team_id} not found")
            return False

        team = self.teams[team_id]

        # Find both roles
        from_role_obj = None
        to_role_obj = None
        for role in team.roles:
            if role.name == from_role:
                from_role_obj = role
            if role.name == to_role:
                to_role_obj = role

        # Validate both roles exist
        if from_role_obj is None:
            self.logger.error("from_role_not_found", {
                "team_id": team_id,
                "from_role": from_role
            })
            print(f"❌ Role '{from_role}' not found in {team.name}")
            return False

        if to_role_obj is None:
            self.logger.error("to_role_not_found", {
                "team_id": team_id,
                "to_role": to_role
            })
            print(f"❌ Role '{to_role}' not found in {team.name}")
            return False

        # Validate person is actually assigned to from_role
        if from_role_obj.assigned_to != person:
            self.logger.error("person_not_assigned_to_from_role", {
                "team_id": team_id,
                "from_role": from_role,
                "person": person,
                "actual_assignee": from_role_obj.assigned_to
            })
            print(f"❌ '{person}' is not assigned to '{from_role}' in {team.name}")
            return False

        # Perform reassignment
        previous_assignee = to_role_obj.assigned_to
        from_role_obj.assigned_to = None
        to_role_obj.assigned_to = person
        self.save()

        # Log audit trail
        if self.enable_audit and self.audit_logger:
            self.audit_logger.log_action(
                "reassign_role",
                {
                    "team_id": team_id,
                    "team_name": team.name,
                    "person": person,
                    "from_role": from_role,
                    "to_role": to_role,
                    "to_role_previous_assignee": previous_assignee
                },
                self.user_context
            )

        self.logger.info("role_reassigned", {
            "team_id": team_id,
            "team_name": team.name,
            "person": person,
            "from_role": from_role,
            "to_role": to_role
        })

        if previous_assignee:
            print(f"✅ Reassigned {person} from '{from_role}' to '{to_role}' in {team.name}")
            print(f"   Note: {previous_assignee} was previously assigned to '{to_role}'")
        else:
            print(f"✅ Reassigned {person} from '{from_role}' to '{to_role}' in {team.name}")
        return True

    def start_team(self, team_id: int, override: bool = False, reason: Optional[str] = None) -> bool:
        """Mark a team as active.

        SEC-008: Logs audit trail.
        FUNC-010: Supports override for admin users.

        Args:
            team_id: The team ID to start
            override: Whether to override phase gate checks (requires admin)
            reason: Reason for override (required if override=True)

        Returns:
            True if successful, False otherwise
        """
        self.performance_metrics.start_operation("start", team_id=team_id, override=override)
        self.logger.info("team_start_request", {
            "team_id": team_id,
            "override": override,
            "reason": reason
        })

        if team_id not in self.teams:
            self.performance_metrics.end_operation("start", success=False, error_type="team_not_found")
            self.logger.error("team_not_found", {"team_id": team_id})
            return False

        team = self.teams[team_id]

        # FUNC-010: Check for override capability
        if override:
            # Verify admin role
            if self.user_context is None or not self.user_context.has_permission("admin"):
                self.logger.error("override_permission_denied", {
                    "team_id": team_id,
                    "user": self.user_context.user_id if self.user_context else None,
                    "role": self.user_context.role if self.user_context else None
                })
                print(f"❌ Override requires admin role")
                return False

            if not reason:
                self.logger.error("override_missing_reason", {"team_id": team_id})
                print(f"❌ Override requires a reason (--reason)")
                return False

            self.logger.warn("phase_gate_override", {
                "team_id": team_id,
                "team_name": team.name,
                "reason": reason,
                "user": self.user_context.user_id if self.user_context else "system"
            })

        team = self.teams[team_id]
        previous_status = team.status
        team.status = "active"
        team.started_at = datetime.now().isoformat()
        self.save()

        # SEC-008: Log audit trail
        if self.enable_audit and self.audit_logger:
            self.audit_logger.log_action(
                "start_team",
                {
                    "team_id": team_id,
                    "team_name": team.name,
                    "before": previous_status,
                    "after": "active",
                    "started_at": team.started_at
                },
                self.user_context
            )

        self.logger.info("team_started", {
            "team_id": team_id,
            "team_name": team.name,
            "status": team.status,
            "started_at": team.started_at
        })
        self.performance_metrics.end_operation("start", success=True, team_id=team_id)
        return True

    def complete_team(self, team_id: int) -> bool:
        """Mark a team as completed.

        SEC-008: Logs audit trail.
        """
        self.performance_metrics.start_operation("complete", team_id=team_id)
        self.logger.info("team_complete_request", {"team_id": team_id})

        if team_id not in self.teams:
            self.logger.error("team_not_found", {"team_id": team_id})
            self.performance_metrics.end_operation("complete", success=False, error_type="team_not_found")
            return False

        team = self.teams[team_id]
        previous_status = team.status
        team.status = "completed"
        team.completed_at = datetime.now().isoformat()
        self.save()

        # SEC-008: Log audit trail
        if self.enable_audit and self.audit_logger:
            self.audit_logger.log_action(
                "complete_team",
                {
                    "team_id": team_id,
                    "team_name": team.name,
                    "before": previous_status,
                    "after": "completed",
                    "completed_at": team.completed_at
                },
                self.user_context
            )

        self.logger.info("team_completed", {
            "team_id": team_id,
            "team_name": team.name,
            "status": team.status,
            "completed_at": team.completed_at
        })
        self.performance_metrics.end_operation("complete", success=True, team_id=team_id)
        return True

    def query_teams(self, status: Optional[str] = None, phase: Optional[str] = None,
                    assignee: Optional[str] = None, role_name: Optional[str] = None) -> List[Dict[str, Any]]:
        """Query teams with filters.

        FUNC-006: Query API for filtering teams by status, phase, assignee, or role.

        Args:
            status: Filter by team status (not_started, active, completed, blocked)
            phase: Filter by phase name
            assignee: Filter by person assigned to any role
            role_name: Filter by specific role name

        Returns:
            List of teams matching all specified filters (AND logic)
        """
        results = []

        for team in self.teams.values():
            # Check status filter
            if status is not None and team.status != status:
                continue

            # Check phase filter
            if phase is not None and team.phase != phase:
                continue

            # Check assignee filter - person assigned to any role in team
            if assignee is not None:
                assigned_roles = [r for r in team.roles if r.assigned_to == assignee]
                if not assigned_roles:
                    continue

            # Check role_name filter - specific role exists in team
            if role_name is not None:
                matching_roles = [r for r in team.roles if r.name == role_name]
                if not matching_roles:
                    continue

            # Team passed all filters - build result
            team_data = {
                "id": team.id,
                "name": team.name,
                "phase": team.phase,
                "description": team.description,
                "status": team.status,
                "started_at": team.started_at,
                "completed_at": team.completed_at,
                "assigned_count": sum(1 for r in team.roles if r.assigned_to),
                "total_roles": len(team.roles),
                "roles": []
            }

            # Include role details, filtered if assignee or role_name specified
            for role in team.roles:
                if assignee is not None and role.assigned_to != assignee:
                    continue
                if role_name is not None and role.name != role_name:
                    continue
                team_data["roles"].append({
                    "name": role.name,
                    "assigned_to": role.assigned_to,
                    "responsibility": role.responsibility
                })

            results.append(team_data)

        self.logger.info("teams_queried", {
            "status_filter": status,
            "phase_filter": phase,
            "assignee_filter": assignee,
            "role_filter": role_name,
            "result_count": len(results)
        })

        return results

    def get_phase_status(self, phase: str) -> dict:
        """Get status summary for a phase."""
        phase_teams = [t for t in self.teams.values() if t.phase == phase]

        total = len(phase_teams)
        completed = len([t for t in phase_teams if t.status == "completed"])
        active = len([t for t in phase_teams if t.status == "active"])

        result = {
            "phase": phase,
            "total_teams": total,
            "completed": completed,
            "active": active,
            "not_started": total - completed - active,
            "progress_pct": (completed / total * 100) if total > 0 else 0
        }

        self.logger.debug("phase_status_queried", {"phase": phase, "status": result})
        return result

    def list_teams(self, phase: str = None) -> None:
        """Print all teams."""
        # Validate phase filter if provided (FUNC-004)
        if phase is not None:
            try:
                validate_phase(phase)
            except ValueError as e:
                self.logger.error("list_teams_validation_failed", {"error": str(e)})
                print(f"❌ Validation error: {e}", file=sys.stderr)
                return

        teams = self.teams.values()
        if phase:
            teams = [t for t in teams if t.phase == phase]

        team_list = []
        for team in sorted(teams, key=lambda t: (t.phase, t.id)):
            assigned_roles = sum(1 for r in team.roles if r.assigned_to)
            team_list.append({
                "id": team.id,
                "name": team.name,
                "phase": team.phase,
                "status": team.status,
                "assigned_count": assigned_roles,
                "total_roles": len(team.roles)
            })

        self.logger.info("teams_listed", {
            "phase_filter": phase,
            "team_count": len(team_list),
            "teams": team_list
        })

        # Print team information to stdout
        if team_list:
            print(f"\nProject: {self.project_name}")
            print("=" * 50)
            for team in team_list:
                print(f"\nTeam {team['id']}: {team['name']}")
                print(f"  Phase: {team['phase']}")
                print(f"  Status: {team['status']}")
                print(f"  Assigned: {team['assigned_count']}/{team['total_roles']}")
                # Show assigned roles
                t = self.teams.get(team['id'])
                if t:
                    for role in t.roles:
                        if role.assigned_to:
                            print(f"    - {role.name}: {role.assigned_to}")
        else:
            print("No teams found.")

    def get_agent_team(self, agent_type: str) -> Optional[Team]:
        """Map agent type to appropriate team."""
        mapping = {
            "planner": 2,      # Enterprise Architecture
            "coder": 7,        # Core Feature Squad
            "reviewer": 10,    # Quality Engineering
            "security": 9,     # Cybersecurity
            "tester": 10,      # Quality Engineering
            "ops": 11,         # SRE
        }
        team_id = mapping.get(agent_type.lower())
        team = self.teams.get(team_id) if team_id else None

        self.logger.debug("agent_team_mapped", {
            "agent_type": agent_type,
            "team_id": team_id,
            "found": team is not None
        })
        return team

    def validate_team_size(self, team_id: Optional[int] = None) -> dict:
        """Validate team sizes meet configured member requirement.

        Returns dict with validation results.
        Uses team_size_limits from rules.json (FUNC-008).
        """
        loader = get_rules_loader()
        MIN_TEAM_SIZE, MAX_TEAM_SIZE = loader.get_team_size_limits()

        results = {
            "valid": True,
            "violations": [],
            "teams_checked": 0
        }

        teams_to_check = [self.teams[team_id]] if team_id else self.teams.values()

        for team in teams_to_check:
            results["teams_checked"] += 1
            assigned_count = sum(1 for role in team.roles if role.assigned_to)

            if assigned_count < MIN_TEAM_SIZE:
                results["valid"] = False
                results["violations"].append({
                    "team_id": team.id,
                    "team_name": team.name,
                    "issue": "undersized",
                    "assigned": assigned_count,
                    "required": MIN_TEAM_SIZE
                })
                self.logger.warn("team_undersized", {
                    "team_id": team.id,
                    "team_name": team.name,
                    "assigned": assigned_count,
                    "required": MIN_TEAM_SIZE
                })
            elif assigned_count > MAX_TEAM_SIZE:
                results["valid"] = False
                results["violations"].append({
                    "team_id": team.id,
                    "team_name": team.name,
                    "issue": "oversized",
                    "assigned": assigned_count,
                    "maximum": MAX_TEAM_SIZE
                })
                self.logger.warn("team_oversized", {
                    "team_id": team.id,
                    "team_name": team.name,
                    "assigned": assigned_count,
                    "maximum": MAX_TEAM_SIZE
                })

        if results["valid"]:
            self.logger.info("team_size_validation_passed", {
                "teams_checked": results["teams_checked"]
            })
        else:
            self.logger.warn("team_size_validation_failed", {
                "teams_checked": results["teams_checked"],
                "violation_count": len(results["violations"]),
                "violations": results["violations"]
            })

        return results

    def delete_team(self, team_id: int, confirmed: bool = False) -> dict:
        """Delete a specific team from the project.

        Args:
            team_id: The ID of the team to delete
            confirmed: Whether deletion is confirmed (safety check)

        Returns:
            dict with deletion result
        """
        result = {
            "success": False,
            "team_id": team_id,
            "message": "",
            "requires_confirmation": False
        }

        if team_id not in self.teams:
            result["message"] = f"❌ Team {team_id} not found in project '{self.project_name}'"
            return result

        team = self.teams[team_id]

        if not confirmed:
            result["requires_confirmation"] = True
            result["message"] = (
                f"⚠️  Deletion requires confirmation. "
                f"Team {team_id} ({team.name}) will be permanently removed. "
                f"Set confirmed=true to proceed."
            )
            return result

        # Capture team data before deletion for audit
        team_data = asdict(team)
        deleted_team_name = team.name
        del self.teams[team_id]
        self.save()

        # SEC-008: Log audit trail
        if self.enable_audit and self.audit_logger:
            self.audit_logger.log_action(
                "delete_team",
                {
                    "team_id": team_id,
                    "team_name": deleted_team_name,
                    "deleted_data": team_data
                },
                self.user_context
            )

        result["success"] = True
        result["message"] = f"✅ Team {team_id} ({deleted_team_name}) deleted from project '{self.project_name}'"
        return result

    def delete_project(self, confirmed: bool = False) -> dict:
        """Delete the entire project.

        Args:
            confirmed: Whether deletion is confirmed (safety check)

        Returns:
            dict with deletion result
        """
        result = {
            "success": False,
            "project_name": self.project_name,
            "message": "",
            "requires_confirmation": False
        }

        if not self.config_path.exists():
            result["message"] = f"❌ Project '{self.project_name}' not found"
            return result

        if not confirmed:
            result["requires_confirmation"] = True
            team_count = len(self.teams)
            result["message"] = (
                f"⚠️  Deletion requires confirmation. "
                f"Project '{self.project_name}' with {team_count} team(s) will be permanently deleted. "
                f"Set confirmed=true to proceed."
            )
            return result

        # Capture project data before deletion for audit
        team_count = len(self.teams)
        deletion_time = datetime.now().isoformat()

        # SEC-008: Log audit trail before deletion
        if self.enable_audit and self.audit_logger:
            self.audit_logger.log_action(
                "delete_project",
                {
                    "project_name": self.project_name,
                    "team_count": team_count,
                    "teams": [asdict(team) for team in self.teams.values()],
                    "deleted_at": deletion_time
                },
                self.user_context
            )

        # Delete the project file
        try:
            self.config_path.unlink()

            result["success"] = True
            result["message"] = f"✅ Project '{self.project_name}' ({team_count} teams) deleted successfully"
        except Exception as e:
            result["message"] = f"❌ Error deleting project: {e}"


    # FUNC-012: Duplicate Detection Methods
    def get_person_assignments(self, person: str) -> List[Dict[str, Any]]:
        """Get all role assignments for a person across all teams.

        Args:
            person: The person name/email to look up

        Returns:
            List of assignments with team_id, team_name, role_name
        """
        assignments = []
        person_lower = person.lower()

        for team in self.teams.values():
            for role in team.roles:
                if role.assigned_to and role.assigned_to.lower() == person_lower:
                    assignments.append({
                        "team_id": team.id,
                        "team_name": team.name,
                        "role_name": role.name,
                        "person": role.assigned_to
                    })

        return assignments

    def check_duplicate_assignment(self, person: str, team_id: Optional[int] = None) -> Dict[str, Any]:
        """Check if assigning this person would create a duplicate.

        FUNC-012: Duplicate detection with configurable scope and action.

        Args:
            person: The person being assigned
            team_id: The team being assigned to (optional, for scope checking)

        Returns:
            Dict with duplicate check results:
            {
                "is_duplicate": bool,
                "existing_assignments": List[Dict],
                "action": "allow" | "warn" | "block",
                "message": str
            }
        """
        loader = get_rules_loader()
        config = loader.get_duplicate_detection_config()

        result = {
            "is_duplicate": False,
            "existing_assignments": [],
            "action": "allow",
            "message": ""
        }

        if not config.get("enabled", True):
            return result

        scope = config.get("scope", "project")
        action = config.get("action", "warn")

        # Get existing assignments
        existing = self.get_person_assignments(person)

        if not existing:
            return result

        # Check scope
        if scope == "team":
            # Only check if person is already in the same team
            existing = [a for a in existing if a["team_id"] == team_id]

        if existing:
            result["is_duplicate"] = True
            result["existing_assignments"] = existing
            result["action"] = action

            if action == "block":
                result["message"] = f"❌ Cannot assign '{person}': already assigned to {len(existing)} role(s)"
            else:
                result["message"] = f"⚠️  Warning: '{person}' is already assigned to {len(existing)} role(s)"

        return result

    def validate_no_duplicates(self, team_id: Optional[int] = None) -> Dict[str, Any]:
        """Validate entire project for duplicate assignments.

        Args:
            team_id: Optional team to limit validation to

        Returns:
            Dict with validation results
        """
        loader = get_rules_loader()
        config = loader.get_duplicate_detection_config()

        result = {
            "valid": True,
            "duplicates": [],
            "total_affected": 0
        }

        if not config.get("enabled", True):
            return result

        # Build map of person -> assignments
        person_assignments: Dict[str, List[Dict]] = {}

        teams_to_check = [self.teams[team_id]] if team_id else self.teams.values()

        for team in teams_to_check:
            for role in team.roles:
                if role.assigned_to:
                    person = role.assigned_to.lower()
                    if person not in person_assignments:
                        person_assignments[person] = []
                    person_assignments[person].append({
                        "team_id": team.id,
                        "team_name": team.name,
                        "role_name": role.name,
                        "person": role.assigned_to
                    })

        # Find duplicates (people with multiple assignments)
        for person, assignments in person_assignments.items():
            if len(assignments) > 1:
                result["valid"] = False
                result["duplicates"].append({
                    "person": person,
                    "assignment_count": len(assignments),
                    "assignments": assignments
                })
                result["total_affected"] += 1

        return result

        return result

    def list_backups(self) -> List[Dict[str, Any]]:
        """List all available backups for this project.

        Returns:
            List of backup info dicts
        """
        if self.backup_manager:
            return self.backup_manager.list_backups()
        return []

    def restore_backup(self, backup_filename: str) -> dict:
        """Restore from a backup file.

        Args:
            backup_filename: Name of the backup file to restore

        Returns:
            Dict with restore result
        """
        result = {
            "success": False,
            "message": "",
            "backup_file": backup_filename
        }

        if not self.backup_manager:
            result["message"] = "❌ Backup manager not enabled"
            return result

        backup_path = self.backup_manager.backup_dir / backup_filename
        if not backup_path.exists():
            result["message"] = f"❌ Backup file not found: {backup_filename}"
            return result

        # Create backup of current state before restore
        if self.config_path.exists():
            current_backup = self.backup_manager.create_backup(self.config_path)
            if current_backup:
                print(f"💾 Created pre-restore backup: {current_backup.name}", file=sys.stderr)

        # Perform restore
        if self.backup_manager.restore_backup(backup_path, self.config_path):
            # Reload the teams data
            self.load()

            # Log audit trail
            if self.enable_audit and self.audit_logger:
                self.audit_logger.log_action(
                    "restore_backup",
                    {
                        "backup_file": backup_filename,
                        "restored_path": str(self.config_path),
                        "team_count": len(self.teams)
                    },
                    self.user_context
                )

            result["success"] = True
            result["message"] = f"✅ Successfully restored from {backup_filename}"
            result["team_count"] = len(self.teams)
        else:
            result["message"] = f"❌ Failed to restore from {backup_filename}"

        return result

    def query_audit(self, **filters) -> List[Dict[str, Any]]:
        """Query the audit log.

        Args:
            **filters: Optional filters like user, action, team_id, start_time, end_time, limit

        Returns:
            List of audit entries
        """
        if not self.audit_logger:
            return []
        return self.audit_logger.query_audit_log(**filters)

    def get_recent_audit(self, count: int = 10) -> List[Dict[str, Any]]:
        """Get recent audit entries.

        Args:
            count: Number of entries to return

        Returns:
            List of recent audit entries
        """
        if not self.audit_logger:
            return []
        return self.audit_logger.get_recent_actions(count)

    def get_team_history(self, team_id: int,
                         start_date: Optional[datetime] = None,
                         end_date: Optional[datetime] = None) -> List[Dict[str, Any]]:
        """Get history of changes for a specific team.

        FUNC-011: Team history view with date range filtering.

        Args:
            team_id: The team ID to get history for
            start_date: Optional start date filter
            end_date: Optional end date filter

        Returns:
            List of audit entries for the team
        """
        if not self.audit_logger:
            return []

        entries = self.audit_logger.query_audit_log(
            team_id=team_id,
            start_time=start_date,
            end_time=end_date,
            limit=1000
        )

        self.logger.info("team_history_queried", {
            "team_id": team_id,
            "start_date": start_date.isoformat() if start_date else None,
            "end_date": end_date.isoformat() if end_date else None,
            "entry_count": len(entries)
        })

        return entries

    def get_project_timeline(self,
                             start_date: Optional[datetime] = None,
                             end_date: Optional[datetime] = None) -> List[Dict[str, Any]]:
        """Get timeline of all project events.

        FUNC-011: Project timeline view with date range filtering.

        Args:
            start_date: Optional start date filter
            end_date: Optional end date filter

        Returns:
            List of all project events sorted by timestamp
        """
        if not self.audit_logger:
            return []

        entries = self.audit_logger.query_audit_log(
            start_time=start_date,
            end_time=end_date,
            limit=1000
        )

        self.logger.info("project_timeline_queried", {
            "project": self.project_name,
            "start_date": start_date.isoformat() if start_date else None,
            "end_date": end_date.isoformat() if end_date else None,
            "entry_count": len(entries)
        })

        return entries

    def health_check(self) -> dict:
        """Perform health check on team manager.

        OPS-003: Checks Python backend status and file system access.

        Returns:
            dict with health status information
        """
        health = {
            "status": "healthy",
            "checks": {},
            "timestamp": datetime.utcnow().isoformat() + "Z",
            "version": "1.0.0"
        }

        # Check 1: Python environment
        try:
            import sys
            health["checks"]["python"] = {
                "status": "pass",
                "version": f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
            }
        except Exception as e:
            health["checks"]["python"] = {
                "status": "fail",
                "error": str(e)
            }
            health["status"] = "unhealthy"

        # Check 2: File system access
        try:
            import tempfile
            test_dir = Path(tempfile.gettempdir()) / "team_manager_health"
            test_dir.mkdir(exist_ok=True)
            test_file = test_dir / ".health_check"
            test_file.write_text("ok")
            content = test_file.read_text()
            test_file.unlink()
            test_dir.rmdir()

            if content == "ok":
                health["checks"]["filesystem"] = {
                    "status": "pass",
                    "writable": True
                }
            else:
                health["checks"]["filesystem"] = {
                    "status": "fail",
                    "error": "Read/write mismatch"
                }
                health["status"] = "unhealthy"
        except Exception as e:
            health["checks"]["filesystem"] = {
                "status": "fail",
                "error": str(e)
            }
            health["status"] = "unhealthy"

        # Check 3: Config directory access
        try:
            self.config_path.parent.mkdir(parents=True, exist_ok=True)
            health["checks"]["config_dir"] = {
                "status": "pass",
                "path": str(self.config_path.parent),
                "accessible": True
            }
        except Exception as e:
            health["checks"]["config_dir"] = {
                "status": "fail",
                "path": str(self.config_path.parent),
                "error": str(e)
            }
            health["status"] = "unhealthy"

        # Check 4: JSON serialization
        try:
            test_data = {"test": True, "timestamp": datetime.utcnow().isoformat()}
            json.dumps(test_data)
            health["checks"]["json"] = {
                "status": "pass"
            }
        except Exception as e:
            health["checks"]["json"] = {
                "status": "fail",
                "error": str(e)
            }
            health["status"] = "unhealthy"

        return health

    # FUNC-005: Batch operation wrappers
    def import_csv_file(self, csv_path: Path, dry_run: bool = False) -> Dict[str, Any]:
        """Import role assignments from CSV file."""
        return import_csv(self, csv_path, dry_run)

    def export_csv_file(self, csv_path: Path) -> Dict[str, Any]:
        """Export role assignments to CSV file."""
        return export_csv(self, csv_path)

    def import_json_file(self, json_path: Path, dry_run: bool = False) -> Dict[str, Any]:
        """Import role assignments from JSON file."""
        return import_json(self, json_path, dry_run)

    def export_json_file(self, json_path: Path, pretty: bool = True) -> Dict[str, Any]:
        """Export project state to JSON file."""
        return export_json(self, json_path, pretty)


def main():
    parser = argparse.ArgumentParser(description="Team Manager - Standardized Team Layout")
    parser.add_argument("--project", required=True, help="Project name")
    parser.add_argument("--request-id", help="Correlation ID for request tracing")
    parser.add_argument("--test-mode", action="store_true", help="Run in test mode (skips authentication)")

    subparsers = parser.add_subparsers(dest="command", help="Command to run")

    # Init command
    init_parser = subparsers.add_parser("init", help="Initialize new project")

    # List command
    list_parser = subparsers.add_parser("list", help="List teams")
    list_parser.add_argument("--phase", help="Filter by phase")

    # Query command (FUNC-006)
    query_parser = subparsers.add_parser("query", help="Query teams with filters")
    query_parser.add_argument("--status", choices=["not_started", "active", "completed", "blocked"],
                              help="Filter by team status")
    query_parser.add_argument("--phase", help="Filter by phase")
    query_parser.add_argument("--assignee", help="Filter by person assigned to any role")
    query_parser.add_argument("--role", help="Filter by specific role name")
    query_parser.add_argument("--format", choices=["table", "json"], default="table",
                              help="Output format (default: table)")

    # Assign command
    assign_parser = subparsers.add_parser("assign", help="Assign person to role")
    assign_parser.add_argument("--team", type=int, required=True, help="Team ID")
    assign_parser.add_argument("--role", required=True, help="Role name")
    assign_parser.add_argument("--person", required=True, help="Person name")

    # Unassign command
    unassign_parser = subparsers.add_parser("unassign", help="Remove person from role")
    unassign_parser.add_argument("--team", type=int, required=True, help="Team ID")
    unassign_parser.add_argument("--role", required=True, help="Role name")

    # Reassign command (FUNC-009)
    reassign_parser = subparsers.add_parser("reassign", help="Reassign person from one role to another")
    reassign_parser.add_argument("--team", type=int, required=True, help="Team ID")
    reassign_parser.add_argument("--from-role", required=True, help="Role to move from")
    reassign_parser.add_argument("--to-role", required=True, help="Role to move to")
    reassign_parser.add_argument("--person", required=True, help="Person to reassign")

    # Start command
    start_parser = subparsers.add_parser("start", help="Start a team")
    start_parser.add_argument("--team", type=int, required=True, help="Team ID")
    start_parser.add_argument("--override", action="store_true", help="Override phase gate check (admin only)")
    start_parser.add_argument("--reason", help="Reason for override (required with --override)")

    # Complete command
    complete_parser = subparsers.add_parser("complete", help="Complete a team")
    complete_parser.add_argument("--team", type=int, required=True, help="Team ID")

    # Status command
    status_parser = subparsers.add_parser("status", help="Show phase status")
    status_parser.add_argument("--phase", help="Phase name")

    # Validate-size command
    validate_size_parser = subparsers.add_parser("validate-size", help="Validate team sizes (4-6 members)")
    validate_size_parser.add_argument("--team", type=int, help="Specific team ID to validate (optional)")

    # Delete-team command
    delete_team_parser = subparsers.add_parser("delete-team", help="Delete a specific team from the project")
    delete_team_parser.add_argument("--team", type=int, required=True, help="Team ID to delete")
    delete_team_parser.add_argument("--confirmed", action="store_true", help="Confirm deletion (required)")

    # Delete-project command
    delete_project_parser = subparsers.add_parser("delete-project", help="Delete the entire project")
    delete_project_parser.add_argument("--confirmed", action="store_true", help="Confirm deletion (required)")

    # SEC-007: Encrypt-project command
    encrypt_parser = subparsers.add_parser("encrypt-project", help="Encrypt project data at rest")

    # SEC-007: Decrypt-project command
    decrypt_parser = subparsers.add_parser("decrypt-project", help="Decrypt project data")

    # List-backups command (OPS-004)
    list_backups_parser = subparsers.add_parser("list-backups", help="List available backups")

    # Restore command (OPS-004)
    restore_parser = subparsers.add_parser("restore", help="Restore from a backup")
    restore_parser.add_argument("--backup", required=True, help="Backup filename to restore")

    # FUNC-008: Reload-rules command
    reload_rules_parser = subparsers.add_parser("reload-rules", help="Reload rules from rules.json")

    # OPS-006: Migrate command
    migrate_parser = subparsers.add_parser("migrate", help="Migrate project to current version")

    # Audit command (SEC-008)
    audit_parser = subparsers.add_parser("audit", help="Query audit log")
    audit_parser.add_argument("--user", help="Filter by user")
    audit_parser.add_argument("--action", help="Filter by action type")
    audit_parser.add_argument("--team", type=int, help="Filter by team ID")
    audit_parser.add_argument("--limit", type=int, default=20, help="Maximum entries to show (default: 20)")
    audit_parser.add_argument("--recent", action="store_true", help="Show most recent entries")

    # Team history command (FUNC-011)
    history_parser = subparsers.add_parser("team-history", help="Show history for a team")
    history_parser.add_argument("--team", type=int, required=True, help="Team ID")
    history_parser.add_argument("--start-date", help="Start date (ISO format: YYYY-MM-DD)")
    history_parser.add_argument("--end-date", help="End date (ISO format: YYYY-MM-DD)")
    history_parser.add_argument("--format", choices=["table", "json"], default="table", help="Output format")

    # Project timeline command (FUNC-011)
    timeline_parser = subparsers.add_parser("project-timeline", help="Show timeline of all project events")
    timeline_parser.add_argument("--start-date", help="Start date (ISO format: YYYY-MM-DD)")
    timeline_parser.add_argument("--end-date", help="End date (ISO format: YYYY-MM-DD)")
    timeline_parser.add_argument("--format", choices=["table", "json"], default="table", help="Output format")

    # Health command (OPS-003)
    health_parser = subparsers.add_parser("health", help="Check team manager health status")

    # OPS-008: Performance report command
    perf_parser = subparsers.add_parser("performance-report", help="Show performance metrics report")
    perf_parser.add_argument("--days", type=int, default=7, help="Number of days to include (default: 7)")
    perf_parser.add_argument("--operation", help="Filter by operation type")
    perf_parser.add_argument("--export", choices=["json", "csv"], help="Export to file format")
    perf_parser.add_argument("--output", help="Output file path for export")

    # FUNC-005: Batch operations commands
    # Import CSV command
    import_csv_parser = subparsers.add_parser("import-csv", help="Import role assignments from CSV")
    import_csv_parser.add_argument("--file", required=True, help="Path to CSV file")
    import_csv_parser.add_argument("--dry-run", action="store_true", help="Validate without making changes")

    # Export CSV command
    export_csv_parser = subparsers.add_parser("export-csv", help="Export role assignments to CSV")
    export_csv_parser.add_argument("--file", required=True, help="Path to output CSV file")

    # Import JSON command
    import_json_parser = subparsers.add_parser("import-json", help="Import role assignments from JSON")
    import_json_parser.add_argument("--file", required=True, help="Path to JSON file")
    import_json_parser.add_argument("--dry-run", action="store_true", help="Validate without making changes")

    # Export JSON command
    export_json_parser = subparsers.add_parser("export-json", help="Export project state to JSON")
    export_json_parser.add_argument("--file", required=True, help="Path to output JSON file")
    export_json_parser.add_argument("--compact", action="store_true", help="Output compact JSON (no indentation)")

    # Template commands
    template_csv_parser = subparsers.add_parser("template-csv", help="Create CSV template for bulk assignments")
    template_csv_parser.add_argument("--file", default="assignments_template.csv", help="Output file path")

    template_json_parser = subparsers.add_parser("template-json", help="Create JSON template for bulk assignments")
    template_json_parser.add_argument("--file", default="assignments_template.json", help="Output file path")

    args = parser.parse_args()

    # Suppress deprecation warnings in test mode for cleaner output
    if args.test_mode:
        warnings.filterwarnings("ignore", category=DeprecationWarning)

    # Validate project name to prevent command injection
    validate_project_name(args.project)

    # Create logger for main and generate request_id if not provided
    request_id = args.request_id or f"tm-{datetime.utcnow().strftime('%Y%m%d%H%M%S')}-{os.getpid()}"
    cli_logger = StructuredLogger("team_manager_cli", request_id)
    cli_logger.info("cli_start", {"command": args.command, "project": args.project})

    # SEC-006: Handle SecurityError from path validation
    try:
        manager = TeamManager(args.project, test_mode=args.test_mode)
    except SecurityError as e:
        cli_logger.error("security_error", {"error": str(e)})
        print(f"🔒 Security error: {e}", file=sys.stderr)
        sys.exit(1)

    # SEC-005: Handle RateLimitExceeded
    except RateLimitExceeded as e:
        retry_msg = f" Retry after {e.retry_after}s." if e.retry_after else ""
        cli_logger.error("rate_limit_exceeded", {"error": str(e), "retry_after": e.retry_after})
        print(f"⏱️  Rate limit exceeded.{retry_msg}", file=sys.stderr)
        sys.exit(429)

    if args.command == "init":
        manager.initialize_project()
        print(f"\nTeams configuration saved to: {manager.config_path}")

    elif args.command in ["list", "assign", "unassign", "start", "complete", "status", "validate-size", "delete-team", "delete-project", "list-backups", "restore", "audit", "import-csv", "export-csv", "import-json", "export-json"]:
        if args.command in ["delete-team", "delete-project"]:
            # For delete commands, project may not exist yet (delete-project)
            if args.command == "delete-team" and not manager.load():
                print(f"❌ Project '{args.project}' not found.")
                sys.exit(1)
            if args.command == "delete-project":
                # Try to load but don't fail if file doesn't exist
                manager.load()
        else:
            if not manager.load():
                print(f"❌ Project '{args.project}' not found. Run: team_manager.py --project {args.project} init")
                sys.exit(1)

        if args.command == "list":
            manager.list_teams(args.phase)

        elif args.command == "query":
            results = manager.query_teams(
                status=args.status,
                phase=args.phase,
                assignee=args.assignee,
                role_name=args.role
            )
            if args.format == "json":
                print(json.dumps(results, indent=2))
            else:
                # Table format
                if not results:
                    print("No teams match the specified filters.")
                else:
                    print(f"\nFound {len(results)} team(s) matching filters:\n")
                    for team in results:
                        print(f"Team {team['id']}: {team['name']}")
                        print(f"  Phase: {team['phase']}")
                        print(f"  Status: {team['status']}")
                        print(f"  Assigned: {team['assigned_count']}/{team['total_roles']}")
                        if team['roles']:
                            print("  Matching Roles:")
                            for role in team['roles']:
                                assignee = role['assigned_to'] or "(unassigned)"
                                print(f"    - {role['name']}: {assignee}")
                        print()

        elif args.command == "assign":
            if manager.assign_role(args.team, args.role, args.person):
                print(f"✅ Assigned {args.person} to {args.role} in Team {args.team}")

        elif args.command == "unassign":
            manager.unassign_role(args.team, args.role)

        elif args.command == "reassign":
            manager.reassign_role(args.team, args.from_role, args.to_role, args.person)

        elif args.command == "start":
            manager.start_team(args.team, override=args.override, reason=args.reason)

        elif args.command == "complete":
            manager.complete_team(args.team)

        elif args.command == "status":
            if args.phase:
                status = manager.get_phase_status(args.phase)
                print(f"\n{status['phase']}")
                print(f"  Progress: {status['progress_pct']:.0f}%")
                print(f"  Teams: {status['completed']}/{status['total_teams']} complete")
                print(f"  Active: {status['active']}, Not started: {status['not_started']}")
            else:
                # Show all phases
                phases = set(t.phase for t in manager.teams.values())
                for phase in sorted(phases, key=lambda p: p.split(":")[0]):
                    status = manager.get_phase_status(phase)
                    print(f"\n{status['phase']}: {status['progress_pct']:.0f}% complete")

        elif args.command == "validate-size":
            results = manager.validate_team_size(args.team)
            if results["valid"]:
                print(f"✅ All {results['teams_checked']} teams have valid size (4-6 members)")
                sys.exit(0)
            else:
                print(f"❌ Team size violations found:")
                for violation in results["violations"]:
                    print(f"   {violation['message']}")
                sys.exit(1)

        elif args.command == "delete-team":
            result = manager.delete_team(args.team, confirmed=args.confirmed)
            print(result["message"])
            if not result["success"] and not result["requires_confirmation"]:
                sys.exit(1)

        elif args.command == "delete-project":
            result = manager.delete_project(confirmed=args.confirmed)
            print(result["message"])
            if not result["success"] and not result["requires_confirmation"]:
                sys.exit(1)

        elif args.command == "list-backups":
            backups = manager.list_backups()
            if backups:
                print(f"\n📦 Available backups for '{args.project}':")
                print(f"{'Filename':<50} {'Size':>10} {'Created At'}")
                print("-" * 90)
                for backup in backups:
                    size_kb = backup["size_bytes"] / 1024
                    print(f"{backup['filename']:<50} {size_kb:>9.1f}KB {backup['created_at']}")
                print(f"\nTotal backups: {len(backups)}")
            else:
                print(f"ℹ️  No backups found for '{args.project}'")

        elif args.command == "restore":
            result = manager.restore_backup(args.backup)
            print(result["message"])
            if result["success"]:
                print(f"\n📊 Project now has {result.get('team_count', 0)} team(s)")
            else:
                sys.exit(1)

        elif args.command == "audit":
            if args.recent:
                entries = manager.get_recent_audit(args.limit)
            else:
                filters = {"limit": args.limit}
                if args.user:
                    filters["user"] = args.user
                if args.action:
                    filters["action"] = args.action
                if args.team:
                    filters["team_id"] = args.team
                entries = manager.query_audit(**filters)

            if entries:
                print(f"\n📋 Audit log entries for '{args.project}':")
                print(f"{'Timestamp':<25} {'User':<15} {'Action':<20} {'Details'}")
                print("-" * 100)
                for entry in entries:
                    ts = entry["timestamp"].replace("T", " ").replace("Z", "")[:19]
                    user = entry.get("user", "unknown")[:14]
                    action = entry.get("action", "unknown")[:19]
                    details = json.dumps(entry.get("details", {}))[:50]
                    print(f"{ts:<25} {user:<15} {action:<20} {details}")
                print(f"\nTotal entries: {len(entries)}")
            else:
                print(f"ℹ️  No audit entries found for '{args.project}'")

        elif args.command == "team-history":
            from datetime import datetime as dt
            start_date = None
            end_date = None
            if args.start_date:
                start_date = dt.fromisoformat(args.start_date)
            if args.end_date:
                end_date = dt.fromisoformat(args.end_date)

            entries = manager.get_team_history(args.team, start_date, end_date)

            if args.format == "json":
                print(json.dumps(entries, indent=2))
            else:
                if entries:
                    team = manager.teams.get(args.team)
                    team_name = team.name if team else f"Team {args.team}"
                    print(f"\n📜 History for {team_name}:")
                    print(f"{'Timestamp':<25} {'Action':<20} {'Details'}")
                    print("-" * 80)
                    for entry in entries:
                        ts = entry["timestamp"].replace("T", " ").replace("Z", "")[:19]
                        action = entry.get("action", "unknown")[:19]
                        details = json.dumps(entry.get("details", {}))[:40]
                        print(f"{ts:<25} {action:<20} {details}")
                    print(f"\nTotal entries: {len(entries)}")
                else:
                    print(f"ℹ️  No history found for team {args.team}")

        elif args.command == "project-timeline":
            from datetime import datetime as dt
            start_date = None
            end_date = None
            if args.start_date:
                start_date = dt.fromisoformat(args.start_date)
            if args.end_date:
                end_date = dt.fromisoformat(args.end_date)

            entries = manager.get_project_timeline(start_date, end_date)

            if args.format == "json":
                print(json.dumps(entries, indent=2))
            else:
                if entries:
                    print(f"\n📅 Project Timeline for '{args.project}':")
                    print(f"{'Timestamp':<25} {'User':<15} {'Action':<20} {'Team/Details'}")
                    print("-" * 90)
                    for entry in entries:
                        ts = entry["timestamp"].replace("T", " ").replace("Z", "")[:19]
                        user = entry.get("user", "unknown")[:14]
                        action = entry.get("action", "unknown")[:19]
                        details = entry.get("details", {})
                        team_info = f"Team {details.get('team_id', 'N/A')}"
                        print(f"{ts:<25} {user:<15} {action:<20} {team_info}")
                    print(f"\nTotal entries: {len(entries)}")
                else:
                    print(f"ℹ️  No timeline entries found for '{args.project}'")

        # FUNC-005: Batch operation handlers
        elif args.command == "import-csv":
            result = manager.import_csv_file(Path(args.file), dry_run=args.dry_run)
            if result["dry_run"]:
                print(f"🔍 Dry run results for {args.file}:")
            else:
                print(f"📥 Imported from {args.file}:")
            print(f"   Success: {result['success']}")
            print(f"   Imported: {result['imported']}")
            print(f"   Skipped: {result['skipped']}")
            if result["errors"]:
                print(f"   Errors: {len(result['errors'])}")
                for error in result["errors"][:5]:  # Show first 5 errors
                    print(f"      Row {error.get('row', 'N/A')}: {error.get('error', 'Unknown error')}")
                if len(result["errors"]) > 5:
                    print(f"      ... and {len(result['errors']) - 5} more errors")
            if not result["success"] and not result["dry_run"]:
                sys.exit(1)

        elif args.command == "export-csv":
            result = manager.export_csv_file(Path(args.file))
            if result["success"]:
                print(f"✅ Exported {result['exported']} roles to {result['file_path']}")
            else:
                print(f"❌ Export failed: {result['errors']}")
                sys.exit(1)

        elif args.command == "import-json":
            result = manager.import_json_file(Path(args.file), dry_run=args.dry_run)
            if result["dry_run"]:
                print(f"🔍 Dry run results for {args.file}:")
            else:
                print(f"📥 Imported from {args.file}:")
            print(f"   Success: {result['success']}")
            print(f"   Imported: {result['imported']}")
            print(f"   Skipped: {result['skipped']}")
            if result["errors"]:
                print(f"   Errors: {len(result['errors'])}")
                for error in result["errors"][:5]:
                    idx = error.get('index', 'N/A')
                    err_msg = error.get('error', 'Unknown error')
                    print(f"      Entry {idx}: {err_msg}")
                if len(result["errors"]) > 5:
                    print(f"      ... and {len(result['errors']) - 5} more errors")
            if not result["success"] and not result["dry_run"]:
                sys.exit(1)

        elif args.command == "export-json":
            pretty = not args.compact
            result = manager.export_json_file(Path(args.file), pretty=pretty)
            if result["success"]:
                print(f"✅ Exported {result['team_count']} teams to {result['file_path']}")
            else:
                print(f"❌ Export failed: {result['errors']}")
                sys.exit(1)

    elif args.command in ["template-csv", "template-json"]:
        # Template commands don't require project to exist
        if args.command == "template-csv":
            result = create_csv_template(Path(args.file))
            if result["success"]:
                print(f"✅ Created CSV template: {result['file_path']}")
                print("   Edit this file and run: team_manager.py --project <name> import-csv --file " + args.file)
            else:
                print(f"❌ Failed to create template: {result['errors']}")
                sys.exit(1)
        elif args.command == "template-json":
            result = create_json_template(Path(args.file))
            if result["success"]:
                print(f"✅ Created JSON template: {result['file_path']}")
                print("   Edit this file and run: team_manager.py --project <name> import-json --file " + args.file)
            else:
                print(f"❌ Failed to create template: {result['errors']}")
                sys.exit(1)

    elif args.command == "reload-rules":
        # FUNC-008: Reload rules
        reload_rules_cmd()

    elif args.command == "migrate":
        # OPS-006: Migration check and run
        status = manager.migration_manager.get_migration_status()
        if status["status"] == "needs_migration":
            print(f"🔄 Project '{args.project}' needs migration:")
            print(f"   Current: v{status['current_version']}")
            print(f"   Target:  v{status['target_version']}")
            # Load will trigger migration
            manager.load()
            print(f"✅ Migration complete")
        elif status["status"] == "current":
            print(f"✅ Project '{args.project}' is at current version (v{status['current_version']})")
        elif status["status"] == "not_found":
            print(f"❌ Project '{args.project}' not found")
            sys.exit(1)
        else:
            print(f"❌ Error: {status.get('error', 'Unknown error')}")
            sys.exit(1)

    elif args.command == "health":
        # OPS-003: Health check - doesn't require project to exist
        health = manager.health_check()
        # FUNC-008: Add rules check
        try:
            loader = get_rules_loader()
            health["checks"]["rules"] = {
                "status": "pass",
                "rules_path": str(loader.rules_path),
                "rules_loaded": bool(loader.rules)
            }
        except Exception as e:
            health["checks"]["rules"] = {
                "status": "fail",
                "error": str(e)
            }
            health["status"] = "unhealthy"
        print(json.dumps(health, indent=2))
        if health["status"] != "healthy":
            sys.exit(1)


    elif args.command == "performance-report":
        # OPS-008: Performance metrics report
        if args.export and args.output:
            success = manager.performance_metrics.export_report(
                Path(args.output), format=args.export, days=args.days
            )
            if success:
                print(f"✅ Performance report exported to {args.output}")
            else:
                sys.exit(1)
        else:
            if args.operation:
                stats = manager.performance_metrics.get_operation_stats(
                    operation=args.operation, since=datetime.utcnow() - timedelta(days=args.days)
                )
                print(f"\n📊 Performance Report: {args.operation}")
                print(f"{'='*50}")
            else:
                stats = manager.performance_metrics.get_report(days=args.days)
                print(f"\n📊 Performance Report (last {args.days} days)")
                print(f"{'='*50}")
                print(f"Project: {stats['project']}")
                print(f"Generated: {stats['generated_at']}")
                print()
                # Overall stats
                overall = stats['overall']
                print(f"Overall Operations: {overall['count']}")
                print(f"  Success: {overall['success_count']} ({overall['success_rate']}%)")
                print(f"  Failures: {overall['failure_count']} ({overall['error_rate']}%)")
                if 'duration_stats' in overall:
                    ds = overall['duration_stats']
                    print(f"\nDuration Statistics:")
                    print(f"  Average: {ds['avg_ms']}ms")
                    print(f"  Median: {ds['median_ms']}ms")
                    print(f"  Min: {ds['min_ms']}ms")
                    print(f"  Max: {ds['max_ms']}ms")
                # By operation
                if 'by_operation' in stats and stats['by_operation']:
                    print(f"\nBy Operation:")
                    for op, op_stats in stats['by_operation'].items():
                        if op_stats['count'] > 0:
                            print(f"  {op}: {op_stats['count']} ops, avg {op_stats.get('duration_stats', {}).get('avg_ms', 'N/A')}ms")
            if args.export == "json" and not args.output:
                print(json.dumps(stats, indent=2))

    elif args.command in ["encrypt-project", "decrypt-project"]:
        # SEC-007: Encryption/Decryption commands
        if not manager.encryption_manager.enabled:
            print("❌ Encryption not enabled. Set TEAM_ENCRYPTION_KEY environment variable.")
            sys.exit(1)

        if not manager.config_path.exists():
            print(f"❌ Project '{args.project}' not found.")
            sys.exit(1)

        # Load current data
        with open(manager.config_path, 'r') as f:
            data = json.load(f)

        # Define sensitive fields to encrypt/decrypt
        sensitive_fields = ["assigned_to", "assignee", "person", "user", "user_id"]
        encrypted_count = 0

        def process_encrypted_value(value, encrypt):
            """Process a potentially encrypted value."""
            if not isinstance(value, str):
                return value, 0
            if encrypt:
                if value.startswith('gAAAA'):
                    return value, 0
                return manager.encryption_manager.encrypt(value), 1
            else:
                if not value.startswith('gAAAA'):
                    return value, 0
                return manager.encryption_manager.decrypt(value), 1

        def process_dict(d, encrypt):
            """Process a dictionary recursively."""
            count = 0
            for key, value in d.items():
                if key in sensitive_fields and isinstance(value, str):
                    d[key], c = process_encrypted_value(value, encrypt)
                    count += c
                elif isinstance(value, dict):
                    count += process_dict(value, encrypt)
                elif isinstance(value, list):
                    for item in value:
                        if isinstance(item, dict):
                            count += process_dict(item, encrypt)
            return count

        encrypted_count = process_dict(data, args.command == "encrypt-project")

        # Save back
        with open(manager.config_path, 'w') as f:
            json.dump(data, f, indent=2)

        action = "Encrypted" if args.command == "encrypt-project" else "Decrypted"
        print(f"✅ {action} {encrypted_count} sensitive fields in project '{args.project}'")

    else:
        parser.print_help()


if __name__ == "__main__":
    main()
