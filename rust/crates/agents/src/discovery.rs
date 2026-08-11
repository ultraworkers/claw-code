use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use runtime::strip_verbatim_prefix;

fn read_file_lossy(path: &Path) -> Result<String, std::io::Error> {
    let bytes = std::fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DefinitionSource {
    ProjectClaw,
    ProjectClaude,
    UserClawConfigHome,
    UserClaw,
    UserClaude,
    Plugin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DefinitionScope {
    Project,
    UserConfigHome,
    UserHome,
    Plugin,
}

impl DefinitionScope {
    pub fn label(self) -> &'static str {
        match self {
            Self::Project => "Project roots",
            Self::UserConfigHome => "User config roots",
            Self::UserHome => "User home roots",
            Self::Plugin => "Plugin agents",
        }
    }
}

impl DefinitionSource {
    pub fn report_scope(self) -> DefinitionScope {
        match self {
            Self::ProjectClaw | Self::ProjectClaude => {
                DefinitionScope::Project
            }
            Self::UserClawConfigHome => DefinitionScope::UserConfigHome,
            Self::UserClaw | Self::UserClaude => DefinitionScope::UserHome,
            Self::Plugin => DefinitionScope::Plugin,
        }
    }

    pub fn label(self) -> &'static str {
        self.report_scope().label()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSummary {
    pub name: String,
    pub description: Option<String>,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub source: DefinitionSource,
    pub shadowed_by: Option<DefinitionSource>,
    pub plugin: Option<String>,
    pub mode: Option<String>,
}

impl AgentSummary {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

pub struct AgentDiscovery {
    agents: Vec<AgentSummary>,
    active_names: Vec<String>,
}

impl AgentDiscovery {
    pub fn new(cwd: &Path) -> Self {
        let mut agents = Vec::new();
        let roots = discover_definition_roots(cwd, "agents");
        if let Ok(mut found) = load_agents_from_roots(&roots) {
            agents.append(&mut found);
        }
        agents.sort_by(|a, b| a.name.cmp(&b.name));
        let active_names = agents
            .iter()
            .filter(|a| a.shadowed_by.is_none())
            .map(|a| a.name.clone())
            .collect();
        Self { agents, active_names }
    }

    pub fn with_plugins(
        cwd: &Path,
        plugin_agent_paths: &BTreeMap<String, Vec<PathBuf>>,
    ) -> Self {
        let mut agents = Vec::new();
        let roots = discover_definition_roots(cwd, "agents");
        if let Ok(mut found) = load_agents_from_roots(&roots) {
            agents.append(&mut found);
        }
        let root_names: BTreeSet<String> = agents
            .iter()
            .filter(|a| a.shadowed_by.is_none())
            .map(|a| a.name.to_ascii_lowercase())
            .collect();
        let plugin_agents = load_plugin_agents(plugin_agent_paths);
        for mut agent in plugin_agents {
            if root_names.contains(&agent.name.to_ascii_lowercase()) {
                agent.shadowed_by = Some(DefinitionSource::ProjectClaw);
            }
            agents.push(agent);
        }
        agents.sort_by(|a, b| a.name.cmp(&b.name));
        let active_names = agents
            .iter()
            .filter(|a| a.shadowed_by.is_none())
            .map(|a| a.name.clone())
            .collect();
        Self { agents, active_names }
    }

    pub fn all(&self) -> &[AgentSummary] {
        &self.agents
    }

    pub fn active(&self) -> Vec<&AgentSummary> {
        self.agents
            .iter()
            .filter(|a| a.shadowed_by.is_none())
            .collect()
    }

    pub fn active_names(&self) -> &[String] {
        &self.active_names
    }

    pub fn active_names_list(&self) -> Vec<String> {
        self.active_names.clone()
    }

    pub fn find(&self, name: &str) -> Option<&AgentSummary> {
        let lowered = name.to_ascii_lowercase();
        self.agents
            .iter()
            .find(|a| a.shadowed_by.is_none() && a.name.to_ascii_lowercase() == lowered)
    }
}

fn discover_definition_roots(cwd: &Path, leaf: &str) -> Vec<(DefinitionSource, PathBuf)> {
    let mut roots = Vec::new();

    let home_boundaries: Vec<PathBuf> = std::env::var_os("HOME")
        .into_iter()
        .chain(std::env::var_os("USERPROFILE"))
        .filter_map(|p| std::fs::canonicalize(PathBuf::from(p)).ok())
        .collect();

    for ancestor in cwd.ancestors() {
        if home_boundaries.iter().any(|b| {
            if let Ok(canon_ancestor) = std::fs::canonicalize(ancestor) {
                b == &canon_ancestor
            } else {
                false
            }
        }) {
            break;
        }
        push_unique_root(&mut roots, DefinitionSource::ProjectClaw, ancestor.join(".claw").join(leaf));
        push_unique_root(&mut roots, DefinitionSource::ProjectClaude, ancestor.join(".claude").join(leaf));
    }

    if let Ok(claw_config_home) = std::env::var("CLAW_CONFIG_HOME") {
        push_unique_root(&mut roots, DefinitionSource::UserClawConfigHome, PathBuf::from(claw_config_home).join(leaf));
    }

    if let Ok(claude_config_dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        push_unique_root(&mut roots, DefinitionSource::UserClaude, PathBuf::from(claude_config_dir).join(leaf));
    }

    let home = home_boundaries.first().cloned();
    if let Some(ref home) = home {
        let home = strip_verbatim_prefix(home.clone());
        push_unique_root(&mut roots, DefinitionSource::UserClaw, home.join(".claw").join(leaf));
        push_unique_root(&mut roots, DefinitionSource::UserClaude, home.join(".claude").join(leaf));
    }

    roots
}

/// Returns the root directories that may contain agent definitions,
/// in discovery-priority order (project → config-home → user-home).
/// Uses the same search logic as [`AgentDiscovery`].
pub fn discover_agent_roots(cwd: &Path) -> Vec<PathBuf> {
    discover_definition_roots(cwd, "agents")
        .into_iter()
        .map(|(_, path)| path)
        .collect()
}

fn push_unique_root(
    roots: &mut Vec<(DefinitionSource, PathBuf)>,
    source: DefinitionSource,
    path: PathBuf,
) {
    if path.is_dir() && !roots.iter().any(|(_, existing)| existing == &path) {
        roots.push((source, path));
    }
}

fn load_agents_from_roots(
    roots: &[(DefinitionSource, PathBuf)],
) -> Result<Vec<AgentSummary>, String> {
    let mut agents = Vec::new();
    let mut active_sources = BTreeMap::<String, DefinitionSource>::new();

    for (source, root) in roots {
        let mut root_agents = Vec::new();
        let dir = match std::fs::read_dir(root) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[agents] warning: could not read {root:?}: {e}");
                continue;
            }
        };
        for entry in dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let skill_path = path.join("SKILL.md");
                if skill_path.is_file() {
                    if let Ok(contents) = read_file_lossy(&skill_path) {
                        let fm = plugins::frontmatter::parse_frontmatter(&contents)
                            .ok()
                            .map(|p| p.frontmatter);
                        let name = fm
                            .as_ref()
                            .and_then(|f| f.name.clone())
                            .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
                        root_agents.push(AgentSummary {
                            name,
                            description: fm.as_ref().and_then(|f| f.description.clone()),
                            model: fm.as_ref().and_then(|f| f.model.clone()),
                            reasoning_effort: fm.as_ref().and_then(|f| f.reasoning_effort.clone()),
                            mode: fm.as_ref().and_then(|f| f.mode.clone()),
                            source: *source,
                            shadowed_by: None,
                            plugin: None,
                        });
                    }
                    continue;
                }
            }

            if path.extension().is_some_and(|ext| ext == "md") {
                if let Ok(contents) = read_file_lossy(&path) {
                    let fm = plugins::frontmatter::parse_frontmatter(&contents)
                        .ok()
                        .map(|p| p.frontmatter);
                    let fallback_name = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
                    root_agents.push(AgentSummary {
                        name: fm
                            .as_ref()
                            .and_then(|f| f.name.clone())
                            .unwrap_or(fallback_name),
                        description: fm.as_ref().and_then(|f| f.description.clone()),
                        model: fm.as_ref().and_then(|f| f.model.clone()),
                        reasoning_effort: fm.as_ref().and_then(|f| f.reasoning_effort.clone()),
                        mode: fm.as_ref().and_then(|f| f.mode.clone()),
                        source: *source,
                        shadowed_by: None,
                        plugin: None,
                    });
                }
                continue;
            }

            if path.extension().is_none_or(|ext| ext != "toml") {
                continue;
            }
            if let Ok(contents) = read_file_lossy(&path) {
                let fallback_name = path.file_stem().map_or_else(
                    || entry.file_name().to_string_lossy().to_string(),
                    |stem| stem.to_string_lossy().to_string(),
                );
                root_agents.push(AgentSummary {
                    name: parse_toml_string(&contents, "name").unwrap_or(fallback_name),
                    description: parse_toml_string(&contents, "description"),
                    model: parse_toml_string(&contents, "model"),
                    reasoning_effort: parse_toml_string(&contents, "model_reasoning_effort"),
                    mode: parse_toml_string(&contents, "mode"),
                    source: *source,
                    shadowed_by: None,
                    plugin: None,
                });
            }
        }
        root_agents.sort_by(|left, right| left.name.cmp(&right.name));

        for mut agent in root_agents {
            let key = agent.name.to_ascii_lowercase();
            if let Some(existing) = active_sources.get(&key) {
                agent.shadowed_by = Some(*existing);
            } else {
                active_sources.insert(key, agent.source);
            }
            agents.push(agent);
        }
    }

    Ok(agents)
}

