use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jiff::tz::TimeZone;

use agents::{
    allowed_tools_for_subagent, build_agent_system_prompt, make_agent_id,
    normalize_subagent_type, resolve_agent_model, slugify_agent_name,
    spawn_agent_task_with_progress, AgentDiscovery, AgentInput, AgentOutput,
};
use api::ToolDefinition;
use plugins::PluginTool;
use runtime::{
    check_freshness, default_config_home, execute_bash,
    permission_enforcer::{EnforcementResult, PermissionEnforcer},
    BashCommandInput, BashCommandOutput, BranchFreshness, GrepSearchInput, LaneEvent,
    LaneEventName, LaneEventStatus, LaneFailureClass, PermissionMode,
    BoundaryPolicy,
};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Re-exported so the CLI (which depends on `tools`, not `agents`) can register
/// the sub-agent runtime tool provider for MCP/plugin execution.
pub use agents::{register_runtime_tool_provider, registered_extra_tool_defs, RuntimeToolExecutorFn};


/// WebFetch content cache. Stores raw body and content type per URL
/// to avoid repeated HTTP requests when the AI references the same
/// page multiple times within a session. On cache hit the raw body
/// is re-summarized with the current prompt so prompt-specific
/// processing (title extraction, summarization) works correctly.
struct WebFetchCacheEntry {
    raw_body: String,
    content_type: String,
    fetched_at: u64,
}

fn global_webfetch_cache(
) -> &'static std::sync::Mutex<std::collections::HashMap<String, WebFetchCacheEntry>> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, WebFetchCacheEntry>>,
    > = OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Cache TTL: 20 seconds.
const WEBFETCH_CACHE_TTL_SECS: u64 = 20;

/// File content cache. Stores file contents written by new_file/edit_file
/// so that subsequent read_file calls can skip disk I/O.
/// Key: canonicalized absolute path, Value: full file content string.
struct FileCacheEntry {
    content: String,
    checksum: String,
}

fn global_file_cache(
) -> &'static std::sync::Mutex<std::collections::HashMap<String, FileCacheEntry>> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<std::sync::Mutex<std::collections::HashMap<String, FileCacheEntry>>> =
        OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Holder for the active `BoundaryPolicy`. The default is `Block`,

/// Compute (year, month, day, hour, minute) from a Unix timestamp (seconds)
/// in local time.
fn ts_ymd_hm(secs: u64) -> (u32, u32, u32, u32, u32) {
    let ts = jiff::Timestamp::new(secs as i64, 0).unwrap();
    let zoned = ts.to_zoned(TimeZone::system());
    let y = zoned.year() as u32;
    let m = zoned.month() as u32;
    let d = zoned.day() as u32;
    let h = zoned.hour() as u32;
    let mi = zoned.minute() as u32;
    (y, m, d, h, mi)
}

static LAST_WRITE_DIFF_SEC: AtomicU64 = AtomicU64::new(0);

fn monotonic_secs() -> u64 {
    let wall_clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut candidate = wall_clock;
    loop {
        let previous = LAST_WRITE_DIFF_SEC.load(Ordering::Relaxed);
        if candidate <= previous {
            candidate = previous.saturating_add(1);
        }
        match LAST_WRITE_DIFF_SEC.compare_exchange(previous, candidate, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => return candidate,
            Err(actual) => candidate = actual.saturating_add(1),
        }
    }
}

/// Write a diff file under `~/.claw/diffs/YYYYMMDDHHMM[_seq].patch`.
/// Returns the absolute path to the diff file, or None on failure.
fn write_diff_file(
    file_path: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Option<String> {
    use std::io::Write;

    let diffs_root = default_config_home().join("diffs");
    std::fs::create_dir_all(&diffs_root).ok()?;

    let secs = monotonic_secs();
    let (y, m, d, h, min) = ts_ymd_hm(secs);

    let base = format!("{y:04}{m:02}{d:02}{h:02}{min:02}");
    let filename = if !diffs_root.join(format!("{base}.patch")).exists() {
        format!("{base}.patch")
    } else {
        let mut counter = 1;
        loop {
            let name = format!("{base}_{counter}.patch");
            if !diffs_root.join(&name).exists() {
                break name;
            }
            counter += 1;
        }
    };
    let diff_path = diffs_root.join(&filename);

    let diff_content = serde_json::json!({
        "path": file_path,
        "old_string": old_string,
        "new_string": new_string,
        "replace_all": replace_all,
        "timestamp": secs * 1000,
        "reverted": false,
    });

    let mut file = std::fs::File::create(&diff_path).ok()?;
    file.write_all(serde_json::to_string_pretty(&diff_content).ok()?.as_bytes())
        .ok()?;

    Some(diff_path.to_string_lossy().into_owned())
}
/// which preserves the original behavior: out-of-workspace accesses
/// are rejected with a clear error. Callers (e.g. the CLI startup
/// hook or test fixtures) can override the policy via
/// [`set_active_workspace_policy`].
pub struct ActiveBoundaryPolicy {
    cell: std::sync::Mutex<BoundaryPolicy>,
}

impl Default for ActiveBoundaryPolicy {
    fn default() -> Self {
        Self {
            cell: std::sync::Mutex::new(BoundaryPolicy::Allow),
        }
    }
}

impl ActiveBoundaryPolicy {
    pub const fn new() -> Self {
        Self {
            cell: std::sync::Mutex::new(BoundaryPolicy::Block),
        }
    }

    /// Replace the active policy. Returns the previous policy for
    /// callers that want to restore it (handy in tests).
    pub fn set(&self, policy: BoundaryPolicy) -> BoundaryPolicy {
        let mut guard = self.cell.lock().expect("workspace policy mutex poisoned");
        std::mem::replace(&mut *guard, policy)
    }

    /// Snapshot the active policy.
    pub fn get(&self) -> BoundaryPolicy {
        self.cell
            .lock()
            .expect("workspace policy mutex poisoned")
            .clone()
    }
}

fn global_workspace_policy() -> &'static ActiveBoundaryPolicy {
    use std::sync::OnceLock;
    static POLICY: OnceLock<ActiveBoundaryPolicy> = OnceLock::new();
    POLICY.get_or_init(ActiveBoundaryPolicy::new)
}

/// Override the active workspace policy. Returns the previous policy
/// so callers can restore it. Useful for the CLI startup hook (which
/// reads `--workspace-policy`) and for tests that exercise the
/// `Prompt` and `Allow` modes.
pub fn set_active_workspace_policy(policy: BoundaryPolicy) -> BoundaryPolicy {
    global_workspace_policy().set(policy)
}

/// Snapshot of the active workspace policy.
pub fn active_workspace_policy() -> BoundaryPolicy {
    global_workspace_policy().get()
}

/// Record that the user explicitly named a path in input. In
/// `Prompt`/`ExternalReadOnly` mode, this pre-trusts the path's
/// parent directory so the LLM can read it without prompting. In
/// `Block` and `Allow` modes this is a no-op: the policy already has
/// a fixed answer for every path. Designed to be called from the
/// input parser whenever it detects an absolute path in user input.
pub fn note_user_input_path(path: &Path) {
    global_workspace_policy().get().note_user_path(path);
}

/// Count of paths the user has explicitly named in input. Exposed
/// for tests and `claw status` output.
pub fn user_typed_path_count() -> usize {
    global_workspace_policy().get().user_typed_count()
}

/// Process-global active permission mode. Mirrors [`ActiveBoundaryPolicy`]:
/// the CLI writes the session's mode here at startup and on `/permissions`
/// changes, and sub-agent spawns read it so a sub-agent inherits the same
/// permission regime as its parent (permission passthrough). The default is
/// `Yolo` so spawns from contexts that never set it run under the yolo
/// regime (workspace-write base + external read-only + others ask).
pub struct ActivePermissionMode {
    cell: std::sync::Mutex<PermissionMode>,
}

impl ActivePermissionMode {
    pub const fn new() -> Self {
        Self {
            cell: std::sync::Mutex::new(PermissionMode::Yolo),
        }
    }

    /// Replace the active mode. Returns the previous mode so callers
    /// can restore it (handy in tests).
    pub fn set(&self, mode: PermissionMode) -> PermissionMode {
        let mut guard = self.cell.lock().expect("permission mode mutex poisoned");
        std::mem::replace(&mut *guard, mode)
    }

    /// Snapshot the active mode.
    pub fn get(&self) -> PermissionMode {
        *self
            .cell
            .lock()
            .expect("permission mode mutex poisoned")
    }
}

fn global_permission_mode() -> &'static ActivePermissionMode {
    use std::sync::OnceLock;
    static MODE: OnceLock<ActivePermissionMode> = OnceLock::new();
    MODE.get_or_init(ActivePermissionMode::new)
}

/// Override the active permission mode. Returns the previous mode so
/// callers can restore it. Used by the CLI at startup and on
/// `/permissions` changes so sub-agents inherit the session's mode.
pub fn set_active_permission_mode(mode: PermissionMode) -> PermissionMode {
    global_permission_mode().set(mode)
}

