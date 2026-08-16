use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Map, Value};
use telemetry::SessionTracer;

use crate::compact::{
    compact_session, estimate_session_tokens, CompactionConfig, CompactionResult,
};
use crate::compression_config::CompressionConfig;
use crate::config::RuntimeFeatureConfig;
use crate::hooks::{HookAbortSignal, HookProgressReporter, HookRunResult, HookRunner};
use crate::permissions::{
    PermissionContext, PermissionOutcome, PermissionPolicy, PermissionPrompter,
};
use crate::image_store::ImageStore;
use crate::session::{externalize_message_images, ContentBlock, ConversationMessage, MessageRole, Session};
use crate::usage::{TokenUsage, UsageTracker};

const DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD: u32 = 300_000;
const AUTO_COMPACTION_THRESHOLD_ENV_VAR: &str = "CLAUDE_CODE_AUTO_COMPACT_INPUT_TOKENS";
const MAX_CONSECUTIVE_COMPACTIONS: usize = 3;

fn parse_input_content(input: &str) -> Vec<ContentBlock> {
    let mut blocks = Vec::new();

    let file_marker = "<input_file ";
    let file_end_marker = "</input_file>";
    let image_marker = "<input_image ";
    let image_end_marker = "/>";

    if let Some(start) = input.find(file_marker) {
        if let Some(end) = input.find(file_end_marker) {
            let before = input[..start].trim();
            let inner = &input[start + file_marker.len()..end];
            let after = input[end + file_end_marker.len()..].trim();

            let mut source_path = None;
            let mut file_content = String::new();

            for part in inner.split_whitespace() {
                if part.starts_with("path=\"") {
                    if let Some(path_part) = part.get(5..) {
                        if let Some(end_quote) = path_part.rfind('"') {
                            if end_quote > 0 {
                                source_path = Some(path_part[..end_quote].to_string());
                            }
                        }
                    }
                }
            }

            if let Some(pos) = inner.find(">\n") {
                file_content = inner[pos + 1..].trim().to_string();
            } else {
                for part in inner.split_whitespace() {
                    if !part.starts_with("path=\"") {
                        file_content = part.to_string();
                        break;
                    }
                }
            }

            if !before.is_empty() {
                blocks.push(ContentBlock::Text {
                    text: before.to_string(),
                });
            }

            if let Some(path) = source_path {
                let path_lower = path.to_lowercase();
                let file_content = if path_lower.ends_with(".pdf") {
                    // For PDF files, suggest using read_file tool
                    format!("[PDF file detected: {}]\n\nTo read this PDF, please use the read_file tool by saying: read_file \"{}\"", path, path)
                } else {
                    format!("[File: {}]", path)
                };
                blocks.push(ContentBlock::Text { text: file_content });
            }

            if !file_content.is_empty() {
                blocks.push(ContentBlock::Text { text: file_content });
            }
            if !after.is_empty() {
                blocks.push(ContentBlock::Text {
                    text: after.to_string(),
                });
            }

            return blocks;
        }
    }

    // Loop over all `<input_image .../>` tags (previously only the first was parsed)
    let mut remaining = input;
    let mut has_images = false;

    while let Some(start) = remaining.find(image_marker) {
        has_images = true;
        if let Some(end_rel) = remaining[start..].find(image_end_marker) {
            let end = start + end_rel;
            let before = remaining[..start].trim();
            let inner = &remaining[start + image_marker.len()..end];
            remaining = &remaining[end + image_end_marker.len()..];

            if !before.is_empty() {
                blocks.push(ContentBlock::Text {
                    text: before.to_string(),
                });
            }

            let mut mime_type = String::new();
            let mut base64_data = String::new();
            let mut hash_hex = String::new();

            let mut filename = None;

            for part in inner.split_whitespace() {
                if part.starts_with("mime=\"") {
                    if let Some(equals_pos) = part.find('=') {
                        let after_equals = &part[equals_pos + 1..];
                        if after_equals.starts_with('"') && after_equals.ends_with('"') {
                            mime_type = after_equals[1..after_equals.len() - 1].to_string();
                        }
                    }
                }
                if part.starts_with("base64=\"") {
                    if let Some(equals_pos) = part.find('=') {
                        let after_equals = &part[equals_pos + 1..];
                        if after_equals.starts_with('"') && after_equals.ends_with('"') {
                            base64_data = after_equals[1..after_equals.len() - 1].to_string();
                        }
                    }
                }
                if part.starts_with("hash=\"") {
                    if let Some(equals_pos) = part.find('=') {
                        let after_equals = &part[equals_pos + 1..];
                        if after_equals.starts_with('"') && after_equals.ends_with('"') {
                            let raw = after_equals[1..after_equals.len() - 1].to_string();
                            if raw.len() >= 2 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
                                hash_hex = raw;
                            }
                        }
                    }
                }
                if part.starts_with("path=\"") {
                    if let Some(equals_pos) = part.find('=') {
                        let after_equals = &part[equals_pos + 1..];
                        if after_equals.starts_with('"') && after_equals.ends_with('"') {
                            filename = Some(after_equals[1..after_equals.len() - 1].to_string());
                        }
                    }
                }
            }

            if !mime_type.is_empty() && !hash_hex.is_empty() {
                blocks.push(ContentBlock::ImageRef {
                    mime_type,
                    hash_hex,
                    filename,
                });
            } else if !mime_type.is_empty() && !base64_data.is_empty() {
                blocks.push(ContentBlock::Image {
                    mime_type,
                    data: base64_data,
                    filename,
                });
            }
        } else {
            break;
        }
    }

    if has_images {
        let tail = remaining.trim();
        if !tail.is_empty() {
            blocks.push(ContentBlock::Text {
                text: tail.to_string(),
            });
        }
    } else {
        blocks.push(ContentBlock::Text {
            text: input.to_string(),
        });
    }
    blocks
}

/// Placeholder signature attached to the thinking block of a synthesized
/// forced-delegation assistant turn (see `run_turn_forced`). Anthropic's
/// extended-thinking contract requires every assistant turn that carries a
/// `ToolUse` block to echo back a thinking block; `convert.rs` only forwards
/// thinking blocks that carry a `signature`, so the synthesized turn needs a
/// signature value even though it was never produced by the model. The value
/// itself is opaque to the API round-trip.
const FORCED_DELEGATION_THINKING_SIGNATURE: &str = "forced-delegation";

/// Fully assembled request payload sent to the upstream model client.
///
/// Both `system_prompt` and `messages` use `Arc` for O(1) cloning.
/// In the agentic loop, `Arc::clone` is used to hand the request to
/// the API client, and `Arc::make_mut` provides copy-on-write mutation
/// when appending new messages — so the full history is never deep-copied
/// after the initial `filter_for_api` pass.
#[derive(Debug, Clone)]
pub struct ApiRequest {
    /// Pre-joined system prompt (computed once per session, cheap `Arc` clone).
    pub system_prompt: Arc<str>,
    /// Shared message history. `Arc::clone` is O(1); `Arc::make_mut`
    /// gives copy-on-write semantics for appending new messages.
    pub messages: Arc<Vec<ConversationMessage>>,
    pub image_cache: Option<Arc<Mutex<HashMap<String, String>>>>,
    pub image_store: Option<ImageStore>,
}

impl PartialEq for ApiRequest {
    fn eq(&self, other: &Self) -> bool {
        self.system_prompt == other.system_prompt && self.messages == other.messages
    }
}

impl Eq for ApiRequest {}

/// Streamed events emitted while processing a single assistant turn.
#[derive(Debug, Clone, PartialEq)]
pub enum AssistantEvent {
    TextDelta(String),
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    Usage(TokenUsage),
    PromptCache(PromptCacheEvent),
    MessageStop,
    /// Accumulated thinking content from a thinking block, plus its signature
    /// (captured from `signature_delta`) which the Anthropic API requires when
    /// the block is echoed back on a follow-up request.
    Thinking {
        text: String,
        signature: Option<String>,
    },
    /// A redacted thinking block returned by the provider. The `data` ciphertext
    /// is opaque but must be echoed back verbatim on the tool-use round-trip.
    RedactedThinking {
        data: String,
    },
    // Added to handle image output events
    Image {
        data: String,
        mime_type: String,
    },
}

/// Prompt-cache telemetry captured from the provider response stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptCacheEvent {
    pub unexpected: bool,
    pub reason: String,
    pub previous_cache_read_input_tokens: u32,
    pub current_cache_read_input_tokens: u32,
    pub token_drop: u32,
}

/// Minimal streaming API contract required by [`ConversationRuntime`].
pub trait ApiClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError>;
}

/// Trait implemented by tool dispatchers that execute model-requested tools.
pub trait ToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError>;
}

/// Error returned when a tool invocation fails locally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolError {
    message: String,
}

impl ToolError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for ToolError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ToolError {}

/// Error returned when a conversation turn cannot be completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    message: String,
}

impl RuntimeError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl RuntimeError {
    #[must_use]
    pub fn is_context_window_error(&self) -> bool {
        let msg = self.message.to_lowercase();
        msg.contains("exceed_context_size_error")
            || msg.contains("maximum context length")
            || msg.contains("context window")
            || msg.contains("context length")
            || msg.contains("too many tokens")
            || msg.contains("prompt is too long")
            || msg.contains("input is too long")
            || msg.contains("request is too large")
    }

    /// Returns true when the failure indicates the account balance/credits are
    /// exhausted (e.g. a relay or gateway returned insufficient_quota or 余额不足).
    /// Such failures are deterministic — retrying cannot help — and should be
    /// surfaced as a normal "unavailable" notice instead of terminating the
    /// session.
    #[must_use]
    pub fn is_balance_error(&self) -> bool {
        let msg = self.message.to_lowercase();
        msg.contains("insufficient_quota")
            || msg.contains("insufficient quota")
            || msg.contains("insufficient balance")
            || msg.contains("insufficient_balance")
            || msg.contains("balance is insufficient")
            || msg.contains("your account balance")
            || msg.contains("account balance is")
            || msg.contains("no credits")
            || msg.contains("out of credits")
            || msg.contains("credit balance")
            || msg.contains("insufficient credits")
            || msg.contains("balance is too low")
            || msg.contains("余额不足")
            || msg.contains("payment required")
    }

    /// Parse the model's context window size from the error message.
    /// The error message may contain "context size (N tokens)" (runtime API error)
    /// or "Context window   N tokens" (preflight check).
    #[must_use]
    pub fn context_window_tokens(&self) -> Option<u32> {
        let msg = &self.message;
        if let Some(start) = msg.find("context size (") {
            let rest = &msg[start + "context size (".len()..];
            if let Some(end) = rest.find(" tokens") {
                return rest[..end].parse().ok();
            }
        }
        if let Some(start) = msg.find("Context window   ") {
            let rest = &msg[start + "Context window   ".len()..];
            if let Some(end) = rest.find(" tokens") {
                return rest[..end].parse().ok();
            }
        }
        if let Some(start) = msg.find("context window ") {
            let rest = &msg[start + "context window ".len()..];
            if let Some(end) = rest.find('\n') {
                return rest[..end].trim().parse().ok();
            }
        }
        None
    }
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RuntimeError {}

/// Summary of one completed runtime turn, including tool results and usage.
#[derive(Debug, Clone, PartialEq)]
pub struct TurnSummary {
    pub assistant_messages: Vec<ConversationMessage>,
    pub tool_results: Vec<ConversationMessage>,
    pub prompt_cache_events: Vec<PromptCacheEvent>,
    pub iterations: usize,
    pub usage: TokenUsage,
    pub auto_compaction: Option<AutoCompactionEvent>,
}

/// Details about automatic session compaction applied during a turn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutoCompactionEvent {
    pub removed_message_count: usize,
    /// Ratio of estimated tokens removed to total tokens before compaction.
    /// Used by the CLI to display "Compression saved X% of context".
    pub savings_ratio: f64,
}

