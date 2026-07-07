use crate::*;

use api::{
    detect_provider_kind, model_family_identity_for, resolve_startup_auth_source, AnthropicClient,
    AuthSource, ContentBlockDelta, InputContentBlock, InputMessage, MessageRequest,
    MessageResponse, OutputContentBlock, PromptCache, ProviderClient as ApiProviderClient,
    ProviderKind, StreamEvent as ApiStreamEvent, ToolChoice, ToolDefinition,
    ToolResultContentBlock,
};
use commands::{
    classify_skills_slash_command, handle_agents_slash_command, handle_agents_slash_command_json,
    handle_mcp_slash_command, handle_mcp_slash_command_json, handle_plugins_slash_command,
    handle_skills_slash_command, handle_skills_slash_command_json, render_slash_command_help,
    render_slash_command_help_filtered, resolve_skill_invocation, resume_supported_slash_commands,
    slash_command_specs, validate_slash_command_input, PluginsCommandResult, SkillSlashDispatch,
    SlashCommand,
};
use init::initialize_repo;
use log::debug;
use plugins::{PluginHooks, PluginManager, PluginManagerConfig, PluginRegistry};
use render::{MarkdownStreamState, Spinner, TerminalRenderer};
use runtime::{
    check_base_commit, format_stale_base_warning, format_usd, load_oauth_credentials,
    load_system_prompt, load_system_prompt_with_context, pricing_for_model, resolve_expected_base,
    resolve_sandbox_status, ApiClient, ApiRequest, AssistantEvent, BaseCommitState,
    CompactionConfig, ConfigFileReport, ConfigLoader, ConfigSource, ContentBlock, ContextFile,
    ConversationMessage, ConversationRuntime, McpConfigCollection, McpInvalidServerConfig,
    McpServer, McpServerManager, McpServerSpec, McpTool, MessageRole, ModelPricing, PermissionMode,
    PermissionPolicy, ProjectContext, PromptCacheEvent, ResolvedPermissionMode, RuntimeError,
    RuntimeInvalidHookConfig, Session, TokenUsage, ToolError, ToolExecutor, UsageTracker,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::net::TcpListener;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, UNIX_EPOCH};
use tools::{
    canonical_allowed_tool_name, execute_tool, mvp_tool_specs, GlobalToolRegistry,
    RuntimeToolDefinition, ToolSearchOutput,
};

/// #77: Classify a stringified error message into a machine-readable kind.
///
/// Returns a `snake_case` token that downstream consumers can switch on instead
/// of regex-scraping the prose. The classification is best-effort prefix/keyword
/// matching against the error messages produced throughout the CLI surface.
pub(crate) fn classify_error_kind(message: &str) -> &'static str {
    // Check specific patterns first (more specific before generic)
    if message.starts_with("unknown_slash_command:") {
        "unknown_slash_command"
    } else if message.starts_with("command_not_found:") {
        "command_not_found"
    } else if message.contains("missing Anthropic credentials") {
        "missing_credentials"
    } else if message.contains("Manifest source files are missing")
        || message.starts_with("missing_manifests:")
    {
        "missing_manifests"
    } else if message.contains("no worker state file found") {
        "missing_worker_state"
    } else if message.contains("session not found") {
        "session_not_found"
    } else if message.contains("no managed sessions found") {
        "no_managed_sessions"
    } else if message.contains("legacy session is missing workspace binding") {
        // #780: must precede the generic "failed to restore session" arm — the full
        // error message is "failed to restore session: legacy session is missing workspace
        // binding: ...", so the specific arm must be checked first.
        "legacy_session_no_workspace_binding"
    } else if message.contains("Is a directory") || message.contains("os error 21") {
        // #787: --resume given a directory path instead of a .jsonl file
        "session_path_is_directory"
    } else if message.contains("failed to restore session") {
        "session_load_failed"
    } else if message.contains("unsupported ACP invocation") {
        "unsupported_acp_invocation"
    } else if message.starts_with("missing_argument:") {
        "missing_argument"
    } else if message.contains("unsupported skills action") {
        "unsupported_skills_action"
    } else if message.starts_with("invalid_install_source:") {
        "invalid_install_source"
    } else if message.starts_with("invalid_cwd:") {
        "invalid_cwd"
    } else if message.starts_with("invalid_output_path:") {
        "invalid_output_path"
    } else if message.starts_with("invalid_output_format:") {
        "invalid_output_format"
    } else if message.starts_with("invalid_tool_name:") {
        "invalid_tool_name"
    } else if message.contains("unrecognized argument") || message.contains("unknown option") {
        "cli_parse"
    } else if message.starts_with("missing_flag_value:") {
        "missing_flag_value"
    } else if message.starts_with("invalid_permission_mode:") {
        "invalid_permission_mode"
    } else if message.starts_with("invalid_flag_value:") {
        "invalid_flag_value"
    } else if message.starts_with("invalid_model:") {
        "invalid_model"
    } else if message.contains("invalid model syntax") {
        "invalid_model_syntax"
    } else if message.contains("is not yet implemented") {
        "unsupported_command"
    } else if message.contains("unsupported resumed command") {
        "unsupported_resumed_command"
    } else if message.contains("confirmation required") {
        "confirmation_required"
    } else if (message.contains("api failed") || message.contains("api returned"))
        && (message.contains("401")
            || message.contains("Unauthorized")
            || message.contains("authentication_error"))
    {
        // #781: sub-classify auth failures so wrappers can distinguish from rate-limit / server errors
        "api_auth_error"
    } else if (message.contains("api failed") || message.contains("api returned"))
        && (message.contains("429")
            || message.contains("rate_limit")
            || message.contains("rate limit"))
    {
        // #781: sub-classify rate-limit failures
        "api_rate_limit_error"
    } else if message.contains("api failed") || message.contains("api returned") {
        "api_http_error"
    } else if message.contains("mcpServers") {
        "malformed_mcp_config"
    } else if message.contains(".claw/settings.json") || message.contains(".claw.json") {
        // #763: config file JSON parse / validation errors (e.g. unterminated string, type mismatch)
        "config_parse_error"
    } else if message.starts_with("empty prompt") {
        "empty_prompt"
    } else if message.starts_with("interactive_only:") || message.contains("stdin is not a TTY") {
        "interactive_only"
    } else if message.starts_with("unknown agents subcommand:") {
        "unknown_agents_subcommand"
    } else if message.starts_with("agent not found:") {
        "agent_not_found"
    } else if message.contains("is not installed") || message.starts_with("plugin_not_found:") {
        "plugin_not_found"
    } else if message.contains("plugin source") && message.contains("was not found") {
        // #794: `plugins install /nonexistent/path` → "plugin source ... was not found"
        "plugin_source_not_found"
    } else if (message.contains("skill source") && message.contains("not found"))
        || message.starts_with("skill '")
    {
        "skill_not_found"
    } else if message.contains("Unsupported config section") {
        "unsupported_config_section"
    } else if message.contains("unknown_plugins_action") {
        "unknown_plugins_action"
    } else if message.starts_with("invalid_history_count:") || message.contains("invalid count") {
        "invalid_history_count"
    } else if message.starts_with("missing_prompt:") {
        "missing_prompt"
    } else if message.contains("has been removed.") {
        // #765: removed subcommands (login, logout) — hint contains migration guidance
        "removed_subcommand"
    } else if message.starts_with("unknown subcommand:") {
        // #785/#825: typo/unknown top-level subcommand (e.g. `claw dump` → did you mean dump-manifests?)
        // Unified under command_not_found in #825.
        "command_not_found"
    } else if message.starts_with("unexpected extra arguments")
        || message.starts_with("unexpected_extra_args:")
    {
        // #766: extra positionals after commands that take no arguments (e.g. claw diff)
        // #784: export extra-positional errors use the typed prefix form
        "unexpected_extra_args"
    } else if message.starts_with("invalid_resume_argument:") {
        // #768: --resume trailing arg is not a slash command
        "invalid_resume_argument"
    } else if message.starts_with("unknown_option:") {
        "unknown_option"
    } else if message.contains("is a slash command")
        || message.starts_with("interactive_only:")
        // #735: "slash command /X is interactive-only" emitted by interactive-only guard
        || (message.starts_with("slash command") && message.contains("interactive-only"))
    {
        "interactive_only"
    } else {
        "unknown"
    }
}

/// #77: Split a multi-line error message into (`short_reason`, `optional_hint`).
///
/// The `short_reason` is the first line (up to the first newline), and the hint
/// is the remaining text or `None` if there's no newline. This prevents the
/// runbook prose from being stuffed into the `error` field that downstream
/// parsers expect to be the short reason alone.
pub(crate) fn split_error_hint(message: &str) -> (String, Option<String>) {
    match message.split_once('\n') {
        Some((short, hint)) => (short.to_string(), Some(hint.trim().to_string())),
        None => (message.to_string(), None),
    }
}