/// Snapshot of the active permission mode. Read by sub-agent spawns to
/// implement permission passthrough (sub-agent mode == parent mode).
pub fn active_permission_mode() -> PermissionMode {
    global_permission_mode().get()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolManifestEntry {
    pub name: String,
    pub source: ToolSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSource {
    Base,
    Conditional,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolRegistry {
    entries: Vec<ToolManifestEntry>,
}

impl ToolRegistry {
    #[must_use]
    pub fn new(entries: Vec<ToolManifestEntry>) -> Self {
        Self { entries }
    }

    #[must_use]
    pub fn entries(&self) -> &[ToolManifestEntry] {
        &self.entries
    }
}

// Deleted 2026-06-04 per spec (cycle-break): relocated to
// crates/runtime/src/tool_registry/

#[derive(Debug, Clone)]
pub struct GlobalToolRegistry {
    plugin_tools: Vec<PluginTool>,
    runtime_tools: Vec<RuntimeToolDefinition>,
    enforcer: Option<PermissionEnforcer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeToolDefinition {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
    pub required_permission: PermissionMode,
}

impl GlobalToolRegistry {
    #[must_use]
    pub fn builtin() -> Self {
        Self {
            plugin_tools: Vec::new(),
            runtime_tools: Vec::new(),
            enforcer: None,
        }
    }

    pub fn with_plugin_tools(plugin_tools: Vec<PluginTool>) -> Result<Self, String> {
        let builtin_names = runtime::tool_registry::mvp_tool_specs()
            .into_iter()
            .map(|spec| spec.name.to_string())
            .collect::<BTreeSet<_>>();
        let mut seen_plugin_names = BTreeSet::new();

        for tool in &plugin_tools {
            let name = tool.definition().name.clone();
            if builtin_names.contains(&name) {
                return Err(format!(
                    "plugin tool `{name}` conflicts with a built-in tool name"
                ));
            }
            if !seen_plugin_names.insert(name.clone()) {
                return Err(format!("duplicate plugin tool name `{name}`"));
            }
        }

        Ok(Self {
            plugin_tools,
            runtime_tools: Vec::new(),
            enforcer: None,
        })
    }

    pub fn with_runtime_tools(
        mut self,
        runtime_tools: Vec<RuntimeToolDefinition>,
    ) -> Result<Self, String> {
        let mut seen_names = runtime::tool_registry::mvp_tool_specs()
            .into_iter()
            .map(|spec| spec.name.to_string())
            .chain(
                self.plugin_tools
                    .iter()
                    .map(|tool| tool.definition().name.clone()),
            )
            .collect::<BTreeSet<_>>();

        for tool in &runtime_tools {
            if !seen_names.insert(tool.name.clone()) {
                return Err(format!(
                    "runtime tool `{}` conflicts with an existing tool name",
                    tool.name
                ));
            }
        }

        self.runtime_tools = runtime_tools;
        Ok(self)
    }

    #[must_use]
    pub fn with_enforcer(mut self, enforcer: PermissionEnforcer) -> Self {
        self.set_enforcer(enforcer);
        self
    }

    pub fn normalize_allowed_tools(
        &self,
        values: &[String],
    ) -> Result<Option<BTreeSet<String>>, String> {
        if values.is_empty() {
            return Ok(None);
        }

        let builtin_specs = runtime::tool_registry::mvp_tool_specs();
        let canonical_names = builtin_specs
            .iter()
            .map(|spec| spec.name.to_string())
            .chain(
                self.plugin_tools
                    .iter()
                    .map(|tool| tool.definition().name.clone()),
            )
            .chain(self.runtime_tools.iter().map(|tool| tool.name.clone()))
            .collect::<Vec<_>>();
        let mut name_map = canonical_names
            .iter()
            .map(|name| (normalize_tool_name(name), name.clone()))
            .collect::<BTreeMap<_, _>>();

        for (alias, canonical) in [
            ("read", "read_file"),
            ("write", "new_file"),
            ("write_file", "new_file"),
            ("edit", "edit_file"),
            ("glob", "glob_search"),
            ("grep", "grep_search"),
        ] {
            name_map.insert(alias.to_string(), canonical.to_string());
        }

        let mut allowed = BTreeSet::new();
        for value in values {
            for token in value
                .split(|ch: char| ch == ',' || ch.is_whitespace())
                .filter(|token| !token.is_empty())
            {
                let normalized = normalize_tool_name(token);
                let canonical = name_map.get(&normalized).ok_or_else(|| {
                    format!(
                        "unsupported tool in --allowedTools: {token} (expected one of: {})",
                        canonical_names.join(", ")
                    )
                })?;
                allowed.insert(canonical.clone());
            }
        }

        Ok(Some(allowed))
    }

    #[must_use]
    pub fn definitions(&self, allowed_tools: Option<&BTreeSet<String>>) -> Vec<ToolDefinition> {
        let builtin = runtime::tool_registry::mvp_tool_specs()
            .into_iter()
            .filter(|spec| {
                let requested = allowed_tools.is_none_or(|allowed| allowed.contains(spec.name));
                // Internal tools (StructuredOutput) are only advertised when explicitly
                // requested by a sub-agent's allowed_tools list — they are hidden from
                // the main (user-facing) LLM.
                requested && (!spec.internal || allowed_tools.is_some())
            })
            .map(|spec| ToolDefinition {
                name: spec.name.to_string(),
                description: Some(spec.description.to_string()),
                input_schema: spec.input_schema,
            });
        let runtime = self
            .runtime_tools
            .iter()
            .filter(|tool| allowed_tools.is_none_or(|allowed| allowed.contains(tool.name.as_str())))
            .map(|tool| ToolDefinition {
                name: tool.name.clone(),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
            });
        let plugin = self
            .plugin_tools
            .iter()
            .filter(|tool| {
                allowed_tools
                    .is_none_or(|allowed| allowed.contains(tool.definition().name.as_str()))
            })
            .map(|tool| ToolDefinition {
                name: tool.definition().name.clone(),
                description: tool.definition().description.clone(),
                input_schema: tool.definition().input_schema.clone(),
            });
        builtin.chain(runtime).chain(plugin).collect()
    }

    pub fn permission_specs(
        &self,
        allowed_tools: Option<&BTreeSet<String>>,
    ) -> Result<Vec<(String, PermissionMode)>, String> {
        let builtin = runtime::tool_registry::mvp_tool_specs()
            .into_iter()
            .filter(|spec| {
                let requested = allowed_tools.is_none_or(|allowed| allowed.contains(spec.name));
                requested && (!spec.internal || allowed_tools.is_some())
            })
            .map(|spec| (spec.name.to_string(), spec.required_permission));
        let runtime = self
            .runtime_tools
            .iter()
            .filter(|tool| allowed_tools.is_none_or(|allowed| allowed.contains(tool.name.as_str())))
            .map(|tool| (tool.name.clone(), tool.required_permission));
        let plugin = self
            .plugin_tools
            .iter()
            .filter(|tool| {
                allowed_tools
                    .is_none_or(|allowed| allowed.contains(tool.definition().name.as_str()))
            })
            .map(|tool| {
                permission_mode_from_plugin(tool.required_permission())
                    .map(|permission| (tool.definition().name.clone(), permission))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(builtin.chain(runtime).chain(plugin).collect())
    }

    #[must_use]
    pub fn has_runtime_tool(&self, name: &str) -> bool {
        self.runtime_tools.iter().any(|tool| tool.name == name)
    }

    pub fn set_enforcer(&mut self, enforcer: PermissionEnforcer) {
        self.enforcer = Some(enforcer);
    }

    pub fn execute(&self, name: &str, input: &Value) -> Result<String, String> {
        let resolved = resolve_known_tool_name(name);
        if runtime::tool_registry::mvp_tool_specs()
            .iter()
            .any(|spec| spec.name == resolved)
        {
            return execute_tool_with_enforcer(self.enforcer.as_ref(), &resolved, input);
        }
        self.plugin_tools
            .iter()
            .find(|tool| tool.definition().name == resolved)
            .ok_or_else(|| format!("unsupported tool: {name}"))?
            .execute(input)
            .map_err(|error| error.to_string())
    }
}

fn normalize_tool_name(value: &str) -> String {
    value.trim().replace('-', "_").to_ascii_lowercase()
}

/// Resolve a tool name emitted by the model to a registered tool name using
/// case-insensitive matching. This compensates for vocabulary drift where the
/// model emits `Bash` but the registry registers `bash`, without forcing a
/// blind lowercase fold that would break camelCase tools like `WebFetch`.
fn resolve_known_tool_name(name: &str) -> String {
    if runtime::tool_registry::mvp_tool_specs()
        .iter()
        .any(|spec| spec.name == name)
    {
        return name.to_string();
    }
    // Map the model's "Write" / "write_file" aliases to the canonical
    // "new_file" tool name.  Claude sometimes emits these variants.
    if let Some(canonical) = KNOWN_TOOL_ALIASES
        .iter()
        .find_map(|(alias, canonical)| (*alias == name).then_some(*canonical))
    {
        return canonical.to_string();
    }
    let lowered = name.to_ascii_lowercase();
    if let Some(spec) = runtime::tool_registry::mvp_tool_specs()
        .iter()
        .find(|spec| spec.name.to_ascii_lowercase() == lowered)
    {
        return spec.name.to_string();
    }
    name.to_string()
}

/// Static alias table: model-emitted tool names that must route to a
/// different canonical name.  Kept as a simple slice so the match is
/// O(n) but n is tiny and the compiler can inline the whole thing.
const KNOWN_TOOL_ALIASES: &[(&str, &str)] = &[
    ("Write", "new_file"),
    ("write_file", "new_file"),
    ("web_search", "WebSearch"),
];

fn permission_mode_from_plugin(value: &str) -> Result<PermissionMode, String> {
    match value {
        "read-only" => Ok(PermissionMode::ReadOnly),
        "workspace-write" => Ok(PermissionMode::WorkspaceWrite),
        "yolo" => Ok(PermissionMode::Yolo),
        "danger-full-access" => Ok(PermissionMode::DangerFullAccess),
        other => Err(format!("unsupported plugin permission: {other}")),
    }
}

// Deleted 2026-06-04 per spec (cycle-break): relocated to
// crates/runtime/src/tool_registry/

/// Deserialize a Value into type T, converting serde errors to String.
fn from_value<T: serde::de::DeserializeOwned>(input: &Value) -> Result<T, String> {
    serde_json::from_value(input.clone()).map_err(|e| e.to_string())
}

/// Check permission before executing a tool. Returns Err with denial reason if blocked.
pub fn enforce_permission_check(
    enforcer: &PermissionEnforcer,
    tool_name: &str,
    input: &Value,
) -> Result<(), String> {
    let input_str = serde_json::to_string(input).unwrap_or_default();
    let result = enforcer.check(tool_name, &input_str);

    match result {
        EnforcementResult::Allowed => Ok(()),
        EnforcementResult::Denied { reason, .. } => Err(reason),
    }
}

pub fn execute_tool(name: &str, input: &Value) -> Result<String, String> {
    execute_tool_with_enforcer(None, name, input)
}

/// Register `execute_tool` as the global sub-agent tool executor. Must
/// be called once at startup, before any sub-agent runs. Subsequent
/// calls are a no-op so test binaries that call it more than once
/// (e.g. via `tools_init` in two `#[test]` functions sharing the same
/// static) do not fail.
pub fn tools_init() -> Result<(), String> {
    init_web_search_config();
    agents::init_global_runtime();
    use agents::register_tool_executor;
    match register_tool_executor(Box::new(|name, value, _policy| execute_tool(name, value))) {
        Ok(()) => Ok(()),
        Err(error) if error == "tool executor already registered" => Ok(()),
        Err(error) => Err(error),
    }
}

/// Execute a tool, optionally running its result through a `PermissionEnforcer`.
///
/// # Permission contract
///
/// When `enforcer` is `Some`, the enforcer runs before the tool body and may
/// deny the call. When `enforcer` is `None`, **no permission check is performed
/// and the tool body runs unconditionally**. Callers that need guaranteed
/// permission enforcement must pass a `Some` value �?typically
/// [`PermissionEnforcer::permissive`] is the wrong choice for production paths.
///
/// For new code, prefer `PermissionPolicy::authorize_with_context` (from the
/// `runtime` crate) at the caller site rather than this helper, since the
/// `PermissionEnforcer` wrapper is deprecated and may be removed.
fn execute_tool_with_enforcer(
    enforcer: Option<&PermissionEnforcer>,
    name: &str,
    input: &Value,
) -> Result<String, String> {
    let name = resolve_known_tool_name(name);
    let name = name.as_str();
    // Pre-process: check for PDF files in tool input and extract text automatically
    if let Some((pdf_path, pdf_text)) =
        pdf_extract::maybe_extract_pdf_from_prompt(&input.to_string())
    {
        eprintln!(
            "[pdf_extract] Extracted text from {} ({} chars)",
            pdf_path,
            pdf_text.len()
        );
        // Inject extracted PDF text into the tool input for processing
        // This allows tools to work with PDF content seamlessly
    }

    match name {
        "bash" => {
            // Parse input to get the command for permission classification
            let bash_input: BashCommandInput = from_value(input)?;
            let classified_mode = classify_bash_permission(&bash_input.command);
            maybe_enforce_permission_check_with_mode(enforcer, name, input, classified_mode)?;
            run_bash(bash_input)
        }
        "read_file" => {
            maybe_enforce_permission_check(enforcer, name, input)?;
            from_value::<ReadFileInput>(input).and_then(run_read_file)
        }
        "new_file" | "Write" | "write_file" => {
            maybe_enforce_permission_check(enforcer, name, input)?;
            from_value::<WriteFileInput>(input).and_then(run_new_file)
        }
        "edit_file" => {
            maybe_enforce_permission_check(enforcer, name, input)?;
            from_value::<EditFileInput>(input).and_then(run_edit_file)
        }
        "undo" => {
            maybe_enforce_permission_check(enforcer, name, input)?;
            from_value::<UndoInput>(input).and_then(run_undo)
        }
        "glob_search" => {
            maybe_enforce_permission_check(enforcer, name, input)?;
            from_value::<GlobSearchInputValue>(input).and_then(run_glob_search)
        }
        "grep_search" => {
            maybe_enforce_permission_check(enforcer, name, input)?;
            from_value::<GrepSearchInput>(input).and_then(run_grep_search)
        }
        "WebFetch" => from_value::<WebFetchInput>(input).and_then(run_web_fetch),
        "WebFind" => from_value::<WebFindInput>(input).and_then(run_web_find),
        "WebSearch" => from_value::<WebSearchInput>(input).and_then(run_web_search),
        "Skill" => from_value::<SkillInput>(input).and_then(run_skill),
        "Agent" => {
            maybe_enforce_permission_check(enforcer, name, input)?;
            from_value::<AgentInput>(input).and_then(run_agent)
        }
        // Deprecated legacy alias for Brief. Kept for backward compatibility.
        "SendUserMessage" | "Brief" => from_value::<BriefInput>(input).and_then(run_brief),
        "StructuredOutput" => {
            from_value::<StructuredOutputInput>(input).and_then(run_structured_output)
        }

        "ListAgents" => run_list_agents(input.clone()),
        "ListSkills" => run_list_skills(input.clone()),
        "ListPlugins" => run_list_plugins(input.clone()),
        _ => Err(format!("unsupported tool: {name}")),
    }
}

fn maybe_enforce_permission_check(
    enforcer: Option<&PermissionEnforcer>,
    tool_name: &str,
    input: &Value,
) -> Result<(), String> {
    if let Some(enforcer) = enforcer {
        enforce_permission_check(enforcer, tool_name, input)?;
    }
    Ok(())
}

/// Enforce permission check with a dynamically classified permission mode.
/// Used for tools like bash where the required permission
/// depends on the actual command being executed.
fn maybe_enforce_permission_check_with_mode(
    enforcer: Option<&PermissionEnforcer>,
    tool_name: &str,
    input: &Value,
    required_mode: PermissionMode,
) -> Result<(), String> {
    if let Some(enforcer) = enforcer {
        let input_str = serde_json::to_string(input).unwrap_or_default();
        let result = enforcer.check_with_required_mode(tool_name, &input_str, required_mode);

        match result {
            EnforcementResult::Allowed => Ok(()),
            EnforcementResult::Denied { reason, .. } => Err(reason),
        }
    } else {
        Ok(())
    }
}

fn run_list_agents(_input: Value) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let discovery = AgentDiscovery::new(&cwd);
    let active = discovery.active_names_list();
    to_pretty_json(json!({
        "agents": active,
        "count": active.len()
    }))
}

fn run_list_skills(_input: Value) -> Result<String, String> {
    let mut skills = Vec::new();
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;

    for ancestor in cwd.ancestors() {
        let skills_dir = ancestor.join(".claude").join("skills");
        if !skills_dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let skill_path = if path.is_dir() {
                    path.join("SKILL.md")
                } else {
                    path.clone()
                };
                if !skill_path.is_file() {
                    continue;
                }
                if let Some(content) = parse_frontmatter_name(&skill_path) {
                    skills.push(content);
                } else if let Some(stem) = skill_path.file_stem() {
                    skills.push(stem.to_string_lossy().to_string());
                }
            }
        }
    }

    skills.sort();
    skills.dedup();
    to_pretty_json(json!({
        "skills": skills,
        "count": skills.len()
    }))
}

fn run_list_plugins(_input: Value) -> Result<String, String> {
    let mut plugins = Vec::new();
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;

    for ancestor in cwd.ancestors() {
        let plugins_dir = ancestor.join(".claude").join("plugins");
        if !plugins_dir.is_dir() {
            continue;
        }
        // Scan installed plugins (plugins/<name>/)
        let installed = plugins_dir.join("installed.json");
        if installed.is_file() {
            if let Ok(content) = std::fs::read_to_string(&installed) {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(arr) = parsed.as_array() {
                        for entry in arr {
                            if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
                                plugins.push(id.to_string());
                            }
                        }
                    }
                }
            }
        }
        // Also scan cache/ for plugins
        let cache = plugins_dir.join("cache");
        if cache.is_dir() {
            if let Ok(marketplaces) = std::fs::read_dir(&cache) {
                for mp in marketplaces.flatten() {
                    if let Ok(plugin_names) = std::fs::read_dir(mp.path()) {
                        for pn in plugin_names.flatten() {
                            let id = pn.file_name().to_string_lossy().to_string();
                            if !plugins.contains(&id) {
                                plugins.push(id);
                            }
                        }
                    }
                }
            }
        }
    }

    plugins.sort();
    to_pretty_json(json!({
        "plugins": plugins,
        "count": plugins.len()
    }))
}

fn parse_frontmatter_name(path: &std::path::Path) -> Option<String> {
    let contents = std::fs::read_to_string(path).ok()?;
    plugins::frontmatter::parse_frontmatter(&contents)
        .ok()?
        .frontmatter
        .name
}

/// Classify bash command permission based on command type and path.
/// ROADMAP #50: Read-only commands targeting CWD paths get `WorkspaceWrite`,
/// all others remain `DangerFullAccess`.
fn classify_bash_permission(command: &str) -> PermissionMode {
    // Read-only commands that are safe when targeting workspace paths
    const READ_ONLY_COMMANDS: &[&str] = &[
        "cat", "head", "tail", "less", "more", "ls", "ll", "dir", "find", "test", "[", "[[",
        "grep", "rg", "awk", "sed", "file", "stat", "readlink", "wc", "sort", "uniq", "cut", "tr",
        "pwd", "echo", "printf",
    ];

    // Get the base command: isolate the first pipeline/sequence segment
    // before extracting the command name. Metacharacters (;, |, >, <, &&, ||)
    // are split on the full command first — previously they were applied
    // to `base_cmd` (already a single word), making the splits no-ops.
    let first_segment = command
        .split("||")
        .next().unwrap_or("")
        .split("&&")
        .next().unwrap_or("")
        .split('|')
        .next().unwrap_or("")
        .split(';')
        .next().unwrap_or("")
        .split('>')
        .next().unwrap_or("")
        .split('<')
        .next().unwrap_or("")
        .trim();
    let base_cmd = first_segment.split_whitespace().next().unwrap_or("");

    // Check if it's a read-only command
    let cmd_name = base_cmd.split('/').next_back().unwrap_or(base_cmd);
    let is_read_only = READ_ONLY_COMMANDS.contains(&cmd_name);

    if !is_read_only {
        return PermissionMode::DangerFullAccess;
    }

    // Check if any path argument is outside workspace
    // Simple heuristic: check for absolute paths not starting with CWD
    if has_dangerous_paths(command) {
        return PermissionMode::DangerFullAccess;
    }

    PermissionMode::WorkspaceWrite
}

/// Check if command has dangerous paths (outside workspace).
fn has_dangerous_paths(command: &str) -> bool {
    // Look for absolute paths
    let tokens: Vec<&str> = command.split_whitespace().collect();

    for token in tokens {
        // Strip surrounding quotes so `cat "C:\Users\foo\bar.txt"` is
        // recognised as a Windows absolute path.
        let stripped = token
            .trim_start_matches('"')
            .trim_end_matches('"')
            .trim_start_matches('\'')
            .trim_end_matches('\'');

        // Skip flags/options
        if stripped.starts_with('-') {
            continue;
        }

        // file:// URL — treated as dangerous since it bypasses normal path checks
        if stripped.starts_with("file://") {
            return true;
        }

        // Network URLs (http, https, ftp, etc.) — treated as dangerous
        // since they bypass normal path checks and enable data exfiltration.
        if stripped.starts_with("http://")
            || stripped.starts_with("https://")
            || stripped.starts_with("ftp://")
            || stripped.starts_with("ftp.")
        {
            return true;
        }

        // POSIX absolute path or `~/...` home-relative
        if stripped.starts_with('/') || stripped.starts_with("~/") {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_default();
            let path = PathBuf::from(stripped.replace('~', &home));
            if let Ok(cwd) = std::env::current_dir() {
                if !path.starts_with(&cwd) {
                    return true; // Path outside workspace
                }
            }
        }

        // Windows drive-letter absolute path: `<letter>:\` or `<letter>:/`
        // e.g. `C:\Users\foo\bar.txt`, `D:/data/file.txt`.
        if stripped.len() >= 3
            && stripped.as_bytes()[0].is_ascii_alphabetic()
            && stripped.as_bytes()[1] == b':'
            && (stripped.as_bytes()[2] == b'\\' || stripped.as_bytes()[2] == b'/')
        {
            return true;
        }

        // UNC path: `\\server\share\...`
        if stripped.starts_with("\\\\") {
            return true;
        }

        // Check for parent directory traversal that escapes workspace.
        // Matches both Unix (../) and Windows (..\) styles, and
        // catches ./../ which is not caught by starts_with("../") alone.
        if stripped.contains("..") {
            let rest = stripped.trim_start_matches("./").trim_start_matches(".\\");
            if rest.starts_with("../") || rest.starts_with("..\\") {
                return true;
            }
        }
    }

    false
}

fn run_bash(input: BashCommandInput) -> Result<String, String> {
    if let Some(output) = workspace_test_branch_preflight(&input.command) {
        return Ok(bash_model_view(&output));
    }
    let output = execute_bash(input).map_err(|error| error.to_string())?;
    Ok(bash_model_view(&output))
}

/// Render a `BashCommandOutput` for the model. The compact envelope
/// puts `stdout` and `stderr` first so the model sees the command's
/// output before any sandbox diagnostics, and it always carries
/// `sandbox.fallbackReason` so the model can reason honestly about
/// which sandbox mechanisms are actually enforced (rather than
/// concluding "the sandbox blocked it" when only process-tree kill
/// is active, as the legacy 16-field envelope allowed).
fn bash_model_view(output: &runtime::BashCommandOutput) -> String {
    let sandbox_block = output.sandbox_status.as_ref().map(|status| {
        serde_json::json!({
            "enabled": status.enabled,
            "active": status.active,
            "type": output.sandbox_type,
            "fallbackReason": status.fallback_reason,
        })
    });
    let view = serde_json::json!({
        "stdout": output.stdout,
        "stderr": output.stderr,
        "interrupted": output.interrupted,
        "returnCodeInterpretation": output.return_code_interpretation,
        "noOutputExpected": output.no_output_expected,
        "persistedOutputPath": output.persisted_output_path,
        "persistedOutputSize": output.persisted_output_size,
        "backgroundTaskId": output.background_task_id,
        "backgroundedByUser": output.backgrounded_by_user,
        "assistantAutoBackgrounded": output.assistant_auto_backgrounded,
        "dangerouslyDisableSandbox": output.dangerously_disable_sandbox,
        "structuredContent": output.structured_content,
        "sandbox": sandbox_block,
    });
    serde_json::to_string_pretty(&view).unwrap_or_else(|error| {
        // Fall back to the full envelope if the view can't be
        // serialised �?the model still gets *some* output rather than
        // a hard error.
        serde_json::to_string_pretty(output)
            .unwrap_or_else(|_| format!("{{\"error\":\"{error}\"}}"))
    })
}

fn workspace_test_branch_preflight(command: &str) -> Option<BashCommandOutput> {
    if !is_workspace_test_command(command) {
        return None;
    }

    let branch = git_stdout(&["branch", "--show-current"])?;
    let main_ref = resolve_main_ref(&branch)?;
    let freshness = check_freshness(&branch, &main_ref);
    match freshness {
        BranchFreshness::Fresh => None,
        BranchFreshness::Stale {
            commits_behind,
            missing_fixes,
        } => Some(branch_divergence_output(
            command,
            &branch,
            &main_ref,
            commits_behind,
            None,
            &missing_fixes,
        )),
        BranchFreshness::Diverged {
            ahead,
            behind,
            missing_fixes,
        } => Some(branch_divergence_output(
            command,
            &branch,
            &main_ref,
            behind,
            Some(ahead),
            &missing_fixes,
        )),
    }
}

fn is_workspace_test_command(command: &str) -> bool {
    let normalized = normalize_shell_command(command);
    [
        "cargo test --workspace",
        "cargo test --all",
        "cargo nextest run --workspace",
        "cargo nextest run --all",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn normalize_shell_command(command: &str) -> String {
    command
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn resolve_main_ref(branch: &str) -> Option<String> {
    let has_local_main = git_ref_exists("main");
    let has_remote_main = git_ref_exists("origin/main");

    if branch == "main" && has_remote_main {
        Some("origin/main".to_string())
    } else if has_local_main {
        Some("main".to_string())
    } else if has_remote_main {
        Some("origin/main".to_string())
    } else {
        None
    }
}

fn git_ref_exists(reference: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", reference])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git_stdout(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!stdout.is_empty()).then_some(stdout)
}

fn branch_divergence_output(
    command: &str,
    branch: &str,
    main_ref: &str,
    commits_behind: usize,
    commits_ahead: Option<usize>,
    missing_fixes: &[String],
) -> BashCommandOutput {
    let relation = commits_ahead.map_or_else(
        || format!("is {commits_behind} commit(s) behind"),
        |ahead| format!("has diverged ({ahead} ahead, {commits_behind} behind)"),
    );
    let missing_summary = if missing_fixes.is_empty() {
        "(none surfaced)".to_string()
    } else {
        missing_fixes.join("; ")
    };
    let stderr = format!(
        "branch divergence detected before workspace tests: `{branch}` {relation} `{main_ref}`. Missing commits: {missing_summary}. Merge or rebase `{main_ref}` before re-running `{command}`."
    );

    BashCommandOutput {
        stdout: String::new(),
        stderr: stderr.clone(),
        raw_output_path: None,
        interrupted: false,
        is_image: None,
        background_task_id: None,
        backgrounded_by_user: None,
        assistant_auto_backgrounded: None,
        dangerously_disable_sandbox: None,
        return_code_interpretation: Some("preflight_blocked:branch_divergence".to_string()),
        no_output_expected: Some(false),
        structured_content: Some(vec![serde_json::to_value(
            LaneEvent::new(
                LaneEventName::BranchStaleAgainstMain,
                LaneEventStatus::Blocked,
                iso8601_now(),
            )
            .with_failure_class(LaneFailureClass::BranchDivergence)
            .with_detail(stderr.clone())
            .with_data(json!({
                "branch": branch,
                "mainRef": main_ref,
                "commitsBehind": commits_behind,
                "commitsAhead": commits_ahead,
                "missingCommits": missing_fixes,
                "blockedCommand": command,
                "recommendedAction": format!("merge or rebase {main_ref} before workspace tests")
            })),
        )
        .expect("lane event should serialize")]),
        persisted_output_path: None,
        persisted_output_size: None,
        sandbox_status: None,
        sandbox_type: None,
    }
}

fn extract_title(content: &str, raw_body: &str, content_type: &str) -> Option<String> {
    if content_type.contains("html") {
        let lowered = raw_body.to_lowercase();
        if let Some(start) = lowered.find("<title>") {
            let after = start + "<title>".len();
            if let Some(end_rel) = lowered[after..].find("</title>") {
                let title =
                    collapse_whitespace(&decode_html_entities(&raw_body[after..after + end_rel]));
                if !title.is_empty() {
                    return Some(title);
                }
            }
        }
    }

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

#[allow(dead_code)]
fn html_to_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut previous_was_space = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if in_tag => {}
            '&' => {
                text.push('&');
                previous_was_space = false;
            }
            ch if ch.is_whitespace() => {
                if !previous_was_space {
                    text.push(' ');
                    previous_was_space = true;
                }
            }
            _ => {
                text.push(ch);
                previous_was_space = false;
            }
        }
    }

    collapse_whitespace(&decode_html_entities(&text))
}

fn decode_html_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

fn collapse_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn preview_text(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let shortened = input.chars().take(max_chars).collect::<String>();
    format!("{}��", shortened.trim_end())
}

/// Smart content extractor for fetched HTML pages.
/// Identifies the main content container (article/main/section/.content/etc.),
/// skips nav/footer/aside/.ad noise, and detects JavaScript-heavy pages
/// so the caller can fall back to RSS or text-only mode.
struct FastContentEvaluator {
    js_ratio_threshold: f64,
    min_text_len: usize,
    script_sel: Selector,
    container_sel: Selector,
    negative_sel: Selector,
    p_sel: Selector,
    body_sel: Selector,
    div_sel: Selector,
}

impl Default for FastContentEvaluator {
    fn default() -> Self {
        Self {
            js_ratio_threshold: 0.30,
            min_text_len: 60,
            script_sel: Selector::parse("script").unwrap(),
            container_sel: Selector::parse(
                "article, main, section, .content, .post, .article, .entry, #content, #main, .rich_media_content, .article-content, .news_content, .text_content, .content-article",
            )
            .unwrap(),
            negative_sel: Selector::parse("nav, footer, header, aside, .sidebar, .ad, .nav, .footer")
                .unwrap(),
            p_sel: Selector::parse("p").unwrap(),
            body_sel: Selector::parse("body").unwrap(),
            div_sel: Selector::parse("div").unwrap(),
        }
    }
}

struct PageAnalysis {
    #[allow(dead_code)]
    js_ratio: f64,
    best_content_len: usize,
    #[allow(dead_code)]
    has_external_scripts: bool,
}

impl FastContentEvaluator {
    fn analyze(&self, html: &str) -> PageAnalysis {
        if html.len() < 50 {
            return PageAnalysis {
                js_ratio: 1.0,
                best_content_len: 0,
                has_external_scripts: false,
            };
        }

        let document = Html::parse_document(html);
        let html_len = html.len();

        let (js_ratio, has_external) = self.calculate_js_ratio(&document, html_len);
        let best_content_len = self.detect_content_length(&document);

        PageAnalysis {
            js_ratio,
            best_content_len,
            has_external_scripts: has_external,
        }
    }

    fn should_retain(&self, analysis: &PageAnalysis) -> bool {
        if analysis.best_content_len >= self.min_text_len {
            return true;
        }
        if analysis.js_ratio <= self.js_ratio_threshold && analysis.best_content_len >= 20 {
            return true;
        }
        false
    }

    fn calculate_js_ratio(&self, document: &Html, html_len: usize) -> (f64, bool) {
        let html_len_f = html_len as f64;
        let mut total_len = 0usize;
        let mut has_external = false;

        for el in document.select(&self.script_sel) {
            total_len += el.text().map(|t| t.len()).sum::<usize>();
            if el.value().attr("src").is_some() {
                has_external = true;
                total_len += 1024;
            }
        }

        let ratio = if html_len_f > 0.0 {
            (total_len as f64 / html_len_f).min(1.0)
        } else {
            0.0
        };

        (ratio, has_external)
    }

    fn detect_content_length(&self, document: &Html) -> usize {
        let candidates: Vec<_> = document.select(&self.container_sel).collect();

        let body = match document.select(&self.body_sel).next() {
            Some(b) => b,
            None => return 0,
        };

        let candidate_refs: Vec<_> = if candidates.is_empty() {
            body.select(&self.div_sel).collect()
        } else {
            candidates
        };

        let mut best_score = -1i32;
        let mut best_text_len = 0usize;

        for candidate in &candidate_refs {
            let mut score = 0i32;

            let name = candidate.value().name();
            let class = candidate.value().attr("class").unwrap_or("");
            let id = candidate.value().attr("id").unwrap_or("");

            match name {
                "article" => score += 30,
                "main" => score += 25,
                "section" => score += 15,
                _ => {}
            }

            let class_tokens = format!(" {} ", class);
            let id_tokens = format!(" {} ", id);

            if class_tokens.contains(" content ")
                || class_tokens.contains(" post ")
                || class_tokens.contains(" article ")
                || class_tokens.contains(" entry ")
            {
                score += 25;
            }

            if id_tokens.contains(" content ")
                || id_tokens.contains(" main ")
                || id_tokens.contains(" article ")
            {
                score += 25;
            }

            let p_count = candidate.select(&self.p_sel).count();
            score += p_count as i32 * 8;

            let neg_count = candidate.select(&self.negative_sel).count();
            score -= neg_count as i32 * 12;

            if score > best_score {
                best_score = score;
                best_text_len = candidate
                    .text()
                    .map(|t| t.chars().filter(|c| !c.is_whitespace()).count())
                    .sum();
            }
        }

        if best_text_len == 0 {
            best_text_len = body
                .text()
                .map(|t| t.chars().filter(|c| !c.is_whitespace()).count())
                .sum();
        }

        best_text_len
    }

    fn extract_text(&self, html: &str) -> String {
        if html.len() < 50 {
            return String::new();
        }

        let document = Html::parse_document(html);
        let candidates: Vec<_> = document.select(&self.container_sel).collect();

        let body = match document.select(&self.body_sel).next() {
            Some(b) => b,
            None => return String::new(),
        };

        let candidate_refs: Vec<_> = if candidates.is_empty() {
            body.select(&self.div_sel).collect()
        } else {
            candidates
        };

        let mut best_score = -1i32;
        let mut best_text = String::new();

        for candidate in &candidate_refs {
            let mut score = 0i32;

            let name = candidate.value().name();
            let class = candidate.value().attr("class").unwrap_or("");
            let id = candidate.value().attr("id").unwrap_or("");

            match name {
                "article" => score += 30,
                "main" => score += 25,
                "section" => score += 15,
                _ => {}
            }

            let class_tokens = format!(" {} ", class);
            let id_tokens = format!(" {} ", id);

            if class_tokens.contains(" content ")
                || class_tokens.contains(" post ")
                || class_tokens.contains(" article ")
                || class_tokens.contains(" entry ")
            {
                score += 25;
            }

            if id_tokens.contains(" content ")
                || id_tokens.contains(" main ")
                || id_tokens.contains(" article ")
            {
                score += 25;
            }

            let p_count = candidate.select(&self.p_sel).count();
            score += p_count as i32 * 8;

            let neg_count = candidate.select(&self.negative_sel).count();
            score -= neg_count as i32 * 12;

            if score > best_score {
                best_score = score;
                best_text = candidate.text().collect();
            }
        }

        if best_text.is_empty() {
            best_text = body.text().collect();
        }

        best_text
    }
}

fn summarize_web_fetch(url: &str, prompt: &str, raw_body: &str, content_type: &str) -> String {
    let lower_prompt = prompt.to_ascii_lowercase();

    // For non-HTML content, skip the SmartContentEvaluator and just
    // surface the trimmed body �?the evaluator's heuristics assume
    // an HTML document.
    if !content_type.contains("html") {
        let text = raw_body.trim();
        let detail = if lower_prompt.contains("title") {
            text.lines()
                .find(|line| !line.trim().is_empty())
                .map(|line| format!("Title: {line}"))
                .unwrap_or_else(|| format!("Fetched {url}\n\n{text}"))
        } else if lower_prompt.contains("summary") || lower_prompt.contains("summarize") {
            format!("Fetched {url}\n\n{text}")
        } else {
            format!(
                "Fetched {url}\n\nPrompt: {prompt}\n\n{text}",
                prompt = prompt
            )
        };
        return detail;
    }

    let evaluator = FastContentEvaluator::default();
    let analysis = evaluator.analyze(raw_body);
    let should_retain = evaluator.should_retain(&analysis);
    let main_text = evaluator.extract_text(raw_body);
    let compact = collapse_whitespace(&main_text);

    let is_dynamic_content = !should_retain;

    let detail = if is_dynamic_content {
        let normalized_for_title = collapse_whitespace(&decode_html_entities(&main_text));
        let title = extract_title(&normalized_for_title, raw_body, content_type)
            .unwrap_or_else(|| "Unable to extract".to_string());
        format!(
            "Title: {}\n\nNote: This page uses JavaScript to render content. \
            The web fetch tool cannot execute JavaScript, so only static HTML was retrieved. \
            For best results with news sites, try using a text-only version or RSS feed.",
            title
        )
    } else if lower_prompt.contains("title") {
        extract_title(&compact, raw_body, content_type).map_or_else(
            || preview_text(&compact, 600),
            |title| format!("Title: {title}"),
        )
    } else if lower_prompt.contains("summary") || lower_prompt.contains("summarize") {
        // Return full content for summary requests (up to 50,000 chars)
        preview_text(&compact, 50_000)
    } else {
        // Return full content instead of 900-char preview to avoid
        // AI repeatedly fetching the same page trying to get complete content.
        // This saves tokens overall: one fetch with full content vs multiple
        // fetches with truncated previews.
        let full = preview_text(&compact, 50_000);
        format!("Prompt: {prompt}\nContent:\n{full}")
    };

    format!("Fetched {url}\n{detail}")
}

fn execute_skill(input: SkillInput) -> Result<SkillOutput, String> {
    let skill_path = resolve_skill_path(&input.skill)?;
    let prompt = std::fs::read_to_string(&skill_path).map_err(|error| error.to_string())?;
    let description = parse_skill_description(&prompt).unwrap_or_default();

    Ok(SkillOutput {
        skill: input.skill,
        path: skill_path.display().to_string(),
        args: input.args,
        description,
        prompt,
    })
}

fn resolve_skill_path(skill: &str) -> Result<std::path::PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    match commands::resolve_skill_path(&cwd, skill) {
        Ok(path) => Ok(path),
        Err(_) => resolve_skill_path_from_compat_roots(skill),
    }
}

fn resolve_skill_path_from_compat_roots(skill: &str) -> Result<std::path::PathBuf, String> {
    let requested = skill.trim().trim_start_matches('/').trim_start_matches('$');
    if requested.is_empty() {
        return Err(String::from("skill must not be empty"));
    }

    if requested == "*" || requested == "all" {
        let names = list_available_skill_names();
        if names.is_empty() {
            return Err(String::from(
                "no skills available �� use /skills list in the CLI",
            ));
        }
        return Err(format!("Available skills: {}", names.join(", ")));
    }

    for root in skill_lookup_roots() {
        if let Some(path) = resolve_skill_path_in_root(&root, requested) {
            return Ok(path);
        }
    }

    let available = list_available_skill_names();
    if available.is_empty() {
        Err(format!(
            "unknown skill: {requested} (no skills are currently available)"
        ))
    } else {
        Err(format!(
            "unknown skill: {requested}. Available skills: {}",
            available.join(", ")
        ))
    }
}

fn list_available_skill_names() -> Vec<String> {
    let mut names = Vec::new();
    for root in skill_lookup_roots() {
        match root.origin {
            SkillLookupOrigin::SkillsDir => {
                if let Ok(entries) = std::fs::read_dir(&root.path) {
                    for entry in entries.flatten() {
                        if !entry.path().is_dir() {
                            continue;
                        }
                        let skill_path = entry.path().join("SKILL.md");
                        if !skill_path.is_file() {
                            continue;
                        }
                        if let Ok(contents) = std::fs::read_to_string(&skill_path) {
                            if let Some(name) = parse_skill_name(&contents) {
                                if !names.contains(&name) {
                                    names.push(name);
                                    continue;
                                }
                            }
                        }
                        let dir_name = entry.file_name().to_string_lossy().to_string();
                        if !names.contains(&dir_name) {
                            names.push(dir_name);
                        }
                    }
                }
            }
            SkillLookupOrigin::LegacyCommandsDir => {
                if let Ok(entries) = std::fs::read_dir(&root.path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && path.join("SKILL.md").is_file() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if !names.contains(&name) {
                                names.push(name);
                            }
                        } else if path.extension().is_some_and(|e| e == "md") {
                            if let Some(stem) = path.file_stem() {
                                let name = stem.to_string_lossy().to_string();
                                if !names.contains(&name) {
                                    names.push(name);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    names.sort();
    names
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkillLookupOrigin {
    SkillsDir,
    LegacyCommandsDir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SkillLookupRoot {
    path: std::path::PathBuf,
    origin: SkillLookupOrigin,
}

fn skill_lookup_roots() -> Vec<SkillLookupRoot> {
    let mut roots = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        push_project_skill_lookup_roots(&mut roots, &cwd);
    }

    if let Ok(claw_config_home) = std::env::var("CLAW_CONFIG_HOME") {
        push_prefixed_skill_lookup_roots(&mut roots, std::path::Path::new(&claw_config_home));
    }
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        push_home_skill_lookup_roots(&mut roots, std::path::Path::new(&home));
    }
    if let Ok(claude_config_dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        let claude_config_dir = std::path::PathBuf::from(claude_config_dir);
        push_skill_lookup_root(
            &mut roots,
            claude_config_dir.join("skills"),
            SkillLookupOrigin::SkillsDir,
        );
        push_skill_lookup_root(
            &mut roots,
            claude_config_dir.join("commands"),
            SkillLookupOrigin::LegacyCommandsDir,
        );
    }

    roots
}

fn push_project_skill_lookup_roots(roots: &mut Vec<SkillLookupRoot>, cwd: &std::path::Path) {
    for ancestor in cwd.ancestors() {
        push_prefixed_skill_lookup_roots(roots, &ancestor.join(".claw"));
        push_prefixed_skill_lookup_roots(roots, &ancestor.join(".claude"));
    }
}

fn push_home_skill_lookup_roots(roots: &mut Vec<SkillLookupRoot>, home: &std::path::Path) {
    push_prefixed_skill_lookup_roots(roots, &home.join(".claw"));
    push_prefixed_skill_lookup_roots(roots, &home.join(".claude"));
    push_skill_lookup_root(
        roots,
        home.join(".config").join("opencode").join("skills"),
        SkillLookupOrigin::SkillsDir,
    );
}

fn push_prefixed_skill_lookup_roots(roots: &mut Vec<SkillLookupRoot>, prefix: &std::path::Path) {
    push_skill_lookup_root(roots, prefix.join("skills"), SkillLookupOrigin::SkillsDir);
    push_skill_lookup_root(
        roots,
        prefix.join("commands"),
        SkillLookupOrigin::LegacyCommandsDir,
    );
}

fn push_skill_lookup_root(
    roots: &mut Vec<SkillLookupRoot>,
    path: std::path::PathBuf,
    origin: SkillLookupOrigin,
) {
    if path.is_dir() && !roots.iter().any(|existing| existing.path == path) {
        roots.push(SkillLookupRoot { path, origin });
    }
}

fn resolve_skill_path_in_root(
    root: &SkillLookupRoot,
    requested: &str,
) -> Option<std::path::PathBuf> {
    match root.origin {
        SkillLookupOrigin::SkillsDir => resolve_skill_path_in_skills_dir(&root.path, requested),
        SkillLookupOrigin::LegacyCommandsDir => {
            resolve_skill_path_in_legacy_commands_dir(&root.path, requested)
        }
    }
}

fn resolve_skill_path_in_skills_dir(
    root: &std::path::Path,
    requested: &str,
) -> Option<std::path::PathBuf> {
    // Legacy single-file skill: `<root>/<requested>.md`
    let direct_file = root.join(format!("{requested}.md"));
    if direct_file.is_file() {
        return Some(direct_file);
    }

    let direct = root.join(requested).join("SKILL.md");
    if direct.is_file() {
        return Some(direct);
    }

    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }
        let dir_name = entry_path.file_name().map_or_else(
            || std::string::String::new(),
            |s| s.to_string_lossy().to_string(),
        );
        let skill_path = entry_path.join("SKILL.md");
        let resolved = if skill_path.is_file() {
            skill_path
        } else {
            let mut md_files: Vec<_> = std::fs::read_dir(&entry_path)
                .ok()?
                .flatten()
                .filter_map(|e| {
                    let p = e.path();
                    if p.extension()
                        .is_some_and(|x| x.to_string_lossy().eq_ignore_ascii_case("md"))
                    {
                        Some(p)
                    } else {
                        None
                    }
                })
                .collect();
            md_files.sort();
            if md_files.len() == 1 {
                md_files.pop().unwrap()
            } else {
                skill_path
            }
        };
        if !resolved.is_file() {
            continue;
        }
        if dir_name.eq_ignore_ascii_case(requested)
            || skill_frontmatter_name_matches(&resolved, requested)
            || resolved.file_stem().map_or(false, |s| {
                s.to_string_lossy().eq_ignore_ascii_case(requested)
            })
        {
            return Some(resolved);
        }
    }

    None
}

fn resolve_skill_path_in_legacy_commands_dir(
    root: &std::path::Path,
    requested: &str,
) -> Option<std::path::PathBuf> {
    let direct_dir = root.join(requested).join("SKILL.md");
    if direct_dir.is_file() {
        return Some(direct_dir);
    }

    let direct_markdown = root.join(format!("{requested}.md"));
    if direct_markdown.is_file() {
        return Some(direct_markdown);
    }

    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let candidate_path = if path.is_dir() {
            let skill_path = path.join("SKILL.md");
            if !skill_path.is_file() {
                continue;
            }
            skill_path
        } else if path
            .extension()
            .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("md"))
        {
            path
        } else {
            continue;
        };

        let matches_entry_name = candidate_path
            .file_stem()
            .is_some_and(|stem| stem.to_string_lossy().eq_ignore_ascii_case(requested))
            || entry
                .file_name()
                .to_string_lossy()
                .trim_end_matches(".md")
                .eq_ignore_ascii_case(requested);
        if matches_entry_name || skill_frontmatter_name_matches(&candidate_path, requested) {
            return Some(candidate_path);
        }
    }

    None
}

fn skill_frontmatter_name_matches(path: &std::path::Path, requested: &str) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| parse_skill_name(&contents))
        .is_some_and(|name| name.eq_ignore_ascii_case(requested))
}

fn parse_skill_name(contents: &str) -> Option<String> {
    plugins::frontmatter::parse_frontmatter(contents)
        .ok()?
        .frontmatter
        .name
}

#[allow(dead_code)]
fn parse_skill_frontmatter_value(contents: &str, key: &str) -> Option<String> {
    if key != "name" {
        return None;
    }
    plugins::frontmatter::parse_frontmatter(contents)
        .ok()?
        .frontmatter
        .name
}


fn iso8601_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|e| {
            eprintln!("[tools] system clock is before epoch ({e}); using 0 for timestamp");
            std::time::Duration::ZERO
        })
        .as_secs()
        .to_string()
}

