#!/bin/bash
# Pre-Execution Hook - Runs before file modifications
# Enforces: read-before-edit, scope verification

set -euo pipefail

echo "[GUARDRAILS] Pre-execution checks running..."

# Check if CLAUDE.md exists in project root
if [ ! -f "CLAUDE.md" ]; then
    echo "[WARNING] CLAUDE.md not found in repository root"
fi

# Check if AGENT_GUARDRAILS.md exists
if [ ! -f "docs/AGENT_GUARDRAILS.md" ]; then
    echo "[WARNING] AGENT_GUARDRAILS.md not found - guardrails may not be configured"
fi

# Log operation start
echo "[GUARDRAILS] Operation started at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "[GUARDRAILS] Remember: Read before editing, stay in scope, halt when uncertain"