/// Merge a piped stdin payload into a prompt argument.
///
/// When `stdin_content` is `None` or empty after trimming, the prompt is
/// returned unchanged. Otherwise the trimmed stdin content is appended to the
/// prompt separated by a blank line so the model sees the prompt first and the
/// piped context immediately after it.
pub(crate) fn merge_prompt_with_stdin(prompt: &str, stdin_content: Option<&str>) -> String {
    let Some(raw) = stdin_content else {
        return prompt.to_string();
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return prompt.to_string();
    }
    if prompt.is_empty() {
        return trimmed.to_string();
    }
    format!("{prompt}\n\n{trimmed}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliAction {
    DumpManifests {
        output_format: CliOutputFormat,
        manifests_dir: Option<PathBuf>,
    },
    BootstrapPlan {
        output_format: CliOutputFormat,
    },
    Agents {
        args: Option<String>,
        output_format: CliOutputFormat,
    },
    Mcp {
        args: Option<String>,
        output_format: CliOutputFormat,
    },
    Skills {
        args: Option<String>,
        output_format: CliOutputFormat,
    },
    Plugins {
        action: Option<String>,
        target: Option<String>,
        output_format: CliOutputFormat,
    },
    PrintSystemPrompt {
        cwd: PathBuf,
        date: String,
        model: String,
        output_format: CliOutputFormat,
    },
    Version {
        output_format: CliOutputFormat,
    },
    SessionList {
        output_format: CliOutputFormat,
    },
    ResumeSession {
        session_path: PathBuf,
        commands: Vec<String>,
        output_format: CliOutputFormat,
        allow_broad_cwd: bool,
    },
    Status {
        model: String,
        // #148: raw `--model` flag input (pre-alias-resolution), if any.
        // None means no flag was supplied; env/config/default fallback is
        // resolved inside `print_status_snapshot`.
        model_flag_raw: Option<String>,
        permission_mode: PermissionModeProvenance,
        output_format: CliOutputFormat,
        allowed_tools: Option<AllowedToolSet>,
    },
    Sandbox {
        output_format: CliOutputFormat,
    },
    Prompt {
        prompt: String,
        model: String,
        output_format: CliOutputFormat,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
        compact: bool,
        base_commit: Option<String>,
        reasoning_effort: Option<String>,
        allow_broad_cwd: bool,
    },
    Doctor {
        output_format: CliOutputFormat,
        permission_mode: PermissionModeProvenance,
    },
    Acp {
        output_format: CliOutputFormat,
    },
    State {
        output_format: CliOutputFormat,
    },
    Init {
        output_format: CliOutputFormat,
    },
    Setup {
        output_format: CliOutputFormat,
    },
    // #146: `claw config` and `claw diff` are pure-local read-only
    // introspection commands; wire them as standalone CLI subcommands.
    Config {
        section: Option<String>,
        output_format: CliOutputFormat,
    },
    Models {
        action: Option<String>,
        output_format: CliOutputFormat,
    },
    Diff {
        output_format: CliOutputFormat,
    },
    Export {
        session_reference: String,
        output_path: Option<PathBuf>,
        output_format: CliOutputFormat,
    },
    Repl {
        model: String,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
        base_commit: Option<String>,
        reasoning_effort: Option<String>,
        allow_broad_cwd: bool,
    },
    HelpTopic {
        topic: LocalHelpTopic,
        output_format: CliOutputFormat,
    },
    // prompt-mode formatting is only supported for non-interactive runs
    Help {
        output_format: CliOutputFormat,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalHelpTopic {
    Status,
    Sandbox,
    Doctor,
    Acp,
    // #141: extend the local-help pattern to every subcommand so
    // `claw <subcommand> --help` has one consistent contract.
    Init,
    State,
    Resume,
    Session,
    Compact,
    Export,
    Version,
    SystemPrompt,
    DumpManifests,
    BootstrapPlan,
    // #720: subsystem help topics so `claw help agents` etc. route to usage JSON
    Agents,
    Skills,
    Plugins,
    Mcp,
    Config,
    Model,
    Settings,
    Diff,
    Setup,
}

#[allow(clippy::too_many_lines)]
pub(crate) fn parse_args(args: &[String]) -> Result<CliAction, String> {
    let mut model = DEFAULT_MODEL.to_string();
    // #148: when user passes --model/--model=, capture the raw input so we
    // can attribute source: "flag" later. None means no flag was supplied.
    let mut model_flag_raw: Option<String> = None;
    let mut output_format_selection = if cli_has_output_format_flag(args) {
        OutputFormatSelection::default()
    } else {
        output_format_selection_from_env()?
    };
    set_current_output_format_selection(&output_format_selection);
    let mut output_format = output_format_selection.format;
    let mut permission_mode_override = None;
    let mut wants_help = false;
    let mut wants_version = false;
    let mut allowed_tool_values = Vec::new();
    let mut compact = false;
    let mut base_commit: Option<String> = None;
    let mut reasoning_effort: Option<String> = None;
    let mut allow_broad_cwd = false;

    // #755: -p prompt text captured as single token; remaining args continue
    // flag parsing. None until `-p <text>` is seen.
    let mut short_p_prompt: Option<String> = None;
    let mut rest: Vec<String> = Vec::new();
    let mut positional_after_separator = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--help" | "-h" if rest.is_empty() => {
                wants_help = true;
                index += 1;
            }
            "--help" | "-h"
                if !rest.is_empty()
                    && matches!(rest[0].as_str(), "prompt" | "commit" | "pr" | "issue") =>
            {
                // `--help` following a subcommand that would otherwise forward
                // the arg to the API (e.g. `claw prompt --help`) should show
                // top-level help instead. Subcommands that consume their own
                // args (agents, mcp, plugins, skills) and local help-topic
                // subcommands (status, sandbox, doctor, init, state, export,
                // version, system-prompt, dump-manifests, bootstrap-plan) must
                // NOT be intercepted here — they handle --help in their own
                // dispatch paths via parse_local_help_action(). See #141.
                wants_help = true;
                index += 1;
            }
            "--version" | "-V" => {
                wants_version = true;
                index += 1;
            }
            "--model" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing_flag_value: missing value for --model.\nUsage: --model <provider/model>  e.g. --model anthropic/claude-opus-4-7".to_string())?;
                // #468: track duplicate --model flags
                if model_flag_raw.is_some() {
                    push_duplicate_flag(&format!(
                        "--model (previous: {}, new: {})",
                        model_flag_raw.as_deref().unwrap_or(""),
                        value
                    ));
                }
                let resolved = resolve_model_alias_with_config(value);
                debug!("Resolved --model '{}' -> '{}'", value, resolved);
                validate_model_syntax(&resolved)?;
                model = resolved;
                model_flag_raw = Some(value.clone()); // #148
                index += 2;
            }

            flag if flag.starts_with("--model=") => {
                let value = &flag[8..];
                let resolved = resolve_model_alias_with_config(value);
                debug!("Resolved --model='{}' -> '{}'", value, resolved);
                validate_model_syntax(&resolved)?;
                model = resolved;
                model_flag_raw = Some(value.to_string()); // #148
                index += 1;
            }
            "--output-format" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing_flag_value: missing value for --output-format.\nUsage: --output-format text  or  --output-format json".to_string())?;
                // #468: track duplicate --output-format flags
                if output_format != CliOutputFormat::Text
                    || output_format_selection.format != CliOutputFormat::Text
                {
                    push_duplicate_flag("--output-format (overwriting previous value)");
                }
                output_format = apply_output_format_flag(&mut output_format_selection, value)?;
                index += 2;
            }
            "--permission-mode" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing_flag_value: missing value for --permission-mode.\nUsage: --permission-mode read-only|workspace-write|danger-full-access".to_string())?;
                // #468: track duplicate --permission-mode flags
                if permission_mode_override.is_some() {
                    push_duplicate_flag("--permission-mode (overwriting previous value)");
                }
                permission_mode_override = Some(parse_permission_mode_arg(value)?);
                index += 2;
            }

            flag if flag.starts_with("--output-format=") => {
                output_format =
                    apply_output_format_flag(&mut output_format_selection, &flag[16..])?;
                index += 1;
            }
            flag if flag.starts_with("--permission-mode=") => {
                permission_mode_override = Some(parse_permission_mode_arg(&flag[18..])?);
                index += 1;
            }
            "--dangerously-skip-permissions" | "--skip-permissions" => {
                permission_mode_override = Some(PermissionMode::DangerFullAccess);
                index += 1;
            }
            "--compact" => {
                compact = true;
                index += 1;
            }
            "--base-commit" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing_flag_value: missing value for --base-commit.\nUsage: --base-commit <git-sha>".to_string())?;
                // #122: validate that base-commit looks like a git SHA (hex, 7-64 chars)
                if value.len() < 7
                    || value.len() > 64
                    || !value.chars().all(|c| c.is_ascii_hexdigit())
                {
                    return Err(format!(
                        "invalid_flag_value: --base-commit expects a hex SHA (7-64 chars), got '{}'.\nUsage: --base-commit <git-sha>",
                        value
                    ));
                }
                base_commit = Some(value.clone());
                index += 2;
            }
            flag if flag.starts_with("--base-commit=") => {
                base_commit = Some(flag[14..].to_string());
                index += 1;
            }
            "--reasoning-effort" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing_flag_value: missing value for --reasoning-effort.\nUsage: --reasoning-effort low|medium|high".to_string())?;
                if !matches!(value.as_str(), "low" | "medium" | "high") {
                    return Err(format!(
                        "invalid_flag_value: invalid value for --reasoning-effort: '{value}'.\nUsage: --reasoning-effort low|medium|high"
                    ));
                }
                reasoning_effort = Some(value.clone());
                index += 2;
            }
            flag if flag.starts_with("--reasoning-effort=") => {
                let value = &flag[19..];
                if !matches!(value, "low" | "medium" | "high") {
                    return Err(format!(
                        "invalid_flag_value: invalid value for --reasoning-effort: '{value}'.\nUsage: --reasoning-effort low|medium|high"
                    ));
                }
                reasoning_effort = Some(value.to_string());
                index += 1;
            }
            "--allow-broad-cwd" => {
                allow_broad_cwd = true;
                index += 1;
            }
            "--" => {
                if rest.is_empty() {
                    positional_after_separator = true;
                    rest.extend(args[index + 1..].iter().cloned());
                } else {
                    rest.push("--".to_string());
                    rest.extend(args[index + 1..].iter().cloned());
                }
                break;
            }
            "-p" => {
                // Claw Code compat: -p "prompt" = one-shot prompt.
                // #755: consume exactly one token so subsequent flags like
                // --model/--output-format are parsed normally instead of
                // being swallowed into the prompt string (#117).
                let next = args.get(index + 1).map(|s| s.as_str());
                match next {
                    None | Some("") => {
                        return Err("missing_prompt: -p requires a prompt string.\nUsage: claw -p <text>  or  claw prompt <text>".to_string());
                    }
                    Some(tok) if tok.starts_with('-') && tok != "--" => {
                        // Looks like a flag, not a prompt. Reject so the user
                        // knows to quote the literal text or use `--`.
                        return Err(format!(
                            "missing_prompt: -p requires a prompt string before flags; got `{tok}`.\nUsage: claw -p <text> --model sonnet  or  claw -p -- {tok} (literal)"
                        ));
                    }
                    Some(tok) => {
                        // `--` sentinel: skip it and take the token after as literal
                        let (prompt_text, skip) = if tok == "--" {
                            match args.get(index + 2) {
                                Some(t) => (t.as_str(), 3usize),
                                None => return Err("missing_prompt: -p -- requires a prompt string after `--`.\nUsage: claw -p -- <text>".to_string()),
                            }
                        } else {
                            (tok, 2usize)
                        };
                        if prompt_text.trim().is_empty() {
                            return Err("missing_prompt: -p requires a non-empty prompt string.\nUsage: claw -p <text>  or  claw prompt <text>".to_string());
                        }
                        short_p_prompt = Some(prompt_text.to_string());
                        index += skip;
                        continue;
                    }
                }
            }
            "--print" => {
                // Claw Code compat: --print makes output non-interactive
                output_format = CliOutputFormat::Text;
                index += 1;
            }
            "--resume" if rest.is_empty() => {
                rest.push("--resume".to_string());
                index += 1;
            }
            // #457: --help after --resume should show resume help, not be consumed as session-id
            "--help" | "-h" if rest.first().map(String::as_str) == Some("--resume") => {
                wants_help = true;
                index += 1;
            }
            flag if rest.is_empty() && flag.starts_with("--resume=") => {
                rest.push("--resume".to_string());
                rest.push(flag[9..].to_string());
                index += 1;
            }
            "--acp" | "-acp" => {
                rest.push("acp".to_string());
                index += 1;
            }
            "--allowedTools" | "--allowed-tools" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(allowed_tools_missing_error)?;
                if value.starts_with('-') || is_known_top_level_subcommand(value) {
                    return Err(allowed_tools_missing_error());
                }
                allowed_tool_values.push(value.clone());
                index += 2;
            }
            flag if flag.starts_with("--allowedTools=") => {
                let value = flag[15..].to_string();
                if value.trim().is_empty() {
                    return Err(allowed_tools_missing_error());
                }
                allowed_tool_values.push(value);
                index += 1;
            }
            flag if flag.starts_with("--allowed-tools=") => {
                let value = flag[16..].to_string();
                if value.trim().is_empty() {
                    return Err(allowed_tools_missing_error());
                }
                allowed_tool_values.push(value);
                index += 1;
            }
            other if rest.is_empty() && other.starts_with('-') => {
                if should_reject_unknown_option_like(other) {
                    return Err(format_unknown_option(other));
                }
                rest.push(other.to_string());
                index += 1;
            }
            other => {
                rest.push(other.to_string());
                index += 1;
            }
        }
    }

    if wants_help {
        // #684: --help before subcommand should still route to subcommand-specific
        // help when the subcommand is one of the local-help-topic commands.
        if let Some(action) = parse_local_help_action(&rest, output_format) {
            return action;
        }
        // When --help was consumed before the subcommand, rest has no help flag.
        // If rest is a simple local-help subcommand with no extra args, route there.
        if !rest.is_empty() && rest[1..].iter().all(|a| is_help_flag(a)) {
            let topic = match rest[0].as_str() {
                "status" => Some(LocalHelpTopic::Status),
                "sandbox" => Some(LocalHelpTopic::Sandbox),
                "doctor" => Some(LocalHelpTopic::Doctor),
                "acp" => Some(LocalHelpTopic::Acp),
                "init" => Some(LocalHelpTopic::Init),
                "setup" => Some(LocalHelpTopic::Setup),
                "state" => Some(LocalHelpTopic::State),
                "resume" => Some(LocalHelpTopic::Resume),
                "session" => Some(LocalHelpTopic::Session),
                "compact" => Some(LocalHelpTopic::Compact),
                "--resume" => Some(LocalHelpTopic::Resume),
                "export" => Some(LocalHelpTopic::Export),
                "version" => Some(LocalHelpTopic::Version),
                "system-prompt" => Some(LocalHelpTopic::SystemPrompt),
                "dump-manifests" => Some(LocalHelpTopic::DumpManifests),
                "bootstrap-plan" => Some(LocalHelpTopic::BootstrapPlan),
                "agents" | "agent" => Some(LocalHelpTopic::Agents),
                "skills" | "skill" => Some(LocalHelpTopic::Skills),
                "plugins" | "plugin" | "marketplace" => Some(LocalHelpTopic::Plugins),
                "mcp" => Some(LocalHelpTopic::Mcp),
                "config" => Some(LocalHelpTopic::Config),
                "model" | "models" => Some(LocalHelpTopic::Model),
                "settings" => Some(LocalHelpTopic::Settings),
                "diff" => Some(LocalHelpTopic::Diff),
                _ => None,
            };
            if let Some(topic) = topic {
                return Ok(CliAction::HelpTopic {
                    topic,
                    output_format,
                });
            }
        }
        return Ok(CliAction::Help { output_format });
    }

    if wants_version {
        return Ok(CliAction::Version { output_format });
    }

    let allowed_tools = normalize_allowed_tools(&allowed_tool_values)?;

    // #755: -p consumed exactly one token; dispatch now that all flags are parsed
    if let Some(prompt) = short_p_prompt {
        return Ok(CliAction::Prompt {
            prompt,
            model: resolve_model_alias_with_config(&model),
            output_format,
            allowed_tools,
            permission_mode: permission_mode_override.unwrap_or_else(default_permission_mode),
            compact,
            base_commit,
            reasoning_effort,
            allow_broad_cwd,
        });
    }

    if positional_after_separator && !rest.is_empty() {
        let permission_mode = permission_mode_override.unwrap_or_else(default_permission_mode);
        return Ok(CliAction::Prompt {
            prompt: rest.join(" "),
            model,
            output_format,
            allowed_tools,
            permission_mode,
            compact,
            base_commit,
            reasoning_effort: reasoning_effort.clone(),
            allow_broad_cwd,
        });
    }

    if rest.is_empty() {
        let permission_mode = permission_mode_override.unwrap_or_else(default_permission_mode);
        let stdin_is_terminal = std::io::stdin().is_terminal();
        if compact && stdin_is_terminal {
            return Err(compact_missing_argument_error());
        }
        // When stdin is not a terminal (pipe/redirect) and no prompt is given on the
        // command line, read stdin as the prompt and dispatch as a one-shot Prompt
        // rather than starting the interactive REPL (which would consume the pipe and
        // print the startup banner, then exit without sending anything to the API).
        if !stdin_is_terminal {
            let mut buf = String::new();
            let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf);
            let piped = buf.trim().to_string();
            if !piped.is_empty() {
                return Ok(CliAction::Prompt {
                    model,
                    prompt: piped,
                    allowed_tools,
                    permission_mode,
                    output_format,
                    compact,
                    base_commit,
                    reasoning_effort,
                    allow_broad_cwd,
                });
            }
            if compact {
                return Err(compact_missing_argument_error());
            }
            // Non-TTY stdin with no piped content: refuse to start the interactive
            // REPL (it would block forever waiting for input that will never arrive).
            // (#696: emit a typed error instead of hanging indefinitely)
            // Skip this guard in test builds (parse_args tests run in non-TTY context).
            #[cfg(not(test))]
            // #746: newline before remediation so split_error_hint populates hint field
            return Err("interactive_only: claw requires an interactive terminal.\nStdin is not a TTY and no prompt was provided — pipe a prompt with `echo 'task' | claw` or run `claw` in an interactive terminal.".into());
        }
        return Ok(CliAction::Repl {
            model,
            allowed_tools,
            permission_mode,
            base_commit,
            reasoning_effort: reasoning_effort.clone(),
            allow_broad_cwd,
        });
    }
    if let Some(action) = parse_local_help_action(&rest, output_format) {
        return action;
    }
    if rest.first().map(String::as_str) == Some("--resume") {
        return parse_resume_args(&rest[1..], output_format, allow_broad_cwd);
    }
    if rest.first().map(String::as_str) == Some("resume") {
        return parse_resume_args(&rest[1..], output_format, allow_broad_cwd);
    }
    // #696: `claw compact` is the bare name of the interactive `/compact`
    // slash command, not a prompt. When extra args such as `--help` appear
    // after the word `compact`, the generic prompt fallback used to send
    // `compact --help` to provider startup and could hang under closed stdin /
    // JSON output. Fail closed before any provider, prompt, TUI, or spinner
    // startup. `claw --resume SESSION.jsonl /compact` remains the supported
    // non-interactive session compaction path.
    if rest.first().map(String::as_str) == Some("compact") {
        return Err(compact_interactive_only_error());
    }
    if let Some(action) = parse_single_word_command_alias(
        &rest,
        &model,
        model_flag_raw.as_deref(),
        permission_mode_override,
        output_format,
        allowed_tools.clone(),
    ) {
        return action;
    }

    // Keep config-backed defaults lazy so pure-local JSON surfaces (notably
    // `claw --output-format json config`) can report config warnings
    // structurally without an earlier default-resolution load writing prose
    // warnings to stderr.
    let permission_mode = || permission_mode_override.unwrap_or_else(default_permission_mode);
    let permission_mode_provenance = || {
        permission_mode_override
            .map(PermissionModeProvenance::from_flag)
            .unwrap_or_else(permission_mode_provenance_for_current_dir)
    };

    // #98: --compact is only meaningful for prompt mode. When a known non-prompt
    // subcommand is being dispatched, reject --compact so callers don't silently
    // lose the flag.
    if compact
        && rest
            .first()
            .map(|s| s.as_str())
            .is_some_and(|s| s != "prompt")
    {
        // Allow compact for the default prompt fallback (unknown tokens).
        // Only reject for known top-level subcommands that don't use compact.
        let first = rest[0].as_str();
        if is_known_top_level_subcommand(first) && first != "prompt" {
            return Err(format!(
                "invalid_flag_value: --compact is only supported with prompt mode.\nUsage: claw --compact \"<prompt>\" or echo \"<prompt>\" | claw --compact"
            ));
        }
    }

    match rest[0].as_str() {
        "dump-manifests" => parse_dump_manifests_args(&rest[1..], output_format),
        "bootstrap-plan" => Ok(CliAction::BootstrapPlan { output_format }),
        "agents" => Ok(CliAction::Agents {
            args: join_optional_args(&rest[1..]),
            output_format,
        }),
        "mcp" => Ok(CliAction::Mcp {
            args: join_optional_args(&rest[1..]),
            output_format,
        }),
        // #145: `plugins` was routed through the prompt fallback because no
        // top-level parser arm produced CliAction::Plugins. That made `claw
        // plugins` (and `claw plugins --help`, `claw plugins list`, ...)
        // attempt an Anthropic network call, surfacing the misleading error
        // `missing Anthropic credentials` even though the command is purely
        // local introspection. Mirror `agents`/`mcp`/`skills`: action is the
        // first positional arg, target is the second.
        // `plugin` (singular) and `marketplace` are aliases for `plugins`.
        // All three must route to the same local handler so that no form
        // falls through to the LLM/prompt path.
        "plugins" | "plugin" | "marketplace" => {
            let tail = &rest[1..];
            let action = tail.first().cloned();
            let target = tail.get(1).cloned();
            if tail.len() > 2 {
                // #797: append \n usage hint so split_error_hint extracts it (parity with #791 config fix)
                return Err(format!(
                    "unexpected extra arguments after `claw {} {}`: {}\nUsage: claw plugins [list|show <id>|install <id>|enable <id>|disable <id>|uninstall <id>|update <id>|help]",
                    rest[0],
                    tail[..2].join(" "),
                    tail[2..].join(" ")
                ));
            }
            Ok(CliAction::Plugins {
                action,
                target,
                output_format,
            })
        }
        // #146: `config` is pure-local read-only introspection (merges
        // `.claw.json` + `.claw/settings.json` from disk, no network, no
        // state mutation). Previously callers had to spin up a session with
        // `claw --resume SESSION.jsonl /config` to see their own config,
        // which is synthetic friction. Accepts an optional section name
        // (env|hooks|model|plugins) matching the slash command shape.
        "config" => {
            let tail = &rest[1..];
            let section = tail.first().cloned();
            if tail.len() > 1 {
                // #791: append \n hint so split_error_hint extracts it and hint is non-null
                return Err(format!(
                    "unexpected extra arguments after `claw config {}`: {}\nUsage: claw config [env|hooks|model|plugins|mcp|settings]",
                    tail[0],
                    tail[1..].join(" ")
                ));
            }
            Ok(CliAction::Config {
                section,
                output_format,
            })
        }
        // #146: `diff` is pure-local (shells out to `git diff --cached` +
        // `git diff`). No session needed to inspect the working tree.
        "diff" => {
            if rest.len() > 1 {
                // #3129: keep malformed `diff ... --output-format json` on the
                // parser/error path, not the prompt/TUI fallback. The newline
                // before Usage is part of the JSON hint contract.
                return Err(unexpected_diff_args_error(&rest[1..]));
            }
            Ok(CliAction::Diff { output_format })
        }
        // `claw permissions <mode>` falls through to the LLM when called
        // with a subcommand argument because parse_single_word_command_alias
        // only intercepts the bare single-word form. Catch all multi-word
        // forms here and return a structured guidance error so no network
        // call or session is created.
        "permissions" => Err(
            "`claw permissions` is a slash command. Start `claw` and run `/permissions` inside the REPL.\n  Usage  /permissions [read-only|workspace-write|danger-full-access]"
                .to_string(),
        ),
        // #767: `claw session bogus` bypassed parse_single_word_command_alias (rest.len()>1),
        // had no match arm, and fell to CliAction::Prompt — reaching the credential gate
        // instead of a structured error. Mirror the guard on `permissions`.
        "session" => {
            // #449: `claw session list` is a pure local filesystem read that
            // requires no API credentials. Route directly to SessionList instead
            // of falling through to the resume/auth path.
            if rest.get(1).map(|s| s.as_str()) == Some("list") {
                Ok(CliAction::SessionList { output_format })
            } else {
                let action_hint = rest.get(1).map_or(String::new(), |a| format!(" (got: `{a}`)" ));
                Err(format!(
                    "interactive_only: `claw session` is a slash command{action_hint}.\nUse `claw --resume SESSION.jsonl /session <action>` or start `claw` and run `/session [list|exists|switch|fork|delete]`."
                ))
            }
        }
        // #770: same fallthrough gap as #767 — these slash commands had no multi-arg match arm
        // and fell to CliAction::Prompt reaching the credential gate when called with args.
        "cost" => Err(
            "interactive_only: `claw cost` is a slash command.\nUse `claw --resume SESSION.jsonl /cost` or start `claw` and run `/cost`."
                .to_string(),
        ),
        "clear" => Err(
            "interactive_only: `claw clear` is a slash command.\nUse `claw --resume SESSION.jsonl /clear [--confirm]` or start `claw` and run `/clear`."
                .to_string(),
        ),
        "memory" => Err(
            "interactive_only: `claw memory` is a slash command.\nStart `claw` and run `/memory` inside the REPL."
                .to_string(),
        ),
        "ultraplan" => Err(
            "interactive_only: `claw ultraplan` is a slash command.\nStart `claw` and run `/ultraplan` inside the REPL."
                .to_string(),
        ),
        "model" | "models" => {
            let tail = &rest[1..];
            let action = tail.first().cloned();
            if tail.len() > 1 {
                return Err(format!(
                    "unexpected extra arguments after `claw {} {}`: {}\nUsage: claw {} [help] [--output-format json]",
                    rest[0],
                    tail[0],
                    tail[1..].join(" "),
                    rest[0]
                ));
            }
            Ok(CliAction::Models {
                action,
                output_format,
            })
        }
        // #771: usage/stats/fork are slash-only verbs with no multi-arg match arms
        "usage" => Err(
            "interactive_only: `claw usage` is a slash command.\nUse `claw --resume SESSION.jsonl /usage` or start `claw` and run `/usage`."
                .to_string(),
        ),
        "stats" => Err(
            "interactive_only: `claw stats` is a slash command.\nUse `claw --resume SESSION.jsonl /stats` or start `claw` and run `/stats`."
                .to_string(),
        ),
        "fork" => Err(
            "interactive_only: `claw fork` is a slash command.\nStart `claw` and run `/session fork [branch-name]` inside the REPL."
                .to_string(),
        ),
        "skills" => {
            let args = join_optional_args(&rest[1..]);
            if let Some(action) = args.as_deref() {
                let first_word = action.split_whitespace().next().unwrap_or(action);
                if matches!(first_word, "add") {
                    return Err(format!(
                        "unsupported skills action: {first_word}. Supported actions: list, show <name>, install <path>, uninstall <name>, help, or <skill> [args]"
                    ));
                }
            }
            match classify_skills_slash_command(args.as_deref()) {
                SkillSlashDispatch::Invoke(prompt) => Ok(CliAction::Prompt {
                    prompt,
                    model,
                    output_format,
                    allowed_tools,
                    permission_mode: permission_mode(),
                    compact,
                    base_commit,
                    reasoning_effort: reasoning_effort.clone(),
                    allow_broad_cwd,
                }),
                SkillSlashDispatch::Local => Ok(CliAction::Skills {
                    args,
                    output_format,
                }),
            }
        }
        "settings" => {
            let tail = &rest[1..];
            if tail.is_empty() {
                Ok(CliAction::Config {
                    section: Some("settings".to_string()),
                    output_format,
                })
            } else if tail.len() == 1 && matches!(tail[0].as_str(), "help" | "--help" | "-h") {
                Ok(CliAction::HelpTopic {
                    topic: LocalHelpTopic::Settings,
                    output_format,
                })
            } else {
                Err(format!(
                    "unexpected extra arguments after `claw settings`: {}\nUsage: claw settings [help] [--output-format json]",
                    tail.join(" ")
                ))
            }
        }
        "system-prompt" => parse_system_prompt_args(&rest[1..], model, output_format),
        "acp" => parse_acp_args(&rest[1..], output_format),
        "login" | "logout" => Err(removed_auth_surface_error(rest[0].as_str())),
        "init" => {
            // #771: extra positional args to `init` were silently ignored — now rejected
            if rest.len() > 1 {
                let extra = rest[1..].join(" ");
                return Err(format!(
                    "unexpected extra arguments after `claw init`: {extra}\nUsage: claw init [--cwd <dir>] [--date <date>] [--session <session-id>]"
                ));
            }
            Ok(CliAction::Init { output_format })
        }
        "setup" => {
            if rest.len() > 1 {
                let extra = rest[1..].join(" ");
                return Err(format!(
                    "unexpected extra arguments after `claw setup`: {extra}\nUsage: claw setup"
                ));
            }
            Ok(CliAction::Setup { output_format })
        }
        "export" => parse_export_args(&rest[1..], output_format),
        "prompt" => {
            let mut read_stdin = false;
            let prompt_parts = rest[1..]
                .iter()
                .filter_map(|arg| {
                    if matches!(arg.as_str(), "--stdin" | "--prompt-stdin") {
                        read_stdin = true;
                        None
                    } else {
                        Some(arg.as_str())
                    }
                })
                .collect::<Vec<_>>();
            let positional_prompt = prompt_parts.join(" ");
            let stdin_prompt = if read_stdin || positional_prompt.trim().is_empty() {
                read_piped_stdin()
            } else {
                None
            };
            let prompt = if read_stdin {
                merge_prompt_with_stdin(&positional_prompt, stdin_prompt.as_deref())
            } else {
                stdin_prompt
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or(&positional_prompt)
                    .to_string()
            };
            if prompt.trim().is_empty() {
                // #750/#823/#423: provide error_kind-compatible prefix + \n for hint extraction.
                return Err("missing_prompt: prompt subcommand requires a prompt string.
Usage: claw prompt <text>  or  echo '<text>' | claw prompt".to_string());
            }
            Ok(CliAction::Prompt {
                prompt,
                model,
                output_format,
                allowed_tools,
                permission_mode: permission_mode(),
                compact,
                base_commit: base_commit.clone(),
                reasoning_effort: reasoning_effort.clone(),
                allow_broad_cwd,
            })
        }
        other if other.starts_with('/') => parse_direct_slash_cli_action(
            &rest,
            model,
            output_format,
            allowed_tools,
            permission_mode_provenance(),
            compact,
            base_commit,
            reasoning_effort,
            allow_broad_cwd,
        ),
        other => {
            if !compact
                && !other.starts_with('-')
                && looks_like_subcommand_typo(other)
                && (rest.len() == 1
                    || (output_format == CliOutputFormat::Json && model_flag_raw.is_none()))
            {
                // #825/#826: emit command_not_found before provider startup for
                // command-shaped tokens that do not match known subcommands.
                // Text-mode multi-word prompt shorthand remains available, but
                // JSON-mode automation must not turn an unknown command into a
                // credential-gated prompt request.
                let mut message = format!("command_not_found: unknown subcommand: {other}.");
                if let Some(suggestions) = suggest_similar_subcommand(other) {
                    if let Some(line) = render_suggestion_line("Did you mean", &suggestions) {
                        message.push('\n');
                        message.push_str(&line);
                    }
                }
                message.push_str(
                    "\nRun `claw --help` for the full list. If you meant to send a prompt literally, use `claw prompt <text>`.",
                );
                return Err(message);
            }
            // #147: guard empty/whitespace-only prompts at the fallthrough
            // path the same way `"prompt"` arm above does. Without this,
            // `claw ""`, `claw "   "`, and `claw "" ""` silently route to
            // the Anthropic call and surface a misleading
            // `missing Anthropic credentials` error (or burn API tokens on
            // an empty prompt when credentials are present).
            let joined = rest.join(" ");
            if joined.trim().is_empty() {
                // #798: add \n hint so split_error_hint extracts it (was empty_prompt + null)
                return Err(
                    "empty prompt: provide a subcommand or a non-empty prompt string.\nUsage: claw <subcommand> or claw -p <prompt>. Run `claw --help` for the full list."
                        .to_string(),
                );
            }
            Ok(CliAction::Prompt {
                prompt: joined,
                model,
                output_format,
                allowed_tools,
                permission_mode: permission_mode(),
                compact,
                base_commit,
                reasoning_effort: reasoning_effort.clone(),
                allow_broad_cwd,
            })
        }
    }
}

