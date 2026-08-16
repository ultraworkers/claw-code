use runtime::{pricing_for_model, TokenUsage, UsageCostEstimate};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// Anthropic extended thinking configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThinkingConfig {
    #[serde(rename = "type")]
    pub config_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
}

/// Reasoning effort level. Escalation order is `Off < Low < Medium < High < Max`.
/// `Off` disables reasoning (omits the wire field); the remaining levels map
/// to a provider-specific wire spelling via [`crate::providers::reasoning`].
/// Serialises as the lowercase level name so the wire value stays stable and
/// matches the prior `Option<String>` representation (`"low"`, `"medium"`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Off,
    Low,
    Medium,
    High,
    Max,
}

impl ReasoningEffort {
    /// The lowercase wire name (`"off"`, `"low"`, `"medium"`, `"high"`, `"max"`).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Max => "max",
        }
    }

    /// Parse a level name (case-insensitive). Returns `None` for unknown names
    /// so callers can produce a precise "must be one of …" diagnostic.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "off" => Some(Self::Off),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "max" => Some(Self::Max),
            _ => None,
        }
    }

    /// Every level in escalation order.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Off,
            Self::Low,
            Self::Medium,
            Self::High,
            Self::Max,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MessageRequest {
    pub model: String,
    pub max_tokens: u32,
    /// Shared message list wrapped in `Arc` so that `MessageRequest::clone()`
    /// is O(1) for the (typically large) messages vector.
    pub messages: Arc<Vec<InputMessage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Arc<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stream: bool,
    /// OpenAI-compatible tuning parameters. Optional — omitted from payload when None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Reasoning effort level for OpenAI-compatible reasoning models (e.g. `o4-mini`).
    /// Accepted values: `"low"`, `"medium"`, `"high"`. Omitted when `None`.
    /// Silently ignored by backends that do not support it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    /// Anthropic extended thinking configuration. Omitted when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    /// Pre-cached serialised JSON `Value`s for each message, typically produced
    /// by `convert_messages_cached`.  The `IncrementalBody` will use these to
    /// skip re-serialisation of unchanged messages.
    /// Empty when not using the cache.
    /// Wrapped in `Arc` so that `MessageRequest::clone()` is O(1).
    #[serde(skip)]
    pub cached_message_values: Arc<Vec<Option<Value>>>,
    /// If `true`, omit the `tools` field when serialising the request body.
    /// Set on requests 2+ when tool definitions haven't changed, saving ~24KB
    /// per turn for Anthropic server-side prompt cache.
    /// NOTE: only respected by the Anthropic provider — OpenAI-compat and xAI
    /// always send full tool definitions.
    #[serde(skip)]
    pub skip_tools: bool,
    /// If `true`, tool definitions have been embedded in the system prompt
    /// text as a deterministic JSON block. The `tools` field should be omitted
    /// from the wire format to avoid duplication.
    /// Used for local inference (llama.cpp, LM Studio, Ollama) where KV cache
    /// prefix stability depends on stable token sequences.
    #[serde(skip)]
    pub tools_in_system_prompt: bool,
}

impl MessageRequest {
    #[must_use]
    pub fn with_streaming(mut self) -> Self {
        self.stream = true;
        self
    }

    /// Render the request body in Anthropic API JSON format.
    ///
    /// Post-processing steps:
    /// 1. Strip tools when `skip_tools` is set (tools unchanged since prior
    ///    request — saves ~24KB per turn via Anthropic server-side cache).
    /// 2. Split system prompt at `SYSTEM_PROMPT_DYNAMIC_BOUNDARY` into blocks
    ///    with `cache_control: ephemeral` on the static portion.
    /// 3. Add `cache_control: ephemeral` to the last tool definition.
    #[inline]
    pub fn render_anthropic_body(&self) -> Result<Value, serde_json::Error> {
        let mut body = serde_json::to_value(self)?;
        // Anthropic's wire uses `thinking` (derived from the reasoning level),
        // not the OpenAI-style `reasoning_effort` field. Strip the pass-through
        // field so it never reaches a backend that would reject or misread it.
        if let Value::Object(ref mut obj) = body {
            obj.remove("reasoning_effort");
        }
        if self.skip_tools {
            if let Value::Object(ref mut obj) = body {
                obj.remove("tools");
            }
        } else {
            Self::apply_tools_cache_control(&mut body);
        }
        Self::apply_system_prompt_cache_control(&mut body);
        Self::apply_messages_cache_control(&mut body);
        Self::apply_cache_reference(&mut body);
        Ok(body)
    }

