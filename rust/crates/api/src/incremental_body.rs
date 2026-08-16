use serde_json::{json, Map, Value};

use crate::types::MessageRequest;

/// Incrementally-built JSON request body that caches per-message serialization
/// and avoids re-serializing the entire message list on every API call.
///
/// ## Typical workflow (per agentic-loop iteration)
/// 1. Build a fresh `MessageRequest` (or reuse the previous one with a new
///    message appended).
/// 2. Call `update(&request)` — only new/uncached messages are serialized.
/// 3. Call `build()` or `build_bytes()` to obtain the final body.
///
/// ## Base invalidation
/// The "base" portion (`model`, `max_tokens`, `system`, `tools`, `tool_choice`,
/// `stream`, tuning knobs) is cached until a field actually changes.  Changes
/// are detected via a simplified content hash of the non-message fields.
///
/// ## Zero-alloc build\_bytes
/// Messages are cached as pre-serialized `Vec<u8>` so `build_bytes()` can
/// concatenate them directly into a single buffer without any intermediate
/// `Value` tree allocation.
#[derive(Debug, Clone)]
pub struct IncrementalBody {
    /// Cached serialisation of the non-message fields (model, system, tools, …).
    base: Option<Map<String, Value>>,
    /// Per-message pre-serialised JSON bytes.
    cached_message_bytes: Vec<Vec<u8>>,
    /// Hash of the base-determining fields at the last rebuild.
    base_hash: u64,
}

impl IncrementalBody {
    pub fn new() -> Self {
        Self {
            base: None,
            cached_message_bytes: Vec::new(),
            base_hash: 0,
        }
    }

    /// Update the cache with a new request.
    ///
    /// * If the base (non-message fields) changed → rebuild base.
    /// * If messages grew (delta) → serialise only the new messages.
    /// * If messages shrunk (e.g. after compaction) → truncate internal cache.
    ///
    /// When `request.cached_message_values` is non-empty, cached JSON values
    /// from that vector are used for delta messages, skipping re-serialisation.
    pub fn update(&mut self, request: &MessageRequest) {
        let new_hash = hash_base(request);

        if self.base.is_none() || new_hash != self.base_hash {
            self.base = Some(serialise_base(request));
            self.base_hash = new_hash;
        }

        let msg_count = request.messages.len();

        if msg_count > self.cached_message_bytes.len() {
            let base_len = self.cached_message_bytes.len();
            for (i, msg) in request.messages[base_len..]
                .iter()
                .enumerate()
            {
                let abs_idx = base_len + i;
                let bytes: Vec<u8> = request
                    .cached_message_values
                    .get(abs_idx)
                    .and_then(|v| v.clone())
                    .map(|val| serde_json::to_vec(&val).unwrap_or_default())
                    .unwrap_or_else(|| serde_json::to_vec(msg).unwrap_or_default());
                self.cached_message_bytes.push(bytes);
            }
        } else if msg_count < self.cached_message_bytes.len() {
            self.cached_message_bytes.truncate(msg_count);
        }
    }

    /// Build the full request body as a JSON `Value`.
    ///
    /// Post-processing (image normalisation, system-prompt cache-control,
    /// tools cache-control) must be applied separately if needed.
    pub fn build(&self) -> Value {
        let mut body = self.base.clone().unwrap_or_default();
        body.insert(
            "messages".to_string(),
            Value::Array(
                self.cached_message_bytes
                    .iter()
                    .map(|b| serde_json::from_slice(b).unwrap_or(Value::Null))
                    .collect(),
            ),
        );
        Value::Object(body)
    }

    /// Build the full request body as serialised JSON bytes.
    ///
    /// Concatenates pre-serialised base fields and pre-serialised messages
    /// directly into a single buffer — no intermediate `Value` trees are
    /// allocated beyond the base fields that are stored as `Value`.
    pub fn build_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(b'{');

