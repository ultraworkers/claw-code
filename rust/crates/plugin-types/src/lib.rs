pub mod config;
pub mod lifecycle;
pub mod mcp;

pub use config::RuntimePluginConfig;
pub use lifecycle::{
    DegradedMode, DiscoveryResult, PluginHealthcheck, PluginLifecycle, PluginLifecycleEvent,
    PluginState, ResourceInfo, ServerHealth, ServerStatus, ToolInfo,
};
pub use mcp::{McpResourceInfo, McpToolInfo};
