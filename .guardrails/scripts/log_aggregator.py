#!/usr/bin/env python3
"""
Log Aggregator - Centralized log aggregation for team operations.

OPS-007: Aggregates logs from .teams/audit.log and structured logs.
Provides CLI commands for querying and analyzing logs.
"""

import argparse
import csv
import fcntl
import json
import sys
import re
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Dict, List, Optional, Iterator


class LogAggregator:
    """Aggregates and queries logs from multiple sources."""

    def __init__(self, project_name: str, teams_dir: Path = None):
        self.project_name = project_name
        self.teams_dir = teams_dir or Path(".teams")
        self.audit_file = self.teams_dir / "audit.log"
        self.structured_logs_dir = self.teams_dir / "logs"

    def _parse_timestamp(self, ts: str) -> datetime:
        """Parse ISO8601 timestamp."""
        # Handle both with and without Z suffix
        ts = ts.replace("Z", "+00:00")
        try:
            return datetime.fromisoformat(ts)
        except ValueError:
            # Try alternate format
            return datetime.strptime(ts.split("+")[0], "%Y-%m-%dT%H:%M:%S.%f")

    def _read_audit_log(self) -> Iterator[Dict[str, Any]]:
        """Read entries from the audit log file."""
        if not self.audit_file.exists():
            return

        try:
            with open(self.audit_file, 'r') as f:
                # Use shared lock for reading
                fcntl.flock(f.fileno(), fcntl.LOCK_SH)
                try:
                    for line in f:
                        line = line.strip()
                        if not line:
                            continue
                        try:
                            entry = json.loads(line)
                            entry["_source"] = "audit"
                            yield entry
                        except json.JSONDecodeError:
                            continue
                finally:
                    fcntl.flock(f.fileno(), fcntl.LOCK_UN)
        except Exception as e:
            print(f"⚠️  Error reading audit log: {e}", file=sys.stderr)

    def _read_structured_logs(self) -> Iterator[Dict[str, Any]]:
        """Read entries from structured log files."""
        if not self.structured_logs_dir.exists():
            return

        for log_file in sorted(self.structured_logs_dir.glob("*.jsonl")):
            try:
                with open(log_file, 'r') as f:
                    for line in f:
                        line = line.strip()
                        if not line:
                            continue
                        try:
                            entry = json.loads(line)
                            entry["_source"] = "structured"
                            entry["_log_file"] = log_file.name
                            yield entry
                        except json.JSONDecodeError:
                            continue
            except Exception as e:
                print(f"⚠️  Error reading structured log {log_file}: {e}", file=sys.stderr)

    def _matches_filters(self, entry: Dict[str, Any],
                         level: Optional[str] = None,
                         team_id: Optional[int] = None,
                         user: Optional[str] = None,
                         action: Optional[str] = None,
                         operation_type: Optional[str] = None,
                         since: Optional[datetime] = None,
                         until: Optional[datetime] = None,
                         text_search: Optional[str] = None) -> bool:
        """Check if entry matches all filters."""
        # Level filter (check both 'level' and nested in details)
        if level:
            entry_level = entry.get("level", "").upper()
            if entry_level != level.upper():
                # Check in details for structured logs
                details_level = entry.get("details", {}).get("level", "").upper()
                if details_level != level.upper():
                    return False

        # Team ID filter
        if team_id is not None:
            entry_team_id = entry.get("details", {}).get("team_id")
            if entry_team_id != team_id:
                return False

        # User filter
        if user:
            entry_user = entry.get("user", "")
            if entry_user != user:
                return False

        # Action filter
        if action:
            entry_action = entry.get("action", "")
            if entry_action != action:
                return False

        # Operation type filter (structured logs)
        if operation_type:
            entry_op = entry.get("details", {}).get("operation", "")
            if entry_op != operation_type:
                return False

        # Time range filters
        if since or until:
            ts_str = entry.get("timestamp", "")
            if not ts_str:
                return False
            try:
                entry_time = self._parse_timestamp(ts_str)
                if since and entry_time < since:
                    return False
                if until and entry_time > until:
                    return False
            except (ValueError, TypeError):
                return False

        # Text search
        if text_search:
            entry_text = json.dumps(entry).lower()
            if text_search.lower() not in entry_text:
                return False

        return True

    def query(self,
              level: Optional[str] = None,
              team_id: Optional[int] = None,
              user: Optional[str] = None,
              action: Optional[str] = None,
              operation_type: Optional[str] = None,
              since: Optional[datetime] = None,
              until: Optional[datetime] = None,
              text_search: Optional[str] = None,
              limit: int = 1000,
              sources: Optional[List[str]] = None) -> List[Dict[str, Any]]:
        """Query logs with filters.

        Args:
            level: Filter by log level (DEBUG, INFO, WARN, ERROR)
            team_id: Filter by team ID
            user: Filter by user name
            action: Filter by action type
            operation_type: Filter by operation type
            since: Filter entries after this time
            until: Filter entries before this time
            text_search: Free-text search in entries
            limit: Maximum entries to return
            sources: List of sources to include (audit, structured)

        Returns:
            List of matching log entries
        """
        results = []
        sources = sources or ["audit", "structured"]

        # Read from audit log if requested
        if "audit" in sources:
            for entry in self._read_audit_log():
                if self._matches_filters(entry, level, team_id, user, action,
                                        operation_type, since, until, text_search):
                    results.append(entry)
                    if len(results) >= limit:
                        return results

        # Read from structured logs if requested
        if "structured" in sources:
            for entry in self._read_structured_logs():
                if self._matches_filters(entry, level, team_id, user, action,
                                        operation_type, since, until, text_search):
                    results.append(entry)
                    if len(results) >= limit:
                        return results

        # Sort by timestamp
        results.sort(key=lambda x: x.get("timestamp", ""))
        return results

    def get_summary(self) -> Dict[str, Any]:
        """Get summary statistics for logs."""
        summary = {
            "audit_log": {"entries": 0, "actions": set(), "users": set()},
            "structured_logs": {"entries": 0, "levels": {}, "events": set()},
            "time_range": {"oldest": None, "newest": None}
        }

        # Analyze audit log
        for entry in self._read_audit_log():
            summary["audit_log"]["entries"] += 1
            summary["audit_log"]["actions"].add(entry.get("action", "unknown"))
            summary["audit_log"]["users"].add(entry.get("user", "unknown"))

            ts = entry.get("timestamp")
            if ts:
                if summary["time_range"]["oldest"] is None or ts < summary["time_range"]["oldest"]:
                    summary["time_range"]["oldest"] = ts
                if summary["time_range"]["newest"] is None or ts > summary["time_range"]["newest"]:
                    summary["time_range"]["newest"] = ts

        # Analyze structured logs
        for entry in self._read_structured_logs():
            summary["structured_logs"]["entries"] += 1
            level = entry.get("level", "unknown")
            summary["structured_logs"]["levels"][level] = summary["structured_logs"]["levels"].get(level, 0) + 1
            summary["structured_logs"]["events"].add(entry.get("event", "unknown"))

            ts = entry.get("timestamp")
            if ts:
                if summary["time_range"]["oldest"] is None or ts < summary["time_range"]["oldest"]:
                    summary["time_range"]["oldest"] = ts
                if summary["time_range"]["newest"] is None or ts > summary["time_range"]["newest"]:
                    summary["time_range"]["newest"] = ts

        # Convert sets to lists for JSON serialization
        summary["audit_log"]["actions"] = list(summary["audit_log"]["actions"])
        summary["audit_log"]["users"] = list(summary["audit_log"]["users"])
        summary["structured_logs"]["events"] = list(summary["structured_logs"]["events"])

        return summary

    def export_csv(self, entries: List[Dict[str, Any]], output_path: Path) -> bool:
        """Export entries to CSV format."""
        if not entries:
            print("No entries to export")
            return False

        # Flatten entries for CSV
        flattened = []
        for entry in entries:
            flat = {
                "timestamp": entry.get("timestamp", ""),
                "level": entry.get("level", entry.get("details", {}).get("level", "")),
                "source": entry.get("_source", ""),
                "user": entry.get("user", ""),
                "action": entry.get("action", entry.get("event", "")),
                "team_id": entry.get("details", {}).get("team_id", ""),
                "details": json.dumps(entry.get("details", {}))
            }
            flattened.append(flat)

        try:
            with open(output_path, 'w', newline='') as f:
                writer = csv.DictWriter(f, fieldnames=flattened[0].keys())
                writer.writeheader()
                writer.writerows(flattened)
            return True
        except Exception as e:
            print(f"❌ Export failed: {e}", file=sys.stderr)
            return False


