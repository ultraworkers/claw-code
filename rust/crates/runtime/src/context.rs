//! Context management for LLM API requests.
//!
//! This module filters session messages before sending to the LLM:
//! - Preserves Thinking blocks verbatim (Anthropic requires them for round-trip)
//! - Estimates token usage
//! - Truncates messages that exceed context window

use std::sync::OnceLock;
use std::time::Instant;

use crate::compression_config::CompressionConfig;
use crate::session::{ContentBlock, ConversationMessage};

/// Tools whose output should be compressed in subsequent API rounds.
const FILTER_TOOLS: &[&str] = &[
    "WebFetch", "WebSearch", "read_file", "new_file", "edit_file", "bash", "grep_search",
];

/// Check whether a time-sensitive tool result has exceeded its TTL.
fn tool_result_expired(tool_name: &str, created_at: Instant, config: &CompressionConfig) -> bool {
    let ttl = match tool_name {
        "WebSearch" => config.websearch_ttl_secs,
        "WebFetch" => config.webfetch_ttl_secs,
        _ => return false,
    };
    created_at.elapsed().as_secs() >= ttl
}

/// Filters conversation messages for LLM API requests, using the global defaults.
///
/// - Thinking blocks: content + signature preserved verbatim for API round-trip.
/// - Large ToolResult (WebFetch, read_file, new_file, edit_file, bash, grep_search):
///   output replaced with structured summary to avoid re-sending content that
///   the AI has already processed.
/// - Position-aware: the last N messages (configurable via env var) keep their
///   full ToolResult output so the model retains access to recent context.
pub fn filter_for_api(messages: &[ConversationMessage]) -> Vec<ConversationMessage> {
    filter_for_api_with_config(messages, CompressionConfig::global())
}

/// Filters conversation messages with explicit config.
pub fn filter_for_api_with_config(
    messages: &[ConversationMessage],
    config: &CompressionConfig,
) -> Vec<ConversationMessage> {
    let preserve_from = messages.len().saturating_sub(config.preserve_recent_messages);
    messages
        .iter()
        .enumerate()
        .filter_map(|(idx, msg)| {
            let is_recent = idx >= preserve_from;
            let filtered_blocks: Vec<ContentBlock> = msg
                .blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Thinking { thinking, signature } => {
                        // Preserve the block verbatim (content + signature).
                        // Anthropic extended thinking requires thinking blocks to
                        // be echoed back to the API unchanged for tool-use turns;
                        // the server authenticates the `signature`.
                        ContentBlock::Thinking {
                            thinking: thinking.clone(),
                            signature: signature.clone(),
                        }
                    }
                    ContentBlock::ToolResult {
                        tool_use_id,
                        tool_name,
                        output,
                        is_error,
                    } if !is_error
                        && output.len() > config.toolresult_min_bytes
                        && FILTER_TOOLS.contains(&tool_name.as_str())
                        && (!is_recent
                            || tool_result_expired(tool_name, msg.created_at, config)) =>
                    {
                        // Generate structured summary preserving key metadata.
                        let summary = summarize_tool_result(tool_name, output);
                        ContentBlock::ToolResult {
                            tool_use_id: tool_use_id.clone(),
                            tool_name: tool_name.clone(),
                            output: summary,
                            is_error: *is_error,
                        }
                    }
                    other => other.clone(),
                })
                .collect();

            // Drop messages that contain ONLY Thinking blocks.  After API
            // conversion strips Thinking blocks entirely (convert.rs), such
            // messages would produce an empty `content` array and get skipped,
            // causing `cached_message_values` to be shorter than
            // `request.messages`.  That index misalignment corrupts the
            // IncrementalBody per-message byte cache, producing duplicate
            // same-role messages that the Anthropic API rejects with
            // "Cannot have 2 or more assistant messages at the end of the
            // list" (or the equivalent user-role error).
            let has_non_thinking = filtered_blocks
                .iter()
                .any(|b| !matches!(b, ContentBlock::Thinking { .. }));
            if !has_non_thinking {
                return None;
            }

            Some(ConversationMessage {
                role: msg.role,
                blocks: filtered_blocks,
                usage: msg.usage.clone(),
                created_at: msg.created_at,
                cached_tokens: msg.cached_tokens.clone(),
                cached_input_message: OnceLock::new(),
            })
        })
        .collect()
}

