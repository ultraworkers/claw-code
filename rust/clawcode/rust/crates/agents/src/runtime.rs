use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};

use api::{
    convert_messages_cached, convert_messages_inner, detect_provider_kind, is_local_inference,
    max_tokens_for_model, render_tools_block, resolve_model_alias, ApiError, ContentBlockDelta,
    InputMessage, MessageRequest, MessageResponse, OutputContentBlock, ProviderClient,
    ProviderKind, StreamEvent as ApiStreamEvent, ToolChoice, ToolDefinition,
};
use runtime::{
    extract_embedded_tools, load_system_prompt,
    ApiClient, ApiRequest, AssistantEvent, ConfigLoader, ConversationRuntime,
    PermissionMode, PermissionOutcome, PermissionPolicy, ProviderFallbackConfig,
    RuntimeError, Session, ThinkParser, ToolError, ToolExecutor,
};
use serde_json::Value;

use crate::types::{
    push_progress_event, set_current_activity, AgentJob, AgentStatus, SharedProgress,
    SubagentProgressEvent,
};

// Global hook for the tools crate to register its real tool executor.
static GLOBAL_TOOL_EXECUTOR: OnceLock<
    Box<dyn Fn(&str, &Value, Option<&PermissionPolicy>) -> Result<String, String> + Send + Sync>,
> = OnceLock::new();

/// Shared tokio runtime for all subagent execution. Initialized once
/// at startup so every spawned agent reuses the same thread pool
/// instead of creating its own `Runtime` (which is expensive and
/// risks "Cannot start a runtime from within a runtime" panics).
static GLOBAL_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn init_global_runtime() {
    GLOBAL_RUNTIME.get_or_init(|| {
        tokio::runtime::Runtime::new().expect("failed to create global tokio runtime")
    });
}

pub fn register_tool_executor(
    f: Box<
        dyn Fn(&str, &Value, Option<&PermissionPolicy>) -> Result<String, String>
            + Send
            + Sync,
    >,
) -> Result<(), String> {
    GLOBAL_TOOL_EXECUTOR
        .set(f)
        .map_err(|_| String::from("tool executor already registered"))
}

pub type RuntimeToolExecutorFn = dyn Fn(&str, &Value, Option<&PermissionPolicy>) -> Result<String, String>
    + Send
    + Sync;

/// Global hook that lets sub-agents execute runtime tools (MCP, plugin) that
/// the built-in [`GLOBAL_TOOL_EXECUTOR`] cannot handle. The CLI registers this
/// after it builds the MCP state and plugin registry, capturing its own
/// `mcp_state` and `GlobalToolRegistry` clones. See
/// `SubagentToolExecutor::execute` for the fallback routing.
static GLOBAL_RUNTIME_EXECUTOR: OnceLock<Box<RuntimeToolExecutorFn>> = OnceLock::new();

/// Extra tool definitions (MCP discovery/wrapper + plugin tools) advertised to
/// sub-agent models so they can see and invoke these tools. Merged into
/// `tool_specs_for_allowed_tools` alongside the built-in mvp specs.
static GLOBAL_EXTRA_TOOL_DEFS: OnceLock<Arc<Vec<ToolDefinition>>> = OnceLock::new();

/// Register the runtime (MCP + plugin) tool executor and its tool definitions
/// for sub-agent execution. Must be called after `register_tool_executor`.
/// Repeated registration is a no-op (returns an error, mirroring
/// `register_tool_executor`), so test binaries that call it more than once do
/// not fail.
pub fn register_runtime_tool_provider(
    executor: Box<RuntimeToolExecutorFn>,
    tool_defs: Vec<ToolDefinition>,
) -> Result<(), String> {
    match GLOBAL_RUNTIME_EXECUTOR.set(executor) {
        Ok(()) => {
            let _ = GLOBAL_EXTRA_TOOL_DEFS.set(Arc::new(tool_defs));
            Ok(())
        }
        Err(_) => Err(String::from("runtime tool executor already registered")),
    }
}

/// Accessor for the registered extra tool definitions. Returns `None` when the
/// CLI has not registered a runtime tool provider (no MCP/plugin config).
pub fn registered_extra_tool_defs() -> Option<Arc<Vec<ToolDefinition>>> {
    GLOBAL_EXTRA_TOOL_DEFS.get().cloned()
}

struct ProviderEntry {
    model: String,
    client: ProviderClient,
    provider_kind: ProviderKind,
}

/// Tracks `Arc` pointer identity across consecutive `ApiClient::stream()` calls
/// to detect when messages are merely appended (not rebuilt) so we can skip
/// re-converting the full message list. Also tracks tool definition changes
/// so that requests 2+ can set `skip_tools = true` when tools are unchanged.
struct MessageCache {
    /// `Arc::as_ptr` value of the last seen `ApiRequest.messages`.
    last_ptr: usize,
    /// Number of messages from the start that we've already converted.
    last_len: usize,
    /// Accumulated converted `InputMessage`s.
    input_messages: Arc<Vec<InputMessage>>,
    /// Accumulated cached JSON `Value`s for `IncrementalBody`.
    cached_values: Arc<Vec<Option<Value>>>,
    /// Hash of the tool definitions from the prior request.
    /// Used to detect tool changes and enable `skip_tools`.
    tools_hash: u64,
}

pub struct ProviderRuntimeClient {
    chain: Vec<ProviderEntry>,
    allowed_tools: BTreeSet<String>,
    message_cache: Option<MessageCache>,
    progress: Option<(String, SharedProgress)>,
    reasoning_effort: Option<String>,
}

impl ProviderRuntimeClient {
    pub fn new(model: String, allowed_tools: BTreeSet<String>) -> Result<Self, String> {
        let fallback_config = load_provider_fallback_config();
        Self::new_with_fallback_config(model, allowed_tools, &fallback_config)
    }

    pub fn new_with_fallback_config(
        model: String,
        allowed_tools: BTreeSet<String>,
        fallback_config: &ProviderFallbackConfig,
    ) -> Result<Self, String> {
        let primary_model = fallback_config.primary().map_or(model, str::to_string);
        let primary = build_provider_entry(&primary_model)?;
        let mut chain = vec![primary];
        for fallback_model in fallback_config.fallbacks() {
            match build_provider_entry(fallback_model) {
                Ok(entry) => chain.push(entry),
                Err(_error) => {
                    // Silently skip unavailable fallback providers.
                    // eprintln! would corrupt the subagent progress overlay.
                }
            }
        }
        Ok(Self {
            chain,
            allowed_tools,
            message_cache: None,
            progress: None,
            reasoning_effort: None,
        })
    }

