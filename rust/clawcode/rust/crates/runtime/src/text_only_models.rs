/// Text-only model detection.
///
/// Reads `LLM_ONLY_MODEL.config` from two locations, first found wins:
/// 1. Project-level: `{cwd}/.claw/LLM_ONLY_MODEL.config`
/// 2. User-level:    `~/.claw/LLM_ONLY_MODEL.config` (`$CLAW_CONFIG_HOME`)
///
/// File format:
/// - One model name per line
/// - Lines starting with `#` are comments
/// - Empty lines are ignored
/// - Model names are compared case-insensitively
/// - `prefix:` format matches models starting with the prefix
///   (e.g. `gpt-:` matches `gpt-4`, `gpt-4o`, etc.)
/// - Plain model names match if the model ID contains the entry as a substring
///   (e.g. `claude-opus-4-6` matches exactly, `claude-opus` matches `claude-opus-4-6`)

use std::sync::RwLock;

use crate::config::default_config_home;

const FILE_NAME: &str = "LLM_ONLY_MODEL.config";

static TEXT_ONLY_MODELS: RwLock<Option<Vec<String>>> = RwLock::new(None);

fn user_file_path() -> std::path::PathBuf {
    default_config_home().join(FILE_NAME)
}

/// Check `.claw/LLM_ONLY_MODEL.config` in the current working directory.
fn project_file_path() -> Option<std::path::PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let candidate = cwd.join(".claw").join(FILE_NAME);
    if candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

fn parse_entries(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .map(|line| line.trim().to_lowercase())
        .collect()
}

fn load_one(path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .ok()
        .map_or(Vec::new(), |content| parse_entries(&content))
}

fn load_all() -> Vec<String> {
    // Project first, user fallback — first found wins.
    if let Some(project_path) = project_file_path() {
        return load_one(&project_path);
    }
    let user_path = user_file_path();
    if user_path.is_file() {
        return load_one(&user_path);
    }
    Vec::new()
}

fn load_or_reload(is_reload: bool) {
    if !is_reload {
        if let Ok(guard) = TEXT_ONLY_MODELS.read() {
            if guard.is_some() {
                return;
            }
        }
    }
    let entries = load_all();
    if let Ok(mut guard) = TEXT_ONLY_MODELS.write() {
        *guard = Some(entries);
    }
}

fn ensure_loaded() {
    load_or_reload(false);
}

/// Reload the text-only model list from disk.
/// Useful after modifying `LLM_ONLY_MODEL.config` at runtime.
pub fn reload() {
    load_or_reload(true);
}

/// Check if the given model name is a text-only model.
///
/// Matching rules:
/// - If the entry ends with `:`, it's a prefix match:
///   `gpt-` matches `gpt-4`, `gpt-4o`, etc.
/// - Otherwise, the model name must contain the entry as a substring:
///   `claude-opus` matches `claude-opus-4-6`
///   `claude-opus-4-6` matches exactly
#[must_use]
pub fn is_text_only_model(model_name: &str) -> bool {
    ensure_loaded();
    if let Ok(guard) = TEXT_ONLY_MODELS.read() {
        if let Some(ref entries) = *guard {
            return matches(entries, model_name);
        }
    }
    false
}

