use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use runtime::Session;
use serde_json::Value;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn help_emits_json_when_requested() {
    let root = unique_temp_dir("help-json");
    fs::create_dir_all(&root).expect("temp dir should exist");

    let parsed = assert_json_command(&root, &["--output-format", "json", "help"]);
    assert_eq!(parsed["kind"], "help");
    assert!(parsed["message"]
        .as_str()
        .expect("help text")
        .contains("Usage:"));
}

#[test]
fn export_help_emits_bounded_json_when_requested_384() {
    let root = unique_temp_dir("export-help-json");
    fs::create_dir_all(&root).expect("temp dir should exist");

    let parsed = assert_json_command(&root, &["export", "--help", "--output-format", "json"]);
    assert_eq!(parsed["kind"], "help");
    assert_eq!(parsed["topic"], "export");
    assert_eq!(parsed["command"], "export");
    assert_eq!(
        parsed["usage"],
        "claw export [--session <id|latest>] [--output <path>] [--output-format <format>]"
    );
    assert_eq!(parsed["defaults"]["session"], "latest");
    assert!(parsed["options"].as_array().expect("options").len() >= 4);
    assert!(parsed.get("message").is_none());
}

#[test]
fn export_help_preserves_plaintext_in_text_mode_384() {
    let root = unique_temp_dir("export-help-text");
    fs::create_dir_all(&root).expect("temp dir should exist");

    let output = run_claw(&root, &["export", "--help"], &[]);
    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.starts_with("Export\n"));
    assert!(stdout.contains("Usage            claw export"));
    serde_json::from_str::<Value>(&stdout).expect_err("text help should remain plaintext");
}

#[test]
fn version_emits_json_when_requested() {
    let root = unique_temp_dir("version-json");
    fs::create_dir_all(&root).expect("temp dir should exist");

    let parsed = assert_json_command(&root, &["--output-format", "json", "version"]);
    assert_eq!(parsed["kind"], "version");
    assert_eq!(parsed["version"], env!("CARGO_PKG_VERSION"));
    // Provenance fields must be present for binary identification (#507).
    assert!(
        parsed["build_date"].is_string(),
        "build_date must be a string in version JSON"
    );
    assert!(
        parsed["executable_path"].is_string(),
        "executable_path must be a string in version JSON so callers can identify which binary is running"
    );
}

#[test]
fn status_and_sandbox_emit_json_when_requested() {
    let root = unique_temp_dir("status-sandbox-json");
    fs::create_dir_all(&root).expect("temp dir should exist");

    let status = assert_json_command(&root, &["--output-format", "json", "status"]);
    assert_eq!(status["kind"], "status");
    assert!(status["workspace"]["cwd"].as_str().is_some());

    let sandbox = assert_json_command(&root, &["--output-format", "json", "sandbox"]);
    assert_eq!(sandbox["kind"], "sandbox");
    assert!(sandbox["filesystem_mode"].as_str().is_some());
}

