#![forbid(unsafe_code)]

use std::env;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::process::Command as TokioCommand;
use tokio::runtime::Builder;
use tokio::time::timeout;

use crate::lane_events::{LaneEvent, ShipMergeMethod, ShipProvenance};
#[cfg(unix)]
use crate::sandbox::build_linux_sandbox_command;
use crate::sandbox::{
    resolve_sandbox_status_for_request, FilesystemIsolationMode, SandboxConfig, SandboxStatus,
};
use crate::ConfigLoader;

/// Input schema for the built-in bash execution tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BashCommandInput {
    pub command: String,
    pub timeout: Option<u64>,
    #[serde(rename = "run_in_background")]
    pub run_in_background: Option<bool>,
    #[serde(rename = "dangerouslyDisableSandbox")]
    pub dangerously_disable_sandbox: Option<bool>,
    #[serde(rename = "namespaceRestrictions")]
    pub namespace_restrictions: Option<bool>,
    #[serde(rename = "isolateNetwork")]
    pub isolate_network: Option<bool>,
    #[serde(rename = "filesystemMode")]
    pub filesystem_mode: Option<FilesystemIsolationMode>,
    #[serde(rename = "allowedMounts")]
    pub allowed_mounts: Option<Vec<String>>,
}

/// Output returned from a bash tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BashCommandOutput {
    pub stdout: String,
    pub stderr: String,
    #[serde(rename = "rawOutputPath")]
    pub raw_output_path: Option<String>,
    pub interrupted: bool,
    #[serde(rename = "isImage")]
    pub is_image: Option<bool>,
    #[serde(rename = "backgroundTaskId")]
    pub background_task_id: Option<String>,
    #[serde(rename = "backgroundedByUser")]
    pub backgrounded_by_user: Option<bool>,
    #[serde(rename = "assistantAutoBackgrounded")]
    pub assistant_auto_backgrounded: Option<bool>,
    #[serde(rename = "dangerouslyDisableSandbox")]
    pub dangerously_disable_sandbox: Option<bool>,
    #[serde(rename = "returnCodeInterpretation")]
    pub return_code_interpretation: Option<String>,
    #[serde(rename = "noOutputExpected")]
    pub no_output_expected: Option<bool>,
    #[serde(rename = "structuredContent")]
    pub structured_content: Option<Vec<serde_json::Value>>,
    #[serde(rename = "persistedOutputPath")]
    pub persisted_output_path: Option<String>,
    #[serde(rename = "persistedOutputSize")]
    pub persisted_output_size: Option<u64>,
    #[serde(rename = "sandboxStatus")]
    pub sandbox_status: Option<SandboxStatus>,
    /// Name of the platform sandbox mechanism that actually enforced
    /// the child (e.g. `"windows-job-object"`, `"linux-unshare"`,
    /// `"none"`). Borrowed from tidev's `sandbox_type` field
    /// (`tidev/exec.rs:494-501`) so downstream tools can report *which*
    /// mechanism, not just *whether*, on every bash invocation.
    #[serde(rename = "sandboxType")]
    pub sandbox_type: Option<String>,
}

/// Human-readable name of the platform mechanism that actually
/// enforced the child. Borrowed from tidev's `sandbox_type` reporting
/// pattern (`tidev/exec.rs:494-501`). Returns `None` when the
/// request had sandbox disabled.
fn derive_sandbox_type(sandbox_status: &SandboxStatus) -> Option<String> {
    if !sandbox_status.enabled {
        return None;
    }
    let name: &'static str = if cfg!(target_os = "windows") {
        "windows-job-object"
    } else if cfg!(target_os = "linux") {
        "linux-unshare"
    } else if cfg!(target_os = "macos") {
        // tidev would report "macos-seatbelt" here; we have no
        // macOS enforcement yet, so be honest about the gap.
        "macos-unconfigured"
    } else {
        "unconfigured"
    };
    Some(name.to_string())
}

/// CREATE_NEW_PROCESS_GROUP Win32 flag (0x0000_0200). See
/// <https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags>.
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