fn matches(entries: &[String], model_name: &str) -> bool {
    let model_lower = model_name.to_ascii_lowercase();
    for entry in entries {
        if entry.ends_with(':') {
            let prefix = &entry[..entry.len() - 1];
            if model_lower.starts_with(prefix) {
                return true;
            }
            continue;
        }

        // Exact match first — prevents "gpt-4" from false-matching "gpt-4o"
        // or "gpt-4-vision" when the user only wants the exact model.
        if model_lower == *entry {
            return true;
        }

        // Word-boundary substring match with self-delimiter awareness.
        //
        // If the entry itself starts or ends with non-alphanumeric
        // (e.g. `"gpt-"`), it is self-delimiting and no boundary check is
        // enforced on that side.  If the entry is purely alphanumeric
        // (e.g. `"gpt4"`), the adjacent character must not also be
        // alphanumeric — this prevents `"gpt4"` from matching inside
        // `"gpt4o"` or `"mygpt4model"`.
        //
        //   "gpt-4"   matches "gpt-4-turbo"  ✓
        //   "gpt-"    matches "gpt-4"         ✓  (self-delimiting)
        //   "claude-opus" matches "claude-opus-4-6"  ✓
        //   "gpt-4"   DOES NOT match "gpt-4o" ✗
        //   "llama"   DOES NOT match "llama3"  ✗
        let entry_bytes = entry.as_bytes();
        let first_alphanum = entry_bytes.first().is_some_and(|b| b.is_ascii_alphanumeric());
        let last_alphanum = entry_bytes.last().is_some_and(|b| b.is_ascii_alphanumeric());
        let mut search_start: usize = 0;
        while let Some(pos) = model_lower[search_start..].find(entry.as_str()) {
            let abs_pos = search_start + pos;
            let before_ok = !first_alphanum
                || abs_pos == 0
                || !model_lower.as_bytes()[abs_pos - 1].is_ascii_alphanumeric();
            let after_pos = abs_pos + entry.len();
            let after_ok = !last_alphanum
                || after_pos >= model_lower.len()
                || !model_lower.as_bytes()[after_pos].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return true;
            }
            search_start = abs_pos + 1;
        }
    }
    false
}

#[doc(hidden)]
pub fn set_test_entries(entries: Vec<String>) {
    if let Ok(mut guard) = TEXT_ONLY_MODELS.write() {
        *guard = Some(entries);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_entries_skips_comments_and_empty() {
        let content = "# comment\nclaude-opus\n\n# another\nclaude-sonnet\n";
        let entries = parse_entries(content);
        assert_eq!(entries, vec!["claude-opus", "claude-sonnet"]);
    }

    #[test]
    fn test_matches_exact() {
        let entries = parse_entries("claude-opus-4-6");
        assert!(matches(&entries, "claude-opus-4-6"));
        assert!(!matches(&entries, "claude-sonnet-4-6"));
    }

    #[test]
    fn test_matches_partial() {
        let entries = parse_entries("claude-opus");
        assert!(matches(&entries, "claude-opus-4-6"));
        assert!(matches(&entries, "claude-opus-4-5"));
        assert!(!matches(&entries, "claude-sonnet-4-6"));
    }

    #[test]
    fn test_matches_prefix() {
        let entries = parse_entries("gpt-:");
        assert!(matches(&entries, "gpt-4"));
        assert!(matches(&entries, "gpt-4o"));
        assert!(matches(&entries, "gpt-4-turbo"));
        assert!(!matches(&entries, "claude-sonnet-4-6"));
    }

    #[test]
    fn test_matches_case_insensitive() {
        let entries = parse_entries("Claude-Opus");
        assert!(matches(&entries, "claude-opus-4-6"));
        assert!(matches(&entries, "CLAUDE-OPUS-4-6"));
    }

    #[test]
    fn test_matches_multiple_entries() {
        let entries = parse_entries("gpt-:\nllama");
        assert!(matches(&entries, "gpt-4"));
        assert!(matches(&entries, "llama-3.1-8b"));
        assert!(!matches(&entries, "claude-sonnet-4-6"));
    }

    #[test]
    fn test_no_match() {
        let entries = parse_entries("unknown-model");
        assert!(!matches(&entries, "claude-sonnet-4-6"));
        assert!(!matches(&entries, ""));
    }

    #[test]
    fn test_prefix_no_colon_is_plain_match() {
        let entries = parse_entries("gpt-");
        assert!(matches(&entries, "gpt-4"));
        assert!(matches(&entries, "my-gpt-4"));
    }

    #[test]
    fn test_load_one_from_nonexistent_path() {
        let entries = load_one(std::path::Path::new("/nonexistent/path.txt"));
        assert!(entries.is_empty());
    }
}
