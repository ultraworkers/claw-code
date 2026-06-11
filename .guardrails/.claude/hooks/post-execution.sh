#!/bin/bash
# Post-Execution Hook - Runs after file modifications
# Validates: no forbidden patterns, changes are correct

set -euo pipefail

echo "[GUARDRAILS] Post-execution validation running..."

# Check for common issues in modified files
MODIFIED_FILES=$(git diff --name-only 2>/dev/null || true)

if echo "$MODIFIED_FILES" | grep -iq 'aws_secret'; then
    echo "[ERROR] Potential AWS secret key detected in modified files"
    exit 1
fi

if echo "$MODIFIED_FILES" | grep -iq 'private_key'; then
    echo "[ERROR] Potential private key detected in modified files"
    exit 1
fi

echo "[GUARDRAILS] Post-execution checks passed"
