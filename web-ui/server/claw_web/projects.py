"""Project registry — list, create, mark-recent."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from .db import connect, now_ms


def list_projects(db_path: Path) -> list[dict[str, Any]]:
    with connect(db_path) as conn:
        rows = conn.execute(
            "SELECT id, name, path, created_at, last_opened_at "
            "FROM projects ORDER BY last_opened_at DESC"
        ).fetchall()
        return [dict(r) for r in rows]


def get_project(db_path: Path, project_id: int) -> dict[str, Any] | None:
    with connect(db_path) as conn:
        row = conn.execute(
            "SELECT id, name, path, created_at, last_opened_at "
            "FROM projects WHERE id = ?",
            (project_id,),
        ).fetchone()
        return dict(row) if row else None


def create_project(
    db_path: Path, name: str, path: str
) -> dict[str, Any]:
    """Register a project. `path` should already exist on disk; the caller
    is responsible for creating it (the new-project flow). We don't enforce
    existence here so phase-2 remote-mount scenarios remain open."""
    resolved = str(Path(path).expanduser().resolve())
    ts = now_ms()
    with connect(db_path) as conn:
        cur = conn.execute(
            "INSERT INTO projects (name, path, created_at, last_opened_at) "
            "VALUES (?, ?, ?, ?)",
            (name, resolved, ts, ts),
        )
        conn.commit()
        pid = cur.lastrowid
    return {
        "id": pid,
        "name": name,
        "path": resolved,
        "created_at": ts,
        "last_opened_at": ts,
    }


def touch_project(db_path: Path, project_id: int) -> None:
    with connect(db_path) as conn:
        conn.execute(
            "UPDATE projects SET last_opened_at = ? WHERE id = ?",
            (now_ms(), project_id),
        )
        conn.commit()
