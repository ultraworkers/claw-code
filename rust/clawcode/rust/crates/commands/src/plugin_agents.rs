use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{AgentSummary, DefinitionSource};

fn read_file_lossy(path: &Path) -> Result<String, std::io::Error> {
    let bytes = std::fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

pub fn load_plugin_agents(
    plugin_agent_paths: &BTreeMap<String, Vec<PathBuf>>,
) -> Vec<AgentSummary> {
    let mut agents = Vec::new();
    for (plugin_id, paths) in plugin_agent_paths {
        for path in paths {
            if !path.is_file() {
                continue;
            }
            let contents = match read_file_lossy(path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[plugin agents] error reading {}: {e}", path.display());
                    continue;
                }
            };
            let fm = plugins::frontmatter::parse_frontmatter(&contents)
                .ok()
                .map(|p| p.frontmatter);
            let fallback_name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            agents.push(AgentSummary {
                name: fm
                    .as_ref()
                    .and_then(|f| f.name.clone())
                    .unwrap_or(fallback_name),
                description: fm.as_ref().and_then(|f| f.description.clone()),
                model: fm.as_ref().and_then(|f| f.model.clone()),
                reasoning_effort: fm.as_ref().and_then(|f| f.reasoning_effort.clone()),
                mode: fm.as_ref().and_then(|f| f.mode.clone()),
                subagent_type: fm.as_ref().and_then(|f| f.subagent_type.clone()),
                tools: fm.as_ref().and_then(|f| f.tools.clone()),
                skills: fm.as_ref().and_then(|f| f.skills.clone()),
                permission: plugins::frontmatter::parse_permission_from_content(&contents),
                source: DefinitionSource::Plugin,
                shadowed_by: None,
                plugin: Some(plugin_id.clone()),
            });
        }
    }
    agents
}