pub(crate) fn parse_local_help_action(
    rest: &[String],
    output_format: CliOutputFormat,
) -> Option<Result<CliAction, String>> {
    if rest.is_empty() {
        return None;
    }
    if !rest.iter().any(|a| is_help_flag(a)) {
        return None;
    }

    let topic = match rest[0].as_str() {
        "status" => LocalHelpTopic::Status,
        "sandbox" => LocalHelpTopic::Sandbox,
        "doctor" => LocalHelpTopic::Doctor,
        "acp" => LocalHelpTopic::Acp,
        "init" => LocalHelpTopic::Init,
        "setup" => LocalHelpTopic::Setup,
        "state" => LocalHelpTopic::State,
        "export" => LocalHelpTopic::Export,
        "version" => LocalHelpTopic::Version,
        "system-prompt" => LocalHelpTopic::SystemPrompt,
        "dump-manifests" => LocalHelpTopic::DumpManifests,
        "bootstrap-plan" => LocalHelpTopic::BootstrapPlan,
        "resume" | "--resume" => LocalHelpTopic::Resume,
        "session" => LocalHelpTopic::Session,
        "compact" => LocalHelpTopic::Compact,
        "model" | "models" => LocalHelpTopic::Model,
        "settings" => LocalHelpTopic::Settings,
        _ => return None,
    };
    let has_non_help = rest[1..].iter().any(|a| !is_help_flag(a));
    if has_non_help {
        return None;
    }
    Some(Ok(CliAction::HelpTopic {
        topic,
        output_format,
    }))
}

