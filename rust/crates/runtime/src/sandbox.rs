use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum FilesystemIsolationMode {
    Off,
    #[default]
    WorkspaceOnly,
    AllowList,
}

impl FilesystemIsolationMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::WorkspaceOnly => "workspace-only",
            Self::AllowList => "allow-list",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SandboxConfig {
    pub enabled: Option<bool>,
    pub namespace_restrictions: Option<bool>,
    pub network_isolation: Option<bool>,
    pub filesystem_mode: Option<FilesystemIsolationMode>,
    pub allowed_mounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SandboxRequest {
    pub enabled: bool,
    pub namespace_restrictions: bool,
    pub network_isolation: bool,
    pub filesystem_mode: FilesystemIsolationMode,
    pub allowed_mounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ContainerEnvironment {
    pub in_container: bool,
    pub markers: Vec<String>,
}

/// Linux-shaped container detection inputs (filesystem markers + cgroup).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SandboxDetectionInputs<'a> {
    pub env_pairs: Vec<(String, String)>,
    pub dockerenv_exists: bool,
    pub containerenv_exists: bool,
    pub proc_1_cgroup: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxSandboxCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// Windows-shaped sandbox command. AppContainer spawn is not yet wired into
/// the tool execution pipeline, so this is currently descriptive only — the
/// same shape as `LinuxSandboxCommand` but tagged with the AppContainer
/// profile name we would pass to `CreateProcess` if/when enforcement lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsSandboxCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub app_container_profile: String,
    pub capabilities: Vec<String>,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SandboxStatus {
    pub enabled: bool,
    pub requested: SandboxRequest,
    pub supported: bool,
    pub active: bool,
    pub namespace_supported: bool,
    pub namespace_active: bool,
    pub network_supported: bool,
    pub network_active: bool,
    pub filesystem_mode: FilesystemIsolationMode,
    pub filesystem_active: bool,
    pub allowed_mounts: Vec<String>,
    pub in_container: bool,
    pub container_markers: Vec<String>,
    pub fallback_reason: Option<String>,
}

impl SandboxConfig {
    #[must_use]
    pub fn resolve_request(
        &self,
        enabled_override: Option<bool>,
        namespace_override: Option<bool>,
        network_override: Option<bool>,
        filesystem_mode_override: Option<FilesystemIsolationMode>,
        allowed_mounts_override: Option<Vec<String>>,
    ) -> SandboxRequest {
        SandboxRequest {
            enabled: enabled_override.unwrap_or(self.enabled.unwrap_or(true)),
            namespace_restrictions: namespace_override
                .unwrap_or(self.namespace_restrictions.unwrap_or(true)),
            network_isolation: network_override.unwrap_or(self.network_isolation.unwrap_or(false)),
            filesystem_mode: filesystem_mode_override
                .or(self.filesystem_mode)
                .unwrap_or_default(),
            allowed_mounts: allowed_mounts_override.unwrap_or_else(|| self.allowed_mounts.clone()),
        }
    }
}

/// Cross-platform container detection. Dispatches to the platform-shaped
/// parser so the same `ContainerEnvironment` value flows back to the rest of
/// the runtime regardless of host OS.
#[must_use]
pub fn detect_container_environment() -> ContainerEnvironment {
    if cfg!(target_os = "windows") {
        let env_pairs: Vec<(String, String)> = env::vars().collect();
        let identity_exists = Path::new(r"C:\identity.txt").exists();
        let dev_marker = env::var_os("CLAWD_IN_DEV_CONTAINER").is_some_and(|v| !v.is_empty());
        return detect_container_environment_windows_from(dev_marker, identity_exists, &env_pairs);
    }
    let proc_1_cgroup = fs::read_to_string("/proc/1/cgroup").ok();
    let dockerenv_exists = cfg!(target_os = "linux") && Path::new("/.dockerenv").exists();
    let containerenv_exists = cfg!(target_os = "linux") && Path::new("/run/.containerenv").exists();
    detect_container_environment_linux_from(SandboxDetectionInputs {
        env_pairs: env::vars().collect(),
        dockerenv_exists,
        containerenv_exists,
        proc_1_cgroup: proc_1_cgroup.as_deref(),
    })
}

/// Linux-shaped container detection: dockerenv + containerenv files, cgroup
/// contents, and the standard env-var set (`container`, `docker`, `podman`,
/// `KUBERNETES_SERVICE_HOST`).
#[must_use]
pub fn detect_container_environment_linux_from(
    inputs: SandboxDetectionInputs<'_>,
) -> ContainerEnvironment {
    let mut markers = Vec::new();
    if inputs.dockerenv_exists {
        markers.push("/.dockerenv".to_string());
    }
    if inputs.containerenv_exists {
        markers.push("/run/.containerenv".to_string());
    }
    for (key, value) in inputs.env_pairs {
        let normalized = key.to_ascii_lowercase();
        if matches!(
            normalized.as_str(),
            "container" | "docker" | "podman" | "kubernetes_service_host"
        ) && !value.is_empty()
        {
            markers.push(format!("env:{key}={value}"));
        }
    }
    if let Some(cgroup) = inputs.proc_1_cgroup {
        for needle in ["docker", "containerd", "kubepods", "podman", "libpod"] {
            if cgroup.contains(needle) {
                markers.push(format!("/proc/1/cgroup:{needle}"));
            }
        }
    }
    markers.sort();
    markers.dedup();
    ContainerEnvironment {
        in_container: !markers.is_empty(),
        markers,
    }
}

/// Backwards-compatible alias. Existing callers and the pre-split test
/// fixtures import `detect_container_environment_from`; keep that name
/// pointing at the Linux implementation so no consumer has to change.
#[must_use]
pub fn detect_container_environment_from(
    inputs: SandboxDetectionInputs<'_>,
) -> ContainerEnvironment {
    detect_container_environment_linux_from(inputs)
}

/// Windows-shaped container detection. Walks the env-var set first, then
/// checks for the Hyper-V `C:\identity.txt` marker that process-isolated
/// Windows containers leave behind. The `clawd_in_dev_container` flag is
/// derived by the caller from `CLAWD_IN_DEV_CONTAINER` so this function
/// stays pure for unit tests.
#[must_use]
pub fn detect_container_environment_windows_from(
    clawd_in_dev_container: bool,
    identity_txt_exists: bool,
    env_pairs: &[(String, String)],
) -> ContainerEnvironment {
    let mut markers = Vec::new();
    if identity_txt_exists {
        markers.push(r"C:\identity.txt".to_string());
    }
    if clawd_in_dev_container {
        markers.push("env:CLAWD_IN_DEV_CONTAINER".to_string());
    }
    for (key, value) in env_pairs {
        if value.is_empty() {
            continue;
        }
        let normalized = key.to_ascii_uppercase();
        match normalized.as_str() {
            "CONTAINER_SAS_URL" | "CONTAINER_NAME" | "CONTAINER_ID" | "KUBERNETES_SERVICE_HOST" => {
                markers.push(format!("env:{key}={value}"));
            }
            _ => {}
        }
    }
    markers.sort();
    markers.dedup();
    ContainerEnvironment {
        in_container: !markers.is_empty(),
        markers,
    }
}

#[must_use]
pub fn resolve_sandbox_status(config: &SandboxConfig, cwd: &Path) -> SandboxStatus {
    let request = config.resolve_request(None, None, None, None, None);
    resolve_sandbox_status_for_request(&request, cwd)
}

#[must_use]
pub fn resolve_sandbox_status_for_request(request: &SandboxRequest, cwd: &Path) -> SandboxStatus {
    let container = detect_container_environment();
    let isolation_supported = platform_isolation_supported();
    let filesystem_active =
        request.enabled && request.filesystem_mode != FilesystemIsolationMode::Off;
    let mut fallback_reasons = Vec::new();

    if request.enabled && request.namespace_restrictions && !isolation_supported {
        fallback_reasons.push(namespace_fallback_message());
    }
    if request.enabled && request.network_isolation && !isolation_supported {
        fallback_reasons.push(network_fallback_message());
    }
    if request.enabled
        && request.filesystem_mode == FilesystemIsolationMode::AllowList
        && request.allowed_mounts.is_empty()
    {
        fallback_reasons
            .push("filesystem allow-list requested without configured mounts".to_string());
    }
    // On Windows, the current status snapshot reports filesystem modes
    // that the runtime cannot yet *enforce* via AppContainer at execution
    // time. Job Object kill-on-close is wired in `bash.rs`, but the
    // workspace-only / allow-list filesystem mode still lands when
    // AppContainer spawn is wired into the tool pipeline. Surface the gap
    // honestly so users don't think `filesystem_mode: workspace-only` is
    // actively confining child processes on Windows.
    if cfg!(target_os = "windows")
        && request.enabled
        && request.filesystem_mode == FilesystemIsolationMode::WorkspaceOnly
    {
        fallback_reasons.push(
            "filesystem_mode: workspace-only is reported but AppContainer enforcement \
             is not yet wired into tool execution on Windows (process tree kill via \
             Job Object is active)"
                .to_string(),
        );
    }

    let active = request.enabled
        && (!request.namespace_restrictions || isolation_supported)
        && (!request.network_isolation || isolation_supported);

    let allowed_mounts = normalize_mounts(&request.allowed_mounts, cwd);

    SandboxStatus {
        enabled: request.enabled,
        requested: request.clone(),
        supported: isolation_supported,
        active,
        namespace_supported: isolation_supported,
        namespace_active: request.enabled && request.namespace_restrictions && isolation_supported,
        network_supported: isolation_supported,
        network_active: request.enabled && request.network_isolation && isolation_supported,
        filesystem_mode: request.filesystem_mode,
        filesystem_active,
        allowed_mounts,
        in_container: container.in_container,
        container_markers: container.markers,
        fallback_reason: (!fallback_reasons.is_empty()).then(|| fallback_reasons.join("; ")),
    }
}

/// Returns true when the current host can enforce the sandbox request's
/// namespace / network isolation. On Linux this means `unshare(1)` actually
/// works; on Windows it means we're on Win10+ where AppContainer is
/// available; on other targets the answer is always `false`.
#[must_use]
pub fn platform_isolation_supported() -> bool {
    #[cfg(target_os = "linux")]
    {
        unshare_user_namespace_works()
    }
    #[cfg(target_os = "windows")]
    {
        appcontainer_is_supported()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        false
    }
}

#[cfg(target_os = "linux")]
fn namespace_fallback_message() -> String {
    "namespace isolation unavailable (requires Linux with `unshare`)".to_string()
}

#[cfg(target_os = "windows")]
fn namespace_fallback_message() -> String {
    "namespace isolation unavailable (requires Windows 10+ with AppContainer; \
     enforcement into tool execution is not yet wired)"
        .to_string()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn namespace_fallback_message() -> String {
    "namespace isolation unavailable on this platform".to_string()
}

#[cfg(target_os = "linux")]
fn network_fallback_message() -> String {
    "network isolation unavailable (requires Linux with `unshare`)".to_string()
}

#[cfg(target_os = "windows")]
fn network_fallback_message() -> String {
    "network isolation unavailable (requires AppContainer profile with no \
     INTERNET_CLIENT/INTERNET_SERVER capability; enforcement is not yet wired)"
        .to_string()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn network_fallback_message() -> String {
    "network isolation unavailable on this platform".to_string()
}

#[must_use]
pub fn build_linux_sandbox_command(
    command: &str,
    cwd: &Path,
    status: &SandboxStatus,
) -> Option<LinuxSandboxCommand> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (command, cwd, status);
        return None;
    }
    #[cfg(target_os = "linux")]
    {
        if !status.enabled || (!status.namespace_active && !status.network_active) {
            return None;
        }

        let mut args = vec![
            "--user".to_string(),
            "--map-root-user".to_string(),
            "--mount".to_string(),
            "--ipc".to_string(),
            "--pid".to_string(),
            "--uts".to_string(),
            "--fork".to_string(),
        ];
        if status.network_active {
            args.push("--net".to_string());
        }
        args.push("sh".to_string());
        args.push("-lc".to_string());
        args.push(command.to_string());

        let sandbox_home = cwd.join(".sandbox-home");
        let sandbox_tmp = cwd.join(".sandbox-tmp");
        let mut env = vec![
            ("HOME".to_string(), sandbox_home.display().to_string()),
            ("TMPDIR".to_string(), sandbox_tmp.display().to_string()),
            (
                "CLAWD_SANDBOX_FILESYSTEM_MODE".to_string(),
                status.filesystem_mode.as_str().to_string(),
            ),
            (
                "CLAWD_SANDBOX_ALLOWED_MOUNTS".to_string(),
                status.allowed_mounts.join(":"),
            ),
        ];
        if let Ok(path) = env::var("PATH") {
            env.push(("PATH".to_string(), path));
        }

        Some(LinuxSandboxCommand {
            program: "unshare".to_string(),
            args,
            env,
        })
    }
}

/// Windows equivalent of `build_linux_sandbox_command`. Currently a
/// descriptive builder: it never spawns anything itself, but the returned
/// `WindowsSandboxCommand` documents what AppContainer + CreateProcess call
/// we would make once the tool-execution pipeline is wired to consume it.
#[must_use]
pub fn build_windows_sandbox_command(
    command: &str,
    cwd: &Path,
    status: &SandboxStatus,
) -> Option<WindowsSandboxCommand> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (command, cwd, status);
        return None;
    }
    #[cfg(target_os = "windows")]
    {
        if !status.enabled {
            return None;
        }
        let profile = format!("clawcode-{}", profile_suffix(cwd));
        let mut capabilities = Vec::new();
        if status.network_active {
            // In the eventual enforcement pass we'd *omit* the INTERNET_CLIENT
            // capability to actually isolate. The descriptive builder lists
            // the capability the non-sandboxed parent would carry so the
            // diff between sandboxed vs non-sandboxed is visible at a glance.
            capabilities.push("INTERNET_CLIENT".to_string());
        }
        let args = vec!["cmd".to_string(), "/C".to_string(), command.to_string()];
        let mut env = vec![(
            "CLAWD_SANDBOX_FILESYSTEM_MODE".to_string(),
            status.filesystem_mode.as_str().to_string(),
        )];
        if let Ok(path) = env::var("PATH") {
            env.push(("PATH".to_string(), path));
        }
        Some(WindowsSandboxCommand {
            program: "CreateProcessW".to_string(),
            args,
            env,
            app_container_profile: profile,
            capabilities,
        })
    }
}

