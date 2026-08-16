from __future__ import annotations

import json
import re
from dataclasses import asdict, dataclass
from pathlib import Path


@dataclass(frozen=True)
class StoredSession:
    session_id: str
    messages: tuple[str, ...]
    input_tokens: int
    output_tokens: int


DEFAULT_SESSION_DIR = Path('.port_sessions')

# Session ids become filenames inside the session directory, so they must not be
# able to escape it via path separators or traversal. Restrict to the charset
# produced by legitimate ids (uuid4 hex, `session-<digits>-<n>`).
_SESSION_ID_PATTERN = re.compile(r'\A[A-Za-z0-9_-]+\Z')


def _validate_session_id(session_id: str) -> str:
    if not _SESSION_ID_PATTERN.match(session_id):
        raise ValueError(
            f'invalid session_id {session_id!r}: only [A-Za-z0-9_-] characters are allowed'
        )
    return session_id


def save_session(session: StoredSession, directory: Path | None = None) -> Path:
    session_id = _validate_session_id(session.session_id)
    target_dir = directory or DEFAULT_SESSION_DIR
    target_dir.mkdir(parents=True, exist_ok=True)
    path = target_dir / f'{session_id}.json'
    path.write_text(json.dumps(asdict(session), indent=2))
    return path


def load_session(session_id: str, directory: Path | None = None) -> StoredSession:
    session_id = _validate_session_id(session_id)
    target_dir = directory or DEFAULT_SESSION_DIR
    data = json.loads((target_dir / f'{session_id}.json').read_text())
    return StoredSession(
        session_id=data['session_id'],
        messages=tuple(data['messages']),
        input_tokens=data['input_tokens'],
        output_tokens=data['output_tokens'],
    )
