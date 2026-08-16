use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_AGENT_MODEL: &str = "claude-opus-4-6";
pub const DEFAULT_AGENT_SYSTEM_DATE: &str = "2026-03-31";
pub const DEFAULT_AGENT_MAX_ITERATIONS: usize = 32;
pub const DEFAULT_AGENT_TIMEOUT_SECS: u64 = 300;

static AGENT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn make_agent_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|error| {
            eprintln!("[agent] system clock is before epoch ({error}); using 0 for agent ID");
            std::time::Duration::ZERO
        })
        .as_nanos();
    let n = AGENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("agent-{nanos:x}-{n:x}")
}

pub fn slugify_agent_name(description: &str) -> String {
    let mut out: String = description
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').chars().take(32).collect()
}

/// Extract a commit SHA reference from a free-form result string.
pub fn extract_commit_sha(result: &str) -> Option<String> {
    for token in result.split(|c: char| !c.is_ascii_hexdigit()) {
        if token.len() == 40 {
            return Some(token.to_string());
        }
    }
    let lower = result.to_ascii_lowercase();
    for marker in ["commit ", "sha ", "sha:", "@"] {
        if let Some(idx) = lower.find(marker) {
            let after = &result[idx + marker.len()..];
            let token: String = after.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
            if (7..=12).contains(&token.len()) {
                return Some(token);
            }
        }
    }
    None
}
