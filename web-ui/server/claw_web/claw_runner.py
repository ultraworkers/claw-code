"""Subprocess-mode runner: invoke `claw` once per turn, parse JSON output.

Streaming with stdin-piped permission mediation. The runner spawns claw with
`--output-format json` (so we still get a structured blob at end-of-turn),
but reads stdout incrementally so it can detect claw's `Approve this tool
call? [y/N]:` prompts mid-turn, surface them through an awaitable callback,
and write the user's `y` / `n` answer back to claw's stdin. Multiple
prompts per turn round-trip cleanly.

Why subprocess instead of linking the runtime crate? Subprocess decouples
us from claw internals — version skew is a Cargo.toml concern, not an API
one — and lets the wrapper survive claw refactors. The cost is per-turn
startup (~1s on this box). Acceptable for the prototype.
"""

from __future__ import annotations

import asyncio
import json
import os
import re
import shlex
import shutil
from dataclasses import dataclass
from pathlib import Path
from typing import AsyncIterator, Awaitable, Callable


@dataclass
class TurnEvent:
    """A single update from a turn-in-progress.

    `kind` is one of:
      - "status"            : informational ("starting", "running", ...).
      - "approval_request"  : the runner needs an Allow/Deny decision before
                              the turn can continue. Payload carries the
                              parsed approval-prompt fields. The runner
                              awaits the callback before continuing.
      - "final"             : terminal event; payload carries assistant text
                              + claw session id.
      - "error"             : terminal event; payload.message set.
    """

    kind: str
    payload: dict


@dataclass
class ApprovalRequest:
    tool: str
    current_mode: str
    required_mode: str
    reason: str
    input: str

    def as_dict(self) -> dict:
        return {
            "tool": self.tool,
            "current_mode": self.current_mode,
            "required_mode": self.required_mode,
            "reason": self.reason,
            "input": self.input,
        }


ApprovalCallback = Callable[[ApprovalRequest], Awaitable[bool]]


# Matches claw's permission approval block, ANSI-stripped. Tolerant of:
# - mode names with hyphens / spaces (`read-only`, `danger-full-access`).
# - reason / input on a single line.
# - missing trailing whitespace after the `[y/N]:` marker (claw flushes
#   the prompt then waits for stdin without a newline).
_APPROVAL_RE = re.compile(
    r"Permission approval required\s*\n"
    r"\s+Tool\s+(?P<tool>\S+)\s*\n"
    r"\s+Current mode\s+(?P<current>\S[^\n]*?)\s*\n"
    r"\s+Required mode\s+(?P<required>\S[^\n]*?)\s*\n"
    r"\s+Reason\s+(?P<reason>[^\n]+)\n"
    r"\s+Input\s+(?P<input>[^\n]+)\n"
    r"Approve this tool call\? \[y/N\]:[ \t]*"
)

# ANSI CSI escape stripper. Claw's TUI uses color/cursor codes; the prompt
# block is plain text but neighboring output may carry escapes that would
# break our regex if left in.
_ANSI_RE = re.compile(r"\x1b\[[0-9;]*[a-zA-Z]")


