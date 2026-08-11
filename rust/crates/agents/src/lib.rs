//! Sub-agent subsystem.
//!

pub mod discovery;
mod normalize;
mod persist;
mod runtime;
mod spawn;
pub mod types;

pub use self::discovery::{
    definition_source_id, definition_source_json, discover_agent_roots, render_agents_report,
    render_agents_report_json, AgentDiscovery, AgentSummary, DefinitionScope, DefinitionSource,
};
pub use self::normalize::{allowed_tools_for_subagent, normalize_subagent_type, SubagentKind};
pub use self::persist::{
    extract_commit_sha, make_agent_id, slugify_agent_name,
};
pub use self::runtime::{
    build_agent_runtime, build_agent_runtime_inner, build_agent_system_prompt,
    init_global_runtime, register_runtime_tool_provider, register_tool_executor,
    registered_extra_tool_defs, resolve_agent_model, ProviderRuntimeClient, SubagentToolExecutor,
    RuntimeToolExecutorFn,
};
pub use self::spawn::{spawn_agent_task, spawn_agent_task_with_progress, AgentHandle, TryAgain};
pub use self::types::{
    AgentInput, AgentJob, AgentOutput, AgentProgress, AgentStatus, ProgressStore, SharedProgress,
    SubagentProgressEvent, new_shared_progress, push_progress_event, set_current_activity,
};
