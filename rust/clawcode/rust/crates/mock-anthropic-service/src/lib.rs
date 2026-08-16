use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use api::{MessageResponse, OutputContentBlock, Usage};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;

pub const SCENARIO_PREFIX: &str = "PARITY_SCENARIO:";
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-6";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub scenario: String,
    pub stream: bool,
    pub raw_body: String,
}

pub struct MockAnthropicService {
    base_url: String,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    shutdown: Option<oneshot::Sender<()>>,
    join_handle: JoinHandle<()>,
}

impl MockAnthropicService {
    pub async fn spawn() -> io::Result<Self> {
        Self::spawn_on("127.0.0.1:0").await
    }

    pub async fn spawn_on(bind_addr: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        let address = listener.local_addr()?;
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let request_state = Arc::clone(&requests);

        let join_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accepted = listener.accept() => {
                        let Ok((socket, _)) = accepted else {
                            break;
                        };
                        let request_state = Arc::clone(&request_state);
                        tokio::spawn(async move {
                            let _ = handle_connection(socket, request_state).await;
                        });
                    }
                }
            }
        });

        Ok(Self {
            base_url: format!("http://{address}"),
            requests,
            shutdown: Some(shutdown_tx),
            join_handle,
        })
    }

    #[must_use]
    pub fn base_url(&self) -> String {
        self.base_url.clone()
    }

    pub async fn captured_requests(&self) -> Vec<CapturedRequest> {
        self.requests.lock().await.clone()
    }
}

impl Drop for MockAnthropicService {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.join_handle.abort();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scenario {
    StreamingText,
    ReadFileRoundtrip,
    GrepChunkAssembly,
    WriteFileAllowed,
    WriteFileDenied,
    BashStdoutRoundtrip,
    BashPermissionPromptApproved,
    BashPermissionPromptDenied,
    PluginToolRoundtrip,
    AutoCompactTriggered,
    TokenCostReporting,
}

impl Scenario {
    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "streaming_text" => Some(Self::StreamingText),
            "read_file_roundtrip" => Some(Self::ReadFileRoundtrip),
            "grep_chunk_assembly" => Some(Self::GrepChunkAssembly),
            "new_file_allowed" => Some(Self::WriteFileAllowed),
            "new_file_denied" => Some(Self::WriteFileDenied),
            "bash_stdout_roundtrip" => Some(Self::BashStdoutRoundtrip),
            "bash_permission_prompt_approved" => Some(Self::BashPermissionPromptApproved),
            "bash_permission_prompt_denied" => Some(Self::BashPermissionPromptDenied),
            "plugin_tool_roundtrip" => Some(Self::PluginToolRoundtrip),
            "auto_compact_triggered" => Some(Self::AutoCompactTriggered),
            "token_cost_reporting" => Some(Self::TokenCostReporting),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::StreamingText => "streaming_text",
            Self::ReadFileRoundtrip => "read_file_roundtrip",
            Self::GrepChunkAssembly => "grep_chunk_assembly",
            Self::WriteFileAllowed => "new_file_allowed",
            Self::WriteFileDenied => "new_file_denied",
            Self::BashStdoutRoundtrip => "bash_stdout_roundtrip",
            Self::BashPermissionPromptApproved => "bash_permission_prompt_approved",
            Self::BashPermissionPromptDenied => "bash_permission_prompt_denied",
            Self::PluginToolRoundtrip => "plugin_tool_roundtrip",
            Self::AutoCompactTriggered => "auto_compact_triggered",
            Self::TokenCostReporting => "token_cost_reporting",
        }
    }
}

async fn handle_connection(
    mut socket: tokio::net::TcpStream,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
) -> io::Result<()> {
    let (method, path, headers, raw_body) = read_http_request(&mut socket).await?;

    // The count_tokens endpoint shares the request shape but expects a
    // different response envelope. Always answer it so the client never blocks.
    if path == "/v1/messages/count_tokens" {
        let response = build_count_tokens_response();
        socket.write_all(response.as_bytes()).await?;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let _ = socket.shutdown().await;
        return Ok(());
    }

    let body: Value = serde_json::from_str(&raw_body)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let stream = body.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);
    // Fall back to StreamingText so every request gets a valid response
    // instead of dropping the socket (which would hang the client).
    let scenario = detect_scenario_from_value(&body).unwrap_or(Scenario::StreamingText);

    requests.lock().await.push(CapturedRequest {
        method,
        path,
        headers,
        scenario: scenario.name().to_string(),
        stream,
        raw_body,
    });

    let response = build_http_response_for_value(&body, scenario);
    socket.write_all(response.as_bytes()).await?;
    // Brief delay so the client has time to drain the response before the
    // socket closes, avoiding a race on Windows where reqwest may not
    // detect EOF on a multi-packet chunked response.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let _ = socket.shutdown().await;
    Ok(())
}

