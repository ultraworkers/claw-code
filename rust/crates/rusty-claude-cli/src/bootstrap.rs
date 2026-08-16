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

pub(crate) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    // #824: suppress config deprecation prose warnings to stderr when JSON
    // output mode is active.  Scan the raw argv before parse_args so the
    // suppression is in place before any settings file is loaded.
    let json_mode = raw_args_request_json_output(&args);
    if json_mode {
        runtime::suppress_config_warnings_for_json_mode();
    }
    let (args, cwd) = split_global_cwd_args(&args)?;
    apply_global_cwd(cwd)?;
    match parse_args(&args)? {
        CliAction::DumpManifests {
            output_format,
            manifests_dir,
        } => dump_manifests(manifests_dir.as_deref(), output_format)?,
        CliAction::BootstrapPlan { output_format } => print_bootstrap_plan(output_format)?,
        CliAction::Agents {
            args,
            output_format,
        } => LiveCli::print_agents(args.as_deref(), output_format)?,
        CliAction::Mcp {
            args,
            output_format,
        } => LiveCli::print_mcp(args.as_deref(), output_format)?,
        CliAction::Skills {
            args,
            output_format,
        } => LiveCli::print_skills(args.as_deref(), output_format)?,
        CliAction::Plugins {
            action,
            target,
            output_format,
        } => LiveCli::print_plugins(action.as_deref(), target.as_deref(), output_format)?,
        CliAction::PrintSystemPrompt {
            cwd,
            date,
            model,
            output_format,
        } => print_system_prompt(cwd, date, &model, output_format)?,
        CliAction::Version { output_format } => print_version(output_format)?,
        CliAction::ResumeSession {
            session_path,
            commands,
            output_format,
            allow_broad_cwd,
        } => {
            enforce_broad_cwd_policy(allow_broad_cwd, output_format)?;
            resume_session(&session_path, &commands, output_format)
        }
        CliAction::Status {
            model,
            model_flag_raw,
            permission_mode,
            output_format,
            allowed_tools,
        } => print_status_snapshot(
            &model,
            model_flag_raw.as_deref(),
            permission_mode,
            output_format,
            allowed_tools.as_ref(),
        )?,
        CliAction::Sandbox { output_format } => print_sandbox_status_snapshot(output_format)?,
        CliAction::Prompt {
            prompt,
            model,
            output_format,
            allowed_tools,
            permission_mode,
            compact,
            base_commit,
            reasoning_effort,
            allow_broad_cwd,
        } => {
            enforce_broad_cwd_policy(allow_broad_cwd, output_format)?;
            run_stale_base_preflight(base_commit.as_deref());
            // Only consume piped stdin as prompt context when the permission
            // mode is fully unattended. In modes where the permission
            // prompter may invoke CliPermissionPrompter::decide(), stdin
            // must remain available for interactive approval; otherwise the
            // prompter's read_line() would hit EOF and deny every request.
            let stdin_context = if matches!(permission_mode, PermissionMode::DangerFullAccess) {
                read_piped_stdin()
            } else {
                None
            };
            let effective_prompt = merge_prompt_with_stdin(&prompt, stdin_context.as_deref());
            let resolved_model = resolve_repl_model(model)?;
            let mut cli = LiveCli::new(resolved_model, true, allowed_tools, permission_mode)?;
            cli.set_reasoning_effort(reasoning_effort);
            cli.run_turn_with_output(&effective_prompt, output_format, compact)?;
        }
        CliAction::Doctor {
            output_format,
            permission_mode,
        } => run_doctor(output_format, permission_mode)?,
        CliAction::Acp { output_format } => {
            print_acp_status(output_format)?;
            std::process::exit(2);
        }
        CliAction::SessionList { output_format } => run_session_list(output_format)?,
        CliAction::State { output_format } => run_worker_state(output_format)?,
        CliAction::Init { output_format } => run_init(output_format)?,
        CliAction::Setup { output_format: _ } => run_setup()?,
        // #146: dispatch pure-local introspection. Text mode uses existing
        // render_config_report/render_diff_report; JSON mode uses the
        // corresponding _json helpers already exposed for resume sessions.
        CliAction::Config {
            section,
            output_format,
        } => match output_format {
            CliOutputFormat::Text => {
                println!("{}", render_config_report(section.as_deref())?);
            }
            CliOutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render_config_json(section.as_deref())?)?
                );
            }
        },
        CliAction::Models {
            action,
            output_format,
        } => print_models(action.as_deref(), output_format)?,
        CliAction::Diff { output_format } => match output_format {
            CliOutputFormat::Text => {
                println!("{}", render_diff_report()?);
            }
            CliOutputFormat::Json => {
                let cwd = friendly_cwd(env::current_dir()?);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render_diff_json_for(&cwd)?)?
                );
            }
        },
        CliAction::Export {
            session_reference,
            output_path,
            output_format,
        } => run_export(&session_reference, output_path.as_deref(), output_format)?,
        CliAction::Repl {
            model,
            allowed_tools,
            permission_mode,
            base_commit,
            reasoning_effort,
            allow_broad_cwd,
        } => run_repl(
            model,
            allowed_tools,
            permission_mode,
            base_commit,
            reasoning_effort,
            allow_broad_cwd,
        )?,
        CliAction::HelpTopic {
            topic,
            output_format,
        } => print_help_topic(topic, output_format)?,
        CliAction::Help { output_format } => print_help(output_format)?,
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliOutputFormat {
    Text,
    Json,
}

impl CliOutputFormat {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value.trim() {
            value if value.eq_ignore_ascii_case("text") => Ok(Self::Text),
            value if value.eq_ignore_ascii_case("json") => Ok(Self::Json),
            other => Err(format!(
                "invalid_output_format: unsupported value for --output-format: {other}\nExpected: text, json\nHint: Use --output-format text or --output-format json."
            )),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
        }
    }
}

pub(crate) fn run_doctor(
    output_format: CliOutputFormat,
    permission_mode: PermissionModeProvenance,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = render_doctor_report(
        match output_format {
            CliOutputFormat::Json => ConfigWarningMode::SuppressStderr,
            CliOutputFormat::Text => ConfigWarningMode::EmitStderr,
        },
        permission_mode,
    )?;
    let message = report.render();
    match output_format {
        CliOutputFormat::Text => println!("{message}"),
        CliOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&report.json_value())?);
        }
    }
    if report.has_failures() {
        return Err("doctor found failing checks".into());
    }
    Ok(())
}

/// Starts a minimal Model Context Protocol server that exposes claw's
/// built-in tools over stdio.
///
/// Tool descriptors come from [`tools::mvp_tool_specs`] and calls are
/// dispatched through [`tools::execute_tool`], so this server exposes exactly
/// Read `.claw/worker-state.json` from the current working directory and print it.
/// This is the file-based worker observability surface: `push_event()` in `worker_boot.rs`
/// atomically writes state transitions here so external observers (clawhip, orchestrators)
/// can poll current `WorkerStatus` without needing an HTTP route on the opencode binary.
pub(crate) fn run_worker_state(
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let state_path = cwd.join(".claw").join("worker-state.json");
    if !state_path.exists() {
        // #139: this error used to say "run a worker first" without telling
        // callers how to run one. "worker" is an internal concept (there is
        // no `claw worker` subcommand), so claws/CI had no discoverable path
        // from the error to a fix. Emit an actionable, structured error that
        // names the two concrete commands that produce worker state.
        //
        // Format in both text and JSON modes is stable so scripts can match:
        //   error: no worker state file found at <path>
        //     Hint: worker state is written by the interactive REPL or a non-interactive prompt.
        //     Run:   claw               # start the REPL (writes state on first turn)
        //     Or:    claw prompt <text> # run one non-interactive turn
        //     Then rerun: claw state [--output-format json]
        return Err(format!(
            "no worker state file found at {path}\n  Hint: worker state is written by the interactive REPL or a non-interactive prompt.\n  Run:   claw               # start the REPL (writes state on first turn)\n  Or:    claw prompt <text> # run one non-interactive turn\n  Then rerun: claw state [--output-format json]",
            path = state_path.display()
        )
        .into());
    }
    let raw = std::fs::read_to_string(&state_path)?;
    match output_format {
        CliOutputFormat::Text => println!("{raw}"),
        CliOutputFormat::Json => {
            // Validate it parses as JSON before re-emitting
            let _: serde_json::Value = serde_json::from_str(&raw)?;
            println!("{raw}");
        }
    }
    Ok(())
}

/// the same surface the in-process agent loop uses.
pub(crate) fn run_mcp_serve() -> Result<(), Box<dyn std::error::Error>> {
    let tools = mvp_tool_specs()
        .into_iter()
        .map(|spec| McpTool {
            name: spec.name.to_string(),
            description: Some(spec.description.to_string()),
            input_schema: Some(spec.input_schema),
            annotations: None,
            meta: None,
        })
        .collect();

    let spec = McpServerSpec {
        server_name: "claw".to_string(),
        server_version: VERSION.to_string(),
        tools,
        tool_handler: Box::new(execute_tool),
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let mut server = McpServer::new(spec);
        server.run().await
    })?;
    Ok(())
}

pub(crate) fn dump_manifests(
    manifests_dir: Option<&Path>,
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace_dir = env::current_dir()?;
    dump_manifests_at_path(&workspace_dir, manifests_dir, output_format)
}

// Internal function for testing that accepts a workspace directory path.
pub(crate) fn dump_manifests_at_path(
    workspace_dir: &std::path::Path,
    manifests_dir: Option<&Path>,
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let discovery_root = manifests_dir.unwrap_or(workspace_dir);
    let resolved_root = discovery_root
        .canonicalize()
        .unwrap_or_else(|_| discovery_root.to_path_buf());

    if !resolved_root.exists() {
        return Err(format!(
            "missing_manifests: manifest discovery directory does not exist.\n  looked in: {}\n  {DUMP_MANIFESTS_USAGE_HINT}",
            resolved_root.display(),
        )
        .into());
    }
    if !resolved_root.is_dir() {
        return Err(format!(
            "missing_manifests: manifest discovery path is not a directory.\n  looked in: {}\n  {DUMP_MANIFESTS_USAGE_HINT}",
            resolved_root.display(),
        )
        .into());
    }

    let manifest = build_rust_resolver_manifest(&resolved_root)?;
    match output_format {
        CliOutputFormat::Text => {
            println!("Manifest Dump");
            println!("  Source           rust-resolver");
            println!("  Workspace        {}", resolved_root.display());
            println!("  Commands         {}", manifest["commands"]);
            println!("  Tools            {}", manifest["tools"]);
            println!("  Agents           {}", manifest["agents"]);
            println!("  Skills           {}", manifest["skills"]);
            println!("  Bootstrap phases {}", manifest["bootstrap_phases"]);
        }
        CliOutputFormat::Json => println!("{}", serde_json::to_string_pretty(&manifest)?),
    }
    Ok(())
}

