/// Approval Token System
///
/// Structured permission grants with delegation chain and audit trail.
/// Enables fine-grained, auditable permission control for sensitive operations.
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// An approval scope defines what the token authorizes
#[derive(Debug, Clone)]
pub struct ApprovalScope {
    pub policy: String,
    pub action: String,
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub paths: Vec<String>,
    pub max_uses: u32,
}

impl ApprovalScope {
    pub fn new(policy: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            policy: policy.into(),
            action: action.into(),
            repository: None,
            branch: None,
            paths: vec![],
            max_uses: 1,
        }
    }

    pub fn with_repository(mut self, repo: impl Into<String>) -> Self {
        self.repository = Some(repo.into());
        self
    }

    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    pub fn with_paths(mut self, paths: Vec<String>) -> Self {
        self.paths = paths;
        self
    }

    pub fn with_max_uses(mut self, max: u32) -> Self {
        self.max_uses = max;
        self
    }
}

/// A hop in the delegation chain
#[derive(Debug, Clone)]
pub struct ApprovalDelegationHop {
    pub actor: String,
    pub reason: String,
    pub session_id: Option<String>,
    pub timestamp: u64,
}

impl ApprovalDelegationHop {
    pub fn new(actor: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            actor: actor.into(),
            reason: reason.into(),
            session_id: None,
            timestamp: current_timestamp(),
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// Status of an approval token
#[derive(Debug, Clone, PartialEq)]
pub enum ApprovalTokenStatus {
    Pending,
    Granted,
    Revoked,
    Expired,
    Exhausted,
}

/// An approval token grant
#[derive(Debug, Clone)]
pub struct ApprovalTokenGrant {
    pub token: String,
    pub scope: ApprovalScope,
    pub status: ApprovalTokenStatus,
    pub granted_at: u64,
    pub expires_at: Option<u64>,
    pub uses_remaining: u32,
    pub delegation_chain: Vec<ApprovalDelegationHop>,
    pub metadata: HashMap<String, String>,
}

impl ApprovalTokenGrant {
    pub fn pending(scope: ApprovalScope) -> Self {
        let token = generate_token();
        let max_uses = scope.max_uses;
        Self {
            token,
            scope,
            status: ApprovalTokenStatus::Pending,
            granted_at: current_timestamp(),
            expires_at: None,
            uses_remaining: max_uses,
            delegation_chain: vec![],
            metadata: HashMap::new(),
        }
    }

    pub fn granted(mut self, approver: impl Into<String>, reason: &str) -> Self {
        self.status = ApprovalTokenStatus::Granted;
        self.delegation_chain
            .push(ApprovalDelegationHop::new(approver, reason));
        self
    }

    pub fn approve(&mut self) {
        self.status = ApprovalTokenStatus::Granted;
    }

    pub fn revoke(&mut self) {
        self.status = ApprovalTokenStatus::Revoked;
    }

    pub fn set_expires_at(&mut self, epoch_seconds: u64) {
        self.expires_at = Some(epoch_seconds);
    }

    pub fn set_max_uses(&mut self, max: u32) {
        self.scope.max_uses = max;
        self.uses_remaining = max;
    }

    pub fn add_delegation_hop(&mut self, hop: ApprovalDelegationHop) {
        self.delegation_chain.push(hop);
    }

    pub fn is_valid(&self) -> bool {
        if self.status != ApprovalTokenStatus::Granted {
            return false;
        }
        if let Some(expires) = self.expires_at {
            if current_timestamp() > expires {
                return false;
            }
        }
        self.uses_remaining > 0
    }

    pub fn consume_use(&mut self) -> Result<(), &'static str> {
        if !self.is_valid() {
            return Err("Token is not valid");
        }
        self.uses_remaining -= 1;
        if self.uses_remaining == 0 {
            self.status = ApprovalTokenStatus::Exhausted;
        }
        Ok(())
    }

    pub fn delegation_chain(&self) -> &[ApprovalDelegationHop] {
        &self.delegation_chain
    }
}

/// Audit log entry for token operations
#[derive(Debug, Clone)]
pub struct ApprovalTokenAudit {
    pub token: String,
    pub action: TokenAuditAction,
    pub actor: String,
    pub timestamp: u64,
    pub details: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum TokenAuditAction {
    Created,
    Granted,
    Revoked,
    Consumed,
    Expired,
}

/// Errors in token operations
#[derive(Debug, Clone)]
pub enum ApprovalTokenError {
    TokenNotFound,
    TokenExpired,
    TokenExhausted,
    TokenRevoked,
    InvalidToken,
}

/// Approval token ledger — tracks all tokens and provides audit trail
#[derive(Debug, Default)]
pub struct ApprovalTokenLedger {
    tokens: HashMap<String, ApprovalTokenGrant>,
    audit_log: Vec<ApprovalTokenAudit>,
}

impl ApprovalTokenLedger {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
            audit_log: vec![],
        }
    }