async fn read_http_request(
    socket: &mut tokio::net::TcpStream,
) -> io::Result<(String, String, HashMap<String, String>, String)> {
    let mut buffer = Vec::new();
    let mut header_end = None;

    loop {
        let mut chunk = [0_u8; 1024];
        let read = socket.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(position) = find_header_end(&buffer) {
            header_end = Some(position);
            break;
        }
    }

    let header_end = header_end
        .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "missing http headers"))?;
    let (header_bytes, remaining) = buffer.split_at(header_end);
    let header_text = String::from_utf8(header_bytes.to_vec())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing request line"))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing method"))?
        .to_string();
    let path = request_parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing path"))?
        .to_string();

    let mut headers = HashMap::new();
    let mut content_length = 0_usize;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line.split_once(':').ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "malformed http header line")
        })?;
        let value = value.trim().to_string();
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.parse().map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid content-length: {error}"),
                )
            })?;
        }
        headers.insert(name.to_ascii_lowercase(), value);
    }

    let mut body = remaining[4..].to_vec();
    while body.len() < content_length {
        let mut chunk = vec![0_u8; content_length - body.len()];
        let read = socket.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }

    let body = String::from_utf8(body)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    Ok((method, path, headers, body))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn detect_scenario_from_value(body: &Value) -> Option<Scenario> {
    let messages = body.get("messages")?.as_array()?;
    for message in messages.iter().rev() {
        let Some(content) = message.get("content").and_then(|v| v.as_array()) else {
            continue;
        };
        for block in content.iter().rev() {
            let Some(text) = block.get("text").and_then(|v| v.as_str()) else {
                continue;
            };
            for token in text.split_whitespace() {
                if let Some(suffix) = token.strip_prefix(SCENARIO_PREFIX) {
                    return Scenario::parse(suffix);
                }
            }
        }
    }
    None
}

fn has_tool_result_in_value(body: &Value) -> bool {
    body.get("messages")
        .and_then(|v| v.as_array())
        .is_some_and(|messages| {
            messages.iter().any(|msg| {
                msg.get("content")
                    .and_then(|c| c.as_array())
                    .is_some_and(|content| {
                        content.iter().any(|b| {
                            b.get("type").and_then(|t| t.as_str()) == Some("tool_result")
                        })
                    })
            })
        })
}

fn latest_tool_result_from_value(body: &Value) -> Option<(String, bool)> {
    let messages = body.get("messages")?.as_array()?;
    for message in messages.iter().rev() {
        let content = message.get("content")?.as_array()?;
        for block in content.iter().rev() {
            if block.get("type").and_then(|v| v.as_str()) == Some("tool_result") {
                let content = flatten_value_content(block.get("content")?);
                let is_error = block.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
                return Some((content, is_error));
            }
        }
    }
    None
}

fn flatten_value_content(content: &Value) -> String {
    match content {
        Value::Array(arr) => arr.iter().filter_map(|b| {
            b.get("text").and_then(|v| v.as_str()).map(String::from)
        }).collect::<Vec<_>>().join("\n"),
        Value::String(s) => s.clone(),
        _ => content.to_string(),
    }
}

fn build_http_response_for_value(body: &Value, scenario: Scenario) -> String {
    let stream = body.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);
    if stream {
        let sse_body = build_value_stream_body(body, scenario);
        http_response(
            "200 OK",
            "text/event-stream",
            &sse_body,
            &[("x-request-id", request_id_for(scenario))],
        )
    } else {
        let response = build_value_message_response(body, scenario);
        http_response(
            "200 OK",
            "application/json",
            &serde_json::to_string(&response).expect("message response should serialize"),
            &[("request-id", request_id_for(scenario))],
        )
    }
}

