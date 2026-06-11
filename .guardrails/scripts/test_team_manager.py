#!/usr/bin/env python3
"""
Unit tests for TeamManager class in team_manager.py

Run with: python -m pytest scripts/test_team_manager.py -v
Or: python scripts/test_team_manager.py
"""

import json
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

# Add parent directory to path to import team_manager
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from scripts.team_manager import (
    TeamManager, Role, Team, validate_project_name,
    UserContext, StructuredLogger, PermissionDenied
)


def create_admin_context() -> UserContext:
    """Create an admin user context for testing."""
    return UserContext(user_id="test-admin", role="admin")


def create_team_lead_context(team_id: int = None) -> UserContext:
    """Create a team-lead user context for testing."""
    return UserContext(user_id="test-lead", role="team-lead", team_id=team_id)


def create_test_logger() -> StructuredLogger:
    """Create a test logger."""
    return StructuredLogger("test", request_id="test-123")


class TestValidateProjectName(unittest.TestCase):
    """Tests for validate_project_name function."""

    def test_valid_names(self):
        """Test valid project names."""
        valid_names = [
            "my-project",
            "my_project",
            "project123",
            "ProjectName",
            "a",
            "A" * 64,
        ]
        for name in valid_names:
            with self.subTest(name=name):
                # Should not raise
                validate_project_name(name)

    def test_empty_name(self):
        """Test empty project name raises error."""
        with self.assertRaises(ValueError) as ctx:
            validate_project_name("")
        self.assertIn("required", str(ctx.exception).lower())

    def test_none_name(self):
        """Test None project name raises error."""
        with self.assertRaises((ValueError, TypeError)) as ctx:
            validate_project_name(None)

    def test_name_too_long(self):
        """Test project name over 64 characters raises error."""
        with self.assertRaises(ValueError) as ctx:
            validate_project_name("a" * 65)
        self.assertIn("64", str(ctx.exception))

    def test_invalid_characters(self):
        """Test invalid characters in project name raise error."""
        invalid_names = [
            "project with spaces",
            "project;rm -rf",
            "project/test",
            "project.json",
            "project$(whoami)",
            "project|cat /etc/passwd",
            "project`id`",
            "project<script>",
        ]
        for name in invalid_names:
            with self.subTest(name=name):
                with self.assertRaises(ValueError):
                    validate_project_name(name)


class TestUserContext(unittest.TestCase):
    """Tests for UserContext RBAC."""

    def test_admin_has_permission(self):
        """Test admin has all permissions."""
        ctx = create_admin_context()
        self.assertTrue(ctx.has_permission("viewer"))
        self.assertTrue(ctx.has_permission("team-lead"))
        self.assertTrue(ctx.has_permission("admin"))

    def test_team_lead_permissions(self):
        """Test team-lead has correct permissions."""
        ctx = create_team_lead_context()
        self.assertTrue(ctx.has_permission("viewer"))
        self.assertTrue(ctx.has_permission("team-lead"))
        self.assertFalse(ctx.has_permission("admin"))

    def test_invalid_role(self):
        """Test invalid role raises ValueError."""
        with self.assertRaises(ValueError):
            UserContext(user_id="test", role="invalid-role")

    def test_can_modify_team_admin(self):
        """Test admin can modify any team."""
        ctx = create_admin_context()
        for team_id in range(1, 13):
            self.assertTrue(ctx.can_modify_team(team_id))

    def test_can_modify_team_team_lead(self):
        """Test team-lead can only modify their team."""
        ctx = create_team_lead_context(team_id=5)
        self.assertTrue(ctx.can_modify_team(5))
        self.assertFalse(ctx.can_modify_team(3))


class TestTeamManagerInitialization(unittest.TestCase):
    """Tests for TeamManager initialization."""

    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.config_path = Path(self.temp_dir) / "test-project.json"
        self.user_ctx = create_admin_context()
        self.logger = create_test_logger()

    def tearDown(self):
        """Clean up test fixtures."""
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_init_basic(self):
        """Test basic initialization."""
        manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        self.assertEqual(manager.project_name, "test-project")
        self.assertEqual(manager.teams, {})

    def test_init_creates_config_path(self):
        """Test that initialization uses correct config path."""
        manager = TeamManager("test-project", user_context=self.user_ctx, logger=self.logger)
        self.assertEqual(manager.config_path, Path(".teams/test-project.json"))

    def test_init_requires_auth(self):
        """Test that operations require authentication."""
        manager = TeamManager("test-project", self.config_path, None, self.logger)
        with self.assertRaises(PermissionDenied):
            manager.initialize_project()