    pub fn insert(&mut self, grant: ApprovalTokenGrant) {
        self.audit_log.push(ApprovalTokenAudit {
            token: grant.token.clone(),
            action: TokenAuditAction::Created,
            actor: "system".to_string(),
            timestamp: current_timestamp(),
            details: HashMap::new(),
        });
        self.tokens.insert(grant.token.clone(), grant);
    }

    pub fn get(&self, token: &str) -> Option<&ApprovalTokenGrant> {
        self.tokens.get(token)
    }

    pub fn get_mut(&mut self, token: &str) -> Option<&mut ApprovalTokenGrant> {
        self.tokens.get_mut(token)
    }

    pub fn revoke(
        &mut self,
        token: &str,
        actor: &str,
    ) -> Result<ApprovalTokenAudit, ApprovalTokenError> {
        let grant = self
            .tokens
            .get_mut(token)
            .ok_or(ApprovalTokenError::TokenNotFound)?;
        grant.revoke();
        let audit = ApprovalTokenAudit {
            token: token.to_string(),
            action: TokenAuditAction::Revoked,
            actor: actor.to_string(),
            timestamp: current_timestamp(),
            details: [("reason".to_string(), "manual revocation".to_string())]
                .into_iter()
                .collect(),
        };
        self.audit_log.push(audit.clone());
        Ok(audit)
    }

    pub fn verify(&self, token: &str) -> Result<&ApprovalTokenGrant, ApprovalTokenError> {
        let grant = self
            .tokens
            .get(token)
            .ok_or(ApprovalTokenError::TokenNotFound)?;
        if grant.status == ApprovalTokenStatus::Expired {
            return Err(ApprovalTokenError::TokenExpired);
        }
        if grant.status == ApprovalTokenStatus::Exhausted {
            return Err(ApprovalTokenError::TokenExhausted);
        }
        if grant.status == ApprovalTokenStatus::Revoked {
            return Err(ApprovalTokenError::TokenRevoked);
        }
        if grant.status != ApprovalTokenStatus::Granted {
            return Err(ApprovalTokenError::InvalidToken);
        }
        Ok(grant)
    }

    pub fn consume(&mut self, token: &str) -> Result<(), ApprovalTokenError> {
        let grant = self
            .tokens
            .get_mut(token)
            .ok_or(ApprovalTokenError::TokenNotFound)?;
        if let Err(_e) = grant.consume_use() {
            return Err(ApprovalTokenError::InvalidToken);
        }
        self.audit_log.push(ApprovalTokenAudit {
            token: token.to_string(),
            action: TokenAuditAction::Consumed,
            actor: "system".to_string(),
            timestamp: current_timestamp(),
            details: HashMap::new(),
        });
        Ok(())
    }

    pub fn audit_log(&self) -> &[ApprovalTokenAudit] {
        &self.audit_log
    }

    pub fn active_tokens(&self) -> Vec<&ApprovalTokenGrant> {
        self.tokens.values().filter(|t| t.is_valid()).collect()
    }

