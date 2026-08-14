use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubagentKind {
    GeneralPurpose,
    Explore,
    Plan,
    Verification,
    ClawGuide,
    StatuslineSetup,
    Custom(String),
}

impl SubagentKind {
    pub fn from_str(s: Option<&str>) -> Self {
        match canonical_tool_token(s.map(str::trim).unwrap_or_default()).as_str() {
            "general" | "generalpurpose" | "generalpurposeagent" => Self::GeneralPurpose,
            "explore" | "explorer" | "exploreagent" => Self::Explore,
            "plan" | "planagent" => Self::Plan,
            "verification" | "verificationagent" | "verify" | "verifier" => Self::Verification,
            "clawguide" | "clawguideagent" | "guide" => Self::ClawGuide,
            "statusline" | "statuslinesetup" => Self::StatuslineSetup,
            other => Self::Custom(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::GeneralPurpose => "general-purpose",
            Self::Explore => "Explore",
            Self::Plan => "Plan",
            Self::Verification => "Verification",
            Self::ClawGuide => "claw-guide",
            Self::StatuslineSetup => "statusline-setup",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn allowed_tools(&self) -> BTreeSet<String> {
        let tools: Vec<&str> = match self {
            Self::Explore => vec![
                "read_file", "glob_search", "grep_search", "WebFetch", "WebSearch",
                "ToolSearch", "Skill", "StructuredOutput",
            ],
            Self::Plan => vec![
                "read_file", "glob_search", "grep_search", "WebFetch", "WebSearch",
                "ToolSearch", "Skill", "StructuredOutput",
            ],
            Self::Verification => vec![
                "bash", "read_file", "glob_search", "grep_search", "WebSearch",
                "ToolSearch", "StructuredOutput",
            ],
            Self::ClawGuide => vec![
                "read_file", "glob_search", "grep_search", "WebFetch", "WebSearch",
                "ToolSearch", "Skill", "StructuredOutput",
            ],
            Self::StatuslineSetup => vec![
                "bash", "read_file", "new_file", "edit_file", "glob_search",
                "grep_search", "ToolSearch",
            ],
            Self::GeneralPurpose => vec![
                "bash", "read_file", "new_file", "edit_file", "glob_search",
                "grep_search", "WebFetch", "WebSearch", "Skill",
                "StructuredOutput",
            ],
            Self::Custom(_) => vec![],
        };
        tools.into_iter().map(str::to_string).collect()
    }
}

pub fn normalize_subagent_type(subagent_type: Option<&str>) -> String {
    SubagentKind::from_str(subagent_type).as_str().to_string()
}

pub fn allowed_tools_for_subagent(subagent_type: &str) -> BTreeSet<String> {
    SubagentKind::from_str(Some(subagent_type)).allowed_tools()
}

fn canonical_tool_token(value: &str) -> String {
    let mut canonical: String = value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect();
    if let Some(stripped) = canonical.strip_suffix("tool") {
        canonical = stripped.to_string();
    }
    canonical
}