class ClawRunner:
    def __init__(self, claw_bin: Path, model: str | None = None):
        self.claw_bin = claw_bin
        self.model = model

    def is_available(self) -> bool:
        return self.claw_bin.exists() or shutil.which(str(self.claw_bin)) is not None

    async def run_turn(
        self,
        cwd: Path,
        prompt: str,
        claw_session_id: str | None,
        env_overrides: dict[str, str] | None = None,
        approval_callback: ApprovalCallback | None = None,
    ) -> AsyncIterator[TurnEvent]:
        if not self.is_available():
            yield TurnEvent(
                "error",
                {"message": f"claw binary not found at {self.claw_bin}"},
            )
            return

        cmd: list[str] = [str(self.claw_bin), "--output-format", "json"]
        if self.model:
            cmd += ["--model", self.model]
        # claw's CLI doesn't support continuing a session with a new user
        # prompt (`--resume` is for slash-command inspection only). As a
        # stopgap we replay prior turns from the previous session JSONL
        # as plain text inside the new prompt. Token-wasteful but works
        # without claw changes. See PLAN.md "Phase 1.6 — multi-turn
        # context" for the upstream-fix path.
        effective_prompt = prompt
        if claw_session_id:
            jsonl_path = Path(claw_session_id)
            if jsonl_path.is_file():
                history = _read_session_history(jsonl_path)
                if history:
                    effective_prompt = _format_history_prompt(history, prompt)
        cmd += ["prompt", effective_prompt]

        env = os.environ.copy()
        if env_overrides:
            env.update(env_overrides)

        env_fp = _env_fingerprint(env)
        # Capture spawn time so we can find the JSONL claw writes during
        # this turn, even if older session JSONLs exist for the same cwd.
        import time as _time
        spawn_time = _time.time()
        yield TurnEvent(
            "status",
            {"message": "starting", "cmd": shlex.join(cmd), "env": env_fp},
        )

        try:
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                cwd=str(cwd),
                env=env,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
        except FileNotFoundError as e:
            yield TurnEvent("error", {"message": f"spawn failed: {e}"})
            return

        # Drain stderr concurrently so a noisy claw can't deadlock its own
        # pipe while we're busy reading stdout.
        stderr_buf = bytearray()

        async def drain_stderr():
            while True:
                chunk = await proc.stderr.read(4096)
                if not chunk:
                    break
                stderr_buf.extend(chunk)

        stderr_task = asyncio.create_task(drain_stderr())

        # `streaming_done` distinguishes a clean EOF from any abnormal exit
        # (CancelledError, GeneratorExit from `.aclose()`, or a thrown
        # exception). Anything other than a clean EOF means we should kill
        # the subprocess on the way out — claw blocking on stdin must not
        # outlive its WebSocket.
        streaming_done = False
        clean_buf = ""
        approvals_handled = 0

        # The post-spawn try/finally MUST wrap every later yield. If the
        # consumer breaks/aclose-s while suspended on a yield outside this
        # block, the cleanup is skipped and claw outlives its caller.
        try:
            yield TurnEvent("status", {"message": "running", "pid": proc.pid})

            try:
                async for evt, new_clean in self._stream_stdout(
                    proc, clean_buf, approval_callback
                ):
                    if evt is not None:
                        if evt.kind == "approval_request":
                            approvals_handled += 1
                        yield evt
                    clean_buf = new_clean
                streaming_done = True
            except Exception as e:
                # Surface ordinary exceptions; CancelledError /
                # GeneratorExit fall through to finally without yielding.
                yield TurnEvent(
                    "error",
                    {"message": f"stream error: {type(e).__name__}: {e}"},
                )
        finally:
            if not streaming_done and proc.returncode is None:
                try:
                    proc.kill()
                except (ProcessLookupError, Exception):
                    pass
            if proc.returncode is None:
                try:
                    await asyncio.wait_for(proc.wait(), timeout=2.0)
                except (asyncio.TimeoutError, Exception):
                    pass
            if not stderr_task.done():
                stderr_task.cancel()
                try:
                    await stderr_task
                except (asyncio.CancelledError, Exception):
                    pass

        if not streaming_done:
            return

        stderr = bytes(stderr_buf).decode("utf-8", errors="replace")

        if proc.returncode != 0:
            yield TurnEvent(
                "error",
                {
                    "message": f"claw exited {proc.returncode}",
                    "stderr": stderr.strip(),
                    "approvals_handled": approvals_handled,
                },
            )
            return

        parsed: dict | None = None
        try:
            obj = json.loads(clean_buf)
            parsed = obj if isinstance(obj, dict) else None
        except json.JSONDecodeError:
            parsed = _extract_last_json_object(clean_buf)

        if parsed is None:
            yield TurnEvent(
                "final",
                {
                    "text": clean_buf.strip(),
                    "claw_session_id": None,
                    "raw": True,
                    "approvals_handled": approvals_handled,
                },
            )
            return

        text = _extract_text(parsed)
        sid = (
            parsed.get("session_id")
            or parsed.get("session")
            or parsed.get("session_uuid")
            or parsed.get("id")
        )
        # If claw doesn't expose the session id in the JSON output (the
        # case as of 2026-04-29), fall back to the JSONL file path it just
        # wrote. `--resume` accepts a path, a session id, or `latest`, so
        # the path works as a stable reference for subsequent turns.
        if not sid:
            jsonl = _find_recent_session_jsonl(cwd, since=spawn_time)
            if jsonl is not None:
                sid = str(jsonl)
        payload: dict = {
            "text": text,
            "claw_session_id": sid,
            "raw": False,
            "blob": parsed,
            "approvals_handled": approvals_handled,
            "debug_keys": sorted(parsed.keys()) if isinstance(parsed, dict) else [],
        }
        yield TurnEvent("final", payload)

    async def _stream_stdout(
        self,
        proc: asyncio.subprocess.Process,
        clean_buf: str,
        approval_callback: ApprovalCallback | None,
    ):
        """Read stdout in chunks, yielding (TurnEvent, updated_clean_buf).

        Yields `(None, clean_buf)` to refresh the caller's view when no
        event was produced for a chunk; yields `(TurnEvent, clean_buf)` for
        approval requests. The final JSON blob accumulates into `clean_buf`
        and is returned via the last yield.

        Emits a `chunk` status event for every read so the UI can tell
        whether claw is producing output at all (vs. stalled / deadlocked
        on a flush). Includes a short preview so prompt-format drift is
        visible without needing to crack the runner open.
        """
        assert proc.stdout is not None
        chunks_received = 0
        bytes_total = 0
        while True:
            chunk = await proc.stdout.read(4096)
            if not chunk:
                yield None, clean_buf
                return
            chunks_received += 1
            bytes_total += len(chunk)
            decoded = chunk.decode("utf-8", errors="replace")
            cleaned = _ANSI_RE.sub("", decoded)
            clean_buf += cleaned

            yield TurnEvent("status", {
                "message": "chunk",
                "chunks": chunks_received,
                "bytes": bytes_total,
                "preview": _preview(cleaned),
            }), clean_buf

            # Drain every prompt currently visible in the buffer before
            # reading more. claw can emit multiple prompts back-to-back
            # (e.g. sequential bash calls) and we want each round-trip to
            # complete before pulling the next chunk.
            while True:
                m = _APPROVAL_RE.search(clean_buf)
                if not m:
                    break
                req = ApprovalRequest(
                    tool=m.group("tool").strip(),
                    current_mode=m.group("current").strip(),
                    required_mode=m.group("required").strip(),
                    reason=m.group("reason").strip(),
                    input=m.group("input").strip(),
                )
                yield TurnEvent("approval_request", req.as_dict()), clean_buf

                if approval_callback is not None:
                    try:
                        approve = bool(await approval_callback(req))
                    except Exception:
                        approve = False
                else:
                    approve = False

                if proc.stdin is not None:
                    proc.stdin.write(b"y\n" if approve else b"n\n")
                    try:
                        await proc.stdin.drain()
                    except (ConnectionResetError, BrokenPipeError):
                        pass

                # Drop the matched prompt out of the buffer so we don't
                # re-detect it and so the trailing JSON blob ends up alone.
                clean_buf = clean_buf[m.end():]
                yield None, clean_buf