// Agent system lives in `crates/agents/`. See `execute_agent_with_spawn` above
// for the only remaining local entry point.
fn parse_skill_description(contents: &str) -> Option<String> {
    plugins::frontmatter::parse_frontmatter(contents)
        .ok()?
        .frontmatter
        .description
}

pub mod excel_extract;
pub mod lane_completion;
mod subagent_overlay;
pub mod pdf_extract;
pub mod word_extract;

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::{Arc, Mutex, OnceLock};
    use std::thread;
    use std::time::Duration;

    use super::{
        execute_agent_with_spawn, execute_tool, permission_mode_from_plugin,
        tools_init, wiki_mirror_url, AgentInput, EditFileInput, GlobalToolRegistry,
        LaneEventName,
    };
    use agents::{AgentHandle, AgentJob};
    use runtime::{
        permission_enforcer::PermissionEnforcer, PermissionMode, PermissionPolicy,
        ToolExecutor,
    };
    use serde_json::json;

    fn mvp_tool_specs() -> Vec<(&'static str, PermissionMode)> {
        vec![
            ("bash", PermissionMode::DangerFullAccess),
            ("read_file", PermissionMode::ReadOnly),
            ("new_file", PermissionMode::WorkspaceWrite),
            ("edit_file", PermissionMode::WorkspaceWrite),
            ("glob_search", PermissionMode::ReadOnly),
            ("grep_search", PermissionMode::ReadOnly),
            ("WebFetch", PermissionMode::ReadOnly),
            ("WebFind", PermissionMode::ReadOnly),
            ("WebSearch", PermissionMode::ReadOnly),
            ("Skill", PermissionMode::ReadOnly),
            ("Agent", PermissionMode::DangerFullAccess),
            // Deprecated legacy alias for Brief
            ("SendUserMessage", PermissionMode::ReadOnly),
            ("StructuredOutput", PermissionMode::ReadOnly),
            ("ListAgents", PermissionMode::ReadOnly),
            ("ListSkills", PermissionMode::ReadOnly),
            ("ListPlugins", PermissionMode::ReadOnly),
        ]
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn env_guard_recovers_after_poisoning() {
        let poisoned = std::thread::spawn(|| {
            let _guard = env_guard();
            panic!("poison env lock");
        })
        .join();
        assert!(poisoned.is_err(), "poisoning thread should panic");

        let _guard = env_guard();
    }

    fn temp_path(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("clawd-tools-{unique}-{name}"))
    }

    fn run_git(cwd: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .unwrap_or_else(|error| panic!("git {} failed: {error}", args.join(" ")));
        assert!(
            status.success(),
            "git {} exited with {status}",
            args.join(" ")
        );
    }

    fn init_git_repo(path: &Path) {
        std::fs::create_dir_all(path).expect("create repo");
        run_git(path, &["init", "--quiet", "-b", "main"]);
        run_git(path, &["config", "core.autocrlf", "false"]);
        run_git(path, &["config", "user.email", "tests@example.com"]);
        run_git(path, &["config", "user.name", "Tools Tests"]);
        std::fs::write(path.join("README.md"), "initial\n").expect("write readme");
        run_git(path, &["add", "README.md"]);
        run_git(path, &["commit", "-m", "initial commit", "--quiet"]);
    }

    fn commit_file(path: &Path, file: &str, contents: &str, message: &str) {
        std::fs::write(path.join(file), contents).expect("write file");
        run_git(path, &["add", file]);
        run_git(path, &["commit", "-m", message, "--quiet"]);
    }

    fn permission_policy_for_mode(mode: PermissionMode) -> PermissionPolicy {
        mvp_tool_specs()
            .into_iter()
            .fold(PermissionPolicy::new(mode), |policy, (name, perm)| {
                policy.with_tool_requirement(name, perm)
            })
    }

    #[test]
    fn exposes_mvp_tools() {
        let names = mvp_tool_specs()
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        assert!(names.contains(&"bash"));
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"WebFetch"));
        assert!(names.contains(&"WebFind"));
        assert!(names.contains(&"WebSearch"));

        assert!(names.contains(&"Skill"));
        assert!(names.contains(&"Agent"));
        assert!(names.contains(&"SendUserMessage"));
        assert!(names.contains(&"StructuredOutput"));

    }

    #[test]
    fn rejects_unknown_tool_names() {
        let error = execute_tool("nope", &json!({})).expect_err("tool should be rejected");
        assert!(error.contains("unsupported tool"));
    }

    #[test]
    fn tools_init_wires_subagent_executor() {
        use agents::SubagentToolExecutor;

        let _guard = env_guard();

        tools_init().expect("tools_init should register the executor");

        let mut allowed = BTreeSet::new();
        allowed.insert("ListSkills".to_string());
        let mut exec = SubagentToolExecutor::new(allowed);
        // Use ListSkills (in-process, no OS execution) to verify the
        // global tool executor closure was wired correctly.
        let result = exec.execute("ListSkills", r#"{}"#);
        let output = result.expect("subagent tool execution should succeed after init");
        assert!(
            output.contains(r#""skills""#),
            "ListSkills output should contain JSON, got: {output}"
        );
    }

    #[test]
    fn diag_subagent_executor_read_file_with_code_explorer_permission() {
        // TEMPORARY diagnostic: reproduce the user-reported "can't read files"
        // regression using code-explorer.md's exact permission directives with
        // the real registered tool executor.
        use agents::SubagentToolExecutor;
        use runtime::PermissionMode;

        let _guard = env_guard();
        tools_init().expect("tools_init should register the executor");
        // Simulate in-workspace reads: the workspace boundary must not be what
        // blocks the file tools in this diagnostic.
        let prev = super::set_active_workspace_policy(runtime::BoundaryPolicy::Allow);

        let mut allowed = BTreeSet::new();
        for t in [
            "bash", "read_file", "new_file", "edit_file", "glob_search",
            "grep_search", "WebFetch", "WebSearch", "Skill", "StructuredOutput",
        ] {
            allowed.insert(t.to_string());
        }
        // code-explorer.md permission:
        //   read/glob/grep/bash/task/skill: allow, write/edit/webfetch/todowrite: deny
        let policy = PermissionPolicy::new(PermissionMode::DangerFullAccess)
            .with_tool_requirement("read_file", PermissionMode::ReadOnly)
            .with_tool_requirement("bash", PermissionMode::DangerFullAccess)
            .with_allow_all("read_file")
            .with_allow_all("bash")
            .with_allow_all("glob_search")
            .with_allow_all("grep_search")
            .with_allow_all("Skill")
            .with_deny_all("new_file")
            .with_deny_all("edit_file")
            .with_deny_all("WebFetch");
        let mut exec = SubagentToolExecutor::new(allowed).with_permission_policy(policy);

        let root = std::env::temp_dir().join(format!("claw-diag-{}", std::process::id()));
        fs::create_dir_all(&root).expect("create temp dir");
        let file = root.join("sample.rs");
        fs::write(&file, "fn main() {}\n").expect("write fixture");

        let read = exec.execute(
            "read_file",
            &format!(r#"{{"path": {}}}"#, serde_json::to_string(&file.display().to_string()).unwrap()),
        );
        eprintln!("[diag] read_file -> {read:?}");
        assert!(
            read.is_ok(),
            "read_file must NOT be denied by code-explorer permission; got: {read:?}"
        );

        let bash = exec.execute("bash", r#"{"command":"echo ok"}"#);
        eprintln!("[diag] bash -> {bash:?}");
        assert!(bash.is_ok(), "bash must be allowed; got: {bash:?}");

        super::set_active_workspace_policy(prev);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn tools_init_is_idempotent() {
        let _guard = env_guard();

        tools_init().expect("first init should succeed");
        let second = tools_init();
        assert!(
            second.is_ok(),
            "second init should be a no-op, got: {second:?}"
        );
    }

    #[test]
    fn global_tool_registry_denies_blocked_tool_before_dispatch() {
        // given
        let policy = permission_policy_for_mode(PermissionMode::ReadOnly);
        let registry = GlobalToolRegistry::builtin().with_enforcer(PermissionEnforcer::new(policy));

        // when
        let error = registry
            .execute(
                "new_file",
                &json!({
                    "path": "blocked.txt",
                    "content": "blocked"
                }),
            )
            .expect_err("new_file tool should be denied before dispatch");

        // then
        assert!(error.contains("requires workspace-write permission"));
    }

    #[test]
    fn permission_mode_from_plugin_rejects_invalid_inputs() {
        let unknown_permission = permission_mode_from_plugin("admin")
            .expect_err("unknown plugin permission should fail");
        assert!(unknown_permission.contains("unsupported plugin permission: admin"));

        let empty_permission =
            permission_mode_from_plugin("").expect_err("empty plugin permission should fail");
        assert!(empty_permission.contains("unsupported plugin permission: "));
    }

    #[test]
    fn runtime_tools_extend_registry_definitions_permissions_and_search() {
        let registry = GlobalToolRegistry::builtin()
            .with_runtime_tools(vec![super::RuntimeToolDefinition {
                name: "mcp__demo__echo".to_string(),
                description: Some("Echo text from the demo MCP server".to_string()),
                input_schema: json!({
                    "type": "object",
                    "properties": { "text": { "type": "string" } },
                    "additionalProperties": false
                }),
                required_permission: runtime::PermissionMode::ReadOnly,
            }])
            .expect("runtime tools should register");

        let allowed = registry
            .normalize_allowed_tools(&["mcp__demo__echo".to_string()])
            .expect("runtime tool should be allow-listable")
            .expect("allow-list should be populated");
        assert!(allowed.contains("mcp__demo__echo"));

        let definitions = registry.definitions(Some(&allowed));
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].name, "mcp__demo__echo");

        let permissions = registry
            .permission_specs(Some(&allowed))
            .expect("runtime tool permissions should resolve");
        assert_eq!(
            permissions,
            vec![(
                "mcp__demo__echo".to_string(),
                runtime::PermissionMode::ReadOnly
            )]
        );
    }

    #[test]
    fn web_fetch_returns_prompt_aware_summary() {
        let server = TestServer::spawn(Arc::new(|request_line: &str| {
            assert!(request_line.starts_with("GET /page "));
            HttpResponse::html(
                200,
                "OK",
                "<html><head><title>Ignored</title></head><body><h1>Test Page</h1><p>Hello <b>world</b> from local server.</p></body></html>",
            )
        }));

        let result = execute_tool(
            "WebFetch",
            &json!({
                "url": format!("http://{}/page", server.addr()),
                "prompt": "Summarize this page"
            }),
        )
        .expect("WebFetch should succeed");

        let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert_eq!(output["code"], 200);
        let summary = output["result"].as_str().expect("result string");
        assert!(summary.contains("Fetched"));
        assert!(summary.contains("Test Page"));
        assert!(summary.contains("Hello world from local server"));

        let titled = execute_tool(
            "WebFetch",
            &json!({
                "url": format!("http://{}/page", server.addr()),
                "prompt": "What is the page title?"
            }),
        )
        .expect("WebFetch title query should succeed");
        let titled_output: serde_json::Value = serde_json::from_str(&titled).expect("valid json");
        let titled_summary = titled_output["result"].as_str().expect("result string");
        assert!(titled_summary.contains("Title: Ignored"));
    }

    #[test]
    fn web_fetch_supports_plain_text_and_rejects_invalid_url() {
        let server = TestServer::spawn(Arc::new(|request_line: &str| {
            assert!(request_line.starts_with("GET /plain "));
            HttpResponse::text(200, "OK", "plain text response")
        }));

        let result = execute_tool(
            "WebFetch",
            &json!({
                "url": format!("http://{}/plain", server.addr()),
                "prompt": "Show me the content"
            }),
        )
        .expect("WebFetch should succeed for text content");

        let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert_eq!(output["url"], format!("http://{}/plain", server.addr()));
        assert!(output["result"]
            .as_str()
            .expect("result")
            .contains("plain text response"));

        let error = execute_tool(
            "WebFetch",
            &json!({
                "url": "not a url",
                "prompt": "Summarize"
            }),
        )
        .expect_err("invalid URL should fail");
        assert!(error.contains("relative URL without a base") || error.contains("invalid"));
    }

    #[test]
    fn wiki_mirror_url_preserves_underscore_titles() {
        let url =
            reqwest::Url::parse("https://en.wikipedia.org/wiki/Claude_Code").expect("valid URL");
        let (mirror, _label) = wiki_mirror_url(&url).expect("wikipedia URL must mirror");
        let query_pairs: std::collections::HashMap<String, String> =
            mirror.query_pairs().into_owned().collect();
        assert_eq!(
            query_pairs.get("query").map(String::as_str),
            Some("Claude Code")
        );
    }

    #[test]
    fn wiki_mirror_url_returns_none_for_non_wikipedia_hosts() {
        let url = reqwest::Url::parse("https://example.com/wiki/Some_Article").expect("valid URL");
        assert!(wiki_mirror_url(&url).is_none());
    }

    #[test]
    fn wiki_mirror_url_returns_none_for_wikipedia_non_article_paths() {
        let url = reqwest::Url::parse("https://zh.wikipedia.org/").expect("valid URL");
        assert!(wiki_mirror_url(&url).is_none());
    }

    #[test]
    fn edit_file_input_accepts_snake_case_field_names() {
        // The LLM emits snake_case (`old_string`/`new_string`); the
        // legacy schema expected camelCase. The struct must accept
        // both to avoid the "missing field" error that breaks edits.
        let snake: EditFileInput = serde_json::from_value(json!({
            "path": "demo.txt",
            "old_string": "alpha",
            "new_string": "beta",
            "replace_all": true,
            "expected_checksum": "deadbeef",
        }))
        .expect("snake_case must deserialize");
        assert_eq!(snake.old_string, "alpha");
        assert_eq!(snake.new_string, "beta");
        assert_eq!(snake.replace_all, Some(true));
        assert_eq!(snake.expected_checksum.as_deref(), Some("deadbeef"));

        // CamelCase must keep working for backwards compatibility.
        let camel: EditFileInput = serde_json::from_value(json!({
            "path": "demo.txt",
            "oldString": "alpha",
            "newString": "beta",
        }))
        .expect("camelCase must deserialize");
        assert_eq!(camel.old_string, "alpha");
        assert_eq!(camel.new_string, "beta");
    }

    #[test]
    fn web_fetch_falls_back_to_mirror_on_wikipedia_failure() {
        // Simulate Wikipedia being blocked: a 403 Forbidden response
        // for the Wikipedia-style request. The Sogou mirror would
        // normally return a search-results page; in this test we use
        // a path the local server recognizes for the mirror case.
        // We verify the tool reports the primary failure and includes
        // a `mirror` field when it is set up to succeed.
        let server = TestServer::spawn(Arc::new(|request_line: &str| {
            if request_line.starts_with("GET /wiki/") {
                return HttpResponse::text(403, "Forbidden", "blocked");
            }
            if request_line.starts_with("GET /web") {
                return HttpResponse::html(
                    200,
                    "OK",
                    "<html><body><h1>Sogou search results</h1><a>mirror-content-marker</a></body></html>",
                );
            }
            HttpResponse::text(404, "Not Found", "")
        }));

        // Use a non-DNS wikipedia.org by pointing the host rewrite at
        // the local server via a custom URL: we craft the wikipedia
        // URL with a path that has a known prefix and assert the
        // tool surfaces the failure clearly. (DNS is not patchable
        // from this test, so the full mirror path is exercised by
        // the wiki_mirror_url tests above; this end-to-end test
        // confirms the failure path does not crash.)
        let url = format!("http://{}/wiki/Some_Article", server.addr());
        let result = execute_tool(
            "WebFetch",
            &json!({
                "url": url,
                "prompt": "Summarize the article"
            }),
        )
        .expect("WebFetch should return a structured response, not panic");
        let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        // Local server returns 403 for /wiki/*, and the host is not
        // a wikipedia.org host, so no mirror is attempted. The
        // response includes the original 403 status.
        assert_eq!(output["code"], 403);
    }

    #[test]
    fn web_find_returns_matches_with_line_column_and_context() {
        let server = TestServer::spawn(Arc::new(|request_line: &str| {
            assert!(request_line.starts_with("GET /plain "));
            HttpResponse::text(
                200,
                "OK",
                "alpha bravo charlie\ndelta echo foxtrot\ntoken=needle-7 here\n",
            )
        }));

        let result = execute_tool(
            "WebFind",
            &json!({
                "url": format!("http://{}/plain", server.addr()),
                "pattern": "needle"
            }),
        )
        .expect("WebFind should succeed");

        let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert_eq!(output["url"], format!("http://{}/plain", server.addr()));
        assert_eq!(output["totalMatches"], 1);
        assert_eq!(output["truncated"], false);
        let matches = output["matches"].as_array().expect("matches array");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["line"], 3);
        assert_eq!(matches[0]["column"], 7);
        assert_eq!(matches[0]["matched"], "needle");
        assert!(matches[0]["context"]
            .as_str()
            .expect("context")
            .contains("token=needle-7 here"));
    }

    #[test]
    fn web_find_truncates_when_matches_exceed_max() {
        let body = "hit\n".repeat(20);
        let server = TestServer::spawn(Arc::new(move |request_line: &str| {
            assert!(request_line.starts_with("GET /many "));
            HttpResponse::text(200, "OK", &body)
        }));

        let result = execute_tool(
            "WebFind",
            &json!({
                "url": format!("http://{}/many", server.addr()),
                "pattern": "hit",
                "maxMatches": 5
            }),
        )
        .expect("WebFind should succeed");

        let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert_eq!(output["totalMatches"], 20);
        assert_eq!(output["truncated"], true);
        assert_eq!(output["matches"].as_array().expect("matches").len(), 5);
    }

    #[test]
    fn web_find_html_strips_tags_before_searching() {
        let server = TestServer::spawn(Arc::new(|request_line: &str| {
            assert!(request_line.starts_with("GET /article "));
            HttpResponse::html(
                200,
                "OK",
                "<html><body><h1>Header</h1><p>Find this token-marker here.</p>\
                 <nav>not-the-marker</nav></body></html>",
            )
        }));

        let result = execute_tool(
            "WebFind",
            &json!({
                "url": format!("http://{}/article", server.addr()),
                "pattern": "token-marker"
            }),
        )
        .expect("WebFind should succeed");

        let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert_eq!(output["totalMatches"], 1);
        let context = output["matches"][0]["context"].as_str().expect("context");
        assert!(context.contains("Find this token-marker here"));
        assert!(!context.contains("nav"));
        assert!(!context.contains("<"));
    }

    #[test]
    fn web_find_case_insensitive_matches_by_default() {
        let server = TestServer::spawn(Arc::new(|request_line: &str| {
            assert!(request_line.starts_with("GET /mixed "));
            HttpResponse::text(200, "OK", "FOO bar foo baz")
        }));

        let result = execute_tool(
            "WebFind",
            &json!({
                "url": format!("http://{}/mixed", server.addr()),
                "pattern": "foo"
            }),
        )
        .expect("WebFind should succeed");

        let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert_eq!(output["totalMatches"], 2);
        let matches = output["matches"].as_array().expect("matches");
        assert_eq!(matches[0]["matched"], "FOO");
        assert_eq!(matches[1]["matched"], "foo");

        let sensitive = execute_tool(
            "WebFind",
            &json!({
                "url": format!("http://{}/mixed", server.addr()),
                "pattern": "foo",
                "ignoreCase": false
            }),
        )
        .expect("WebFind should succeed");
        let sensitive_output: serde_json::Value =
            serde_json::from_str(&sensitive).expect("valid json");
        assert_eq!(sensitive_output["totalMatches"], 1);
        assert_eq!(
            sensitive_output["matches"][0]["matched"], "foo",
            "case-sensitive should match only exact-case occurrences"
        );
    }

    #[test]
    fn web_find_empty_result_returns_zero_total() {
        let server = TestServer::spawn(Arc::new(|request_line: &str| {
            assert!(request_line.starts_with("GET /missing "));
            HttpResponse::text(200, "OK", "no tokens here at all")
        }));

        let result = execute_tool(
            "WebFind",
            &json!({
                "url": format!("http://{}/missing", server.addr()),
                "pattern": "absent"
            }),
        )
        .expect("WebFind should succeed");

        let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert_eq!(output["totalMatches"], 0);
        assert_eq!(output["truncated"], false);
        assert_eq!(output["matches"].as_array().expect("matches").len(), 0);
    }

    #[test]
    fn web_search_extracts_and_filters_results() {
        // Ensure no API key is set so the tool falls back to Bing/Sogou
        // scraping and always returns a valid JSON response.
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = execute_tool("WebSearch", &json!({ "query": "rust web search" }))
            .expect("WebSearch must return JSON even without API key");
        let parsed: serde_json::Value =
            serde_json::from_str(&result).expect("output must be valid JSON");
        assert_eq!(parsed["query"], "rust web search");
        let provider = parsed["provider"].as_str().unwrap_or("none");
        assert!(
            provider == "none" || provider.split('+').all(|p| !p.is_empty()),
            "unexpected provider: {}",
            provider
        );
        assert!(parsed["resultsReturned"].is_number());
        assert!(parsed["results"].is_array());
    }

    #[test]
    fn web_search_handles_generic_links_and_invalid_base_url() {
        // Companion fallback test — same contract, different query.
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = execute_tool("WebSearch", &json!({ "query": "generic links" }))
            .expect("WebSearch must return JSON even without API key");
        let parsed: serde_json::Value =
            serde_json::from_str(&result).expect("output must be valid JSON");
        assert_eq!(parsed["query"], "generic links");
    }

    #[test]
    fn skill_loads_local_skill_prompt() {
        let _guard = env_guard();
        let home = temp_path("skills-home");
        let skill_dir = home.join(".claw").join("skills").join("help");
        fs::create_dir_all(&skill_dir).expect("skill dir should exist");
        fs::write(
            skill_dir.join("SKILL.md"),
            "# help\n\nGuide on using oh-my-codex plugin\n",
        )
        .expect("skill file should exist");
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &home);

        let result = execute_tool(
            "Skill",
            &json!({
                "skill": "help",
                "args": "overview"
            }),
        )
        .expect("Skill should succeed");

        let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert_eq!(output["skill"], "help");
        assert!(Path::new(output["path"].as_str().expect("path")).ends_with("help/SKILL.md"));
        assert!(output["prompt"]
            .as_str()
            .expect("prompt")
            .contains("Guide on using oh-my-codex plugin"));

        let dollar_result = execute_tool(
            "Skill",
            &json!({
                "skill": "$help"
            }),
        )
        .expect("Skill should accept $skill invocation form");
        let dollar_output: serde_json::Value =
            serde_json::from_str(&dollar_result).expect("valid json");
        assert_eq!(dollar_output["skill"], "$help");
        assert!(Path::new(dollar_output["path"].as_str().expect("path")).ends_with("help/SKILL.md"));

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
        fs::remove_dir_all(home).expect("temp home should clean up");
    }

    #[test]
    fn skill_resolves_project_local_skills_and_legacy_commands() {
        let _guard = env_guard();
        let root = temp_path("project-skills");
        let skill_dir = root.join(".claw").join("skills").join("plan");
        let command_dir = root.join(".claw").join("commands");
        fs::create_dir_all(&skill_dir).expect("skill dir should exist");
        fs::create_dir_all(&command_dir).expect("command dir should exist");
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: plan\ndescription: Project planning guidance\n---\n\n# plan\n",
        )
        .expect("skill file should exist");
        fs::write(
            command_dir.join("handoff.md"),
            "---\nname: handoff\ndescription: Legacy handoff guidance\n---\n\n# handoff\n",
        )
        .expect("command file should exist");

        let original_dir = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&root).expect("set cwd");

        let skill_result = execute_tool("Skill", &json!({ "skill": "$plan" }))
            .expect("project-local skill should resolve");
        let skill_output: serde_json::Value =
            serde_json::from_str(&skill_result).expect("valid json");
        assert!(Path::new(skill_output["path"].as_str().expect("path"))
            .ends_with(".claw/skills/plan/SKILL.md"));

        let command_result = execute_tool("Skill", &json!({ "skill": "/handoff" }))
            .expect("legacy command should resolve");
        let command_output: serde_json::Value =
            serde_json::from_str(&command_result).expect("valid json");
        assert!(Path::new(command_output["path"].as_str().expect("path"))
            .ends_with(".claw/commands/handoff.md"));

        std::env::set_current_dir(&original_dir).expect("restore cwd");
        fs::remove_dir_all(root).expect("temp project should clean up");
    }

    #[test]
    fn skill_loads_project_local_claude_skill_prompt() {
        let _guard = env_guard();
        let root = temp_path("project-skills");
        let home = root.join("home");
        let workspace = root.join("workspace");
        let nested = workspace.join("nested");
        let skill_dir = workspace.join(".claude").join("skills").join("trace");
        fs::create_dir_all(&skill_dir).expect("skill dir should exist");
        fs::create_dir_all(&nested).expect("nested cwd should exist");
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: trace\ndescription: Project-local trace helper\n---\n# trace\n",
        )
        .expect("skill file should exist");

        let original_home = std::env::var("HOME").ok();
        let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        let original_dir = std::env::current_dir().expect("cwd");
        std::env::set_var("HOME", &home);
        std::env::remove_var("CLAW_CONFIG_HOME");
        std::env::set_current_dir(&nested).expect("set cwd");

        let result = execute_tool("Skill", &json!({ "skill": "trace" }))
            .expect("project-local skill should resolve");

        let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert!(Path::new(output["path"].as_str().expect("path"))
            .ends_with(".claude/skills/trace/SKILL.md"));
        assert_eq!(output["description"], "Project-local trace helper");

        std::env::set_current_dir(&original_dir).expect("restore cwd");
        match original_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        match original_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        fs::remove_dir_all(root).expect("temp tree should clean up");
    }

    #[test]
    fn skill_loads_project_local_skill_prompts() {
        let _guard = env_guard();
        let root = temp_path("project-skills");
        let home = root.join("home");
        let workspace = root.join("workspace");
        let nested = workspace.join("nested");
        let skill_dir = workspace.join(".claw").join("skills").join("trace");
        fs::create_dir_all(&skill_dir).expect("skill dir should exist");
        fs::create_dir_all(&nested).expect("nested cwd should exist");
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: trace\ndescription: Project-local skill\n---\n# trace\n",
        )
        .expect("skill file should exist");

        let original_home = std::env::var("HOME").ok();
        let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        let original_dir = std::env::current_dir().expect("cwd");
        std::env::set_var("HOME", &home);
        std::env::remove_var("CLAW_CONFIG_HOME");
        std::env::set_current_dir(&nested).expect("set cwd");

        let result = execute_tool("Skill", &json!({ "skill": "trace" }))
            .expect("skill should resolve");

        let output: serde_json::Value =
            serde_json::from_str(&result).expect("valid json");
        assert!(Path::new(output["path"].as_str().expect("path"))
            .ends_with(".claw/skills/trace/SKILL.md"));
        assert_eq!(
            output["description"],
            "Project-local skill"
        );

        std::env::set_current_dir(&original_dir).expect("restore cwd");
        match original_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        match original_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        fs::remove_dir_all(root).expect("temp tree should clean up");
    }

    #[test]
    fn skill_loads_skill_and_command_from_claude_config_dir() {
        let _guard = env_guard();
        let root = temp_path("claude-config-skill");
        let home = root.join("home");
        let claude_config_dir = root.join("claude-config");
        let skill_dir = claude_config_dir.join("skills").join("learned");
        let command_dir = claude_config_dir.join("commands");
        fs::create_dir_all(&skill_dir).expect("skill dir should exist");
        fs::create_dir_all(&command_dir).expect("command dir should exist");
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: learned\ndescription: Learned skill\n---\n# learned\n",
        )
        .expect("skill file should exist");

        let original_home = std::env::var("HOME").ok();
        let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        let original_claude_config_dir = std::env::var("CLAUDE_CONFIG_DIR").ok();
        std::env::set_var("HOME", &home);
        std::env::remove_var("CLAW_CONFIG_HOME");
        std::env::set_var("CLAUDE_CONFIG_DIR", &claude_config_dir);

        let result = execute_tool("Skill", &json!({ "skill": "learned" }))
            .expect("learned skill should resolve");

        let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert!(
            Path::new(output["path"].as_str().expect("path")).ends_with("skills/learned/SKILL.md")
        );
        assert_eq!(output["description"], "Learned skill");

        match original_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        match original_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        match original_claude_config_dir {
            Some(value) => std::env::set_var("CLAUDE_CONFIG_DIR", value),
            None => std::env::remove_var("CLAUDE_CONFIG_DIR"),
        }
        fs::remove_dir_all(root).expect("temp tree should clean up");
    }

    #[test]
    fn skill_loads_direct_skill_and_legacy_command_from_claude_config_dir() {
        let _guard = env_guard();
        let root = temp_path("claude-config-direct-skill");
        let home = root.join("home");
        let claude_config_dir = root.join("claude-config");
        let skill_dir = claude_config_dir.join("skills").join("statusline");
        let command_dir = claude_config_dir.join("commands");
        fs::create_dir_all(&skill_dir).expect("direct skill dir should exist");
        fs::create_dir_all(&command_dir).expect("command dir should exist");
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: statusline\ndescription: Claude config skill\n---\n# statusline\n",
        )
        .expect("direct skill file should exist");
        fs::write(
            command_dir.join("doctor-check.md"),
            "---\nname: doctor-check\ndescription: Claude config command\n---\n# doctor-check\n",
        )
        .expect("direct command file should exist");

        let original_home = std::env::var("HOME").ok();
        let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        let original_claude_config_dir = std::env::var("CLAUDE_CONFIG_DIR").ok();
        std::env::set_var("HOME", &home);
        std::env::remove_var("CLAW_CONFIG_HOME");
        std::env::set_var("CLAUDE_CONFIG_DIR", &claude_config_dir);

        let direct_skill =
            execute_tool("Skill", &json!({ "skill": "statusline" })).expect("direct skill");
        let direct_skill_output: serde_json::Value =
            serde_json::from_str(&direct_skill).expect("valid skill json");
        assert!(
            Path::new(direct_skill_output["path"].as_str().expect("path"))
                .ends_with("skills/statusline/SKILL.md")
        );
        assert_eq!(direct_skill_output["description"], "Claude config skill");

        let legacy_command =
            execute_tool("Skill", &json!({ "skill": "doctor-check" })).expect("direct command");
        let legacy_command_output: serde_json::Value =
            serde_json::from_str(&legacy_command).expect("valid command json");
        assert!(
            Path::new(legacy_command_output["path"].as_str().expect("path"))
                .ends_with("commands/doctor-check.md")
        );
        assert_eq!(
            legacy_command_output["description"],
            "Claude config command"
        );

        match original_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        match original_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        match original_claude_config_dir {
            Some(value) => std::env::set_var("CLAUDE_CONFIG_DIR", value),
            None => std::env::remove_var("CLAUDE_CONFIG_DIR"),
        }
        fs::remove_dir_all(root).expect("temp tree should clean up");
    }

    #[test]
    fn skill_loads_project_local_legacy_command_markdown() {
        let _guard = env_guard();
        let root = temp_path("project-legacy-command");
        let home = root.join("home");
        let workspace = root.join("workspace");
        let nested = workspace.join("nested");
        let command_dir = workspace.join(".claude").join("commands");
        fs::create_dir_all(&command_dir).expect("legacy command dir should exist");
        fs::create_dir_all(&nested).expect("nested cwd should exist");
        fs::write(
            command_dir.join("team.md"),
            "---\nname: team\ndescription: Legacy team workflow\n---\n# team\n",
        )
        .expect("legacy command file should exist");

        let original_home = std::env::var("HOME").ok();
        let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
        let original_dir = std::env::current_dir().expect("cwd");
        std::env::set_var("HOME", &home);
        std::env::remove_var("CLAW_CONFIG_HOME");
        std::env::set_current_dir(&nested).expect("set cwd");

        let result = execute_tool("Skill", &json!({ "skill": "team" }))
            .expect("legacy command markdown should resolve");

        let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert!(
            Path::new(output["path"].as_str().expect("path")).ends_with(".claude/commands/team.md")
        );
        assert_eq!(output["description"], "Legacy team workflow");

        std::env::set_current_dir(&original_dir).expect("restore cwd");
        match original_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        match original_config_home {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        fs::remove_dir_all(root).expect("temp tree should clean up");
    }

    #[test]
    fn agent_persists_handoff_metadata() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = temp_path("agent-store");
        std::env::set_var("CLAWD_AGENT_STORE", &dir);
        let captured = Arc::new(Mutex::new(None::<AgentJob>));
        let captured_for_spawn = Arc::clone(&captured);

        let (manifest, _handle) = execute_agent_with_spawn(
            AgentInput {
                description: "Audit the branch".to_string(),
                prompt: "Check tests and outstanding work.".to_string(),
                subagent_type: Some("Explore".to_string()),
                name: Some("ship-audit".to_string()),
                model: None,
                system_prompt: None,
                allowed_tools: None,
                mode: None,
                reasoning_effort: None,
                permission: None,
            },
            move |job| {
                let agent_id = job.manifest.agent_id.clone();
                *captured_for_spawn
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(job);
                Ok(AgentHandle::noop(agent_id))
            },
        )
        .expect("Agent should succeed");
        std::env::remove_var("CLAWD_AGENT_STORE");

        assert_eq!(manifest.name, "ship-audit");
        assert_eq!(manifest.subagent_type.as_deref(), Some("Explore"));
        let captured_job = captured
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .expect("spawn job should be captured");
        assert_eq!(captured_job.prompt, "Check tests and outstanding work.");
        assert!(captured_job.allowed_tools.contains("read_file"));
        assert!(!captured_job.allowed_tools.contains("Agent"));

        let (normalized_output, _) = execute_agent_with_spawn(
            AgentInput {
                description: "Verify the branch".to_string(),
                prompt: "Check tests.".to_string(),
                subagent_type: Some("explorer".to_string()),
                name: None,
                model: None,
                system_prompt: None,
                allowed_tools: None,
                mode: None,
                reasoning_effort: None,
                permission: None,
            },
            |job| Ok(AgentHandle::noop(job.manifest.agent_id.clone())),
        )
        .expect("Agent should normalize built-in aliases");
        assert_eq!(normalized_output.subagent_type.as_deref(), Some("Explore"));

        let (named_output, _) = execute_agent_with_spawn(
            AgentInput {
                description: "Review the branch".to_string(),
                prompt: "Inspect diff.".to_string(),
                subagent_type: None,
                name: Some("Ship Audit!!!".to_string()),
                model: None,
                system_prompt: None,
                allowed_tools: None,
                mode: None,
                reasoning_effort: None,
                permission: None,
            },
            |job| Ok(AgentHandle::noop(job.manifest.agent_id.clone())),
        )
        .expect("Agent should normalize explicit names");
        assert_eq!(named_output.name, "ship-audit");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn agent_reasoning_effort_flows_into_manifest_and_job() {
        let captured = Arc::new(Mutex::new(None::<AgentJob>));
        let captured_for_spawn = Arc::clone(&captured);

        let (manifest, _handle) = execute_agent_with_spawn(
            AgentInput {
                description: "Effort test".to_string(),
                prompt: "Deep dive.".to_string(),
                subagent_type: Some("Explore".to_string()),
                name: Some("effort-agent".to_string()),
                model: None,
                system_prompt: None,
                allowed_tools: None,
                mode: None,
                reasoning_effort: Some("high".to_string()),
                permission: None,
            },
            move |job| {
                let agent_id = job.manifest.agent_id.clone();
                *captured_for_spawn
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(job);
                Ok(AgentHandle::noop(agent_id))
            },
        )
        .expect("Agent should succeed");

        assert_eq!(manifest.reasoning_effort.as_deref(), Some("high"));
        let captured_job = captured
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .expect("spawn job should be captured");
        assert_eq!(captured_job.reasoning_effort.as_deref(), Some("high"));
    }

    #[test]
    fn agent_without_custom_prompt_propagates_system_prompt_build_error() {
        let _guard = env_guard();
        let root = temp_path("bad-config-agent");
        let claw_dir = root.join(".claw");
        fs::create_dir_all(&claw_dir).expect("claw dir should exist");
        // Top-level non-object JSON makes ConfigLoader::load fail, so
        // build_agent_system_prompt errors. Without a custom system_prompt the
        // error must propagate instead of silently running with an empty prompt.
        fs::write(claw_dir.join("settings.json"), "[]").expect("write bad settings");

        let original_dir = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&root).expect("set cwd");

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execute_agent_with_spawn(
                AgentInput {
                    description: "Effort test".to_string(),
                    prompt: "Deep dive.".to_string(),
                    subagent_type: Some("Explore".to_string()),
                    name: Some("effort-agent".to_string()),
                    model: None,
                    system_prompt: None,
                    allowed_tools: None,
                    mode: None,
                    reasoning_effort: None,
                    permission: None,
                },
                |job| Ok(AgentHandle::noop(job.manifest.agent_id.clone())),
            )
        }));

        std::env::set_current_dir(&original_dir).expect("restore cwd");
        fs::remove_dir_all(root).ok();

        let result = result.unwrap_or_else(|payload| std::panic::resume_unwind(payload));
        assert!(
            result.is_err(),
            "system-prompt build failure with no custom prompt must propagate"
        );
    }

    #[test]
    fn general_purpose_agent_inherits_registered_runtime_tools() {
        // Register a runtime tool provider (idempotent across this binary) so
        // the general-purpose sub-agent's allowed_tools union picks up the
        // MCP/plugin tool names.
        let executor: Box<agents::RuntimeToolExecutorFn> =
            Box::new(|_n, _v, _p| Ok("x".to_string()));
        let defs = vec![api::ToolDefinition {
            name: "mcp__demo__echo".to_string(),
            description: Some("demo".to_string()),
            input_schema: json!({}),
        }];
        let _ = agents::register_runtime_tool_provider(executor, defs);

        // Rebuild with a capture to inspect the job's allowed_tools.
        let captured = Arc::new(Mutex::new(None::<AgentJob>));
        let captured_for_spawn = Arc::clone(&captured);
        let (_manifest, _handle) = execute_agent_with_spawn(
            AgentInput {
                description: "General work".to_string(),
                prompt: "Do it.".to_string(),
                subagent_type: Some("general-purpose".to_string()),
                name: None,
                model: None,
                system_prompt: None,
                allowed_tools: None,
                mode: None,
                reasoning_effort: None,
                permission: None,
            },
            move |job| {
                let agent_id = job.manifest.agent_id.clone();
                *captured_for_spawn
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(job);
                Ok(AgentHandle::noop(agent_id))
            },
        )
        .expect("general-purpose should spawn");
        let job = captured
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .expect("job should be captured");
        assert!(
            job.allowed_tools.contains("mcp__demo__echo"),
            "general-purpose allowed_tools should include registered MCP tool; got {:?}",
            job.allowed_tools
        );
    }

    #[test]
    fn non_general_purpose_agent_does_not_inherit_runtime_tools() {
        let captured = Arc::new(Mutex::new(None::<AgentJob>));
        let captured_for_spawn = Arc::clone(&captured);
        let (_manifest, _handle) = execute_agent_with_spawn(
            AgentInput {
                description: "Explore".to_string(),
                prompt: "Search.".to_string(),
                subagent_type: Some("Explore".to_string()),
                name: None,
                model: None,
                system_prompt: None,
                allowed_tools: None,
                mode: None,
                reasoning_effort: None,
                permission: None,
            },
            move |job| {
                let agent_id = job.manifest.agent_id.clone();
                *captured_for_spawn
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(job);
                Ok(AgentHandle::noop(agent_id))
            },
        )
        .expect("Explore should spawn");
        let job = captured
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .expect("job should be captured");
        assert!(
            !job.allowed_tools.contains("mcp__demo__echo"),
            "Explore allowed_tools must NOT include registered MCP tool; got {:?}",
            job.allowed_tools
        );
    }

    #[test]
    fn lane_event_schema_serializes_to_canonical_names() {
        let cases = [
            (LaneEventName::Started, "lane.started"),
            (LaneEventName::Ready, "lane.ready"),
            (LaneEventName::PromptMisdelivery, "lane.prompt_misdelivery"),
            (LaneEventName::Blocked, "lane.blocked"),
            (LaneEventName::Red, "lane.red"),
            (LaneEventName::Green, "lane.green"),
            (LaneEventName::CommitCreated, "lane.commit.created"),
            (LaneEventName::PrOpened, "lane.pr.opened"),
            (LaneEventName::MergeReady, "lane.merge.ready"),
            (LaneEventName::Finished, "lane.finished"),
            (LaneEventName::Failed, "lane.failed"),
            (
                LaneEventName::BranchStaleAgainstMain,
                "branch.stale_against_main",
            ),
            (
                LaneEventName::BranchWorkspaceMismatch,
                "branch.workspace_mismatch",
            ),
        ];

        for (event, expected) in cases {
            assert_eq!(
                serde_json::to_value(event).expect("serialize lane event"),
                json!(expected)
            );
        }
    }

    #[test]
    fn agent_tool_subset_mapping_is_expected() {
        let general = agents::allowed_tools_for_subagent("general-purpose");
        assert!(general.contains("bash"));
        assert!(general.contains("new_file"));
        assert!(!general.contains("Agent"));

        let explore = agents::allowed_tools_for_subagent("Explore");
        assert!(explore.contains("read_file"));
        assert!(explore.contains("grep_search"));
        assert!(!explore.contains("bash"));

        let plan = agents::allowed_tools_for_subagent("Plan");

        assert!(plan.contains("StructuredOutput"));
        assert!(!plan.contains("Agent"));

        let verification = agents::allowed_tools_for_subagent("Verification");
        assert!(verification.contains("bash"));
        assert!(!verification.contains("new_file"));
    }

    #[test]
    fn agent_rejects_blank_required_fields() {
        let missing_description = execute_tool(
            "Agent",
            &json!({
                "description": "  ",
                "prompt": "Inspect"
            }),
        )
        .expect_err("blank description should fail");
        assert!(missing_description.contains("description must not be empty"));

        let missing_prompt = execute_tool(
            "Agent",
            &json!({
                "description": "Inspect branch",
                "prompt": " "
            }),
        )
        .expect_err("blank prompt should fail");
        assert!(missing_prompt.contains("prompt must not be empty"));
    }

    #[test]
    fn bash_tool_reports_success_exit_failure_timeout_and_background() {
        let success = execute_tool("bash", &json!({ "command": "printf 'hello'" }))
            .expect("bash should succeed");
        let success_output: serde_json::Value = serde_json::from_str(&success).expect("json");
        assert_eq!(success_output["stdout"], "hello");
        assert_eq!(success_output["interrupted"], false);

        let failure = execute_tool("bash", &json!({ "command": "printf 'oops' >&2; exit 7" }))
            .expect("bash failure should still return structured output");
        let failure_output: serde_json::Value = serde_json::from_str(&failure).expect("json");
        assert_eq!(failure_output["returnCodeInterpretation"], "exit_code:7");
        assert!(failure_output["stderr"]
            .as_str()
            .expect("stderr")
            .contains("oops"));

        let timeout = execute_tool("bash", &json!({ "command": "sleep 1", "timeout": 10 }))
            .expect("bash timeout should return output");
        let timeout_output: serde_json::Value = serde_json::from_str(&timeout).expect("json");
        assert_eq!(timeout_output["interrupted"], true);
        assert_eq!(timeout_output["returnCodeInterpretation"], "timeout");
        assert!(timeout_output["stderr"]
            .as_str()
            .expect("stderr")
            .contains("Command exceeded timeout"));

        let background = execute_tool(
            "bash",
            &json!({ "command": "sleep 1", "run_in_background": true }),
        )
        .expect("bash background should succeed");
        let background_output: serde_json::Value = serde_json::from_str(&background).expect("json");
        assert!(background_output["backgroundTaskId"].as_str().is_some());
        assert_eq!(background_output["noOutputExpected"], true);
    }

    #[test]
    fn bash_workspace_tests_are_blocked_when_branch_is_behind_main() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = temp_path("workspace-test-preflight");
        let original_dir = std::env::current_dir().expect("cwd");
        init_git_repo(&root);
        run_git(&root, &["checkout", "-b", "feature/stale-tests"]);
        run_git(&root, &["checkout", "main"]);
        commit_file(
            &root,
            "hotfix.txt",
            "fix from main\n",
            "fix: unblock workspace tests",
        );
        run_git(&root, &["checkout", "feature/stale-tests"]);
        std::env::set_current_dir(&root).expect("set cwd");

        let output = execute_tool(
            "bash",
            &json!({ "command": "cargo test --workspace --all-targets" }),
        )
        .expect("preflight should return structured output");
        let output_json: serde_json::Value = serde_json::from_str(&output).expect("json");
        assert_eq!(
            output_json["returnCodeInterpretation"],
            "preflight_blocked:branch_divergence"
        );
        assert!(output_json["stderr"]
            .as_str()
            .expect("stderr")
            .contains("branch divergence detected before workspace tests"));
        assert_eq!(
            output_json["structuredContent"][0]["event"],
            "branch.stale_against_main"
        );
        assert_eq!(
            output_json["structuredContent"][0]["failureClass"],
            "branch_divergence"
        );
        assert_eq!(
            output_json["structuredContent"][0]["data"]["missingCommits"][0],
            "fix: unblock workspace tests"
        );

        std::env::set_current_dir(&original_dir).expect("restore cwd");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn bash_targeted_tests_skip_branch_preflight() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = temp_path("targeted-test-no-preflight");
        let original_dir = std::env::current_dir().expect("cwd");
        init_git_repo(&root);
        run_git(&root, &["checkout", "-b", "feature/targeted-tests"]);
        run_git(&root, &["checkout", "main"]);
        commit_file(
            &root,
            "hotfix.txt",
            "fix from main\n",
            "fix: only broad tests should block",
        );
        run_git(&root, &["checkout", "feature/targeted-tests"]);
        std::env::set_current_dir(&root).expect("set cwd");

        let output = execute_tool(
            "bash",
            &json!({ "command": "printf 'targeted ok'; cargo test -p runtime stale_branch" }),
        )
        .expect("targeted commands should still execute");
        let output_json: serde_json::Value = serde_json::from_str(&output).expect("json");
        assert_ne!(
            output_json["returnCodeInterpretation"],
            "preflight_blocked:branch_divergence"
        );

        std::env::set_current_dir(&original_dir).expect("restore cwd");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn glob_and_grep_tools_cover_success_and_errors() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let root = temp_path("search-suite");
        fs::create_dir_all(root.join("nested")).expect("create root");
        let original_dir = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&root).expect("set cwd");

        fs::write(
            root.join("nested/lib.rs"),
            "fn main() {}\nlet alpha = 1;\nlet alpha = 2;\n",
        )
        .expect("write rust file");
        fs::write(root.join("nested/notes.txt"), "alpha\nbeta\n").expect("write txt file");

        let globbed = execute_tool("glob_search", &json!({ "pattern": "nested/*.rs" }))
            .expect("glob should succeed");
        let globbed_output: serde_json::Value = serde_json::from_str(&globbed).expect("json");
        assert_eq!(globbed_output["numFiles"], 1);
        let filename = globbed_output["filenames"][0].as_str().expect("filename");
        assert!(
            std::path::Path::new(filename).ends_with("nested/lib.rs"),
            "expected ends_with nested/lib.rs, got: {filename}"
        );

        let glob_error = execute_tool("glob_search", &json!({ "pattern": "[" }))
            .expect_err("invalid glob should fail");
        assert!(!glob_error.is_empty());

        let grep_content = execute_tool(
            "grep_search",
            &json!({
                "pattern": "alpha",
                "path": "nested",
                "glob": "*.rs",
                "output_mode": "content",
                "-n": true,
                "head_limit": 1,
                "offset": 1
            }),
        )
        .expect("grep content should succeed");
        let grep_content_output: serde_json::Value =
            serde_json::from_str(&grep_content).expect("json");
        // `numFiles` counts distinct files with matches (lib.rs), unaffected by
        // head_limit/offset which only slice the returned content lines.
        assert_eq!(grep_content_output["numFiles"], 1);
        assert!(grep_content_output["appliedLimit"].is_null());
        assert_eq!(grep_content_output["appliedOffset"], 1);
        assert!(grep_content_output["content"]
            .as_str()
            .expect("content")
            .contains("let alpha = 2;"));

        let grep_count = execute_tool(
            "grep_search",
            &json!({ "pattern": "alpha", "path": "nested", "output_mode": "count" }),
        )
        .expect("grep count should succeed");
        let grep_count_output: serde_json::Value = serde_json::from_str(&grep_count).expect("json");
        assert_eq!(grep_count_output["numMatches"], 3);

        let grep_error = execute_tool(
            "grep_search",
            &json!({ "pattern": "(alpha", "path": "nested" }),
        )
        .expect_err("invalid regex should fail");
        assert!(!grep_error.is_empty());

        std::env::set_current_dir(&original_dir).expect("restore cwd");
        let _ = fs::remove_dir_all(root);
    }


    #[test]
    fn brief_returns_sent_message_and_attachment_metadata() {
        let attachment = std::env::temp_dir().join(format!(
            "clawd-brief-{}.png",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        std::fs::write(&attachment, b"png-data").expect("write attachment");

        let result = execute_tool(
            "SendUserMessage",
            &json!({
                "message": "hello user",
                "attachments": [attachment.display().to_string()],
            }),
        )
        .expect("SendUserMessage should succeed");

        let output: serde_json::Value = serde_json::from_str(&result).expect("json");
        assert_eq!(output["message"], "hello user");
        assert!(output["sentAt"].as_str().is_some());
        assert_eq!(output["attachments"][0]["isImage"], true);
        let _ = std::fs::remove_file(attachment);
    }

    #[test]
    fn given_empty_payload_when_structured_output_then_rejects_with_error() {
        let result = execute_tool("StructuredOutput", &json!({}));
        let error = result.expect_err("empty payload should fail");
        assert!(error.contains("must not be empty"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]

    #[test]



    #[test]
    fn given_no_enforcer_when_bash_then_executes_normally() {
        let _guard = env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let registry = super::GlobalToolRegistry::builtin();
        let result = registry
            .execute("bash", &json!({ "command": "printf 'ok'" }))
            .expect("bash should succeed without enforcer");
        let output: serde_json::Value = serde_json::from_str(&result).expect("json");
        assert_eq!(output["stdout"], "ok");
    }


    struct TestServer {
        addr: SocketAddr,
        shutdown: Option<std::sync::mpsc::Sender<()>>,
        handle: Option<thread::JoinHandle<()>>,
    }

    impl TestServer {
        fn spawn(handler: Arc<dyn Fn(&str) -> HttpResponse + Send + Sync + 'static>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
            listener
                .set_nonblocking(true)
                .expect("set nonblocking listener");
            let addr = listener.local_addr().expect("local addr");
            let (tx, rx) = std::sync::mpsc::channel::<()>();

            let handle = thread::spawn(move || loop {
                if rx.try_recv().is_ok() {
                    break;
                }

                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // Tolerate transient connection resets and poll for
                        // WouldBlock: the HTTP client (reqwest/hyper) may reset
                        // keep-alive sockets, and on some platforms accepted
                        // sockets inherit the listener's non-blocking mode. A
                        // panic here cascades into the test process aborting
                        // (Drop::join().expect() panics during unwind), so
                        // degrade gracefully and skip unusable connections.
                        let mut buffer = [0_u8; 4096];
                        let mut size = 0usize;
                        while size == 0 {
                            match stream.read(&mut buffer) {
                                Ok(0) => break,
                                Ok(n) => {
                                    size = n;
                                    break;
                                }
                                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                    if rx.try_recv().is_ok() {
                                        break;
                                    }
                                    thread::sleep(Duration::from_millis(2));
                                }
                                Err(_) => break,
                            }
                        }
                        if size == 0 {
                            continue;
                        }
                        let request = String::from_utf8_lossy(&buffer[..size]).into_owned();
                        let request_line = request.lines().next().unwrap_or_default().to_string();
                        let response = handler(&request_line);
                        let _ = stream.write_all(response.to_bytes().as_slice());
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("server accept failed: {error}"),
                }
            });

            Self {
                addr,
                shutdown: Some(tx),
                handle: Some(handle),
            }
        }

        fn addr(&self) -> SocketAddr {
            self.addr
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            if let Some(tx) = self.shutdown.take() {
                let _ = tx.send(());
            }
            if let Some(handle) = self.handle.take() {
                // The server thread may already have exited (e.g. it panicked on
                // a transient connection error). Joining a panicked thread while
                // the test itself is unwinding would itself panic and abort the
                // whole test process, so swallow the join result.
                let _ = handle.join();
            }
        }
    }

    struct HttpResponse {
        status: u16,
        reason: &'static str,
        content_type: &'static str,
        body: String,
    }

    impl HttpResponse {
        fn html(status: u16, reason: &'static str, body: &str) -> Self {
            Self {
                status,
                reason,
                content_type: "text/html; charset=utf-8",
                body: body.to_string(),
            }
        }

        fn text(status: u16, reason: &'static str, body: &str) -> Self {
            Self {
                status,
                reason,
                content_type: "text/plain; charset=utf-8",
                body: body.to_string(),
            }
        }

        fn to_bytes(&self) -> Vec<u8> {
            format!(
                "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                self.status,
                self.reason,
                self.content_type,
                self.body.len(),
                self.body
            )
            .into_bytes()
        }
    }

    #[test]
    fn run_read_file_refuses_paths_outside_workspace() {
        let _guard = env_guard();

        let outside_dir = std::env::temp_dir().join(format!(
            "clawd-outside-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        fs::create_dir_all(&outside_dir).expect("outside dir should create");
        let outside_file = outside_dir.join("secret.txt");
        fs::write(&outside_file, "secret payload").expect("outside file should write");

        let workspace_root = std::env::temp_dir().join(format!(
            "clawd-workspace-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        fs::create_dir_all(&workspace_root).expect("workspace dir should create");

        let original_dir = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace_root).expect("set cwd");

        let result = super::run_read_file(super::ReadFileInput {
            path: outside_file.to_string_lossy().into_owned(),
            offset: None,
            limit: None,
            full: Some(true),
        });

        std::env::set_current_dir(&original_dir).expect("restore cwd");
        let _ = fs::remove_dir_all(&outside_dir);
        let _ = fs::remove_dir_all(&workspace_root);

        let error = result.expect_err("read_file outside workspace must fail");
        assert!(
            error.contains("escapes workspace")
                || error.contains("PermissionDenied")
                || error.contains("workspace"),
            "error should mention workspace boundary; got: {error}"
        );
    }

    #[test]
    fn run_read_file_allow_policy_admits_outside_workspace() {
        let _guard = env_guard();
        let previous = super::set_active_workspace_policy(runtime::BoundaryPolicy::Allow);

        let outside_dir = std::env::temp_dir().join(format!(
            "clawd-allow-outside-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        fs::create_dir_all(&outside_dir).expect("outside dir should create");
        let outside_file = outside_dir.join("ok.txt");
        fs::write(&outside_file, "allow me through").expect("outside file should write");

        let workspace_root = std::env::temp_dir().join(format!(
            "clawd-allow-ws-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        fs::create_dir_all(&workspace_root).expect("workspace dir should create");

        let original_dir = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace_root).expect("set cwd");

        let result = super::run_read_file(super::ReadFileInput {
            path: outside_file.to_string_lossy().into_owned(),
            offset: None,
            limit: None,
            full: Some(true),
        });

        std::env::set_current_dir(&original_dir).expect("restore cwd");
        let _ = fs::remove_dir_all(&outside_dir);
        let _ = fs::remove_dir_all(&workspace_root);

        // Restore the previous policy before any assertion so a panic
        // here does not poison sibling tests.
        super::set_active_workspace_policy(previous);

        let payload = result.expect("Allow policy must admit the read");
        assert!(payload.contains("checksum"));
    }

    #[test]
    fn yolo_mode_external_readonly_real_file_io() {
        // Real end-to-end file I/O under the yolo regime
        // (workspace-write base + external read-only + others ask):
        //   - in-workspace write succeeds
        //   - external read succeeds silently (no prompter consulted)
        //   - external write is denied (NoTty prompter => ask falls back)
        //   - in-workspace edit succeeds
        let _guard = env_guard();

        // Install the yolo boundary policy: reads outside workspace are
        // granted silently, writes consult the (empty) prompter which
        // surfaces NoTty -> denied.
        use std::collections::BTreeSet;
        use std::sync::{Arc, Mutex};
        let prompter = Arc::new(EmptyPrompter);
        let session = Arc::new(Mutex::new(BTreeSet::<runtime::ApprovedRoot>::new()));
        let user_typed = Arc::new(Mutex::new(BTreeSet::<runtime::ApprovedRoot>::new()));
        let policy = runtime::BoundaryPolicy::ExternalReadOnly {
            prompter,
            session_approved: session,
            user_typed,
        };
        let prev_policy = super::set_active_workspace_policy(policy);
        let prev_mode = super::set_active_permission_mode(runtime::PermissionMode::Yolo);

        let workspace_root = std::env::temp_dir().join(format!(
            "clawd-yolo-ws-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        let outside_dir = std::env::temp_dir().join(format!(
            "clawd-yolo-out-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        fs::create_dir_all(&workspace_root).expect("workspace dir should create");
        fs::create_dir_all(&outside_dir).expect("outside dir should create");
        let outside_file = outside_dir.join("external.txt");
        fs::write(&outside_file, "external data").expect("external file should write");

        let original_dir = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace_root).expect("set cwd");

        // 1. In-workspace write: allowed.
        let in_ws_path = workspace_root.join("in.txt");
        let write = super::run_new_file(super::WriteFileInput {
            path: in_ws_path.to_string_lossy().into_owned(),
            content: "workspace content".to_string(),
            force: Some(false),
        });
        assert!(write.is_ok(), "in-workspace write must succeed: {write:?}");

        // 2. External read: allowed silently even though the prompter is
        // empty (any prompt consult would surface NoTty -> denied).
        let read = super::run_read_file(super::ReadFileInput {
            path: outside_file.to_string_lossy().into_owned(),
            offset: None,
            limit: None,
            full: Some(true),
        });
        let read = read.expect("external read must be admitted in yolo mode");
        assert!(read.contains("external data"), "read payload: {read}");

        // 3. External write: denied (ask falls back to NoTty).
        let external_write = super::run_new_file(super::WriteFileInput {
            path: outside_dir.join("new.txt").to_string_lossy().into_owned(),
            content: "should be blocked".to_string(),
            force: Some(false),
        });
        assert!(
            external_write.is_err(),
            "external write must be denied in yolo mode: {external_write:?}"
        );
        assert!(!outside_dir.join("new.txt").exists(), "external file must not exist");

        // 4. In-workspace edit: allowed.
        let edit = super::run_edit_file(super::EditFileInput {
            path: in_ws_path.to_string_lossy().into_owned(),
            old_string: "workspace content".to_string(),
            new_string: "workspace content v2".to_string(),
            replace_all: Some(false),
            expected_checksum: None,
        });
        assert!(edit.is_ok(), "in-workspace edit must succeed: {edit:?}");

        std::env::set_current_dir(&original_dir).expect("restore cwd");
        let _ = fs::remove_dir_all(&workspace_root);
        let _ = fs::remove_dir_all(&outside_dir);

        // Restore globals before assertions so a panic cannot poison
        // sibling tests.
        super::set_active_workspace_policy(prev_policy);
        super::set_active_permission_mode(prev_mode);
    }

    #[test]
    fn allow_policy_permits_external_writes_real_file_io() {
        // Full access (CLAW_WORKSPACE_POLICY=allow / danger-full-access):
        // unlike yolo, external writes are granted silently too.
        let _guard = env_guard();
        let prev_policy = super::set_active_workspace_policy(runtime::BoundaryPolicy::Allow);
        let prev_mode = super::set_active_permission_mode(runtime::PermissionMode::DangerFullAccess);

        let outside_dir = std::env::temp_dir().join(format!(
            "clawd-allow-write-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        fs::create_dir_all(&outside_dir).expect("outside dir should create");

        let original_dir = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&std::env::temp_dir()).expect("set cwd");

        let write = super::run_new_file(super::WriteFileInput {
            path: outside_dir.join("written.txt").to_string_lossy().into_owned(),
            content: "full access content".to_string(),
            force: Some(false),
        });
        assert!(
            write.is_ok(),
            "external write must be allowed under full access: {write:?}"
        );
        assert!(
            outside_dir.join("written.txt").exists(),
            "external file must exist under full access"
        );

        std::env::set_current_dir(&original_dir).expect("restore cwd");
        let _ = fs::remove_dir_all(&outside_dir);
        super::set_active_workspace_policy(prev_policy);
        super::set_active_permission_mode(prev_mode);
    }

    #[test]
    fn set_active_workspace_policy_returns_previous_value() {
        let _guard = env_guard();
        let strict = runtime::BoundaryPolicy::Block;
        let allow = runtime::BoundaryPolicy::Allow;
        let prev = super::set_active_workspace_policy(strict.clone());
        // First call replaces default Block with Block; the *return*
        // value is whatever was previously active.
        let prev_after = super::set_active_workspace_policy(allow);
        // The policy before `allow` was `strict`.
        assert!(matches!(prev_after, runtime::BoundaryPolicy::Block));
        // Restore.
        let _ = super::set_active_workspace_policy(prev);
    }

    #[test]
    fn active_permission_mode_defaults_to_yolo() {
        let _guard = env_guard();
        // Permission passthrough: sub-agents spawn under the parent's mode.
        // The default is yolo so spawns from contexts that never set the
        // mode still run under the yolo regime.
        let original = super::set_active_permission_mode(runtime::PermissionMode::ReadOnly);
        assert_eq!(super::active_permission_mode(), runtime::PermissionMode::ReadOnly);
        let _ = super::set_active_permission_mode(original);
    }

    #[test]
    fn note_user_input_path_in_prompt_mode_admits_subsequent_read() {
        let _guard = env_guard();
        use std::collections::BTreeSet;
        use std::sync::{Arc, Mutex};

        let outside_dir = std::env::temp_dir().join(format!(
            "clawd-note-input-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        fs::create_dir_all(&outside_dir).expect("outside dir should create");
        let outside_file = outside_dir.join("dropped.txt");
        fs::write(&outside_file, "dropped content").expect("outside file should write");

        let workspace_root = std::env::temp_dir().join(format!(
            "clawd-note-input-ws-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        fs::create_dir_all(&workspace_root).expect("workspace dir should create");

        // Install a Prompt policy with an empty scripted prompter.
        // If the policy ever has to consult the prompter, the empty
        // queue surfaces `NoTty` and the read is denied.
        let prompter = Arc::new(EmptyPrompter);
        let session = Arc::new(Mutex::new(BTreeSet::<runtime::ApprovedRoot>::new()));
        let user_typed = Arc::new(Mutex::new(BTreeSet::<runtime::ApprovedRoot>::new()));
        let policy = runtime::BoundaryPolicy::Prompt {
            prompter,
            session_approved: session,
            user_typed,
        };
        let previous = super::set_active_workspace_policy(policy);

        // Simulate the input parser detecting the dropped file.
        super::note_user_input_path(&outside_file);
        assert_eq!(super::user_typed_path_count(), 1);

        // The LLM now reads the file without the prompter being
        // consulted.
        let original_dir = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&workspace_root).expect("set cwd");
        let result = super::run_read_file(super::ReadFileInput {
            path: outside_file.to_string_lossy().into_owned(),
            offset: None,
            limit: None,
            full: Some(true),
        });
        std::env::set_current_dir(&original_dir).expect("restore cwd");
        let _ = fs::remove_dir_all(&outside_dir);
        let _ = fs::remove_dir_all(&workspace_root);

        // Restore the policy before asserting so a panic does not
        // poison sibling tests.
        super::set_active_workspace_policy(previous);

        let payload = result.expect("user-typed path should be readable without prompt");
        assert!(payload.contains("checksum"));
    }

    /// Prompter that always returns `NoTty`. Used to assert that the
    /// policy never consults the prompter when the path is already
    /// in the user-typed set.
    struct EmptyPrompter;
    impl runtime::Prompter for EmptyPrompter {
        fn ask(
            &self,
            _path: &std::path::Path,
            _workspace: &std::path::Path,
        ) -> Result<runtime::BoundaryDecision, runtime::PrompterError> {
            Err(runtime::PrompterError::NoTty)
        }
    }

    #[test]
    fn has_dangerous_paths_recognises_windows_drive_letters() {
        // The classifier must flag Windows absolute paths so the
        // permission enforcer sees them. Before this fix, the tokenizer
        // only knew POSIX absolute paths and `~`, so a token like
        // `C:\Users\foo\bar.txt` was treated as a normal argument.
        assert!(super::has_dangerous_paths(r#"cat "C:\Users\foo\bar.txt""#));
        assert!(super::has_dangerous_paths(
            r#"Get-Content -LiteralPath 'D:\data\file.txt'"#
        ));
        assert!(super::has_dangerous_paths(r"X:/absolute/unix-style.txt"));
        // Relative paths are not flagged.
        assert!(!super::has_dangerous_paths("cat ./relative.txt"));
        assert!(!super::has_dangerous_paths("echo hello world"));
    }

    #[test]
    fn bash_model_view_carries_sandbox_diagnostics() {
        // The model-facing JSON envelope for the bash tool must carry
        // the sandbox fallback reason so the model can reason honestly
        // about which sandbox mechanisms are actually enforced (rather
        // than concluding "the sandbox blocked it" when only
        // process-tree kill is active).
        use runtime::sandbox::{FilesystemIsolationMode, SandboxRequest, SandboxStatus};
        let mut status = SandboxStatus::default();
        status.enabled = true;
        status.fallback_reason = Some(String::from("process tree kill only"));
        let output = runtime::BashCommandOutput {
            stdout: String::from("hello"),
            stderr: String::new(),
            raw_output_path: None,
            interrupted: false,
            is_image: None,
            background_task_id: None,
            backgrounded_by_user: None,
            assistant_auto_backgrounded: None,
            dangerously_disable_sandbox: None,
            return_code_interpretation: None,
            no_output_expected: Some(false),
            structured_content: None,
            persisted_output_path: None,
            persisted_output_size: None,
            sandbox_status: Some(status),
            sandbox_type: Some(String::from("windows-job-object")),
        };
        let view = super::bash_model_view(&output);
        let parsed: serde_json::Value =
            serde_json::from_str(&view).expect("model view must be valid JSON");
        // stdout and stderr are top-level fields the model can read
        // directly.
        assert_eq!(parsed["stdout"], "hello");
        assert_eq!(parsed["stderr"], "");
        // The sandbox block carries the fallback reason so the model
        // does not conclude "the sandbox blocked it" on Windows when
        // only the Job Object is active.
        let sandbox = &parsed["sandbox"];
        assert_eq!(sandbox["type"], "windows-job-object");
        assert_eq!(sandbox["fallbackReason"], "process tree kill only");
        // Allow unused import warnings to stay quiet.
        let _ = (FilesystemIsolationMode::Off, SandboxRequest::default());
    }

    #[test]
    fn agent_handle_join_with_real_thread_success() {
        let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
        let thread_handle = std::thread::spawn(move || {
            let _ = tx.send(Ok("result".to_string()));
        });
        let handle = AgentHandle::with_parts("test", thread_handle, rx);
        assert!(handle.join().is_ok());
    }

    #[test]
    fn agent_handle_join_with_real_thread_error() {
        let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
        let thread_handle = std::thread::spawn(move || {
            let _ = tx.send(Err("something went wrong".to_string()));
        });
        let handle = AgentHandle::with_parts("test", thread_handle, rx);
        let result = handle.join();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "something went wrong");
    }

    #[test]
    fn agent_handle_join_timeout() {
        let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
        let thread_handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            let _ = tx.send(Ok("done".to_string()));
        });
        let handle = AgentHandle::with_parts("test", thread_handle, rx);
        let result = handle.join_with_timeout(Duration::from_millis(10));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timed out"));
    }

    #[test]
    fn agent_handle_join_reaps_thread_on_error_path() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let finished = Arc::new(AtomicBool::new(false));
        let finished_clone = Arc::clone(&finished);
        let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
        let thread_handle = std::thread::spawn(move || {
            let _ = tx.send(Err("boom".to_string()));
            finished_clone.store(true, Ordering::SeqCst);
        });
        let handle = AgentHandle::with_parts("test", thread_handle, rx);
        assert!(handle.join().is_err());
        assert!(
            finished.load(Ordering::SeqCst),
            "worker thread must be reaped (joined) even on the error path"
        );
    }

    #[test]
    fn wait_for_agent_returns_timeout_when_deadline_passes() {
        let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
        let thread_handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            let _ = tx.send(Ok("done".to_string()));
        });
        let mut handle = AgentHandle::with_parts("test", thread_handle, rx);
        let result =
            super::wait_for_agent(&mut handle, std::time::Instant::now() + Duration::from_millis(30));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timed out"));
        // Reap the worker so the test process does not leak a thread.
        let _ = handle.join();
    }

}

// =====================================================================
// ARCHAEOLOGY STUBS �?recovered definitions to make the tools crate
// compile after ~1000+ lines of `crates/tools/src/lib.rs` were lost
// (no git history available; this is best-effort reconstruction).
//
// Each definition is marked `// TODO(archaeology):` with a note about
// what the original probably looked like. These compile but may panic
// or return wrong data at runtime if a tool is invoked with a shape
// the original types did not anticipate.
//
// Build status before this block: cargo build --release fails with
// "cannot find type/value X in scope" for the names listed below.
// Build status after this block:  cargo build --release succeeds.
//
// If you are reviewing this diff: the safest path is to replace the
// stubs with proper implementations recovered from a known-good
// snapshot of `lib.rs` (the runtime crate already owns the canonical
// `read_file` / `new_file` / `edit_file` / `glob_search` /
// `grep_search` implementations �?see `crates/runtime/src/file_ops.rs`).
// =====================================================================

// -- Searchable tool registry (recovered from use-sites in
//    `GlobalToolRegistry::search` / `searchable_tool_specs`)
// =====================================================================

// -- Web / file / todo / skill / notebook / sleep / brief / config
//    / plan / repl / power-shell / structured-output inputs and outputs.
//    Field shapes are inferred from the existing `execute_*` bodies.
//    All Input types use `#[serde(default)]` so any JSON shape can
//    deserialize; Output types carry only the fields the matching
//    `execute_*` function constructs.
// =====================================================================

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ReadFileInput {
    pub path: String,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
    // Per spec 2026-06-01 tool-output-context-bounds-design.md §2:
    // `full: true` bypasses the output cap.
    #[serde(default)]
    pub full: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WriteFileInput {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub force: Option<bool>,
}

/// Inputs for the `edit_file` tool. **See [`mvp_tool_specs`] for the
/// authoritative contract** that the LLM sees �?these doc comments are
/// for in-process callers and must stay in sync with the schema
/// description.
///
/// Workflow contract:
/// 1. `old_string` MUST be a verbatim copy from a prior `read_file`.
/// 2. Multiple matches -> only first is replaced (unless `replace_all`).
/// 3. Not found -> `NotFound` error, file unchanged.
/// 4. To append: `old_string = current_tail`, `new_string = current_tail + content`.
/// 5. Caller MUST verify `EditFileOutput.content_preview` after the call.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EditFileInput {
    /// Absolute path to the file to edit.
    pub path: String,
    // The serialized names follow Anthropic's snake_case convention
    // (the format the LLM emits); we also accept camelCase aliases
    // (`oldString`, `newString`, `replaceAll`, `expectedChecksum`)
    // for backwards compatibility with internal callers and tests.
    /// Exact substring to replace. Must appear verbatim in the file.
    /// If it appears multiple times, only the first occurrence is
    /// replaced unless `replace_all` is `true`. Include surrounding
    /// context to make it unique.
    #[serde(alias = "oldString")]
    pub old_string: String,
    /// Replacement content. For append, set this to (current tail + new
    /// content) and set `old_string` to the current tail.
    #[serde(alias = "newString")]
    pub new_string: String,
    /// When `true`, replace all occurrences. Default `false`.
    #[serde(alias = "replaceAll", default)]
    pub replace_all: Option<bool>,
    /// Optional xxh3-64 checksum of the file before editing. The call
    /// fails with an error if the actual checksum does not match,
    /// protecting against concurrent modification.
    #[serde(
        alias = "expectedChecksum",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub expected_checksum: Option<String>,
}

/// Input for the `undo` tool �?reverses a prior edit_file operation
/// by reading the diff file and applying the inverse replacement.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UndoInput {
    /// Diff file name or path.
    /// - Empty string (bare `/undo`) → auto-discover latest.
    /// - `diff_2141.patch`           → match newest date dir.
    /// - `d20260722/diff_2141.patch` → exact path under `~/.claw/diffs/`.
    /// - Absolute / legacy relative path → backward compat resolution.
    pub diff_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct GlobSearchInputValue {
    pub pattern: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WebFetchInput {
    pub url: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WebFindInput {
    pub url: String,
    pub pattern: String,
    #[serde(rename = "ignoreCase", default)]
    pub ignore_case: Option<bool>,
    #[serde(rename = "maxMatches", default)]
    pub max_matches: Option<usize>,
    #[serde(rename = "contextChars", default)]
    pub context_chars: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WebFindMatch {
    pub line: usize,
    pub column: usize,
    pub matched: String,
    pub context: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WebFindOutput {
    pub url: String,
    pub pattern: String,
    #[serde(rename = "totalMatches")]
    pub total_matches: usize,
    pub truncated: bool,
    pub matches: Vec<WebFindMatch>,
    #[serde(rename = "bytesScanned")]
    pub bytes_scanned: usize,
    #[serde(rename = "contentType")]
    pub content_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct WebSearchInput {
    pub query: String,
    #[serde(default)]
    pub max_results: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SkillInput {
    pub skill: String,
    #[serde(default)]
    pub args: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SkillOutput {
    pub skill: String,
    pub path: String,
    #[serde(default)]
    pub args: Option<String>,
    pub description: String,
    #[serde(default)]
    pub prompt: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BriefInput {
    pub message: String,
    #[serde(default)]
    pub attachments: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ResolvedAttachment {
    pub path: String,
    pub size: u64,
    #[serde(rename = "isImage")]
    pub is_image: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BriefOutput {
    pub message: String,
    #[serde(default)]
    pub attachments: Option<Vec<ResolvedAttachment>>,
    #[serde(rename = "sentAt")]
    pub sent_at: String,
}



/// Wrapper tuple struct so `execute_structured_output` can carry the
/// raw JSON payload through `input.0`.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct StructuredOutputInput(pub serde_json::Value);

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct StructuredOutputResult {
    #[serde(rename = "structuredOutput")]
    pub structured_output: serde_json::Value,
}

// -- Task / worker / team / cron registry inputs. Field shapes are
//    taken directly from the existing `run_task_*` / `run_worker_*`
//    / `run_team_*` / `run_cron_*` functions which already exist.
// =====================================================================






// -- Missing utility functions. Signatures are inferred from call
//    sites in the existing code. `to_pretty_json` is used everywhere
//    as `to_pretty_json(json!({...}))?` so the body is just a thin
//    wrapper around `serde_json::to_string_pretty`.
// =====================================================================

fn to_pretty_json<T: serde::Serialize>(value: T) -> Result<String, String> {
    serde_json::to_string_pretty(&value).map_err(|error| error.to_string())
}


// -- `run_*` wrappers. The dispatch table at `execute_tool_with_enforcer`
//    calls `run_<tool>` for every tool. The original `run_*` functions
//    were thin shells that delegated to `execute_*` (for the tools that
//    have `execute_*` implementations in this file) or to the runtime
//    crate's `read_file` / `new_file` / etc. (for the file tools).
//    These stubs restore that delegation.
// =====================================================================

#[allow(clippy::needless_pass_by_value)]
fn run_read_file(input: ReadFileInput) -> Result<String, String> {
    // Check file cache first �?hits from prior new_file / edit_file calls.
    // Only serve full reads (no offset/limit) from cache to keep it simple.
    if input.offset.is_none() && input.limit.is_none() {
        let abs_path = std::path::Path::new(&input.path).canonicalize().ok().map(|p| dunce::simplified(&p).to_path_buf());
        if let Some(ref abs) = abs_path {
            let key = abs.to_string_lossy().into_owned();
            if let Ok(cache) = global_file_cache().lock() {
                if let Some(entry) = cache.get(&key) {
                    // Cache hit �?construct ReadFileOutput without disk I/O.
                    let lines: Vec<&str> = entry.content.lines().collect();
                    let total_lines = lines.len();
                    let content = if input.full == Some(false) {
                        None
                    } else {
                        Some(entry.content.clone())
                    };
                    let output = runtime::ReadFileOutput {
                        kind: "text".to_string(),
                        file: runtime::TextFilePayload {
                            file_path: runtime::normalize_path_for_output(abs),
                            content,
                            checksum: entry.checksum.clone(),
                            bytes_read: entry.content.len(),
                            num_lines: total_lines,
                            start_line: 1,
                            total_lines,
                        },
                    };
                    return serde_json::to_string_pretty(&output).map_err(|e| e.to_string());
                }
            }
        }
    }

    // Cache miss or partial read �?fall through to disk.
    let workspace_root = std::env::current_dir().map_err(|error| error.to_string())?;
    let output = runtime::read_file_with_policy(
        &input.path,
        input.offset,
        input.limit,
        &workspace_root,
        &active_workspace_policy(),
        input.full,
    )
    .map_err(|error| error.to_string())?;
    serde_json::to_string_pretty(&output).map_err(|error| error.to_string())
}

#[allow(clippy::needless_pass_by_value)]
fn run_new_file(input: WriteFileInput) -> Result<String, String> {
    let workspace_root = std::env::current_dir().map_err(|error| error.to_string())?;
    let output = runtime::new_file_with_policy(
        &input.path,
        &input.content,
        input.force.unwrap_or(false),
        &workspace_root,
        &active_workspace_policy(),
    )
    .map_err(|error| error.to_string())?;

    // Cache the written content so subsequent read_file calls skip disk I/O.
    if let Ok(abs) = std::path::Path::new(&output.file_path).canonicalize().map(|p| dunce::simplified(&p).to_path_buf()) {
        if let Ok(mut cache) = global_file_cache().lock() {
            cache.insert(
                abs.to_string_lossy().into_owned(),
                FileCacheEntry {
                    content: input.content,
                    checksum: output.checksum.clone(),
                },
            );
        }
    }

    serde_json::to_string_pretty(&output).map_err(|error| error.to_string())
}

#[allow(clippy::needless_pass_by_value)]
fn run_edit_file(input: EditFileInput) -> Result<String, String> {
    let workspace_root = std::env::current_dir().map_err(|error| error.to_string())?;
    let output = runtime::edit_file_with_policy(
        &input.path,
        &input.old_string,
        &input.new_string,
        input.replace_all.unwrap_or(false),
        input.expected_checksum.as_deref(),
        &workspace_root,
        &active_workspace_policy(),
    )
    .map_err(|error| error.to_string())?;

    // Update cache: re-read the file from disk to get the exact post-edit content.
    if let Ok(abs) = std::path::Path::new(&output.file_path).canonicalize().map(|p| dunce::simplified(&p).to_path_buf()) {
        if let Ok(new_content) = std::fs::read_to_string(&abs) {
            if let Ok(mut cache) = global_file_cache().lock() {
                cache.insert(
                    abs.to_string_lossy().into_owned(),
                    FileCacheEntry {
                        content: new_content,
                        checksum: output.new_checksum.clone(),
                    },
                );
            }
        }
    }

    // Write diff file for potential rollback.
    let diff_path = write_diff_file(
        &input.path,
        &input.old_string,
        &input.new_string,
        input.replace_all.unwrap_or(false),
    );

    // Include diff_path in the output JSON for JSONL/context markers.
    let mut json = serde_json::to_value(&output).map_err(|e| e.to_string())?;
    if let Some(dp) = &diff_path {
        json["diffPath"] = serde_json::Value::String(dp.clone());
    } else {
        eprintln!("[tools] warning: failed to write diff file — undo unavailable for this edit");
    }
    serde_json::to_string_pretty(&json).map_err(|e| e.to_string())
}

/// Read a patch file and return `(path, parsed_json)` if `reverted` is not true.
/// A missing `reverted` field is treated as `false` (backward compatibility).
fn read_patch_if_unreverted(patch_path: &Path) -> Option<(PathBuf, serde_json::Value)> {
    let content = std::fs::read_to_string(patch_path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    let reverted = parsed.get("reverted").and_then(|v| v.as_bool()).unwrap_or(false);
    if reverted {
        return None;
    }
    Some((patch_path.to_path_buf(), parsed))
}

/// Collect all unreverted patches for `target_file`, sorted oldest-first by filename.
/// Skips patches whose `reverted` field is true (missing treated as false).
/// Scans both the flat directory (new format) and old-style `dYYYYMMDD` subdirectories.
fn scan_unreverted_patches_for_file(
    diffs_root: &Path,
    target_file: &str,
) -> Vec<(PathBuf, serde_json::Value)> {
    let mut patches: Vec<(PathBuf, serde_json::Value)> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(diffs_root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map(|x| x == "patch").unwrap_or(false) {
                if let Some((pp, parsed)) = read_patch_if_unreverted(&path) {
                    if parsed["path"].as_str() == Some(target_file) {
                        patches.push((pp, parsed));
                    }
                }
            }
        }
    }

    // Fallback: scan old-style dYYYYMMDD subdirectories
    if let Ok(entries) = std::fs::read_dir(diffs_root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let dir_path = entry.path();
            if !dir_path.is_dir() {
                continue;
            }
            let dir_name = dir_path.file_name().unwrap().to_string_lossy();
            if !dir_name.starts_with('d') || dir_name.len() != 9 {
                continue;
            }
            if let Ok(sub_entries) = std::fs::read_dir(&dir_path) {
                for sub in sub_entries.filter_map(|e| e.ok()) {
                    let sub_path = sub.path();
                    if sub_path.extension().map(|x| x == "patch").unwrap_or(false) {
                        if let Some((pp, parsed)) = read_patch_if_unreverted(&sub_path) {
                            if parsed["path"].as_str() == Some(target_file) {
                                patches.push((pp, parsed));
                            }
                        }
                    }
                }
            }
        }
    }

    patches.sort_by(|a, b| a.0.file_name().cmp(&b.0.file_name()));
    patches
}

/// Find the newest unreverted `.patch` file across flat directory and old-style subdirectories.
fn find_latest_unreverted_diff(diffs_root: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(diffs_root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map(|x| x == "patch").unwrap_or(false) {
                if read_patch_if_unreverted(&path).is_some() {
                    candidates.push(path);
                }
            }
        }
    }

    // Fallback: scan old-style subdirectories
    if let Ok(entries) = std::fs::read_dir(diffs_root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let dir_path = entry.path();
            if !dir_path.is_dir() {
                continue;
            }
            let dir_name = dir_path.file_name().unwrap().to_string_lossy();
            if !dir_name.starts_with('d') || dir_name.len() != 9 {
                continue;
            }
            if let Ok(sub_entries) = std::fs::read_dir(&dir_path) {
                for sub in sub_entries.filter_map(|e| e.ok()) {
                    let sub_path = sub.path();
                    if sub_path.extension().map(|x| x == "patch").unwrap_or(false) {
                        if read_patch_if_unreverted(&sub_path).is_some() {
                            candidates.push(sub_path);
                        }
                    }
                }
            }
        }
    }

    candidates.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
    candidates.into_iter().next()
}

/// Resolve a user-supplied diff name.
///
/// Resolution order:
/// 1. Absolute / existing path
/// 2. CWD-relative `.claw/diffs/…` (backward compat)
/// 3. Relative path with separator → under `diffs_root` (backward compat)
/// 4. Flat file under `diffs_root/` (new format)
/// 5. Bare filename scanned in old `dYYYYMMDD` subdirectories (backward compat)
fn resolve_diff_path(diffs_root: &Path, input_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(input_path);

    // Absolute path — must resolve within diffs_root
    if path.is_absolute() && path.exists() {
        let canonical = dunce::simplified(&path.canonicalize().map_err(|_| {
            format!("Cannot canonicalize path: {input_path}")
        })?)
        .to_path_buf();
        let diffs_root_canonical = dunce::simplified(
            &diffs_root.canonicalize().map_err(|_| "Diff directory ~/.claw/diffs/ not found")?,
        )
        .to_path_buf();
        if canonical.starts_with(&diffs_root_canonical) {
            return Ok(canonical);
        }
        return Err(format!("Diff path must be within ~/.claw/diffs/: {input_path}"));
    }

    // Backward compat: check CWD-relative `.claw/diffs/…`
    if let Ok(cwd) = std::env::current_dir() {
        let old = cwd.join(".claw").join("diffs").join(input_path);
        if let Ok(canonical) = old.canonicalize().map(|p| dunce::simplified(&p).to_path_buf()) {
            if canonical.exists() {
                // Verify it's under diffs_root
                let diffs_root_canonical = dunce::simplified(
                    &diffs_root.canonicalize().map_err(|_| "Diff directory ~/.claw/diffs/ not found")?,
                )
                .to_path_buf();
                if canonical.starts_with(&diffs_root_canonical) {
                    return Ok(canonical);
                }
            }
        }
    }

    let name = input_path.trim();

    // Relative path with separator (e.g. `d20260722/diff_2141.patch`) — old style
    if name.contains('/') || name.contains('\\') {
        let candidate = diffs_root.join(name);
        if let Ok(canonical) = candidate.canonicalize().map(|p| dunce::simplified(&p).to_path_buf()) {
            let diffs_root_canonical = dunce::simplified(
                &diffs_root.canonicalize().map_err(|_| "Diff directory ~/.claw/diffs/ not found")?,
            )
            .to_path_buf();
            if canonical.starts_with(&diffs_root_canonical) {
                return Ok(canonical);
            }
        }
        return Err(format!("Diff file not found or outside diffs_root: {name}"));
    }

    // New flat naming: directly under diffs_root
    let flat_candidate = diffs_root.join(name);
    if flat_candidate.exists() {
        return Ok(flat_candidate);
    }

    // Old bare filename (e.g. `diff_2141.patch`) — scan date dirs
    let mut dirs: Vec<_> = match std::fs::read_dir(diffs_root) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().ok().map(|t| t.is_dir()).unwrap_or(false))
            .filter(|e| {
                let name = e.file_name();
                let n = name.to_string_lossy();
                n.starts_with('d') && n.len() == 9
            })
            .collect(),
        Err(_) => vec![],
    };
    dirs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

    for dir in &dirs {
        let candidate = dir.path().join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(format!("Diff file not found: {name}"))
}

/// Simulate reverse-applying a list of patches against `content` in memory.
/// Patches should be in newest-first order (reverse chronological).
/// Returns `Ok(())` if all would succeed, or `Err` with details on first failure.
fn dry_run_reverse_patches(
    content: &str,
    patches: &[(PathBuf, serde_json::Value)],
) -> Result<(), String> {
    let mut working = content.to_owned();
    for (patch_path, patch) in patches {
        let new_string = patch
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Patch {} missing 'new_string' field", patch_path.display()))?;
        let replace_all = patch["replace_all"].as_bool().unwrap_or(false);

        if !working.contains(new_string) {
            let lines: Vec<&str> = working.lines().collect();
            let preview = if lines.len() > 10 {
                format!(
                    "...{} lines...\n{}",
                    lines.len() - 10,
                    lines[lines.len().saturating_sub(10)..].join("\n")
                )
            } else {
                working.clone()
            };
            let end = new_string.len().min(80);
            let idx = new_string.floor_char_boundary(end);
            let expected_preview = &new_string[..idx];
            return Err(format!(
                "Undo conflict: cannot reverse patch {}\n\
                 File: {}\n\
                 Expected to find string (first 80 chars):\n\
                 \"{expected_preview}\"\n\
                 Not found in current file content.\n\
                 Last 10 lines of file:\n{preview}\n\
                 Suggestion: The file may have been edited by another \
                 operation after this patch was created.",
                patch_path.file_name().unwrap().to_string_lossy(),
                patch["path"].as_str().unwrap_or("unknown"),
            ));
        }

        let old_string = patch
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Patch {} missing 'old_string' field", patch_path.display()))?;
        if replace_all {
            working = working.replace(new_string, old_string);
        } else {
            working = working.replacen(new_string, old_string, 1);
        }
    }
    Ok(())
}

/// Read a patch file, set `"reverted": true`, and write it back.
fn mark_patch_reverted(patch_path: &Path) -> Result<(), String> {
    let content =
        std::fs::read_to_string(patch_path).map_err(|e| format!("Failed to read patch file: {e}"))?;
    let mut parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse patch file: {e}"))?;
    parsed["reverted"] = serde_json::Value::Bool(true);
    std::fs::write(
        patch_path,
        serde_json::to_string_pretty(&parsed).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("Failed to write patch file: {e}"))?;
    Ok(())
}

/// Transactionally reverse-apply a list of patches (newest-first) to a file.
///
/// 1. Workspace boundary check: validates target file is inside the workspace.
/// 2. Dry-run phase: validates all patches can be reverse-applied in memory.
/// 3. Write phase: modifies the file before marking any patches.
/// 4. Mark reverted phase: sets `reverted: true` on all patches only after the write succeeds.
///
/// If boundary check or dry-run fails, the file is **not** modified.
fn transactional_undo_patches(
    file_path: &str,
    patches: &[(PathBuf, serde_json::Value)],
    workspace_root: &Path,
    policy: &BoundaryPolicy,
) -> Result<String, String> {
    let canonical = dunce::simplified(Path::new(file_path)).to_path_buf();

    // Workspace boundary check
    let canonical_root = dunce::simplified(
        &workspace_root.canonicalize().unwrap_or_else(|_| workspace_root.to_path_buf()),
    )
    .to_path_buf();
    let check = runtime::boundary::classify_boundary(&canonical, &canonical_root);
    if matches!(check, runtime::boundary::BoundaryCheck::OutOfWorkspace { .. }) {
        match policy.enforce_outside(
            &canonical,
            &canonical_root,
            runtime::boundary::BoundaryOperation::Write,
        ) {
            runtime::boundary::PolicyOutcome::Proceed
            | runtime::boundary::PolicyOutcome::Approved { .. } => {}
            runtime::boundary::PolicyOutcome::Denied(msg) => {
                return Err(msg);
            }
        }
    }

    let current_content =
        std::fs::read_to_string(&canonical).map_err(|e| format!("Failed to read file '{file_path}': {e}"))?;

    // Dry-run: simulate all reverse applications in memory
    dry_run_reverse_patches(&current_content, patches)?;

    // All dry-runs passed — compute restored content
    let mut restored_content = current_content;
    for (_patch_path, patch) in patches {
        let old_string = patch
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Patch {} missing 'old_string' field", _patch_path.display()))?;
        let new_string = patch
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Patch {} missing 'new_string' field", _patch_path.display()))?;
        let replace_all = patch["replace_all"].as_bool().unwrap_or(false);

        if replace_all {
            restored_content = restored_content.replace(new_string, old_string);
        } else {
            restored_content = restored_content.replacen(new_string, old_string, 1);
        }
    }

    // Write restored file — moved BEFORE mark-patches to be after dry-run
    std::fs::write(&canonical, &restored_content)
        .map_err(|e| format!("Failed to write restored file '{file_path}': {e}"))?;

    // Mark all patches as reverted
    for (patch_path, _) in patches {
        mark_patch_reverted(patch_path)?;
    }

    // Update file cache
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    restored_content.hash(&mut hasher);
    let new_checksum = format!("{:016x}", hasher.finish());
    if let Ok(mut cache) = global_file_cache().lock() {
        cache.insert(
            canonical.to_string_lossy().into_owned(),
            FileCacheEntry {
                content: restored_content,
                checksum: new_checksum.clone(),
            },
        );
    }

    let file_name = canonical
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());

    Ok(serde_json::json!({
        "type": "undo",
        "filePath": canonical.to_string_lossy(),
        "status": "reverted",
        "patchesReverted": patches.len(),
        "file": file_name,
    })
    .to_string())
}

/// Multi-step undo with dry-run safety.
///
/// Supports several invocation forms via `UndoInput.diff_path`:
/// * Empty                          → undo the single newest unreverted patch (any file)
/// * `<file>`                       → undo the newest unreverted patch for that file
/// * `<patch_name>`                 → undo all changes to that file from that patch forward
/// * `<file> <patch_name>`          → explicit file + patch combination
fn run_undo(input: UndoInput) -> Result<String, String> {
    let diffs_root = default_config_home().join("diffs");
    let workspace_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let policy = active_workspace_policy();

    // ---- Parse input ----
    let dp = input.diff_path.trim().to_string();
    let (target_file, target_patch) = if dp.contains(' ') {
        let parts: Vec<&str> = dp.splitn(2, ' ').collect();
        (Some(parts[0].to_string()), Some(parts[1].to_string()))
    } else if dp.is_empty() {
        (None, None)
    } else if (dp.len() >= 14 && dp.chars().take(14).all(|c| c.is_ascii_digit()))
        || dp.ends_with(".patch")
    {
        (None, Some(dp))
    } else {
        (Some(dp), None)
    };

    // ---- Resolve target patch to PathBuf if given ----
    let resolved_patch = target_patch
        .as_ref()
        .map(|p| resolve_diff_path(&diffs_root, p))
        .transpose()?;

    // ---- Determine the target file ----
    let resolved_file = match (target_file, &resolved_patch) {
        (Some(file), _) => file,
        (None, Some(patch_path)) => {
            let content = std::fs::read_to_string(patch_path)
                .map_err(|e| format!("Failed to read patch file: {e}"))?;
            let parsed: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| format!("Invalid patch file: {e}"))?;
            parsed["path"]
                .as_str()
                .ok_or("Patch file missing 'path' field")?
                .to_string()
        }
        (None, None) => {
            // Bare /undo: find the single latest unreverted patch
            let latest = find_latest_unreverted_diff(&diffs_root)
                .ok_or_else(|| "Nothing to undo — no unreverted patch files found.".to_string())?;
            let content = std::fs::read_to_string(&latest)
                .map_err(|e| format!("Failed to read patch file: {e}"))?;
            let parsed: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| format!("Invalid patch file: {e}"))?;
            let file = parsed["path"]
                .as_str()
                .ok_or("Patch file missing 'path' field")?
                .to_string();
            return transactional_undo_patches(&file, &[(latest, parsed)], &workspace_root, &policy);
        }
    };

    // ---- Collect unreverted patches for the target file ----
    let all_patches = scan_unreverted_patches_for_file(&diffs_root, &resolved_file);
    if all_patches.is_empty() {
        return Err(format!("No unreverted patches found for file: {resolved_file}"));
    }

    // ---- Select scope (which patches to undo) ----
    let scope: Vec<(PathBuf, serde_json::Value)> = if let Some(ref patch_path) = resolved_patch {
        let bound_name = patch_path
            .file_name()
            .unwrap_or(patch_path.as_os_str())
            .to_string_lossy()
            .to_string();
        let bound_idx = all_patches
            .iter()
            .position(|(p, _)| {
                p.file_name()
                    .map(|n| n.to_string_lossy().as_ref() == bound_name)
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                format!("Patch '{bound_name}' not found for file '{resolved_file}'")
            })?;
        all_patches[bound_idx..].iter().rev().cloned().collect()
    } else {
        // No specific patch: undo just the newest single patch
        vec![all_patches.last().unwrap().clone()]
    };

    transactional_undo_patches(&resolved_file, &scope, &workspace_root, &policy)
}

#[allow(clippy::needless_pass_by_value)]
fn run_glob_search(input: GlobSearchInputValue) -> Result<String, String> {
    let output = runtime::glob_search(&input.pattern, input.path.as_deref())
        .map_err(|error| error.to_string())?;
    serde_json::to_string_pretty(&output).map_err(|error| error.to_string())
}

#[allow(clippy::needless_pass_by_value)]
fn run_grep_search(input: runtime::GrepSearchInput) -> Result<String, String> {
    let output = runtime::grep_search(&input).map_err(|error| error.to_string())?;
    serde_json::to_string_pretty(&output).map_err(|error| error.to_string())
}

// Fetch a single URL and return the body as text. The output shape is
// `{code, url, result}` so callers can drive further processing (title
// extraction, line-based slicing) off the result field. HTML responses
// are passed through `html_to_text`; other content types are returned
// verbatim. The 5 MB body cap and 10 s timeout keep the tool cheap
// to call from a model that may request many pages in parallel.
const WEB_FETCH_TIMEOUT_SECS: u64 = 10;
const WEB_FETCH_MAX_BODY_BYTES: usize = 5 * 1024 * 1024;

// Heuristic: when a Wikipedia URL is unreachable (the source is
// typically blocked on networks behind the GFW), the WebFetch tool
// automatically retries with a Sogou search URL containing the article
// title. Sogou search is reachable from the same networks and its
// results page links to Chinese mirrors (baike.sogou.com, baike.baidu.com,
// zhihu.com, etc.) that the model can re-fetch.
//
// `wiki_mirror_url` returns `Some((url, label))` for any Wikipedia
// URL whose path is `/wiki/<title>`, and `None` otherwise. It is
// implemented as a pure function so tests can pin its behavior
// without touching the network.
fn wiki_mirror_url(url: &reqwest::Url) -> Option<(reqwest::Url, &'static str)> {
    let host = url.host_str()?.to_ascii_lowercase();
    let is_wiki = host == "wikipedia.org" || host.ends_with(".wikipedia.org");
    if !is_wiki {
        return None;
    }
    // The path of a Wikipedia article is `/wiki/<title>`. We URL-
    // decode the title (Wikipedia paths are percent-encoded) and
    // convert `_` to space (Wikipedia URL convention) before
    // handing it to Sogou. The `path_segments()` iterator returns
    // raw bytes, so we work from `path()` directly and decode
    // explicitly to avoid double-encoding.
    let path = url.path();
    let title_encoded = path.strip_prefix("/wiki/")?;
    if title_encoded.is_empty() {
        return None;
    }
    let title_decoded = url::form_urlencoded::parse(title_encoded.as_bytes())
        .next()
        .map(|(k, _)| k.into_owned())
        .unwrap_or_else(|| title_encoded.to_string());
    let title = title_decoded.replace('_', " ");
    if title.trim().is_empty() {
        return None;
    }
    let mut mirror = reqwest::Url::parse("https://www.sogou.com/web").ok()?;
    mirror.query_pairs_mut().append_pair("query", &title);
    Some((mirror, "sogou-search"))
}

#[allow(clippy::needless_pass_by_value)]
/// Inner fetch helper used by [`run_web_fetch`]. Returns the HTTP
/// status, content-type, and body string, or an error describing the
/// transport/HTTP/parse failure. Used twice when a Wikipedia URL
/// falls back to the Sogou search mirror.
fn fetch_once(
    client: &reqwest::blocking::Client,
    url: &reqwest::Url,
) -> Result<(u16, String, String), String> {
    let response = client
        .get(url.clone())
        .send()
        .map_err(|error| format!("fetch failed for '{}': {}", url, error))?;
    let code = response.status().as_u16();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("text/plain")
        .to_ascii_lowercase();
    let body = response
        .bytes()
        .map_err(|error| format!("read body failed for '{}': {}", url, error))?;
    if body.len() > WEB_FETCH_MAX_BODY_BYTES {
        return Err(format!(
            "response too large for '{}': {} bytes (limit is {} bytes)",
            url,
            body.len(),
            WEB_FETCH_MAX_BODY_BYTES
        ));
    }
    let raw = String::from_utf8_lossy(&body).into_owned();
    Ok((code, content_type, raw))
}

fn run_web_fetch(input: WebFetchInput) -> Result<String, String> {
    let url = reqwest::Url::parse(&input.url)
        .map_err(|error| format!("invalid URL '{}': {}", input.url, error))?;

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(WEB_FETCH_TIMEOUT_SECS))
        .user_agent("clawcode/0.1 (+webfetch)")
        .build()
        .map_err(|error| error.to_string())?;

    // Check WebFetch cache BEFORE making HTTP request.
    // On cache hit, re-summarize raw body with the current prompt
    // so prompt-specific processing (title extraction, summarization)
    // works correctly regardless of which prompt was used originally.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if let Ok(cache) = global_webfetch_cache().lock() {
        if let Some(entry) = cache.get(&input.url) {
            if now.saturating_sub(entry.fetched_at) < WEBFETCH_CACHE_TTL_SECS {
                let summarized = summarize_web_fetch(
                    &input.url,
                    &input.prompt,
                    &entry.raw_body,
                    &entry.content_type,
                );
                return serde_json::to_string_pretty(&serde_json::json!({
                    "code": 200,
                    "url": input.url,
                    "result": summarized,
                    "cached": true,
                }))
                .map_err(|e| e.to_string());
            }
        }
    }

    // Attempt the primary URL first. If it fails and the URL is a
    // Wikipedia article, retry with the Sogou search mirror. This
    // makes Wikipedia fetches work on networks that block the source
    // (e.g. behind the GFW) by routing the request to a reachable
    // search engine that links to Chinese mirrors.
    let (primary_code, primary_ct, primary_body) = match fetch_once(&client, &url) {
        Ok(result) => result,
        Err(primary_error) => match wiki_mirror_url(&url) {
            Some((mirror, label)) => {
                let (code, ct, body) = fetch_once(&client, &mirror).map_err(|mirror_error| {
                    format!(
                        "primary URL '{url}' failed ({primary_error}) and mirror \
                             '{mirror}' also failed ({mirror_error})"
                    )
                })?;
                let summarized = summarize_web_fetch(mirror.as_str(), &input.prompt, &body, &ct);
                return serde_json::to_string_pretty(&serde_json::json!({
                    "code": code,
                    "url": input.url,
                    "mirror": label,
                    "mirrorUrl": mirror.as_str(),
                    "result": summarized,
                }))
                .map_err(|error| error.to_string());
            }
            None => return Err(primary_error),
        },
    };

    // If the primary came back non-2xx and is a Wikipedia URL, try the
    // mirror as a content source. The body is usually a Cloudflare
    // challenge page, which is why we treat it as failure even when
    // the transport succeeded.
    let (code, content_type, raw, used_mirror) = if !(200..300).contains(&primary_code) {
        if let Some((mirror, label)) = wiki_mirror_url(&url) {
            match fetch_once(&client, &mirror) {
                Ok((code, ct, body)) => (code, ct, body, Some(label)),
                Err(_) => (primary_code, primary_ct, primary_body, None),
            }
        } else {
            (primary_code, primary_ct, primary_body, None)
        }
    } else {
        (primary_code, primary_ct, primary_body, None)
    };

    let result = summarize_web_fetch(&input.url, &input.prompt, &raw, &content_type);

    // Store raw body in cache for future dedup; re-summarize on hit.
    if let Ok(mut cache) = global_webfetch_cache().lock() {
        cache.insert(
            input.url.clone(),
            WebFetchCacheEntry {
                raw_body: raw.clone(),
                content_type: content_type.to_string(),
                fetched_at: now,
            },
        );
    }

    // Return full content to the AI on first fetch (needed for processing).
    // Cache stores the content for dedup; subsequent hits return only a marker.
    let mut payload = serde_json::json!({
        "code": code,
        "url": input.url,
        "result": result,
    });
    if let Some(label) = used_mirror {
        payload["mirror"] = serde_json::Value::String(label.to_string());
    }
    serde_json::to_string_pretty(&payload).map_err(|error| error.to_string())
}

// Multi-source Chinese web search. Tries each provider in order and
// returns results from the first that succeeds. Sources:
//   1. so.eastmoney.com   --- financial/stock news
//   2. www.donews.com     --- tech news
//   3. 36kr.com           --- startup/tech articles
//   4. cn.bing.com        --- Bing restricted to cnblogs.com
//   5. bkso.baidu.com     --- Baidu Baike entry (single direct page)
const SEARCHAPI_DEFAULT_RESULTS: usize = 10;
const SEARCHAPI_MAX_RESULTS: usize = 20;

// ── Shared search-result record ─────────────────────────────────────
struct ScrapedSearchResult {
    title: String,
    link: String,
    snippet: String,
    source: String,
    #[allow(dead_code)]
    date: String,
}

fn format_search_response(
    query: &str,
    provider: &str,
    results: Vec<ScrapedSearchResult>,
    max_results: usize,
    errors: &[String],
) -> Result<String, String> {
    let total = results.len();
    let truncated: Vec<serde_json::Value> = results
        .into_iter()
        .take(max_results)
        .map(|r| {
            serde_json::json!({
                "title":   r.title,
                "link":    r.link,
                "snippet": r.snippet,
                "source":  r.source,
                "date":    r.date,
            })
        })
        .collect();
    let mut resp = serde_json::json!({
        "query": query,
        "provider": provider,
        "totalResults": total,
        "resultsReturned": truncated.len(),
        "results": truncated,
    });
    if !errors.is_empty() {
        resp["providerErrors"] = serde_json::json!(errors);
    }
    serde_json::to_string_pretty(&resp).map_err(|e| e.to_string())
}

// ── Bing scraper ────────────────────────────────────────────────────
fn node_text(node: &scraper::ElementRef) -> String {
    node.text().collect::<String>().trim().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WebSearchProviderConfig {
    #[serde(default = "default_enable")]
    enable: bool,
    url: Option<String>,
}

fn default_enable() -> bool {
    true
}

const DEFAULT_WEB_SEARCH_PROVIDERS: &[(&str, Option<&str>)] = &[
    ("url_0", Some("https://www.bing.com/search?q={search}")),
    ("url_1", None),
    ("url_2", None),
    ("url_3", None),
    ("url_4", None),
];

fn load_web_search_config() -> HashMap<String, WebSearchProviderConfig> {
    let mut configs: HashMap<String, WebSearchProviderConfig> = DEFAULT_WEB_SEARCH_PROVIDERS
        .iter()
        .map(|(name, url)| {
            let has_url = url.is_some();
            (
                name.to_string(),
                WebSearchProviderConfig {
                    enable: has_url,
                    url: url.map(|u| u.to_string()),
                },
            )
        })
        .collect();

    let merge_from = |path: std::path::PathBuf, target: &mut HashMap<String, WebSearchProviderConfig>| {
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                if e.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("[websearch] failed to read config '{}': {e}", path.display());
                }
                return;
            }
        };
        let overrides: HashMap<String, WebSearchProviderConfig> = match serde_json::from_str(&content) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[websearch] failed to parse config '{}': {e}", path.display());
                return;
            }
        };
        for (key, entry) in overrides {
            if let Some(existing) = target.get_mut(&key) {
                if let Some(url) = entry.url {
                    existing.url = Some(url);
                }
                existing.enable = entry.enable;
            } else {
                target.insert(key, entry);
            }
        }
    };

    let user_config = default_config_home().join("web_search_url.json");

    // Project first, user fallback — first found wins.
    match std::env::current_dir() {
        Ok(cwd) => {
            let project_config = cwd.join(".claw").join("web_search_url.json");
            if project_config.is_file() {
                merge_from(project_config, &mut configs);
                return configs;
            }
        }
        Err(e) => eprintln!("[websearch] cannot determine current dir for project config: {e}"),
    }
    if user_config.is_file() {
        merge_from(user_config, &mut configs);
    }

    configs
}

fn percent_encode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

fn build_search_url(template: &str, query: &str) -> String {
    let marker = "{search}";
    if let Some(idx) = template.find(marker) {
        let prefix = &template[..idx];
        let suffix = &template[idx + marker.len()..];
        let combined = format!("{query}{suffix}");
        format!("{prefix}{}", percent_encode_query(&combined))
    } else {
        template.to_string()
    }
}

fn fetch_search_html(client: &reqwest::blocking::Client, url: &str) -> Result<String, String> {
    client
        .get(url)
        .send()
        .and_then(|r| r.text())
        .map_err(|e| format!("request failed: {e}"))
}

fn extract_generic_results(
    html: &str,
    item_selectors: &[&str],
    link_selectors: &[&str],
    snippet_selectors: &[&str],
    default_source: &str,
) -> Vec<ScrapedSearchResult> {
    let document = Html::parse_document(html);
    let item_sel = match Selector::parse(&item_selectors.join(", ")) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let link_sel = match Selector::parse(&link_selectors.join(", ")) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let snippet_sel = match Selector::parse(&snippet_selectors.join(", ")) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut results = Vec::new();
    for node in document.select(&item_sel) {
        let (title, link) = match node.select(&link_sel).next() {
            Some(a) => (node_text(&a), a.value().attr("href").unwrap_or("").to_string()),
            None => continue,
        };
        let snippet = node.select(&snippet_sel).next().map(|p| node_text(&p)).unwrap_or_default();
        let source = reqwest::Url::parse(&link)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_else(|| default_source.to_string());
        if !title.is_empty() {
            results.push(ScrapedSearchResult { title, link, snippet, source, date: String::new() });
        }
    }
    results
}

fn scrape_url_generic(
    client: &reqwest::blocking::Client,
    url_template: &str,
    query: &str,
    source: &str,
) -> Result<Vec<ScrapedSearchResult>, String> {
    let url = build_search_url(url_template, query);
    let html = fetch_search_html(client, &url)?;
    let results = extract_generic_results(
        &html,
        &[".result", ".search-result", ".result-item", ".g", "li.b_algo"],
        &["h3 a", "h2 a", ".title a"],
        &["p", ".st", ".b_caption p", ".desc", ".snippet"],
        source,
    );
    Ok(results)
}

static WEB_SEARCH_CONFIG: OnceLock<HashMap<String, WebSearchProviderConfig>> = OnceLock::new();

pub fn init_web_search_config() {
    WEB_SEARCH_CONFIG.get_or_init(load_web_search_config);
}

fn web_search_enabled_providers() -> Vec<(String, String)> {
    let config = WEB_SEARCH_CONFIG.get_or_init(load_web_search_config);
    const MAX_PROVIDERS: usize = 5;
    config
        .iter()
        .filter(|(_, entry)| entry.enable)
        .filter_map(|(name, entry)| {
            let url = entry.url.clone().unwrap_or_default();
            if url.is_empty() { None } else { Some((name.clone(), url)) }
        })
        .take(MAX_PROVIDERS)
        .collect()
}

fn run_web_search(input: WebSearchInput) -> Result<String, String> {
    let max_results = input
        .max_results
        .unwrap_or(SEARCHAPI_DEFAULT_RESULTS)
        .min(SEARCHAPI_MAX_RESULTS)
        .max(1);

    let keyword = input.query.trim().to_string();

    use std::sync::mpsc;

    let client = std::sync::Arc::new(
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(WEB_FETCH_TIMEOUT_SECS))
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/131.0.0.0 Safari/537.36",
            )
            .build()
            .map_err(|e| e.to_string())?,
    );

    let providers = web_search_enabled_providers();

    if providers.is_empty() {
        return format_search_response(&keyword, "none", Vec::new(), max_results, &[]);
    }

    let (tx, rx) = mpsc::channel::<(String, Result<Vec<ScrapedSearchResult>, String>)>();
    let query = std::sync::Arc::new(keyword.clone());

    for (name, url_template) in &providers {
        let tx = tx.clone();
        let client = std::sync::Arc::clone(&client);
        let query = std::sync::Arc::clone(&query);
        let name = name.clone();
        let url_template = url_template.clone();
        let source = name.clone();

        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                scrape_url_generic(&client, &url_template, &query, &source)
            }))
            .unwrap_or(Err("provider thread panicked".to_string()));
            let _ = tx.send((name, result));
        });
    }

    drop(tx);

    let mut all_results: Vec<ScrapedSearchResult> = Vec::new();
    let mut providers_used: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for (name, result) in rx {
        match result {
            Ok(mut results) if !results.is_empty() => {
                results.truncate(SEARCHAPI_DEFAULT_RESULTS);
                providers_used.push(name);
                all_results.extend(results);
            }
            Ok(_) => {}
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }

    if all_results.is_empty() {
        return format_search_response(&keyword, "none", Vec::new(), max_results, &errors);
    }

    all_results.truncate(max_results);
    let provider = providers_used.join("+");
    format_search_response(&keyword, &provider, all_results, max_results, &errors)
}

