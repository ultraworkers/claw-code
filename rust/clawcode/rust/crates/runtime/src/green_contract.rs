use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GreenLevel {
    TargetedTests,
    Package,
    Workspace,
    MergeReady,
}

impl GreenLevel {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TargetedTests => "targeted_tests",
            Self::Package => "package",
            Self::Workspace => "workspace",
            Self::MergeReady => "merge_ready",
        }
    }
}

impl std::fmt::Display for GreenLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GreenContract {
    pub required_level: GreenLevel,
}

impl GreenContract {
    #[must_use]
    pub fn new(required_level: GreenLevel) -> Self {
        Self { required_level }
    }

    #[must_use]
    pub fn evaluate(self, observed_level: Option<GreenLevel>) -> GreenContractOutcome {
        match observed_level {
            Some(level) if level >= self.required_level => GreenContractOutcome::Satisfied {
                required_level: self.required_level,
                observed_level: level,
            },
            _ => GreenContractOutcome::Unsatisfied {
                required_level: self.required_level,
                observed_level,
            },
        }
    }

    #[must_use]
    pub fn is_satisfied_by(self, observed_level: GreenLevel) -> bool {
        observed_level >= self.required_level
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum GreenContractOutcome {
    Satisfied {
        required_level: GreenLevel,
        observed_level: GreenLevel,
    },
    Unsatisfied {
        required_level: GreenLevel,
        observed_level: Option<GreenLevel>,
    },
}

impl GreenContractOutcome {
    #[must_use]
    pub fn is_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_matching_level_when_evaluating_contract_then_it_is_satisfied() {
        // given
        let contract = GreenContract::new(GreenLevel::Package);

        // when
        let outcome = contract.evaluate(Some(GreenLevel::Package));

        // then
        assert_eq!(
            outcome,
            GreenContractOutcome::Satisfied {
                required_level: GreenLevel::Package,
                observed_level: GreenLevel::Package,
            }
        );
        assert!(outcome.is_satisfied());
    }

    #[test]
    fn given_higher_level_when_checking_requirement_then_it_still_satisfies_contract() {
        // given
        let contract = GreenContract::new(GreenLevel::TargetedTests);

        // when
        let is_satisfied = contract.is_satisfied_by(GreenLevel::Workspace);

        // then
        assert!(is_satisfied);
    }

    #[test]
    fn given_lower_level_when_evaluating_contract_then_it_is_unsatisfied() {
        // given
        let contract = GreenContract::new(GreenLevel::Workspace);

        // when
        let outcome = contract.evaluate(Some(GreenLevel::Package));

        // then
        assert_eq!(
            outcome,
            GreenContractOutcome::Unsatisfied {
                required_level: GreenLevel::Workspace,
                observed_level: Some(GreenLevel::Package),
            }
        );
        assert!(!outcome.is_satisfied());
    }

    #[test]
    fn given_no_green_level_when_evaluating_contract_then_contract_is_unsatisfied() {
        // given
        let contract = GreenContract::new(GreenLevel::MergeReady);

        // when
        let outcome = contract.evaluate(None);

        // then
        assert_eq!(
            outcome,
            GreenContractOutcome::Unsatisfied {
                required_level: GreenLevel::MergeReady,
                observed_level: None,
            }
        );
    }
}