fn build_value_stream_body(body: &Value, scenario: Scenario) -> String {
    match scenario {
        Scenario::StreamingText => streaming_text_sse(),
        Scenario::ReadFileRoundtrip if !has_tool_result_in_value(body) => tool_use_sse(
            "toolu_read_fixture",
            "read_file",
            &[r#"{"path":"fixture.txt"}"#],
        ),
        Scenario::ReadFileRoundtrip => {
            let content = latest_tool_result_from_value(body)
                .map(|(output, _)| extract_read_content(&output))
                .unwrap_or_default();
            final_text_sse(&format!("read_file roundtrip complete: {content}"))
        }
        Scenario::GrepChunkAssembly if !has_tool_result_in_value(body) => tool_use_sse(
            "toolu_grep_fixture",
            "grep_search",
            &[
                "{\"pattern\":\"par",
                "ity\",\"path\":\"fixture.txt\"",
                ",\"output_mode\":\"count\"}",
            ],
        ),
        Scenario::GrepChunkAssembly => {
            let count = latest_tool_result_from_value(body)
                .map(|(output, _)| extract_num_matches(&output))
                .unwrap_or(0);
            final_text_sse(&format!("grep_search matched {count} occurrences"))
        }
        Scenario::WriteFileAllowed if !has_tool_result_in_value(body) => tool_use_sse(
            "toolu_write_allowed",
            "new_file",
            &[r#"{"path":"generated/output.txt","content":"created by mock service\n"}"#],
        ),
        Scenario::WriteFileAllowed => {
            let path = latest_tool_result_from_value(body)
                .map(|(output, _)| extract_file_path(&output))
                .unwrap_or_default();
            final_text_sse(&format!("new_file succeeded: {path}"))
        }
        Scenario::WriteFileDenied if !has_tool_result_in_value(body) => tool_use_sse(
            "toolu_write_denied",
            "new_file",
            &[r#"{"path":"generated/denied.txt","content":"should not exist\n"}"#],
        ),
        Scenario::WriteFileDenied => {
            let output = latest_tool_result_from_value(body)
                .map(|(out, _)| out)
                .unwrap_or_default();
            final_text_sse(&format!("new_file denied as expected: {output}"))
        }
        Scenario::BashStdoutRoundtrip if !has_tool_result_in_value(body) => tool_use_sse(
            "toolu_bash_stdout",
            "bash",
            &[r#"{"command":"printf 'alpha from bash'","timeout":10000}"#],
        ),
        Scenario::BashStdoutRoundtrip => {
            let output = latest_tool_result_from_value(body)
                .map(|(out, _)| extract_bash_stdout(&out))
                .unwrap_or_default();
            final_text_sse(&format!("bash completed: {output}"))
        }
        Scenario::BashPermissionPromptApproved if !has_tool_result_in_value(body) => tool_use_sse(
            "toolu_bash_prompt_allow",
            "bash",
            &[r#"{"command":"printf 'approved via prompt'","timeout":10000}"#],
        ),
        Scenario::BashPermissionPromptApproved => {
            let (output, is_error) = latest_tool_result_from_value(body).unwrap_or_default();
            if is_error {
                final_text_sse(&format!("bash approval unexpectedly failed: {output}"))
            } else {
                final_text_sse(&format!("bash approved and executed: {output}"))
            }
        }
        Scenario::BashPermissionPromptDenied if !has_tool_result_in_value(body) => tool_use_sse(
            "toolu_bash_prompt_deny",
            "bash",
            &[r#"{"command":"printf 'should not run'","timeout":1000}"#],
        ),
        Scenario::BashPermissionPromptDenied => {
            let output = latest_tool_result_from_value(body)
                .map(|(out, _)| out)
                .unwrap_or_default();
            final_text_sse(&format!("bash denied as expected: {output}"))
        }
        Scenario::PluginToolRoundtrip if !has_tool_result_in_value(body) => tool_use_sse(
            "toolu_plugin_echo",
            "plugin_echo",
            &[r#"{"message":"hello from plugin parity"}"#],
        ),
        Scenario::PluginToolRoundtrip => {
            let message = latest_tool_result_from_value(body)
                .map(|(output, _)| extract_plugin_message(&output))
                .unwrap_or_default();
            final_text_sse(&format!("plugin tool completed: {message}"))
        }
        Scenario::AutoCompactTriggered => {
            final_text_sse_with_usage("auto compact parity complete.", 50_000, 200)
        }
        Scenario::TokenCostReporting => {
            final_text_sse_with_usage("token cost reporting parity complete.", 1_000, 500)
        }
    }
}

fn build_value_message_response(body: &Value, scenario: Scenario) -> MessageResponse {
    match scenario {
        Scenario::StreamingText => text_message_response(
            "msg_streaming_text",
            "Mock streaming says hello from the parity harness.",
        ),
        Scenario::ReadFileRoundtrip if !has_tool_result_in_value(body) => tool_message_response(
            "msg_read_file_tool",
            "toolu_read_fixture",
            "read_file",
            json!({"path": "fixture.txt"}),
        ),
        Scenario::ReadFileRoundtrip => {
            let content = latest_tool_result_from_value(body)
                .map(|(output, _)| extract_read_content(&output))
                .unwrap_or_default();
            text_message_response(
                "msg_read_file_final",
                &format!("read_file roundtrip complete: {content}"),
            )
        }
        Scenario::GrepChunkAssembly if !has_tool_result_in_value(body) => tool_message_response(
            "msg_grep_tool",
            "toolu_grep_fixture",
            "grep_search",
            json!({"pattern": "parity", "path": "fixture.txt", "output_mode": "count"}),
        ),
        Scenario::GrepChunkAssembly => {
            let count = latest_tool_result_from_value(body)
                .map(|(output, _)| extract_num_matches(&output))
                .unwrap_or(0);
            text_message_response(
                "msg_grep_final",
                &format!("grep_search matched {count} occurrences"),
            )
        }
        Scenario::WriteFileAllowed if !has_tool_result_in_value(body) => tool_message_response(
            "msg_write_allowed_tool",
            "toolu_write_allowed",
            "new_file",
            json!({"path": "generated/output.txt", "content": "created by mock service\n"}),
        ),
        Scenario::WriteFileAllowed => {
            let path = latest_tool_result_from_value(body)
                .map(|(output, _)| extract_file_path(&output))
                .unwrap_or_default();
            text_message_response(
                "msg_write_allowed_final",
                &format!("new_file succeeded: {path}"),
            )
        }
        Scenario::WriteFileDenied if !has_tool_result_in_value(body) => tool_message_response(
            "msg_write_denied_tool",
            "toolu_write_denied",
            "new_file",
            json!({"path": "generated/denied.txt", "content": "should not exist\n"}),
        ),
        Scenario::WriteFileDenied => {
            let output = latest_tool_result_from_value(body)
                .map(|(out, _)| out)
                .unwrap_or_default();
            text_message_response(
                "msg_write_denied_final",
                &format!("new_file denied as expected: {output}"),
            )
        }
        Scenario::BashStdoutRoundtrip if !has_tool_result_in_value(body) => tool_message_response(
            "msg_bash_stdout_tool",
            "toolu_bash_stdout",
            "bash",
            json!({"command": "printf 'alpha from bash'", "timeout": 10000}),
        ),
        Scenario::BashStdoutRoundtrip => {
            let output = latest_tool_result_from_value(body)
                .map(|(out, _)| extract_bash_stdout(&out))
                .unwrap_or_default();
            text_message_response(
                "msg_bash_stdout_final",
                &format!("bash completed: {output}"),
            )
        }
        Scenario::BashPermissionPromptApproved if !has_tool_result_in_value(body) => tool_message_response(
            "msg_bash_prompt_allow_tool",
            "toolu_bash_prompt_allow",
            "bash",
            json!({"command": "printf 'approved via prompt'", "timeout": 1000}),
        ),
        Scenario::BashPermissionPromptApproved => {
            let (output, is_error) = latest_tool_result_from_value(body).unwrap_or_default();
            if is_error {
                text_message_response(
                    "msg_bash_prompt_allow_error",
                    &format!("bash approval unexpectedly failed: {output}"),
                )
            } else {
                text_message_response(
                    "msg_bash_prompt_allow_final",
                    &format!("bash approved and executed: {output}"),
                )
            }
        }
        Scenario::BashPermissionPromptDenied if !has_tool_result_in_value(body) => tool_message_response(
            "msg_bash_prompt_deny_tool",
            "toolu_bash_prompt_deny",
            "bash",
            json!({"command": "printf 'should not run'", "timeout": 1000}),
        ),
        Scenario::BashPermissionPromptDenied => {
            let output = latest_tool_result_from_value(body)
                .map(|(out, _)| out)
                .unwrap_or_default();
            text_message_response(
                "msg_bash_prompt_deny_final",
                &format!("bash denied as expected: {output}"),
            )
        }
        Scenario::PluginToolRoundtrip if !has_tool_result_in_value(body) => tool_message_response(
            "msg_plugin_tool_start",
            "toolu_plugin_echo",
            "plugin_echo",
            json!({"message": "hello from plugin parity"}),
        ),
        Scenario::PluginToolRoundtrip => {
            let message = latest_tool_result_from_value(body)
                .map(|(output, _)| extract_plugin_message(&output))
                .unwrap_or_default();
            text_message_response(
                "msg_plugin_tool_final",
                &format!("plugin tool completed: {message}"),
            )
        }
        Scenario::AutoCompactTriggered => text_message_response_with_usage(
            "msg_auto_compact_triggered",
            "auto compact parity complete.",
            50_000,
            200,
        ),
        Scenario::TokenCostReporting => text_message_response_with_usage(
            "msg_token_cost_reporting",
            "token cost reporting parity complete.",
            1_000,
            500,
        ),
    }
}

fn request_id_for(scenario: Scenario) -> &'static str {
    match scenario {
        Scenario::StreamingText => "req_streaming_text",
        Scenario::ReadFileRoundtrip => "req_read_file_roundtrip",
        Scenario::GrepChunkAssembly => "req_grep_chunk_assembly",
        Scenario::WriteFileAllowed => "req_new_file_allowed",
        Scenario::WriteFileDenied => "req_new_file_denied",
        Scenario::BashStdoutRoundtrip => "req_bash_stdout_roundtrip",
        Scenario::BashPermissionPromptApproved => "req_bash_permission_prompt_approved",
        Scenario::BashPermissionPromptDenied => "req_bash_permission_prompt_denied",
        Scenario::PluginToolRoundtrip => "req_plugin_tool_roundtrip",
        Scenario::AutoCompactTriggered => "req_auto_compact_triggered",
        Scenario::TokenCostReporting => "req_token_cost_reporting",
    }
}

fn http_response(status: &str, content_type: &str, body: &str, headers: &[(&str, &str)]) -> String {
    let mut extra_headers = String::new();
    for (name, value) in headers {
        use std::fmt::Write as _;
        write!(&mut extra_headers, "{name}: {value}\r\n").expect("header write should succeed");
    }
    // Use Transfer-Encoding: chunked so the client detects EOF via the
    // terminal `0\r\n\r\n` rather than relying on TCP socket shutdown
    // (which behaves differently across platforms, notably on Windows).
    let chunk = format!("{:x}\r\n{body}\r\n", body.len());
    let trailer = "0\r\n\r\n";
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\n{extra_headers}transfer-encoding: chunked\r\nconnection: close\r\n\r\n{chunk}{trailer}",
    )
}

/// Always-valid answer for `POST /v1/messages/count_tokens`.
/// The client (`count_tokens`, anthropic.rs:604-629) only reads `input_tokens`.
fn build_count_tokens_response() -> String {
    let body = json!({ "input_tokens": 10 }).to_string();
    http_response("200 OK", "application/json", &body, &[])
}

fn text_message_response(id: &str, text: &str) -> MessageResponse {
    MessageResponse {
        id: id.to_string(),
        kind: "message".to_string(),
        role: "assistant".to_string(),
        content: vec![OutputContentBlock::Text {
            text: text.to_string(),
        }],
        model: DEFAULT_MODEL.to_string(),
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
        usage: Usage {
            input_tokens: 10,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            output_tokens: 6,
        },
        request_id: None,
    }
}

fn text_message_response_with_usage(
    id: &str,
    text: &str,
    input_tokens: u32,
    output_tokens: u32,
) -> MessageResponse {
    MessageResponse {
        id: id.to_string(),
        kind: "message".to_string(),
        role: "assistant".to_string(),
        content: vec![OutputContentBlock::Text {
            text: text.to_string(),
        }],
        model: DEFAULT_MODEL.to_string(),
        stop_reason: Some("end_turn".to_string()),
        stop_sequence: None,
        usage: Usage {
            input_tokens,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            output_tokens,
        },
        request_id: None,
    }
}

fn tool_message_response(
    id: &str,
    tool_id: &str,
    tool_name: &str,
    input: Value,
) -> MessageResponse {
    tool_message_response_many(
        id,
        &[ToolUseMessage {
            tool_id,
            tool_name,
            input,
        }],
    )
}

struct ToolUseMessage<'a> {
    tool_id: &'a str,
    tool_name: &'a str,
    input: Value,
}

fn tool_message_response_many(id: &str, tool_uses: &[ToolUseMessage<'_>]) -> MessageResponse {
    MessageResponse {
        id: id.to_string(),
        kind: "message".to_string(),
        role: "assistant".to_string(),
        content: tool_uses
            .iter()
            .map(|tool_use| OutputContentBlock::ToolUse {
                id: tool_use.tool_id.to_string(),
                name: tool_use.tool_name.to_string(),
                input: tool_use.input.clone(),
            })
            .collect(),
        model: DEFAULT_MODEL.to_string(),
        stop_reason: Some("tool_use".to_string()),
        stop_sequence: None,
        usage: Usage {
            input_tokens: 10,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            output_tokens: 3,
        },
        request_id: None,
    }
}

fn streaming_text_sse() -> String {
    let mut body = String::new();
    append_sse(
        &mut body,
        "message_start",
        json!({
            "type": "message_start",
            "message": {
                "id": "msg_streaming_text",
                "type": "message",
                "role": "assistant",
                "content": [],
                "model": DEFAULT_MODEL,
                "stop_reason": null,
                "stop_sequence": null,
                "usage": usage_json(11, 0)
            }
        }),
    );
    append_sse(
        &mut body,
        "content_block_start",
        json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "text", "text": ""}
        }),
    );
    append_sse(
        &mut body,
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "Mock streaming "}
        }),
    );
    append_sse(
        &mut body,
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "says hello from the parity harness."}
        }),
    );
    append_sse(
        &mut body,
        "content_block_stop",
        json!({
            "type": "content_block_stop",
            "index": 0
        }),
    );
    append_sse(
        &mut body,
        "message_delta",
        json!({
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn", "stop_sequence": null},
            "usage": usage_json(11, 8)
        }),
    );
    append_sse(&mut body, "message_stop", json!({"type": "message_stop"}));
    body
}

