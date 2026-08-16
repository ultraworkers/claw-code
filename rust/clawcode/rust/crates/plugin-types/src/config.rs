use std::collections::BTreeMap;

/// Parsed plugin-related settings extracted from runtime config.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimePluginConfig {
    pub enabled_plugins: BTreeMap<String, bool>,
    pub external_directories: Vec<String>,
    pub install_root: Option<String>,
    pub registry_path: Option<String>,
    pub max_output_tokens: Option<u32>,
    /// Default reasoning-effort level (e.g. `low`/`medium`/`high`/`max`/`off`)
    /// applied when no CLI flag or agent frontmatter selects one. Lower
    /// precedence than the `CLAW_REASONING_EFFORT` env var. Validated against
    /// the resolved model at request time.
    pub reasoning_effort: Option<String>,
}

impl RuntimePluginConfig {
    #[must_use]
    pub fn enabled_plugins(&self) -> &BTreeMap<String, bool> {
        &self.enabled_plugins
    }

    #[must_use]
    pub fn external_directories(&self) -> &[String] {
        &self.external_directories
    }

    #[must_use]
    pub fn install_root(&self) -> Option<&str> {
        self.install_root.as_deref()
    }

    #[must_use]
    pub fn registry_path(&self) -> Option<&str> {
        self.registry_path.as_deref()
    }

    #[must_use]
    pub fn max_output_tokens(&self) -> Option<u32> {
        self.max_output_tokens
    }

    pub fn set_max_output_tokens(&mut self, max_output_tokens: Option<u32>) {
        self.max_output_tokens = max_output_tokens;
    }

    /// The default reasoning-effort level from `settings.json`
    /// (`plugins.reasoningEffort`), or `None` when unset. Lower precedence than
    /// the `CLAW_REASONING_EFFORT` env var and any CLI flag or agent
    /// frontmatter value.
    #[must_use]
    pub fn reasoning_effort(&self) -> Option<&str> {
        self.reasoning_effort.as_deref()
    }

    pub fn set_plugin_state(&mut self, plugin_id: String, enabled: bool) {
        self.enabled_plugins.insert(plugin_id, enabled);
    }

    #[must_use]
    pub fn state_for(&self, plugin_id: &str, default_enabled: bool) -> bool {
        self.enabled_plugins
            .get(plugin_id)
            .copied()
            .unwrap_or(default_enabled)
    }
}