pub(crate) fn is_help_flag(value: &str) -> bool {
    matches!(value, "--help" | "-h")
}

pub(crate) fn parse_single_word_command_alias(
    rest: &[String],
    model: &str,
    // #148: raw --model flag input for status provenance. None = no flag.
    model_flag_raw: Option<&str>,
    permission_mode_override: Option<PermissionMode>,
    output_format: CliOutputFormat,
    allowed_tools: Option<AllowedToolSet>,
) -> Option<Result<CliAction, String>> {
    if rest.is_empty() {
        return None;
    }

    // Diagnostic verbs (help, version, status, sandbox, doctor, state) accept only the verb itself
    // or --help / -h as a suffix. Any other suffix args are unrecognized.
    let verb = &rest[0];
    let is_diagnostic = matches!(
        verb.as_str(),
        "help" | "version" | "status" | "sandbox" | "doctor" | "setup" | "state"
    );

    if is_diagnostic && rest.len() > 1 {
        // Diagnostic verb with trailing args: reject unrecognized suffix
        let all_extra_are_help = rest[1..].iter().all(|a| is_help_flag(a));
        if all_extra_are_help {
            // "doctor --help -h" is valid, routed to parse_local_help_action() instead
            return None;
        }
        // #720: `claw help <topic>` — when the verb is "help" and exactly one
        // non-flag argument follows, try to route to the topic's handler.
        if verb == "help" && rest.len() == 2 {
            let topic_name = rest[1].as_str();
            let topic = match topic_name {
                "status" => Some(LocalHelpTopic::Status),
                "sandbox" => Some(LocalHelpTopic::Sandbox),
                "doctor" => Some(LocalHelpTopic::Doctor),
                "acp" => Some(LocalHelpTopic::Acp),
                "init" => Some(LocalHelpTopic::Init),
                "setup" => Some(LocalHelpTopic::Setup),
                "state" => Some(LocalHelpTopic::State),
                "export" => Some(LocalHelpTopic::Export),
                "version" => Some(LocalHelpTopic::Version),
                "system-prompt" => Some(LocalHelpTopic::SystemPrompt),
                "dump-manifests" => Some(LocalHelpTopic::DumpManifests),
                "bootstrap-plan" => Some(LocalHelpTopic::BootstrapPlan),
                "resume" => Some(LocalHelpTopic::Resume),
                "session" => Some(LocalHelpTopic::Session),
                "compact" => Some(LocalHelpTopic::Compact),
                "agents" | "agent" => Some(LocalHelpTopic::Agents),
                "skills" | "skill" => Some(LocalHelpTopic::Skills),
                "plugins" | "plugin" | "marketplace" => Some(LocalHelpTopic::Plugins),
                "mcp" => Some(LocalHelpTopic::Mcp),
                "config" => Some(LocalHelpTopic::Config),
                "model" | "models" => Some(LocalHelpTopic::Model),
                "settings" => Some(LocalHelpTopic::Settings),
                "diff" => Some(LocalHelpTopic::Diff),
                _ => None,
            };
            if let Some(t) = topic {
                return Some(Ok(CliAction::HelpTopic {
                    topic: t,
                    output_format,
                }));
            }
            // Unknown topic: fall through to generic help.
            return Some(Ok(CliAction::Help { output_format }));
        }
        // Unrecognized suffix like "--json"
        let mut msg = format!(
            "unrecognized argument `{}` for subcommand `{}`",
            rest[1], verb
        );
        // #152: common mistake — users type `--json` expecting JSON output.
        // Hint at the correct flag so they don't have to re-read --help.
        if rest[1] == "--json" {
            msg.push_str("\nDid you mean `--output-format json`?");
        } else {
            // #752: generic fallback hint so cli_parse errors always have non-null hint
            msg.push_str(&format!("\nRun `claw {} --help` for usage.", verb));
        }
        return Some(Err(msg));
    }

    // #720: `claw help <topic>` — when `help` is the verb and a topic follows,
    // try to route to the topic's help handler instead of erroring.
    if rest.len() == 2 && rest[0] == "help" {
        let topic_name = rest[1].as_str();
        let topic = match topic_name {
            "status" => Some(LocalHelpTopic::Status),
            "sandbox" => Some(LocalHelpTopic::Sandbox),
            "doctor" => Some(LocalHelpTopic::Doctor),
            "acp" => Some(LocalHelpTopic::Acp),
            "init" => Some(LocalHelpTopic::Init),
            "setup" => Some(LocalHelpTopic::Setup),
            "state" => Some(LocalHelpTopic::State),
            "export" => Some(LocalHelpTopic::Export),
            "version" => Some(LocalHelpTopic::Version),
            "system-prompt" => Some(LocalHelpTopic::SystemPrompt),
            "dump-manifests" => Some(LocalHelpTopic::DumpManifests),
            "bootstrap-plan" => Some(LocalHelpTopic::BootstrapPlan),
            "resume" => Some(LocalHelpTopic::Resume),
            "session" => Some(LocalHelpTopic::Session),
            "compact" => Some(LocalHelpTopic::Compact),
            "agents" | "agent" => Some(LocalHelpTopic::Agents),
            "skills" | "skill" => Some(LocalHelpTopic::Skills),
            "plugins" | "plugin" | "marketplace" => Some(LocalHelpTopic::Plugins),
            "mcp" => Some(LocalHelpTopic::Mcp),
            "config" => Some(LocalHelpTopic::Config),
            "model" | "models" => Some(LocalHelpTopic::Model),
            "settings" => Some(LocalHelpTopic::Settings),
            "diff" => Some(LocalHelpTopic::Diff),
            _ => None,
        };
        if let Some(t) = topic {
            return Some(Ok(CliAction::HelpTopic {
                topic: t,
                output_format,
            }));
        }
        // Unknown topic falls through to the generic help action.
        return Some(Ok(CliAction::Help { output_format }));
    }

    // #453: fire guard for multi-word CLI subcommands too (claw cost list, claw model list, etc.)
    // For slash commands that are commonly used as prompts (explain, cost, tokens, etc.),
    // only fire the guard when there's exactly one token.
    if rest.is_empty() {
        return None;
    }
    // Known CLI subcommands that don't accept additional arguments
    const CLI_SUBCOMMANDS: &[&str] = &[
        "help", "version", "status", "sandbox", "doctor", "state", "config", "diff",
    ];
    if rest.len() > 1 && !CLI_SUBCOMMANDS.contains(&rest[0].as_str()) {
        return None;
    }

    match rest[0].as_str() {
        "help" => Some(Ok(CliAction::Help { output_format })),
        "version" => Some(Ok(CliAction::Version { output_format })),
        "status" => Some(Ok(CliAction::Status {
            model: model.to_string(),
            model_flag_raw: model_flag_raw.map(str::to_string), // #148
            permission_mode: permission_mode_override
                .map(PermissionModeProvenance::from_flag)
                .unwrap_or_else(permission_mode_provenance_for_current_dir),
            output_format,
            allowed_tools,
        })),
        "sandbox" => Some(Ok(CliAction::Sandbox { output_format })),
        "doctor" => Some(Ok(CliAction::Doctor {
            output_format,
            permission_mode: permission_mode_override
                .map(PermissionModeProvenance::from_flag)
                .unwrap_or_else(permission_mode_provenance_for_current_dir),
        })),
        "setup" => Some(Ok(CliAction::Setup { output_format })),
        "state" => Some(Ok(CliAction::State { output_format })),
        // #146: let `config` and `diff` fall through to parse_subcommand
        // where they are wired as pure-local introspection, instead of
        // producing the "is a slash command" guidance. Zero-arg cases
        // reach parse_subcommand too via this None.
        "config" | "diff" => None,
        other => bare_slash_command_guidance(other).map(Err),
    }
}

pub(crate) fn bare_slash_command_guidance(command_name: &str) -> Option<String> {
    if matches!(
        command_name,
        "dump-manifests"
            | "bootstrap-plan"
            | "agents"
            | "mcp"
            | "plugin"
            | "plugins"
            | "marketplace"
            | "skills"
            | "system-prompt"
            | "init"
            | "prompt"
            | "export"
    ) {
        return None;
    }
    let slash_command = slash_command_specs()
        .iter()
        // #772: check both spec.name and spec.aliases for command-line invocations
        .find(|spec| spec.name == command_name || spec.aliases.contains(&command_name))?;
    let canonical_name = slash_command.name;
    // #745: newline before remediation text so split_error_hint populates hint field
    let guidance = if slash_command.resume_supported {
        format!(
            "`claw {command_name}` is a slash command.\nUse `claw --resume SESSION.jsonl /{canonical_name}` or start `claw` and run `/{canonical_name}`."
        )
    } else {
        format!(
            "`claw {command_name}` is a slash command.\nStart `claw` and run `/{canonical_name}` inside the REPL."
        )
    };
    // #772: help text still mentions the alias, but the remediation shows canonical form
    Some(guidance)
}

pub(crate) fn removed_auth_surface_error(command_name: &str) -> String {
    // #765: two-line format so split_error_hint() extracts hint into JSON envelope
    format!(
        "`claw {command_name}` has been removed.\nSet ANTHROPIC_API_KEY or ANTHROPIC_AUTH_TOKEN instead."
    )
}

pub(crate) fn parse_acp_args(
    args: &[String],
    output_format: CliOutputFormat,
) -> Result<CliAction, String> {
    match args {
        [] => Ok(CliAction::Acp { output_format }),
        [subcommand] if subcommand == "serve" => Ok(CliAction::Acp { output_format }),
        _ => Err(String::from(
            "unsupported_acp_invocation: unsupported ACP invocation. Use `claw acp` or `claw acp serve`.\nACP/Zed editor integration is not implemented yet; `claw acp serve` reports status only.",
        )),
    }
}

