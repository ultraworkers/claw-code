#![recursion_limit = "256"]
#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    clippy::doc_markdown,
    clippy::len_zero,
    clippy::manual_string_new,
    clippy::match_same_arms,
    clippy::result_large_err,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::unneeded_struct_pattern,
    clippy::unnecessary_wraps,
    clippy::unused_self
)]
mod init;
mod input;
mod render;
mod setup_wizard;

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

use log::debug;

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
use tools::{
    canonical_allowed_tool_name, execute_tool, mvp_tool_specs, GlobalToolRegistry,
    RuntimeToolDefinition, ToolSearchOutput,
};

mod bootstrap;
use bootstrap::*;
mod cli_parse;
use cli_parse::*;
mod model_provenance;
use model_provenance::*;
mod preflight;
use preflight::*;
mod progress;
use progress::*;
mod provider_client;
use provider_client::*;
mod skill_dispatch;
use skill_dispatch::*;

const DEFAULT_MODEL: &str = "anthropic/claude-opus-4-7";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PermissionModeSource {
    Flag,
    Env,
    Config,
    Default,
}

impl PermissionModeSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Flag => "flag",
            Self::Env => "env",
            Self::Config => "config",
            Self::Default => "default",
        }
    }

    fn is_explicit(self) -> bool {
        !matches!(self, Self::Default)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PermissionModeProvenance {
    mode: PermissionMode,
    source: PermissionModeSource,
    env_var: Option<&'static str>,
}

impl PermissionModeProvenance {
    fn from_flag(mode: PermissionMode) -> Self {
        Self {
            mode,
            source: PermissionModeSource::Flag,
            env_var: None,
        }
    }

    fn default_fallback() -> Self {
        Self {
            mode: PermissionMode::WorkspaceWrite,
            source: PermissionModeSource::Default,
            env_var: None,
        }
    }
}

struct EnvModel {
    name: &'static str,
    value: String,
}

fn env_model_for_runtime() -> Option<EnvModel> {
    ["CLAW_MODEL", "ANTHROPIC_MODEL", "ANTHROPIC_DEFAULT_MODEL"]
        .into_iter()
        .find_map(|name| {
            env::var(name)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .map(|value| EnvModel { name, value })
        })
}

// Build-time constants injected by build.rs (fall back to static values when
// build.rs hasn't run, e.g. in doc-test or unusual toolchain environments).
const DEFAULT_DATE: &str = match option_env!("BUILD_DATE") {
    Some(d) => d,
    None => "unknown",
};

const DEFAULT_OAUTH_CALLBACK_PORT: u16 = 4545;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const BUILD_TARGET: Option<&str> = option_env!("TARGET");

const GIT_SHA: Option<&str> = option_env!("GIT_SHA");

const GIT_SHA_SHORT: Option<&str> = option_env!("GIT_SHA_SHORT");

const GIT_DIRTY: Option<&str> = option_env!("GIT_DIRTY");

const GIT_BRANCH: Option<&str> = option_env!("GIT_BRANCH");

const GIT_COMMIT_DATE: Option<&str> = option_env!("GIT_COMMIT_DATE");

const GIT_COMMIT_TIMESTAMP: Option<&str> = option_env!("GIT_COMMIT_TIMESTAMP");

const RUSTC_VERSION: Option<&str> = option_env!("RUSTC_VERSION");

const INTERNAL_PROGRESS_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(3);

const POST_TOOL_STALL_TIMEOUT: Duration = Duration::from_secs(10);

const PRIMARY_SESSION_EXTENSION: &str = "jsonl";

const LEGACY_SESSION_EXTENSION: &str = "json";

const OFFICIAL_REPO_URL: &str = "https://github.com/ultraworkers/claw-code";

const OFFICIAL_REPO_SLUG: &str = "ultraworkers/claw-code";

const DEPRECATED_INSTALL_COMMAND: &str = "cargo install claw-code";

const LATEST_SESSION_REFERENCE: &str = "latest";

const SESSION_REFERENCE_ALIASES: &[&str] = &[LATEST_SESSION_REFERENCE, "last", "recent"];

const CLI_OPTION_SUGGESTIONS: &[&str] = &[
    "--help",
    "-h",
    "--version",
    "-V",
    "--model",
    "--output-format",
    "--permission-mode",
    "--cwd",
    "--directory",
    "-C",
    "--skip-permissions",
    "--dangerously-skip-permissions",
    "--allowedTools",
    "--allowed-tools",
    "--resume",
    "--acp",
    "-acp",
    "--print",
    "--compact",
    "--base-commit",
    "-p",
];

fn is_registered_cli_flag_token(value: &str) -> bool {
    let flag = value.split_once('=').map_or(value, |(flag, _)| flag);
    CLI_OPTION_SUGGESTIONS.contains(&flag)
}

fn should_reject_unknown_option_like(value: &str) -> bool {
    is_registered_cli_flag_token(value)
        || (value.starts_with("--")
            && suggest_closest_term(value, CLI_OPTION_SUGGESTIONS).is_some())
}

type AllowedToolSet = BTreeSet<String>;
type RuntimePluginStateBuildOutput = (
    Option<Arc<Mutex<RuntimeMcpState>>>,
    Vec<RuntimeToolDefinition>,
);

fn main() {
    if let Err(error) = run() {
        let message = error.to_string();
        // When --output-format json is active, emit errors as JSON so downstream
        // tools can parse failures the same way they parse successes (ROADMAP #42).
        let argv: Vec<String> = std::env::args().collect();
        let json_output = raw_args_request_json_output(&argv[1..]);
        if json_output {
            // #77/#696: classify error by prefix so downstream claws can route
            // without regex-scraping prose. Keep the legacy `type`/`kind`
            // fields and add the stable status/error_kind/action contract used
            // by non-interactive command guards.
            let kind = classify_error_kind(&message);
            let (short_reason, inline_hint) = split_error_hint(&message);
            // #781: fall back to a kind-derived hint when the message has no \n-delimited hint
            let hint = inline_hint.or_else(|| fallback_hint_for_error_kind(kind).map(String::from));
            let mut error_json = serde_json::json!({
                "type": "error",
                "kind": kind,
                "status": "error",
                "error_kind": kind,
                "error": short_reason,
                "message": short_reason,
                "action": "abort",
                "hint": hint,
                "exit_code": 1,
            });
            if kind == "invalid_cwd" {
                if let Some(error) = error.downcast_ref::<InvalidCwdError>() {
                    if let Some(object) = error_json.as_object_mut() {
                        object.insert("path".to_string(), serde_json::json!(&error.path));
                        object.insert(
                            "reason".to_string(),
                            serde_json::json!(error.reason.as_str()),
                        );
                    }
                }
            } else if kind == "invalid_output_path" {
                if let Some(error) = error.downcast_ref::<InvalidOutputPathError>() {
                    if let Some(object) = error_json.as_object_mut() {
                        object.insert("path".to_string(), serde_json::json!(&error.path));
                        object.insert(
                            "reason".to_string(),
                            serde_json::json!(error.reason.as_str()),
                        );
                    }
                }
            } else if kind == "invalid_output_format" {
                if let Some(object) = error_json.as_object_mut() {
                    object.insert(
                        "value".to_string(),
                        serde_json::json!(invalid_output_format_value(&message)),
                    );
                    object.insert("expected".to_string(), serde_json::json!(["text", "json"]));
                }
            } else if kind == "invalid_tool_name" {
                let (tool_name, available, aliases) = invalid_tool_name_details(&message);
                if let Some(object) = error_json.as_object_mut() {
                    if let Some(tool_name) = tool_name {
                        object.insert("tool_name".to_string(), serde_json::json!(tool_name));
                    }
                    object.insert("available".to_string(), serde_json::json!(available));
                    object.insert("tool_aliases".to_string(), aliases);
                }
            } else if kind == "missing_argument" {
                if let Some(object) = error_json.as_object_mut() {
                    if message.contains("--allowedTools") {
                        object.insert("argument".to_string(), serde_json::json!("--allowedTools"));
                    } else if message.contains("prompt or subcommand") {
                        object.insert(
                            "argument".to_string(),
                            serde_json::json!("prompt or subcommand"),
                        );
                    }
                }
            }
            // #819/#820/#823: JSON mode error envelopes must go to stdout so machine
            // consumers can parse failures from stdout byte 0 (parity with all
            // non-interactive command guards that already use println! / to_stdout).
            println!("{}", error_json);
        } else {
            // #156: Add machine-readable error kind to text output so stderr observers
            // don't need to regex-scrape the prose.
            let kind = classify_error_kind(&message);
            if message.contains("`claw --help`") {
                eprintln!(
                    "[error-kind: {kind}]
error: {message}"
                );
            } else {
                eprintln!(
                    "[error-kind: {kind}]
error: {message}

Run `claw --help` for usage."
                );
            }
        }
        std::process::exit(1);
    }
}

fn invalid_tool_name_details(message: &str) -> (Option<String>, Vec<String>, Value) {
    let tool_name = message
        .strip_prefix("invalid_tool_name: unsupported tool in --allowedTools:")
        .and_then(|rest| rest.lines().next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let available = message
        .lines()
        .find_map(|line| line.strip_prefix("Available:"))
        .map(|line| {
            line.split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let aliases = message
        .lines()
        .find_map(|line| line.strip_prefix("Aliases:"))
        .map(|line| {
            line.split(',')
                .filter_map(|entry| entry.trim().split_once('='))
                .map(|(alias, canonical)| {
                    (
                        alias.trim().to_string(),
                        Value::String(canonical.trim().to_string()),
                    )
                })
                .collect::<Map<_, _>>()
        })
        .unwrap_or_default();
    (tool_name, available, Value::Object(aliases))
}

fn invalid_output_format_value(message: &str) -> Option<String> {
    message
        .strip_prefix("invalid_output_format: unsupported value for --output-format:")
        .and_then(|rest| rest.lines().next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// #781: derive a stable fallback hint from a classified error kind when the error
/// message itself has no `\n`-delimited hint. Returns `None` for kinds where the
/// message is self-explanatory or no canonical remediation exists.
fn fallback_hint_for_error_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "api_auth_error" => {
            Some("Check that ANTHROPIC_API_KEY or ANTHROPIC_AUTH_TOKEN is set and valid.")
        }
        "api_rate_limit_error" => {
            Some("You have hit the API rate limit. Wait and retry, or reduce request frequency.")
        }
        "missing_credentials" => {
            Some("Set ANTHROPIC_API_KEY or ANTHROPIC_AUTH_TOKEN before running claw.")
        }
        "config_parse_error" => Some(
            "Fix the JSON syntax or schema in the referenced .claw/settings.json or .claw.json file, then rerun the command.",
        ),
        // #787: session load failures have no \n-delimited hint from the OS error path
        "session_load_failed" => Some(
            "Pass a path to a .jsonl session file, not a directory. Managed sessions live in .claw/sessions/.",
        ),
        "session_path_is_directory" => Some(
            "--resume expects a .jsonl session file path, not a directory. Run `claw --output-format json /session list` to list managed sessions.",
        ),
        // #793: plugins uninstall/enable/disable of non-existing plugin propagates through
        // the ? operator with no \n delimiter, so split_error_hint returns None.
        "plugin_not_found" => Some("Run `claw plugins list` to see installed plugins."),
        // #794: plugins install with a path that doesn't exist
        "plugin_source_not_found" => Some(
            "Check that the path or URL is correct. Use a local directory or a valid registry id.",
        ),
        // #795: skills install/show of a non-existing skill path or name
        "skill_not_found" => Some(
            "Run `claw skills list` to see available skills, or `claw skills install <path>` to install a new one.",
        ),
        // #795/#431: unsupported/invalid skills lifecycle input should include actionable local guidance.
        "unsupported_skills_action" => Some(
            "Supported: list, show <name>, install <path>, uninstall <name>, help. Run `claw skills help` for details.",
        ),
        "invalid_install_source" => Some(
            "Pass a local skill directory containing SKILL.md or a standalone markdown file.",
        ),
        "invalid_tool_name" => Some(
            "Use canonical snake_case tool names from `available` or documented aliases from `tool_aliases`.",
        ),
        "invalid_output_format" => Some("Use --output-format text or --output-format json."),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InvalidCwdReason {
    Empty,
    NotFound,
    NotADirectory,
}

impl InvalidCwdReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::NotFound => "not_found",
            Self::NotADirectory => "not_a_directory",
        }
    }
}

#[derive(Debug)]
struct InvalidCwdError {
    path: String,
    reason: InvalidCwdReason,
}

impl InvalidCwdError {
    fn new(path: impl Into<String>, reason: InvalidCwdReason) -> Self {
        Self {
            path: path.into(),
            reason,
        }
    }
}

impl std::fmt::Display for InvalidCwdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid_cwd: {}: `{}`\nUsage: --cwd <path>, -C <path>, or --directory <path>",
            self.reason.as_str(),
            self.path
        )
    }
}

impl std::error::Error for InvalidCwdError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InvalidOutputPathReason {
    Empty,
    ParentNotFound,
    ParentNotADirectory,
    PathIsDirectory,
}

impl InvalidOutputPathReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::ParentNotFound => "parent_not_found",
            Self::ParentNotADirectory => "parent_not_a_directory",
            Self::PathIsDirectory => "path_is_directory",
        }
    }
}

#[derive(Debug)]
struct InvalidOutputPathError {
    path: String,
    reason: InvalidOutputPathReason,
}

impl InvalidOutputPathError {
    fn new(path: impl Into<String>, reason: InvalidOutputPathReason) -> Self {
        Self {
            path: path.into(),
            reason,
        }
    }
}

impl std::fmt::Display for InvalidOutputPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid_output_path: {}: `{}`\nUsage: claw export [PATH] [--session SESSION] [--output PATH]",
            self.reason.as_str(),
            self.path
        )
    }
}

impl std::error::Error for InvalidOutputPathError {}

fn split_global_cwd_args(
    args: &[String],
) -> Result<(Vec<String>, Option<PathBuf>), Box<dyn std::error::Error>> {
    let mut filtered = Vec::with_capacity(args.len());
    let mut cwd = None;
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--cwd" | "-C" | "--directory" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "missing_flag_value: missing value for --cwd.\nUsage: --cwd <path>, -C <path>, or --directory <path>",
                    )
                })?;
                cwd = Some(validate_global_cwd(value)?);
                index += 2;
            }
            flag if flag.starts_with("--cwd=") => {
                let value = &flag[6..];
                cwd = Some(validate_global_cwd(value)?);
                index += 1;
            }
            flag if flag.starts_with("--directory=") => {
                let value = &flag[12..];
                cwd = Some(validate_global_cwd(value)?);
                index += 1;
            }
            flag if global_flag_takes_value(flag) => {
                filtered.push(arg.clone());
                if let Some(value) = args.get(index + 1) {
                    filtered.push(value.clone());
                    index += 2;
                } else {
                    index += 1;
                }
            }
            flag if global_flag_is_value_inline(flag) => {
                filtered.push(arg.clone());
                index += 1;
            }
            flag if global_flag_without_value(flag) => {
                filtered.push(arg.clone());
                index += 1;
            }
            "--" => {
                filtered.extend(args[index..].iter().cloned());
                break;
            }
            other if other.starts_with('-') => {
                filtered.push(arg.clone());
                index += 1;
            }
            _ => {
                filtered.extend(args[index..].iter().cloned());
                break;
            }
        }
    }

    Ok((filtered, cwd))
}

fn global_flag_takes_value(flag: &str) -> bool {
    matches!(
        flag,
        "--model"
            | "--output-format"
            | "--permission-mode"
            | "--base-commit"
            | "--reasoning-effort"
            | "--allowedTools"
            | "--allowed-tools"
    )
}

fn global_flag_is_value_inline(flag: &str) -> bool {
    flag.starts_with("--model=")
        || flag.starts_with("--output-format=")
        || flag.starts_with("--permission-mode=")
        || flag.starts_with("--base-commit=")
        || flag.starts_with("--reasoning-effort=")
        || flag.starts_with("--allowedTools=")
        || flag.starts_with("--allowed-tools=")
}

fn global_flag_without_value(flag: &str) -> bool {
    matches!(
        flag,
        "--help"
            | "-h"
            | "--version"
            | "-V"
            | "--dangerously-skip-permissions"
            | "--skip-permissions"
            | "--compact"
            | "--allow-broad-cwd"
            | "--print"
            | "--acp"
            | "-acp"
    )
}

fn validate_global_cwd(value: &str) -> Result<PathBuf, InvalidCwdError> {
    if value.trim().is_empty() {
        return Err(InvalidCwdError::new(value, InvalidCwdReason::Empty));
    }
    let path = PathBuf::from(value);
    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_dir() => Ok(path),
        Ok(_) => Err(InvalidCwdError::new(value, InvalidCwdReason::NotADirectory)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(InvalidCwdError::new(value, InvalidCwdReason::NotFound))
        }
        Err(_) => Err(InvalidCwdError::new(value, InvalidCwdReason::NotFound)),
    }
}

fn apply_global_cwd(cwd: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(cwd) = cwd {
        env::set_current_dir(cwd)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormatSource {
    Default,
    Env,
    Flag,
}

impl OutputFormatSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Env => "env",
            Self::Flag => "flag",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutputFormatSelection {
    format: CliOutputFormat,
    source: OutputFormatSource,
    raw: Option<String>,
    overridden: Vec<String>,
}

impl Default for OutputFormatSelection {
    fn default() -> Self {
        Self {
            format: CliOutputFormat::Text,
            source: OutputFormatSource::Default,
            raw: None,
            overridden: Vec::new(),
        }
    }
}

static OUTPUT_FORMAT_SELECTION: OnceLock<Mutex<OutputFormatSelection>> = OnceLock::new();

// #468: duplicate global flag occurrences for provenance reporting
static DUPLICATE_FLAGS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

fn output_format_selection_cell() -> &'static Mutex<OutputFormatSelection> {
    OUTPUT_FORMAT_SELECTION.get_or_init(|| Mutex::new(OutputFormatSelection::default()))
}

fn duplicate_flags_cell() -> &'static Mutex<Vec<String>> {
    DUPLICATE_FLAGS.get_or_init(|| Mutex::new(Vec::new()))
}

fn push_duplicate_flag(flag: &str) {
    if let Ok(mut flags) = duplicate_flags_cell().lock() {
        flags.push(flag.to_string());
    }
}

fn take_duplicate_flags() -> Vec<String> {
    duplicate_flags_cell()
        .lock()
        .map(|mut flags| std::mem::take(&mut *flags))
        .unwrap_or_default()
}

fn set_current_output_format_selection(selection: &OutputFormatSelection) {
    *output_format_selection_cell()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = selection.clone();
}

fn current_output_format_selection() -> OutputFormatSelection {
    output_format_selection_cell()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

fn cli_has_output_format_flag(args: &[String]) -> bool {
    args.iter()
        .take_while(|arg| arg.as_str() != "--")
        .any(|arg| arg == "--output-format" || arg.starts_with("--output-format="))
}

fn raw_args_request_json_output(args: &[String]) -> bool {
    let mut values = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--" {
            break;
        }
        if arg == "--output-format" {
            if let Some(value) = args.get(index + 1) {
                values.push(value.as_str());
            }
            index += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--output-format=") {
            values.push(value);
        }
        index += 1;
    }
    if let Some(value) = values.last() {
        let value = value.trim();
        return !value.eq_ignore_ascii_case("text");
    }
    env::var("CLAW_OUTPUT_FORMAT").ok().is_some_and(|value| {
        let value = value.trim();
        !value.is_empty() && !value.eq_ignore_ascii_case("text")
    })
}

fn output_format_selection_from_env() -> Result<OutputFormatSelection, String> {
    match env::var("CLAW_OUTPUT_FORMAT") {
        Ok(raw) if !raw.trim().is_empty() => Ok(OutputFormatSelection {
            format: CliOutputFormat::parse(&raw)?,
            source: OutputFormatSource::Env,
            raw: Some(raw),
            overridden: Vec::new(),
        }),
        _ => Ok(OutputFormatSelection::default()),
    }
}

fn apply_output_format_flag(
    selection: &mut OutputFormatSelection,
    value: &str,
) -> Result<CliOutputFormat, String> {
    let parsed = CliOutputFormat::parse(value)?;
    if selection.source == OutputFormatSource::Flag {
        let previous = selection
            .raw
            .clone()
            .unwrap_or_else(|| selection.format.as_str().to_string());
        eprintln!("warning: --output-format specified multiple times; using last value '{value}'");
        selection.overridden.push(previous);
    }
    selection.format = parsed;
    selection.source = OutputFormatSource::Flag;
    selection.raw = Some(value.to_string());
    set_current_output_format_selection(selection);
    Ok(parsed)
}

fn compact_interactive_only_error() -> String {
    // #749: newline before remediation so split_error_hint populates hint field
    "interactive_only: `claw compact` is an interactive/session command.\nStart `claw` and run `/compact`, or use `claw --resume SESSION.jsonl /compact` to compact an existing session."
        .to_string()
}

fn unexpected_diff_args_error(extra: &[String]) -> String {
    format!(
        "unexpected extra arguments after `claw diff`: {}\nUsage: claw diff",
        extra.join(" ")
    )
}

fn is_known_top_level_subcommand(value: &str) -> bool {
    matches!(
        value,
        "help"
            | "version"
            | "status"
            | "sandbox"
            | "doctor"
            | "state"
            | "dump-manifests"
            | "bootstrap-plan"
            | "agents"
            | "agent"
            | "mcp"
            | "skills"
            | "skill"
            | "plugins"
            | "plugin"
            | "marketplace"
            | "system-prompt"
            | "acp"
            | "init"
            | "export"
            | "prompt"
            | "resume"
            | "session"
            | "compact"
            | "config"
            | "model"
            | "models"
            | "settings"
            | "diff"
    )
}

fn is_bare_provider_model(model: &str) -> bool {
    model.starts_with("claude-") || model.starts_with("gpt-")
}

fn is_local_openai_model_syntax(model: &str) -> bool {
    if let Some(rest) = model.strip_prefix("local/") {
        return !rest.is_empty() && rest.split('/').all(|segment| !segment.is_empty());
    }
    std::env::var_os("OPENAI_BASE_URL").is_some() && (model.contains(':') || model.contains('.'))
}

fn allowed_tools_missing_error() -> String {
    "missing_argument: --allowedTools requires a tool list before subcommands or flags.\nUsage: --allowedTools <tool-name>[,<tool-name>...]  e.g. --allowedTools read,glob".to_string()
}

fn compact_missing_argument_error() -> String {
    "missing_argument: --compact requires prompt text, piped stdin, or a subcommand. argument: prompt or subcommand\nUsage: claw --compact <prompt>  or  echo '<prompt>' | claw --compact"
        .to_string()
}

fn allowed_tool_aliases_json(registry: &GlobalToolRegistry) -> Value {
    Value::Object(
        registry
            .allowed_tool_aliases()
            .into_iter()
            .map(|(alias, canonical)| (alias, Value::String(canonical)))
            .collect(),
    )
}

fn permission_mode_provenance_for_current_dir() -> PermissionModeProvenance {
    if let Some(mode) = env::var("RUSTY_CLAUDE_PERMISSION_MODE")
        .ok()
        .as_deref()
        .and_then(normalize_permission_mode)
        .map(permission_mode_from_label)
    {
        return PermissionModeProvenance {
            mode,
            source: PermissionModeSource::Env,
            env_var: Some("RUSTY_CLAUDE_PERMISSION_MODE"),
        };
    }

    if let Some(mode) = config_permission_mode_for_current_dir() {
        return PermissionModeProvenance {
            mode,
            source: PermissionModeSource::Config,
            env_var: None,
        };
    }

    PermissionModeProvenance::default_fallback()
}

fn print_model_validation_warning_status(
    error: &str,
    usage: StatusUsage,
    permission_mode: &str,
    context: &StatusContext,
    allowed_tools: Option<&AllowedToolSet>,
) -> Result<(), Box<dyn std::error::Error>> {
    let kind = classify_error_kind(error);
    let (short_reason, inline_hint) = split_error_hint(error);
    let hint = inline_hint.or_else(|| fallback_hint_for_error_kind(kind).map(String::from));
    let format_selection = current_output_format_selection();
    let mut value = status_json_value(
        None,
        usage,
        permission_mode,
        context,
        None,
        None,
        allowed_tools,
        Some(&format_selection),
    );
    let object = value
        .as_object_mut()
        .expect("status_json_value should render an object");
    object.insert("status".to_string(), serde_json::json!("warn"));
    object.insert("error_kind".to_string(), serde_json::json!(kind));
    object.insert(
        "model_validation_error".to_string(),
        serde_json::json!(short_reason),
    );
    object.insert(
        "model_validation_error_kind".to_string(),
        serde_json::json!(kind),
    );
    object.insert("model_validation_hint".to_string(), serde_json::json!(hint));
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ConfigWarningMode {
    EmitStderr,
    SuppressStderr,
}

fn load_config_with_warning_mode(
    loader: &ConfigLoader,
    mode: ConfigWarningMode,
) -> Result<runtime::RuntimeConfig, runtime::ConfigError> {
    match mode {
        ConfigWarningMode::EmitStderr => loader.load(),
        ConfigWarningMode::SuppressStderr => loader
            .load_collecting_warnings()
            .map(|(runtime_config, _warnings)| runtime_config),
    }
}

/// Run the interactive setup wizard to configure provider, API key, and model.
fn run_setup() -> Result<(), Box<dyn std::error::Error>> {
    setup_wizard::run_setup_wizard()
}

/// #466: validate provider BASE_URL env vars
fn check_base_url_health() -> DiagnosticCheck {
    let base_url_vars = [
        ("ANTHROPIC_BASE_URL", "https://api.anthropic.com"),
        ("OPENAI_BASE_URL", "https://api.openai.com"),
        ("XAI_BASE_URL", "https://api.x.ai"),
        ("DASHSCOPE_BASE_URL", "https://dashscope.aliyuncs.com"),
    ];
    let mut issues: Vec<String> = Vec::new();
    let mut details: Vec<String> = Vec::new();
    for (var_name, default_url) in &base_url_vars {
        if let Ok(value) = env::var(var_name) {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                issues.push(format!("{var_name} is empty"));
                details.push(format!(
                    "{var_name}  empty (will use default: {default_url})"
                ));
            } else if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
                issues.push(format!("{var_name}={trimmed} is not a valid HTTP(S) URL"));
                details.push(format!("{var_name}  invalid ({trimmed})"));
            } else {
                details.push(format!("{var_name}  {trimmed}"));
            }
        }
    }
    if issues.is_empty() {
        DiagnosticCheck::new(
            "Base URLs",
            DiagnosticLevel::Ok,
            "provider base URL env vars are valid or unset",
        )
        .with_details(details)
    } else {
        DiagnosticCheck::new(
            "Base URLs",
            DiagnosticLevel::Warn,
            format!("{} base URL issue(s) found", issues.len()),
        )
        .with_details(details)
        .with_hint("Fix the reported BASE_URL env vars or unset them to use provider defaults.")
    }
}

fn check_mcp_validation_health(summary: &McpValidationSummary) -> DiagnosticCheck {
    let mut details = vec![
        format!("Total entries     {}", summary.total_configured),
        format!("Valid entries     {}", summary.valid_count),
        format!("Invalid entries   {}", summary.invalid_count()),
    ];
    details.extend(
        summary
            .invalid_servers
            .iter()
            .map(|server| format!("Invalid server   {} ({})", server.name, server.reason)),
    );

    DiagnosticCheck::new(
        "MCP validation",
        if summary.has_invalid_servers() {
            DiagnosticLevel::Warn
        } else {
            DiagnosticLevel::Ok
        },
        if summary.has_invalid_servers() {
            format!(
                "{} MCP server entries are invalid; {} valid entries remain loaded",
                summary.invalid_count(),
                summary.valid_count
            )
        } else {
            format!("{} MCP server entries validated", summary.valid_count)
        },
    )
    .with_hint(if summary.has_invalid_servers() {
        "Inspect `claw mcp list --output-format json` invalid_servers and fix each rejected mcpServers entry."
    } else {
        ""
    })
    .with_details(details)
    .with_data(Map::from_iter([
        (
            "total_configured".to_string(),
            json!(summary.total_configured),
        ),
        ("valid_count".to_string(), json!(summary.valid_count)),
        ("invalid_count".to_string(), json!(summary.invalid_count())),
        (
            "invalid_servers".to_string(),
            Value::Array(invalid_mcp_servers_json(&summary.invalid_servers)),
        ),
    ]))
}