#[test]
fn status_json_surfaces_permission_mode_override_for_security_audit() {
    let root = unique_temp_dir("status-json-permission-mode");
    fs::create_dir_all(&root).expect("temp dir should exist");

    let parsed = assert_json_command(
        &root,
        &[
            "--permission-mode",
            "read-only",
            "--output-format",
            "json",
            "status",
        ],
    );

    assert_eq!(parsed["kind"], "status");
    assert_eq!(parsed["permission_mode"], "read-only");
    assert!(
        parsed["workspace"]["cwd"].as_str().is_some(),
        "status JSON should retain workspace context with permission mode"
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn acp_guidance_emits_json_when_requested() {
    let root = unique_temp_dir("acp-json");
    fs::create_dir_all(&root).expect("temp dir should exist");

    let acp = assert_json_command(&root, &["--output-format", "json", "acp"]);
    assert_eq!(acp["kind"], "acp");
    assert_eq!(acp["schema_version"], "1.0");
    assert_eq!(acp["status"], "unsupported");
    assert_eq!(acp["phase"], "discoverability_only");
    assert_eq!(acp["supported"], false);
    assert_eq!(acp["exit_code"], 0);
    assert_eq!(acp["serve_alias_only"], true);
    assert_eq!(acp["protocol"]["json_rpc"], false);
    assert_eq!(acp["protocol"]["daemon"], false);
    assert!(acp["protocol"]["endpoint"].is_null());
    assert_eq!(
        acp["contracts"]["unsupported_invocation_kind"],
        "unsupported_acp_invocation"
    );
    assert_eq!(acp["discoverability_tracking"], "ROADMAP #64a");
    assert_eq!(acp["tracking"], "ROADMAP #76 / #3033 / #3004");
    assert!(acp["message"]
        .as_str()
        .expect("acp message")
        .contains("discoverability alias"));
}

#[test]
fn inventory_commands_emit_structured_json_when_requested() {
    let root = unique_temp_dir("inventory-json");
    fs::create_dir_all(&root).expect("temp dir should exist");

    let isolated_home = root.join("home");
    let isolated_config = root.join("config-home");
    let isolated_codex = root.join("codex-home");
    fs::create_dir_all(&isolated_home).expect("isolated home should exist");

    let agents = assert_json_command_with_env(
        &root,
        &["--output-format", "json", "agents"],
        &[
            ("HOME", isolated_home.to_str().expect("utf8 home")),
            (
                "CLAW_CONFIG_HOME",
                isolated_config.to_str().expect("utf8 config home"),
            ),
            (
                "CODEX_HOME",
                isolated_codex.to_str().expect("utf8 codex home"),
            ),
        ],
    );
    assert_eq!(agents["kind"], "agents");
    assert_eq!(agents["action"], "list");
    assert_eq!(agents["count"], 0);
    assert_eq!(agents["summary"]["active"], 0);
    assert!(agents["agents"]
        .as_array()
        .expect("agents array")
        .is_empty());

    let mcp = assert_json_command(&root, &["--output-format", "json", "mcp"]);
    assert_eq!(mcp["kind"], "mcp");
    assert_eq!(mcp["action"], "list");
    assert_eq!(mcp["status"], "ok");
    assert!(mcp["config_load_error"].is_null());

    let skills = assert_json_command(&root, &["--output-format", "json", "skills"]);
    assert_eq!(skills["kind"], "skills");
    assert_eq!(skills["action"], "list");

    let plugins = assert_json_command(&root, &["--output-format", "json", "plugins"]);
    assert_eq!(plugins["kind"], "plugin");
    assert_eq!(plugins["action"], "list");
    assert_eq!(plugins["status"], "ok");
    assert!(plugins["config_load_error"].is_null());
    assert!(
        plugins["reload_runtime"].is_boolean(),
        "plugins reload_runtime should be a boolean"
    );
    assert!(
        plugins["target"].is_null(),
        "plugins target should be null when no plugin is targeted"
    );
    assert_eq!(plugins["status"], "ok");
    let plugin_entries = plugins["plugins"].as_array().expect("plugins array");
    for plugin in plugin_entries {
        assert!(
            plugin["lifecycle_state"].is_string(),
            "plugin entries should expose lifecycle_state"
        );
        assert!(
            plugin["lifecycle"]["configured"].is_boolean(),
            "plugin entries should expose lifecycle contract summary"
        );
    }
    assert!(plugins["load_failures"]
        .as_array()
        .expect("plugin load failures array")
        .is_empty());
}

#[test]
fn plugins_json_surfaces_lifecycle_contract_when_plugin_is_installed() {
    let root = unique_temp_dir("plugin-lifecycle-json");
    let workspace = root.join("workspace");
    let home = root.join("home");
    let config_home = root.join("config-home");
    let plugin_root = root.join("source-plugin");
    fs::create_dir_all(&workspace).expect("workspace should exist");
    fs::create_dir_all(plugin_root.join(".claude-plugin")).expect("manifest dir should exist");
    fs::create_dir_all(plugin_root.join("lifecycle")).expect("lifecycle dir should exist");
    fs::write(
        plugin_root.join("lifecycle").join("init.sh"),
        "#!/bin/sh\nexit 0\n",
    )
    .expect("init lifecycle script should write");
    fs::write(
        plugin_root.join("lifecycle").join("shutdown.sh"),
        "#!/bin/sh\nexit 0\n",
    )
    .expect("shutdown lifecycle script should write");
    fs::write(
        plugin_root.join(".claude-plugin").join("plugin.json"),
        r#"{
  "name": "lifecycle-json",
  "version": "1.0.0",
  "description": "lifecycle JSON fixture",
  "lifecycle": {
    "Init": ["./lifecycle/init.sh"],
    "Shutdown": ["./lifecycle/shutdown.sh"]
  }
}"#,
    )
    .expect("plugin manifest should write");

    let parsed = assert_json_command_with_env(
        &workspace,
        &[
            "--output-format",
            "json",
            "plugins",
            "install",
            plugin_root
                .to_str()
                .expect("plugin source path should be utf8"),
        ],
        &[
            ("HOME", home.to_str().expect("home path should be utf8")),
            (
                "CLAW_CONFIG_HOME",
                config_home.to_str().expect("config path should be utf8"),
            ),
        ],
    );

    assert_eq!(parsed["kind"], "plugin");
    assert_eq!(parsed["action"], "install");
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["reload_runtime"], true);
    assert!(parsed["load_failures"]
        .as_array()
        .expect("load_failures array")
        .is_empty());
    let plugins = parsed["plugins"].as_array().expect("plugins array");
    let plugin = plugins
        .iter()
        .find(|plugin| plugin["id"] == "lifecycle-json@external")
        .expect("installed plugin should be present");
    assert_eq!(plugin["enabled"], true);
    assert_eq!(plugin["lifecycle_state"], "ready");
    assert_eq!(plugin["lifecycle"]["configured"], true);
    assert_eq!(plugin["lifecycle"]["init"]["configured"], true);
    assert_eq!(plugin["lifecycle"]["init"]["command_count"], 1);
    assert_eq!(plugin["lifecycle"]["shutdown"]["configured"], true);
    assert_eq!(plugin["lifecycle"]["shutdown"]["command_count"], 1);
}

