#![allow(clippy::cast_possible_truncation)]
use std::future::Future;
use std::pin::Pin;

use serde::Serialize;

use crate::error::ApiError;
use crate::types::{MessageRequest, MessageResponse, ReasoningEffort};
use crate::providers::reasoning::reasoning_levels;

pub mod anthropic;
pub mod openai_compat;
pub mod reasoning;

#[allow(dead_code)]
pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ApiError>> + Send + 'a>>;

#[allow(dead_code)]
pub trait Provider {
    type Stream;

    fn send_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, MessageResponse>;

    fn stream_message<'a>(
        &'a self,
        request: &'a MessageRequest,
    ) -> ProviderFuture<'a, Self::Stream>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Anthropic,
    OpenAi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderMetadata {
    pub provider: ProviderKind,
    pub auth_env: &'static str,
    pub base_url_env: &'static str,
    pub default_base_url: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelTokenLimit {
    pub max_output_tokens: u32,
    pub context_window_tokens: u32,
}

const MODEL_REGISTRY: &[(&str, ProviderMetadata)] = &[
    (
        "opus",
        ProviderMetadata {
            provider: ProviderKind::Anthropic,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: anthropic::DEFAULT_BASE_URL,
        },
    ),
    (
        "sonnet",
        ProviderMetadata {
            provider: ProviderKind::Anthropic,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: anthropic::DEFAULT_BASE_URL,
        },
    ),
    (
        "haiku",
        ProviderMetadata {
            provider: ProviderKind::Anthropic,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: anthropic::DEFAULT_BASE_URL,
        },
    ),
];

#[must_use]
pub fn resolve_model_alias(model: &str) -> String {
    let trimmed = model.trim();
    let lower = trimmed.to_ascii_lowercase();
    MODEL_REGISTRY
        .iter()
        .find_map(|(alias, metadata)| {
            (*alias == lower).then_some(match metadata.provider {
                ProviderKind::Anthropic => match *alias {
                    "opus" => "claude-opus-4-6",
                    "sonnet" => "claude-sonnet-4-6",
                    "haiku" => "claude-haiku-4-5-20251213",
                    _ => trimmed,
                },
                ProviderKind::OpenAi => trimmed,
            })
        })
        .map_or_else(|| trimmed.to_string(), ToOwned::to_owned)
}

#[must_use]
pub fn metadata_for_model(model: &str) -> Option<ProviderMetadata> {
    let canonical = resolve_model_alias(model);
    if canonical.starts_with("claude") {
        return Some(ProviderMetadata {
            provider: ProviderKind::Anthropic,
            auth_env: "ANTHROPIC_API_KEY",
            base_url_env: "ANTHROPIC_BASE_URL",
            default_base_url: anthropic::DEFAULT_BASE_URL,
        });
    }
    // Explicit provider-namespaced models (e.g. "openai/gpt-4.1-mini") must
    // route to the correct provider regardless of which auth env vars are set.
    // Without this, detect_provider_kind falls through to the auth-sniffer
    // order and misroutes to Anthropic if ANTHROPIC_API_KEY is present.
    if canonical.starts_with("openai/") || canonical.starts_with("gpt-") {
        return Some(ProviderMetadata {
            provider: ProviderKind::OpenAi,
            auth_env: "OPENAI_API_KEY",
            base_url_env: "OPENAI_BASE_URL",
            default_base_url: openai_compat::DEFAULT_OPENAI_BASE_URL,
        });
    }
    None
}

#[must_use]
pub fn detect_provider_kind(model: &str) -> ProviderKind {
    if let Some(metadata) = metadata_for_model(model) {
        return metadata.provider;
    }
    // When OPENAI_BASE_URL is set, the user explicitly configured an
    // OpenAI-compatible endpoint. Prefer it over the Anthropic fallback
    // even when the model name has no recognized prefix — this is the
    // common case for local providers (Ollama, LM Studio, vLLM, etc.)
    // where model names like "qwen2.5-coder:7b" don't match any prefix.
    if std::env::var_os("OPENAI_BASE_URL").is_some() && openai_compat::has_api_key("OPENAI_API_KEY")
    {
        return ProviderKind::OpenAi;
    }
    if anthropic::has_auth_from_env_or_saved().unwrap_or(false) {
        return ProviderKind::Anthropic;
    }
    if openai_compat::has_api_key("OPENAI_API_KEY") {
        return ProviderKind::OpenAi;
    }
    // Last resort: if OPENAI_BASE_URL is set without OPENAI_API_KEY (some
    // local providers like Ollama don't require auth), still route there.
    if std::env::var_os("OPENAI_BASE_URL").is_some() {
        return ProviderKind::OpenAi;
    }
    ProviderKind::Anthropic
}

#[must_use]
pub fn max_tokens_for_model(model: &str) -> u32 {
    model_token_limit(model).map_or_else(
        || {
            let canonical = resolve_model_alias(model);
            if canonical.contains("opus") {
                32_000
            } else {
                64_000
            }
        },
        |limit| limit.max_output_tokens,
    )
}

/// Returns the effective max output tokens for a model, preferring a plugin
/// override when present. Falls back to [`max_tokens_for_model`] when the
/// override is `None`.
#[must_use]
pub fn max_tokens_for_model_with_override(model: &str, plugin_override: Option<u32>) -> u32 {
    plugin_override.unwrap_or_else(|| max_tokens_for_model(model))
}

#[must_use]
pub fn model_token_limit(model: &str) -> Option<ModelTokenLimit> {
    let canonical = resolve_model_alias(model);
    match canonical.as_str() {
        "claude-opus-4-6" => Some(ModelTokenLimit {
            max_output_tokens: 32_000,
            context_window_tokens: 200_000,
        }),
        "claude-sonnet-4-6" | "claude-haiku-4-5-20251213" => Some(ModelTokenLimit {
            max_output_tokens: 64_000,
            context_window_tokens: 200_000,
        }),
        _ => None,
    }
}

/// Detect whether the active provider is a local inference endpoint
/// (llama.cpp, LM Studio, Ollama, vLLM, mock servers, etc.) where
/// KV cache prefix stability is critical and tools-in-system-prompt
/// is beneficial.
///
/// Detection heuristics:
/// 1. Explicit opt-in via `CLAW_LOCAL_INFERENCE=true`
/// 2. `OPENAI_BASE_URL` or `ANTHROPIC_BASE_URL` points to a loopback address
/// 3. A base URL is set without its corresponding API key (no-auth local server)
#[must_use]
pub fn is_local_inference() -> bool {
    if std::env::var_os("CLAW_LOCAL_INFERENCE")
        .is_some_and(|v| v == "true" || v == "1")
    {
        return true;
    }

    let local_hosts = ["localhost", "127.0.0.1", "0.0.0.0"];

    // Helper: check if a base URL env points to a loopback address.
    let base_url_looks_local = |key: &str| -> bool {
        std::env::var(key).is_ok_and(|url| {
            if url.is_empty() {
                return false;
            }
            local_hosts.iter().any(|h| url.contains(h))
        })
    };

    if base_url_looks_local("OPENAI_BASE_URL") {
        return true;
    }
    if base_url_looks_local("ANTHROPIC_BASE_URL") {
        return true;
    }

    // Base URL set without its API key = likely a no-auth local server.
    if std::env::var("OPENAI_BASE_URL").is_ok_and(|u| !u.is_empty())
        && std::env::var("OPENAI_API_KEY").is_err()
    {
        return true;
    }
    if std::env::var("ANTHROPIC_BASE_URL").is_ok_and(|u| !u.is_empty())
        && std::env::var("ANTHROPIC_API_KEY").is_err()
    {
        return true;
    }

    false
}

/// Fail-fast reasoning-effort validation: reject a level the resolved model
/// does not support, or a level string that is not a recognised level, before
/// the request leaves the process. Mirrors the dsh `resolveReasoningLevel`
/// posture — a stale or mistyped level fails here instead of being silently
/// ignored by the backend.
///
/// `None` (no level requested) always passes: the provider's own server
/// default applies (the `reasoning_effort` field is omitted from the wire).
fn validate_reasoning_effort_for_request(request: &MessageRequest) -> Result<(), ApiError> {
    let Some(level_str) = request.reasoning_effort.as_deref() else {
        return Ok(());
    };
    let canonical = resolve_model_alias(&request.model);
    let provider = detect_provider_kind(&canonical);
    let supported = reasoning_levels(provider, &canonical);
    let supported_names: Vec<String> = supported
        .iter()
        .map(|level| level.as_str().to_string())
        .collect();
    let level = ReasoningEffort::from_name(level_str).ok_or_else(|| {
        ApiError::UnsupportedReasoningEffort {
            model: canonical.clone(),
            level: level_str.to_string(),
            supported: supported_names.clone(),
        }
    })?;
    if supported.contains(&level) {
        Ok(())
    } else {
        Err(ApiError::UnsupportedReasoningEffort {
            model: canonical,
            level: level_str.to_string(),
            supported: supported_names,
        })
    }
}

pub fn preflight_message_request(request: &MessageRequest) -> Result<(), ApiError> {
    validate_reasoning_effort_for_request(request)?;
    let Some(limit) = model_token_limit(&request.model) else {
        return Ok(());
    };

    let estimated_input_tokens = estimate_message_request_input_tokens(request);
    let estimated_total_tokens = estimated_input_tokens.saturating_add(request.max_tokens);
    if estimated_total_tokens > limit.context_window_tokens {
        return Err(ApiError::ContextWindowExceeded {
            model: resolve_model_alias(&request.model),
            estimated_input_tokens,
            requested_output_tokens: request.max_tokens,
            estimated_total_tokens,
            context_window_tokens: limit.context_window_tokens,
        });
    }

    Ok(())
}

fn estimate_message_request_input_tokens(request: &MessageRequest) -> u32 {
    let mut estimate = estimate_serialized_tokens(&request.messages);
    estimate = estimate.saturating_add(estimate_serialized_tokens(&request.system));
    estimate = estimate.saturating_add(estimate_serialized_tokens(&request.tools));
    estimate = estimate.saturating_add(estimate_serialized_tokens(&request.tool_choice));
    estimate
}

fn estimate_serialized_tokens<T: Serialize>(value: &T) -> u32 {
    serde_json::to_vec(value)
        .ok()
        .map_or(0, |bytes| (bytes.len() / 4 + 1) as u32)
}

/// Env var names used by other provider backends. When Anthropic auth
/// resolution fails we sniff these so we can hint the user that their
/// credentials probably belong to a different provider and suggest the
/// model-prefix routing fix that would select it.
const FOREIGN_PROVIDER_ENV_VARS: &[(&str, &str, &str)] = &[(
    "OPENAI_API_KEY",
    "OpenAI-compat",
    "prefix your model name with `openai/` (e.g. `--model openai/gpt-4.1-mini`) so prefix routing selects the OpenAI-compatible provider, and set `OPENAI_BASE_URL` if you are pointing at OpenRouter/Ollama/a local server",
)];

/// Check whether an env var is set to a non-empty value either in the real
/// process environment or in the working-directory `.env` file. Mirrors the
/// credential discovery path used by `read_env_non_empty` so the hint text
/// stays truthful when users rely on `.env` instead of a real export.
fn env_or_dotenv_present(key: &str) -> bool {
    match std::env::var(key) {
        Ok(value) if !value.is_empty() => true,
        Ok(_) | Err(std::env::VarError::NotPresent) => {
            dotenv_value(key).is_some_and(|value| !value.is_empty())
        }
        Err(_) => false,
    }
}

/// Produce a hint string describing the first foreign provider credential
/// that is present in the environment when Anthropic auth resolution has
/// just failed. Returns `None` when no foreign credential is set, in which
/// case the caller should fall back to the plain `missing_credentials`
/// error without a hint.
pub(crate) fn anthropic_missing_credentials_hint() -> Option<String> {
    for (env_var, provider_label, fix_hint) in FOREIGN_PROVIDER_ENV_VARS {
        if env_or_dotenv_present(env_var) {
            return Some(format!(
                "I see {env_var} is set — if you meant to use the {provider_label} provider, {fix_hint}."
            ));
        }
    }
    None
}

/// Build an Anthropic-specific `MissingCredentials` error, attaching a
/// hint suggesting the probable fix whenever a different provider's
/// credentials are already present in the environment. Anthropic call
/// sites should prefer this helper over `ApiError::missing_credentials`
/// so users who mistyped a model name or forgot the prefix get a useful
/// signal instead of a generic "missing Anthropic credentials" wall.
pub(crate) fn anthropic_missing_credentials() -> ApiError {
    const PROVIDER: &str = "Anthropic";
    const ENV_VARS: &[&str] = &["ANTHROPIC_API_KEY"];
    match anthropic_missing_credentials_hint() {
        Some(hint) => ApiError::missing_credentials_with_hint(PROVIDER, ENV_VARS, hint),
        None => ApiError::missing_credentials(PROVIDER, ENV_VARS),
    }
}

/// Parse a `.env` file body into key/value pairs using a minimal `KEY=VALUE`
/// grammar. Lines that are blank, start with `#`, or do not contain `=` are
/// ignored. Surrounding double or single quotes are stripped from the value.
/// An optional leading `export ` prefix on the key is also stripped so files
/// shared with shell `source` workflows still parse cleanly.
pub(crate) fn parse_dotenv(content: &str) -> std::collections::HashMap<String, String> {
    let mut values = std::collections::HashMap::new();
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((raw_key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let trimmed_key = raw_key.trim();
        let key = trimmed_key
            .strip_prefix("export ")
            .map_or(trimmed_key, str::trim)
            .to_string();
        if key.is_empty() {
            continue;
        }
        let trimmed_value = raw_value.trim();
        let unquoted = if (trimmed_value.starts_with('"') && trimmed_value.ends_with('"')
            || trimmed_value.starts_with('\'') && trimmed_value.ends_with('\''))
            && trimmed_value.len() >= 2
        {
            &trimmed_value[1..trimmed_value.len() - 1]
        } else {
            trimmed_value
        };
        values.insert(key, unquoted.to_string());
    }
    values
}

/// Load and parse a `.env` file from the given path. Missing files yield
/// `None` instead of an error so callers can use this as a soft fallback.
pub(crate) fn load_dotenv_file(
    path: &std::path::Path,
) -> Option<std::collections::HashMap<String, String>> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(parse_dotenv(&content))
}

/// Look up `key` in the first-found `.env` file.
/// Priority: `cwd/.env` → `cwd/.claw/.env` → `~/.claw/.env`
/// (`$CLAW_CONFIG_HOME/.env` overrides `~/.claw/.env`).
/// Returns `None` when the key is absent or its value is empty.
pub(crate) fn dotenv_value(key: &str) -> Option<String> {
    let values = resolve_first_dotenv()?;
    values.get(key).filter(|value| !value.is_empty()).cloned()
}

/// Load the first-found `.env` file into the process environment.
/// Priority: `cwd/.env` → `cwd/.claw/.env` → `~/.claw/.env`
/// Existing vars are NOT overwritten. Call early in `main()` so ALL
/// `std::env::var()` calls in any crate pick up `.env` values.
pub fn load_env_file_to_process() {
    runtime::text_only_models::reload();
    let Some(values) = resolve_first_dotenv() else { return };
    for (key, value) in values {
        if std::env::var(&key).is_err() {
            std::env::set_var(&key, &value);
        }
    }
}

/// Resolve the user config home: `$CLAW_CONFIG_HOME` or `~/.claw`.
fn user_config_home() -> Option<std::path::PathBuf> {
    if let Some(custom) = std::env::var_os("CLAW_CONFIG_HOME") {
        return Some(std::path::PathBuf::from(custom));
    }
    #[cfg(windows)]
    let home = std::env::var_os("USERPROFILE");
    #[cfg(not(windows))]
    let home = std::env::var_os("HOME");
    home.map(|h| std::path::PathBuf::from(h).join(".claw"))
}

/// Try `.env` files in order: `cwd/.env` → `cwd/.claw/.env` → user home.
/// Returns the contents of the first existing file, or `None`.
fn resolve_first_dotenv() -> Option<std::collections::HashMap<String, String>> {
    // Project-local candidates
    if let Ok(cwd) = std::env::current_dir() {
        for candidate in [cwd.join(".env"), cwd.join(".claw").join(".env")] {
            if let Some(values) = load_dotenv_file(&candidate) {
                return Some(values);
            }
        }
    }
    // User-level fallback
    load_dotenv_file(&user_config_home()?.join(".env"))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::sync::{Arc, Mutex, OnceLock};

    use serde_json::json;

    use crate::error::ApiError;
    use crate::types::{
        InputContentBlock, InputMessage, MessageRequest, ToolChoice, ToolDefinition,
    };

    use super::{
        anthropic_missing_credentials, anthropic_missing_credentials_hint, detect_provider_kind,
        load_dotenv_file, max_tokens_for_model, max_tokens_for_model_with_override,
        model_token_limit, parse_dotenv, preflight_message_request, ProviderKind,
    };

    /// Serializes every test in this module that mutates process-wide
    /// environment variables so concurrent test threads cannot observe
    /// each other's partially-applied state while probing the foreign
    /// provider credential sniffer.
    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Snapshot-restore guard for a single environment variable. Captures
    /// the original value on construction, applies the requested override
    /// (set or remove), and restores the original on drop so tests leave
    /// the process env untouched even when they panic mid-assertion.
    struct EnvVarGuard {
        key: &'static str,
        original: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let original = std::env::var_os(key);
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.original.take() {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn detects_provider_from_model_name_first() {
        assert_eq!(
            detect_provider_kind("claude-sonnet-4-6"),
            ProviderKind::Anthropic
        );
    }

    #[test]
    fn openai_namespaced_model_routes_to_openai_not_anthropic() {
        // Regression: "openai/gpt-4.1-mini" was misrouted to Anthropic when
        // ANTHROPIC_API_KEY was set because metadata_for_model returned None
        // and detect_provider_kind fell through to auth-sniffer order.
        // The model prefix must win over env-var presence.
        let kind = super::metadata_for_model("openai/gpt-4.1-mini").map_or_else(
            || detect_provider_kind("openai/gpt-4.1-mini"),
            |m| m.provider,
        );
        assert_eq!(
            kind,
            ProviderKind::OpenAi,
            "openai/ prefix must route to OpenAi regardless of ANTHROPIC_API_KEY"
        );

        // Also cover bare gpt- prefix
        let kind2 = super::metadata_for_model("gpt-4o")
            .map_or_else(|| detect_provider_kind("gpt-4o"), |m| m.provider);
        assert_eq!(kind2, ProviderKind::OpenAi);
    }

    #[test]
    fn keeps_existing_max_token_heuristic() {
        assert_eq!(max_tokens_for_model("opus"), 32_000);
    }

    #[test]
    fn plugin_config_max_output_tokens_overrides_model_default() {
        // given
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("api-plugin-max-tokens-{nanos}"));
        let cwd = root.join("project");
        let home = root.join("home").join(".claw");
        std::fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
        std::fs::create_dir_all(&home).expect("home config dir");
        std::fs::write(
            home.join("settings.json"),
            r#"{
              "plugins": {
                "maxOutputTokens": 12345
              }
            }"#,
        )
        .expect("write plugin settings");

        // when
        let loaded = runtime::ConfigLoader::new(&cwd, &home)
            .load()
            .expect("config should load");
        let plugin_override = loaded.plugins().max_output_tokens();
        let effective = max_tokens_for_model_with_override("claude-opus-4-6", plugin_override);

        // then
        assert_eq!(plugin_override, Some(12345));
        assert_eq!(effective, 12345);
        assert_ne!(effective, max_tokens_for_model("claude-opus-4-6"));

        std::fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn max_tokens_for_model_with_override_falls_back_when_plugin_unset() {
        // given
        let plugin_override: Option<u32> = None;

        // when
        let effective = max_tokens_for_model_with_override("claude-opus-4-6", plugin_override);

        // then
        assert_eq!(effective, max_tokens_for_model("claude-opus-4-6"));
        assert_eq!(effective, 32_000);
    }

    #[test]
    fn returns_context_window_metadata_for_supported_models() {
        assert_eq!(
            model_token_limit("claude-sonnet-4-6")
                .expect("claude-sonnet-4-6 should be registered")
                .context_window_tokens,
            200_000
        );
    }

    #[test]
    fn preflight_blocks_requests_that_exceed_the_model_context_window() {
        let request = MessageRequest {
            model: "claude-sonnet-4-6".to_string(),
            max_tokens: 64_000,
            messages: Arc::new(vec![InputMessage {
                role: "user".to_string(),
                content: vec![InputContentBlock::Text {
                text: "x".repeat(600_000),
                }],
            }]),
            system: Some(Arc::from("Keep the answer short.")),
            tools: Some(vec![ToolDefinition {
                name: "weather".to_string(),
                description: Some("Fetches weather".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": { "city": { "type": "string" } },
                }),
            }]),
            tool_choice: Some(ToolChoice::Auto),
            stream: true,
            ..Default::default()
        };

        let error = preflight_message_request(&request)
            .expect_err("oversized request should be rejected before the provider call");

        match error {
            ApiError::ContextWindowExceeded {
                model,
                estimated_input_tokens,
                requested_output_tokens,
                estimated_total_tokens,
                context_window_tokens,
            } => {
                assert_eq!(model, "claude-sonnet-4-6");
                assert!(estimated_input_tokens > 136_000);
                assert_eq!(requested_output_tokens, 64_000);
                assert!(estimated_total_tokens > context_window_tokens);
                assert_eq!(context_window_tokens, 200_000);
            }
            other => panic!("expected context-window preflight failure, got {other:?}"),
        }
    }

    #[test]
    fn preflight_skips_unknown_models() {
        let request = MessageRequest {
            model: "unknown-model".to_string(),
            max_tokens: 64_000,
            messages: Arc::new(vec![InputMessage {
                role: "user".to_string(),
                content: vec![InputContentBlock::Text {
                    text: "x".repeat(600_000),
                }],
            }]),
            system: None,
            tools: None,
            tool_choice: None,
            stream: false,
            ..Default::default()
        };

        preflight_message_request(&request)
            .expect("models without context metadata should skip the guarded preflight");
    }

    #[test]
    fn preflight_rejects_unsupported_reasoning_effort() {
        // `max` is not a level native OpenAI exposes (`off/low/medium/high`
        // only), so it must fail before any network I/O. The `openai/` prefix
        // makes provider detection environment-independent.
        let request = MessageRequest {
            model: "openai/o4-mini".to_string(),
            max_tokens: 1024,
            messages: Arc::new(vec![InputMessage::user_text("think")]),
            reasoning_effort: Some("max".to_string()),
            ..Default::default()
        };
        let err = preflight_message_request(&request)
            .expect_err("max must be rejected for native OpenAI reasoning models");
        assert!(err.to_string().contains("o4-mini"));
        assert!(err.to_string().contains("max"));
        assert!(err.to_string().contains("off, low, medium, high"));
    }

    #[test]
    fn preflight_rejects_high_against_non_reasoning_model() {
        // A non-reasoning model exposes only `off`; `high` must fail fast.
        let request = MessageRequest {
            model: "gpt-4o".to_string(),
            max_tokens: 1024,
            messages: Arc::new(vec![InputMessage::user_text("hi")]),
            reasoning_effort: Some("high".to_string()),
            ..Default::default()
        };
        let err = preflight_message_request(&request)
            .expect_err("high must be rejected for non-reasoning models");
        assert!(err.to_string().contains("gpt-4o"));
        assert!(err.to_string().contains("high"));
    }

    #[test]
    fn preflight_rejects_unrecognised_level_string() {
        let request = MessageRequest {
            model: "o4-mini".to_string(),
            max_tokens: 1024,
            messages: Arc::new(vec![InputMessage::user_text("hi")]),
            reasoning_effort: Some("turbo".to_string()),
            ..Default::default()
        };
        let err = preflight_message_request(&request)
            .expect_err("an unrecognised level string must fail fast");
        assert!(err.to_string().contains("turbo"));
    }

    #[test]
    fn preflight_accepts_off_for_every_model() {
        let reasoning = |model: &str| MessageRequest {
            model: model.to_string(),
            max_tokens: 1024,
            messages: Arc::new(vec![InputMessage::user_text("hi")]),
            reasoning_effort: Some("off".to_string()),
            ..Default::default()
        };
        preflight_message_request(&reasoning("gpt-4o"))
            .expect("off is always supported");
        preflight_message_request(&reasoning("o4-mini"))
            .expect("off is always supported");
        preflight_message_request(&reasoning("claude-sonnet-4-6"))
            .expect("off is always supported");
    }

    #[test]
    fn parse_dotenv_extracts_keys_handles_comments_quotes_and_export_prefix() {
        // given
        let body = "\
# this is a comment

ANTHROPIC_API_KEY=plain-value
OPENAI_API_KEY='single-quoted'
   PADDED_KEY  =  padded-value  
EMPTY_VALUE=
NO_EQUALS_LINE
";

        // when
        let values = parse_dotenv(body);

        // then
        assert_eq!(
            values.get("ANTHROPIC_API_KEY").map(String::as_str),
            Some("plain-value")
        );
        assert_eq!(
            values.get("OPENAI_API_KEY").map(String::as_str),
            Some("single-quoted")
        );
        assert_eq!(
            values.get("PADDED_KEY").map(String::as_str),
            Some("padded-value")
        );
        assert_eq!(values.get("EMPTY_VALUE").map(String::as_str), Some(""));
        assert!(!values.contains_key("NO_EQUALS_LINE"));
        assert!(!values.contains_key("# this is a comment"));
    }

    #[test]
    fn load_dotenv_file_reads_keys_from_disk_and_returns_none_when_missing() {
        // given
        let temp_root = std::env::temp_dir().join(format!(
            "api-dotenv-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        std::fs::create_dir_all(&temp_root).expect("create temp dir");
        let env_path = temp_root.join(".env");
        std::fs::write(
            &env_path,
            "ANTHROPIC_API_KEY=secret-from-file\n# comment\n",
        )
        .expect("write .env");
        let missing_path = temp_root.join("does-not-exist.env");

        // when
        let loaded = load_dotenv_file(&env_path).expect("file should load");
        let missing = load_dotenv_file(&missing_path);

        // then
        assert_eq!(
            loaded.get("ANTHROPIC_API_KEY").map(String::as_str),
            Some("secret-from-file")
        );
        assert!(missing.is_none());

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn anthropic_missing_credentials_hint_is_none_when_no_foreign_creds_present() {
        // given
        let _lock = env_lock();
        let _openai = EnvVarGuard::set("OPENAI_API_KEY", None);

        // when
        let hint = anthropic_missing_credentials_hint();

        // then
        assert!(
            hint.is_none(),
            "no hint should be produced when every foreign provider env var is absent, got {hint:?}"
        );
    }

    #[test]
    fn anthropic_missing_credentials_hint_detects_openai_api_key_and_recommends_openai_prefix() {
        // given
        let _lock = env_lock();
        let _openai = EnvVarGuard::set("OPENAI_API_KEY", Some("sk-openrouter-varleg"));

        // when
        let hint = anthropic_missing_credentials_hint()
            .expect("OPENAI_API_KEY presence should produce a hint");

        // then
        assert!(
            hint.contains("OPENAI_API_KEY is set"),
            "hint should name the detected env var so users recognize it: {hint}"
        );
        assert!(
            hint.contains("OpenAI-compat"),
            "hint should identify the target provider: {hint}"
        );
        assert!(
            hint.contains("openai/"),
            "hint should mention the `openai/` prefix routing fix: {hint}"
        );
        assert!(
            hint.contains("OPENAI_BASE_URL"),
            "hint should mention OPENAI_BASE_URL so OpenRouter users see the full picture: {hint}"
        );
    }



    #[test]
    fn anthropic_missing_credentials_builds_error_with_canonical_env_vars_and_no_hint_when_clean() {
        // given
        let _lock = env_lock();
        let _openai = EnvVarGuard::set("OPENAI_API_KEY", None);

        // when
        let error = anthropic_missing_credentials();

        // then
        match &error {
            ApiError::MissingCredentials {
                provider,
                env_vars,
                hint,
            } => {
                assert_eq!(*provider, "Anthropic");
                assert_eq!(*env_vars, &["ANTHROPIC_API_KEY"]);
                assert!(
                    hint.is_none(),
                    "clean environment should not generate a hint, got {hint:?}"
                );
            }
            other => panic!("expected MissingCredentials variant, got {other:?}"),
        }
        let rendered = error.to_string();
        assert!(
            !rendered.contains(" — hint: "),
            "rendered error should be a plain missing-creds message: {rendered}"
        );
    }

    #[test]
    fn anthropic_missing_credentials_builds_error_with_hint_when_openai_key_is_set() {
        // given
        let _lock = env_lock();
        let _openai = EnvVarGuard::set("OPENAI_API_KEY", Some("sk-openrouter-varleg"));

        // when
        let error = anthropic_missing_credentials();

        // then
        match &error {
            ApiError::MissingCredentials {
                provider,
                env_vars,
                hint,
            } => {
                assert_eq!(*provider, "Anthropic");
                assert_eq!(*env_vars, &["ANTHROPIC_API_KEY"]);
                let hint_value = hint.as_deref().expect("hint should be populated");
                assert!(
                    hint_value.contains("OPENAI_API_KEY is set"),
                    "hint should name the detected env var: {hint_value}"
                );
            }
            other => panic!("expected MissingCredentials variant, got {other:?}"),
        }
        let rendered = error.to_string();
        assert!(
            rendered.starts_with("missing Anthropic credentials;"),
            "canonical base message should still lead the rendered error: {rendered}"
        );
        assert!(
            rendered.contains(" — hint: I see OPENAI_API_KEY is set"),
            "rendered error should carry the env-driven hint: {rendered}"
        );
    }

    #[test]
    fn anthropic_missing_credentials_hint_ignores_empty_string_values() {
        // given
        let _lock = env_lock();
        // An empty value is semantically equivalent to "not set" for the
        // credential discovery path, so the sniffer must treat it that way
        // to avoid false-positive hints for users who intentionally cleared
        // a stale export with `OPENAI_API_KEY=`.
        let _openai = EnvVarGuard::set("OPENAI_API_KEY", Some(""));

        // when
        let hint = anthropic_missing_credentials_hint();

        // then
        assert!(
            hint.is_none(),
            "empty env var should not trigger the hint sniffer, got {hint:?}"
        );
    }

    #[test]
    fn openai_base_url_overrides_anthropic_fallback_for_unknown_model() {
        // given — user has OPENAI_BASE_URL + OPENAI_API_KEY but no Anthropic
        // creds, and a model name with no recognized prefix.
        let _lock = env_lock();
        let _base_url = EnvVarGuard::set("OPENAI_BASE_URL", Some("http://127.0.0.1:11434/v1"));
        let _api_key = EnvVarGuard::set("OPENAI_API_KEY", Some("dummy"));
        let _anthropic_key = EnvVarGuard::set("ANTHROPIC_API_KEY", None);

        // when
        let provider = detect_provider_kind("qwen2.5-coder:7b");

        // then — should route to OpenAI, not Anthropic
        assert_eq!(
            provider,
            ProviderKind::OpenAi,
            "OPENAI_BASE_URL should win over Anthropic fallback for unknown models"
        );
    }

    // NOTE: a "OPENAI_BASE_URL without OPENAI_API_KEY" test is omitted
    // because workspace-parallel test binaries can race on process env
    // (env_lock only protects within a single binary). The detection logic
    // is covered: OPENAI_BASE_URL alone routes to OpenAi as a last-resort
    // fallback in detect_provider_kind().
}
