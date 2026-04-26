# PR #3217 — Execution Plan: Auto-Compact Retry

**Branch**: `worktree-auto-compact-retry`
**Worktree**: `.claude/worktrees/wf_616683a9-d46-1`
**CI State**: Build ✅ Test ✅ Clippy ✅ Docs ✅ Windows ✅ **Fmt ❌**

## Reviewer Feedback

From `1716775457damn`:
> "Nice improvement — extending auto-compact retry from REPL-only to all execution paths. Using RuntimeError::is_context_window_failure() via the canonical api crate markers is the right approach. The progressive compaction strategy (4→2→0) with MAX_COMPACT_RETRIES=3 is well-bounded. One note: I see cargo fmt is failing in CI — consider running cargo fmt --all before merge to keep checks green."

## Investigation Findings

The ONLY issue is `cargo fmt`. Uncommitted fmt changes already exist in the worktree from a prior run (5 insertions, 10 deletions in `main.rs`). These are legitimate formatting fixes:
- Collapsed multi-line `format_compact_report()` call to single line
- Reformatted `match self.prepare_turn_runtime(true)` block

No other CI issues. The reviewer had no code concerns, just the fmt fix.

## Execution Plan

### Step 1: Commit the fmt changes (already applied)

The uncommitted changes are from `cargo fmt --all` — just formatting, no behavior change.

### Step 2: Verify — cargo test -p rusty-claude-cli

### Step 3: Push

## Commit Details

**Commit**: `592bae5` on `worktree-auto-compact-retry`

```
style: run cargo fmt --all

Fixes cargo fmt CI check failure noted in reviewer feedback on #3217.
```

**Files changed**:
- `rust/crates/rusty-claude-cli/src/main.rs` — fmt: collapsed multi-line `format_compact_report()` call to single line, reformatted `match self.prepare_turn_runtime(true)` block (5 ins / 10 dels)

## Acceptance Criteria

- [x] `cargo fmt --all` changes committed and pushed (commit `592bae5`)
- [x] CI Fmt ❌ → ✅ expected
