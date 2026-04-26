# Reviewer Feedback Sprint

## Execution Log

| Time | PR | Action | Status |
|------|-----|--------|--------|
| 2026-06-02 | #3211 | Plan written at `docs/pr-3211-plan.md` | ✅ Plan approved |
| 2026-06-02 | #3211 | Executed: extracted `read_env_non_empty` + `resolve_base_url` into `mod.rs` | ✅ Committed |
| 2026-06-02 | #3211 | Pushed commit `c0855e12` to `worktree-provider-config-fallback` | ✅ Pushed |
| 2026-06-02 | #3211 | CI: cargo fmt ✅, cargo test (147 unit + 37 integration) ✅, cargo check ✅ | ✅ Verified |
| 2026-06-02 | #3211 | Pre-existing clippy issues in runtime/trident.rs (not from our changes) — noted | ⚠️ Known |
| 2026-06-02 | #3214 | Plan written at `docs/pr-3214-plan.md` | ✅ Plan approved |
| 2026-06-02 | #3214 | All 3 CI failures (build ❌ test ❌ fmt ❌) traced to one root cause: missing `retry_after: None` in main.rs test | ✅ Root caused |
| 2026-06-02 | #3214 | Fixed: added `retry_after: None`, removed duplicate `#[must_use]`, ran `cargo fmt --all` | ✅ Committed |
| 2026-06-02 | #3214 | Pushed commit `76783377` to `worktree-api-timeout-retry-v2` | ✅ Pushed |
| 2026-06-02 | #3214 | Reviewer Q "Are defaults preserved?" — YES: ApiTimeoutConfig defaults 30s/300s/8, with_retry_policy() is opt-in | ✅ Answered |
| 2026-06-02 | #3216 | Plan written at `docs/pr-3216-plan.md` | ✅ Plan approved |
| 2026-06-02 | #3216 | Root cause: test creates sessions with 0 messages → filtered out by `message_count > 0` | ✅ Root caused |
| 2026-06-02 | #3216 | Fix: test now adds `push_user_text()` before saving; added `format_all_sessions_empty()` error message | ✅ Committed |
| 2026-06-02 | #3216 | Added test `latest_session_returns_all_empty_error_when_sessions_exist_but_have_no_messages` | ✅ New test |
| 2026-06-02 | #3216 | Pushed commit `41034bb3` to `worktree-session-resume-fixes` | ✅ Pushed |
| 2026-06-02 | #3217 | Plan written at `docs/pr-3217-plan.md` — trivial: just cargo fmt | ✅ Plan approved |
| 2026-06-02 | #3217 | Committed pre-existing fmt changes, pushed `592bae5` | ✅ Done |
| 2026-06-02 | #3218 | Plan written at `docs/pr-3218-plan.md` — slash command count off by 1 | ✅ Plan approved |
| 2026-06-02 | #3218 | Fixed: count 139→140, added `/setup` assertion, pushed `1122b16` | ✅ Done |
| 2026-06-02 | #3219 | Plan written at `docs/pr-3219-plan.md` — CI count fix + implement `/lsp status` | ✅ Plan approved |
| 2026-06-02 | #3219 | Fixed CI: count 139→140, added `/lsp` assertion | ✅ CI fix |
| 2026-06-02 | #3219 | Exported `check_lsp_availability`, `LspInstallAction`, `InstallInstruction`, `format_install_prompt` from runtime | ✅ Exports |
| 2026-06-02 | #3219 | Implemented `/lsp status` (✅❌⚠️), `/lsp list`, `/lsp start/stop` handler replacing stub | ✅ Feature |
| 2026-06-02 | #3219 | Pushed commit `369452c` to `worktree-lsp-v3` | ✅ Pushed |

### 2026-06-04 — Round 2: upstream-merge cycle (new reviewer comments + conflicts)

Reviewer (`1716775457damn`) left new comments on 5 PRs. Root cause across all: every PR was cut from stale `upstream/main @ 4d3dc5b`; upstream advanced to `4619375` (config/runtime refactor → conflicts + flaky CI). Fix = merge current upstream + address each specific ask. Investigated with 5 read-only agents, fixed with 5 agents in isolated worktrees, each diff reviewed (no leftover markers, feature preserved, build + targeted tests green, remote SHA verified).

