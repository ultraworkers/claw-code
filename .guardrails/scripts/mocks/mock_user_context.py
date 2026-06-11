#!/usr/bin/env python3
"""
Mock user context for testing RBAC without actual authentication.
"""

from typing import Optional


class MockUserContext:
    """
    Mock user context with configurable RBAC for testing.

    Simulates different user roles without requiring actual authentication.
    """

    ROLE_LEVELS = {
        "viewer": 1,
        "team-lead": 2,
        "admin": 3
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

    def __repr__(self) -> str:
        return f"MockUserContext(user_id='{self.user_id}', role='{self.role}', team_id={self.team_id})"


def create_admin_context(user_id: str = "test-admin") -> MockUserContext:
    """Factory for creating an admin user context."""
    return MockUserContext(user_id=user_id, role="admin")


def create_team_lead_context(team_id: int = None, user_id: str = "test-lead") -> MockUserContext:
    """Factory for creating a team-lead user context."""
    return MockUserContext(user_id=user_id, role="team-lead", team_id=team_id)


def create_viewer_context(user_id: str = "test-viewer") -> MockUserContext:
    """Factory for creating a viewer user context."""
    return MockUserContext(user_id=user_id, role="viewer")