pub(crate) fn print_bootstrap_plan(
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let phases = runtime::BootstrapPlan::claude_code_default();
    match output_format {
        CliOutputFormat::Text => {
            for phase in phases.phases() {
                println!("- {phase:?}");
            }
        }
        CliOutputFormat::Json => {
            // #412: emit structured phase objects with label and description
            let phase_objects: Vec<serde_json::Value> = phases
                .phases()
                .iter()
                .enumerate()
                .map(|(i, phase)| {
                    let (label, description) = bootstrap_phase_metadata(phase);
                    json!({
                        "id": format!("{phase:?}"),
                        "label": label,
                        "description": description,
                        "order": i,
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "kind": "bootstrap-plan",
                    "action": "show",
                    "status": "ok",
                    "total_phases": phases.phases().len(),
                    "phases": phase_objects,
                }))?
            );
        }
    }
    Ok(())
}

pub(crate) fn print_system_prompt(
    cwd: PathBuf,
    date: String,
    model: &str,
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let (sections, project_context) = load_system_prompt_with_context(
        cwd,
        date,
        env::consts::OS,
        "unknown",
        model_family_identity_for(model),
    )?;
    let (project_root, _) =
        parse_git_status_metadata_for(&project_context.cwd, project_context.git_status.as_deref());
    let memory_files = memory_file_summaries_for(
        &project_context.cwd,
        project_root.as_deref(),
        &project_context.instruction_files,
    );
    let message = sections.join(
        "

",
    );
    // #418: filter out the internal boundary sentinel from the sections array
    // and expose the boundary index as a structured field.
    let filtered_sections: Vec<&str> = sections
        .iter()
        .filter(|s| !s.contains("__SYSTEM_PROMPT_DYNAMIC_BOUNDARY__"))
        .map(|s| s.as_str())
        .collect();
    let boundary_index = sections
        .iter()
        .position(|s| s.contains("__SYSTEM_PROMPT_DYNAMIC_BOUNDARY__"));
    match output_format {
        CliOutputFormat::Text => println!("{message}"),
        CliOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "kind": "system-prompt",
                "action": "show",
                "status": "ok",
                "message": message,
                "sections": filtered_sections,
                "boundary_index": boundary_index,
                "memory_file_count": memory_files.len(),
                "memory_files": memory_files_json(&memory_files),
            }))?
        ),
    }
    Ok(())
}

pub(crate) fn print_version(
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match output_format {
        CliOutputFormat::Text => println!("{}", render_version_report()),
        CliOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&version_json_value())?);
        }
    }
    Ok(())
}

pub(crate) fn format_auto_compaction_notice(removed: usize) -> String {
    format!("[auto-compacted: removed {removed} messages]")
}

