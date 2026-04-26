# Upstream PR Sprint Plan

**Created:** 2026-06-02
**Updated:** 2026-06-02
**Branch:** `feat-tui` (working branch)
**Upstream:** `ultraworkers/claw-code`
**Author:** TheArchitectit

---

## Sprint Progress

### Completed (9/9) — ALL ITEMS DONE ✅

| # | Item | PR | Branch |
|---|------|-----|--------|
| 1 | Provider config 3-tier fallback | [ultraworkers/claw-code#3211](https://github.com/ultraworkers/claw-code/pull/3211) | `worktree-provider-config-fallback` |
| 8 | GitShow --format | [ultraworkers/claw-code#3212](https://github.com/ultraworkers/claw-code/pull/3212) | `worktree-gitshow-format` |
| 4 | API timeout/retry v2 | [ultraworkers/claw-code#3214](https://github.com/ultraworkers/claw-code/pull/3214) | `worktree-api-timeout-retry-v2` |
| 3 | Auto-compact & retry on context window errors | [ultraworkers/claw-code#3217](https://github.com/ultraworkers/claw-code/pull/3217) | `worktree-auto-compact-retry` |
| 6 | Session resume fixes | [ultraworkers/claw-code#3216](https://github.com/ultraworkers/claw-code/pull/3216) | `worktree-session-resume-fixes` |
| 9 | README referral links | [ultraworkers/claw-code#3215](https://github.com/ultraworkers/claw-code/pull/3215) | `worktree-readme-referral` |
| 2 | Wizard entry points | [ultraworkers/claw-code#3218](https://github.com/ultraworkers/claw-code/pull/3218) | `worktree-wizard-entry-points` |
| 5 | LSP clean rebuild | [ultraworkers/claw-code#3219](https://github.com/ultraworkers/claw-code/pull/3219) | `worktree-lsp-v3` |
| 7 | Project rules rebuild | [ultraworkers/claw-code#3220](https://github.com/ultraworkers/claw-code/pull/3220) | `worktree-project-rules-v3` |

### Stale PRs Closed

| PR | Closed In Favor Of |
|----|-------------------|
| #2816 | #3214 (fresh branch, cherry-picked commits) |
| #2810 | #3211 + #3218 (split into credential resolution + entry points) |

---

## Context

We have 16 PRs submitted to upstream `ultraworkers/claw-code`. Four merged successfully, ten are closed (superseded, conflicts, or review issues), and two are open but stale with merge conflicts. This sprint addresses every actionable item.

### Upstream Reviewer Roster

| Handle | Role | Pattern |
|--------|------|---------|
| `code-yeongyu` | CI/rebase gatekeeper | Requests rebase on any merge conflict; marks `CHANGES_REQUESTED` |
| `1716775457damn` | Feature reviewer | Gives detailed positive feedback + suggestions; looks at UX and architecture |
| `Yeachan-Heo` | Repo owner (gaebal-gajae/clawdbot) | Closes PRs that are "not reviewable" — reverting upstream commits triggers this |

### Critical Lesson Learned

Our LSP PRs (#3098, #3099) were closed by the repo owner because rebased branches **reverted upstream commits**. The net diff showed +731/−2309 across only 3 files, making it appear we were deleting upstream fixes rather than adding a feature. Root cause: the branch was forked before 5 critical upstream commits landed, and our rebases kept the old versions of `config.rs` and `main.rs`.

**Rule going forward:** Every new PR branch must be created from **current `upstream/main`**, not rebased from an old fork. Verify with `git log` that no upstream commits are missing before opening a PR.

### Commits That Must Never Be Reverted

| Commit | PR# | Description |
|--------|-----|-------------|
| `b64df991` | #698 | Process-lifetime dedup of config deprecation warnings (`EMITTED_CONFIG_WARNINGS` / `emit_config_warning_once`) + tempfile dev-dep fix |
| `91a0681a` | #697 | Typed errors for providers |
| `c345ce6d` | — | MCP/agents/skills envelope exit-1 |
| `85e736c7` | — | Add status to sandbox JSON envelope |
| `5b79413e` | #458 | Add status to version/init/system-prompt envelopes |

After branching from `upstream/main`, verify: `git log --oneline | grep -E "b64df99|91a0681|c345ce6|85e736c|5b79413"` — all five must be present.

---

## Completed PRs (No Action Needed)

| PR | Title | Status | Reviewer Comments |
|----|-------|--------|-------------------|
| #3017 | Interactive provider wizard with fast model selection | **MERGED** | Positive. Requested screenshots/description of wizard flow — address in future PRs. |
| #3015 | Streaming robustness — OpenAI parsing, error detection, reasoning content | **MERGED** | Very positive. Called "solid groundwork for downstream feature PRs." |
| #2832 | Git-aware context tools | **MERGED** | Positive. Suggested `GitShow --format` option → tracked as Sprint Item 8. |
| #2811 | /resume latest searches all workspaces | **MERGED** | No reviewer feedback. |
| #2808 | Auto-compact and retry on context window errors | **MERGED** | No reviewer feedback. |

---

## Superseded PRs (Stay Closed)

| PR | Title | Reason Closed | Why No Action |
|----|-------|---------------|---------------|
| #2983 | Handle reasoning/thinking content from models | Reviewer objected to `PR_DESCRIPTION.md` + referral links | **Superseded** — PR #3015 already merged the reasoning content parsing at the streaming layer. Our commit `c6da823` on `feat-tui` covers the runtime-layer additions. |
| #2831 | Parallel tool execution + agent teams | Self-closed — too large for review | Split approach is correct. Individual features can be re-submitted. |
| #2830 | Project rules with .claw/rules/ (v1) | Conflicts, superseded by #3018 | Replaced by v2 attempt. |
| #2814 | Claw setup writes model at wrong JSON level | Closed | Fix was included in subsequent PRs. |

---

## Sprint Backlog

### Execution Order

```
Item 1 → Item 8 → Item 4 → Item 3 → Item 2 → Item 6 → Item 5 → Item 7 → Item 9
```

Rationale:
- **Item 1 first** — Broken UX on upstream main right now (wizard saved but never read)
- **Item 8 second** — Zero-dependency quick win, can ship while working on Item 1
- **Item 4 before 5** — Both touch `openai_compat.rs`; LSP rebuild is easier after timeout/retry lands
- **Items 5 and 7 last** — Highest risk/effort, benefit from all other changes being settled

---

### Item 1: Provider Config 3-Tier Fallback

**Priority:** P0 — BROKEN UX ON UPSTREAM MAIN
**Effort:** 2–3 hours
**Risk:** LOW
**Related PR:** #2810 (open, stale)
**Branch:** `feat/provider-config-fallback` (new, from `upstream/main`)

**Problem:** PR #3017 merged `setup_wizard.rs` but `anthropic.rs` and `openai_compat.rs` read API keys only from env vars. Settings saved by the wizard are never consumed. Users who run the wizard see no effect.

**Scope (4 files):**
- `rust/crates/api/src/providers/mod.rs` — Add `read_env_or_config()`, `provider_config_value()`, `stored_provider_kind()`
- `rust/crates/api/src/providers/anthropic.rs` — Replace inline `env::var()` calls with `read_env_or_config()`
- `rust/crates/api/src/providers/openai_compat.rs` — Same treatment as anthropic.rs
- `rust/crates/runtime/src/config.rs` — `save_user_provider_settings()` / `clear_user_provider_settings()` (may already be in from #3017; verify)

**3-Tier Resolution Logic:**
1. Environment variable (highest priority)
2. `.env` file
3. Stored config in settings.json (lowest priority)

**Steps:**
```bash
git fetch upstream
git checkout -b feat/provider-config-fallback upstream/main
# Verify critical commits present:
git log --oneline | grep -E "b64df99|91a0681|c345ce6|85e736c|5b79413"

# Transplant the config fallback functions from feat-tui
# Verify per-file diff is clean — no reverts of upstream changes
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
scripts/fmt.sh --check
```

**Verification:**
- [ ] `read_env_or_config()` resolves env → .env → stored config
- [ ] `anthropic.rs` uses `read_env_or_config()` for API key
- [ ] `openai_compat.rs` uses `read_env_or_config()` for API key
- [ ] Settings saved by wizard are actually consumed by providers
- [ ] All existing tests pass
- [ ] No upstream commits are reverted in the diff

---

### Item 2: Wizard Entry Points

**Priority:** P0 — Dead code without this
**Effort:** 3–4 hours
**Risk:** LOW-MEDIUM
**Depends on:** Item 1
**Related PR:** #2810 (open, stale)
**Branch:** `feat/provider-wizard-entry` (new, from `upstream/main`)

**Problem:** Without entry points (Ctrl+P, `/setup`, `claw setup`), `setup_wizard.rs` is unreachable code. Users cannot access the wizard that Item 1 makes functional.

**Scope (3 files):**
- `rust/crates/rusty-claude-cli/src/input.rs` — Ctrl+P hotkey handling
- `rust/crates/rusty-claude-cli/src/main.rs` — `/setup` command dispatch + `claw setup` subcommand
- `rust/crates/commands/src/lib.rs` — `/setup` slash command registration

**Steps:**
```bash
git checkout -b feat/provider-wizard-entry upstream/main
# Cherry-pick: "fix: Ctrl+P provider swap with visual feedback + clippy cleanup"
# Manually add /setup dispatch and claw setup subcommand
# Verify TUI input loop is not broken by Ctrl+P handler
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

**Caveat:** `input.rs` Ctrl+P handling may need adjustment if the TUI input loop has diverged on upstream. Visual feedback for provider swap must not break the existing input flow.

**Verification:**
- [ ] `/setup` command launches wizard from REPL
- [ ] `claw setup` launches wizard from CLI
- [ ] Ctrl+P triggers provider swap with visual feedback
- [ ] Existing input handling unaffected
- [ ] All tests pass

---

### Item 3: Auto-Compact & Retry on Context Window Errors

**Priority:** P1
**Effort:** 3–5 hours
**Risk:** MEDIUM
**Independent:** Yes (but enhanced by Item 4's error detection improvements)
**Branch:** `feat/auto-compact-retry` (new, from `upstream/main`)

**Problem:** Hitting a context window error is currently a hard stop. The session must be manually compacted and retried.

**Scope:**
- `rust/crates/rusty-claude-cli/src/main.rs` — Catch context window overflow, invoke compact, retry turn
- `rust/crates/api/src/error.rs` — `CONTEXT_WINDOW_ERROR_MARKERS` (enhanced by Item 4)

**Steps:**
```bash
git checkout -b feat/auto-compact-retry upstream/main
# Cherry-pick the auto-compact-retry commit
# Add max-retry guard to prevent infinite loops
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

**Risk:** Must correctly identify context-window errors and avoid infinite retry loops (compact fails to shrink enough → retries forever). Add a `MAX_COMPACT_RETRIES = 3` guard.

**Verification:**
- [ ] Context window error detected and auto-compaction triggered
- [ ] Turn retried after successful compaction
- [ ] Max retry guard prevents infinite loops
- [ ] Non-context-window errors are not caught by this path
- [ ] All tests pass

---

### Item 4: API Timeout/Retry — Full Rebase & Resubmit

**Priority:** P1
**Effort:** 4–6 hours
**Risk:** MEDIUM
**Related PR:** #2816 (open, stale, needs rebase)
**Branch:** `feat/api-timeout-retry-v2` (new, from `upstream/main`)

**Problem:** PR #2816 is open but hasn't been rebased since May 25. It has merge conflicts and `CHANGES_REQUESTED` status. The 6 commits are self-contained and valuable, but must be cherry-picked onto fresh `upstream/main`.

**Scope (9 files, 6 commits — cherry-pick individually):**

| # | Commit | Description | Note |
|---|--------|-------------|------|
| 1 | `ade853984` | Core: timeout config, Retry-After, configurable retry | Will conflict with `openai_compat.rs` |
| 2 | `8a8834307` | Retry 400 with transient gateway error | |
| 3 | `ed91a61ee` | "No parseable body" error marker | Complements Item 3 |
| 4 | `453ab6421` | Optional `id` field in OpenAI response | **May already be in via PR #3015** — check, skip if so |
| 5 | `baa8d1ba6` | HTML response detection in streaming | **May already be in via PR #3015** — check, skip if so |
| 6 | `33d2f7892` | Raw JSON error detection in streaming | **May already be in via PR #3015** — check, skip if so |

**Steps:**
```bash
git fetch upstream
git checkout -b feat/api-timeout-retry-v2 upstream/main

# Verify critical commits:
git log --oneline | grep -E "b64df99|91a0681|c345ce6|85e736c|5b79413"

# Cherry-pick each commit, resolving conflicts per file
git cherry-pick ade853984   # timeout/retry core
git cherry-pick 8a8834307   # 400 transient retry
git cherry-pick ed91a61ee   # no-parseable-body marker

# Check if commits 4-6 are already in upstream:
git log upstream/main --oneline | grep -i "optional.*id\|make id"
git log upstream/main --oneline | grep -i "html response\|json error"

# If not in upstream, cherry-pick:
git cherry-pick 453ab6421   # optional id
git cherry-pick baa8d1ba6   # HTML detection
git cherry-pick 33d2f7892   # JSON error detection

# Full verification:
cd rust && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
scripts/fmt.sh --check
```

**After verification:** Push and update PR #2816, or close it and open a new PR with a clean diff.

**Verification:**
- [ ] All 6 commits cherry-picked (or skipped if already upstream)
- [ ] No upstream commits reverted in the diff
- [ ] `--api-timeout` / `CLAW_API_TIMEOUT` works
- [ ] Retry-After header parsing works
- [ ] Exponential backoff with jitter
- [ ] All tests pass

---

### Item 5: LSP Integration — Clean Rebuild from `upstream/main`

**Priority:** P2
**Effort:** 12–16 hours
**Risk:** HIGH
**Do after:** Item 4 (avoids re-resolving same `openai_compat.rs` conflicts)
**Related PRs:** #2813, #3016, #3098, #3099 (all closed)
**Branch:** `feat/lsp-v3` (new, from `upstream/main`)

**Problem:** All 4 LSP PRs were closed because rebased branches reverted upstream commits. The repo owner explicitly stated: "Please rebuild the branch from current `origin/main` and verify locally before opening a new PR."

**Approach:** Fresh branch from `upstream/main`. Transplant LSP module files from `feat-tui`. Manually integrate modifications to existing files. Include reviewer-requested additions.

**New Files to Transplant (12 modules from `feat-tui`):**
```
rust/crates/runtime/src/lsp_client/mod.rs
rust/crates/runtime/src/lsp_client/dispatch.rs
rust/crates/runtime/src/lsp_client/types.rs
rust/crates/runtime/src/lsp_client/tests.rs
rust/crates/runtime/src/lsp_client/tests_lifecycle.rs
rust/crates/runtime/src/lsp_discovery.rs
rust/crates/runtime/src/lsp_process/mod.rs
rust/crates/runtime/src/lsp_process/parse.rs
rust/crates/runtime/src/lsp_process/tests.rs
rust/crates/runtime/src/lsp_transport/mod.rs
rust/crates/runtime/src/lsp_transport/tests.rs
```

**Existing Files Modified (~10):**
- `rust/crates/commands/src/lib.rs` — `/lsp` slash command registration
- `rust/crates/tools/src/lib.rs` — LSP diagnostic injection into tool responses
- `rust/crates/runtime/src/config.rs` — `lspAutoStart` config
- `rust/crates/runtime/src/config_validate.rs` — LSP config validation
- `rust/crates/runtime/src/lib.rs` — Module exports
- `rust/crates/runtime/src/compact.rs` — LSP awareness during compaction
- `rust/crates/runtime/src/policy_engine.rs` — LSP health policy rules
- `rust/crates/runtime/src/sandbox.rs` — LSP process sandboxing
- `rust/crates/rusty-claude-cli/src/main.rs` — LSP server lifecycle on startup
- `rust/crates/api/src/providers/openai_compat.rs` — LSP-related config

**Reviewer-Requested Additions (from PR #2813 review by 1716775457damn):**
- [ ] `/lsp status` command — users can check which servers are running
- [ ] Configurable diagnostic wait timeout (don't block startup indefinitely)
- [ ] Surface LSP server version info in status output

**Steps:**
```bash
git fetch upstream
git checkout -b feat/lsp-v3 upstream/main

# Verify critical commits are present:
git log --oneline | grep -E "b64df99|91a0681|c345ce6|85e736c|5b79413"
# ALL FIVE must be present. If any missing, STOP — wrong base.

# Copy LSP module files from feat-tui branch:
for f in lsp_client/mod.rs lsp_client/dispatch.rs lsp_client/types.rs \
         lsp_client/tests.rs lsp_client/tests_lifecycle.rs lsp_discovery.rs \
         lsp_process/mod.rs lsp_process/parse.rs lsp_process/tests.rs \
         lsp_transport/mod.rs lsp_transport/tests.rs; do
  git show feat-tui:rust/crates/runtime/src/$f > rust/crates/runtime/src/$f
done

# Manually apply modifications to existing files
# (commands, tools, config, lib, compact, policy_engine, sandbox, main, openai_compat)
# DO NOT copy entire files from feat-tui — only add the LSP-specific sections
# Compare: git diff upstream/main..feat-tui -- <file> | grep "^+" | grep -i lsp

# Add reviewer-requested features:
# - /lsp status command
# - Diagnostic wait timeout config
# - LSP server version reporting

# Full verification:
cd rust && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
scripts/fmt.sh --check

# CRITICAL — Before pushing, verify no upstream commits are reverted:
git diff upstream/main --stat
# The diff should show ONLY additions and LSP-specific changes.
# If any file shows large net-negative changes, STOP and investigate.
```

**Verification:**
- [ ] LSP servers auto-start on REPL launch (14 languages)
- [ ] Tool responses append LSP diagnostics
- [ ] Distro-aware install prompts work
- [ ] `/lsp start|stop|restart|status` commands work
- [ ] Diagnostic wait timeout is configurable
- [ ] LSP server version info surfaced
- [ ] No upstream commits are reverted in the diff
- [ ] `git diff upstream/main --stat` shows feature-only changes
- [ ] All tests pass

---

### Item 6: Session Resume Fixes

**Priority:** P1
**Effort:** 1–2 hours
**Risk:** LOW
**Independent:** Yes
**Related PR:** #2810 (session resume commits)
**Branch:** `feat/session-resume-fixes` (new, from `upstream/main`)

**Scope (2 files):**
- `rust/crates/runtime/src/session_control.rs` — `load_session_reference_excluding`
- `rust/crates/rusty-claude-cli/src/main.rs` — Session resume path updates

**Steps:**
```bash
git checkout -b feat/session-resume-fixes upstream/main
# Cherry-pick the 3 session resume commits from #2810
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

**Verification:**
- [ ] `/resume latest` correctly searches all workspaces
- [ ] Session exclusion works
- [ ] All tests pass

---

### Item 7: Project Rules — Clean Rebuild

**Priority:** P2
**Effort:** 4–6 hours
**Risk:** MEDIUM
**Do after:** Items 1 and 5 (avoids `config.rs` / `prompt.rs` conflict re-resolution)
**Related PR:** #3018 (closed, conflicts)
**Branch:** `feat/project-rules-v3` (new, from `upstream/main`)

**Problem:** PR #3018 was closed due to merge conflicts. The `feat/project-rules` branch predates upstream changes. Must extract only the rules-specific diff and omit `ROADMAP.md`/`progress.txt` changes.

**Scope (rules-specific from commit `ddc788e`):**
- `rust/crates/runtime/src/config.rs` — `.claw/rules/` config loading
- `rust/crates/runtime/src/prompt.rs` — Rules injection into system prompt
- ~10 other files with partial rules support

**Steps:**
```bash
git checkout -b feat/project-rules-v3 upstream/main
# Extract only rules-specific changes from commit ddc788e
# Use: git show ddc788e -- <files> to get clean patches
# OMIT: ROADMAP.md, progress.txt (reviewer objections)
# Verify: cargo test --workspace
```

**Verification:**
- [ ] `.claw/rules/` directory loaded and parsed
- [ ] Multi-framework auto-import works (CLAUDE.md, .cursorrules, .windsurfrules, etc.)
- [ ] Rules injected into system prompt
- [ ] No `ROADMAP.md` or `progress.txt` in the diff
- [ ] All tests pass

---

### Item 8: GitShow `--format` Enhancement

**Priority:** P1
**Effort:** 1–2 hours
**Risk:** VERY LOW
**Independent:** Yes — quick win, do anytime
**Related PR:** #2832 (merged — reviewer suggestion)
**Branch:** `feat/gitshow-format` (new, from `upstream/main`)

**Problem:** The reviewer of our merged PR #2832 suggested adding a `GitShow --format` option so the model can request patch vs metadata selectively.

**Scope (1 file):**
- `rust/crates/tools/src/lib.rs` — `GitShowInput` struct + `run_git_show()`

**Design:**
```
format: Option<String>  // "patch" (default) | "stat" | "metadata"

"patch"    → git show <ref>              (full diff, default - backward compatible)
"stat"     → git show --stat <ref>       (summary stats)
"metadata" → git show --format=medium --no-patch <ref>  (commit info only)
```

**Steps:**
```bash
git checkout -b feat/gitshow-format upstream/main
# In GitShowInput: add format: Option<String> (serde rename = "format")
# In run_git_show: map format values to git flags
# Update ToolSpec input_schema and tool description
# Keep stat: Option<bool> with deprecation note for backward compat
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

**Verification:**
- [ ] `format: "patch"` produces full diff
- [ ] `format: "stat"` produces summary stats
- [ ] `format: "metadata"` produces commit info only
- [ ] Default (no format) is "patch" — backward compatible
- [ ] Tool description updated for model discoverability
- [ ] All tests pass

---

### Item 9: README Referral Links

**Priority:** P3
**Effort:** 30 minutes
**Risk:** VERY LOW
**Independent:** Yes
**Related PR:** #2810 (4 referral link commits)
**Branch:** `feat/readme-referral` (new, from `upstream/main`)

**Scope:** `README.md` only — purely cosmetic addition of provider referral links.

**Caveat:** PR #2983's reviewer objected to referral links as "unrelated promotional content." Submit as a standalone PR with a clear, narrow scope — if reviewers push back, drop without impact.

**Verification:**
- [ ] Referral links added to README
- [ ] No other files changed
- [ ] markdown renders cleanly

---

## Dependency Graph

```
Item 1 (provider config fallback) ─── P0
  ├─> Item 2 (wizard entry points) ─── P0
  └─> Item 4 shares openai_compat.rs ── do before LSP
        └─> Item 5 (LSP rebuild) ────── shares main.rs, config.rs, openai_compat.rs
              └─> Item 7 (project rules) ── shares config.rs, prompt.rs

Item 3 (auto-compact-retry) ─── independent, enhanced by Item 4
Item 6 (session resume) ──────── independent
Item 8 (GitShow --format) ───── independent quick win
Item 9 (README referral) ────── independent, lowest priority
```

## Effort Summary

| # | Item | Effort | Risk | Priority | Branch Name |
|---|------|--------|------|----------|-------------|
| 1 | Provider config fallback | 2–3h | LOW | **P0** | `feat/provider-config-fallback` |
| 2 | Wizard entry points | 3–4h | LOW-MED | **P0** | `feat/provider-wizard-entry` |
| 3 | Auto-compact-retry | 3–5h | MED | P1 | `feat/auto-compact-retry` |
| 4 | API timeout/retry v2 | 4–6h | MED | P1 | `feat/api-timeout-retry-v2` |
| 5 | LSP clean rebuild | 12–16h | HIGH | P2 | `feat/lsp-v3` |
| 6 | Session resume fixes | 1–2h | LOW | P1 | `feat/session-resume-fixes` |
| 7 | Project rules rebuild | 4–6h | MED | P2 | `feat/project-rules-v3` |
| 8 | GitShow --format | 1–2h | V.LOW | P1 | `feat/gitshow-format` |
| 9 | README referral links | 30m | V.LOW | P3 | `feat/readme-referral` |

**Total estimated effort:** 31–45 hours

---

## PR Submission Checklist

For every PR opened against `ultraworkers/claw-code`:

- [ ] Branch created from **current `upstream/main`** (not rebased from old fork)
- [ ] All 5 critical upstream commits are present in the branch: `b64df991`, `91a0681a`, `c345ce6d`, `85e736c7`, `5b79413e`
- [ ] `git diff upstream/main --stat` shows **feature-only changes** — no large net-negative diffs
- [ ] No `PR_DESCRIPTION.md` or other non-code files that belong in the PR body
- [ ] No referral links unless the PR is specifically about that (Item 9)
- [ ] No `ROADMAP.md` or `progress.txt` changes
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes
- [ ] `scripts/fmt.sh --check` passes
- [ ] PR body includes clear description, screenshots if applicable
- [ ] PR is scoped to a single feature/fix

---

## Reviewer Feedback Tracker

| Reviewer | Feedback | Action | Sprint Item |
|----------|----------|--------|-------------|
| code-yeongyu | Rebase needed on PRs #2810, #2816 | Fresh branches from upstream/main | Items 1, 4 |
| code-yeongyu | LSP PRs revert upstream commits | Rebuild from scratch, verify diff | Item 5 |
| 1716775457damn | Add `/lsp status` command | Include in LSP rebuild | Item 5 |
| 1716775457damn | Make diagnostics configurable (wait timeout) | Include in LSP rebuild | Item 5 |
| 1716775457damn | Surface LSP server version info | Include in LSP rebuild | Item 5 |
| 1716775457damn | Add `GitShow --format` for patch vs metadata | Standalone PR | Item 8 |
| 1716775457damn | Add screenshots for wizard flow | Address in future PR description | — |
| 1716775457damn | Positive on provider wizard 3-tier auth | Confirm working with Item 1 | Item 1 |
| Yeachan-Heo | LSP PRs "not reviewable" — rebuild from main | Clean rebuild | Item 5 |