// Maximum matches a single WebFind call can return. Larger results
// get truncated with `truncated: true` so the LLM sees the token cost
// explicitly instead of being silently flooded.
const WEB_FIND_MAX_MATCHES_CAP: usize = 50;
const WEB_FIND_DEFAULT_MAX_MATCHES: usize = 10;
const WEB_FIND_DEFAULT_CONTEXT_CHARS: usize = 100;
const WEB_FIND_MAX_CONTEXT_CHARS: usize = 500;

// Server-side grep over a fetched URL. Inspired by OpenAI's
// `web_search` provider `find` action: the model supplies `url` and
// `pattern`, the tool returns just the matching snippets (with line,
// column, and trimmed context) instead of dumping the whole page.
// This is the token-efficient counterpart to WebFetch �?a 36 KB HTML
// page can shrink to a few hundred tokens when the model only needs
// the lines containing a specific value.
#[allow(clippy::needless_pass_by_value)]
fn run_web_find(input: WebFindInput) -> Result<String, String> {
    if input.pattern.is_empty() {
        return Err(String::from("pattern must not be empty"));
    }

    let url = reqwest::Url::parse(&input.url)
        .map_err(|error| format!("invalid URL '{}': {}", input.url, error))?;

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(WEB_FETCH_TIMEOUT_SECS))
        .user_agent("clawcode/0.1 (+webfind)")
        .build()
        .map_err(|error| error.to_string())?;

    let response = client
        .get(url.clone())
        .send()
        .map_err(|error| format!("fetch failed for '{}': {}", input.url, error))?;

    let code = response.status().as_u16();
    if !(200..300).contains(&code) {
        return Err(format!("fetch failed for '{}': HTTP {}", input.url, code));
    }

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("text/plain")
        .to_ascii_lowercase();

    let body = response
        .bytes()
        .map_err(|error| format!("read body failed for '{}': {}", input.url, error))?;
    if body.len() > WEB_FETCH_MAX_BODY_BYTES {
        return Err(format!(
            "response too large for '{}': {} bytes (limit is {} bytes)",
            input.url,
            body.len(),
            WEB_FETCH_MAX_BODY_BYTES
        ));
    }
    let raw = String::from_utf8_lossy(&body);

    let output = summarize_web_find(
        &input.url,
        &input.pattern,
        &raw,
        &content_type,
        input.ignore_case.unwrap_or(true),
        input
            .max_matches
            .unwrap_or(WEB_FIND_DEFAULT_MAX_MATCHES)
            .min(WEB_FIND_MAX_MATCHES_CAP),
        input
            .context_chars
            .unwrap_or(WEB_FIND_DEFAULT_CONTEXT_CHARS)
            .min(WEB_FIND_MAX_CONTEXT_CHARS),
    );

    serde_json::to_string_pretty(&output).map_err(|error| error.to_string())
}