    #[must_use]
    pub fn with_progress(mut self, agent_id: String, progress: SharedProgress) -> Self {
        self.progress = Some((agent_id, progress));
        self
    }

    #[must_use]
    pub fn with_reasoning_effort(mut self, reasoning_effort: Option<String>) -> Self {
        self.reasoning_effort = reasoning_effort;
        self
    }
}

fn build_provider_entry(model: &str) -> Result<ProviderEntry, String> {
    let resolved = resolve_model_alias(model).clone();
    let client = ProviderClient::from_model(&resolved)
        .map_err(|error| error.to_string())?
        .with_incremental_body();
    let provider_kind = detect_provider_kind(&resolved);
    Ok(ProviderEntry {
        model: resolved,
        client,
        provider_kind,
    })
}

fn load_provider_fallback_config() -> ProviderFallbackConfig {
    std::env::current_dir()
        .ok()
        .and_then(|cwd| ConfigLoader::default_for(cwd).load().ok())
        .map_or_else(ProviderFallbackConfig::default, |config| {
            config.provider_fallbacks().clone()
        })
}

impl ApiClient for ProviderRuntimeClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        let mut tools = tool_specs_for_allowed_tools(Some(&self.allowed_tools))
            .into_iter()
            .map(|spec| ToolDefinition {
                name: spec.name.to_string(),
                description: Some(spec.description.to_string()),
                input_schema: spec.input_schema.clone(),
            })
            .collect::<Vec<_>>();
        // Advertise runtime tools (MCP discovery/wrapper + plugin tools) so the
        // sub-agent model can see and invoke them. Only tools that pass the
        // sub-agent's allowed_tools filter are included.
        if let Some(extra) = registered_extra_tool_defs() {
            tools.extend(
                extra
                    .iter()
                    .filter(|def| self.allowed_tools.contains(def.name.as_str()))
                    .cloned(),
            );
        }

        let tools_hash = compute_tools_hash(&tools);

            let primary_model = self.chain.first().map(|entry| entry.model.as_str());
            let (messages, cached_values) = {
            let msg_ptr = Arc::as_ptr(&request.messages) as usize;
            let msg_len = request.messages.len();

            if let Some(cache) = &self.message_cache {
                if cache.last_ptr == msg_ptr && cache.last_len <= msg_len {
                    let cache = self.message_cache.as_mut().unwrap();
                    if msg_len > cache.last_len {
                        let (delta_inputs, delta_cached) = convert_messages_inner(
                            &request.messages[cache.last_len..],
                            None,
                            None,
                            primary_model,
                        );
                        Arc::make_mut(&mut cache.input_messages).extend(delta_inputs);
                        Arc::make_mut(&mut cache.cached_values).extend(delta_cached);
                        cache.last_len = msg_len;
                    }
                    let messages = Arc::clone(&cache.input_messages);
                    let cached_values = Arc::clone(&cache.cached_values);
                    (messages, cached_values)
                } else {
                    full_convert_and_cache(
                        &mut self.message_cache,
                        &request,
                        msg_ptr,
                        msg_len,
                        primary_model,
                    )
                }
            } else {
                full_convert_and_cache(
                    &mut self.message_cache,
                    &request,
                    msg_ptr,
                    msg_len,
                    primary_model,
                )
            }
        };

        let progress_reporter = self.progress.clone();

        let system =
            (!request.system_prompt.is_empty()).then(|| Arc::clone(&request.system_prompt));
        let tool_choice = (!self.allowed_tools.is_empty()).then_some(ToolChoice::Auto);

        let chain = &self.chain;
        let is_local = is_local_inference();
        let mut last_error: Option<ApiError> = None;
        for (index, entry) in chain.iter().enumerate() {
            let (skip_tools, tools_in_system_prompt, per_entry_tools, per_entry_system) =
                if entry.provider_kind == ProviderKind::Anthropic {
                    let sk = self
                        .message_cache
                        .as_ref()
                        .is_some_and(|cache| cache.tools_hash == tools_hash);
                    let tools_val = (!tools.is_empty()).then(|| tools.clone());
                    (sk, false, tools_val, system.clone())
                } else if is_local {
                    let tools_block = render_tools_block(&tools);
                    let system_with_tools = match &system {
                        Some(s) if !s.is_empty() => format!("{s}\n\n{tools_block}"),
                        _ => tools_block,
                    };
                    (false, true, None, Some(Arc::from(system_with_tools.as_str())))
                } else {
                    let tools_val = (!tools.is_empty()).then(|| tools.clone());
                    (false, false, tools_val, system.clone())
                };
            let message_request = MessageRequest {
                model: entry.model.clone(),
                max_tokens: max_tokens_for_model(&entry.model),
                messages: messages.clone(),
                system: per_entry_system,
                tools: per_entry_tools,
                tool_choice: tool_choice.clone(),
                stream: true,
                cached_message_values: Arc::clone(&cached_values),
                skip_tools,
                tools_in_system_prompt,
                reasoning_effort: self.reasoning_effort.clone(),
                ..Default::default()
            };

            let rt = GLOBAL_RUNTIME.get_or_init(|| {
                tokio::runtime::Runtime::new()
                    .expect("failed to create global tokio runtime")
            });
            let attempt = rt.block_on(stream_with_provider(
                &entry.client,
                &message_request,
                &progress_reporter,
            ));
            match attempt {
                Ok(events) => {
                    if let Some(cache) = &mut self.message_cache {
                        cache.tools_hash = tools_hash;
                    }
                    return Ok(events);
                }
                Err(error) if error.is_retryable() && index + 1 < chain.len() => {
                    last_error = Some(error);
                    // Push a progress event so the user sees the fallback instead
                    // of corrupting the overlay with an eprintln!.
                    if let Some((ref agent_id, ref shared)) = self.progress {
                        crate::push_progress_event(
                            shared,
                            agent_id,
                            crate::SubagentProgressEvent::Thinking {
                                text: format!(
                                    "retrying with fallback provider {}",
                                    chain[index + 1].model
                                ),
                            },
                        );
                    }
                }
                Err(error) => return Err(RuntimeError::new(error.to_string())),
            }
        }

        Err(RuntimeError::new(last_error.map_or_else(
            || String::from("provider chain exhausted with no attempts"),
            |error| error.to_string(),
        )))
    }
}

