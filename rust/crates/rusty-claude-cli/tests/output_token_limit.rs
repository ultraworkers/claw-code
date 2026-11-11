mod support;

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use mock_anthropic_service::{MockAnthropicService, SCENARIO_PREFIX};
use serde_json::{json, Value};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn max_output_tokens_reaches_openai_compatible_tool_roundtrip() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local mock server");
    listener
        .set_nonblocking(true)
        .expect("nonblocking listener");
    let base_url = format!(
        "http://{}/v1",
        listener.local_addr().expect("server address")
    );
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for turn in 0..2 {
            let deadline = Instant::now() + Duration::from_secs(30);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "CLI did not send turn {turn}");
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("mock server accept failed: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            requests.push(read_openai_request(&mut stream));
            eprintln!("OpenAI mock: received request {}/2", turn + 1);
            let (delta, finish_reason) = if turn == 0 {
                (
                    json!({"role": "assistant", "tool_calls": [{
                        "index": 0, "id": "call_read", "type": "function",
                        "function": {"name": "read_file", "arguments": "{\"path\":\"fixture.txt\"}"}
                    }]}),
                    "tool_calls",
                )
            } else {
                (
                    json!({"role": "assistant", "content": "Local read complete."}),
                    "stop",
                )
            };
            let chunk = json!({
                "id": "chatcmpl-local", "object": "chat.completion.chunk",
                "created": 0, "model": "local-test-model",
                "choices": [{"index": 0, "delta": delta, "finish_reason": finish_reason}]
            });
            let body = format!("data: {chunk}\n\ndata: [DONE]\n\n");
            write!(stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ).expect("write mock response");
            eprintln!("OpenAI mock: sent response {}/2", turn + 1);
        }
        requests
    });

    let workspace = unique_temp_dir("openai workspace with spaces");
    let config_home = workspace.join("config-home");
    let home = workspace.join("home");
    fs::create_dir_all(&config_home).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::write(
        config_home.join("settings.json"),
        json!({"plugins": {"maxOutputTokens": 512}}).to_string(),
    )
    .unwrap();
    fs::write(
        workspace.join("fixture.txt"),
        "verified local file contents",
    )
    .unwrap();
    let output = output_with_deadline(
        Command::new(env!("CARGO_BIN_EXE_claw"))
            .current_dir(&workspace)
            .env_clear()
            .envs(support::process_environment())
            .env("OPENAI_BASE_URL", base_url)
            .env("OPENAI_API_KEY", "local-test-key")
            .env("CLAW_CONFIG_HOME", &config_home)
            .env("HOME", &home)
            .env("NO_COLOR", "1")
            .args([
                "--model",
                "openai/local-test-model",
                "--permission-mode",
                "read-only",
                "--allowedTools",
                "read_file",
                "--compact",
                "Read fixture.txt once.",
            ]),
    );
    let requests = server.join().expect("mock server should finish");
    assert!(
        output.status.success(),
        "stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Local read complete."));
    for request in &requests {
        assert_eq!(request["max_tokens"], 512);
        assert_eq!(request["model"], "local-test-model");
    }
    assert!(requests[1]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|message| message["role"] == "tool"
            && message["content"]
                .as_str()
                .is_some_and(|content| content.contains("verified local file contents"))));
    fs::remove_dir_all(workspace).unwrap();
}

fn read_openai_request(stream: &mut TcpStream) -> Value {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).expect("request line");
    assert!(line.starts_with("POST /v1/chat/completions "), "{line}");
    let mut length = None;
    loop {
        line.clear();
        assert!(reader.read_line(&mut line).expect("header") > 0);
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = Some(value.trim().parse::<usize>().expect("content length"));
            }
        }
    }
    let length = length.expect("request must have content length");
    assert!(length <= 1024 * 1024, "unexpectedly large mock request");
    let mut body = vec![0; length];
    reader.read_exact(&mut body).expect("request body");
    serde_json::from_slice(&body).expect("request JSON")
}

#[test]
fn max_output_tokens_config_reaches_every_tool_roundtrip_request() {
    assert_max_output_tokens_on_wire(Some(1024), None, 1024);
}

#[test]
fn max_output_tokens_local_config_overrides_user_config() {
    assert_max_output_tokens_on_wire(Some(2048), Some(512), 512);
}