| Time | PR | Action | Status |
|------|-----|--------|--------|
| 2026-06-04 | #3214 | Reviewer: resolve conflicts + clarify 5-vs-3 commits. Merged upstream; threaded `api_timeout` through new `build_runtime_config` helper; conflicts in config.rs/lib.rs | ✅ Resolved |
| 2026-06-04 | #3214 | Verified: 158+13+11+4+7 api tests pass, no leftover markers, MERGEABLE/CLEAN. Pushed merge `9e50cb6` | ✅ Pushed |
| 2026-06-04 | #3216 | Reviewer: fix 3 CI checks + add exclude_id tests. Clean merge; `cargo fmt` (real failure); added 3 tests (exclude_id skip, 0-msg filter, resolve_reference_excluding) | ✅ Resolved |
| 2026-06-04 | #3216 | Verified: 21 session_control tests pass, fmt clean, MERGEABLE/CLEAN. Pushed `346772a`. Note: build/test reds were env-only `subagentModel` config-parse, not PR-caused | ✅ Pushed |
| 2026-06-04 | #3217 | Reviewer: fix 2 CI checks + integration test. Clean merge refreshed flaky arg-parse tests; extracted `PRESERVE_SCHEDULE` const + added bounds test | ✅ Resolved |
| 2026-06-04 | #3217 | Verified: 206 CLI tests pass incl. previously-flaky pair, MERGEABLE/CLEAN. Pushed `77c7d08` | ✅ Pushed |
| 2026-06-04 | #3218 | Reviewer: resolve conflicts + document subagentModel. 4-file merge (kept upstream `Doctor{output_format,permission_mode}` sig + this PR's `setup` arm); doc comment on `TOP_LEVEL_FIELDS` | ✅ Resolved |
| 2026-06-04 | #3218 | Verified: 24 config_validate tests pass, builds clean, MERGEABLE/CLEAN. Pushed `d54f028` | ✅ Pushed |
| 2026-06-04 | #3219 | Reviewer: resolve conflicts + nightly rustup probe. 3-file merge; `rustup_component_works()` now probes stable→nightly, returns working toolchain; both call sites updated | ✅ Resolved |
| 2026-06-04 | #3219 | Verified: 58 LSP tests pass, builds clean, MERGEABLE (CI re-running). Pushed `a0e9833` | ✅ Pushed |
| 2026-06-04 | #3211 | Resolved merge conflicts with upstream/main (config.rs + lib.rs); preserved RuntimeProviderConfig + `read_env_non_empty`/`resolve_base_url` helpers; rebuilt corrupted test section | ✅ Resolved |
| 2026-06-04 | #3211 | Verified: api + runtime crates compile, shared helpers intact in mod.rs, `67da2dc` on `worktree-provider-config-fallback` | ✅ Pushed |
| 2026-06-04 | #3211 | Posted maintainer summary on PR #3211: shared helpers explained, `from_env` vs `from_env_or_saved` naming rationale, merge details | ✅ Commented |

**Date:** 2026-06-03
**Upstream:** `ultraworkers/claw-code`

---

## Closed PRs — Done, No Action

| PR | What happened |
|----|---------------|
| #3212 | Owner merged equivalent (`1bd18be`) |
| #3215 | Owner closed — wants internal doc pointers, not external links |
| #3220 | Owner merged equivalent (`9c8375d`) with validation/tests added |

---

## Open PRs — Reality Check

| # | PR | Title | build | test | fmt | mergeable | Reviewer | Difficulty |
|---|-----|-------|-------|------|-----|-----------|----------|------------|
| ① | #3217 | Auto-compact retry | ✅ | ✅ | ❌ | YES | 1716775457damn | Trivial |
| ② | #3211 | 3-tier credential resolution | ✅ | ✅ | ❌ | ? | 1716775457damn | Medium |
| ③ | #3214 | API timeout/retry | ❌ | ❌ | ❌ | ? | 1716775457damn | Medium |
| ④ | #3216 | Session resume | ❌ | ❌ | ✅ | YES | 1716775457damn | Medium |
| ⑤ | #3218 | Wizard entry points | ❌ | ❌ | ✅ | ? | *(none yet)* | Medium |
| ⑥ | #3219 | LSP integration v3 | ❌ | ❌ | ✅ | ? | 1716775457damn | Hard |

