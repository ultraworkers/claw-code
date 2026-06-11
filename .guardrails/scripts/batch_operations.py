#!/usr/bin/env python3
"""
Batch Operations Module for Team Manager (FUNC-005)

Provides CSV and JSON import/export functionality for bulk team operations.
"""

import csv
import json
import io
from pathlib import Path
from typing import List, Dict, Any, Optional, Tuple
from dataclasses import asdict
from datetime import datetime


def import_csv(manager, csv_path: Path, dry_run: bool = False) -> Dict[str, Any]:
    """
    Import role assignments from a CSV file.

    Args:
        manager: TeamManager instance
        csv_path: Path to CSV file
        dry_run: If True, validate without making changes

    Returns:
        Dict with import results:
        {
            "success": bool,
            "imported": int,
            "skipped": int,
            "errors": List[Dict],
            "dry_run": bool
        }
    """
    result = {
        "success": True,
        "imported": 0,
        "skipped": 0,
        "errors": [],
        "dry_run": dry_run
    }

    if not csv_path.exists():
        result["success"] = False
        result["errors"].append({
            "row": 0,
            "error": f"File not found: {csv_path}"
        })
        return result

    # Validate file is UTF-8
    try:
        with open(csv_path, 'r', encoding='utf-8') as f:
            content = f.read()
    except UnicodeDecodeError as e:
        result["success"] = False
        result["errors"].append({
            "row": 0,
            "error": f"File is not valid UTF-8: {e}"
        })
        return result

    # Parse CSV
    reader = csv.DictReader(io.StringIO(content))
    expected_columns = {'team_id', 'role_name', 'assignee'}

    if not reader.fieldname:
        result["success"] = False
        result["errors"].append({
            "row": 0,
            "error": "CSV file is empty or has no header row"
        })
        return result

    # Check for required columns
    missing = expected_columns - set(reader.fieldname)
    if missing:
        result["success"] = False
        result["errors"].append({
            "row": 0,
            "error": f"Missing required columns: {', '.join(missing)}"
        })
        return result

    # Process rows
    rows = list(reader)

    # First pass: validate all rows
    validated_rows = []
    for i, row in enumerate(rows, start=2):  # Start at 2 (1 for header, 1 for 0-index)
        validation = _validate_csv_row(manager, row, i)
        if validation["valid"]:
            validated_rows.append((i, row, validation))
        else:
            result["errors"].append({
                "row": i,
                "data": row,
                "error": validation["error"]
            })

    if result["errors"]:
        result["success"] = False
        if not dry_run:
            return result

    # Second pass: apply changes (unless dry run)
    if not dry_run:
        for row_num, row, validation in validated_rows:
            try:
                team_id = int(row['team_id'])
                role_name = row['role_name'].strip()
                assignee = row['assignee'].strip()

                success = manager.assign_role(team_id, role_name, assignee)
                if success:
                    result["imported"] += 1
                else:
                    result["skipped"] += 1
                    result["errors"].append({
                        "row": row_num,
                        "data": row,
                        "error": "assign_role returned False"
                    })
            except Exception as e:
                result["errors"].append({
                    "row": row_num,
                    "data": row,
                    "error": str(e)
                })

    return result


def _validate_csv_row(manager, row: Dict[str, str], row_num: int) -> Dict[str, Any]:
    """Validate a single CSV row."""
    result = {"valid": True, "error": None}

    # Check required fields
    if not row.get('team_id', '').strip():
        result["valid"] = False
        result["error"] = "team_id is required"
        return result

    if not row.get('role_name', '').strip():
        result["valid"] = False
        result["error"] = "role_name is required"
        return result

    if not row.get('assignee', '').strip():
        result["valid"] = False
        result["error"] = "assignee is required"
        return result

    # Validate team_id is integer
    try:
        team_id = int(row['team_id'].strip())
    except ValueError:
        result["valid"] = False
        result["error"] = f"team_id must be an integer, got: {row['team_id']}"
        return result

    # Check team exists
    if team_id not in manager.teams:
        result["valid"] = False
        result["error"] = f"Team {team_id} does not exist"
        return result

    # Check role exists in team
    team = manager.teams[team_id]
    role_name = row['role_name'].strip()
    valid_roles = [r.name for r in team.roles]
    if role_name not in valid_roles:
        result["valid"] = False
        result["error"] = f"Role '{role_name}' not found in team {team_id}. Valid roles: {', '.join(valid_roles)}"
        return result

    return result


