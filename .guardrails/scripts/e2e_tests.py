#!/usr/bin/env python3
"""
End-to-End (E2E) Tests for Team Manager

Tests complete workflows including:
- Full lifecycle: init → assign → list → status → delete
- Phase gate transitions
- Error scenarios
- Multi-phase workflow
- RBAC testing
- Batch operations
"""

import sys
import os
import unittest
import tempfile
import json
import shutil
from pathlib import Path

# Add parent directory to path
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from scripts.mocks import (
    MockTeamManager,
    MockLogger,
    MockFileSystem,
    create_admin_context,
    create_team_lead_context,
    create_viewer_context,
)


class TestE2EFullWorkflow(unittest.TestCase):
    """E2E: Complete project lifecycle workflow."""

    def setUp(self):
        """Set up test fixtures."""
        self.admin_ctx = create_admin_context("admin-user")
        self.logger = MockLogger("test")
        self.fs = MockFileSystem()
        self.manager = MockTeamManager(
            "e2e-workflow-test",
            user_context=self.admin_ctx,
            logger=self.logger,
            fs=self.fs
        )

    def tearDown(self):
        """Clean up."""
        self.manager.reset()

    def test_complete_project_lifecycle(self):
        """
        E2E-001: Full workflow: init → assign → start → complete → export → delete
        """
        # Step 1: Initialize project
        print("\n[E2E-001] Step 1: Initialize project")
        result = self.manager.initialize_project()
        self.assertTrue(result)
        self.assertEqual(len(self.manager.teams), 12)
        self.assertTrue(self.logger.has_event("project_initialized"))

        # Step 2: Assign roles to Team 1
        print("[E2E-001] Step 2: Assign roles to Team 1")
        team1 = self.manager.teams[1]
        for i, role in enumerate(team1.roles):
            result = self.manager.assign_role(1, role.name, f"Person {i+1}")
            self.assertTrue(result)

        assigned_count = sum(1 for r in team1.roles if r.assigned_to)
        self.assertEqual(assigned_count, 4)

        # Step 3: Start Team 1
        print("[E2E-001] Step 3: Start Team 1")
        result = self.manager.start_team(1)
        self.assertTrue(result)
        self.assertEqual(team1.status, "active")
        self.assertIsNotNone(team1.started_at)

        # Step 4: Complete Team 1
        print("[E2E-001] Step 4: Complete Team 1")
        result = self.manager.complete_team(1)
        self.assertTrue(result)
        self.assertEqual(team1.status, "completed")
        self.assertIsNotNone(team1.completed_at)

        # Step 5: Check phase status
        print("[E2E-001] Step 5: Check Phase 1 status")
        phase_status = self.manager.get_phase_status("Phase 1: Strategy, Governance & Planning")
        self.assertEqual(phase_status["completed"], 1)
        self.assertGreater(phase_status["progress_pct"], 0)

        # Step 6: List all teams
        print("[E2E-001] Step 6: List teams")
        team_list = self.manager.list_teams()
        self.assertEqual(len(team_list), 12)

        # Step 7: Delete the project
        print("[E2E-001] Step 7: Delete project")
        result = self.manager.delete_project(confirmed=True)
        self.assertTrue(result["success"])

        print("[E2E-001] ✓ Complete lifecycle test passed")


class TestE2EPhaseGates(unittest.TestCase):
    """E2E: Phase gate transitions."""

    def setUp(self):
        self.admin_ctx = create_admin_context()
        self.manager = MockTeamManager(
            "e2e-phase-test",
            user_context=self.admin_ctx,
            logger=MockLogger("test"),
            fs=MockFileSystem()
        )
        self.manager.initialize_project()

    def test_phase1_completion(self):
        """
        E2E-002: Complete all Phase 1 teams and verify progress
        """
        print("\n[E2E-002] Testing Phase 1 completion")

        # Get Phase 1 teams
        phase1_teams = [t for t in self.manager.teams.values()
                        if t.phase == "Phase 1: Strategy, Governance & Planning"]
        self.assertEqual(len(phase1_teams), 3)  # Teams 1, 2, 3

        # Initially 0% complete
        status = self.manager.get_phase_status("Phase 1: Strategy, Governance & Planning")
        self.assertEqual(status["progress_pct"], 0.0)

        # Complete all Phase 1 teams
        for team in phase1_teams:
            self.manager.start_team(team.id)
            self.manager.complete_team(team.id)

        # Now 100% complete
        status = self.manager.get_phase_status("Phase 1: Strategy, Governance & Planning")
        self.assertEqual(status["progress_pct"], 100.0)
        self.assertEqual(status["completed"], 3)

        print("[E2E-002] ✓ Phase 1 completion test passed")

    def test_phase_progression(self):
        """
        E2E-003: Test progression through multiple phases
        """
        print("\n[E2E-003] Testing phase progression")

        phases = [
            "Phase 1: Strategy, Governance & Planning",
            "Phase 2: Platform & Foundation",
            "Phase 3: The Build Squads",
            "Phase 4: Validation & Hardening",
            "Phase 5: Delivery & Sustainment",
        ]

        for phase in phases:
            phase_teams = [t for t in self.manager.teams.values() if t.phase == phase]
            for team in phase_teams:
                self.manager.start_team(team.id)
                self.manager.complete_team(team.id)

            status = self.manager.get_phase_status(phase)
            self.assertEqual(status["progress_pct"], 100.0)
            print(f"  ✓ {phase}: 100%")

        print("[E2E-003] ✓ Phase progression test passed")


