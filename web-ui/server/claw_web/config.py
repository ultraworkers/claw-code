"""Settings for the claw-web server.

Single-user, single-machine. Reads from environment variables with sensible
defaults. No config file yet — keep the surface small until the design
settles.
"""

from __future__ import annotations

import os
import shlex
import shutil
from dataclasses import dataclass
from pathlib import Path


def _env_path(name: str, default: Path) -> Path:
    raw = os.environ.get(name)
    return Path(raw).expanduser() if raw else default


@dataclass(frozen=True)
class Settings:
    # Where the SQLite project registry + session metadata live.
    data_dir: Path

    # Path to the claw binary. Phase 1 spawns it as a subprocess per turn.
    claw_bin: Path

    # Backend mode: "mock" replays canned responses; "subprocess" actually
    # invokes claw. Default mock so the UI is walkable on a fresh checkout.
    mode: str

    # Optional model override passed to claw as `--model`. None means let
    # claw resolve from its own settings.json. Set this for OpenAI-compat
    # routes (LMStudio, Ollama) where the prefix decides routing.
    model: str | None

    # Command (program + args) to launch inside each tmux session for the
    # phase-2 terminal panel. Defaults to the user's `cl` wrapper at
    # `~/.local/bin/cl` if present, else plain `claw`. Override via
    # `CLAW_WEB_LAUNCH_CMD="claw --model ..."` for testing.
    launch_cmd: list[str]

    # HTTP bind. Localhost-only by default; PWA / remote access is phase 2.
    host: str
    port: int

    # Where the static frontend assets live, relative to the repo.
    static_dir: Path

    @classmethod
    def from_env(cls) -> "Settings":
        # config.py lives at <repo>/web-ui/server/claw_web/config.py;
        # walk up four to reach the repo root, three to reach web-ui/.
        web_ui_root = Path(__file__).resolve().parents[2]
        repo_root = web_ui_root.parent
        default_data = Path.home() / ".claw" / "web"
        default_static = web_ui_root / "static"
        # Default launch_cmd: prefer the user's `cl` wrapper (so model,
        # OPENAI_BASE_URL, and friends match how they'd invoke claw from
        # a normal shell), fall back to `claw` from PATH.
        env_launch = os.environ.get("CLAW_WEB_LAUNCH_CMD")
        if env_launch:
            launch_cmd = shlex.split(env_launch)
        else:
            cl_path = Path.home() / ".local" / "bin" / "cl"
            if cl_path.exists():
                launch_cmd = [str(cl_path)]
            elif shutil.which("claw"):
                launch_cmd = ["claw"]
            else:
                launch_cmd = ["claw"]  # will fail loudly when invoked
        return cls(
            data_dir=_env_path("CLAW_WEB_DATA_DIR", default_data),
            claw_bin=_env_path(
                "CLAW_WEB_CLAW_BIN",
                repo_root / "rust" / "target" / "release" / "claw",
            ),
            mode=os.environ.get("CLAW_WEB_MODE", "mock"),
            model=os.environ.get("CLAW_WEB_MODEL") or None,
            host=os.environ.get("CLAW_WEB_HOST", "127.0.0.1"),
            port=int(os.environ.get("CLAW_WEB_PORT", "7683")),
            static_dir=_env_path("CLAW_WEB_STATIC_DIR", default_static),
            launch_cmd=launch_cmd,
        )