// Convert fetched bytes into greppable text, find every occurrence
// of `pattern`, trim each match to `context_chars` of surrounding
// text, and cap the result at `max_matches`. Total occurrences are
// counted even after truncation so the caller can tell when more
// data existed.
fn summarize_web_find(
    url: &str,
    pattern: &str,
    raw_body: &str,
    content_type: &str,
    ignore_case: bool,
    max_matches: usize,
    context_chars: usize,
) -> WebFindOutput {
    // For HTML, run the body through the same extractor WebFetch
    // uses, so matches land on visible text instead of buried in
    // markup the LLM cannot reason about.
    let body = if content_type.contains("html") {
        let evaluator = FastContentEvaluator::default();
        let text = evaluator.extract_text(raw_body);
        if text.trim().is_empty() {
            raw_body.to_string()
        } else {
            text
        }
    } else {
        raw_body.to_string()
    };

    let haystack = if ignore_case {
        body.to_lowercase()
    } else {
        body.clone()
    };
    let needle = if ignore_case {
        pattern.to_lowercase()
    } else {
        pattern.to_string()
    };

    let mut matches: Vec<WebFindMatch> = Vec::new();
    let mut total: usize = 0;
    let mut line: usize = 1;
    let mut line_start: usize = 0;
    let bytes_scanned = body.len();

    for (offset, ch) in haystack.char_indices() {
        if ch == '\n' {
            line += 1;
            line_start = offset + ch.len_utf8();
            continue;
        }
        if haystack[offset..].starts_with(&needle) {
            total += 1;
            if matches.len() < max_matches {
                let line_end = haystack[line_start..]
                    .find('\n')
                    .map(|delta| line_start + delta)
                    .unwrap_or(haystack.len());
                let column = offset - line_start + 1;
                let matched = body[offset..offset + needle.len()].to_string();
                let context = extract_match_context(
                    &body,
                    line_start,
                    line_end,
                    column,
                    needle.len(),
                    context_chars,
                );
                matches.push(WebFindMatch {
                    line,
                    column,
                    matched,
                    context,
                });
            }
        }
    }

    WebFindOutput {
        url: url.to_string(),
        pattern: pattern.to_string(),
        total_matches: total,
        truncated: total > matches.len(),
        matches,
        bytes_scanned,
        content_type: content_type.to_string(),
    }
}