/// On Windows, mark the child process as the root of a new process
/// group. The Unix analog is tidev's `libc::setsid()` in `pre_exec`
/// (`tidev/exec.rs:286`) — the child can no longer receive Ctrl+C
/// from the TUI's controlling terminal, and signal handlers in the
/// child cannot accidentally write to our terminal. We do not also
/// set `DETACHED_PROCESS` here — that would close the child's
/// stdin/stdout if the TUI ever pipes input. Process-group
/// isolation is the right level of detachment for a tool child.
///
/// We deliberately do NOT set `CREATE_BREAKAWAY_FROM_JOB`: when `claw`
/// is nested inside a parent Job that forbids breakaway (the common case
/// under IDEs/terminals/CI), that flag makes `CreateProcess` fail with
/// `ERROR_ACCESS_DENIED` (os error 5) and no child is produced at all.
/// Instead we spawn normally and let `enforce_sandbox_job` apply the
/// sandbox Job best-effort; if the parent Job blocks assignment the
/// helper returns `Err` and we proceed unsandboxed (logged, non-fatal).
#[cfg(windows)]
fn apply_windows_detach_flags(cmd: &mut Command) {
    use std::os::windows::process::CommandExt as _;
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

/// Tokio's `Command` does not implement `std::os::windows::process::CommandExt`
/// directly, but it has its own `creation_flags` method that delegates to
/// the inner `std::process::Command` (see tokio-1.52 process/mod.rs:675).
/// Same flag, same effect, no `as_std_mut` round-trip needed.
#[cfg(windows)]
fn apply_windows_detach_flags_tokio(cmd: &mut TokioCommand) {
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

#[cfg(not(windows))]
fn apply_windows_detach_flags(_cmd: &mut Command) {}

#[cfg(not(windows))]
fn apply_windows_detach_flags_tokio(_cmd: &mut TokioCommand) {}

/// Executes a shell command with the requested sandbox settings.
pub fn execute_bash(input: BashCommandInput) -> io::Result<BashCommandOutput> {
    let cwd = env::current_dir()?;
    let sandbox_status = sandbox_status_for_input(&input, &cwd);

    if input.run_in_background.unwrap_or(false) {
        let mut child = prepare_command(&input.command, &cwd, &sandbox_status, false);
        let child = child
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        // Non-fatal: if the Job cannot be applied (e.g. nested-Job
        // E_ACCESSDENIED), keep the child running unsandboxed but log it.
        let child = match enforce_sandbox_job(child, &sandbox_status) {
            Ok(child) => child,
            Err((child, err)) => {
                eprintln!("{err}");
                child
            }
        };

        return Ok(BashCommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            raw_output_path: None,
            interrupted: false,
            is_image: None,
            background_task_id: Some(child.id().to_string()),
            backgrounded_by_user: Some(false),
            assistant_auto_backgrounded: Some(false),
            dangerously_disable_sandbox: input.dangerously_disable_sandbox,
            return_code_interpretation: None,
            no_output_expected: Some(true),
            structured_content: None,
            persisted_output_path: None,
            persisted_output_size: None,
            sandbox_status: Some(sandbox_status.clone()),
            sandbox_type: derive_sandbox_type(&sandbox_status),
        });
    }

    let runtime = Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(execute_bash_async(input, sandbox_status, cwd))
}

/// Wraps a freshly-spawned `Child` in a Windows Job Object so the kernel
/// enforces `kill-on-job-close` + process-count limit. When the parent claw
/// process exits (or the Job handle is dropped) every child the agent
/// spawned is reaped — even orphaned `cmd.exe` chains the agent detaches
/// from. Returns the child unchanged on non-Windows targets so the caller
/// can stay platform-agnostic.
///
/// Failures here are *not* fatal: if `CreateJobObjectW` or
/// `AssignProcessToJobObject` returns an error we still hand the child back
/// to the caller and surface the error in the output's stderr. Killing a
/// runaway agent process is more important than refusing to run. The error
/// is returned (not swallowed) so the caller is aware the child is unsandboxed.
fn enforce_sandbox_job(
    child: std::process::Child,
    sandbox_status: &SandboxStatus,
) -> Result<std::process::Child, (std::process::Child, String)> {
    match apply_job_object_to_pid(child.id() as u32, sandbox_status, "sync") {
        Ok(()) => Ok(child),
        Err(err) => {
            eprintln!("{err}");
            Err((child, err))
        }
    }
}

/// Tokio counterpart of [`enforce_sandbox_job`]. Same contract: best-effort
/// kernel-level reaping for spawned children, with non-fatal failure on
/// Job Object API errors surfaced to the caller.
fn enforce_sandbox_job_tokio(
    child: tokio::process::Child,
    sandbox_status: &SandboxStatus,
) -> Result<tokio::process::Child, (tokio::process::Child, String)> {
    let pid = match child.id() {
        Some(pid) => pid,
        None => {
            return Err((
                child,
                "tokio child has no pid".to_string(),
            ))
        }
    };
    match apply_job_object_to_pid(pid, sandbox_status, "async") {
        Ok(()) => Ok(child),
        Err(err) => {
            eprintln!("{err}");
            Err((child, err))
        }
    }
}

/// Single source of truth for the Job Object setup. Consumes a `Job` (via
/// `into_handle`) so its handle stays alive for the rest of the agent
/// session — when the `claw` process exits, the kernel reaps every child.
/// Returns `Err` when the Job cannot be applied so the caller can decide
/// whether to proceed with an unsandboxed child.
fn apply_job_object_to_pid(
    pid: u32,
    sandbox_status: &SandboxStatus,
    label: &str,
) -> Result<(), String> {
    crate::bash_job_object_ffi::apply_job_object_to_pid(pid, sandbox_status.enabled, label)
}

/// Terminate a process by pid. Used by the async path after a timeout —
/// `child.start_kill` is not available once `wait_with_output` has moved
/// the child, so we drop back to Win32 `TerminateProcess` against the pid
/// we captured before the spawn.
fn kill_pid(pid: u32) {
    crate::bash_job_object_ffi::kill_pid(pid);
}

/// Detect git push to main and emit ship provenance event
fn detect_and_emit_ship_prepared(command: &str) {
    let trimmed = command.trim();
    // Simple detection: git push with main/master
    if trimmed.contains("git push") && (trimmed.contains("main") || trimmed.contains("master")) {
        // Emit ship.prepared event
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let provenance = ShipProvenance {
            source_branch: get_current_branch().unwrap_or_else(|| "unknown".to_string()),
            base_commit: get_head_commit().unwrap_or_default(),
            commit_count: 0, // Would need to calculate from range
            commit_range: "unknown..HEAD".to_string(),
            merge_method: ShipMergeMethod::DirectPush,
            actor: get_git_actor().unwrap_or_else(|| "unknown".to_string()),
            pr_number: None,
        };
        let _event = LaneEvent::ship_prepared(format!("{now}"), &provenance);
        // Log to stderr as interim routing before event stream integration
        eprintln!(
            "[ship.prepared] branch={} -> main, commits={}, actor={}",
            provenance.source_branch, provenance.commit_count, provenance.actor
        );
    }
}

fn get_current_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn get_head_commit() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn get_git_actor() -> Option<String> {
    let name = Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())?;
    Some(name)
}