/// Enforce the broad-CWD policy: when running from home or root, either
/// require the --allow-broad-cwd flag, or prompt for confirmation (interactive),
/// or exit with an error (non-interactive).
pub(crate) fn enforce_broad_cwd_policy(
    allow_broad_cwd: bool,
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    if allow_broad_cwd {
        return Ok(());
    }
    let Some(cwd) = detect_broad_cwd() else {
        return Ok(());
    };

    let is_interactive = io::stdin().is_terminal();

    if is_interactive {
        // Interactive mode: print warning and ask for confirmation
        eprintln!(
            "Warning: claw is running from a very broad directory ({}).\n\
             The agent can read and search everything under this path.\n\
             Consider running from inside your project: cd /path/to/project && claw",
            cwd.display()
        );
        eprint!("Continue anyway? [y/N]: ");
        io::stderr().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim().to_lowercase();
        if trimmed != "y" && trimmed != "yes" {
            eprintln!("Aborted.");
            std::process::exit(0);
        }
        Ok(())
    } else {
        // Non-interactive mode: exit with error (JSON or text)
        let message = format!(
            "claw is running from a very broad directory ({}). \
             The agent can read and search everything under this path. \
             Use --allow-broad-cwd to proceed anyway, \
             or run from inside your project: cd /path/to/project && claw",
            cwd.display()
        );
        match output_format {
            CliOutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::json!({
                        "kind": "broad_cwd",
                        "action": "abort",
                        "status": "error",
                        "error_kind": "broad_cwd",
                        "error": message,
                        "hint": "Change to a more specific project directory, or use --cwd to set the workspace root.",
                        "exit_code": 1,
                    })
                );
            }
            CliOutputFormat::Text => {
                eprintln!("error: {message}");
            }
        }
        std::process::exit(1);
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_repl(
    model: String,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    base_commit: Option<String>,
    reasoning_effort: Option<String>,
    allow_broad_cwd: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    enforce_broad_cwd_policy(allow_broad_cwd, CliOutputFormat::Text)?;
    run_stale_base_preflight(base_commit.as_deref());
    let resolved_model = resolve_repl_model(model)?;
    let mut cli = LiveCli::new(resolved_model, true, allowed_tools, permission_mode)?;
    cli.set_reasoning_effort(reasoning_effort);
    let mut editor =
        input::LineEditor::new("> ", cli.repl_completion_candidates().unwrap_or_default());
    println!("{}", cli.startup_banner());
    println!("{}", format_connected_line(&cli.model));

    loop {
        editor.set_completions(cli.repl_completion_candidates().unwrap_or_default());
        match editor.read_line()? {
            input::ReadOutcome::Submit(input) => {
                let trimmed = input.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                if matches!(trimmed.as_str(), "/exit" | "/quit") {
                    cli.persist_session()?;
                    break;
                }
                match SlashCommand::parse(&trimmed) {
                    Ok(Some(command)) => {
                        if cli.handle_repl_command(command)? {
                            cli.persist_session()?;
                        }
                        continue;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("{error}");
                        continue;
                    }
                }
                // Bare-word skill dispatch: if the first token of the input
                // matches a known skill name, invoke it as `/skills <input>`
                // rather than forwarding raw text to the LLM (ROADMAP #36).
                let cwd = std::env::current_dir().unwrap_or_default();
                if let Some(prompt) = try_resolve_bare_skill_prompt(&cwd, &trimmed) {
                    editor.push_history(input);
                    cli.record_prompt_history(&trimmed);
                    cli.run_turn(&prompt)?;
                    continue;
                }
                editor.push_history(input);
                cli.record_prompt_history(&trimmed);
                cli.run_turn(&trimmed)?;
            }
            input::ReadOutcome::Cancel => {}
            input::ReadOutcome::Exit => {
                cli.persist_session()?;
                break;
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct SessionHandle {
    pub(crate) id: String,
    pub(crate) path: PathBuf,
}

pub(crate) struct LiveCli {
    pub(crate) model: String,
    pub(crate) allowed_tools: Option<AllowedToolSet>,
    pub(crate) permission_mode: PermissionMode,
    pub(crate) system_prompt: Vec<String>,
    pub(crate) runtime: BuiltRuntime,
    pub(crate) session: SessionHandle,
    pub(crate) prompt_history: Vec<PromptHistoryEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReadMcpResourceRequest {
    pub(crate) server: String,
    pub(crate) uri: String,
}

impl LiveCli {
    pub(crate) fn new(
        model: String,
        enable_tools: bool,
        allowed_tools: Option<AllowedToolSet>,
        permission_mode: PermissionMode,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let system_prompt = build_system_prompt(&model)?;
        let session_state = new_cli_session()?;
        let session = create_managed_session_handle(&session_state.session_id)?;
        let runtime = build_runtime(
            session_state.with_persistence_path(session.path.clone()),
            &session.id,
            model.clone(),
            system_prompt.clone(),
            enable_tools,
            true,
            allowed_tools.clone(),
            permission_mode,
            None,
        )?;
        let cli = Self {
            model,
            allowed_tools,
            permission_mode,
            system_prompt,
            runtime,
            session,
            prompt_history: Vec::new(),
        };
        cli.persist_session()?;
        Ok(cli)
    }

    pub(crate) fn set_reasoning_effort(&mut self, effort: Option<String>) {
        if let Some(rt) = self.runtime.runtime.as_mut() {
            rt.api_client_mut().set_reasoning_effort(effort);
        }
    }

    pub(crate) fn startup_banner(&self) -> String {
        let cwd = env::current_dir().map_or_else(
            |_| "<unknown>".to_string(),
            |path| path.display().to_string(),
        );
        let status = status_context(None).ok();
        let git_branch = status
            .as_ref()
            .and_then(|context| context.git_branch.as_deref())
            .unwrap_or("unknown");
        let workspace = status.as_ref().map_or_else(
            || "unknown".to_string(),
            |context| context.git_summary.headline(),
        );
        let session_path = self.session.path.strip_prefix(Path::new(&cwd)).map_or_else(
            |_| self.session.path.display().to_string(),
            |path| path.display().to_string(),
        );
        format!(
            "\x1b[38;5;196m\
 ██████╗██╗      █████╗ ██╗    ██╗\n\
██╔════╝██║     ██╔══██╗██║    ██║\n\
██║     ██║     ███████║██║ █╗ ██║\n\
██║     ██║     ██╔══██║██║███╗██║\n\
╚██████╗███████╗██║  ██║╚███╔███╔╝\n\
 ╚═════╝╚══════╝╚═╝  ╚═╝ ╚══╝╚══╝\x1b[0m \x1b[38;5;208mCode\x1b[0m 🦞\n\n\
  \x1b[2mModel\x1b[0m            {}\n\
  \x1b[2mPermissions\x1b[0m      {}\n\
  \x1b[2mBranch\x1b[0m           {}\n\
  \x1b[2mWorkspace\x1b[0m        {}\n\
  \x1b[2mDirectory\x1b[0m        {}\n\
  \x1b[2mSession\x1b[0m          {}\n\
  \x1b[2mAuto-save\x1b[0m        {}\n\n\
  Type \x1b[1m/help\x1b[0m for commands · \x1b[1m/status\x1b[0m for live context · \x1b[2m/resume latest\x1b[0m jumps back to the newest session · \x1b[1m/diff\x1b[0m then \x1b[1m/commit\x1b[0m to ship · \x1b[2mTab\x1b[0m for workflow completions · \x1b[2mShift+Enter\x1b[0m for newline",
            self.model,
            self.permission_mode.as_str(),
            git_branch,
            workspace,
            cwd,
            self.session.id,
            session_path,
        )
    }

    pub(crate) fn repl_completion_candidates(
        &self,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(slash_command_completion_candidates_with_sessions(
            &self.model,
            Some(&self.session.id),
            list_managed_sessions()?
                .into_iter()
                .map(|session| session.id)
                .collect(),
        ))
    }

    pub(crate) fn prepare_turn_runtime(
        &self,
        emit_output: bool,
    ) -> Result<(BuiltRuntime, HookAbortMonitor), Box<dyn std::error::Error>> {
        let hook_abort_signal = runtime::HookAbortSignal::new();
        let runtime = build_runtime(
            self.runtime.session().clone(),
            &self.session.id,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            emit_output,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?
        .with_hook_abort_signal(hook_abort_signal.clone());
        let hook_abort_monitor = HookAbortMonitor::spawn(hook_abort_signal);

        Ok((runtime, hook_abort_monitor))
    }

    pub(crate) fn replace_runtime(
        &mut self,
        runtime: BuiltRuntime,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.runtime.shutdown_plugins()?;
        self.runtime = runtime;
        Ok(())
    }

    pub(crate) fn run_turn(&mut self, input: &str) -> Result<(), Box<dyn std::error::Error>> {
        let (mut runtime, hook_abort_monitor) = self.prepare_turn_runtime(true)?;
        let mut spinner = Spinner::new();
        let mut stdout = io::stdout();
        spinner.tick(
            "🦀 Thinking...",
            TerminalRenderer::new().color_theme(),
            &mut stdout,
        )?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let result = runtime.run_turn(input, Some(&mut permission_prompter));
        hook_abort_monitor.stop();
        match result {
            Ok(summary) => {
                self.replace_runtime(runtime)?;
                spinner.finish(
                    "✨ Done",
                    TerminalRenderer::new().color_theme(),
                    &mut stdout,
                )?;
                let final_text = final_assistant_text(&summary);
                if !final_text.is_empty() {
                    println!("{final_text}");
                }
                println!();
                if let Some(event) = summary.auto_compaction {
                    println!(
                        "{}",
                        format_auto_compaction_notice(event.removed_message_count)
                    );
                }
                self.persist_session()?;
                Ok(())
            }
            Err(error) => {
                runtime.shutdown_plugins()?;
                spinner.fail(
                    "❌ Request failed",
                    TerminalRenderer::new().color_theme(),
                    &mut stdout,
                )?;

                // ============================================================================
                // Auto-compact retry on context window errors
                // ============================================================================
                // When the model API returns a context_window_blocked error (because the request
                // exceeds the model's context window), we automatically:
                // 1. Compact the session (remove old messages to free up space)
                // 2. Retry the original request with the compacted session
                // 3. Report results to the user
                //
                // This eliminates the need for users to manually run /compact when they
                // hit context limits - the recovery happens automatically.
                //
                // Detection: We look for "context_window" or "Context window" in the error
                // message, which covers error types like:
                // - "context_window_blocked"
                // - "Context window blocked"
                // - "This model's maximum context length is X tokens..."
                // ============================================================================

                let error_str = error.to_string();
                // Detect context window overflow. Some providers (e.g. OpenAI-compat backends)
                // return 400 with "no parseable body" instead of a proper context_length_exceeded
                // error when the request is too large to even parse — treat that as context overflow too.
                // Also detect model-specific context error markers (e.g. llama.cpp returns
                // "Context size has been exceeded." / "exceed_context_size_error" / "exceeds the available context size").
                let is_context_window = error_str.contains("context_window")
                    || error_str.contains("Context window")
                    || error_str.contains("no parseable body")
                    || error_str.contains("exceed_context_size")
                    || error_str.contains("exceeds the available context size")
                    || error_str
                        .to_ascii_lowercase()
                        .contains("context size has been exceeded");

                // Also treat "assistant stream produced no content" and reqwest decode failures
                // as recoverable errors that may benefit from auto-compaction. Some backends (e.g.
                // llama.cpp) return a non-SSE HTTP 500 body when context overflows, causing
                // reqwest to fail with "error decoding response body" — treat that as context overflow too.
                let is_no_content = error_str.contains("assistant stream produced no content")
                    || error_str.contains("Failed to parse input at pos")
                    || error_str.contains("error decoding response body");

                if is_context_window || is_no_content {
                    // If the error tells us the server's actual context window, adapt our
                    // auto-compaction threshold so future auto-compact-trigger checks are accurate.
                    if let Some(window) = extract_context_window_tokens_from_error(&error_str) {
                        // Set threshold at 70% of the reported window to leave headroom.
                        let threshold: u32 = (window as f64 * 0.7).round() as u32;
                        println!(
                            "  Server context window: {} tokens — setting auto-compaction threshold to {}",
                            window, threshold
                        );
                        runtime.set_auto_compaction_input_tokens_threshold(threshold);
                    }

                    // A single compaction pass may not free enough context space.
                    // Progressive retry: each round preserves fewer recent messages (4→2→1→0),
                    // trading conversation continuity for a smaller payload until it fits.
                    // Max 4 rounds before giving up and surfacing the error to the user.
                    let max_compact_rounds = 4;
                    let preserve_schedule = [4, 2, 1, 0];

                    for round in 0..max_compact_rounds {
                        let preserve = preserve_schedule[round];
                        println!(
                            "  Auto-compacting session (round {}/{}, preserving {} recent messages)...",
                            round + 1,
                            max_compact_rounds,
                            preserve
                        );

                        // Run Trident pipeline then summary-based compaction
                        let result = runtime::trident::trident_compact_session(
                            runtime.session(),
                            CompactionConfig {
                                preserve_recent_messages: preserve,
                                max_estimated_tokens: 0,
                            },
                            &runtime::trident::TridentConfig::default(),
                        );
                        let removed = result.removed_message_count;

                        if removed == 0 && round > 0 {
                            // No more messages to compact — further rounds won't help
                            println!("  No further compaction possible.");
                            break;
                        }

                        if removed > 0 {
                            println!(
                                "{}",
                                format_compact_report(
                                    removed,
                                    result.compacted_session.messages.len(),
                                    false
                                )
                            );
                        }

                        // Without this, prepare_turn_runtime() reads from self.runtime.session()
                        // which still holds the ORIGINAL un-compacted session, so every retry round
                        // would send the same bloated request — compaction was wasted.
                        *self.runtime.session_mut() = result.compacted_session.clone();

                        // Build a new runtime with the compacted session and retry
                        let (mut new_runtime, hook_abort_monitor) =
                            self.prepare_turn_runtime(true)?;
                        drop(hook_abort_monitor);

                        let mut rp = CliPermissionPrompter::new(self.permission_mode);
                        match new_runtime.run_turn(input, Some(&mut rp)) {
                            Ok(summary) => {
                                self.replace_runtime(new_runtime)?;
                                spinner.finish(
                                    if round == 0 {
                                        "✨ Done (after auto-compact)"
                                    } else {
                                        "✨ Done (after aggressive auto-compact)"
                                    },
                                    TerminalRenderer::new().color_theme(),
                                    &mut stdout,
                                )?;
                                println!();
                                if let Some(event) = summary.auto_compaction {
                                    println!(
                                        "{}",
                                        format_auto_compaction_notice(event.removed_message_count)
                                    );
                                }
                                self.persist_session()?;
                                return Ok(());
                            }
                            Err(retry_error) => {
                                let retry_str = retry_error.to_string();
                                let still_context_window = retry_str.contains("context_window")
                                    || retry_str.contains("Context window")
                                    || retry_str.contains("no parseable body")
                                    || retry_str.contains("exceed_context_size")
                                    || retry_str.contains("exceeds the available context size")
                                    || retry_str
                                        .to_ascii_lowercase()
                                        .contains("context size has been exceeded");
                                let still_no_content = retry_str
                                    .contains("assistant stream produced no content")
                                    || retry_str.contains("Failed to parse input at pos")
                                    || retry_str.contains("error decoding response body");

                                if (still_context_window || still_no_content)
                                    && round + 1 < max_compact_rounds
                                {
                                    // If the retry error reveals the context window, adapt threshold.
                                    if let Some(window) =
                                        extract_context_window_tokens_from_error(&retry_str)
                                    {
                                        let threshold: u32 = (window as f64 * 0.7).round() as u32;
                                        new_runtime
                                            .set_auto_compaction_input_tokens_threshold(threshold);
                                    }

                                    // The compacted session was still too large for the model's context.
                                    // Shut down the old runtime, adopt the partially-compacted one,
                                    // and loop — the next round will compact more aggressively.
                                    runtime.shutdown_plugins()?;
                                    runtime = new_runtime;
                                    continue;
                                }

                                // Not a context window error, or out of rounds
                                return Err(Box::new(retry_error));
                            }
                        }
                    }
                }

                // If not a context window error, return original error
                Err(Box::new(error))
            }
        }
    }

    pub(crate) fn run_turn_with_output(
        &mut self,
        input: &str,
        output_format: CliOutputFormat,
        compact: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match output_format {
            CliOutputFormat::Json if compact => self.run_prompt_compact_json(input),
            CliOutputFormat::Text if compact => self.run_prompt_compact(input),
            CliOutputFormat::Text => self.run_turn(input),
            CliOutputFormat::Json => self.run_prompt_json(input),
        }
    }

    pub(crate) fn run_prompt_compact(
        &mut self,
        input: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (mut runtime, hook_abort_monitor) = self.prepare_turn_runtime(false)?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let result = runtime.run_turn(input, Some(&mut permission_prompter));
        hook_abort_monitor.stop();
        let summary = result?;
        self.replace_runtime(runtime)?;
        self.persist_session()?;
        let final_text = final_assistant_text(&summary);
        println!("{final_text}");
        Ok(())
    }

    pub(crate) fn run_prompt_compact_json(
        &mut self,
        input: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (mut runtime, hook_abort_monitor) = self.prepare_turn_runtime(false)?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let result = runtime.run_turn(input, Some(&mut permission_prompter));
        hook_abort_monitor.stop();
        let summary = result?;
        self.replace_runtime(runtime)?;
        self.persist_session()?;
        println!(
            "{}",
            json!({
                "message": final_assistant_text(&summary),
                "compact": true,
                "model": self.model,
                "usage": {
                    "input_tokens": summary.usage.input_tokens,
                    "output_tokens": summary.usage.output_tokens,
                    "cache_creation_input_tokens": summary.usage.cache_creation_input_tokens,
                    "cache_read_input_tokens": summary.usage.cache_read_input_tokens,
                },
            })
        );
        Ok(())
    }

    pub(crate) fn run_prompt_json(
        &mut self,
        input: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (mut runtime, hook_abort_monitor) = self.prepare_turn_runtime(false)?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let result = runtime.run_turn(input, Some(&mut permission_prompter));
        hook_abort_monitor.stop();
        let summary = result?;
        self.replace_runtime(runtime)?;
        self.persist_session()?;
        println!(
            "{}",
            json!({
                "message": final_assistant_text(&summary),
                "model": self.model,
                "iterations": summary.iterations,
                "auto_compaction": summary.auto_compaction.map(|event| json!({
                    "removed_messages": event.removed_message_count,
                    "notice": format_auto_compaction_notice(event.removed_message_count),
                })),
                "tool_uses": collect_tool_uses(&summary),
                "tool_results": collect_tool_results(&summary),
                "prompt_cache_events": collect_prompt_cache_events(&summary),
                "usage": {
                    "input_tokens": summary.usage.input_tokens,
                    "output_tokens": summary.usage.output_tokens,
                    "cache_creation_input_tokens": summary.usage.cache_creation_input_tokens,
                    "cache_read_input_tokens": summary.usage.cache_read_input_tokens,
                },
                "estimated_cost": format_usd(
                    summary.usage.estimate_cost_usd_with_pricing(
                        pricing_for_model(&self.model)
                            .unwrap_or_else(runtime::ModelPricing::default_sonnet_tier)
                    ).total_cost_usd()
                )
            })
        );
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn handle_repl_command(
        &mut self,
        command: SlashCommand,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(match command {
            SlashCommand::Help => {
                println!("{}", render_repl_help());
                false
            }
            SlashCommand::Status => {
                self.print_status();
                false
            }
            SlashCommand::Bughunter { scope } => {
                self.run_bughunter(scope.as_deref())?;
                false
            }
            SlashCommand::Commit => {
                self.run_commit(None)?;
                false
            }
            SlashCommand::Pr { context } => {
                self.run_pr(context.as_deref())?;
                false
            }
            SlashCommand::Issue { context } => {
                self.run_issue(context.as_deref())?;
                false
            }
            SlashCommand::Ultraplan { task } => {
                self.run_ultraplan(task.as_deref())?;
                false
            }
            SlashCommand::Teleport { target } => {
                Self::run_teleport(target.as_deref())?;
                false
            }
            SlashCommand::DebugToolCall => {
                self.run_debug_tool_call(None)?;
                false
            }
            SlashCommand::Sandbox => {
                Self::print_sandbox_status();
                false
            }
            SlashCommand::Compact => {
                self.compact()?;
                false
            }
            SlashCommand::Model { model } => self.set_model(model)?,
            SlashCommand::Permissions { mode } => self.set_permissions(mode)?,
            SlashCommand::Clear { confirm } => self.clear_session(confirm)?,
            SlashCommand::Cost => {
                self.print_cost();
                false
            }
            SlashCommand::Resume { session_path } => self.resume_session(session_path)?,
            SlashCommand::Config { section } => {
                Self::print_config(section.as_deref())?;
                false
            }
            SlashCommand::Mcp { action, target } => {
                let args = match (action.as_deref(), target.as_deref()) {
                    (None, None) => None,
                    (Some(action), None) => Some(action.to_string()),
                    (Some(action), Some(target)) => Some(format!("{action} {target}")),
                    (None, Some(target)) => Some(target.to_string()),
                };
                Self::print_mcp(args.as_deref(), CliOutputFormat::Text)?;
                false
            }
            SlashCommand::Memory => {
                Self::print_memory()?;
                false
            }
            SlashCommand::Init => {
                run_init(CliOutputFormat::Text)?;
                false
            }
            SlashCommand::Diff => {
                Self::print_diff()?;
                false
            }
            SlashCommand::Version => {
                Self::print_version(CliOutputFormat::Text);
                false
            }
            SlashCommand::Export { path } => {
                self.export_session(path.as_deref())?;
                false
            }
            SlashCommand::Session { action, target } => {
                self.handle_session_command(action.as_deref(), target.as_deref())?
            }
            SlashCommand::Plugins { action, target } => {
                self.handle_plugins_command(action.as_deref(), target.as_deref())?
            }
            SlashCommand::Agents { args } => {
                if let Err(error) = Self::print_agents(args.as_deref(), CliOutputFormat::Text) {
                    eprintln!("{error}");
                }
                false
            }
            SlashCommand::Skills { args } => {
                match classify_skills_slash_command(args.as_deref()) {
                    SkillSlashDispatch::Invoke(prompt) => self.run_turn(&prompt)?,
                    SkillSlashDispatch::Local => {
                        if let Err(error) =
                            Self::print_skills(args.as_deref(), CliOutputFormat::Text)
                        {
                            eprintln!("{error}");
                        }
                    }
                }
                false
            }
            SlashCommand::Doctor => {
                println!(
                    "{}",
                    render_doctor_report(
                        ConfigWarningMode::EmitStderr,
                        permission_mode_provenance_for_current_dir(),
                    )?
                    .render()
                );
                false
            }
            SlashCommand::Setup => {
                if let Err(e) = setup_wizard::run_setup_wizard() {
                    eprintln!("Setup wizard failed: {e}");
                }
                false
            }
            SlashCommand::History { count } => {
                self.print_prompt_history(count.as_deref());
                false
            }
            SlashCommand::Stats => {
                let usage = UsageTracker::from_session(self.runtime.session()).cumulative_usage();
                println!("{}", format_cost_report(usage));
                false
            }
            SlashCommand::Login
            | SlashCommand::Logout
            | SlashCommand::Vim
            | SlashCommand::Upgrade
            | SlashCommand::Share
            | SlashCommand::Feedback
            | SlashCommand::Files
            | SlashCommand::Fast
            | SlashCommand::Exit
            | SlashCommand::Summary
            | SlashCommand::Desktop
            | SlashCommand::Brief
            | SlashCommand::Advisor
            | SlashCommand::Stickers
            | SlashCommand::Insights
            | SlashCommand::Thinkback
            | SlashCommand::ReleaseNotes
            | SlashCommand::SecurityReview
            | SlashCommand::Keybindings
            | SlashCommand::PrivacySettings
            | SlashCommand::Plan { .. }
            | SlashCommand::Review { .. }
            | SlashCommand::Tasks { .. }
            | SlashCommand::Theme { .. }
            | SlashCommand::Voice { .. }
            | SlashCommand::Usage { .. }
            | SlashCommand::Rename { .. }
            | SlashCommand::Copy { .. }
            | SlashCommand::Hooks { .. }
            | SlashCommand::Context { .. }
            | SlashCommand::Color { .. }
            | SlashCommand::Effort { .. }
            | SlashCommand::Branch { .. }
            | SlashCommand::Rewind { .. }
            | SlashCommand::Ide { .. }
            | SlashCommand::Tag { .. }
            | SlashCommand::OutputStyle { .. }
            | SlashCommand::AddDir { .. }
            | SlashCommand::Team { .. } => {
                let cmd_name = command.slash_name();
                eprintln!("{cmd_name} is not yet implemented in this build.");
                false
            }
            SlashCommand::Unknown(name) => {
                eprintln!("{}", format_unknown_slash_command(&name));
                false
            }
        })
    }

    pub(crate) fn persist_session(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.runtime.session().save_to_path(&self.session.path)?;
        Ok(())
    }

    pub(crate) fn print_status(&self) {
        let cumulative = self.runtime.usage().cumulative_usage();
        let latest = self.runtime.usage().current_turn_usage();
        println!(
            "{}",
            format_status_report(
                &self.model,
                StatusUsage {
                    message_count: self.runtime.session().messages.len(),
                    turns: self.runtime.usage().turns(),
                    latest,
                    cumulative,
                    estimated_tokens: self.runtime.estimated_tokens(),
                },
                self.permission_mode.as_str(),
                &status_context(Some(&self.session.path)).expect("status context should load"),
                None, // #148: REPL /status doesn't carry flag provenance
                None,
            )
        );
    }

    pub(crate) fn record_prompt_history(&mut self, prompt: &str) {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map_or(self.runtime.session().updated_at_ms, |duration| {
                u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
            });
        let entry = PromptHistoryEntry {
            timestamp_ms,
            text: prompt.to_string(),
        };
        self.prompt_history.push(entry);
        if let Err(error) = self.runtime.session_mut().push_prompt_entry(prompt) {
            eprintln!("warning: failed to persist prompt history: {error}");
        }
    }

    pub(crate) fn print_prompt_history(&self, count: Option<&str>) {
        let limit = match parse_history_count(count) {
            Ok(limit) => limit,
            Err(message) => {
                eprintln!("{message}");
                return;
            }
        };
        let session_entries = &self.runtime.session().prompt_history;
        let entries = if session_entries.is_empty() {
            if self.prompt_history.is_empty() {
                collect_session_prompt_history(self.runtime.session())
            } else {
                self.prompt_history
                    .iter()
                    .map(|entry| PromptHistoryEntry {
                        timestamp_ms: entry.timestamp_ms,
                        text: entry.text.clone(),
                    })
                    .collect()
            }
        } else {
            session_entries
                .iter()
                .map(|entry| PromptHistoryEntry {
                    timestamp_ms: entry.timestamp_ms,
                    text: entry.text.clone(),
                })
                .collect()
        };
        println!("{}", render_prompt_history_report(&entries, limit));
    }

    pub(crate) fn print_sandbox_status() {
        let cwd = env::current_dir().expect("current dir");
        let loader = ConfigLoader::default_for(&cwd);
        let runtime_config = loader
            .load()
            .unwrap_or_else(|_| runtime::RuntimeConfig::empty());
        println!(
            "{}",
            format_sandbox_report(&resolve_sandbox_status(runtime_config.sandbox(), &cwd))
        );
    }

    pub(crate) fn set_model(
        &mut self,
        model: Option<String>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(model) = model else {
            println!(
                "{}",
                format_model_report(
                    &self.model,
                    self.runtime.session().messages.len(),
                    self.runtime.usage().turns(),
                )
            );
            return Ok(false);
        };

        let model = resolve_model_alias_with_config(&model);

        if model == self.model {
            println!(
                "{}",
                format_model_report(
                    &self.model,
                    self.runtime.session().messages.len(),
                    self.runtime.usage().turns(),
                )
            );
            return Ok(false);
        }

        let previous = self.model.clone();
        let session = self.runtime.session().clone();
        let message_count = session.messages.len();
        let runtime = build_runtime(
            session,
            &self.session.id,
            model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.replace_runtime(runtime)?;
        self.model.clone_from(&model);
        println!(
            "{}",
            format_model_switch_report(&previous, &model, message_count)
        );
        Ok(true)
    }

    pub(crate) fn set_permissions(
        &mut self,
        mode: Option<String>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(mode) = mode else {
            println!(
                "{}",
                format_permissions_report(self.permission_mode.as_str())
            );
            return Ok(false);
        };

        let normalized = normalize_permission_mode(&mode).ok_or_else(|| {
            format!(
                "invalid_flag_value: unsupported permission mode '{mode}'.\nUsage: --permission-mode read-only|workspace-write|danger-full-access"
            )
        })?;

        if normalized == self.permission_mode.as_str() {
            println!("{}", format_permissions_report(normalized));
            return Ok(false);
        }

        let previous = self.permission_mode.as_str().to_string();
        let session = self.runtime.session().clone();
        self.permission_mode = permission_mode_from_label(normalized);
        let runtime = build_runtime(
            session,
            &self.session.id,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.replace_runtime(runtime)?;
        println!(
            "{}",
            format_permissions_switch_report(&previous, normalized)
        );
        Ok(true)
    }

    pub(crate) fn clear_session(
        &mut self,
        confirm: bool,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        if !confirm {
            println!(
                "clear: confirmation required; run /clear --confirm to start a fresh session."
            );
            return Ok(false);
        }

        let previous_session = self.session.clone();
        let session_state = new_cli_session()?;
        self.session = create_managed_session_handle(&session_state.session_id)?;
        let runtime = build_runtime(
            session_state.with_persistence_path(self.session.path.clone()),
            &self.session.id,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.replace_runtime(runtime)?;
        println!(
            "Session cleared\n  Mode             fresh session\n  Previous session {}\n  Resume previous  /resume {}\n  Preserved model  {}\n  Permission mode  {}\n  New session      {}\n  Session file     {}",
            previous_session.id,
            previous_session.id,
            self.model,
            self.permission_mode.as_str(),
            self.session.id,
            self.session.path.display(),
        );
        Ok(true)
    }

    pub(crate) fn print_cost(&self) {
        let cumulative = self.runtime.usage().cumulative_usage();
        println!("{}", format_cost_report(cumulative));
    }

    pub(crate) fn resume_session(
        &mut self,
        session_path: Option<String>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let Some(session_ref) = session_path else {
            println!("{}", render_resume_usage());
            return Ok(false);
        };

        let (handle, session) =
            load_session_reference_excluding(&session_ref, Some(&self.session.id))?;
        let message_count = session.messages.len();
        let session_id = session.session_id.clone();
        let runtime = build_runtime(
            session,
            &handle.id,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.replace_runtime(runtime)?;
        self.session = SessionHandle {
            id: session_id,
            path: handle.path,
        };
        println!(
            "{}",
            format_resume_report(
                &self.session.path.display().to_string(),
                message_count,
                self.runtime.usage().turns(),
            )
        );
        Ok(true)
    }

    pub(crate) fn print_config(section: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", render_config_report(section)?);
        Ok(())
    }

    pub(crate) fn print_memory() -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", render_memory_report()?);
        Ok(())
    }

    pub(crate) fn print_agents(
        args: Option<&str>,
        output_format: CliOutputFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        match output_format {
            CliOutputFormat::Text => println!("{}", handle_agents_slash_command(args, &cwd)?),
            CliOutputFormat::Json => {
                let value = handle_agents_slash_command_json(args, &cwd)?;
                // #789: parity with print_mcp/#788 print_skills — exit 1 when envelope
                // reports an error so automation can rely on exit code instead of
                // parsing the JSON status field.
                let is_error = value.get("status").and_then(|v| v.as_str()) == Some("error");
                println!("{}", serde_json::to_string_pretty(&value)?);
                if is_error {
                    std::process::exit(1);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn print_mcp(
        args: Option<&str>,
        output_format: CliOutputFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // `claw mcp serve` starts a stdio MCP server exposing claw's built-in
        // tools. All other `mcp` subcommands fall through to the existing
        // configured-server reporter (`list`, `status`, ...).
        if matches!(args.map(str::trim), Some("serve")) {
            return run_mcp_serve();
        }
        let cwd = env::current_dir()?;
        match output_format {
            CliOutputFormat::Text => println!("{}", handle_mcp_slash_command(args, &cwd)?),
            CliOutputFormat::Json => {
                let value = handle_mcp_slash_command_json(args, &cwd)?;
                // Propagate ok:false → non-zero exit so automation callers
                // can rely on exit code instead of inspecting the envelope.
                // (#68: mcp error envelopes previously always exited 0.)
                let is_error = value.get("ok").and_then(serde_json::Value::as_bool) == Some(false)
                    || value.get("status").and_then(serde_json::Value::as_str) == Some("error");
                println!("{}", serde_json::to_string_pretty(&value)?);
                if is_error {
                    std::process::exit(1);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn print_skills(
        args: Option<&str>,
        output_format: CliOutputFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        match output_format {
            CliOutputFormat::Text => println!("{}", handle_skills_slash_command(args, &cwd)?),
            CliOutputFormat::Json => {
                let result = handle_skills_slash_command_json(args, &cwd)?;
                let is_error = result.get("status").and_then(|v| v.as_str()) == Some("error");
                // #739: action:"help" with unexpected set is a usage response, not a fatal error;
                // don't return Err which would emit a second error envelope from the generic path.
                let is_help_action = result.get("action").and_then(|v| v.as_str()) == Some("help");
                println!("{}", serde_json::to_string_pretty(&result)?);
                if is_error && !is_help_action {
                    // #788: the error JSON is already emitted above; returning Err here
                    // would cause the top-level handler to emit a second error envelope.
                    // Exit directly to signal failure without a duplicate envelope.
                    std::process::exit(1);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn print_plugins(
        action: Option<&str>,
        target: Option<&str>,
        output_format: CliOutputFormat,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        // #803: reject flag-shaped tokens in list filter for BOTH text and JSON modes.
        // Previously the guard was JSON-only (#793); text mode silently returned empty success.
        if action.as_deref() == Some("list") {
            if let Some(filter) = target.as_deref() {
                if filter.starts_with('-') {
                    if matches!(output_format, CliOutputFormat::Json) {
                        // ROADMAP #817: this is a handled local inventory parse error.
                        // Keep it on stdout in JSON mode so `plugins list --` matches the
                        // sibling JSON inventory/local surfaces instead of falling through
                        // to the top-level stderr error path.
                        let obj = json!({
                            "type": "error",
                            "kind": "plugin",
                            "action": "list",
                            "status": "error",
                            "error_kind": "cli_parse",
                            "error": format!("unknown option for `claw plugins list`: {filter}"),
                            "message": format!("unknown option for `claw plugins list`: {filter}"),
                            "unexpected": filter,
                            "hint": "Usage: claw plugins list [<filter>]\nFilters are id substrings, not flags.",
                            "exit_code": 1,
                        });
                        println!("{}", serde_json::to_string_pretty(&obj)?);
                        std::process::exit(1);
                    }
                    return Err(format!(
                        "unknown option for `claw plugins list`: {filter}\nUsage: claw plugins list [<filter>]\nFilters are id substrings, not flags."
                    ).into());
                }
            }
        }
        let payload = plugins_command_payload_for(
            &cwd,
            action,
            target,
            match output_format {
                CliOutputFormat::Json => ConfigWarningMode::SuppressStderr,
                CliOutputFormat::Text => ConfigWarningMode::EmitStderr,
            },
        )?;
        match output_format {
            CliOutputFormat::Text => {
                // #806: text-mode show must return error when plugin not found (parity with JSON)
                let action_str = action.unwrap_or("list");
                if matches!(action_str, "show" | "info" | "describe") {
                    if let Some(name) = target {
                        let needle = name.to_lowercase();
                        let found = payload.plugins.iter().any(|p| {
                            p.get("id")
                                .and_then(|v| v.as_str())
                                .map(|id| id.to_lowercase() == needle)
                                .unwrap_or(false)
                        });
                        if !found {
                            return Err(format!(
                                "plugin_not_found: plugin '{}' not found\nRun `claw plugins list` to see available plugins.",
                                name
                            ).into());
                        }
                    }
                }
                println!("{}", payload.message);
            }
            CliOutputFormat::Json => {
                let action_str = action.unwrap_or("list");
                // #743/#420: plugins help must return a usage envelope matching agents/mcp/skills help shape.
                if matches!(action_str, "help" | "-h" | "--help") {
                    let cwd_str = cwd.display().to_string();
                    let obj = json!({
                        "kind": "plugin",
                        "action": "help",
                        "status": "ok",
                        "unexpected": null,
                        "usage": {
                            "direct_cli": "claw plugins [list|show <id>|install <id>|enable <id>|disable <id>|uninstall <id>|update <id>|help]",
                            "slash_command": "/plugins [list|show <id>|install <id>|enable <id>|disable <id>|uninstall <id>|update <id>|help]",
                        },
                        "cwd": cwd_str,
                    });
                    println!("{}", serde_json::to_string_pretty(&obj)?);
                    return Ok(());
                }
                // For show/info/describe, filter to the named plugin (exact match).
                // For list with a target, treat target as a substring filter.
                let is_show_action = matches!(action_str, "show" | "info" | "describe");
                let is_list_action = action_str == "list";
                let filtered_plugins: Vec<_> = if is_show_action {
                    if let Some(name) = target {
                        let needle = name.to_lowercase();
                        payload
                            .plugins
                            .iter()
                            .filter(|p| {
                                p.get("id")
                                    .and_then(|v| v.as_str())
                                    .map(|id| id.to_lowercase() == needle)
                                    .unwrap_or(false)
                            })
                            .cloned()
                            .collect()
                    } else {
                        payload.plugins.clone()
                    }
                } else if is_list_action {
                    if let Some(filter) = target {
                        let needle = filter.to_lowercase();
                        payload
                            .plugins
                            .iter()
                            .filter(|p| {
                                p.get("id")
                                    .and_then(|v| v.as_str())
                                    .map(|id| id.to_lowercase().contains(&needle))
                                    .unwrap_or(false)
                            })
                            .cloned()
                            .collect()
                    } else {
                        payload.plugins.clone()
                    }
                } else {
                    payload.plugins.clone()
                };
                // Return not-found error for show with missing target.
                if is_show_action {
                    if let Some(name) = target {
                        if filtered_plugins.is_empty() {
                            let obj = json!({
                                "kind": "plugin",
                                "action": action_str,
                                "status": "error",
                                "error_kind": "plugin_not_found",
                                "requested": name,
                                // #734: parity with skills show which always emits a message field
                                "message": format!("plugin '{}' not found", name),
                                // #760: hint so callers know how to enumerate available plugins
                                "hint": "Run `claw plugins list` to see available plugins.",
                            });
                            println!("{}", serde_json::to_string_pretty(&obj)?);
                            // #789: exit 1 on not-found so automation can rely on exit code
                            std::process::exit(1);
                        }
                    }
                }
                let enabled_count = filtered_plugins
                    .iter()
                    .filter(|p| p.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false))
                    .count();
                let disabled_count = filtered_plugins.len().saturating_sub(enabled_count);
                let mut obj = json!({
                    "kind": "plugin",
                    "action": action_str,
                    "status": payload.status,
                    "summary": {
                        "total": filtered_plugins.len(),
                        "enabled": enabled_count,
                        "disabled": disabled_count,
                        "load_failures": payload.load_failures.len(),
                    },
                    "config_load_error": payload.config_load_error,
                    "mcp_validation": payload.mcp_validation.json_value(),
                    "plugins": filtered_plugins,
                    "load_failures": payload.load_failures,
                });
                // Only include operation-result fields for mutating actions (not list/show)
                if action_str != "list" && !is_show_action {
                    obj["target"] = json!(target);
                    obj["reload_runtime"] = json!(payload.reload_runtime);
                    obj["message"] = json!(payload.message);
                }
                println!("{}", serde_json::to_string_pretty(&obj)?);
            }
        }
        Ok(())
    }

    pub(crate) fn print_diff() -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", render_diff_report()?);
        Ok(())
    }

    pub(crate) fn print_version(output_format: CliOutputFormat) {
        let _ = crate::print_version(output_format);
    }

    pub(crate) fn export_session(
        &self,
        requested_path: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let export_path = resolve_export_path(requested_path, self.runtime.session())?;
        fs::write(&export_path, render_export_text(self.runtime.session()))?;
        println!(
            "Export\n  Result           wrote transcript\n  File             {}\n  Messages         {}",
            export_path.display(),
            self.runtime.session().messages.len(),
        );
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn handle_session_command(
        &mut self,
        action: Option<&str>,
        target: Option<&str>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match action {
            None | Some("list") => {
                println!("{}", render_session_list(&self.session.id)?);
                Ok(false)
            }
            Some("exists") => {
                let Some(target) = target else {
                    println!("Usage: /session exists <session-id>");
                    return Ok(false);
                };
                let exists = session_reference_exists(target)?;
                let handle = resolve_session_reference(target).ok();
                println!(
                    "Session exists\n  Session          {target}\n  Exists           {exists}{}",
                    handle
                        .as_ref()
                        .map(|handle| format!("\n  File             {}", handle.path.display()))
                        .unwrap_or_default()
                );
                Ok(false)
            }
            Some("switch") => {
                let Some(target) = target else {
                    println!("Usage: /session switch <session-id>");
                    return Ok(false);
                };
                let (handle, session) = load_session_reference(target)?;
                let message_count = session.messages.len();
                let session_id = session.session_id.clone();
                let runtime = build_runtime(
                    session,
                    &handle.id,
                    self.model.clone(),
                    self.system_prompt.clone(),
                    true,
                    true,
                    self.allowed_tools.clone(),
                    self.permission_mode,
                    None,
                )?;
                self.replace_runtime(runtime)?;
                self.session = SessionHandle {
                    id: session_id,
                    path: handle.path,
                };
                println!(
                    "Session switched\n  Active session   {}\n  File             {}\n  Messages         {}",
                    self.session.id,
                    self.session.path.display(),
                    message_count,
                );
                Ok(true)
            }
            Some("fork") => {
                let forked = self.runtime.fork_session(target.map(ToOwned::to_owned));
                let parent_session_id = self.session.id.clone();
                let handle = create_managed_session_handle(&forked.session_id)?;
                let branch_name = forked
                    .fork
                    .as_ref()
                    .and_then(|fork| fork.branch_name.clone());
                let forked = forked.with_persistence_path(handle.path.clone());
                let message_count = forked.messages.len();
                forked.save_to_path(&handle.path)?;
                let runtime = build_runtime(
                    forked,
                    &handle.id,
                    self.model.clone(),
                    self.system_prompt.clone(),
                    true,
                    true,
                    self.allowed_tools.clone(),
                    self.permission_mode,
                    None,
                )?;
                self.replace_runtime(runtime)?;
                self.session = handle;
                println!(
                    "Session forked\n  Parent session   {}\n  Active session   {}\n  Branch           {}\n  File             {}\n  Messages         {}",
                    parent_session_id,
                    self.session.id,
                    branch_name.as_deref().unwrap_or("(unnamed)"),
                    self.session.path.display(),
                    message_count,
                );
                Ok(true)
            }
            Some("delete") => {
                let Some(target) = target else {
                    println!("Usage: /session delete <session-id> [--force]");
                    return Ok(false);
                };
                let handle = resolve_session_reference(target)?;
                if handle.id == self.session.id {
                    println!(
                        "delete: refusing to delete the active session '{}'.\nSwitch to another session first with /session switch <session-id>.",
                        handle.id
                    );
                    return Ok(false);
                }
                if !confirm_session_deletion(&handle.id) {
                    println!("delete: cancelled.");
                    return Ok(false);
                }
                delete_managed_session(&handle.path)?;
                println!(
                    "Session deleted\n  Deleted session  {}\n  File             {}",
                    handle.id,
                    handle.path.display(),
                );
                Ok(false)
            }
            Some("delete-force") => {
                let Some(target) = target else {
                    println!("Usage: /session delete <session-id> [--force]");
                    return Ok(false);
                };
                let handle = resolve_session_reference(target)?;
                if handle.id == self.session.id {
                    println!(
                        "delete: refusing to delete the active session '{}'.\nSwitch to another session first with /session switch <session-id>.",
                        handle.id
                    );
                    return Ok(false);
                }
                delete_managed_session(&handle.path)?;
                println!(
                    "Session deleted\n  Deleted session  {}\n  File             {}",
                    handle.id,
                    handle.path.display(),
                );
                Ok(false)
            }
            Some(other) => {
                println!(
                    "Unknown /session action '{other}'. Use /session list, /session exists <session-id>, /session switch <session-id>, /session fork [branch-name], or /session delete <session-id> [--force]."
                );
                Ok(false)
            }
        }
    }

    pub(crate) fn handle_plugins_command(
        &mut self,
        action: Option<&str>,
        target: Option<&str>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let cwd = env::current_dir()?;
        let payload =
            plugins_command_payload_for(&cwd, action, target, ConfigWarningMode::EmitStderr)?;
        println!("{}", payload.message);
        if payload.reload_runtime {
            self.reload_runtime_features()?;
        }
        Ok(false)
    }

    pub(crate) fn reload_runtime_features(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let runtime = build_runtime(
            self.runtime.session().clone(),
            &self.session.id,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.replace_runtime(runtime)?;
        self.persist_session()
    }

    pub(crate) fn compact(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let result = self.runtime.compact(CompactionConfig::default());
        let removed = result.removed_message_count;
        let kept = result.compacted_session.messages.len();
        let skipped = removed == 0;
        let runtime = build_runtime(
            result.compacted_session,
            &self.session.id,
            self.model.clone(),
            self.system_prompt.clone(),
            true,
            true,
            self.allowed_tools.clone(),
            self.permission_mode,
            None,
        )?;
        self.replace_runtime(runtime)?;
        self.persist_session()?;
        println!("{}", format_compact_report(removed, kept, skipped));
        Ok(())
    }

    pub(crate) fn run_internal_prompt_text_with_progress(
        &self,
        prompt: &str,
        enable_tools: bool,
        progress: Option<InternalPromptProgressReporter>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let session = self.runtime.session().clone();
        let mut runtime = build_runtime(
            session,
            &self.session.id,
            self.model.clone(),
            self.system_prompt.clone(),
            enable_tools,
            false,
            self.allowed_tools.clone(),
            self.permission_mode,
            progress,
        )?;
        let mut permission_prompter = CliPermissionPrompter::new(self.permission_mode);
        let summary = runtime.run_turn(prompt, Some(&mut permission_prompter))?;
        let text = final_assistant_text(&summary).trim().to_string();
        runtime.shutdown_plugins()?;
        Ok(text)
    }

    pub(crate) fn run_internal_prompt_text(
        &self,
        prompt: &str,
        enable_tools: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.run_internal_prompt_text_with_progress(prompt, enable_tools, None)
    }

    pub(crate) fn run_bughunter(
        &self,
        scope: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", format_bughunter_report(scope));
        Ok(())
    }

    pub(crate) fn run_ultraplan(
        &self,
        task: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", format_ultraplan_report(task));
        Ok(())
    }

    pub(crate) fn run_teleport(target: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let Some(target) = target.map(str::trim).filter(|value| !value.is_empty()) else {
            println!("Usage: /teleport <symbol-or-path>");
            return Ok(());
        };

        println!("{}", render_teleport_report(target)?);
        Ok(())
    }

    pub(crate) fn run_debug_tool_call(
        &self,
        args: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        validate_no_args("/debug-tool-call", args)?;
        println!("{}", render_last_tool_debug_report(self.runtime.session())?);
        Ok(())
    }

    pub(crate) fn run_commit(
        &mut self,
        args: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        validate_no_args("/commit", args)?;
        let status = git_output(&["status", "--short", "--branch"])?;
        let summary = parse_git_workspace_summary(Some(&status));
        let branch = parse_git_status_branch(Some(&status));
        if summary.is_clean() {
            println!("{}", format_commit_skipped_report());
            return Ok(());
        }

        println!(
            "{}",
            format_commit_preflight_report(branch.as_deref(), summary)
        );
        Ok(())
    }

    pub(crate) fn run_pr(&self, context: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let branch =
            resolve_git_branch_for(&env::current_dir()?).unwrap_or_else(|| "unknown".to_string());
        println!("{}", format_pr_report(&branch, context));
        Ok(())
    }

    pub(crate) fn run_issue(
        &self,
        context: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", format_issue_report(context));
        Ok(())
    }
}

pub(crate) fn sessions_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(current_session_store()?.sessions_dir().to_path_buf())
}

pub(crate) fn current_session_store() -> Result<runtime::SessionStore, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    runtime::SessionStore::from_cwd(&cwd).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

pub(crate) fn new_cli_session() -> Result<Session, Box<dyn std::error::Error>> {
    Ok(Session::new().with_workspace_root(env::current_dir()?))
}

pub(crate) fn create_managed_session_handle(
    session_id: &str,
) -> Result<SessionHandle, Box<dyn std::error::Error>> {
    let handle = current_session_store()?.create_handle(session_id);
    Ok(SessionHandle {
        id: handle.id,
        path: handle.path,
    })
}

pub(crate) fn resolve_session_reference(
    reference: &str,
) -> Result<SessionHandle, Box<dyn std::error::Error>> {
    let handle = current_session_store()?
        .resolve_reference(reference)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(SessionHandle {
        id: handle.id,
        path: handle.path,
    })
}

pub(crate) fn session_reference_exists(
    reference: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(current_session_store()?.session_exists(reference))
}

pub(crate) fn resolve_managed_session_path(
    session_id: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    current_session_store()?
        .resolve_managed_path(session_id)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

pub(crate) fn list_managed_sessions(
) -> Result<Vec<ManagedSessionSummary>, Box<dyn std::error::Error>> {
    let store = current_session_store()?;
    let lifecycle = classify_session_lifecycle_for(store.workspace_root());
    Ok(store
        .list_sessions()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
        .into_iter()
        .map(|session| ManagedSessionSummary {
            id: session.id,
            path: session.path,
            created_at_ms: session.created_at_ms,
            updated_at_ms: session.updated_at_ms,
            modified_epoch_millis: session.modified_epoch_millis,
            message_count: session.message_count,
            parent_session_id: session.parent_session_id,
            branch_name: session.branch_name,
            lifecycle: lifecycle.clone(),
        })
        .collect())
}

pub(crate) fn latest_managed_session() -> Result<ManagedSessionSummary, Box<dyn std::error::Error>>
{
    let store = current_session_store()?;
    let lifecycle = classify_session_lifecycle_for(store.workspace_root());
    let session = store
        .latest_session()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(ManagedSessionSummary {
        id: session.id,
        path: session.path,
        created_at_ms: session.created_at_ms,
        updated_at_ms: session.updated_at_ms,
        modified_epoch_millis: session.modified_epoch_millis,
        message_count: session.message_count,
        parent_session_id: session.parent_session_id,
        branch_name: session.branch_name,
        lifecycle,
    })
}

pub(crate) fn load_session_reference(
    reference: &str,
) -> Result<(SessionHandle, Session), Box<dyn std::error::Error>> {
    load_session_reference_excluding(reference, None)
}

pub(crate) fn delete_managed_session(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err(format!("session file does not exist: {}", path.display()).into());
    }
    fs::remove_file(path)?;
    Ok(())
}

pub(crate) fn session_exists_json(
    target: &str,
    active_session_id: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let handle = create_managed_session_handle(target)?;
    let resolved = resolve_session_reference(target).ok();
    let exists = resolved.is_some();
    let resolved_id = resolved
        .as_ref()
        .map_or(target, |handle| handle.id.as_str());
    Ok(serde_json::json!({
        "kind": "session_exists",
        "action": "exists",
        "status": "ok",
        "session_id": resolved_id,
        "session": target,
        "requested": target,
        "exists": exists,
        "active": resolved_id == active_session_id,
        "path": resolved
            .as_ref()
            .map(|handle| handle.path.display().to_string()),
        "candidate_path": handle.path.display().to_string(),
    }))
}

pub(crate) fn run_resumed_session_command(
    session_path: &Path,
    session: &Session,
    action: Option<&str>,
    target: Option<&str>,
) -> Result<ResumeCommandOutcome, Box<dyn std::error::Error>> {
    match action {
        None | Some("list") => {
            let sessions = list_managed_sessions().unwrap_or_default();
            let session_ids: Vec<String> = sessions.iter().map(|s| s.id.clone()).collect();
            let active_id = session.session_id.clone();
            let text = render_session_list(&active_id).unwrap_or_else(|e| format!("error: {e}"));
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(text),
                json: Some(serde_json::json!({
                    "kind": "sessions",
                    "status": "ok",
                    "action": "list",
                    "sessions": session_ids,
                    "session_details": session_details_json(&sessions),
                    "active": active_id,
                })),
            })
        }
        Some("exists") => {
            let Some(target) = target else {
                return Err("/session exists requires a session id.\nUsage: claw --resume <session> /session exists <session-id>".into());
            };
            let value = session_exists_json(target, &session.session_id)?;
            let exists = value
                .get("exists")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format!(
                    "Session exists\n  Session          {}\n  Exists           {}",
                    target,
                    if exists { "yes" } else { "no" }
                )),
                json: Some(value),
            })
        }
        Some("delete") => {
            let Some(target) = target else {
                return Err("/session delete requires a session id.\nUsage: claw --resume <session> /session delete <session-id> --force".into());
            };
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format!(
                    "delete: confirmation required; rerun with /session delete {target} --force"
                )),
                json: Some(serde_json::json!({
                    "kind": "error",
                    "error": "confirmation required",
                    "hint": format!("rerun with /session delete {target} --force"),
                    "session_id": target,
                })),
            })
        }
        Some("delete-force") => {
            let Some(target) = target else {
                return Err("/session delete requires a session id.\nUsage: claw --resume <session> /session delete <session-id> --force".into());
            };
            let handle = resolve_session_reference(target)?;
            if handle.id == session.session_id || handle.path == session_path {
                return Err(format!(
                    "delete: refusing to delete the active session '{}'. Resume or switch to another session first.",
                    handle.id
                )
                .into());
            }
            delete_managed_session(&handle.path)?;
            Ok(ResumeCommandOutcome {
                session: session.clone(),
                message: Some(format!(
                    "Session deleted\n  Deleted session  {}\n  File             {}",
                    handle.id,
                    handle.path.display(),
                )),
                json: Some(serde_json::json!({
                    "kind": "session_delete",
                    "action": "delete",
                    "status": "ok",
                    "deleted": true,
                    "session_id": handle.id,
                    "path": handle.path.display().to_string(),
                })),
            })
        }
        // #113: /session switch and /session fork require an interactive REPL —
        // return structured JSON instead of a raw error so resume callers can
        // detect the limitation programmatically.
        Some(switch_or_fork @ ("switch" | "fork")) => Ok(ResumeCommandOutcome {
            session: session.clone(),
            message: Some(format!(
                "/session {switch_or_fork} requires an interactive REPL.\nUsage: claw (then /session {switch_or_fork} <id>)"
            )),
            json: Some(serde_json::json!({
                "kind": "error",
                "error_kind": "unsupported_resumed_command",
                "status": "error",
                "action": switch_or_fork,
                "error": format!("/session {switch_or_fork} requires an interactive REPL"),
                "hint": format!("Start a new claw session and use /session {switch_or_fork} <id> interactively"),
            })),
        }),
        Some(other) => Err(format!("unsupported_resumed_command: /session {other} is not supported in resume mode.\nSupported: list, exists, delete").into()),
    }
}

pub(crate) fn render_session_list(
    active_session_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let sessions = list_managed_sessions()?;
    let mut lines = vec![
        "Sessions".to_string(),
        format!("  Directory         {}", sessions_dir()?.display()),
    ];
    if sessions.is_empty() {
        lines.push("  No managed sessions saved yet.".to_string());
        return Ok(lines.join("\n"));
    }
    for session in sessions {
        let marker = if session.id == active_session_id {
            "● current"
        } else {
            "○ saved"
        };
        let lineage = match (
            session.branch_name.as_deref(),
            session.parent_session_id.as_deref(),
        ) {
            (Some(branch_name), Some(parent_session_id)) => {
                format!(" branch={branch_name} from={parent_session_id}")
            }
            (None, Some(parent_session_id)) => format!(" from={parent_session_id}"),
            (Some(branch_name), None) => format!(" branch={branch_name}"),
            (None, None) => String::new(),
        };
        lines.push(format!(
            "  {id:<20} {marker:<10} lifecycle={lifecycle} msgs={msgs:<4} modified={modified}{lineage} path={path}",
            id = session.id,
            lifecycle = session.lifecycle.signal(),
            msgs = session.message_count,
            modified = format_session_modified_age(session.modified_epoch_millis),
            lineage = lineage,
            path = session.path.display(),
        ));
    }
    Ok(lines.join("\n"))
}

pub(crate) fn format_session_modified_age(modified_epoch_millis: u128) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map_or(modified_epoch_millis, |duration| duration.as_millis());
    let delta_seconds = now
        .saturating_sub(modified_epoch_millis)
        .checked_div(1_000)
        .unwrap_or_default();
    match delta_seconds {
        0..=4 => "just-now".to_string(),
        5..=59 => format!("{delta_seconds}s-ago"),
        60..=3_599 => format!("{}m-ago", delta_seconds / 60),
        3_600..=86_399 => format!("{}h-ago", delta_seconds / 3_600),
        _ => format!("{}d-ago", delta_seconds / 86_400),
    }
}

pub(crate) fn print_status_snapshot(
    model: &str,
    model_flag_raw: Option<&str>,
    permission_mode: PermissionModeProvenance,
    output_format: CliOutputFormat,
    allowed_tools: Option<&AllowedToolSet>,
) -> Result<(), Box<dyn std::error::Error>> {
    let usage = StatusUsage {
        message_count: 0,
        turns: 0,
        latest: TokenUsage::default(),
        cumulative: TokenUsage::default(),
        estimated_tokens: 0,
    };
    let context = status_context(None)?;
    // #148: resolve model provenance. If user passed --model, source is
    // "flag" with the raw input preserved. Otherwise probe env -> config
    // -> default and record the winning source.
    let provenance_result = match model_flag_raw {
        Some(raw) => Ok(ModelProvenance::from_flag(raw, model)),
        None => ModelProvenance::from_env_or_config_or_default(model),
    };
    let provenance = match provenance_result {
        Ok(provenance) => provenance,
        Err(error) => match output_format {
            CliOutputFormat::Json => {
                return print_model_validation_warning_status(
                    &error,
                    usage,
                    permission_mode.mode.as_str(),
                    &context,
                    allowed_tools,
                );
            }
            CliOutputFormat::Text => return Err(error.into()),
        },
    };
    let format_selection = current_output_format_selection();
    match output_format {
        CliOutputFormat::Text => println!(
            "{}",
            format_status_report(
                &provenance.resolved,
                usage,
                permission_mode.mode.as_str(),
                &context,
                Some(&provenance),
                Some(&permission_mode),
            )
        ),
        CliOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&status_json_value(
                Some(&provenance.resolved),
                usage,
                permission_mode.mode.as_str(),
                &context,
                Some(&provenance),
                Some(&permission_mode),
                allowed_tools,
                Some(&format_selection),
            ))?
        ),
    }
    Ok(())
}

pub(crate) fn print_sandbox_status_snapshot(
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let runtime_config = loader
        .load()
        .unwrap_or_else(|_| runtime::RuntimeConfig::empty());
    let status = resolve_sandbox_status(runtime_config.sandbox(), &cwd);
    match output_format {
        CliOutputFormat::Text => println!("{}", format_sandbox_report(&status)),
        CliOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&sandbox_json_value(&status))?
        ),
    }
    Ok(())
}

pub(crate) fn print_help_topic(
    topic: LocalHelpTopic,
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir().unwrap_or_default();
    // For subsystem topics in JSON mode, delegate to the subsystem's usage JSON.
    if output_format == CliOutputFormat::Json {
        match topic {
            LocalHelpTopic::Agents => {
                let json = commands::handle_agents_slash_command_json(Some("--help"), &cwd)
                    .unwrap_or_else(
                        |_| serde_json::json!({"kind":"agents","action":"help","status":"error"}),
                    );
                println!("{}", serde_json::to_string_pretty(&json)?);
                return Ok(());
            }
            LocalHelpTopic::Skills => {
                let json = commands::handle_skills_slash_command_json(Some("--help"), &cwd)
                    .unwrap_or_else(
                        |_| serde_json::json!({"kind":"skills","action":"help","status":"error"}),
                    );
                println!("{}", serde_json::to_string_pretty(&json)?);
                return Ok(());
            }
            _ => {}
        }
    }
    match output_format {
        CliOutputFormat::Text => println!("{}", render_help_topic(topic)),
        CliOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&render_help_topic_json(topic))?
        ),
    }
    Ok(())
}

pub(crate) fn print_acp_status(
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match output_format {
        CliOutputFormat::Text => {
            println!(
                "ACP / Zed\n  Status           not implemented\n  Launch           `claw acp serve` reports status only; no editor daemon or JSON-RPC endpoint is available yet\n  Today            use `claw prompt`, the REPL, or `claw doctor` for local verification\n  Message          {}",
                acp_status_message()
            );
        }
        CliOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&acp_status_json())?);
        }
    }
    Ok(())
}

pub(crate) fn render_memory_json() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let project_context = ProjectContext::discover(&cwd, DEFAULT_DATE)?;
    let files: Vec<_> = project_context
        .instruction_files
        .iter()
        .map(|f| {
            json!({
                "path": f.path.display().to_string(),
                "lines": f.content.lines().count(),
                "preview": f.content.lines().next().unwrap_or("").trim(),
            })
        })
        .collect();
    Ok(json!({
        "kind": "memory",
        "action": "list",
        "status": "ok",
        "cwd": cwd.display().to_string(),
        "instruction_files": files.len(),
        "files": files,
    }))
}

pub(crate) fn init_claude_md() -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    Ok(initialize_repo(&cwd)?.render())
}

pub(crate) fn run_init(output_format: CliOutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let report = initialize_repo(&cwd)?;
    let message = report.render();
    match output_format {
        CliOutputFormat::Text => println!("{message}"),
        CliOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&init_json_value(&report, &message))?
        ),
    }
    Ok(())
}