class TestTeamManagerInitializeProject(unittest.TestCase):
    """Tests for initialize_project method."""

    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.config_path = Path(self.temp_dir) / "test-project.json"
        self.user_ctx = create_admin_context()
        self.logger = create_test_logger()
        self.manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)

    def tearDown(self):
        """Clean up test fixtures."""
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_initialize_project_creates_teams(self):
        """Test that initialize_project creates all 12 teams."""
        self.manager.initialize_project()
        self.assertEqual(len(self.manager.teams), 12)

    def test_initialize_project_team_ids(self):
        """Test that initialize_project creates teams with correct IDs."""
        self.manager.initialize_project()
        for i in range(1, 13):
            self.assertIn(i, self.manager.teams)

    def test_initialize_project_saves_file(self):
        """Test that initialize_project saves to file."""
        self.manager.initialize_project()
        self.assertTrue(self.manager.config_path.exists())

    def test_initialize_project_file_content(self):
        """Test that saved file has correct content."""
        self.manager.initialize_project()
        with open(self.manager.config_path) as f:
            data = json.load(f)
        self.assertEqual(data["project_name"], "test-project")
        self.assertEqual(len(data["teams"]), 12)
        self.assertIn("updated_at", data)

    def test_initialize_project_requires_admin(self):
        """Test that initialize_project requires admin role."""
        team_lead_ctx = create_team_lead_context()
        manager = TeamManager("test-project", self.config_path, team_lead_ctx, self.logger)
        with self.assertRaises(PermissionDenied):
            manager.initialize_project()


class TestTeamManagerLoad(unittest.TestCase):
    """Tests for load method."""

    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.config_path = Path(self.temp_dir) / "test-project.json"
        self.user_ctx = create_admin_context()
        self.logger = create_test_logger()

    def tearDown(self):
        """Clean up test fixtures."""
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_load_nonexistent_file(self):
        """Test load returns False when file doesn't exist."""
        manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        result = manager.load()
        self.assertFalse(result)

    def test_load_existing_file(self):
        """Test load returns True and loads data when file exists."""
        # Initialize and save
        manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        manager.initialize_project()

        # Create new manager and load
        new_manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        result = new_manager.load()

        self.assertTrue(result)
        self.assertEqual(len(new_manager.teams), 12)

    def test_load_preserves_team_data(self):
        """Test that load preserves team data correctly."""
        # Initialize, assign role, and save
        manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        manager.initialize_project()
        manager.assign_role(1, "Business Relationship Manager", "John Doe")

        # Create new manager and load
        new_manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        new_manager.load()

        # Check that assignment was preserved
        team = new_manager.teams[1]
        role = next(r for r in team.roles if r.name == "Business Relationship Manager")
        self.assertEqual(role.assigned_to, "John Doe")


class TestTeamManagerAssignRole(unittest.TestCase):
    """Tests for assign_role method."""

    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.config_path = Path(self.temp_dir) / "test-project.json"
        self.user_ctx = create_admin_context()
        self.logger = create_test_logger()
        self.manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        self.manager.initialize_project()

    def tearDown(self):
        """Clean up test fixtures."""
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_assign_role_valid(self):
        """Test assigning a role to a valid team."""
        result = self.manager.assign_role(
            1, "Business Relationship Manager", "John Doe"
        )
        self.assertTrue(result)

        # Verify assignment
        team = self.manager.teams[1]
        role = next(r for r in team.roles if r.name == "Business Relationship Manager")
        self.assertEqual(role.assigned_to, "John Doe")

    def test_assign_role_invalid_team(self):
        """Test assigning to invalid team returns False."""
        result = self.manager.assign_role(99, "Some Role", "John Doe")
        self.assertFalse(result)

    def test_assign_role_invalid_role(self):
        """Test assigning invalid role returns False."""
        result = self.manager.assign_role(1, "Nonexistent Role", "John Doe")
        self.assertFalse(result)

    def test_assign_role_saves_to_file(self):
        """Test that assign_role saves to file."""
        self.manager.assign_role(1, "Business Relationship Manager", "John Doe")

        # Load file and verify
        with open(self.manager.config_path) as f:
            data = json.load(f)

        team_data = next(t for t in data["teams"] if t["id"] == 1)
        role_data = next(r for r in team_data["roles"] if r["name"] == "Business Relationship Manager")
        self.assertEqual(role_data["assigned_to"], "John Doe")

    def test_assign_multiple_roles(self):
        """Test assigning multiple roles to a team."""
        team = self.manager.teams[1]
        for i, role in enumerate(team.roles):
            result = self.manager.assign_role(1, role.name, f"Person {i}")
            self.assertTrue(result)

        # Verify all assignments
        for i, role in enumerate(team.roles):
            self.assertEqual(role.assigned_to, f"Person {i}")

    def test_assign_role_requires_permission(self):
        """Test that assign_role requires team-lead or admin permission."""
        # Create viewer context
        viewer_ctx = UserContext(user_id="test-viewer", role="viewer")
        manager = TeamManager("test-project", self.config_path, viewer_ctx, self.logger)
        manager.initialize_project()

        with self.assertRaises(PermissionDenied):
            manager.assign_role(1, "Business Relationship Manager", "John Doe")


