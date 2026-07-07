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

/// #148: Model provenance for `claw status` JSON/text output. Records where
/// the resolved model string came from so claws don't have to re-read argv
/// to audit whether their `--model` flag was honored vs falling back to env
/// or config or default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModelSource {
    /// Explicit `--model` / `--model=` CLI flag.
    Flag,
    /// Runtime model environment variable (when no flag was passed).
    Env,
    /// `model` key in `.claw.json` / `.claw/settings.json` (when neither
    /// flag nor env set it).
    Config,
    /// Compiled-in `DEFAULT_MODEL` fallback.
    Default,
}

impl ModelSource {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ModelSource::Flag => "flag",
            ModelSource::Env => "env",
            ModelSource::Config => "config",
            ModelSource::Default => "default",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelProvenance {
    /// Resolved model string (after alias expansion).
    pub(crate) resolved: String,
    /// Raw user input before alias resolution. None when source is Default.
    pub(crate) raw: Option<String>,
    /// Where the resolved model string originated.
    pub(crate) source: ModelSource,
    /// Alias-expanded target when `raw` differs from `resolved`.
    pub(crate) alias_resolved_to: Option<String>,
    /// Environment variable that supplied the model, when source is Env.
    pub(crate) env_var: Option<String>,
}

impl ModelProvenance {
    pub(crate) fn default_fallback() -> Self {
        Self {
            resolved: DEFAULT_MODEL.to_string(),
            raw: None,
            source: ModelSource::Default,
            alias_resolved_to: None,
            env_var: None,
        }
    }

    pub(crate) fn from_flag(raw: &str, resolved: &str) -> Self {
        Self::from_resolved(raw, resolved, ModelSource::Flag, None)
    }

    pub(crate) fn from_raw(raw: &str, source: ModelSource, env_var: Option<&str>) -> Self {
        let resolved = resolve_model_alias_with_config(raw);
        Self::from_resolved(raw, &resolved, source, env_var)
    }

    pub(crate) fn from_resolved(
        raw: &str,
        resolved: &str,
        source: ModelSource,
        env_var: Option<&str>,
    ) -> Self {
        let raw_trimmed = raw.trim();
        let alias_resolved_to = (raw_trimmed != resolved).then(|| resolved.to_string());
        Self {
            resolved: resolved.to_string(),
            raw: Some(raw.to_string()),
            source,
            alias_resolved_to,
            env_var: env_var.map(str::to_string),
        }
    }

    pub(crate) fn from_env_or_config_or_default(cli_model: &str) -> Result<Self, String> {
        // Only called when no --model flag was passed. Probe env first,
        // then config, else fall back to default. Mirrors the logic in
        // resolve_repl_model() but captures the source.
        if cli_model != DEFAULT_MODEL {
            let provenance = Self::from_resolved(cli_model, cli_model, ModelSource::Flag, None);
            provenance.validate()?;
            return Ok(provenance);
        }
        if let Some(env_model) = env_model_for_runtime() {
            let provenance =
                Self::from_raw(&env_model.value, ModelSource::Env, Some(env_model.name));
            provenance.validate()?;
            return Ok(provenance);
        }
        if let Some(config_model) = config_model_for_current_dir() {
            let provenance = Self::from_raw(&config_model, ModelSource::Config, None);
            provenance.validate()?;
            return Ok(provenance);
        }
        Ok(Self::default_fallback())
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        validate_model_syntax(&self.resolved).map_err(|error| {
            let source = match self.source {
                ModelSource::Flag => "--model",
                ModelSource::Env => self.env_var.as_deref().unwrap_or("environment"),
                ModelSource::Config => "config model",
                ModelSource::Default => "default model",
            };
            if let Some(raw) = &self.raw {
                format!(
                    "invalid_model: {source} model `{raw}` is invalid after alias resolution to `{}`.\n{error}",
                    self.resolved
                )
            } else {
                error
            }
        })
    }
}

pub(crate) fn resolve_model_alias(model: &str) -> &str {
    match model {
        "opus" => "anthropic/claude-opus-4-7",
        "sonnet" => "anthropic/claude-sonnet-4-6",
        "haiku" => "anthropic/claude-haiku-4-5-20251213",
        _ => model,
    }
}

/// Resolve a model name through user-defined config aliases first, then fall
/// back to the built-in alias table. This is the entry point used wherever a
/// user-supplied model string is about to be dispatched to a provider.
pub(crate) fn resolve_model_alias_with_config(model: &str) -> String {
    let trimmed = model.trim();
    if let Some(resolved) = config_alias_for_current_dir(trimmed) {
        return resolve_model_alias(&resolved).to_string();
    }
    resolve_model_alias(trimmed).to_string()
}

pub(crate) fn config_alias_for_current_dir(alias: &str) -> Option<String> {
    if alias.is_empty() {
        return None;
    }
    let cwd = env::current_dir().ok()?;
    let loader = ConfigLoader::default_for(&cwd);
    let config = loader.load().ok()?;
    config.aliases().get(alias).cloned()
}

pub(crate) fn config_model_for_current_dir() -> Option<String> {
    let cwd = env::current_dir().ok()?;
    let loader = ConfigLoader::default_for(&cwd);
    loader.load().ok()?.model().map(ToOwned::to_owned)
}

pub(crate) fn resolve_repl_model(cli_model: String) -> Result<String, String> {
    Ok(ModelProvenance::from_env_or_config_or_default(&cli_model)?.resolved)
}
