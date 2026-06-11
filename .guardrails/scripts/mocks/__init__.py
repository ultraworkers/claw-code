#!/usr/bin/env python3
"""
Mock infrastructure for Team Manager testing.

This package provides mock implementations for testing team_manager.py
without requiring actual file system operations or Python execution.
"""

from .mock_file_system import MockFileSystem, MockFileLock
from .mock_logger import MockLogger, LogEntry
from .mock_user_context import MockUserContext, create_admin_context, create_team_lead_context, create_viewer_context
from .mock_team_manager import MockTeamManager

__all__ = [
    'MockFileSystem',
    'MockFileLock',
    'MockLogger',
    'LogEntry',
    'MockUserContext',
    'create_admin_context',
    'create_team_lead_context',
    'create_viewer_context',
    'MockTeamManager',
]
