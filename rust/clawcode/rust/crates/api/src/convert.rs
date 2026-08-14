use std::collections::HashMap;
use std::sync::Arc;

use runtime::image_store::ImageStore;
use runtime::{ContentBlock, ConversationMessage, MessageRole};

use crate::types::ImageSource;
use crate::{InputContentBlock, InputMessage, ToolResultContentBlock};

use serde_json::Value;

/// Core conversion logic.  Returns plain `Vec` (no `Arc` wrapper) so callers
/// that maintain their own accumulator can append delta conversions without
/// an intermediate `Arc` allocation.
///
/// Delta messages (assistant replies, tool results) never contain `ImageRef`
/// blocks, so callers may pass `None` for both `image_cache` and `image_store`
/// when converting a slice that is known to contain no user-originated messages.
///
/// When `model_name` is `Some` and the model is text-only (listed in
/// `LLM_ONLY_MODEL.txt`), all Image and ImageRef blocks are filtered out and
/// replaced with text placeholders describing the attached image.
pub fn convert_messages_inner(
    messages: &[ConversationMessage],
    image_cache: Option<&HashMap<String, String>>,
    image_store: Option<&ImageStore>,
    model_name: Option<&str>,
) -> (Vec<InputMessage>, Vec<Option<Value>>) {
    let is_text_only = model_name.is_some_and(runtime::text_only_models::is_text_only_model);
    let mut input_messages = Vec::with_capacity(messages.len());
    let mut cached_values = Vec::with_capacity(messages.len());

    for message in messages {
        let role = match message.role {
            MessageRole::System | MessageRole::User | MessageRole::Tool => "user",
            MessageRole::Assistant => "assistant",
        };
        let content: Vec<InputContentBlock> = message
            .blocks
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Thinking { thinking, signature } => {
                    // Anthropic extended thinking requires thinking blocks to be
                    // echoed back to the API (content + signature) when the
                    // assistant turn is included in a follow-up request; the
                    // server authenticates the `signature`. Only signed blocks
                    // are passed back — signature-less thinking (provider
                    // redaction placeholders, non-Anthropic reasoning models)
                    // is dropped, matching the pre-fix behaviour.
                    signature.clone().map(|signature| InputContentBlock::Thinking {
                        thinking: thinking.clone(),
                        signature: Some(signature),
                    })
                }
                ContentBlock::RedactedThinking { data } => {
                    // Redacted thinking carries no signature; the ciphertext
                    // `data` itself is the authentication token. Echo it back
                    // verbatim so the Anthropic API can authenticate the
                    // tool-use round-trip.
                    Some(InputContentBlock::RedactedThinking {
                        data: serde_json::Value::String(data.clone()),
                    })
                }
                ContentBlock::Text { text } => {
                    Some(InputContentBlock::Text { text: text.clone() })
                }
                ContentBlock::ToolUse { id, name, input } => Some(InputContentBlock::ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                }),
                ContentBlock::Image {
                    mime_type, data, filename, ..
                } => {
                    if is_text_only {
                        let label = filename.as_deref().unwrap_or(mime_type);
                        Some(InputContentBlock::Text {
                            text: format!(
                                "[Image attached: {label}] (not supported by this model)"
                            ),
                        })
                    } else {
                        Some(InputContentBlock::Image {
                            source: ImageSource {
                                source_type: "base64".to_string(),
                                media_type: mime_type.clone(),
                                data: data.clone(),
                            },
                        })
                    }
                }
                ContentBlock::ImageRef { hash_hex, mime_type, .. } => {
                    if is_text_only {
                        Some(InputContentBlock::Text {
                            text: format!(
                                "[Image attached: {mime_type}] (not supported by this model)"
                            ),
                        })
                    } else {
                        let base64_data = image_cache
                            .and_then(|cache| cache.get(hash_hex))
                            .cloned()
                            .or_else(|| {
                                image_store
                                    .and_then(|store| store.load_base64(hash_hex, mime_type).ok())
                            })
                            .unwrap_or_default();
                        if base64_data.is_empty() {
                            eprintln!(
                                "[IMAGE] Failed to resolve base64 for hash {hash_hex} (mime: {mime_type})"
                            );
                        }
                        Some(InputContentBlock::Image {
                            source: ImageSource {
                                source_type: "base64".to_string(),
                                media_type: mime_type.clone(),
                                data: base64_data,
                            },
                        })
                    }
                }
                ContentBlock::ToolResult {
                    tool_use_id,
                    output,
                    is_error,
                    ..
                } => Some(InputContentBlock::ToolResult {
                    tool_use_id: tool_use_id.clone(),
                    content: vec![ToolResultContentBlock::Text {
                        text: output.clone(),
                    }],
                    is_error: *is_error,
                    cache_reference: None,
                }),
            })
            .collect();

        if content.is_empty() {
            // Message has no non-Thinking content (e.g. only Thinking blocks
            // that were stripped above).  Include a placeholder text block so
            // the message count stays aligned with `cached_message_values` —
            // dropping it here would make `cached_values` shorter than the
            // original message list, corrupting the IncrementalBody per-message
            // byte cache used by `send_raw_request`.
            let input_msg = InputMessage {
                role: role.to_string(),
                content: vec![InputContentBlock::Text {
                    text: String::new(),
                }],
            };
            cached_values.push(None);
            input_messages.push(input_msg);
            continue;
        }

        let input_msg = InputMessage {
            role: role.to_string(),
            content,
        };

        let cached = message
            .cached_input_message
            .get_or_init(|| serde_json::to_value(&input_msg).unwrap_or(Value::Null));

        cached_values.push(Some(cached.clone()));
        input_messages.push(input_msg);
    }

    (input_messages, cached_values)
}