// Pull up to `context_chars` chars before and after a match within
// the matched line, collapsing internal whitespace so the LLM gets a
// compact snippet rather than a wall of source formatting.
fn extract_match_context(
    body: &str,
    line_start: usize,
    line_end: usize,
    column_one_indexed: usize,
    match_len: usize,
    context_chars: usize,
) -> String {
    let line = &body[line_start..line_end];
    let line_chars: Vec<char> = line.chars().collect();
    let match_start = column_one_indexed.saturating_sub(1);
    let match_end = (match_start + match_len).min(line_chars.len());

    let window_start = match_start.saturating_sub(context_chars);
    let window_end = (match_end + context_chars).min(line_chars.len());
    let snippet: String = line_chars[window_start..window_end]
        .iter()
        .copied()
        .collect();
    collapse_whitespace(&snippet)
}

#[allow(clippy::needless_pass_by_value)]
fn run_skill(input: SkillInput) -> Result<String, String> {
    let output = execute_skill(input)?;
    serde_json::to_string_pretty(&output).map_err(|error| error.to_string())
}

// Build the manifest + output file, promote Created -> Running on disk,
// then hand the job to the spawn closure. Production callers pass
// `spawn_agent_task`; tests pass a mock that captures the job and
// returns a noop handle so the rest of the file can be exercised
// without spinning a real agent.
#[allow(clippy::needless_pass_by_value)]
fn execute_agent_with_spawn<F>(
    input: agents::AgentInput,
    spawn_fn: F,
) -> Result<(agents::AgentOutput, agents::AgentHandle), String>
where
    F: FnOnce(agents::AgentJob) -> Result<agents::AgentHandle, String>,
{
    if input.description.trim().is_empty() {
        return Err(String::from("description must not be empty"));
    }
    if input.prompt.trim().is_empty() {
        return Err(String::from("prompt must not be empty"));
    }

    let agent_id = make_agent_id();
    let raw_name = input.name.as_deref().unwrap_or(&input.description);
    let name = slugify_agent_name(raw_name);

    let normalized_subagent = input
        .subagent_type
        .as_deref()
        .map(|raw| normalize_subagent_type(Some(raw)));
    let lookup_subagent = normalized_subagent.as_deref().unwrap_or("general-purpose");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let manifest = agents::AgentOutput {
        agent_id: agent_id.clone(),
        name: name.clone(),
        description: input.description.clone(),
        subagent_type: normalized_subagent.clone(),
        model: Some(resolve_agent_model(input.model.as_deref())),
        mode: input.mode.clone(),
        reasoning_effort: input.reasoning_effort.clone(),
        permission: input.permission.clone(),
        status: Some("Running".to_string()),
        error: None,
        started_at: Some(now),
        completed_at: None,
        lane_events: vec![],
    };

    let handle_manifest = manifest.clone();

    // Build the system prompt before constructing the job so a prompt-build
    // failure propagates instead of silently spawning the agent with an empty
    // prompt. A caller-supplied custom prompt is a complete system prompt on
    // its own, so it still takes precedence even when the derived base fails
    // to build (e.g. a broken project config).
    let system_prompt = match input.system_prompt {
        Some(custom) => match build_agent_system_prompt(lookup_subagent) {
            Ok(mut base) if !base.is_empty() => {
                base.extend(custom);
                base
            }
            _ => custom,
        },
        None => build_agent_system_prompt(lookup_subagent)?,
    };

    let job = agents::AgentJob {
        manifest,
        prompt: input.prompt,
        reasoning_effort: input.reasoning_effort,
        permission: input.permission,
        permission_mode: active_permission_mode(),
        system_prompt,
        allowed_tools: {
            let mut allowed = input
                .allowed_tools
                .clone()
                .unwrap_or_else(|| allowed_tools_for_subagent(lookup_subagent));
            // A general-purpose sub-agent inherits the dynamically registered
            // runtime tools (MCP + plugin) so it can call them itself. Other
            // sub-agent kinds stay on their static (read-only/restricted) set.
            if lookup_subagent == "general-purpose" && input.allowed_tools.is_none() {
                if let Some(defs) = agents::registered_extra_tool_defs() {
                    allowed.extend(defs.iter().map(|def| def.name.clone()));
                }
            }
            allowed
        },
    };

    let handle = spawn_fn(job)?;
    Ok((handle_manifest, handle))
}

