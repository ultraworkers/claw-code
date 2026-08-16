#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapPhase {
    CliEntry,
    FastPathVersion,
    StartupProfiler,
    SystemPromptFastPath,
    ChromeMcpFastPath,
    DaemonWorkerFastPath,
    BridgeFastPath,
    DaemonFastPath,
    BackgroundSessionFastPath,
    TemplateFastPath,
    EnvironmentRunnerFastPath,
    MainRuntime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapPlan {
    phases: Vec<BootstrapPhase>,
}

impl BootstrapPlan {
    #[must_use]
    pub fn claude_code_default() -> Self {
        Self::from_phases(vec![
            BootstrapPhase::CliEntry,
            BootstrapPhase::FastPathVersion,
            BootstrapPhase::StartupProfiler,
            BootstrapPhase::SystemPromptFastPath,
            BootstrapPhase::ChromeMcpFastPath,
            BootstrapPhase::DaemonWorkerFastPath,
            BootstrapPhase::BridgeFastPath,
            BootstrapPhase::DaemonFastPath,
            BootstrapPhase::BackgroundSessionFastPath,
            BootstrapPhase::TemplateFastPath,
            BootstrapPhase::EnvironmentRunnerFastPath,
            BootstrapPhase::MainRuntime,
        ])
    }

    #[must_use]
    pub fn from_phases(phases: Vec<BootstrapPhase>) -> Self {
        let mut deduped = Vec::new();
        for phase in phases {
            if !deduped.contains(&phase) {
                deduped.push(phase);
            }
        }
        Self { phases: deduped }
    }

    #[must_use]
    pub fn phases(&self) -> &[BootstrapPhase] {
        &self.phases
    }
}

#[cfg(test)]
mod tests {
    use super::{BootstrapPhase, BootstrapPlan};

    #[test]
    fn from_phases_deduplicates_while_preserving_order() {
        // given
        let phases = vec![
            BootstrapPhase::CliEntry,
            BootstrapPhase::FastPathVersion,
            BootstrapPhase::CliEntry,
            BootstrapPhase::MainRuntime,
            BootstrapPhase::FastPathVersion,
        ];

        // when
        let plan = BootstrapPlan::from_phases(phases);

        // then
        assert_eq!(
            plan.phases(),
            &[
                BootstrapPhase::CliEntry,
                BootstrapPhase::FastPathVersion,
                BootstrapPhase::MainRuntime,
            ]
        );
    }

    #[test]
    fn claude_code_default_covers_each_phase_once() {
        // given
        let expected = [
            BootstrapPhase::CliEntry,
            BootstrapPhase::FastPathVersion,
            BootstrapPhase::StartupProfiler,
            BootstrapPhase::SystemPromptFastPath,
            BootstrapPhase::ChromeMcpFastPath,
            BootstrapPhase::DaemonWorkerFastPath,
            BootstrapPhase::BridgeFastPath,
            BootstrapPhase::DaemonFastPath,
            BootstrapPhase::BackgroundSessionFastPath,
            BootstrapPhase::TemplateFastPath,
            BootstrapPhase::EnvironmentRunnerFastPath,
            BootstrapPhase::MainRuntime,
        ];

        // when
        let plan = BootstrapPlan::claude_code_default();

        // then
        assert_eq!(plan.phases(), &expected);
    }
}