#[test]
fn agents_command_emits_structured_agent_entries_when_requested() {
    let root = unique_temp_dir("agents-json-populated");
    let workspace = root.join("workspace");
    let project_agents = workspace.join(".codex").join("agents");
    let home = root.join("home");
    let user_agents = home.join(".codex").join("agents");
    let isolated_config = root.join("config-home");
    let isolated_codex = root.join("codex-home");
    fs::create_dir_all(&workspace).expect("workspace should exist");
    write_agent(
        &project_agents,
        "planner",
        "Project planner",
        "gpt-5.4",
        "medium",
    );
    write_agent(
        &project_agents,
        "verifier",
        "Verification agent",
        "gpt-5.4-mini",
        "high",
    );
    write_agent(
        &user_agents,
        "planner",
        "User planner",
        "gpt-5.4-mini",
        "high",
    );

    let parsed = assert_json_command_with_env(
        &workspace,
        &["--output-format", "json", "agents"],
        &[
            ("HOME", home.to_str().expect("utf8 home")),
            (
                "CLAW_CONFIG_HOME",
                isolated_config.to_str().expect("utf8 config home"),
            ),
            (
                "CODEX_HOME",
                isolated_codex.to_str().expect("utf8 codex home"),
            ),
        ],
    );

    assert_eq!(parsed["kind"], "agents");
    assert_eq!(parsed["action"], "list");
    assert_eq!(parsed["count"], 3);
    assert_eq!(parsed["summary"]["active"], 2);
    assert_eq!(parsed["summary"]["shadowed"], 1);
    assert_eq!(parsed["agents"][0]["name"], "planner");
    assert_eq!(parsed["agents"][0]["source"]["id"], "project_claw");
    assert_eq!(parsed["agents"][0]["active"], true);
    assert_eq!(parsed["agents"][1]["name"], "verifier");
    assert_eq!(parsed["agents"][2]["name"], "planner");
    assert_eq!(parsed["agents"][2]["active"], false);
    assert_eq!(parsed["agents"][2]["shadowed_by"]["id"], "project_claw");
}

#[test]
fn bootstrap_and_system_prompt_emit_json_when_requested() {
    let root = unique_temp_dir("bootstrap-system-prompt-json");
    fs::create_dir_all(&root).expect("temp dir should exist");

    let plan = assert_json_command(&root, &["--output-format", "json", "bootstrap-plan"]);
    assert_eq!(plan["kind"], "bootstrap-plan");
    assert!(plan["phases"].as_array().expect("phases").len() > 1);

    let prompt = assert_json_command(&root, &["--output-format", "json", "system-prompt"]);
    assert_eq!(prompt["kind"], "system-prompt");
    assert!(prompt["message"]
        .as_str()
        .expect("prompt text")
        .contains("interactive agent"));
}

