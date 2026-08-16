#![allow(clippy::unnested_or_patterns, clippy::map_unwrap_or)]
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpLifecyclePhase {
    ConfigLoad,
    ServerRegistration,
    SpawnConnect,
    InitializeHandshake,
    ToolDiscovery,
    ResourceDiscovery,
    Ready,
    Invocation,
    ErrorSurfacing,
    Shutdown,
    Cleanup,
}

impl McpLifecyclePhase {
    #[must_use]
    pub fn all() -> [Self; 11] {
        [
            Self::ConfigLoad,
            Self::ServerRegistration,
            Self::SpawnConnect,
            Self::InitializeHandshake,
            Self::ToolDiscovery,
            Self::ResourceDiscovery,
            Self::Ready,
            Self::Invocation,
            Self::ErrorSurfacing,
            Self::Shutdown,
            Self::Cleanup,
        ]
    }
}

impl std::fmt::Display for McpLifecyclePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigLoad => write!(f, "config_load"),
            Self::ServerRegistration => write!(f, "server_registration"),
            Self::SpawnConnect => write!(f, "spawn_connect"),
            Self::InitializeHandshake => write!(f, "initialize_handshake"),
            Self::ToolDiscovery => write!(f, "tool_discovery"),
            Self::ResourceDiscovery => write!(f, "resource_discovery"),
            Self::Ready => write!(f, "ready"),
            Self::Invocation => write!(f, "invocation"),
            Self::ErrorSurfacing => write!(f, "error_surfacing"),
            Self::Shutdown => write!(f, "shutdown"),
            Self::Cleanup => write!(f, "cleanup"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpErrorSurface {
    pub phase: McpLifecyclePhase,
    pub server_name: Option<String>,
    pub message: String,
    pub context: BTreeMap<String, String>,
    pub recoverable: bool,
    pub timestamp: u64,
}

impl McpErrorSurface {
    #[must_use]
    pub fn new(
        phase: McpLifecyclePhase,
        server_name: Option<String>,
        message: impl Into<String>,
        context: BTreeMap<String, String>,
        recoverable: bool,
    ) -> Self {
        Self {
            phase,
            server_name,
            message: message.into(),
            context,
            recoverable,
            timestamp: now_secs(),
        }
    }
}

impl std::fmt::Display for McpErrorSurface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MCP lifecycle error during {}: {}",
            self.phase, self.message
        )?;
        if let Some(server_name) = &self.server_name {
            write!(f, " (server: {server_name})")?;
        }
        if !self.context.is_empty() {
            write!(f, " with context {:?}", self.context)?;
        }
        if self.recoverable {
            write!(f, " [recoverable]")?;
        }
        Ok(())
    }
}