/// Generate a structured summary for a tool result, preserving key metadata
/// (file paths, exit codes, URLs) while dropping bulk content.
pub(crate) fn summarize_tool_result(tool_name: &str, output: &str) -> String {
    match tool_name {
        "read_file" => {
            let path = extract_json_str(output, "filePath").unwrap_or_default();
            let lines = extract_json_num(output, "numLines")
                .or_else(|| extract_json_num(output, "lineCount"))
                .unwrap_or_default();
            let bytes = extract_json_num(output, "bytesRead").unwrap_or_default();
            format!("[read_file: {path}, {lines} lines, {bytes} bytes \u{2014} content processed]")
        }
        "new_file" => {
            let path = extract_json_str(output, "filePath")
                .or_else(|| extract_json_str(output, "path"))
                .unwrap_or_default();
            let bytes = extract_json_num(output, "bytesWritten")
                .or_else(|| extract_json_num(output, "bytes"))
                .unwrap_or_default();
            format!("[new_file: {path}, {bytes} bytes written \u{2014} content processed]")
        }
        "edit_file" => {
            let path = extract_json_str(output, "filePath")
                .or_else(|| extract_json_str(output, "path"))
                .unwrap_or_default();
            let changed = extract_json_num(output, "linesChanged").unwrap_or_default();
            let diff = extract_json_str(output, "diffPath").unwrap_or_default();
            format!("[edit_file: {path}, {changed} lines changed, diff={diff} \u{2014} content processed]")
        }
        "bash" => {
            let exit = extract_json_num(output, "exitCode")
                .or_else(|| extract_json_num(output, "code"))
                .unwrap_or_default();
            // Keep first 200 chars of stdout for context
            let preview = extract_json_str(output, "stdout")
                .or_else(|| extract_json_str(output, "output"))
                .map(|s| {
                    if s.len() > 200 {
                        let idx = s.char_indices().map(|(i, _)| i).nth(200).unwrap_or(s.len());
                        format!("{}...", &s[..idx])
                    } else {
                        s
                    }
                })
                .unwrap_or_default();
            format!("[bash: exit={exit}, output: {preview}]")
        }
        "WebFetch" => {
            let url = extract_json_str(output, "url").unwrap_or_default();
            format!("[WebFetch: {url} \u{2014} content processed]")
        }
        "WebSearch" => {
            let query = extract_json_str(output, "query").unwrap_or_default();
            let provider = extract_json_str(output, "provider").unwrap_or_default();
            let returned = extract_json_num(output, "resultsReturned").unwrap_or_default();
            format!("[WebSearch: \"{query}\" via {provider}, {returned} results \u{2014} results reviewed]")
        }
        "grep_search" => {
            let files = extract_json_num(output, "num_files").unwrap_or_default();
            let lines = extract_json_num(output, "num_lines").unwrap_or_default();
            format!("[grep_search: {files} files, {lines} matches \u{2014} content processed]")
        }
        _ => {
            let chars = output.chars().count();
            format!("[{tool_name}: {chars} chars — content processed]")
        }
    }
}

/// Extract a string value from a JSON object by key.
fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\":");
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let rest = rest.trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let inner = &rest[1..];
    let mut end = None;
    let mut chars = inner.char_indices();
    while let Some((i, c)) = chars.next() {
        if c == '\\' {
            chars.next();
        } else if c == '"' {
            end = Some(i);
            break;
        }
    }
    Some(inner[..end?].to_string())
}

/// Extract a numeric value from a JSON object by key.
fn extract_json_num(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\":");
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let rest = rest.trim_start();
    if rest.starts_with("null") {
        return None;
    }
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-' && c != '.' && c != 'e' && c != 'E')
        .unwrap_or(rest.len());
    if end > 0 {
        Some(rest[..end].to_string())
    } else {
        None
    }
}