/// Deterministic hash of full tool definitions including input_schema.
/// Used for skip_tools detection (Anthropic) and TCSP change detection
/// (local inference). Every field matters — any change alters the hash
/// and triggers re-sending of tools.
fn compute_tools_hash(tools: &[ToolDefinition]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for tool in tools {
        tool.name.hash(&mut hasher);
        tool.description.hash(&mut hasher);
        tool.input_schema.to_string().hash(&mut hasher);
    }
    hasher.finish()
}

/// Full conversion pass that populates the message cache.
fn full_convert_and_cache(
    cache: &mut Option<MessageCache>,
    request: &ApiRequest,
    msg_ptr: usize,
    msg_len: usize,
    model_name: Option<&str>,
) -> (Arc<Vec<InputMessage>>, Arc<Vec<Option<Value>>>) {
    let image_cache = request
        .image_cache
        .as_ref()
        .map(|arc| arc.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
    let image_store = request.image_store.as_ref();
    let (msgs_arc, vals) = convert_messages_cached(
        &request.messages,
        image_cache.as_deref(),
        image_store,
        model_name,
    );

    let vals_arc = Arc::new(vals);

    *cache = Some(MessageCache {
        last_ptr: msg_ptr,
        last_len: msg_len,
        input_messages: Arc::clone(&msgs_arc),
        cached_values: Arc::clone(&vals_arc),
        tools_hash: 0,
    });

    (msgs_arc, vals_arc)
}

async fn stream_with_provider(
    client: &ProviderClient,
    message_request: &MessageRequest,
    progress_reporter: &Option<(String, SharedProgress)>,
) -> Result<Vec<AssistantEvent>, ApiError> {
    let mut stream = client.stream_message(message_request).await?;
    let mut events = Vec::new();
    let mut pending_tools: BTreeMap<u32, (String, String, String)> = BTreeMap::new();
    let mut saw_stop = false;
    let mut accumulated_thinking = String::new();
    let mut pending_thinking_signature: Option<String> = None;
    let mut block_is_thinking = false;
    // Accumulated visible text deltas. Buffered so a `<DSML>` tool call that
    // spans multiple text chunks can be extracted as a whole at block stop.
    let mut accumulated_visible = String::new();
    // ThinkParser strips inline `<think>…</think>` tags from text deltas
    // so reasoning models that emit thinking inline (DeepSeek-R1, GLM-Z1,
    // some Qwen variants) don't leak the thinking into the visible
    // content stream.
    let mut think_parser = ThinkParser::new();
    // Number of content blocks that have STARTED but not yet STOPPED. A stream
    // that reaches EOF with this > 0 was truncated mid-block, so a synthetic
    // MessageStop would falsely mark a partial response as complete.
    let mut open_blocks = 0usize;

    while let Some(event) = stream.next_event().await? {
        match event {
            ApiStreamEvent::MessageStart(start) => {
                events.push(AssistantEvent::Usage(start.message.usage.token_usage()));
                for (index, block) in start.message.content.into_iter().enumerate() {
                    push_output_block(block, index as u32, &mut events, &mut pending_tools, true);
                }
            }
            ApiStreamEvent::ContentBlockStart(start) => {
                open_blocks += 1;
                if matches!(start.content_block, OutputContentBlock::Thinking { .. }) {
                    pending_thinking_signature = None;
                }
                let (trailing_visible, trailing_reasoning) = think_parser.finish();
                if !trailing_visible.is_empty() {
                    accumulated_visible.push_str(&trailing_visible);
                }
                if !trailing_reasoning.is_empty() {
                    accumulated_thinking.push_str(&trailing_reasoning);
                }
                block_is_thinking = matches!(
                    start.content_block,
                    OutputContentBlock::Thinking { .. }
                );
                if !block_is_thinking {
                    flush_thinking_block(
                        &mut events,
                        &mut accumulated_thinking,
                        &mut pending_thinking_signature,
                        progress_reporter,
                    );
                }
                // A new non-text block means the previous text block ended;
                // flush any accumulated visible text and its embedded tools.
                if !matches!(start.content_block, OutputContentBlock::Text { .. }) {
                    flush_visible_text(&mut events, &mut accumulated_visible);
                }
                push_output_block(
                    start.content_block,
                    start.index,
                    &mut events,
                    &mut pending_tools,
                    true,
                );
            }
            ApiStreamEvent::ContentBlockDelta(delta) => match delta.delta {
                ContentBlockDelta::TextDelta { text } => {
                    if !text.is_empty() {
                        // Route text through the ThinkParser to extract any
                        // inline `<think>…</think>` content. Reasoning
                        // extracted from the visible stream is folded into
                        // `accumulated_thinking` and flushed with the
                        // provider-native thinking deltas.
                        let (visible, reasoning) = think_parser.push(&text);
                        if !visible.is_empty() {
                            // Accumulate visible text so a `<DSML>` tool call
                            // that straddles chunk boundaries can still be
                            // extracted when the content block stops. Flush it
                            // as clean text there, after embedded tools have
                            // been pulled out.
                            accumulated_visible.push_str(&visible);
                            // Mirror the thinking-delta preview so the overlay
                            // keeps refreshing while plain text streams.
                            report_visible_text_progress(
                                progress_reporter.as_ref(),
                                &accumulated_visible,
                            );
                        }
                        if !reasoning.is_empty() {
                            accumulated_thinking.push_str(&reasoning);
                            block_is_thinking = true;
                        }
                    }
                }
                ContentBlockDelta::InputJsonDelta { partial_json } => {
                    if let Some((_, _, input)) = pending_tools.get_mut(&delta.index) {
                        input.push_str(&partial_json);
                    }
                }
                ContentBlockDelta::ThinkingDelta { thinking } => {
                    if !thinking.is_empty() {
                        accumulated_thinking.push_str(&thinking);
                        if let Some((ref agent_id, ref shared)) = progress_reporter {
                            let preview: String = accumulated_thinking.chars().take(60).collect();
                            set_current_activity(
                                shared,
                                agent_id,
                                Some(format!("thinking... {}", preview)),
                            );
                        }
                    }
                }
                ContentBlockDelta::SignatureDelta { signature } => {
                    pending_thinking_signature = Some(signature);
                }
            },
            ApiStreamEvent::ContentBlockStop(stop) => {
                open_blocks = open_blocks.saturating_sub(1);
                let (trailing_visible, trailing_reasoning) = think_parser.finish();
                if !trailing_visible.is_empty() {
                    accumulated_visible.push_str(&trailing_visible);
                }
                if !trailing_reasoning.is_empty() {
                    accumulated_thinking.push_str(&trailing_reasoning);
                }
                if block_is_thinking || !accumulated_thinking.is_empty() {
                    flush_thinking_block(
                        &mut events,
                        &mut accumulated_thinking,
                        &mut pending_thinking_signature,
                        progress_reporter,
                    );
                    block_is_thinking = false;
                    if let Some((ref agent_id, ref shared)) = progress_reporter {
                        set_current_activity(shared, agent_id, None);
                    }
                } else {
                    // A plain text block ended — flush its accumulated text
                    // (extracting any embedded `<DSML>` tool calls).
                    flush_visible_text(&mut events, &mut accumulated_visible);
                }
                if let Some((id, name, input)) = pending_tools.remove(&stop.index) {
                    let input = serde_json::from_str(&input)
                        .unwrap_or_else(|_| serde_json::json!({ "raw": input }));
                    events.push(AssistantEvent::ToolUse { id, name, input });
                }
            }
            ApiStreamEvent::MessageDelta(delta) => {
                events.push(AssistantEvent::Usage(delta.usage.token_usage()));
            }
            ApiStreamEvent::MessageStop(_) => {
                saw_stop = true;
                let (trailing_visible, trailing_reasoning) = think_parser.finish();
                if !trailing_visible.is_empty() {
                    accumulated_visible.push_str(&trailing_visible);
                }
                if !trailing_reasoning.is_empty() {
                    accumulated_thinking.push_str(&trailing_reasoning);
                }
                if block_is_thinking || !accumulated_thinking.is_empty() {
                    flush_thinking_block(
                        &mut events,
                        &mut accumulated_thinking,
                        &mut pending_thinking_signature,
                        progress_reporter,
                    );
                    block_is_thinking = false;
                }
                flush_visible_text(&mut events, &mut accumulated_visible);
                // A tool block may still be streaming when message_stop arrives
                // (max_tokens cut mid-call, or the stop frame racing the final
                // content_block_stop). Flush it rather than silently dropping
                // the call; drain_pending_tools keeps the raw fallback.
                drain_pending_tools(&mut events, &mut pending_tools);
                events.push(AssistantEvent::MessageStop);
            }
        }
    }

    push_prompt_cache_record(client, &mut events);

    if events
        .iter()
        .any(|event| matches!(event, AssistantEvent::MessageStop))
    {
        return Ok(events);
    }

    // EOF without a real message_stop. Only a provably complete stream may be
    // synthesized into a successful completion; a truncated stream (open
    // blocks or in-flight tools) falls through to the non-streaming retry.
    let has_content = events.iter().any(|event| {
        matches!(event, AssistantEvent::TextDelta(text) if !text.is_empty())
            || matches!(event, AssistantEvent::ToolUse { .. })
    });
    if should_synthesize_stop(saw_stop, has_content, open_blocks, pending_tools.is_empty()) {
        events.push(AssistantEvent::MessageStop);
        return Ok(events);
    }

    // Truncated or empty stream: recover via a complete non-streaming request.
    // If that also fails, the error propagates instead of completing silently
    // with partial output.
    let response = client
        .send_message(&MessageRequest {
            stream: false,
            ..message_request.clone()
        })
        .await?;
    let mut events = response_to_events(response);
    push_prompt_cache_record(client, &mut events);
    Ok(events)
}

/// Decide whether an EOF without a real `message_stop` frame should be
/// presented as a complete assistant message. Only a provably complete stream
/// (every started block stopped, no tool block still streaming, and at least
/// one piece of content) may be synthesized; anything else is truncation and
/// must go through the non-streaming retry.
fn should_synthesize_stop(
    saw_stop: bool,
    has_content: bool,
    open_blocks: usize,
    pending_tools_empty: bool,
) -> bool {
    !saw_stop && has_content && open_blocks == 0 && pending_tools_empty
}

/// Flush in-flight tool blocks into `ToolUse` events. Mirrors the
/// `ContentBlockStop` fallback: when the accumulated JSON never parsed
/// completely, the raw text is wrapped in `{"raw": ...}` so the tool still
/// reaches the agent loop instead of being silently dropped.
fn drain_pending_tools(
    events: &mut Vec<AssistantEvent>,
    pending_tools: &mut BTreeMap<u32, (String, String, String)>,
) {
    let drained = std::mem::take(pending_tools);
    for (_index, (_id, name, input)) in drained {
        let input =
            serde_json::from_str(&input).unwrap_or_else(|_| serde_json::json!({ "raw": input }));
        events.push(AssistantEvent::ToolUse {
            id: _id,
            name,
            input,
        });
    }
}

fn push_output_block(
    block: OutputContentBlock,
    block_index: u32,
    events: &mut Vec<AssistantEvent>,
    pending_tools: &mut BTreeMap<u32, (String, String, String)>,
    streaming_tool_input: bool,
) {
    match block {
        OutputContentBlock::Text { text } => {
            if !text.is_empty() {
                // DeepSeek-family endpoints may emit tool calls as text XML
                // (`<DSML>` markers) rather than structured ToolUse blocks.
                // Extract them so the agent loop can execute the tool instead
                // of treating the raw XML as the final answer.
                let (clean, tool_calls) = extract_embedded_tools(&text);
                for (id, name, input) in tool_calls {
                    events.push(AssistantEvent::ToolUse { id, name, input });
                }
                if !clean.is_empty() {
                    events.push(AssistantEvent::TextDelta(clean));
                }
            }
        }
        OutputContentBlock::ToolUse { id, name, input } => {
            let initial_input = if streaming_tool_input
                && input.is_object()
                && input.as_object().is_some_and(serde_json::Map::is_empty)
            {
                String::new()
            } else {
                input.to_string()
            };
            pending_tools.insert(block_index, (id, name, initial_input));
        }
        OutputContentBlock::Thinking { thinking, signature } => {
            if streaming_tool_input && thinking.is_empty() {
                // Streaming: text arrives via ThinkingDelta — do nothing yet;
                // the deltas accumulate and are flushed at block stop.
            } else if !thinking.is_empty() {
                let (clean, tool_calls) = extract_embedded_tools(&thinking);
                for (id, name, input) in tool_calls {
                    events.push(AssistantEvent::ToolUse { id, name, input });
                }
                if !clean.trim().is_empty() {
                    events.push(AssistantEvent::Thinking {
                        text: clean,
                        signature,
                    });
                }
            } else if let Some(sig) = signature {
                // Non-streaming `display: "omitted"` block: empty text but the
                // signature is mandatory for the tool-use round-trip — keep it.
                events.push(AssistantEvent::Thinking {
                    text: String::new(),
                    signature: Some(sig),
                });
            }
        }
        OutputContentBlock::RedactedThinking { data } => {
            // Redacted thinking has no signature; the ciphertext `data` is the
            // authentication token and must survive into the conversation so
            // the tool-use round-trip can echo it back to the API verbatim.
            let data = data
                .as_str()
                .map(ToOwned::to_owned)
                .unwrap_or_default();
            events.push(AssistantEvent::RedactedThinking { data });
        }
        OutputContentBlock::Image { .. } => {}
    }
}

fn response_to_events(response: MessageResponse) -> Vec<AssistantEvent> {
    let mut events = Vec::new();
    let mut pending_tools = BTreeMap::new();

    for (index, block) in response.content.into_iter().enumerate() {
        let index = u32::try_from(index).expect("response block index overflow");
        push_output_block(block, index, &mut events, &mut pending_tools, false);
        if let Some((id, name, input)) = pending_tools.remove(&index) {
            let input = serde_json::from_str(&input)
                .unwrap_or_else(|_| serde_json::json!({ "raw": input }));
            events.push(AssistantEvent::ToolUse { id, name, input });
        }
    }

    events.push(AssistantEvent::Usage(response.usage.token_usage()));
    events.push(AssistantEvent::MessageStop);
    events
}

fn push_prompt_cache_record(client: &ProviderClient, events: &mut Vec<AssistantEvent>) {
    if let Some(record) = client.take_last_prompt_cache_record() {
        if let Some(event) = prompt_cache_record_to_runtime_event(record) {
            events.push(AssistantEvent::PromptCache(event));
        }
    }
}

fn prompt_cache_record_to_runtime_event(
    record: api::PromptCacheRecord,
) -> Option<runtime::PromptCacheEvent> {
    let cache_break = record.cache_break?;
    Some(runtime::PromptCacheEvent {
        unexpected: cache_break.unexpected,
        reason: cache_break.reason,
        previous_cache_read_input_tokens: cache_break.previous_cache_read_input_tokens,
        current_cache_read_input_tokens: cache_break.current_cache_read_input_tokens,
        token_drop: cache_break.token_drop,
    })
}

/// Flush accumulated visible text, extracting any embedded `<DSML>` tool calls
/// so the agent loop executes them instead of treating the raw XML as output.
fn flush_visible_text(events: &mut Vec<AssistantEvent>, accumulated_visible: &mut String) {
    if accumulated_visible.is_empty() {
        return;
    }
    let text = std::mem::take(accumulated_visible);
    let (clean, tool_calls) = extract_embedded_tools(&text);
    for (id, name, input) in tool_calls {
        events.push(AssistantEvent::ToolUse { id, name, input });
    }
    if !clean.is_empty() {
        events.push(AssistantEvent::TextDelta(clean));
    }
}

fn flush_thinking_block(
    events: &mut Vec<AssistantEvent>,
    accumulated_thinking: &mut String,
    pending_thinking_signature: &mut Option<String>,
    progress_reporter: &Option<(String, SharedProgress)>,
) {
    if accumulated_thinking.is_empty() && pending_thinking_signature.is_none() {
        return;
    }
    let text = std::mem::take(accumulated_thinking);
    let (clean, tool_calls) = extract_embedded_tools(&text);

    for (id, name, input) in tool_calls {
        events.push(AssistantEvent::ToolUse { id, name, input });
    }

    if let Some((agent_id, shared)) = progress_reporter {
        let report = clean.trim();
        if !report.is_empty() {
            let preview: String = report.chars().take(120).collect();
            push_progress_event(shared, agent_id, SubagentProgressEvent::Thinking { text: preview });
        }
    }

    let signature = pending_thinking_signature.take();
    if !clean.trim().is_empty() || signature.is_some() {
        events.push(AssistantEvent::Thinking {
            text: clean,
            signature,
        });
    }
}



/// Surface the sub-agent's visible text live in the progress overlay. Without
/// this the overlay stops refreshing once thinking ends — `set_current_activity`
/// is the only thing bumping `event_seq` during model streaming, so the elapsed
/// timer freezes and the agent looks stuck while it is still generating text.
fn report_visible_text_progress(
    progress_reporter: Option<&(String, SharedProgress)>,
    accumulated_visible: &str,
) {
    if let Some((agent_id, shared)) = progress_reporter {
        let preview: String = accumulated_visible.chars().take(60).collect();
        let clean = preview.replace(['\r', '\n'], " ");
        set_current_activity(shared, agent_id, Some(format!("writing... {clean}")));
    }
}

pub struct SubagentToolExecutor {
    allowed_tools: BTreeSet<String>,
    policy: Option<PermissionPolicy>,
    progress: Option<(String, SharedProgress)>,
}/// Route a tool call through the builtin executor, falling back to the runtime
/// (MCP/plugin) executor on the `unsupported tool` marker. Kept as a free
/// function so the fallback policy is unit-testable without touching the
/// process-global `OnceLock`s.
fn route_tool_call(
    builtin: &RuntimeToolExecutorFn,
    runtime: Option<&RuntimeToolExecutorFn>,
    tool_name: &str,
    value: &Value,
    policy: Option<&PermissionPolicy>,
) -> Result<String, String> {
    let mut result = builtin(tool_name, value, policy);
    if matches!(&result, Err(error) if error.starts_with("unsupported tool: ")) {
        if let Some(runtime_exec) = runtime {
            result = runtime_exec(tool_name, value, policy);
        }
    }
    result
}

impl SubagentToolExecutor {
    pub fn new(allowed_tools: BTreeSet<String>) -> Self {
        Self {
            allowed_tools,
            policy: None,
            progress: None,
        }
    }

    pub fn with_permission_policy(mut self, policy: PermissionPolicy) -> Self {
        self.policy = Some(policy);
        self
    }

    pub fn with_progress(mut self, agent_id: String, progress: SharedProgress) -> Self {
        self.progress = Some((agent_id, progress));
        self
    }
}

impl ToolExecutor for SubagentToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        if !self.allowed_tools.contains(tool_name) {
            return Err(ToolError::new(format!(
                "tool `{tool_name}` is not enabled for this sub-agent"
            )));
        }
        // Belt-and-suspenders: the conversation loop already authorized this
        // call, but re-check the policy here so a `permission:` deny directive
        // cannot be bypassed by any path that reaches the executor directly.
        // A sub-agent has no interactive prompter, so `ask` rules deny here
        // too (matching the conversation layer's behavior with `None`).
        if let Some(policy) = &self.policy {
            if matches!(
                policy.authorize(tool_name, input, None),
                PermissionOutcome::Deny { .. }
            ) {
                return Err(ToolError::new(format!(
                    "tool `{tool_name}` denied by the sub-agent permission policy"
                )));
            }
        }
        let value: Value = serde_json::from_str(input)
            .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;

        // Report tool call progress
        if let Some((agent_id, shared)) = &self.progress {
            set_current_activity(
                shared,
                agent_id,
                Some(format!("executing {}", tool_name)),
            );
            push_progress_event(
                shared,
                agent_id,
                SubagentProgressEvent::ToolCall {
                    tool_name: tool_name.to_string(),
                    input: value.clone(),
                },
            );
            push_progress_event(
                shared,
                agent_id,
                SubagentProgressEvent::StatusChange {
                    status: AgentStatus::UsingTool,
                },
            );
        }

        let exec = GLOBAL_TOOL_EXECUTOR.get().ok_or_else(|| {
            ToolError::new(
                "subagent tool executor not registered; \
                 call agents::runtime::register_tool_executor from the tools crate"
                    .to_string(),
            )
        })?;
        let result = route_tool_call(
            exec,
            GLOBAL_RUNTIME_EXECUTOR.get().map(|v| &**v),
            tool_name,
            &value,
            self.policy.as_ref(),
        );

        // Report tool result progress
        if let Some((agent_id, shared)) = &self.progress {
            set_current_activity(shared, agent_id, None);
            match &result {
                Ok(out) => {
                    let truncated: String = out.chars().take(200).collect();
                    push_progress_event(
                        shared,
                        agent_id,
                        SubagentProgressEvent::ToolResult {
                            tool_name: tool_name.to_string(),
                            truncated_result: truncated,
                        },
                    );
                }
                Err(err) => {
                    let truncated: String = err.chars().take(200).collect();
                    push_progress_event(
                        shared,
                        agent_id,
                        SubagentProgressEvent::ToolResult {
                            tool_name: tool_name.to_string(),
                            truncated_result: format!("ERROR: {truncated}"),
                        },
                    );
                }
            }
            push_progress_event(
                shared,
                agent_id,
                SubagentProgressEvent::StatusChange {
                    status: AgentStatus::Thinking,
                },
            );
        }

        result.map_err(ToolError::new)
    }
}