#[test]
fn dump_manifests_and_init_emit_json_when_requested() {
    let root = unique_temp_dir("manifest-init-json");
    fs::create_dir_all(&root).expect("temp dir should exist");

    let upstream = write_upstream_fixture(&root);
    let manifests = assert_json_command(
        &root,
        &[
            "--output-format",
            "json",
            "dump-manifests",
            "--manifests-dir",
            upstream.to_str().expect("utf8 upstream"),
        ],
    );
    assert_eq!(manifests["kind"], "dump-manifests");
    assert_eq!(manifests["commands"], 1);
    assert_eq!(manifests["tools"], 1);

    let workspace = root.join("workspace");
    fs::create_dir_all(&workspace).expect("workspace should exist");
    let init = assert_json_command(&workspace, &["--output-format", "json", "init"]);
    assert_eq!(init["kind"], "init");
    assert!(workspace.join("CLAUDE.md").exists());
}

#[test]
fn doctor_and_resume_status_emit_json_when_requested() {
    let root = unique_temp_dir("doctor-resume-json");
    fs::create_dir_all(&root).expect("temp dir should exist");

    let doctor = assert_json_command(&root, &["--output-format", "json", "doctor"]);
    assert_eq!(doctor["kind"], "doctor");
    assert!(
        matches!(doctor["status"].as_str(), Some("ok" | "warn")),
        "doctor may warn on platforms without namespace sandbox/tmux support: {doctor}"
    );
    assert!(doctor["message"].is_string());
    let summary = doctor["summary"].as_object().expect("doctor summary");
    assert!(summary["ok"].as_u64().is_some());
    assert!(summary["warnings"].as_u64().is_some());
    assert!(summary["failures"].as_u64().is_some());

    let checks = doctor["checks"].as_array().expect("doctor checks");
    assert_eq!(checks.len(), 7);
    let check_names = checks
        .iter()
        .map(|check| {
            assert!(check["status"].as_str().is_some());
            assert!(check["summary"].as_str().is_some());
            assert!(check["details"].is_array());
            check["name"].as_str().expect("doctor check name")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        check_names,
        vec![
            "auth",
            "config",
            "install source",
            "workspace",
            "boot preflight",
            "sandbox",
            "system"
        ]
    );

    let install_source = checks
        .iter()
        .find(|check| check["name"] == "install source")
        .expect("install source check");
    assert_eq!(
        install_source["official_repo"],
        "https://github.com/ultraworkers/claw-code"
    );
    assert_eq!(
        install_source["deprecated_install"],
        "cargo install claw-code"
    );

    let workspace = checks
        .iter()
        .find(|check| check["name"] == "workspace")
        .expect("workspace check");
    assert!(workspace["cwd"].as_str().is_some());
    assert!(workspace["in_git_repo"].is_boolean());

    let boot_preflight = checks
        .iter()
        .find(|check| check["name"] == "boot preflight")
        .expect("boot preflight check");
    assert!(boot_preflight["boot_preflight"]["repo"]["exists"].is_boolean());
    assert!(boot_preflight["boot_preflight"]["mcp_startup"]["eligible"].is_boolean());
    assert!(boot_preflight["boot_preflight"]["required_binaries"].is_array());

    let sandbox = checks
        .iter()
        .find(|check| check["name"] == "sandbox")
        .expect("sandbox check");
    assert!(sandbox["filesystem_mode"].as_str().is_some());
    assert!(sandbox["enabled"].is_boolean());
    assert!(sandbox["fallback_reason"].is_null() || sandbox["fallback_reason"].is_string());

    let session_path = write_session_fixture(&root, "resume-json", Some("hello"));
    let resumed = assert_json_command(
        &root,
        &[
            "--output-format",
            "json",
            "--resume",
            session_path.to_str().expect("utf8 session path"),
            "/status",
        ],
    );
    assert_eq!(resumed["kind"], "status");
    // model is null in resume mode (not known without --model flag)
    assert!(resumed["model"].is_null());
    assert_eq!(resumed["usage"]["messages"], 1);
    assert!(resumed["workspace"]["cwd"].as_str().is_some());
    assert!(resumed["sandbox"]["filesystem_mode"].as_str().is_some());
}

#[test]
fn resumed_inventory_commands_emit_structured_json_when_requested() {
    let root = unique_temp_dir("resume-inventory-json");
    let config_home = root.join("config-home");
    let home = root.join("home");
    fs::create_dir_all(&config_home).expect("config home should exist");
    fs::create_dir_all(&home).expect("home should exist");

    let session_path = write_session_fixture(&root, "resume-inventory-json", Some("inventory"));

    let mcp = assert_json_command_with_env(
        &root,
        &[
            "--output-format",
            "json",
            "--resume",
            session_path.to_str().expect("utf8 session path"),
            "/mcp",
        ],
        &[
            (
                "CLAW_CONFIG_HOME",
                config_home.to_str().expect("utf8 config home"),
            ),
            ("HOME", home.to_str().expect("utf8 home")),
        ],
    );
    assert_eq!(mcp["kind"], "mcp");
    assert_eq!(mcp["action"], "list");
    assert!(mcp["servers"].is_array());

    let skills = assert_json_command_with_env(
        &root,
        &[
            "--output-format",
            "json",
            "--resume",
            session_path.to_str().expect("utf8 session path"),
            "/skills",
        ],
        &[
            (
                "CLAW_CONFIG_HOME",
                config_home.to_str().expect("utf8 config home"),
            ),
            ("HOME", home.to_str().expect("utf8 home")),
        ],
    );
    assert_eq!(skills["kind"], "skills");
    assert_eq!(skills["action"], "list");
    assert!(skills["summary"]["total"].is_number());
    assert!(skills["skills"].is_array());

    let agents = assert_json_command_with_env(
        &root,
        &[
            "--output-format",
            "json",
            "--resume",
            session_path.to_str().expect("utf8 session path"),
            "/agents",
        ],
        &[
            (
                "CLAW_CONFIG_HOME",
                config_home.to_str().expect("utf8 config home"),
            ),
            ("HOME", home.to_str().expect("utf8 home")),
        ],
    );
    assert_eq!(agents["kind"], "agents");
    assert_eq!(agents["action"], "list");
    assert!(
        agents["agents"].is_array(),
        "agents field must be a JSON array"
    );
    assert!(
        agents["count"].is_number(),
        "count must be a number, not a text render"
    );

    let plugins = assert_json_command_with_env(
        &root,
        &[
            "--output-format",
            "json",
            "--resume",
            session_path.to_str().expect("utf8 session path"),
            "/plugins",
        ],
        &[
            (
                "CLAW_CONFIG_HOME",
                config_home.to_str().expect("utf8 config home"),
            ),
            ("HOME", home.to_str().expect("utf8 home")),
        ],
    );
    assert_eq!(plugins["kind"], "plugin");
    assert_eq!(plugins["action"], "list");
    assert_eq!(plugins["status"], "ok");
    assert!(plugins["config_load_error"].is_null());
    assert!(
        plugins["reload_runtime"].is_boolean(),
        "plugins reload_runtime should be a boolean"
    );
    assert!(
        plugins["target"].is_null(),
        "plugins target should be null when no plugin is targeted"
    );
}

#[test]
fn resumed_version_and_init_emit_structured_json_when_requested() {
    let root = unique_temp_dir("resume-version-init-json");
    fs::create_dir_all(&root).expect("temp dir should exist");

    let session_path = write_session_fixture(&root, "resume-version-init-json", None);

    let version = assert_json_command(
        &root,
        &[
            "--output-format",
            "json",
            "--resume",
            session_path.to_str().expect("utf8 session path"),
            "/version",
        ],
    );
    assert_eq!(version["kind"], "version");
    assert_eq!(version["version"], env!("CARGO_PKG_VERSION"));

    let init = assert_json_command(
        &root,
        &[
            "--output-format",
            "json",
            "--resume",
            session_path.to_str().expect("utf8 session path"),
            "/init",
        ],
    );
    assert_eq!(init["kind"], "init");
    assert!(root.join("CLAUDE.md").exists());
}

#[test]
fn config_section_json_emits_section_and_value() {
    let root = unique_temp_dir("config-section-json");
    fs::create_dir_all(&root).expect("temp dir should exist");

    // Without a section: should return base envelope (no section field).
    let base = assert_json_command(&root, &["--output-format", "json", "config"]);
    assert_eq!(base["kind"], "config");
    assert!(base["loaded_files"].is_number());
    assert!(base["merged_keys"].is_number());
    assert!(
        base.get("section").is_none(),
        "no section field without section arg"
    );

    // With a known section: should add section + section_value fields.
    for section in &["model", "env", "hooks", "plugins"] {
        let result = assert_json_command(&root, &["--output-format", "json", "config", section]);
        assert_eq!(result["kind"], "config", "section={section}");
        assert_eq!(
            result["section"].as_str(),
            Some(*section),
            "section field must match requested section, got {result:?}"
        );
        assert!(
            result.get("section_value").is_some(),
            "section_value field must be present for section={section}"
        );
    }

    // With an unsupported section: should return ok:false + error field.
    let bad = assert_json_command(&root, &["--output-format", "json", "config", "unknown"]);
    assert_eq!(bad["kind"], "config");
    assert_eq!(bad["ok"], false);
    assert!(bad["error"].as_str().is_some());
    assert!(bad["section"].as_str().is_some());
}

#[test]
fn mcp_json_reports_required_optional_and_redacts_secret_values() {
    let root = unique_temp_dir("mcp-required-optional");
    let config_home = root.join("config-home");
    let home = root.join("home");
    fs::create_dir_all(root.join(".claw")).expect("workspace config should exist");
    fs::create_dir_all(&config_home).expect("config home should exist");
    fs::create_dir_all(&home).expect("home should exist");
    fs::write(
        root.join(".claw").join("settings.json"),
        r#"{
          "mcpServers": {
            "required-stdio": {
              "command": "python3",
              "args": ["-c", "print('ready')"],
              "env": {"TOKEN": "secret-token-value"},
              "required": true
            },
            "optional-remote": {
              "type": "http",
              "url": "https://example.test/mcp",
              "headers": {
                "Authorization": "Bearer secret-header-value",
                "X-Trace": "visible-key-only"
              },
              "required": false
            }
          }
        }"#,
    )
    .expect("mcp config should write");

    let envs = [
        (
            "CLAW_CONFIG_HOME",
            config_home.to_str().expect("config home"),
        ),
        ("HOME", home.to_str().expect("home")),
    ];
    let list = assert_json_command_with_env(&root, &["--output-format", "json", "mcp"], &envs);

    assert_eq!(list["kind"], "mcp");
    assert_eq!(list["action"], "list");
    assert_eq!(list["status"], "ok");
    assert_eq!(list["configured_servers"], 2);
    let servers = list["servers"].as_array().expect("servers array");
    let required = servers
        .iter()
        .find(|server| server["name"] == "required-stdio")
        .expect("required stdio server should be listed");
    let optional = servers
        .iter()
        .find(|server| server["name"] == "optional-remote")
        .expect("optional remote server should be listed");
    assert_eq!(required["required"], true);
    assert_eq!(optional["required"], false);
    assert_eq!(required["details"]["env_keys"][0], "TOKEN");
    assert_eq!(optional["details"]["header_keys"][0], "Authorization");
    assert_eq!(optional["details"]["header_keys"][1], "X-Trace");

    let list_text = serde_json::to_string(&list).expect("mcp list json should serialize");
    assert!(!list_text.contains("secret-token-value"));
    assert!(!list_text.contains("secret-header-value"));
    assert!(!list_text.contains("visible-key-only"));

    let show = assert_json_command_with_env(
        &root,
        &["--output-format", "json", "mcp", "show", "optional-remote"],
        &envs,
    );
    assert_eq!(show["action"], "show");
    assert_eq!(show["status"], "ok");
    assert_eq!(show["server"]["required"], false);
    assert_eq!(show["server"]["details"]["header_keys"][0], "Authorization");
    let show_text = serde_json::to_string(&show).expect("mcp show json should serialize");
    assert!(!show_text.contains("secret-header-value"));
    assert!(!show_text.contains("visible-key-only"));
}

