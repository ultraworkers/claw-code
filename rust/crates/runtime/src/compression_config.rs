use std::sync::OnceLock;

const CLAW_TOOLRESULT_MIN_BYTES: &str = "CLAW_TOOLRESULT_MIN_BYTES";
const CLAW_CONTEXT_PRESERVE_MSGS: &str = "CLAW_CONTEXT_PRESERVE_MSGS";
const CLAW_WEBSEARCH_TTL_SECS: &str = "CLAW_WEBSEARCH_TTL_SECS";
const CLAW_WEBFETCH_TTL_SECS: &str = "CLAW_WEBFETCH_TTL_SECS";
const CLAW_COMPACT_PRESERVE_MSGS: &str = "CLAW_COMPACT_PRESERVE_MSGS";
const CLAW_COMPACT_PRESERVE_TOKENS: &str = "CLAW_COMPACT_PRESERVE_TOKENS";
const CLAW_COMPACT_MAX_TOKENS: &str = "CLAW_COMPACT_MAX_TOKENS";
const CLAW_COMPACT_PRESERVE_TURNS: &str = "CLAW_COMPACT_PRESERVE_TURNS";
const CLAW_SUMMARY_MAX_CHARS: &str = "CLAW_SUMMARY_MAX_CHARS";
const CLAW_SUMMARY_MAX_LINES: &str = "CLAW_SUMMARY_MAX_LINES";
const CLAW_SUMMARY_MAX_LINE_CHARS: &str = "CLAW_SUMMARY_MAX_LINE_CHARS";
const CLAW_COMPACT_ANTITHRASH_RATIO: &str = "CLAW_COMPACT_ANTITHRASH_RATIO";

#[derive(Debug, Clone, PartialEq)]
pub struct CompressionConfig {
    pub toolresult_min_bytes: usize,
    pub preserve_recent_messages: usize,
    pub websearch_ttl_secs: u64,
    pub webfetch_ttl_secs: u64,
    pub compact_preserve_recent_messages: usize,
    pub compact_preserve_recent_tokens: usize,
    pub compact_max_estimated_tokens: usize,
    pub compact_preserve_last_n_turns: usize,
    pub summary_max_chars: usize,
    pub summary_max_lines: usize,
    pub summary_max_line_chars: usize,
    pub antithrash_ratio: f64,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            toolresult_min_bytes: 500,
            preserve_recent_messages: 6,
            websearch_ttl_secs: 15,
            webfetch_ttl_secs: 30,
            compact_preserve_recent_messages: 4,
            compact_preserve_recent_tokens: 2000,
            compact_max_estimated_tokens: 10_000,
            compact_preserve_last_n_turns: 0,
            summary_max_chars: 1_200,
            summary_max_lines: 24,
            summary_max_line_chars: 160,
            antithrash_ratio: 0.10,
        }
    }
}

impl CompressionConfig {
    pub fn from_env() -> Self {
        Self {
            toolresult_min_bytes: read_env(CLAW_TOOLRESULT_MIN_BYTES).unwrap_or(500),
            preserve_recent_messages: read_env(CLAW_CONTEXT_PRESERVE_MSGS).unwrap_or(6),
            websearch_ttl_secs: read_env(CLAW_WEBSEARCH_TTL_SECS).unwrap_or(15),
            webfetch_ttl_secs: read_env(CLAW_WEBFETCH_TTL_SECS).unwrap_or(30),
            compact_preserve_recent_messages: read_env(CLAW_COMPACT_PRESERVE_MSGS).unwrap_or(4),
            compact_preserve_recent_tokens: read_env(CLAW_COMPACT_PRESERVE_TOKENS).unwrap_or(2000),
            compact_max_estimated_tokens: read_env(CLAW_COMPACT_MAX_TOKENS).unwrap_or(10_000),
            compact_preserve_last_n_turns: read_env(CLAW_COMPACT_PRESERVE_TURNS).unwrap_or(0),
            summary_max_chars: read_env(CLAW_SUMMARY_MAX_CHARS).unwrap_or(1_200),
            summary_max_lines: read_env(CLAW_SUMMARY_MAX_LINES).unwrap_or(24),
            summary_max_line_chars: read_env(CLAW_SUMMARY_MAX_LINE_CHARS).unwrap_or(160),
            antithrash_ratio: read_env_f64(CLAW_COMPACT_ANTITHRASH_RATIO)
                .map(|r| r.clamp(0.0, 1.0))
                .unwrap_or(0.10),
        }
    }

    pub fn global() -> &'static Self {
        static GLOBAL: OnceLock<CompressionConfig> = OnceLock::new();
        GLOBAL.get_or_init(Self::from_env)
    }
}

fn read_env<T>(key: &str) -> Option<T>
where
    T: std::str::FromStr,
{
    std::env::var(key).ok()?.trim().parse().ok()
}

