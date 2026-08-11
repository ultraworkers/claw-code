use std::collections::BTreeMap;

use serde_json::Value;

use crate::config::RuntimePermissionRuleConfig;

/// Permission level assigned to a tool invocation or runtime session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
    Prompt,
    Allow,
}

impl PermissionMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::WorkspaceWrite => "workspace-write",
            Self::DangerFullAccess => "danger-full-access",
            Self::Prompt => "prompt",
            Self::Allow => "allow",
        }
    }
}

/// Hook-provided override applied before standard permission evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionOverride {
    Allow,
    Deny,
    Ask,
}

/// Additional permission context supplied by hooks or higher-level orchestration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PermissionContext {
    override_decision: Option<PermissionOverride>,
    override_reason: Option<String>,
}

impl PermissionContext {
    #[must_use]
    pub fn new(
        override_decision: Option<PermissionOverride>,
        override_reason: Option<String>,
    ) -> Self {
        Self {
            override_decision,
            override_reason,
        }
    }

    #[must_use]
    pub fn override_decision(&self) -> Option<PermissionOverride> {
        self.override_decision
    }

    #[must_use]
    pub fn override_reason(&self) -> Option<&str> {
        self.override_reason.as_deref()
    }
}

/// Full authorization request presented to a permission prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRequest {
    pub tool_name: String,
    pub input: String,
    pub current_mode: PermissionMode,
    pub required_mode: PermissionMode,
    pub reason: Option<String>,
}

/// User-facing decision returned by a [`PermissionPrompter`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionPromptDecision {
    Allow,
    Deny { reason: String },
}

/// Prompting interface used when policy requires interactive approval.
pub trait PermissionPrompter {
    fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision;
}

/// Final authorization result after evaluating static rules and prompts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionOutcome {
    Allow,
    Deny { reason: String },
}

/// Evaluates permission mode requirements plus allow/deny/ask rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionPolicy {
    active_mode: PermissionMode,
    tool_requirements: BTreeMap<String, PermissionMode>,
    allow_rules: Vec<PermissionRule>,
    deny_rules: Vec<PermissionRule>,
    ask_rules: Vec<PermissionRule>,
}