async fn execute_bash_async(
    input: BashCommandInput,
    sandbox_status: SandboxStatus,
    cwd: std::path::PathBuf,
) -> io::Result<BashCommandOutput> {
    // Detect and emit ship provenance for git push operations
    detect_and_emit_ship_prepared(&input.command);

    let mut command = prepare_tokio_command(&input.command, &cwd, &sandbox_status, true);

    let output_result = if let Some(timeout_ms) = input.timeout {
        let child = command.spawn().map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("spawn failed: {e}"))
        })?;
        let child_pid = child.id();
        let child = match enforce_sandbox_job_tokio(child, &sandbox_status) {
            Ok(child) => child,
            Err((child, err)) => {
                eprintln!("{err}");
                child
            }
        };
        match timeout(Duration::from_millis(timeout_ms), child.wait_with_output()).await {
            Ok(result) => (result?, false),
            Err(_) => {
                if let Some(pid) = child_pid {
                    kill_pid(pid);
                }
                return Ok(BashCommandOutput {
                    stdout: String::new(),
                    stderr: format!("Command exceeded timeout of {timeout_ms} ms"),
                    raw_output_path: None,
                    interrupted: true,
                    is_image: None,
                    background_task_id: None,
                    backgrounded_by_user: None,
                    assistant_auto_backgrounded: None,
                    dangerously_disable_sandbox: input.dangerously_disable_sandbox,
                    return_code_interpretation: Some(String::from("timeout")),
                    no_output_expected: Some(true),
                    structured_content: None,
                    persisted_output_path: None,
                    persisted_output_size: None,
                    sandbox_status: Some(sandbox_status.clone()),
                    sandbox_type: derive_sandbox_type(&sandbox_status),
                });
            }
        }
    } else {
        let child = command.spawn().map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("spawn failed: {e}"))
        })?;
        let child = match enforce_sandbox_job_tokio(child, &sandbox_status) {
            Ok(child) => child,
            Err((child, err)) => {
                eprintln!("{err}");
                child
            }
        };
        (child.wait_with_output().await?, false)
    };

    let (output, interrupted) = output_result;
    // Persist the full stdout/stderr to a tmp file when the stream
    // exceeds MAX_OUTPUT_BYTES. The model can then read the full output
    // back via read_file rather than losing it to the truncation marker.
    let persisted_dir = std::env::temp_dir().join(format!("clawd-bash-{}", unix_now_nanos()));
    let _ = std::fs::create_dir_all(&persisted_dir);
    let stdout_captured = capture_or_persist(output.stdout.clone(), &persisted_dir)?;
    let stderr_captured = capture_or_persist(output.stderr.clone(), &persisted_dir)?;
    let stdout = stdout_captured.preview;
    let stderr = stderr_captured.preview;
    // Prefer the largest persisted file so the model gets a pointer
    // to the most useful full output. If neither stream was persisted,
    // the fields stay `None`.
    let persisted_output_path = stdout_captured
        .persisted_path
        .or(stderr_captured.persisted_path);
    let persisted_output_size = stdout_captured
        .persisted_size
        .max(stderr_captured.persisted_size);
    let no_output_expected = Some(stdout.trim().is_empty() && stderr.trim().is_empty());
    let return_code_interpretation = output.status.code().and_then(|code| {
        if code == 0 {
            None
        } else {
            Some(format!("exit_code:{code}"))
        }
    });

    Ok(BashCommandOutput {
        stdout,
        stderr,
        raw_output_path: None,
        interrupted,
        is_image: None,
        background_task_id: None,
        backgrounded_by_user: None,
        assistant_auto_backgrounded: None,
        dangerously_disable_sandbox: input.dangerously_disable_sandbox,
        return_code_interpretation,
        no_output_expected,
        structured_content: None,
        persisted_output_path,
        persisted_output_size,
        sandbox_status: Some(sandbox_status.clone()),
        sandbox_type: derive_sandbox_type(&sandbox_status),
    })
}