pub(crate) fn render_diff_report() -> Result<String, Box<dyn std::error::Error>> {
    render_diff_report_for(&env::current_dir()?)
}

pub(crate) fn render_diff_json_for(
    cwd: &Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let in_git_repo = std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(cwd)
        .output()
        .is_ok_and(|o| o.status.success());
    if !in_git_repo {
        // #801: add error_kind, hint, message fields for envelope parity with other error paths
        return Ok(serde_json::json!({
            "kind": "diff",
            "action": "diff",
            "status": "error",
            "error_kind": "no_git_repo",
            "result": "no_git_repo",
            "message": format!("{} is not inside a git project", cwd.display()),
            "hint": "Run `git init` to create a repository, or change to a directory that is inside a git project.",
            "working_directory": cwd.display().to_string(),
            "detail": format!("{} is not inside a git project", cwd.display()),
        }));
    }
    let staged = run_git_diff_command_in(cwd, &["diff", "--cached"])?;
    let unstaged = run_git_diff_command_in(cwd, &["diff"])?;
    // #733: add changed_file_count so callers don't have to count diff hunks
    let staged_files =
        run_git_diff_command_in(cwd, &["diff", "--cached", "--name-only"]).unwrap_or_default();
    let unstaged_files = run_git_diff_command_in(cwd, &["diff", "--name-only"]).unwrap_or_default();
    let mut changed: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for line in staged_files.lines().chain(unstaged_files.lines()) {
        let t = line.trim();
        if !t.is_empty() {
            changed.insert(t);
        }
    }
    let changed_file_count = changed.len();
    Ok(serde_json::json!({
        "kind": "diff",
        "action": "diff",
        "status": "ok",
        "working_directory": cwd.display().to_string(),
        "result": if staged.trim().is_empty() && unstaged.trim().is_empty() { "clean" } else { "changes" },
        "changed_file_count": changed_file_count,
        "staged": staged.trim(),
        "unstaged": unstaged.trim(),
    }))
}