/// Coordinates the model loop, tool execution, hooks, and session updates.
pub struct ConversationRuntime<C, T> {
    session: Session,
    api_client: C,
    tool_executor: T,
    permission_policy: PermissionPolicy,
    /// Pre-joined system prompt string. Computed once in the constructor;
    /// `Arc::clone` in the agentic loop avoids re-joining every iteration.
    system_prompt_joined: Arc<str>,
    max_iterations: usize,
    usage_tracker: UsageTracker,
    hook_runner: HookRunner,
    auto_compaction_input_tokens_threshold: u32,
    compression_config: CompressionConfig,
    hook_abort_signal: HookAbortSignal,
    hook_progress_reporter: Option<Box<dyn HookProgressReporter>>,
    session_tracer: Option<SessionTracer>,
    image_store: Option<ImageStore>,
    image_base64_cache: Arc<Mutex<HashMap<String, String>>>,
    /// Tool-use ids synthesized by `run_turn_forced` (deterministic
    /// `$skill` / `@agent` delegation). These are auto-allowed so the
    /// delegation never blocks on an interactive permission prompt.
    forced_tool_ids: HashSet<String>,
    /// Optional cooperative cancellation signal (e.g. a timed-out sub-agent
    /// reap). When set, `drive_turn_loop` aborts at the next iteration
    /// boundary instead of continuing to call the provider.
    cancel_signal: Option<Arc<AtomicBool>>,
}

