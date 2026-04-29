"""SQLite-backed project registry and session metadata.

One file at `<data_dir>/registry.sqlite`. Schema is intentionally tiny — we
let claw own conversation history (its session JSONL is the source of truth);
this DB only tracks the *project list* and *which claw session belongs to
which project*.

Schema:
    projects(id, name, path, created_at, last_opened_at)
    sessions(id, project_id, claw_session_id, title, created_at, updated_at)

Why not Postgres like Sentinel? Single user, single machine, file-shaped
data. SQLite is the right tool. Migrating later is a 50-line shim.
"""

from __future__ import annotations

import sqlite3
import time
from contextlib import contextmanager
from pathlib import Path
from typing import Iterator


SCHEMA = """
CREATE TABLE IF NOT EXISTS projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    last_opened_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    claw_session_id TEXT,
    title TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_id);
"""


def init_db(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with sqlite3.connect(path) as conn:
        conn.executescript(SCHEMA)
        conn.commit()


@contextmanager
def connect(path: Path) -> Iterator[sqlite3.Connection]:
    init_db(path)
    conn = sqlite3.connect(path)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON")
    try:
        yield conn
    finally:
        conn.close()


def now_ms() -> int:
    return int(time.time() * 1000)
