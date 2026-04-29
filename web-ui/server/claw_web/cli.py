"""`claw-web` CLI: a thin uvicorn launcher.

Avoids the user having to remember the `--app-dir` / `claw_web.api:app`
incantation. `--reload` for dev.
"""

from __future__ import annotations

import argparse

import uvicorn

from .config import Settings


def main() -> None:
    settings = Settings.from_env()
    parser = argparse.ArgumentParser(prog="claw-web")
    parser.add_argument("--host", default=settings.host)
    parser.add_argument("--port", type=int, default=settings.port)
    parser.add_argument("--reload", action="store_true")
    args = parser.parse_args()
    uvicorn.run(
        "claw_web.api:app",
        host=args.host,
        port=args.port,
        reload=args.reload,
    )


if __name__ == "__main__":
    main()