fn check_hook_validation_health(summary: &HookValidationSummary) -> DiagnosticCheck {
    let mut details = vec![
        format!("Valid entries     {}", summary.valid_count),
        format!("Invalid entries   {}", summary.invalid_count()),
    ];
    details.extend(
        summary
            .invalid_hooks
            .iter()
            .map(|hook| format!("Invalid hook     {} ({})", hook.event, hook.reason)),
    );

    DiagnosticCheck::new(
        "Hook validation",
        if summary.has_invalid_hooks() {
            DiagnosticLevel::Warn
        } else {
            DiagnosticLevel::Ok
        },
        if summary.has_invalid_hooks() {
            format!(
                "{} hook entries are invalid; {} valid entries remain loaded",
                summary.invalid_count(),
                summary.valid_count
            )
        } else {
            format!("{} hook entries validated", summary.valid_count)
        },
    )
    .with_hint(if summary.has_invalid_hooks() {
        "Inspect `claw status --output-format json` hook_validation.invalid_hooks and fix each rejected hooks entry."
    } else {
        ""
    })
    .with_details(details)
    .with_data(Map::from_iter([
        ("valid_count".to_string(), json!(summary.valid_count)),
        ("invalid_count".to_string(), json!(summary.invalid_count())),
        (
            "invalid_hooks".to_string(),
            Value::Array(invalid_hooks_json(&summary.invalid_hooks)),
        ),
    ]))
}

fn check_permission_health(permission_mode: PermissionModeProvenance) -> DiagnosticCheck {
    let mode = permission_mode.mode.as_str();
    let source = permission_mode.source.as_str();
    let explicit = permission_mode.source.is_explicit();
    let warning = matches!(permission_mode.mode, PermissionMode::DangerFullAccess) && !explicit;
    let message = if warning {
        "running with full access without explicit opt-in"
    } else if matches!(permission_mode.mode, PermissionMode::DangerFullAccess) {
        "danger-full-access was explicitly selected"
    } else if matches!(permission_mode.mode, PermissionMode::WorkspaceWrite) && !explicit {
        "default permission mode is workspace-write"
    } else {
        "permission mode is explicitly bounded below danger-full-access"
    };
    let source_detail = permission_mode.env_var.map_or_else(
        || source.to_string(),
        |env_var| format!("{source}:{env_var}"),
    );
    let specs = mvp_tool_specs();
    let tools_satisfied = specs
        .iter()
        .filter(|spec| permission_mode.mode >= spec.required_permission)
        .map(|spec| spec.name)
        .collect::<Vec<_>>();
    let tools_gated = specs
        .iter()
        .filter(|spec| permission_mode.mode < spec.required_permission)
        .map(|spec| spec.name)
        .collect::<Vec<_>>();

    DiagnosticCheck::new(
        "Permissions",
        if warning {
            DiagnosticLevel::Warn
        } else {
            DiagnosticLevel::Ok
        },
        message,
    )
    .with_details(vec![
        format!("Mode             {mode}"),
        format!("Source           {source_detail}"),
        format!("Explicit opt-in  {explicit}"),
        format!("Tools allowed    {}", tools_satisfied.join(", ")),
        format!("Tools gated      {}", tools_gated.join(", ")),
    ])
    .with_hint(if warning {
        "Use the workspace-write default, or pass --permission-mode danger-full-access / --dangerously-skip-permissions only when full filesystem, network, and command access is intentional."
    } else {
        "Use --permission-mode read-only|workspace-write|danger-full-access to make the runtime permission boundary explicit."
    })
    .with_data(Map::from_iter([
        ("mode".to_string(), json!(mode)),
        ("source".to_string(), json!(source)),
        ("source_explicit".to_string(), json!(explicit)),
        ("env_var".to_string(), json!(permission_mode.env_var)),
        ("message".to_string(), json!(message)),
        ("tools_satisfied".to_string(), json!(tools_satisfied)),
        ("tools_gated".to_string(), json!(tools_gated)),
    ]))
}

fn check_memory_health(context: &StatusContext) -> DiagnosticCheck {
    let has_unloaded = !context.unloaded_memory_files.is_empty();
    let has_outside_project = context.memory_files.iter().any(|file| file.outside_project);
    let mut details = vec![format!("Loaded files     {}", context.memory_file_count)];
    details.extend(context.memory_files.iter().map(|file| {
        format!(
            "Loaded          {} ({}, chars={})",
            file.path, file.source, file.chars
        )
    }));
    details.extend(
        context
            .unloaded_memory_files
            .iter()
            .map(|path| format!("Unloaded        {path}")),
    );

    DiagnosticCheck::new(
        "Memory",
        if has_unloaded || has_outside_project {
            DiagnosticLevel::Warn
        } else {
            DiagnosticLevel::Ok
        },
        if has_outside_project {
            "memory files outside the current git project are loaded".to_string()
        } else if has_unloaded {
            "some workspace memory files exist but were not loaded".to_string()
        } else {
            format!("{} workspace memory files loaded", context.memory_file_count)
        },
    )
    .with_hint(if has_outside_project {
        "Inspect workspace.memory_files in `claw status --output-format json`; move unintended ancestor instructions inside the git project or run from the intended workspace root."
    } else if has_unloaded {
        "Move instructions into CLAUDE.md, CLAW.md, or AGENTS.md within the current workspace ancestry, or inspect workspace.memory_files in `claw status --output-format json`."
    } else {
        ""
    })
    .with_details(details)
    .with_data(Map::from_iter([
        (
            "memory_file_count".to_string(),
            json!(context.memory_file_count),
        ),
        (
            "memory_files".to_string(),
            Value::Array(memory_files_json(&context.memory_files)),
        ),
        (
            "unloaded_memory_files".to_string(),
            json!(context.unloaded_memory_files),
        ),
    ]))
}

const DUMP_MANIFESTS_USAGE_HINT: &str =
    "Usage: claw dump-manifests [--manifests-dir <path>] [--output-format json]";

fn build_rust_resolver_manifest(workspace_dir: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let command_entries = slash_command_specs()
        .iter()
        .map(|spec| {
            json!({
                "name": spec.name,
                "aliases": spec.aliases,
                "summary": spec.summary,
                "argument_hint": spec.argument_hint,
                "resume_supported": spec.resume_supported,
                "implemented": !STUB_COMMANDS.contains(&spec.name),
            })
        })
        .collect::<Vec<_>>();

    let tool_entries = mvp_tool_specs()
        .into_iter()
        .map(|spec| {
            json!({
                "name": spec.name,
                "description": spec.description,
                "required_permission": spec.required_permission.as_str(),
                "input_schema": spec.input_schema,
            })
        })
        .collect::<Vec<_>>();

    let agent_report = handle_agents_slash_command_json(None, workspace_dir)?;
    let skill_report = handle_skills_slash_command_json(None, workspace_dir)?;
    let agents = agent_report
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let skills = skill_report
        .get("skills")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let bootstrap = runtime::BootstrapPlan::claude_code_default()
        .phases()
        .iter()
        .map(|phase| format!("{phase:?}"))
        .collect::<Vec<_>>();

    Ok(json!({
        "kind": "dump-manifests",
        "action": "dump",
        "status": "ok",
        "source": "rust-resolver",
        "workspace": workspace_dir.display().to_string(),
        "commands": command_entries.len(),
        "tools": tool_entries.len(),
        "agents": agents.len(),
        "skills": skills.len(),
        "bootstrap_phases": bootstrap.len(),
        "command_manifests": command_entries,
        "tool_manifests": tool_entries,
        "agent_manifests": agents,
        "skill_manifests": skills,
        "bootstrap_manifest": bootstrap,
    }))
}

fn bootstrap_phase_metadata(phase: &runtime::BootstrapPhase) -> (&'static str, &'static str) {
    use runtime::BootstrapPhase::*;
    match phase {
        CliEntry => (
            "CLI Entry",
            "Command-line argument parsing and global flag resolution",
        ),
        FastPathVersion => (
            "Fast-Path Version",
            "Short-circuit version/help requests before full startup",
        ),
        StartupProfiler => (
            "Startup Profiler",
            "Instrument startup timing for diagnostics",
        ),
        SystemPromptFastPath => (
            "System Prompt Fast-Path",
            "Serve system-prompt requests without provider init",
        ),
        ChromeMcpFastPath => (
            "Chrome MCP Fast-Path",
            "Serve Chrome MCP requests without full runtime",
        ),
        DaemonWorkerFastPath => (
            "Daemon Worker Fast-Path",
            "Handle daemon worker requests without full init",
        ),
        BridgeFastPath => (
            "Bridge Fast-Path",
            "Bridge/sibling process communication without full init",
        ),
        DaemonFastPath => (
            "Daemon Fast-Path",
            "Daemon lifecycle management without full runtime",
        ),
        BackgroundSessionFastPath => (
            "Background Session Fast-Path",
            "Resume/list background sessions without full init",
        ),
        TemplateFastPath => (
            "Template Fast-Path",
            "Template rendering without full runtime",
        ),
        EnvironmentRunnerFastPath => (
            "Environment Runner Fast-Path",
            "Environment/runner dispatch without full init",
        ),
        MainRuntime => (
            "Main Runtime",
            "Full interactive REPL or one-shot prompt execution",
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoryFileSummary {
    path: String,
    source: String,
    chars: usize,
    origin: String,
    scope_path: String,
    outside_project: bool,
    contributes: bool,
}

impl MemoryFileSummary {
    fn json_value(&self) -> serde_json::Value {
        json!({
            "path": self.path,
            "source": self.source,
            "chars": self.chars,
            "origin": self.origin,
            "scope_path": self.scope_path,
            "outside_project": self.outside_project,
            "contributes": self.contributes,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct McpValidationSummary {
    total_configured: usize,
    valid_count: usize,
    invalid_servers: Vec<McpInvalidServerConfig>,
}

impl McpValidationSummary {
    fn from_collection(collection: &McpConfigCollection) -> Self {
        Self {
            total_configured: collection.total_configured(),
            valid_count: collection.valid_count(),
            invalid_servers: collection.invalid_servers().to_vec(),
        }
    }

    fn invalid_count(&self) -> usize {
        self.invalid_servers.len()
    }

    fn has_invalid_servers(&self) -> bool {
        !self.invalid_servers.is_empty()
    }

    fn json_value(&self) -> serde_json::Value {
        json!({
            "total_configured": self.total_configured,
            "valid_count": self.valid_count,
            "invalid_count": self.invalid_count(),
            "invalid_servers": invalid_mcp_servers_json(&self.invalid_servers),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct HookValidationSummary {
    valid_count: usize,
    invalid_hooks: Vec<RuntimeInvalidHookConfig>,
}

impl HookValidationSummary {
    fn from_config(config: &runtime::RuntimeConfig) -> Self {
        let hooks = config.hooks();
        Self {
            valid_count: hooks.pre_tool_use_entries().len()
                + hooks.post_tool_use_entries().len()
                + hooks.post_tool_use_failure_entries().len(),
            invalid_hooks: hooks.invalid_hooks().to_vec(),
        }
    }

    fn invalid_count(&self) -> usize {
        self.invalid_hooks.len()
    }

    fn has_invalid_hooks(&self) -> bool {
        !self.invalid_hooks.is_empty()
    }

    fn json_value(&self) -> serde_json::Value {
        json!({
            "valid_count": self.valid_count,
            "invalid_count": self.invalid_count(),
            "invalid_hooks": invalid_hooks_json(&self.invalid_hooks),
        })
    }
}

fn invalid_hooks_json(invalid_hooks: &[RuntimeInvalidHookConfig]) -> Vec<serde_json::Value> {
    invalid_hooks
        .iter()
        .map(|hook| {
            json!({
                "event": &hook.event,
                "index": hook.index,
                "hook_index": hook.hook_index,
                "kind": &hook.kind,
                "error_field": &hook.error_field,
                "reason": &hook.reason,
                "valid": false,
            })
        })
        .collect()
}

fn invalid_mcp_servers_json(invalid_servers: &[McpInvalidServerConfig]) -> Vec<serde_json::Value> {
    invalid_servers
        .iter()
        .map(|server| {
            json!({
                "name": &server.name,
                "scope": config_source_json_value(server.scope),
                "path": server.path.display().to_string(),
                "error_field": &server.error_field,
                "reason": &server.reason,
                "valid": false,
            })
        })
        .collect()
}

fn config_source_json_value(source: ConfigSource) -> serde_json::Value {
    let id = match source {
        ConfigSource::User => "user",
        ConfigSource::Project => "project",
        ConfigSource::Local => "local",
    };
    json!({"id": id, "label": id})
}

fn memory_file_summaries_for(
    cwd: &Path,
    project_root: Option<&Path>,
    files: &[ContextFile],
) -> Vec<MemoryFileSummary> {
    let cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    let project_root =
        project_root.map(|path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf()));
    files
        .iter()
        .map(|file| {
            let path = file
                .path
                .canonicalize()
                .unwrap_or_else(|_| file.path.clone());
            let scope_path = memory_scope_path(&path);
            let origin = memory_origin(&cwd, project_root.as_deref(), &scope_path);
            let outside_project = project_root
                .as_ref()
                .is_some_and(|root| !path.starts_with(root));
            MemoryFileSummary {
                path: file.path.display().to_string(),
                source: file.source().to_string(),
                origin: origin.to_string(),
                scope_path: scope_path.display().to_string(),
                chars: file.char_count(),
                outside_project,
                contributes: true,
            }
        })
        .collect()
}

fn memory_scope_path(path: &Path) -> PathBuf {
    let Some(parent) = path.parent() else {
        return PathBuf::from(".");
    };
    let parent_name = parent.file_name().and_then(|name| name.to_str());
    if matches!(parent_name, Some(".claw" | ".claude")) {
        return parent.parent().unwrap_or(parent).to_path_buf();
    }
    if matches!(parent_name, Some("rules" | "rules.local")) {
        if let Some(grandparent) = parent.parent() {
            if grandparent.file_name().and_then(|name| name.to_str()) == Some(".claw") {
                return grandparent.parent().unwrap_or(grandparent).to_path_buf();
            }
        }
    }
    parent.to_path_buf()
}

fn memory_origin(cwd: &Path, project_root: Option<&Path>, scope_path: &Path) -> &'static str {
    if scope_path == cwd {
        return "workspace";
    }
    if project_root.is_some_and(|root| !scope_path.starts_with(root)) {
        return "outside_project";
    }
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        let home = home.canonicalize().unwrap_or(home);
        if scope_path == home {
            return "home";
        }
    }
    if cwd.parent().is_some_and(|parent| parent == scope_path) {
        return "parent_dir";
    }
    if cwd.starts_with(scope_path) {
        return "ancestor";
    }
    "workspace"
}

fn memory_files_json(files: &[MemoryFileSummary]) -> Vec<serde_json::Value> {
    files.iter().map(MemoryFileSummary::json_value).collect()
}

fn unloaded_memory_candidates(
    cwd: &Path,
    project_root: Option<&Path>,
    files: &[MemoryFileSummary],
) -> Vec<String> {
    let mut loaded = files
        .iter()
        .map(|file| PathBuf::from(&file.path))
        .collect::<Vec<_>>();
    loaded.sort();

    let boundary = project_root.unwrap_or(cwd);
    let mut missing = Vec::new();
    let mut cursor = Some(cwd);
    while let Some(dir) = cursor {
        for name in ["CLAW.md", "AGENTS.md"] {
            let candidate = dir.join(name);
            if candidate.is_file() && !loaded.iter().any(|path| path == &candidate) {
                missing.push(candidate.display().to_string());
            }
        }
        if dir == boundary {
            break;
        }
        cursor = dir.parent();
    }
    missing.sort();
    missing.dedup();
    missing
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BinaryProvenance {
    git_sha: Option<String>,
    git_sha_short: Option<String>,
    is_dirty: bool,
    branch: Option<String>,
    commit_date: String,
    commit_timestamp: i64,
    rustc_version: String,
    target: Option<String>,
    build_date: String,
    executable_path: Option<String>,
    workspace_git_sha: Option<String>,
    workspace_match: Option<bool>,
    hint: Option<String>,
}

impl BinaryProvenance {
    fn status(&self) -> &'static str {
        if self.git_sha.is_some() {
            "known"
        } else {
            "unknown"
        }
    }

    fn json_value(&self) -> serde_json::Value {
        json!({
            "status": self.status(),
            "git_sha": self.git_sha,
            "git_sha_short": self.git_sha_short,
            "is_dirty": self.is_dirty,
            "branch": self.branch,
            "commit_date": self.commit_date,
            "commit_timestamp": self.commit_timestamp,
            "rustc_version": self.rustc_version,
            "target": self.target,
            "build_date": self.build_date,
            "executable_path": self.executable_path,
            "workspace_git_sha": self.workspace_git_sha,
            "workspace_match": self.workspace_match,
            "hint": self.hint,
        })
    }
}

fn known_build_metadata(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() || value == "unknown" {
        None
    } else {
        Some(value.to_string())
    }
}

fn parse_build_bool(value: Option<&str>) -> bool {
    value
        .map(str::trim)
        .is_some_and(|value| value.eq_ignore_ascii_case("true") || value == "1")
}

fn parse_build_timestamp(value: Option<&str>) -> i64 {
    value
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(0)
}

fn binary_provenance_for(cwd: Option<&Path>) -> BinaryProvenance {
    let git_sha = known_build_metadata(GIT_SHA);
    let git_sha_short = known_build_metadata(GIT_SHA_SHORT).or_else(|| {
        git_sha
            .as_ref()
            .map(|sha| sha.chars().take(12).collect::<String>())
    });
    let target = known_build_metadata(BUILD_TARGET);
    let workspace_git_sha = cwd.and_then(|cwd| {
        run_git_capture_in(cwd, &["rev-parse", "HEAD"])
            .map(|sha| sha.trim().to_string())
            .filter(|sha| !sha.is_empty())
    });
    let workspace_match = git_sha
        .as_deref()
        .zip(workspace_git_sha.as_deref())
        .map(|(binary, workspace)| binary == workspace);
    let hint = if git_sha.is_none() {
        Some(
            "Build metadata did not include a git SHA; rebuild from a git checkout before filing provenance-sensitive dogfood reports."
                .to_string(),
        )
    } else if workspace_match == Some(false) {
        Some(
            "The running binary was built from a different commit than the current workspace HEAD; rebuild or switch binaries before attributing behavior to this checkout."
                .to_string(),
        )
    } else {
        None
    };
    BinaryProvenance {
        git_sha,
        git_sha_short,
        is_dirty: parse_build_bool(GIT_DIRTY),
        branch: known_build_metadata(GIT_BRANCH),
        commit_date: known_build_metadata(GIT_COMMIT_DATE).unwrap_or_else(|| "unknown".to_string()),
        commit_timestamp: parse_build_timestamp(GIT_COMMIT_TIMESTAMP),
        rustc_version: known_build_metadata(RUSTC_VERSION).unwrap_or_else(|| "unknown".to_string()),
        target,
        build_date: DEFAULT_DATE.to_string(),
        executable_path: env::current_exe()
            .ok()
            .map(|path| path.display().to_string()),
        workspace_git_sha,
        workspace_match,
        hint,
    }
}

/// #89: mid-operation git states detected from branch header in `git status --short --branch`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum GitOperation {
    #[default]
    None,
    Rebase,
    Merge,
    CherryPick,
    Bisect,
}

impl GitOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Rebase => "rebase-in-progress",
            Self::Merge => "merge-in-progress",
            Self::CherryPick => "cherry-pick-in-progress",
            Self::Bisect => "bisect-in-progress",
        }
    }
}

fn load_session_reference_excluding(
    reference: &str,
    exclude_id: Option<&str>,
) -> Result<(SessionHandle, Session), Box<dyn std::error::Error>> {
    let store = current_session_store()?;
    let loaded = store
        .load_session_excluding(reference, exclude_id)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok((
        SessionHandle {
            id: loaded.handle.id,
            path: loaded.handle.path,
        },
        loaded.session,
    ))
}

/// #449: credentials-free session list that works without API keys.
/// `claw session list --output-format json` should work in CI/offline.
fn run_session_list(output_format: CliOutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let sessions = list_managed_sessions().unwrap_or_default();
    let session_ids: Vec<String> = sessions.iter().map(|s| s.id.clone()).collect();
    let session_details = session_details_json(&sessions);
    match output_format {
        CliOutputFormat::Text => {
            let text = render_session_list("").unwrap_or_else(|e| format!("error: {e}"));
            println!("{text}");
        }
        CliOutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "kind": "sessions",
                    "status": "ok",
                    "action": "list",
                    "sessions": session_ids,
                    "session_details": session_details,
                    "active": serde_json::Value::Null,
                })
            );
        }
    }
    Ok(())
}

/// #421: Strip macOS `/private` symlink prefix from paths so that
/// `status`, `doctor`, and `mcp list` JSON output matches the
/// user-visible invocation cwd instead of the canonicalized path.
fn friendly_cwd(path: PathBuf) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Ok(stripped) = path.strip_prefix("/private") {
            if stripped.is_absolute() {
                return stripped.to_path_buf();
            }
        }
    }
    path
}

fn print_models(
    action: Option<&str>,
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let help_requested = action.is_some_and(|value| matches!(value, "help" | "--help" | "-h"));
    if help_requested {
        return print_help_topic(LocalHelpTopic::Model, output_format);
    }
    if let Some(action) = action {
        return Err(format!(
            "unsupported_models_action: unsupported models action: {action}.\nUsage: claw models [help] [--output-format json]"
        )
        .into());
    }

    let configured_model = config_model_for_current_dir();
    let resolved_config_model = configured_model
        .as_deref()
        .map(resolve_model_alias_with_config);

    match output_format {
        CliOutputFormat::Text => {
            println!("Models");
            println!("  Default          {DEFAULT_MODEL}");
            println!("  Built-in aliases opus, sonnet, haiku");
            if let Some(raw) = configured_model.as_deref() {
                println!(
                    "  Config model     {raw}{}",
                    resolved_config_model
                        .as_deref()
                        .filter(|resolved| *resolved != raw)
                        .map(|resolved| format!(" -> {resolved}"))
                        .unwrap_or_default()
                );
            } else {
                println!("  Config model     <unset>");
            }
            println!("  Usage            claw --model <provider/model> prompt <text>");
        }
        CliOutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "kind": "models",
                    "action": "list",
                    "status": "ok",
                    "default_model": DEFAULT_MODEL,
                    "aliases": [
                        {"name": "opus", "model": resolve_model_alias("opus")},
                        {"name": "sonnet", "model": resolve_model_alias("sonnet")},
                        {"name": "haiku", "model": resolve_model_alias("haiku")}
                    ],
                    "configured_model": configured_model,
                    "resolved_configured_model": resolved_config_model,
                    "local_only": true,
                    "requires_credentials": false,
                    "requires_provider_request": false,
                    "message": "Use --model <provider/model> or configure a model in claw settings."
                }))?
            );
        }
    }
    Ok(())
}

fn render_doctor_help_json() -> serde_json::Value {
    json!({
        "kind": "help",
        "action": "help",
        "status": "ok",
        "topic": "doctor",
        "command": "doctor",
        "schema_version": "1.0",
        "usage": "claw doctor [--output-format <format>]",
        "purpose": "diagnose local auth, config, workspace memory, permissions, sandbox, boot preflight, and build metadata",
        "formats": ["text", "json"],
        "local_only": true,
        "requires_credentials": false,
        "requires_provider_request": false,
        "requires_session_resume": false,
        "mutates_workspace": false,
        "output_fields": ["kind", "action", "status", "message", "report", "has_failures", "summary", "checks", "allowed_tools"],
        "check_names": ["auth", "config", "mcp validation", "hook validation", "install source", "workspace", "memory", "boot preflight", "sandbox", "permissions", "system"],
        "status_values": ["ok", "warn", "fail"],
        "options": [
            {
                "name": "--output-format",
                "value": "<format>",
                "values": ["text", "json"],
                "default": "text",
                "description": "format for the doctor report or help envelope"
            },
            {
                "name": "--help",
                "aliases": ["-h"],
                "description": "show help for the doctor command without running diagnostics"
            }
        ],
        "related": ["/doctor", "claw --resume latest /doctor"],
        "message": render_help_topic(LocalHelpTopic::Doctor),
    })
}