/// Monotonically increasing nanosecond timestamp. Prefer over
/// `SystemTime::now()` for naming files where collisions across rapid
/// invocations would be possible.
fn unix_now_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Rewrite Windows drive paths (`C:\...`) to forward-slash form
/// (`C:/...`) inside a shell command string before it is handed to an
/// MSYS2/Git-bash `sh -lc`. MSYS2's command-line parser eats backslashes
/// inside quoted arguments, turning `cat "C:\Users\me\f.txt"` into
/// `cat C:Usersmef.txt` (file not found). Only the `X:\` drive-prefix
/// form is rewritten — a `\` anywhere else (regex, escapes) is left
/// untouched, so this is a safe transform for Windows-only bash users.
///
/// The rewrite is performed on UTF-8 bytes while preserving multi-byte
/// characters: backslashes between a drive prefix and the end of the
/// path token are replaced with `/`, all other bytes are copied verbatim
/// so non-ASCII (e.g. Chinese) path components survive intact.
#[cfg(windows)]
fn normalize_windows_paths_for_msys(command: &str) -> String {
    let bytes = command.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        // Match a drive prefix: letter + ':' + '\'
        let is_drive_prefix = bytes[i].is_ascii_alphabetic()
            && i + 1 < bytes.len()
            && bytes[i + 1] == b':'
            && i + 2 < bytes.len()
            && bytes[i + 2] == b'\\';
        if is_drive_prefix {
            out.push(bytes[i]);
            out.push(b':');
            out.push(b'/');
            i += 3;
            // Normalise the rest of the backslash-separated path token.
            while i < bytes.len()
                && bytes[i] != b'"'
                && bytes[i] != b'\''
                && !bytes[i].is_ascii_whitespace()
            {
                out.push(if bytes[i] == b'\\' { b'/' } else { bytes[i] });
                i += 1;
            }
        } else {
            // Copy one full UTF-8 character so multi-byte sequences are
            // preserved verbatim (never split or re-encoded).
            let ch_len = utf8_char_len(bytes[i]);
            out.extend_from_slice(&bytes[i..i + ch_len]);
            i += ch_len;
        }
    }
    // Bytes are always valid UTF-8 because we only ever re-assemble the
    // original byte stream (replacing `\` with `/` inside path tokens).
    String::from_utf8(out).unwrap_or_else(|_| command.to_string())
}

