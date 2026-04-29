"""Session metadata.

Conversation *content* lives in claw's own session JSONL files
(`<project>/.claw/sessions/<hash>/session-*.jsonl`). This module only tracks:

  - which sessions belong to which project (so we can list them)
  - the claw session id (so we can pass `--resume <id>` next turn)
  - a human title (the first user message, truncated)

That keeps us out of the business of duplicating claw's history format.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from .db import connect, now_ms


def list_sessions(db_path: Path, project_id: int) -> list[dict[str, Any]]:
    with connect(db_path) as conn:
        rows = conn.execute(
            "SELECT id, project_id, claw_session_id, title, created_at, updated_at "
            "FROM sessions WHERE project_id = ? "
            "ORDER BY updated_at DESC",
            (project_id,),
        ).fetchall()
        return [dict(r) for r in rows]


def get_session(db_path: Path, session_id: int) -> dict[str, Any] | None:
    with connect(db_path) as conn:
        row = conn.execute(
            "SELECT id, project_id, claw_session_id, title, created_at, updated_at "
            "FROM sessions WHERE id = ?",
            (session_id,),
        ).fetchone()
        return dict(row) if row else None


def create_session(
    db_path: Path, project_id: int, title: str
) -> dict[str, Any]:
    ts = now_ms()
    with connect(db_path) as conn:
        cur = conn.execute(
            "INSERT INTO sessions (project_id, claw_session_id, title, "
            "created_at, updated_at) VALUES (?, NULL, ?, ?, ?)",
            (project_id, title, ts, ts),
        )
        conn.commit()
        sid = cur.lastrowid
    return {
        "id": sid,
        "project_id": project_id,
        "claw_session_id": None,
        "title": title,
        "created_at": ts,
        "updated_at": ts,
    }


def attach_claw_session(
    db_path: Path, session_id: int, claw_session_id: str
) -> None:
    """Called the first time claw reports back a session id, so we can
    `--resume <id>` on subsequent turns."""
    with connect(db_path) as conn:
        conn.execute(
            "UPDATE sessions SET claw_session_id = ?, updated_at = ? "
            "WHERE id = ?",
            (claw_session_id, now_ms(), session_id),
        )
        conn.commit()


def touch_session(db_path: Path, session_id: int) -> None:
    with connect(db_path) as conn:
        conn.execute(
            "UPDATE sessions SET updated_at = ? WHERE id = ?",
            (now_ms(), session_id),
        )
        conn.commit()