impl<C, T> ConversationRuntime<C, T>
where
    C: ApiClient,
    T: ToolExecutor,
{
    #[must_use]
    pub fn new(
        session: Session,
        api_client: C,
        tool_executor: T,
        permission_policy: PermissionPolicy,
        system_prompt: Vec<String>,
    ) -> Self {
        Self::new_with_features(
            session,
            api_client,
            tool_executor,
            permission_policy,
            system_prompt,
            &RuntimeFeatureConfig::default(),
        )
    }

    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn new_with_features(
        session: Session,
        api_client: C,
        tool_executor: T,
        permission_policy: PermissionPolicy,
        system_prompt: Vec<String>,
        feature_config: &RuntimeFeatureConfig,
    ) -> Self {
        let usage_tracker = UsageTracker::from_session(&session);
        let image_store = Self::init_image_store();
        // Pre-join system prompt once; downstream consumers receive Arc<str>
        // which is O(1) to clone instead of re-joining Vec<String> every iteration.
        let system_prompt_joined: Arc<str> = Arc::from(system_prompt.join("\n\n"));
        let runtime = Self {
            session,
            api_client,
            tool_executor,
            permission_policy,
            system_prompt_joined,
            max_iterations: usize::MAX,
            usage_tracker,
            hook_runner: HookRunner::from_feature_config(feature_config),
            auto_compaction_input_tokens_threshold: auto_compaction_threshold_from_env(),
            compression_config: CompressionConfig::from_env(),
            hook_abort_signal: HookAbortSignal::default(),
            hook_progress_reporter: None,
            session_tracer: None,
            image_store,
            image_base64_cache: Arc::new(Mutex::new(HashMap::new())),
            forced_tool_ids: HashSet::new(),
            cancel_signal: None,
        };
        // Pre-populate cache for all existing ImageRef blocks in the session
        if let Some(ref store) = runtime.image_store {
            let mut cache = runtime.image_base64_cache.lock().unwrap();
            for msg in &runtime.session.messages {
                for block in &msg.blocks {
                    if let ContentBlock::ImageRef { hash_hex, mime_type, .. } = block {
                        if !cache.contains_key(hash_hex) {
                            if let Ok(b64) = store.load_base64(hash_hex, mime_type) {
                                cache.insert(hash_hex.clone(), b64);
                            }
                        }
                    }
                }
            }
        }
        runtime.emit_lifecycle_hook("SessionStart");
        runtime
    }

    #[must_use]
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    #[must_use]
    pub fn with_cancel_signal(mut self, cancel_signal: Arc<AtomicBool>) -> Self {
        self.cancel_signal = Some(cancel_signal);
        self
    }

    #[must_use]
    pub fn with_auto_compaction_input_tokens_threshold(mut self, threshold: u32) -> Self {
        self.auto_compaction_input_tokens_threshold = threshold;
        self
    }

    #[must_use]
    pub fn with_compression_config(mut self, config: CompressionConfig) -> Self {
        self.compression_config = config;
        self
    }

    #[must_use]
    pub fn with_hook_abort_signal(mut self, hook_abort_signal: HookAbortSignal) -> Self {
        self.hook_abort_signal = hook_abort_signal;
        self
    }

    #[must_use]
    pub fn with_hook_progress_reporter(
        mut self,
        hook_progress_reporter: Box<dyn HookProgressReporter>,
    ) -> Self {
        self.hook_progress_reporter = Some(hook_progress_reporter);
        self
    }

    #[must_use]
    pub fn with_session_tracer(mut self, session_tracer: SessionTracer) -> Self {
        self.session_tracer = Some(session_tracer);
        self
    }

    fn run_pre_tool_use_hook(&mut self, tool_name: &str, input: &str) -> HookRunResult {
        if let Some(reporter) = self.hook_progress_reporter.as_mut() {
            self.hook_runner.run_pre_tool_use_with_context(
                tool_name,
                input,
                Some(&self.hook_abort_signal),
                Some(reporter.as_mut()),
            )
        } else {
            self.hook_runner.run_pre_tool_use_with_context(
                tool_name,
                input,
                Some(&self.hook_abort_signal),
                None,
            )
        }
    }

    fn run_post_tool_use_hook(
        &mut self,
        tool_name: &str,
        input: &str,
        output: &str,
        is_error: bool,
    ) -> HookRunResult {
        if let Some(reporter) = self.hook_progress_reporter.as_mut() {
            self.hook_runner.run_post_tool_use_with_context(
                tool_name,
                input,
                output,
                is_error,
                Some(&self.hook_abort_signal),
                Some(reporter.as_mut()),
            )
        } else {
            self.hook_runner.run_post_tool_use_with_context(
                tool_name,
                input,
                output,
                is_error,
                Some(&self.hook_abort_signal),
                None,
            )
        }
    }

    fn run_post_tool_use_failure_hook(
        &mut self,
        tool_name: &str,
        input: &str,
        output: &str,
    ) -> HookRunResult {
        if let Some(reporter) = self.hook_progress_reporter.as_mut() {
            self.hook_runner.run_post_tool_use_failure_with_context(
                tool_name,
                input,
                output,
                Some(&self.hook_abort_signal),
                Some(reporter.as_mut()),
            )
        } else {
            self.hook_runner.run_post_tool_use_failure_with_context(
                tool_name,
                input,
                output,
                Some(&self.hook_abort_signal),
                None,
            )
        }
    }

    /// Fire a lifecycle hook event (e.g. `SessionStart`, `UserPromptSubmit`,
    /// `Stop`, `PreCompact`, `PostCompact`) with the current session id and
    /// working directory. Lifecycle hooks receive no tool metadata.
    fn emit_lifecycle_hook(&self, event: &str) {
        let session_id = self.session.session_id.as_str();
        let cwd = self
            .session
            .workspace_root
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());
        let result = self.hook_runner.run_event(event, Some(session_id), cwd.as_deref());
        if result.is_failed() || result.is_cancelled() {
            let rendered = result.messages().join("; ");
            eprintln!("warn: {event} hook reported issues: {rendered}");
        }
    }

    /// Run a session health probe to verify the runtime is functional after compaction.
    /// Returns Ok(()) if healthy, Err if the session appears broken.
    fn run_session_health_probe(&mut self) -> Result<(), String> {
        // Check if we have basic session integrity
        if self.session.messages.is_empty() && self.session.compaction.is_some() {
            // Freshly compacted with no messages - this is normal
            return Ok(());
        }

        // Verify tool executor is responsive with a non-destructive probe
        // Using glob_search with a pattern that won't match anything
        let probe_input = r#"{"pattern": "*.health-check-probe-"}"#;
        match self.tool_executor.execute("glob_search", probe_input) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Tool executor probe failed: {e}")),
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn run_turn(
        &mut self,
        user_input: impl Into<String>,
        mut prompter: Option<&mut dyn PermissionPrompter>,
    ) -> Result<TurnSummary, RuntimeError> {
        let user_input = user_input.into();

        // ROADMAP #38: Session-health canary - probe if context was compacted
        if self.session.compaction.is_some() {
            if let Err(error) = self.run_session_health_probe() {
                return Err(RuntimeError::new(format!(
                    "Session health probe failed after compaction: {error}. \
                     The session may be in an inconsistent state. \
                     Consider starting a fresh session with /session new."
                )));
            }
        }

        self.record_turn_started(&user_input);

        self.emit_lifecycle_hook("UserPromptSubmit");

        let content_blocks = parse_input_content(&user_input);

        self.session
            .push_user_content(content_blocks)
            .map_err(|error| RuntimeError::new(error.to_string()))?;
        // Externalize + cache base64 for the new message (cache populated once per image)
        let store = self.image_store().cloned();
        if let Some(ref store) = store {
            if let Some(last_msg) = self.session.messages.last_mut() {
                externalize_message_images(last_msg, store, &mut self.image_base64_cache.lock().unwrap())
                    .map_err(|e| RuntimeError::new(format!("Failed to externalize images: {e}")))?;
            }
        }

        let mut assistant_messages = Vec::new();
        let mut tool_results = Vec::new();
        let mut prompt_cache_events = Vec::new();
        let mut iterations = 0;

        // Keep ImageRef in api_messages — resolved lazily in convert_messages via cache
        // Use context::filter_for_api to strip Thinking content before sending to LLM.
        // Wrap in Arc for copy-on-write: Arc::clone is O(1) per iteration,
        // Arc::make_mut mutates in place when we're the sole owner (which is
        // the case after the previous stream() call has completed and dropped
        // its Arc handle).
        let mut api_messages = Arc::new(crate::context::filter_for_api_with_config(
            &self.session.messages,
            &self.compression_config,
        ));

        self.drive_turn_loop(
            &mut prompter,
            &mut api_messages,
            &mut assistant_messages,
            &mut tool_results,
            &mut prompt_cache_events,
            &mut iterations,
        )?;

        let auto_compaction = self.maybe_auto_compact();

        let summary = TurnSummary {
            assistant_messages,
            tool_results,
            prompt_cache_events,
            iterations,
            usage: self.usage_tracker.cumulative_usage(),
            auto_compaction,
        };
        self.record_turn_completed(&summary);

        self.emit_lifecycle_hook("Stop");

        Ok(summary)
    }

    /// Shared agentic loop body used by both `run_turn` and `run_turn_forced`.
    /// Streams the model, executes any `ToolUse` blocks via `execute_one_tool_use`,
    /// and continues until the model emits a turn with no pending tool uses.
    fn drive_turn_loop(
        &mut self,
        prompter: &mut Option<&mut dyn PermissionPrompter>,
        api_messages: &mut Arc<Vec<ConversationMessage>>,
        assistant_messages: &mut Vec<ConversationMessage>,
        tool_results: &mut Vec<ConversationMessage>,
        prompt_cache_events: &mut Vec<PromptCacheEvent>,
        iterations: &mut usize,
    ) -> Result<(), RuntimeError> {
        let mut compaction_retries = 0;
        loop {
            if self
                .cancel_signal
                .as_ref()
                .is_some_and(|signal| signal.load(Ordering::Relaxed))
            {
                let error = RuntimeError::new("agent cancelled");
                self.record_turn_failed(*iterations, &error);
                return Err(error);
            }
            *iterations += 1;
            if *iterations > self.max_iterations {
                let error = RuntimeError::new(
                    "conversation loop exceeded the maximum number of iterations",
                );
                self.record_turn_failed(*iterations, &error);
                return Err(error);
            }
            let request = ApiRequest {
                system_prompt: Arc::clone(&self.system_prompt_joined),
                messages: Arc::clone(api_messages), // O(1) ref-count bump
                image_cache: Some(self.image_base64_cache.clone()),
                image_store: self.image_store.clone(),
            };
            let events = match self.api_client.stream(request) {
                Ok(events) => events,
                Err(error) if error.is_context_window_error() => {
                    if compaction_retries < MAX_CONSECUTIVE_COMPACTIONS {
                        compaction_retries += 1;
                        // Recover by compacting: preserve the recent tail and
                        // fold the earlier history into a summary, then retry
                        // with the rebuilt message list. This never wipes the
                        // conversation (the old behaviour silently reset it to
                        // a single empty user message and could return Ok with
                        // zero output).
                        let base = CompactionConfig::from_config(&self.compression_config);
                        // The provider reported an over-budget request even
                        // though our session estimate may not reflect it
                        // (system prompt, tool definitions, image caches live
                        // outside the transcript). Force compaction whenever
                        // there is anything removable rather than re-checking
                        // our own budget.
                        let result = compact_session(
                            &self.session,
                            CompactionConfig {
                                max_estimated_tokens: 0,
                                ..base
                            },
                        );
                        if result.removed_message_count == 0 {
                            // Nothing removable — surface the real error rather
                            // than looping on an unshrinkable session.
                            self.record_turn_failed(*iterations, &error);
                            return Err(error);
                        }
                        self.session = result.compacted_session;
                        *api_messages = Arc::new(
                            crate::context::filter_for_api_with_config(
                                &self.session.messages,
                                &self.compression_config,
                            ),
                        );
                        continue;
                    }
                    // All retries exhausted — return a real error instead of
                    // silently succeeding with an empty transcript.
                    self.record_turn_failed(*iterations, &error);
                    return Err(error);
                }
                Err(error) => {
                    self.record_turn_failed(*iterations, &error);
                    return Err(error);
                }
            };
            let (assistant_message, usage, turn_prompt_cache_events) =
                match build_assistant_message(events) {
                    Ok(result) => result,
                    Err(error) => {
                        self.record_turn_failed(*iterations, &error);
                        return Err(error);
                    }
                };
            if let Some(usage) = usage {
                self.usage_tracker.record(usage);
            }
            prompt_cache_events.extend(turn_prompt_cache_events);
            let pending_tool_uses = assistant_message
                .blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::ToolUse { id, name, input } => {
                        Some((id.clone(), name.clone(), input.clone()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            self.record_assistant_iteration(*iterations, &assistant_message, pending_tool_uses.len());

            self.session
                .push_message(assistant_message.clone())
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            // COW push: if we're the sole owner (stream() already dropped
            // its Arc), this mutates in place — no deep copy.
            Arc::make_mut(api_messages).push(assistant_message.clone());
            assistant_messages.push(assistant_message);

            if pending_tool_uses.is_empty() {
                break;
            }

            // Anthropic requires every tool_result responding to a tool_use
            // turn to live in the single user message immediately following
            // the assistant message. Emitting each result as its own message
            // breaks that pairing for parallel tool calls (400: `tool_use`
            // ids found without `tool_result` blocks immediately after).
            let mut results = Vec::new();
            for (tool_use_id, tool_name, input) in pending_tool_uses {
                let input_str = match &input {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                let result_message =
                    self.execute_one_tool_use(tool_use_id, tool_name, input_str, prompter, *iterations)?;
                self.record_tool_finished(*iterations, &result_message);
                tool_results.push(result_message.clone());
                results.push(result_message);
            }
            let merged = merge_tool_result_messages(results);
            self.session
                .push_message(merged.clone())
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            Arc::make_mut(api_messages).push(merged);
        }
        Ok(())
    }

    /// Execute a single tool use with the full permission/hook lifecycle.
    /// A tool-use id present in `forced_tool_ids` is auto-allowed, which makes
    /// deterministic `$skill` / `@agent` delegation non-interactive.
    fn execute_one_tool_use(
        &mut self,
        tool_use_id: String,
        tool_name: String,
        input: String,
        prompter: &mut Option<&mut dyn PermissionPrompter>,
        iterations: usize,
    ) -> Result<ConversationMessage, RuntimeError> {
        let pre_hook_result = self.run_pre_tool_use_hook(&tool_name, &input);
        let effective_input: String = pre_hook_result
            .updated_input()
            .map_or_else(|| input.clone(), ToOwned::to_owned);
        let permission_context = PermissionContext::new(
            pre_hook_result.permission_override(),
            pre_hook_result.permission_reason().map(ToOwned::to_owned),
        );

        // A forced tool-use id (deterministic `$skill` / `@agent` delegation)
        // is auto-allowed so delegation stays non-interactive. That exemption
        // must NOT swallow an explicit PreToolUse hook veto: a hook that
        // cancels/fails/denies the delegation still blocks it, otherwise a
        // forced id would silently elevate past every hook gate.
        let permission_outcome = if self.forced_tool_ids.contains(&tool_use_id)
            && !pre_hook_result.is_cancelled()
            && !pre_hook_result.is_failed()
            && !pre_hook_result.is_denied()
        {
            PermissionOutcome::Allow
        } else if pre_hook_result.is_cancelled() {
            PermissionOutcome::Deny {
                reason: format_hook_message(
                    &pre_hook_result,
                    &format!("PreToolUse hook cancelled tool `{tool_name}`"),
                ),
            }
        } else if pre_hook_result.is_failed() {
            PermissionOutcome::Deny {
                reason: format_hook_message(
                    &pre_hook_result,
                    &format!("PreToolUse hook failed for tool `{tool_name}`"),
                ),
            }
        } else if pre_hook_result.is_denied() {
            PermissionOutcome::Deny {
                reason: format_hook_message(
                    &pre_hook_result,
                    &format!("PreToolUse hook denied tool `{tool_name}`"),
                ),
            }
        } else if let Some(prompt) = prompter.as_mut() {
            self.permission_policy.authorize_with_context(
                &tool_name,
                &effective_input,
                &permission_context,
                Some(*prompt),
            )
        } else {
            self.permission_policy.authorize_with_context(
                &tool_name,
                &effective_input,
                &permission_context,
                None,
            )
        };

        let result_message = match permission_outcome {
            PermissionOutcome::Allow => {
                if tool_name == "WebFetch" {
                    let new_url: Option<String> = serde_json::from_str::<Value>(&effective_input)
                        .ok()
                        .and_then(|v| v.get("url")?.as_str().map(String::from));
                    let mut mutated_indices = Vec::new();
                    for (msg_index, msg) in self.session.messages.iter_mut().enumerate() {
                        for block in &mut msg.blocks {
                            if let ContentBlock::ToolResult { tool_name: tn, output, .. } = block {
                                if tn == "WebFetch" && !output.is_empty() {
                                    let same_url = new_url.as_ref().and_then(|nu| {
                                        serde_json::from_str::<Value>(output).ok().and_then(|v| {
                                            v.get("url")?.as_str().map(|u| u == nu)
                                        })
                                    });
                                    if same_url == Some(true) {
                                        *output = String::new();
                                    } else {
                                        *output = crate::context::summarize_tool_result("WebFetch", output);
                                    }
                                    mutated_indices.push(msg_index);
                                }
                            }
                        }
                    }
                    // In-place output mutation invalidates the cached token
                    // estimate and cached wire message for the affected
                    // messages; otherwise later estimates (including the
                    // auto-compaction budget check) use stale inflated values.
                    for msg_index in mutated_indices {
                        if let Some(msg) = self.session.messages.get_mut(msg_index) {
                            msg.cached_tokens = OnceLock::new();
                            msg.cached_input_message = OnceLock::new();
                        }
                    }
                }
                self.record_tool_started(iterations, &tool_name);
                let (mut output, mut is_error) = match self.tool_executor.execute(&tool_name, &effective_input) {
                    Ok(output) => (output, false),
                    Err(error) => (error.to_string(), true),
                };
                output = merge_hook_feedback(pre_hook_result.messages(), output, false);

                let post_hook_result = if is_error {
                    self.run_post_tool_use_failure_hook(&tool_name, &effective_input, &output)
                } else {
                    self.run_post_tool_use_hook(&tool_name, &effective_input, &output, false)
                };
                if post_hook_result.is_denied() || post_hook_result.is_failed() || post_hook_result.is_cancelled() {
                    is_error = true;
                }
                output = merge_hook_feedback(
                    post_hook_result.messages(),
                    output,
                    post_hook_result.is_denied() || post_hook_result.is_failed() || post_hook_result.is_cancelled(),
                );

                ConversationMessage::tool_result(tool_use_id, tool_name, output, is_error)
            }
            PermissionOutcome::Deny { reason } => {
                ConversationMessage::tool_result(
                    tool_use_id,
                    tool_name,
                    merge_hook_feedback(pre_hook_result.messages(), reason, true),
                    true,
                )
            }
        };
        Ok(result_message)
    }

    /// Deterministic delegation entry point for `$skill` and `@agent`.
    ///
    /// Instead of forwarding the literal `$skill` / `@agent` text to the model
    /// and hoping it voluntarily emits a `Skill` / `Agent` tool_use (which it
    /// often role-plays instead of actually calling), this synthesizes an
    /// assistant `ToolUse` message for the requested tool, executes it through
    /// the real tool pipeline (so the skill/agent actually runs), then lets the
    /// normal agentic loop stream the model's response to the tool result.
    ///
    /// `forced_tool_name`/`forced_tool_input` are the already-serialized tool
    /// name and JSON input (e.g. `("Skill", {"skill":"browser-harness",...})`).
    pub fn run_turn_forced(
        &mut self,
        user_input: impl Into<String>,
        forced_tool_name: String,
        forced_tool_input: String,
        mut prompter: Option<&mut dyn PermissionPrompter>,
    ) -> Result<TurnSummary, RuntimeError> {
        let user_input = user_input.into();

        // Defensive: any stale forced id from a previous (failed) forced turn
        // must not survive into this turn's permission checks.
        self.forced_tool_ids.clear();

        if self.session.compaction.is_some() {
            if let Err(error) = self.run_session_health_probe() {
                return Err(RuntimeError::new(format!(
                    "Session health probe failed after compaction: {error}. \
                     The session may be in an inconsistent state. \
                     Consider starting a fresh session with /session new."
                )));
            }
        }

        self.record_turn_started(&user_input);

        let content_blocks = parse_input_content(&user_input);
        self.session
            .push_user_content(content_blocks)
            .map_err(|error| RuntimeError::new(error.to_string()))?;

        let mut assistant_messages = Vec::new();
        let mut tool_results = Vec::new();
        let mut prompt_cache_events = Vec::new();
        let mut iterations = 0;

        // Synthesize the forced tool_use assistant message (no model call).
        let forced_id = format!(
            "forced_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let forced_input_value: Value = serde_json::from_str(&forced_tool_input)
            .unwrap_or_else(|_| Value::String(forced_tool_input.clone()));
        // Anthropic extended thinking requires every assistant turn that carries
        // a `ToolUse` block to also echo back a thinking block on the follow-up
        // request. This synthesized turn never went through the model, so there
        // is no real reasoning to attach — inject a placeholder thinking block
        // so the request stays valid instead of failing with
        // `The content[].thinking in the thinking mode must be passed back to
        // the API`. `convert.rs` only echoes thinking blocks that carry a
        // signature, so a fixed placeholder signature is required here.
        let assistant = ConversationMessage::assistant(vec![
            ContentBlock::Thinking {
                thinking: format!(
                    "[forced delegation] Executing `{forced_tool_name}` on the user's behalf."
                ),
                signature: Some(FORCED_DELEGATION_THINKING_SIGNATURE.to_string()),
            },
            ContentBlock::ToolUse {
                id: forced_id.clone(),
                name: forced_tool_name.clone(),
                input: forced_input_value,
            },
        ]);
        self.session
            .push_message(assistant.clone())
            .map_err(|error| RuntimeError::new(error.to_string()))?;
        self.forced_tool_ids.insert(forced_id.clone());
        let result_message = self.execute_one_tool_use(
            forced_id.clone(),
            forced_tool_name.clone(),
            forced_tool_input,
            &mut prompter,
            0,
        )?;
        self.session
            .push_message(result_message.clone())
            .map_err(|error| RuntimeError::new(error.to_string()))?;
        self.forced_tool_ids.clear();

        // api_messages now reflects user + synthetic assistant + tool result.
        let mut api_messages = Arc::new(crate::context::filter_for_api_with_config(
            &self.session.messages,
            &self.compression_config,
        ));

        self.drive_turn_loop(
            &mut prompter,
            &mut api_messages,
            &mut assistant_messages,
            &mut tool_results,
            &mut prompt_cache_events,
            &mut iterations,
        )?;

        let auto_compaction = self.maybe_auto_compact();

        let summary = TurnSummary {
            assistant_messages,
            tool_results,
            prompt_cache_events,
            iterations,
            usage: self.usage_tracker.cumulative_usage(),
            auto_compaction,
        };
        self.record_turn_completed(&summary);

        Ok(summary)
    }

    #[must_use]
    pub fn compact(&self, config: CompactionConfig) -> CompactionResult {
        self.emit_lifecycle_hook("PreCompact");
        let result = compact_session(&self.session, config);
        self.emit_lifecycle_hook("PostCompact");
        result
    }

    #[must_use]
    pub fn estimated_tokens(&self) -> usize {
        estimate_session_tokens(&self.session)
    }

    #[must_use]
    pub fn usage(&self) -> &UsageTracker {
        &self.usage_tracker
    }

    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn api_client_mut(&mut self) -> &mut C {
        &mut self.api_client
    }

    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    #[must_use]
    pub fn fork_session(&self, branch_name: Option<String>) -> Session {
        self.session.fork(branch_name)
    }

    #[must_use]
    pub fn into_session(self) -> Session {
        self.session
    }

    fn maybe_auto_compact(&mut self) -> Option<AutoCompactionEvent> {
        if self.usage_tracker.cumulative_usage().input_tokens
            < self.auto_compaction_input_tokens_threshold
        {
            return None;
        }

        // Anti-thrashing hysteresis: only compact when the CURRENT session is
        // large enough to warrant it. After a compaction the session is a
        // summary plus a small preserved tail — far below the budget — so the
        // next turn naturally skips re-compaction instead of shredding the
        // session every single turn once the cumulative threshold is crossed.
        if estimate_session_tokens(&self.session)
            < self.compression_config.compact_max_estimated_tokens
        {
            return None;
        }

        // Anti-thrashing: if last compaction saved below threshold,
        // skip this auto-compaction. Reset the lock so next turn re-evaluates.
        if let Some(ratio) = self.session.compaction.as_ref().and_then(|c| c.last_savings_ratio)
        {
            if ratio < self.compression_config.antithrash_ratio {
                self.session.set_compaction_savings_ratio(None);
                return None;
            }
        }

        let base = CompactionConfig::from_config(&self.compression_config);
        let result = compact_session(
            &self.session,
            CompactionConfig {
                max_estimated_tokens: 0,
                ..base
            },
        );

        if result.removed_message_count == 0 {
            return None;
        }

        // Compute savings ratio.
        let total_before = estimate_session_tokens(&self.session);
        let total_after = estimate_session_tokens(&result.compacted_session);
        let compactable_tokens = total_before.saturating_sub(total_after);
        let savings_ratio = if total_before > 0 {
            compactable_tokens as f64 / total_before as f64
        } else {
            0.0
        };

        // Persist ratio on the compacted session for future anti-thrashing
        // checks. Order matters: the ratio must be written AFTER the session
        // reference is swapped, otherwise the write lands on the old session
        // object that is immediately discarded by the replacement below.
        self.session = result.compacted_session;
        self.session.set_compaction_savings_ratio(Some(savings_ratio));

        Some(AutoCompactionEvent {
            removed_message_count: result.removed_message_count,
            savings_ratio,
        })
    }

    fn init_image_store() -> Option<ImageStore> {
        let store_path = crate::config::default_config_home().join("images");
        ImageStore::try_new(&store_path).ok()
    }

    fn image_store(&self) -> Option<&ImageStore> {
        self.image_store.as_ref()
    }

    fn record_turn_started(&self, user_input: &str) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert(
            "user_input".to_string(),
            Value::String(user_input.to_string()),
        );
        session_tracer.record("turn_started", attributes);
    }

    fn record_assistant_iteration(
        &self,
        iteration: usize,
        assistant_message: &ConversationMessage,
        pending_tool_use_count: usize,
    ) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("iteration".to_string(), Value::from(iteration as u64));
        attributes.insert(
            "assistant_blocks".to_string(),
            Value::from(assistant_message.blocks.len() as u64),
        );
        attributes.insert(
            "pending_tool_use_count".to_string(),
            Value::from(pending_tool_use_count as u64),
        );
        session_tracer.record("assistant_iteration_completed", attributes);
    }

    fn record_tool_started(&self, iteration: usize, tool_name: &str) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("iteration".to_string(), Value::from(iteration as u64));
        attributes.insert(
            "tool_name".to_string(),
            Value::String(tool_name.to_string()),
        );
        session_tracer.record("tool_execution_started", attributes);
    }

    fn record_tool_finished(&self, iteration: usize, result_message: &ConversationMessage) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let Some(ContentBlock::ToolResult {
            tool_name,
            is_error,
            ..
        }) = result_message.blocks.first()
        else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("iteration".to_string(), Value::from(iteration as u64));
        attributes.insert("tool_name".to_string(), Value::String(tool_name.clone()));
        attributes.insert("is_error".to_string(), Value::Bool(*is_error));
        session_tracer.record("tool_execution_finished", attributes);
    }

    fn record_turn_completed(&self, summary: &TurnSummary) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert(
            "iterations".to_string(),
            Value::from(summary.iterations as u64),
        );
        attributes.insert(
            "assistant_messages".to_string(),
            Value::from(summary.assistant_messages.len() as u64),
        );
        attributes.insert(
            "tool_results".to_string(),
            Value::from(summary.tool_results.len() as u64),
        );
        attributes.insert(
            "prompt_cache_events".to_string(),
            Value::from(summary.prompt_cache_events.len() as u64),
        );
        session_tracer.record("turn_completed", attributes);
    }

    fn record_turn_failed(&self, iteration: usize, error: &RuntimeError) {
        let Some(session_tracer) = &self.session_tracer else {
            return;
        };

        let mut attributes = Map::new();
        attributes.insert("iteration".to_string(), Value::from(iteration as u64));
        attributes.insert("error".to_string(), Value::String(error.to_string()));
        session_tracer.record("turn_failed", attributes);
    }
}

