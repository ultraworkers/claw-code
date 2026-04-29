"""claw-web — local web UI wrapper for the claw agent CLI.

Phase 1 scope: project picker + per-project conversation + persistence.
PWA / remote-access framing is stubbed for phase 2.

This package is intentionally small. The hard problem (structured agent I/O
without a PTY) is concentrated in `claw_runner.py`; everything else is
plumbing around it.
"""

__version__ = "0.1.0"