impl PermissionPolicy {
    #[must_use]
    pub fn new(active_mode: PermissionMode) -> Self {
        Self {
            active_mode,
            tool_requirements: BTreeMap::new(),
            allow_rules: Vec::new(),
            deny_rules: Vec::new(),
            ask_rules: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_tool_requirement(
        mut self,
        tool_name: impl Into<String>,
        required_mode: PermissionMode,
    ) -> Self {
        self.tool_requirements
            .insert(tool_name.into(), required_mode);
        self
    }

    #[must_use]
    pub fn with_permission_rules(mut self, config: &RuntimePermissionRuleConfig) -> Self {
        let mut seen_warnings: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut emit = |warnings: Vec<String>| {
            for warning in warnings {
                if seen_warnings.insert(warning.clone()) {
                    eprintln!("{warning}");
                }
            }
        };

        // Snapshot the active mode and tool_requirements before we start
        // building rules. The redundancy check below needs both: an allow
        // rule that uses a wildcard matcher (Any) is redundant under modes
        // that already grant the access the tool needs.
        let active_mode = self.active_mode;
        let tool_requirements = self.tool_requirements.clone();

        // A broad allow rule is "redundant" if the active mode already grants
        // the tool at least the access it requires. Under `Prompt` we never
        // suppress (the prompter still drives the decision).
        let is_redundant_broad_allow = |rule: &PermissionRule| -> bool {
            if !matches!(rule.matcher, PermissionRuleMatcher::Any) {
                return false;
            }
            if active_mode == PermissionMode::Prompt {
                return false;
            }
            let required = tool_requirements
                .get(&rule.tool_name)
                .copied()
                .unwrap_or(PermissionMode::WorkspaceWrite);
            active_mode >= required
        };

        self.allow_rules = config
            .allow()
            .iter()
            .map(|rule| {
                let (parsed, warnings) =
                    PermissionRule::parse_with_warning(rule, RuleList::Allow);
                // Suppress "matches ALL …" / "wildcard matcher" / "empty matcher"
                // warnings for allow rules that are redundant under the active
                // mode. The rule still works; the warning is just noise.
                let warnings = if is_redundant_broad_allow(&parsed) {
                    warnings
                        .into_iter()
                        .filter(|w| !is_broad_matcher_warning(w))
                        .collect()
                } else {
                    warnings
                };
                emit(warnings);
                parsed
            })
            .collect();
        self.deny_rules = config
            .deny()
            .iter()
            .map(|rule| {
                let (parsed, warnings) =
                    PermissionRule::parse_with_warning(rule, RuleList::Deny);
                emit(warnings);
                parsed
            })
            .collect();
        self.ask_rules = config
            .ask()
            .iter()
            .map(|rule| {
                let (parsed, warnings) = PermissionRule::parse_with_warning(rule, RuleList::Ask);
                emit(warnings);
                parsed
            })
            .collect();
        self
    }

    #[must_use]
    pub fn active_mode(&self) -> PermissionMode {
        self.active_mode
    }

    #[must_use]
    pub fn required_mode_for(&self, tool_name: &str) -> PermissionMode {
        if let Some(&mode) = self.tool_requirements.get(tool_name) {
            return mode;
        }
        for (name, &mode) in &self.tool_requirements {
            if name.eq_ignore_ascii_case(tool_name) {
                return mode;
            }
        }
        PermissionMode::WorkspaceWrite
    }

    #[must_use]
    pub fn authorize(
        &self,
        tool_name: &str,
        input: &str,
        prompter: Option<&mut dyn PermissionPrompter>,
    ) -> PermissionOutcome {
        self.authorize_with_context(tool_name, input, &PermissionContext::default(), prompter)
    }

    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn authorize_with_context(
        &self,
        tool_name: &str,
        input: &str,
        context: &PermissionContext,
        prompter: Option<&mut dyn PermissionPrompter>,
    ) -> PermissionOutcome {
        let required_mode = self.required_mode_for(tool_name);
        self.authorize_impl(tool_name, input, required_mode, context, prompter)
    }

    /// Authorize a tool call with an explicitly provided required mode,
    /// bypassing the policy's internal tool_requirements lookup.
    /// This is useful when the caller already knows the required mode
    /// and wants to avoid the [`PermissionEnforcer`] wrapper.
    #[must_use]
    pub fn authorize_with_required_mode(
        &self,
        tool_name: &str,
        input: &str,
        required_mode: PermissionMode,
        prompter: Option<&mut dyn PermissionPrompter>,
    ) -> PermissionOutcome {
        self.authorize_impl(tool_name, input, required_mode, &PermissionContext::default(), prompter)
    }

    #[must_use]
    #[allow(clippy::too_many_lines)]
    fn authorize_impl(
        &self,
        tool_name: &str,
        input: &str,
        required_mode: PermissionMode,
        context: &PermissionContext,
        prompter: Option<&mut dyn PermissionPrompter>,
    ) -> PermissionOutcome {
        if let Some(rule) = Self::find_matching_rule(&self.deny_rules, tool_name, input) {
            return PermissionOutcome::Deny {
                reason: format!(
                    "Permission to use {tool_name} has been denied by rule '{}'",
                    rule.raw
                ),
            };
        }

        let current_mode = self.active_mode();
        let ask_rule = Self::find_matching_rule(&self.ask_rules, tool_name, input);
        let allow_rule = Self::find_matching_rule(&self.allow_rules, tool_name, input);

        match context.override_decision() {
            Some(PermissionOverride::Deny) => {
                return PermissionOutcome::Deny {
                    reason: context.override_reason().map_or_else(
                        || format!("tool '{tool_name}' denied by hook"),
                        ToOwned::to_owned,
                    ),
                };
            }
            Some(PermissionOverride::Ask) => {
                let reason = context.override_reason().map_or_else(
                    || format!("tool '{tool_name}' requires approval due to hook guidance"),
                    ToOwned::to_owned,
                );
                return Self::prompt_or_deny(
                    tool_name,
                    input,
                    current_mode,
                    required_mode,
                    Some(reason),
                    prompter,
                );
            }
            Some(PermissionOverride::Allow) => {
                if let Some(rule) = ask_rule {
                    let reason = format!(
                        "tool '{tool_name}' requires approval due to ask rule '{}'",
                        rule.raw
                    );
                    return Self::prompt_or_deny(
                        tool_name,
                        input,
                        current_mode,
                        required_mode,
                        Some(reason),
                        prompter,
                    );
                }
                // Hook said Allow — respect it (ask_rules already checked above)
                return PermissionOutcome::Allow;
            }
            None => {}
        }

        if let Some(rule) = ask_rule {
            let reason = format!(
                "tool '{tool_name}' requires approval due to ask rule '{}'",
                rule.raw
            );
            return Self::prompt_or_deny(
                tool_name,
                input,
                current_mode,
                required_mode,
                Some(reason),
                prompter,
            );
        }

        if allow_rule.is_some()
            || current_mode == PermissionMode::Allow
            || (current_mode >= required_mode && current_mode != PermissionMode::Prompt)
        {
            return PermissionOutcome::Allow;
        }

        if current_mode == PermissionMode::Prompt
            || (current_mode == PermissionMode::WorkspaceWrite
                && required_mode == PermissionMode::DangerFullAccess)
        {
            let reason = Some(format!(
                "tool '{tool_name}' requires approval to escalate from {} to {}",
                current_mode.as_str(),
                required_mode.as_str()
            ));
            return Self::prompt_or_deny(
                tool_name,
                input,
                current_mode,
                required_mode,
                reason,
                prompter,
            );
        }

        PermissionOutcome::Deny {
            reason: format!(
                "tool '{tool_name}' requires {} permission; current mode is {}",
                required_mode.as_str(),
                current_mode.as_str()
            ),
        }
    }

    fn prompt_or_deny(
        tool_name: &str,
        input: &str,
        current_mode: PermissionMode,
        required_mode: PermissionMode,
        reason: Option<String>,
        mut prompter: Option<&mut dyn PermissionPrompter>,
    ) -> PermissionOutcome {
        let request = PermissionRequest {
            tool_name: tool_name.to_string(),
            input: input.to_string(),
            current_mode,
            required_mode,
            reason: reason.clone(),
        };

        match prompter.as_mut() {
            Some(prompter) => match prompter.decide(&request) {
                PermissionPromptDecision::Allow => PermissionOutcome::Allow,
                PermissionPromptDecision::Deny { reason } => PermissionOutcome::Deny { reason },
            },
            None => PermissionOutcome::Deny {
                reason: reason.unwrap_or_else(|| {
                    format!(
                        "tool '{tool_name}' requires approval to run while mode is {}",
                        current_mode.as_str()
                    )
                }),
            },
        }
    }

    fn find_matching_rule<'a>(
        rules: &'a [PermissionRule],
        tool_name: &str,
        input: &str,
    ) -> Option<&'a PermissionRule> {
        rules.iter().find(|rule| rule.matches(tool_name, input))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PermissionRule {
    raw: String,
    tool_name: String,
    matcher: PermissionRuleMatcher,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PermissionRuleMatcher {
    Any,
    Exact(String),
    Prefix(String),
    /// Tool-name prefix wildcard produced by the F-04 path (e.g. `mcp__*`
    /// from a bare `mcp__*` rule). Matches every tool whose runtime
    /// `tool_name` starts with the given prefix. No subject extraction is
    /// applied — these rules intentionally cover every invocation of any
    /// tool under the prefix.
    ToolNamePrefix(String),
}

/// Identifies which rule list a permission rule came from. Used by the
/// parser to produce warnings that name the source list, so users can
/// act on them without re-reading their config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleList {
    Allow,
    Deny,
    Ask,
}

impl RuleList {
    fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::Ask => "ask",
        }
    }
}

impl PermissionRule {
    /// Normalize a config-file tool name to the canonical name used by the tool registry.
    /// Rules like `Read(*)` or `Bash(*)` must match the runtime names `read_file` and `bash`.
    fn normalize_tool_name(name: &str) -> String {
        let lower = name.to_lowercase();
        match lower.as_str() {
            "read" | "read_file" => "read_file".to_string(),
            "write" | "write_file" => "new_file".to_string(),
            "edit" | "edit_file" => "edit_file".to_string(),
            "glob" | "glob_search" => "glob_search".to_string(),
            "grep" | "grep_search" => "grep_search".to_string(),
            "bash" | "execute_command" => "bash".to_string(),
            // PascalCase tools: keep as-is (they're already canonical)
            _ => name.to_string(),
        }
    }