        let mut written = false;
        if let Some(ref base) = self.base {
            for (i, (key, val)) in base.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                written = true;
                append_json_string(&mut buf, key);
                buf.push(b':');
                append_json_value(&mut buf, val);
            }
        }

        if written {
            buf.push(b',');
        }
        buf.extend_from_slice(b"\"messages\":[");
        for (i, msg_bytes) in self.cached_message_bytes.iter().enumerate() {
            if i > 0 {
                buf.push(b',');
            }
            buf.extend_from_slice(msg_bytes);
        }
        buf.push(b']');

        buf.push(b'}');
        buf
    }

    /// Clear the cache entirely (forces a full rebuild on next `update`).
    pub fn invalidate(&mut self) {
        self.base = None;
        self.cached_message_bytes.clear();
        self.base_hash = 0;
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Build a `Map` of only the non-message fields from a `MessageRequest`.
///
/// Unlike serialising the full `MessageRequest` and removing `"messages"`,
/// this constructs the map directly from individual fields — never
/// touching (let alone serialising) the potentially-large message vector.
fn serialise_base(request: &MessageRequest) -> Map<String, Value> {
    let mut map = Map::new();

    map.insert("model".into(), Value::String(request.model.clone()));
    map.insert("max_tokens".into(), json!(request.max_tokens));

    serialise_system_cache_control(&mut map, request.system.as_deref());
    if !request.skip_tools {
        serialise_tools_cache_control(&mut map, &request.tools);
    }

    if let Some(ref tc) = request.tool_choice {
        map.insert("tool_choice".into(), serde_json::to_value(tc).unwrap_or_default());
    }

    if request.stream {
        map.insert("stream".into(), Value::Bool(true));
    }

    if let Some(ref v) = request.temperature {
        map.insert("temperature".into(), json!(v));
    }
    if let Some(ref v) = request.top_p {
        map.insert("top_p".into(), json!(v));
    }
    // frequency_penalty and presence_penalty are not supported by Anthropic's
    // /v1/messages endpoint, so we intentionally omit them here.
    // `stop` is renamed to `stop_sequences` for Anthropic.
    if let Some(ref v) = request.stop {
        if !v.is_empty() {
            map.insert("stop_sequences".into(), serde_json::to_value(v).unwrap_or_default());
        }
    }
    if let Some(ref v) = request.reasoning_effort {
        map.insert("reasoning_effort".into(), Value::String(v.clone()));
    }
    if let Some(ref v) = request.thinking {
        map.insert("thinking".into(), serde_json::to_value(v).unwrap_or_default());
    }

    map
}

/// Split the flat system-prompt string at the dynamic boundary and emit
/// the Anthropic block array with `cache_control: ephemeral` on the static
/// portion.  Mirrors `MessageRequest::apply_system_prompt_cache_control`.
fn serialise_system_cache_control(map: &mut Map<String, Value>, system: Option<&str>) {
    let Some(system_str) = system.filter(|s| !s.is_empty()) else {
        return;
    };
    let boundary = runtime::SYSTEM_PROMPT_DYNAMIC_BOUNDARY;
    let blocks = if let Some(split_pos) = system_str.find(boundary) {
        let static_part = system_str[..split_pos].trim_end();
        let dynamic_part = system_str[split_pos + boundary.len()..].trim_start();
        let mut blocks = Vec::new();
        if !static_part.is_empty() {
            blocks.push(serde_json::json!({
                "type": "text",
                "text": static_part,
                "cache_control": { "type": "ephemeral" }
            }));
        }
        if !dynamic_part.is_empty() {
            // The dynamic portion changes every request, so a cache breakpoint
            // here is useless and fragments the prefix cache. Only the static
            // block above keeps `cache_control`.
            blocks.push(serde_json::json!({
                "type": "text",
                "text": dynamic_part
            }));
        }
        blocks
    } else {
        vec![serde_json::json!({
            "type": "text",
            "text": system_str,
            "cache_control": { "type": "ephemeral" }
        })]
    };
    if !blocks.is_empty() {
        map.insert("system".into(), Value::Array(blocks));
    }
}

/// Add `cache_control: ephemeral` to the last tool definition.
/// Mirrors `MessageRequest::apply_tools_cache_control`.
fn serialise_tools_cache_control(map: &mut Map<String, Value>, tools: &Option<Vec<crate::types::ToolDefinition>>) {
    let Some(ref tools) = tools else {
        return;
    };
    if tools.is_empty() {
        return;
    }
    let mut values: Vec<Value> = Vec::with_capacity(tools.len());
    for (i, tool) in tools.iter().enumerate() {
        let mut val = serde_json::to_value(tool).unwrap_or_default();
        if i == tools.len() - 1 {
            if let Some(obj) = val.as_object_mut() {
                obj.insert(
                    "cache_control".to_string(),
                    serde_json::json!({ "type": "ephemeral" }),
                );
            }
        }
        values.push(val);
    }
    map.insert("tools".into(), Value::Array(values));
}

/// Deterministic hash of the non-message fields so we can detect changes.
fn hash_base(request: &MessageRequest) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    request.model.hash(&mut hasher);
    request.max_tokens.hash(&mut hasher);
    request.system.hash(&mut hasher);
    request.stream.hash(&mut hasher);

    if let Some(ref tools) = request.tools {
        for t in tools {
            t.name.hash(&mut hasher);
        }
    }
    request.tool_choice.hash(&mut hasher);
    request.temperature.map(|v| v.to_bits()).hash(&mut hasher);
    request.top_p.map(|v| v.to_bits()).hash(&mut hasher);
    request.frequency_penalty.map(|v| v.to_bits()).hash(&mut hasher);
    request.presence_penalty.map(|v| v.to_bits()).hash(&mut hasher);
    request.stop.hash(&mut hasher);
    request.reasoning_effort.hash(&mut hasher);
    request.thinking.hash(&mut hasher);
    request.skip_tools.hash(&mut hasher);
    hasher.finish()
}