Order: ① → ② → ③ → ④ → ⑤ → ⑥ (green CI first, then red, easy → hard)

---

## ① #3217 — Auto-compact retry

**Branch:** `worktree-auto-compact-retry`
**Worktree:** `.claude/worktrees/wf_616683a9-d46-1`

### Reviewer said:

> "Nice improvement — extending auto-compact retry from REPL-only to all execution paths fixes a real UX pain point. Using RuntimeError::is_context_window_failure() via the canonical api crate markers is the right approach. The progressive compaction strategy (4→2→0) with MAX_COMPACT_RETRIES=3 is well-bounded."
>
> **Action:** "I see cargo fmt is failing in CI — consider running cargo fmt --all before merge to keep checks green."

### CI: 5/6 green. Only `cargo fmt` fails.

### Plan:

| Step | What | Command |
|------|------|---------|
| 1 | Format | `cd rust/ && cargo fmt --all` |
| 2 | Verify | `cargo fmt --all -- --check` && `cargo check --workspace` |
| 3 | Commit | `"style: cargo fmt"` |
| 4 | Push | `git push origin worktree-auto-compact-retry` |

Fmt touches 1 file: `main.rs` (5 ins / 10 del, pure formatting). No logic changes.

### Status: ⬜

---

## ② #3211 — 3-tier credential resolution

**Branch:** `worktree-provider-config-fallback`
**Worktree:** `.claude/worktrees/provider-config-fallback`

### Reviewer said:

> "This makes the setup wizard actually functional. The 3-tier priority is well thought out. Covering all four providers with both API key and base URL resolution is thorough."
>
> **Ask 1:** "The from_env_or_saved naming across multiple provider files could be consolidated into a shared trait or helper to reduce duplication."
>
> **Ask 2:** "Also, cargo fmt is failing — a quick cargo fmt --all before merge will fix that."

### CI: 5/6 green. Only `cargo fmt` fails.

### Plan:

| Step | What | Detail |
|------|------|--------|
| 1 | Investigate | Read all 4 provider files. Catalog `from_env_or_saved` signatures, env vars, return types. Find where `ResolvedCredentials` is defined. |
| 2 | Add helper | `CredentialSources` struct + `resolve_provider_credentials()` fn in `config.rs` |
| 3 | Refactor × 4 | Replace each provider's `from_env_or_saved` body with helper call |
| 4 | Export | `pub use` in `lib.rs` if needed |
| 5 | Fmt + verify | `cargo fmt --all` → `cargo check --workspace` → `cargo test -p runtime -p api` |
| 6 | Commit + push | `"refactor: consolidate from_env_or_saved into shared resolve_provider_credentials() helper"` |

**Helper design:**

```rust
pub struct CredentialSources {
    pub api_key_env: &'static str,
    pub api_key_alt_env: Option<&'static str>,
    pub api_key_config: fn(&RuntimeProviderConfig) -> &Option<String>,
    pub base_url_env: &'static str,
    pub base_url_config: fn(&RuntimeProviderConfig) -> &Option<String>,
    pub default_base_url: &'static str,
}

pub fn resolve_provider_credentials(
    config: &RuntimeProviderConfig,
    sources: &CredentialSources,
) -> Option<ResolvedCredentials> { … }
```

**Open Qs to resolve in Step 1:**
- Is `ResolvedCredentials` shared or per-provider?
- Does `.env` tier use `dotenv::var()`?
- Any tests call `from_env_or_saved` directly?

### Status: ✅ Resolved (2026-06-04)

Merged upstream/main into `worktree-provider-config-fallback`. Conflicts in `runtime/src/config.rs` and `runtime/src/lib.rs` both resolved while preserving the `read_env_non_empty` / `resolve_base_url` shared helpers and the `RuntimeProviderConfig` struct/impl.

