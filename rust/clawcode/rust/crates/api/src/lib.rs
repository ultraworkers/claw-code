mod client;
mod convert;
mod error;
mod http_client;
pub mod incremental_body;
mod prompt_cache;
mod providers;
mod sse;
mod types;

pub use convert::{convert_messages, convert_messages_cached, convert_messages_inner};

pub use client::{
    oauth_token_is_expired, read_base_url, resolve_saved_oauth_token,
    resolve_startup_auth_source, MessageStream, OAuthTokenSet, ProviderClient,
};
pub use error::ApiError;
pub use http_client::{
    build_http_client, build_http_client_or_default, build_http_client_with, ProxyConfig,
};
pub use prompt_cache::{
    CacheBreakEvent, PromptCache, PromptCacheConfig, PromptCachePaths, PromptCacheRecord,
    PromptCacheStats,
};
pub use providers::anthropic::{AnthropicClient, AnthropicClient as ApiClient, AuthSource};
pub use providers::openai_compat::{
    build_chat_completion_request, flatten_tool_result_content, is_reasoning_model,
    model_rejects_is_error_field, translate_message, OpenAiCompatClient, OpenAiCompatConfig,
};
pub use providers::{
    detect_provider_kind, is_local_inference, load_env_file_to_process, max_tokens_for_model,
    max_tokens_for_model_with_override, resolve_model_alias, ProviderKind,
};
pub use sse::{parse_frame, SseParser};
pub use types::{
    ContentBlockDelta, ContentBlockDeltaEvent, ContentBlockStartEvent, ContentBlockStopEvent,
    InputContentBlock, InputMessage, MessageDelta, MessageDeltaEvent, MessageRequest,
    MessageResponse, MessageStartEvent, MessageStopEvent, OutputContentBlock, ReasoningEffort,
    StreamEvent, ThinkingConfig, ToolChoice, ToolDefinition, ToolResultContentBlock, Usage,
};
pub use types::render_tools_block;
pub use providers::reasoning::{
    anthropic_thinking_budget, default_reasoning_effort, effective_thinking_config,
    openai_wire_effort, reasoning_levels, supports_level, validate_reasoning_effort,
    UnsupportedReasoningEffort,
};

pub use telemetry::{
    AnalyticsEvent, AnthropicRequestProfile, ClientIdentity, JsonlTelemetrySink,
    MemoryTelemetrySink, SessionTraceRecord, SessionTracer, TelemetryEvent, TelemetrySink,
    DEFAULT_ANTHROPIC_VERSION,
};