fn tool_use_sse(tool_id: &str, tool_name: &str, partial_json_chunks: &[&str]) -> String {
    tool_uses_sse(&[ToolUseSse {
        tool_id,
        tool_name,
        partial_json_chunks,
    }])
}

struct ToolUseSse<'a> {
    tool_id: &'a str,
    tool_name: &'a str,
    partial_json_chunks: &'a [&'a str],
}

fn tool_uses_sse(tool_uses: &[ToolUseSse<'_>]) -> String {
    let mut body = String::new();
    let message_id = tool_uses.first().map_or_else(
        || "msg_tool_use".to_string(),
        |tool_use| format!("msg_{}", tool_use.tool_id),
    );
    append_sse(
        &mut body,
        "message_start",
        json!({
            "type": "message_start",
            "message": {
                "id": message_id,
                "type": "message",
                "role": "assistant",
                "content": [],
                "model": DEFAULT_MODEL,
                "stop_reason": null,
                "stop_sequence": null,
                "usage": usage_json(12, 0)
            }
        }),
    );
    for (index, tool_use) in tool_uses.iter().enumerate() {
        append_sse(
            &mut body,
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": index,
                "content_block": {
                    "type": "tool_use",
                    "id": tool_use.tool_id,
                    "name": tool_use.tool_name,
                    "input": {}
                }
            }),
        );
        for chunk in tool_use.partial_json_chunks {
            append_sse(
                &mut body,
                "content_block_delta",
                json!({
                    "type": "content_block_delta",
                    "index": index,
                    "delta": {"type": "input_json_delta", "partial_json": chunk}
                }),
            );
        }
        append_sse(
            &mut body,
            "content_block_stop",
            json!({
                "type": "content_block_stop",
                "index": index
            }),
        );
    }
    append_sse(
        &mut body,
        "message_delta",
        json!({
            "type": "message_delta",
            "delta": {"stop_reason": "tool_use", "stop_sequence": null},
            "usage": usage_json(12, 4)
        }),
    );
    append_sse(&mut body, "message_stop", json!({"type": "message_stop"}));
    body
}

