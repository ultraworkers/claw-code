//! Thin compatibility wrapper around [`PermissionPolicy`].
//! Kept for crate-level consumers (`tools`) that still reference
//! `PermissionEnforcer` and `EnforcementResult`. Direct usage is
//! deprecated — use `PermissionPolicy` and `PermissionOutcome` instead.
//!
//! # SAFETY
//!
//! The [`PermissionEnforcer`] has no [`PermissionPrompter`], so in
//! [`PermissionMode::Prompt`] it **unconditionally allows every tool**
//! (see `check` / `check_with_required_mode`). This is safe in the
//! current architecture because the conversation layer
//! (`conversation.rs`) performs the real authorization with a prompter
//! **before** calling into the enforcer. However, any caller that
//! invokes the enforcer **without** the conversation layer's prior
//! authorization will silently bypass all Prompt-mode controls.
//!
//! **Do not use this enforcer as a standalone security boundary.**

use crate::permissions::{PermissionMode, PermissionOutcome, PermissionPolicy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome")]
pub enum EnforcementResult {
    /// Tool execution is allowed.
    Allowed,
    /// Tool execution was denied due to insufficient permissions.
    Denied {
        tool: String,
        active_mode: String,
        required_mode: String,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermissionEnforcer {
    policy: PermissionPolicy,
}

impl PermissionEnforcer {
    #[must_use]
    pub fn new(policy: PermissionPolicy) -> Self {
        Self { policy }
    }

    /// Check whether a tool can be executed under the current permission policy.
    pub fn check(&self, tool_name: &str, input: &str) -> EnforcementResult {
        // When the active mode is Prompt, defer to the caller's interactive
        // prompt flow rather than hard-denying (the enforcer has no prompter).
        if self.policy.active_mode() == PermissionMode::Prompt {
            return EnforcementResult::Allowed;
        }
        match self.policy.authorize(tool_name, input, None) {
            PermissionOutcome::Allow => EnforcementResult::Allowed,
            PermissionOutcome::Deny { reason } => EnforcementResult::Denied {
                tool: tool_name.to_string(),
                active_mode: self.policy.active_mode().as_str().to_string(),
                required_mode: String::new(),
                reason,
            },
        }
    }

    /// Check whether a tool can be executed with an explicitly provided required mode.
    pub fn check_with_required_mode(
        &self,
        tool_name: &str,
        input: &str,
        required_mode: PermissionMode,
    ) -> EnforcementResult {
        // Same Prompt-mode deferral as [`check`] — the enforcer has no prompter,
        // so let the caller (agent runtime) handle interactive prompting.
        if self.policy.active_mode() == PermissionMode::Prompt {
            return EnforcementResult::Allowed;
        }
        match self
            .policy
            .authorize_with_required_mode(tool_name, input, required_mode, None)
        {
            PermissionOutcome::Allow => EnforcementResult::Allowed,
            PermissionOutcome::Deny { reason } => EnforcementResult::Denied {
                tool: tool_name.to_string(),
                active_mode: self.policy.active_mode().as_str().to_string(),
                required_mode: required_mode.as_str().to_string(),
                reason,
            },
        }
    }
}