pub(crate) fn join_optional_args(args: &[String]) -> Option<String> {
    let joined = args.join(" ");
    let trimmed = joined.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
pub(crate) fn parse_direct_slash_cli_action(
    rest: &[String],
    model: String,
    output_format: CliOutputFormat,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionModeProvenance,
    compact: bool,
    base_commit: Option<String>,
    reasoning_effort: Option<String>,
    allow_broad_cwd: bool,
) -> Result<CliAction, String> {
    let raw = rest.join(" ");
    match SlashCommand::parse(&raw) {
        Ok(Some(SlashCommand::Help)) => Ok(CliAction::Help { output_format }),
        Ok(Some(SlashCommand::Status)) => Ok(CliAction::Status {
            model,
            model_flag_raw: None,
            permission_mode,
            output_format,
            allowed_tools,
        }),
        Ok(Some(SlashCommand::Sandbox)) => Ok(CliAction::Sandbox { output_format }),
        Ok(Some(SlashCommand::Diff)) => Ok(CliAction::Diff { output_format }),
        Ok(Some(SlashCommand::Version)) => Ok(CliAction::Version { output_format }),
        Ok(Some(SlashCommand::Doctor)) => Ok(CliAction::Doctor {
            output_format,
            permission_mode,
        }),
        Ok(Some(SlashCommand::Agents { args })) => Ok(CliAction::Agents {
            args,
            output_format,
        }),
        Ok(Some(SlashCommand::Mcp { action, target })) => Ok(CliAction::Mcp {
            args: match (action, target) {
                (None, None) => None,
                (Some(action), None) => Some(action),
                (Some(action), Some(target)) => Some(format!("{action} {target}")),
                (None, Some(target)) => Some(target),
            },
            output_format,
        }),
        Ok(Some(SlashCommand::Skills { args })) => {
            match classify_skills_slash_command(args.as_deref()) {
                SkillSlashDispatch::Invoke(prompt) => Ok(CliAction::Prompt {
                    prompt,
                    model,
                    output_format,
                    allowed_tools,
                    permission_mode: permission_mode.mode,
                    compact,
                    base_commit,
                    reasoning_effort: reasoning_effort.clone(),
                    allow_broad_cwd,
                }),
                SkillSlashDispatch::Local => Ok(CliAction::Skills {
                    args,
                    output_format,
                }),
            }
        }
        Ok(Some(SlashCommand::Unknown(name))) => {
            // #828: /approve and /deny are valid REPL-only slash commands that
            // are not SlashCommand enum variants (they require an active tool
            // call in the REPL to be meaningful). Emit interactive_only so
            // machine consumers see the correct error_kind instead of
            // unknown_slash_command.
            if matches!(name.as_str(), "approve" | "yes" | "y" | "deny" | "no" | "n") {
                Err(format!(
                    "interactive_only: /{name} requires an active tool call in the REPL.\nStart `claw` and use /{name} to approve or deny a pending tool execution."
                ))
            } else {
                Err(format_unknown_direct_slash_command(&name))
            }
        }
        Ok(Some(command)) => Err({
            let _ = command;
            let command_name = &rest[0];
            // #829: only suggest --resume when the command is actually
            // resume-safe. Non-resume-safe commands (e.g. /commit, /pr)
            // previously suggested --resume, which just re-triggered
            // interactive_only on a second invocation.
            let bare_name = command_name.trim_start_matches('/');
            let is_resume_safe = commands::resume_supported_slash_commands()
                .iter()
                .any(|spec| spec.name == bare_name);
            if is_resume_safe {
                format!(
                    // #738: newline before remediation so split_error_hint populates hint field
                    "interactive_only: slash command {command_name} requires a live session.\nStart `claw` and run it there, or use `claw --resume SESSION.jsonl {command_name}` / `claw --resume {latest} {command_name}`.",
                    latest = LATEST_SESSION_REFERENCE,
                )
            } else {
                format!(
                    "interactive_only: slash command {command_name} requires a live REPL session.\nStart `claw` and run it there."
                )
            }
        }),
        Ok(None) => Err(format!("unknown subcommand: {}", rest[0])),
        Err(error) => Err(error.to_string()),
    }
}

pub(crate) fn format_unknown_option(option: &str) -> String {
    if option == "--" {
        return "end_of_flags: `--` terminates flag parsing. Pass literal prompt text after it, for example `claw -- \"-literal prompt\"`.\nRun `claw --help` for usage.".to_string();
    }
    let mut message = format!("unknown option: {option}");
    if let Some(suggestion) = suggest_closest_term(option, CLI_OPTION_SUGGESTIONS) {
        message.push_str("\nDid you mean ");
        message.push_str(suggestion);
        message.push('?');
    }
    message.push_str("\nRun `claw --help` for usage.");
    message
}

pub(crate) fn format_unknown_direct_slash_command(name: &str) -> String {
    // #827: prefix with classifier-friendly token so classify_error_kind
    // returns "unknown_slash_command" instead of the opaque fallback.
    let mut message =
        format!("unknown_slash_command: unknown slash command outside the REPL: /{name}");
    if let Some(suggestions) = render_suggestion_line("Did you mean", &suggest_slash_commands(name))
    {
        message.push('\n');
        message.push_str(&suggestions);
    }
    if let Some(note) = omc_compatibility_note_for_unknown_slash_command(name) {
        message.push('\n');
        message.push_str(note);
    }
    message.push_str("\nRun `claw --help` for CLI usage, or start `claw` and use /help.");
    message
}

pub(crate) fn format_unknown_slash_command(name: &str) -> String {
    // #827: prefix with classifier-friendly token so classify_error_kind
    // can return "unknown_slash_command" instead of the opaque fallback.
    let mut message = format!("unknown_slash_command: Unknown slash command: /{name}");
    if let Some(suggestions) = render_suggestion_line("Did you mean", &suggest_slash_commands(name))
    {
        message.push('\n');
        message.push_str(&suggestions);
    }
    if let Some(note) = omc_compatibility_note_for_unknown_slash_command(name) {
        message.push('\n');
        message.push_str(note);
    }
    message.push_str("\n  Help             /help lists available slash commands");
    message
}

pub(crate) fn omc_compatibility_note_for_unknown_slash_command(name: &str) -> Option<&'static str> {
    name.starts_with("oh-my-claudecode:")
        .then_some(
            "Compatibility note: `/oh-my-claudecode:*` is a Claude Code/OMC plugin command. `claw` does not yet load plugin slash commands, Claude statusline stdin, or OMC session hooks.",
        )
}

pub(crate) fn render_suggestion_line(label: &str, suggestions: &[String]) -> Option<String> {
    (!suggestions.is_empty()).then(|| format!("  {label:<16} {}", suggestions.join(", "),))
}

pub(crate) fn suggest_slash_commands(input: &str) -> Vec<String> {
    let mut candidates = slash_command_specs()
        .iter()
        .flat_map(|spec| {
            std::iter::once(spec.name)
                .chain(spec.aliases.iter().copied())
                .map(|name| format!("/{name}"))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.dedup();
    let candidate_refs = candidates.iter().map(String::as_str).collect::<Vec<_>>();
    ranked_suggestions(input.trim_start_matches('/'), &candidate_refs)
        .into_iter()
        .map(str::to_string)
        .collect()
}

pub(crate) fn suggest_closest_term<'a>(input: &str, candidates: &'a [&'a str]) -> Option<&'a str> {
    ranked_suggestions(input, candidates).into_iter().next()
}

pub(crate) fn suggest_similar_subcommand(input: &str) -> Option<Vec<String>> {
    const KNOWN_SUBCOMMANDS: &[&str] = &[
        "help",
        "version",
        "status",
        "sandbox",
        "doctor",
        "setup",
        "state",
        "dump-manifests",
        "bootstrap-plan",
        "agents",
        "mcp",
        "skills",
        "system-prompt",
        "acp",
        "init",
        "export",
        "prompt",
        "list",
    ];

    let normalized_input = input.to_ascii_lowercase();
    let mut ranked = KNOWN_SUBCOMMANDS
        .iter()
        .filter_map(|candidate| {
            let normalized_candidate = candidate.to_ascii_lowercase();
            let distance = levenshtein_distance(&normalized_input, &normalized_candidate);
            let prefix_match = common_prefix_len(&normalized_input, &normalized_candidate) >= 4;
            let substring_match = normalized_candidate.contains(&normalized_input)
                || normalized_input.contains(&normalized_candidate);
            ((distance <= 2) || prefix_match || substring_match).then_some((distance, *candidate))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.cmp(right).then_with(|| left.1.cmp(right.1)));
    ranked.dedup_by(|left, right| left.1 == right.1);
    let suggestions = ranked
        .into_iter()
        .map(|(_, candidate)| candidate.to_string())
        .take(3)
        .collect::<Vec<_>>();
    (!suggestions.is_empty()).then_some(suggestions)
}

pub(crate) fn common_prefix_len(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(l, r)| l == r)
        .count()
}

pub(crate) fn looks_like_subcommand_typo(input: &str) -> bool {
    !input.is_empty()
        && input
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || ch == '-')
}

pub(crate) fn ranked_suggestions<'a>(input: &str, candidates: &'a [&'a str]) -> Vec<&'a str> {
    let normalized_input = input.trim_start_matches('/').to_ascii_lowercase();
    let mut ranked = candidates
        .iter()
        .filter_map(|candidate| {
            let normalized_candidate = candidate.trim_start_matches('/').to_ascii_lowercase();
            let distance = levenshtein_distance(&normalized_input, &normalized_candidate);
            let prefix_bonus = usize::from(
                !(normalized_candidate.starts_with(&normalized_input)
                    || normalized_input.starts_with(&normalized_candidate)),
            );
            let score = distance + prefix_bonus;
            (score <= 4).then_some((score, *candidate))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.cmp(right).then_with(|| left.1.cmp(right.1)));
    ranked
        .into_iter()
        .map(|(_, candidate)| candidate)
        .take(3)
        .collect()
}

pub(crate) fn levenshtein_distance(left: &str, right: &str) -> usize {
    if left.is_empty() {
        return right.chars().count();
    }
    if right.is_empty() {
        return left.chars().count();
    }

    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution_cost = usize::from(left_char != *right_char);
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution_cost);
        }
        previous.clone_from(&current);
    }

    previous[right_chars.len()]
}

/// Validate model syntax at parse time.
/// Accepts: known aliases (opus, sonnet, haiku) or provider/model pattern.
/// Rejects: empty, whitespace-only, strings with spaces, or invalid chars.
pub(crate) fn validate_model_syntax(model: &str) -> Result<(), String> {
    let trimmed = model.trim();
    // Ollama models use names like "qwen3:8b" that don't match provider/model
    // syntax. Skip strict validation when OLLAMA_HOST is configured.
    if std::env::var_os("OLLAMA_HOST").is_some() {
        if trimmed.is_empty() {
            return Err("invalid model syntax: model string cannot be empty.\nUsage: --model <model-name>  e.g. --model qwen3:8b".to_string());
        }
        return Ok(());
    }
    if trimmed.is_empty() {
        return Err("invalid model syntax: model string cannot be empty.\nUsage: --model <provider/model>  e.g. --model anthropic/claude-opus-4-7".to_string());
    }
    // Check for spaces (malformed)
    if trimmed.contains(' ') {
        return Err(format!(
            "invalid model syntax: '{}' contains spaces.\nUse provider/model format (e.g., anthropic/claude-opus-4-7) or a known alias.",
            trimmed
        ));
    }
    if is_bare_provider_model(trimmed) {
        return Ok(());
    }
    if is_local_openai_model_syntax(trimmed) {
        return Ok(());
    }
    // Check provider/model format: provider_id/model_id
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        // #154: hint if the model looks like it belongs to a different provider
        let mut err_msg = format!(
            "invalid model syntax: '{}'.\nExpected provider/model (e.g., anthropic/claude-opus-4-7)",
            trimmed
        );
        if trimmed.starts_with("gpt-") || trimmed.starts_with("gpt_") {
            err_msg.push_str("\nDid you mean `openai/");
            err_msg.push_str(trimmed);
            err_msg.push_str("`? (Requires OPENAI_API_KEY env var)");
        } else if trimmed.starts_with("qwen") && trimmed.contains(':') {
            err_msg.push_str("\nFor a local Ollama model, set `OPENAI_BASE_URL=http://127.0.0.1:11434/v1` before using tagged names like `");
            err_msg.push_str(trimmed);
            err_msg.push_str("`.");
        } else if trimmed.starts_with("qwen") {
            err_msg.push_str("\nDid you mean `qwen/");
            err_msg.push_str(trimmed);
            err_msg.push_str("`? (Requires DASHSCOPE_API_KEY env var)");
        } else if trimmed.starts_with("grok") {
            err_msg.push_str("\nDid you mean `xai/");
            err_msg.push_str(trimmed);
            err_msg.push_str("`? (Requires XAI_API_KEY env var)");
        }
        return Err(err_msg);
    }
    Ok(())
}

pub(crate) fn parse_permission_mode_arg(value: &str) -> Result<PermissionMode, String> {
    normalize_permission_mode(value)
        .ok_or_else(|| {
            format!(
                "invalid_permission_mode: unsupported permission mode '{value}'.\nUsage: --permission-mode read-only|workspace-write|danger-full-access"
            )
        })
        .map(permission_mode_from_label)
}

pub(crate) fn permission_mode_from_label(mode: &str) -> PermissionMode {
    match mode {
        "read-only" => PermissionMode::ReadOnly,
        "workspace-write" => PermissionMode::WorkspaceWrite,
        "danger-full-access" => PermissionMode::DangerFullAccess,
        other => panic!("unsupported permission mode label: {other}"),
    }
}

pub(crate) fn permission_mode_from_resolved(mode: ResolvedPermissionMode) -> PermissionMode {
    match mode {
        ResolvedPermissionMode::ReadOnly => PermissionMode::ReadOnly,
        ResolvedPermissionMode::WorkspaceWrite => PermissionMode::WorkspaceWrite,
        ResolvedPermissionMode::DangerFullAccess => PermissionMode::DangerFullAccess,
    }
}

