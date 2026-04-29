"""Smoke test for the claw-web scaffold.

Exercises the project + session round-trip and one mock-mode WS turn.
Not a functional test of claw integration — that depends on the binary
being built locally.
"""

from __future__ import annotations

import json
import os
import tempfile
from pathlib import Path

import pytest


def test_extract_last_json_object_after_prompt_text():
    """Real-world: claw can emit an approval prompt + JSON on the same
    line under --output-format json when a tool needs permission. The
    extractor must still find the JSON object."""
    from claw_web.claw_runner import _extract_last_json_object, _extract_text

    stdout = (
        'Permission approval required\n'
        '  Tool             bash\n'
        '  Required mode    danger-full-access\n'
        'Approve this tool call? [y/N]: {"auto_compaction":null,'
        '"estimated_cost":"$0.68","iterations":2,"message":"hi there",'
        '"model":"openai/x","tool_uses":[],"tool_results":[]}\n'
    )
    parsed = _extract_last_json_object(stdout)
    assert parsed is not None
    assert parsed["message"] == "hi there"
    assert _extract_text(parsed) == "hi there"


def test_extract_last_json_object_clean_json():
    from claw_web.claw_runner import _extract_last_json_object
    parsed = _extract_last_json_object('{"message":"ok"}')
    assert parsed == {"message": "ok"}


def test_extract_last_json_object_nothing():
    from claw_web.claw_runner import _extract_last_json_object
    assert _extract_last_json_object("just plain text") is None
    assert _extract_last_json_object("") is None


def test_approval_regex_matches_real_world_prompt():
    from claw_web.claw_runner import _APPROVAL_RE

    blob = (
        "Permission approval required\n"
        "  Tool             bash\n"
        "  Current mode     read-only\n"
        "  Required mode    danger-full-access\n"
        "  Reason           tool 'bash' requires approval due to ask rule 'bash'\n"
        '  Input            {"command":"git status","description":"Check status"}\n'
        "Approve this tool call? [y/N]: "
    )
    m = _APPROVAL_RE.search(blob)
    assert m is not None
    assert m.group("tool") == "bash"
    assert m.group("current") == "read-only"
    assert m.group("required") == "danger-full-access"
    assert "ask rule" in m.group("reason")
    assert m.group("input").startswith('{"command":"git status"')


def test_approval_regex_handles_workspace_write_mode():
    from claw_web.claw_runner import _APPROVAL_RE

    blob = (
        "Permission approval required\n"
        "  Tool             write_file\n"
        "  Current mode     workspace-write\n"
        "  Required mode    workspace-write\n"
        "  Reason           ask rule on write_file\n"
        '  Input            {"path":"x.txt"}\n'
        "Approve this tool call? [y/N]:"
    )
    m = _APPROVAL_RE.search(blob)
    assert m is not None
    assert m.group("tool") == "write_file"
    assert m.group("current") == "workspace-write"


def test_mock_approval_round_trip_allow(app, tmp_path):
    """End-to-end: client sends a `@@approve` prompt, server emits an
    approval_request, client sends approval_response with approve=true,
    server resumes and sends final containing the granted-flag."""
    from fastapi.testclient import TestClient

    proj = tmp_path / "proj"
    proj.mkdir()

    with TestClient(app) as client:
        pid = client.post(
            "/api/projects", json={"name": "t", "path": str(proj)},
        ).json()["id"]
        sid = client.post(
            f"/api/projects/{pid}/sessions", json={"title": "approve"},
        ).json()["id"]

        with client.websocket_connect(f"/api/sessions/{sid}/stream") as ws:
            assert ws.receive_json()["kind"] == "ready"
            ws.send_text(json.dumps({"prompt": "@@approve and proceed"}))

            saw_request = False
            while True:
                evt = ws.receive_json()
                if evt["kind"] == "approval_request":
                    saw_request = True
                    assert evt["payload"]["tool"] == "bash"
                    ws.send_text(json.dumps({
                        "kind": "approval_response", "approve": True,
                    }))
                elif evt["kind"] == "final":
                    assert saw_request
                    assert "approval granted" in evt["payload"]["text"]
                    assert evt["payload"]["approvals_handled"] == 1
                    break
                elif evt["kind"] == "error":
                    raise AssertionError(evt["payload"])


def test_mock_approval_round_trip_deny(app, tmp_path):
    from fastapi.testclient import TestClient

    proj = tmp_path / "proj"
    proj.mkdir()

    with TestClient(app) as client:
        pid = client.post(
            "/api/projects", json={"name": "t", "path": str(proj)},
        ).json()["id"]
        sid = client.post(
            f"/api/projects/{pid}/sessions", json={"title": "deny"},
        ).json()["id"]

        with client.websocket_connect(f"/api/sessions/{sid}/stream") as ws:
            assert ws.receive_json()["kind"] == "ready"
            ws.send_text(json.dumps({"prompt": "@@approve nope"}))

            while True:
                evt = ws.receive_json()
                if evt["kind"] == "approval_request":
                    ws.send_text(json.dumps({
                        "kind": "approval_response", "approve": False,
                    }))
                elif evt["kind"] == "final":
                    assert "approval denied" in evt["payload"]["text"]
                    break
                elif evt["kind"] == "error":
                    raise AssertionError(evt["payload"])