/// Estimates token count for a single message.
/// Delegates to the canonical implementation in `compact.rs`.
pub fn estimate_message_tokens(message: &ConversationMessage) -> usize {
    crate::compact::estimate_message_tokens(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compression_config::CompressionConfig;
    use crate::session::MessageRole;
    use std::sync::OnceLock;

    fn make_thinking_block(content: &str, sig: Option<&str>) -> ContentBlock {
        ContentBlock::Thinking {
            thinking: content.to_string(),
            signature: sig.map(String::from),
        }
    }

    fn config_with_preserve(preserve: usize) -> CompressionConfig {
        CompressionConfig {
            preserve_recent_messages: preserve,
            ..CompressionConfig::default()
        }
    }

    #[test]
    fn filter_preserves_thinking_content_for_api_round_trip() {
        let msg = ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![
                ContentBlock::Text {
                    text: "Hello".to_string(),
                },
                make_thinking_block("Long thinking content...", Some("sig123")),
            ],
            usage: None,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        };

        let filtered = filter_for_api(&[msg]);
        assert_eq!(filtered.len(), 1);

        // Thinking block must be preserved verbatim (content + signature)
        // because the Anthropic API requires it for multi-turn tool use.
        let thinking_block = filtered[0]
            .blocks
            .iter()
            .find(|b| matches!(b, ContentBlock::Thinking { .. }));
        assert!(thinking_block.is_some());

        if let ContentBlock::Thinking { thinking, signature } = thinking_block.unwrap() {
            assert_eq!(thinking, "Long thinking content...");
            assert_eq!(signature, &Some("sig123".to_string()));
        }
    }

    #[test]
    fn filter_preserves_non_thinking_blocks() {
        let msg = ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![
                ContentBlock::Text {
                    text: "Answer".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "1".to_string(),
                    name: "bash".to_string(),
                    input: serde_json::json!({}),
                },
            ],
            usage: None,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        };

        let filtered = filter_for_api(&[msg]);
        assert_eq!(filtered[0].blocks.len(), 2);
    }

    #[test]
    fn filter_replaces_large_toolresult_with_structured_summary() {
        let json_output = r#"{"filePath":"src/session.rs","lineCount":1200,"bytesRead":45000,"content":"use std::..."}"#;
        let long_output = format!("{json_output}{}", "X".repeat(1000));
        let msg = ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "tu1".to_string(),
                tool_name: "read_file".to_string(),
                output: long_output,
                is_error: false,
            }],
            usage: None,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        };

        let config = config_with_preserve(0);
        let filtered = filter_for_api_with_config(&[msg], &config);
        if let ContentBlock::ToolResult { output, .. } = &filtered[0].blocks[0] {
            assert!(output.starts_with("[read_file: src/session.rs"));
            assert!(output.contains("1200 lines"));
            assert!(output.contains("45000 bytes"));
            assert!(output.contains("content processed"));
        } else {
            panic!("Expected ToolResult");
        }
    }

    #[test]
    fn filter_replaces_bash_with_exit_code() {
        let json_output = format!(
            r#"{{"stdout":"test result: ok. 42 passed{}","exitCode":0}}"#,
            "X".repeat(600)
        );
        let msg = ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "tu3".to_string(),
                tool_name: "bash".to_string(),
                output: json_output,
                is_error: false,
            }],
            usage: None,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        };

        let config = config_with_preserve(0);
        let filtered = filter_for_api_with_config(&[msg], &config);
        if let ContentBlock::ToolResult { output, .. } = &filtered[0].blocks[0] {
            assert!(output.starts_with("[bash: exit=0"));
            assert!(output.contains("test result: ok"));
        } else {
            panic!("Expected ToolResult");
        }
    }

    #[test]
    fn filter_preserves_error_results() {
        let msg = ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "tu2".to_string(),
                tool_name: "WebFetch".to_string(),
                output: "Connection refused".to_string(),
                is_error: true,
            }],
            usage: None,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        };

        let filtered = filter_for_api(&[msg]);
        if let ContentBlock::ToolResult { output, .. } = &filtered[0].blocks[0] {
            assert_eq!(output, "Connection refused");
        } else {
            panic!("Expected ToolResult");
        }
    }

    #[test]
    fn filter_preserves_recent_tool_results_verbatim() {
        let make_tool_msg = |id: &str, output: &str| ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: id.to_string(),
                tool_name: "read_file".to_string(),
                output: output.to_string(),
                is_error: false,
            }],
            usage: None,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        };

        let big_output = format!(
            r#"{{"filePath":"big.rs","lineCount":500,"bytesRead":20000,"content":"{}"}}
"#,
            "X".repeat(1000)
        );

        // Create 8 messages: 2 old + 6 recent (within preserve window)
        let messages: Vec<ConversationMessage> = (0..8)
            .map(|i| make_tool_msg(&format!("tu{i}"), &big_output))
            .collect();

        let filtered = filter_for_api(&messages);

        // Old messages (index 0, 1) should be compressed
        if let ContentBlock::ToolResult { output, .. } = &filtered[0].blocks[0] {
            assert!(
                output.starts_with("[read_file:"),
                "old message should be compressed, got: {output}"
            );
        } else {
            panic!("Expected ToolResult at index 0");
        }

        // Recent messages (index 2-7) should preserve full output
        for i in 2..8 {
            if let ContentBlock::ToolResult { output, .. } = &filtered[i].blocks[0] {
                assert!(
                    output.contains("\"filePath\":\"big.rs\""),
                    "recent message {i} should be preserved verbatim, got: {output}"
                );
            } else {
                panic!("Expected ToolResult at index {i}");
            }
        }
    }

    #[test]
    fn filter_drops_thinking_only_messages() {
        let msg = ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![make_thinking_block("some reasoning...", Some("sig_abc"))],
            usage: None,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        };

        let filtered = filter_for_api(&[msg]);
        assert!(
            filtered.is_empty(),
            "thinking-only message should be dropped, got {} messages",
            filtered.len()
        );
    }

    #[test]
    fn filter_preserves_assistant_with_text_and_thinking() {
        let msg = ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![
                ContentBlock::Text {
                    text: "Hello".to_string(),
                },
                make_thinking_block("thinking...", Some("sig1")),
            ],
            usage: None,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        };

        let filtered = filter_for_api(&[msg]);
        assert_eq!(filtered.len(), 1, "text+thinking message should be kept");
    }

    #[test]
    fn filter_compresses_websearch_results() {
        let search_output = serde_json::json!({
            "query": "rust async runtime",
            "provider": "bing",
            "totalResults": 1234567,
            "resultsReturned": 10,
            "results": [
                {"title": "Tokio - An asynchronous Rust runtime", "link": "https://tokio.rs", "snippet": "Tokio is an asynchronous runtime for the Rust programming language that provides the building blocks needed for writing network applications.", "source": "tokio.rs", "date": "2024-01-15"},
                {"title": "Async programming in Rust", "link": "https://rust-lang.github.io/async-book/", "snippet": "This book aims to be a thorough guide to asynchronous programming in Rust, covering everything from basic concepts to advanced patterns.", "source": "rust-lang.github.io", "date": "2024-02-20"},
                {"title": "Understanding async/await", "link": "https://example.com/async-await", "snippet": "A deep dive into how async/await works under the hood in Rust, including the state machine transformation and Future trait.", "source": "example.com", "date": "2024-03-10"},
            ]
        }).to_string();

        assert!(search_output.len() > 500, "test data too small: {} bytes", search_output.len());

        let msg = ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "tu_search".to_string(),
                tool_name: "WebSearch".to_string(),
                output: search_output,
                is_error: false,
            }],
            usage: None,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        };

        let config = config_with_preserve(0);
        let filtered = filter_for_api_with_config(&[msg], &config);

        if let ContentBlock::ToolResult { output, .. } = &filtered[0].blocks[0] {
            assert!(output.starts_with("[WebSearch:"), "got: {output}");
            assert!(output.contains("rust async runtime"), "got: {output}");
            assert!(output.contains("bing"), "got: {output}");
            assert!(output.contains("10 results"), "got: {output}");
            assert!(output.len() < 200, "summary should be short, got {} chars", output.len());
        } else {
            panic!("Expected ToolResult");
        }
    }

    fn large_websearch_output() -> String {
        serde_json::json!({
            "query": "rust async runtime",
            "provider": "bing",
            "totalResults": 1234567,
            "resultsReturned": 10,
            "results": [
                {"title": "Tokio", "link": "https://tokio.rs", "snippet": "Tokio is an asynchronous runtime for the Rust programming language that provides the building blocks needed for writing network applications.", "source": "tokio.rs", "date": "2024-01-15"},
                {"title": "Async book", "link": "https://rust-lang.github.io/async-book/", "snippet": "This book aims to be a thorough guide to asynchronous programming in Rust, covering everything from basic concepts to advanced patterns.", "source": "rust-lang.github.io", "date": "2024-02-20"},
            ]
        })
        .to_string()
    }

    #[test]
    fn websearch_expires_by_default_ttl() {
        let output = large_websearch_output();
        assert!(output.len() > 500);

        let msg = ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "tu_ws".to_string(),
                tool_name: "WebSearch".to_string(),
                output,
                is_error: false,
            }],
            usage: None,
            // 31s > default 15s TTL → expired
            created_at: Instant::now() - std::time::Duration::from_secs(31),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        };

        let config = config_with_preserve(10);
        let filtered = filter_for_api_with_config(&[msg], &config);
        if let ContentBlock::ToolResult { output, .. } = &filtered[0].blocks[0] {
            assert!(
                output.starts_with("[WebSearch:"),
                "expired WebSearch should be compressed, got: {output}"
            );
        } else {
            panic!("Expected ToolResult");
        }
    }

    #[test]
    fn webfetch_expires_by_default_ttl() {
        let output = serde_json::json!({
            "url": "https://example.com",
            "content": "X".repeat(600)
        })
        .to_string();
        assert!(output.len() > 500);

        let msg = ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "tu_wf".to_string(),
                tool_name: "WebFetch".to_string(),
                output,
                is_error: false,
            }],
            usage: None,
            // 61s > default 30s TTL → expired
            created_at: Instant::now() - std::time::Duration::from_secs(61),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        };

        let config = config_with_preserve(10);
        let filtered = filter_for_api_with_config(&[msg], &config);
        if let ContentBlock::ToolResult { output, .. } = &filtered[0].blocks[0] {
            assert!(
                output.starts_with("[WebFetch:"),
                "expired WebFetch should be compressed, got: {output}"
            );
        } else {
            panic!("Expected ToolResult");
        }
    }

    #[test]
    fn recent_websearch_preserved_within_ttl() {
        let output = large_websearch_output();
        assert!(output.len() > 500);

        let msg = ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "tu_ws_fresh".to_string(),
                tool_name: "WebSearch".to_string(),
                output,
                is_error: false,
            }],
            usage: None,
            // 5s < default 15s TTL → not expired
            created_at: Instant::now() - std::time::Duration::from_secs(5),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        };

        let config = config_with_preserve(10);
        let filtered = filter_for_api_with_config(&[msg], &config);
        if let ContentBlock::ToolResult { output, .. } = &filtered[0].blocks[0] {
            assert!(
                output.contains("rust async runtime"),
                "fresh WebSearch in recent window should be preserved, got: {output}"
            );
        } else {
            panic!("Expected ToolResult");
        }
    }

    #[test]
    fn extract_json_str_handles_escaped_quotes() {
        let json = r#"{"path":"file with \"quotes\"","other":"val"}"#;
        assert_eq!(
            extract_json_str(json, "path"),
            Some(r#"file with \"quotes\""#.to_string())
        );
    }

    #[test]
    fn extract_json_str_handles_backslash() {
        let json = r#"{"path":"C:\\Users\\file.txt"}"#;
        assert_eq!(
            extract_json_str(json, "path"),
            Some(r#"C:\\Users\\file.txt"#.to_string())
        );
    }

    #[test]
    fn extract_json_str_missing_key_returns_none() {
        let json = r#"{"a":1,"b":2}"#;
        assert_eq!(extract_json_str(json, "c"), None);
    }

    #[test]
    fn extract_json_num_returns_none_for_null() {
        let json = r#"{"exitCode":null}"#;
        assert_eq!(extract_json_num(json, "exitCode"), None);
    }

    #[test]
    fn extract_json_num_handles_scientific_notation() {
        let json = r#"{"value":1.5e10}"#;
        assert_eq!(extract_json_num(json, "value"), Some("1.5e10".to_string()));
    }

    #[test]
    fn extract_json_num_missing_key_returns_none() {
        let json = r#"{"a":1}"#;
        assert_eq!(extract_json_num(json, "b"), None);
    }
}