/// #683-#692: extract structured metadata from help prose
fn extract_help_metadata(
    topic: LocalHelpTopic,
) -> (
    Option<String>,      // usage
    Option<String>,      // purpose
    Option<String>,      // output description
    Option<Vec<String>>, // formats
    Option<Vec<String>>, // related
    Option<Vec<String>>, // aliases
    bool,                // local_only
    bool,                // requires_credentials
) {
    let text = render_help_topic(topic);
    let mut usage = None;
    let mut purpose = None;
    let mut output_desc = None;
    let formats = Some(vec!["text".to_string(), "json".to_string()]);
    let mut related = None;
    let mut aliases = None;
    let local_only = matches!(
        topic,
        LocalHelpTopic::Status
            | LocalHelpTopic::Sandbox
            | LocalHelpTopic::Doctor
            | LocalHelpTopic::Version
            | LocalHelpTopic::State
            | LocalHelpTopic::Init
            | LocalHelpTopic::Export
            | LocalHelpTopic::SystemPrompt
            | LocalHelpTopic::DumpManifests
            | LocalHelpTopic::BootstrapPlan
    );
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Usage") {
            let value = rest.trim();
            if !value.is_empty() {
                usage = Some(value.to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("Purpose") {
            purpose = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("Output") {
            output_desc = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("Aliases") {
            let parts: Vec<String> = rest
                .split('·')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !parts.is_empty() {
                aliases = Some(parts);
            }
        } else if let Some(rest) = trimmed.strip_prefix("Related") {
            let parts: Vec<String> = rest
                .split('·')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !parts.is_empty() {
                related = Some(parts);
            }
        }
    }
    (
        usage,
        purpose,
        output_desc,
        formats,
        related,
        aliases,
        local_only,
        !local_only,
    )
}

fn config_file_report_json(file: &ConfigFileReport) -> serde_json::Value {
    let source = match file.entry.source {
        ConfigSource::User => "user",
        ConfigSource::Project => "project",
        ConfigSource::Local => "local",
    };
    let mut object = serde_json::Map::new();
    object.insert(
        "path".to_string(),
        serde_json::Value::String(file.entry.path.display().to_string()),
    );
    object.insert(
        "source".to_string(),
        serde_json::Value::String(source.to_string()),
    );
    object.insert("loaded".to_string(), serde_json::Value::Bool(file.loaded));
    object.insert(
        "precedence_rank".to_string(),
        serde_json::Value::Number(serde_json::Number::from(file.precedence_rank)),
    );
    object.insert(
        "wins_for_keys".to_string(),
        serde_json::Value::Array(
            file.wins_for_keys
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        ),
    );
    object.insert(
        "shadowed_keys".to_string(),
        serde_json::Value::Array(
            file.shadowed_keys
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        ),
    );
    object.insert(
        "status".to_string(),
        serde_json::Value::String(file.status.as_str().to_string()),
    );
    if let Some(reason) = &file.reason {
        object.insert(
            "reason".to_string(),
            serde_json::Value::String(reason.clone()),
        );
        object.insert(
            "skip_reason".to_string(),
            serde_json::Value::String(reason.clone()),
        );
    }
    if let Some(detail) = &file.detail {
        object.insert(
            "detail".to_string(),
            serde_json::Value::String(detail.clone()),
        );
    }
    serde_json::Value::Object(object)
}

const DEFAULT_HISTORY_LIMIT: usize = 20;

fn validate_export_output_path(path: Option<&Path>) -> Result<(), InvalidOutputPathError> {
    let Some(path) = path else {
        return Ok(());
    };
    let raw = path.to_string_lossy();
    if raw.trim().is_empty() {
        return Err(InvalidOutputPathError::new(
            raw.to_string(),
            InvalidOutputPathReason::Empty,
        ));
    }
    if matches!(fs::metadata(path), Ok(metadata) if metadata.is_dir()) {
        return Err(InvalidOutputPathError::new(
            raw.to_string(),
            InvalidOutputPathReason::PathIsDirectory,
        ));
    }
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        match fs::metadata(parent) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                return Err(InvalidOutputPathError::new(
                    raw.to_string(),
                    InvalidOutputPathReason::ParentNotADirectory,
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(InvalidOutputPathError::new(
                    raw.to_string(),
                    InvalidOutputPathReason::ParentNotFound,
                ));
            }
            Err(_) => {
                return Err(InvalidOutputPathError::new(
                    raw.to_string(),
                    InvalidOutputPathReason::ParentNotFound,
                ));
            }
        }
    }
    Ok(())
}

const SESSION_MARKDOWN_TOOL_SUMMARY_LIMIT: usize = 280;

/// Extract the server-reported context window size from an error message.
/// Returns `None` if no window size can be parsed.  The server must
/// mention something like "context size (81920 tokens)" or "available
/// context size (81920 tokens)" — the number inside parens after the
/// parenthesised phrase is taken as the window.
///
/// Known formats:
///   - "exceeds the available context size (81920 tokens)"
///   - "context size (128000 tokens)"
///   - "maximum context length is 200000 tokens"
fn extract_context_window_tokens_from_error(error_str: &str) -> Option<u32> {
    // Pattern: "(NNNNNN tokens)" appearing after context-size markers
    for line in error_str.lines() {
        let lowered = line.to_ascii_lowercase();
        if lowered.contains("context size")
            || lowered.contains("context length")
            || lowered.contains("context window")
        {
            // Try parenthesised form: (81920 tokens)
            if let Some(start) = lowered.find('(') {
                if let Some(end) = lowered.find(")") {
                    if start < end {
                        let inner = &line[start + 1..end];
                        let digits: String =
                            inner.chars().take_while(|c| c.is_ascii_digit()).collect();
                        if let Ok(n) = digits.parse::<u32>() {
                            if n > 1000 {
                                return Some(n);
                            }
                        }
                    }
                }
            }
            // Try "maximum context length is NNNNNN tokens"
            if let Some(pos) = lowered.find("is ") {
                let rest = &line[pos + 3..];
                let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(n) = digits.parse::<u32>() {
                    if n > 1000 {
                        return Some(n);
                    }
                }
            }
            // Try "configured limit of NNNNNN tokens"
            if let Some(pos) = lowered.find("of ") {
                let rest = &line[pos + 3..];
                let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(n) = digits.parse::<u32>() {
                    if n > 1000 {
                        return Some(n);
                    }
                }
            }
        }
    }
    None
}

/// Slash commands that are registered in the spec list but not yet implemented
/// in this build. Used to filter both REPL completions and help output so the
/// discovery surface only shows commands that actually work (ROADMAP #39).
const STUB_COMMANDS: &[&str] = &[
    "login",
    "logout",
    "vim",
    "upgrade",
    "share",
    "feedback",
    "files",
    "fast",
    "exit",
    "summary",
    "desktop",
    "brief",
    "advisor",
    "stickers",
    "insights",
    "thinkback",
    "release-notes",
    "security-review",
    "keybindings",
    "privacy-settings",
    "plan",
    "review",
    "tasks",
    "theme",
    "voice",
    "usage",
    "rename",
    "copy",
    "hooks",
    "context",
    "color",
    "effort",
    "branch",
    "rewind",
    "ide",
    "tag",
    "output-style",
    "add-dir",
    // Spec entries with no parse arm — produce circular "Did you mean" error
    // without this guard. Adding here routes them to the proper unsupported
    // message and excludes them from REPL completions / help.
    // NOTE: do NOT add "stats", "tokens", "cache" — they are implemented.
    "allowed-tools",
    "bookmarks",
    "workspace",
    "reasoning",
    "budget",
    "rate-limit",
    "changelog",
    "diagnostics",
    "metrics",
    "tool-details",
    "focus",
    "unfocus",
    "pin",
    "unpin",
    "language",
    "profile",
    "max-tokens",
    "temperature",
    "system-prompt",
    "notifications",
    "telemetry",
    "env",
    "project",
    "terminal-setup",
    "api-key",
    "reset",
    "undo",
    "stop",
    "retry",
    "paste",
    "screenshot",
    "image",
    "search",
    "listen",
    "speak",
    "format",
    "test",
    "lint",
    "build",
    "run",
    "git",
    "stash",
    "blame",
    "log",
    "cron",
    "team",
    "benchmark",
    "migrate",
    "templates",
    "explain",
    "refactor",
    "docs",
    "fix",
    "perf",
    "chat",
    "web",
    "map",
    "symbols",
    "references",
    "definition",
    "hover",
    "autofix",
    "multi",
    "macro",
    "alias",
    "parallel",
    "subagent",
    "agent",
];

const DISPLAY_TRUNCATION_NOTICE: &str =
    "\x1b[2m… output truncated for display; full result preserved in session.\x1b[0m";

const READ_DISPLAY_MAX_LINES: usize = 80;

const READ_DISPLAY_MAX_CHARS: usize = 6_000;

const TOOL_OUTPUT_DISPLAY_MAX_LINES: usize = 60;

const TOOL_OUTPUT_DISPLAY_MAX_CHARS: usize = 4_000;

#[cfg(test)]
mod tests {
    use super::{
        acp_status_json, build_runtime_plugin_state_with_loader, build_runtime_with_plugin_state,
        classify_error_kind, classify_session_lifecycle_from_panes, collect_session_prompt_history,
        create_managed_session_handle, describe_tool_progress, filter_tool_specs,
        format_bughunter_report, format_commit_preflight_report, format_commit_skipped_report,
        format_compact_report, format_connected_line, format_cost_report, format_history_timestamp,
        format_internal_prompt_progress_line, format_issue_report, format_model_report,
        format_model_switch_report, format_permissions_report, format_permissions_switch_report,
        format_pr_report, format_resume_report, format_status_report, format_tool_call_start,
        format_tool_result, format_ultraplan_report, format_unknown_slash_command,
        format_unknown_slash_command_message, format_user_visible_api_error,
        merge_prompt_with_stdin, normalize_permission_mode, parse_args, parse_export_args,
        parse_git_status_branch, parse_git_status_metadata_for, parse_git_workspace_summary,
        parse_history_count, permission_policy, print_help_to, push_output_block,
        render_config_report, render_diff_report, render_diff_report_for, render_help_topic,
        render_help_topic_json, render_memory_report, render_prompt_history_report,
        render_repl_help, render_resume_usage, render_session_list, render_session_markdown,
        resolve_model_alias, resolve_model_alias_with_config, resolve_repl_model,
        resolve_session_reference, response_to_events, resume_supported_slash_commands,
        run_resume_command, short_tool_id, slash_command_completion_candidates_with_sessions,
        split_error_hint, status_context, status_json_value, summarize_tool_payload_for_markdown,
        try_resolve_bare_skill_prompt, validate_no_args, write_mcp_server_fixture, CliAction,
        CliOutputFormat, CliToolExecutor, GitOperation, GitWorkspaceSummary,
        InternalPromptProgressEvent, InternalPromptProgressState, LiveCli, LocalHelpTopic,
        PermissionModeProvenance, PromptHistoryEntry, SessionLifecycleKind,
        SessionLifecycleSummary, SlashCommand, StatusUsage, TmuxPaneSnapshot, DEFAULT_MODEL,
        LATEST_SESSION_REFERENCE, STUB_COMMANDS,
    };
    use api::{ApiError, MessageResponse, OutputContentBlock, Usage};
    use plugins::{
        PluginManager, PluginManagerConfig, PluginTool, PluginToolDefinition, PluginToolPermission,
    };
    use runtime::{
        load_oauth_credentials, save_oauth_credentials, AssistantEvent, ConfigLoader, ContentBlock,
        ConversationMessage, MessageRole, OAuthConfig, PermissionMode, Session, ToolExecutor,
    };
    use serde_json::json;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tools::GlobalToolRegistry;

    fn registry_with_plugin_tool() -> GlobalToolRegistry {
        GlobalToolRegistry::with_plugin_tools(vec![PluginTool::new(
            "plugin-demo@external",
            "plugin-demo",
            PluginToolDefinition {
                name: "plugin_echo".to_string(),
                description: Some("Echo plugin payload".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string" }
                    },
                    "required": ["message"],
                    "additionalProperties": false
                }),
            },
            "echo".to_string(),
            Vec::new(),
            PluginToolPermission::WorkspaceWrite,
            None,
        )])
        .expect("plugin tool registry should build")
    }

    #[test]
    fn opaque_provider_wrapper_surfaces_failure_class_session_and_trace() {
        let error = ApiError::Api {
            status: "500".parse().expect("status"),
            error_type: Some("api_error".to_string()),
            message: Some(
                "Something went wrong while processing your request. Please try again, or use /new to start a fresh session."
                    .to_string(),
            ),
            request_id: Some("req_jobdori_789".to_string()),
            body: String::new(),
            retryable: true,
            suggested_action: None,
            retry_after: None,
};

        let rendered = format_user_visible_api_error("session-issue-22", &error);
        assert!(rendered.contains("provider_internal"));
        assert!(rendered.contains("session session-issue-22"));
        assert!(rendered.contains("trace req_jobdori_789"));
    }

    #[test]
    fn retry_exhaustion_uses_retry_failure_class_for_generic_provider_wrapper() {
        let error = ApiError::RetriesExhausted {
            attempts: 3,
            last_error: Box::new(ApiError::Api {
                status: "502".parse().expect("status"),
                error_type: Some("api_error".to_string()),
                message: Some(
                    "Something went wrong while processing your request. Please try again, or use /new to start a fresh session."
                        .to_string(),
                ),
                request_id: Some("req_jobdori_790".to_string()),
                body: String::new(),
                retryable: true,
                suggested_action: None,
                retry_after: None,
}),
        };

        let rendered = format_user_visible_api_error("session-issue-22", &error);
        assert!(rendered.contains("provider_retry_exhausted"), "{rendered}");
        assert!(rendered.contains("session session-issue-22"));
        assert!(rendered.contains("trace req_jobdori_790"));
    }

    #[test]
    fn context_window_preflight_errors_render_recovery_steps() {
        let error = ApiError::ContextWindowExceeded {
            model: "anthropic/claude-sonnet-4-6".to_string(),
            estimated_input_tokens: 182_000,
            requested_output_tokens: 64_000,
            estimated_total_tokens: 246_000,
            context_window_tokens: 200_000,
        };

        let rendered = format_user_visible_api_error("session-issue-32", &error);
        assert!(rendered.contains("Context window blocked"), "{rendered}");
        assert!(rendered.contains("context_window_blocked"), "{rendered}");
        assert!(
            rendered.contains("Session          session-issue-32"),
            "{rendered}"
        );
        assert!(
            rendered.contains("Model            anthropic/claude-sonnet-4-6"),
            "{rendered}"
        );
        assert!(
            rendered.contains("Input estimate   ~182000 tokens (heuristic)"),
            "{rendered}"
        );
        assert!(
            rendered.contains("Total estimate   ~246000 tokens (heuristic)"),
            "{rendered}"
        );
        assert!(rendered.contains("Compact          /compact"), "{rendered}");
        assert!(
            rendered.contains("Resume compact   claw --resume session-issue-32 /compact"),
            "{rendered}"
        );
        assert!(
            rendered.contains("Fresh session    /clear --confirm"),
            "{rendered}"
        );
        assert!(rendered.contains("Reduce scope"), "{rendered}");
        assert!(rendered.contains("Retry            rerun"), "{rendered}");
    }

    #[test]
    fn provider_context_window_errors_are_reframed_with_same_guidance() {
        let error = ApiError::Api {
            status: "400".parse().expect("status"),
            error_type: Some("invalid_request_error".to_string()),
            message: Some(
                "This model's maximum context length is 200000 tokens, but your request used 230000 tokens."
                    .to_string(),
            ),
            request_id: Some("req_ctx_456".to_string()),
            body: String::new(),
            retryable: false,
            suggested_action: None,
            retry_after: None,
};

        let rendered = format_user_visible_api_error("session-issue-32", &error);
        assert!(rendered.contains("context_window_blocked"), "{rendered}");
        assert!(
            rendered.contains("Trace            req_ctx_456"),
            "{rendered}"
        );
        assert!(
            rendered
                .contains("Detail           This model's maximum context length is 200000 tokens"),
            "{rendered}"
        );
        assert!(rendered.contains("Compact          /compact"), "{rendered}");
        assert!(
            rendered.contains("Fresh session    /clear --confirm"),
            "{rendered}"
        );
    }

    #[test]
    fn openai_configured_limit_errors_are_rendered_as_context_window_guidance() {
        let error = ApiError::Api {
            status: "400".parse().expect("status"),
            error_type: Some("invalid_request_error".to_string()),
            message: Some(
                "Input tokens exceed the configured limit of 922000 tokens. Your messages resulted in 1860900 tokens. Please reduce the length of the messages."
                    .to_string(),
            ),
            request_id: Some("req_ctx_openai_456".to_string()),
            body: String::new(),
            retryable: false,
            suggested_action: None,
            retry_after: None,
        };

        let rendered = format_user_visible_api_error("session-issue-32", &error);
        assert!(rendered.contains("Context window blocked"), "{rendered}");
        assert!(rendered.contains("context_window_blocked"), "{rendered}");
        assert!(
            rendered.contains("Trace            req_ctx_openai_456"),
            "{rendered}"
        );
        assert!(
            rendered.contains(
                "Detail           Input tokens exceed the configured limit of 922000 tokens."
            ),
            "{rendered}"
        );
        assert!(rendered.contains("Compact          /compact"), "{rendered}");
        assert!(
            rendered.contains("Fresh session    /clear --confirm"),
            "{rendered}"
        );
    }

    #[test]
    fn retry_wrapped_context_window_errors_keep_recovery_guidance() {
        let error = ApiError::RetriesExhausted {
            attempts: 2,
            last_error: Box::new(ApiError::Api {
                status: "413".parse().expect("status"),
                error_type: Some("invalid_request_error".to_string()),
                message: Some("Request is too large for this model's context window.".to_string()),
                request_id: Some("req_ctx_retry_789".to_string()),
                body: String::new(),
                retryable: false,
                suggested_action: None,
                retry_after: None,
            }),
        };

        let rendered = format_user_visible_api_error("session-issue-32", &error);
        assert!(rendered.contains("Context window blocked"), "{rendered}");
        assert!(rendered.contains("context_window_blocked"), "{rendered}");
        assert!(
            rendered.contains("Trace            req_ctx_retry_789"),
            "{rendered}"
        );
        assert!(
            rendered
                .contains("Detail           Request is too large for this model's context window."),
            "{rendered}"
        );
        assert!(rendered.contains("Compact          /compact"), "{rendered}");
        assert!(
            rendered.contains("Resume compact   claw --resume session-issue-32 /compact"),
            "{rendered}"
        );
    }

    fn temp_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("rusty-claude-cli-{nanos}-{unique}"))
    }

    fn git(args: &[&str], cwd: &Path) {
        let status = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .expect("git command should run");
        assert!(
            status.success(),
            "git command failed: git {}",
            args.join(" ")
        );
    }

    fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn with_current_dir<T>(cwd: &Path, f: impl FnOnce() -> T) -> T {
        let _guard = cwd_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = std::env::current_dir().expect("cwd should load");
        std::env::set_current_dir(cwd).expect("cwd should change");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        std::env::set_current_dir(previous).expect("cwd should restore");
        match result {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    fn write_skill_fixture(root: &Path, name: &str, description: &str) {
        let skill_dir = root.join(name);
        fs::create_dir_all(&skill_dir).expect("skill dir should exist");
        fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n"),
        )
        .expect("skill file should write");
    }

    fn write_plugin_fixture(root: &Path, name: &str, include_hooks: bool, include_lifecycle: bool) {
        fs::create_dir_all(root.join(".claude-plugin")).expect("manifest dir");
        if include_hooks {
            fs::create_dir_all(root.join("hooks")).expect("hooks dir");
            fs::write(
                root.join("hooks").join("pre.sh"),
                "#!/bin/sh\nprintf 'plugin pre hook'\n",
            )
            .expect("write hook");
        }
        if include_lifecycle {
            fs::create_dir_all(root.join("lifecycle")).expect("lifecycle dir");
            fs::write(
                root.join("lifecycle").join("init.sh"),
                "#!/bin/sh\nprintf 'init\\n' >> lifecycle.log\n",
            )
            .expect("write init lifecycle");
            fs::write(
                root.join("lifecycle").join("shutdown.sh"),
                "#!/bin/sh\nprintf 'shutdown\\n' >> lifecycle.log\n",
            )
            .expect("write shutdown lifecycle");
        }

        let hooks = if include_hooks {
            ",\n  \"hooks\": {\n    \"PreToolUse\": [\"./hooks/pre.sh\"]\n  }"
        } else {
            ""
        };
        let lifecycle = if include_lifecycle {
            ",\n  \"lifecycle\": {\n    \"Init\": [\"./lifecycle/init.sh\"],\n    \"Shutdown\": [\"./lifecycle/shutdown.sh\"]\n  }"
        } else {
            ""
        };
        fs::write(
            root.join(".claude-plugin").join("plugin.json"),
            format!(
                "{{\n  \"name\": \"{name}\",\n  \"version\": \"1.0.0\",\n  \"description\": \"runtime plugin fixture\"{hooks}{lifecycle}\n}}"
            ),
        )
        .expect("write plugin manifest");
    }
    #[test]
    fn defaults_to_repl_when_no_args() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        assert_eq!(
            parse_args(&[]).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                allowed_tools: None,
                permission_mode: PermissionMode::WorkspaceWrite,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn default_permission_mode_uses_project_config_when_env_is_unset() {
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(cwd.join(".claw")).expect("project config dir should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            cwd.join(".claw").join("settings.json"),
            r#"{"permissionMode":"acceptEdits"}"#,
        )
        .expect("project config should write");

        let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        let original_permission_mode = std::env::var("RUSTY_CLAUDE_PERMISSION_MODE").ok();
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");

        let resolved = with_current_dir(&cwd, super::default_permission_mode);

        match original_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        match original_permission_mode {
            Some(value) => std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", value),
            None => std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        assert_eq!(resolved, PermissionMode::WorkspaceWrite);
    }

    #[test]
    fn env_permission_mode_overrides_project_config_default() {
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(cwd.join(".claw")).expect("project config dir should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            cwd.join(".claw").join("settings.json"),
            r#"{"permissionMode":"acceptEdits"}"#,
        )
        .expect("project config should write");

        let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        let original_permission_mode = std::env::var("RUSTY_CLAUDE_PERMISSION_MODE").ok();
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", "read-only");

        let resolved = with_current_dir(&cwd, super::default_permission_mode);

        match original_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        match original_permission_mode {
            Some(value) => std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", value),
            None => std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        assert_eq!(resolved, PermissionMode::ReadOnly);
    }

    #[test]
    fn resolve_cli_auth_source_ignores_saved_oauth_credentials() {
        let _guard = env_lock();
        let config_home = temp_dir();
        std::fs::create_dir_all(&config_home).expect("config home should exist");

        let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        let original_api_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let original_auth_token = std::env::var("ANTHROPIC_AUTH_TOKEN").ok();
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("ANTHROPIC_AUTH_TOKEN");

        save_oauth_credentials(&runtime::OAuthTokenSet {
            access_token: "expired-access-token".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            expires_at: Some(0),
            scopes: vec!["org:create_api_key".to_string(), "user:profile".to_string()],
        })
        .expect("save expired oauth credentials");

        let error = super::resolve_cli_auth_source_for_cwd()
            .expect_err("saved oauth should be ignored without env auth");

        match original_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        match original_api_key {
            Some(value) => std::env::set_var("ANTHROPIC_API_KEY", value),
            None => std::env::remove_var("ANTHROPIC_API_KEY"),
        }
        match original_auth_token {
            Some(value) => std::env::set_var("ANTHROPIC_AUTH_TOKEN", value),
            None => std::env::remove_var("ANTHROPIC_AUTH_TOKEN"),
        }
        std::fs::remove_dir_all(config_home).expect("temp config home should clean up");

        assert!(error.to_string().contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn parses_prompt_subcommand() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        let args = vec![
            "prompt".to_string(),
            "hello".to_string(),
            "world".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "hello world".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::WorkspaceWrite,
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn merge_prompt_with_stdin_returns_prompt_unchanged_when_no_pipe() {
        // given
        let prompt = "Review this";

        // when
        let merged = merge_prompt_with_stdin(prompt, None);

        // then
        assert_eq!(merged, "Review this");
    }

    #[test]
    fn merge_prompt_with_stdin_ignores_whitespace_only_pipe() {
        // given
        let prompt = "Review this";
        let piped = "   \n\t\n  ";

        // when
        let merged = merge_prompt_with_stdin(prompt, Some(piped));

        // then
        assert_eq!(merged, "Review this");
    }

    #[test]
    fn merge_prompt_with_stdin_appends_piped_content_as_context() {
        // given
        let prompt = "Review this";
        let piped = "fn main() { println!(\"hi\"); }\n";

        // when
        let merged = merge_prompt_with_stdin(prompt, Some(piped));

        // then
        assert_eq!(merged, "Review this\n\nfn main() { println!(\"hi\"); }");
    }

    #[test]
    fn merge_prompt_with_stdin_trims_surrounding_whitespace_on_pipe() {
        // given
        let prompt = "Summarize";
        let piped = "\n\n  some notes  \n\n";

        // when
        let merged = merge_prompt_with_stdin(prompt, Some(piped));

        // then
        assert_eq!(merged, "Summarize\n\nsome notes");
    }

    #[test]
    fn merge_prompt_with_stdin_returns_pipe_when_prompt_is_empty() {
        // given
        let prompt = "";
        let piped = "standalone body";

        // when
        let merged = merge_prompt_with_stdin(prompt, Some(piped));

        // then
        assert_eq!(merged, "standalone body");
    }

    #[test]
    fn parses_bare_prompt_and_json_output_flag() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        let args = vec![
            "--output-format=json".to_string(),
            "--model".to_string(),
            "opus".to_string(),
            "explain".to_string(),
            "this".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "explain this".to_string(),
                model: "anthropic/claude-opus-4-7".to_string(),
                output_format: CliOutputFormat::Json,
                allowed_tools: None,
                permission_mode: PermissionMode::WorkspaceWrite,
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn parses_dash_prefixed_prompt_text_434() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");

        assert_eq!(
            parse_args(&["--".to_string(), "-prompt-with-dash".to_string()])
                .expect("-- should terminate flag parsing"),
            CliAction::Prompt {
                prompt: "-prompt-with-dash".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::WorkspaceWrite,
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );

        assert_eq!(
            parse_args(&["-not-a-flag".to_string()])
                .expect("unknown dash-prefixed shorthand prompt should parse as prompt text"),
            CliAction::Prompt {
                prompt: "-not-a-flag".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::WorkspaceWrite,
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );

        assert_eq!(
            parse_args(&["--bogus-flag-like".to_string(), "literal".to_string()])
                .expect("unknown double-dash text should stay eligible for prompt shorthand"),
            CliAction::Prompt {
                prompt: "--bogus-flag-like literal".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::WorkspaceWrite,
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );

        assert!(parse_args(&["--".to_string()]).is_ok());

        let error = parse_args(&["--resum".to_string()])
            .expect_err("nearby real flags should still be rejected as unknown options");
        assert!(error.contains("unknown option: --resum"));
        assert!(error.contains("Did you mean --resume?"));
    }

    #[test]
    fn parses_compact_flag_for_prompt_mode() {
        // given a bare prompt invocation that includes the --compact flag
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        let args = vec![
            "--compact".to_string(),
            "summarize".to_string(),
            "this".to_string(),
        ];

        // when parse_args interprets the flag
        let parsed = parse_args(&args).expect("args should parse");

        // then compact mode is propagated and other defaults stay unchanged
        assert_eq!(
            parsed,
            CliAction::Prompt {
                prompt: "summarize this".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::WorkspaceWrite,
                compact: true,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
        assert_eq!(
            parse_args(&["--compact".to_string(), "hello".to_string()])
                .expect("compact single-word prompt should parse"),
            CliAction::Prompt {
                prompt: "hello".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::WorkspaceWrite,
                compact: true,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn prompt_subcommand_defaults_compact_to_false() {
        // given a `prompt` subcommand invocation without --compact
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        let args = vec!["prompt".to_string(), "hello".to_string()];

        // when parse_args runs
        let parsed = parse_args(&args).expect("args should parse");

        // then compact stays false (opt-in flag)
        match parsed {
            CliAction::Prompt { compact, .. } => assert!(!compact),
            other => panic!("expected Prompt action, got {other:?}"),
        }
    }

    #[test]
    fn resolves_model_aliases_in_args() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        let args = vec![
            "--model".to_string(),
            "opus".to_string(),
            "explain".to_string(),
            "this".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Prompt {
                prompt: "explain this".to_string(),
                model: "anthropic/claude-opus-4-7".to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::WorkspaceWrite,
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn resolves_known_model_aliases() {
        assert_eq!(resolve_model_alias("opus"), "anthropic/claude-opus-4-7");
        assert_eq!(resolve_model_alias("sonnet"), "anthropic/claude-sonnet-4-6");
        assert_eq!(
            resolve_model_alias("haiku"),
            "anthropic/claude-haiku-4-5-20251213"
        );
        assert_eq!(resolve_model_alias("claude-opus"), "claude-opus");
    }

    #[test]
    fn default_model_alias_uses_anthropic_routing_prefix() {
        assert_eq!(DEFAULT_MODEL, "anthropic/claude-opus-4-7");
        assert_eq!(resolve_model_alias("opus"), "anthropic/claude-opus-4-7");
    }

    #[test]
    fn user_defined_aliases_resolve_before_provider_dispatch() {
        // given
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(cwd.join(".claw")).expect("project config dir should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            cwd.join(".claw").join("settings.json"),
            r#"{"aliases":{"fast":"anthropic/claude-haiku-4-5-20251213","smart":"opus","cheap":"grok-3-mini"}}"#,
        )
        .expect("project config should write");

        let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);

        // when
        let direct = with_current_dir(&cwd, || resolve_model_alias_with_config("fast"));
        let chained = with_current_dir(&cwd, || resolve_model_alias_with_config("smart"));
        let cross_provider = with_current_dir(&cwd, || resolve_model_alias_with_config("cheap"));
        let unknown = with_current_dir(&cwd, || resolve_model_alias_with_config("unknown-model"));
        let builtin = with_current_dir(&cwd, || resolve_model_alias_with_config("haiku"));

        match original_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        std::fs::remove_dir_all(root).expect("temp config root should clean up");

        // then
        assert_eq!(direct, "anthropic/claude-haiku-4-5-20251213");
        assert_eq!(chained, "anthropic/claude-opus-4-7");
        assert_eq!(cross_provider, "grok-3-mini");
        assert_eq!(unknown, "unknown-model");
        assert_eq!(builtin, "anthropic/claude-haiku-4-5-20251213");
    }

    #[test]
    fn parses_version_flags_without_initializing_prompt_mode() {
        assert_eq!(
            parse_args(&["--version".to_string()]).expect("args should parse"),
            CliAction::Version {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["-V".to_string()]).expect("args should parse"),
            CliAction::Version {
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_permission_mode_flag() {
        let args = vec!["--permission-mode=read-only".to_string()];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                allowed_tools: None,
                permission_mode: PermissionMode::ReadOnly,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn dangerously_skip_permissions_flag_forces_danger_full_access_in_repl() {
        let _guard = env_lock();
        std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", "read-only");
        let args = vec!["--dangerously-skip-permissions".to_string()];
        let parsed = parse_args(&args).expect("args should parse");
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");

        assert_eq!(
            parsed,
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                allowed_tools: None,
                permission_mode: PermissionMode::DangerFullAccess,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn dangerously_skip_permissions_flag_applies_to_prompt_subcommand() {
        let _guard = env_lock();
        std::env::set_var("RUSTY_CLAUDE_PERMISSION_MODE", "read-only");
        let args = vec![
            "--dangerously-skip-permissions".to_string(),
            "prompt".to_string(),
            "do".to_string(),
            "the".to_string(),
            "thing".to_string(),
        ];
        let parsed = parse_args(&args).expect("args should parse");
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");

        assert_eq!(
            parsed,
            CliAction::Prompt {
                prompt: "do the thing".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::DangerFullAccess,
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn parses_allowed_tools_flags_with_aliases_and_lists() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        let args = vec![
            "--allowedTools".to_string(),
            "read,glob".to_string(),
            "--allowed-tools=write_file".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::Repl {
                model: DEFAULT_MODEL.to_string(),
                allowed_tools: Some(
                    ["glob_search", "read_file", "write_file"]
                        .into_iter()
                        .map(str::to_string)
                        .collect()
                ),
                permission_mode: PermissionMode::WorkspaceWrite,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn rejects_allowed_tools_followed_by_subcommand_or_flag_432() {
        let _env_guard = env_lock();
        let _cwd_guard = cwd_guard();
        for args in [
            vec!["--allowedTools".to_string(), "status".to_string()],
            vec![
                "--allowedTools".to_string(),
                "status".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
            ],
            vec!["--allowedTools".to_string(), "--output-format".to_string()],
            vec!["--allowedTools=".to_string()],
        ] {
            let error = parse_args(&args).expect_err("allowedTools missing value should reject");
            assert!(
                error.starts_with("missing_argument: --allowedTools requires a tool list"),
                "unexpected error for {args:?}: {error}"
            );
        }
    }

    #[test]
    fn rejects_unknown_allowed_tools() {
        let _env_guard = env_lock();
        let _cwd_guard = cwd_guard();
        let error = parse_args(&["--allowedTools".to_string(), "teleport".to_string()])
            .expect_err("tool should be rejected");
        assert!(error.starts_with("invalid_tool_name:"));
        assert!(error.contains("unsupported tool in --allowedTools: teleport"));
        assert!(error.contains("Available: "));
        assert!(error.contains("web_fetch"));
        assert!(error.contains("Aliases: "));
        assert!(error.contains("WebFetch=web_fetch"));
    }

    #[test]
    fn rejects_empty_allowed_tools_flag() {
        let _env_guard = env_lock();
        let _cwd_guard = cwd_guard();
        for raw in ["", ",,"] {
            let error = parse_args(&["--allowedTools".to_string(), raw.to_string()])
                .expect_err("empty allowedTools should be rejected");
            assert!(
                error.contains("--allowedTools was provided with no usable tool names"),
                "unexpected error for {raw:?}: {error}"
            );
        }
    }

    #[test]
    fn parses_system_prompt_options() {
        // given: system-prompt options for cwd and date
        let args = vec![
            "system-prompt".to_string(),
            "--cwd".to_string(),
            "/tmp".to_string(),
            "--date".to_string(),
            "2026-04-01".to_string(),
        ];

        // when: parsing the direct system-prompt command
        let action = parse_args(&args).expect("args should parse");

        // then: the action carries prompt options and default model
        assert_eq!(
            action,
            CliAction::PrintSystemPrompt {
                cwd: PathBuf::from("/tmp"),
                date: "2026-04-01".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_global_model_for_system_prompt() {
        // given: a global OpenAI-compatible model before system-prompt
        let args = vec![
            "--model".to_string(),
            "openai/gpt-4.1-mini".to_string(),
            "system-prompt".to_string(),
        ];

        // when: parsing the CLI arguments
        let action = parse_args(&args).expect("args should parse");

        // then: the system-prompt action carries the selected model
        match action {
            CliAction::PrintSystemPrompt { model, .. } => {
                assert_eq!(model, "openai/gpt-4.1-mini");
            }
            other => panic!("expected PrintSystemPrompt, got {other:?}"),
        }
    }

    #[test]
    fn removed_login_and_logout_subcommands_error_helpfully() {
        let login = parse_args(&["login".to_string()]).expect_err("login should be removed");
        assert!(login.contains("ANTHROPIC_API_KEY"));
        let logout = parse_args(&["logout".to_string()]).expect_err("logout should be removed");
        assert!(logout.contains("ANTHROPIC_AUTH_TOKEN"));
        assert_eq!(
            parse_args(&["doctor".to_string()]).expect("doctor should parse"),
            CliAction::Doctor {
                output_format: CliOutputFormat::Text,
                permission_mode: PermissionModeProvenance::default_fallback(),
            }
        );
        assert_eq!(
            parse_args(&["state".to_string()]).expect("state should parse"),
            CliAction::State {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "state".to_string(),
                "--output-format".to_string(),
                "json".to_string()
            ])
            .expect("state --output-format json should parse"),
            CliAction::State {
                output_format: CliOutputFormat::Json,
            }
        );
        assert_eq!(
            parse_args(&["init".to_string()]).expect("init should parse"),
            CliAction::Init {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["agents".to_string()]).expect("agents should parse"),
            CliAction::Agents {
                args: None,
                output_format: CliOutputFormat::Text
            }
        );
        assert_eq!(
            parse_args(&["mcp".to_string()]).expect("mcp should parse"),
            CliAction::Mcp {
                args: None,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["skills".to_string()]).expect("skills should parse"),
            CliAction::Skills {
                args: None,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "skills".to_string(),
                "help".to_string(),
                "overview".to_string()
            ])
            .expect("skills help overview should invoke"),
            CliAction::Prompt {
                prompt: "$help overview".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: crate::default_permission_mode(),
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
        assert_eq!(
            parse_args(&["agents".to_string(), "--help".to_string()])
                .expect("agents help should parse"),
            CliAction::Agents {
                args: Some("--help".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
        // #145: `plugins` must parse as CliAction::Plugins (not fall through
        // to the prompt path, which would hit the Anthropic API for a purely
        // local introspection command).
        assert_eq!(
            parse_args(&["plugins".to_string()]).expect("plugins should parse"),
            CliAction::Plugins {
                action: None,
                target: None,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["plugins".to_string(), "list".to_string()])
                .expect("plugins list should parse"),
            CliAction::Plugins {
                action: Some("list".to_string()),
                target: None,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "plugins".to_string(),
                "enable".to_string(),
                "example-bundled".to_string(),
            ])
            .expect("plugins enable <target> should parse"),
            CliAction::Plugins {
                action: Some("enable".to_string()),
                target: Some("example-bundled".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "plugins".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
            ])
            .expect("plugins --output-format json should parse"),
            CliAction::Plugins {
                action: None,
                target: None,
                output_format: CliOutputFormat::Json,
            }
        );
        for alias in ["plugin", "marketplace"] {
            assert_eq!(
                parse_args(&[alias.to_string()]).expect("plugin alias should parse"),
                CliAction::Plugins {
                    action: None,
                    target: None,
                    output_format: CliOutputFormat::Text,
                },
                "{alias} should route to local plugin handling, not Prompt"
            );
            assert_eq!(
                parse_args(&[alias.to_string(), "list".to_string()])
                    .expect("plugin alias list should parse"),
                CliAction::Plugins {
                    action: Some("list".to_string()),
                    target: None,
                    output_format: CliOutputFormat::Text,
                },
                "{alias} list should route to local plugin handling, not Prompt"
            );
            assert_eq!(
                parse_args(&[
                    alias.to_string(),
                    "install".to_string(),
                    "./fixtures/plugin-demo".to_string(),
                ])
                .expect("plugin alias install should parse"),
                CliAction::Plugins {
                    action: Some("install".to_string()),
                    target: Some("./fixtures/plugin-demo".to_string()),
                    output_format: CliOutputFormat::Text,
                },
                "{alias} install should route to local plugin handling, not Prompt"
            );
        }
        // #146: `config` and `diff` must parse as standalone CLI actions,
        // not fall through to the "is a slash command" error. Both are
        // pure-local read-only introspection.
        assert_eq!(
            parse_args(&["config".to_string()]).expect("config should parse"),
            CliAction::Config {
                section: None,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["config".to_string(), "env".to_string()])
                .expect("config env should parse"),
            CliAction::Config {
                section: Some("env".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "config".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
            ])
            .expect("config --output-format json should parse"),
            CliAction::Config {
                section: None,
                output_format: CliOutputFormat::Json,
            }
        );
        assert_eq!(
            parse_args(&["diff".to_string()]).expect("diff should parse"),
            CliAction::Diff {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "diff".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
            ])
            .expect("diff --output-format json should parse"),
            CliAction::Diff {
                output_format: CliOutputFormat::Json,
            }
        );
        // #147: empty / whitespace-only positional args must be rejected
        // with a specific error instead of falling through to the prompt
        // path (where they surface a misleading "missing Anthropic
        // credentials" error or burn API tokens on an empty prompt).
        let empty_err =
            parse_args(&["".to_string()]).expect_err("empty positional arg should be rejected");
        assert!(
            empty_err.starts_with("empty prompt:"),
            "empty-arg error should be specific, got: {empty_err}"
        );
        let whitespace_err = parse_args(&["   ".to_string()])
            .expect_err("whitespace-only positional arg should be rejected");
        assert!(
            whitespace_err.starts_with("empty prompt:"),
            "whitespace-only error should be specific, got: {whitespace_err}"
        );
        let multi_empty_err = parse_args(&["".to_string(), "".to_string()])
            .expect_err("multiple empty positional args should be rejected");
        assert!(
            multi_empty_err.starts_with("empty prompt:"),
            "multi-empty error should be specific, got: {multi_empty_err}"
        );
        // Typo guard from #108 must still take precedence for non-empty
        // single-word non-prompt-looking inputs.
        let typo_err = parse_args(&["sttaus".to_string()])
            .expect_err("typo'd subcommand should be caught by #108 guard");
        assert!(
            typo_err.contains("unknown subcommand:"),
            "typo guard should fire for 'sttaus', got: {typo_err}"
        );
        // #148: `--model` flag must be captured as model_flag_raw so status
        // JSON can report provenance (source: flag, raw: <user-input>).
        match parse_args(&[
            "--model".to_string(),
            "sonnet".to_string(),
            "status".to_string(),
        ])
        .expect("--model sonnet status should parse")
        {
            CliAction::Status {
                model,
                model_flag_raw,
                ..
            } => {
                assert_eq!(
                    model, "anthropic/claude-sonnet-4-6",
                    "sonnet alias should resolve"
                );
                assert_eq!(
                    model_flag_raw.as_deref(),
                    Some("sonnet"),
                    "raw flag input should be preserved"
                );
            }
            other => panic!("expected CliAction::Status, got: {other:?}"),
        }
        // --model= form should also capture raw.
        match parse_args(&[
            "--model=anthropic/claude-opus-4-6".to_string(),
            "status".to_string(),
        ])
        .expect("--model=... status should parse")
        {
            CliAction::Status {
                model,
                model_flag_raw,
                ..
            } => {
                assert_eq!(model, "anthropic/claude-opus-4-6");
                assert_eq!(
                    model_flag_raw.as_deref(),
                    Some("anthropic/claude-opus-4-6"),
                    "--model= form should also preserve raw input"
                );
            }
            other => panic!("expected CliAction::Status, got: {other:?}"),
        }
        match parse_args(&["--model=claude-opus-4-6".to_string(), "status".to_string()])
            .expect("bare Anthropic model should parse")
        {
            CliAction::Status {
                model,
                model_flag_raw,
                ..
            } => {
                assert_eq!(model, "claude-opus-4-6");
                assert_eq!(model_flag_raw.as_deref(), Some("claude-opus-4-6"));
            }
            other => panic!("expected CliAction::Status, got: {other:?}"),
        }
    }

    #[test]
    fn dump_manifests_subcommand_accepts_explicit_manifest_dir() {
        assert_eq!(
            parse_args(&[
                "dump-manifests".to_string(),
                "--manifests-dir".to_string(),
                "/tmp/upstream".to_string(),
            ])
            .expect("dump-manifests should parse"),
            CliAction::DumpManifests {
                output_format: CliOutputFormat::Text,
                manifests_dir: Some(PathBuf::from("/tmp/upstream")),
            }
        );
        assert_eq!(
            parse_args(&[
                "dump-manifests".to_string(),
                "--manifests-dir=/tmp/upstream".to_string()
            ])
            .expect("inline dump-manifests flag should parse"),
            CliAction::DumpManifests {
                output_format: CliOutputFormat::Text,
                manifests_dir: Some(PathBuf::from("/tmp/upstream")),
            }
        );
    }

    #[test]
    fn parses_acp_command_surfaces() {
        assert_eq!(
            parse_args(&["acp".to_string()]).expect("acp should parse"),
            CliAction::Acp {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["acp".to_string(), "serve".to_string()]).expect("acp serve should parse"),
            CliAction::Acp {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["--acp".to_string()]).expect("--acp should parse"),
            CliAction::Acp {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["-acp".to_string()]).expect("-acp should parse"),
            CliAction::Acp {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "acp".to_string(),
                "serve".to_string(),
                "--output-format".to_string(),
                "json".to_string()
            ])
            .expect("acp serve json should parse"),
            CliAction::Acp {
                output_format: CliOutputFormat::Json,
            }
        );
        let unsupported = parse_args(&["acp".to_string(), "start".to_string()])
            .expect_err("unknown ACP subcommand should fail with a typed contract");
        assert!(unsupported.contains("unsupported ACP invocation"));
    }

    #[test]
    fn acp_status_json_is_truthful_unsupported_contract() {
        let value = acp_status_json();
        assert_eq!(value["schema_version"], "1.0");
        assert_eq!(value["kind"], "acp");
        assert_eq!(value["status"], "not_implemented");
        assert_eq!(value["supported"], false);
        assert_eq!(value["protocol"]["json_rpc"], false);
        assert_eq!(value["protocol"]["daemon"], false);
        assert_eq!(value["protocol"]["serve_starts_daemon"], false);
        assert!(value["protocol"]["endpoint"].is_null());
        assert_eq!(
            value["contracts"]["unsupported_invocation_kind"],
            "unsupported_acp_invocation"
        );
    }

    #[test]
    fn local_command_help_flags_stay_on_the_local_parser_path() {
        assert_eq!(
            parse_args(&["status".to_string(), "--help".to_string()])
                .expect("status help should parse"),
            CliAction::HelpTopic {
                topic: LocalHelpTopic::Status,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["sandbox".to_string(), "-h".to_string()])
                .expect("sandbox help should parse"),
            CliAction::HelpTopic {
                topic: LocalHelpTopic::Sandbox,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["doctor".to_string(), "--help".to_string()])
                .expect("doctor help should parse"),
            CliAction::HelpTopic {
                topic: LocalHelpTopic::Doctor,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["acp".to_string(), "--help".to_string()]).expect("acp help should parse"),
            CliAction::HelpTopic {
                topic: LocalHelpTopic::Acp,
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn subcommand_help_flag_has_one_contract_across_all_subcommands_141() {
        // #141: every documented subcommand must resolve `<subcommand> --help`
        // to a subcommand-specific help topic, never to global help, never to
        // an "unknown option" error, never to the subcommand's primary output.
        let cases: &[(&str, LocalHelpTopic)] = &[
            ("status", LocalHelpTopic::Status),
            ("sandbox", LocalHelpTopic::Sandbox),
            ("doctor", LocalHelpTopic::Doctor),
            ("acp", LocalHelpTopic::Acp),
            ("init", LocalHelpTopic::Init),
            ("state", LocalHelpTopic::State),
            ("export", LocalHelpTopic::Export),
            ("version", LocalHelpTopic::Version),
            ("system-prompt", LocalHelpTopic::SystemPrompt),
            ("dump-manifests", LocalHelpTopic::DumpManifests),
            ("bootstrap-plan", LocalHelpTopic::BootstrapPlan),
        ];
        for (subcommand, expected_topic) in cases {
            for flag in ["--help", "-h"] {
                let parsed = parse_args(&[subcommand.to_string(), flag.to_string()])
                    .unwrap_or_else(|error| {
                        panic!("`{subcommand} {flag}` should parse as help but errored: {error}")
                    });
                assert_eq!(
                    parsed,
                    CliAction::HelpTopic {
                        topic: *expected_topic,
                        output_format: CliOutputFormat::Text,
                    },
                    "`{subcommand} {flag}` should resolve to HelpTopic({expected_topic:?})"
                );
            }
            let json_parsed = parse_args(&[
                subcommand.to_string(),
                "--help".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
            ])
            .unwrap_or_else(|error| {
                panic!("`{subcommand} --help --output-format json` should parse: {error}")
            });
            assert_eq!(
                json_parsed,
                CliAction::HelpTopic {
                    topic: *expected_topic,
                    output_format: CliOutputFormat::Json,
                },
                "`{subcommand} --help --output-format json` should preserve json output format"
            );
            // And the rendered help must actually mention the subcommand name
            // (or its canonical title) so users know they got the right help.
            let rendered = render_help_topic(*expected_topic);
            assert!(
                !rendered.is_empty(),
                "{subcommand} help text should not be empty"
            );
            assert!(
                rendered.contains("Usage"),
                "{subcommand} help text should contain a Usage line"
            );
        }
    }

    #[test]
    fn export_help_json_is_bounded_and_parseable_384() {
        let value = render_help_topic_json(LocalHelpTopic::Export);
        assert_eq!(value["kind"], "help");
        assert_eq!(value["topic"], "export");
        assert_eq!(value["command"], "export");
        assert_eq!(
            value["usage"],
            "claw export [--session <id|latest>] [--output <path>] [--output-format <format>]"
        );
        assert_eq!(value["defaults"]["session"], LATEST_SESSION_REFERENCE);
        assert!(value["options"].as_array().expect("options array").len() >= 4);
        assert!(
            value.get("message").is_none(),
            "export help json should be a bounded envelope, not plaintext help wrapped in json"
        );
    }

    #[test]
    fn plugins_degrades_on_invalid_mcp_server_without_global_config_error_440() {
        // #440: invalid MCP entries should not make local plugin introspection
        // unusable, and should surface as validation metadata instead of a
        // whole-config parse failure.
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project-with-malformed-mcp-for-plugins");
        let config_home = root.join("config-home");
        std::fs::create_dir_all(&cwd).expect("project dir should exist");
        std::fs::create_dir_all(&config_home).expect("config home should exist");
        std::fs::write(
            cwd.join(".claw.json"),
            r#"{
  "mcpServers": {
    "missing-command": {"args": ["arg-only-no-command"]}
  }
}
"#,
        )
        .expect("write malformed .claw.json");

        let previous_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        let payload = super::plugins_command_payload_for(
            &cwd,
            None,
            None,
            super::ConfigWarningMode::EmitStderr,
        )
        .expect("plugins list should not hard-fail on malformed MCP config");
        match previous_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }

        assert_eq!(payload.status, "degraded");
        assert!(payload.config_load_error.is_none());
        assert_eq!(payload.mcp_validation.total_configured, 1);
        assert_eq!(payload.mcp_validation.valid_count, 0);
        assert_eq!(payload.mcp_validation.invalid_count(), 1);
        assert_eq!(
            payload.mcp_validation.invalid_servers[0].name,
            "missing-command"
        );
        assert!(payload.mcp_validation.invalid_servers[0]
            .reason
            .contains("missing string field command"));
        assert!(payload.message.contains("MCP validation"));
        assert!(payload.message.contains("valid MCP siblings only"));
        assert!(payload.message.contains("Plugins"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn status_degrades_gracefully_on_malformed_mcp_config_143() {
        // #143: previously `claw status` hard-failed on any config parse error,
        // taking down the entire health surface for one malformed MCP entry.
        // `claw doctor` already degrades gracefully; this test locks `status`
        // to the same contract.
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project-with-malformed-mcp");
        std::fs::create_dir_all(&cwd).expect("project dir should exist");
        // Top-level `mcpServers` shape errors still degrade through the
        // config_load_error path; per-server errors are handled by the #440
        // MCP validation summary instead.
        std::fs::write(
            cwd.join(".claw.json"),
            r#"{
  "mcpServers": "not-an-object"
}
"#,
        )
        .expect("write malformed .claw.json");

        let context = with_current_dir(&cwd, || {
            super::status_context(None)
                .expect("status_context should not hard-fail on config parse errors (#143)")
        });

        // Config-shape errors still populate config_load_error.
        let err = context
            .config_load_error
            .as_ref()
            .expect("config_load_error should be Some when config shape parsing fails");
        assert!(
            err.contains("mcpServers"),
            "config_load_error should name the malformed mcpServers path: {err}"
        );
        assert!(
            err.contains("must be an object"),
            "config_load_error should carry the underlying parse error: {err}"
        );

        // Phase 1 contract: workspace/git/sandbox fields are still populated
        // (independent of config parse). Sandbox falls back to defaults.
        assert_eq!(context.cwd, cwd.canonicalize().unwrap_or(cwd.clone()));
        assert_eq!(
            context.loaded_config_files, 0,
            "loaded_config_files should be 0 when config parse fails"
        );
        assert!(
            context.discovered_config_files > 0,
            "discovered_config_files should still count the file that failed to parse"
        );

        // JSON output contract: top-level `status: "degraded"` + config_load_error field.
        let usage = super::StatusUsage {
            message_count: 0,
            turns: 0,
            latest: runtime::TokenUsage::default(),
            cumulative: runtime::TokenUsage::default(),
            estimated_tokens: 0,
        };
        let json = super::status_json_value(
            Some("test-model"),
            usage,
            "workspace-write",
            &context,
            None,
            None,
            None,
            None,
        );
        assert_eq!(
            json.get("status").and_then(|v| v.as_str()),
            Some("degraded"),
            "top-level status marker should be 'degraded' when config parse failed: {json}"
        );
        assert!(
            json.get("config_load_error")
                .and_then(|v| v.as_str())
                .is_some_and(|s| s.contains("mcpServers")),
            "config_load_error should surface in JSON output: {json}"
        );
        // Independent fields still populated.
        assert_eq!(
            json.get("model").and_then(|v| v.as_str()),
            Some("test-model")
        );
        assert!(
            json.get("workspace").is_some(),
            "workspace field still reported"
        );
        assert_eq!(
            json.pointer("/lane_board/status_json_supported")
                .and_then(|v| v.as_bool()),
            Some(true),
            "status JSON should advertise lane board support: {json}"
        );
        assert_eq!(
            json.pointer("/lane_board/freshness_states/2")
                .and_then(|v| v.as_str()),
            Some("transport_dead"),
            "status JSON should advertise transport-dead freshness: {json}"
        );
        assert!(
            json.get("sandbox").is_some(),
            "sandbox field still reported"
        );
        assert_eq!(
            json.pointer("/allowed_tools/source")
                .and_then(|v| v.as_str()),
            Some("default"),
            "default status should expose unrestricted tool source: {json}"
        );
        assert_eq!(
            json.pointer("/allowed_tools/restricted")
                .and_then(|v| v.as_bool()),
            Some(false),
            "default status should expose unrestricted tool state: {json}"
        );
        assert_eq!(
            json.pointer("/allowed_tools/available/0")
                .and_then(|v| v.as_str()),
            Some("agent"),
            "status JSON should expose canonical snake_case available tools: {json}"
        );
        assert_eq!(
            json.pointer("/allowed_tools/aliases/WebFetch")
                .and_then(|v| v.as_str()),
            Some("web_fetch"),
            "status JSON should expose allowed-tool aliases: {json}"
        );

        let allowed: super::AllowedToolSet = ["read_file", "grep_search"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let restricted_json = super::status_json_value(
            Some("test-model"),
            usage,
            "workspace-write",
            &context,
            None,
            None,
            Some(&allowed),
            None,
        );
        assert_eq!(
            restricted_json
                .pointer("/allowed_tools/source")
                .and_then(|v| v.as_str()),
            Some("flag"),
            "flag status should expose allow-list source: {restricted_json}"
        );
        assert_eq!(
            restricted_json
                .pointer("/allowed_tools/entries")
                .and_then(|v| v.as_array())
                .map(Vec::len),
            Some(2),
            "flag status should expose allow-list entries: {restricted_json}"
        );

        // Clean path: no config error → status: "ok", config_load_error: null.
        let clean_cwd = root.join("project-with-clean-config");
        std::fs::create_dir_all(&clean_cwd).expect("clean project dir");
        let clean_context = with_current_dir(&clean_cwd, || {
            super::status_context(None).expect("clean status_context should succeed")
        });
        assert!(clean_context.config_load_error.is_none());
        let clean_json = super::status_json_value(
            Some("test-model"),
            usage,
            "workspace-write",
            &clean_context,
            None,
            None,
            None,
            None,
        );
        assert_eq!(
            clean_json.get("status").and_then(|v| v.as_str()),
            Some("ok"),
            "clean run should report status: 'ok'"
        );
    }

    #[test]
    fn state_error_surfaces_actionable_worker_commands_139() {
        // #139: the error for missing `.claw/worker-state.json` must name
        // the concrete commands that produce worker state, otherwise claws
        // have no discoverable path from the error to a fix.
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project-with-no-state");
        std::fs::create_dir_all(&cwd).expect("project dir should exist");

        let error = with_current_dir(&cwd, || {
            super::run_worker_state(CliOutputFormat::Text).expect_err("missing state should error")
        });
        let message = error.to_string();

        // Keep the original locator so scripts grepping for it still work.
        assert!(
            message.contains("no worker state file found at"),
            "error should keep the canonical prefix: {message}"
        );
        // New actionable hints — this is what #139 is fixing.
        assert!(
            message.contains("claw prompt"),
            "error should name `claw prompt <text>` as a producer: {message}"
        );
        assert!(
            message.contains("REPL"),
            "error should mention the interactive REPL as a producer: {message}"
        );
        assert!(
            message.contains("claw state"),
            "error should tell the user what to rerun once state exists: {message}"
        );
        // And the State --help topic must document the worker relationship
        // so claws can discover the contract without hitting the error first.
        let state_help = render_help_topic(LocalHelpTopic::State);
        assert!(
            state_help.contains("Produces state"),
            "state help must document how state is produced: {state_help}"
        );
        assert!(
            state_help.contains("claw prompt"),
            "state help must name `claw prompt <text>` as a producer: {state_help}"
        );
    }

    #[test]
    fn parses_single_word_command_aliases_without_falling_back_to_prompt_mode() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        assert_eq!(
            parse_args(&["help".to_string()]).expect("help should parse"),
            CliAction::Help {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["version".to_string()]).expect("version should parse"),
            CliAction::Version {
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["status".to_string()]).expect("status should parse"),
            CliAction::Status {
                model: DEFAULT_MODEL.to_string(),
                model_flag_raw: None, // #148: no --model flag passed
                permission_mode: PermissionModeProvenance::default_fallback(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
            }
        );
        assert_eq!(
            parse_args(&["sandbox".to_string()]).expect("sandbox should parse"),
            CliAction::Sandbox {
                output_format: CliOutputFormat::Text,
            }
        );
        // #152: `--json` on diagnostic verbs should hint the correct flag.
        let err = parse_args(&["doctor".to_string(), "--json".to_string()])
            .expect_err("`doctor --json` should fail with hint");
        assert!(
            err.contains("unrecognized argument `--json` for subcommand `doctor`"),
            "error should name the verb: {err}"
        );
        assert!(
            err.contains("Did you mean `--output-format json`?"),
            "error should hint the correct flag: {err}"
        );
        // Other unrecognized args should NOT trigger the --json hint.
        let err_other = parse_args(&["doctor".to_string(), "garbage".to_string()])
            .expect_err("`doctor garbage` should fail without --json hint");
        assert!(
            !err_other.contains("--output-format json"),
            "unrelated args should not trigger --json hint: {err_other}"
        );
        // #424: bare canonical GPT model ids should parse and route via provider
        // detection instead of forcing the local-only `openai/` routing prefix.
        match parse_args(&[
            "prompt".to_string(),
            "test".to_string(),
            "--model".to_string(),
            "gpt-4".to_string(),
        ])
        .expect("`--model gpt-4` should parse as a bare OpenAI model")
        {
            CliAction::Prompt { model, .. } => assert_eq!(model, "gpt-4"),
            other => panic!("expected CliAction::Prompt, got: {other:?}"),
        }
        let err_qwen = parse_args(&[
            "prompt".to_string(),
            "test".to_string(),
            "--model".to_string(),
            "qwen-plus".to_string(),
        ])
        .expect_err("`--model qwen-plus` should fail with DashScope hint");
        assert!(
            err_qwen.contains("Did you mean `qwen/qwen-plus`?"),
            "Qwen model error should hint qwen/ prefix: {err_qwen}"
        );
        assert!(
            err_qwen.contains("DASHSCOPE_API_KEY"),
            "Qwen model error should mention env var: {err_qwen}"
        );
        // Unrelated invalid model should NOT get a hint
        let err_garbage = parse_args(&[
            "prompt".to_string(),
            "test".to_string(),
            "--model".to_string(),
            "asdfgh".to_string(),
        ])
        .expect_err("`--model asdfgh` should fail");
        assert!(
            !err_garbage.contains("Did you mean"),
            "Unrelated model errors should not get a hint: {err_garbage}"
        );

        let original_openai_base_url = std::env::var_os("OPENAI_BASE_URL");
        std::env::set_var("OPENAI_BASE_URL", "http://127.0.0.1:11434/v1");
        match parse_args(&[
            "prompt".to_string(),
            "test".to_string(),
            "--model".to_string(),
            "qwen2.5-coder:7b".to_string(),
        ])
        .expect("Ollama-style tag should parse when OPENAI_BASE_URL is set")
        {
            CliAction::Prompt { model, .. } => assert_eq!(model, "qwen2.5-coder:7b"),
            other => panic!("expected CliAction::Prompt, got: {other:?}"),
        }
        match parse_args(&[
            "prompt".to_string(),
            "test".to_string(),
            "--model".to_string(),
            "local/Qwen/Qwen3.6-27B-FP8".to_string(),
        ])
        .expect("local/ slash-containing model should parse")
        {
            CliAction::Prompt { model, .. } => assert_eq!(model, "local/Qwen/Qwen3.6-27B-FP8"),
            other => panic!("expected CliAction::Prompt, got: {other:?}"),
        }
        match original_openai_base_url {
            Some(value) => std::env::set_var("OPENAI_BASE_URL", value),
            None => std::env::remove_var("OPENAI_BASE_URL"),
        }
    }

    #[test]
    fn classify_error_kind_returns_correct_discriminants() {
        // #77: error kind classification for JSON error payloads
        assert_eq!(
            classify_error_kind("missing Anthropic credentials; export ..."),
            "missing_credentials"
        );
        assert_eq!(
            classify_error_kind("no worker state file found at /tmp/..."),
            "missing_worker_state"
        );
        assert_eq!(
            classify_error_kind("session not found: abc123"),
            "session_not_found"
        );
        // #780: "no managed sessions found" is more specific than generic "failed to restore"
        // session_load_failed; the reordered classifier now correctly returns no_managed_sessions.
        assert_eq!(
            classify_error_kind("failed to restore session: no managed sessions found"),
            "no_managed_sessions"
        );
        // Bare session load failures that aren't no_managed_sessions or legacy_binding still map here
        assert_eq!(
            classify_error_kind("failed to restore session: file not found"),
            "session_load_failed"
        );
        // #787: directory-as-session-path gets its own kind (precedes generic session_load_failed)
        assert_eq!(
            classify_error_kind("failed to restore session: Is a directory (os error 21)"),
            "session_path_is_directory"
        );
        assert_eq!(
            classify_error_kind("unrecognized argument `--foo` for subcommand `doctor`"),
            "cli_parse"
        );
        // #785/#825: unknown top-level subcommand (typo or unrecognised command)
        assert_eq!(
            classify_error_kind("unknown subcommand: dump.\nDid you mean     dump-manifests"),
            "command_not_found" // #825: unified from unknown_subcommand
        );
        assert_eq!(
            classify_error_kind("unsupported ACP invocation. Use `claw acp`."),
            "unsupported_acp_invocation"
        );
        assert_eq!(
            classify_error_kind("invalid model syntax: 'gpt-4'. Expected ..."),
            "invalid_model_syntax"
        );
        assert_eq!(
            classify_error_kind("unsupported resumed command: /blargh"),
            "unsupported_resumed_command"
        );
        assert_eq!(
            classify_error_kind("api failed after 3 attempts: ..."),
            "api_http_error"
        );
        assert_eq!(
            classify_error_kind("/tmp/settings.json: mcpServers.foo: expected JSON object"),
            "malformed_mcp_config"
        );
        assert_eq!(
            classify_error_kind("settings.json: mcpServers: field must be an object"),
            "malformed_mcp_config"
        );
        assert_eq!(
            classify_error_kind("empty prompt: provide a subcommand or a non-empty prompt string"),
            "empty_prompt"
        );
        assert_eq!(
            classify_error_kind("something completely unknown"),
            "unknown"
        );
        // #762: coverage for all classifier arms added since #77 — prevents silent fallback
        // to "unknown" if discriminant strings drift.
        assert_eq!(
            classify_error_kind("Manifest source files are missing: /tmp/x"),
            "missing_manifests"
        );
        assert_eq!(
            classify_error_kind("no managed sessions found in /tmp"),
            "no_managed_sessions"
        );
        assert_eq!(
            classify_error_kind("legacy session is missing workspace binding"),
            "legacy_session_no_workspace_binding"
        );
        // #780: full error string produced by resume_session includes the
        // "failed to restore session: " prefix — the specific arm must win.
        assert_eq!(
            classify_error_kind("failed to restore session: legacy session is missing workspace binding: /path/to/session.jsonl"),
            "legacy_session_no_workspace_binding"
        );
        assert_eq!(
            classify_error_kind("unsupported skills action: bogus. Supported actions: list"),
            "unsupported_skills_action"
        );
        assert_eq!(
            classify_error_kind("invalid_install_source: bogus"),
            "invalid_install_source"
        );
        assert_eq!(
            classify_error_kind("invalid_tool_name: unsupported tool in --allowedTools: teleport"),
            "invalid_tool_name"
        );
        assert_eq!(
            classify_error_kind(
                "invalid_output_format: unsupported value for --output-format: YAML"
            ),
            "invalid_output_format"
        );
        assert_eq!(
            classify_error_kind(
                "missing_flag_value: missing value for --model.\nUsage: --model <provider/model>"
            ),
            "missing_flag_value"
        );
        assert_eq!(
            classify_error_kind("invalid_permission_mode: unsupported permission mode 'bogus'.\nUsage: --permission-mode read-only|workspace-write|danger-full-access"),
            "invalid_permission_mode"
        );
        assert_eq!(
            classify_error_kind("invalid_cwd: not_found: `/tmp/missing`\nUsage: --cwd <path>"),
            "invalid_cwd"
        );
        assert_eq!(
            classify_error_kind("is not yet implemented"),
            "unsupported_command"
        );
        assert_eq!(
            classify_error_kind("confirmation required before running destructive operation"),
            "confirmation_required"
        );
        // #781: 429 and 401 now sub-classify; generic 5xx/other still api_http_error
        assert_eq!(
            classify_error_kind("api returned unexpected status 429"),
            "api_rate_limit_error"
        );
        assert_eq!(
            classify_error_kind(
                "api returned 401 Unauthorized (authentication_error): invalid x-api-key"
            ),
            "api_auth_error"
        );
        assert_eq!(
            classify_error_kind("api returned 500 Internal Server Error"),
            "api_http_error"
        );
        assert_eq!(
            classify_error_kind("interactive_only: this command requires an interactive terminal"),
            "interactive_only"
        );
        assert_eq!(
            classify_error_kind("slash command /compact is interactive-only"),
            "interactive_only"
        );
        // #774: agents now uses \n-delimited format — update test string to match real emission
        assert_eq!(
            classify_error_kind("unknown agents subcommand: bogus.\nSupported: list, show, help"),
            "unknown_agents_subcommand"
        );
        assert_eq!(
            classify_error_kind("agent not found: my-agent"),
            "agent_not_found"
        );
        assert_eq!(
            classify_error_kind("my-plugin is not installed"),
            "plugin_not_found"
        );
        // #794: plugins install with missing source path
        assert_eq!(
            classify_error_kind("plugin source `/nonexistent/path` was not found"),
            "plugin_source_not_found"
        );
        assert_eq!(
            classify_error_kind("skill source /path/to/skill not found"),
            "skill_not_found"
        );
        assert_eq!(
            classify_error_kind("skill 'my-skill' does not exist"),
            "skill_not_found"
        );
        assert_eq!(
            classify_error_kind("Unsupported config section 'show'. Use: env, hooks, model"),
            "unsupported_config_section"
        );
        assert_eq!(
            classify_error_kind("unknown_plugins_action: bogus"),
            "unknown_plugins_action"
        );
        assert_eq!(
            classify_error_kind(
                "missing_prompt: -p requires a prompt string.\nUsage: claw -p <text>"
            ),
            "missing_prompt"
        );
        assert_eq!(
            classify_error_kind("/tmp/.claw/settings.json: expected ',', found end of input"),
            "config_parse_error"
        );
        assert_eq!(
            classify_error_kind(
                "/path/to/.claw.json: field \"model\" must be a string, got a number"
            ),
            "config_parse_error"
        );
        // #765: removed auth subcommands must classify as removed_subcommand
        assert_eq!(
            classify_error_kind(
                "`claw login` has been removed.\nSet ANTHROPIC_API_KEY or ANTHROPIC_AUTH_TOKEN instead."
            ),
            "removed_subcommand"
        );
        // #766: unexpected extra arguments must classify as unexpected_extra_args
        assert_eq!(
            classify_error_kind(
                "unexpected extra arguments after `claw diff`: --bogus\nUsage: claw diff"
            ),
            "unexpected_extra_args"
        );
        assert_eq!(
            classify_error_kind(
                "`claw logout` has been removed.\nSet ANTHROPIC_API_KEY or ANTHROPIC_AUTH_TOKEN instead."
            ),
            "removed_subcommand"
        );
        // #768: invalid resume trailing arg must classify as invalid_resume_argument
        assert_eq!(
            classify_error_kind(
                "invalid_resume_argument: `compact` is not a slash command.\nUsage: claw --resume <session-id|latest> /<slash-command>"
            ),
            "invalid_resume_argument"
        );
        // coverage: invalid_history_count arm
        assert_eq!(
            classify_error_kind("invalid_history_count: abc is not a valid count"),
            "invalid_history_count"
        );
        assert_eq!(
            classify_error_kind("something invalid count something"),
            "invalid_history_count"
        );
        // coverage: unknown_option arm (#790)
        assert_eq!(
            classify_error_kind("unknown_option: unknown system-prompt option: --foo."),
            "unknown_option"
        );
        // #830: known command with missing required argument must not collapse to unknown.
        assert_eq!(
            classify_error_kind("missing_argument: mcp show requires a server name."),
            "missing_argument"
        );
    }

    #[test]
    fn split_error_hint_separates_reason_from_runbook() {
        // #77: short reason / hint separation for JSON error payloads
        let (short, hint) = split_error_hint("missing credentials\nHint: export ANTHROPIC_API_KEY");
        assert_eq!(short, "missing credentials");
        assert_eq!(hint, Some("Hint: export ANTHROPIC_API_KEY".to_string()));

        let (short, hint) = split_error_hint("simple error with no hint");
        assert_eq!(short, "simple error with no hint");
        assert_eq!(hint, None);
    }

    #[test]
    fn parses_bare_export_subcommand_targeting_latest_session() {
        // given
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        let args = vec!["export".to_string()];

        // when
        let parsed = parse_args(&args).expect("bare export should parse");

        // then
        assert_eq!(
            parsed,
            CliAction::Export {
                session_reference: LATEST_SESSION_REFERENCE.to_string(),
                output_path: None,
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_export_subcommand_with_positional_output_path() {
        // given
        let args = vec!["export".to_string(), "conversation.md".to_string()];

        // when
        let parsed = parse_args(&args).expect("export with path should parse");

        // then
        assert_eq!(
            parsed,
            CliAction::Export {
                session_reference: LATEST_SESSION_REFERENCE.to_string(),
                output_path: Some(PathBuf::from("conversation.md")),
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_export_subcommand_with_session_and_output_flags() {
        // given
        let args = vec![
            "export".to_string(),
            "--session".to_string(),
            "session-alpha".to_string(),
            "--output".to_string(),
            "/tmp/share.md".to_string(),
        ];

        // when
        let parsed = parse_args(&args).expect("export flags should parse");

        // then
        assert_eq!(
            parsed,
            CliAction::Export {
                session_reference: "session-alpha".to_string(),
                output_path: Some(PathBuf::from("/tmp/share.md")),
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_export_subcommand_with_inline_flag_values() {
        // given
        let args = vec![
            "export".to_string(),
            "--session=session-beta".to_string(),
            "--output=/tmp/beta.md".to_string(),
        ];

        // when
        let parsed = parse_args(&args).expect("export inline flags should parse");

        // then
        assert_eq!(
            parsed,
            CliAction::Export {
                session_reference: "session-beta".to_string(),
                output_path: Some(PathBuf::from("/tmp/beta.md")),
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn parses_export_subcommand_with_json_output_format() {
        // given
        let args = vec![
            "--output-format=json".to_string(),
            "export".to_string(),
            "/tmp/notes.md".to_string(),
        ];

        // when
        let parsed = parse_args(&args).expect("json export should parse");

        // then
        assert_eq!(
            parsed,
            CliAction::Export {
                session_reference: LATEST_SESSION_REFERENCE.to_string(),
                output_path: Some(PathBuf::from("/tmp/notes.md")),
                output_format: CliOutputFormat::Json,
            }
        );
    }

    #[test]
    fn rejects_unknown_export_options_with_helpful_message() {
        // given
        let args = vec!["export".to_string(), "--bogus".to_string()];

        // when
        let error = parse_args(&args).expect_err("unknown export option should fail");

        // then
        assert!(error.contains("unknown export option: --bogus"));
    }

    #[test]
    fn rejects_export_with_extra_positional_after_path() {
        // given
        let args = vec![
            "export".to_string(),
            "first.md".to_string(),
            "second.md".to_string(),
        ];

        // when
        let error = parse_args(&args).expect_err("multiple positionals should fail");

        // then
        assert!(error.contains("unexpected export argument: second.md"));
    }

    #[test]
    fn parse_export_args_helper_defaults_to_latest_reference_and_no_output() {
        // given
        let args: Vec<String> = vec![];

        // when
        let parsed = parse_export_args(&args, CliOutputFormat::Text)
            .expect("empty export args should parse");

        // then
        assert_eq!(
            parsed,
            CliAction::Export {
                session_reference: LATEST_SESSION_REFERENCE.to_string(),
                output_path: None,
                output_format: CliOutputFormat::Text,
            }
        );
    }

    #[test]
    fn render_session_markdown_includes_header_and_summarized_tool_calls() {
        // given
        let mut session = Session::new();
        session.session_id = "session-export-test".to_string();
        session.messages = vec![
            ConversationMessage::user_text("How do I list files?"),
            ConversationMessage::assistant(vec![
                ContentBlock::Text {
                    text: "I'll run a tool.".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "toolu_abcdefghijklmnop".to_string(),
                    name: "bash".to_string(),
                    input: r#"{"command":"ls -la"}"#.to_string(),
                },
            ]),
            ConversationMessage {
                role: MessageRole::Tool,
                blocks: vec![ContentBlock::ToolResult {
                    tool_use_id: "toolu_abcdefghijklmnop".to_string(),
                    tool_name: "bash".to_string(),
                    output: "total 8\ndrwxr-xr-x  2 user staff   64 Apr  7 12:00 .".to_string(),
                    is_error: false,
                }],
                usage: None,
            },
        ];

        // when
        let markdown = render_session_markdown(
            &session,
            "session-export-test",
            std::path::Path::new("/tmp/sessions/session-export-test.jsonl"),
        );

        // then
        assert!(markdown.starts_with("# Conversation Export"));
        assert!(markdown.contains("- **Session**: `session-export-test`"));
        assert!(markdown.contains("- **Messages**: 3"));
        assert!(markdown.contains("## 1. User"));
        assert!(markdown.contains("How do I list files?"));
        assert!(markdown.contains("## 2. Assistant"));
        assert!(markdown.contains("**Tool call** `bash`"));
        assert!(markdown.contains("toolu_abcdef…"));
        assert!(markdown.contains("ls -la"));
        assert!(markdown.contains("## 3. Tool"));
        assert!(markdown.contains("**Tool result** `bash`"));
        assert!(markdown.contains("ok"));
        assert!(markdown.contains("total 8"));
    }

    #[test]
    fn render_session_markdown_marks_tool_errors_and_skips_empty_summaries() {
        // given
        let mut session = Session::new();
        session.session_id = "errs".to_string();
        session.messages = vec![ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "short".to_string(),
                tool_name: "read_file".to_string(),
                output: "   ".to_string(),
                is_error: true,
            }],
            usage: None,
        }];

        // when
        let markdown =
            render_session_markdown(&session, "errs", std::path::Path::new("errs.jsonl"));

        // then
        assert!(markdown.contains("**Tool result** `read_file` _(id `short`, error)_"));
        // an empty summary should not produce a stray blockquote line
        assert!(!markdown.contains("> \n"));
    }

    #[test]
    fn summarize_tool_payload_for_markdown_compacts_json_and_truncates_overflow() {
        // given
        let json_payload = r#"{
            "command":   "ls -la",
            "cwd": "/tmp"
        }"#;
        let long_payload = "a".repeat(600);

        // when
        let compacted = summarize_tool_payload_for_markdown(json_payload);
        let truncated = summarize_tool_payload_for_markdown(&long_payload);

        // then
        assert_eq!(compacted, r#"{"command":"ls -la","cwd":"/tmp"}"#);
        assert!(truncated.ends_with('…'));
        assert!(truncated.chars().count() <= 281);
    }

    #[test]
    fn short_tool_id_truncates_long_identifiers_with_ellipsis() {
        // given
        let long = "toolu_01ABCDEFGHIJKLMN";
        let short = "tool_1";

        // when
        let trimmed_long = short_tool_id(long);
        let trimmed_short = short_tool_id(short);

        // then
        assert_eq!(trimmed_long, "toolu_01ABCD…");
        assert_eq!(trimmed_short, "tool_1");
    }

    #[test]
    fn parses_json_output_for_mcp_and_skills_commands() {
        assert_eq!(
            parse_args(&["--output-format=json".to_string(), "mcp".to_string()])
                .expect("json mcp should parse"),
            CliAction::Mcp {
                args: None,
                output_format: CliOutputFormat::Json,
            }
        );
        assert_eq!(
            parse_args(&[
                "--output-format=json".to_string(),
                "/skills".to_string(),
                "help".to_string(),
            ])
            .expect("json /skills help should parse"),
            CliAction::Skills {
                args: Some("help".to_string()),
                output_format: CliOutputFormat::Json,
            }
        );
    }

    #[test]
    fn single_word_slash_command_names_return_guidance_instead_of_hitting_prompt_mode() {
        let error = parse_args(&["cost".to_string()]).expect_err("cost should return guidance");
        assert!(error.contains("slash command"));
        assert!(error.contains("/cost"));
    }

    #[test]
    fn multi_word_prompt_still_uses_shorthand_prompt_mode() {
        let _guard = env_lock();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        // Input is ["--model", "opus", "please", "debug", "this"] so the joined
        // prompt shorthand must stay a normal multi-word prompt while still
        // honoring alias validation at parse time.
        assert_eq!(
            parse_args(&[
                "--model".to_string(),
                "opus".to_string(),
                "please".to_string(),
                "debug".to_string(),
                "this".to_string(),
            ])
            .expect("prompt shorthand should still work"),
            CliAction::Prompt {
                prompt: "please debug this".to_string(),
                model: "anthropic/claude-opus-4-7".to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: crate::default_permission_mode(),
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn parses_direct_agents_mcp_and_skills_slash_commands() {
        let _guard = env_lock();
        let _cwd_guard = cwd_guard();
        std::env::remove_var("RUSTY_CLAUDE_PERMISSION_MODE");
        assert_eq!(
            parse_args(&["/agents".to_string()]).expect("/agents should parse"),
            CliAction::Agents {
                args: None,
                output_format: CliOutputFormat::Text
            }
        );
        assert_eq!(
            parse_args(&["/mcp".to_string(), "show".to_string(), "demo".to_string()])
                .expect("/mcp show demo should parse"),
            CliAction::Mcp {
                args: Some("show demo".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["/skills".to_string()]).expect("/skills should parse"),
            CliAction::Skills {
                args: None,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["/skill".to_string()]).expect("/skill should parse"),
            CliAction::Skills {
                args: None,
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["/skills".to_string(), "help".to_string()])
                .expect("/skills help should parse"),
            CliAction::Skills {
                args: Some("help".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["/skill".to_string(), "list".to_string()])
                .expect("/skill list should parse"),
            CliAction::Skills {
                args: Some("list".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&[
                "/skills".to_string(),
                "help".to_string(),
                "overview".to_string()
            ])
            .expect("/skills help overview should invoke"),
            CliAction::Prompt {
                prompt: "$help overview".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: crate::default_permission_mode(),
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
        assert_eq!(
            parse_args(&[
                "/skills".to_string(),
                "install".to_string(),
                "./fixtures/help-skill".to_string(),
            ])
            .expect("/skills install should parse"),
            CliAction::Skills {
                args: Some("install ./fixtures/help-skill".to_string()),
                output_format: CliOutputFormat::Text,
            }
        );
        assert_eq!(
            parse_args(&["/skills".to_string(), "/test".to_string()])
                .expect("/skills /test should normalize to a single skill prompt prefix"),
            CliAction::Prompt {
                prompt: "$test".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: crate::default_permission_mode(),
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
        assert_eq!(
            parse_args(&["/status".to_string()]).expect("/status should parse as local status"),
            CliAction::Status {
                model: DEFAULT_MODEL.to_string(),
                model_flag_raw: None,
                permission_mode: PermissionModeProvenance::default_fallback(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
            }
        );
    }

    #[test]
    fn direct_slash_commands_surface_shared_validation_errors() {
        let compact_error = parse_args(&["/compact".to_string(), "now".to_string()])
            .expect_err("invalid /compact shape should be rejected");
        assert!(compact_error.contains("Unexpected arguments for /compact."));
        assert!(compact_error.contains("Usage            /compact"));

        let plugins_error = parse_args(&[
            "/plugins".to_string(),
            "list".to_string(),
            "extra".to_string(),
        ])
        .expect_err("invalid /plugins list shape should be rejected");
        assert!(plugins_error.contains("Usage: /plugin list"));
        assert!(plugins_error.contains("Aliases          /plugins, /marketplace"));

        for alias in ["/plugin", "/plugins", "/marketplace"] {
            let error = parse_args(&[alias.to_string()])
                .expect_err("valid plugin slash aliases are local/interactive, never prompts");
            // #829: prefix changed from "interactive-only" to "interactive_only:"
            assert!(
                error.contains("interactive_only:") || error.contains("interactive-only"),
                "{alias} should reject as an interactive plugin command outside the REPL, got: {error}"
            );
        }
    }

    #[test]
    fn formats_unknown_slash_command_with_suggestions() {
        let report = format_unknown_slash_command_message("statsu");
        assert!(report.contains("unknown slash command: /statsu"));
        assert!(report.contains("Did you mean"));
        assert!(report.contains("Use /help"));
    }

    #[test]
    fn typoed_doctor_subcommand_returns_did_you_mean_error() {
        let error = parse_args(&["doctorr".to_string()]).expect_err("doctorr should error");
        assert!(error.contains("unknown subcommand: doctorr."));
        assert!(error.contains("Did you mean"));
        assert!(error.contains("doctor"));
    }

    #[test]
    fn typoed_skills_subcommand_returns_did_you_mean_error() {
        let error = parse_args(&["skilsl".to_string()]).expect_err("skilsl should error");
        assert!(error.contains("unknown subcommand: skilsl."));
        assert!(error.contains("skills"));
    }

    #[test]
    fn unsupported_skills_actions_return_typed_error_683() {
        let error = parse_args(&["skills".to_string(), "add".to_string()])
            .expect_err("skills add should error");
        assert!(
            error.contains("unsupported skills action"),
            "skills add should contain 'unsupported skills action', got: {error}"
        );
        assert_eq!(
            classify_error_kind(&error),
            "unsupported_skills_action",
            "skills add should classify as unsupported_skills_action, got: {error}"
        );

        for action in ["remove", "uninstall", "delete"] {
            assert_eq!(
                parse_args(&["skills".to_string(), action.to_string()])
                    .expect(&format!("skills {action} should parse")),
                CliAction::Skills {
                    args: Some(action.to_string()),
                    output_format: CliOutputFormat::Text,
                },
                "skills {action} should route locally so missing targets are handled without credentials"
            );
        }
    }

    #[test]
    fn typoed_status_subcommand_returns_did_you_mean_error() {
        let error = parse_args(&["statuss".to_string()]).expect_err("statuss should error");
        assert!(error.contains("unknown subcommand: statuss."));
        assert!(error.contains("status"));
    }

    #[test]
    fn typoed_export_subcommand_returns_did_you_mean_error() {
        let error = parse_args(&["exporrt".to_string()]).expect_err("exporrt should error");
        assert!(error.contains("unknown subcommand: exporrt."));
        assert!(error.contains("Did you mean"));
        assert!(error.contains("export"));
    }

    #[test]
    fn typoed_mcp_subcommand_returns_did_you_mean_error() {
        let error = parse_args(&["mcpp".to_string()]).expect_err("mcpp should error");
        assert!(error.contains("unknown subcommand: mcpp."));
        assert!(error.contains("mcp"));
    }

    #[test]
    fn multi_word_prompt_still_bypasses_subcommand_typo_guard() {
        assert_eq!(
            parse_args(&[
                "hello".to_string(),
                "world".to_string(),
                "this".to_string(),
                "is".to_string(),
                "a".to_string(),
                "prompt".to_string(),
            ])
            .expect("multi-word prompt should still parse"),
            CliAction::Prompt {
                prompt: "hello world this is a prompt".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: crate::default_permission_mode(),
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn prompt_subcommand_allows_literal_typo_word() {
        assert_eq!(
            parse_args(&["prompt".to_string(), "doctorr".to_string()])
                .expect("explicit prompt subcommand should allow literal typo word"),
            CliAction::Prompt {
                prompt: "doctorr".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::WorkspaceWrite,
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn punctuation_bearing_single_token_still_dispatches_to_prompt() {
        // #140: Guard against test pollution — isolate cwd + env so this test
        // doesn't pick up a stale .claw/settings.json from other tests that
        // may have set `permissionMode: acceptEdits` in a shared cwd.
        let _guard = env_lock();
        let root = temp_dir();
        let cwd = root.join("project");
        std::fs::create_dir_all(&cwd).expect("project dir should exist");
        let result = with_current_dir(&cwd, || {
            parse_args(&["PARITY_SCENARIO:bash_permission_prompt_approved".to_string()])
                .expect("scenario token should still dispatch to prompt")
        });
        assert_eq!(
            result,
            CliAction::Prompt {
                prompt: "PARITY_SCENARIO:bash_permission_prompt_approved".to_string(),
                model: DEFAULT_MODEL.to_string(),
                output_format: CliOutputFormat::Text,
                allowed_tools: None,
                permission_mode: PermissionMode::WorkspaceWrite,
                compact: false,
                base_commit: None,
                reasoning_effort: None,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn formats_namespaced_omc_slash_command_with_contract_guidance() {
        let report = format_unknown_slash_command_message("oh-my-claudecode:hud");
        assert!(report.contains("unknown slash command: /oh-my-claudecode:hud"));
        assert!(report.contains("Claude Code/OMC plugin command"));
        assert!(report.contains("plugin slash commands"));
        assert!(report.contains("statusline"));
        assert!(report.contains("session hooks"));
    }

    #[test]
    fn parses_resume_flag_with_slash_command() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/compact".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec!["/compact".to_string()],
                output_format: CliOutputFormat::Text,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn parses_resume_flag_without_path_as_latest_session() {
        assert_eq!(
            parse_args(&["--resume".to_string()]).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("latest"),
                commands: vec![],
                output_format: CliOutputFormat::Text,
                allow_broad_cwd: false,
            }
        );
        assert_eq!(
            parse_args(&["--resume".to_string(), "/status".to_string()])
                .expect("resume shortcut should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("latest"),
                commands: vec!["/status".to_string()],
                output_format: CliOutputFormat::Text,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn parses_resume_flag_with_multiple_slash_commands() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/status".to_string(),
            "/compact".to_string(),
            "/cost".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec![
                    "/status".to_string(),
                    "/compact".to_string(),
                    "/cost".to_string(),
                ],
                output_format: CliOutputFormat::Text,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn rejects_unknown_options_with_helpful_guidance() {
        let error = parse_args(&["--resum".to_string()]).expect_err("unknown option should fail");
        assert!(error.contains("unknown option: --resum"));
        assert!(error.contains("Did you mean --resume?"));
        assert!(error.contains("claw --help"));
    }

    #[test]
    fn parses_resume_flag_with_slash_command_arguments() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/export".to_string(),
            "notes.txt".to_string(),
            "/clear".to_string(),
            "--confirm".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec![
                    "/export notes.txt".to_string(),
                    "/clear --confirm".to_string(),
                ],
                output_format: CliOutputFormat::Text,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn parses_resume_flag_with_absolute_export_path() {
        let args = vec![
            "--resume".to_string(),
            "session.jsonl".to_string(),
            "/export".to_string(),
            "/tmp/notes.txt".to_string(),
            "/status".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args should parse"),
            CliAction::ResumeSession {
                session_path: PathBuf::from("session.jsonl"),
                commands: vec!["/export /tmp/notes.txt".to_string(), "/status".to_string()],
                output_format: CliOutputFormat::Text,
                allow_broad_cwd: false,
            }
        );
    }

    #[test]
    fn filtered_tool_specs_respect_allowlist() {
        let allowed = ["read_file", "grep_search"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let filtered = filter_tool_specs(&GlobalToolRegistry::builtin(), Some(&allowed));
        let names = filtered
            .into_iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["read_file", "grep_search"]);
    }

    #[test]
    fn filtered_tool_specs_include_plugin_tools() {
        let filtered = filter_tool_specs(&registry_with_plugin_tool(), None);
        let names = filtered
            .into_iter()
            .map(|definition| definition.name)
            .collect::<Vec<_>>();
        assert!(names.contains(&"bash".to_string()));
        assert!(names.contains(&"plugin_echo".to_string()));
    }

    #[test]
    fn permission_policy_uses_plugin_tool_permissions() {
        let feature_config = runtime::RuntimeFeatureConfig::default();
        let policy = permission_policy(
            PermissionMode::ReadOnly,
            &feature_config,
            &registry_with_plugin_tool(),
        )
        .expect("permission policy should build");
        let required = policy.required_mode_for("plugin_echo");
        assert_eq!(required, PermissionMode::WorkspaceWrite);
    }

    #[test]
    fn shared_help_uses_resume_annotation_copy() {
        let help = commands::render_slash_command_help();
        assert!(help.contains("Slash commands"));
        assert!(help.contains("works with --resume SESSION.jsonl"));
    }

    #[test]
    fn bare_skill_dispatch_resolves_known_project_skill_to_prompt() {
        let _guard = env_lock();
        let workspace = temp_dir();
        write_skill_fixture(
            &workspace.join(".codex").join("skills"),
            "caveman",
            "Project skill fixture",
        );

        let prompt = try_resolve_bare_skill_prompt(&workspace, "caveman sharpen club")
            .expect("known bare skill should dispatch");
        assert_eq!(prompt, "$caveman sharpen club");

        fs::remove_dir_all(workspace).expect("workspace should clean up");
    }

    #[test]
    fn bare_skill_dispatch_ignores_unknown_or_non_skill_input() {
        let _guard = env_lock();
        let workspace = temp_dir();
        fs::create_dir_all(&workspace).expect("workspace should exist");

        assert_eq!(
            try_resolve_bare_skill_prompt(&workspace, "not-a-known-skill do thing"),
            None
        );
        assert_eq!(try_resolve_bare_skill_prompt(&workspace, "/status"), None);

        fs::remove_dir_all(workspace).expect("workspace should clean up");
    }

    #[test]
    fn repl_help_includes_shared_commands_and_exit() {
        let help = render_repl_help();
        assert!(help.contains("REPL"));
        assert!(help.contains("/help"));
        assert!(help.contains("Complete commands, modes, and recent sessions"));
        assert!(help.contains("/status"));
        assert!(help.contains("/sandbox"));
        assert!(help.contains("/model [model]"));
        assert!(help.contains("/permissions [read-only|workspace-write|danger-full-access]"));
        assert!(help.contains("/clear [--confirm]"));
        assert!(help.contains("/cost"));
        assert!(help.contains("/resume <session-path>"));
        assert!(help.contains("/config [env|hooks|model|plugins]"));
        assert!(help.contains("/mcp [list|show <server>|help]"));
        assert!(help.contains("/memory"));
        assert!(help.contains("/init"));
        assert!(help.contains("/diff"));
        assert!(help.contains("/version"));
        assert!(help.contains("/export [file]"));
        // Batch 5 added `/session delete`; match on the stable core rather than
        // the trailing bracket so future additions don't re-break this.
        assert!(help
            .contains("/session [list|exists <session-id>|switch <session-id>|fork [branch-name]"));
        assert!(help.contains(
            "/plugin [list|install <path>|enable <name>|disable <name>|uninstall <id>|update <id>]"
        ));
        assert!(help.contains("aliases: /plugins, /marketplace"));
        assert!(help.contains("/agents"));
        assert!(help.contains("/skills"));
        assert!(help.contains("/exit"));
        assert!(help.contains(
            "Auto-save            .claw/sessions/<workspace-fingerprint>/<session-id>.jsonl"
        ));
        assert!(help.contains("Resume latest        /resume latest"));
    }

    #[test]
    fn completion_candidates_include_workflow_shortcuts_and_dynamic_sessions() {
        let completions = slash_command_completion_candidates_with_sessions(
            "sonnet",
            Some("session-current"),
            vec!["session-old".to_string()],
        );

        assert!(completions.contains(&"/model anthropic/claude-sonnet-4-6".to_string()));
        assert!(completions.contains(&"/permissions workspace-write".to_string()));
        assert!(completions.contains(&"/session list".to_string()));
        assert!(completions.contains(&"/session switch session-current".to_string()));
        assert!(completions.contains(&"/resume session-old".to_string()));
        assert!(completions.contains(&"/mcp list".to_string()));
        assert!(completions.contains(&"/ultraplan ".to_string()));
    }

    #[test]
    fn startup_banner_mentions_workflow_completions() {
        let _guard = env_lock();
        // Inject dummy credentials so LiveCli can construct without real Anthropic key
        std::env::set_var("ANTHROPIC_API_KEY", "test-dummy-key-for-banner-test");
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");

        let banner = with_current_dir(&root, || {
            LiveCli::new(
                "anthropic/claude-sonnet-4-6".to_string(),
                true,
                None,
                PermissionMode::DangerFullAccess,
            )
            .expect("cli should initialize")
            .startup_banner()
        });

        assert!(banner.contains("Tab"));
        assert!(banner.contains("workflow completions"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn format_connected_line_renders_anthropic_provider_for_claude_model() {
        let model = "anthropic/claude-sonnet-4-6";

        let line = format_connected_line(model);

        assert_eq!(line, "Connected: anthropic/claude-sonnet-4-6 via anthropic");
    }

    #[test]
    fn format_connected_line_renders_xai_provider_for_grok_model() {
        let model = "grok-3";

        let line = format_connected_line(model);

        assert_eq!(line, "Connected: grok-3 via xai");
    }

    #[test]
    fn resolve_repl_model_returns_user_supplied_model_unchanged_when_explicit() {
        let user_model = "anthropic/claude-sonnet-4-6".to_string();

        let resolved = resolve_repl_model(user_model).expect("explicit model should resolve");

        assert_eq!(resolved, "anthropic/claude-sonnet-4-6");
    }

    #[test]
    fn resolve_repl_model_falls_back_to_anthropic_model_env_when_default() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        let config_home = root.join("config");
        fs::create_dir_all(&config_home).expect("config home dir");
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        std::env::remove_var("ANTHROPIC_MODEL");
        std::env::set_var("ANTHROPIC_MODEL", "sonnet");

        let resolved = with_current_dir(&root, || resolve_repl_model(DEFAULT_MODEL.to_string()))
            .expect("env model should resolve");

        assert_eq!(resolved, "anthropic/claude-sonnet-4-6");

        std::env::remove_var("ANTHROPIC_MODEL");
        std::env::remove_var("CLAW_CONFIG_HOME");
        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn resolve_repl_model_returns_default_when_env_unset_and_no_config() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        let config_home = root.join("config");
        fs::create_dir_all(&config_home).expect("config home dir");
        std::env::set_var("CLAW_CONFIG_HOME", &config_home);
        std::env::remove_var("ANTHROPIC_MODEL");

        let resolved = with_current_dir(&root, || resolve_repl_model(DEFAULT_MODEL.to_string()))
            .expect("default model should resolve");

        assert_eq!(resolved, DEFAULT_MODEL);

        std::env::remove_var("CLAW_CONFIG_HOME");
        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn resume_supported_command_list_matches_expected_surface() {
        let names = resume_supported_slash_commands()
            .into_iter()
            .map(|spec| spec.name)
            .collect::<Vec<_>>();
        // Now with 135+ slash commands, verify minimum resume support
        assert!(
            names.len() >= 39,
            "expected at least 39 resume-supported commands, got {}",
            names.len()
        );
        // Verify key resume commands still exist
        assert!(names.contains(&"help"));
        assert!(names.contains(&"status"));
        assert!(names.contains(&"compact"));
    }

    #[test]
    fn session_exists_resume_command_reports_json_contract() {
        let session = Session::new();
        let path = PathBuf::from("missing-session.jsonl");
        let outcome = run_resume_command(
            &path,
            &session,
            &SlashCommand::Session {
                action: Some("exists".to_string()),
                target: Some("definitely-missing-session".to_string()),
            },
        )
        .expect("exists command should not fail for missing sessions");

        let json = outcome.json.expect("json contract");
        assert_eq!(json["kind"], "session_exists");
        assert_eq!(json["exists"], false);
        assert_eq!(json["session"], "definitely-missing-session");
    }

    #[test]
    fn resume_report_uses_sectioned_layout() {
        let report = format_resume_report("session.jsonl", 14, 6);
        assert!(report.contains("Session resumed"));
        assert!(report.contains("Session file     session.jsonl"));
        assert!(report.contains("Messages         14"));
        assert!(report.contains("Turns            6"));
    }

    #[test]
    fn compact_report_uses_structured_output() {
        let compacted = format_compact_report(8, 5, false);
        assert!(compacted.contains("Compact"));
        assert!(compacted.contains("Result           compacted"));
        assert!(compacted.contains("Messages removed 8"));
        let skipped = format_compact_report(0, 3, true);
        assert!(skipped.contains("Result           skipped"));
    }

    #[test]
    fn cost_report_uses_sectioned_layout() {
        let report = format_cost_report(runtime::TokenUsage {
            input_tokens: 20,
            output_tokens: 8,
            cache_creation_input_tokens: 3,
            cache_read_input_tokens: 1,
        });
        assert!(report.contains("Cost"));
        assert!(report.contains("Input tokens     20"));
        assert!(report.contains("Output tokens    8"));
        assert!(report.contains("Cache create     3"));
        assert!(report.contains("Cache read       1"));
        assert!(report.contains("Total tokens     32"));
        assert!(report.contains("Estimated cost"));
    }

    #[test]
    fn permissions_report_uses_sectioned_layout() {
        let report = format_permissions_report("workspace-write");
        assert!(report.contains("Permissions"));
        assert!(report.contains("Active mode      workspace-write"));
        assert!(report.contains("Modes"));
        assert!(report.contains("read-only          ○ available Read/search tools only"));
        assert!(report.contains("workspace-write    ● current   Edit files inside the workspace"));
        assert!(report.contains("danger-full-access ○ available Unrestricted tool access"));
    }

    #[test]
    fn permissions_switch_report_is_structured() {
        let report = format_permissions_switch_report("read-only", "workspace-write");
        assert!(report.contains("Permissions updated"));
        assert!(report.contains("Result           mode switched"));
        assert!(report.contains("Previous mode    read-only"));
        assert!(report.contains("Active mode      workspace-write"));
        assert!(report.contains("Applies to       subsequent tool calls"));
    }

    #[test]
    fn init_help_mentions_direct_subcommand() {
        let mut help = Vec::new();
        print_help_to(&mut help).expect("help should render");
        let help = String::from_utf8(help).expect("help should be utf8");
        assert!(help.contains("claw help"));
        assert!(help.contains("claw version"));
        assert!(help.contains("claw status"));
        assert!(help.contains("claw sandbox"));
        assert!(help.contains("claw init"));
        assert!(help.contains("claw acp [serve]"));
        assert!(help.contains("claw agents"));
        assert!(help.contains("claw mcp"));
        assert!(help.contains("claw skills"));
        assert!(help.contains("claw /skills"));
        assert!(help.contains("ultraworkers/claw-code"));
        assert!(help.contains("cargo install claw-code"));
        assert!(!help.contains("claw login"));
        assert!(!help.contains("claw logout"));
    }

    #[test]
    fn model_report_uses_sectioned_layout() {
        let report = format_model_report("claude-sonnet", 12, 4);
        assert!(report.contains("Model"));
        assert!(report.contains("Current model    claude-sonnet"));
        assert!(report.contains("Session messages 12"));
        assert!(report.contains("Switch models with /model <name>"));
    }

    fn test_branch_freshness() -> super::BranchFreshness {
        super::BranchFreshness {
            upstream: Some("origin/main".to_string()),
            ahead: 0,
            behind: 0,
            fresh: Some(true),
        }
    }

    fn test_boot_preflight() -> super::BootPreflightSnapshot {
        super::BootPreflightSnapshot {
            repo_exists: true,
            worktree_exists: true,
            git_dir_exists: true,
            branch_freshness: test_branch_freshness(),
            trust_gate_allowed: Some(false),
            trusted_roots_count: 0,
            required_binaries: Vec::new(),
            control_sockets: Vec::new(),
            mcp_startup_eligible: true,
            mcp_servers_configured: 0,
            plugin_startup_eligible: true,
            plugins_configured: 0,
            last_failed_boot_reason: None,
        }
    }

    #[test]
    fn model_switch_report_preserves_context_summary() {
        let report = format_model_switch_report("claude-sonnet", "claude-opus", 9);
        assert!(report.contains("Model updated"));
        assert!(report.contains("Previous         claude-sonnet"));
        assert!(report.contains("Current          claude-opus"));
        assert!(report.contains("Preserved msgs   9"));
    }

    #[test]
    fn status_line_reports_model_and_token_totals() {
        let status = format_status_report(
            "claude-sonnet",
            StatusUsage {
                message_count: 7,
                turns: 3,
                latest: runtime::TokenUsage {
                    input_tokens: 5,
                    output_tokens: 4,
                    cache_creation_input_tokens: 1,
                    cache_read_input_tokens: 0,
                },
                cumulative: runtime::TokenUsage {
                    input_tokens: 20,
                    output_tokens: 8,
                    cache_creation_input_tokens: 2,
                    cache_read_input_tokens: 1,
                },
                estimated_tokens: 128,
            },
            "workspace-write",
            &super::StatusContext {
                cwd: PathBuf::from("/tmp/project"),
                session_path: Some(PathBuf::from("session.jsonl")),
                loaded_config_files: 2,
                discovered_config_files: 3,
                memory_file_count: 4,
                memory_files: vec![super::MemoryFileSummary {
                    path: "/tmp/project/CLAUDE.md".to_string(),
                    source: "claude_md".to_string(),
                    origin: "workspace".to_string(),
                    scope_path: "/tmp/project".to_string(),
                    outside_project: false,
                    chars: 42,
                    contributes: true,
                }],
                unloaded_memory_files: Vec::new(),
                project_root: Some(PathBuf::from("/tmp")),
                git_branch: Some("main".to_string()),
                git_summary: GitWorkspaceSummary {
                    changed_files: 3,
                    staged_files: 1,
                    unstaged_files: 1,
                    untracked_files: 1,
                    conflicted_files: 0,
                    operation: GitOperation::None,
                },
                branch_freshness: test_branch_freshness(),
                stale_base_state: super::BaseCommitState::NoExpectedBase,
                session_lifecycle: SessionLifecycleSummary {
                    kind: SessionLifecycleKind::IdleShell,
                    pane_id: Some("%7".to_string()),
                    pane_command: Some("zsh".to_string()),
                    pane_path: Some(PathBuf::from("/tmp/project")),
                    workspace_dirty: true,
                    abandoned: true,
                    all_panes: vec![],
                },
                boot_preflight: test_boot_preflight(),
                sandbox_status: runtime::SandboxStatus::default(),
                binary_provenance: super::binary_provenance_for(None),
                config_load_error: None,
                config_load_error_kind: None,
                mcp_validation: super::McpValidationSummary::default(),

                hook_validation: super::HookValidationSummary::default(),
                duplicate_flags: Vec::new(),
            },
            None, // #148
            None,
        );
        assert!(status.contains("Status"));
        assert!(status.contains("Model            claude-sonnet"));
        assert!(status.contains("Permission mode  workspace-write"));
        assert!(status.contains("Messages         7"));
        assert!(status.contains("Latest total     10"));
        assert!(status.contains("Cache create     2"));
        assert!(status.contains("Cache read       1"));
        assert!(status.contains("Cumulative total 31"));
        assert!(status.contains("Estimated cost"));
        assert!(status.contains("Cwd              /tmp/project"));
        assert!(status.contains("Project root     /tmp"));
        assert!(status.contains("Git branch       main"));
        assert!(
            status.contains("Git state        dirty · 3 files · 1 staged, 1 unstaged, 1 untracked")
        );
        assert!(status.contains("Changed files    3"));
        assert!(status.contains("Loaded memory    claude_md:/tmp/project/CLAUDE.md"));
        assert!(status.contains("Staged           1"));
        assert!(status.contains("Unstaged         1"));
        assert!(status.contains("Untracked        1"));
        assert!(status.contains("Session          session.jsonl"));
        assert!(
            status.contains("Lifecycle        idle shell · dirty worktree · abandoned? · cmd=zsh")
        );
        assert!(status.contains("Config files     loaded 2/3"));
        assert!(status.contains("Memory files     4"));
        assert!(status.contains("Suggested flow   /status → /diff → /commit"));
    }

    #[test]
    fn session_lifecycle_prefers_running_process_over_idle_shell() {
        let workspace = PathBuf::from("/tmp/project");
        let lifecycle = classify_session_lifecycle_from_panes(
            &workspace,
            vec![
                TmuxPaneSnapshot {
                    pane_id: "%1".to_string(),
                    current_command: "zsh".to_string(),
                    current_path: workspace.clone(),
                },
                TmuxPaneSnapshot {
                    pane_id: "%2".to_string(),
                    current_command: "claw".to_string(),
                    current_path: workspace.join("rust"),
                },
            ],
        );

        assert_eq!(lifecycle.kind, SessionLifecycleKind::RunningProcess);
        assert_eq!(lifecycle.pane_id.as_deref(), Some("%2"));
        assert_eq!(lifecycle.pane_command.as_deref(), Some("claw"));
        assert!(!lifecycle.abandoned);
    }

    #[test]
    fn session_lifecycle_marks_dirty_idle_shell_as_abandoned() {
        let _guard = env_lock();
        let workspace = temp_workspace("dirty-idle-shell");
        fs::create_dir_all(&workspace).expect("workspace should create");
        git(&["init", "--quiet"], &workspace);
        git(&["config", "user.email", "tests@example.com"], &workspace);
        git(&["config", "user.name", "Rusty Claude Tests"], &workspace);
        fs::write(workspace.join("tracked.txt"), "hello\n").expect("write tracked");
        git(&["add", "tracked.txt"], &workspace);
        git(&["commit", "-m", "init", "--quiet"], &workspace);
        fs::write(workspace.join("tracked.txt"), "hello\nchanged\n").expect("dirty tracked");

        let lifecycle = classify_session_lifecycle_from_panes(
            &workspace,
            vec![TmuxPaneSnapshot {
                pane_id: "%3".to_string(),
                current_command: "bash".to_string(),
                current_path: workspace.clone(),
            }],
        );

        assert_eq!(lifecycle.kind, SessionLifecycleKind::IdleShell);
        assert!(lifecycle.workspace_dirty);
        assert!(lifecycle.abandoned);

        fs::remove_dir_all(workspace).expect("cleanup temp dir");
    }

    #[test]
    fn session_list_surfaces_saved_dirty_abandoned_lifecycle() {
        let _guard = cwd_guard();
        let workspace = temp_workspace("session-list-lifecycle");
        fs::create_dir_all(&workspace).expect("workspace should create");
        git(&["init", "--quiet"], &workspace);
        git(&["config", "user.email", "tests@example.com"], &workspace);
        git(&["config", "user.name", "Rusty Claude Tests"], &workspace);
        fs::write(workspace.join(".gitignore"), ".claw/\n").expect("write gitignore");
        fs::write(workspace.join("tracked.txt"), "hello\n").expect("write tracked");
        git(&["add", ".gitignore", "tracked.txt"], &workspace);
        git(&["commit", "-m", "init", "--quiet"], &workspace);

        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace).expect("switch cwd");
        let handle = create_managed_session_handle("session-alpha").expect("session handle");
        Session::new()
            .with_workspace_root(workspace.clone())
            .with_persistence_path(handle.path.clone())
            .save_to_path(&handle.path)
            .expect("session should save");
        fs::write(workspace.join("tracked.txt"), "hello\nchanged\n").expect("dirty tracked");

        let report = render_session_list("session-alpha").expect("session list should render");

        assert!(report.contains("session-alpha"));
        assert!(report.contains("lifecycle=saved only · dirty worktree · abandoned?"));

        std::env::set_current_dir(previous).expect("restore cwd");
        fs::remove_dir_all(workspace).expect("cleanup temp dir");
    }

    #[test]
    fn workspace_health_warns_when_stale_base_diverged() {
        let context = super::StatusContext {
            cwd: PathBuf::from("/tmp/project"),
            session_path: None,
            loaded_config_files: 0,
            discovered_config_files: 0,
            memory_file_count: 0,
            memory_files: Vec::new(),
            unloaded_memory_files: Vec::new(),
            project_root: Some(PathBuf::from("/tmp/project")),
            git_branch: Some("feature/stale-base".to_string()),
            git_summary: GitWorkspaceSummary::default(),
            branch_freshness: test_branch_freshness(),
            stale_base_state: super::BaseCommitState::Diverged {
                expected: "base".to_string(),
                actual: "head".to_string(),
            },
            session_lifecycle: SessionLifecycleSummary {
                kind: SessionLifecycleKind::SavedOnly,
                pane_id: None,
                pane_command: None,
                pane_path: None,
                workspace_dirty: false,
                abandoned: false,
                all_panes: vec![],
            },
            boot_preflight: test_boot_preflight(),
            sandbox_status: runtime::SandboxStatus::default(),
            binary_provenance: super::binary_provenance_for(None),
            config_load_error: None,
            config_load_error_kind: None,
            mcp_validation: super::McpValidationSummary::default(),

            hook_validation: super::HookValidationSummary::default(),
            duplicate_flags: Vec::new(),
        };

        let check = super::check_workspace_health(&context);

        assert_eq!(check.level, super::DiagnosticLevel::Warn);
        assert_eq!(check.data["stale_base"]["status"], "diverged");
        assert_eq!(check.data["stale_base"]["fresh"], false);
        assert!(check
            .details
            .iter()
            .any(|detail| detail.contains("stale codebase")));
    }

    #[test]
    fn memory_health_surfaces_loaded_and_unloaded_files_438() {
        let context = super::StatusContext {
            cwd: PathBuf::from("/tmp/project"),
            session_path: None,
            loaded_config_files: 0,
            discovered_config_files: 0,
            memory_file_count: 1,
            memory_files: vec![super::MemoryFileSummary {
                path: "/tmp/project/CLAUDE.md".to_string(),
                source: "claude_md".to_string(),
                origin: "workspace".to_string(),
                scope_path: "/tmp/project".to_string(),
                outside_project: false,
                chars: 12,
                contributes: true,
            }],
            unloaded_memory_files: vec!["/tmp/project/AGENTS.md".to_string()],
            project_root: Some(PathBuf::from("/tmp/project")),
            git_branch: Some("main".to_string()),
            git_summary: GitWorkspaceSummary::default(),
            branch_freshness: test_branch_freshness(),
            stale_base_state: super::BaseCommitState::NoExpectedBase,
            session_lifecycle: SessionLifecycleSummary {
                kind: SessionLifecycleKind::SavedOnly,
                pane_id: None,
                pane_command: None,
                pane_path: None,
                workspace_dirty: false,
                abandoned: false,
                all_panes: vec![],
            },
            boot_preflight: test_boot_preflight(),
            sandbox_status: runtime::SandboxStatus::default(),
            binary_provenance: super::binary_provenance_for(None),
            config_load_error: None,
            config_load_error_kind: None,
            mcp_validation: super::McpValidationSummary::default(),

            hook_validation: super::HookValidationSummary::default(),
            duplicate_flags: Vec::new(),
        };

        let check = super::check_memory_health(&context);

        assert_eq!(check.level, super::DiagnosticLevel::Warn);
        assert_eq!(check.data["memory_file_count"], 1);
        assert_eq!(check.data["memory_files"][0]["source"], "claude_md");
        assert_eq!(
            check.data["unloaded_memory_files"][0],
            "/tmp/project/AGENTS.md"
        );
    }

    #[test]
    fn status_json_surfaces_session_lifecycle_for_clawhip() {
        let context = super::StatusContext {
            cwd: PathBuf::from("/tmp/project"),
            session_path: None,
            loaded_config_files: 0,
            discovered_config_files: 0,
            memory_file_count: 0,
            memory_files: Vec::new(),
            unloaded_memory_files: Vec::new(),
            project_root: Some(PathBuf::from("/tmp/project")),
            git_branch: Some("feature/session-lifecycle".to_string()),
            git_summary: GitWorkspaceSummary::default(),
            branch_freshness: test_branch_freshness(),
            stale_base_state: super::BaseCommitState::NoExpectedBase,
            session_lifecycle: SessionLifecycleSummary {
                kind: SessionLifecycleKind::RunningProcess,
                pane_id: Some("%9".to_string()),
                pane_command: Some("claw".to_string()),
                pane_path: Some(PathBuf::from("/tmp/project")),
                workspace_dirty: false,
                abandoned: false,
                all_panes: vec![],
            },
            boot_preflight: test_boot_preflight(),
            sandbox_status: runtime::SandboxStatus::default(),
            binary_provenance: super::binary_provenance_for(None),
            config_load_error: None,
            config_load_error_kind: None,
            mcp_validation: super::McpValidationSummary::default(),

            hook_validation: super::HookValidationSummary::default(),
            duplicate_flags: Vec::new(),
        };

        let value = status_json_value(
            Some("claude-sonnet"),
            StatusUsage {
                message_count: 0,
                turns: 0,
                latest: runtime::TokenUsage::default(),
                cumulative: runtime::TokenUsage::default(),
                estimated_tokens: 0,
            },
            "workspace-write",
            &context,
            None,
            None,
            None,
            None,
        );

        assert_eq!(
            value["workspace"]["session_lifecycle"]["kind"],
            "running_process"
        );
        assert_eq!(
            value["workspace"]["session_lifecycle"]["pane_command"],
            "claw"
        );
        assert_eq!(value["workspace"]["session_lifecycle"]["abandoned"], false);
        assert_eq!(value["workspace"]["branch_freshness"]["fresh"], true);
        assert_eq!(
            value["workspace"]["boot_preflight"]["repo"]["worktree_exists"],
            true
        );
        assert_eq!(
            value["workspace"]["boot_preflight"]["mcp_startup"]["eligible"],
            true
        );
        assert_eq!(
            value["workspace"]["boot_preflight"]["last_failed_boot_reason"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn branch_freshness_parses_ahead_behind_status_header() {
        let freshness = super::BranchFreshness::from_git_status(Some(
            "## feature/boot...origin/feature/boot [ahead 2, behind 3]\n M src/main.rs",
        ));

        assert_eq!(freshness.upstream.as_deref(), Some("origin/feature/boot"));
        assert_eq!(freshness.ahead, 2);
        assert_eq!(freshness.behind, 3);
        assert_eq!(freshness.fresh, Some(false));
    }

    #[test]
    fn boot_preflight_snapshot_reports_machine_readable_contract_fields() {
        let _guard = env_lock();
        let workspace = temp_workspace("boot-preflight-json");
        fs::create_dir_all(&workspace).expect("workspace should create");
        git(&["init", "--quiet"], &workspace);
        git(&["config", "user.email", "tests@example.com"], &workspace);
        git(&["config", "user.name", "Rusty Claude Tests"], &workspace);
        fs::write(workspace.join("tracked.txt"), "hello\n").expect("write tracked");
        fs::write(workspace.join(".claw.json"), r#"{"trustedRoots": ["."]}"#)
            .expect("write config");
        git(&["add", "tracked.txt"], &workspace);
        git(&["commit", "-m", "init", "--quiet"], &workspace);

        let loader = ConfigLoader::default_for(&workspace);
        let config = loader.load().expect("config should load");
        let status = super::run_git_capture_in(&workspace, &["status", "--short", "--branch"]);
        let snapshot = super::build_boot_preflight_snapshot(
            &workspace,
            Some(&workspace),
            status.as_deref(),
            Some(&config),
            None,
        );
        let json = snapshot.json_value();

        assert_eq!(json["repo"]["exists"], true);
        assert_eq!(json["repo"]["worktree_exists"], true);
        assert_eq!(json["trust_gate"]["allowlisted"], true);
        assert_eq!(json["mcp_startup"]["eligible"], true);
        assert!(json["required_binaries"]
            .as_array()
            .is_some_and(|items| { items.iter().any(|item| item["name"] == "git") }));
        fs::remove_dir_all(workspace).expect("cleanup temp dir");
    }

    #[test]
    fn commit_reports_surface_workspace_context() {
        let summary = GitWorkspaceSummary {
            changed_files: 2,
            staged_files: 1,
            unstaged_files: 1,
            untracked_files: 0,
            conflicted_files: 0,
            operation: GitOperation::None,
        };

        let preflight = format_commit_preflight_report(Some("feature/ux"), summary);
        assert!(preflight.contains("Result           ready"));
        assert!(preflight.contains("Branch           feature/ux"));
        assert!(preflight.contains("Workspace        dirty · 2 files · 1 staged, 1 unstaged"));
        assert!(preflight
            .contains("Action           create a git commit from the current workspace changes"));
    }

    #[test]
    fn commit_skipped_report_points_to_next_steps() {
        let report = format_commit_skipped_report();
        assert!(report.contains("Reason           no workspace changes"));
        assert!(report
            .contains("Action           create a git commit from the current workspace changes"));
        assert!(report.contains("/status to inspect context"));
        assert!(report.contains("/diff to inspect repo changes"));
    }

    #[test]
    fn runtime_slash_reports_describe_command_behavior() {
        let bughunter = format_bughunter_report(Some("runtime"));
        assert!(bughunter.contains("Scope            runtime"));
        assert!(bughunter.contains("inspect the selected code for likely bugs"));

        let ultraplan = format_ultraplan_report(Some("ship the release"));
        assert!(ultraplan.contains("Task             ship the release"));
        assert!(ultraplan.contains("break work into a multi-step execution plan"));

        let pr = format_pr_report("feature/ux", Some("ready for review"));
        assert!(pr.contains("Branch           feature/ux"));
        assert!(pr.contains("draft or create a pull request"));

        let issue = format_issue_report(Some("flaky test"));
        assert!(issue.contains("Context          flaky test"));
        assert!(issue.contains("draft or create a GitHub issue"));
    }

    #[test]
    fn no_arg_commands_reject_unexpected_arguments() {
        assert!(validate_no_args("/commit", None).is_ok());

        let error = validate_no_args("/commit", Some("now"))
            .expect_err("unexpected arguments should fail")
            .to_string();
        assert!(error.contains("/commit does not accept arguments"));
        assert!(error.contains("Received: now"));
    }

    #[test]
    fn config_report_supports_section_views() {
        let report = render_config_report(Some("env")).expect("config report should render");
        assert!(report.contains("Merged section: env"));
        let plugins_report =
            render_config_report(Some("plugins")).expect("plugins config report should render");
        assert!(plugins_report.contains("Merged section: plugins"));
    }

    #[test]
    fn memory_report_uses_sectioned_layout() {
        let report = render_memory_report().expect("memory report should render");
        assert!(report.contains("Memory"));
        assert!(report.contains("Working directory"));
        assert!(report.contains("Instruction files"));
        assert!(report.contains("Discovered files"));
    }

    #[test]
    fn config_report_uses_sectioned_layout() {
        let report = render_config_report(None).expect("config report should render");
        assert!(report.contains("Config"));
        assert!(report.contains("Discovered files"));
        assert!(report.contains("Merged JSON"));
    }

    #[test]
    fn parses_git_status_metadata() {
        let _guard = env_lock();
        let temp_root = temp_dir();
        fs::create_dir_all(&temp_root).expect("root dir");
        let (project_root, branch) = parse_git_status_metadata_for(
            &temp_root,
            Some(
                "## rcc/cli...origin/rcc/cli
 M src/main.rs",
            ),
        );
        assert_eq!(branch.as_deref(), Some("rcc/cli"));
        assert!(project_root.is_none());
        fs::remove_dir_all(temp_root).expect("cleanup temp dir");
    }

    #[test]
    fn parses_detached_head_from_status_snapshot() {
        let _guard = env_lock();
        assert_eq!(
            parse_git_status_branch(Some(
                "## HEAD (no branch)
 M src/main.rs"
            )),
            Some("detached HEAD".to_string())
        );
    }

    #[test]
    fn parses_git_workspace_summary_counts() {
        let summary = parse_git_workspace_summary(Some(
            "## feature/ux
M  src/main.rs
 M README.md
?? notes.md
UU conflicted.rs",
        ));

        assert_eq!(
            summary,
            GitWorkspaceSummary {
                changed_files: 4,
                staged_files: 2,
                unstaged_files: 2,
                untracked_files: 1,
                conflicted_files: 1,
                operation: GitOperation::None,
            }
        );
        assert_eq!(
            summary.headline(),
            "dirty · 4 files · 2 staged, 2 unstaged, 1 untracked, 1 conflicted"
        );
    }

    #[test]
    fn render_diff_report_shows_clean_tree_for_committed_repo() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Rusty Claude Tests"], &root);
        fs::write(root.join("tracked.txt"), "hello\n").expect("write file");
        git(&["add", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);

        let report = render_diff_report_for(&root).expect("diff report should render");
        assert!(report.contains("clean working tree"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn render_diff_report_includes_staged_and_unstaged_sections() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Rusty Claude Tests"], &root);
        fs::write(root.join("tracked.txt"), "hello\n").expect("write file");
        git(&["add", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);

        fs::write(root.join("tracked.txt"), "hello\nstaged\n").expect("update file");
        git(&["add", "tracked.txt"], &root);
        fs::write(root.join("tracked.txt"), "hello\nstaged\nunstaged\n")
            .expect("update file twice");

        let report = render_diff_report_for(&root).expect("diff report should render");
        assert!(report.contains("Staged changes:"));
        assert!(report.contains("Unstaged changes:"));
        assert!(report.contains("tracked.txt"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn render_diff_report_omits_ignored_files() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Rusty Claude Tests"], &root);
        fs::write(root.join(".gitignore"), ".omx/\nignored.txt\n").expect("write gitignore");
        fs::write(root.join("tracked.txt"), "hello\n").expect("write tracked");
        git(&["add", ".gitignore", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);
        fs::create_dir_all(root.join(".omx")).expect("write omx dir");
        fs::write(root.join(".omx").join("state.json"), "{}").expect("write ignored omx");
        fs::write(root.join("ignored.txt"), "secret\n").expect("write ignored file");
        fs::write(root.join("tracked.txt"), "hello\nworld\n").expect("write tracked change");

        let report = render_diff_report_for(&root).expect("diff report should render");
        assert!(report.contains("tracked.txt"));
        assert!(!report.contains("+++ b/ignored.txt"));
        assert!(!report.contains("+++ b/.omx/state.json"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn resume_diff_command_renders_report_for_saved_session() {
        let _guard = env_lock();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        git(&["init", "--quiet"], &root);
        git(&["config", "user.email", "tests@example.com"], &root);
        git(&["config", "user.name", "Rusty Claude Tests"], &root);
        fs::write(root.join("tracked.txt"), "hello\n").expect("write tracked");
        git(&["add", "tracked.txt"], &root);
        git(&["commit", "-m", "init", "--quiet"], &root);
        fs::write(root.join("tracked.txt"), "hello\nworld\n").expect("modify tracked");
        let session_path = root.join("session.json");
        Session::new()
            .save_to_path(&session_path)
            .expect("session should save");

        let session = Session::load_from_path(&session_path).expect("session should load");
        let outcome = with_current_dir(&root, || {
            run_resume_command(&session_path, &session, &SlashCommand::Diff)
                .expect("resume diff should work")
        });
        let message = outcome.message.expect("diff message should exist");
        assert!(message.contains("Unstaged changes:"));
        assert!(message.contains("tracked.txt"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn status_context_reads_real_workspace_metadata() {
        let context = status_context(None).expect("status context should load");
        assert!(context.cwd.is_absolute());
        assert!(context.discovered_config_files >= context.loaded_config_files);
        assert!(context.loaded_config_files <= context.discovered_config_files);
    }

    #[test]
    fn normalizes_supported_permission_modes() {
        assert_eq!(normalize_permission_mode("read-only"), Some("read-only"));
        assert_eq!(
            normalize_permission_mode("workspace-write"),
            Some("workspace-write")
        );
        assert_eq!(
            normalize_permission_mode("danger-full-access"),
            Some("danger-full-access")
        );
        assert_eq!(normalize_permission_mode("unknown"), None);
    }

    #[test]
    fn clear_command_requires_explicit_confirmation_flag() {
        assert_eq!(
            SlashCommand::parse("/clear"),
            Ok(Some(SlashCommand::Clear { confirm: false }))
        );
        assert_eq!(
            SlashCommand::parse("/clear --confirm"),
            Ok(Some(SlashCommand::Clear { confirm: true }))
        );
    }

    #[test]
    fn parses_resume_and_config_slash_commands() {
        assert_eq!(
            SlashCommand::parse("/resume saved-session.jsonl"),
            Ok(Some(SlashCommand::Resume {
                session_path: Some("saved-session.jsonl".to_string())
            }))
        );
        assert_eq!(
            SlashCommand::parse("/clear --confirm"),
            Ok(Some(SlashCommand::Clear { confirm: true }))
        );
        assert_eq!(
            SlashCommand::parse("/config"),
            Ok(Some(SlashCommand::Config { section: None }))
        );
        assert_eq!(
            SlashCommand::parse("/config env"),
            Ok(Some(SlashCommand::Config {
                section: Some("env".to_string())
            }))
        );
        assert_eq!(
            SlashCommand::parse("/memory"),
            Ok(Some(SlashCommand::Memory))
        );
        assert_eq!(SlashCommand::parse("/init"), Ok(Some(SlashCommand::Init)));
        assert_eq!(
            SlashCommand::parse("/session fork incident-review"),
            Ok(Some(SlashCommand::Session {
                action: Some("fork".to_string()),
                target: Some("incident-review".to_string())
            }))
        );
    }

    #[test]
    fn help_mentions_jsonl_resume_examples() {
        let mut help = Vec::new();
        print_help_to(&mut help).expect("help should render");
        let help = String::from_utf8(help).expect("help should be utf8");
        assert!(help.contains("claw --resume [SESSION.jsonl|session-id|latest]"));
        assert!(help.contains("Use `latest` with --resume, /resume, or /session switch"));
        assert!(help.contains("claw --resume latest"));
        assert!(help.contains("claw --resume latest /status /diff /export notes.txt"));
    }

    #[test]
    fn managed_sessions_default_to_jsonl_and_resolve_legacy_json() {
        let _guard = cwd_guard();
        let workspace = temp_workspace("session-resolution");
        std::fs::create_dir_all(&workspace).expect("workspace should create");
        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace).expect("switch cwd");

        let handle = create_managed_session_handle("session-alpha").expect("jsonl handle");
        assert!(handle.path.ends_with("session-alpha.jsonl"));

        let legacy_path = workspace.join(".claw/sessions/legacy.json");
        std::fs::create_dir_all(
            legacy_path
                .parent()
                .expect("legacy path should have parent directory"),
        )
        .expect("session dir should exist");
        Session::new()
            .with_workspace_root(workspace.clone())
            .with_persistence_path(legacy_path.clone())
            .save_to_path(&legacy_path)
            .expect("legacy session should save");

        let resolved = resolve_session_reference("legacy").expect("legacy session should resolve");
        assert_eq!(
            resolved
                .path
                .canonicalize()
                .expect("resolved path should exist"),
            legacy_path
                .canonicalize()
                .expect("legacy path should exist")
        );

        std::env::set_current_dir(previous).expect("restore cwd");
        std::fs::remove_dir_all(workspace).expect("workspace should clean up");
    }

    #[test]
    fn resumed_session_exists_and_delete_have_json_contracts() {
        let _guard = cwd_guard();
        let workspace = temp_workspace("resume-session-json-contracts");
        std::fs::create_dir_all(&workspace).expect("workspace should create");
        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace).expect("switch cwd");

        let active = create_managed_session_handle("session-active").expect("active handle");
        let active_session = Session::new()
            .with_workspace_root(workspace.clone())
            .with_persistence_path(active.path.clone());
        active_session
            .save_to_path(&active.path)
            .expect("active session should save");
        let saved = create_managed_session_handle("session-saved").expect("saved handle");
        Session::new()
            .with_workspace_root(workspace.clone())
            .with_persistence_path(saved.path.clone())
            .save_to_path(&saved.path)
            .expect("saved session should save");

        let exists_command = SlashCommand::parse("/session exists session-saved")
            .expect("parse should succeed")
            .expect("command should exist");
        let exists = run_resume_command(&active.path, &active_session, &exists_command)
            .expect("exists should run")
            .json
            .expect("exists should return json");
        assert_eq!(exists["kind"], "session_exists");
        assert_eq!(exists["session_id"], "session-saved");
        assert_eq!(exists["exists"], true);
        assert_eq!(exists["active"], false);
        assert!(exists["path"].as_str().is_some());

        let missing_command = SlashCommand::parse("/session exists missing-session")
            .expect("parse should succeed")
            .expect("command should exist");
        let missing = run_resume_command(&active.path, &active_session, &missing_command)
            .expect("missing exists should run")
            .json
            .expect("missing exists should return json");
        assert_eq!(missing["kind"], "session_exists");
        assert_eq!(missing["exists"], false);
        assert_eq!(missing["session_id"], "missing-session");
        assert!(missing["candidate_path"].as_str().is_some());

        let list_command = SlashCommand::parse("/session list")
            .expect("parse should succeed")
            .expect("command should exist");
        let list = run_resume_command(&active.path, &active_session, &list_command)
            .expect("list should run")
            .json
            .expect("list should return json");
        assert_eq!(list["kind"], "sessions");
        let details = list["session_details"]
            .as_array()
            .expect("session_details should be an array");
        let saved_path = saved.path.display().to_string();
        let saved_detail = details
            .iter()
            .find(|detail| detail["path"] == saved_path)
            .expect("saved session detail should exist");
        let created_at_ms = saved_detail["created_at_ms"]
            .as_u64()
            .expect("created_at_ms should be present");
        let updated_at_ms = saved_detail["updated_at_ms"]
            .as_u64()
            .expect("updated_at_ms should be present");
        assert!(
            created_at_ms <= updated_at_ms,
            "created_at_ms should not be after updated_at_ms"
        );

        let delete_command = SlashCommand::parse("/session delete session-saved --force")
            .expect("parse should succeed")
            .expect("command should exist");
        let deleted = run_resume_command(&active.path, &active_session, &delete_command)
            .expect("delete should run")
            .json
            .expect("delete should return json");
        assert_eq!(deleted["kind"], "session_delete");
        assert_eq!(deleted["deleted"], true);
        assert!(!saved.path.exists(), "saved session should be deleted");

        std::env::set_current_dir(previous).expect("restore cwd");
        std::fs::remove_dir_all(workspace).expect("workspace should clean up");
    }

    #[test]
    fn latest_session_alias_resolves_most_recent_managed_session() {
        let _guard = cwd_guard();
        let workspace = temp_workspace("latest-session-alias");
        std::fs::create_dir_all(&workspace).expect("workspace should create");
        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace).expect("switch cwd");

        let older = create_managed_session_handle("session-older").expect("older handle");
        {
            let mut session = Session::new().with_persistence_path(older.path.clone());
            session
                .push_user_text("older session message")
                .expect("older message should save");
            session
                .save_to_path(&older.path)
                .expect("older session should save");
        }
        std::thread::sleep(Duration::from_millis(20));
        let newer = create_managed_session_handle("session-newer").expect("newer handle");
        {
            let mut session = Session::new().with_persistence_path(newer.path.clone());
            session
                .push_user_text("newer session message")
                .expect("newer message should save");
            session
                .save_to_path(&newer.path)
                .expect("newer session should save");
        }

        let resolved = resolve_session_reference("latest").expect("latest session should resolve");
        assert_eq!(
            resolved
                .path
                .canonicalize()
                .expect("resolved path should exist"),
            newer.path.canonicalize().expect("newer path should exist")
        );

        std::env::set_current_dir(previous).expect("restore cwd");
        std::fs::remove_dir_all(workspace).expect("workspace should clean up");
    }

    #[test]
    fn load_session_reference_rejects_workspace_mismatch() {
        let _guard = cwd_guard();
        let workspace_a = temp_workspace("session-mismatch-a");
        let workspace_b = temp_workspace("session-mismatch-b");
        std::fs::create_dir_all(&workspace_a).expect("workspace a should create");
        std::fs::create_dir_all(&workspace_b).expect("workspace b should create");
        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace_b).expect("switch cwd");

        let session_path = workspace_a.join(".claw/sessions/legacy-cross.jsonl");
        std::fs::create_dir_all(
            session_path
                .parent()
                .expect("session path should have parent directory"),
        )
        .expect("session dir should exist");
        Session::new()
            .with_workspace_root(workspace_a.clone())
            .with_persistence_path(session_path.clone())
            .save_to_path(&session_path)
            .expect("session should save");

        let error = crate::load_session_reference(&session_path.display().to_string())
            .expect_err("mismatched workspace should fail");
        assert!(
            error.to_string().contains("session workspace mismatch"),
            "unexpected error: {error}"
        );
        assert!(
            error
                .to_string()
                .contains(&workspace_b.display().to_string()),
            "expected current workspace in error: {error}"
        );
        assert!(
            error
                .to_string()
                .contains(&workspace_a.display().to_string()),
            "expected originating workspace in error: {error}"
        );

        std::env::set_current_dir(previous).expect("restore cwd");
        std::fs::remove_dir_all(workspace_a).expect("workspace a should clean up");
        std::fs::remove_dir_all(workspace_b).expect("workspace b should clean up");
    }

    #[test]
    fn unknown_slash_command_guidance_suggests_nearby_commands() {
        let message = format_unknown_slash_command("stats");
        assert!(message.contains("Unknown slash command: /stats"));
        assert!(message.contains("/status"));
        assert!(message.contains("/help"));
    }

    #[test]
    fn unknown_omc_slash_command_guidance_explains_runtime_gap() {
        let message = format_unknown_slash_command("oh-my-claudecode:hud");
        assert!(message.contains("Unknown slash command: /oh-my-claudecode:hud"));
        assert!(message.contains("Claude Code/OMC plugin command"));
        assert!(message.contains("does not yet load plugin slash commands"));
    }

    #[test]
    fn resume_usage_mentions_latest_shortcut() {
        let usage = render_resume_usage();
        assert!(usage.contains("/resume <session-path|session-id|latest>"));
        assert!(usage.contains(".claw/sessions/<workspace-fingerprint>/<session-id>.jsonl"));
        assert!(usage.contains("/session list"));
    }

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn cwd_guard() -> MutexGuard<'static, ()> {
        cwd_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn cwd_guard_recovers_after_poisoning() {
        let poisoned = std::thread::spawn(|| {
            let _guard = cwd_guard();
            panic!("poison cwd lock");
        })
        .join();
        assert!(poisoned.is_err(), "poisoning thread should panic");

        let _guard = cwd_guard();
    }

    fn temp_workspace(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("claw-cli-{label}-{nanos}"))
    }

    #[test]
    fn init_template_mentions_detected_rust_workspace() {
        let _guard = cwd_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let rendered = crate::init::render_init_claude_md(&workspace_root);
        assert!(rendered.contains("# CLAUDE.md"));
        assert!(rendered.contains("cargo clippy --workspace --all-targets -- -D warnings"));
    }

    #[test]
    fn converts_tool_roundtrip_messages() {
        let messages = vec![
            ConversationMessage::user_text("hello"),
            ConversationMessage::assistant(vec![ContentBlock::ToolUse {
                id: "tool-1".to_string(),
                name: "bash".to_string(),
                input: "{\"command\":\"pwd\"}".to_string(),
            }]),
            ConversationMessage {
                role: MessageRole::Tool,
                blocks: vec![ContentBlock::ToolResult {
                    tool_use_id: "tool-1".to_string(),
                    tool_name: "bash".to_string(),
                    output: "ok".to_string(),
                    is_error: false,
                }],
                usage: None,
            },
        ];

        let converted = super::convert_messages(&messages);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[1].role, "assistant");
        assert_eq!(converted[2].role, "user");
    }
    #[test]
    fn repl_help_mentions_history_completion_and_multiline() {
        let help = render_repl_help();
        assert!(help.contains("Up/Down"));
        assert!(help.contains("Tab"));
        assert!(help.contains("Shift+Enter/Ctrl+J"));
        assert!(help.contains("Ctrl-R"));
        assert!(help.contains("Reverse-search prompt history"));
        assert!(help.contains("/history [count]"));
    }

    #[test]
    fn parse_history_count_defaults_to_twenty_when_missing() {
        // given
        let raw: Option<&str> = None;

        // when
        let parsed = parse_history_count(raw);

        // then
        assert_eq!(parsed, Ok(20));
    }

    #[test]
    fn parse_history_count_accepts_positive_integers() {
        // given
        let raw = Some("25");

        // when
        let parsed = parse_history_count(raw);

        // then
        assert_eq!(parsed, Ok(25));
    }

    #[test]
    fn parse_history_count_rejects_zero() {
        // given
        let raw = Some("0");

        // when
        let parsed = parse_history_count(raw);

        // then
        assert!(parsed.is_err());
        assert!(parsed.unwrap_err().contains("greater than 0"));
    }

    #[test]
    fn parse_history_count_rejects_non_numeric() {
        // given
        let raw = Some("abc");

        // when
        let parsed = parse_history_count(raw);

        // then
        // #776: updated to match new invalid_history_count: prefix format
        let err = parsed.expect_err("non-numeric count should fail");
        assert!(err.contains("invalid_history_count:") && err.contains("'abc'"));
    }

    #[test]
    fn format_history_timestamp_renders_iso8601_utc() {
        // given
        // 2023-01-15T12:34:56.789Z -> 1673786096789 ms
        let timestamp_ms: u64 = 1_673_786_096_789;

        // when
        let formatted = format_history_timestamp(timestamp_ms);

        // then
        assert_eq!(formatted, "2023-01-15T12:34:56.789Z");
    }

    #[test]
    fn format_history_timestamp_renders_unix_epoch_origin() {
        // given
        let timestamp_ms: u64 = 0;

        // when
        let formatted = format_history_timestamp(timestamp_ms);

        // then
        assert_eq!(formatted, "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn render_prompt_history_report_lists_entries_with_timestamps() {
        // given
        let entries = vec![
            PromptHistoryEntry {
                timestamp_ms: 1_673_786_096_000,
                text: "first prompt".to_string(),
            },
            PromptHistoryEntry {
                timestamp_ms: 1_673_786_100_000,
                text: "second prompt".to_string(),
            },
        ];

        // when
        let rendered = render_prompt_history_report(&entries, 10);

        // then
        assert!(rendered.contains("Prompt history"));
        assert!(rendered.contains("Total            2"));
        assert!(rendered.contains("Showing          2 most recent"));
        assert!(rendered.contains("Reverse search   Ctrl-R in the REPL"));
        assert!(rendered.contains("2023-01-15T12:34:56.000Z"));
        assert!(rendered.contains("first prompt"));
        assert!(rendered.contains("second prompt"));
    }

    #[test]
    fn render_prompt_history_report_truncates_to_limit_from_the_tail() {
        // given
        let entries = vec![
            PromptHistoryEntry {
                timestamp_ms: 1_000,
                text: "older".to_string(),
            },
            PromptHistoryEntry {
                timestamp_ms: 2_000,
                text: "middle".to_string(),
            },
            PromptHistoryEntry {
                timestamp_ms: 3_000,
                text: "latest".to_string(),
            },
        ];

        // when
        let rendered = render_prompt_history_report(&entries, 2);

        // then
        assert!(rendered.contains("Total            3"));
        assert!(rendered.contains("Showing          2 most recent"));
        assert!(!rendered.contains("older"));
        assert!(rendered.contains("middle"));
        assert!(rendered.contains("latest"));
    }

    #[test]
    fn render_prompt_history_report_handles_empty_history() {
        // given
        let entries: Vec<PromptHistoryEntry> = Vec::new();

        // when
        let rendered = render_prompt_history_report(&entries, 10);

        // then
        assert!(rendered.contains("no prompts recorded yet"));
    }

    #[test]
    fn collect_session_prompt_history_extracts_user_text_blocks() {
        // given
        let mut session = Session::new();
        session.push_user_text("hello").unwrap();
        session.push_user_text("world").unwrap();

        // when
        let entries = collect_session_prompt_history(&session);

        // then
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "hello");
        assert_eq!(entries[1].text, "world");
    }

    #[test]
    fn tool_rendering_helpers_compact_output() {
        let start = format_tool_call_start("read_file", r#"{"path":"src/main.rs"}"#);
        assert!(start.contains("read_file"));
        assert!(start.contains("src/main.rs"));

        let done = format_tool_result(
            "read_file",
            r#"{"file":{"filePath":"src/main.rs","content":"hello","numLines":1,"startLine":1,"totalLines":1}}"#,
            false,
        );
        assert!(done.contains("📄 Read src/main.rs"));
        assert!(done.contains("hello"));
    }

    #[test]
    fn tool_rendering_truncates_large_read_output_for_display_only() {
        let content = (0..200)
            .map(|index| format!("line {index:03}"))
            .collect::<Vec<_>>()
            .join("\n");
        let output = json!({
            "file": {
                "filePath": "src/main.rs",
                "content": content,
                "numLines": 200,
                "startLine": 1,
                "totalLines": 200
            }
        })
        .to_string();

        let rendered = format_tool_result("read_file", &output, false);

        assert!(rendered.contains("line 000"));
        assert!(rendered.contains("line 079"));
        assert!(!rendered.contains("line 199"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("line 199"));
    }

    #[test]
    fn tool_rendering_truncates_large_bash_output_for_display_only() {
        let stdout = (0..120)
            .map(|index| format!("stdout {index:03}"))
            .collect::<Vec<_>>()
            .join("\n");
        let output = json!({
            "stdout": stdout,
            "stderr": "",
            "returnCodeInterpretation": "completed successfully"
        })
        .to_string();

        let rendered = format_tool_result("bash", &output, false);

        assert!(rendered.contains("stdout 000"));
        assert!(rendered.contains("stdout 059"));
        assert!(!rendered.contains("stdout 119"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("stdout 119"));
    }

    #[test]
    fn tool_rendering_truncates_generic_long_output_for_display_only() {
        let items = (0..120)
            .map(|index| format!("payload {index:03}"))
            .collect::<Vec<_>>();
        let output = json!({
            "summary": "plugin payload",
            "items": items,
        })
        .to_string();

        let rendered = format_tool_result("plugin_echo", &output, false);

        assert!(rendered.contains("plugin_echo"));
        assert!(rendered.contains("payload 000"));
        assert!(rendered.contains("payload 040"));
        assert!(!rendered.contains("payload 080"));
        assert!(!rendered.contains("payload 119"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("payload 119"));
    }

    #[test]
    fn tool_rendering_truncates_raw_generic_output_for_display_only() {
        let output = (0..120)
            .map(|index| format!("raw {index:03}"))
            .collect::<Vec<_>>()
            .join("\n");

        let rendered = format_tool_result("plugin_echo", &output, false);

        assert!(rendered.contains("plugin_echo"));
        assert!(rendered.contains("raw 000"));
        assert!(rendered.contains("raw 059"));
        assert!(!rendered.contains("raw 119"));
        assert!(rendered.contains("full result preserved in session"));
        assert!(output.contains("raw 119"));
    }

    #[test]
    fn ultraplan_progress_lines_include_phase_step_and_elapsed_status() {
        let snapshot = InternalPromptProgressState {
            command_label: "Ultraplan",
            task_label: "ship plugin progress".to_string(),
            step: 3,
            phase: "running read_file".to_string(),
            detail: Some("reading rust/crates/rusty-claude-cli/src/main.rs".to_string()),
            saw_final_text: false,
        };

        let started = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Started,
            &snapshot,
            Duration::from_secs(0),
            None,
        );
        let heartbeat = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Heartbeat,
            &snapshot,
            Duration::from_secs(9),
            None,
        );
        let completed = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Complete,
            &snapshot,
            Duration::from_secs(12),
            None,
        );
        let failed = format_internal_prompt_progress_line(
            InternalPromptProgressEvent::Failed,
            &snapshot,
            Duration::from_secs(12),
            Some("network timeout"),
        );

        assert!(started.contains("planning started"));
        assert!(started.contains("current step 3"));
        assert!(heartbeat.contains("heartbeat"));
        assert!(heartbeat.contains("9s elapsed"));
        assert!(heartbeat.contains("phase running read_file"));
        assert!(completed.contains("completed"));
        assert!(completed.contains("3 steps total"));
        assert!(failed.contains("failed"));
        assert!(failed.contains("network timeout"));
    }

    #[test]
    fn describe_tool_progress_summarizes_known_tools() {
        assert_eq!(
            describe_tool_progress("read_file", r#"{"path":"src/main.rs"}"#),
            "reading src/main.rs"
        );
        assert!(
            describe_tool_progress("bash", r#"{"command":"cargo test -p rusty-claude-cli"}"#)
                .contains("cargo test -p rusty-claude-cli")
        );
        assert_eq!(
            describe_tool_progress("grep_search", r#"{"pattern":"ultraplan","path":"rust"}"#),
            "grep `ultraplan` in rust"
        );
    }

    #[test]
    fn push_output_block_renders_markdown_text() {
        let mut out = Vec::new();
        let mut events = Vec::new();
        let mut pending_tool = None;
        let mut block_has_thinking_summary = false;

        push_output_block(
            OutputContentBlock::Text {
                text: "# Heading".to_string(),
            },
            &mut out,
            &mut events,
            &mut pending_tool,
            false,
            &mut block_has_thinking_summary,
        )
        .expect("text block should render");

        let rendered = String::from_utf8(out).expect("utf8");
        assert!(rendered.contains("Heading"));
        assert!(rendered.contains('\u{1b}'));
    }

    #[test]
    fn push_output_block_skips_empty_object_prefix_for_tool_streams() {
        let mut out = Vec::new();
        let mut events = Vec::new();
        let mut pending_tool = None;
        let mut block_has_thinking_summary = false;

        push_output_block(
            OutputContentBlock::ToolUse {
                id: "tool-1".to_string(),
                name: "read_file".to_string(),
                input: json!({}),
            },
            &mut out,
            &mut events,
            &mut pending_tool,
            true,
            &mut block_has_thinking_summary,
        )
        .expect("tool block should accumulate");

        assert!(events.is_empty());
        assert_eq!(
            pending_tool,
            Some(("tool-1".to_string(), "read_file".to_string(), String::new(),))
        );
    }

    #[test]
    fn response_to_events_preserves_empty_object_json_input_outside_streaming() {
        let mut out = Vec::new();
        let events = response_to_events(
            MessageResponse {
                id: "msg-1".to_string(),
                kind: "message".to_string(),
                model: "anthropic/claude-opus-4-6".to_string(),
                role: "assistant".to_string(),
                content: vec![OutputContentBlock::ToolUse {
                    id: "tool-1".to_string(),
                    name: "read_file".to_string(),
                    input: json!({}),
                }],
                stop_reason: Some("tool_use".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                request_id: None,
            },
            &mut out,
        )
        .expect("response conversion should succeed");

        assert!(matches!(
            &events[0],
            AssistantEvent::ToolUse { name, input, .. }
                if name == "read_file" && input == "{}"
        ));
    }

    #[test]
    fn response_to_events_preserves_non_empty_json_input_outside_streaming() {
        let mut out = Vec::new();
        let events = response_to_events(
            MessageResponse {
                id: "msg-2".to_string(),
                kind: "message".to_string(),
                model: "anthropic/claude-opus-4-6".to_string(),
                role: "assistant".to_string(),
                content: vec![OutputContentBlock::ToolUse {
                    id: "tool-2".to_string(),
                    name: "read_file".to_string(),
                    input: json!({ "path": "rust/Cargo.toml" }),
                }],
                stop_reason: Some("tool_use".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                request_id: None,
            },
            &mut out,
        )
        .expect("response conversion should succeed");

        assert!(matches!(
            &events[0],
            AssistantEvent::ToolUse { name, input, .. }
                if name == "read_file" && input == "{\"path\":\"rust/Cargo.toml\"}"
        ));
    }

    #[test]
    fn response_to_events_renders_collapsed_thinking_summary() {
        let mut out = Vec::new();
        let events = response_to_events(
            MessageResponse {
                id: "msg-3".to_string(),
                kind: "message".to_string(),
                model: "anthropic/claude-opus-4-6".to_string(),
                role: "assistant".to_string(),
                content: vec![
                    OutputContentBlock::Thinking {
                        thinking: "step 1".to_string(),
                        signature: Some("sig_123".to_string()),
                    },
                    OutputContentBlock::Text {
                        text: "Final answer".to_string(),
                    },
                ],
                stop_reason: Some("end_turn".to_string()),
                stop_sequence: None,
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
                request_id: None,
            },
            &mut out,
        )
        .expect("response conversion should succeed");

        assert!(matches!(
            &events[0],
            AssistantEvent::Thinking {
                thinking,
                signature
            } if thinking == "step 1" && signature.as_deref() == Some("sig_123")
        ));
        assert!(matches!(
            &events[1],
            AssistantEvent::TextDelta(text) if text == "Final answer"
        ));
        let rendered = String::from_utf8(out).expect("utf8");
        assert!(rendered.contains("▶ Thinking (6 chars hidden)"));
        assert!(!rendered.contains("step 1"));
    }

    #[test]
    fn build_runtime_plugin_state_merges_plugin_hooks_into_runtime_features() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        let source_root = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&source_root).expect("source root");
        write_plugin_fixture(&source_root, "hook-runtime-demo", true, false);

        let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
        manager
            .install(source_root.to_str().expect("utf8 source path"))
            .expect("plugin install should succeed");
        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let state = build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config)
            .expect("plugin state should load");
        let pre_hooks = state.feature_config.hooks().pre_tool_use();
        assert_eq!(pre_hooks.len(), 1);
        assert!(
            pre_hooks[0].ends_with("hooks/pre.sh"),
            "expected installed plugin hook path, got {pre_hooks:?}"
        );

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(source_root);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn build_runtime_plugin_state_discovers_mcp_tools_and_surfaces_pending_servers() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        let script_path = workspace.join("fixture-mcp.py");
        write_mcp_server_fixture(&script_path);
        fs::write(
            config_home.join("settings.json"),
            format!(
                r#"{{
                  "mcpServers": {{
                    "alpha": {{
                      "command": "python3",
                      "args": ["{}"]
                    }},
                    "broken": {{
                      "command": "python3",
                      "args": ["-c", "import sys; sys.exit(0)"]
                    }}
                  }}
                }}"#,
                script_path.to_string_lossy()
            ),
        )
        .expect("write mcp settings");

        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let state = build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config)
            .expect("runtime plugin state should load");

        let allowed = state
            .tool_registry
            .normalize_allowed_tools(&["mcp__alpha__echo".to_string(), "MCPTool".to_string()])
            .expect("mcp tools should be allow-listable")
            .expect("allow-list should exist");
        assert!(allowed.contains("mcp__alpha__echo"));
        assert!(allowed.contains("mcp_tool"));

        let mut executor = CliToolExecutor::new(
            None,
            false,
            state.tool_registry.clone(),
            state.mcp_state.clone(),
        );

        let tool_output = executor
            .execute("mcp__alpha__echo", r#"{"text":"hello"}"#)
            .expect("discovered mcp tool should execute");
        let tool_json: serde_json::Value =
            serde_json::from_str(&tool_output).expect("tool output should be json");
        assert_eq!(tool_json["structuredContent"]["echoed"], "hello");

        let wrapped_output = executor
            .execute(
                "MCPTool",
                r#"{"qualifiedName":"mcp__alpha__echo","arguments":{"text":"wrapped"}}"#,
            )
            .expect("generic mcp wrapper should execute");
        let wrapped_json: serde_json::Value =
            serde_json::from_str(&wrapped_output).expect("wrapped output should be json");
        assert_eq!(wrapped_json["structuredContent"]["echoed"], "wrapped");

        let search_output = executor
            .execute("ToolSearch", r#"{"query":"alpha echo","max_results":5}"#)
            .expect("tool search should execute");
        let search_json: serde_json::Value =
            serde_json::from_str(&search_output).expect("search output should be json");
        assert_eq!(search_json["matches"][0], "mcp__alpha__echo");
        assert_eq!(search_json["pending_mcp_servers"][0], "broken");
        assert_eq!(
            search_json["mcp_degraded"]["failed_servers"][0]["server_name"],
            "broken"
        );
        assert_eq!(
            search_json["mcp_degraded"]["failed_servers"][0]["phase"],
            "tool_discovery"
        );
        assert_eq!(
            search_json["mcp_degraded"]["available_tools"][0],
            "mcp__alpha__echo"
        );

        let listed = executor
            .execute("ListMcpResourcesTool", r#"{"server":"alpha"}"#)
            .expect("resources should list");
        let listed_json: serde_json::Value =
            serde_json::from_str(&listed).expect("resource output should be json");
        assert_eq!(listed_json["resources"][0]["uri"], "file://guide.txt");

        let read = executor
            .execute(
                "ReadMcpResourceTool",
                r#"{"server":"alpha","uri":"file://guide.txt"}"#,
            )
            .expect("resource should read");
        let read_json: serde_json::Value =
            serde_json::from_str(&read).expect("resource read output should be json");
        assert_eq!(
            read_json["contents"][0]["text"],
            "contents for file://guide.txt"
        );

        if let Some(mcp_state) = state.mcp_state {
            mcp_state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .shutdown()
                .expect("mcp shutdown should succeed");
        }

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn build_runtime_plugin_state_surfaces_unsupported_mcp_servers_structurally() {
        let config_home = temp_dir();
        let workspace = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::write(
            config_home.join("settings.json"),
            r#"{
              "mcpServers": {
                "remote": {
                  "url": "https://example.test/mcp"
                }
              }
            }"#,
        )
        .expect("write mcp settings");

        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let state = build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config)
            .expect("runtime plugin state should load");
        let mut executor = CliToolExecutor::new(
            None,
            false,
            state.tool_registry.clone(),
            state.mcp_state.clone(),
        );

        let search_output = executor
            .execute("ToolSearch", r#"{"query":"remote","max_results":5}"#)
            .expect("tool search should execute");
        let search_json: serde_json::Value =
            serde_json::from_str(&search_output).expect("search output should be json");
        assert_eq!(search_json["pending_mcp_servers"][0], "remote");
        assert_eq!(
            search_json["mcp_degraded"]["failed_servers"][0]["server_name"],
            "remote"
        );
        assert_eq!(
            search_json["mcp_degraded"]["failed_servers"][0]["phase"],
            "server_registration"
        );
        assert_eq!(
            search_json["mcp_degraded"]["failed_servers"][0]["error"]["context"]["transport"],
            "http"
        );

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn build_runtime_runs_plugin_lifecycle_init_and_shutdown() {
        // Serialize access to process-wide env vars so parallel tests that
        // set/remove ANTHROPIC_API_KEY do not race with this test.
        let _guard = env_lock();
        let config_home = temp_dir();
        // Inject a dummy API key so runtime construction succeeds without real credentials.
        // This test only exercises plugin lifecycle (init/shutdown), never calls the API.
        std::env::set_var("ANTHROPIC_API_KEY", "test-dummy-key-for-plugin-lifecycle");
        let workspace = temp_dir();
        let source_root = temp_dir();
        fs::create_dir_all(&config_home).expect("config home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&source_root).expect("source root");
        write_plugin_fixture(&source_root, "lifecycle-runtime-demo", false, true);

        let mut manager = PluginManager::new(PluginManagerConfig::new(&config_home));
        let install = manager
            .install(source_root.to_str().expect("utf8 source path"))
            .expect("plugin install should succeed");
        let log_path = install.install_path.join("lifecycle.log");
        let loader = ConfigLoader::new(&workspace, &config_home);
        let runtime_config = loader.load().expect("runtime config should load");
        let runtime_plugin_state =
            build_runtime_plugin_state_with_loader(&workspace, &loader, &runtime_config)
                .expect("plugin state should load");
        let mut runtime = build_runtime_with_plugin_state(
            Session::new(),
            "runtime-plugin-lifecycle",
            DEFAULT_MODEL.to_string(),
            vec!["test system prompt".to_string()],
            true,
            false,
            None,
            PermissionMode::DangerFullAccess,
            None,
            runtime_plugin_state,
        )
        .expect("runtime should build");

        assert_eq!(
            fs::read_to_string(&log_path).expect("init log should exist"),
            "init\n"
        );

        runtime
            .shutdown_plugins()
            .expect("plugin shutdown should succeed");

        assert_eq!(
            fs::read_to_string(&log_path).expect("shutdown log should exist"),
            "init\nshutdown\n"
        );

        let _ = fs::remove_dir_all(config_home);
        let _ = fs::remove_dir_all(workspace);
        let _ = fs::remove_dir_all(source_root);
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn rejects_invalid_reasoning_effort_value() {
        let err = parse_args(&[
            "--reasoning-effort".to_string(),
            "turbo".to_string(),
            "prompt".to_string(),
            "hello".to_string(),
        ])
        .unwrap_err();
        assert!(
            err.contains("invalid value for --reasoning-effort"),
            "unexpected error: {err}"
        );
        assert!(err.contains("turbo"), "unexpected error: {err}");
    }

    #[test]
    fn accepts_valid_reasoning_effort_values() {
        for value in ["low", "medium", "high"] {
            let result = parse_args(&[
                "--reasoning-effort".to_string(),
                value.to_string(),
                "prompt".to_string(),
                "hello".to_string(),
            ]);
            assert!(
                result.is_ok(),
                "--reasoning-effort {value} should be accepted, got: {result:?}"
            );
            if let Ok(CliAction::Prompt {
                reasoning_effort, ..
            }) = result
            {
                assert_eq!(reasoning_effort.as_deref(), Some(value));
            }
        }
    }

    #[test]
    fn stub_commands_absent_from_repl_completions() {
        let candidates =
            slash_command_completion_candidates_with_sessions("claude-3-5-sonnet", None, vec![]);
        for stub in STUB_COMMANDS {
            let with_slash = format!("/{stub}");
            assert!(
                !candidates.contains(&with_slash),
                "stub command {with_slash} should not appear in REPL completions"
            );
        }
    }

    #[test]
    fn stub_commands_absent_from_resume_safe_help() {
        let mut help = Vec::new();
        print_help_to(&mut help).expect("help should render");
        let help = String::from_utf8(help).expect("help should be utf8");
        let resume_line = help
            .lines()
            .find(|line| line.starts_with("Resume-safe commands:"))
            .expect("resume-safe command line should exist");
        let resume_roots = resume_line
            .trim_start_matches("Resume-safe commands:")
            .split(',')
            .filter_map(|entry| entry.trim().strip_prefix('/'))
            .filter_map(|entry| entry.split_whitespace().next())
            .collect::<Vec<_>>();

        for stub in STUB_COMMANDS {
            assert!(
                !resume_roots.contains(stub),
                "stub command /{stub} should not appear in resume-safe command list"
            );
        }

        assert!(resume_roots.contains(&"status"));
    }
}

fn write_mcp_server_fixture(script_path: &Path) {
    let script = [
            "#!/usr/bin/env python3",
            "import json, sys",
            "",
            "def read_message():",
            "    header = b''",
            r"    while not header.endswith(b'\r\n\r\n'):",
            "        chunk = sys.stdin.buffer.read(1)",
            "        if not chunk:",
            "            return None",
            "        header += chunk",
            "    length = 0",
            r"    for line in header.decode().split('\r\n'):",
            r"        if line.lower().startswith('content-length:'):",
            "            length = int(line.split(':', 1)[1].strip())",
            "    payload = sys.stdin.buffer.read(length)",
            "    return json.loads(payload.decode())",
            "",
            "def send_message(message):",
            "    payload = json.dumps(message).encode()",
            r"    sys.stdout.buffer.write(f'Content-Length: {len(payload)}\r\n\r\n'.encode() + payload)",
            "    sys.stdout.buffer.flush()",
            "",
            "while True:",
            "    request = read_message()",
            "    if request is None:",
            "        break",
            "    method = request['method']",
            "    if method == 'initialize':",
            "        send_message({",
            "            'jsonrpc': '2.0',",
            "            'id': request['id'],",
            "            'result': {",
            "                'protocolVersion': request['params']['protocolVersion'],",
            "                'capabilities': {'tools': {}, 'resources': {}},",
            "                'serverInfo': {'name': 'fixture', 'version': '1.0.0'}",
            "            }",
            "        })",
            "    elif method == 'tools/list':",
            "        send_message({",
            "            'jsonrpc': '2.0',",
            "            'id': request['id'],",
            "            'result': {",
            "                'tools': [",
            "                    {",
            "                        'name': 'echo',",
            "                        'description': 'Echo from MCP fixture',",
            "                        'inputSchema': {",
            "                            'type': 'object',",
            "                            'properties': {'text': {'type': 'string'}},",
            "                            'required': ['text'],",
            "                            'additionalProperties': False",
            "                        },",
            "                        'annotations': {'readOnlyHint': True}",
            "                    }",
            "                ]",
            "            }",
            "        })",
            "    elif method == 'tools/call':",
            "        args = request['params'].get('arguments') or {}",
            "        send_message({",
            "            'jsonrpc': '2.0',",
            "            'id': request['id'],",
            "            'result': {",
            "                'content': [{'type': 'text', 'text': f\"echo:{args.get('text', '')}\"}],",
            "                'structuredContent': {'echoed': args.get('text', '')},",
            "                'isError': False",
            "            }",
            "        })",
            "    elif method == 'resources/list':",
            "        send_message({",
            "            'jsonrpc': '2.0',",
            "            'id': request['id'],",
            "            'result': {",
            "                'resources': [{'uri': 'file://guide.txt', 'name': 'guide', 'mimeType': 'text/plain'}]",
            "            }",
            "        })",
            "    elif method == 'resources/read':",
            "        uri = request['params']['uri']",
            "        send_message({",
            "            'jsonrpc': '2.0',",
            "            'id': request['id'],",
            "            'result': {",
            "                'contents': [{'uri': uri, 'mimeType': 'text/plain', 'text': f'contents for {uri}'}]",
            "            }",
            "        })",
            "    else:",
            "        send_message({",
            "            'jsonrpc': '2.0',",
            "            'id': request['id'],",
            "            'error': {'code': -32601, 'message': method}",
            "        })",
            "",
        ]
        .join("\n");
    fs::write(script_path, script).expect("mcp fixture script should write");
}

#[cfg(test)]
mod sandbox_report_tests {
    use super::{format_sandbox_report, HookAbortMonitor};
    use runtime::HookAbortSignal;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn sandbox_report_renders_expected_fields() {
        let report = format_sandbox_report(&runtime::SandboxStatus::default());
        assert!(report.contains("Sandbox"));
        assert!(report.contains("Enabled"));
        assert!(report.contains("Filesystem mode"));
        assert!(report.contains("Fallback reason"));
    }

    #[test]
    fn hook_abort_monitor_stops_without_aborting() {
        let abort_signal = HookAbortSignal::new();
        let (ready_tx, ready_rx) = mpsc::channel();
        let monitor = HookAbortMonitor::spawn_with_waiter(
            abort_signal.clone(),
            move |stop_rx, abort_signal| {
                ready_tx.send(()).expect("ready signal");
                let _ = stop_rx.recv();
                assert!(!abort_signal.is_aborted());
            },
        );

        ready_rx.recv().expect("waiter should be ready");
        monitor.stop();

        assert!(!abort_signal.is_aborted());
    }

    #[test]
    fn hook_abort_monitor_propagates_interrupt() {
        let abort_signal = HookAbortSignal::new();
        let (done_tx, done_rx) = mpsc::channel();
        let monitor = HookAbortMonitor::spawn_with_waiter(
            abort_signal.clone(),
            move |_stop_rx, abort_signal| {
                abort_signal.abort();
                done_tx.send(()).expect("done signal");
            },
        );

        done_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("interrupt should complete");
        monitor.stop();

        assert!(abort_signal.is_aborted());
    }
}

#[cfg(test)]
mod dump_manifests_tests {
    use super::{build_rust_resolver_manifest, dump_manifests_at_path, CliOutputFormat};
    use std::fs;

    #[test]
    fn dump_manifests_defaults_to_rust_resolver_inventory() {
        let root =
            std::env::temp_dir().join(format!("claw_test_rust_manifests_{}", std::process::id()));
        let workspace = root.join("workspace");
        fs::create_dir_all(&workspace).expect("workspace should exist");

        let manifest = build_rust_resolver_manifest(&workspace).expect("manifest should build");
        assert_eq!(manifest["kind"], "dump-manifests");
        assert_eq!(manifest["source"], "rust-resolver");
        assert!(manifest["commands"].as_u64().expect("commands count") > 0);
        assert!(manifest["tools"].as_u64().expect("tools count") > 0);
        assert!(manifest["command_manifests"]
            .as_array()
            .expect("command manifests")
            .iter()
            .any(|entry| entry["name"] == "status"));
        assert!(manifest["tool_manifests"]
            .as_array()
            .expect("tool manifests")
            .iter()
            .any(|entry| entry["name"] == "read_file"));
        assert!(dump_manifests_at_path(&workspace, None, CliOutputFormat::Text).is_ok());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn dump_manifests_scopes_explicit_manifest_dir_without_upstream_ts() {
        let root = std::env::temp_dir().join(format!(
            "claw_test_explicit_manifest_dir_{}",
            std::process::id()
        ));
        let workspace = root.join("workspace");
        let manifest_dir = root.join("manifest-source");
        fs::create_dir_all(&workspace).expect("workspace should exist");
        fs::create_dir_all(&manifest_dir).expect("manifest dir should exist");

        let result = dump_manifests_at_path(&workspace, Some(&manifest_dir), CliOutputFormat::Text);
        assert!(
            result.is_ok(),
            "explicit manifest dir should not require upstream TS files: {result:?}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn dump_manifests_missing_explicit_dir_has_typed_kind() {
        let root = std::env::temp_dir().join(format!(
            "claw_test_missing_manifest_dir_{}",
            std::process::id()
        ));
        let workspace = root.join("workspace");
        let missing = root.join("missing");
        fs::create_dir_all(&workspace).expect("workspace should exist");

        let result = dump_manifests_at_path(&workspace, Some(&missing), CliOutputFormat::Text);
        let error = result.expect_err("missing explicit manifest dir should fail");
        let error_msg = error.to_string();
        assert!(error_msg.starts_with("missing_manifests:"));
        assert!(error_msg.contains(&missing.display().to_string()));
        assert!(!error_msg.contains("CLAUDE_CODE_UPSTREAM"));
        assert!(!error_msg.contains("src/commands.ts"));

        let _ = fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod alias_resolution_tests {
    fn ollama_env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .expect("ollama env lock poisoned")
    }

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn unset(key: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, previous }
        }

        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    use super::{resolve_model_alias_with_config, validate_model_syntax};

    #[test]
    fn test_alias_resolution_builtin() {
        // Built-in aliases should resolve to their full IDs
        assert_eq!(
            resolve_model_alias_with_config("opus"),
            "anthropic/claude-opus-4-7"
        );
        assert_eq!(
            resolve_model_alias_with_config("sonnet"),
            "anthropic/claude-sonnet-4-6"
        );
        assert_eq!(
            resolve_model_alias_with_config("haiku"),
            "anthropic/claude-haiku-4-5-20251213"
        );
    }

    #[test]
    fn test_alias_resolution_syntax_validation() {
        let _guard = ollama_env_lock();
        let _env = EnvVarGuard::unset("OLLAMA_HOST");
        // Resolved aliases should pass syntax validation
        let resolved = resolve_model_alias_with_config("opus");
        assert!(validate_model_syntax(&resolved).is_ok());

        // Raw aliases should FAIL syntax validation (this is why we resolve first!)
        assert!(validate_model_syntax("opus").is_err());
    }

    #[test]
    fn test_unknown_alias_fails_validation() {
        let _guard = ollama_env_lock();
        let _env = EnvVarGuard::unset("OLLAMA_HOST");
        // Unknown aliases resolve to themselves
        let resolved = resolve_model_alias_with_config("unknown-alias");
        assert_eq!(resolved, "unknown-alias");

        // And then fail validation with a helpful error
        let result = validate_model_syntax(&resolved);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid model syntax"));
    }

    #[test]
    fn qwen_invalid_model_hint_mentions_local_ollama_openai_base_url() {
        let _guard = ollama_env_lock();
        let _ollama_env = EnvVarGuard::unset("OLLAMA_HOST");
        let _openai_env = EnvVarGuard::unset("OPENAI_BASE_URL");
        let result = validate_model_syntax("qwen3:8b");

        let error = result.expect_err("Ollama tag without local base URL should fail");
        assert!(
            error.contains("Ollama"),
            "Qwen Ollama tag error should mention Ollama: {error}"
        );
        assert!(
            error.contains("OPENAI_BASE_URL"),
            "Qwen Ollama tag error should mention OPENAI_BASE_URL: {error}"
        );
        assert!(
            error.contains("http://127.0.0.1:11434/v1"),
            "Qwen Ollama tag error should show local Ollama OpenAI URL: {error}"
        );
    }

    #[test]
    fn test_direct_provider_model_passes() {
        // Direct provider/model strings should remain unchanged and pass
        let model = "openai/gpt-4o";
        assert_eq!(resolve_model_alias_with_config(model), model);
        assert!(validate_model_syntax(model).is_ok());
    }
    #[test]
    fn test_ollama_host_bypasses_model_validation() {
        let _guard = ollama_env_lock();
        let _env = EnvVarGuard::set("OLLAMA_HOST", "http://127.0.0.1:11434");
        // Ollama model names with colons pass
        assert!(validate_model_syntax("qwen3:8b").is_ok());
        assert!(validate_model_syntax("gemma4:e2b").is_ok());
        assert!(validate_model_syntax("qwen3.6:27b-nvfp4").is_ok());
        // Empty model still rejected
        assert!(validate_model_syntax("").is_err());
    }
}
