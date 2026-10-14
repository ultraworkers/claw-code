use crate::session::Session;

const DEFAULT_INPUT_COST_PER_MILLION: f64 = 3.0;
const DEFAULT_OUTPUT_COST_PER_MILLION: f64 = 15.0;
const DEFAULT_CACHE_CREATION_COST_PER_MILLION: f64 = 3.75;
const DEFAULT_CACHE_READ_COST_PER_MILLION: f64 = 0.3;

/// Per-million-token pricing used for cost estimation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPricing {
    pub input_cost_per_million: f64,
    pub output_cost_per_million: f64,
    pub cache_creation_cost_per_million: f64,
    pub cache_read_cost_per_million: f64,
}

impl ModelPricing {
    #[must_use]
    pub const fn default_sonnet_tier() -> Self {
        Self::per_million(
            DEFAULT_INPUT_COST_PER_MILLION,
            DEFAULT_OUTPUT_COST_PER_MILLION,
            DEFAULT_CACHE_CREATION_COST_PER_MILLION,
            DEFAULT_CACHE_READ_COST_PER_MILLION,
        )
    }

    const fn per_million(input: f64, output: f64, cache_creation: f64, cache_read: f64) -> Self {
        Self {
            input_cost_per_million: input,
            output_cost_per_million: output,
            cache_creation_cost_per_million: cache_creation,
            cache_read_cost_per_million: cache_read,
        }
    }
}

/// Token counters accumulated for a conversation turn or session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
}

/// Estimated dollar cost derived from a [`TokenUsage`] sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UsageCostEstimate {
    pub input_cost_usd: f64,
    pub output_cost_usd: f64,
    pub cache_creation_cost_usd: f64,
    pub cache_read_cost_usd: f64,
}

impl UsageCostEstimate {
    #[must_use]
    pub fn total_cost_usd(self) -> f64 {
        self.input_cost_usd
            + self.output_cost_usd
            + self.cache_creation_cost_usd
            + self.cache_read_cost_usd
    }
}

/// Returns pricing metadata for a known model alias or family.
///
/// Anthropic rates follow <https://platform.claude.com/docs/en/about-claude/pricing>, using
/// 5-minute cache writes. Unversioned ids fall back to the tier the bare alias resolves to.
#[must_use]
pub fn pricing_for_model(model: &str) -> Option<ModelPricing> {
    let normalized = model.to_ascii_lowercase();
    if normalized.contains("fable") || normalized.contains("mythos") {
        let version =
            family_version(&normalized, "fable").or_else(|| family_version(&normalized, "mythos"));
        return Some(match version {
            Some((5, 1)) => ModelPricing::per_million(10.0, 50.0, 12.5, 0.25),
            _ => ModelPricing::per_million(10.0, 50.0, 12.5, 1.0),
        });
    }
    if normalized.contains("haiku") {
        if normalized.contains("3-5-haiku") {
            return Some(ModelPricing::per_million(0.8, 4.0, 1.0, 0.08));
        }
        return Some(match family_version(&normalized, "haiku") {
            // Haiku 5.5 bills prompts over 100K tokens at 5x; usage totals are not
            // tracked per request, so estimates use the base tier.
            Some((5, 5)) => ModelPricing::per_million(0.1, 0.5, 0.125, 0.01),
            _ => ModelPricing::per_million(1.0, 5.0, 1.25, 0.1),
        });
    }
    if normalized.contains("opus") {
        return Some(match family_version(&normalized, "opus") {
            Some((5, 5)) => ModelPricing::per_million(4.0, 20.0, 5.0, 0.2),
            // Opus 4/4.1 remain on Bedrock and Google Cloud at the original tier.
            Some((4, 0 | 1)) => ModelPricing::per_million(15.0, 75.0, 18.75, 1.5),
            _ => ModelPricing::per_million(5.0, 25.0, 6.25, 0.5),
        });
    }
    if normalized.contains("sonnet") {
        return Some(match family_version(&normalized, "sonnet") {
            Some((5, 5)) => ModelPricing::per_million(2.0, 10.0, 2.5, 0.1),
            Some((5, 0)) => ModelPricing::per_million(2.0, 10.0, 2.5, 0.2),
            _ => ModelPricing::default_sonnet_tier(),
        });
    }
    // DeepSeek API peak rates (<https://api-docs.deepseek.com/quick_start/pricing>);
    // off-peak usage is billed at half. Cache misses are billed as regular input,
    // with no separate cache-write charge. The retired `deepseek-chat` and
    // `deepseek-reasoner` ids were served by V4 Flash.
    if normalized.contains("deepseek-v4-pro") {
        return Some(ModelPricing::per_million(1.32, 3.96, 1.32, 0.044));
    }
    if [
        "deepseek-flash",
        "deepseek-v4-flash",
        "deepseek-chat",
        "deepseek-reasoner",
    ]
    .iter()
    .any(|id| normalized.contains(id))
    {
        return Some(ModelPricing::per_million(0.3, 1.2, 0.3, 0.006));
    }
    None
}

