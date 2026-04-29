# claw-web

A small local web UI that wraps the [`claw`](../rust/) agent CLI.

**Status:** phase 1 + permission mediation landed. Mock + subprocess
modes both wired end-to-end. The streaming runner detects claw's
`Approve this tool call? [y/N]:` prompts mid-turn and surfaces them as a
modal in the UI; the user's Allow / Deny answer is piped back to claw's
stdin. Multiple prompts per turn round-trip cleanly. See [PLAN.md](./PLAN.md).

## What this is

A single-user, local web app that owns claw subprocesses and exposes them
through a structured UI:

- **Project picker as a first-class action.** Pick / create / switch
  projects without burning tokens on `cd ../whatever` prompts.
- **Per-project conversations.** Each session is a claw `--resume`-able
  thread, persisted across browser disconnects.
- **Light-client friendly.** Vanilla HTML + JS, no build step. PWA
  manifest + service worker are stubbed for phase 2 (iPad install, remote
  access via Cloudflare tunnel).
- **NOT a terminal emulator.** ttyd was the v0; this replaces it. We pipe
  claw via structured JSON I/O, not a PTY, so the UI can render diffs,
  tool calls, and file trees as real widgets — not as ANSI escape codes.

## What this is NOT

- Not a Sentinel fork. Sentinel (`~/src/ai-local-hardware-deploy`) is
  multi-user, postgres-backed, tmux+bwrap+xterm.js. Borrows from it
  (per-session `CLAW_CONFIG_HOME` injection, exit-loop concept, naming
  conventions) but doesn't share code. Single-user simplicity is the
  whole point.
- Not a hosted service. Runs on your box. Phase 2 adds remote access via
  Cloudflare Tunnel + Access; the backend never moves off the local
  machine.
- Not a packager. The cross-machine install story lives elsewhere; see
  the brain memory `queued-package-multi-machine-install`.

## Layout

```
web-ui/
├── README.md           # this file
├── PLAN.md             # phase plan, design rationale, open questions
├── server/             # Python FastAPI backend
│   ├── pyproject.toml
│   ├── claw_web/
│   │   ├── api.py            # FastAPI app, REST + WS routes
│   │   ├── config.py         # env-var settings
│   │   ├── db.py             # SQLite schema + connection helper
│   │   ├── projects.py       # project registry CRUD
│   │   ├── sessions.py       # session metadata CRUD
│   │   ├── claw_runner.py    # subprocess-mode runner
│   │   ├── mock_runner.py    # canned-response runner (default)
│   │   └── cli.py            # `claw-web` uvicorn launcher
│   └── tests/
│       └── test_smoke.py
├── static/             # vanilla HTML/JS frontend (no build step)
│   ├── index.html
│   ├── styles.css
│   ├── app.js
│   ├── manifest.json
│   ├── sw.js
│   └── icons/
└── scripts/
    └── dev.sh
```

## Run it

```sh
cd web-ui
./scripts/dev.sh
# → claw-web on http://127.0.0.1:7683 in mock mode
```

That bootstraps a venv at `web-ui/.venv`, installs the package editable,
and starts uvicorn with `--reload`. Open the URL, click `+ New project`,
point it at any directory, click `+ New session`, type a prompt — you'll
see the mock runner round-trip it.

To actually invoke claw:

```sh
export CLAW_WEB_MODE=subprocess
export CLAW_WEB_CLAW_BIN=/mnt/d/src/claw-code/rust/target/release/claw
./scripts/dev.sh
```

## Approval flow demo (mock mode)

To exercise the approval modal without claw running, send a prompt that
starts with `@@approve`:

```
@@approve list the files in this dir
```

The mock runner emits a fake `approval_request` for a `bash` tool call;
the dialog pops up; clicking Allow/Deny resolves the round-trip and the
final response notes the outcome.

## Configuration

All env-driven — no config file yet. Defaults are picked for this
machine; override per host.

| Env var                | Default                                              |
|------------------------|------------------------------------------------------|
| `CLAW_WEB_MODE`        | `mock`                                               |
| `CLAW_WEB_CLAW_BIN`    | `<repo>/rust/target/release/claw`                    |
| `CLAW_WEB_DATA_DIR`    | `~/.claw/web`                                        |
| `CLAW_WEB_HOST`        | `127.0.0.1`                                          |
| `CLAW_WEB_PORT`        | `7683`                                               |
| `CLAW_WEB_STATIC_DIR`  | `<repo>/web-ui/static`                               |

Data directory contents:

- `registry.sqlite` — project + session metadata.
- (claw owns the conversation JSONLs in each project's `.claw/sessions/`.)

## Tests

```sh
cd web-ui/server
pip install -e .[dev]
pytest
```

The smoke test exercises project + session CRUD and a mock-mode WS turn.
It does NOT need the claw binary built.
