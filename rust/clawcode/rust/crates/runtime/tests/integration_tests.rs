#![allow(clippy::doc_markdown, clippy::uninlined_format_args, unused_imports)]
//! Integration tests for cross-module wiring.
//!
//! These tests verify that adjacent modules in the runtime crate actually
//! connect correctly — catching wiring gaps that unit tests miss.

use std::time::Duration;

use runtime::green_contract::{GreenContract, GreenContractOutcome, GreenLevel};
use runtime::{
    apply_policy, BranchFreshness, DiffScope, LaneBlocker, LaneContext, PolicyAction,
    PolicyCondition, PolicyEngine, PolicyRule, ReconcileReason, ReviewStatus, StaleBranchAction,
    StaleBranchPolicy,
};

/// stale_branch + policy_engine integration:
/// When a branch is detected stale, does it correctly flow through
/// PolicyCondition::StaleBranch to generate the expected action?
#[test]
fn stale_branch_detection_flows_into_policy_engine() {
    // given — a stale branch context (2 hours behind main, threshold is 1 hour)
    let stale_context = LaneContext::new(
        "stale-lane",
        0,
        Duration::from_secs(2 * 60 * 60), // 2 hours stale
        LaneBlocker::None,
        ReviewStatus::Pending,
        DiffScope::Full,
        false,
    );

    let engine = PolicyEngine::new(vec![PolicyRule::new(
        "stale-merge-forward",
        PolicyCondition::StaleBranch,
        PolicyAction::MergeForward,
        10,
    )]);

    // when
    let actions = engine.evaluate(&stale_context);

    // then
    assert_eq!(actions, vec![PolicyAction::MergeForward]);
}

/// stale_branch + policy_engine: Fresh branch does NOT trigger stale rules
#[test]
fn fresh_branch_does_not_trigger_stale_policy() {
    let fresh_context = LaneContext::new(
        "fresh-lane",
        0,
        Duration::from_secs(30 * 60), // 30 min stale — under 1 hour threshold
        LaneBlocker::None,
        ReviewStatus::Pending,
        DiffScope::Full,
        false,
    );

    let engine = PolicyEngine::new(vec![PolicyRule::new(
        "stale-merge-forward",
        PolicyCondition::StaleBranch,
        PolicyAction::MergeForward,
        10,
    )]);

    let actions = engine.evaluate(&fresh_context);
    assert!(actions.is_empty());
}

/// green_contract + policy_engine integration:
/// A lane that meets its green contract should be mergeable
#[test]
fn green_contract_satisfied_allows_merge() {
    let contract = GreenContract::new(GreenLevel::Workspace);
    let satisfied = contract.is_satisfied_by(GreenLevel::Workspace);
    assert!(satisfied);

    let exceeded = contract.is_satisfied_by(GreenLevel::MergeReady);
    assert!(exceeded);

    let insufficient = contract.is_satisfied_by(GreenLevel::Package);
    assert!(!insufficient);
}

/// green_contract + policy_engine:
/// Lane with green level below contract requirement gets blocked
#[test]
fn green_contract_unsatisfied_blocks_merge() {
    let context = LaneContext::new(
        "partial-green-lane",
        1, // GreenLevel::Package as u8
        Duration::from_secs(0),
        LaneBlocker::None,
        ReviewStatus::Pending,
        DiffScope::Full,
        false,
    );

    // This is a conceptual test — we need a way to express "requires workspace green"
    // Currently LaneContext has raw green_level: u8, not a contract
    // For now we just verify the policy condition works
    let engine = PolicyEngine::new(vec![PolicyRule::new(
        "workspace-green-required",
        PolicyCondition::GreenAt { level: 3 }, // GreenLevel::Workspace
        PolicyAction::MergeToDev,
        10,
    )]);

    let actions = engine.evaluate(&context);
    assert!(actions.is_empty()); // level 1 < 3, so no merge
}

/// reconciliation + policy_engine integration:
/// A reconciled lane should be handled by reconcile rules, not generic closeout
#[test]
fn reconciled_lane_matches_reconcile_condition() {
    let context = LaneContext::reconciled("reconciled-lane");

    let engine = PolicyEngine::new(vec![
        PolicyRule::new(
            "reconcile-first",
            PolicyCondition::LaneReconciled,
            PolicyAction::Reconcile {
                reason: ReconcileReason::AlreadyMerged,
            },
            5,
        ),
        PolicyRule::new(
            "generic-closeout",
            PolicyCondition::LaneCompleted,
            PolicyAction::CloseoutLane,
            30,
        ),
    ]);

    let actions = engine.evaluate(&context);

    // Both rules fire — reconcile (priority 5) first, then closeout (priority 30)
    assert_eq!(
        actions,
        vec![
            PolicyAction::Reconcile {
                reason: ReconcileReason::AlreadyMerged,
            },
            PolicyAction::CloseoutLane,
        ]
    );
}