/// Parses the `(major, minor)` version after `family` in ids such as
/// `claude-opus-4-1-20250805`, `anthropic/claude-sonnet-5-5`, or `claude-opus-4@20250514`.
/// Date suffixes are not treated as a minor version.
fn family_version(model: &str, family: &str) -> Option<(u32, u32)> {
    let rest = model[model.find(family)? + family.len()..].strip_prefix('-')?;
    let mut parts = rest.split(|c: char| !c.is_ascii_digit());
    let parse = |part: &str| part.parse::<u32>().ok().filter(|_| part.len() <= 2);
    let major = parse(parts.next()?)?;
    let minor = parts.next().and_then(parse).unwrap_or(0);
    Some((major, minor))
}

impl TokenUsage {
    #[must_use]
    pub fn total_tokens(self) -> u32 {
        self.input_tokens
            + self.output_tokens
            + self.cache_creation_input_tokens
            + self.cache_read_input_tokens
    }

    #[must_use]
    pub fn estimate_cost_usd(self) -> UsageCostEstimate {
        self.estimate_cost_usd_with_pricing(ModelPricing::default_sonnet_tier())
    }

    #[must_use]
    pub fn estimate_cost_usd_with_pricing(self, pricing: ModelPricing) -> UsageCostEstimate {
        UsageCostEstimate {
            input_cost_usd: cost_for_tokens(self.input_tokens, pricing.input_cost_per_million),
            output_cost_usd: cost_for_tokens(self.output_tokens, pricing.output_cost_per_million),
            cache_creation_cost_usd: cost_for_tokens(
                self.cache_creation_input_tokens,
                pricing.cache_creation_cost_per_million,
            ),
            cache_read_cost_usd: cost_for_tokens(
                self.cache_read_input_tokens,
                pricing.cache_read_cost_per_million,
            ),
        }
    }

    #[must_use]
    pub fn summary_lines(self, label: &str) -> Vec<String> {
        self.summary_lines_for_model(label, None)
    }

    #[must_use]
    pub fn summary_lines_for_model(self, label: &str, model: Option<&str>) -> Vec<String> {
        let pricing = model.and_then(pricing_for_model);
        let cost = pricing.map_or_else(
            || self.estimate_cost_usd(),
            |pricing| self.estimate_cost_usd_with_pricing(pricing),
        );
        let model_suffix =
            model.map_or_else(String::new, |model_name| format!(" model={model_name}"));
        let pricing_suffix = if pricing.is_some() {
            ""
        } else if model.is_some() {
            " pricing=estimated-default"
        } else {
            ""
        };
        vec![
            format!(
                "{label}: total_tokens={} input={} output={} cache_write={} cache_read={} estimated_cost={}{}{}",
                self.total_tokens(),
                self.input_tokens,
                self.output_tokens,
                self.cache_creation_input_tokens,
                self.cache_read_input_tokens,
                format_usd(cost.total_cost_usd()),
                model_suffix,
                pricing_suffix,
            ),
            format!(
                "  cost breakdown: input={} output={} cache_write={} cache_read={}",
                format_usd(cost.input_cost_usd),
                format_usd(cost.output_cost_usd),
                format_usd(cost.cache_creation_cost_usd),
                format_usd(cost.cache_read_cost_usd),
            ),
        ]
    }
}

fn cost_for_tokens(tokens: u32, usd_per_million_tokens: f64) -> f64 {
    f64::from(tokens) / 1_000_000.0 * usd_per_million_tokens
}

#[must_use]
/// Formats a dollar-denominated value for CLI display.
pub fn format_usd(amount: f64) -> String {
    format!("${amount:.4}")
}

/// Aggregates token usage across a running session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageTracker {
    latest_turn: TokenUsage,
    cumulative: TokenUsage,
    turns: u32,
}

impl UsageTracker {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_session(session: &Session) -> Self {
        let mut tracker = Self::new();
        for message in &session.messages {
            if let Some(usage) = message.usage {
                tracker.record(usage);
            }
        }
        tracker
    }

    pub fn record(&mut self, usage: TokenUsage) {
        self.latest_turn = usage;
        self.cumulative.input_tokens += usage.input_tokens;
        self.cumulative.output_tokens += usage.output_tokens;
        self.cumulative.cache_creation_input_tokens += usage.cache_creation_input_tokens;
        self.cumulative.cache_read_input_tokens += usage.cache_read_input_tokens;
        self.turns += 1;
    }

    #[must_use]
    pub fn current_turn_usage(&self) -> TokenUsage {
        self.latest_turn
    }

    #[must_use]
    pub fn cumulative_usage(&self) -> TokenUsage {
        self.cumulative
    }

    #[must_use]
    pub fn turns(&self) -> u32 {
        self.turns
    }
}

#[cfg(test)]
mod tests {
    use super::{format_usd, pricing_for_model, TokenUsage, UsageTracker};
    use crate::session::{ContentBlock, ConversationMessage, MessageRole, Session};