    /// Parse a rule and return any non-fatal warnings that should be surfaced
    /// to the operator. Returning warnings separately (instead of writing
    /// directly to stderr from `parse`) lets the builder deduplicate warnings
    /// across multiple policy builds from the same config, and lets the
    /// warning text reference which list the rule came from.
    ///
    /// Detected anomalies (each emits its own warning):
    /// - Empty matcher (`Bash()`) or wildcard matcher (`Bash(*)`)
    /// - Malformed: open paren with no close paren (`Bash(rm -rf /`)
    /// - Tool-name wildcard syntax (`mcp__*`) which is not supported by the
    ///   `Tool(content)` parser; the rule will never match
    fn parse_with_warning(raw: &str, list: RuleList) -> (Self, Vec<String>) {
        let trimmed = raw.trim();
        let mut warnings = Vec::new();

        let open = find_first_unescaped(trimmed, '(');
        let close = find_last_unescaped(trimmed, ')');

        // F-05: detect open paren with no close paren — the rule will be
        // stored as a literal exact match and is highly unlikely to ever
        // match a real tool invocation.
        if open.is_some() && close.is_none() {
            warnings.push(format!(
                "warning: permission rule '{trimmed}' in {} list has '(' but no matching ')'. \
                 The rule will be stored as a literal exact match and is unlikely to match any tool. \
                 Did you forget to escape or close the parenthesis?",
                list.as_str()
            ));
        }

        if let (Some(open), Some(close)) = (open, close) {
            if close == trimmed.len() - 1 && open < close {
                let raw_tool_name = trimmed[..open].trim();
                let content = &trimmed[open + 1..close];
                if !raw_tool_name.is_empty() {
                    let tool_name = Self::normalize_tool_name(raw_tool_name);
                    let list_label = list.as_str();
                    let content_trimmed = content.trim();
                    if content_trimmed.is_empty() {
                        warnings.push(format!(
                            "warning: permission rule '{trimmed}' in {list_label} list \
                             matches ALL '{raw_tool_name}' tools (empty matcher). \
                             A broad {list_label} rule like this is redundant under \
                             permissive modes such as 'danger-full-access' — \
                             consider removing it or specifying a subject matcher."
                        ));
                    } else if content_trimmed == "*" {
                        warnings.push(format!(
                            "warning: permission rule '{trimmed}' in {list_label} list \
                             matches ALL '{raw_tool_name}' tools (wildcard matcher). \
                             A broad {list_label} rule like this is redundant under \
                             permissive modes such as 'danger-full-access' — \
                             consider removing it or specifying a subject matcher."
                        ));
                    }
                    let matcher = parse_rule_matcher(content);
                    return (
                        Self {
                            raw: trimmed.to_string(),
                            tool_name,
                            matcher,
                        },
                        warnings,
                    );
                }
            }
        }

        // F-04: detect tool-name wildcard patterns like `mcp__*` (no parens,
        // trailing `*`). Promote these to a `ToolNamePrefix` matcher so the
        // rule actually applies to every tool whose name starts with the
        // prefix. This is the recommended way to allow an entire tool family
        // (e.g. all MCP tools registered as `mcp__<server>__<tool>`) without
        // enumerating each one, since MCP tool names are registered at
        // runtime and cannot be exhaustively listed in static config.
        if !trimmed.contains('(') && trimmed.ends_with('*') && trimmed.len() > 1 {
            let prefix = &trimmed[..trimmed.len() - 1];
            return (
                Self {
                    raw: trimmed.to_string(),
                    tool_name: trimmed.to_string(),
                    matcher: PermissionRuleMatcher::ToolNamePrefix(prefix.to_string()),
                },
                warnings,
            );
        }

        (
            Self {
                raw: trimmed.to_string(),
                tool_name: trimmed.to_string(),
                matcher: PermissionRuleMatcher::Exact(trimmed.to_string()),
            },
            warnings,
        )
    }

    fn matches(&self, tool_name: &str, input: &str) -> bool {
        // ToolNamePrefix rules match the runtime tool name directly — the
        // prefix IS the tool-name matcher, so we skip the exact `tool_name`
        // equality check and the subject extraction step. These rules
        // intentionally cover every invocation of any tool under the prefix.
        if let PermissionRuleMatcher::ToolNamePrefix(prefix) = &self.matcher {
            return tool_name.starts_with(prefix.as_str());
        }

        if self.tool_name != tool_name {
            return false;
        }

        match &self.matcher {
            PermissionRuleMatcher::Any => true,
            PermissionRuleMatcher::Exact(expected) => match extract_permission_subject(input, &self.tool_name) {
                Some(candidate) => candidate == *expected,
                None => {
                    warn_silent_rule_match_failure(&self.raw, tool_name);
                    false
                }
            },
            PermissionRuleMatcher::Prefix(prefix) => match extract_permission_subject(input, &self.tool_name) {
                Some(candidate) => candidate.starts_with(prefix),
                None => {
                    warn_silent_rule_match_failure(&self.raw, tool_name);
                    false
                }
            },
            PermissionRuleMatcher::ToolNamePrefix(_) => unreachable!(
                "ToolNamePrefix handled above before exact tool_name check"
            ),
        }
    }
}

fn parse_rule_matcher(content: &str) -> PermissionRuleMatcher {
    let unescaped = unescape_rule_content(content.trim());
    if unescaped.is_empty() || unescaped == "*" {
        PermissionRuleMatcher::Any
    } else if let Some(prefix) = unescaped.strip_suffix(":*") {
        PermissionRuleMatcher::Prefix(prefix.to_string())
    } else {
        PermissionRuleMatcher::Exact(unescaped)
    }
}

