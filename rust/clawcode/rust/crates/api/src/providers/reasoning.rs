//! Reasoning-effort registry: per-provider supported levels, wire-translation
//! helpers, and fail-fast validation.
//!
//! This mirrors the dsh reasoning-effort model: a typed [`ReasoningEffort`]
//! enum resolves to a provider-specific wire spelling — or field omission for
//! [`ReasoningEffort::Off`] — before any network I/O. The registry is the
//! single source of truth for which levels a (provider, model) pair exposes
//! and what each level emits on the wire; the provider emit code calls these
//! helpers instead of open-coding the translation.
//!
//! Design notes:
//! - Levels and defaults are structural facts about providers, not deployment
//!   choices, so they live in compiled code (like `MODEL_REGISTRY` and
//!   `is_reasoning_model`). Deployment-varying values — the *selected* level —
//!   flow in through env / settings.json / CLI / agent frontmatter.
//! - `Off` always means "omit the wire field" (`None`), never send the string
//!   `"off"`: OpenAI-compat has no `off` spelling and would reject it.
//! - A non-reasoning model exposes only `Off`; any other level is rejected at
//!   validation time so a stale `--reasoning-effort high` against a
//!   non-reasoning model fails before the request leaves the process.

use crate::providers::openai_compat::is_reasoning_model;
use crate::providers::ProviderKind;
use crate::types::ReasoningEffort;

/// Levels offered by an OpenAI-compatible reasoning model. OpenAI's native
/// `reasoning_effort` accepts `low` / `medium` / `high`; `Off` omits the
/// field. `Max` is absent: native OpenAI has no spelling above `high`, so a
/// profile advertising it would let the selector pick a level the wire cannot
/// honour. Gateways that remap `Max` to a custom spelling should do so in
/// their own provider block, not here.
const OPENAI_REASONING_LEVELS: &[ReasoningEffort] = &[
    ReasoningEffort::Off,
    ReasoningEffort::Low,
    ReasoningEffort::Medium,
    ReasoningEffort::High,
];

/// Levels offered by an Anthropic model under extended thinking. `Off`
/// disables thinking; `Max` maps to the largest budget that fits under the
/// model's output cap.
const ANTHROPIC_LEVELS: &[ReasoningEffort] = &[
    ReasoningEffort::Off,
    ReasoningEffort::Low,
    ReasoningEffort::Medium,
    ReasoningEffort::High,
    ReasoningEffort::Max,
];

/// A model that cannot reason at all exposes only `Off` — reasoning is off and
/// no other level is selectable. Requesting `High` against such a model fails
/// at validation rather than being silently ignored on the wire.
const OFF_ONLY: &[ReasoningEffort] = &[ReasoningEffort::Off];

/// The supported reasoning levels for a (provider, model) pair, in escalation
/// order. Used by selectors and by [`validate_reasoning_effort`].
#[must_use]
pub fn reasoning_levels(provider: ProviderKind, model: &str) -> &'static [ReasoningEffort] {
    match provider {
        ProviderKind::Anthropic => ANTHROPIC_LEVELS,
        ProviderKind::OpenAi => {
            if is_reasoning_model(model) {
                OPENAI_REASONING_LEVELS
            } else {
                OFF_ONLY
            }
        }
    }
}

/// The default reasoning level when no CLI flag, agent frontmatter, env, or
/// settings value selects one. `Off` preserves the provider's own server
/// default (the wire field is omitted); Anthropic defaults to `High` — the
/// highest budget that clears every registered model's output cap (16 384
/// fits under opus's 32 000, where `Max`'s 32 000 would tie it and collide) — so extended
/// thinking stays on with ample headroom unless explicitly disabled.
#[must_use]
pub fn default_reasoning_effort(provider: ProviderKind, _model: &str) -> ReasoningEffort {
    match provider {
        ProviderKind::Anthropic => ReasoningEffort::High,
        ProviderKind::OpenAi => ReasoningEffort::Off,
    }
}

/// Whether a (provider, model) pair honours the given level.
#[must_use]
pub fn supports_level(
    provider: ProviderKind,
    model: &str,
    level: ReasoningEffort,
) -> bool {
    reasoning_levels(provider, model).contains(&level)
}