impl std::error::Error for McpErrorSurface {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpPhaseResult {
    Success {
        phase: McpLifecyclePhase,
        duration: Duration,
    },
    Failure {
        phase: McpLifecyclePhase,
        error: McpErrorSurface,
    },
    Timeout {
        phase: McpLifecyclePhase,
        waited: Duration,
        error: McpErrorSurface,
    },
}

impl McpPhaseResult {
    #[must_use]
    pub fn phase(&self) -> McpLifecyclePhase {
        match self {
            Self::Success { phase, .. }
            | Self::Failure { phase, .. }
            | Self::Timeout { phase, .. } => *phase,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct McpLifecycleState {
    current_phase: Option<McpLifecyclePhase>,
    phase_errors: BTreeMap<McpLifecyclePhase, Vec<McpErrorSurface>>,
    phase_timestamps: BTreeMap<McpLifecyclePhase, u64>,
    phase_results: Vec<McpPhaseResult>,
}

impl McpLifecycleState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn current_phase(&self) -> Option<McpLifecyclePhase> {
        self.current_phase
    }

    #[must_use]
    pub fn errors_for_phase(&self, phase: McpLifecyclePhase) -> &[McpErrorSurface] {
        self.phase_errors
            .get(&phase)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    #[must_use]
    pub fn results(&self) -> &[McpPhaseResult] {
        &self.phase_results
    }

    #[must_use]
    pub fn phase_timestamps(&self) -> &BTreeMap<McpLifecyclePhase, u64> {
        &self.phase_timestamps
    }

    #[must_use]
    pub fn phase_timestamp(&self, phase: McpLifecyclePhase) -> Option<u64> {
        self.phase_timestamps.get(&phase).copied()
    }

    fn record_phase(&mut self, phase: McpLifecyclePhase) {
        self.current_phase = Some(phase);
        self.phase_timestamps.insert(phase, now_secs());
    }

    fn record_error(&mut self, error: McpErrorSurface) {
        self.phase_errors
            .entry(error.phase)
            .or_default()
            .push(error);
    }

    fn record_result(&mut self, result: McpPhaseResult) {
        self.phase_results.push(result);
    }

    fn can_resume_after_error(&self) -> bool {
        match self.phase_results.last() {
            Some(McpPhaseResult::Failure { error, .. } | McpPhaseResult::Timeout { error, .. }) => {
                error.recoverable
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpFailedServer {
    pub server_name: String,
    pub phase: McpLifecyclePhase,
    pub error: McpErrorSurface,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpDegradedReport {
    pub working_servers: Vec<String>,
    pub failed_servers: Vec<McpFailedServer>,
    pub available_tools: Vec<String>,
    pub missing_tools: Vec<String>,
}

impl McpDegradedReport {
    #[must_use]
    pub fn new(
        working_servers: Vec<String>,
        failed_servers: Vec<McpFailedServer>,
        available_tools: Vec<String>,
        expected_tools: Vec<String>,
    ) -> Self {
        let working_servers = dedupe_sorted(working_servers);
        let available_tools = dedupe_sorted(available_tools);
        let available_tool_set: BTreeSet<_> = available_tools.iter().cloned().collect();
        let expected_tools = dedupe_sorted(expected_tools);
        let missing_tools = expected_tools
            .into_iter()
            .filter(|tool| !available_tool_set.contains(tool))
            .collect();

        Self {
            working_servers,
            failed_servers,
            available_tools,
            missing_tools,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct McpLifecycleValidator {
    state: McpLifecycleState,
}

impl McpLifecycleValidator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn state(&self) -> &McpLifecycleState {
        &self.state
    }

    #[must_use]
    pub fn validate_phase_transition(from: McpLifecyclePhase, to: McpLifecyclePhase) -> bool {
        match (from, to) {
            (McpLifecyclePhase::ConfigLoad, McpLifecyclePhase::ServerRegistration)
            | (McpLifecyclePhase::ServerRegistration, McpLifecyclePhase::SpawnConnect)
            | (McpLifecyclePhase::SpawnConnect, McpLifecyclePhase::InitializeHandshake)
            | (McpLifecyclePhase::InitializeHandshake, McpLifecyclePhase::ToolDiscovery)
            | (McpLifecyclePhase::ToolDiscovery, McpLifecyclePhase::ResourceDiscovery)
            | (McpLifecyclePhase::ToolDiscovery, McpLifecyclePhase::Ready)
            | (McpLifecyclePhase::ResourceDiscovery, McpLifecyclePhase::Ready)
            | (McpLifecyclePhase::Ready, McpLifecyclePhase::Invocation)
            | (McpLifecyclePhase::Invocation, McpLifecyclePhase::Ready)
            | (McpLifecyclePhase::ErrorSurfacing, McpLifecyclePhase::Ready)
            | (McpLifecyclePhase::ErrorSurfacing, McpLifecyclePhase::Shutdown)
            | (McpLifecyclePhase::Shutdown, McpLifecyclePhase::Cleanup) => true,
            (_, McpLifecyclePhase::Shutdown) => from != McpLifecyclePhase::Cleanup,
            (_, McpLifecyclePhase::ErrorSurfacing) => {
                from != McpLifecyclePhase::Cleanup && from != McpLifecyclePhase::Shutdown
            }
            _ => false,
        }
    }

    pub fn run_phase(&mut self, phase: McpLifecyclePhase) -> McpPhaseResult {
        let started = Instant::now();

        if let Some(current_phase) = self.state.current_phase() {
            if current_phase == McpLifecyclePhase::ErrorSurfacing
                && phase == McpLifecyclePhase::Ready
                && !self.state.can_resume_after_error()
            {
                return self.record_failure(McpErrorSurface::new(
                    phase,
                    None,
                    "cannot return to ready after a non-recoverable MCP lifecycle failure",
                    BTreeMap::from([
                        ("from".to_string(), current_phase.to_string()),
                        ("to".to_string(), phase.to_string()),
                    ]),
                    false,
                ));
            }

            if !Self::validate_phase_transition(current_phase, phase) {
                return self.record_failure(McpErrorSurface::new(
                    phase,
                    None,
                    format!("invalid MCP lifecycle transition from {current_phase} to {phase}"),
                    BTreeMap::from([
                        ("from".to_string(), current_phase.to_string()),
                        ("to".to_string(), phase.to_string()),
                    ]),
                    false,
                ));
            }
        } else if phase != McpLifecyclePhase::ConfigLoad {
            return self.record_failure(McpErrorSurface::new(
                phase,
                None,
                format!("invalid initial MCP lifecycle phase {phase}"),
                BTreeMap::from([("phase".to_string(), phase.to_string())]),
                false,
            ));
        }

        self.state.record_phase(phase);
        let result = McpPhaseResult::Success {
            phase,
            duration: started.elapsed(),
        };
        self.state.record_result(result.clone());
        result
    }

    pub fn record_failure(&mut self, error: McpErrorSurface) -> McpPhaseResult {
        let phase = error.phase;
        self.state.record_error(error.clone());
        self.state.record_phase(McpLifecyclePhase::ErrorSurfacing);
        let result = McpPhaseResult::Failure { phase, error };
        self.state.record_result(result.clone());
        result
    }

    pub fn record_timeout(
        &mut self,
        phase: McpLifecyclePhase,
        waited: Duration,
        server_name: Option<String>,
        mut context: BTreeMap<String, String>,
    ) -> McpPhaseResult {
        context.insert("waited_ms".to_string(), waited.as_millis().to_string());
        let error = McpErrorSurface::new(
            phase,
            server_name,
            format!(
                "MCP lifecycle phase {phase} timed out after {} ms",
                waited.as_millis()
            ),
            context,
            true,
        );
        self.state.record_error(error.clone());
        self.state.record_phase(McpLifecyclePhase::ErrorSurfacing);
        let result = McpPhaseResult::Timeout {
            phase,
            waited,
            error,
        };
        self.state.record_result(result.clone());
        result
    }
}

fn dedupe_sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    #[test]
    fn phase_display_matches_serde_name() {
        // given
        let phases = McpLifecyclePhase::all();

        // when
        let serialized = phases
            .into_iter()
            .map(|phase| {
                (
                    phase.to_string(),
                    serde_json::to_value(phase).expect("serialize phase"),
                )
            })
            .collect::<Vec<_>>();

        // then
        for (display, json_value) in serialized {
            assert_eq!(json_value, json!(display));
        }
    }

    #[test]
    fn given_startup_path_when_running_to_cleanup_then_each_control_transition_succeeds() {
        // given
        let mut validator = McpLifecycleValidator::new();
        let phases = [
            McpLifecyclePhase::ConfigLoad,
            McpLifecyclePhase::ServerRegistration,
            McpLifecyclePhase::SpawnConnect,
            McpLifecyclePhase::InitializeHandshake,
            McpLifecyclePhase::ToolDiscovery,
            McpLifecyclePhase::ResourceDiscovery,
            McpLifecyclePhase::Ready,
            McpLifecyclePhase::Invocation,
            McpLifecyclePhase::Ready,
            McpLifecyclePhase::Shutdown,
            McpLifecyclePhase::Cleanup,
        ];

        // when
        let results = phases
            .into_iter()
            .map(|phase| validator.run_phase(phase))
            .collect::<Vec<_>>();

        // then
        assert!(results
            .iter()
            .all(|result| matches!(result, McpPhaseResult::Success { .. })));
        assert_eq!(
            validator.state().current_phase(),
            Some(McpLifecyclePhase::Cleanup)
        );
        for phase in [
            McpLifecyclePhase::ConfigLoad,
            McpLifecyclePhase::ServerRegistration,
            McpLifecyclePhase::SpawnConnect,
            McpLifecyclePhase::InitializeHandshake,
            McpLifecyclePhase::ToolDiscovery,
            McpLifecyclePhase::ResourceDiscovery,
            McpLifecyclePhase::Ready,
            McpLifecyclePhase::Invocation,
            McpLifecyclePhase::Shutdown,
            McpLifecyclePhase::Cleanup,
        ] {
            assert!(validator.state().phase_timestamp(phase).is_some());
        }
    }

    #[test]
    fn given_tool_discovery_when_resource_discovery_is_skipped_then_ready_is_still_allowed() {
        // given
        let mut validator = McpLifecycleValidator::new();
        for phase in [
            McpLifecyclePhase::ConfigLoad,
            McpLifecyclePhase::ServerRegistration,
            McpLifecyclePhase::SpawnConnect,
            McpLifecyclePhase::InitializeHandshake,
            McpLifecyclePhase::ToolDiscovery,
        ] {
            let result = validator.run_phase(phase);
            assert!(matches!(result, McpPhaseResult::Success { .. }));
        }

        // when
        let result = validator.run_phase(McpLifecyclePhase::Ready);

        // then
        assert!(matches!(result, McpPhaseResult::Success { .. }));
        assert_eq!(
            validator.state().current_phase(),
            Some(McpLifecyclePhase::Ready)
        );
    }

    #[test]
    fn validates_expected_phase_transitions() {
        // given
        let valid_transitions = [
            (
                McpLifecyclePhase::ConfigLoad,
                McpLifecyclePhase::ServerRegistration,
            ),
            (
                McpLifecyclePhase::ServerRegistration,
                McpLifecyclePhase::SpawnConnect,
            ),
            (
                McpLifecyclePhase::SpawnConnect,
                McpLifecyclePhase::InitializeHandshake,
            ),
            (
                McpLifecyclePhase::InitializeHandshake,
                McpLifecyclePhase::ToolDiscovery,
            ),
            (
                McpLifecyclePhase::ToolDiscovery,
                McpLifecyclePhase::ResourceDiscovery,
            ),
            (McpLifecyclePhase::ToolDiscovery, McpLifecyclePhase::Ready),
            (
                McpLifecyclePhase::ResourceDiscovery,
                McpLifecyclePhase::Ready,
            ),
            (McpLifecyclePhase::Ready, McpLifecyclePhase::Invocation),
            (McpLifecyclePhase::Invocation, McpLifecyclePhase::Ready),
            (McpLifecyclePhase::Ready, McpLifecyclePhase::Shutdown),
            (
                McpLifecyclePhase::Invocation,
                McpLifecyclePhase::ErrorSurfacing,
            ),
            (
                McpLifecyclePhase::ErrorSurfacing,
                McpLifecyclePhase::Shutdown,
            ),
            (McpLifecyclePhase::Shutdown, McpLifecyclePhase::Cleanup),
        ];

        // when / then
        for (from, to) in valid_transitions {
            assert!(McpLifecycleValidator::validate_phase_transition(from, to));
        }
        assert!(!McpLifecycleValidator::validate_phase_transition(
            McpLifecyclePhase::Ready,
            McpLifecyclePhase::ConfigLoad,
        ));
        assert!(!McpLifecycleValidator::validate_phase_transition(
            McpLifecyclePhase::Cleanup,
            McpLifecyclePhase::Ready,
        ));
    }

    #[test]
    fn given_invalid_transition_when_running_phase_then_structured_failure_is_recorded() {
        // given
        let mut validator = McpLifecycleValidator::new();
        let _ = validator.run_phase(McpLifecyclePhase::ConfigLoad);
        let _ = validator.run_phase(McpLifecyclePhase::ServerRegistration);

        // when
        let result = validator.run_phase(McpLifecyclePhase::Ready);

        // then
        match result {
            McpPhaseResult::Failure { phase, error } => {
                assert_eq!(phase, McpLifecyclePhase::Ready);
                assert!(!error.recoverable);
                assert_eq!(error.phase, McpLifecyclePhase::Ready);
                assert_eq!(
                    error.context.get("from").map(String::as_str),
                    Some("server_registration")
                );
                assert_eq!(error.context.get("to").map(String::as_str), Some("ready"));
            }
            other => panic!("expected failure result, got {other:?}"),
        }
        assert_eq!(
            validator.state().current_phase(),
            Some(McpLifecyclePhase::ErrorSurfacing)
        );
        assert_eq!(
            validator
                .state()
                .errors_for_phase(McpLifecyclePhase::Ready)
                .len(),
            1
        );
    }

    #[test]
    fn given_each_phase_when_failure_is_recorded_then_error_is_tracked_per_phase() {
        // given
        let mut validator = McpLifecycleValidator::new();

        // when / then
        for phase in McpLifecyclePhase::all() {
            let result = validator.record_failure(McpErrorSurface::new(
                phase,
                Some("alpha".to_string()),
                format!("failure at {phase}"),
                BTreeMap::from([("server".to_string(), "alpha".to_string())]),
                phase == McpLifecyclePhase::ResourceDiscovery,
            ));

            match result {
                McpPhaseResult::Failure {
                    phase: failed_phase,
                    error,
                } => {
                    assert_eq!(failed_phase, phase);
                    assert_eq!(error.phase, phase);
                    assert_eq!(
                        error.recoverable,
                        phase == McpLifecyclePhase::ResourceDiscovery
                    );
                }
                other => panic!("expected failure result, got {other:?}"),
            }
            assert_eq!(validator.state().errors_for_phase(phase).len(), 1);
        }
    }

    #[test]
    fn given_spawn_connect_timeout_when_recorded_then_waited_duration_is_preserved() {
        // given
        let mut validator = McpLifecycleValidator::new();
        let waited = Duration::from_millis(250);

        // when
        let result = validator.record_timeout(
            McpLifecyclePhase::SpawnConnect,
            waited,
            Some("alpha".to_string()),
            BTreeMap::from([("attempt".to_string(), "1".to_string())]),
        );

        // then
        match result {
            McpPhaseResult::Timeout {
                phase,
                waited: actual,
                error,
            } => {
                assert_eq!(phase, McpLifecyclePhase::SpawnConnect);
                assert_eq!(actual, waited);
                assert!(error.recoverable);
                assert_eq!(error.server_name.as_deref(), Some("alpha"));
            }
            other => panic!("expected timeout result, got {other:?}"),
        }
        let errors = validator
            .state()
            .errors_for_phase(McpLifecyclePhase::SpawnConnect);
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].context.get("waited_ms").map(String::as_str),
            Some("250")
        );
        assert_eq!(
            validator.state().current_phase(),
            Some(McpLifecyclePhase::ErrorSurfacing)
        );
    }

    #[test]
    fn given_partial_server_health_when_building_degraded_report_then_missing_tools_are_reported() {
        // given
        let failed = vec![McpFailedServer {
            server_name: "broken".to_string(),
            phase: McpLifecyclePhase::InitializeHandshake,
            error: McpErrorSurface::new(
                McpLifecyclePhase::InitializeHandshake,
                Some("broken".to_string()),
                "initialize failed",
                BTreeMap::from([("reason".to_string(), "broken pipe".to_string())]),
                false,
            ),
        }];

        // when
        let report = McpDegradedReport::new(
            vec!["alpha".to_string(), "beta".to_string(), "alpha".to_string()],
            failed,
            vec![
                "alpha.echo".to_string(),
                "beta.search".to_string(),
                "alpha.echo".to_string(),
            ],
            vec![
                "alpha.echo".to_string(),
                "beta.search".to_string(),
                "broken.fetch".to_string(),
            ],
        );

        // then
        assert_eq!(
            report.working_servers,
            vec!["alpha".to_string(), "beta".to_string()]
        );
        assert_eq!(report.failed_servers.len(), 1);
        assert_eq!(report.failed_servers[0].server_name, "broken");
        assert_eq!(
            report.available_tools,
            vec!["alpha.echo".to_string(), "beta.search".to_string()]
        );
        assert_eq!(report.missing_tools, vec!["broken.fetch".to_string()]);
    }

    #[test]
    fn given_failure_during_resource_discovery_when_shutting_down_then_cleanup_still_succeeds() {
        // given
        let mut validator = McpLifecycleValidator::new();
        for phase in [
            McpLifecyclePhase::ConfigLoad,
            McpLifecyclePhase::ServerRegistration,
            McpLifecyclePhase::SpawnConnect,
            McpLifecyclePhase::InitializeHandshake,
            McpLifecyclePhase::ToolDiscovery,
        ] {
            let result = validator.run_phase(phase);
            assert!(matches!(result, McpPhaseResult::Success { .. }));
        }
        let _ = validator.record_failure(McpErrorSurface::new(
            McpLifecyclePhase::ResourceDiscovery,
            Some("alpha".to_string()),
            "resource listing failed",
            BTreeMap::from([("reason".to_string(), "timeout".to_string())]),
            true,
        ));

        // when
        let shutdown = validator.run_phase(McpLifecyclePhase::Shutdown);
        let cleanup = validator.run_phase(McpLifecyclePhase::Cleanup);

        // then
        assert!(matches!(shutdown, McpPhaseResult::Success { .. }));
        assert!(matches!(cleanup, McpPhaseResult::Success { .. }));
        assert_eq!(
            validator.state().current_phase(),
            Some(McpLifecyclePhase::Cleanup)
        );
        assert!(validator
            .state()
            .phase_timestamp(McpLifecyclePhase::ErrorSurfacing)
            .is_some());
    }

    #[test]
    fn error_surface_display_includes_phase_server_and_recoverable_flag() {
        // given
        let error = McpErrorSurface::new(
            McpLifecyclePhase::SpawnConnect,
            Some("alpha".to_string()),
            "process exited early",
            BTreeMap::from([("exit_code".to_string(), "1".to_string())]),
            true,
        );

        // when
        let rendered = error.to_string();

        // then
        assert!(rendered.contains("spawn_connect"));
        assert!(rendered.contains("process exited early"));
        assert!(rendered.contains("server: alpha"));
        assert!(rendered.contains("recoverable"));
        let trait_object: &dyn std::error::Error = &error;
        assert_eq!(trait_object.to_string(), rendered);
    }

    #[test]
    fn given_nonrecoverable_failure_when_returning_to_ready_then_validator_rejects_resume() {
        // given
        let mut validator = McpLifecycleValidator::new();
        for phase in [
            McpLifecyclePhase::ConfigLoad,
            McpLifecyclePhase::ServerRegistration,
            McpLifecyclePhase::SpawnConnect,
            McpLifecyclePhase::InitializeHandshake,
            McpLifecyclePhase::ToolDiscovery,
            McpLifecyclePhase::Ready,
        ] {
            let result = validator.run_phase(phase);
            assert!(matches!(result, McpPhaseResult::Success { .. }));
        }
        let _ = validator.record_failure(McpErrorSurface::new(
            McpLifecyclePhase::Invocation,
            Some("alpha".to_string()),
            "tool call corrupted the session",
            BTreeMap::from([("reason".to_string(), "invalid frame".to_string())]),
            false,
        ));

        // when
        let result = validator.run_phase(McpLifecyclePhase::Ready);

        // then
        match result {
            McpPhaseResult::Failure { phase, error } => {
                assert_eq!(phase, McpLifecyclePhase::Ready);
                assert!(!error.recoverable);
                assert!(error.message.contains("non-recoverable"));
            }
            other => panic!("expected failure result, got {other:?}"),
        }
        assert_eq!(
            validator.state().current_phase(),
            Some(McpLifecyclePhase::ErrorSurfacing)
        );
    }

    #[test]
    fn given_recoverable_failure_when_returning_to_ready_then_validator_allows_resume() {
        // given
        let mut validator = McpLifecycleValidator::new();
        for phase in [
            McpLifecyclePhase::ConfigLoad,
            McpLifecyclePhase::ServerRegistration,
            McpLifecyclePhase::SpawnConnect,
            McpLifecyclePhase::InitializeHandshake,
            McpLifecyclePhase::ToolDiscovery,
            McpLifecyclePhase::Ready,
        ] {
            let result = validator.run_phase(phase);
            assert!(matches!(result, McpPhaseResult::Success { .. }));
        }
        let _ = validator.record_failure(McpErrorSurface::new(
            McpLifecyclePhase::Invocation,
            Some("alpha".to_string()),
            "tool call failed but can be retried",
            BTreeMap::from([("reason".to_string(), "upstream timeout".to_string())]),
            true,
        ));

        // when
        let result = validator.run_phase(McpLifecyclePhase::Ready);

        // then
        assert!(matches!(result, McpPhaseResult::Success { .. }));
        assert_eq!(
            validator.state().current_phase(),
            Some(McpLifecyclePhase::Ready)
        );
    }
}