#[test]
fn max_output_tokens_default_is_preserved_without_config() {
    assert_max_output_tokens_on_wire(None, None, 64_000);
}

fn assert_max_output_tokens_on_wire(user: Option<u32>, local: Option<u32>, expected: u32) {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should build");
    let server = runtime
        .block_on(MockAnthropicService::spawn())
        .expect("mock service should start");
    let workspace = unique_temp_dir("workspace with spaces");
    let config_home = workspace.join("config-home");
    let home = workspace.join("home");
    fs::create_dir_all(&config_home).expect("config home should exist");
    fs::create_dir_all(&home).expect("home should exist");
    fs::create_dir_all(workspace.join(".claw")).expect("project config should exist");
    fs::write(workspace.join("fixture.txt"), "alpha parity line\n").expect("fixture should write");
    for (path, limit) in [
        (config_home.join("settings.json"), user),
        (workspace.join(".claw/settings.local.json"), local),
    ] {
        if let Some(limit) = limit {
            fs::write(
                path,
                serde_json::json!({"plugins": {"maxOutputTokens": limit}}).to_string(),
            )
            .expect("token limit config should write");
        }
    }

    let prompt = format!("{SCENARIO_PREFIX}read_file_roundtrip");
    let output = run_claw(
        &workspace,
        &config_home,
        &home,
        &server.base_url(),
        &[
            "--model",
            "sonnet",
            "--permission-mode",
            "read-only",
            "--allowedTools",
            "read_file",
            "--compact",
            &prompt,
        ],
    );
    assert!(
        output.status.success(),
        "claw should complete the tool roundtrip\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let captured = runtime.block_on(server.captured_requests());
    let requests: Vec<_> = captured
        .iter()
        .filter(|request| request.path == "/v1/messages")
        .collect();
    assert_eq!(
        requests.len(),
        2,
        "both sides of the tool roundtrip must run"
    );
    for request in requests {
        let body: Value = serde_json::from_str(&request.raw_body).expect("request should be JSON");
        assert_eq!(
            body["max_tokens"], expected,
            "outgoing token limit must match"
        );
    }
    fs::remove_dir_all(&workspace).expect("workspace cleanup should succeed");
}

fn run_claw(
    cwd: &std::path::Path,
    config_home: &std::path::Path,
    home: &std::path::Path,
    base_url: &str,
    args: &[&str],
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_claw"));
    command
        .current_dir(cwd)
        .env_clear()
        .envs(support::process_environment())
        .env("ANTHROPIC_API_KEY", "test-compact-key")
        .env("ANTHROPIC_BASE_URL", base_url)
        .env("CLAW_CONFIG_HOME", config_home)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .args(args);
    output_with_deadline(&mut command)
}

fn output_with_deadline(command: &mut Command) -> Output {
    let cwd = command.get_current_dir().expect("isolated workspace");
    let stdout_path = cwd.join("claw-stdout.txt");
    let stderr_path = cwd.join("claw-stderr.txt");
    // Files avoid pipe backpressure and let timeout failures report partial output.
    command
        .stdin(Stdio::null())
        .stdout(fs::File::create(&stdout_path).expect("stdout capture"))
        .stderr(fs::File::create(&stderr_path).expect("stderr capture"));
    let mut child = command.spawn().expect("claw should launch");
    // Release the parent's file handles before workspace cleanup on Windows.
    command.stdout(Stdio::null()).stderr(Stdio::null());
    let deadline = Instant::now() + Duration::from_secs(60);
    let (status, timed_out) = loop {
        if let Some(status) = child.try_wait().expect("poll claw") {
            break (status, false);
        }
        if Instant::now() >= deadline {
            child.kill().expect("stop timed-out claw");
            break (child.wait().expect("reap timed-out claw"), true);
        }
        thread::sleep(Duration::from_millis(20));
    };
    let output = Output {
        status,
        stdout: fs::read(&stdout_path).expect("read captured stdout"),
        stderr: fs::read(&stderr_path).expect("read captured stderr"),
    };
    assert!(
        !timed_out,
        "claw exceeded the 60-second test deadline; child terminated\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_millis();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "claw-output-token-limit-{label}-{}-{millis}-{counter}",
        std::process::id()
    ))
}