/// OpenAI-compat wire spelling for a level. `None` means omit the
/// `reasoning_effort` field (used for [`ReasoningEffort::Off`]). Callers must
/// have already validated the level against [`reasoning_levels`]; `Max` returns
/// `None` here only as a defensive fallback because validation rejects it
/// first for native OpenAI models.
#[must_use]
pub fn openai_wire_effort(level: ReasoningEffort) -> Option<&'static str> {
    match level {
        ReasoningEffort::Off | ReasoningEffort::Max => None,
        ReasoningEffort::Low => Some("low"),
        ReasoningEffort::Medium => Some("medium"),
        ReasoningEffort::High => Some("high"),
    }
}

/// Anthropic extended-thinking budget (in tokens) for a level. `None`
/// disables thinking (no `thinking` field on the wire). Ladder: 4 096 /
/// 8 192 / 16 384 / 32 000. `High` (16 384) is the default — the top level
/// that clears every registered model's output cap; `Max` (32 000) ties
/// opus's 32 000 cap (Anthropic requires `budget_tokens < max_tokens`), so it
/// is only safe on the 64 000-cap models and is clamped by the caller when
/// `max_tokens` is lower.
#[must_use]
pub fn anthropic_thinking_budget(level: ReasoningEffort) -> Option<u32> {
    match level {
        ReasoningEffort::Off => None,
        ReasoningEffort::Low => Some(4_096),
        ReasoningEffort::Medium => Some(8_192),
        ReasoningEffort::High => Some(16_384),
        ReasoningEffort::Max => Some(32_000),
    }
}

/// Resolve the Anthropic [`ThinkingConfig`] for a request: derive the level
/// from the `reasoning_effort` string when set, otherwise fall back to the
/// provider default, then map it to a thinking budget. `Off` returns `None`
/// (no `thinking` field on the wire → thinking disabled).
///
/// The returned config is consumed by the Anthropic provider; the OpenAI-compat
/// path ignores `thinking` entirely (it emits `reasoning_effort` instead), so
/// calling this for an OpenAI model is harmless and simply yields `None` under
/// the `Off` default.
#[must_use]
pub fn effective_thinking_config(
    model: &str,
    reasoning_effort: Option<&str>,
) -> Option<crate::types::ThinkingConfig> {
    let canonical = crate::providers::resolve_model_alias(model);
    let provider = crate::providers::detect_provider_kind(&canonical);
    let level = reasoning_effort
        .and_then(ReasoningEffort::from_name)
        .unwrap_or_else(|| default_reasoning_effort(provider, &canonical));
    anthropic_thinking_budget(level).map(|budget| crate::types::ThinkingConfig {
        config_type: "enabled".to_string(),
        budget_tokens: Some(budget),
    })
}

/// Validation failure: the requested level is not supported by the
/// (provider, model) pair. Produced by [`validate_reasoning_effort`] and
/// surfaced before any network I/O so a stale or mistyped level fails fast
/// instead of being silently dropped by the backend.
#[derive(Debug)]
pub struct UnsupportedReasoningEffort {
    /// The model the level was requested against.
    pub model: String,
    /// The level that was rejected.
    pub level: ReasoningEffort,
    /// Levels the (provider, model) pair does support.
    pub supported: &'static [ReasoningEffort],
}

impl std::fmt::Display for UnsupportedReasoningEffort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let supported = self
            .supported
            .iter()
            .map(ReasoningEffort::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        write!(
            f,
            "model \"{}\" does not support reasoning effort \"{}\"; supported: {}",
            self.model,
            self.level.as_str(),
            supported
        )
    }
}

impl std::error::Error for UnsupportedReasoningEffort {}