class TestTeamManagerStartTeam(unittest.TestCase):
    """Tests for start_team method."""

    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.config_path = Path(self.temp_dir) / "test-project.json"
        self.user_ctx = create_admin_context()
        self.logger = create_test_logger()
        self.manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        self.manager.initialize_project()

    def tearDown(self):
        """Clean up test fixtures."""
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_start_team_valid(self):
        """Test starting a valid team."""
        result = self.manager.start_team(1)
        self.assertTrue(result)
        self.assertEqual(self.manager.teams[1].status, "active")
        self.assertIsNotNone(self.manager.teams[1].started_at)

    def test_start_team_invalid(self):
        """Test starting invalid team returns False."""
        result = self.manager.start_team(99)
        self.assertFalse(result)


class TestTeamManagerCompleteTeam(unittest.TestCase):
    """Tests for complete_team method."""

    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.config_path = Path(self.temp_dir) / "test-project.json"
        self.user_ctx = create_admin_context()
        self.logger = create_test_logger()
        self.manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        self.manager.initialize_project()

    def tearDown(self):
        """Clean up test fixtures."""
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_complete_team_valid(self):
        """Test completing a valid team."""
        result = self.manager.complete_team(1)
        self.assertTrue(result)
        self.assertEqual(self.manager.teams[1].status, "completed")
        self.assertIsNotNone(self.manager.teams[1].completed_at)

    def test_complete_team_invalid(self):
        """Test completing invalid team returns False."""
        result = self.manager.complete_team(99)
        self.assertFalse(result)


class TestTeamManagerGetPhaseStatus(unittest.TestCase):
    """Tests for get_phase_status method."""

    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.config_path = Path(self.temp_dir) / "test-project.json"
        self.user_ctx = create_admin_context()
        self.logger = create_test_logger()
        self.manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        self.manager.initialize_project()

    def tearDown(self):
        """Clean up test fixtures."""
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_get_phase_status_structure(self):
        """Test that get_phase_status returns correct structure."""
        status = self.manager.get_phase_status("Phase 1: Strategy, Governance & Planning")

        required_keys = ["phase", "total_teams", "completed", "active", "not_started", "progress_pct"]
        for key in required_keys:
            self.assertIn(key, status)

    def test_get_phase_status_phase_1(self):
        """Test phase status for Phase 1."""
        status = self.manager.get_phase_status("Phase 1: Strategy, Governance & Planning")
        self.assertEqual(status["phase"], "Phase 1: Strategy, Governance & Planning")
        self.assertEqual(status["total_teams"], 3)  # Teams 1, 2, 3

    def test_get_phase_status_progress_calculation(self):
        """Test progress percentage calculation."""
        # Initially all not started
        status = self.manager.get_phase_status("Phase 1: Strategy, Governance & Planning")
        self.assertEqual(status["progress_pct"], 0.0)

        # Complete one team
        self.manager.complete_team(1)
        status = self.manager.get_phase_status("Phase 1: Strategy, Governance & Planning")
        self.assertEqual(status["progress_pct"], 100.0 / 3.0)

    def test_get_phase_status_invalid_phase(self):
        """Test phase status for non-existent phase."""
        status = self.manager.get_phase_status("Nonexistent Phase")
        self.assertEqual(status["total_teams"], 0)
        self.assertEqual(status["progress_pct"], 0)