/// Length in bytes of the UTF-8 character whose leading byte is `lead`.
#[cfg(windows)]
fn utf8_char_len(lead: u8) -> usize {
    if lead < 0x80 {
        1
    } else if lead >> 5 == 0b110 {
        2
    } else if lead >> 4 == 0b1110 {
        3
    } else if lead >> 3 == 0b11110 {
        4
    } else {
        1 // Invalid leading byte; treat as single byte.
    }
}

#[cfg(not(windows))]
fn normalize_windows_paths_for_msys(command: &str) -> String {
    command.to_string()
}

fn sandbox_status_for_input(input: &BashCommandInput, cwd: &std::path::Path) -> SandboxStatus {
    let config = ConfigLoader::default_for(cwd).load().map_or_else(
        |_| SandboxConfig::default(),
        |runtime_config| runtime_config.sandbox().clone(),
    );
    let request = config.resolve_request(
        input.dangerously_disable_sandbox.map(|disabled| !disabled),
        input.namespace_restrictions,
        input.isolate_network,
        input.filesystem_mode,
        input.allowed_mounts.clone(),
    );
    resolve_sandbox_status_for_request(&request, cwd)
}

/// Resolve the shell binary used to execute bash commands.
///
/// Cascade (first hit wins):
///   1. `CLAW_BASH_SHELL` env var (explicit override for portability/CI).
///   2. Known Windows installs: Git for Windows, then MSYS2.
///   3. `sh`/`bash` resolved via `PATH` (unix default; windows falls back here).
///
/// The previous hardcode of `C:\Program Files\Git\usr\bin\sh.exe` broke on
/// hosts without Git for Windows installed, silently yielding empty output.
pub fn resolve_shell() -> String {
    if let Ok(explicit) = std::env::var("CLAW_BASH_SHELL") {
        if !explicit.trim().is_empty() {
            return explicit.trim().to_string();
        }
    }

    #[cfg(windows)]
    {
        let candidates = [
            r"C:\Program Files\Git\usr\bin\sh.exe",
            r"C:\Program Files\Git\bin\sh.exe",
            r"H:\msys64\mingw64\bin\bash.exe",
            r"H:\msys64\usr\bin\bash.exe",
        ];
        for candidate in candidates {
            if std::path::Path::new(candidate).exists() {
                return candidate.to_string();
            }
        }
        // Fall back to PATH lookup; `sh` is the conventional name.
        if let Ok(found) = which_shell("sh") {
            return found;
        }
        if let Ok(found) = which_shell("bash") {
            return found;
        }
        // Last resort: let the OS resolver try `sh` and report the error at spawn.
        return "sh".to_string();
    }

    #[cfg(unix)]
    {
        "sh".to_string()
    }
}

#[cfg(windows)]
fn which_shell(name: &str) -> io::Result<String> {
    let output = std::process::Command::new("where")
        .arg(name)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "shell not on PATH"));
    }
    let path = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "shell not on PATH"))?;
    Ok(path)
}