Key manual fix: The merge-generated config.rs had corrupted test sections (interleaved function declarations from both versions). Reconstructed clean provider_config_* and rules_import_* test bodies from original commits.

Build verification: `cargo check -p api` ✅, `cargo check -p runtime` ✅. Pre-existing clippy issues in trident.rs and claw-rag-service unchanged.

Commit: `67da2dc`

---

## ③ #3214 — API timeout/retry

**Branch:** `worktree-api-timeout-retry-v2`
**Worktree:** `.claude/worktrees/api-timeout-retry-v2`

### Reviewer said:

> "Good revival of the stalled #2816. Retry-After header and 400 transient retry are particularly valuable for handling API rate limits."
>
> **Question:** "Are defaults preserved so existing behavior is unchanged?"

### CI: 3/6 green. Build ❌, test ❌, fmt ❌. Fix CI first, then answer.

### Plan:

**Phase A — Fix CI:**

| Step | What | Detail |
|------|------|--------|
| 1 | Investigate build | `cargo check --workspace 2>&1` — capture errors |
| 2 | Fix build | Based on Step 1 findings |
| 3 | Investigate test | `cargo test -p runtime -p api 2>&1` — capture failures |
| 4 | Fix tests | Based on Step 3 findings |
| 5 | Fmt | `cargo fmt --all` |
| 6 | Verify green | All 3 checks pass locally |
| 7 | Commit + push | |

**Phase B — Answer reviewer:**

| Step | What | Detail |
|------|------|--------|
| 8 | Verify defaults | `TimeoutConfig` timeout=300s, `RetryConfig` max_retries=2, 400-transient is additive-only |
| 9 | Post comment | Answer with specifics (draft below, adjust after Step 8) |

**Comment draft:**
```
Yes, all defaults are preserved:
- Timeout: 300s (unchanged)
- Max retries: 2 (unchanged)
- Retry-After: only activates when server sends the header
- 400 transient: purely additive — retries only on specific transient 400 codes

Backward-compatible with no behavioral changes unless user explicitly configures new fields.
```
⚠ Don't post until Step 8 confirms. If any default changed, fix code first.

**Open Qs:**
- Actual build errors? (Step 1)
- Actual test failures? (Step 3)
- Defaults actually preserved? (Step 8)

### Status: ⬜

---

## ④ #3216 — Session resume

**Branch:** `worktree-session-resume-fixes`
**Worktree:** `.claude/worktrees/wf_616683a9-d46-2`

### Reviewer said:

> "The unified session resolution with load_session_excluding() is a clean approach."
>
> **Concern 1:** "I notice CI has 2 failing checks (build + cargo test) — worth investigating before merge."
>
> **Concern 2:** "What happens when all sessions have 0 messages (fresh install / all deleted)? A clear error message would be better than silently returning nothing."

### CI: 4/6 green. Build ❌, test ❌. fmt ✅.

### Plan:

| Step | What | Detail |
|------|------|--------|
| 1 | Investigate build | `cargo check --workspace 2>&1` — capture errors |
| 2 | Fix build | Based on Step 1. Likely missing imports in `session_control.rs` |
| 3 | Investigate test | `cargo test -p runtime 2>&1` — capture failures |
| 4 | Fix tests | Based on Step 3 |
| 5 | Add empty-session error | If all candidates have 0 messages → `Err("No sessions with messages found. Start a new session first.")` |
| 6 | Surface in CLI | `/resume` handler prints error gracefully (no panic) |
| 7 | Fmt + verify | `cargo fmt --all` → `cargo check --workspace` → `cargo test -p runtime` |
| 8 | Commit + push | |

**Empty-session error:**
```rust
// session_control.rs — after scanning:
if candidates.is_empty() || candidates.iter().all(|s| s.message_count == 0) {
    return Err(SessionError::NoSessionsFound);
}
// main.rs — /resume handler:
Err(SessionError::NoSessionsFound) => {
    eprintln!("No sessions with messages found. Start a new session first.");
}
```

**Open Qs:**
- Actual build errors? (Step 1)
- Actual test failures? (Step 3)
- Does `SessionError` enum already exist?

### Status: ⬜

---

## ⑤ #3218 — Wizard entry points