fn tool_specs_for_allowed_tools(
    allowed_tools: Option<&BTreeSet<String>>,
) -> Vec<runtime::tool_registry::ToolSpec> {
    runtime::tool_registry::mvp_tool_specs()
        .into_iter()
        .filter(|spec| allowed_tools.is_none_or(|allowed| allowed.contains(spec.name)))
        .collect()
}

// Deleted 2026-06-04 per spec §5.4 (cycle-break Option 2).
// The 18-spec subset was the *permission-relevant* view; until the
// PermissionMode filter criterion is decided (spec §11), callers see all 53.

/// Build the sub-agent's permission policy.
///
/// Permission passthrough: the base mode is the parent session's active
/// mode (threaded through `AgentJob::permission_mode`), so a sub-agent is
/// constrained by exactly the same permission regime as its parent. The
/// frontmatter `permission:` directives are no longer converted into hard
/// allow/deny/ask rules — a `bash: deny` in an agent definition can no
/// longer block a sub-agent from running a read-only command that the
/// parent mode permits. Tool-requirement escalation (e.g. `bash` under
/// `WorkspaceWrite`) still prompts under the inherited mode.
fn agent_permission_policy(base_mode: PermissionMode) -> PermissionPolicy {
    runtime::tool_registry::mvp_tool_specs().into_iter().fold(
        PermissionPolicy::new(base_mode),
        |policy, spec| policy.with_tool_requirement(spec.name, spec.required_permission),
    )
}