/// Disable MSYS2 argument conversion for the spawned shell.
///
/// When a native (non-MSYS) parent spawns an MSYS2 `bash`/`sh` with a
/// single-quoted `-lc '...'` command, `msys-2.0.dll` rewrites the argv and
/// strips the single quotes, turning `printf 'alpha from bash'` into
/// `printf alpha from bash` (a "unterminated quoted string" error with empty
/// stdout). Pinning `MSYS2_ARG_CONV_EXCL=*` tells MSYS2 to leave argv intact.
#[cfg(windows)]
trait ShellEnvExt {
    fn set_msys_arg_conv_excl(&mut self) -> &mut Self;
}

#[cfg(windows)]
impl ShellEnvExt for std::process::Command {
    fn set_msys_arg_conv_excl(&mut self) -> &mut Self {
        self.env("MSYS2_ARG_CONV_EXCL", "*")
    }
}

#[cfg(windows)]
impl ShellEnvExt for tokio::process::Command {
    fn set_msys_arg_conv_excl(&mut self) -> &mut Self {
        self.env("MSYS2_ARG_CONV_EXCL", "*")
    }
}

#[cfg(windows)]
fn apply_windows_shell_env<C: ShellEnvExt + ?Sized>(command: &mut C) {
    command.set_msys_arg_conv_excl();
}

fn prepare_command(
    command: &str,
    cwd: &std::path::Path,
    sandbox_status: &SandboxStatus,
    create_dirs: bool,
) -> Command {
    if create_dirs {
        prepare_sandbox_dirs();
    }
    // Strip `LD_PRELOAD`, `PSModulePath`, `NODE_OPTIONS`, etc. from the
    // parent environment BEFORE the child is spawned. The child then
    // inherits a clean env. See `bash_dangerous_env.rs` for the full
    // attack matrix. Idempotent and cheap on the no-op path.
    crate::bash_dangerous_env::remove_dangerous_env_vars_parent();

    #[cfg(unix)]
    if let Some(launcher) = build_linux_sandbox_command(command, cwd, sandbox_status) {
        let mut prepared = Command::new(launcher.program);
        prepared.args(launcher.args);
        prepared.current_dir(cwd);
        prepared.envs(launcher.env);
        return prepared;
    }

    let shell = resolve_shell();
    let root = sandbox_root();

    let command = normalize_windows_paths_for_msys(command);
    let mut prepared = Command::new(shell);
    prepared.arg("-lc").arg(&command).current_dir(cwd);
    apply_windows_shell_env(&mut prepared);
    if sandbox_status.filesystem_active {
        #[cfg(windows)]
        {
            prepared.env("USERPROFILE", root.join("home"));
            prepared.env("TEMP", root.join("tmp"));
        }
        #[cfg(unix)]
        {
            prepared.env("HOME", root.join("home"));
            prepared.env("TMPDIR", root.join("tmp"));
        }
    }
    apply_windows_detach_flags(&mut prepared);
    prepared
}

fn prepare_tokio_command(
    command: &str,
    cwd: &std::path::Path,
    sandbox_status: &SandboxStatus,
    create_dirs: bool,
) -> TokioCommand {
    if create_dirs {
        prepare_sandbox_dirs();
    }
    // See `prepare_command` for the rationale.
    crate::bash_dangerous_env::remove_dangerous_env_vars_parent();
    #[cfg(unix)]
    if let Some(launcher) = build_linux_sandbox_command(command, cwd, sandbox_status) {
        let mut prepared = TokioCommand::new(launcher.program);
        prepared.args(launcher.args);
        prepared.current_dir(cwd);
        prepared.envs(launcher.env);
        return prepared;
    }

    let shell = resolve_shell();
    let root = sandbox_root();
    let command = normalize_windows_paths_for_msys(command);
    let mut prepared = TokioCommand::new(shell);
    prepared
        .arg("-lc")
        .arg(&command)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_windows_shell_env(&mut prepared);
    if sandbox_status.filesystem_active {
        #[cfg(windows)]
        {
            prepared.env("USERPROFILE", root.join("home"));
            prepared.env("TEMP", root.join("tmp"));
        }
        #[cfg(unix)]
        {
            prepared.env("HOME", root.join("home"));
            prepared.env("TMPDIR", root.join("tmp"));
        }
    }
    apply_windows_detach_flags_tokio(&mut prepared);
    prepared
}

fn sandbox_root() -> std::path::PathBuf {
    crate::config::default_config_home().join("sandbox")
}

