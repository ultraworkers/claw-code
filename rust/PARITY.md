# Parity Status — claw-code Rust Port

Last updated: 2026-04-03

## Mock parity harness — milestone 1

- [x] Deterministic Anthropic-compatible mock service (`rust/crates/mock-anthropic-service`)
- [x] Reproducible clean-environment CLI harness (`rust/crates/rusty-claude-cli/tests/mock_parity_harness.rs`)
- [x] Scripted scenarios: `streaming_text`, `read_file_roundtrip`, `grep_chunk_assembly`, `write_file_allowed`, `write_file_denied`

## Mock parity harness — milestone 2 (behavioral expansion)

- [x] Scripted multi-tool turn coverage: `multi_tool_turn_roundtrip`
- [x] Scripted bash coverage: `bash_stdout_roundtrip`
- [x] Scripted permission prompt coverage: `bash_permission_prompt_approved`, `bash_permission_prompt_denied`
- [x] Scripted plugin-path coverage: `plugin_tool_roundtrip`
- [x] Behavioral diff/checklist runner: `rust/scripts/run_mock_parity_diff.py`

## Harness v2 behavioral checklist

Canonical scenario map: `rust/mock_parity_scenarios.json`

- Multi-tool assistant turns
- Bash flow roundtrips
- Permission enforcement across tool paths
- Plugin tool execution path
- File tools — harness-validated flows

## Completed Behavioral Parity Work

Hashes below come from `git log --oneline`. Merge line counts come from `git show --stat <merge>`.

| Lane | Status | Feature commit | Merge commit | Diff stat |
|------|--------|----------------|--------------|-----------|
| Bash validation (9 submodules) | ✅ complete | `36dac6c` | — (`jobdori/bash-validation-submodules`) | `1005 insertions` |
| CI fix | ✅ complete | `89104eb` | `f1969ce` | `22 insertions, 1 deletion` |
| File-tool edge cases | ✅ complete | `284163b` | `a98f2b6` | `195 insertions, 1 deletion` |
| TaskRegistry | ✅ complete | `5ea138e` | `21a1e1d` | `336 insertions` |
| Task tool wiring | ✅ complete | `e8692e4` | `d994be6` | `79 insertions, 35 deletions` |
| Team + cron runtime | ✅ complete | `c486ca6` | `49653fe` | `441 insertions, 37 deletions` |
| MCP lifecycle | ✅ complete | `730667f` | `cc0f92e` | `491 insertions, 24 deletions` |
| LSP client | ✅ complete | `2d66503` | `d7f0dc6` | `461 insertions, 9 deletions` |
| Permission enforcement | ✅ complete | `66283f4` | `336f820` | `357 insertions` |

## Tool Surface (spec parity plus v2.1.201 host contracts)

### Real Implementations (behavioral parity — varying depth)

| Tool | Rust Impl | Behavioral Notes |
|------|-----------|-----------------|
| **bash** | `runtime::bash` 283 LOC | subprocess exec, timeout, background, sandbox — **strong parity**. 9/9 requested validation submodules are now tracked as complete via `36dac6c`, with on-main sandbox + permission enforcement runtime support |
| **read_file** | `runtime::file_ops` | offset/limit read — **good parity** |
| **write_file** | `runtime::file_ops` | file create/overwrite — **good parity** |
| **edit_file** | `runtime::file_ops` | old/new string replacement — **good parity**. Missing: replace_all was recently added |
| **glob_search** | `runtime::file_ops` | glob pattern matching — **good parity** |
| **grep_search** | `runtime::file_ops` | ripgrep-style search — **good parity** |
| **WebFetch** | `tools` | URL fetch + content extraction — **moderate parity** (need to verify content truncation, redirect handling vs upstream) |
| **WebSearch** | `tools` | search query execution — **moderate parity** |
| **TodoWrite** | `tools` | todo/note persistence — **moderate parity** |
| **Skill** | `tools` | skill discovery/install — **moderate parity** |
| **Agent** | `tools` | agent delegation — **moderate parity** |
| **TaskCreate** | `runtime::task_registry` + `tools` | in-memory task creation wired into tool dispatch — **good parity** |
| **TaskGet** | `runtime::task_registry` + `tools` | task lookup + metadata payload — **good parity** |
| **TaskList** | `runtime::task_registry` + `tools` | registry-backed task listing — **good parity** |
| **TaskStop** | `runtime::task_registry` + `tools` | terminal-state stop handling — **good parity** |
| **TaskUpdate** | `runtime::task_registry` + `tools` | registry-backed message updates — **good parity** |
| **TaskOutput** | `runtime::task_registry` + `tools` | output capture retrieval — **good parity** |
| **TeamCreate** | `runtime::team_cron_registry` + `tools` | team lifecycle + task assignment — **good parity** |
| **TeamDelete** | `runtime::team_cron_registry` + `tools` | team delete lifecycle — **good parity** |
| **CronCreate** | `runtime::team_cron_registry` + `tools` | cron entry creation — **good parity** |
| **CronDelete** | `runtime::team_cron_registry` + `tools` | cron entry removal — **good parity** |
| **CronList** | `runtime::team_cron_registry` + `tools` | registry-backed cron listing — **good parity** |
| **LSP** | `runtime::lsp_client` + `tools` | registry + dispatch for diagnostics, hover, definition, references, completion, symbols, formatting — **good parity** |
| **ListMcpResources** | `runtime::mcp_tool_bridge` + `tools` | connected-server resource listing — **good parity** |
| **ReadMcpResource** | `runtime::mcp_tool_bridge` + `tools` | connected-server resource reads — **good parity** |
| **ReadMcpResourceDir** | `runtime::mcp_tool_bridge` + `tools` | prefix-filtered MCP resource directory listing — **contract parity** |
| **McpAuth** | `runtime::mcp_tool_bridge` + `tools` | structured status/login/logout host-flow payloads — **contract parity**, no browser/OAuth host integration |
| **MCP** | `runtime::mcp_tool_bridge` + `tools` | stateful MCP tool invocation bridge — **good parity** |
| **ToolSearch** | `tools` | tool discovery — **good parity** |
| **NotebookEdit** | `tools` | jupyter notebook cell editing — **moderate parity** |
| **Sleep** | `tools` | delay execution — **good parity** |
| **SendUserMessage/Brief** | `tools` | user-facing message — **good parity** |
| **Config** | `tools` | config inspection — **moderate parity** |
| **EnterPlanMode** | `tools` | worktree plan mode toggle — **good parity** |
| **ExitPlanMode** | `tools` | worktree plan mode restore — **good parity** |
| **StructuredOutput** | `tools` | passthrough JSON — **good parity** |
| **REPL** | `tools` | subprocess code execution — **moderate parity** |
| **PowerShell** | `tools` | Windows PowerShell execution — **moderate parity** |
| **Workflow / Monitor** | `tools` | v2.1.201 compatibility contracts with stable unsupported payloads; local workflow JS and live monitor scheduling are not implemented |
| **ScheduleWakeup / PushNotification / ReportFindings / Artifact / Projects / ClaudeDesign / ShowOnboardingRolePicker** | `tools` | host-facing v2.1.201 contract payloads — **contract parity** |