pub fn build_agent_system_prompt(subagent_type: &str) -> Result<Vec<String>, String> {
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    use crate::persist::DEFAULT_AGENT_SYSTEM_DATE;
    let mut prompt = load_system_prompt(
        cwd,
        DEFAULT_AGENT_SYSTEM_DATE.to_string(),
        std::env::consts::OS,
        "unknown",
    )
    .map_err(|error| error.to_string())?;
    prompt.push(format!(
        "You are a background sub-agent of type `{subagent_type}`. \
         Work only on the delegated task, use only the tools available to you, \
         do not ask the user questions, and finish with a concise result."
    ));
    prompt.push(
        "You may have access to MCP tools, plugin tools, and skills when they are \
         available to you. Use them to complete the delegated task when appropriate."
            .to_string(),
    );
    prompt.push(
        "Complete the task yourself end-to-end: you are the final executor and \
         already have every tool you need. Use your own tools to finish the \
         work directly."
            .to_string(),
    );
    Ok(prompt)
}

pub fn resolve_agent_model(model: Option<&str>) -> String {
    use crate::persist::DEFAULT_AGENT_MODEL;
    model
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .unwrap_or(DEFAULT_AGENT_MODEL)
        .to_string()
}

pub fn build_agent_runtime(
    job: &AgentJob,
) -> Result<ConversationRuntime<ProviderRuntimeClient, SubagentToolExecutor>, String> {
    build_agent_runtime_inner(job, None, None)
}