/// Reads the automatic compaction threshold from the environment.
#[must_use]
pub fn auto_compaction_threshold_from_env() -> u32 {
    parse_auto_compaction_threshold(
        std::env::var(AUTO_COMPACTION_THRESHOLD_ENV_VAR)
            .ok()
            .as_deref(),
    )
}

#[must_use]
fn parse_auto_compaction_threshold(value: Option<&str>) -> u32 {
    value
        .and_then(|raw| raw.trim().parse::<u32>().ok())
        .filter(|threshold| *threshold > 0)
        .unwrap_or(DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD)
}

fn build_assistant_message(
    events: Vec<AssistantEvent>,
) -> Result<
    (
        ConversationMessage,
        Option<TokenUsage>,
        Vec<PromptCacheEvent>,
    ),
    RuntimeError,
> {
    let mut text = String::new();
    let mut blocks = Vec::new();
    let mut prompt_cache_events = Vec::new();
    let mut finished = false;
    let mut usage: Option<TokenUsage> = None;

    for event in events {
        match event {
            AssistantEvent::TextDelta(delta) => text.push_str(&delta),
            AssistantEvent::ToolUse { id, name, input } => {
                flush_text_block(&mut text, &mut blocks);
                blocks.push(ContentBlock::ToolUse { id, name, input });
            }
            AssistantEvent::Usage(value) => {
                usage = match usage {
                    Some(existing) => Some(TokenUsage {
                        input_tokens: existing.input_tokens + value.input_tokens,
                        output_tokens: existing.output_tokens + value.output_tokens,
                        cache_creation_input_tokens: existing.cache_creation_input_tokens
                            + value.cache_creation_input_tokens,
                        cache_read_input_tokens: existing.cache_read_input_tokens
                            + value.cache_read_input_tokens,
                    }),
                    None => Some(value),
                };
            }
            AssistantEvent::PromptCache(event) => prompt_cache_events.push(event),
            AssistantEvent::Thinking { text: thinking_text, signature } => {
                // Flush the OUTER accumulated text-delta content first (the
                // binding is named `thinking_text`, not `text`, so it does not
                // shadow the outer `text` accumulator), then insert the
                // thinking block verbatim.
                flush_text_block(&mut text, &mut blocks);
                blocks.push(ContentBlock::Thinking {
                    thinking: thinking_text,
                    signature,
                });
            }
            AssistantEvent::RedactedThinking { data } => {
                flush_text_block(&mut text, &mut blocks);
                blocks.push(ContentBlock::RedactedThinking { data });
            }
            AssistantEvent::MessageStop => {
                finished = true;
            }
            AssistantEvent::Image { data, mime_type } => {
                // Add image block
                flush_text_block(&mut text, &mut blocks);
                blocks.push(ContentBlock::Image {
                    mime_type,
                    data,
                    filename: None,
                });
            }
        }
    }

    flush_text_block(&mut text, &mut blocks);

    if !finished {
        return Err(RuntimeError::new(
            "assistant stream ended without a message stop event",
        ));
    }
    if blocks.is_empty() {
        return Err(RuntimeError::new("assistant stream produced no content"));
    }

    Ok((
        ConversationMessage::assistant_with_usage(blocks, usage),
        usage,
        prompt_cache_events,
    ))
}