/// Convert the runtime-level `ConversationMessage` list into the
/// API-level `InputMessage` list suitable for Anthropic / OpenAI requests.
///
/// * Thinking blocks are dropped.
/// * `ImageRef` blocks are resolved to base64 via `image_cache` / `image_store`.
/// * When `model_name` is `Some` and the model is text-only, images are
///   replaced with text placeholders.
/// * Returns `Arc<Vec<InputMessage>>` so callers can cheaply share the
///   result across clones (e.g. in `MessageRequest`).
#[must_use]
pub fn convert_messages(
    messages: &[ConversationMessage],
    image_cache: Option<&HashMap<String, String>>,
    image_store: Option<&ImageStore>,
    model_name: Option<&str>,
) -> Arc<Vec<InputMessage>> {
    Arc::new(convert_messages_inner(messages, image_cache, image_store, model_name).0)
}

/// Like `convert_messages` but also returns cached serialised JSON `Value`s
/// for each converted message.
///
/// The cached values are stored in `ConversationMessage.cached_input_message`
/// on the first call and reused on subsequent calls within the same
/// `filter_for_api` batch.  Callers that use `IncrementalBody` should prefer
/// this variant so the body builder can skip re-serialising unchanged messages.
#[must_use]
pub fn convert_messages_cached(
    messages: &[ConversationMessage],
    image_cache: Option<&HashMap<String, String>>,
    image_store: Option<&ImageStore>,
    model_name: Option<&str>,
) -> (Arc<Vec<InputMessage>>, Vec<Option<Value>>) {
    let (msgs, vals) = convert_messages_inner(messages, image_cache, image_store, model_name);
    (Arc::new(msgs), vals)
}

#[cfg(test)]
mod tests {
    use runtime::text_only_models;
    use runtime::{ContentBlock, ConversationMessage, MessageRole};
    use std::sync::{Mutex, OnceLock};

    use super::*;

    fn text_only_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn make_message(blocks: Vec<ContentBlock>) -> ConversationMessage {
        ConversationMessage {
            role: MessageRole::User,
            blocks,
            usage: None,
            created_at: std::time::Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        }
    }