pub fn build_agent_runtime_inner(
    job: &AgentJob,
    progress: Option<SharedProgress>,
    agent_id: Option<String>,
) -> Result<ConversationRuntime<ProviderRuntimeClient, SubagentToolExecutor>, String> {
    use crate::persist::DEFAULT_AGENT_MODEL;
    let model = job
        .manifest
        .model
        .clone()
        .unwrap_or_else(|| DEFAULT_AGENT_MODEL.to_string());
    let allowed_tools = job.allowed_tools.clone();
    let mut api_client = ProviderRuntimeClient::new(model, allowed_tools.clone())?
        .with_reasoning_effort(job.reasoning_effort.clone());
    let permission_policy = agent_permission_policy(job.permission_mode);
    let mut tool_executor = SubagentToolExecutor::new(allowed_tools)
        .with_permission_policy(permission_policy.clone());

    if let (Some(progress), Some(aid)) = (&progress, &agent_id) {
        api_client = api_client.with_progress(aid.clone(), Arc::clone(progress));
        tool_executor = tool_executor.with_progress(aid.clone(), Arc::clone(progress));
    }

    Ok(ConversationRuntime::new(
        Session::new(),
        api_client,
        tool_executor,
        permission_policy,
        job.system_prompt.clone(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn builtin_unsupported() -> &'static RuntimeToolExecutorFn {
        &|_name, _value, _policy| Err("unsupported tool: some_tool".to_string())
    }

    fn builtin_ok() -> &'static RuntimeToolExecutorFn {
        &|_name, _value, _policy| Ok("builtin ok".to_string())
    }

    fn runtime_echo() -> &'static RuntimeToolExecutorFn {
        &|name, _value, _policy| Ok(format!("runtime handled {name}"))
    }

    #[test]
    fn route_falls_back_to_runtime_executor_on_unsupported() {
        let result = route_tool_call(
            builtin_unsupported(),
            Some(runtime_echo()),
            "mcp__demo__echo",
            &json!({}),
            None,
        );
        assert_eq!(result.unwrap(), "runtime handled mcp__demo__echo");
    }

    #[test]
    fn route_keeps_builtin_success() {
        let result = route_tool_call(
            builtin_ok(),
            Some(runtime_echo()),
            "read_file",
            &json!({}),
            None,
        );
        assert_eq!(result.unwrap(), "builtin ok");
    }

    #[test]
    fn push_output_block_extracts_dsml_tool_from_text() {
        // DeepSeek-family endpoints emit tool calls as `<DSML>`-prefixed text
        // XML; the Text arm must extract them into ToolUse events and keep the
        // clean narration as TextDelta instead of leaking raw XML.
        let full = "\u{ff5c}\u{ff5c}";
        let text = format!(
            "Let me look first.\n<{full}DSML{full}tool_calls>\n\
             <{full}DSML{full}invoke name=\"bash\">\n\
             <{full}DSML{full}parameter name=\"command\" string=\"true\">ls</{full}DSML{full}parameter>\n\
             </{full}DSML{full}invoke>\n\
             </{full}DSML{full}tool_calls>"
        );
        let mut events = Vec::new();
        let mut pending = BTreeMap::new();
        push_output_block(
            OutputContentBlock::Text { text: text.clone() },
            0,
            &mut events,
            &mut pending,
            false,
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AssistantEvent::ToolUse { name, .. } if name == "bash")),
            "DSML tool call in text must be extracted into a ToolUse event: {events:?}"
        );
        let texts: Vec<&String> = events
            .iter()
            .filter_map(|e| match e {
                AssistantEvent::TextDelta(t) if !t.is_empty() => Some(t),
                _ => None,
            })
            .collect();
        assert!(
            !texts.iter().any(|t| t.contains("DSML")),
            "raw DSML XML must not leak into TextDelta events: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("Let me look first")),
            "clean narration must survive extraction: {texts:?}"
        );
    }

    #[test]
    fn route_preserves_unsupported_when_no_runtime_executor() {
        let result = route_tool_call(
            builtin_unsupported(),
            None,
            "mcp__demo__echo",
            &json!({}),
            None,
        );
        assert!(result.unwrap_err().contains("unsupported tool"));
    }

    #[test]
    fn register_runtime_tool_provider_exposes_defs_and_rejects_second_registration() {
        let executor: Box<RuntimeToolExecutorFn> = Box::new(|_n, _v, _p| Ok("x".to_string()));
        let defs = vec![ToolDefinition {
            name: "mcp__demo__echo".to_string(),
            description: Some("demo".to_string()),
            input_schema: json!({}),
        }];
        // The OnceLock is shared across tests in this binary, so registration
        // may already have happened. Either way the defs must be available and
        // a subsequent registration must report the "already registered"
        // marker rather than panicking.
        let _ = register_runtime_tool_provider(executor, defs);
        let registered = registered_extra_tool_defs().expect("defs should be registered");
        assert_eq!(registered[0].name, "mcp__demo__echo");

        let second: Box<RuntimeToolExecutorFn> = Box::new(|_n, _v, _p| Ok("y".to_string()));
        let err = register_runtime_tool_provider(second, vec![])
            .expect_err("second registration should fail");
        assert!(err.contains("already registered"));
    }

    #[test]
    fn subagent_system_prompt_mentions_runtime_tools() {
        let prompt = build_agent_system_prompt("general-purpose")
            .expect("system prompt should build");
        let joined = prompt.join("\n");
        assert!(
            joined.contains("MCP tools") && joined.contains("plugin tools"),
            "system prompt should mention MCP/plugin tools: {joined}"
        );
    }

    #[test]
    fn subagent_system_prompt_guides_direct_completion() {
        let prompt = build_agent_system_prompt("general-purpose")
            .expect("system prompt should build");
        let joined = prompt.join("\n");
        let lower = joined.to_lowercase();
        // Positive guidance instead of a bare prohibition: the sub-agent is
        // told it is expected to complete the task itself with its own tools.
        assert!(
            lower.contains("complete the task yourself"),
            "system prompt should positively instruct self-completion, got: {joined}"
        );
        assert!(
            lower.contains("your own tools"),
            "system prompt should point the sub-agent at its own tools, got: {joined}"
        );
        assert!(
            !lower.contains("never call the agent tool"),
            "guidance should be phrased positively, not as a prohibition, got: {joined}"
        );
    }

    #[test]
    fn should_synthesize_stop_requires_a_provably_complete_stream() {
        use super::should_synthesize_stop;
        // A real message_stop arrived — the arm already pushed it; never synth.
        assert!(!should_synthesize_stop(true, true, 0, true));
        // EOF with no content at all → non-streaming retry.
        assert!(!should_synthesize_stop(false, false, 0, true));
        // EOF mid-block (open block counter > 0) → truncation, no synth.
        assert!(!should_synthesize_stop(false, true, 1, true));
        // EOF with an in-flight tool block → truncation, no synth.
        assert!(!should_synthesize_stop(false, true, 0, false));
        // EOF, clean, with content → synthesize the missing stop.
        assert!(should_synthesize_stop(false, true, 0, true));
    }

    #[test]
    fn drain_pending_tools_emits_tool_use_with_raw_fallback() {
        let mut events = Vec::new();
        let mut pending = BTreeMap::new();
        pending.insert(
            0,
            (
                "toolu_a".to_string(),
                "bash".to_string(),
                "{\"command\"".to_string(),
            ),
        );
        pending.insert(
            1,
            (
                "toolu_b".to_string(),
                "read_file".to_string(),
                "{\"path\":\"x\"}".to_string(),
            ),
        );
        super::drain_pending_tools(&mut events, &mut pending);
        assert!(pending.is_empty(), "pending tools must be drained");
        let mut names = Vec::new();
        for event in &events {
            if let AssistantEvent::ToolUse { name, input, .. } = event {
                names.push(name.as_str());
                if name == "bash" {
                    assert_eq!(
                        input,
                        &json!({ "raw": "{\"command\"" }),
                        "unparseable partial JSON must fall back to raw"
                    );
                }
                if name == "read_file" {
                    assert_eq!(input, &json!({ "path": "x" }));
                }
            }
        }
        assert_eq!(names, vec!["bash", "read_file"]);
    }

    #[test]
    fn visible_text_progress_keeps_overlay_alive() {
        use std::sync::atomic::Ordering as AtomicOrdering;
        use crate::types::{new_shared_progress, AgentProgress};

        let shared = new_shared_progress();
        {
            let mut guard = shared.agents.lock().unwrap_or_else(|e| e.into_inner());
            guard.push(AgentProgress {
                agent_id: "a1".to_string(),
                name: "test".to_string(),
                subagent_type: "general-purpose".to_string(),
                status: AgentStatus::Running,
                events: vec![],
                started_at: std::time::Instant::now(),
                iteration_count: 0,
                final_event: None,
                current_activity: None,
            });
        }

        let seq_before = shared.event_seq.load(AtomicOrdering::Acquire);
        report_visible_text_progress(
            Some(&("a1".to_string(), shared.clone())),
            "Let me inspect the build output",
        );

        let guard = shared.agents.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(
            guard[0].current_activity.as_deref(),
            Some("writing... Let me inspect the build output"),
            "plain text streaming must surface a live preview like thinking does"
        );
        let seq_after = shared.event_seq.load(AtomicOrdering::Acquire);
        assert!(
            seq_after > seq_before,
            "event_seq must advance so the overlay re-renders and the elapsed timer moves"
        );
    }

    #[test]
    fn permission_policy_inherits_parent_mode() {
        // Permission passthrough: the sub-agent policy uses the parent
        // session's active mode as its base. Under DangerFullAccess every
        // tool is allowed; frontmatter permission directives no longer
        // translate into hard deny rules.
        let policy = agent_permission_policy(PermissionMode::DangerFullAccess);
        assert_eq!(
            policy.authorize("read_file", r#"{"path":"/tmp/x"}"#, None),
            PermissionOutcome::Allow
        );
        assert_eq!(
            policy.authorize("new_file", r#"{"path":"/workspace/x.rs"}"#, None),
            PermissionOutcome::Allow
        );
        assert_eq!(
            policy.authorize("bash", r#"{"command":"ls"}"#, None),
            PermissionOutcome::Allow
        );
    }

    #[test]
    fn permission_policy_restricts_under_read_only_parent() {
        // Under a read-only parent mode, workspace writes are denied and
        // reads are allowed — the sub-agent is constrained by the parent.
        let policy = agent_permission_policy(PermissionMode::ReadOnly);
        assert_eq!(
            policy.authorize("read_file", r#"{"path":"/tmp/x"}"#, None),
            PermissionOutcome::Allow
        );
        assert!(matches!(
            policy.authorize("new_file", r#"{"path":"/workspace/x.rs"}"#, None),
            PermissionOutcome::Deny { .. }
        ));
        assert!(matches!(
            policy.authorize("bash", r#"{"command":"ls"}"#, None),
            PermissionOutcome::Deny { .. }
        ));
    }

    #[test]
    fn permission_policy_yolo_auto_approves_work_and_asks_for_sensitive() {
        // Yolo (workspace-write base + external readonly) permits workspace
        // writes and auto-approves ordinary bash commands, but keeps
        // dangerous/sensitive commands at DangerFullAccess; without a
        // prompter that escalation is denied.
        let policy = agent_permission_policy(PermissionMode::Yolo);
        assert_eq!(
            policy.authorize("new_file", r#"{"path":"/workspace/x.rs"}"#, None),
            PermissionOutcome::Allow
        );
        assert_eq!(
            policy.authorize("bash", r#"{"command":"ls"}"#, None),
            PermissionOutcome::Allow
        );
        assert_eq!(
            policy.authorize("bash", r#"{"command":"git status"}"#, None),
            PermissionOutcome::Allow
        );
        assert!(matches!(
            policy.authorize("bash", r#"{"command":"cat /etc/passwd"}"#, None),
            PermissionOutcome::Deny { .. }
        ));
    }

    #[test]
    fn diag_repro_read_agent_permission_files() {
        // TEMPORARY diagnostic: reproduce the user-reported "can't read files"
        // regression against the real agent definitions on disk.
        use std::path::Path;
        let dirs = [
            r"C:\Users\Incredible\.claw\agents",
            r"C:\Users\Incredible\AppData\Roaming\claw\agents",
        ];
        let mut checked = 0usize;
        for dir in dirs {
            let path = Path::new(dir);
            if !path.is_dir() {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(path) else { continue };
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let Ok(contents) = std::fs::read_to_string(&p) else { continue };
                let Some(perm) = plugins::frontmatter::parse_permission_from_content(&contents)
                else {
                    continue;
                };
                checked += 1;
                let policy = agent_permission_policy(PermissionMode::DangerFullAccess);
                let mut rows = Vec::new();
                for (tool, input) in [
                    ("read_file", r#"{"path":"/workspace/x.rs"}"#),
                    ("bash", r#"{"command":"ls"}"#),
                    ("glob_search", r#"{"pattern":"**/*.rs"}"#),
                    ("grep_search", r#"{"pattern":"x"}"#),
                    ("new_file", r#"{"path":"/workspace/x.rs"}"#),
                    ("edit_file", r#"{"path":"/workspace/x.rs"}"#),
                    ("WebFetch", r#"{"url":"https://example.com"}"#),
                    ("Skill", r#"{"skill":"x"}"#),
                ] {
                    let o = policy.authorize(tool, input, None);
                    let label = match o {
                        PermissionOutcome::Allow => "ALLOW",
                        PermissionOutcome::Deny { .. } => "DENY",
                    };
                    rows.push(format!("  {tool}: {label}"));
                }
                eprintln!(
                    "\n### {} (perm keys: {})\n{}",
                    p.file_name().unwrap_or_default().to_string_lossy(),
                    perm.keys().cloned().collect::<Vec<_>>().join(","),
                    rows.join("\n")
                );
            }
        }
        eprintln!("\n[diag] checked {checked} agent files with permission blocks");
    }
}