fn flush_text_block(text: &mut String, blocks: &mut Vec<ContentBlock>) {
    if !text.is_empty() {
        blocks.push(ContentBlock::Text {
            text: std::mem::take(text),
        });
    }
}

/// Merge the tool-result messages produced for a single assistant tool_use
/// turn into one message. The Anthropic API requires all `tool_result` blocks
/// responding to an assistant turn to be in the single user message that
/// immediately follows it; emitting one message per result breaks that pairing
/// for parallel tool calls. `created_at` is taken from the earliest result so
/// time-based tool-result expiry (WebSearch/WebFetch TTL) stays conservative.
pub(crate) fn merge_tool_result_messages(results: Vec<ConversationMessage>) -> ConversationMessage {
    let mut blocks = Vec::new();
    let mut created_at = std::time::Instant::now();
    for (i, result) in results.into_iter().enumerate() {
        if i == 0 {
            created_at = result.created_at;
        }
        blocks.extend(result.blocks);
    }
    ConversationMessage {
        role: MessageRole::Tool,
        blocks,
        usage: None,
        created_at,
        cached_tokens: OnceLock::new(),
        cached_input_message: OnceLock::new(),
    }
}

/// Re-exported from [`crate::thinking::extract`] for backward compatibility
/// with code that imports `extract_embedded_tools` from the `runtime` crate
/// root. New code should import from `runtime::thinking::extract` directly.
pub use crate::thinking::extract::extract_embedded_tools;

fn format_hook_message(result: &HookRunResult, fallback: &str) -> String {
    if result.messages().is_empty() {
        fallback.to_string()
    } else {
        result.messages().join("\n")
    }
}

fn merge_hook_feedback(messages: &[String], output: String, is_error: bool) -> String {
    if messages.is_empty() {
        return output;
    }

    let mut sections = Vec::new();
    if !output.trim().is_empty() {
        sections.push(output);
    }
    let label = if is_error {
        "Hook feedback (error)"
    } else {
        "Hook feedback"
    };
    sections.push(format!("{label}:\n{}", messages.join("\n")));
    sections.join("\n\n")
}

type ToolHandler = Box<dyn FnMut(&str) -> Result<String, ToolError>>;

/// Simple in-memory tool executor for tests and lightweight integrations.
#[derive(Default)]
pub struct StaticToolExecutor {
    handlers: BTreeMap<String, ToolHandler>,
}

impl StaticToolExecutor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn register(
        mut self,
        tool_name: impl Into<String>,
        handler: impl FnMut(&str) -> Result<String, ToolError> + 'static,
    ) -> Self {
        self.handlers.insert(tool_name.into(), Box::new(handler));
        self
    }
}