    pub fn grant_for_action(&mut self, policy: &str, action: &str, approver: &str) -> String {
        let scope = ApprovalScope::new(policy, action);
        let mut grant = ApprovalTokenGrant::pending(scope);
        grant = grant.granted(approver, "auto-approved for action");
        let token = grant.token.clone();
        self.insert(grant);
        token
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn generate_token() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("approval_{:x}", ts)
}

#[allow(dead_code)]
/// Helper to create a one-time approval token
pub fn one_time_approval(policy: &str, action: &str, approver: &str) -> ApprovalTokenGrant {
    let scope = ApprovalScope::new(policy, action);
    ApprovalTokenGrant::pending(scope).granted(approver, "one-time approval")
}

#[allow(dead_code)]
/// Helper to create a multi-use approval token
pub fn multi_use_approval(
    policy: &str,
    action: &str,
    approver: &str,
    max_uses: u32,
    expires_in_secs: u64,
) -> ApprovalTokenGrant {
    let scope = ApprovalScope::new(policy, action).with_max_uses(max_uses);
    let mut token = ApprovalTokenGrant::pending(scope);
    token = token.granted(approver, "multi-use approval");
    token.set_expires_at(current_timestamp() + expires_in_secs);
    token.set_max_uses(max_uses);
    token
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_lifecycle() {
        let mut ledger = ApprovalTokenLedger::new();
        let grant = ApprovalTokenGrant::pending(ApprovalScope::new("test_policy", "test_action"))
            .granted("admin", "test approval");
        let token = grant.token.clone();
        ledger.insert(grant);
        assert!(ledger.verify(&token).is_ok());
        assert!(ledger.consume(&token).is_ok());
        assert!(matches!(
            ledger.verify(&token),
            Err(ApprovalTokenError::TokenExhausted)
        ));
    }

    #[test]
    fn test_token_revocation() {
        let mut ledger = ApprovalTokenLedger::new();
        let grant = ApprovalTokenGrant::pending(ApprovalScope::new("test_policy", "test_action"))
            .granted("admin", "test approval");
        let token = grant.token.clone();
        ledger.insert(grant);
        assert!(ledger.revoke(&token, "admin").is_ok());
        assert!(matches!(
            ledger.verify(&token),
            Err(ApprovalTokenError::TokenRevoked)
        ));
    }

    #[test]
    fn test_delegation_chain() {
        let mut grant = ApprovalTokenGrant::pending(ApprovalScope::new("policy", "action"))
            .granted("admin", "initial approval");
        grant.add_delegation_hop(ApprovalDelegationHop::new(
            "delegate",
            "delegated authority",
        ));
        assert_eq!(grant.delegation_chain().len(), 2);
        assert_eq!(grant.delegation_chain()[0].actor, "admin");
        assert_eq!(grant.delegation_chain()[1].actor, "delegate");
    }

    #[test]
    fn test_multi_use_token() {
        let mut ledger = ApprovalTokenLedger::new();
        let mut grant =
            ApprovalTokenGrant::pending(ApprovalScope::new("test", "action").with_max_uses(3))
                .granted("admin", "multi-use");
        grant.set_max_uses(3);
        let token = grant.token.clone();
        ledger.insert(grant);
        assert!(ledger.consume(&token).is_ok());
        assert!(ledger.consume(&token).is_ok());
        assert!(ledger.consume(&token).is_ok());
        assert!(ledger.consume(&token).is_err());
    }

    #[test]
    fn test_audit_log() {
        let mut ledger = ApprovalTokenLedger::new();
        let grant = ApprovalTokenGrant::pending(ApprovalScope::new("test", "action"))
            .granted("admin", "test");
        let token = grant.token.clone();
        ledger.insert(grant);
        ledger.consume(&token).unwrap();
        assert_eq!(ledger.audit_log().len(), 2);
        assert!(matches!(
            ledger.audit_log()[0].action,
            TokenAuditAction::Created
        ));
        assert!(matches!(
            ledger.audit_log()[1].action,
            TokenAuditAction::Consumed
        ));
    }
}