pub(crate) fn default_permission_mode() -> PermissionMode {
    permission_mode_provenance_for_current_dir().mode
}

pub(crate) fn config_permission_mode_for_current_dir() -> Option<PermissionMode> {
    let cwd = env::current_dir().ok()?;
    let loader = ConfigLoader::default_for(&cwd);
    loader
        .load()
        .ok()?
        .permission_mode()
        .map(permission_mode_from_resolved)
}

pub(crate) fn provider_label(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Anthropic => "anthropic",
        ProviderKind::Xai => "xai",
        ProviderKind::OpenAi => "openai",
    }
}

pub(crate) fn format_connected_line(model: &str) -> String {
    let provider = provider_label(detect_provider_kind(model));
    format!("Connected: {model} via {provider}")
}

pub(crate) fn parse_system_prompt_args(
    args: &[String],
    model: String,
    output_format: CliOutputFormat,
) -> Result<CliAction, String> {
    let mut cwd = env::current_dir().map_err(|error| error.to_string())?;
    let mut date = DEFAULT_DATE.to_string();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--cwd" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    "missing_flag_value: missing value for --cwd.\nUsage: --cwd <path>".to_string()
                })?;
                cwd = PathBuf::from(value);
                // #99: validate --cwd path exists and is a directory
                if !cwd.exists() {
                    return Err(format!(
                        "invalid_cwd: path '{value}' does not exist.\nUsage: claw system-prompt --cwd <existing-directory>"
                    ));
                }
                if !cwd.is_dir() {
                    return Err(format!(
                        "invalid_cwd: path '{value}' is not a directory.\nUsage: claw system-prompt --cwd <existing-directory>"
                    ));
                }
                index += 2;
            }
            "--date" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    "missing_flag_value: missing value for --date.\nUsage: --date <YYYY-MM-DD>"
                        .to_string()
                })?;
                // #99: validate --date is a plausible date string (no newlines, reasonable length)
                if value.contains('\n') || value.contains('\r') {
                    return Err(format!(
                        "invalid_flag_value: --date value contains invalid characters.\nUsage: --date <YYYY-MM-DD>"
                    ));
                }
                if value.len() > 20 {
                    return Err(format!(
                        "invalid_flag_value: --date value is too long ({len} chars, expected YYYY-MM-DD).\nUsage: --date <YYYY-MM-DD>",
                        len = value.len()
                    ));
                }
                date.clone_from(value);
                index += 2;
            }

            other => {
                // #152: hint `--output-format json` when user types `--json`.
                // #790: use unknown_option: prefix + \n hint so classify_error_kind returns
                // unknown_option and split_error_hint extracts the remediation text.
                let hint = if other == "--json" {
                    "Did you mean `--output-format json`? Usage: claw system-prompt [--cwd <dir>] [--date <YYYY-MM-DD>] [--output-format text|json]".to_string()
                } else {
                    "Usage: claw system-prompt [--cwd <dir>] [--date <YYYY-MM-DD>] [--output-format text|json]".to_string()
                };
                return Err(format!(
                    "unknown_option: unknown system-prompt option: {other}.\n{hint}"
                ));
            }
        }
    }

    Ok(CliAction::PrintSystemPrompt {
        cwd,
        date,
        model,
        output_format,
    })
}

pub(crate) fn parse_export_args(
    args: &[String],
    output_format: CliOutputFormat,
) -> Result<CliAction, String> {
    let mut session_reference = LATEST_SESSION_REFERENCE.to_string();
    let mut output_path: Option<PathBuf> = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--session" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "missing_flag_value: missing value for --session.\nUsage: --session <session-id>".to_string())?;
                session_reference.clone_from(value);
                index += 2;
            }
            flag if flag.starts_with("--session=") => {
                session_reference = flag[10..].to_string();
                index += 1;
            }
            "--output" | "-o" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("missing_flag_value: missing value for {}.\nUsage: claw export [PATH] [--session SESSION] [--output PATH]", args[index]))?;
                output_path = Some(PathBuf::from(value));
                index += 2;
            }
            flag if flag.starts_with("--output=") => {
                output_path = Some(PathBuf::from(&flag[9..]));
                index += 1;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown_option: unknown export option: {other}.\nRun `claw export --help` for usage."));
            }
            other if output_path.is_none() => {
                output_path = Some(PathBuf::from(other));
                index += 1;
            }
            other => {
                // #784: use typed prefix so classify_error_kind returns unexpected_extra_args
                return Err(format!("unexpected_extra_args: unexpected export argument: {other}.\nUsage: claw export [PATH] [--session SESSION] [--output PATH]"));
            }
        }
    }

    Ok(CliAction::Export {
        session_reference,
        output_path,
        output_format,
    })
}

pub(crate) fn parse_dump_manifests_args(
    args: &[String],
    output_format: CliOutputFormat,
) -> Result<CliAction, String> {
    let mut manifests_dir: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--manifests-dir" {
            let value = args
                .get(index + 1)
                .ok_or_else(|| String::from("missing_flag_value: --manifests-dir requires a path.\nUsage: claw dump-manifests --manifests-dir <path> [--output-format json]"))?;
            manifests_dir = Some(PathBuf::from(value));
            index += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--manifests-dir=") {
            if value.is_empty() {
                // #786: empty --manifests-dir= is also a missing value
                return Err(String::from("missing_flag_value: --manifests-dir requires a path.\nUsage: claw dump-manifests --manifests-dir <path> [--output-format json]"));
            }
            manifests_dir = Some(PathBuf::from(value));
            index += 1;
            continue;
        }
        return Err(format!("unknown_option: unknown dump-manifests option: {arg}.\nRun `claw dump-manifests --help` for usage."));
    }

    Ok(CliAction::DumpManifests {
        output_format,
        manifests_dir,
    })
}

pub(crate) fn parse_resume_args(
    args: &[String],
    output_format: CliOutputFormat,
    allow_broad_cwd: bool,
) -> Result<CliAction, String> {
    let (session_path, command_tokens): (PathBuf, &[String]) = match args.first() {
        None => (PathBuf::from(LATEST_SESSION_REFERENCE), &[]),
        Some(first) if looks_like_slash_command_token(first) => {
            (PathBuf::from(LATEST_SESSION_REFERENCE), args)
        }
        Some(first) => (PathBuf::from(first), &args[1..]),
    };
    let mut commands = Vec::new();
    let mut current_command = String::new();

    for token in command_tokens {
        if token.trim_start().starts_with('/') {
            if resume_command_can_absorb_token(&current_command, token) {
                current_command.push(' ');
                current_command.push_str(token);
                continue;
            }
            if !current_command.is_empty() {
                commands.push(current_command);
            }
            current_command = String::from(token.as_str());
            continue;
        }

        if current_command.is_empty() {
            // #768: typed prefix + \n hint so split_error_hint() extracts hint into JSON envelope
            return Err(format!(
                "invalid_resume_argument: `{token}` is not a slash command.\nUsage: claw --resume <session-id|latest> /<slash-command>  (e.g. /compact, /status)"
            ));
        }

        current_command.push(' ');
        current_command.push_str(token);
    }

    if !current_command.is_empty() {
        commands.push(current_command);
    }

    Ok(CliAction::ResumeSession {
        session_path,
        commands,
        output_format,
        allow_broad_cwd,
    })
}

pub(crate) fn resume_command_can_absorb_token(current_command: &str, token: &str) -> bool {
    matches!(
        SlashCommand::parse(current_command),
        Ok(Some(SlashCommand::Export { path: None }))
    ) && !looks_like_slash_command_token(token)
}

pub(crate) fn looks_like_slash_command_token(token: &str) -> bool {
    let trimmed = token.trim_start();
    let Some(name) = trimmed.strip_prefix('/').and_then(|value| {
        value
            .split_whitespace()
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }) else {
        return false;
    };

    slash_command_specs()
        .iter()
        .any(|spec| spec.name == name || spec.aliases.contains(&name))
}