fn read_env_f64(key: &str) -> Option<f64> {
    let raw = std::env::var(key).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values_are_sane() {
        let config = CompressionConfig::default();
        assert_eq!(config.toolresult_min_bytes, 500);
        assert_eq!(config.preserve_recent_messages, 6);
        assert_eq!(config.websearch_ttl_secs, 15);
        assert_eq!(config.webfetch_ttl_secs, 30);
        assert_eq!(config.compact_preserve_recent_messages, 4);
        assert_eq!(config.compact_preserve_recent_tokens, 2000);
        assert_eq!(config.compact_max_estimated_tokens, 10_000);
        assert_eq!(config.compact_preserve_last_n_turns, 0);
        assert_eq!(config.summary_max_chars, 1_200);
        assert_eq!(config.summary_max_lines, 24);
        assert_eq!(config.summary_max_line_chars, 160);
        assert!((config.antithrash_ratio - 0.10).abs() < f64::EPSILON);
    }

    #[test]
    fn from_env_reads_and_clamps_vars() {
        let _lock = crate::test_env_lock();
        std::env::set_var("CLAW_TOOLRESULT_MIN_BYTES", "999");
        std::env::set_var("CLAW_CONTEXT_PRESERVE_MSGS", "3");
        std::env::set_var("CLAW_WEBSEARCH_TTL_SECS", "10");
        std::env::set_var("CLAW_WEBFETCH_TTL_SECS", "45");
        std::env::set_var("CLAW_COMPACT_PRESERVE_MSGS", "2");
        std::env::set_var("CLAW_COMPACT_PRESERVE_TOKENS", "1000");
        std::env::set_var("CLAW_COMPACT_MAX_TOKENS", "5000");
        std::env::set_var("CLAW_COMPACT_PRESERVE_TURNS", "1");
        std::env::set_var("CLAW_SUMMARY_MAX_CHARS", "800");
        std::env::set_var("CLAW_SUMMARY_MAX_LINES", "10");
        std::env::set_var("CLAW_SUMMARY_MAX_LINE_CHARS", "100");
        std::env::set_var("CLAW_COMPACT_ANTITHRASH_RATIO", "0.05");
        let config = CompressionConfig::from_env();
        assert_eq!(config.toolresult_min_bytes, 999);
        assert_eq!(config.preserve_recent_messages, 3);
        assert_eq!(config.websearch_ttl_secs, 10);
        assert_eq!(config.webfetch_ttl_secs, 45);
        assert_eq!(config.compact_preserve_recent_messages, 2);
        assert_eq!(config.compact_preserve_recent_tokens, 1000);
        assert_eq!(config.compact_max_estimated_tokens, 5000);
        assert_eq!(config.compact_preserve_last_n_turns, 1);
        assert_eq!(config.summary_max_chars, 800);
        assert_eq!(config.summary_max_lines, 10);
        assert_eq!(config.summary_max_line_chars, 100);
        assert!((config.antithrash_ratio - 0.05).abs() < f64::EPSILON);
        std::env::remove_var("CLAW_TOOLRESULT_MIN_BYTES");
        std::env::remove_var("CLAW_CONTEXT_PRESERVE_MSGS");
        std::env::remove_var("CLAW_WEBSEARCH_TTL_SECS");
        std::env::remove_var("CLAW_WEBFETCH_TTL_SECS");
        std::env::remove_var("CLAW_COMPACT_PRESERVE_MSGS");
        std::env::remove_var("CLAW_COMPACT_PRESERVE_TOKENS");
        std::env::remove_var("CLAW_COMPACT_MAX_TOKENS");
        std::env::remove_var("CLAW_COMPACT_PRESERVE_TURNS");
        std::env::remove_var("CLAW_SUMMARY_MAX_CHARS");
        std::env::remove_var("CLAW_SUMMARY_MAX_LINES");
        std::env::remove_var("CLAW_SUMMARY_MAX_LINE_CHARS");
        std::env::remove_var("CLAW_COMPACT_ANTITHRASH_RATIO");

        // clamp negative
        std::env::set_var("CLAW_COMPACT_ANTITHRASH_RATIO", "-0.5");
        let config = CompressionConfig::from_env();
        assert!(
            (config.antithrash_ratio - 0.0).abs() < f64::EPSILON,
            "negative value should clamp to 0.0, got {}",
            config.antithrash_ratio
        );
        std::env::remove_var("CLAW_COMPACT_ANTITHRASH_RATIO");

        // clamp >1.0
        std::env::set_var("CLAW_COMPACT_ANTITHRASH_RATIO", "1.5");
        let config = CompressionConfig::from_env();
        assert!(
            (config.antithrash_ratio - 1.0).abs() < f64::EPSILON,
            "value >1.0 should clamp to 1.0, got {}",
            config.antithrash_ratio
        );
        std::env::remove_var("CLAW_COMPACT_ANTITHRASH_RATIO");
    }
}