class TestTeamManagerValidateTeamSize(unittest.TestCase):
    """Tests for validate_team_size method."""

    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.config_path = Path(self.temp_dir) / "test-project.json"
        self.user_ctx = create_admin_context()
        self.logger = create_test_logger()
        self.manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        self.manager.initialize_project()

    def tearDown(self):
        """Clean up test fixtures."""
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_validate_all_teams_empty(self):
        """Test validating all teams with no assignments."""
        results = self.manager.validate_team_size()
        self.assertFalse(results["valid"])
        self.assertEqual(results["teams_checked"], 12)
        self.assertEqual(len(results["violations"]), 12)  # All undersized

    def test_validate_specific_team(self):
        """Test validating specific team."""
        results = self.manager.validate_team_size(1)
        self.assertFalse(results["valid"])
        self.assertEqual(results["teams_checked"], 1)

    def test_validate_team_with_assignments(self):
        """Test validating team with assignments."""
        # Assign 4 people to team 1 (minimum valid size)
        team = self.manager.teams[1]
        for i, role in enumerate(team.roles[:4]):
            role.assigned_to = f"Person {i}"

        results = self.manager.validate_team_size(1)
        self.assertTrue(results["valid"])
        self.assertEqual(len(results["violations"]), 0)

    def test_validate_team_oversized(self):
        """Test validating oversized team."""
        # Team 7 has 5 roles - assigning all is still valid (5 <= 6)
        team = self.manager.teams[7]
        for i, role in enumerate(team.roles):
            role.assigned_to = f"Person {i}"

        results = self.manager.validate_team_size(7)
        self.assertTrue(results["valid"])

    def test_violation_structure(self):
        """Test that violations have correct structure."""
        results = self.manager.validate_team_size(1)

        if results["violations"]:
            violation = results["violations"][0]
            required_keys = ["team_id", "team_name", "issue", "message"]
            for key in required_keys:
                self.assertIn(key, violation)


class TestTeamManagerGetAgentTeam(unittest.TestCase):
    """Tests for get_agent_team method."""

    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.config_path = Path(self.temp_dir) / "test-project.json"
        self.user_ctx = create_admin_context()
        self.logger = create_test_logger()
        self.manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        self.manager.initialize_project()

    def tearDown(self):
        """Clean up test fixtures."""
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_get_agent_team_valid(self):
        """Test getting team for valid agent types."""
        test_cases = [
            ("planner", 2),
            ("coder", 7),
            ("reviewer", 10),
            ("security", 9),
            ("tester", 10),
            ("ops", 11),
        ]
        for agent_type, expected_team in test_cases:
            with self.subTest(agent_type=agent_type):
                team = self.manager.get_agent_team(agent_type)
                self.assertIsNotNone(team)
                self.assertEqual(team.id, expected_team)

    def test_get_agent_team_case_insensitive(self):
        """Test that agent type matching is case insensitive."""
        team_lower = self.manager.get_agent_team("PLANNER")
        team_upper = self.manager.get_agent_team("planner")
        self.assertEqual(team_lower.id, team_upper.id)

    def test_get_agent_team_invalid(self):
        """Test getting team for invalid agent type."""
        team = self.manager.get_agent_team("nonexistent")
        self.assertIsNone(team)


class TestTeamManagerListTeams(unittest.TestCase):
    """Tests for list_teams method."""

    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.config_path = Path(self.temp_dir) / "test-project.json"
        self.user_ctx = create_admin_context()
        self.logger = create_test_logger()
        self.manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        self.manager.initialize_project()

    def tearDown(self):
        """Clean up test fixtures."""
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_list_teams_all(self):
        """Test listing all teams."""
        import io
        from contextlib import redirect_stdout

        f = io.StringIO()
        with redirect_stdout(f):
            self.manager.list_teams()
        output = f.getvalue()

        # Should contain all 12 teams
        self.assertIn("Team 1:", output)
        self.assertIn("Team 12:", output)
        self.assertIn("Business & Product Strategy", output)

    def test_list_teams_filtered(self):
        """Test listing teams filtered by phase."""
        import io
        from contextlib import redirect_stdout

        f = io.StringIO()
        with redirect_stdout(f):
            self.manager.list_teams("Phase 1: Strategy, Governance & Planning")
        output = f.getvalue()

        # Should only contain Phase 1 teams
        self.assertIn("Phase 1:", output)
        self.assertIn("Team 1:", output)
        self.assertIn("Team 2:", output)
        self.assertIn("Team 3:", output)