#[allow(clippy::too_many_lines)]
pub(crate) fn resume_session(
    session_path: &Path,
    commands: &[String],
    output_format: CliOutputFormat,
) {
    let session_reference = session_path.display().to_string();
    let (handle, session) = match load_session_reference(&session_reference) {
        Ok(loaded) => loaded,
        Err(error) => {
            if output_format == CliOutputFormat::Json {
                // #77: classify session load errors for downstream consumers
                let full_message = format!("failed to restore session: {error}");
                let kind = classify_error_kind(&full_message);
                let (short_reason, inline_hint) = split_error_hint(&full_message);
                // #787: fall back to kind-derived hint when message has no \n delimiter
                let hint =
                    inline_hint.or_else(|| fallback_hint_for_error_kind(kind).map(String::from));
                let sessions_dir = sessions_dir().ok().map(|path| path.display().to_string());
                // #819: JSON mode resume errors go to stdout for parity with other
                // non-interactive command guards.
                println!(
                    "{}",
                    serde_json::json!({
                        "kind": kind,
                        "action": "restore",
                        "status": "error",
                        "error_kind": kind,
                        "error": short_reason,
                        "exit_code": 1,
                        "hint": hint,
                        "sessions_dir": sessions_dir,
                    })
                );
            } else {
                eprintln!("failed to restore session: {error}");
            }
            std::process::exit(1);
        }
    };
    let resolved_path = handle.path.clone();

    if commands.is_empty() {
        if output_format == CliOutputFormat::Json {
            println!(
                "{}",
                serde_json::json!({
                    "kind": "restored",
                    "action": "restore",
                    "status": "ok",
                    "session_id": session.session_id,
                    "path": handle.path.display().to_string(),
                    "message_count": session.messages.len(),
                })
            );
        } else {
            println!(
                "Restored session from {} ({} messages).",
                handle.path.display(),
                session.messages.len()
            );
        }
        return;
    }

    let mut session = session;
    for raw_command in commands {
        // Intercept spec commands that have no parse arm before calling
        // SlashCommand::parse — they return Err(SlashCommandParseError) which
        // formats as the confusing circular "Did you mean /X?" message.
        // STUB_COMMANDS covers both completions-filtered stubs and parse-less
        // spec entries; treat both as unsupported in resume mode.
        {
            let cmd_root = raw_command
                .trim_start_matches('/')
                .split_whitespace()
                .next()
                .unwrap_or("");
            if STUB_COMMANDS.contains(&cmd_root) {
                if output_format == CliOutputFormat::Json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "kind": "unsupported_command",
                            "action": "resume",
                            "status": "error",
                            "error_kind": "unsupported_command",
                            "error": format!("/{cmd_root} is not yet implemented in this build"),
                            "hint": "This command is not available in the current build. Update claw or use a different command.",
                            "exit_code": 2,
                            "command": raw_command,
                        })
                    );
                } else {
                    eprintln!("/{cmd_root} is not yet implemented in this build");
                }
                std::process::exit(2);
            }
        }
        let command = match SlashCommand::parse(raw_command) {
            Ok(Some(command)) => command,
            Ok(None) => {
                if output_format == CliOutputFormat::Json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "kind": "unsupported_resumed_command",
                            "action": "resume",
                            "status": "error",
                            "error_kind": "unsupported_resumed_command",
                            "error": format!("unsupported resumed command: {raw_command}"),
                            "hint": "This command cannot be used with --resume. Use it in an interactive REPL session instead.",
                            "exit_code": 2,
                            "command": raw_command,
                        })
                    );
                } else {
                    eprintln!("unsupported resumed command: {raw_command}");
                }
                std::process::exit(2);
            }
            Err(error) => {
                if output_format == CliOutputFormat::Json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "kind": "cli_parse",
                            "action": "resume",
                            "status": "error",
                            "error_kind": "cli_parse",
                            "error": error.to_string(),
                            "hint": "Run `claw --help` for usage.",
                            "exit_code": 2,
                            "command": raw_command,
                        })
                    );
                } else {
                    eprintln!("{error}");
                }
                std::process::exit(2);
            }
        };
        match run_resume_command(&resolved_path, &session, &command) {
            Ok(ResumeCommandOutcome {
                session: next_session,
                message,
                json,
            }) => {
                session = next_session;
                if output_format == CliOutputFormat::Json {
                    if let Some(value) = json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&value)
                                .expect("resume command json output")
                        );
                    } else if let Some(message) = message {
                        println!("{message}");
                    }
                } else if let Some(message) = message {
                    println!("{message}");
                }
            }
            Err(error) => {
                if output_format == CliOutputFormat::Json {
                    // #776: classify + split so wrappers get typed fields instead of
                    // hardcoded "resume_command_error" + prose in the error field
                    let full_error = error.to_string();
                    let error_kind = classify_error_kind(&full_error);
                    let (short_reason, inline_hint) = split_error_hint(&full_error);
                    // #787: fall back to kind-derived hint when error has no \n delimiter
                    let hint = inline_hint
                        .or_else(|| fallback_hint_for_error_kind(error_kind).map(String::from));
                    println!(
                        "{}",
                        serde_json::json!({
                            "kind": error_kind,
                            "action": "resume",
                            "status": "error",
                            "error_kind": error_kind,
                            "error": short_reason,
                            "hint": hint,
                            "exit_code": 2,
                            "command": raw_command,
                        })
                    );
                } else {
                    eprintln!("{error}");
                }
                std::process::exit(2);
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn format_unknown_slash_command_message(name: &str) -> String {
    let suggestions = suggest_slash_commands(name);
    let mut message = format!("unknown slash command: /{name}.");
    if !suggestions.is_empty() {
        message.push_str(" Did you mean ");
        message.push_str(&suggestions.join(", "));
        message.push('?');
    }
    if let Some(note) = omc_compatibility_note_for_unknown_slash_command(name) {
        message.push(' ');
        message.push_str(note);
    }
    message.push_str(" Use /help to list available commands.");
    message
}

pub(crate) fn format_model_report(model: &str, message_count: usize, turns: u32) -> String {
    format!(
        "Model
  Current model    {model}
  Session messages {message_count}
  Session turns    {turns}

Usage
  Inspect current model with /model
  Switch models with /model <name>"
    )
}

pub(crate) fn format_model_switch_report(
    previous: &str,
    next: &str,
    message_count: usize,
) -> String {
    format!(
        "Model updated
  Previous         {previous}
  Current          {next}
  Preserved msgs   {message_count}"
    )
}

pub(crate) fn format_permissions_report(mode: &str) -> String {
    let modes = [
        ("read-only", "Read/search tools only", mode == "read-only"),
        (
            "workspace-write",
            "Edit files inside the workspace",
            mode == "workspace-write",
        ),
        (
            "danger-full-access",
            "Unrestricted tool access",
            mode == "danger-full-access",
        ),
    ]
    .into_iter()
    .map(|(name, description, is_current)| {
        let marker = if is_current {
            "● current"
        } else {
            "○ available"
        };
        format!("  {name:<18} {marker:<11} {description}")
    })
    .collect::<Vec<_>>()
    .join(
        "
",
    );

    format!(
        "Permissions
  Active mode      {mode}
  Mode status      live session default

Modes
{modes}

Usage
  Inspect current mode with /permissions
  Switch modes with /permissions <mode>"
    )
}

pub(crate) fn format_permissions_switch_report(previous: &str, next: &str) -> String {
    format!(
        "Permissions updated
  Result           mode switched
  Previous mode    {previous}
  Active mode      {next}
  Applies to       subsequent tool calls
  Usage            /permissions to inspect current mode"
    )
}

pub(crate) fn format_cost_report(usage: TokenUsage) -> String {
    let estimated_cost = usage.estimate_cost_usd();
    format!(
        "Cost
  Input tokens     {}
  Output tokens    {}
  Cache create     {}
  Cache read       {}
  Total tokens     {}
  Estimated cost   {}",
        usage.input_tokens,
        usage.output_tokens,
        usage.cache_creation_input_tokens,
        usage.cache_read_input_tokens,
        usage.total_tokens(),
        format_usd(estimated_cost.total_cost_usd()),
    )
}

pub(crate) fn format_resume_report(session_path: &str, message_count: usize, turns: u32) -> String {
    format!(
        "Session resumed
  Session file     {session_path}
  Messages         {message_count}
  Turns            {turns}"
    )
}

pub(crate) fn render_resume_usage() -> String {
    format!(
        "Resume
  Usage            /resume <session-path|session-id|{LATEST_SESSION_REFERENCE}>
  Auto-save        .claw/sessions/<workspace-fingerprint>/<session-id>.{PRIMARY_SESSION_EXTENSION}
  Tip              use /session list to inspect saved sessions"
    )
}

pub(crate) fn format_compact_report(
    removed: usize,
    resulting_messages: usize,
    skipped: bool,
) -> String {
    if skipped {
        format!(
            "Compact
  Result           skipped
  Reason           session below compaction threshold
  Messages kept    {resulting_messages}"
        )
    } else {
        format!(
            "Compact
  Result           compacted
  Messages removed {removed}
  Messages kept    {resulting_messages}"
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PromptHistoryEntry {
    pub(crate) timestamp_ms: u64,
    pub(crate) text: String,
}

pub(crate) fn render_repl_help() -> String {
    [
        "REPL".to_string(),
        "  /exit                Quit the REPL".to_string(),
        "  /quit                Quit the REPL".to_string(),
        "  Up/Down              Navigate prompt history".to_string(),
        "  Ctrl-R               Reverse-search prompt history".to_string(),
        "  Tab                  Complete commands, modes, and recent sessions".to_string(),
        "  Ctrl-C               Clear input (or exit on empty prompt)".to_string(),
        "  Shift+Enter/Ctrl+J   Insert a newline".to_string(),
        "  Auto-save            .claw/sessions/<workspace-fingerprint>/<session-id>.jsonl"
            .to_string(),
        "  Resume latest        /resume latest".to_string(),
        "  Browse sessions      /session list".to_string(),
        "  Show prompt history  /history [count]".to_string(),
        String::new(),
        render_slash_command_help_filtered(STUB_COMMANDS),
    ]
    .join(
        "
",
    )
}

pub(crate) fn format_commit_skipped_report() -> String {
    "Commit
  Result           skipped
  Reason           no workspace changes
  Action           create a git commit from the current workspace changes
  Next             /status to inspect context · /diff to inspect repo changes"
        .to_string()
}

pub(crate) fn render_help_topic(topic: LocalHelpTopic) -> String {
    match topic {
        LocalHelpTopic::Status => "Status
  Usage            claw status [--output-format <format>]
  Purpose          show the local workspace snapshot without entering the REPL
  Output           model, permissions, git state, config files, and sandbox status
  Formats          text (default), json
  Related          /status · claw --resume latest /status"
            .to_string(),
        LocalHelpTopic::Sandbox => "Sandbox
  Usage            claw sandbox [--output-format <format>]
  Purpose          inspect the resolved sandbox and isolation state for the current directory
  Output           namespace, network, filesystem, and fallback details
  Formats          text (default), json
  Related          /sandbox · claw status"
            .to_string(),
        LocalHelpTopic::Doctor => "Doctor
  Usage            claw doctor [--output-format <format>]
  Purpose          diagnose local auth, config, workspace, sandbox, and build metadata
  Output           local-only health report; no provider request or session resume required
  Formats          text (default), json
  Related          /doctor · claw --resume latest /doctor"
            .to_string(),
        LocalHelpTopic::Acp => "ACP / Zed
  Usage            claw acp [serve] [--output-format <format>]
  Aliases          claw --acp · claw -acp
  Purpose          explain the current editor-facing ACP/Zed launch contract without starting the runtime
  Status           discoverability only; `serve` is a status alias and does not launch a daemon yet
  Formats          text (default), json
  Related          ROADMAP #64a (discoverability) · ROADMAP #76 (real ACP support) · claw --help"
            .to_string(),
        LocalHelpTopic::Init => "Init
  Usage            claw init [--output-format <format>]
  Purpose          create .claw/settings.json, .claw.json, .gitignore, and CLAUDE.md in the current project
  Output           per-artifact created/updated/partial/deferred/skipped status (idempotent: safe to re-run)
  Formats          text (default), json
  Related          claw status · claw doctor"
            .to_string(),
        LocalHelpTopic::State => "State
  Usage            claw state [--output-format <format>]
  Purpose          read .claw/worker-state.json written by the interactive REPL or a one-shot prompt
  Output           worker id, model, permissions, session reference (text or json)
  Formats          text (default), json
  Produces state   `claw` (interactive REPL) or `claw prompt <text>` (one non-interactive turn)
  Observes state   `claw state` reads; clawhip/CI may poll this file without HTTP
  Exit codes       0 if state file exists and parses; 1 with actionable hint otherwise
  Related          claw status · ROADMAP #139 (this worker-concept contract)"
            .to_string(),
        LocalHelpTopic::Resume => format!(
            "Resume\n  Usage            claw resume [session-path|session-id|{LATEST_SESSION_REFERENCE}] [/slash-command ...] [--output-format <format>]\n  Alias            claw --resume [session-path|session-id|{LATEST_SESSION_REFERENCE}]\n  Purpose          restore or inspect a saved session without starting a new provider turn\n  Output           session restore or resume-safe command output; missing sessions return session_not_found\n  Formats          text (default), json\n  Related          /resume · /session list · claw --resume {LATEST_SESSION_REFERENCE} /status"
        ),
        LocalHelpTopic::Session => "Session
  Usage            claw session --help [--output-format <format>]
  Purpose          show /session command guidance without loading config, credentials, or a session
  Actions          list · exists <id> · switch <id> · fork <name> · delete <id>
  Direct use       run /session in the REPL or claw --resume SESSION.jsonl /session <action>
  Formats          text (default), json
  Related          claw resume · claw export · .claw/sessions/"
            .to_string(),
        LocalHelpTopic::Compact => "Compact
  Usage            claw compact --help [--output-format <format>]
  Purpose          show compaction guidance without loading config, credentials, or a session
  Direct use       run /compact in the REPL or claw --resume SESSION.jsonl /compact
  Output           compaction removes older tool-detail messages when the selected session is large enough
  Formats          text (default), json
  Related          claw resume · /compact · /status"
            .to_string(),
        LocalHelpTopic::Export => "Export
  Usage            claw export [--session <id|latest>] [--output <path>] [--output-format <format>]
  Purpose          serialize a managed session to JSON for review, transfer, or archival
  Defaults         --session latest (most recent managed session in .claw/sessions/)
  Formats          text (default), json
  Related          /session list · claw --resume latest"
            .to_string(),
        LocalHelpTopic::Version => "Version
  Usage            claw version [--output-format <format>]
  Aliases          claw --version · claw -V
  Purpose          print the claw CLI version and build metadata
  Formats          text (default), json
  Related          claw doctor (full build/auth/config diagnostic)"
            .to_string(),
        LocalHelpTopic::SystemPrompt => "System Prompt
  Usage            claw system-prompt [--cwd <path>] [--date YYYY-MM-DD] [--output-format <format>]
  Purpose          render the resolved system prompt that `claw` would send for the given cwd + date
  Options          --cwd overrides the workspace dir · --date injects a deterministic date stamp
  Formats          text (default), json
  Related          claw doctor · claw dump-manifests"
            .to_string(),
        LocalHelpTopic::DumpManifests => "Dump Manifests
  Usage            claw dump-manifests [--manifests-dir <path>] [--output-format <format>]
  Purpose          emit every skill/agent/tool manifest the resolver would load for the current cwd
  Options          --manifests-dir scopes discovery to a specific directory
  Formats          text (default), json
  Related          claw skills · claw agents · claw doctor"
            .to_string(),
        LocalHelpTopic::BootstrapPlan => "Bootstrap Plan
  Usage            claw bootstrap-plan [--output-format <format>]
  Purpose          list the ordered startup phases the CLI would execute before dispatch
  Output           phase names (text) or structured phase list (json) — primary output is the plan itself
  Formats          text (default), json
  Related          claw doctor · claw status"
            .to_string(),
        LocalHelpTopic::Agents => commands::handle_agents_slash_command(
            Some("--help"),
            &env::current_dir().unwrap_or_default(),
        )
        .unwrap_or_else(|_| "agents help unavailable".to_string()),
        LocalHelpTopic::Skills => commands::handle_skills_slash_command(
            Some("--help"),
            &env::current_dir().unwrap_or_default(),
        )
        .unwrap_or_else(|_| "skills help unavailable".to_string()),
        LocalHelpTopic::Plugins => "Plugins
  Usage            claw plugins [list|show <name>|install <path>|enable <name>|disable <name>|uninstall <name>]
  Purpose          manage lifecycle of plugins that extend tool and hook capabilities
  Formats          text (default), json
  Related          /plugins · claw plugins --help"
            .to_string(),
        LocalHelpTopic::Mcp => "MCP Servers
  Usage            claw mcp [list|show <server>] [--output-format <format>]
  Purpose          inspect configured MCP servers and their connection status
  Formats          text (default), json
  Related          /mcp · claw mcp list"
            .to_string(),
        LocalHelpTopic::Config => "Config
  Usage            claw config [section] [--output-format <format>]
  Purpose          show effective runtime configuration (model, hooks, plugins, env)
  Formats          text (default), json
  Related          /config · claw doctor"
            .to_string(),
        LocalHelpTopic::Model => "Models
  Usage            claw models [help] [--output-format <format>]
  Aliases          claw model
  Purpose          show bounded local model command guidance without entering the REPL
  Output           supported model-selection surfaces and current config model value
  Formats          text (default), json
  Related          /model · claw config model · claw status"
            .to_string(),
        LocalHelpTopic::Settings => "Settings
  Usage            claw settings [help] [--output-format <format>]
  Purpose          show effective settings/config using the local config envelope
  Output           same as claw config settings; no provider request or session resume required
  Formats          text (default), json
  Related          claw config · claw doctor"
            .to_string(),
        LocalHelpTopic::Diff => "Diff
  Usage            claw diff [--output-format <format>]
  Purpose          show the diff of changes relative to the expected base commit
  Formats          text (default), json
  Related          /diff · ROADMAP #148"
            .to_string(),
        LocalHelpTopic::Setup => "Setup
  Usage            claw setup
  Aliases          /setup (inside the REPL)
  Purpose          run the interactive provider setup wizard to configure API key, model, and base URL
  Output           writes provider settings to ~/.claw/settings.json (0600 permissions)
  Related          /model · /config · claw doctor"
            .to_string(),
    }
}

pub(crate) fn local_help_topic_command(topic: LocalHelpTopic) -> &'static str {
    match topic {
        LocalHelpTopic::Status => "status",
        LocalHelpTopic::Sandbox => "sandbox",
        LocalHelpTopic::Doctor => "doctor",
        LocalHelpTopic::Acp => "acp",
        LocalHelpTopic::Init => "init",
        LocalHelpTopic::State => "state",
        LocalHelpTopic::Resume => "resume",
        LocalHelpTopic::Session => "session",
        LocalHelpTopic::Compact => "compact",
        LocalHelpTopic::Export => "export",
        LocalHelpTopic::Version => "version",
        LocalHelpTopic::SystemPrompt => "system-prompt",
        LocalHelpTopic::DumpManifests => "dump-manifests",
        LocalHelpTopic::BootstrapPlan => "bootstrap-plan",
        LocalHelpTopic::Agents => "agents",
        LocalHelpTopic::Skills => "skills",
        LocalHelpTopic::Plugins => "plugins",
        LocalHelpTopic::Mcp => "mcp",
        LocalHelpTopic::Config => "config",
        LocalHelpTopic::Model => "models",
        LocalHelpTopic::Settings => "settings",
        LocalHelpTopic::Diff => "diff",
        LocalHelpTopic::Setup => "setup",
    }
}

pub(crate) fn render_export_help_json() -> serde_json::Value {
    json!({
        "kind": "help",
        "action": "help",
        "status": "ok",
        "topic": "export",
        "command": "export",
        "usage": "claw export [--session <id|latest>] [--output <path>] [--output-format <format>]",
        "purpose": "serialize a managed session to JSON for review, transfer, or archival",
        "defaults": {
            "session": LATEST_SESSION_REFERENCE,
            "session_source": ".claw/sessions/",
            "output": "derived from the selected session when omitted"
        },
        "formats": ["text", "json"],
        "options": [
            {
                "name": "--session",
                "value": "<id|latest>",
                "default": LATEST_SESSION_REFERENCE,
                "description": "managed session to export"
            },
            {
                "name": "--output",
                "aliases": ["-o"],
                "value": "<path>",
                "description": "write the exported transcript to this path"
            },
            {
                "name": "--output-format",
                "value": "<format>",
                "values": ["text", "json"],
                "default": "text",
                "description": "format for the command result envelope"
            },
            {
                "name": "--help",
                "aliases": ["-h"],
                "description": "show help for the export command"
            }
        ],
        "related": ["/session list", "claw --resume latest"]
    })
}

pub(crate) fn render_help_topic_json(topic: LocalHelpTopic) -> serde_json::Value {
    if topic == LocalHelpTopic::Export {
        return render_export_help_json();
    }
    if topic == LocalHelpTopic::Doctor {
        return render_doctor_help_json();
    }

    // #683-#692: extract structured metadata from help prose for machine consumption
    let (usage, purpose, output_desc, formats, related, aliases, local_only, requires_credentials) =
        extract_help_metadata(topic);
    let mut obj = serde_json::json!({
        "kind": "help",
        "action": "help",
        "status": "ok",
        "topic": local_help_topic_command(topic),
        "command": local_help_topic_command(topic),
        "message": render_help_topic(topic),
        "usage": usage,
        "purpose": purpose,
        "formats": formats,
        "related": related,
        "local_only": local_only,
        "requires_credentials": requires_credentials,
    });
    if let Some(desc) = output_desc {
        obj["output_fields"] = serde_json::Value::String(desc);
    }
    if let Some(a) = aliases {
        obj["aliases"] = serde_json::json!(a);
    }
    obj
}

pub(crate) fn acp_status_message() -> &'static str {
    "ACP/Zed editor integration is not implemented in claw-code yet. `claw acp serve` reports status only and does not launch a daemon or JSON-RPC endpoint. Use the normal terminal surfaces for now."
}

pub(crate) fn acp_status_json() -> serde_json::Value {
    json!({
        "schema_version": "1.0",
        "kind": "acp",
        "action": "status",
        "status": "not_implemented",
        "supported": false,
        "message": acp_status_message(),
        "launch_command": serde_json::Value::Null,
        "protocol": {
            "name": "ACP/Zed",
            "json_rpc": false,
            "daemon": false,
            "endpoint": serde_json::Value::Null,
            "serve_starts_daemon": false
        },
        "contracts": {
            "blocking_gates": [
                "task_packet_schema",
                "session_control_schema",
                "event_report_schema"
            ],
            "stable_status_surface": "claw acp [serve] --output-format json",
            "unsupported_invocation_kind": "unsupported_acp_invocation"
        },
        "aliases": ["acp", "--acp", "-acp"],
    })
}

pub(crate) fn render_memory_report() -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let project_context = ProjectContext::discover(&cwd, DEFAULT_DATE)?;
    let mut lines = vec![format!(
        "Memory
  Working directory {}
  Instruction files {}",
        cwd.display(),
        project_context.instruction_files.len()
    )];
    if project_context.instruction_files.is_empty() {
        lines.push("Discovered files".to_string());
        lines.push(
            "  No CLAUDE.md, CLAW.md, AGENTS.md, or scoped instruction files discovered in the current directory ancestry."
                .to_string(),
        );
    } else {
        lines.push("Discovered files".to_string());
        for (index, file) in project_context.instruction_files.iter().enumerate() {
            let preview = file.content.lines().next().unwrap_or("").trim();
            let preview = if preview.is_empty() {
                "<empty>"
            } else {
                preview
            };
            lines.push(format!("  {}. {}", index + 1, file.path.display(),));
            lines.push(format!(
                "     source={} lines={} chars={} preview={}",
                file.source(),
                file.content.lines().count(),
                file.char_count(),
                preview
            ));
        }
    }
    Ok(lines.join(
        "
",
    ))
}

pub(crate) fn normalize_permission_mode(mode: &str) -> Option<&'static str> {
    match mode.trim() {
        "default" | "plan" | "read-only" => Some("read-only"),
        "acceptEdits" | "auto" | "workspace-write" => Some("workspace-write"),
        "dontAsk" | "bypassPermissions" | "dangerFullAccess" | "danger-full-access" => {
            Some("danger-full-access")
        }
        _ => None,
    }
}