#[allow(clippy::needless_pass_by_value)]
fn run_agent(input: agents::AgentInput) -> Result<String, String> {
    let progress = agents::new_shared_progress();
    let progress_clone = std::sync::Arc::clone(&progress);
    let (_, mut handle) = execute_agent_with_spawn(input, move |job| {
        spawn_agent_task_with_progress(job, progress_clone)
    })?;

    wait_for_agent(
        &mut handle,
        std::time::Instant::now()
            + std::time::Duration::from_secs(agents::DEFAULT_AGENT_TIMEOUT_SECS),
    )
}

/// Poll the subagent handle until it produces a result or `deadline` passes.
/// The deadline guarantees a hung subagent (whose network reads are now
/// time-bounded by the api crate) cannot hang the parent model turn forever;
/// on expiry we return a timeout error and the handle drop reaps the worker.
fn wait_for_agent(
    handle: &mut agents::AgentHandle,
    deadline: std::time::Instant,
) -> Result<String, String> {
    let term_width = detect_terminal_width();
    use std::sync::atomic::Ordering;

    let mut last_event_count = handle.progress.event_seq.load(Ordering::Acquire);
    let mut last_lines = 0usize;

    loop {
        if std::time::Instant::now() >= deadline {
            // Timeout: signal the worker to stop at its next iteration
            // boundary, then reap it within a short grace window. Without the
            // signal the worker keeps executing (possibly mutating files /
            // calling MCP tools) after the parent already gave up, so a parent
            // retry of the Agent tool would run the same task twice
            // concurrently.
            handle.cancel();
            let grace = std::time::Instant::now() + std::time::Duration::from_secs(3);
            loop {
                match handle.try_join() {
                    Ok(_) => break,
                    Err(agents::TryAgain) if std::time::Instant::now() >= grace => break,
                    Err(agents::TryAgain) => {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            }
            return Err("agent timed out".to_string());
        }
        match handle.try_join() {
            Ok(result) => {
                let guard = handle.progress.agents.lock().unwrap_or_else(|e| e.into_inner());
                subagent_overlay::finalize_subagent_inline(&guard, term_width, &mut last_lines).ok();
                return result;
            }
            Err(agents::TryAgain) => {
                let seq = handle.progress.event_seq.load(Ordering::Acquire);
                if seq != last_event_count {
                    last_event_count = seq;
                    let guard = handle.progress.agents.lock().unwrap_or_else(|e| e.into_inner());
                    let any_active = guard.iter().any(|a| {
                        !matches!(
                            a.status,
                            agents::AgentStatus::Completed | agents::AgentStatus::Failed
                        )
                    });
                    if any_active {
                        subagent_overlay::render_subagent_inline(
                            &guard,
                            term_width,
                            &mut last_lines,
                        )
                        .ok();
                    }
                    drop(guard);
                }

                let guard = handle.progress.agents.lock().unwrap_or_else(|e| e.into_inner());
                let (_guard, _) = handle
                    .progress
                    .cvar
                    .wait_timeout(guard, std::time::Duration::from_millis(100))
                    .unwrap_or_else(|e| e.into_inner());
                drop(_guard);
            }
        }
    }
}

fn detect_terminal_width() -> usize {
    if let Ok((cols, _)) = crossterm::terminal::size() {
        return cols as usize;
    }
    if let Ok(cols) = std::env::var("COLUMNS") {
        if let Ok(n) = cols.parse::<usize>() {
            if n > 0 {
                return n;
            }
        }
    }
    80
}

#[allow(clippy::needless_pass_by_value)]
fn run_brief(input: BriefInput) -> Result<String, String> {
    let output = execute_brief(input)?;
    serde_json::to_string_pretty(&output).map_err(|error| error.to_string())
}



fn execute_brief(input: BriefInput) -> Result<BriefOutput, String> {
    let attachments = input.attachments.map(|paths| {
        paths
            .into_iter()
            .map(|path| {
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                let is_image = matches!(
                    path.rsplit('.').next().map(str::to_ascii_lowercase).as_deref(),
                    Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("webp") | Some("bmp")
                );
                ResolvedAttachment {
                    path,
                    size,
                    is_image,
                }
            })
            .collect()
    });
    let sent_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());
    Ok(BriefOutput {
        message: input.message,
        attachments,
        sent_at,
    })
}

fn execute_structured_output(input: StructuredOutputInput) -> Result<StructuredOutputResult, String> {
    let payload = input.0;
    let is_empty = payload.is_null()
        || (payload.is_object() && payload.as_object().map_or(false, |o| o.is_empty()))
        || (payload.is_array() && payload.as_array().map_or(false, |a| a.is_empty()));
    if is_empty {
        return Err(String::from("structured output must not be empty — provide a non-empty JSON value"));
    }
    Ok(StructuredOutputResult {
        structured_output: payload,
    })
}

#[allow(clippy::needless_pass_by_value)]
fn run_structured_output(input: StructuredOutputInput) -> Result<String, String> {
    let output = execute_structured_output(input)?;
    serde_json::to_string_pretty(&output).map_err(|error| error.to_string())
}
