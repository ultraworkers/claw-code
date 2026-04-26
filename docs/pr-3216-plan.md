# PR #3216 — Execution Plan: Session Resume

**Branch**: `worktree-session-resume-fixes`
**Worktree**: `.claude/worktrees/wf_616683a9-d46-2`
**CI State**: Build ❌ Test ❌ Fmt ✅ Clippy ✅ Docs ✅ Windows ✅

## Reviewer Feedback

From `1716775457damn`:
> "The unified session resolution with load_session_excluding() is a clean approach. I notice CI has 2 failing checks (build + cargo test) — worth investigating before merge. One edge case question: what happens when all sessions have 0 messages (fresh install / all deleted)? A clear error message would be better than silently returning nothing."

## Investigation Findings

### Root cause of CI failures: test creates empty sessions

The test `latest_session_alias_resolves_most_recent_managed_session` creates sessions with `Session::new().save_to_path()`, which produces sessions with **0 messages**. The new `latest_session_excluding()` method filters out sessions with `message_count > 0` — so the test sessions are excluded and `resolve_session_reference("latest")` returns `Err`.

Error from CI:
```
latest session should resolve: Format("no managed sessions found in .claw/sessions/a0f293440386ce2b/\nStart `claw` to create a session, then rerun with `--resume latest`.\nNote: /resume latest searches all workspaces.")
```

The build ❌ is also from this — CI's build step includes test compilation, and the test runtime failure causes the step to fail.

### Reviewer's "empty sessions" edge case

Currently, `latest_session_excluding()` returns the same "no managed sessions found" error whether there are zero sessions at all OR all sessions have 0 messages. These are different situations:
1. **No sessions exist** → fresh install, user has never started a session
2. **Sessions exist but all empty** → user just started claw in another terminal, hasn't typed anything yet

The reviewer wants these cases distinguished with clear error messages.

## Execution Plan

### Step 1: Fix the failing test

The test `latest_session_alias_resolves_most_recent_managed_session` needs to create sessions with at least one message so they pass the `message_count > 0` filter.

**File**: `rust/crates/rusty-claude-cli/src/main.rs`

Instead of:
```rust
Session::new()
    .with_persistence_path(older.path.clone())
    .save_to_path(&older.path)
    .expect("older session should save");
```

We need to add a message to the session before saving:
```rust
let mut older_session = Session::new()
    .with_persistence_path(older.path.clone());
older_session.add_user_message("test message for older session");
older_session.save_to_path(&older.path).expect("older session should save");
```

**Need to check**: What's the actual API for adding messages to a `Session`? The `Session` struct might use `ConversationMessage` or similar.

### Step 2: Add "all sessions empty" error distinction

When `latest_session_excluding()` finds sessions but they all have 0 messages, return a distinct error message:

- **No sessions at all**: "No managed sessions found in `{path}`. Start `claw` to create a session, then rerun with `--resume latest`."
- **Sessions exist but all empty**: "All sessions are empty (0 messages). This usually means a fresh `claw` session has been started but no messages have been sent yet. Wait for a response in your other session, then try `--resume latest` again."

Implementation: In `latest_session_excluding()`, before returning the error, check if `list_sessions()` returns any sessions at all (regardless of message count). If so, use the "all empty" message.

**File**: `rust/crates/runtime/src/session_control.rs`

### Step 3: Update existing tests in session_control.rs

The runtime crate may also have tests that create empty sessions and test `latest_session()`. Need to verify and fix.

**File**: `rust/crates/runtime/src/session_control.rs`

### Step 4: Verify — cargo test -p runtime, cargo test -p rusty-claude-cli

## Commit Details

**Commit**: `41034bb3` on `worktree-session-resume-fixes`

```
fix: address CI test failure and add empty-session error message

- Fix latest_session_alias_resolves_most_recent_managed_session test:
  the test created sessions with 0 messages, which are now filtered out
  by the message_count > 0 check in latest_session_excluding(). Updated
  the test to call push_user_text() before saving so sessions have
  at least one message and are findable by /resume latest.

- Add distinct error message when all sessions are empty (0 messages).
  Previously, the same "no managed sessions found" message was returned
  whether there were zero sessions or all sessions had 0 messages. Now:
  - No sessions at all → "no managed sessions found in {path}. Start
    claw to create a session..."
  - Sessions exist but all empty → "all sessions are empty (0 messages)
    in {path}. This usually means a fresh claw session is running but
    no messages have been sent yet. Wait for a response in your other
    session, then try --resume latest again."

- Add test for the all-sessions-empty error path:
  latest_session_returns_all_empty_error_when_sessions_exist_but_have_no_messages

  Addresses reviewer feedback on #3216.
```

**Files changed**:
- `rust/crates/runtime/src/session_control.rs` — added `format_all_sessions_empty()`, added "has any session" check in `latest_session_excluding()`, added new test
- `rust/crates/rusty-claude-cli/src/main.rs` — fixed test to use `push_user_text()` before saving

## Acceptance Criteria

- [x] Test `latest_session_alias_resolves_most_recent_managed_session` fixed (sessions now have messages)
- [x] `format_all_sessions_empty()` added — distinct error when sessions exist but all have 0 messages
- [x] `format_no_managed_sessions()` preserved — error when no sessions exist at all
- [x] New test `latest_session_returns_all_empty_error_when_sessions_exist_but_have_no_messages` passes
- [x] All 17 session_control tests pass
- [x] Changes committed and pushed (commit `41034bb3`)