pub(crate) fn run_git_diff_command_in(
    cwd: &Path,
    args: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git {} failed: {stderr}", args.join(" ")).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

pub(crate) fn render_teleport_report(target: &str) -> Result<String, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;

    let file_list = Command::new("rg")
        .args(["--files"])
        .current_dir(&cwd)
        .output()?;
    let file_matches = if file_list.status.success() {
        String::from_utf8(file_list.stdout)?
            .lines()
            .filter(|line| line.contains(target))
            .take(10)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let content_output = Command::new("rg")
        .args(["-n", "-S", "--color", "never", target, "."])
        .current_dir(&cwd)
        .output()?;

    let mut lines = vec![
        "Teleport".to_string(),
        format!("  Target           {target}"),
        "  Action           search workspace files and content for the target".to_string(),
    ];
    if !file_matches.is_empty() {
        lines.push(String::new());
        lines.push("File matches".to_string());
        lines.extend(file_matches.into_iter().map(|path| format!("  {path}")));
    }

    if content_output.status.success() {
        let matches = String::from_utf8(content_output.stdout)?;
        if !matches.trim().is_empty() {
            lines.push(String::new());
            lines.push("Content matches".to_string());
            lines.push(truncate_for_prompt(&matches, 4_000));
        }
    }

    if lines.len() == 1 {
        lines.push("  Result           no matches found".to_string());
    }

    Ok(lines.join("\n"))
}

pub(crate) fn render_last_tool_debug_report(
    session: &Session,
) -> Result<String, Box<dyn std::error::Error>> {
    let last_tool_use = session
        .messages
        .iter()
        .rev()
        .find_map(|message| {
            message.blocks.iter().rev().find_map(|block| match block {
                ContentBlock::ToolUse { id, name, input } => {
                    Some((id.clone(), name.clone(), input.clone()))
                }
                _ => None,
            })
        })
        .ok_or_else(|| "no prior tool call found in session".to_string())?;

    let tool_result = session.messages.iter().rev().find_map(|message| {
        message.blocks.iter().rev().find_map(|block| match block {
            ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } if tool_use_id == &last_tool_use.0 => {
                Some((tool_name.clone(), output.clone(), *is_error))
            }
            _ => None,
        })
    });

    let mut lines = vec![
        "Debug tool call".to_string(),
        "  Action           inspect the last recorded tool call and its result".to_string(),
        format!("  Tool id          {}", last_tool_use.0),
        format!("  Tool name        {}", last_tool_use.1),
        "  Input".to_string(),
        indent_block(&last_tool_use.2, 4),
    ];

    match tool_result {
        Some((tool_name, output, is_error)) => {
            lines.push("  Result".to_string());
            lines.push(format!("    name           {tool_name}"));
            lines.push(format!(
                "    status         {}",
                if is_error { "error" } else { "ok" }
            ));
            lines.push(indent_block(&output, 4));
        }
        None => lines.push("  Result           missing tool result".to_string()),
    }

    Ok(lines.join("\n"))
}