fn final_text_sse(text: &str) -> String {
    let mut body = String::new();
    append_sse(
        &mut body,
        "message_start",
        json!({
            "type": "message_start",
            "message": {
                "id": unique_message_id(),
                "type": "message",
                "role": "assistant",
                "content": [],
                "model": DEFAULT_MODEL,
                "stop_reason": null,
                "stop_sequence": null,
                "usage": usage_json(14, 0)
            }
        }),
    );
    append_sse(
        &mut body,
        "content_block_start",
        json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "text", "text": ""}
        }),
    );
    append_sse(
        &mut body,
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": text}
        }),
    );
    append_sse(
        &mut body,
        "content_block_stop",
        json!({
            "type": "content_block_stop",
            "index": 0
        }),
    );
    append_sse(
        &mut body,
        "message_delta",
        json!({
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn", "stop_sequence": null},
            "usage": usage_json(14, 7)
        }),
    );
    append_sse(&mut body, "message_stop", json!({"type": "message_stop"}));
    body
}

fn final_text_sse_with_usage(text: &str, input_tokens: u32, output_tokens: u32) -> String {
    let mut body = String::new();
    append_sse(
        &mut body,
        "message_start",
        json!({
            "type": "message_start",
            "message": {
                "id": unique_message_id(),
                "type": "message",
                "role": "assistant",
                "content": [],
                "model": DEFAULT_MODEL,
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {
                    "input_tokens": input_tokens,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0,
                    "output_tokens": 0
                }
            }
        }),
    );
    append_sse(
        &mut body,
        "content_block_start",
        json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {"type": "text", "text": ""}
        }),
    );
    append_sse(
        &mut body,
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": text}
        }),
    );
    append_sse(
        &mut body,
        "content_block_stop",
        json!({
            "type": "content_block_stop",
            "index": 0
        }),
    );
    append_sse(
        &mut body,
        "message_delta",
        json!({
            "type": "message_delta",
            "delta": {"stop_reason": "end_turn", "stop_sequence": null},
            "usage": {
                "input_tokens": input_tokens,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0,
                "output_tokens": output_tokens
            }
        }),
    );
    append_sse(&mut body, "message_stop", json!({"type": "message_stop"}));
    body
}