#[test]
fn mcp_degraded_config_and_failed_usage_are_distinct_json_contracts() {
    let root = unique_temp_dir("mcp-degraded-vs-failed");
    let config_home = root.join("config-home");
    let home = root.join("home");
    fs::create_dir_all(&root).expect("workspace should exist");
    fs::create_dir_all(&config_home).expect("config home should exist");
    fs::create_dir_all(&home).expect("home should exist");
    fs::write(
        root.join(".claw.json"),
        r#"{
          "mcpServers": {
            "missing-command": {
              "args": ["arg-only-no-command"],
              "required": true
            }
          }
        }"#,
    )
    .expect("malformed mcp config should write");
    let envs = [
        (
            "CLAW_CONFIG_HOME",
            config_home.to_str().expect("config home"),
        ),
        ("HOME", home.to_str().expect("home")),
    ];

    let degraded = assert_json_command_with_env(&root, &["--output-format", "json", "mcp"], &envs);
    assert_eq!(degraded["kind"], "mcp");
    assert_eq!(degraded["action"], "list");
    assert_eq!(degraded["status"], "degraded");
    assert!(degraded["config_load_error"]
        .as_str()
        .is_some_and(|error| error.contains("mcpServers.missing-command")));
    assert_eq!(degraded["configured_servers"], 0);
    assert!(degraded["servers"].as_array().expect("servers").is_empty());

    let failed_output = run_claw(
        &root,
        &["--output-format", "json", "mcp", "list", "extra"],
        &envs,
    );
    assert!(
        !failed_output.status.success(),
        "unsupported MCP action should exit non-zero"
    );
    let failed: Value =
        serde_json::from_slice(&failed_output.stdout).expect("failed stdout should be json");
    assert_eq!(failed["kind"], "mcp");
    assert_eq!(failed["action"], "error");
    assert_eq!(failed["ok"], false);
    assert_eq!(failed["error_kind"], "unsupported_action");
    assert!(failed.get("config_load_error").is_none());
}