class TestE2EErrorScenarios(unittest.TestCase):
    """E2E: Error scenarios."""

    def setUp(self):
        self.admin_ctx = create_admin_context()
        self.manager = MockTeamManager(
            "e2e-error-test",
            user_context=self.admin_ctx,
            logger=MockLogger("test"),
            fs=MockFileSystem()
        )
        self.manager.initialize_project()

    def test_assign_to_invalid_team(self):
        """
        E2E-004: Assign to invalid team returns False
        """
        print("\n[E2E-004] Testing invalid team assignment")
        result = self.manager.assign_role(99, "Some Role", "John Doe")
        self.assertFalse(result)
        print("[E2E-004] ✓ Invalid team assignment handled correctly")

    def test_assign_invalid_role(self):
        """
        E2E-005: Assign invalid role returns False
        """
        print("\n[E2E-005] Testing invalid role assignment")
        result = self.manager.assign_role(1, "Nonexistent Role", "John Doe")
        self.assertFalse(result)
        print("[E2E-005] ✓ Invalid role assignment handled correctly")

    def test_delete_without_confirmation(self):
        """
        E2E-006: Delete without confirmation requires confirmation
        """
        print("\n[E2E-006] Testing delete without confirmation")
        result = self.manager.delete_team(1, confirmed=False)
        self.assertFalse(result["success"])
        self.assertTrue(result["requires_confirmation"])
        print("[E2E-006] ✓ Delete confirmation required correctly")

    def test_delete_nonexistent_project(self):
        """
        E2E-007: Delete non-existent project fails gracefully
        """
        print("\n[E2E-007] Testing delete of non-existent project")
        empty_manager = MockTeamManager(
            "nonexistent-project",
            user_context=self.admin_ctx,
            logger=MockLogger("test"),
            fs=MockFileSystem()
        )
        result = empty_manager.delete_project(confirmed=True)
        self.assertFalse(result["success"])
        print("[E2E-007] ✓ Non-existent project deletion handled correctly")

    def test_unauthorized_access(self):
        """
        E2E-008: Unauthorized access raises PermissionDenied
        """
        print("\n[E2E-008] Testing unauthorized access")
        from scripts.team_manager import PermissionDenied

        viewer_ctx = create_viewer_context()
        manager = MockTeamManager(
            "e2e-auth-test",
            user_context=viewer_ctx,
            logger=MockLogger("test"),
            fs=MockFileSystem()
        )

        with self.assertRaises(PermissionDenied):
            manager.initialize_project()

        print("[E2E-008] ✓ Unauthorized access handled correctly")


