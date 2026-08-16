use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Task scope resolution for defining the granularity of work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskScope {
    /// Work across the entire workspace
    Workspace,
    /// Work within a specific module/crate
    Module,
    /// Work on a single file
    SingleFile,
    /// Custom scope defined by the user
    Custom,
}

impl std::fmt::Display for TaskScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Workspace => write!(f, "workspace"),
            Self::Module => write!(f, "module"),
            Self::SingleFile => write!(f, "single-file"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPacket {
    pub objective: String,
    pub scope: TaskScope,
    /// Optional scope path when scope is `Module`, `SingleFile`, or `Custom`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_path: Option<String>,
    pub repo: String,
    /// Worktree path for the task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
    pub branch_policy: String,
    pub acceptance_tests: Vec<String>,
    pub commit_policy: String,
    pub reporting_contract: String,
    pub escalation_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskPacketValidationError {
    errors: Vec<String>,
}

impl TaskPacketValidationError {
    #[must_use]
    pub fn new(errors: Vec<String>) -> Self {
        Self { errors }
    }

    #[must_use]
    pub fn errors(&self) -> &[String] {
        &self.errors
    }
}

impl Display for TaskPacketValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.errors.join("; "))
    }
}

impl std::error::Error for TaskPacketValidationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPacket(TaskPacket);

impl ValidatedPacket {
    #[must_use]
    pub fn packet(&self) -> &TaskPacket {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> TaskPacket {
        self.0
    }
}

pub fn validate_packet(packet: TaskPacket) -> Result<ValidatedPacket, TaskPacketValidationError> {
    let mut errors = Vec::new();

    validate_required("objective", &packet.objective, &mut errors);
    validate_required("repo", &packet.repo, &mut errors);
    validate_required("branch_policy", &packet.branch_policy, &mut errors);
    validate_required("commit_policy", &packet.commit_policy, &mut errors);
    validate_required(
        "reporting_contract",
        &packet.reporting_contract,
        &mut errors,
    );
    validate_required("escalation_policy", &packet.escalation_policy, &mut errors);

    // Validate scope-specific requirements
    validate_scope_requirements(&packet, &mut errors);

    for (index, test) in packet.acceptance_tests.iter().enumerate() {
        if test.trim().is_empty() {
            errors.push(format!(
                "acceptance_tests contains an empty value at index {index}"
            ));
        }
    }

    if errors.is_empty() {
        Ok(ValidatedPacket(packet))
    } else {
        Err(TaskPacketValidationError::new(errors))
    }
}

fn validate_scope_requirements(packet: &TaskPacket, errors: &mut Vec<String>) {
    // Scope path is required for Module, SingleFile, and Custom scopes
    let needs_scope_path = matches!(
        packet.scope,
        TaskScope::Module | TaskScope::SingleFile | TaskScope::Custom
    );

    if needs_scope_path
        && packet
            .scope_path
            .as_ref()
            .is_none_or(|p| p.trim().is_empty())
    {
        errors.push(format!(
            "scope_path is required for scope '{}'",
            packet.scope
        ));
    }
}

fn validate_required(field: &str, value: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{field} must not be empty"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_packet() -> TaskPacket {
        TaskPacket {
            objective: "Implement typed task packet format".to_string(),
            scope: TaskScope::Module,
            scope_path: Some("runtime/task system".to_string()),
            repo: "clawcode-parity".to_string(),
            worktree: Some("/tmp/wt-1".to_string()),
            branch_policy: "origin/main only".to_string(),
            acceptance_tests: vec![
                "cargo build --workspace".to_string(),
                "cargo test --workspace".to_string(),
            ],
            commit_policy: "single verified commit".to_string(),
            reporting_contract: "print build result, test result, commit sha".to_string(),
            escalation_policy: "stop only on destructive ambiguity".to_string(),
        }
    }

    #[test]
    fn valid_packet_passes_validation() {
        let packet = sample_packet();
        let validated = validate_packet(packet.clone()).expect("packet should validate");
        assert_eq!(validated.packet(), &packet);
        assert_eq!(validated.into_inner(), packet);
    }

    #[test]
    fn invalid_packet_accumulates_errors() {
        use super::TaskScope;
        let packet = TaskPacket {
            objective: " ".to_string(),
            scope: TaskScope::Workspace,
            scope_path: None,
            worktree: None,
            repo: String::new(),
            branch_policy: "\t".to_string(),
            acceptance_tests: vec!["ok".to_string(), " ".to_string()],
            commit_policy: String::new(),
            reporting_contract: String::new(),
            escalation_policy: String::new(),
        };

        let error = validate_packet(packet).expect_err("packet should be rejected");

        assert!(error.errors().len() >= 7);
        assert!(error
            .errors()
            .contains(&"objective must not be empty".to_string()));
        assert!(error
            .errors()
            .contains(&"repo must not be empty".to_string()));
        assert!(error
            .errors()
            .contains(&"acceptance_tests contains an empty value at index 1".to_string()));
    }

    #[test]
    fn serialization_roundtrip_preserves_packet() {
        let packet = sample_packet();
        let serialized = serde_json::to_string(&packet).expect("packet should serialize");
        let deserialized: TaskPacket =
            serde_json::from_str(&serialized).expect("packet should deserialize");
        assert_eq!(deserialized, packet);
    }
}