def parse_time_expr(expr: str) -> datetime:
    """Parse time expression like '1h', '30m', '1d'."""
    expr = expr.lower().strip()
    now = datetime.utcnow()

    # Try absolute ISO format first
    try:
        return datetime.fromisoformat(expr.replace("Z", "+00:00").replace("z", "+00:00"))
    except ValueError:
        pass

    # Try relative time expressions
    match = re.match(r'^(\d+)([hmd])$', expr)
    if match:
        value, unit = int(match.group(1)), match.group(2)
        if unit == 'h':
            return now - timedelta(hours=value)
        elif unit == 'm':
            return now - timedelta(minutes=value)
        elif unit == 'd':
            return now - timedelta(days=value)

    raise ValueError(f"Invalid time expression: {expr}. Use ISO8601 or relative (e.g., 1h, 30m, 1d)")


def main():
    parser = argparse.ArgumentParser(
        description="Log Aggregator - Query and analyze team operation logs"
    )
    parser.add_argument("--project", required=True, help="Project name")
    parser.add_argument("--teams-dir", type=Path, default=Path(".teams"),
                        help="Directory containing team data (default: .teams)")

    subparsers = parser.add_subparsers(dest="command", help="Command to run")

    # Query command
    query_parser = subparsers.add_parser("query", help="Query logs with filters")
    query_parser.add_argument("--since", help="Filter entries after this time (e.g., 1h, 30m, ISO8601)")
    query_parser.add_argument("--until", help="Filter entries before this time")
    query_parser.add_argument("--team", type=int, help="Filter by team ID")
    query_parser.add_argument("--user", help="Filter by user name")
    query_parser.add_argument("--action", help="Filter by action type")
    query_parser.add_argument("--level", choices=["DEBUG", "INFO", "WARN", "ERROR"],
                              help="Filter by log level")
    query_parser.add_argument("--operation-type", help="Filter by operation type")
    query_parser.add_argument("--search", help="Free-text search in log entries")
    query_parser.add_argument("--limit", type=int, default=100,
                              help="Maximum entries to show (default: 100)")
    query_parser.add_argument("--source", choices=["audit", "structured", "all"],
                              default="all", help="Log sources to query")

    # Summary command
    summary_parser = subparsers.add_parser("summary", help="Show log summary statistics")

    # Export command
    export_parser = subparsers.add_parser("export", help="Export logs to file")
    export_parser.add_argument("--since", help="Filter entries after this time")
    export_parser.add_argument("--team", type=int, help="Filter by team ID")
    export_parser.add_argument("--user", help="Filter by user name")
    export_parser.add_argument("--format", choices=["json", "csv"], default="json",
                               help="Export format (default: json)")
    export_parser.add_argument("--output", required=True, help="Output file path")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(1)

    aggregator = LogAggregator(args.project, args.teams_dir)

    if args.command == "query":
        # Parse time filters
        since = None
        until = None
        if args.since:
            try:
                since = parse_time_expr(args.since)
            except ValueError as e:
                print(f"❌ {e}", file=sys.stderr)
                sys.exit(1)
        if args.until:
            try:
                until = parse_time_expr(args.until)
            except ValueError as e:
                print(f"❌ {e}", file=sys.stderr)
                sys.exit(1)

        # Determine sources
        sources = None
        if args.source == "audit":
            sources = ["audit"]
        elif args.source == "structured":
            sources = ["structured"]

        results = aggregator.query(
            level=args.level,
            team_id=args.team,
            user=args.user,
            action=args.action,
            operation_type=args.operation_type,
            since=since,
            until=until,
            text_search=args.search,
            limit=args.limit,
            sources=sources
        )

        if results:
            print(f"\n📋 Found {len(results)} log entries:\n")
            for entry in results:
                ts = entry.get("timestamp", "unknown")
                source = entry.get("_source", "unknown")
                level = entry.get("level", entry.get("details", {}).get("level", ""))
                user = entry.get("user", entry.get("details", {}).get("user", "system"))
                action = entry.get("action", entry.get("event", "unknown"))

                # Format output
                level_tag = f"[{level}]" if level else ""
                print(f"[{ts}] {level_tag} [{source}] {action}")
                if user and user != "system":
                    print(f"  User: {user}")

                # Show team_id if present
                team_id = entry.get("details", {}).get("team_id")
                if team_id:
                    print(f"  Team: {team_id}")

                # Show truncated details
                details = entry.get("details", {})
                if details:
                    # Filter out internal fields
                    display_details = {k: v for k, v in details.items()
                                       if k not in ["user", "level", "team_id"]}
                    if display_details:
                        detail_str = json.dumps(display_details)[:80]
                        if len(json.dumps(display_details)) > 80:
                            detail_str += "..."
                        print(f"  Details: {detail_str}")
                print()
        else:
            print("ℹ️  No log entries found matching the criteria")

    elif args.command == "summary":
        summary = aggregator.get_summary()

        print("\n📊 Log Summary")
        print("=" * 50)

        print(f"\nAudit Log:")
        print(f"  Entries: {summary['audit_log']['entries']}")
        print(f"  Unique Actions: {len(summary['audit_log']['actions'])}")
        print(f"  Unique Users: {len(summary['audit_log']['users'])}")

        print(f"\nStructured Logs:")
        print(f"  Entries: {summary['structured_logs']['entries']}")
        print(f"  Levels: {dict(summary['structured_logs']['levels'])}")
        print(f"  Unique Events: {len(summary['structured_logs']['events'])}")

        print(f"\nTime Range:")
        if summary['time_range']['oldest']:
            print(f"  Oldest: {summary['time_range']['oldest']}")
        if summary['time_range']['newest']:
            print(f"  Newest: {summary['time_range']['newest']}")

    elif args.command == "export":
        # Parse time filters
        since = None
        if args.since:
            try:
                since = parse_time_expr(args.since)
            except ValueError as e:
                print(f"❌ {e}", file=sys.stderr)
                sys.exit(1)

        results = aggregator.query(
            team_id=args.team,
            user=args.user,
            since=since,
            limit=10000  # Export more entries
        )

        output_path = Path(args.output)

        if args.format == "json":
            try:
                with open(output_path, 'w') as f:
                    json.dump(results, f, indent=2)
                print(f"✅ Exported {len(results)} entries to {output_path}")
            except Exception as e:
                print(f"❌ Export failed: {e}", file=sys.stderr)
                sys.exit(1)
        else:  # csv
            if aggregator.export_csv(results, output_path):
                print(f"✅ Exported {len(results)} entries to {output_path}")
            else:
                sys.exit(1)


if __name__ == "__main__":
    main()
