#!/usr/bin/env bash
# Rebase claw-code feat-tui onto latest upstream/main (ultracode)
# Run from repo root: ./rebase-upstream.sh
#
# Prereq: remotes configured
#   upstream → ultracode/claw (or whatever the upstream org/repo is)
#   origin   → your fork

set -euo pipefail

BRANCH="feat-tui"
REMOTE_UPSTREAM="upstream"
REMOTE_ORIGIN="origin"

cd "$(git rev-parse --show-toplevel)"

echo "🔄 Rebasing $BRANCH onto $REMOTE_UPSTREAM/main"

# Make sure we're on the right branch
current=$(git branch --show-current)
if [ "$current" != "$BRANCH" ]; then
    echo "❌ Not on $BRANCH (on $current). Switch first."
    exit 1
fi

# Stash any uncommitted work
if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "📦 Stashing uncommitted changes..."
    git stash push -m "auto-stash before rebase $(date +%Y%m%d-%H%M%S)"
    STASHED=true
else
    STASHED=false
fi

# Fetch upstream
echo "⬇️  Fetching $REMOTE_UPSTREAM..."
git fetch "$REMOTE_UPSTREAM"

# Check if upstream/main moved
OLD_BASE=$(git merge-base "$BRANCH" "$REMOTE_UPSTREAM/main")
NEW_HEAD=$(git rev-parse "$REMOTE_UPSTREAM/main")

if [ "$OLD_BASE" = "$NEW_HEAD" ]; then
    echo "✅ Already up to date with $REMOTE_UPSTREAM/main ($NEW_HEAD)"
else
    echo "📋 Rebasing... ($OLD_BASE → $NEW_HEAD)"
    echo "   $(git log --oneline $OLD_BASE..$NEW_HEAD | wc -l) new upstream commits"
    if ! git rebase "$REMOTE_UPSTREAM/main"; then
        echo ""
        echo "⚠️  CONFLICTS! Resolve them, then:"
        echo "   git add <resolved files>"
        echo "   git rebase --continue"
        echo ""
        echo "   To abort: git rebase --abort"
        exit 1
    fi
    echo "✅ Rebase complete"
fi

# Force push to origin (safe — only your fork)
echo "⬆️  Force-pushing to $REMOTE_ORIGIN/$BRANCH..."
git push --force-with-lease "$REMOTE_ORIGIN" "$BRANCH"

# Restore stashed work
if [ "$STASHED" = true ]; then
    echo "📦 Restoring stashed changes..."
    git stash pop
fi

echo ""
echo "🎉 Done! $BRANCH is now rebased on $REMOTE_UPSTREAM/main"
echo "   HEAD: $(git log --oneline -1)"
echo "   $(git log --oneline "$REMOTE_UPSTREAM/main".."$BRANCH" | wc -l) commits ahead of upstream"