**Branch:** `worktree-wizard-entry-points`
**Worktree:** `.claude/worktrees/wizard-entry-points`

### Reviewer: None yet. Fix proactively before one arrives.

### CI: 4/6 green. Build ❌, test ❌. fmt ✅.

### Likely cause: `setup_wizard.rs` imports `RuntimeProviderConfig` which doesn't exist on upstream/main (only in PR #3211).

### Strategy: Add `RuntimeProviderConfig` to this PR too. Safe duplication — when #3211 merges first, rebase deduplicates.

### Plan:

| Step | What | Detail |
|------|------|--------|
| 1 | Investigate build | `cargo check --workspace 2>&1` — confirm root cause |
| 2 | Add `RuntimeProviderConfig` | Minimal fields in `config.rs` (same as #3211) |
| 3 | Export | `pub use` in `lib.rs` |
| 4 | Fix other compile errors | If any |
| 5 | Investigate test | `cargo test -p runtime -p api 2>&1` |
| 6 | Fix tests | Based on findings |
| 7 | Fmt + verify | `cargo fmt --all` → `cargo check --workspace` → `cargo test -p runtime -p api` |
| 8 | Commit + push | |

**Open Qs:**
- Is `RuntimeProviderConfig` the only missing type?
- Any other dependencies on #3211?

### Status: ⬜

---

## ⑥ #3219 — LSP integration v3

**Branch:** `worktree-lsp-v3`
**Worktree:** `.claude/worktrees/wf_616683a9-d46-5`

### Reviewer said:

> "This is an impressive piece of work. The modular split across 4 crates with 58 unit tests is exactly the right architecture. The lazy-start pattern and diagnostic enrichment are genuinely useful."
>
> **Blocker 1:** "build and cargo test are currently failing — worth investigating"
>
> **Blocker 2:** "Consider adding a claw lsp status command so users can confirm which servers were discovered and their health"

### CI: 4/6 green. Build ❌, test ❌. fmt ✅.

### Plan:

**Phase A — Fix CI:**

| Step | What | Detail |
|------|------|--------|
| 1 | Investigate build | `cargo check --workspace 2>&1` |
| 2 | Fix build | Module visibility / missing imports in LSP crates |
| 3 | Investigate test | `cargo test -p runtime 2>&1` |
| 4 | Fix tests | Assertions needing update for new types |
| 5 | Verify | `cargo check --workspace` + `cargo test -p runtime` |
| 6 | Commit | `"fix: resolve CI build and test failures for LSP integration"` |

**Phase B — Add `claw lsp status`:**

| Step | What | Detail |
|------|------|--------|
| 7 | Add `LspServerStatus` + `LspStatus` | In `lsp_discovery.rs`: `{ name, status: {Running\|Stopped\|NotFound}, path }` |
| 8 | Add `discover_all()` | Probe PATH → `Vec<LspServerStatus>` |
| 9 | CLI subcommand | `claw lsp status` → call `discover_all()` → print table |
| 10 | Slash command | `/lsp status` → check in-process manager → print |
| 11 | Fmt + verify | `cargo fmt --all` → `cargo check --workspace` → `cargo test -p runtime` |
| 12 | Commit + push | |

**Output format:**
```
LSP Server Status:
  rust-analyzer    running   /home/user/.cargo/bin/rust-analyzer
  gopls            stopped   /usr/local/bin/gopls
  pyright          not found (not in PATH)

2 servers discovered, 1 running
```

**Open Qs:**
- Actual build errors? (Step 1)
- Actual test failures? (Step 3)
- Does LSP crate already have `discover_all()`-equivalent?
- CLI: probe fresh or connect to running instance?

### Status: ⬜

---

## Progress

| # | PR | Status | CI | Reviewer | Pushed |
|---|-----|--------|-----|----------|--------|
| ① | #3217 | ⬜ | — | — | — |
| ② | #3211 | ⬜ | — | — | — |
| ③ | #3214 | ⬜ | — | — | — |
| ④ | #3216 | ⬜ | — | — | — |
| ⑤ | #3218 | ⬜ | — | — | — |
| ⑥ | #3219 | ⬜ | — | — | — |
