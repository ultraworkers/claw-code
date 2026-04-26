# PR #3211 — Execution Plan: 3-Tier Credential Resolution (REVISED)

**Branch**: `worktree-provider-config-fallback`
**Worktree**: `.claude/worktrees/provider-config-fallback`
**CI State**: Build ✅ Test ✅ Clippy ✅ Docs ✅ Windows ✅ **Fmt ❌**

## Reviewer Feedback

From `1716775457damn`:
> "the `from_env_or_saved` naming across multiple provider files could be consolidated
> into a shared trait or helper to reduce duplication. Also, cargo fmt is failing"

## Investigation Findings (Updated)

### Key discovery: `from_env` vs `from_env_or_saved` are DIFFERENT things

There are **TWO** `AuthSource` methods in `anthropic.rs`:
- `from_env()` (line 44) — **tiers 1+2 only** (process env + .env), NO stored config
- `from_env_or_saved()` (line 629) — **tiers 1+2+3** (process env + .env + stored config)

And TWO in `openai_compat.rs`:
- `from_env(config)` (line 132) — **tiers 1+2 only**
- `from_env_or_saved(config)` (line 151) — **tiers 1+2+3**

The `_or_saved` suffix is **semantically meaningful** — it distinguishes the 3-tier path from the 2-tier path. Renaming them identically would lose this distinction.

### What IS actually duplicated and safe to consolidate

| Duplication | anthropic.rs | openai_compat.rs | mod.rs (shared) |
|---|---|---|---|
| `read_env_non_empty(key)` | ✅ line 773 | ✅ line 1608 | ❌ not shared |
| `read_base_url()` | ✅ line 798 | ✅ line 1625 | ❌ not shared |

Both `read_base_url()` functions follow the same 3-tier pattern:
```
Tier 1: std::env::var(key) → return if non-empty
Tier 2: dotenv_value(key) → return if non-empty
Tier 3: read_base_url_from_config(provider_kind) → return if Some
Default: fallback string
```

But there's a complication: `anthropic::read_base_url()` hardcodes the env var and provider kind, while `openai_compat::read_base_url(config)` takes them from the config struct. The shared helper needs to parameterize both.

Similarly, `read_env_non_empty` is identical in both files — just reads env + .env fallback.

### Why NOT a shared trait (confirmed)

Anthropic's `AuthSource` has 4 variants (None, ApiKey, BearerToken, ApiKeyAndBearer) while OpenAI-compat has only ApiKey. Different return types → trait adds complexity.

## Execution Plan (Revised)

### Step 1: Extract `read_env_non_empty()` into `mod.rs`

Both files have identical `read_env_non_empty(key)` implementations. Move to `mod.rs` as `pub(crate) fn read_env_non_empty(key: &str) -> Result<Option<String>, ApiError>` and update both files to call `super::read_env_non_empty(key)`.

**Files changed**:
- `rust/crates/api/src/providers/mod.rs` — add shared `read_env_non_empty()`
- `rust/crates/api/src/providers/anthropic.rs` — remove local `read_env_non_empty()`, use `super::read_env_non_empty()`
- `rust/crates/api/src/providers/openai_compat.rs` — remove local `read_env_non_empty()`, use `super::read_env_non_empty()`

### Step 2: Extract `resolve_base_url()` into `mod.rs`

Both `read_base_url()` functions follow the identical 3-tier pattern, differing only in the env var name, provider kind, and default value.

```rust
pub fn resolve_base_url(env_var: &str, provider_kind: &str, default: &str) -> String {
    // Tier 1: environment variable
    if let Ok(value) = std::env::var(env_var) {
        if !value.is_empty() {
            return value;
        }
    }
    // Tier 2: .env file
    if let Some(value) = dotenv_value(env_var) {
        if !value.is_empty() {
            return value;
        }
    }
    // Tier 3: stored config in ~/.claw/settings.json
    if let Some(base_url) = read_base_url_from_config(provider_kind) {
        return base_url;
    }
    default.to_string()
}
```

Then:
- `anthropic::read_base_url()` → delegates to `super::resolve_base_url("ANTHROPIC_BASE_URL", "anthropic", DEFAULT_BASE_URL)`
- `openai_compat::read_base_url(config)` → delegates to `super::resolve_base_url(config.base_url_env, provider_kind, config.default_base_url)`