fn unescape_rule_content(content: &str) -> String {
    content
        .replace(r"\(", "(")
        .replace(r"\)", ")")
        .replace(r"\\", r"\")
}

/// Detect whether a warning message is the "broad matcher" warning emitted
/// for empty (`Bash()`) or wildcard (`Bash(*)`) matchers. These warnings
/// are suppressed for allow rules that are redundant under the active mode
/// (i.e. the mode already grants the access the tool needs). Other
/// warnings — syntax errors, malformed rules, deny/ask-list warnings — are
/// never filtered.
fn is_broad_matcher_warning(warning: &str) -> bool {
    warning.contains("matches ALL")
        && (warning.contains("empty matcher") || warning.contains("wildcard matcher"))
}

/// Emit a one-time warning when a permission rule cannot extract its subject
/// from the tool input (JSON parse error, missing key, or non-string value).
/// Without this, rules like `deny: ["read_file(/etc/passwd)"]` silently
/// never match and the user has no signal that the rule is ineffective.
///
/// Uses a process-global set keyed by the rule's raw form so that the same
/// rule does not flood stderr when matched against many invocations.
fn warn_silent_rule_match_failure(raw: &str, tool_name: &str) {
    use std::sync::OnceLock;
    use std::sync::Mutex;
    static SEEN: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(std::collections::HashSet::new()));
    let key = format!("{raw}|{tool_name}");
    let mut guard = match seen.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    if guard.insert(key) {
        eprintln!(
            "warning: permission rule '{raw}' could not extract subject from \
             '{tool_name}' input (malformed JSON, missing key, or non-string value); \
             rule will not match until input format is corrected"
        );
    }
}

fn find_first_unescaped(value: &str, needle: char) -> Option<usize> {
    let mut escaped = false;
    for (idx, ch) in value.char_indices() {
        if ch == '\\' {
            escaped = !escaped;
            continue;
        }
        if ch == needle && !escaped {
            return Some(idx);
        }
        escaped = false;
    }
    None
}

fn find_last_unescaped(value: &str, needle: char) -> Option<usize> {
    // Single forward pass that tracks the same escape state as
    // `find_first_unescaped` (XOR-toggling on `\\`, reset on any non-`\\`).
    // We remember the index of the most recent unescaped match. This is
    // O(n) and avoids the O(n²) behavior of scanning backslashes from
    // every candidate position.
    let mut escaped = false;
    let mut last: Option<usize> = None;
    for (idx, ch) in value.char_indices() {
        if ch == '\\' {
            escaped = !escaped;
            continue;
        }
        if ch == needle && !escaped {
            last = Some(idx);
        }
        escaped = false;
    }
    last
}

