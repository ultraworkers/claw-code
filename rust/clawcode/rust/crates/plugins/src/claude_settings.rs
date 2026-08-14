use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct ClaudeSettings {
    pub enabled_plugins: BTreeMap<String, bool>,
    pub mcp_servers: Option<Value>,
    pub installed_plugins: Vec<ClaudePluginEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudePluginEntry {
    pub name: String,
    pub source: String,
    pub version: Option<String>,
    pub install_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct InstalledPluginsV2 {
    pub plugins: BTreeMap<String, Vec<ClaudeInstallationEntry>>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // serde Deserialize struct — fields match JSON schema
struct ClaudeInstallationEntry {
    pub scope: String,
    #[serde(rename = "installPath")]
    pub install_path: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // serde Deserialize struct — fields match JSON schema
struct InstalledPluginsV1 {
    pub version: u32,
    pub plugins: Vec<ClaudePluginEntry>,
}

pub fn read_claude_settings(config_home: &Path) -> ClaudeSettings {
    let mut settings = ClaudeSettings::default();

    let Some(home) = config_home.parent() else {
        return settings;
    };
    let claude_dir = home.join(".claude");

    let settings_path = claude_dir.join("settings.json");
    if let Ok(contents) = fs::read_to_string(&settings_path) {
        if let Ok(json) = serde_json::from_str::<Value>(&contents) {
            if let Some(obj) = json.as_object() {
                if let Some(plugins) = obj.get("enabledPlugins").and_then(|v| v.as_object()) {
                    for (id, enabled) in plugins {
                        if let Some(b) = enabled.as_bool() {
                            settings.enabled_plugins.insert(id.clone(), b);
                        }
                    }
                }
                if let Some(servers) = obj.get("mcpServers") {
                    settings.mcp_servers = Some(servers.clone());
                }
            }
        }
    }

    let installed_path = claude_dir.join("plugins").join("installed_plugins.json");
    if let Ok(contents) = fs::read_to_string(&installed_path) {
        if let Ok(v2) = serde_json::from_str::<InstalledPluginsV2>(&contents) {
            for entries in v2.plugins.values() {
                for entry in entries {
                    settings.installed_plugins.push(ClaudePluginEntry {
                        name: String::new(),
                        source: String::new(),
                        version: Some(entry.version.clone()),
                        install_path: Some(entry.install_path.clone()),
                    });
                }
            }
        } else if let Ok(v1) = serde_json::from_str::<InstalledPluginsV1>(&contents) {
            settings.installed_plugins = v1.plugins;
        }
    }

    settings
}

pub fn merge_claude_plugin_states(
    claude: &ClaudeSettings,
    claw: &BTreeMap<String, bool>,
) -> BTreeMap<String, bool> {
    let mut merged = claw.clone();
    for (id, enabled) in &claude.enabled_plugins {
        merged.entry(id.clone()).or_insert(*enabled);
    }
    merged
}