def _preview(s: str, n: int = 200) -> str:
    """Tiny human preview of a chunk. Trims whitespace, collapses newlines
    to `\\n`, caps at N chars with an ellipsis."""
    s = s.strip().replace("\n", "\\n").replace("\r", "")
    if len(s) > n:
        s = s[:n] + "…"
    return s


def _read_session_history(jsonl_path: Path) -> list[tuple[str, str]]:
    """Pull (role, text) tuples out of a claw session JSONL.

    Tool calls / tool results are intentionally dropped — the model can
    follow the user/assistant thread without them, and including them
    explodes the prompt budget.
    """
    history: list[tuple[str, str]] = []
    try:
        with jsonl_path.open("r", encoding="utf-8") as f:
            for raw in f:
                raw = raw.strip()
                if not raw:
                    continue
                try:
                    evt = json.loads(raw)
                except json.JSONDecodeError:
                    continue
                if evt.get("type") != "message":
                    continue
                msg = evt.get("message") or {}
                role = msg.get("role")
                if role not in ("user", "assistant"):
                    continue
                text_parts: list[str] = []
                for block in msg.get("blocks") or []:
                    if isinstance(block, dict) and block.get("type") == "text":
                        t = block.get("text") or ""
                        if t.strip():
                            text_parts.append(t)
                if text_parts:
                    history.append((role, "".join(text_parts).strip()))
    except OSError:
        return []
    return history


