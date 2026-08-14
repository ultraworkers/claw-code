use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Running,
    Thinking,
    UsingTool,
    Completed,
    Failed,
}

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentStatus::Running => "Running",
            AgentStatus::Thinking => "Thinking",
            AgentStatus::UsingTool => "UsingTool",
            AgentStatus::Completed => "Completed",
            AgentStatus::Failed => "Failed",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum SubagentProgressEvent {
    Thinking { text: String },
    ToolCall { tool_name: String, input: Value },
    ToolResult { tool_name: String, truncated_result: String },
    StatusChange { status: AgentStatus },
    Completed { result_preview: String },
    Failed { error: String },
}

#[derive(Debug, Clone)]
pub struct AgentProgress {
    pub agent_id: String,
    pub name: String,
    pub subagent_type: String,
    pub status: AgentStatus,
    pub events: Vec<SubagentProgressEvent>,
    pub started_at: Instant,
    pub iteration_count: usize,
    pub final_event: Option<SubagentProgressEvent>,
    pub current_activity: Option<String>,
}

pub struct ProgressStore {
    pub agents: Mutex<Vec<AgentProgress>>,
    pub cvar: Condvar,
    pub event_seq: AtomicUsize,
}

pub type SharedProgress = Arc<ProgressStore>;

pub fn new_shared_progress() -> SharedProgress {
    Arc::new(ProgressStore {
        agents: Mutex::new(Vec::new()),
        cvar: Condvar::new(),
        event_seq: AtomicUsize::new(0),
    })
}

pub fn push_progress_event(
    shared: &SharedProgress,
    agent_id: &str,
    event: SubagentProgressEvent,
) {
    let mut guard = shared.agents.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(entry) = guard.iter_mut().find(|p| p.agent_id == agent_id) {
        if let SubagentProgressEvent::StatusChange { status } = &event {
            entry.status = *status;
            if *status == AgentStatus::UsingTool {
                entry.iteration_count += 1;
            }
        }

        match &event {
            SubagentProgressEvent::Completed { .. }
            | SubagentProgressEvent::Failed { .. } => {
                entry.final_event = Some(event.clone());
            }
            _ => {}
        }

        if entry.events.len() > 50 {
            entry.events.remove(0);
        }
        entry.events.push(event);
    }
    drop(guard);
    shared.event_seq.fetch_add(1, Ordering::Release);
    shared.cvar.notify_all();
}

pub fn set_current_activity(
    shared: &SharedProgress,
    agent_id: &str,
    activity: Option<String>,
) {
    let mut guard = shared.agents.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(entry) = guard.iter_mut().find(|p| p.agent_id == agent_id) {
        entry.current_activity = activity;
    }
    drop(guard);
    shared.event_seq.fetch_add(1, Ordering::Release);
    shared.cvar.notify_all();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    #[serde(rename = "agentId")]
    pub agent_id: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "subagentType")]
    pub subagent_type: Option<String>,
    pub model: Option<String>,
    /// Display-only agent mode echoed from the definition; not consumed by
    /// the runtime or any provider request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    /// `permission:` directives from the agent definition's frontmatter,
    /// as `tool-category → allow|deny|ask`. When present, the spawned
    /// sub-agent's `PermissionPolicy` is built with these as explicit rules
    /// (deny rules are effective even under `DangerFullAccess`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<u64>,
    #[serde(rename = "laneEvents", default, skip_serializing_if = "Vec::is_empty")]
    pub lane_events: Vec<runtime::LaneEvent>,
}

#[derive(Debug, Clone)]
pub struct AgentJob {
    pub manifest: AgentOutput,
    pub prompt: String,
    pub system_prompt: Vec<String>,
    pub allowed_tools: BTreeSet<String>,
    pub reasoning_effort: Option<String>,
    pub permission: Option<BTreeMap<String, String>>,
    /// Permission mode inherited from the parent session (permission
    /// passthrough). The sub-agent's `PermissionPolicy` is built with
    /// this mode as its base instead of always using
    /// `DangerFullAccess`.
    pub permission_mode: runtime::PermissionMode,
}

#[derive(Debug, Deserialize)]
pub struct AgentInput {
    pub description: String,
    pub prompt: String,
    pub subagent_type: Option<String>,
    pub name: Option<String>,
    pub model: Option<String>,
    /// Optional explicit system prompt (e.g. an `@agent` file's contents).
    /// When present, `execute_agent_with_spawn` uses it instead of deriving
    /// the prompt solely from `subagent_type` (which would drop the agent's
    /// own persona).
    #[serde(default)]
    pub system_prompt: Option<Vec<String>>,
    /// Optional allowed-tool allowlist. When present, overrides the tools
    /// inferred from `subagent_type`.
    #[serde(default)]
    pub allowed_tools: Option<BTreeSet<String>>,
    /// Optional agent mode (frontmatter `mode:`). Display-only: echoed into
    /// the manifest/report but NOT consumed by the runtime, spawn, or any
    /// provider request (MessageRequest has no `mode` field). Kept for
    /// reporting parity with the definition.
    #[serde(default)]
    pub mode: Option<String>,
    /// Optional reasoning-effort level (e.g. `low`/`medium`/`high`) forwarded
    /// to the provider's `MessageRequest`. When present, the spawned sub-agent
    /// runs with the agent definition's configured effort instead of the
    /// provider default.
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    /// Optional `permission:` directives from the agent file frontmatter
    /// (`tool-category → allow|deny|ask`). Honored as explicit rules on the
    /// spawned sub-agent's `PermissionPolicy`. Not advertised in the tool
    /// schema: the model must not be able to grant itself permissions.
    #[serde(default)]
    pub permission: Option<BTreeMap<String, String>>,
}
