"""FastAPI app: REST routes for projects + sessions, WS for turn streaming.

Kept in a single module while phase 1 stays small. Split into a `routes/`
package when handlers grow non-trivial.
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
import select
import threading
from contextlib import asynccontextmanager
from pathlib import Path

from fastapi import FastAPI, HTTPException, WebSocket, WebSocketDisconnect
from fastapi.responses import FileResponse, JSONResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field

from . import projects as projects_repo
from . import sessions as sessions_repo
from . import tmux_manager
from .claw_runner import ApprovalRequest, ClawRunner, TurnEvent
from .config import Settings
from .db import init_db
from .mock_runner import MockRunner

log = logging.getLogger("claw_web")


class CreateProjectIn(BaseModel):
    name: str = Field(min_length=1, max_length=200)
    path: str = Field(min_length=1)


class CreateSessionIn(BaseModel):
    title: str = Field(min_length=1, max_length=300)


def _make_runner(settings: Settings):
    if settings.mode == "subprocess":
        return ClawRunner(claw_bin=settings.claw_bin, model=settings.model)
    return MockRunner()


def _data_paths(settings: Settings) -> tuple[Path, Path]:
    settings.data_dir.mkdir(parents=True, exist_ok=True)
    return settings.data_dir / "registry.sqlite", settings.data_dir


def create_app(settings: Settings | None = None) -> FastAPI:
    settings = settings or Settings.from_env()
    db_path, _ = _data_paths(settings)
    init_db(db_path)
    runner = _make_runner(settings)

    @asynccontextmanager
    async def lifespan(app: FastAPI):
        log.info(
            "claw-web starting: mode=%s data=%s static=%s",
            settings.mode,
            settings.data_dir,
            settings.static_dir,
        )
        yield

    app = FastAPI(title="claw-web", version="0.1.0", lifespan=lifespan)

    @app.get("/api/health")
    def health():
        return {
            "ok": True,
            "mode": settings.mode,
            "claw_bin": str(settings.claw_bin),
            "claw_available": runner.is_available(),
            "data_dir": str(settings.data_dir),
        }

    @app.get("/api/projects")
    def list_projects():
        return {"projects": projects_repo.list_projects(db_path)}

    @app.post("/api/projects")
    def create_project(body: CreateProjectIn):
        try:
            return projects_repo.create_project(db_path, body.name, body.path)
        except Exception as e:
            raise HTTPException(status_code=400, detail=str(e)) from e

    @app.get("/api/projects/{project_id}")
    def get_project(project_id: int):
        proj = projects_repo.get_project(db_path, project_id)
        if not proj:
            raise HTTPException(status_code=404, detail="project not found")
        return proj

    @app.get("/api/projects/{project_id}/sessions")
    def list_sessions(project_id: int):
        proj = projects_repo.get_project(db_path, project_id)
        if not proj:
            raise HTTPException(status_code=404, detail="project not found")
        projects_repo.touch_project(db_path, project_id)
        return {"sessions": sessions_repo.list_sessions(db_path, project_id)}

    @app.post("/api/projects/{project_id}/sessions")
    def create_session(project_id: int, body: CreateSessionIn):
        proj = projects_repo.get_project(db_path, project_id)
        if not proj:
            raise HTTPException(status_code=404, detail="project not found")
        return sessions_repo.create_session(db_path, project_id, body.title)

    @app.get("/api/sessions/{session_id}")
    def get_session(session_id: int):
        sess = sessions_repo.get_session(db_path, session_id)
        if not sess:
            raise HTTPException(status_code=404, detail="session not found")
        return sess

    @app.websocket("/api/sessions/{session_id}/stream")
    async def session_stream(ws: WebSocket, session_id: int):
        await ws.accept()
        sess = sessions_repo.get_session(db_path, session_id)
        if not sess:
            await ws.send_json({"kind": "error", "payload": {"message": "no session"}})
            await ws.close()
            return
        proj = projects_repo.get_project(db_path, sess["project_id"])
        if not proj:
            await ws.send_json({"kind": "error", "payload": {"message": "no project"}})
            await ws.close()
            return

        await ws.send_json(
            {
                "kind": "ready",
                "payload": {
                    "session": sess,
                    "project": proj,
                    "mode": settings.mode,
                },
            }
        )

        try:
            while True:
                raw = await ws.receive_text()
                try:
                    msg = json.loads(raw)
                except json.JSONDecodeError:
                    await ws.send_json(
                        {"kind": "error", "payload": {"message": "bad json"}}
                    )
                    continue

                prompt = (msg.get("prompt") or "").strip()
                if not prompt:
                    await ws.send_json(
                        {"kind": "error", "payload": {"message": "empty prompt"}}
                    )
                    continue

                # Refresh the session row so we have the latest claw_session_id.
                sess = sessions_repo.get_session(db_path, session_id) or sess

                # Approval mediator. While a turn is mid-flight, the runner
                # may emit approval_request events; we await the client's
                # approval_response on the same WS. The main message loop
                # is suspended during a turn, so this read can't conflict.
                async def approval_callback(req: ApprovalRequest) -> bool:
                    while True:
                        try:
                            raw = await ws.receive_text()
                        except WebSocketDisconnect:
                            raise  # let the outer handler clean up
                        try:
                            msg = json.loads(raw)
                        except json.JSONDecodeError:
                            continue
                        if msg.get("kind") == "approval_response":
                            return bool(msg.get("approve"))
                        # Drop unrelated messages on the floor while waiting.

                async for event in runner.run_turn(
                    cwd=Path(proj["path"]),
                    prompt=prompt,
                    claw_session_id=sess.get("claw_session_id"),
                    approval_callback=approval_callback,
                ):
                    await ws.send_json(_serialize(event))
                    if event.kind == "final":
                        sid = event.payload.get("claw_session_id")
                        if sid and not sess.get("claw_session_id"):
                            sessions_repo.attach_claw_session(
                                db_path, session_id, sid
                            )
                            sess["claw_session_id"] = sid
                        sessions_repo.touch_session(db_path, session_id)
        except WebSocketDisconnect:
            return
        except Exception as e:  # surface unexpected errors to the client
            log.exception("ws handler crashed")
            try:
                await ws.send_json(
                    {"kind": "error", "payload": {"message": f"{type(e).__name__}: {e}"}}
                )
            except Exception:
                pass

    @app.websocket("/api/sessions/{session_id}/terminal")
    async def session_terminal(ws: WebSocket, session_id: int):
        """Phase-2 path: bidirectional PTY relay to a tmux session running
        the user's claw REPL. Browser opens an xterm.js terminal bound to
        this WS. Conversation state lives in claw's memory, multi-turn is
        native, approval prompts are answered by typing in the terminal.

        Protocol (line-delimited JSON over WS):
          C → S  {"type":"resize","cols":N,"rows":N}
          C → S  {"type":"input","data":"<utf-8>"}
          S → C  {"type":"status","state":"connected|creating|error","message":"..."}
          S → C  {"type":"output","data":"<utf-8>"}
        """
        await ws.accept()
        sess = sessions_repo.get_session(db_path, session_id)
        if not sess:
            await ws.send_json({"type": "status", "state": "error",
                                "message": "session not found"})
            await ws.close()
            return
        proj = projects_repo.get_project(db_path, sess["project_id"])
        if not proj:
            await ws.send_json({"type": "status", "state": "error",
                                "message": "project not found"})
            await ws.close()
            return

        # Wait for the initial resize so tmux is created at the right size.
        # If the client doesn't send one in 5s, fall back to defaults.
        cols, rows = 120, 40
        try:
            raw = await asyncio.wait_for(ws.receive_text(), timeout=5.0)
            msg = json.loads(raw)
            if msg.get("type") == "resize":
                cols = int(msg.get("cols") or cols)
                rows = int(msg.get("rows") or rows)
        except (asyncio.TimeoutError, json.JSONDecodeError, WebSocketDisconnect):
            pass

        # Create the tmux session if it doesn't already exist. tmux holds
        # the claw REPL across browser disconnects and server restarts.
        if not tmux_manager.session_exists(session_id):
            await ws.send_json({"type": "status", "state": "creating",
                                "message": "starting claw…"})
            try:
                tmux_manager.create_session(
                    session_id,
                    cwd=Path(proj["path"]),
                    launch_cmd=settings.launch_cmd,
                    cols=cols,
                    rows=rows,
                )
            except Exception as e:  # surface launch failures to the UI
                log.exception("tmux create failed")
                await ws.send_json({"type": "status", "state": "error",
                                    "message": f"tmux create failed: {e}"})
                await ws.close()
                return

        try:
            master_fd, attach_proc = tmux_manager.attach_pty(session_id)
        except Exception as e:
            log.exception("tmux attach failed")
            await ws.send_json({"type": "status", "state": "error",
                                "message": f"tmux attach failed: {e}"})
            await ws.close()
            return

        # Resize once more — covers the reconnect case where the tmux
        # session existed at a different size than the browser wants now.
        tmux_manager.resize_pty(master_fd, session_id, cols, rows)

        await ws.send_json({"type": "status", "state": "connected"})

        loop = asyncio.get_running_loop()
        stop = threading.Event()

        # Background thread: read PTY non-blocking, push to WS via the
        # asyncio loop. select() on a 100ms tick keeps stop_event responsive.
        def pty_reader():
            while not stop.is_set():
                try:
                    ready, _, _ = select.select([master_fd], [], [], 0.1)
                except (OSError, ValueError):
                    break
                if not ready:
                    continue
                try:
                    data = os.read(master_fd, 4096)
                except (BlockingIOError, OSError):
                    continue
                if not data:
                    asyncio.run_coroutine_threadsafe(
                        ws.send_json({"type": "status", "state": "ended",
                                      "message": "claw exited"}),
                        loop,
                    )
                    break
                text = data.decode("utf-8", errors="replace")
                fut = asyncio.run_coroutine_threadsafe(
                    ws.send_json({"type": "output", "data": text}),
                    loop,
                )
                try:
                    fut.result(timeout=2.0)
                except Exception:
                    break

        reader = threading.Thread(target=pty_reader, daemon=True)
        reader.start()

        try:
            while True:
                raw = await ws.receive_text()
                try:
                    msg = json.loads(raw)
                except json.JSONDecodeError:
                    continue
                kind = msg.get("type")
                if kind == "input":
                    data = msg.get("data") or ""
                    if not data:
                        continue
                    try:
                        os.write(master_fd, data.encode("utf-8"))
                    except OSError:
                        break
                elif kind == "resize":
                    new_cols = int(msg.get("cols") or cols)
                    new_rows = int(msg.get("rows") or rows)
                    tmux_manager.resize_pty(master_fd, session_id, new_cols, new_rows)
                # Drop any other message types silently.
        except WebSocketDisconnect:
            pass
        finally:
            stop.set()
            try:
                os.close(master_fd)
            except OSError:
                pass
            try:
                attach_proc.terminate()
            except (ProcessLookupError, OSError):
                pass
            reader.join(timeout=2.0)
            # NOTE: we deliberately do NOT kill the tmux session here.
            # The user can reconnect and pick up the running claw.

    @app.delete("/api/sessions/{session_id}/terminal")
    def kill_terminal(session_id: int):
        """Explicit teardown — destroys the tmux session (kills claw)."""
        sess = sessions_repo.get_session(db_path, session_id)
        if not sess:
            raise HTTPException(status_code=404, detail="session not found")
        tmux_manager.kill_session(session_id)
        return {"killed": tmux_manager.session_name(session_id)}

    # Static frontend — only mount if the directory exists. In tests we skip
    # this so the app is purely API-shaped.
    if settings.static_dir.exists():
        app.mount(
            "/static",
            StaticFiles(directory=str(settings.static_dir)),
            name="static",
        )

        @app.get("/")
        def root():
            index = settings.static_dir / "index.html"
            if not index.exists():
                return JSONResponse(
                    {"error": "static/index.html missing"}, status_code=500
                )
            return FileResponse(index)

        @app.get("/manifest.json")
        def manifest():
            f = settings.static_dir / "manifest.json"
            return FileResponse(f) if f.exists() else JSONResponse({}, status_code=404)

        @app.get("/sw.js")
        def service_worker():
            f = settings.static_dir / "sw.js"
            return FileResponse(f) if f.exists() else JSONResponse({}, status_code=404)

    return app


def _serialize(evt: TurnEvent) -> dict:
    return {"kind": evt.kind, "payload": evt.payload}


# Module-level app for `uvicorn claw_web.api:app`.
app = create_app()