    /// Post-process the serialised body to add `cache_reference` to tool_result
    /// blocks that fall within the cached prefix (before the last message-level
    /// `cache_control` marker). This lets the server reuse cached tool results.
    pub(crate) fn apply_cache_reference(body: &mut Value) {
        let Some(messages) = body
            .get_mut("messages")
            .and_then(|v| v.as_array_mut())
        else {
            return;
        };
        // Find the last message index that has any cache_control marker
        let mut last_cc_idx = None;
        for (i, msg) in messages.iter().enumerate() {
            if let Some(content) = msg.get("content").and_then(|v| v.as_array()) {
                if content.iter().any(|b| b.get("cache_control").is_some()) {
                    last_cc_idx = Some(i);
                }
            }
        }
        let Some(end) = last_cc_idx else { return };
        // Only messages strictly before the last cache_control marker qualify
        for msg in messages[..end].iter_mut() {
            if msg.get("role").and_then(|v| v.as_str()) != Some("user") {
                continue;
            }
            let Some(content) = msg.get_mut("content").and_then(|v| v.as_array_mut()) else {
                continue;
            };
            for block in content.iter_mut() {
                if block.get("type").and_then(|v| v.as_str()) != Some("tool_result") {
                    continue;
                }
                let Some(tuid) = block
                    .get("tool_use_id")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                else {
                    continue;
                };
                block["cache_reference"] = Value::String(tuid);
            }
        }
    }

    /// Add `cache_control: ephemeral` to the **last** message's last suitable
    /// content block, creating a cached prefix boundary that allows
    /// `apply_cache_reference` to determine which tool_results are in the
    /// cached portion.  This mirrors claude-code's `addCacheBreakpoints`.
    ///
    /// Skipped when the last block is a `tool_result` (Anthropic does not
    /// support `cache_control` on tool_result blocks) or when it already
    /// has a `cache_control`.
    pub(crate) fn apply_messages_cache_control(body: &mut Value) {
        let Some(messages) = body
            .get_mut("messages")
            .and_then(|v| v.as_array_mut())
        else {
            return;
        };
        let Some(last_msg) = messages.last_mut() else {
            return;
        };
        let Some(content) = last_msg
            .get_mut("content")
            .and_then(|v| v.as_array_mut())
        else {
            return;
        };
        let Some(last_block) = content.last_mut() else {
            return;
        };
        // Anthropic does not support cache_control on tool_result blocks
        if last_block
            .get("type")
            .and_then(|v| v.as_str())
            == Some("tool_result")
        {
            return;
        }
        if last_block.get("cache_control").is_some() {
            return;
        }
        last_block["cache_control"] = serde_json::json!({"type": "ephemeral"});
    }

    /// Split the flat system prompt string at `SYSTEM_PROMPT_DYNAMIC_BOUNDARY`
    /// into Anthropic's block format with `cache_control` on the static part.
    ///
    /// Before: `"system": "static...\n\n__SYSTEM_PROMPT_DYNAMIC_BOUNDARY__\n\ndynamic..."`
    /// After:  `"system": [{"type":"text","text":"static...","cache_control":{"type":"ephemeral"}},
    ///                      {"type":"text","text":"dynamic..."}]`
    pub(crate) fn apply_system_prompt_cache_control(body: &mut Value) {
        let Some(system_str) = body
            .get("system")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
        else {
            return;
        };
        let boundary = runtime::SYSTEM_PROMPT_DYNAMIC_BOUNDARY;
        let Some(split_pos) = system_str.find(boundary) else {
            // No boundary marker — wrap entire system as cached
            if !system_str.is_empty() {
                body["system"] = serde_json::json!([{
                    "type": "text",
                    "text": system_str,
                    "cache_control": { "type": "ephemeral" }
                }]);
            }
            return;
        };
        let static_part = system_str[..split_pos].trim_end().to_string();
        let dynamic_part = system_str[split_pos + boundary.len()..]
            .trim_start()
            .to_string();
        let mut blocks = Vec::new();
        if !static_part.is_empty() {
            blocks.push(serde_json::json!({
                "type": "text",
                "text": static_part,
                "cache_control": { "type": "ephemeral" }
            }));
        }
        if !dynamic_part.is_empty() {
            blocks.push(serde_json::json!({
                "type": "text",
                "text": dynamic_part,
                "cache_control": { "type": "ephemeral" }
            }));
        }
        if !blocks.is_empty() {
            body["system"] = Value::Array(blocks);
        }
    }