fn assert_json_command(current_dir: &Path, args: &[&str]) -> Value {
    assert_json_command_with_env(current_dir, args, &[])
}

fn assert_json_command_with_env(current_dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> Value {
    let output = run_claw(current_dir, args, envs);
    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout should be valid json")
}

fn run_claw(current_dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_claw"));
    command.current_dir(current_dir).args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("claw should launch")
}

fn write_upstream_fixture(root: &Path) -> PathBuf {
    let upstream = root.join("claw-code");
    let src = upstream.join("src");
    let entrypoints = src.join("entrypoints");
    fs::create_dir_all(&entrypoints).expect("upstream entrypoints dir should exist");
    fs::write(
        src.join("commands.ts"),
        "import FooCommand from './commands/foo'\n",
    )
    .expect("commands fixture should write");
    fs::write(
        src.join("tools.ts"),
        "import ReadTool from './tools/read'\n",
    )
    .expect("tools fixture should write");
    fs::write(
        entrypoints.join("cli.tsx"),
        "if (args[0] === '--version') {}\nstartupProfiler()\n",
    )
    .expect("cli fixture should write");
    upstream
}

fn write_session_fixture(root: &Path, session_id: &str, user_text: Option<&str>) -> PathBuf {
    let session_path = root.join("session.jsonl");
    let mut session = Session::new()
        .with_workspace_root(root.to_path_buf())
        .with_persistence_path(session_path.clone());
    session.session_id = session_id.to_string();
    if let Some(text) = user_text {
        session
            .push_user_text(text)
            .expect("session fixture message should persist");
    } else {
        session
            .save_to_path(&session_path)
            .expect("session fixture should persist");
    }
    session_path
}

fn write_agent(root: &Path, name: &str, description: &str, model: &str, reasoning: &str) {
    fs::create_dir_all(root).expect("agent root should exist");
    fs::write(
        root.join(format!("{name}.toml")),
        format!(
            "name = \"{name}\"\ndescription = \"{description}\"\nmodel = \"{model}\"\nmodel_reasoning_effort = \"{reasoning}\"\n"
        ),
    )
    .expect("agent fixture should write");
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_millis();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "claw-output-format-{label}-{}-{millis}-{counter}",
        std::process::id()
    ))
}