#[cfg(target_os = "windows")]
fn profile_suffix(cwd: &Path) -> String {
    // Use a short, filesystem-safe suffix of the cwd so AppContainer profile
    // names (which have a 64-char limit) stay within bounds.
    let s = cwd.display().to_string();
    let trimmed = s.replace(['\\', '/', ':', ' '], "_");
    if trimmed.len() > 32 {
        trimmed[trimmed.len() - 32..].to_string()
    } else {
        trimmed
    }
}

fn normalize_mounts(mounts: &[String], cwd: &Path) -> Vec<String> {
    let cwd = cwd.to_path_buf();
    mounts
        .iter()
        .map(|mount| {
            let path = PathBuf::from(mount);
            if path.is_absolute() {
                path
            } else {
                cwd.join(path)
            }
        })
        .map(|path| path.display().to_string())
        .collect()
}

#[cfg(target_os = "linux")]
fn command_exists(command: &str) -> bool {
    env::var_os("PATH")
        .is_some_and(|paths| env::split_paths(&paths).any(|path| path.join(command).exists()))
}

/// Check whether `unshare --user` actually works on this system.
/// On some CI environments (e.g. GitHub Actions), the binary exists but
/// user namespaces are restricted, causing silent failures.
#[cfg(target_os = "linux")]
fn unshare_user_namespace_works() -> bool {
    use std::sync::OnceLock;
    static RESULT: OnceLock<bool> = OnceLock::new();
    *RESULT.get_or_init(|| {
        if !command_exists("unshare") {
            return false;
        }
        std::process::Command::new("unshare")
            .args(["--user", "--map-root-user", "true"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// AppContainer ships in every Windows build that has Rust toolchain
/// support (Windows 10 1709+ / Windows Server 2019+). Rather than pin a
/// specific OS build, we treat the question as "is the host capable" and
/// allow the operator to opt out via `CLAWD_FORCE_APPCONTAINER=0`.
///
/// A real `windows-sys` based probe of `CreateAppContainerProfile` in
/// `userenv.dll` is the right next step when tool execution is wired to
/// consume the `WindowsSandboxCommand`; the snapshot-only contract that
/// `/sandbox` exposes today does not need the dynamic-link call.
#[cfg(target_os = "windows")]
fn appcontainer_is_supported() -> bool {
    use std::sync::OnceLock;
    static RESULT: OnceLock<bool> = OnceLock::new();
    *RESULT.get_or_init(|| {
        if env::var("CLAWD_FORCE_APPCONTAINER")
            .map(|v| v == "0")
            .unwrap_or(false)
        {
            return false;
        }
        true
    })
}

#[cfg(test)]
mod tests {
    use super::{
        detect_container_environment_windows_from, FilesystemIsolationMode, SandboxConfig,
    };

    #[cfg(target_os = "linux")]
    use super::{
        build_linux_sandbox_command, detect_container_environment_from, SandboxDetectionInputs,
    };
    use std::path::Path;

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_detection_picks_up_markers_from_multiple_sources() {
        let detected = detect_container_environment_from(SandboxDetectionInputs {
            env_pairs: vec![("container".to_string(), "docker".to_string())],
            dockerenv_exists: true,
            containerenv_exists: false,
            proc_1_cgroup: Some("12:memory:/docker/abc"),
        });

        assert!(detected.in_container);
        assert!(detected
            .markers
            .iter()
            .any(|marker| marker == "/.dockerenv"));
        assert!(detected
            .markers
            .iter()
            .any(|marker| marker == "env:container=docker"));
        assert!(detected
            .markers
            .iter()
            .any(|marker| marker == "/proc/1/cgroup:docker"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_detection_picks_up_kubernetes_and_identity() {
        let detected = detect_container_environment_windows_from(
            false,
            true,
            &[("KUBERNETES_SERVICE_HOST".to_string(), "10.0.0.1".to_string())],
        );
        assert!(detected.in_container);
        assert!(detected
            .markers
            .iter()
            .any(|marker| marker == r"C:\identity.txt"));
        assert!(detected
            .markers
            .iter()
            .any(|marker| marker == "env:KUBERNETES_SERVICE_HOST=10.0.0.1"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_detection_picks_up_clawd_dev_container_marker() {
        let detected = detect_container_environment_windows_from(
            true,
            false,
            &[("CONTAINER_NAME".to_string(), "claw-dev".to_string())],
        );
        assert!(detected.in_container);
        assert!(detected
            .markers
            .iter()
            .any(|marker| marker == "env:CLAWD_IN_DEV_CONTAINER"));
        assert!(detected
            .markers
            .iter()
            .any(|marker| marker == "env:CONTAINER_NAME=claw-dev"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_detection_ignores_empty_env_values() {
        let detected = detect_container_environment_windows_from(
            false,
            false,
            &[
                ("CONTAINER_SAS_URL".to_string(), String::new()),
                ("KUBERNETES_SERVICE_HOST".to_string(), String::new()),
            ],
        );
        assert!(!detected.in_container);
        assert!(detected.markers.is_empty());
    }

    #[test]
    fn resolves_request_with_overrides() {
        let config = SandboxConfig {
            enabled: Some(true),
            namespace_restrictions: Some(true),
            network_isolation: Some(false),
            filesystem_mode: Some(FilesystemIsolationMode::WorkspaceOnly),
            allowed_mounts: vec!["logs".to_string()],
        };

        let request = config.resolve_request(
            Some(true),
            Some(false),
            Some(true),
            Some(FilesystemIsolationMode::AllowList),
            Some(vec!["tmp".to_string()]),
        );

        assert!(request.enabled);
        assert!(!request.namespace_restrictions);
        assert!(request.network_isolation);
        assert_eq!(request.filesystem_mode, FilesystemIsolationMode::AllowList);
        assert_eq!(request.allowed_mounts, vec!["tmp"]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn builds_linux_launcher_with_network_flag_when_requested() {
        let config = SandboxConfig::default();
        let status = super::resolve_sandbox_status_for_request(
            &config.resolve_request(
                Some(true),
                Some(true),
                Some(true),
                Some(FilesystemIsolationMode::WorkspaceOnly),
                None,
            ),
            Path::new("/workspace"),
        );

        if let Some(launcher) =
            build_linux_sandbox_command("printf hi", Path::new("/workspace"), &status)
        {
            assert_eq!(launcher.program, "unshare");
            assert!(launcher.args.iter().any(|arg| arg == "--mount"));
            assert!(launcher.args.iter().any(|arg| arg == "--net") == status.network_active);
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn builds_windows_descriptive_command_when_enabled() {
        use super::{build_windows_sandbox_command, resolve_sandbox_status_for_request};
        let config = SandboxConfig::default();
        let status = resolve_sandbox_status_for_request(
            &config.resolve_request(
                Some(true),
                Some(true),
                Some(false),
                Some(FilesystemIsolationMode::Off),
                None,
            ),
            Path::new(r"C:\workspace"),
        );

        if let Some(launcher) =
            build_windows_sandbox_command("echo hi", Path::new(r"C:\workspace"), &status)
        {
            assert_eq!(launcher.program, "CreateProcessW");
            assert!(launcher.args.iter().any(|arg| arg == "/C"));
            assert!(launcher.app_container_profile.starts_with("clawcode-"));
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn resolve_sandbox_status_reports_windows_fallback_wording() {
        use super::resolve_sandbox_status_for_request;
        let config = SandboxConfig::default();
        let status = resolve_sandbox_status_for_request(
            &config.resolve_request(
                Some(true),
                Some(true),
                Some(true),
                Some(FilesystemIsolationMode::WorkspaceOnly),
                None,
            ),
            Path::new(r"C:\workspace"),
        );

        if !status.supported {
            // On hosts where AppContainer is force-disabled the wording
            // should mention the platform, not Linux's unshare.
            let reason = status.fallback_reason.unwrap_or_default();
            assert!(
                reason.contains("Windows") || reason.contains("AppContainer"),
                "unexpected fallback_reason: {reason}"
            );
        }
    }
}