/// Fail-fast validation: reject a level the (provider, model) pair does not
/// support before the request leaves the process. Returns `Ok(())` when the
/// level is in [`reasoning_levels`].
///
/// Call this at request-build time (once the model is known), not at CLI parse
/// time — the CLI accepts any well-formed level and lets the registry decide
/// whether the resolved model honours it.
pub fn validate_reasoning_effort(
    provider: ProviderKind,
    model: &str,
    level: ReasoningEffort,
) -> Result<(), UnsupportedReasoningEffort> {
    let levels = reasoning_levels(provider, model);
    if levels.contains(&level) {
        Ok(())
    } else {
        Err(UnsupportedReasoningEffort {
            model: model.to_string(),
            level,
            supported: levels,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_reasoning_model_offers_low_medium_high_plus_off() {
        assert_eq!(
            reasoning_levels(ProviderKind::OpenAi, "o4-mini"),
            OPENAI_REASONING_LEVELS
        );
    }

    #[test]
    fn openai_non_reasoning_model_offers_off_only() {
        assert_eq!(
            reasoning_levels(ProviderKind::OpenAi, "gpt-4o"),
            OFF_ONLY
        );
    }

    #[test]
    fn anthropic_offers_all_levels() {
        assert_eq!(
            reasoning_levels(ProviderKind::Anthropic, "claude-sonnet-4-6"),
            ANTHROPIC_LEVELS
        );
    }

    #[test]
    fn openai_wire_effort_omits_off() {
        assert_eq!(openai_wire_effort(ReasoningEffort::Off), None);
        assert_eq!(openai_wire_effort(ReasoningEffort::Low), Some("low"));
        assert_eq!(openai_wire_effort(ReasoningEffort::Medium), Some("medium"));
        assert_eq!(openai_wire_effort(ReasoningEffort::High), Some("high"));
    }

    #[test]
    fn anthropic_budget_disables_off() {
        assert_eq!(anthropic_thinking_budget(ReasoningEffort::Off), None);
        assert_eq!(
            anthropic_thinking_budget(ReasoningEffort::Medium),
            Some(8_192)
        );
        assert_eq!(anthropic_thinking_budget(ReasoningEffort::Max), Some(32_000));
    }

    #[test]
    fn validate_rejects_high_against_non_reasoning_model() {
        let err = validate_reasoning_effort(ProviderKind::OpenAi, "gpt-4o", ReasoningEffort::High)
            .expect_err("gpt-4o is not a reasoning model");
        assert!(err.to_string().contains("gpt-4o"));
        assert!(err.to_string().contains("high"));
    }

    #[test]
    fn validate_rejects_max_against_native_openai() {
        let err = validate_reasoning_effort(ProviderKind::OpenAi, "o4-mini", ReasoningEffort::Max)
            .expect_err("native OpenAI has no max spelling");
        assert!(err.to_string().contains("max"));
        assert!(err.to_string().contains("off, low, medium, high"));
    }

    #[test]
    fn validate_accepts_off_for_every_model() {
        validate_reasoning_effort(ProviderKind::OpenAi, "gpt-4o", ReasoningEffort::Off)
            .expect("off is always supported");
        validate_reasoning_effort(ProviderKind::Anthropic, "claude-opus-4-6", ReasoningEffort::Off)
            .expect("off is always supported");
    }

    #[test]
    fn default_preserves_provider_behaviour() {
        assert_eq!(
            default_reasoning_effort(ProviderKind::OpenAi, "o4-mini"),
            ReasoningEffort::Off
        );
        assert_eq!(
            default_reasoning_effort(ProviderKind::Anthropic, "claude-sonnet-4-6"),
            ReasoningEffort::High
        );
    }

    #[test]
    fn effective_thinking_config_off_disables_thinking() {
        assert!(effective_thinking_config("claude-sonnet-4-6", Some("off")).is_none());
    }

    #[test]
    fn effective_thinking_config_high_scales_budget() {
        let config = effective_thinking_config("claude-sonnet-4-6", Some("high"))
            .expect("high must produce a thinking config");
        assert_eq!(config.config_type, "enabled");
        assert_eq!(config.budget_tokens, Some(16_384));
    }

    #[test]
    fn effective_thinking_config_default_is_high_budget() {
        // No level requested → Anthropic default High → 16 384 budget. `High`
        // is the top level that clears every model's output cap (opus 32 000);
        // `Max` (32 768) would collide with it.
        let config = effective_thinking_config("claude-sonnet-4-6", None)
            .expect("default must produce a thinking config");
        assert_eq!(config.budget_tokens, Some(16_384));
    }

    #[test]
    fn effective_thinking_config_openai_default_is_none() {
        // OpenAI default is Off → no thinking field (OpenAI uses
        // `reasoning_effort`, not `thinking`). `gpt-4o` carries the `gpt-`
        // prefix so provider detection is environment-independent.
        assert!(effective_thinking_config("gpt-4o", None).is_none());
    }
}