pub(crate) fn indent_block(value: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    value
        .lines()
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn parse_history_count(raw: Option<&str>) -> Result<usize, String> {
    let Some(raw) = raw else {
        return Ok(DEFAULT_HISTORY_LIMIT);
    };
    // #776: use \n-delimited format so split_error_hint extracts hint into JSON envelopes
    let parsed: usize = raw
        .parse()
        .map_err(|_| format!("invalid_history_count: '{raw}' is not a positive integer.\nUsage: /history [count] (default: {DEFAULT_HISTORY_LIMIT})"))?;
    if parsed == 0 {
        return Err(format!("invalid_history_count: count must be greater than 0.\nUsage: /history [count] (default: {DEFAULT_HISTORY_LIMIT})"));
    }
    Ok(parsed)
}

pub(crate) fn format_history_timestamp(timestamp_ms: u64) -> String {
    let secs = timestamp_ms / 1_000;
    let subsec_ms = timestamp_ms % 1_000;
    let days_since_epoch = secs / 86_400;
    let seconds_of_day = secs % 86_400;
    let hours = seconds_of_day / 3_600;
    let minutes = (seconds_of_day % 3_600) / 60;
    let seconds = seconds_of_day % 60;

    let (year, month, day) = civil_from_days(i64::try_from(days_since_epoch).unwrap_or(0));
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}.{subsec_ms:03}Z")
}

// Computes civil (Gregorian) year/month/day from days since the Unix epoch
// (1970-01-01) using Howard Hinnant's `civil_from_days` algorithm.
#[allow(
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]
pub(crate) fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let doe = (z - era * 146_097) as u64; // [0, 146_096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = y + i64::from(m <= 2);
    (y as i32, m as u32, d as u32)
}

pub(crate) fn render_prompt_history_report(entries: &[PromptHistoryEntry], limit: usize) -> String {
    if entries.is_empty() {
        return "Prompt history\n  Result           no prompts recorded yet".to_string();
    }

    let total = entries.len();
    let start = total.saturating_sub(limit);
    let shown = &entries[start..];
    let mut lines = vec![
        "Prompt history".to_string(),
        format!("  Total            {total}"),
        format!("  Showing          {} most recent", shown.len()),
        format!("  Reverse search   Ctrl-R in the REPL"),
        String::new(),
    ];
    for (offset, entry) in shown.iter().enumerate() {
        let absolute_index = start + offset + 1;
        let timestamp = format_history_timestamp(entry.timestamp_ms);
        let first_line = entry.text.lines().next().unwrap_or("").trim();
        let display = if first_line.chars().count() > 80 {
            let truncated: String = first_line.chars().take(77).collect();
            format!("{truncated}...")
        } else {
            first_line.to_string()
        };
        lines.push(format!("  {absolute_index:>3}. [{timestamp}] {display}"));
    }
    lines.join("\n")
}

pub(crate) fn collect_session_prompt_history(session: &Session) -> Vec<PromptHistoryEntry> {
    if !session.prompt_history.is_empty() {
        return session
            .prompt_history
            .iter()
            .map(|entry| PromptHistoryEntry {
                timestamp_ms: entry.timestamp_ms,
                text: entry.text.clone(),
            })
            .collect();
    }
    let timestamp_ms = session.updated_at_ms;
    session
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::User)
        .filter_map(|message| {
            message.blocks.iter().find_map(|block| match block {
                ContentBlock::Text { text } => Some(PromptHistoryEntry {
                    timestamp_ms,
                    text: text.clone(),
                }),
                _ => None,
            })
        })
        .collect()
}

pub(crate) fn truncate_for_prompt(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        value.trim().to_string()
    } else {
        let truncated = value.chars().take(limit).collect::<String>();
        format!("{}\n…[truncated]", truncated.trim_end())
    }
}

pub(crate) fn sanitize_generated_message(value: &str) -> String {
    value.trim().trim_matches('`').trim().replace("\r\n", "\n")
}

pub(crate) fn parse_titled_body(value: &str) -> Option<(String, String)> {
    let normalized = sanitize_generated_message(value);
    let title = normalized
        .lines()
        .find_map(|line| line.strip_prefix("TITLE:").map(str::trim))?;
    let body_start = normalized.find("BODY:")?;
    let body = normalized[body_start + "BODY:".len()..].trim();
    Some((title.to_string(), body.to_string()))
}

pub(crate) fn short_tool_id(id: &str) -> String {
    let char_count = id.chars().count();
    if char_count <= 12 {
        return id.to_string();
    }
    let prefix: String = id.chars().take(12).collect();
    format!("{prefix}…")
}

pub(crate) struct CliPermissionPrompter {
    pub(crate) current_mode: PermissionMode,
}

impl CliPermissionPrompter {
    pub(crate) fn new(current_mode: PermissionMode) -> Self {
        Self { current_mode }
    }
}

impl runtime::PermissionPrompter for CliPermissionPrompter {
    fn decide(
        &mut self,
        request: &runtime::PermissionRequest,
    ) -> runtime::PermissionPromptDecision {
        println!();
        println!("Permission approval required");
        println!("  Tool             {}", request.tool_name);
        println!("  Current mode     {}", self.current_mode.as_str());
        println!("  Required mode    {}", request.required_mode.as_str());
        if let Some(reason) = &request.reason {
            println!("  Reason           {reason}");
        }
        println!("  Input            {}", request.input);
        print!("Approve this tool call? [y/N]: ");
        let _ = io::stdout().flush();

        let mut response = String::new();
        match io::stdin().read_line(&mut response) {
            Ok(_) => {
                let normalized = response.trim().to_ascii_lowercase();
                if matches!(normalized.as_str(), "y" | "yes") {
                    runtime::PermissionPromptDecision::Allow
                } else {
                    runtime::PermissionPromptDecision::Deny {
                        reason: format!(
                            "tool '{}' denied by user approval prompt",
                            request.tool_name
                        ),
                    }
                }
            }
            Err(error) => runtime::PermissionPromptDecision::Deny {
                reason: format!("permission approval failed: {error}"),
            },
        }
    }
}

pub(crate) fn slash_command_completion_candidates_with_sessions(
    model: &str,
    active_session_id: Option<&str>,
    recent_session_ids: Vec<String>,
) -> Vec<String> {
    let mut completions = BTreeSet::new();

    for spec in slash_command_specs() {
        if STUB_COMMANDS.contains(&spec.name) {
            continue;
        }
        completions.insert(format!("/{}", spec.name));
        for alias in spec.aliases {
            if !STUB_COMMANDS.contains(alias) {
                completions.insert(format!("/{alias}"));
            }
        }
    }

    for candidate in [
        "/bughunter ",
        "/clear --confirm",
        "/config ",
        "/config env",
        "/config hooks",
        "/config model",
        "/config plugins",
        "/mcp ",
        "/mcp list",
        "/mcp show ",
        "/export ",
        "/issue ",
        "/model ",
        "/model opus",
        "/model sonnet",
        "/model haiku",
        "/permissions ",
        "/permissions read-only",
        "/permissions workspace-write",
        "/permissions danger-full-access",
        "/plugin list",
        "/plugin install ",
        "/plugin enable ",
        "/plugin disable ",
        "/plugin uninstall ",
        "/plugin update ",
        "/plugins list",
        "/pr ",
        "/resume ",
        "/session list",
        "/session switch ",
        "/session fork ",
        "/teleport ",
        "/ultraplan ",
        "/agents help",
        "/mcp help",
        "/skills help",
    ] {
        completions.insert(candidate.to_string());
    }

    if !model.trim().is_empty() {
        completions.insert(format!("/model {}", resolve_model_alias(model)));
        completions.insert(format!("/model {model}"));
    }

    if let Some(active_session_id) = active_session_id.filter(|value| !value.trim().is_empty()) {
        completions.insert(format!("/resume {active_session_id}"));
        completions.insert(format!("/session switch {active_session_id}"));
    }

    for session_id in recent_session_ids
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .take(10)
    {
        completions.insert(format!("/resume {session_id}"));
        completions.insert(format!("/session switch {session_id}"));
    }

    completions.into_iter().collect()
}