def export_csv(manager, csv_path: Path) -> Dict[str, Any]:
    """
    Export all role assignments to a CSV file.

    Args:
        manager: TeamManager instance
        csv_path: Path for output CSV file

    Returns:
        Dict with export results
    """
    result = {
        "success": True,
        "exported": 0,
        "file_path": str(csv_path),
        "errors": []
    }

    try:
        with open(csv_path, 'w', newline='', encoding='utf-8') as f:
            writer = csv.writer(f)
            # Header
            writer.writerow(['team_id', 'team_name', 'role_name', 'assigned_to', 'status'])

            # Data rows
            for team in sorted(manager.teams.values(), key=lambda t: t.id):
                for role in team.roles:
                    writer.writerow([
                        team.id,
                        team.name,
                        role.name,
                        role.assigned_to or '',
                        team.status
                    ])
                    result["exported"] += 1

    except Exception as e:
        result["success"] = False
        result["errors"].append(str(e))

    return result


def import_json(manager, json_path: Path, dry_run: bool = False) -> Dict[str, Any]:
    """
    Import role assignments from a JSON file.

    Args:
        manager: TeamManager instance
        json_path: Path to JSON file
        dry_run: If True, validate without making changes

    Returns:
        Dict with import results
    """
    result = {
        "success": True,
        "imported": 0,
        "skipped": 0,
        "errors": [],
        "dry_run": dry_run
    }

    if not json_path.exists():
        result["success"] = False
        result["errors"].append(f"File not found: {json_path}")
        return result

    # Load JSON
    try:
        with open(json_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
    except json.JSONDecodeError as e:
        result["success"] = False
        result["errors"].append(f"Invalid JSON: {e}")
        return result
    except Exception as e:
        result["success"] = False
        result["errors"].append(f"Error reading file: {e}")
        return result

    # Handle different JSON formats
    if isinstance(data, dict):
        if "assignments" in data:
            assignments = data["assignments"]
        else:
            result["success"] = False
            result["errors"].append("JSON must contain 'assignments' array or be an array")
            return result
    elif isinstance(data, list):
        assignments = data
    else:
        result["success"] = False
        result["errors"].append("JSON must be an array or object with 'assignments' key")
        return result

    # Validate all entries first (atomic operation)
    validated = []
    for i, entry in enumerate(assignments):
        validation = _validate_json_entry(manager, entry, i)
        if validation["valid"]:
            validated.append((i, entry))
        else:
            result["errors"].append({
                "index": i,
                "entry": entry,
                "error": validation["error"]
            })

    if result["errors"]:
        result["success"] = False
        if not dry_run:
            return result

    # Apply changes (unless dry run)
    if not dry_run:
        for index, entry in validated:
            try:
                team_id = int(entry['team_id'])
                role_name = entry['role_name']
                assignee = entry['assignee']

                success = manager.assign_role(team_id, role_name, assignee)
                if success:
                    result["imported"] += 1
                else:
                    result["skipped"] += 1
            except Exception as e:
                result["errors"].append({
                    "index": index,
                    "entry": entry,
                    "error": str(e)
                })

    return result


def _validate_json_entry(manager, entry: Dict, index: int) -> Dict[str, Any]:
    """Validate a single JSON entry."""
    result = {"valid": True, "error": None}

    if not isinstance(entry, dict):
        result["valid"] = False
        result["error"] = f"Entry must be an object, got {type(entry).__name__}"
        return result

    # Check required fields
    required = ['team_id', 'role_name', 'assignee']
    missing = [f for f in required if f not in entry]
    if missing:
        result["valid"] = False
        result["error"] = f"Missing required fields: {', '.join(missing)}"
        return result

    # Validate team_id
    try:
        team_id = int(entry['team_id'])
    except (ValueError, TypeError):
        result["valid"] = False
        result["error"] = f"team_id must be an integer, got: {entry['team_id']}"
        return result

    # Check team exists
    if team_id not in manager.teams:
        result["valid"] = False
        result["error"] = f"Team {team_id} does not exist"
        return result

    # Check role exists
    team = manager.teams[team_id]
    role_name = entry['role_name']
    valid_roles = [r.name for r in team.roles]
    if role_name not in valid_roles:
        result["valid"] = False
        result["error"] = f"Role '{role_name}' not found in team {team_id}"
        return result

    return result


def export_json(manager, json_path: Path, pretty: bool = True) -> Dict[str, Any]:
    """
    Export full project state to a JSON file.

    Args:
        manager: TeamManager instance
        json_path: Path for output JSON file
        pretty: Whether to pretty-print JSON

    Returns:
        Dict with export results
    """
    result = {
        "success": True,
        "file_path": str(json_path),
        "team_count": len(manager.teams),
        "errors": []
    }

    data = {
        "project_name": manager.project_name,
        "version": getattr(manager, '_data_version', '1.0.0'),  # OPS-005: Include version
        "exported_at": datetime.now().isoformat(),
        "team_count": len(manager.teams),
        "teams": []
    }

    for team in sorted(manager.teams.values(), key=lambda t: t.id):
        team_data = {
            "id": team.id,
            "name": team.name,
            "phase": team.phase,
            "description": team.description,
            "status": team.status,
            "started_at": team.started_at,
            "completed_at": team.completed_at,
            "exit_criteria": team.exit_criteria,
            "roles": [
                {
                    "name": role.name,
                    "responsibility": role.responsibility,
                    "deliverables": role.deliverables,
                    "assigned_to": role.assigned_to
                }
                for role in team.roles
            ]
        }
        data["teams"].append(team_data)

    try:
        with open(json_path, 'w', encoding='utf-8') as f:
            indent = 2 if pretty else None
            json.dump(data, f, indent=indent, ensure_ascii=False)
    except Exception as e:
        result["success"] = False
        result["errors"].append(str(e))

    return result


# For CLI integration
def create_csv_template(csv_path: Path) -> Dict[str, Any]:
    """
    Create a CSV template file for bulk assignments.

    Args:
        csv_path: Path for template file

    Returns:
        Dict with result
    """
    result = {
        "success": True,
        "file_path": str(csv_path),
        "errors": []
    }

    try:
        with open(csv_path, 'w', newline='', encoding='utf-8') as f:
            writer = csv.writer(f)
            writer.writerow(['team_id', 'role_name', 'assignee'])
            # Add example row
            writer.writerow([1, 'Business Relationship Manager', 'John Doe'])
            writer.writerow([7, 'Technical Lead', 'Jane Smith'])
    except Exception as e:
        result["success"] = False
        result["errors"].append(str(e))

    return result


def create_json_template(json_path: Path) -> Dict[str, Any]:
    """
    Create a JSON template file for bulk assignments.

    Args:
        json_path: Path for template file

    Returns:
        Dict with result
    """
    result = {
        "success": True,
        "file_path": str(json_path),
        "errors": []
    }

    template = {
        "description": "Bulk role assignments",
        "assignments": [
            {
                "team_id": 1,
                "role_name": "Business Relationship Manager",
                "assignee": "John Doe"
            },
            {
                "team_id": 7,
                "role_name": "Technical Lead",
                "assignee": "Jane Smith"
            }
        ]
    }

    try:
        with open(json_path, 'w', encoding='utf-8') as f:
            json.dump(template, f, indent=2)
    except Exception as e:
        result["success"] = False
        result["errors"].append(str(e))

    return result
