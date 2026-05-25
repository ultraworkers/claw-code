#!/usr/bin/env bash
# roadmap-next-id.sh — print the next available ROADMAP item id.
# Usage: scripts/roadmap-next-id.sh [path/to/ROADMAP.md]
#
# Designed to be used before appending a new entry so that concurrent
# dogfood claws do not accidentally reuse the same id:
#
#   NEXT=$(scripts/roadmap-next-id.sh)
#   cat >> ROADMAP.md << EOF
#   ${NEXT}. **...description...**
#   EOF
#
# The script reads the highest numeric id prefix from ROADMAP.md and
# prints highest+1.  It does not lock the file; callers working in
# parallel should git-pull immediately before appending, run
# scripts/roadmap-check-ids.sh before push, and resolve any append
# collision at git-push time.
set -euo pipefail

ROADMAP="${1:-ROADMAP.md}"

if [[ ! -f "$ROADMAP" ]]; then
  echo "error: ROADMAP not found at $ROADMAP" >&2
  exit 1
fi

# Find the highest leading integer from lines that start with a number + '.'.
highest=$(grep -E '^[0-9]+\.' "$ROADMAP" | grep -Eo '^[0-9]+' | sort -n | tail -1)

if [[ -z "$highest" ]]; then
  echo 1
else
  echo $(( highest + 1 ))
fi