### Stubs Only (surface parity, no behavior)

| Tool | Status | Notes |
|------|--------|-------|
| **AskUserQuestion** | stub | needs live user I/O integration |
| **RemoteTrigger** | stub | needs HTTP client |
| **TestingPermission** | stub | test-only, low priority |

## Slash Commands: 67/141 upstream entries

- 27 original specs (pre-today) — all with real handlers
- 40 new specs — parse + stub handler ("not yet implemented")
- Remaining ~74 upstream entries are internal modules/dialogs/steps, not user `/commands`

### Behavioral Feature Checkpoints (completed work + remaining gaps)

**Bash tool — 9/9 requested validation submodules complete:**
- [x] `sedValidation` — validate sed commands before execution
- [x] `pathValidation` — validate file paths in commands
- [x] `readOnlyValidation` — block writes in read-only mode
- [x] `destructiveCommandWarning` — warn on rm -rf, etc.
- [x] `commandSemantics` — classify command intent
- [x] `bashPermissions` — permission gating per command type
- [x] `bashSecurity` — security checks
- [x] `modeValidation` — validate against current permission mode
- [x] `shouldUseSandbox` — sandbox decision logic

Harness note: milestone 2 validates bash success plus workspace-write escalation approve/deny flows; dedicated validation submodules landed in `36dac6c`, and on-main runtime also carries sandbox + permission enforcement.

**File tools — completed checkpoint:**
- [x] Path traversal prevention (symlink following, ../ escapes)
- [x] Size limits on read/write
- [x] Binary file detection
- [x] Permission mode enforcement (read-only vs workspace-write)

Harness note: read_file, grep_search, write_file allow/deny, and multi-tool same-turn assembly are now covered by the mock parity harness; file edge cases + permission enforcement landed in `a98f2b6` and `336f820`.

**Config/Plugin/MCP flows:**
- [x] Full MCP server lifecycle (connect, list tools, call tool, disconnect)
- [ ] Plugin install/enable/disable/uninstall full flow
- [ ] Config merge precedence (user > project > local)

Harness note: external plugin discovery + execution is now covered via `plugin_tool_roundtrip`; MCP lifecycle landed in `cc0f92e`, while plugin lifecycle + config merge precedence remain open.

## Runtime Behavioral Gaps

- [x] Permission enforcement across all tools (read-only, workspace-write, danger-full-access)
- [ ] Output truncation (large stdout/file content)
- [ ] Session compaction behavior matching
- [ ] Token counting / cost tracking accuracy
- [x] Streaming response support validated by the mock parity harness

Harness note: current coverage now includes write-file denial, bash escalation approve/deny, and plugin workspace-write execution paths; permission enforcement landed in `336f820`.

## Migration Readiness

- [x] `PARITY.md` maintained and honest
- [ ] No `#[ignore]` tests hiding failures (only 1 allowed: `live_stream_smoke_test`)
- [ ] CI green on every commit
- [ ] Codebase shape clean for handoff