fn load_plugin_agents(
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
                source: DefinitionSource::Plugin,
                shadowed_by: None,
                plugin: Some(plugin_id.clone()),
            });
        }
    }
    agents
}

fn parse_toml_string(contents: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} =");
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        let Some(value) = trimmed.strip_prefix(&prefix) else {
            continue;
        };
        let value = value.trim();
        let Some(value) = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
        else {
            continue;
        };
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

pub fn render_agents_report(agents: &[AgentSummary]) -> String {
    if agents.is_empty() {
        return "No agents found.".to_string();
    }

    let total_active = agents
        .iter()
        .filter(|agent| agent.shadowed_by.is_none())
        .count();
    let mut lines = vec![
        "Agents".to_string(),
        format!("  {total_active} active agents"),
        String::new(),
    ];

    for scope in [
        DefinitionScope::Project,
        DefinitionScope::UserConfigHome,
        DefinitionScope::UserHome,
        DefinitionScope::Plugin,
    ] {
        let group = agents
            .iter()
            .filter(|agent| agent.source.report_scope() == scope)
            .collect::<Vec<_>>();
        if group.is_empty() {
            continue;
        }

        lines.push(format!("{}:", scope.label()));
        for agent in group {
            let detail = agent_detail(agent);
            match agent.shadowed_by {
                Some(winner) => lines.push(format!("  (shadowed by {}) {detail}", winner.label())),
                None => lines.push(format!("  {detail}")),
            }
        }
        lines.push(String::new());
    }

    lines.join("\n").trim_end().to_string()
}