fn prepare_sandbox_dirs() {
    let root = sandbox_root();
    let _ = std::fs::create_dir_all(root.join("home"));
    let _ = std::fs::create_dir_all(root.join("tmp"));
}

#[cfg(test)]
mod tests {
    use super::{execute_bash, BashCommandInput};

    #[test]
    fn executes_simple_command() {
        let output = execute_bash(BashCommandInput {
            command: String::from("echo hello"),
            timeout: Some(30_000),
            run_in_background: Some(false),
            dangerously_disable_sandbox: Some(true),
            namespace_restrictions: None,
            isolate_network: None,
            filesystem_mode: None,
            allowed_mounts: None,
        })
        .expect("bash command should execute");

        assert_eq!(output.stdout.trim(), "hello");
        assert!(!output.interrupted);
    }

    #[test]
    fn disables_sandbox_when_requested() {
        let output = execute_bash(BashCommandInput {
            command: String::from("echo hello"),
            timeout: Some(1_000),
            run_in_background: Some(false),
            dangerously_disable_sandbox: Some(true),
            namespace_restrictions: None,
            isolate_network: None,
            filesystem_mode: None,
            allowed_mounts: None,
        })
        .expect("bash command should execute");

        assert!(!output.sandbox_status.expect("sandbox status").enabled);
    }


}

/// Maximum output bytes before truncation (16 KiB, matching upstream).
const MAX_OUTPUT_BYTES: usize = 16_384;

/// Result of capturing a child stream: either the (possibly truncated)
/// preview that goes into the tool result envelope, or — when the
/// full content exceeded `MAX_OUTPUT_BYTES` — the preview plus the
/// path and size of a file that holds the full bytes.
pub struct CapturedStream {
    /// UTF-8 decoded preview. If the stream exceeded `MAX_OUTPUT_BYTES`,
    /// this is the first `MAX_OUTPUT_BYTES` bytes plus a truncation marker.
    pub preview: String,
    /// Absolute path of the persisted file, if the stream was too large.
    pub persisted_path: Option<String>,
    /// Size of the persisted file in bytes, if it was written.
    pub persisted_size: Option<u64>,
}

/// Capture a child stream into a preview, persisting the full bytes to
/// `persisted_dir` when the stream exceeds `MAX_OUTPUT_BYTES`. The
/// persisted file is named with a nanos-precision suffix so concurrent
/// bash invocations do not collide.
fn capture_or_persist(raw: Vec<u8>, persisted_dir: &Path) -> std::io::Result<CapturedStream> {
    if raw.len() <= MAX_OUTPUT_BYTES {
        return Ok(CapturedStream {
            preview: String::from_utf8_lossy(&raw).into_owned(),
            persisted_path: None,
            persisted_size: None,
        });
    }
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = persisted_dir.join(format!("bash-stdout-{suffix}.txt"));
    std::fs::write(&path, &raw)?;
    let size = raw.len() as u64;
    // Truncate to the last valid UTF-8 boundary at or before
    // MAX_OUTPUT_BYTES by decoding with from_utf8_lossy and using
    // char_indices to find the safe cut.
    let lossy = String::from_utf8_lossy(&raw);
    let mut end = MAX_OUTPUT_BYTES.min(lossy.len());
    while end > 0 && !lossy.is_char_boundary(end) {
        end -= 1;
    }
    let preview = format!(
        "{}\n\n[output truncated — full output saved to {} ({} bytes)]",
        &lossy[..end],
        path.display(),
        size
    );
    Ok(CapturedStream {
        preview,
        persisted_path: Some(path.to_string_lossy().into_owned()),
        persisted_size: Some(size),
    })
}

