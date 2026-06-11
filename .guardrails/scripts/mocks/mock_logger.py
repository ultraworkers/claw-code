#!/usr/bin/env python3
"""
Mock logger for testing without actual logging output.
"""

from dataclasses import dataclass, field
from datetime import datetime
from typing import Dict, List, Optional, Any


@dataclass
class LogEntry:
    """Represents a single log entry."""
    level: str
    component: str
    event: str
    details: Dict[str, Any]
    timestamp: str = field(default_factory=lambda: datetime.now().isoformat())
    request_id: Optional[str] = None


class MockLogger:
    """
    Mock structured logger for testing.

    Captures log entries in memory for assertions instead of printing to stderr.
    """

    def __init__(self, component: str, request_id: Optional[str] = None):
        self.component = component
        self.request_id = request_id
        self._entries: List[LogEntry] = []
        self._capture_stderr = False

    def log(self, event_type: str, details: Dict, level: str = "info") -> None:
        """Log a structured event."""
        entry = LogEntry(
            level=level.upper(),
            component=self.component,
            event=event_type,
            details=details,
            request_id=self.request_id
        )
        self._entries.append(entry)

    def info(self, event_type: str, details: Dict) -> None:
        """Log INFO level event."""
        self.log(event_type, details, "info")

    def warn(self, event_type: str, details: Dict) -> None:
        """Log WARN level event."""
        self.log(event_type, details, "warn")

    def error(self, event_type: str, details: Dict, exc_info: bool = False) -> None:
        """Log ERROR level event."""
        if exc_info:
            import traceback
            details = {**details, "stack_trace": traceback.format_exc()}
        self.log(event_type, details, "error")

    def debug(self, event_type: str, details: Dict) -> None:
        """Log DEBUG level event."""
        self.log(event_type, details, "debug")

    def get_entries(self) -> List[LogEntry]:
        """Get all logged entries."""
        return self._entries.copy()

    def get_entries_by_level(self, level: str) -> List[LogEntry]:
        """Get entries filtered by level."""
        level_upper = level.upper()
        return [e for e in self._entries if e.level == level_upper]

    def get_entries_by_event(self, event_type: str) -> List[LogEntry]:
        """Get entries filtered by event type."""
        return [e for e in self._entries if e.event == event_type]

    def get_entries_by_component(self, component: str) -> List[LogEntry]:
        """Get entries filtered by component."""
        return [e for e in self._entries if e.component == component]

    def has_event(self, event_type: str) -> bool:
        """Check if an event was logged."""
        return any(e.event == event_type for e in self._entries)

    def has_error(self) -> bool:
        """Check if any error was logged."""
        return any(e.level == "ERROR" for e in self._entries)

    def get_last_entry(self) -> Optional[LogEntry]:
        """Get the most recent log entry."""
        if self._entries:
            return self._entries[-1]
        return None

    def clear(self) -> None:
        """Clear all log entries."""
        self._entries.clear()

    def assert_event_logged(self, event_type: str, min_count: int = 1) -> bool:
        """
        Assert that an event was logged at least min_count times.

        Raises AssertionError if assertion fails.
        """
        count = len(self.get_entries_by_event(event_type))
        if count < min_count:
            raise AssertionError(
                f"Expected event '{event_type}' to be logged at least {min_count} times, "
                f"but it was logged {count} times.\n"
                f"Logged events: {[e.event for e in self._entries]}"
            )
        return True

    def assert_no_errors(self) -> bool:
        """
        Assert that no errors were logged.

        Raises AssertionError if any error entries exist.
        """
        errors = self.get_entries_by_level("ERROR")
        if errors:
            error_details = [f"  - {e.event}: {e.details}" for e in errors]
            raise AssertionError(
                f"Expected no errors, but found {len(errors)} error(s):\n" +
                "\n".join(error_details)
            )
        return True

    def to_dict_list(self) -> List[Dict[str, Any]]:
        """Export all entries as list of dicts."""
        return [
            {
                "timestamp": e.timestamp,
                "level": e.level,
                "component": e.component,
                "event": e.event,
                "details": e.details,
                "request_id": e.request_id
            }
            for e in self._entries
        ]