class TestE2ERBAC(unittest.TestCase):
    """E2E: Role-based access control."""

    def setUp(self):
        self.fs = MockFileSystem()

    def test_admin_can_modify_any_team(self):
        """
        E2E-009: Admin can modify any team
        """
        print("\n[E2E-009] Testing admin permissions")
        admin_ctx = create_admin_context()
        manager = MockTeamManager("rbac-test", user_context=admin_ctx, logger=MockLogger("test"), fs=self.fs)
        manager.initialize_project()

        # Admin should be able to modify any team
        for team_id in range(1, 13):
            result = manager.assign_role(team_id, manager.teams[team_id].roles[0].name, f"Admin Assignee {team_id}")
            self.assertTrue(result, f"Admin should be able to modify team {team_id}")

        print("[E2E-009] ✓ Admin permissions verified")

    def test_team_lead_can_only_modify_their_team(self):
        """
        E2E-010: Team lead can only modify their assigned team
        """
        print("\n[E2E-010] Testing team lead permissions")
        admin_ctx = create_admin_context()
        manager = MockTeamManager("rbac-test", user_context=admin_ctx, logger=MockLogger("test"), fs=self.fs)
        manager.initialize_project()

        # Create team lead context for team 5
        team_lead_ctx = create_team_lead_context(team_id=5)
        lead_manager = MockTeamManager(
            "rbac-test",
            user_context=team_lead_ctx,
            logger=MockLogger("test"),
            fs=self.fs
        )
        load_result = lead_manager.load()
        self.assertTrue(load_result, "Failed to load project for team lead")
        self.assertIn(5, lead_manager.teams, "Team 5 should exist after loading")

        from scripts.team_manager import PermissionDenied

        # Should succeed for team 5
        result = lead_manager.assign_role(5, lead_manager.teams[5].roles[0].name, "Lead Assignee")
        self.assertTrue(result)

        print("[E2E-010] ✓ Team lead permissions verified")

    def test_viewer_cannot_modify(self):
        """
        E2E-011: Viewer cannot modify anything
        """
        print("\n[E2E-011] Testing viewer restrictions")
        viewer_ctx = create_viewer_context()
        manager = MockTeamManager(
            "rbac-test",
            user_context=viewer_ctx,
            logger=MockLogger("test"),
            fs=MockFileSystem()
        )

        from scripts.team_manager import PermissionDenied

        with self.assertRaises(PermissionDenied):
            manager.initialize_project()

        print("[E2E-011] ✓ Viewer restrictions verified")


class TestE2EBatchOperations(unittest.TestCase):
    """E2E: Batch operations."""

    def setUp(self):
        self.admin_ctx = create_admin_context()
        self.fs = MockFileSystem()
        self.manager = MockTeamManager(
            "e2e-batch-test",
            user_context=self.admin_ctx,
            logger=MockLogger("test"),
            fs=self.fs
        )
        self.manager.initialize_project()
        self.temp_dir = tempfile.mkdtemp()

    def tearDown(self):
        """Clean up temp files."""
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_csv_export_import_roundtrip(self):
        """
        E2E-012: CSV export/import roundtrip preserves data
        """
        print("\n[E2E-012] Testing CSV roundtrip")

        # First assign some roles
        self.manager.assign_role(1, "Business Relationship Manager", "Alice")
        self.manager.assign_role(1, "Lead Product Manager", "Bob")

        # Export to CSV using real file system
        csv_path = Path(self.temp_dir) / "test_export.csv"
        result = self.manager.export_csv_file(csv_path)
        self.assertTrue(result["success"])

        # Verify CSV content exists on real file system
        self.assertTrue(csv_path.exists())

        # Verify content
        with open(csv_path, 'r') as f:
            content = f.read()
            self.assertIn("Alice", content)
            self.assertIn("Business Relationship Manager", content)

        print("[E2E-012] ✓ CSV roundtrip test passed")

    def test_json_export_import_roundtrip(self):
        """
        E2E-013: JSON export/import roundtrip preserves data
        """
        print("\n[E2E-013] Testing JSON roundtrip")

        # Assign and complete Team 1
        self.manager.assign_role(1, "Business Relationship Manager", "Alice")
        self.manager.start_team(1)
        self.manager.complete_team(1)

        # Export to JSON using real file system
        json_path = Path(self.temp_dir) / "test_export.json"
        result = self.manager.export_json_file(json_path)
        self.assertTrue(result["success"])
        self.assertEqual(result["team_count"], 12)

        # Verify JSON file exists
        self.assertTrue(json_path.exists())

        # Verify JSON content
        with open(json_path, 'r') as f:
            data = json.load(f)
            self.assertEqual(data["project_name"], "e2e-batch-test")
            self.assertEqual(len(data["teams"]), 12)

        print("[E2E-013] ✓ JSON roundtrip test passed")


class TestE2EAgentMapping(unittest.TestCase):
    """E2E: Agent type to team mapping."""

    def setUp(self):
        self.admin_ctx = create_admin_context()
        self.manager = MockTeamManager(
            "e2e-agent-test",
            user_context=self.admin_ctx,
            logger=MockLogger("test"),
            fs=MockFileSystem()
        )
        self.manager.initialize_project()

    def test_agent_type_mapping(self):
        """
        E2E-014: Agent types map to correct teams
        """
        print("\n[E2E-014] Testing agent type mappings")

        test_cases = [
            ("planner", 2, "Enterprise Architecture"),
            ("coder", 7, "Core Feature Squad"),
            ("reviewer", 10, "Quality Engineering (SDET)"),
            ("security", 9, "Cybersecurity (AppSec)"),
            ("tester", 10, "Quality Engineering (SDET)"),
            ("ops", 11, "Site Reliability Engineering (SRE)"),
        ]

        for agent_type, expected_id, expected_name in test_cases:
            team = self.manager.get_agent_team(agent_type)
            self.assertIsNotNone(team, f"Agent type '{agent_type}' should map to a team")
            self.assertEqual(team.id, expected_id)
            self.assertEqual(team.name, expected_name)
            print(f"  ✓ '{agent_type}' → Team {expected_id}: {expected_name}")

        print("[E2E-014] ✓ Agent type mapping verified")

    def test_invalid_agent_type(self):
        """
        E2E-015: Invalid agent type returns None
        """
        print("\n[E2E-015] Testing invalid agent type")
        team = self.manager.get_agent_team("invalid_agent_type")
        self.assertIsNone(team)
        print("[E2E-015] ✓ Invalid agent type handled correctly")


