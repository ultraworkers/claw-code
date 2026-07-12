use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use mock_anthropic_service::{MockAnthropicService, SCENARIO_PREFIX};
use serde_json::{json, Value};

#[test]
fn json_schema_retries_invalid_tool_input_and_emits_validated_structured_output() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let server = runtime
        .block_on(MockAnthropicService::spawn())
        .expect("mock service should start");
    let workspace = create_workspace("json");
    let schema = json!({
        "type": "object",
        "properties": {"name": {"type": "string"}},
        "required": ["name"],
        "additionalProperties": false
    });

    let output = run_claw(
        &workspace,
        Some(&server.base_url()),
        &[
            "--model",
            "sonnet",
            "--permission-mode",
            "read-only",
            "--output-format=json",
            "--json-schema",
            &schema.to_string(),
            "prompt",
            &format!("{SCENARIO_PREFIX}structured_output_retry"),
        ],
    );

    assert_success(&output);
    let response = parse_json_line(&output.stdout);
    assert_eq!(response["structured_output"], json!({"name": "Ada"}));
    assert_eq!(
        response["message"],
        Value::String(r#"{"name":"Ada"}"#.to_string())
    );
    assert_eq!(response["iterations"], Value::from(2));
    assert_eq!(
        response["tool_results"]
            .as_array()
            .expect("tool results array")
            .len(),
        2
    );

    let requests = runtime.block_on(server.captured_requests());
    let message_requests = requests
        .iter()
        .filter(|request| {
            request.path == "/v1/messages" && request.scenario == "structured_output_retry"
        })
        .collect::<Vec<_>>();
    assert_eq!(message_requests.len(), 2);
    for request in &message_requests {
        let body: Value = serde_json::from_str(&request.raw_body).expect("request body JSON");
        let structured_tool = body["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .find(|tool| tool["name"] == "StructuredOutput")
            .expect("StructuredOutput tool should be dynamically installed");
        assert_eq!(structured_tool["input_schema"], schema);
    }
    let retry_body: Value =
        serde_json::from_str(&message_requests[1].raw_body).expect("retry request body JSON");
    assert!(retry_body
        .to_string()
        .contains("Output does not match required schema:"));

    fs::remove_dir_all(workspace).expect("workspace cleanup");
}

#[test]
fn json_schema_equals_form_prints_json_stringify_style_in_text_mode() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let server = runtime
        .block_on(MockAnthropicService::spawn())
        .expect("mock service should start");
    let workspace = create_workspace("text");
    let schema = json!({
        "type": "object",
        "properties": {"name": {"type": "string"}},
        "required": ["name"]
    });
    let schema_flag = format!("--json-schema={schema}");
    let prompt = format!("{SCENARIO_PREFIX}structured_output_retry");

    let output = run_claw(
        &workspace,
        Some(&server.base_url()),
        &[
            "--model",
            "sonnet",
            "--permission-mode",
            "read-only",
            &schema_flag,
            "prompt",
            &prompt,
        ],
    );

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().last(), Some(r#"{"name":"Ada"}"#));

    fs::remove_dir_all(workspace).expect("workspace cleanup");
}

#[test]
fn invalid_json_schemas_fail_before_credentials_or_api_startup() {
    let workspace = create_workspace("invalid");
    for (schema, expected) in [
        ("{", "is not valid JSON"),
        ("[]", "must be a JSON object"),
        (
            r#"{"type":"definitely-not-a-json-schema-type"}"#,
            "is not a valid JSON Schema",
        ),
    ] {
        let output = run_claw(
            &workspace,
            None,
            &[
                "--output-format=json",
                "--json-schema",
                schema,
                "prompt",
                "return data",
            ],
        );
        assert!(
            !output.status.success(),
            "invalid schema unexpectedly succeeded"
        );
        let error = parse_json_line(&output.stdout);
        assert_eq!(error["error_kind"], "invalid_json_schema");
        assert!(
            error["error"]
                .as_str()
                .is_some_and(|message| message.contains(expected)),
            "unexpected error envelope: {error}"
        );
        assert!(!error.to_string().contains("missing_credentials"));
    }
    fs::remove_dir_all(workspace).expect("workspace cleanup");
}

fn run_claw(workspace: &Path, base_url: Option<&str>, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_claw"));
    command
        .current_dir(workspace)
        .env_clear()
        .env("HOME", workspace.join("home"))
        .env("CLAW_CONFIG_HOME", workspace.join("config-home"))
        .env("CLAUDE_CONFIG_DIR", workspace.join("claude-config"))
        .env("NO_COLOR", "1")
        .env("PATH", "/usr/bin:/bin")
        .args(args);
    if let Some(base_url) = base_url {
        command
            .env("ANTHROPIC_API_KEY", "structured-output-test-key")
            .env("ANTHROPIC_BASE_URL", base_url);
    }
    command.output().expect("claw should launch")
}

fn parse_json_line(stdout: &[u8]) -> Value {
    let stdout = String::from_utf8_lossy(stdout);
    stdout
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str(line.trim()).ok())
        .unwrap_or_else(|| panic!("stdout did not contain JSON: {stdout}"))
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "claw failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_workspace(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "claw-structured-output-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("home")).expect("home dir");
    fs::create_dir_all(root.join("config-home")).expect("config dir");
    fs::create_dir_all(root.join("claude-config")).expect("claude config dir");
    root
}
