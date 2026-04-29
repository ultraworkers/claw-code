"""tmux + PTY plumbing for the long-lived REPL architecture.

Each claw-web session owns a tmux session named `claw-web-<id>` that runs
the user's `cl` wrapper (or the configured launch command). tmux owns the
process lifecycle, so:

  - Browser disconnect doesn't kill claw — tmux holds it.
  - Server restart doesn't kill claw — tmux is rooted in the user's
    login session, not the web app process.
  - Multiple browser tabs on the same session can attach concurrently
    (tmux supports multi-attach natively).
  - Multi-turn works because claw is alive across turns; conversation
    state lives in claw's own memory.

Borrowed in shape from `~/src/ai-local-hardware-deploy/sentinel/sentinel/
terminal/manager.py`, stripped to single-user — no bwrap, no postgres,
no ai-account/ccproxy resolution, no integration credential injection.
"""

from __future__ import annotations

import fcntl
import os
import pty
import shlex
import shutil
import struct
import subprocess
import termios
import time
from pathlib import Path


def session_name(session_id: int) -> str:
    """Stable tmux session name per claw-web session id."""
    return f"claw-web-{session_id}"


def _tmux_path() -> str:
    p = shutil.which("tmux")
    if not p:
        raise RuntimeError("tmux is not installed on PATH")
    return p


def session_exists(session_id: int) -> bool:
    try:
        tmux = _tmux_path()
    except RuntimeError:
        return False
    result = subprocess.run(
        [tmux, "has-session", "-t", session_name(session_id)],
        capture_output=True,
    )
    return result.returncode == 0


def create_session(
    session_id: int,
    cwd: Path,
    launch_cmd: list[str],
    *,
    cols: int = 120,
    rows: int = 40,
    extra_env: dict[str, str] | None = None,
) -> None:
    """Create a new tmux session running `launch_cmd` in `cwd`.

    `launch_cmd` is the program (and its args) to run inside tmux —
    typically `["claw"]` or the user's `cl` wrapper. The shell wrapper
    around it sets TERM and exports any `extra_env` keys, then exec's
    the launch command with a thin "press a key to relaunch" loop so a
    transient API error doesn't tear the tmux session down.
    """
    tmux = _tmux_path()
    name = session_name(session_id)

    # Build the bash command run inside tmux. We use a heredoc-style
    # explicit env export rather than relying on tmux to inherit; tmux
    # sessions can outlive the spawning shell, so making env explicit
    # makes the behavior reproducible.
    parts: list[str] = ["#!/usr/bin/env bash", "set -u"]
    parts.append("export TERM=xterm-256color")
    if extra_env:
        for k, v in extra_env.items():
            parts.append(f"export {shlex.quote(k)}={shlex.quote(v)}")
    parts.append(f"cd {shlex.quote(str(cwd))}")
    # Inline relaunch loop. Press Enter to restart, q to quit.
    quoted_cmd = " ".join(shlex.quote(a) for a in launch_cmd)
    parts.append(_relaunch_loop_shell(quoted_cmd))
    shell_cmd = "\n".join(parts)

    subprocess.run(
        [
            tmux,
            "new-session",
            "-d",
            "-s",
            name,
            "-x",
            str(cols),
            "-y",
            str(rows),
            "bash",
            "-c",
            shell_cmd,
        ],
        check=True,
    )

    # Wait briefly for tmux to acknowledge the session.
    deadline = time.time() + 5.0
    while time.time() < deadline:
        if session_exists(session_id):
            break
        time.sleep(0.05)
    else:
        raise RuntimeError(f"tmux session {name} did not start in time")

    # Enable mouse-on so the browser's wheel events translate to native
    # tmux scrollback / copy-mode entry. Without this, the wheel inside
    # an alt-screen TUI (claw) is dead-letter. Server-wide is fine for
    # our single-user case; per-session would also work via `-t`.
    subprocess.run(
        [tmux, "set-option", "-t", name, "mouse", "on"],
        capture_output=True,
    )


def _relaunch_loop_shell(cmd: str) -> str:
    """Bash fragment that runs `cmd`, then offers Enter to relaunch / q to
    quit on exit. Keeps the tmux pane alive across transient API errors,
    /clear-style restarts, etc. — adapted from Sentinel's exit-loop but
    simplified (no rate-limit / module-error classification, no auto-
    retry budget, no session log tee — those belong elsewhere)."""
    return (
        "while true; do\n"
        f"  {cmd}\n"
        "  rc=$?\n"
        "  echo\n"
        "  echo \"[claw exited code $rc — Enter to relaunch, q to quit]\"\n"
        "  IFS= read -r key || key=q\n"
        "  case \"$key\" in\n"
        "    q|Q) exit 0 ;;\n"
        "    *) ;;\n"
        "  esac\n"
        "done\n"
    )


def attach_pty(session_id: int) -> tuple[int, subprocess.Popen]:
    """Open a PTY and `tmux attach-session` inside it.

    Returns (master_fd, popen). master_fd is the parent's read/write end;
    set non-blocking by the caller (or just use os.read which raises on
    no data when the fd is non-blocking).

    The caller is responsible for: closing master_fd, terminating the
    Popen, and joining whatever thread reads from master_fd.
    """
    tmux = _tmux_path()
    name = session_name(session_id)

    master_fd, slave_fd = pty.openpty()

    env = os.environ.copy()
    env["TERM"] = "xterm-256color"

    proc = subprocess.Popen(
        [tmux, "attach-session", "-t", name],
        stdin=slave_fd,
        stdout=slave_fd,
        stderr=slave_fd,
        env=env,
        close_fds=True,
        # New process group so a Ctrl+C in the parent doesn't propagate.
        preexec_fn=os.setsid,
    )
    os.close(slave_fd)

    # Caller will read non-blocking via select; flip the flag.
    flags = fcntl.fcntl(master_fd, fcntl.F_GETFL)
    fcntl.fcntl(master_fd, fcntl.F_SETFL, flags | os.O_NONBLOCK)

    return master_fd, proc


def resize_pty(master_fd: int, session_id: int, cols: int, rows: int) -> None:
    """Resize both the local PTY and the tmux window."""
    winsize = struct.pack("HHHH", rows, cols, 0, 0)
    try:
        fcntl.ioctl(master_fd, termios.TIOCSWINSZ, winsize)
    except OSError:
        pass
    try:
        tmux = _tmux_path()
        subprocess.run(
            [tmux, "resize-window", "-t", session_name(session_id),
             "-x", str(cols), "-y", str(rows)],
            capture_output=True,
        )
    except (RuntimeError, OSError):
        pass


def kill_session(session_id: int) -> None:
    try:
        tmux = _tmux_path()
    except RuntimeError:
        return
    subprocess.run(
        [tmux, "kill-session", "-t", session_name(session_id)],
        capture_output=True,
    )


def list_sessions() -> list[str]:
    """Return tmux session names matching `claw-web-*`."""
    try:
        tmux = _tmux_path()
    except RuntimeError:
        return []
    result = subprocess.run(
        [tmux, "list-sessions", "-F", "#{session_name}"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return []
    return [
        name.strip()
        for name in result.stdout.splitlines()
        if name.strip().startswith("claw-web-")
    ]