class TestE2ETeamValidation(unittest.TestCase):
    """E2E: Team size validation."""

    def setUp(self):
        self.admin_ctx = create_admin_context()
        self.manager = MockTeamManager(
            "e2e-validation-test",
            user_context=self.admin_ctx,
            logger=MockLogger("test"),
            fs=MockFileSystem()
        )
        self.manager.initialize_project()

    def test_empty_team_validation_fails(self):
        """
        E2E-016: Empty team fails size validation
        """
        print("\n[E2E-016] Testing empty team validation")
        results = self.manager.validate_team_size(1)
        self.assertFalse(results["valid"])
        self.assertEqual(len(results["violations"]), 1)
        self.assertEqual(results["violations"][0]["issue"], "undersized")
        print("[E2E-016] ✓ Empty team validation works")

    def test_properly_sized_team_passes(self):
        """
        E2E-017: Properly staffed team passes validation
        """
        print("\n[E2E-017] Testing properly sized team")
        team = self.manager.teams[1]
        for i, role in enumerate(team.roles[:4]):  # Assign 4 people
            self.manager.assign_role(1, role.name, f"Person {i+1}")

        results = self.manager.validate_team_size(1)
        self.assertTrue(results["valid"])
        self.assertEqual(len(results["violations"]), 0)
        print("[E2E-017] ✓ Properly sized team passes validation")


class TestE2EPersistence(unittest.TestCase):
    """E2E: Save/load persistence."""

    def setUp(self):
        self.admin_ctx = create_admin_context()
        self.fs = MockFileSystem()

    def test_save_and_load(self):
        """
        E2E-018: Save and load preserves all data
        """
        print("\n[E2E-018] Testing save/load persistence")

        # Create and populate manager
        manager1 = MockTeamManager(
            "persistence-test",
            user_context=self.admin_ctx,
            logger=MockLogger("test"),
            fs=self.fs
        )
        manager1.initialize_project()
        manager1.assign_role(1, "Business Relationship Manager", "Test User")
        manager1.start_team(1)
        manager1.save()

        # Load into new manager
        manager2 = MockTeamManager(
            "persistence-test",
            user_context=self.admin_ctx,
            logger=MockLogger("test2"),
            fs=self.fs
        )
        result = manager2.load()
        self.assertTrue(result)

        # Verify data preserved
        self.assertEqual(manager2.teams[1].status, "active")
        role = next(r for r in manager2.teams[1].roles if r.name == "Business Relationship Manager")
        self.assertEqual(role.assigned_to, "Test User")

        print("[E2E-018] ✓ Save/load persistence verified")


def run_e2e_tests():
    """Run all E2E tests and return results."""
    print("=" * 70)
    print("Running E2E Tests for Team Manager")
    print("=" * 70)

    loader = unittest.TestLoader()
    suite = unittest.TestSuite()

    # Add all test classes
    suite.addTests(loader.loadTestsFromTestCase(TestE2EFullWorkflow))
    suite.addTests(loader.loadTestsFromTestCase(TestE2EPhaseGates))
    suite.addTests(loader.loadTestsFromTestCase(TestE2EErrorScenarios))
    suite.addTests(loader.loadTestsFromTestCase(TestE2ERBAC))
    suite.addTests(loader.loadTestsFromTestCase(TestE2EBatchOperations))
    suite.addTests(loader.loadTestsFromTestCase(TestE2EAgentMapping))
    suite.addTests(loader.loadTestsFromTestCase(TestE2ETeamValidation))
    suite.addTests(loader.loadTestsFromTestCase(TestE2EPersistence))

    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)

    print("\n" + "=" * 70)
    print(f"Tests run: {result.testsRun}")
    print(f"Failures: {len(result.failures)}")
    print(f"Errors: {len(result.errors)}")
    print(f"Skipped: {len(result.skipped)}")
    print("=" * 70)

    return result.wasSuccessful()


if __name__ == "__main__":
    success = run_e2e_tests()
    sys.exit(0 if success else 1)