def test_read_session_history_extracts_user_assistant(tmp_path):
    """JSONL parser pulls user + assistant text out, ignores tool blocks."""
    from claw_web.claw_runner import _read_session_history

    jsonl = tmp_path / "session.jsonl"
    jsonl.write_text(
        '{"type":"session_meta","session_id":"x","model":"m","workspace_root":"/"}\n'
        '{"type":"message","message":{"role":"user","blocks":[{"type":"text","text":"hi there"}]}}\n'
        '{"type":"message","message":{"role":"assistant","blocks":[{"type":"text","text":"\\n\\n"},'
        '{"type":"tool_use","name":"bash","input":"{}","id":"1"}]}}\n'
        '{"type":"message","message":{"role":"assistant","blocks":[{"type":"text","text":"all good"}]}}\n'
    )
    history = _read_session_history(jsonl)
    assert history == [("user", "hi there"), ("assistant", "all good")]


def test_format_history_prompt_includes_prior_turns():
    from claw_web.claw_runner import _format_history_prompt

    history = [("user", "first"), ("assistant", "second"), ("user", "third")]
    rendered = _format_history_prompt(history, "what now?")
    assert "Earlier in this conversation" in rendered
    assert "User: first" in rendered
    assert "Assistant: second" in rendered
    assert "User: third" in rendered
    assert rendered.endswith("User: what now?")


def test_format_history_prompt_empty_history_passthrough():
    from claw_web.claw_runner import _format_history_prompt

    assert _format_history_prompt([], "hello") == "hello"


def _is_alive(pid: int) -> bool:
    """Return True if the given pid still exists on this OS."""
    try:
        os.kill(pid, 0)
    except (ProcessLookupError, PermissionError):
        return False
    except OSError:
        return False
    return True


def test_subprocess_killed_on_disconnect(tmp_path, monkeypatch):
    """Abort path: spawn a long-running fake claw, disconnect mid-turn,
    confirm the subprocess is killed and doesn't outlive the WebSocket."""
    import asyncio
    import time

    from claw_web.claw_runner import ClawRunner

    # Fake claw binary that prints a status line then sleeps "forever".
    # Using `python -u` so prints are unbuffered and we can read them.
    fake = tmp_path / "fake-claw.sh"
    fake.write_text(
        "#!/usr/bin/env bash\n"
        "echo 'fake claw alive'\n"
        # Sleep 60s — way longer than the test runs.
        "sleep 60\n"
    )
    fake.chmod(0o755)

    runner = ClawRunner(claw_bin=fake)
    proj_dir = tmp_path / "proj"
    proj_dir.mkdir()

    captured_pid: list[int] = []

    async def drive():
        gen = runner.run_turn(
            cwd=proj_dir,
            prompt="hi",
            claw_session_id=None,
            approval_callback=None,
        )
        try:
            async for evt in gen:
                if evt.kind == "status" and evt.payload.get("pid"):
                    captured_pid.append(evt.payload["pid"])
                    break
        finally:
            # Outside the async-for: simulate WS-disconnect cleanup.
            # aclose() throws GeneratorExit at the runner's suspended
            # yield, the runner's finally kills + reaps the proc.
            await gen.aclose()

    asyncio.run(drive())

    assert captured_pid, "runner never reported a pid"
    pid = captured_pid[0]

    # Give the kernel a beat to reap.
    deadline = time.time() + 2.0
    while time.time() < deadline:
        if not _is_alive(pid):
            break
        time.sleep(0.05)
    assert not _is_alive(pid), f"subprocess {pid} survived disconnect"


@pytest.fixture()
def app(monkeypatch):
    tmp = tempfile.mkdtemp(prefix="claw_web_test_")
    monkeypatch.setenv("CLAW_WEB_DATA_DIR", tmp)
    monkeypatch.setenv("CLAW_WEB_MODE", "mock")
    # Force a fresh module reload so the cached settings reflect env.
    import importlib

    import claw_web.api as api_mod

    importlib.reload(api_mod)
    return api_mod.create_app()


def test_health(app):
    from fastapi.testclient import TestClient

    with TestClient(app) as client:
        r = client.get("/api/health")
        assert r.status_code == 200
        body = r.json()
        assert body["ok"] is True
        assert body["mode"] == "mock"


def test_project_session_roundtrip(app, tmp_path):
    from fastapi.testclient import TestClient

    proj_dir = tmp_path / "proj"
    proj_dir.mkdir()

    with TestClient(app) as client:
        r = client.post(
            "/api/projects",
            json={"name": "demo", "path": str(proj_dir)},
        )
        assert r.status_code == 200
        pid = r.json()["id"]

        r = client.get("/api/projects")
        assert r.status_code == 200
        assert any(p["id"] == pid for p in r.json()["projects"])

        r = client.post(
            f"/api/projects/{pid}/sessions",
            json={"title": "first turn"},
        )
        assert r.status_code == 200
        sid = r.json()["id"]

        r = client.get(f"/api/projects/{pid}/sessions")
        assert any(s["id"] == sid for s in r.json()["sessions"])


def test_mock_turn_stream(app, tmp_path):
    from fastapi.testclient import TestClient

    proj_dir = tmp_path / "proj"
    proj_dir.mkdir()

    with TestClient(app) as client:
        pid = client.post(
            "/api/projects",
            json={"name": "demo", "path": str(proj_dir)},
        ).json()["id"]
        sid = client.post(
            f"/api/projects/{pid}/sessions",
            json={"title": "t"},
        ).json()["id"]

        with client.websocket_connect(f"/api/sessions/{sid}/stream") as ws:
            ready = ws.receive_json()
            assert ready["kind"] == "ready"
            ws.send_text(json.dumps({"prompt": "hello world"}))
            saw_final = False
            while True:
                evt = ws.receive_json()
                if evt["kind"] == "final":
                    assert "hello world" in evt["payload"]["text"]
                    saw_final = True
                    break
                if evt["kind"] == "error":
                    raise AssertionError(evt["payload"])
            assert saw_final
