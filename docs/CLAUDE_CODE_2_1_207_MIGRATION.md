# Claude Code 2.1.207 migration status

Last verified: 2026-07-13

Authoritative upstream baseline:

- npm `@anthropic-ai/claude-code@latest`: `2.1.207`
- official changelog: <https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md>
- covered release interval in this update: `2.1.202` through `2.1.207`

This document records implemented behavior separately from compatibility surfaces and open work. A listed upstream changelog item is not considered complete merely because a similarly named command or tool exists.

## Implemented in the migration branch

| Upstream release | Behavior | Local evidence |
|---|---|---|
| 2.1.202 | `dynamicWorkflowSize` accepts `small`, `medium`, or `large` as an advisory workflow fan-out setting | `runtime::DynamicWorkflowSize`, config validation/tests, workflow run manifests |
| 2.1.202 | Workflow child records carry `workflow.run_id` and `workflow.name` attributes | `WorkflowRunOutput.telemetryAttributes` and agent manifests |
| 2.1.205 | `/checkup` aliases `/doctor` | shared slash-command specs, direct CLI dispatch, parser tests |
| 2.1.205 | `Claude Browser` and `Claude Preview` are reserved MCP server names | scoped MCP config validation and regression tests |
| 2.1.206 | Per-server MCP `request_timeout_ms` is honored, with camelCase and legacy timeout aliases | `McpStdioServerConfig.tool_call_timeout_ms`, bootstrap/manager path, config tests |
| 2.1.207 | Shell-form hooks and MCP `headersHelper` reject `${user_config.*}` interpolation | partial config validation records unsafe entries without executing them |
| 2.1.207 | Safe `cd <workspace> && <read command> >/dev/null` is no longer classified as unrestricted shell access | bash permission classifier regression tests |

The former v2.1.201 host-only surfaces also moved forward:

- `Monitor` now polls a command and/or file condition with bounded timeout and interval values.
- `ScheduleWakeup` now persists a durable queue record under `CLAWD_WAKEUP_STORE` or `.clawd-wakeups/`.
- Workflow manifests report the selected size, recommended agent range, and workflow telemetry attributes.
- The built-in `deep-research` workflow fans out to 2, 4, or 8 static agents for small, medium, or large mode.

## Compatibility surface, not full upstream behavior

- Workflow execution still parses a safe static subset of JavaScript-like `agent(...)` calls; it is not an isolated general JavaScript VM.
- Workflow telemetry attributes are persisted in local manifests. There is not yet a complete OTLP exporter that emits them to an external OpenTelemetry collector.
- A scheduled wakeup is durable, but a long-lived host dispatcher still needs to consume due queue entries after the originating CLI exits.
- `PushNotification`, `ReportFindings`, `Artifact`, `Projects`, `ClaudeDesign`, and `ShowOnboardingRolePicker` expose stable host payloads but do not reproduce every desktop/mobile host integration.

## Open 2.1.202–2.1.207 work

The following upstream areas remain unproven or incomplete and must stay on the migration backlog:

- full background daemon upgrade/recovery, cold-resume, cross-agent TaskStop/TaskOutput, and remote-control synchronization;
- additional working directories in MCP `roots/list` plus `roots/list_changed` notifications;
- `--json-schema` request/output enforcement, including accepted JSON Schema `format` keywords and invalid-schema failures;
- complete auto-mode classifier behavior, provider entitlement/default handling, transcript tamper protection, and unresolved-variable `rm -rf` prompting;
- Bedrock, Vertex AI, Foundry, Claude Platform on AWS, gateway login, SSO refresh, and provider-specific model picker behavior;
- updater/installer streaming, launcher/symlink ownership detection, retry behavior, and Homebrew channel diagnostics;
- full `/doctor` remediation/checkup flow and checked-in `CLAUDE.md` trimming recommendations;
- `/cd` directory completion and external `EnterWorktree` confirmation UX;
- desktop/mobile Remote Control attachment, status, slash-command, and reconnection behavior;
- Windows junction/worktree deletion hardening, deleted-CWD handling, AWS credential stall guards, and terminal input regressions;
- plugin LSP failover, verify-skill rewrite suppression, plugin option scope restrictions, and monitor-specific option substitution controls;
- renderer virtualization and responsiveness fixes for long streaming blocks, scrolling, transcript positioning, and agent-view layout;
- worktree sparse-path cleanup and malformed bracket-pattern recovery across rules, skills, ignore files, and worktree includes.

## Verification commands

```bash
scripts/fmt.sh --check
cd rust
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Completion of the overall migration requires the open list above to be implemented or explicitly demonstrated as not applicable, followed by the full workspace gates.