pub(crate) fn validate_no_args(
    command_name: &str,
    args: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(args) = args.map(str::trim).filter(|value| !value.is_empty()) {
        return Err(format!(
            "{command_name} does not accept arguments. Received: {args}\nUsage: {command_name}"
        )
        .into());
    }
    Ok(())
}

pub(crate) fn git_output(args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(env::current_dir()?)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git {} failed: {stderr}", args.join(" ")).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

pub(crate) fn git_status_ok(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(env::current_dir()?)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("git {} failed: {stderr}", args.join(" ")).into());
    }
    Ok(())
}

pub(crate) fn write_temp_text_file(
    filename: &str,
    contents: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = env::temp_dir().join(filename);
    fs::write(&path, contents)?;
    Ok(path)
}

pub(crate) fn recent_user_context(session: &Session, limit: usize) -> String {
    let requests = session
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::User)
        .filter_map(|message| {
            message.blocks.iter().find_map(|block| match block {
                ContentBlock::Text { text } => Some(text.trim().to_string()),
                _ => None,
            })
        })
        .rev()
        .take(limit)
        .collect::<Vec<_>>();

    if requests.is_empty() {
        "<no prior user messages>".to_string()
    } else {
        requests
            .into_iter()
            .rev()
            .enumerate()
            .map(|(index, text)| format!("{}. {}", index + 1, text))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub(crate) fn resolve_export_path(
    requested_path: Option<&str>,
    session: &Session,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let file_name =
        requested_path.map_or_else(|| default_export_filename(session), ToOwned::to_owned);
    let final_name = if Path::new(&file_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
    {
        file_name
    } else {
        format!("{file_name}.txt")
    };
    Ok(cwd.join(final_name))
}

pub(crate) fn run_export(
    session_reference: &str,
    output_path: Option<&Path>,
    output_format: CliOutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_export_output_path(output_path)?;
    let (handle, session) = load_session_reference(session_reference)?;
    let markdown = render_session_markdown(&session, &handle.id, &handle.path);

    if let Some(path) = output_path {
        fs::write(path, &markdown)?;
        let report = format!(
            "Export\n  Result           wrote markdown transcript\n  File             {}\n  Session          {}\n  Messages         {}",
            path.display(),
            handle.id,
            session.messages.len(),
        );
        match output_format {
            CliOutputFormat::Text => println!("{report}"),
            CliOutputFormat::Json => println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "kind": "export",
                    "action": "export",
                    "status": "ok",
                    "message": report,
                    "session_id": handle.id,
                    "file": path.display().to_string(),
                    "messages": session.messages.len(),
                }))?
            ),
        }
        return Ok(());
    }

    match output_format {
        CliOutputFormat::Text => {
            print!("{markdown}");
            if !markdown.ends_with('\n') {
                println!();
            }
        }
        CliOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "kind": "export",
                "action": "export",
                "status": "ok",
                "session_id": handle.id,
                "file": handle.path.display().to_string(),
                "messages": session.messages.len(),
                "markdown": markdown,
            }))?
        ),
    }
    Ok(())
}