impl ToolExecutor for StaticToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        self.handlers
            .get_mut(tool_name)
            .ok_or_else(|| ToolError::new(format!("unknown tool: {tool_name}")))?(input)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_assistant_message, parse_auto_compaction_threshold, ApiClient, ApiRequest,
        AssistantEvent, ConversationRuntime, PromptCacheEvent, RuntimeError,
        StaticToolExecutor, ToolExecutor, DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD,
    };
    use crate::compact::CompactionConfig;
    use crate::config::{RuntimeFeatureConfig, RuntimeHookConfig};
    use crate::permissions::{
        PermissionMode, PermissionPolicy, PermissionPromptDecision, PermissionPrompter,
        PermissionRequest,
    };
    use crate::prompt::{ProjectContext, SystemPromptBuilder};
    use crate::session::{ContentBlock, MessageRole, Session};
    use crate::usage::TokenUsage;
    use std::sync::OnceLock;
    use crate::ToolError;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::time::{SystemTime, UNIX_EPOCH};
    use telemetry::{MemoryTelemetrySink, SessionTracer, TelemetryEvent};

    struct ScriptedApiClient {
        call_count: usize,
    }

    impl ApiClient for ScriptedApiClient {
        fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
            self.call_count += 1;
            match self.call_count {
                1 => {
                    assert!(request
                        .messages
                        .iter()
                        .any(|message| message.role == MessageRole::User));
                    Ok(vec![
                        AssistantEvent::TextDelta("Let me calculate that.".to_string()),
                        AssistantEvent::ToolUse {
                            id: "tool-1".to_string(),
                            name: "add".to_string(),
                            input: serde_json::Value::String("2,2".to_string()),
                        },
                        AssistantEvent::Usage(TokenUsage {
                            input_tokens: 20,
                            output_tokens: 6,
                            cache_creation_input_tokens: 1,
                            cache_read_input_tokens: 2,
                        }),
                        AssistantEvent::MessageStop,
                    ])
                }
                2 => {
                    let last_message = request
                        .messages
                        .last()
                        .expect("tool result should be present");
                    assert_eq!(last_message.role, MessageRole::Tool);
                    Ok(vec![
                        AssistantEvent::TextDelta("The answer is 4.".to_string()),
                        AssistantEvent::Usage(TokenUsage {
                            input_tokens: 24,
                            output_tokens: 4,
                            cache_creation_input_tokens: 1,
                            cache_read_input_tokens: 3,
                        }),
                        AssistantEvent::PromptCache(PromptCacheEvent {
                            unexpected: true,
                            reason:
                                "cache read tokens dropped while prompt fingerprint remained stable"
                                    .to_string(),
                            previous_cache_read_input_tokens: 6_000,
                            current_cache_read_input_tokens: 1_000,
                            token_drop: 5_000,
                        }),
                        AssistantEvent::MessageStop,
                    ])
                }
                _ => unreachable!("extra API call"),
            }
        }
    }

    struct PromptAllowOnce;

    impl PermissionPrompter for PromptAllowOnce {
        fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision {
            assert_eq!(request.tool_name, "add");
            PermissionPromptDecision::Allow
        }
    }

    #[test]
    fn runs_user_to_tool_to_result_loop_end_to_end_and_tracks_usage() {
        let api_client = ScriptedApiClient { call_count: 0 };
        let tool_executor = StaticToolExecutor::new().register("add", |input| {
            let total = input
                .split(',')
                .map(|part| part.parse::<i32>().expect("input must be valid integer"))
                .sum::<i32>();
            Ok(total.to_string())
        });
        let permission_policy = PermissionPolicy::new(PermissionMode::WorkspaceWrite);
        let system_prompt = SystemPromptBuilder::new()
            .with_project_context(ProjectContext {
                cwd: PathBuf::from("/tmp/project"),
                current_date: "2026-03-31".to_string(),
                git_status: None,
                git_diff: None,
                git_context: None,
                instruction_files: Vec::new(),
            })
            .with_os("linux", "6.8")
            .build();
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            api_client,
            tool_executor,
            permission_policy,
            system_prompt,
        );

        let summary = runtime
            .run_turn("what is 2 + 2?", Some(&mut PromptAllowOnce))
            .expect("conversation loop should succeed");

        assert_eq!(summary.iterations, 2);
        assert_eq!(summary.assistant_messages.len(), 2);
        assert_eq!(summary.tool_results.len(), 1);
        assert_eq!(summary.prompt_cache_events.len(), 1);
        assert_eq!(runtime.session().messages.len(), 4);
        assert_eq!(summary.usage.output_tokens, 10);
        assert_eq!(summary.auto_compaction, None);
        assert!(matches!(
            runtime.session().messages[1].blocks[1],
            ContentBlock::ToolUse { .. }
        ));
        assert!(matches!(
            runtime.session().messages[2].blocks[0],
            ContentBlock::ToolResult {
                is_error: false,
                ..
            }
        ));
    }

    #[test]
    fn records_runtime_session_trace_events() {
        let sink = Arc::new(MemoryTelemetrySink::default());
        let tracer = SessionTracer::new("session-runtime", sink.clone());
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            ScriptedApiClient { call_count: 0 },
            StaticToolExecutor::new().register("add", |_input| Ok("4".to_string())),
            PermissionPolicy::new(PermissionMode::WorkspaceWrite),
            vec!["system".to_string()],
        )
        .with_session_tracer(tracer);

        runtime
            .run_turn("what is 2 + 2?", Some(&mut PromptAllowOnce))
            .expect("conversation loop should succeed");

        let events = sink.events();
        let trace_names = events
            .iter()
            .filter_map(|event| match event {
                TelemetryEvent::SessionTrace(trace) => Some(trace.name.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(trace_names.contains(&"turn_started"));
        assert!(trace_names.contains(&"assistant_iteration_completed"));
        assert!(trace_names.contains(&"tool_execution_started"));
        assert!(trace_names.contains(&"tool_execution_finished"));
        assert!(trace_names.contains(&"turn_completed"));
    }

    #[test]
    fn records_denied_tool_results_when_prompt_rejects() {
        struct RejectPrompter;
        impl PermissionPrompter for RejectPrompter {
            fn decide(&mut self, _request: &PermissionRequest) -> PermissionPromptDecision {
                PermissionPromptDecision::Deny {
                    reason: "not now".to_string(),
                }
            }
        }

        struct SingleCallApiClient;
        impl ApiClient for SingleCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                if request
                    .messages
                    .iter()
                    .any(|message| message.role == MessageRole::Tool)
                {
                    return Ok(vec![
                        AssistantEvent::TextDelta("I could not use the tool.".to_string()),
                        AssistantEvent::MessageStop,
                    ]);
                }
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "blocked".to_string(),
                        input: serde_json::Value::String("secret".to_string()),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut runtime = ConversationRuntime::new(
            Session::new(),
            SingleCallApiClient,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::WorkspaceWrite)
                .with_tool_requirement("blocked", PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        let summary = runtime
            .run_turn("use the tool", Some(&mut RejectPrompter))
            .expect("conversation should continue after denied tool");

        assert_eq!(summary.tool_results.len(), 1);
        assert!(matches!(
            &summary.tool_results[0].blocks[0],
            ContentBlock::ToolResult { is_error: true, output, .. } if output == "not now"
        ));
    }

    #[test]
    fn multiple_tool_uses_in_one_turn_merge_results_into_single_message() {
        struct TwoToolApiClient;
        impl ApiClient for TwoToolApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                if request
                    .messages
                    .iter()
                    .any(|message| message.role == MessageRole::Tool)
                {
                    return Ok(vec![
                        AssistantEvent::TextDelta("done.".to_string()),
                        AssistantEvent::MessageStop,
                    ]);
                }
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-a".to_string(),
                        name: "add".to_string(),
                        input: serde_json::json!("1,1"),
                    },
                    AssistantEvent::ToolUse {
                        id: "tool-b".to_string(),
                        name: "add".to_string(),
                        input: serde_json::json!("2,2"),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let tool_executor = StaticToolExecutor::new().register("add", |input| {
            let total: i32 = input
                .split(',')
                .map(|part| part.trim().parse::<i32>().unwrap_or(0))
                .sum();
            Ok(total.to_string())
        });

        let mut runtime = ConversationRuntime::new(
            Session::new(),
            TwoToolApiClient,
            tool_executor,
            PermissionPolicy::new(PermissionMode::WorkspaceWrite),
            vec!["system".to_string()],
        );

        let summary = runtime
            .run_turn("do two adds", Some(&mut PromptAllowOnce))
            .expect("conversation loop should succeed");

        // Anthropic requires every tool_result for a tool_use turn to live in
        // the single user message immediately following the assistant message.
        let tool_messages: Vec<_> = runtime
            .session()
            .messages
            .iter()
            .filter(|message| message.role == MessageRole::Tool)
            .collect();
        assert_eq!(
            tool_messages.len(),
            1,
            "both tool results must be merged into one user message"
        );
        assert_eq!(tool_messages[0].blocks.len(), 2);
        assert_eq!(summary.tool_results.len(), 2);
    }

    #[test]
    fn denies_tool_use_when_pre_tool_hook_blocks() {
        struct SingleCallApiClient;
        impl ApiClient for SingleCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                if request
                    .messages
                    .iter()
                    .any(|message| message.role == MessageRole::Tool)
                {
                    return Ok(vec![
                        AssistantEvent::TextDelta("blocked".to_string()),
                        AssistantEvent::MessageStop,
                    ]);
                }
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "blocked".to_string(),
                        input: serde_json::json!({"path": "secret.txt"}),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut runtime = ConversationRuntime::new_with_features(
            Session::new(),
            SingleCallApiClient,
            StaticToolExecutor::new().register("blocked", |_input| {
                panic!("tool should not execute when hook denies")
            }),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
            &RuntimeFeatureConfig::default().with_hooks(RuntimeHookConfig::new(
                vec![shell_snippet("printf 'blocked by hook'; exit 2")],
                Vec::new(),
                Vec::new(),
            )),
        );

        let summary = runtime
            .run_turn("use the tool", None)
            .expect("conversation should continue after hook denial");

        assert_eq!(summary.tool_results.len(), 1);
        let ContentBlock::ToolResult {
            is_error, output, ..
        } = &summary.tool_results[0].blocks[0]
        else {
            panic!("expected tool result block");
        };
        assert!(
            *is_error,
            "hook denial should produce an error result: {output}"
        );
        assert!(
            output.contains("denied tool") || output.contains("blocked by hook"),
            "unexpected hook denial output: {output:?}"
        );
    }

    #[test]
    fn denies_tool_use_when_pre_tool_hook_fails() {
        struct SingleCallApiClient;
        impl ApiClient for SingleCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                if request
                    .messages
                    .iter()
                    .any(|message| message.role == MessageRole::Tool)
                {
                    return Ok(vec![
                        AssistantEvent::TextDelta("failed".to_string()),
                        AssistantEvent::MessageStop,
                    ]);
                }
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "blocked".to_string(),
                        input: serde_json::json!({"path": "secret.txt"}),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        // given
        let mut runtime = ConversationRuntime::new_with_features(
            Session::new(),
            SingleCallApiClient,
            StaticToolExecutor::new().register("blocked", |_input| {
                panic!("tool should not execute when hook fails")
            }),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
            &RuntimeFeatureConfig::default().with_hooks(RuntimeHookConfig::new(
                vec![shell_snippet("printf 'broken hook'; exit 1")],
                Vec::new(),
                Vec::new(),
            )),
        );

        // when
        let summary = runtime
            .run_turn("use the tool", None)
            .expect("conversation should continue after hook failure");

        // then
        assert_eq!(summary.tool_results.len(), 1);
        let ContentBlock::ToolResult {
            is_error, output, ..
        } = &summary.tool_results[0].blocks[0]
        else {
            panic!("expected tool result block");
        };
        assert!(
            *is_error,
            "hook failure should produce an error result: {output}"
        );
        assert!(
            output.contains("exited with status 1") || output.contains("broken hook"),
            "unexpected hook failure output: {output:?}"
        );
    }

    #[test]
    fn appends_post_tool_hook_feedback_to_tool_result() {
        struct TwoCallApiClient {
            calls: usize,
        }

        impl ApiClient for TwoCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                self.calls += 1;
                match self.calls {
                    1 => Ok(vec![
                        AssistantEvent::ToolUse {
                            id: "tool-1".to_string(),
                            name: "add".to_string(),
                            input: serde_json::json!({"lhs": 2, "rhs": 2}),
                        },
                        AssistantEvent::MessageStop,
                    ]),
                    2 => {
                        assert!(request
                            .messages
                            .iter()
                            .any(|message| message.role == MessageRole::Tool));
                        Ok(vec![
                            AssistantEvent::TextDelta("done".to_string()),
                            AssistantEvent::MessageStop,
                        ])
                    }
                    _ => unreachable!("extra API call"),
                }
            }
        }

        let mut runtime = ConversationRuntime::new_with_features(
            Session::new(),
            TwoCallApiClient { calls: 0 },
            StaticToolExecutor::new().register("add", |_input| Ok("4".to_string())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
            &RuntimeFeatureConfig::default().with_hooks(RuntimeHookConfig::new(
                vec![shell_snippet("printf 'pre hook ran'")],
                vec![shell_snippet("printf 'post hook ran'")],
                Vec::new(),
            )),
        );

        let summary = runtime
            .run_turn("use add", None)
            .expect("tool loop succeeds");

        assert_eq!(summary.tool_results.len(), 1);
        let ContentBlock::ToolResult {
            is_error, output, ..
        } = &summary.tool_results[0].blocks[0]
        else {
            panic!("expected tool result block");
        };
        assert!(
            !*is_error,
            "post hook should preserve non-error result: {output:?}"
        );
        assert!(
            output.contains('4'),
            "tool output missing value: {output:?}"
        );
        assert!(
            output.contains("pre hook ran"),
            "tool output missing pre hook feedback: {output:?}"
        );
        assert!(
            output.contains("post hook ran"),
            "tool output missing post hook feedback: {output:?}"
        );
    }

    #[test]
    fn appends_post_tool_use_failure_hook_feedback_to_tool_result() {
        struct TwoCallApiClient {
            calls: usize,
        }

        impl ApiClient for TwoCallApiClient {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                self.calls += 1;
                match self.calls {
                    1 => Ok(vec![
                        AssistantEvent::ToolUse {
                            id: "tool-1".to_string(),
                            name: "fail".to_string(),
                            input: serde_json::json!({"path": "README.md"}),
                        },
                        AssistantEvent::MessageStop,
                    ]),
                    2 => {
                        assert!(request
                            .messages
                            .iter()
                            .any(|message| message.role == MessageRole::Tool));
                        Ok(vec![
                            AssistantEvent::TextDelta("done".to_string()),
                            AssistantEvent::MessageStop,
                        ])
                    }
                    _ => unreachable!("extra API call"),
                }
            }
        }

        // given
        let mut runtime = ConversationRuntime::new_with_features(
            Session::new(),
            TwoCallApiClient { calls: 0 },
            StaticToolExecutor::new()
                .register("fail", |_input| Err(ToolError::new("tool exploded"))),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
            &RuntimeFeatureConfig::default().with_hooks(RuntimeHookConfig::new(
                Vec::new(),
                vec![shell_snippet("printf 'post hook should not run'")],
                vec![shell_snippet("printf 'failure hook ran'")],
            )),
        );

        // when
        let summary = runtime
            .run_turn("use fail", None)
            .expect("tool loop succeeds");

        // then
        assert_eq!(summary.tool_results.len(), 1);
        let ContentBlock::ToolResult {
            is_error, output, ..
        } = &summary.tool_results[0].blocks[0]
        else {
            panic!("expected tool result block");
        };
        assert!(
            *is_error,
            "failure hook path should preserve error result: {output:?}"
        );
        assert!(
            output.contains("tool exploded"),
            "tool output missing failure reason: {output:?}"
        );
        assert!(
            output.contains("failure hook ran"),
            "tool output missing failure hook feedback: {output:?}"
        );
        assert!(
            !output.contains("post hook should not run"),
            "normal post hook should not run on tool failure: {output:?}"
        );
    }

    #[test]
    fn reconstructs_usage_tracker_from_restored_session() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut session = Session::new();
        session
            .messages
            .push(crate::session::ConversationMessage::assistant_with_usage(
                vec![ContentBlock::Text {
                    text: "earlier".to_string(),
                }],
                Some(TokenUsage {
                    input_tokens: 11,
                    output_tokens: 7,
                    cache_creation_input_tokens: 2,
                    cache_read_input_tokens: 1,
                }),
            ));

        let runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        assert_eq!(runtime.usage().turns(), 1);
        assert_eq!(runtime.usage().cumulative_usage().total_tokens(), 21);
    }

    #[test]
    fn compacts_session_after_turns() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut runtime = ConversationRuntime::new(
            Session::new(),
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );
        runtime.run_turn("a", None).expect("turn a");
        runtime.run_turn("b", None).expect("turn b");
        runtime.run_turn("c", None).expect("turn c");

        let result = runtime.compact(CompactionConfig {
            preserve_recent_messages: 2,
            preserve_recent_tokens: 1,
            max_estimated_tokens: 1,
            preserve_last_n_turns: 0,
            summary_budget: None,
        });
        assert!(result.summary.contains("Conversation summary"));
        assert_eq!(
            result.compacted_session.messages[0].role,
            MessageRole::System
        );
        assert_eq!(
            result.compacted_session.session_id,
            runtime.session().session_id
        );
        assert!(result.compacted_session.compaction.is_some());
    }

    #[test]
    fn persists_conversation_turn_messages_to_jsonl_session() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let path = temp_session_path("persisted-turn");
        let session = Session::new().with_persistence_path(path.clone());
        let mut runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        runtime
            .run_turn("persist this turn", None)
            .expect("turn should succeed");

        let restored = Session::load_from_path(&path).expect("persisted session should reload");
        fs::remove_file(&path).expect("temp session file should be removable");

        assert_eq!(restored.messages.len(), 2);
        assert_eq!(restored.messages[0].role, MessageRole::User);
        assert_eq!(restored.messages[1].role, MessageRole::Assistant);
        assert_eq!(restored.session_id, runtime.session().session_id);
    }

    #[test]
    fn forks_runtime_session_without_mutating_original() {
        let mut session = Session::new();
        session
            .push_user_text("branch me")
            .expect("message should append");

        let runtime = ConversationRuntime::new(
            session.clone(),
            ScriptedApiClient { call_count: 0 },
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        let forked = runtime.fork_session(Some("alt-path".to_string()));

        assert_eq!(forked.messages, session.messages);
        assert_ne!(forked.session_id, session.session_id);
        assert_eq!(
            forked
                .fork
                .as_ref()
                .map(|fork| (fork.parent_session_id.as_str(), fork.branch_name.as_deref())),
            Some((session.session_id.as_str(), Some("alt-path")))
        );
        assert!(runtime.session().fork.is_none());
    }

    fn temp_session_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("runtime-conversation-{label}-{nanos}.json"))
    }

    #[cfg(windows)]
    fn shell_snippet(script: &str) -> String {
        let mut result = script.to_string();
        result = result.replace("printf '%s' ", "echo ");
        result = result.replace("printf ", "echo ");
        result = result.replace('\'', "");
        let parts: Vec<&str> = result.split(';').collect();
        if parts.len() > 1 {
            result = parts.join(" &");
        }
        result = result.replace(">&2", "1>&2");
        result
    }

    #[cfg(not(windows))]
    fn shell_snippet(script: &str) -> String {
        script.to_string()
    }

    #[test]
    fn auto_compacts_when_cumulative_input_threshold_is_crossed() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::Usage(TokenUsage {
                        input_tokens: 120_000,
                        output_tokens: 4,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    }),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut session = Session::new();
        session.messages = vec![
            crate::session::ConversationMessage::user_text(
                "one: Write a script to parse the config file and extract all connection strings.",
            ),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "two: I wrote a Python script that reads the YAML config and outputs connection strings. It handles both TCP and UDP endpoints.".to_string(),
            }]),
            crate::session::ConversationMessage::user_text(
                "three: Update the deployment pipeline to include the new service.",
            ),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "four: I modified the CI/CD YAML to add the service build, test, and deploy stages.".to_string(),
            }]),
            crate::session::ConversationMessage::user_text("five: Check the metrics dashboard for anomalies."),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "six: I reviewed the dashboard and found no anomalies in the last 24 hours.".to_string(),
            }]),
            crate::session::ConversationMessage::user_text("seven: Review the latest PR for the auth module."),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "eight: I reviewed the PR. The changes look good but there's a missing null check.".to_string(),
            }]),
        ];

        let mut runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_auto_compaction_input_tokens_threshold(100_000);
        // The small test session must cross the current-session token budget,
        // otherwise the hysteresis gate (added with the anti-thrash fix) skips
        // compaction entirely even though the cumulative threshold was crossed.
        runtime.compression_config.compact_max_estimated_tokens = 1;

        let summary = runtime
            .run_turn("trigger", None)
            .expect("turn should succeed");

        assert_eq!(
            summary.auto_compaction.map(|e| e.removed_message_count),
            Some(7)
        );
        assert!(
            summary
                .auto_compaction
                .map_or(false, |e| e.savings_ratio >= 0.0),
            "savings_ratio should be non-negative"
        );
        assert_eq!(runtime.session().messages[0].role, MessageRole::System);
    }

    #[test]
    fn does_not_recompact_when_session_is_small_after_compaction() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::Usage(TokenUsage {
                        input_tokens: 120_000,
                        output_tokens: 4,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    }),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        // Each message is ~2000 chars so the pre-compaction session clearly
        // exceeds the token budget while the compacted session (summary + a
        // single preserved tail message) stays well below it.
        let long_prompt = format!(
            "Write a script that parses the YAML configuration file and extracts every connection string, handling both TCP and UDP endpoints, deduplicating repeated entries, and logging each step with a timestamp. {}",
            "The parser must also resolve environment variable references, validate the port range, and skip comments. ".repeat(15)
        );
        let long_answer = format!(
            "I wrote a Python script that reads the YAML config and outputs connection strings. It handles both TCP and UDP endpoints, deduplicates repeated entries, resolves env vars, validates ports, and logs each step. {}",
            "The script is fully tested against the fixtures and passes all cases. ".repeat(15)
        );

        let mut session = Session::new();
        session.messages = vec![
            crate::session::ConversationMessage::user_text(&long_prompt),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: long_answer.clone(),
            }]),
            crate::session::ConversationMessage::user_text(&long_prompt),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: long_answer.clone(),
            }]),
            crate::session::ConversationMessage::user_text(&long_prompt),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: long_answer.clone(),
            }]),
            crate::session::ConversationMessage::user_text(&long_prompt),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: long_answer.clone(),
            }]),
        ];

        let mut runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            StaticToolExecutor::new().register("glob_search", |_input| {
                Ok(r#"{"files": []}"#.to_string())
            }),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_auto_compaction_input_tokens_threshold(100_000);
        // Token budget sits between the pre-compaction session size (~2200
        // tokens) and the post-compaction size (summary + tail, well under),
        // so turn 1 compacts but turn 2 must be suppressed by the gate.
        runtime.compression_config.compact_max_estimated_tokens = 2_000;
        // Keep only one tail message so the compacted session stays small.
        runtime.compression_config.compact_preserve_recent_messages = 1;

        let first = runtime
            .run_turn("trigger", None)
            .expect("first turn should succeed");
        assert!(
            first.auto_compaction.is_some(),
            "first turn should auto-compact once the threshold is crossed"
        );

        // Second turn: cumulative input is still above the threshold, but the
        // session was already shrunk to a summary + tail — the hysteresis gate
        // must suppress re-compaction instead of shredding the session again.
        let second = runtime
            .run_turn("trigger again", None)
            .expect("second turn should succeed");
        assert!(
            second.auto_compaction.is_none(),
            "should not re-compact a session that is still small after compaction"
        );
    }

    #[test]
    fn skips_auto_compaction_below_threshold() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::Usage(TokenUsage {
                        input_tokens: 99_999,
                        output_tokens: 4,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                    }),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut runtime = ConversationRuntime::new(
            Session::new(),
            SimpleApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_auto_compaction_input_tokens_threshold(100_000);

        let summary = runtime
            .run_turn("trigger", None)
            .expect("turn should succeed");
        assert_eq!(summary.auto_compaction, None);
        assert_eq!(runtime.session().messages.len(), 2);
    }

    #[test]
    fn auto_compaction_threshold_defaults_and_parses_values() {
        assert_eq!(
            parse_auto_compaction_threshold(None),
            DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD
        );
        assert_eq!(parse_auto_compaction_threshold(Some("4321")), 4321);
        assert_eq!(
            parse_auto_compaction_threshold(Some("0")),
            DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD
        );
        assert_eq!(
            parse_auto_compaction_threshold(Some("not-a-number")),
            DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD
        );
    }

    #[test]
    fn compaction_health_probe_blocks_turn_when_tool_executor_is_broken() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                panic!("API should not run when health probe fails");
            }
        }

        let mut session = Session::new();
        session.record_compaction("summarized earlier work", 4);
        session
            .push_user_text("previous message")
            .expect("message should append");

        let tool_executor = StaticToolExecutor::new().register("glob_search", |_input| {
            Err(ToolError::new("transport unavailable"))
        });
        let mut runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            tool_executor,
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        let error = runtime
            .run_turn("trigger", None)
            .expect_err("health probe failure should abort the turn");
        assert!(
            error
                .to_string()
                .contains("Session health probe failed after compaction"),
            "unexpected error: {error}"
        );
        assert!(
            error.to_string().contains("transport unavailable"),
            "expected underlying probe error: {error}"
        );
    }

    #[test]
    fn compaction_health_probe_skips_empty_compacted_session() {
        struct SimpleApi;
        impl ApiClient for SimpleApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut session = Session::new();
        session.record_compaction("fresh summary", 2);

        let tool_executor = StaticToolExecutor::new().register("glob_search", |_input| {
            Err(ToolError::new(
                "glob_search should not run for an empty compacted session",
            ))
        });
        let mut runtime = ConversationRuntime::new(
            session,
            SimpleApi,
            tool_executor,
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        let summary = runtime
            .run_turn("trigger", None)
            .expect("empty compacted session should not fail health probe");
        assert_eq!(summary.auto_compaction, None);
        assert_eq!(runtime.session().messages.len(), 2);
    }

    #[test]
    fn build_assistant_message_requires_message_stop_event() {
        // given
        let events = vec![AssistantEvent::TextDelta("hello".to_string())];

        // when
        let error = build_assistant_message(events)
            .expect_err("assistant messages should require a stop event");

        // then
        assert!(error
            .to_string()
            .contains("assistant stream ended without a message stop event"));
    }

    #[test]
    fn build_assistant_message_requires_content() {
        // given
        let events = vec![AssistantEvent::MessageStop];

        // when
        let error =
            build_assistant_message(events).expect_err("assistant messages should require content");

        // then
        assert!(error
            .to_string()
            .contains("assistant stream produced no content"));
    }

    #[test]
    fn build_assistant_message_keeps_thinking_block_verbatim() {
        // given — text delta before the thinking block must stay a Text block
        // and the thinking content (with signature) must round-trip verbatim.
        let events = vec![
            AssistantEvent::TextDelta("Let me reason.".to_string()),
            AssistantEvent::Thinking {
                text: "I should check the API contract.".to_string(),
                signature: Some("sig123".to_string()),
            },
            AssistantEvent::TextDelta("Now the answer.".to_string()),
            AssistantEvent::MessageStop,
        ];

        // when
        let (message, _, _) = build_assistant_message(events).expect("message should build");

        // then
        assert_eq!(message.blocks.len(), 3);
        assert!(matches!(
            &message.blocks[0],
            ContentBlock::Text { text } if text == "Let me reason."
        ));
        assert!(matches!(
            &message.blocks[1],
            ContentBlock::Thinking { thinking, signature }
                if thinking == "I should check the API contract."
                    && signature.as_deref() == Some("sig123")
        ));
        assert!(matches!(
            &message.blocks[2],
            ContentBlock::Text { text } if text == "Now the answer."
        ));
    }

    #[test]
    fn build_assistant_message_keeps_redacted_thinking_block_verbatim() {
        // given — a redacted thinking block must round-trip with its ciphertext
        // so the tool-use round-trip can echo it back to the API unchanged.
        let events = vec![
            AssistantEvent::RedactedThinking {
                data: "ciphertext_blob_xyz".to_string(),
            },
            AssistantEvent::MessageStop,
        ];

        // when
        let (message, _, _) = build_assistant_message(events).expect("message should build");

        // then
        assert_eq!(message.blocks.len(), 1);
        assert!(matches!(
            &message.blocks[0],
            ContentBlock::RedactedThinking { data }
                if data == "ciphertext_blob_xyz"
        ));
    }

    #[test]
    fn static_tool_executor_rejects_unknown_tools() {
        // given
        let mut executor = StaticToolExecutor::new();

        // when
        let error = executor
            .execute("missing", "{}")
            .expect_err("unregistered tools should fail");

        // then
        assert_eq!(error.to_string(), "unknown tool: missing");
    }

    #[test]
    fn run_turn_errors_when_max_iterations_is_exceeded() {
        struct LoopingApi;

        impl ApiClient for LoopingApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "echo".to_string(),
                        input: serde_json::Value::String("payload".to_string()),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        // given
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            LoopingApi,
            StaticToolExecutor::new().register("echo", |input| Ok(input.to_string())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_max_iterations(1);

        // when
        let error = runtime
            .run_turn("loop", None)
            .expect_err("conversation loop should stop after the configured limit");

        // then
        assert!(error
            .to_string()
            .contains("conversation loop exceeded the maximum number of iterations"));
    }

    #[test]
    fn run_turn_aborts_when_cancel_signal_is_set() {
        struct LoopingApi;
        impl ApiClient for LoopingApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "echo".to_string(),
                        input: serde_json::Value::String("payload".to_string()),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let cancel = Arc::new(AtomicBool::new(true));

        // A cancelled runtime must refuse to run a turn rather than looping
        // forever; the worker-thread reap path in `wait_for_agent` relies on
        // this check at every iteration boundary.
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            LoopingApi,
            StaticToolExecutor::new().register("echo", |input| Ok(input.to_string())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        )
        .with_max_iterations(usize::MAX)
        .with_cancel_signal(Arc::clone(&cancel));

        let error = runtime
            .run_turn("loop", None)
            .expect_err("cancelled runtime should abort the turn");

        assert!(
            error.to_string().contains("cancelled"),
            "expected a cancellation error, got: {error}"
        );
    }

    #[test]
    fn runtime_error_identifies_balance_insufficient_messages() {
        let english = RuntimeError::new(
            "api returned 429 (insufficient_quota): Your account balance is insufficient",
        );
        assert!(english.is_balance_error(), "insufficient_quota should flag balance");

        let chinese = RuntimeError::new("api returned 429 (rate_limit_error): 余额不足");
        assert!(chinese.is_balance_error(), "余额不足 should flag balance");

        let payment = RuntimeError::new("api returned 402 (payment required): top up your account");
        assert!(payment.is_balance_error(), "payment required should flag balance");

        let credit_balance =
            RuntimeError::new("api returned 429 (rate_limit_error): Your credit balance is too low to access the Anthropic API");
        assert!(
            credit_balance.is_balance_error(),
            "credit balance wording should flag balance"
        );

        let unrelated = RuntimeError::new("api returned 500 (api_error): boom");
        assert!(
            !unrelated.is_balance_error(),
            "unrelated provider errors must not flag balance"
        );
    }

    #[test]
    fn run_turn_propagates_api_errors() {
        struct FailingApi;
        impl ApiClient for FailingApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Err(RuntimeError::new("upstream failed"))
            }
        }

        // given
        let mut runtime = ConversationRuntime::new(
            Session::new(),
            FailingApi,
            StaticToolExecutor::new(),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        // when
        let error = runtime
            .run_turn("hello", None)
            .expect_err("API failures should propagate");

        // then
        assert_eq!(error.to_string(), "upstream failed");
    }

    #[test]
    fn context_window_error_compacts_and_retries_instead_of_wiping() {
        struct FlakyApi {
            calls: u32,
        }
        impl ApiClient for FlakyApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                // First call: context-window error. Second call: succeeds.
                self.calls += 1;
                if self.calls == 1 {
                    return Err(RuntimeError::new(
                        "error: maximum context length exceeded for this request",
                    ));
                }
                Ok(vec![
                    AssistantEvent::TextDelta("recovered".to_string()),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut session = Session::new();
        session.messages = vec![
            crate::session::ConversationMessage::user_text(
                "one: Write a script to parse the config file and extract all connection strings.",
            ),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "two: I wrote a Python script that reads the YAML config and outputs connection strings.".to_string(),
            }]),
            crate::session::ConversationMessage::user_text(
                "three: Update the deployment pipeline to include the new service.",
            ),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "four: I modified the CI/CD YAML to add the service build, test, and deploy stages.".to_string(),
            }]),
        ];

        let mut runtime = ConversationRuntime::new(
            session,
            FlakyApi { calls: 0 },
            StaticToolExecutor::new().register("glob_search", |_input| {
                Ok(r#"{"files": []}"#.to_string())
            }),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        let summary = runtime
            .run_turn("trigger", None)
            .expect("context-window error should be recovered by compaction");

        // The turn must complete with assistant output, not silently return empty.
        assert!(!summary.assistant_messages.is_empty());
        assert!(
            summary
                .assistant_messages
                .last()
                .is_some_and(|m| m.blocks.iter().any(|b| matches!(b, ContentBlock::Text { text } if text == "recovered"))),
            "expected the recovered assistant text in the summary"
        );
        // The session must still contain the prior conversation (as summary + tail),
        // NOT be wiped to a single empty user message.
        let session = runtime.session();
        assert!(
            session.messages.len() >= 2,
            "session should preserve summary + tail after context-window recovery, got {} messages",
            session.messages.len()
        );
        assert!(
            session.compaction.is_some(),
            "compaction should have been recorded during context-window recovery"
        );
    }

    #[test]
    fn context_window_error_surfaces_real_error_when_nothing_removable() {
        struct TinyApi;
        impl ApiClient for TinyApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                Err(RuntimeError::new(
                    "error: max_tokens exceeded: this is not a context-window issue",
                ))
            }
        }

        let mut session = Session::new();
        session.messages = vec![
            crate::session::ConversationMessage::user_text("hello"),
            crate::session::ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "hi".to_string(),
            }]),
        ];

        let mut runtime = ConversationRuntime::new(
            session,
            TinyApi,
            StaticToolExecutor::new().register("glob_search", |_input| {
                Ok(r#"{"files": []}"#.to_string())
            }),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        let error = runtime
            .run_turn("trigger", None)
            .expect_err("a non-context-window error must propagate, not be swallowed");

        assert!(
            error.to_string().contains("max_tokens exceeded"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn tool_result_mutation_invalidates_token_cache() {
        // Regression (F-5): the WebFetch dedup path mutates ToolResult.output
        // in place on the session messages. The cached token estimate and the
        // cached wire message must be invalidated, otherwise the auto-compaction
        // budget check keeps using the stale pre-mutation (inflated) value.
        struct WebFetchApi {
            calls: u32,
        }
        impl ApiClient for WebFetchApi {
            fn stream(
                &mut self,
                _request: ApiRequest,
            ) -> Result<Vec<AssistantEvent>, RuntimeError> {
                self.calls += 1;
                // The prior WebFetch ToolResult already has role=Tool and is
                // present in the FIRST request, so a role-based gate would fire
                // on call #1. Branch on the call count instead: call 1 emits the
                // ToolUse that triggers the dedup; call 2 (after the tool
                // result was appended) finishes with text.
                if self.calls > 1 {
                    return Ok(vec![
                        AssistantEvent::TextDelta("fetched".to_string()),
                        AssistantEvent::MessageStop,
                    ]);
                }
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "wf-1".to_string(),
                        name: "WebFetch".to_string(),
                        input: serde_json::json!({ "url": "https://example.com" }),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let mut session = Session::new();
        // A prior turn already fetched the same URL with a large payload.
        let big_payload = format!(
            "{{\"url\":\"https://example.com\",\"content\":\"{}\"}}",
            "x".repeat(20_000)
        );
        let prior = crate::session::ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "wf-0".to_string(),
                tool_name: "WebFetch".to_string(),
                output: big_payload,
                is_error: false,
            }],
            usage: None,
            created_at: std::time::Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        };
        // Pre-populate the cache as if a large estimate had already run.
        prior.cached_tokens.set(10_000).ok();
        session.messages.push(prior);

        let mut runtime = ConversationRuntime::new(
            session,
            WebFetchApi { calls: 0 },
            StaticToolExecutor::new()
                .register("glob_search", |_input| Ok(r#"{"files": []}"#.to_string()))
                .register("WebFetch", |_input| {
                    Ok(r#"{"url":"https://example.com","content":"fetched body"}"#.to_string())
                }),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        runtime
            .run_turn("fetch the page", None)
            .expect("turn should succeed");

        // The dedup path should have emptied the prior output (same URL), so the
        // cached 10_000 estimate must be gone and the recomputed estimate must
        // reflect the now-empty payload.
        let session = runtime.session();
        let prior_msg = session
            .messages
            .iter()
            .find(|message| {
                message
                    .blocks
                    .iter()
                    .any(|b| matches!(b, ContentBlock::ToolResult { tool_name, .. } if tool_name == "WebFetch"))
            })
            .expect("prior WebFetch tool result should still be in the session");
        assert!(
            prior_msg.cached_tokens.get().is_none(),
            "cached_tokens must be invalidated after in-place ToolResult mutation"
        );
        let estimated = crate::compact::estimate_message_tokens(prior_msg);
        assert!(
            estimated < 10_000,
            "recomputed estimate should reflect the emptied output, got {estimated}"
        );
    }

    #[test]
    fn run_turn_forced_attaches_thinking_block_to_synthesized_tool_use() {
        // given a runtime whose upstream client records the assistant message
        // list it receives and replies with plain text
        struct RecordingApi {
            assistant_messages: Vec<crate::ConversationMessage>,
        }
        impl ApiClient for RecordingApi {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                self.assistant_messages = request
                    .messages
                    .iter()
                    .filter(|message| message.role == MessageRole::Assistant)
                    .cloned()
                    .collect();
                Ok(vec![
                    AssistantEvent::TextDelta("done".to_string()),
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let session = Session::new();
        let mut runtime = ConversationRuntime::new(
            session,
            RecordingApi {
                assistant_messages: Vec::new(),
            },
            StaticToolExecutor::new()
                .register("Agent", |_input| Ok("delegated ok".to_string())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        runtime
            .run_turn_forced(
                "delegate this task",
                "Agent".to_string(),
                "{\"prompt\":\"x\"}".to_string(),
                None,
            )
            .expect("forced turn should succeed");

        // The synthesized assistant turn must carry a ToolUse block paired with
        // a thinking block, otherwise the extended-thinking round-trip contract
        // is violated and the API rejects the request with
        // `The content[].thinking in the thinking mode must be passed back to
        // the API`.
        let api = runtime.api_client_mut();
        let RecordingApi {
            assistant_messages,
        } = api;
        let synthesized = assistant_messages
            .iter()
            .find(|message| {
                message
                    .blocks
                    .iter()
                    .any(|b| matches!(b, ContentBlock::ToolUse { name, .. } if name == "Agent"))
            })
            .expect("synthesized Agent tool_use turn should reach the upstream client");
        assert!(
            synthesized.blocks.iter().any(|b| matches!(
                b,
                ContentBlock::Thinking {
                    signature: Some(_),
                    ..
                }
            )),
            "synthesized tool_use turn must carry a signed thinking block: {assistant_messages:?}"
        );
    }

    #[test]
    fn run_turn_keeps_extended_thinking_for_normal_model_tool_round_trip() {
        // given a runtime whose upstream client emits a model tool round-trip
        // (thinking + tool_use in the same assistant turn)
        struct ToolRoundTripApi;
        impl ApiClient for ToolRoundTripApi {
            fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
                let has_thinking_block = request.messages.iter().any(|message| {
                    message.role == MessageRole::Assistant
                        && message
                            .blocks
                            .iter()
                            .any(|b| matches!(b, ContentBlock::Thinking { .. }))
                });
                if has_thinking_block {
                    return Ok(vec![
                        AssistantEvent::TextDelta("final answer".to_string()),
                        AssistantEvent::MessageStop,
                    ]);
                }
                Ok(vec![
                    AssistantEvent::Thinking {
                        text: "Let me compute.".to_string(),
                        signature: Some("sig123".to_string()),
                    },
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "add".to_string(),
                        input: serde_json::json!({ "a": 2, "b": 2 }),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
        }

        let session = Session::new();
        let mut runtime = ConversationRuntime::new(
            session,
            ToolRoundTripApi,
            StaticToolExecutor::new()
                .register("add", |_input| Ok("4".to_string())),
            PermissionPolicy::new(PermissionMode::DangerFullAccess),
            vec!["system".to_string()],
        );

        runtime
            .run_turn("compute 2+2", None)
            .expect("normal tool round-trip should succeed");

        // The model's own thinking block (with signature) is echoed back, so the
        // thinking-mode contract is satisfied by model output — no placeholder
        // injection is needed and thinking stays enabled.
        let session = runtime.session();
        let assistant_turn = session
            .messages
            .iter()
            .find(|message| {
                message.role == MessageRole::Assistant
                    && message
                        .blocks
                        .iter()
                        .any(|b| matches!(b, ContentBlock::ToolUse { .. }))
            })
            .expect("model tool turn should be recorded");
        assert!(
            assistant_turn
                .blocks
                .iter()
                .any(|b| matches!(b, ContentBlock::Thinking { .. })),
            "model tool turn must carry its original thinking block"
        );
    }
}