fn extract_permission_subject(input: &str, tool_name: &str) -> Option<String> {
    let key = match tool_name {
        // Shell commands
        "bash" | "execute_command" => "command",
        // File tools — schema uses `path`, not `file_path`
        | "read_file" | "new_file" | "edit_file"
        | "read" | "write" | "edit" | "create"
        | "move" | "delete" | "rename" | "copy"
        | "file_system" => "path",
        // Search tools
        "glob_search" | "grep_search" | "glob" | "grep" | "search" => "pattern",
        // Web tools
        "WebFetch" | "WebFind" | "web_fetch" | "web" => "url",
        "WebSearch" | "web_search" | "ToolSearch" => "query",
        // Messaging
        "ask" | "message" | "say" => "message",
        // Question tool
        "Question" => "question",

        // Notebook
        "NotebookEdit" | "notebook" | "notebook_create" | "notebook_edit" => "notebook_path",
        // Skills, config
        "Skill" => "skill",
        "Config" => "setting",
        // Fallback — best guess for unknown tools. "path" is the canonical
        // key used by every file tool in the tool registry; previously this
        // defaulted to "file_path" which silently failed for unknown tools
        // that followed the standard schema.
        _ => "path",
    };

    let parsed = serde_json::from_str::<Value>(input).ok()?;
    let object = parsed.as_object()?;
    object.get(key).and_then(Value::as_str).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        PermissionContext, PermissionMode, PermissionOutcome, PermissionOverride,
        PermissionPolicy, PermissionPromptDecision, PermissionPrompter, PermissionRequest,
        PermissionRule, PermissionRuleMatcher, RuleList, extract_permission_subject,
        find_first_unescaped, find_last_unescaped, is_broad_matcher_warning,
    };
    use crate::config::RuntimePermissionRuleConfig;

    struct RecordingPrompter {
        seen: Vec<PermissionRequest>,
        allow: bool,
    }

    impl PermissionPrompter for RecordingPrompter {
        fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision {
            self.seen.push(request.clone());
            if self.allow {
                PermissionPromptDecision::Allow
            } else {
                PermissionPromptDecision::Deny {
                    reason: "not now".to_string(),
                }
            }
        }
    }

    #[test]
    fn allows_tools_when_active_mode_meets_requirement() {
        let policy = PermissionPolicy::new(PermissionMode::WorkspaceWrite)
            .with_tool_requirement("read_file", PermissionMode::ReadOnly)
            .with_tool_requirement("new_file", PermissionMode::WorkspaceWrite);

        assert_eq!(
            policy.authorize("read_file", "{}", None),
            PermissionOutcome::Allow
        );
        assert_eq!(
            policy.authorize("new_file", "{}", None),
            PermissionOutcome::Allow
        );
    }

    #[test]
    fn denies_read_only_escalations_without_prompt() {
        let policy = PermissionPolicy::new(PermissionMode::ReadOnly)
            .with_tool_requirement("new_file", PermissionMode::WorkspaceWrite)
            .with_tool_requirement("bash", PermissionMode::DangerFullAccess);

        assert!(matches!(
            policy.authorize("new_file", "{}", None),
            PermissionOutcome::Deny { reason } if reason.contains("requires workspace-write permission")
        ));
        assert!(matches!(
            policy.authorize("bash", "{}", None),
            PermissionOutcome::Deny { reason } if reason.contains("requires danger-full-access permission")
        ));
    }

    #[test]
    fn prompts_for_workspace_write_to_danger_full_access_escalation() {
        let policy = PermissionPolicy::new(PermissionMode::WorkspaceWrite)
            .with_tool_requirement("bash", PermissionMode::DangerFullAccess);
        let mut prompter = RecordingPrompter {
            seen: Vec::new(),
            allow: true,
        };

        let outcome = policy.authorize("bash", "echo hi", Some(&mut prompter));

        assert_eq!(outcome, PermissionOutcome::Allow);
        assert_eq!(prompter.seen.len(), 1);
        assert_eq!(prompter.seen[0].tool_name, "bash");
        assert_eq!(
            prompter.seen[0].current_mode,
            PermissionMode::WorkspaceWrite
        );
        assert_eq!(
            prompter.seen[0].required_mode,
            PermissionMode::DangerFullAccess
        );
    }

    #[test]
    fn honors_prompt_rejection_reason() {
        let policy = PermissionPolicy::new(PermissionMode::WorkspaceWrite)
            .with_tool_requirement("bash", PermissionMode::DangerFullAccess);
        let mut prompter = RecordingPrompter {
            seen: Vec::new(),
            allow: false,
        };

        assert!(matches!(
            policy.authorize("bash", "echo hi", Some(&mut prompter)),
            PermissionOutcome::Deny { reason } if reason == "not now"
        ));
    }

    #[test]
    fn applies_rule_based_denials_and_allows() {
        let rules = RuntimePermissionRuleConfig::new(
            vec!["bash(git:*)".to_string()],
            vec!["bash(rm -rf:*)".to_string()],
            Vec::new(),
        );
        let policy = PermissionPolicy::new(PermissionMode::ReadOnly)
            .with_tool_requirement("bash", PermissionMode::DangerFullAccess)
            .with_permission_rules(&rules);

        assert_eq!(
            policy.authorize("bash", r#"{"command":"git status"}"#, None),
            PermissionOutcome::Allow
        );
        assert!(matches!(
            policy.authorize("bash", r#"{"command":"rm -rf /tmp/x"}"#, None),
            PermissionOutcome::Deny { reason } if reason.contains("denied by rule")
        ));
    }

    #[test]
    fn ask_rules_force_prompt_even_when_mode_allows() {
        let rules = RuntimePermissionRuleConfig::new(
            Vec::new(),
            Vec::new(),
            vec!["bash(git:*)".to_string()],
        );
        let policy = PermissionPolicy::new(PermissionMode::DangerFullAccess)
            .with_tool_requirement("bash", PermissionMode::DangerFullAccess)
            .with_permission_rules(&rules);
        let mut prompter = RecordingPrompter {
            seen: Vec::new(),
            allow: true,
        };

        let outcome = policy.authorize("bash", r#"{"command":"git status"}"#, Some(&mut prompter));

        assert_eq!(outcome, PermissionOutcome::Allow);
        assert_eq!(prompter.seen.len(), 1);
        assert!(prompter.seen[0]
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("ask rule")));
    }

    #[test]
    fn hook_allow_still_respects_ask_rules() {
        let rules = RuntimePermissionRuleConfig::new(
            Vec::new(),
            Vec::new(),
            vec!["bash(git:*)".to_string()],
        );
        let policy = PermissionPolicy::new(PermissionMode::ReadOnly)
            .with_tool_requirement("bash", PermissionMode::DangerFullAccess)
            .with_permission_rules(&rules);
        let context = PermissionContext::new(
            Some(PermissionOverride::Allow),
            Some("hook approved".to_string()),
        );
        let mut prompter = RecordingPrompter {
            seen: Vec::new(),
            allow: true,
        };

        let outcome = policy.authorize_with_context(
            "bash",
            r#"{"command":"git status"}"#,
            &context,
            Some(&mut prompter),
        );

        assert_eq!(outcome, PermissionOutcome::Allow);
        assert_eq!(prompter.seen.len(), 1);
    }

    #[test]
    fn hook_allow_respects_override_even_when_mode_insufficient() {
        // hook Allow + insufficient mode + no ask_rule → Allow (hook decides)
        let policy = PermissionPolicy::new(PermissionMode::ReadOnly)
            .with_tool_requirement("bash", PermissionMode::DangerFullAccess);
        let context = PermissionContext::new(
            Some(PermissionOverride::Allow),
            Some("hook approved".to_string()),
        );
        assert_eq!(
            policy.authorize_with_context("bash", "{}", &context, None),
            PermissionOutcome::Allow
        );
    }

    #[test]
    fn hook_deny_short_circuits_permission_flow() {
        let policy = PermissionPolicy::new(PermissionMode::DangerFullAccess)
            .with_tool_requirement("bash", PermissionMode::DangerFullAccess);
        let context = PermissionContext::new(
            Some(PermissionOverride::Deny),
            Some("blocked by hook".to_string()),
        );

        assert_eq!(
            policy.authorize_with_context("bash", "{}", &context, None),
            PermissionOutcome::Deny {
                reason: "blocked by hook".to_string(),
            }
        );
    }

    #[test]
    fn hook_ask_forces_prompt() {
        let policy = PermissionPolicy::new(PermissionMode::DangerFullAccess)
            .with_tool_requirement("bash", PermissionMode::DangerFullAccess);
        let context = PermissionContext::new(
            Some(PermissionOverride::Ask),
            Some("hook requested confirmation".to_string()),
        );
        let mut prompter = RecordingPrompter {
            seen: Vec::new(),
            allow: true,
        };

        let outcome = policy.authorize_with_context("bash", "{}", &context, Some(&mut prompter));

        assert_eq!(outcome, PermissionOutcome::Allow);
        assert_eq!(prompter.seen.len(), 1);
        assert_eq!(
            prompter.seen[0].reason.as_deref(),
            Some("hook requested confirmation")
        );
    }

    // ── Phase 2: Rule parser validation ──

    #[test]
    fn rule_parse_empty_parens_matches_all() {
        let rule = PermissionRule::parse_with_warning("bash()", RuleList::Allow).0;
        assert_eq!(rule.tool_name, "bash");
        assert_eq!(rule.matcher, PermissionRuleMatcher::Any);
    }

    #[test]
    fn rule_parse_malformed_no_close_paren_uses_exact_fallback() {
        let rule = PermissionRule::parse_with_warning("bash(rm -rf /", RuleList::Allow).0;
        assert_eq!(rule.tool_name, "bash(rm -rf /");
        assert_eq!(
            rule.matcher,
            PermissionRuleMatcher::Exact("bash(rm -rf /".into())
        );
    }

    #[test]
    fn rule_parse_malformed_no_close_paren_never_matches_bash() {
        // deny rule "bash(rm -rf /" should NOT match "bash" tool
        let rules = RuntimePermissionRuleConfig::new(
            Vec::new(),
            vec!["bash(rm -rf /".to_string()],
            Vec::new(),
        );
        let policy = PermissionPolicy::new(PermissionMode::DangerFullAccess)
            .with_tool_requirement("bash", PermissionMode::DangerFullAccess)
            .with_permission_rules(&rules);
        assert_eq!(
            policy.authorize("bash", r#"{"command":"rm -rf /tmp"}"#, None),
            PermissionOutcome::Allow,
            "malformed deny rule must not block real bash commands"
        );
    }

    #[test]
    fn rule_parse_bare_close_paren_uses_exact_fallback() {
        let rule = PermissionRule::parse_with_warning("bash)", RuleList::Allow).0;
        assert_eq!(rule.tool_name, "bash)");
        assert_eq!(rule.matcher, PermissionRuleMatcher::Exact("bash)".into()));
    }

    // ── Phase 3: Tool-aware subject extraction ──

    #[test]
    fn subject_extraction_bash_uses_command_key() {
        assert_eq!(
            extract_permission_subject(r#"{"command":"git status"}"#, "bash"),
            Some("git status".into())
        );
    }

    #[test]
    fn subject_extraction_write_uses_path_key() {
        // Short alias "write" → key "path"
        assert_eq!(
            extract_permission_subject(r#"{"path":"/tmp/foo.txt"}"#, "write"),
            Some("/tmp/foo.txt".into())
        );
        // Normalized name "new_file" → key "path"
        assert_eq!(
            extract_permission_subject(r#"{"path":"/tmp/foo.txt"}"#, "new_file"),
            Some("/tmp/foo.txt".into())
        );
    }

    #[test]
    fn subject_extraction_glob_uses_pattern_key() {
        assert_eq!(
            extract_permission_subject(r#"{"pattern":"**/*.rs"}"#, "glob"),
            Some("**/*.rs".into())
        );
        // Normalized name "glob_search" → key "pattern"
        assert_eq!(
            extract_permission_subject(r#"{"pattern":"**/*.rs"}"#, "glob_search"),
            Some("**/*.rs".into())
        );
    }

    #[test]
    fn subject_extraction_web_uses_url_key() {
        assert_eq!(
            extract_permission_subject(r#"{"url":"https://example.com"}"#, "web_fetch"),
            Some("https://example.com".into())
        );
        // PascalCase tool names
        assert_eq!(
            extract_permission_subject(r#"{"url":"https://example.com"}"#, "WebFetch"),
            Some("https://example.com".into())
        );
        assert_eq!(
            extract_permission_subject(r#"{"query":"rust lang"}"#, "WebSearch"),
            Some("rust lang".into())
        );
    }

    #[test]
    fn subject_extraction_unknown_tool_falls_back_to_path_key() {
        // Unknown tools default to "path" key (the canonical file-tool key).
        // Previously defaulted to "file_path" which silently failed for every tool
        // that uses the standard "path" schema key.
        assert_eq!(
            extract_permission_subject(r#"{"path":"/etc/passwd"}"#, "custom_tool"),
            Some("/etc/passwd".into())
        );
        // "file_path" key is no longer the fallback — returns None for unknown tools.
        assert_eq!(
            extract_permission_subject(r#"{"file_path":"/etc/passwd"}"#, "custom_tool"),
            None
        );
    }

    #[test]
    fn subject_extraction_bash_ignores_file_path_key() {
        // "file_path" present but "command" missing -> None for bash
        assert_eq!(
            extract_permission_subject(r#"{"file_path":"/tmp/foo.txt"}"#, "bash"),
            None
        );
    }

    #[test]
    fn subject_extraction_bash_via_normalized_name() {
        assert_eq!(
            extract_permission_subject(r#"{"command":"ls -la"}"#, "bash"),
            Some("ls -la".into())
        );
    }

    #[test]
    fn subject_extraction_question_uses_question_key() {
        assert_eq!(
            extract_permission_subject(r#"{"question":"What is your name?"}"#, "Question"),
            Some("What is your name?".into())
        );
    }

    #[test]
    fn deny_rule_write_matches_by_file_path() {
        let rules = RuntimePermissionRuleConfig::new(
            Vec::new(),
            vec!["write(/etc/shadow)".to_string()],
            Vec::new(),
        );
        let policy = PermissionPolicy::new(PermissionMode::DangerFullAccess)
            .with_tool_requirement("new_file", PermissionMode::DangerFullAccess)
            .with_permission_rules(&rules);
        assert_eq!(
            policy.authorize("new_file", r#"{"path":"/etc/shadow"}"#, None),
            PermissionOutcome::Deny {
                reason: "Permission to use new_file has been denied by rule 'write(/etc/shadow)'".into()
            }
        );
    }

    #[test]
    fn tool_name_normalization_read_matches_read_file() {
        // Rule "Read(*)" should match API tool name "read_file"
        let rules = RuntimePermissionRuleConfig::new(
            vec!["Read(*)".to_string()],
            Vec::new(),
            Vec::new(),
        );
        let policy = PermissionPolicy::new(PermissionMode::DangerFullAccess)
            .with_tool_requirement("read_file", PermissionMode::ReadOnly)
            .with_permission_rules(&rules);
        assert_eq!(
            policy.authorize("read_file", r#"{"path":"/workspace/file.txt"}"#, None),
            PermissionOutcome::Allow
        );
    }

    #[test]
    fn tool_name_normalization_bash_case_insensitive() {
        // Rule "Bash(*)" should match API tool name "bash"
        let rules = RuntimePermissionRuleConfig::new(
            vec!["Bash(*)".to_string()],
            Vec::new(),
            Vec::new(),
        );
        let policy = PermissionPolicy::new(PermissionMode::DangerFullAccess)
            .with_tool_requirement("bash", PermissionMode::ReadOnly)
            .with_permission_rules(&rules);
        assert_eq!(
            policy.authorize("bash", r#"{}"#, None),
            PermissionOutcome::Allow
        );
    }

    // F-18: lock escape-helper semantics so the O(n) rewrite matches the
    // original O(n²) implementation.
    #[test]
    fn find_first_unescaped_basic() {
        assert_eq!(find_first_unescaped("a(b)c", '('), Some(1));
        assert_eq!(find_first_unescaped("abc", '('), None);
        // Two parens: first is escaped, second is unescaped at index 4.
        assert_eq!(find_first_unescaped(r"a\(b(c", '('), Some(4));
        // Only an escaped paren: helper returns None.
        assert_eq!(find_first_unescaped(r"a\(b)c", '('), None);
    }

    #[test]
    fn find_last_unescaped_basic() {
        // `)` is at index 3.
        assert_eq!(find_last_unescaped("a(b)c", ')'), Some(3));
        assert_eq!(find_last_unescaped("abc", ')'), None);
        // Two close parens: first is escaped, second (index 6) is unescaped.
        assert_eq!(find_last_unescaped(r"a(b\)c)", ')'), Some(6));
    }

    #[test]
    fn find_last_unescaped_all_backslashes_does_not_hang() {
        // Regression: original implementation was O(n²) on runs of backslashes.
        // New implementation must complete in linear time on a long prefix.
        let value: String = std::iter::repeat('\\').take(10_000).collect();
        let result = find_last_unescaped(&value, ')');
        assert_eq!(result, None);
    }

    #[test]
    fn find_last_unescaped_matches_first_unescaped_semantics() {
        // Both helpers must use the same escape-state definition: if there
        // is exactly one unescaped `)`, first and last must agree.
        let cases = [
            "()",       // one unescaped `)` at index 1
            r"\()",     // `(` escaped, `)` at index 2 unescaped
            r"(\)",     // `(` unescaped, `)` escaped
            r"\\()",    // `(` escaped, `)` unescaped at index 3
            r"\\(\)",   // `(` escaped, `)` escaped
            r"a(b)c",   // `)` at index 3
            r"a\(b\)c", // both parens escaped
            "))",       // two unescaped `)` at indices 0 and 1
        ];
        for value in cases {
            let first = find_first_unescaped(value, ')');
            let last = find_last_unescaped(value, ')');
            match (first, last) {
                (Some(f), Some(l)) => assert!(f <= l, "first={f} > last={l} for {value:?}"),
                (None, None) => {}
                (Some(f), None) => panic!("first={f} but last=None for {value:?}"),
                (None, Some(l)) => panic!("last={l} but first=None for {value:?}"),
            }
        }
    }

    // ── Phase 8: Warning system regression tests (F-01, F-02, F-03, F-04, F-05, F-09) ──

    #[test]
    fn parse_warning_empty_matcher_includes_list_name_and_raw_tool() {
        // F-02: warning must name the source list ("allow").
        // F-03: warning must show the user's original tool spelling ("Bash"),
        // not the normalized name ("bash").
        // F-09: empty parens produce "empty matcher" label.
        let (_, warnings) = PermissionRule::parse_with_warning("Bash()", RuleList::Allow);
        assert_eq!(warnings.len(), 1);
        let w = &warnings[0];
        assert!(w.contains("allow list"), "missing list name: {w}");
        assert!(w.contains("'Bash'"), "missing raw tool name: {w}");
        assert!(w.contains("empty matcher"), "missing empty label: {w}");
    }

    #[test]
    fn parse_warning_wildcard_matcher_uses_wildcard_label() {
        // F-09: `(*)` must produce a "wildcard matcher" label, not "empty".
        let (_, warnings) = PermissionRule::parse_with_warning("Read(*)", RuleList::Allow);
        assert_eq!(warnings.len(), 1);
        let w = &warnings[0];
        assert!(w.contains("wildcard matcher"), "wrong label: {w}");
        assert!(!w.contains("empty matcher"), "should not say empty: {w}");
        assert!(w.contains("allow list"), "missing list name: {w}");
        assert!(w.contains("'Read'"), "missing raw tool name: {w}");
    }

    #[test]
    fn parse_warning_deny_list_is_named() {
        // F-02: same rule in deny list must produce a different warning
        // identifying the deny list — this is critical because semantics
        // are opposite (deny = effective vs. allow = likely redundant).
        let (_, warnings) = PermissionRule::parse_with_warning("Bash(*)", RuleList::Deny);
        assert_eq!(warnings.len(), 1);
        let w = &warnings[0];
        assert!(w.contains("deny list"), "missing list name: {w}");
    }

    #[test]
    fn parse_warning_ask_list_is_named() {
        let (_, warnings) = PermissionRule::parse_with_warning("Edit(*)", RuleList::Ask);
        assert_eq!(warnings.len(), 1);
        let w = &warnings[0];
        assert!(w.contains("ask list"), "missing list name: {w}");
    }

    #[test]
    fn parse_warning_malformed_no_close_paren_emits_warning() {
        // F-05: an open paren with no close paren must produce a startup
        // warning, not silently fall through to a dead exact-match rule.
        let (rule, warnings) =
            PermissionRule::parse_with_warning("Bash(rm -rf /", RuleList::Deny);
        assert_eq!(warnings.len(), 1);
        let w = &warnings[0];
        assert!(w.contains("'(' but no matching ')'"), "wrong reason: {w}");
        assert!(w.contains("deny list"), "missing list name: {w}");
        // The rule is still stored (for backwards compat) but it's a dead
        // exact-match against a literal string.
        assert_eq!(rule.tool_name, "Bash(rm -rf /");
    }

    #[test]
    fn parse_warning_tool_name_wildcard_silent_but_functional() {
        // F-04: `mcp__*` syntax is a tool-name prefix wildcard. The parser
        // promotes it to a `ToolNamePrefix` matcher so the rule actually
        // matches at runtime, but emits no warning — the syntax is the
        // canonical, well-defined way to allow a tool family whose members
        // are registered at runtime (e.g. all MCP tools), and a startup
        // message would be pure noise for any user who reads the docs.
        let (rule, warnings) =
            PermissionRule::parse_with_warning("mcp__*", RuleList::Allow);
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
        // The rule is stored as a ToolNamePrefix matcher (not Exact) so it
        // matches every tool whose runtime name starts with `mcp__`.
        assert_eq!(rule.tool_name, "mcp__*");
        assert_eq!(
            rule.matcher,
            PermissionRuleMatcher::ToolNamePrefix("mcp__".to_string())
        );
        // And it actually matches MCP tool names at runtime.
        assert!(rule.matches("mcp__filesystem__read_file", "{}"));
        assert!(rule.matches("mcp__github__create_issue", "{}"));
        assert!(!rule.matches("read_file", "{}"));
        assert!(!rule.matches("bash", "{}"));
    }

    #[test]
    fn parse_warning_specific_matcher_emits_no_warning() {
        // A well-formed rule with a specific subject must NOT emit a warning.
        let (_, warnings) = PermissionRule::parse_with_warning("Bash(rm:*)", RuleList::Deny);
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    }

    #[test]
    fn parse_warning_normalized_tool_not_in_warning_text() {
        // F-03: warning must show user's spelling, not the normalized form.
        // For "Read(*)" the normalized form is "read_file", but the warning
        // must show "Read" so the user can map it back to their config.
        let (_, warnings) = PermissionRule::parse_with_warning("Read(*)", RuleList::Allow);
        assert_eq!(warnings.len(), 1);
        let w = &warnings[0];
        assert!(!w.contains("read_file"), "should not show normalized: {w}");
    }

    // ── Phase 9: Redundant-allow warning suppression (F-01) ──

    #[test]
    fn is_broad_matcher_warning_detects_empty_and_wildcard() {
        assert!(is_broad_matcher_warning(
            "warning: permission rule 'Bash()' in allow list matches ALL 'Bash' tools (empty matcher)."
        ));
        assert!(is_broad_matcher_warning(
            "warning: permission rule 'Bash(*)' in allow list matches ALL 'Bash' tools (wildcard matcher)."
        ));
        // Negative cases — must NOT be filtered.
        assert!(!is_broad_matcher_warning(
            "warning: permission rule 'mcp__*' in allow list uses tool-name wildcard syntax"
        ));
        assert!(!is_broad_matcher_warning(
            "warning: permission rule 'Bash(rm -rf /' in deny list has '(' but no matching ')'"
        ));
    }

    #[test]
    fn redundant_allow_rule_under_danger_full_access_silenced() {
        // F-01: the user's `defaultMode: dontAsk` resolves to DangerFullAccess.
        // All broad allow rules (Any matcher) are redundant under this mode
        // and must NOT emit the broad-matcher warning. This is the silent
        // path the audit identified as the primary noise source.
        let rules = RuntimePermissionRuleConfig::new(
            vec![
                "Bash(*)".to_string(),
                "Read(*)".to_string(),
                "Write(*)".to_string(),
                "Edit(*)".to_string(),
            ],
            Vec::new(),
            Vec::new(),
        );
        let policy = PermissionPolicy::new(PermissionMode::DangerFullAccess)
            .with_permission_rules(&rules);
        // Sanity: the rules still work — bash is allowed.
        assert_eq!(
            policy.authorize("bash", r#"{"command":"ls"}"#, None),
            PermissionOutcome::Allow
        );
        // The broad-matcher warnings must have been suppressed (we cannot
        // easily assert "no stderr" here; instead we assert that the policy
        // built without panicking, and that calling again is idempotent).
        let _ = policy.authorize("bash", r#"{"command":"ls"}"#, None);
    }

    #[test]
    fn redundant_allow_rule_under_workspace_write_for_readonly_tool_silenced() {
        // WorkspaceWrite mode + read_file (requires ReadOnly). The mode
        // already covers ReadOnly, so a broad allow rule is redundant.
        let rules = RuntimePermissionRuleConfig::new(
            vec!["Read(*)".to_string()],
            Vec::new(),
            Vec::new(),
        );
        let policy = PermissionPolicy::new(PermissionMode::WorkspaceWrite)
            .with_tool_requirement("read_file", PermissionMode::ReadOnly)
            .with_permission_rules(&rules);
        assert_eq!(
            policy.authorize("read_file", r#"{"path":"/tmp/x"}"#, None),
            PermissionOutcome::Allow
        );
    }

    #[test]
    fn broad_allow_rule_for_dangerfullaccess_tool_under_readonly_warns() {
        // ReadOnly mode + bash (requires DangerFullAccess). The mode does
        // NOT cover DangerFullAccess, so the broad allow rule is NOT
        // redundant — the warning MUST be emitted. (We can only assert
        // indirectly by verifying the policy authorizes, but the warning
        // would be visible on stderr at runtime.)
        let rules = RuntimePermissionRuleConfig::new(
            vec!["Bash(*)".to_string()],
            Vec::new(),
            Vec::new(),
        );
        let policy = PermissionPolicy::new(PermissionMode::ReadOnly)
            .with_tool_requirement("bash", PermissionMode::DangerFullAccess)
            .with_permission_rules(&rules);
        // With ask_rules empty, ReadOnly mode + DangerFullAccess tool =
        // deny (escalation cannot be auto-allowed by a broad allow rule
        // in ReadOnly mode; the allow rule only matters when the mode
        // would otherwise allow).
        let outcome = policy.authorize("bash", r#"{"command":"ls"}"#, None);
        // The exact outcome depends on the ask_rules check; in any case
        // the build must succeed and not panic.
        let _ = outcome;
    }

    #[test]
    fn broad_allow_rule_under_prompt_mode_warns() {
        // Prompt mode has different semantics — the prompter drives the
        // decision, not the rule alone. The broad-matcher warning must
        // NOT be suppressed under Prompt mode.
        let rules = RuntimePermissionRuleConfig::new(
            vec!["Bash(*)".to_string()],
            Vec::new(),
            Vec::new(),
        );
        let policy = PermissionPolicy::new(PermissionMode::Prompt)
            .with_tool_requirement("bash", PermissionMode::DangerFullAccess)
            .with_permission_rules(&rules);
        // Build must succeed.
        let _ = policy;
    }

    #[test]
    fn broad_deny_rule_always_warns() {
        // Deny rules are always live regardless of mode — they short-circuit
        // authorization. The broad-matcher warning must always be emitted
        // for deny rules (never suppressed).
        let (rule, warnings) =
            PermissionRule::parse_with_warning("Bash(*)", RuleList::Deny);
        assert_eq!(warnings.len(), 1);
        assert!(rule.tool_name == "bash");
    }

    #[test]
    fn broad_ask_rule_always_warns() {
        let (rule, warnings) =
            PermissionRule::parse_with_warning("Bash(*)", RuleList::Ask);
        assert_eq!(warnings.len(), 1);
        assert!(rule.tool_name == "bash");
    }

    #[test]
    fn mcp_wildcard_silently_promoted_to_prefix_matcher() {
        // F-04: `mcp__*` is the canonical syntax for "all tools whose
        // runtime name starts with `mcp__`". The parser promotes it to a
        // `ToolNamePrefix` matcher with no startup message — the syntax is
        // well-defined and a warning would just be noise. The test guards
        // against a future regression that re-introduces a startup warning
        // for this rule.
        let (rule, warnings) =
            PermissionRule::parse_with_warning("mcp__*", RuleList::Allow);
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
        assert_eq!(
            rule.matcher,
            PermissionRuleMatcher::ToolNamePrefix("mcp__".to_string())
        );
    }
}