def _format_history_prompt(
    history: list[tuple[str, str]], new_prompt: str
) -> str:
    """Render a transcript prefix the model can follow.

    Phase-1.6 stopgap: claw has no CLI-driven multi-turn continuation, so
    we replay prior turns as plain text in the new prompt. Token-wasteful
    by design — a real fix needs an upstream structured-stdio mode."""
    if not history:
        return new_prompt
    parts = ["Earlier in this conversation:", ""]
    for role, text in history:
        label = "User" if role == "user" else "Assistant"
        parts.append(f"{label}: {text}")
        parts.append("")
    parts.append("Now respond to the next user message:")
    parts.append("")
    parts.append(f"User: {new_prompt}")
    return "\n".join(parts)


def _find_recent_session_jsonl(cwd: Path, since: float) -> Path | None:
    """Find the JSONL claw most recently wrote under `<cwd>/.claw/sessions/`.

    Restricted to files modified at or after `since` so we don't pick up an
    older session that happens to share the cwd. Returns None if no
    matching file exists (e.g., claw hasn't written one yet, or the layout
    changed).
    """
    sessions_dir = cwd / ".claw" / "sessions"
    if not sessions_dir.is_dir():
        return None
    # Allow a small clock skew tolerance so a JSONL stamped right at spawn
    # time isn't excluded.
    threshold = since - 1.0
    candidates: list[tuple[float, Path]] = []
    try:
        for hash_dir in sessions_dir.iterdir():
            if not hash_dir.is_dir():
                continue
            for f in hash_dir.glob("session-*.jsonl"):
                try:
                    mtime = f.stat().st_mtime
                except OSError:
                    continue
                if mtime >= threshold:
                    candidates.append((mtime, f))
    except OSError:
        return None
    if not candidates:
        return None
    candidates.sort(reverse=True)
    return candidates[0][1]


def _extract_last_json_object(s: str) -> dict | None:
    """Find the last top-level `{...}` JSON object in `s` and parse it."""
    if not s:
        return None
    in_str = False
    escape = False
    depth = 0
    last_close = -1
    for i, ch in enumerate(s):
        if escape:
            escape = False
            continue
        if ch == "\\" and in_str:
            escape = True
            continue
        if ch == '"':
            in_str = not in_str
            continue
        if in_str:
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                last_close = i
    if last_close < 0:
        return None
    depth = 0
    start = -1
    for j in range(last_close, -1, -1):
        ch = s[j]
        if ch == "}":
            depth += 1
        elif ch == "{":
            depth -= 1
            if depth == 0:
                start = j
                break
    if start < 0:
        return None
    candidate = s[start:last_close + 1]
    try:
        result = json.loads(candidate)
        return result if isinstance(result, dict) else None
    except json.JSONDecodeError:
        return None


def _extract_text(parsed) -> str:
    """Pull assistant text out of claw's JSON output."""
    if isinstance(parsed, str):
        return parsed
    if not isinstance(parsed, dict):
        return ""
    for key in ("message", "text", "response", "assistant", "output", "result", "content"):
        v = parsed.get(key)
        if isinstance(v, str) and v.strip():
            return v
        if isinstance(v, list):
            joined = "".join(
                b.get("text", "") for b in v
                if isinstance(b, dict) and b.get("type") == "text"
            )
            if joined.strip():
                return joined
    content = parsed.get("content")
    if isinstance(content, list):
        joined = "".join(
            b.get("text", "") for b in content
            if isinstance(b, dict) and b.get("type") == "text"
        )
        if joined.strip():
            return joined
    return ""


def _env_fingerprint(env: dict) -> dict:
    """Routing-relevant env presence; never leaks credential values."""
    def present(key: str) -> dict:
        v = env.get(key)
        if not v:
            return {"set": False}
        return {"set": True, "len": len(v)}

    return {
        "OPENAI_BASE_URL": env.get("OPENAI_BASE_URL") or None,
        "OPENAI_API_KEY": present("OPENAI_API_KEY"),
        "ANTHROPIC_API_KEY": present("ANTHROPIC_API_KEY"),
        "ANTHROPIC_AUTH_TOKEN": present("ANTHROPIC_AUTH_TOKEN"),
    }
