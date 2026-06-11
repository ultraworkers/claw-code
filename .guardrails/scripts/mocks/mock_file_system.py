#!/usr/bin/env python3
"""
Mock file system for testing without actual file I/O.
"""

import json
import threading
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, Optional, Any, List
from datetime import datetime


@dataclass
class MockFile:
    """Represents a mock file in memory."""
    content: bytes = b""
    created_at: str = field(default_factory=lambda: datetime.now().isoformat())
    modified_at: str = field(default_factory=lambda: datetime.now().isoformat())
    is_locked: bool = False
    lock_holder: Optional[str] = None


class MockFileSystem:
    """
    In-memory file system for testing.

    Simulates file operations without touching the actual file system.
    Thread-safe for concurrent access testing.
    """

    def __init__(self):
        self._files: Dict[str, MockFile] = {}
        self._locks: Dict[str, threading.Lock] = {}
        self._operations_log: List[Dict] = []
        self._lock = threading.Lock()

    def _get_lock(self, path: str) -> threading.Lock:
        """Get or create a lock for a path."""
        with self._lock:
            if path not in self._locks:
                self._locks[path] = threading.Lock()
            return self._locks[path]

    def _log_operation(self, operation: str, path: str, **kwargs) -> None:
        """Log a file operation for test assertions."""
        self._operations_log.append({
            "operation": operation,
            "path": path,
            "timestamp": datetime.now().isoformat(),
            **kwargs
        })

    def exists(self, path: Path) -> bool:
        """Check if file exists."""
        return str(path) in self._files

    def read_text(self, path: Path) -> str:
        """Read file as text."""
        path_str = str(path)
        if path_str not in self._files:
            raise FileNotFoundError(f"File not found: {path}")
        self._log_operation("read_text", path_str)
        return self._files[path_str].content.decode('utf-8')

    def read_bytes(self, path: Path) -> bytes:
        """Read file as bytes."""
        path_str = str(path)
        if path_str not in self._files:
            raise FileNotFoundError(f"File not found: {path}")
        self._log_operation("read_bytes", path_str)
        return self._files[path_str].content

    def write_text(self, path: Path, content: str) -> None:
        """Write text to file."""
        path_str = str(path)
        with self._get_lock(path_str):
            if path_str in self._files:
                self._files[path_str].content = content.encode('utf-8')
                self._files[path_str].modified_at = datetime.now().isoformat()
            else:
                self._files[path_str] = MockFile(content=content.encode('utf-8'))
        self._log_operation("write_text", path_str, size=len(content))

    def write_bytes(self, path: Path, content: bytes) -> None:
        """Write bytes to file."""
        path_str = str(path)
        with self._get_lock(path_str):
            if path_str in self._files:
                self._files[path_str].content = content
                self._files[path_str].modified_at = datetime.now().isoformat()
            else:
                self._files[path_str] = MockFile(content=content)
        self._log_operation("write_bytes", path_str, size=len(content))

    def delete(self, path: Path) -> None:
        """Delete file."""
        path_str = str(path)
        with self._get_lock(path_str):
            if path_str in self._files:
                del self._files[path_str]
        self._log_operation("delete", path_str)

    def mkdir(self, path: Path, parents: bool = False, exist_ok: bool = False) -> None:
        """Create directory (no-op in mock, just logs)."""
        self._log_operation("mkdir", str(path), parents=parents, exist_ok=exist_ok)

    def read_json(self, path: Path) -> Any:
        """Read and parse JSON file."""
        content = self.read_text(path)
        return json.loads(content)

    def write_json(self, path: Path, data: Any, indent: int = 2) -> None:
        """Write data as JSON file."""
        content = json.dumps(data, indent=indent)
        self.write_text(path, content)

    def acquire_lock(self, path: Path, timeout: float = 30.0) -> bool:
        """Acquire file lock."""
        path_str = str(path)
        with self._get_lock(path_str):
            if path_str in self._files:
                if self._files[path_str].is_locked:
                    return False
                self._files[path_str].is_locked = True
                self._files[path_str].lock_holder = f"thread-{threading.current_thread().ident}"
        self._log_operation("acquire_lock", path_str, timeout=timeout)
        return True

    def release_lock(self, path: Path) -> None:
        """Release file lock."""
        path_str = str(path)
        with self._get_lock(path_str):
            if path_str in self._files:
                self._files[path_str].is_locked = False
                self._files[path_str].lock_holder = None
        self._log_operation("release_lock", path_str)

    def is_locked(self, path: Path) -> bool:
        """Check if file is locked."""
        path_str = str(path)
        if path_str in self._files:
            return self._files[path_str].is_locked
        return False

    def get_operations(self) -> List[Dict]:
        """Get logged operations for assertions."""
        return self._operations_log.copy()

    def clear_operations(self) -> None:
        """Clear operation log."""
        self._operations_log.clear()

    def reset(self) -> None:
        """Reset the entire file system."""
        with self._lock:
            self._files.clear()
            self._operations_log.clear()
            self._locks.clear()

    def list_files(self, pattern: str = "*") -> List[str]:
        """List files matching pattern."""
        import fnmatch
        return [p for p in self._files.keys() if fnmatch.fnmatch(p, pattern)]


class MockFileLock:
    """
    Mock file lock context manager.

    Simulates file locking without actual file system calls.
    """

    def __init__(self, fs: MockFileSystem, path: Path, timeout: float = 30.0):
        self.fs = fs
        self.path = path
        self.timeout = timeout
        self._acquired = False

    def __enter__(self):
        """Acquire lock."""
        import time
        start = time.time()
        while time.time() - start < self.timeout:
            if self.fs.acquire_lock(self.path, timeout=0.1):
                self._acquired = True
                return self
            time.sleep(0.05)
        raise TimeoutError(f"Could not acquire lock on {self.path} within {self.timeout}s")

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Release lock."""
        if self._acquired:
            self.fs.release_lock(self.path)
        return False
