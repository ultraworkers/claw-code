#!/usr/bin/env bash
# Dev launcher for claw-web. Defaults to mock mode so the UI walks
# without needing claw built. Set CLAW_WEB_MODE=subprocess to actually
# spawn claw.

set -euo pipefail

cd "$(dirname "$0")/.."

VENV="${VENV:-.venv}"
if [ ! -d "$VENV" ]; then
  python3 -m venv "$VENV"
  "$VENV/bin/pip" install -q --upgrade pip
  "$VENV/bin/pip" install -q -e "server[dev]"
fi

export CLAW_WEB_HOST="${CLAW_WEB_HOST:-127.0.0.1}"
export CLAW_WEB_PORT="${CLAW_WEB_PORT:-7683}"
export CLAW_WEB_MODE="${CLAW_WEB_MODE:-mock}"

exec "$VENV/bin/claw-web" --reload