class TestTeamManagerDelete(unittest.TestCase):
    """Tests for delete operations."""

    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.config_path = Path(self.temp_dir) / "test-project.json"
        self.user_ctx = create_admin_context()
        self.logger = create_test_logger()
        self.manager = TeamManager("test-project", self.config_path, self.user_ctx, self.logger)
        self.manager.initialize_project()

    def tearDown(self):
        """Clean up test fixtures."""
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_unassign_role_valid(self):
        """Test unassigning a role."""
        # First assign
        self.manager.assign_role(1, "Business Relationship Manager", "John Doe")

        # Then unassign
        result = self.manager.unassign_role(1, "Business Relationship Manager")
        self.assertTrue(result)

        # Verify unassignment
        team = self.manager.teams[1]
        role = next(r for r in team.roles if r.name == "Business Relationship Manager")
        self.assertIsNone(role.assigned_to)

    def test_unassign_role_not_assigned(self):
        """Test unassigning a role that was never assigned."""
        result = self.manager.unassign_role(1, "Business Relationship Manager")
        # Already unassigned returns False (no change needed)
        self.assertFalse(result)

    def test_unassign_role_invalid_team(self):
        """Test unassigning from invalid team."""
        result = self.manager.unassign_role(99, "Some Role")
        self.assertFalse(result)

    def test_delete_team_valid(self):
        """Test deleting a team."""
        result = self.manager.delete_team(1)
        self.assertTrue(result)
        self.assertNotIn(1, self.manager.teams)

    def test_delete_team_invalid(self):
        """Test deleting invalid team."""
        result = self.manager.delete_team(99)
        self.assertFalse(result)

    def test_delete_project_valid(self):
        """Test deleting entire project."""
        result = self.manager.delete_project()
        self.assertTrue(result)
        self.assertFalse(self.manager.config_path.exists())


class TestTeamStructure(unittest.TestCase):
    """Tests for Team and Role data structures."""

    def test_role_creation(self):
        """Test Role dataclass creation."""
        role = Role(
            name="Test Role",
            responsibility="Test responsibility",
            deliverables=["Item 1", "Item 2"]
        )
        self.assertEqual(role.name, "Test Role")
        self.assertEqual(role.assigned_to, None)

    def test_team_creation(self):
        """Test Team dataclass creation."""
        team = Team(
            id=1,
            name="Test Team",
            phase="Phase 1",
            description="Test description",
            roles=[],
            exit_criteria=["Criterion 1"]
        )
        self.assertEqual(team.id, 1)
        self.assertEqual(team.status, "not_started")


class TestStandardTeams(unittest.TestCase):
    """Tests for STANDARD_TEAMS constant."""

    def test_standard_teams_count(self):
        """Test that there are exactly 12 standard teams."""
        self.assertEqual(len(TeamManager.STANDARD_TEAMS), 12)

    def test_standard_teams_ids(self):
        """Test that standard teams have IDs 1-12."""
        for i in range(1, 13):
            self.assertIn(i, TeamManager.STANDARD_TEAMS)

    def test_team_1_structure(self):
        """Test Team 1 structure."""
        team = TeamManager.STANDARD_TEAMS[1]
        self.assertEqual(team.id, 1)
        self.assertEqual(team.name, "Business & Product Strategy")
        self.assertEqual(len(team.roles), 4)

    def test_team_7_structure(self):
        """Test Team 7 (Core Feature Squad) structure."""
        team = TeamManager.STANDARD_TEAMS[7]
        self.assertEqual(team.id, 7)
        self.assertEqual(team.name, "Core Feature Squad")
        self.assertEqual(len(team.roles), 5)

    def test_all_teams_have_roles(self):
        """Test that all teams have at least 4 roles."""
        for team_id, team in TeamManager.STANDARD_TEAMS.items():
            with self.subTest(team_id=team_id):
                self.assertGreaterEqual(len(team.roles), 4)

    def test_all_teams_have_exit_criteria(self):
        """Test that all teams have exit criteria."""
        for team_id, team in TeamManager.STANDARD_TEAMS.items():
            with self.subTest(team_id=team_id):
                self.assertGreater(len(team.exit_criteria), 0)


class TestStructuredLogger(unittest.TestCase):
    """Tests for StructuredLogger."""

    def test_logger_creation(self):
        """Test logger initialization."""
        logger = StructuredLogger("test-component", request_id="req-123")
        self.assertEqual(logger.component, "test-component")
        self.assertEqual(logger.request_id, "req-123")

    def test_logger_log_output(self):
        """Test log output is valid JSON."""
        import io
        import sys

        logger = StructuredLogger("test", request_id="test-123")

        # Capture stderr
        old_stderr = sys.stderr
        sys.stderr = io.StringIO()

        logger.info("test_event", {"key": "value"})

        output = sys.stderr.getvalue()
        sys.stderr = old_stderr

        # Parse as JSON
        log_entry = json.loads(output.strip())
        self.assertEqual(log_entry["component"], "test")
        self.assertEqual(log_entry["event"], "test_event")
        self.assertEqual(log_entry["level"], "INFO")
        self.assertEqual(log_entry["details"]["key"], "value")
        self.assertEqual(log_entry["request_id"], "test-123")


if __name__ == "__main__":
    unittest.main(verbosity=2)