    #[test]
    fn tracks_true_cumulative_usage() {
        let mut tracker = UsageTracker::new();
        tracker.record(TokenUsage {
            input_tokens: 10,
            output_tokens: 4,
            cache_creation_input_tokens: 2,
            cache_read_input_tokens: 1,
        });
        tracker.record(TokenUsage {
            input_tokens: 20,
            output_tokens: 6,
            cache_creation_input_tokens: 3,
            cache_read_input_tokens: 2,
        });

        assert_eq!(tracker.turns(), 2);
        assert_eq!(tracker.current_turn_usage().input_tokens, 20);
        assert_eq!(tracker.current_turn_usage().output_tokens, 6);
        assert_eq!(tracker.cumulative_usage().output_tokens, 10);
        assert_eq!(tracker.cumulative_usage().input_tokens, 30);
        assert_eq!(tracker.cumulative_usage().total_tokens(), 48);
    }

    #[test]
    fn computes_cost_summary_lines() {
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 500_000,
            cache_creation_input_tokens: 100_000,
            cache_read_input_tokens: 200_000,
        };

        let cost = usage.estimate_cost_usd();
        assert_eq!(format_usd(cost.input_cost_usd), "$3.0000");
        assert_eq!(format_usd(cost.output_cost_usd), "$7.5000");
        let lines = usage.summary_lines_for_model("usage", Some("claude-sonnet-4-20250514"));
        assert!(lines[0].contains("estimated_cost=$10.9350"));
        assert!(lines[0].contains("model=claude-sonnet-4-20250514"));
        assert!(lines[1].contains("cache_read=$0.0600"));
    }

    #[test]
    fn supports_model_specific_pricing() {
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 500_000,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        };
        let total_for = |model: &str| {
            let pricing = pricing_for_model(model).expect("known model pricing");
            format_usd(
                usage
                    .estimate_cost_usd_with_pricing(pricing)
                    .total_cost_usd(),
            )
        };

        assert_eq!(total_for("claude-fable-5-1"), "$35.0000");
        assert_eq!(total_for("claude-opus-4-6"), "$17.5000");
        assert_eq!(total_for("anthropic/claude-opus-5-5"), "$14.0000");
        assert_eq!(total_for("claude-opus-4-1-20250805"), "$52.5000");
        assert_eq!(total_for("claude-opus-4@20250514"), "$52.5000");
        assert_eq!(total_for("claude-sonnet-5-5"), "$7.0000");
        assert_eq!(total_for("claude-sonnet-4-6"), "$10.5000");
        assert_eq!(total_for("claude-haiku-5-5"), "$0.3500");
        assert_eq!(total_for("claude-haiku-4-5-20251001"), "$3.5000");
        assert_eq!(total_for("claude-3-5-haiku-20241022"), "$2.8000");
    }

    #[test]
    fn distinguishes_cache_read_rates_across_versions() {
        let usage = TokenUsage {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 1_000_000,
        };
        let cache_read_for = |model: &str| {
            let pricing = pricing_for_model(model).expect("known model pricing");
            format_usd(
                usage
                    .estimate_cost_usd_with_pricing(pricing)
                    .cache_read_cost_usd,
            )
        };

        assert_eq!(cache_read_for("claude-fable-5-1"), "$0.2500");
        assert_eq!(cache_read_for("claude-fable-5"), "$1.0000");
        assert_eq!(cache_read_for("claude-sonnet-5-5"), "$0.1000");
        assert_eq!(cache_read_for("claude-sonnet-5"), "$0.2000");
    }

    #[test]
    fn prices_deepseek_api_models_at_peak_rates() {
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 500_000,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 1_000_000,
        };
        let total_for = |model: &str| {
            let pricing = pricing_for_model(model).expect("known model pricing");
            format_usd(
                usage
                    .estimate_cost_usd_with_pricing(pricing)
                    .total_cost_usd(),
            )
        };

        assert_eq!(total_for("deepseek-v4-pro"), "$3.3440");
        assert_eq!(total_for("openai/deepseek-v4-pro"), "$3.3440");
        assert_eq!(total_for("deepseek-flash"), "$0.9060");
        assert_eq!(total_for("deepseek/deepseek-v4-flash"), "$0.9060");
        assert_eq!(total_for("deepseek-reasoner"), "$0.9060");
        // Third-party hosts set their own prices for other DeepSeek models.
        assert_eq!(pricing_for_model("deepseek-r1"), None);
    }

    #[test]
    fn marks_unknown_model_pricing_as_fallback() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 100,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        };
        let lines = usage.summary_lines_for_model("usage", Some("custom-model"));
        assert!(lines[0].contains("pricing=estimated-default"));
    }

    #[test]
    fn reconstructs_usage_from_session_messages() {
        let mut session = Session::new();
        session.messages = vec![ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![ContentBlock::Text {
                text: "done".to_string(),
            }],
            usage: Some(TokenUsage {
                input_tokens: 5,
                output_tokens: 2,
                cache_creation_input_tokens: 1,
                cache_read_input_tokens: 0,
            }),
        }];

        let tracker = UsageTracker::from_session(&session);
        assert_eq!(tracker.turns(), 1);
        assert_eq!(tracker.cumulative_usage().total_tokens(), 8);
    }
}