pub(crate) fn build_system_prompt(model: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(load_system_prompt(
        env::current_dir()?,
        DEFAULT_DATE,
        env::consts::OS,
        "unknown",
        model_family_identity_for(model),
    )?)
}

pub(crate) fn build_runtime_plugin_state() -> Result<RuntimePluginState, Box<dyn std::error::Error>>
{
    let cwd = env::current_dir()?;
    let loader = ConfigLoader::default_for(&cwd);
    let runtime_config = loader.load()?;
    build_runtime_plugin_state_with_loader(&cwd, &loader, &runtime_config)
}

#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_runtime(
    session: Session,
    session_id: &str,
    model: String,
    system_prompt: Vec<String>,
    enable_tools: bool,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    permission_mode: PermissionMode,
    progress_reporter: Option<InternalPromptProgressReporter>,
) -> Result<BuiltRuntime, Box<dyn std::error::Error>> {
    let runtime_plugin_state = build_runtime_plugin_state()?;
    build_runtime_with_plugin_state(
        session,
        session_id,
        model,
        system_prompt,
        enable_tools,
        emit_output,
        allowed_tools,
        permission_mode,
        progress_reporter,
        runtime_plugin_state,
    )
}

pub(crate) fn resolve_cli_auth_source() -> Result<AuthSource, Box<dyn std::error::Error>> {
    Ok(resolve_cli_auth_source_for_cwd()?)
}

pub(crate) fn final_assistant_text(summary: &runtime::TurnSummary) -> String {
    summary
        .assistant_messages
        .last()
        .map(|message| {
            message
                .blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

pub(crate) fn print_help(output_format: CliOutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = Vec::new();
    print_help_to(&mut buffer)?;
    let message = String::from_utf8(buffer)?;
    match output_format {
        CliOutputFormat::Text => print!("{message}"),
        CliOutputFormat::Json => {
            // #325: include structured command list in top-level help JSON
            let commands: Vec<serde_json::Value> = commands::slash_command_specs()
                .iter()
                .map(|spec| {
                    serde_json::json!({
                        "name": spec.name,
                        "summary": spec.summary,
                        "resume_supported": spec.resume_supported,
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "kind": "help",
                    "action": "help",
                    "status": "ok",
                    "message": message,
                    "commands": commands,
                    "total_commands": commands.len(),
                }))?
            );
        }
    }
    Ok(())
}