/// stale_branch module: apply_policy generates correct actions
#[test]
fn stale_branch_apply_policy_produces_rebase_action() {
    let stale = BranchFreshness::Stale {
        commits_behind: 5,
        missing_fixes: vec!["fix-123".to_string()],
    };

    let action = apply_policy(&stale, StaleBranchPolicy::AutoRebase);
    assert_eq!(action, StaleBranchAction::Rebase);
}

#[test]
fn stale_branch_apply_policy_produces_merge_forward_action() {
    let stale = BranchFreshness::Stale {
        commits_behind: 3,
        missing_fixes: vec![],
    };

    let action = apply_policy(&stale, StaleBranchPolicy::AutoMergeForward);
    assert_eq!(action, StaleBranchAction::MergeForward);
}

#[test]
fn stale_branch_apply_policy_warn_only() {
    let stale = BranchFreshness::Stale {
        commits_behind: 2,
        missing_fixes: vec!["fix-456".to_string()],
    };

    let action = apply_policy(&stale, StaleBranchPolicy::WarnOnly);
    match action {
        StaleBranchAction::Warn { message } => {
            assert!(message.contains("2 commit(s) behind main"));
            assert!(message.contains("fix-456"));
        }
        _ => panic!("expected Warn action, got {:?}", action),
    }
}

#[test]
fn stale_branch_fresh_produces_noop() {
    let fresh = BranchFreshness::Fresh;
    let action = apply_policy(&fresh, StaleBranchPolicy::AutoRebase);
    assert_eq!(action, StaleBranchAction::Noop);
}

/// Combined flow: stale detection + policy + action
#[test]
fn end_to_end_stale_lane_gets_merge_forward_action() {
    // Simulating what a harness would do:
    // 1. Detect branch freshness
    // 2. Build lane context from freshness + other signals
    // 3. Run policy engine
    // 4. Return actions

    // given: detected stale state
    let _freshness = BranchFreshness::Stale {
        commits_behind: 5,
        missing_fixes: vec!["fix-123".to_string()],
    };

    // when: build context and evaluate policy
    let context = LaneContext::new(
        "lane-9411",
        3,                                // Workspace green
        Duration::from_secs(5 * 60 * 60), // 5 hours stale, definitely over threshold
        LaneBlocker::None,
        ReviewStatus::Approved,
        DiffScope::Scoped,
        false,
    );

    let engine = PolicyEngine::new(vec![
        // Priority 5: Check if stale first
        PolicyRule::new(
            "auto-merge-forward-if-stale-and-approved",
            PolicyCondition::And(vec![
                PolicyCondition::StaleBranch,
                PolicyCondition::ReviewPassed,
            ]),
            PolicyAction::MergeForward,
            5,
        ),
        // Priority 10: Normal stale handling
        PolicyRule::new(
            "stale-warning",
            PolicyCondition::StaleBranch,
            PolicyAction::Notify {
                channel: "#build-status".to_string(),
            },
            10,
        ),
    ]);

    let actions = engine.evaluate(&context);

    // then: both rules should fire (stale + approved matches both)
    assert_eq!(
        actions,
        vec![
            PolicyAction::MergeForward,
            PolicyAction::Notify {
                channel: "#build-status".to_string(),
            },
        ]
    );
}

/// Fresh branch with approved review should merge (not stale-blocked)
#[test]
fn fresh_approved_lane_gets_merge_action() {
    let context = LaneContext::new(
        "fresh-approved-lane",
        3,                            // Workspace green
        Duration::from_secs(30 * 60), // 30 min — under 1 hour threshold = fresh
        LaneBlocker::None,
        ReviewStatus::Approved,
        DiffScope::Scoped,
        false,
    );

    let engine = PolicyEngine::new(vec![PolicyRule::new(
        "merge-if-green-approved-not-stale",
        PolicyCondition::And(vec![
            PolicyCondition::GreenAt { level: 3 },
            PolicyCondition::ReviewPassed,
            // NOT PolicyCondition::StaleBranch — fresh lanes bypass this
        ]),
        PolicyAction::MergeToDev,
        5,
    )]);

    let actions = engine.evaluate(&context);
    assert_eq!(actions, vec![PolicyAction::MergeToDev]);
}


