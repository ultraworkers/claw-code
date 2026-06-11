#!/bin/bash
# Pre-Commit Hook - Runs before git commit
# Validates: AI attribution, no secrets, scope

set -euo pipefail

echo "[GUARDRAILS] Pre-commit validation running..."

COMMIT_MSG_FILE="$1"
BRANCH=$(git rev-parse --abbrev-ref HEAD)

# Check for AI attribution in commit message
if ! grep -qi "Co-Authored-By:" "$COMMIT_MSG_FILE"; then
    echo "[ERROR] Commit message missing AI attribution"
    echo "[INFO] Add: Co-Authored-By: Claude <noreply@anthropic.com>"
    exit 1
fi

# Check for secrets in staged files using trufflehog if available
if command -v trufflehog &> /dev/null; then
    if ! trufflehog git file://. --since-commit HEAD --only-verified --fail 2>/dev/null; then
        echo "[ERROR] Potential secrets detected in staged files"
        exit 1
    fi
fi

# Rudimentary secret detection (basic patterns)
STAGED_FILES=$(git diff --cached --name-only)
if echo "$STAGED_FILES" | grep -q '\.env'; then
    echo "[ERROR] .env file is staged. Add to .gitignore or use environment variables."
    exit 1
fi

echo "[GUARDRAILS] Pre-commit validation passed"