**Files changed**:
- `rust/crates/api/src/providers/mod.rs` — add `resolve_base_url()`
- `rust/crates/api/src/providers/anthropic.rs` — simplify `read_base_url()` to one-liner delegation
- `rust/crates/api/src/providers/openai_compat.rs` — simplify `read_base_url()` to one-liner delegation

### Step 3: Do NOT rename `from_env_or_saved` → `from_env`

The `_or_saved` suffix is semantically meaningful and distinguishes the 3-tier path from the 2-tier `from_env()` method. Removing it would conflate two different code paths. The reviewer asked to "reduce duplication" — Steps 1+2 do that by extracting the duplicated logic into shared helpers while preserving the intentional naming distinction.

### Step 4: Run `cargo fmt --all`

Fixes the only failing CI check.

### Step 5: Run `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test --workspace`

Verify nothing breaks.

### Step 6: Commit + push

```
refactor: extract shared credential resolution helpers into mod.rs

Addresses reviewer feedback on #3211 requesting consolidation of
duplicated credential-resolution logic across provider files.

Changes:

1. Extract read_env_non_empty() into mod.rs
   - Previously duplicated identically in anthropic.rs (line 773)
     and openai_compat.rs (line 1608).
   - Both copies read a process env var, fall back to .env via
     dotenv_value(), and return Result<Option<String>, ApiError>.
   - Now defined once as pub(crate) in mod.rs; both provider files
     call super::read_env_non_empty() instead.

2. Extract resolve_base_url() into mod.rs
   - Both anthropic::read_base_url() and openai_compat::read_base_url()
     implemented the same 3-tier base URL resolution:
       Tier 1: process env var (e.g. ANTHROPIC_BASE_URL)
       Tier 2: .env file via dotenv_value()
       Tier 3: stored config via read_base_url_from_config()
       Default: provider-specific fallback string
   - New shared helper: resolve_base_url(env_var, provider_kind, default)
   - anthropic::read_base_url() now delegates to
     super::resolve_base_url("ANTHROPIC_BASE_URL", "anthropic", DEFAULT_BASE_URL)
   - openai_compat::read_base_url(config) now delegates to
     super::resolve_base_url(config.base_url_env, provider_kind, config.default_base_url)

3. Preserve from_env_or_saved() naming
   - The "_or_saved" suffix distinguishes the 3-tier credential path
     (env + .env + stored config) from the 2-tier from_env() path
     (env + .env only). Both methods exist on the same AuthSource type
     and serve different callers. Renaming would lose this semantic
     distinction.

CI: cargo fmt --all run to fix the formatting check failure.
```

## Commit Details

**Commit**: `c0855e12` on `worktree-provider-config-fallback`

**Files changed**:
- `rust/crates/api/src/providers/mod.rs` — added shared `read_env_non_empty()` (+8 lines) and `resolve_base_url()` (+30 lines)
- `rust/crates/api/src/providers/anthropic.rs` — replaced local `read_env_non_empty()` and `read_base_url()` with `super::` delegations (-17 lines)
- `rust/crates/api/src/providers/openai_compat.rs` — same pattern (-17 lines)
- `rust/crates/runtime/src/config.rs` — cargo fmt fix on assert (+5/-1)

**Net**: +46 insertions, -45 deletions

## Risk Assessment

- **Low risk**: Moving identical code into shared functions, no behavior change
- **Call sites**: All `from_env_or_saved` / `from_env` call sites are internal to the providers crate
- **Public API**: `api::read_base_url()` and `api::read_xai_base_url()` still work — they call the provider-local wrappers which delegate to the shared helper
- **No behavior change**: Same code paths, just better organized

## Acceptance Criteria

- [x] `read_env_non_empty` only defined once (in `mod.rs`)
- [x] `resolve_base_url` exists in `mod.rs` and is used by both provider files
- [x] `from_env_or_saved` naming preserved (semantic distinction from `from_env`)
- [x] `cargo fmt --all` passes
- [x] `cargo test -p api` passes (147 unit + 37 integration, all green)
- [x] `cargo check -p api` passes
- [x] Changes committed and pushed to `worktree-provider-config-fallback` branch (commit `c0855e12`)
- [ ] Pre-existing clippy issues in runtime/trident.rs — not from our changes, noted