fn append_json_string(buf: &mut Vec<u8>, s: &str) {
    buf.push(b'"');
    for byte in s.bytes() {
        match byte {
            b'"' => buf.extend_from_slice(b"\\\""),
            b'\\' => buf.extend_from_slice(b"\\\\"),
            b'\n' => buf.extend_from_slice(b"\\n"),
            b'\r' => buf.extend_from_slice(b"\\r"),
            b'\t' => buf.extend_from_slice(b"\\t"),
            0x08 => buf.extend_from_slice(b"\\b"),
            0x0C => buf.extend_from_slice(b"\\f"),
            c if c < 0x20 => {
                write_hex_escape(buf, c);
            }
            c => buf.push(c),
        }
    }
    buf.push(b'"');
}

fn write_hex_escape(buf: &mut Vec<u8>, byte: u8) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    buf.push(b'\\');
    buf.push(b'u');
    buf.push(b'0');
    buf.push(b'0');
    buf.push(HEX[(byte >> 4) as usize]);
    buf.push(HEX[(byte & 0x0F) as usize]);
}

fn append_json_value(buf: &mut Vec<u8>, val: &Value) {
    match val {
        Value::Null => buf.extend_from_slice(b"null"),
        Value::Bool(true) => buf.extend_from_slice(b"true"),
        Value::Bool(false) => buf.extend_from_slice(b"false"),
        Value::Number(n) => {
            buf.extend_from_slice(n.to_string().as_bytes());
        }
        Value::String(s) => append_json_string(buf, s),
        Value::Array(arr) => {
            buf.push(b'[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                append_json_value(buf, v);
            }
            buf.push(b']');
        }
        Value::Object(obj) => {
            buf.push(b'{');
            for (i, (key, val)) in obj.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                append_json_string(buf, key);
                buf.push(b':');
                append_json_value(buf, val);
            }
            buf.push(b'}');
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::types::{InputMessage, ToolDefinition, ToolChoice};

    use super::*;

    fn sample_request(msg_count: usize) -> MessageRequest {
        MessageRequest {
            model: "claude-sonnet-4-6".to_string(),
            max_tokens: 1024,
            messages: Arc::new(
                (0..msg_count)
                    .map(|i| InputMessage::user_text(format!("message {i}")))
                    .collect(),
            ),
            system: Some(Arc::from("You are a helpful assistant.")),
            tools: Some(vec![ToolDefinition {
                name: "bash".to_string(),
                description: Some("Run a shell command".to_string()),
                input_schema: serde_json::json!({"type": "object"}),
            }]),
            tool_choice: Some(ToolChoice::Auto),
            stream: true,
            ..Default::default()
        }
    }

    #[test]
    fn full_build_produces_valid_json() {
        let request = sample_request(3);
        let mut body = IncrementalBody::new();
        body.update(&request);

        let value = body.build();
        assert_eq!(value["model"], "claude-sonnet-4-6");
        assert_eq!(value["max_tokens"], 1024);
        // System prompt is now wrapped in cache_control array by serialise_base.
        assert_eq!(
            value["system"][0]["text"],
            "You are a helpful assistant."
        );
        assert!(value.get("tools").is_some());
        assert_eq!(
            value["messages"].as_array().map(Vec::len),
            Some(3)
        );
    }

    #[test]
    fn incremental_update_only_serialises_delta() {
        let mut body = IncrementalBody::new();

        let req1 = sample_request(2);
        body.update(&req1);
        assert_eq!(body.cached_message_bytes.len(), 2);

        let req2 = sample_request(5);
        body.update(&req2);
        assert_eq!(body.cached_message_bytes.len(), 5);

        let value = body.build();
        assert_eq!(
            value["messages"].as_array().map(Vec::len),
            Some(5)
        );
    }

    #[test]
    fn truncation_handles_compaction() {
        let mut body = IncrementalBody::new();
        body.update(&sample_request(10));
        assert_eq!(body.cached_message_bytes.len(), 10);

        body.update(&sample_request(4));
        assert_eq!(body.cached_message_bytes.len(), 4);

        let value = body.build();
        assert_eq!(
            value["messages"].as_array().map(Vec::len),
            Some(4)
        );
    }

    #[test]
    fn base_hash_changes_on_model_switch() {
        let mut body = IncrementalBody::new();
        let req1 = sample_request(1);

        body.update(&req1);
        let hash1 = body.base_hash;

        let mut req2 = sample_request(1);
        req2.model = "claude-opus-4-6".to_string();
        body.update(&req2);

        assert_ne!(body.base_hash, hash1, "model change should alter base hash");
    }

    #[test]
    fn build_bytes_round_trips() {
        let request = sample_request(3);
        let mut body = IncrementalBody::new();
        body.update(&request);

        let bytes = body.build_bytes();
        let parsed: serde_json::Value =
            serde_json::from_slice(&bytes).expect("build_bytes should be valid JSON");

        assert_eq!(parsed["model"], "claude-sonnet-4-6");
        assert_eq!(parsed["max_tokens"], 1024);
        // System prompt is now wrapped in cache_control array by serialise_base.
        assert_eq!(
            parsed["system"][0]["text"],
            "You are a helpful assistant."
        );
        assert_eq!(
            parsed["messages"].as_array().map(Vec::len),
            Some(3)
        );
        assert_eq!(
            parsed["messages"][0]["content"][0]["text"],
            "message 0"
        );
    }

    #[test]
    fn serialise_base_omits_messages() {
        let request = sample_request(100);
        let map = serialise_base(&request);
        assert!(
            !map.contains_key("messages"),
            "serialise_base must not include the messages field"
        );
        assert_eq!(map.get("model").and_then(|v| v.as_str()), Some("claude-sonnet-4-6"));
        assert_eq!(map.get("max_tokens").and_then(|v| v.as_u64()), Some(1024));
        // System is now wrapped in cache_control array rather than flat string.
        assert!(
            map.get("system").and_then(|v| v.as_array()).is_some(),
            "system should be a cache-controlled array"
        );
    }
}
