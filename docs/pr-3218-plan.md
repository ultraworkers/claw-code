# PR #3218 — Execution Plan: Wizard Entry Points

**Branch**: `worktree-wizard-entry-points`
**Worktree**: `.claude/worktrees/wizard-entry-points`
**CI State**: **Build ❌** Test ❌ Fmt ✅ Clippy ✅ Docs ✅ Windows ✅

## Reviewer Feedback

No reviewer comments yet (pre-emptive fix).

## Investigation Findings

### Root cause of CI failures: slash command count assertion off by 1

The test `renders_help_from_shared_specs` in `commands/src/lib.rs:5243` asserts:
```rust
assert_eq!(slash_command_specs().len(), 139);
```

This PR added the `/setup` command (registered at line 723), bringing the total to **140**. The test wasn't updated to match.

Error from CI:
```
assertion `left == right` failed
left: 140
right: 139
```

Both Build ❌ and Test ❌ are from this single test failure.

### No other issues found

The PR description mentions `/setup` command, `claw setup` subcommand, and `RuntimeProviderConfig`. The CI clippy, fmt, docs, and windows checks all pass — only this one test count is wrong.

## Execution Plan

### Step 1: Update the slash command count in the test

Change `139` → `140` at `commands/src/lib.rs:5243`.

### Step 2: Add `/setup` content assertion to the test

The test checks that many slash commands appear in the help output but doesn't verify `/setup`. Add:
```rust
assert!(help.contains("/setup"));
```

### Step 3: Verify — cargo test -p commands

### Step 4: Commit + push

## Commit Details

**Commit**: `1122b16` on `worktree-wizard-entry-points`

```
fix: update slash command count and add /setup assertion in test

- Update slash_command_specs().len() assertion from 139 to 140.
  The /setup command added by this PR increased the spec count by 1
  but the test's expected count was not updated, causing CI failure.

- Add assert!(help.contains("/setup")) to the
  renders_help_from_shared_specs test so the new command is
  verified in the help output.

  Fixes CI Build ❌ and Test ❌ on #3218.
```

**Files changed**:
- `rust/crates/commands/src/lib.rs` — line 5242: added `assert!(help.contains("/setup"));`, line 5243: updated count `139` → `140`

## Acceptance Criteria

- [x] `slash_command_specs().len()` assertion updated to 140
- [x] `/setup` content assertion added
- [x] `cargo test -p commands` passes (42 tests)
- [x] Changes committed and pushed (commit `1122b16`)
