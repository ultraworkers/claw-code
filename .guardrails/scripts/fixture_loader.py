#!/usr/bin/env python3
"""
Fixture loader utility for test data management.

Provides functions to load, copy, and create test fixtures for TeamManager testing.
"""

import json
import shutil
import tempfile
from pathlib import Path
from typing import Dict, Any, Optional, List
import sys
import os

# Add parent directory to path
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

FIXTURES_DIR = Path(".teams/fixtures")


class FixtureLoader:
    """Utility class for loading and managing test fixtures."""

    AVAILABLE_FIXTURES = [
        "minimal-project",
        "full-project",
        "phase1-complete",
        "partial-assignments",
        "empty-project",
    ]

    def __init__(self, fixtures_dir: Path = None):
        """
        Initialize the fixture loader.

        Args:
            fixtures_dir: Directory containing fixture files (default: .teams/fixtures)
        """
        self.fixtures_dir = fixtures_dir or FIXTURES_DIR

    def list_fixtures(self) -> List[str]:
        """List all available fixtures."""
        return self.AVAILABLE_FIXTURES.copy()

    def load_fixture(self, name: str) -> Dict[str, Any]:
        """
        Load a fixture by name.

        Args:
            name: Fixture name (e.g., "minimal-project")

        Returns:
            Dict containing the fixture data

        Raises:
            FileNotFoundError: If fixture doesn't exist
            ValueError: If name is invalid
        """
        if not name or not isinstance(name, str):
            raise ValueError("Fixture name must be a non-empty string")

        # Sanitize name to prevent path traversal
        safe_name = name.replace("..", "").replace("/", "").replace("\\", "")
        fixture_path = self.fixtures_dir / f"{safe_name}.json"

        if not fixture_path.exists():
            available = ", ".join(self.AVAILABLE_FIXTURES)
            raise FileNotFoundError(
                f"Fixture '{name}' not found at {fixture_path}. "
                f"Available fixtures: {available}"
            )

        with open(fixture_path, 'r', encoding='utf-8') as f:
            return json.load(f)

    def load_fixture_to_temp(self, name: str, project_name: Optional[str] = None) -> Path:
        """
        Load a fixture to a temporary directory for test isolation.

        Args:
            name: Fixture name
            project_name: Optional new project name (default: use fixture's project_name)

        Returns:
            Path to the temporary directory containing the fixture
        """
        data = self.load_fixture(name)

        # Use provided project name or fixture's name
        if project_name:
            data["project_name"] = project_name

        # Create temp directory
        temp_dir = Path(tempfile.mkdtemp(prefix=f"fixture_{name}_"))
        config_path = temp_dir / f"{data['project_name']}.json"

        # Write fixture to temp location
        with open(config_path, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2)

        return temp_dir

    def copy_fixture(self, name: str, dest_dir: Path, new_project_name: Optional[str] = None) -> Path:
        """
        Copy a fixture to a destination directory.

        Args:
            name: Fixture name
            dest_dir: Destination directory
            new_project_name: Optional new project name

        Returns:
            Path to the copied fixture file
        """
        data = self.load_fixture(name)

        # Use new project name if provided
        project_name = new_project_name or data["project_name"]
        dest_path = dest_dir / f"{project_name}.json"

        # Update project name in data
        if new_project_name:
            data["project_name"] = new_project_name
            data["updated_at"] = self._get_timestamp()

        # Ensure destination exists
        dest_dir.mkdir(parents=True, exist_ok=True)

        # Write fixture
        with open(dest_path, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2)

        return dest_path

    def create_custom_fixture(self, project_name: str, teams_data: List[Dict],
                              dest_dir: Optional[Path] = None) -> Path:
        """
        Create a custom fixture programmatically.

        Args:
            project_name: Name for the project
            teams_data: List of team data dicts
            dest_dir: Optional destination directory (default: fixtures_dir)

        Returns:
            Path to the created fixture file
        """
        fixture_data = {
            "project_name": project_name,
            "updated_at": self._get_timestamp(),
            "teams": teams_data
        }

        dest_dir = dest_dir or self.fixtures_dir
        dest_path = dest_dir / f"{project_name}.json"

        # Ensure destination exists
        dest_dir.mkdir(parents=True, exist_ok=True)

        with open(dest_path, 'w', encoding='utf-8') as f:
            json.dump(fixture_data, f, indent=2)

        return dest_path

    def create_minimal_team(self, team_id: int, name: str, phase: str) -> Dict[str, Any]:
        """
        Create a minimal team data structure for custom fixtures.

        Args:
            team_id: Team ID
            name: Team name
            phase: Phase name

        Returns:
            Team data dict
        """
        return {
            "id": team_id,
            "name": name,
            "phase": phase,
            "description": f"Team {team_id} description",
            "status": "not_started",
            "started_at": None,
            "completed_at": None,
            "roles": [],
            "exit_criteria": ["Exit criterion 1"]
        }

    def create_minimal_role(self, name: str, assigned_to: Optional[str] = None) -> Dict[str, Any]:
        """
        Create a minimal role data structure.

        Args:
            name: Role name
            assigned_to: Optional assignee name

        Returns:
            Role data dict
        """
        return {
            "name": name,
            "responsibility": f"{name} responsibilities",
            "deliverables": ["Deliverable 1"],
            "assigned_to": assigned_to
        }

    def _get_timestamp(self) -> str:
        """Get current ISO timestamp."""
        from datetime import datetime
        return datetime.now().isoformat()

    def validate_fixture(self, data: Dict[str, Any]) -> List[str]:
        """
        Validate fixture data structure.

        Args:
            data: Fixture data dict

        Returns:
            List of validation errors (empty if valid)
        """
        errors = []

        if "project_name" not in data:
            errors.append("Missing 'project_name' field")

        if "teams" not in data:
            errors.append("Missing 'teams' field")
        elif not isinstance(data["teams"], list):
            errors.append("'teams' must be a list")
        else:
            team_ids = set()
            for i, team in enumerate(data["teams"]):
                if "id" not in team:
                    errors.append(f"Team {i}: Missing 'id' field")
                elif team["id"] in team_ids:
                    errors.append(f"Team {i}: Duplicate team ID {team['id']}")
                else:
                    team_ids.add(team["id"])

                if "name" not in team:
                    errors.append(f"Team {team.get('id', i)}: Missing 'name' field")

                if "roles" not in team:
                    errors.append(f"Team {team.get('id', i)}: Missing 'roles' field")

        return errors


