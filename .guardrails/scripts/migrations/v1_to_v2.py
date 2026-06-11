#!/usr/bin/env python3
"""Example migration from v1.0.0 to v2.0.0 (OPS-006).

This is a template showing how to create migration scripts.
Copy and modify this file for actual migrations.

Migration naming: v<old>_to_v<new>.py
Example: v1_to_v2.py for 1.0.0 -> 2.0.0
"""

from typing import Any, Dict
from datetime import datetime


def migrate(data: Dict[str, Any]) -> Dict[str, Any]:
    """Migrate project data from v1.0.0 to v2.0.0.

    Args:
        data: The project data dict to migrate

    Returns:
        Migrated data dict
    """
    # Example migrations:

    # 1. Add new field to all teams
    # for team in data.get("teams", []):
    #     if "new_field" not in team:
    #         team["new_field"] = "default_value"

    # 2. Rename a field
    # for team in data.get("teams", []):
    #     if "old_name" in team:
    #         team["new_name"] = team.pop("old_name")

    # 3. Restructure data
    # if "flat_field" in data:
    #     data["nested_structure"] = {"field": data.pop("flat_field")}

    # 4. Update role names
    # role_mapping = {"Old Role": "New Role"}
    # for team in data.get("teams", []):
    #     for role in team.get("roles", []):
    #         if role["name"] in role_mapping:
    #             role["name"] = role_mapping[role["name"]]

    # Update version
    data["version"] = "2.0.0"
    data["migrated_at"] = datetime.now().isoformat()

    return data


if __name__ == "__main__":
    # Test migration
    test_data = {
        "version": "1.0.0",
        "project_name": "test",
        "teams": []
    }
    result = migrate(test_data)
    print(f"Migrated: {result}")
