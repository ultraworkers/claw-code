# PR #3219 — Execution Plan: LSP Integration v3

**Branch**: `worktree-lsp-v3`
**Worktree**: `.claude/worktrees/wf_616683a9-d46-5`
**CI State**: **Build ❌** Test ❌ Fmt ✅ Clippy ✅ Docs ✅ Windows ✅

## Reviewer Feedback

From `1716775457damn`:
> "This is an impressive piece of work. The modular split across 4 crates with 58 unit tests is exactly the right architecture. The lazy-start pattern and diagnostic enrichment in tool output make this genuinely useful.
>
> Two things to check before merge:
> - build and cargo test are currently failing in CI — worth investigating if these are test assertions that need updating for the new LSP types
> - The auto-discovery of 14 LSP servers via PATH probing is great, but consider adding a `claw lsp status` command so users can confirm which servers were discovered and their health"

## Investigation Findings

### CI failure: same slash command count issue as #3218

The test `renders_help_from_shared_specs` asserts `slash_command_specs().len() == 139`, but this PR adds `/lsp` (1 new command), making it 140. Same pattern as #3218.

Locally confirmed:
```
assertion `left == right` failed
left: 140
right: 139
```

Both Build ❌ and Test ❌ are from this single test failure.

### LSP module architecture

The LSP code lives as **modules within the `runtime` crate** (not separate crates), despite the PR description:
- `rust/crates/runtime/src/lsp_discovery.rs` — auto-discovery of 14 LSP servers
- `rust/crates/runtime/src/lsp_client/` — LSP client lifecycle, dispatch, types
- `rust/crates/runtime/src/lsp_transport/` — transport layer
- `rust/crates/runtime/src/lsp_process/` — process management

### Current `/lsp` command state

The `/lsp` slash command is **registered but not implemented** — executing it just prints "not yet implemented in this build." at `main.rs:6124`.

The `SlashCommand::Lsp` variant has `action: Option<String>` and `target: Option<String>`, so it supports:
- `/lsp` (no args)
- `/lsp status`
- `/lsp start <target>`
- `/lsp stop <target>`
- `/lsp list`

### Available LSP discovery API

Currently exported from `runtime`:
- `discover_available_servers()` → `Vec<LspServerDescriptor>` — servers found on PATH
- `known_lsp_servers()` → `Vec<LspServerDescriptor>` — all 14 known servers
- `find_server_for_file(path)` → `Option<LspServerDescriptor>` — server for a file extension
- `command_exists_on_path(cmd)` → `bool` — check if a binary is available

Not exported but available:
- `check_lsp_availability()` → `Vec<LspInstallAction>` — detailed status (Installed/Missing/RustupProxyMissing)
- `format_install_prompt(actions)` → `String` — human-readable install instructions

### `LspInstallAction` variants

```rust
pub enum LspInstallAction {
    Installed,
    Missing { language: String, instructions: Vec<InstallInstruction> },
    RustupProxyMissing { language: String, component: String },
}
```

## Execution Plan

### Step 1: Fix CI — update slash command count

Change `139` → `140` at `commands/src/lib.rs:5246`. Add `/lsp` assertion to the test.

### Step 2: Export `check_lsp_availability` and related types from runtime

Add to `runtime/src/lib.rs` pub use:
- `check_lsp_availability`
- `LspInstallAction`
- `InstallInstruction`
- `format_install_prompt`

### Step 3: Implement `/lsp` slash command handler in main.rs

Replace the "not yet implemented" stub with a handler that dispatches on `action`:

| Action | Behavior |
|--------|----------|
| `None` or `"status"` | Show all discovered servers, their availability, and install hints for missing ones |
| `"list"` | List known LSP servers (all 14) with availability status |
| `"start"` / `"stop"` | Print "not yet available" — lazy-start is handled by tool infrastructure, not manual commands |

The `status` output format:
```
LSP Server Status (14 known, X available):

  ✅ rust-analyzer    — .rs files       [installed]
  ✅ typescript-language-server — .ts/.tsx/.js/.jsx [installed]
  ❌ gopls            — .go files       [missing: go install golang.org/x/tools/gopls@latest]
  ❌ pylsp            — .py files       [missing: pip install python-lsp-server]
  ⚠️ rust-analyzer   — .rs files       [rustup proxy: run `rustup component add rust-analyzer`]

Run /lsp list for all known servers.
```

### Step 4: Update the test assertions

Add `assert!(help.contains("/lsp"))` to `renders_help_from_shared_specs`.

### Step 5: Verify — cargo test -p commands, cargo test -p runtime, cargo test -p rusty-claude-cli

### Step 6: Commit + push

## Commit Details

**Commit**: `369452c` on `worktree-lsp-v3`

```
fix: address CI failure and implement /lsp status command

- Fix renders_help_from_shared_specs test: update
  slash_command_specs().len() from 139 to 140. The /lsp command
  added by this PR increased the spec count by 1 but the test
  was not updated. Also add assert!(help.contains("/lsp")).

- Export check_lsp_availability, LspInstallAction,
  InstallInstruction, and format_install_prompt from the runtime
  crate's public API so the CLI can access LSP discovery status.

- Implement /lsp slash command handler:
  - /lsp or /lsp status — show all known LSP servers with
    availability (installed ✅, missing ❌ with install hint,
    rustup proxy ⚠️)
  - /lsp list — list all known servers with language and extensions
  - /lsp start/stop — note that lazy-start is automatic, not manual

  Addresses reviewer feedback on #3219 requesting a way for
  users to confirm which LSP servers were discovered and their
  health.
```

**Files changed**:
- `rust/crates/commands/src/lib.rs` — update count 139→140, add `/lsp` assertion
- `rust/crates/runtime/src/lib.rs` — export `check_lsp_availability`, `LspInstallAction`, `InstallInstruction`, `format_install_prompt`
- `rust/crates/rusty-claude-cli/src/main.rs` — implement `/lsp` slash command handler replacing "not yet implemented" stub, add runtime imports

## Acceptance Criteria

- [x] `slash_command_specs().len()` assertion updated to 140
- [x] `/lsp` content assertion added
- [x] `check_lsp_availability` and related types exported from runtime
- [x] `/lsp status` shows discovered servers with availability + install hints
- [x] `/lsp list` shows all known servers
- [x] `cargo test -p commands` passes (42 tests)
- [x] `cargo test -p runtime` (LSP tests) passes (58 tests)
- [x] `cargo build -p rusty-claude-cli` passes
- [x] Changes committed and pushed (commit `369452c`)
- [x] 8 pre-existing CLI test failures (local config parse issue) — not from our changes