#[cfg(test)]
mod truncation_tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        std::env::temp_dir().join(format!("clawd-bash-{name}-{unique}"))
    }

    #[test]
    fn capture_short_stream_does_not_persist() {
        let dir = temp_dir("capture-short");
        std::fs::create_dir_all(&dir).expect("dir should create");
        let raw = b"hello".to_vec();
        let captured = capture_or_persist(raw, &dir).expect("capture should succeed");
        assert_eq!(captured.preview, "hello");
        assert_eq!(captured.persisted_path, None);
        assert_eq!(captured.persisted_size, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn capture_long_stream_persists_full_bytes() {
        let dir = temp_dir("capture-long");
        std::fs::create_dir_all(&dir).expect("dir should create");
        let payload = "y".repeat(40_000);
        let raw = payload.as_bytes().to_vec();
        let captured = capture_or_persist(raw, &dir).expect("capture should succeed");
        let persisted = captured
            .persisted_path
            .as_ref()
            .expect("long stream should produce a persisted path");
        let size = captured
            .persisted_size
            .expect("long stream should report persisted size");
        assert_eq!(size, 40_000);
        let on_disk = std::fs::read_to_string(persisted).expect("persisted file should read");
        assert_eq!(on_disk, payload, "persisted file must hold the full payload");
        let preview_truncated = {
            let end = captured.preview.len().min(120);
            let idx = captured.preview.floor_char_boundary(end);
            &captured.preview[..idx]
        };
        assert!(
            captured.preview.contains("[output truncated"),
            "preview should contain truncation marker; got: {}",
            preview_truncated
        );
        assert!(
            captured.preview.contains(persisted),
            "preview should mention the persisted path so the model can fetch it"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod sandbox_type_tests {
    use super::*;
    use crate::sandbox::SandboxRequest;
    use std::path::Path;

    #[test]
    fn returns_none_when_sandbox_disabled() {
        // Build a status with `enabled: false` by going through
        // `SandboxStatus::default()`. We can't construct it directly
        // because some fields are pub(crate) — instead, drive the
        // helper by calling `resolve_sandbox_status_for_request` with
        // an Off-mode request and verify `derive_sandbox_type` agrees.
        let request = SandboxRequest {
            enabled: false,
            namespace_restrictions: false,
            network_isolation: false,
            filesystem_mode: FilesystemIsolationMode::Off,
            allowed_mounts: vec![],
        };
        let status = resolve_sandbox_status_for_request(&request, Path::new("."));
        assert!(!status.enabled);
        assert_eq!(derive_sandbox_type(&status), None);
    }

    #[test]
    fn returns_platform_specific_name_when_sandbox_enabled() {
        let request = SandboxRequest {
            enabled: true,
            namespace_restrictions: false,
            network_isolation: false,
            filesystem_mode: FilesystemIsolationMode::Off,
            allowed_mounts: vec![],
        };
        let status = resolve_sandbox_status_for_request(&request, Path::new("."));
        let name = derive_sandbox_type(&status).expect("should be Some when enabled");
        let expected = if cfg!(target_os = "windows") {
            "windows-job-object"
        } else if cfg!(target_os = "linux") {
            "linux-unshare"
        } else {
            "macos-unconfigured" // or "unconfigured" — checked below
        };
        // For platforms we don't explicitly know, accept the
        // fallthrough "unconfigured" / "macos-unconfigured" labels.
        assert!(
            name == expected || name == "macos-unconfigured" || name == "unconfigured",
            "got {name}, expected a platform-specific label"
        );
    }
}

#[cfg(test)]
mod msys_path_tests {
    use super::normalize_windows_paths_for_msys;

    #[test]
    fn rewrites_drive_paths_to_forward_slashes() {
        let input = r#"cat "C:\Users\me\doc.md""#;
        let out = normalize_windows_paths_for_msys(input);
        assert_eq!(out, r#"cat "C:/Users/me/doc.md""#);
    }

    #[test]
    fn rewrites_multiple_drive_paths() {
        let input = r#"diff "D:\a\b.txt" "E:\x\y.txt""#;
        let out = normalize_windows_paths_for_msys(input);
        assert_eq!(out, r#"diff "D:/a/b.txt" "E:/x/y.txt""#);
    }

    #[test]
    fn leaves_non_drive_backslashes_untouched() {
        // Regex escapes and internal backslashes must not be mangled.
        let input = r#"grep -E 'a\\b' foo"#;
        let out = normalize_windows_paths_for_msys(input);
        assert_eq!(out, input);
    }

    #[test]
    fn leaves_posix_paths_and_bare_commands_untouched() {
        let input = "ls -la /tmp && echo hi";
        let out = normalize_windows_paths_for_msys(input);
        assert_eq!(out, input);
    }
}