    #[test]
    fn test_text_only_model_filters_image_blocks() {
        let _lock = text_only_lock();
        text_only_models::set_test_entries(vec!["llama-3-8b".to_string()]);

        let messages = vec![make_message(vec![
            ContentBlock::Text {
                text: "Hello".to_string(),
            },
            ContentBlock::Image {
                mime_type: "image/png".to_string(),
                data: "base64data".to_string(),
                filename: Some("screenshot.png".to_string()),
            },
            ContentBlock::Text {
                text: "Look at this".to_string(),
            },
        ])];

        let (converted, _) = convert_messages_inner(&messages, None, None, Some("llama-3-8b"));

        let blocks = &converted[0].content;
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], InputContentBlock::Text { text } if text == "Hello"));
        assert!(matches!(&blocks[1], InputContentBlock::Text { text } if text.contains("screenshot.png")));
        assert!(matches!(&blocks[2], InputContentBlock::Text { text } if text == "Look at this"));
    }

    #[test]
    fn test_text_only_model_filters_imageref_blocks() {
        let _lock = text_only_lock();
        text_only_models::set_test_entries(vec!["text-only-model".to_string()]);

        let messages = vec![make_message(vec![
            ContentBlock::Text {
                text: "Text".to_string(),
            },
            ContentBlock::ImageRef {
                hash_hex: "abc123".to_string(),
                mime_type: "image/png".to_string(),
                filename: Some("photo.png".to_string()),
            },
        ])];

        let (converted, _) = convert_messages_inner(&messages, None, None, Some("text-only-model"));

        let blocks = &converted[0].content;
        assert_eq!(blocks.len(), 2);
        assert!(matches!(&blocks[0], InputContentBlock::Text { .. }));
        assert!(matches!(&blocks[1], InputContentBlock::Text { text } if text.contains("image/png")));
    }

    #[test]
    fn test_multimodal_model_preserves_image_blocks() {
        let _lock = text_only_lock();
        text_only_models::set_test_entries(vec![]);

        let messages = vec![make_message(vec![ContentBlock::Image {
            mime_type: "image/png".to_string(),
            data: "base64data".to_string(),
            filename: Some("test.png".to_string()),
        }])];

        let (converted, _) = convert_messages_inner(&messages, None, None, Some("claude-sonnet-4"));

        let blocks = &converted[0].content;
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], InputContentBlock::Image { .. }));
    }

    #[test]
    fn test_none_model_defaults_to_image_capable() {
        let _lock = text_only_lock();
        text_only_models::set_test_entries(vec![]);

        let messages = vec![make_message(vec![ContentBlock::Image {
            mime_type: "image/png".to_string(),
            data: "base64data".to_string(),
            filename: None,
        }])];

        let (converted, _) = convert_messages_inner(&messages, None, None, None);

        let blocks = &converted[0].content;
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], InputContentBlock::Image { .. }));
    }

    #[test]
    fn test_thinking_block_is_preserved_for_api_round_trip() {
        let messages = vec![make_message(vec![
            ContentBlock::Thinking {
                thinking: "Let me reason carefully.".to_string(),
                signature: Some("sig123".to_string()),
            },
            ContentBlock::ToolUse {
                id: "tu1".to_string(),
                name: "bash".to_string(),
                input: serde_json::json!({ "command": "ls" }),
            },
        ])];

        let (converted, _) = convert_messages_inner(&messages, None, None, None);

        let blocks = &converted[0].content;
        assert_eq!(
            blocks.len(),
            2,
            "thinking block must not be dropped; Anthropic requires it for round-trip"
        );
        assert!(matches!(
            &blocks[0],
            InputContentBlock::Thinking {
                thinking,
                signature,
            } if thinking == "Let me reason carefully."
                && signature.as_deref() == Some("sig123")
        ));
    }

    #[test]
    fn test_thinking_block_serializes_as_anthropic_thinking_shape() {
        let messages = vec![make_message(vec![ContentBlock::Thinking {
            thinking: String::new(),
            signature: Some("sig_abc".to_string()),
        }])];

        let (converted, _) = convert_messages_inner(&messages, None, None, None);

        let value = serde_json::to_value(&converted[0]).expect("message should serialize");
        let block = &value["content"][0];
        assert_eq!(block["type"], "thinking");
        assert_eq!(block["signature"], "sig_abc");
    }

    #[test]
    fn test_signature_less_thinking_block_is_not_sent_to_api() {
        // Signature-less thinking (redaction placeholders, non-Anthropic
        // reasoning models) cannot be authenticated by the Anthropic API, so
        // it must be dropped rather than emitted as a malformed thinking block.
        let messages = vec![make_message(vec![
            ContentBlock::Thinking {
                thinking: "reasoning without signature".to_string(),
                signature: None,
            },
            ContentBlock::Text {
                text: "visible answer".to_string(),
            },
        ])];

        let (converted, _) = convert_messages_inner(&messages, None, None, None);

        let blocks = &converted[0].content;
        assert_eq!(blocks.len(), 1);
        assert!(matches!(&blocks[0], InputContentBlock::Text { text } if text == "visible answer"));
    }

    #[test]
    fn test_redacted_thinking_block_is_echoed_back_with_data() {
        // Redacted thinking carries no signature; the ciphertext `data` itself
        // is the authentication token. It must be echoed verbatim.
        let messages = vec![make_message(vec![
            ContentBlock::RedactedThinking {
                data: "ciphertext_blob_abc".to_string(),
            },
            ContentBlock::ToolUse {
                id: "tu1".to_string(),
                name: "bash".to_string(),
                input: serde_json::json!({ "command": "ls" }),
            },
        ])];

        let (converted, _) = convert_messages_inner(&messages, None, None, None);

        let blocks = &converted[0].content;
        assert_eq!(
            blocks.len(),
            2,
            "redacted thinking block must be echoed back for the tool-use round-trip"
        );
        assert!(matches!(
            &blocks[0],
            InputContentBlock::RedactedThinking { data }
                if data.as_str() == Some("ciphertext_blob_abc")
        ));
    }
}
