//! Wire contract for turns that issue several tool calls at once.
//!
//! The Anthropic Messages API requires every `tool_use` of an assistant message
//! to be answered by `tool_result` blocks in the message that immediately
//! follows it. A turn with two tool calls used to emit one result message per
//! call, so the second result arrived one message too late and the provider
//! rejected the request with `messages.N: tool_use ids were found without
//! tool_result blocks immediately after`.
//!
//! This test drives the real `claw` binary against the mock Anthropic service
//! and asserts on the payload the provider would receive, so it fails against
//! the pre-fix implementation without depending on any private helper. The
//! mock service does not validate pairing itself, which is why the assertion
//! lives here.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use mock_anthropic_service::{MockAnthropicService, SCENARIO_PREFIX};
use serde_json::{json, Value};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

const SCENARIO: &str = "multi_tool_turn_roundtrip";

#[test]
fn parallel_tool_calls_are_answered_in_one_following_message() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should build");
    let workspace = unique_temp_dir("parallel-tool-pairing");
    let config_home = workspace.join("config-home");
    let home = workspace.join("home");
    fs::create_dir_all(&config_home).expect("config home should exist");
    fs::create_dir_all(&home).expect("home should exist");
    fs::write(
        workspace.join("fixture.txt"),
        "alpha parity line\nbeta line\ngamma parity line\n",
    )
    .expect("fixture should write");
    prepare_config_home(&config_home);

    let server = runtime
        .block_on(MockAnthropicService::spawn())
        .expect("mock service should start");
    let output = run_claw(&workspace, &config_home, &home, &server.base_url());
    assert!(
        output.status.success(),
        "claw should complete the parallel tool turn\nstdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let requests = runtime
        .block_on(server.captured_requests())
        .into_iter()
        .filter(|request| request.path == "/v1/messages")
        .map(|request| {
            serde_json::from_str::<Value>(&request.raw_body).expect("request body should be json")
        })
        .collect::<Vec<_>>();

    // The turn issued two tool calls, so the follow-up request must carry their
    // results. Asserting the count also catches a split into two messages.
    let tool_result_requests = requests
        .iter()
        .filter(|body| messages_have_tool_result(body))
        .count();
    assert_eq!(
        tool_result_requests,
        1,
        "tool results must reach the provider in one message; requests were:\n{}",
        describe_requests(&requests),
    );

    let mut inspected_tool_uses = 0_u64;
    for body in &requests {
        let messages = body
            .get("messages")
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("request should carry messages: {body}"));
        for (index, message) in messages.iter().enumerate() {
            let calls = tool_use_ids(message);
            if calls.is_empty() {
                continue;
            }
            inspected_tool_uses += calls.len() as u64;
            let answered = messages
                .get(index + 1)
                .map(tool_result_ids)
                .unwrap_or_default();
            assert_eq!(
                answered,
                calls,
                "messages.{index} tool_use {calls:?} must be answered by the next message; \
                 requests were:\n{}",
                describe_requests(&requests),
            );
        }
    }
    assert!(
        inspected_tool_uses >= 2,
        "the scenario should exercise parallel tool calls, saw {inspected_tool_uses}: {}",
        describe_requests(&requests),
    );

    fs::remove_dir_all(&workspace).expect("workspace cleanup should succeed");
}

/// Writes the permission preset the mock harness runs with.
fn prepare_config_home(config_home: &PathBuf) {
    fs::write(
        config_home.join("settings.json"),
        json!({ "permissions": { "defaultMode": "read-only" } }).to_string(),
    )
    .expect("settings should write");
}

fn run_claw(workspace: &PathBuf, config_home: &PathBuf, home: &PathBuf, base_url: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_claw"))
        .current_dir(workspace)
        .env_clear()
        .env("ANTHROPIC_API_KEY", "test-parallel-tool-key")
        .env("ANTHROPIC_BASE_URL", base_url)
        .env("CLAW_CONFIG_HOME", config_home)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .env("PATH", "/usr/bin:/bin")
        .args([
            "--model",
            "sonnet",
            "--permission-mode",
            "read-only",
            "--output-format=json",
            "--allowedTools",
            "read_file,grep_search",
        ])
        .arg(format!("{SCENARIO_PREFIX}{SCENARIO}"))
        .output()
        .expect("claw should launch")
}

fn messages_have_tool_result(body: &Value) -> bool {
    body.get("messages")
        .and_then(Value::as_array)
        .is_some_and(|messages| {
            messages
                .iter()
                .any(|message| !tool_result_ids(message).is_empty())
        })
}

fn tool_use_ids(message: &Value) -> Vec<String> {
    block_ids(message, "tool_use", "id")
}

fn tool_result_ids(message: &Value) -> Vec<String> {
    block_ids(message, "tool_result", "tool_use_id")
}

fn block_ids(message: &Value, block_type: &str, id_field: &str) -> Vec<String> {
    message
        .get("content")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .filter(|block| block.get("type").and_then(Value::as_str) == Some(block_type))
                .filter_map(|block| block.get(id_field).and_then(Value::as_str))
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Renders the message role/block shape of every captured request for failures.
fn describe_requests(requests: &[Value]) -> String {
    requests
        .iter()
        .enumerate()
        .map(|(index, body)| {
            let shape = body
                .get("messages")
                .and_then(Value::as_array)
                .map(|messages| {
                    messages
                        .iter()
                        .map(|message| {
                            let role = message
                                .get("role")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown");
                            let blocks = message
                                .get("content")
                                .and_then(Value::as_array)
                                .map(|blocks| {
                                    blocks
                                        .iter()
                                        .map(|block| {
                                            block
                                                .get("type")
                                                .and_then(Value::as_str)
                                                .unwrap_or("unknown")
                                                .to_string()
                                        })
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            format!("{role}{blocks:?}")
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            format!("  request {index}: {shape:?}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_millis();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "claw-{label}-{}-{millis}-{counter}",
        std::process::id()
    ))
}
