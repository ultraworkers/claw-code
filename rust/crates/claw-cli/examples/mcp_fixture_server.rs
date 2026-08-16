//! Minimal, self-contained MCP server used by the `claw-cli` test suite
//! to exercise end-to-end MCP tool/resource discovery and execution.
//!
//! It speaks the LSP-style JSON-RPC-over-stdio framing (HTTP `Content-Length`
//! headers) so the production MCP client can drive it without any external
//! interpreter (no python3/node) being installed. This keeps the suite portable.
//!
//! Run with no args for the working `echo` server. Pass `--broken` to simulate a
//! server that fails to start (the process exits immediately, so tool discovery
//! for that server fails at the `tool_discovery` phase).

use std::io::{Read, Write};

use serde_json::Value;

fn read_message() -> Option<Value> {
    let mut handle = std::io::stdin().lock();
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = handle.read(&mut byte).ok()?;
        if n == 0 {
            return None;
        }
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header_str = String::from_utf8_lossy(&header);
    let mut length: usize = 0;
    for line in header_str.split("\r\n") {
        if let Some(rest) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = rest.trim().parse().ok()?;
        }
    }
    let mut body = vec![0u8; length];
    handle.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn send_message(message: &Value) {
    let payload = serde_json::to_vec(message).expect("message should serialize");
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(format!("Content-Length: {}\r\n\r\n", payload.len()).as_bytes())
        .expect("stdout write");
    stdout.write_all(&payload).expect("stdout write");
    stdout.flush().expect("stdout flush");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--broken") {
        // Simulate a server that fails to start: exit before any handshake.
        std::process::exit(0);
    }

    loop {
        let request = match read_message() {
            Some(req) => req,
            None => break,
        };
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");

        match method {
            "initialize" => {
                let protocol_version = request
                    .get("params")
                    .and_then(|p| p.get("protocolVersion"))
                    .cloned()
                    .unwrap_or_else(|| Value::String("2024-11-05".to_string()));
                send_message(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": protocol_version,
                        "capabilities": { "tools": {}, "resources": {} },
                        "serverInfo": { "name": "fixture", "version": "1.0.0" }
                    }
                }));
            }
            "tools/list" => {
                send_message(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [{
                            "name": "echo",
                            "description": "Echo from MCP fixture",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "text": { "type": "string" } },
                                "required": ["text"],
                                "additionalProperties": false
                            },
                            "annotations": { "readOnlyHint": true }
                        }]
                    }
                }));
            }
            "tools/call" => {
                let arguments = request
                    .get("params")
                    .and_then(|p| p.get("arguments"))
                    .cloned()
                    .unwrap_or_else(|| Value::Object(Default::default()));
                let text = arguments
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                send_message(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": format!("echo:{text}") }],
                        "structuredContent": { "echoed": text },
                        "isError": false
                    }
                }));
            }
            "resources/list" => {
                send_message(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "resources": [{
                            "uri": "file://guide.txt",
                            "name": "guide",
                            "mimeType": "text/plain"
                        }]
                    }
                }));
            }
            "resources/read" => {
                let uri = request
                    .get("params")
                    .and_then(|p| p.get("uri"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                send_message(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "contents": [{
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": format!("contents for {uri}")
                        }]
                    }
                }));
            }
            _ => {
                // Notifications carry no id and must be ignored. Anything else
                // (including unknown requests with an id) is a method-not-found.
                if id.is_some() {
                    send_message(&serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32601, "message": method }
                    }));
                }
            }
        }
    }
}