#[allow(clippy::needless_pass_by_value)]
fn append_sse(buffer: &mut String, event: &str, payload: Value) {
    use std::fmt::Write as _;
    writeln!(buffer, "event: {event}").expect("event write should succeed");
    writeln!(buffer, "data: {payload}").expect("payload write should succeed");
    buffer.push('\n');
}

fn usage_json(input_tokens: u32, output_tokens: u32) -> Value {
    json!({
        "input_tokens": input_tokens,
        "cache_creation_input_tokens": 0,
        "cache_read_input_tokens": 0,
        "output_tokens": output_tokens
    })
}

fn unique_message_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    format!("msg_{nanos}")
}

fn extract_read_content(tool_output: &str) -> String {
    serde_json::from_str::<Value>(tool_output)
        .ok()
        .and_then(|value| {
            value
                .get("file")
                .and_then(|file| file.get("content"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| tool_output.trim().to_string())
}

#[allow(clippy::cast_possible_truncation)]
fn extract_num_matches(tool_output: &str) -> usize {
    serde_json::from_str::<Value>(tool_output)
        .ok()
        .and_then(|value| value.get("numMatches").and_then(Value::as_u64))
        .unwrap_or(0) as usize
}

fn extract_file_path(tool_output: &str) -> String {
    serde_json::from_str::<Value>(tool_output)
        .ok()
        .and_then(|value| {
            value
                .get("filePath")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| tool_output.trim().to_string())
}

fn extract_bash_stdout(tool_output: &str) -> String {
    serde_json::from_str::<Value>(tool_output)
        .ok()
        .and_then(|value| {
            value
                .get("stdout")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| tool_output.trim().to_string())
}

fn extract_plugin_message(tool_output: &str) -> String {
    serde_json::from_str::<Value>(tool_output)
        .ok()
        .and_then(|value| {
            value
                .get("input")
                .and_then(|input| input.get("message"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| tool_output.trim().to_string())
}