def load_fixture(name: str) -> Dict[str, Any]:
    """
    Convenience function to load a fixture.

    Args:
        name: Fixture name

    Returns:
        Fixture data dict
    """
    loader = FixtureLoader()
    return loader.load_fixture(name)


def load_fixture_to_temp(name: str, project_name: Optional[str] = None) -> Path:
    """
    Convenience function to load a fixture to a temp directory.

    Args:
        name: Fixture name
        project_name: Optional new project name

    Returns:
        Path to temp directory
    """
    loader = FixtureLoader()
    return loader.load_fixture_to_temp(name, project_name)


def copy_fixture(name: str, dest_dir: Path, new_project_name: Optional[str] = None) -> Path:
    """
    Convenience function to copy a fixture.

    Args:
        name: Fixture name
        dest_dir: Destination directory
        new_project_name: Optional new project name

    Returns:
        Path to copied fixture
    """
    loader = FixtureLoader()
    return loader.copy_fixture(name, dest_dir, new_project_name)


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Fixture loader utility")
    parser.add_argument("command", choices=["list", "load", "validate"],
                        help="Command to execute")
    parser.add_argument("--fixture", help="Fixture name")
    parser.add_argument("--project-name", help="New project name (for copy)")

    args = parser.parse_args()

    loader = FixtureLoader()

    if args.command == "list":
        print("Available fixtures:")
        for name in loader.list_fixtures():
            print(f"  - {name}")

    elif args.command == "load":
        if not args.fixture:
            print("Error: --fixture is required")
            sys.exit(1)
        data = loader.load_fixture(args.fixture)
        print(json.dumps(data, indent=2))

    elif args.command == "validate":
        if not args.fixture:
            print("Error: --fixture is required")
            sys.exit(1)
        data = loader.load_fixture(args.fixture)
        errors = loader.validate_fixture(data)
        if errors:
            print(f"Validation errors for '{args.fixture}':")
            for error in errors:
                print(f"  - {error}")
            sys.exit(1)
        else:
            print(f"Fixture '{args.fixture}' is valid")