    /// Add `cache_control: ephemeral` to the last tool definition so Anthropic
    /// caches the tool schema across requests within the same turn.
    pub(crate) fn apply_tools_cache_control(body: &mut Value) {
        let Some(tools) = body
            .get_mut("tools")
            .and_then(|v| v.as_array_mut())
        else {
            return;
        };
        if let Some(last_tool) = tools.last_mut() {
            if let Some(obj) = last_tool.as_object_mut() {
                obj.insert(
                    "cache_control".to_string(),
                    serde_json::json!({ "type": "ephemeral" }),
                );
            }
        }
    }

}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputMessage {
    pub role: String,
    pub content: Vec<InputContentBlock>,
}

impl InputMessage {
    #[must_use]
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: vec![InputContentBlock::Text { text: text.into() }],
        }
    }

    #[must_use]
    pub fn user_tool_result(
        tool_use_id: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self {
            role: "user".to_string(),
            content: vec![InputContentBlock::ToolResult {
                tool_use_id: tool_use_id.into(),
                content: vec![ToolResultContentBlock::Text {
                    text: content.into(),
                }],
                is_error,
                cache_reference: None,
            }],
        }
    }
}

/// Nested source block for Anthropic's `{"type":"image","source":{...}}` format.
///
/// Serde serialises this directly into the shape that Anthropic's API expects,
/// eliminating the need for a post-processing pass that walks the entire
/// body tree looking for `Image` blocks to normalise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageSource {
    /// Always `"base64"`.
    #[serde(rename = "type")]
    pub source_type: String,
    /// MIME type of the image (e.g. `"image/png"`, `"image/jpeg"`).
    pub media_type: String,
    /// Base64-encoded image data.
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Vec<ToolResultContentBlock>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        is_error: bool,
        /// When in the cached prefix, reference the tool_use_id so the
        /// server can reuse the cached tool_result instead of re-processing.
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_reference: Option<String>,
    },
    Image {
        /// Nested `source` block in Anthropic's expected format, produced
        /// directly at construction time so no JSON-level post-processing
        /// is needed.
        #[serde(rename = "source")]
        source: ImageSource,
    },
    Thinking {
        /// The reasoning content returned by the model. Must be echoed back
        /// verbatim (with `signature`) when the assistant turn is included in
        /// a follow-up request under Anthropic extended thinking.
        thinking: String,
        /// Opaque signature that the Anthropic API uses to authenticate the
        /// thinking block. Mandatory for round-tripping thinking blocks.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    RedactedThinking {
        /// The encrypted redacted-thinking payload returned by the provider.
        /// Must be echoed back verbatim for the tool-use round-trip; unlike a
        /// normal thinking block it carries no signature, so the data itself
        /// is the authentication token.
        data: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultContentBlock {
    Text { text: String },
    Json { value: Value },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: Value,
}

/// Serialize tool definitions to a deterministic JSON text block for embedding
/// in the system prompt. Same input → identical byte sequence.
/// This ensures KV cache prefix stability for local inference servers.
///
/// Output format:
/// ```text
/// # Tools
/// [{"name":"...","description":"...","parameters":{...}},...]
/// ```
#[must_use]
pub fn render_tools_block(tools: &[ToolDefinition]) -> String {
    use std::fmt::Write;
    let mut block = String::from("# Tools\n[");
    for (i, tool) in tools.iter().enumerate() {
        if i > 0 {
            block.push(',');
        }
        block.push('{');
        write!(&mut block, "\"name\":{}", serde_json::to_string(&tool.name).unwrap_or_default()).ok();
        block.push(',');
        if let Some(ref desc) = tool.description {
            write!(&mut block, "\"description\":{}", serde_json::to_string(desc).unwrap_or_default()).ok();
            block.push(',');
        }
        block.push_str("\"parameters\":");
        block.push_str(&serde_json::to_string(&tool.input_schema).unwrap_or_default());
        block.push('}');
    }
    block.push(']');
    block
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    Any,
    Tool { name: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub role: String,
    pub content: Vec<OutputContentBlock>,
    pub model: String,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub stop_sequence: Option<String>,
    #[serde(default)]
    pub usage: Usage,
    #[serde(default)]
    pub request_id: Option<String>,
}

impl MessageResponse {
    #[must_use]
    pub fn total_tokens(&self) -> u32 {
        self.usage.total_tokens()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        #[serde(default = "serde_json::Value::default")]
        input: Value,
    },
    Thinking {
        #[serde(default)]
        thinking: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    RedactedThinking {
        data: Value,
    },
    // Added image output block
    Image {
        data: String,
        mime_type: String,
        filename: Option<String>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    #[serde(default)]
    pub cache_read_input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
}

impl Usage {
    #[must_use]
    pub const fn total_tokens(&self) -> u32 {
        self.input_tokens
            + self.output_tokens
            + self.cache_creation_input_tokens
            + self.cache_read_input_tokens
    }

    #[must_use]
    pub const fn token_usage(&self) -> TokenUsage {
        TokenUsage {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            cache_creation_input_tokens: self.cache_creation_input_tokens,
            cache_read_input_tokens: self.cache_read_input_tokens,
        }
    }

    #[must_use]
    pub fn estimated_cost_usd(&self, model: &str) -> UsageCostEstimate {
        let usage = self.token_usage();
        pricing_for_model(model).map_or_else(
            || usage.estimate_cost_usd(),
            |pricing| usage.estimate_cost_usd_with_pricing(pricing),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageStartEvent {
    pub message: MessageResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageDeltaEvent {
    pub delta: MessageDelta,
    #[serde(default)]
    pub usage: Usage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDelta {
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub stop_sequence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentBlockStartEvent {
    pub index: u32,
    pub content_block: OutputContentBlock,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentBlockDeltaEvent {
    pub index: u32,
    pub delta: ContentBlockDelta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
    ThinkingDelta { thinking: String },
    SignatureDelta { signature: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentBlockStopEvent {
    pub index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageStopEvent {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart(MessageStartEvent),
    MessageDelta(MessageDeltaEvent),
    ContentBlockStart(ContentBlockStartEvent),
    ContentBlockDelta(ContentBlockDeltaEvent),
    ContentBlockStop(ContentBlockStopEvent),
    MessageStop(MessageStopEvent),
}

#[cfg(test)]
mod tests {
    use runtime::format_usd;

    use super::{MessageResponse, Usage};

    #[test]
    fn usage_total_tokens_includes_cache_tokens() {
        let usage = Usage {
            input_tokens: 10,
            cache_creation_input_tokens: 2,
            cache_read_input_tokens: 3,
            output_tokens: 4,
        };

        assert_eq!(usage.total_tokens(), 19);
        assert_eq!(usage.token_usage().total_tokens(), 19);
    }

    #[test]
    fn message_response_estimates_cost_from_model_usage() {
        let response = MessageResponse {
            id: "msg_cost".to_string(),
            kind: "message".to_string(),
            role: "assistant".to_string(),
            content: Vec::new(),
            model: "claude-sonnet-4-20250514".to_string(),
            stop_reason: Some("end_turn".to_string()),
            stop_sequence: None,
            usage: Usage {
                input_tokens: 1_000_000,
                cache_creation_input_tokens: 100_000,
                cache_read_input_tokens: 200_000,
                output_tokens: 500_000,
            },
            request_id: None,
        };

        let cost = response.usage.estimated_cost_usd(&response.model);
        assert_eq!(format_usd(cost.total_cost_usd()), "$54.6750");
        assert_eq!(response.total_tokens(), 1_800_000);
    }

    #[test]
    fn apply_cache_reference_injects_tool_use_id_on_cached_prefix_tool_results() {
        let mut body = serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 100,
            "system": "Be helpful.",
            "messages": [
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "tu_abc", "content": "result"}
                ]},
                {"role": "assistant", "content": [
                    {"type": "tool_use", "id": "tu_abc", "name": "test", "input": {}}
                ]},
                {"role": "user", "content": [
                    {"type": "text", "text": "continue", "cache_control": {"type": "ephemeral"}}
                ]}
            ]
        });
        super::MessageRequest::apply_cache_reference(&mut body);

        let messages = body["messages"].as_array().unwrap();
        let blocks = messages[0]["content"].as_array().unwrap();
        assert_eq!(blocks[0]["cache_reference"], "tu_abc");
        for i in 1..messages.len() {
            if let Some(content) = messages[i]["content"].as_array() {
                for block in content {
                    assert!(
                        block.get("cache_reference").is_none(),
                        "message {i} should not have cache_reference"
                    );
                }
            }
        }
    }

    #[test]
    fn apply_cache_reference_skips_when_no_cache_control_marker() {
        let mut body = serde_json::json!({
            "messages": [
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "tu_xyz", "content": "ok"}
                ]}
            ]
        });
        super::MessageRequest::apply_cache_reference(&mut body);
        let blocks = body["messages"][0]["content"].as_array().unwrap();
        assert!(blocks[0].get("cache_reference").is_none());
    }

    #[test]
    fn apply_cache_reference_skips_non_user_messages_in_prefix() {
        let mut body = serde_json::json!({
            "messages": [
                {"role": "assistant", "content": [
                    {"type": "tool_result", "tool_use_id": "tu_xyz", "content": "ok"}
                ]},
                {"role": "user", "content": [
                    {"type": "text", "text": "go", "cache_control": {"type": "ephemeral"}}
                ]}
            ]
        });
        super::MessageRequest::apply_cache_reference(&mut body);
        // assistant tool_result should NOT get cache_reference
        let blocks = body["messages"][0]["content"].as_array().unwrap();
        assert!(blocks[0].get("cache_reference").is_none());
    }

    #[test]
    fn apply_messages_cache_control_adds_to_last_text_block() {
        let mut body = serde_json::json!({
            "messages": [
                {"role": "user", "content": [
                    {"type": "text", "text": "hello"}
                ]},
                {"role": "assistant", "content": [
                    {"type": "text", "text": "hi"}
                ]},
                {"role": "user", "content": [
                    {"type": "text", "text": "continue"}
                ]}
            ]
        });
        super::MessageRequest::apply_messages_cache_control(&mut body);
        let last = body["messages"][2]["content"].as_array().unwrap();
        assert_eq!(
            last[0]["cache_control"],
            serde_json::json!({"type": "ephemeral"})
        );
    }

    #[test]
    fn apply_messages_cache_control_skips_tool_result_last_block() {
        let mut body = serde_json::json!({
            "messages": [
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "tu_1", "content": "result"}
                ]}
            ]
        });
        super::MessageRequest::apply_messages_cache_control(&mut body);
        let blocks = body["messages"][0]["content"].as_array().unwrap();
        assert!(blocks[0].get("cache_control").is_none());
    }

    #[test]
    fn apply_messages_cache_control_skips_existing_cache_control() {
        let mut body = serde_json::json!({
            "messages": [
                {"role": "user", "content": [
                    {"type": "text", "text": "done", "cache_control": {"type": "ephemeral"}}
                ]}
            ]
        });
        super::MessageRequest::apply_messages_cache_control(&mut body);
        let blocks = body["messages"][0]["content"].as_array().unwrap();
        assert_eq!(
            blocks[0]["cache_control"],
            serde_json::json!({"type": "ephemeral"})
        );
    }

    #[test]
    fn apply_messages_cache_control_empty_messages_does_not_panic() {
        let mut body = serde_json::json!({"messages": []});
        super::MessageRequest::apply_messages_cache_control(&mut body);
        // no panic = pass
    }

    #[test]
    fn apply_messages_cache_control_no_messages_key_does_not_panic() {
        let mut body = serde_json::json!({"model": "test"});
        super::MessageRequest::apply_messages_cache_control(&mut body);
        // no panic = pass
    }

    #[test]
    fn apply_messages_cache_control_content_not_array_does_not_panic() {
        let mut body = serde_json::json!({
            "messages": [{"role": "user", "content": "string content"}]
        });
        super::MessageRequest::apply_messages_cache_control(&mut body);
        // no panic = pass
    }

    #[test]
    fn redacted_thinking_input_block_serializes_with_data() {
        use super::InputContentBlock;
        let block = InputContentBlock::RedactedThinking {
            data: serde_json::json!("ciphertext_blob_abc"),
        };
        let value = serde_json::to_value(&block).expect("block should serialize");
        assert_eq!(value["type"], "redacted_thinking");
        assert_eq!(value["data"], "ciphertext_blob_abc");
    }
}
