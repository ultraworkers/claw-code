use std::collections::HashMap;
use std::sync::Arc;

use api::{
    ContentBlockDelta, ContentBlockDeltaEvent, ContentBlockStartEvent, ContentBlockStopEvent,
    InputContentBlock, InputMessage, MessageDeltaEvent, MessageRequest, OpenAiCompatClient,
    OpenAiCompatConfig, OutputContentBlock, StreamEvent, ToolChoice, ToolDefinition,
};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn openai_streaming_requests_opt_into_usage_chunks() {
    let state = Arc::new(Mutex::new(Vec::<CapturedRequest>::new()));
    let sse = concat!(
        "data: {\"id\":\"chatcmpl_openai_stream\",\"model\":\"gpt-5\",\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\n",
        "data: {\"id\":\"chatcmpl_openai_stream\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
        "data: {\"id\":\"chatcmpl_openai_stream\",\"choices\":[],\"usage\":{\"prompt_tokens\":9,\"completion_tokens\":4}}\n\n",
        "data: [DONE]\n\n"
    );
    let server = spawn_server(
        state.clone(),
        vec![http_response_with_headers(
            "200 OK",
            "text/event-stream",
            sse,
            &[("x-request-id", "req_openai_stream")],
        )],
    )
    .await;

    let client = OpenAiCompatClient::new("openai-test-key", OpenAiCompatConfig::openai())
        .with_base_url(server.base_url());
    let mut stream = client
        .stream_message(&sample_request(false))
        .await
        .expect("stream should start");

    assert_eq!(stream.request_id(), Some("req_openai_stream"));

    let mut events = Vec::new();
    while let Some(event) = stream.next_event().await.expect("event should parse") {
        events.push(event);
    }

    assert!(matches!(events[0], StreamEvent::MessageStart(_)));
    assert!(matches!(
        events[1],
        StreamEvent::ContentBlockStart(ContentBlockStartEvent {
            content_block: OutputContentBlock::Text { .. },
            ..
        })
    ));
    assert!(matches!(
        events[2],
        StreamEvent::ContentBlockDelta(ContentBlockDeltaEvent {
            delta: ContentBlockDelta::TextDelta { .. },
            ..
        })
    ));
    assert!(matches!(
        events[3],
        StreamEvent::ContentBlockStop(ContentBlockStopEvent { index: 0 })
    ));
    assert!(matches!(
        events[4],
        StreamEvent::MessageDelta(MessageDeltaEvent { .. })
    ));
    assert!(matches!(events[5], StreamEvent::MessageStop(_)));

    match &events[4] {
        StreamEvent::MessageDelta(MessageDeltaEvent { usage, .. }) => {
            assert_eq!(usage.input_tokens, 9);
            assert_eq!(usage.output_tokens, 4);
        }
        other => panic!("expected message delta, got {other:?}"),
    }

    let captured = state.lock().await;
    let request = captured.first().expect("captured request");
    assert_eq!(request.path, "/chat/completions");
    let body: serde_json::Value = serde_json::from_str(&request.body).expect("json body");
    assert_eq!(body["stream"], json!(true));
    assert_eq!(body["stream_options"], json!({"include_usage": true}));
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapturedRequest {
    path: String,
    headers: HashMap<String, String>,
    body: String,
}

struct TestServer {
    base_url: String,
    join_handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    fn base_url(&self) -> String {
        self.base_url.clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

async fn spawn_server(
    state: Arc<Mutex<Vec<CapturedRequest>>>,
    responses: Vec<String>,
) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener.local_addr().expect("listener addr");
    let join_handle = tokio::spawn(async move {
        for response in responses {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = Vec::new();
            let mut header_end = None;
            loop {
                let mut chunk = [0_u8; 1024];
                let read = socket.read(&mut chunk).await.expect("read request");
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
                if let Some(position) = find_header_end(&buffer) {
                    header_end = Some(position);
                    break;
                }
            }

            let header_end = header_end.expect("headers should exist");
            let (header_bytes, remaining) = buffer.split_at(header_end);
            let header_text = String::from_utf8(header_bytes.to_vec()).expect("utf8 headers");
            let mut lines = header_text.split("\r\n");
            let request_line = lines.next().expect("request line");
            let path = request_line
                .split_whitespace()
                .nth(1)
                .expect("path")
                .to_string();
            let mut headers = HashMap::new();
            let mut content_length = 0_usize;
            for line in lines {
                if line.is_empty() {
                    continue;
                }
                let (name, value) = line.split_once(':').expect("header");
                let value = value.trim().to_string();
                if name.eq_ignore_ascii_case("content-length") {
                    content_length = value.parse().expect("content length");
                }
                headers.insert(name.to_ascii_lowercase(), value);
            }

            let mut body = remaining[4..].to_vec();
            while body.len() < content_length {
                let mut chunk = vec![0_u8; content_length - body.len()];
                let read = socket.read(&mut chunk).await.expect("read body");
                if read == 0 {
                    break;
                }
                body.extend_from_slice(&chunk[..read]);
            }

            state.lock().await.push(CapturedRequest {
                path,
                headers,
                body: String::from_utf8(body).expect("utf8 body"),
            });

            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        }
    });

    TestServer {
        base_url: format!("http://{address}"),
        join_handle,
    }
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn http_response_with_headers(
    status: &str,
    content_type: &str,
    body: &str,
    headers: &[(&str, &str)],
) -> String {
    let mut extra_headers = String::new();
    for (name, value) in headers {
        use std::fmt::Write as _;
        write!(&mut extra_headers, "{name}: {value}\r\n").expect("header write");
    }
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\n{extra_headers}content-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn sample_request(stream: bool) -> MessageRequest {
    MessageRequest {
        model: "grok-3".to_string(),
        max_tokens: 64,
        messages: Arc::new(vec![InputMessage {
            role: "user".to_string(),
            content: vec![InputContentBlock::Text {
                text: "Say hello".to_string(),
            }],
        }]),
        system: Some(Arc::from("Use tools when needed")),
        tools: Some(vec![ToolDefinition {
            name: "weather".to_string(),
            description: Some("Fetches weather".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {"city": {"type": "string"}},
                "required": ["city"]
            }),
        }]),
        tool_choice: Some(ToolChoice::Auto),
        stream,
        ..Default::default()
    }
}