pub fn render_agents_report_json(
    cwd: &Path,
    agents: &[AgentSummary],
) -> serde_json::Value {
    let active = agents
        .iter()
        .filter(|agent| agent.shadowed_by.is_none())
        .count();
    serde_json::json!({
        "kind": "agents",
        "action": "list",
        "count": agents.len(),
        "summary": {
            "total": agents.len(),
            "active": active,
            "shadowed": agents.len().saturating_sub(active),
        },
        "working_directory": cwd.display().to_string(),
        "agents": agents.iter().map(agent_summary_json).collect::<Vec<_>>(),
    })
}

pub fn definition_source_id(source: DefinitionSource) -> &'static str {
    match source {
        DefinitionSource::ProjectClaw | DefinitionSource::ProjectClaude => "project_claw",
        DefinitionSource::UserClawConfigHome => "user_claw_config_home",
        DefinitionSource::UserClaw | DefinitionSource::UserClaude => "user_claw",
        DefinitionSource::Plugin => "plugin",
    }
}

pub fn definition_source_json(source: DefinitionSource) -> serde_json::Value {
    serde_json::json!({
        "id": definition_source_id(source),
        "label": source.label(),
    })
}

fn agent_detail(agent: &AgentSummary) -> String {
    let mut parts = vec![agent.name.clone()];
    if let Some(description) = &agent.description {
        parts.push(description.clone());
    }
    if let Some(model) = &agent.model {
        parts.push(model.clone());
    }
    if let Some(reasoning) = &agent.reasoning_effort {
        parts.push(reasoning.clone());
    }
    if let Some(mode) = &agent.mode {
        parts.push(format!("[{mode}]"));
    }
    if let Some(plugin) = &agent.plugin {
        parts.push(format!("[{plugin}]"));
    }
    parts.join(" \u{b7} ")
}

fn agent_summary_json(agent: &AgentSummary) -> serde_json::Value {
    serde_json::json!({
        "name": &agent.name,
        "description": &agent.description,
        "model": &agent.model,
        "reasoning_effort": &agent.reasoning_effort,
        "mode": &agent.mode,
        "source": definition_source_json(agent.source),
        "active": agent.shadowed_by.is_none(),
        "shadowed_by": agent.shadowed_by.map(definition_source_json),
        "plugin": &agent.plugin,
    })
}
