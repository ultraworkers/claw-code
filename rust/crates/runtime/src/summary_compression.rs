use std::collections::BTreeSet;

use crate::compression_config::CompressionConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryCompressionBudget {
    pub max_chars: usize,
    pub max_lines: usize,
    pub max_line_chars: usize,
}

impl Default for SummaryCompressionBudget {
    fn default() -> Self {
        Self::from_config(&CompressionConfig::default())
    }
}

impl SummaryCompressionBudget {
    pub fn from_config(config: &CompressionConfig) -> Self {
        Self {
            max_chars: config.summary_max_chars,
            max_lines: config.summary_max_lines,
            max_line_chars: config.summary_max_line_chars,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryCompressionResult {
    pub summary: String,
    pub original_chars: usize,
    pub compressed_chars: usize,
    pub original_lines: usize,
    pub compressed_lines: usize,
    pub removed_duplicate_lines: usize,
    pub omitted_lines: usize,
    pub truncated: bool,
}

#[must_use]
pub fn compress_summary(
    summary: &str,
    budget: SummaryCompressionBudget,
) -> SummaryCompressionResult {
    let original_chars = summary.chars().count();
    let original_lines = summary.lines().count();

    let normalized = normalize_lines(summary, budget.max_line_chars);
    if normalized.lines.is_empty() || budget.max_chars == 0 || budget.max_lines == 0 {
        return SummaryCompressionResult {
            summary: String::new(),
            original_chars,
            compressed_chars: 0,
            original_lines,
            compressed_lines: 0,
            removed_duplicate_lines: normalized.removed_duplicate_lines,
            omitted_lines: normalized.lines.len(),
            truncated: original_chars > 0,
        };
    }

    let selected = select_line_indexes(&normalized.lines, budget);
    let mut compressed_lines = selected
        .iter()
        .map(|index| normalized.lines[*index].clone())
        .collect::<Vec<_>>();
    if compressed_lines.is_empty() {
        compressed_lines.push(truncate_line(&normalized.lines[0], budget.max_chars));
    }
    let omitted_lines = normalized
        .lines
        .len()
        .saturating_sub(compressed_lines.len());

    if omitted_lines > 0 {
        let omission_notice = omission_notice(omitted_lines);
        push_line_with_budget(&mut compressed_lines, omission_notice, budget);
    }

    let compressed_summary = compressed_lines.join("\n");

    SummaryCompressionResult {
        compressed_chars: compressed_summary.chars().count(),
        compressed_lines: compressed_lines.len(),
        removed_duplicate_lines: normalized.removed_duplicate_lines,
        omitted_lines,
        truncated: compressed_summary != summary.trim(),
        summary: compressed_summary,
        original_chars,
        original_lines,
    }
}

#[must_use]
pub fn compress_summary_text(summary: &str) -> String {
    compress_summary_with_budget(summary, &CompressionConfig::global())
}

pub fn compress_summary_with_budget(summary: &str, config: &CompressionConfig) -> String {
    compress_summary(summary, SummaryCompressionBudget::from_config(config)).summary
}

#[derive(Debug, Default)]
struct NormalizedSummary {
    lines: Vec<String>,
    removed_duplicate_lines: usize,
}

fn normalize_lines(summary: &str, max_line_chars: usize) -> NormalizedSummary {
    let mut seen = BTreeSet::new();
    let mut lines = Vec::new();
    let mut removed_duplicate_lines = 0;

    for raw_line in summary.lines() {
        let normalized = collapse_inline_whitespace(raw_line);
        if normalized.is_empty() {
            continue;
        }

        let truncated = truncate_line(&normalized, max_line_chars);
        let dedupe_key = dedupe_key(&truncated);
        if !seen.insert(dedupe_key) {
            removed_duplicate_lines += 1;
            continue;
        }

        lines.push(truncated);
    }

    NormalizedSummary {
        lines,
        removed_duplicate_lines,
    }
}

fn select_line_indexes(lines: &[String], budget: SummaryCompressionBudget) -> Vec<usize> {
    let mut selected = BTreeSet::<usize>::new();

    for priority in 0..=3 {
        for (index, line) in lines.iter().enumerate() {
            if selected.contains(&index) || line_priority(line) != priority {
                continue;
            }

            let candidate = selected
                .iter()
                .map(|selected_index| lines[*selected_index].as_str())
                .chain(std::iter::once(line.as_str()))
                .collect::<Vec<_>>();

            if candidate.len() > budget.max_lines {
                continue;
            }

            if joined_char_count(&candidate) > budget.max_chars {
                continue;
            }

            selected.insert(index);
        }
    }

    selected.into_iter().collect()
}

fn push_line_with_budget(lines: &mut Vec<String>, line: String, budget: SummaryCompressionBudget) {
    let candidate = lines
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(line.as_str()))
        .collect::<Vec<_>>();

    if candidate.len() <= budget.max_lines && joined_char_count(&candidate) <= budget.max_chars {
        lines.push(line);
    }
}

fn joined_char_count(lines: &[&str]) -> usize {
    lines.iter().map(|line| line.chars().count()).sum::<usize>() + lines.len().saturating_sub(1)
}

fn line_priority(line: &str) -> usize {
    if line == "Summary:" || line == "Conversation summary:" || is_core_detail(line) {
        0
    } else if is_section_header(line) {
        1
    } else if line.starts_with("- ") || line.starts_with("  - ") {
        2
    } else {
        3
    }
}

fn is_core_detail(line: &str) -> bool {
    [
        "- Scope:",
        "- Current work:",
        "- Pending work:",
        "- Key files referenced:",
        "- Tools mentioned:",
        "- Recent user requests:",
        "- Previously compacted context:",
        "- Newly compacted context:",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
}

fn is_section_header(line: &str) -> bool {
    line.ends_with(':')
}

fn omission_notice(omitted_lines: usize) -> String {
    format!("- … {omitted_lines} additional line(s) omitted.")
}

fn collapse_inline_whitespace(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_line(line: &str, max_chars: usize) -> String {
    if max_chars == 0 || line.chars().count() <= max_chars {
        return line.to_string();
    }

    if max_chars == 1 {
        return "…".to_string();
    }

    let mut truncated = line
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}

fn dedupe_key(line: &str) -> String {
    line.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{compress_summary, compress_summary_text, SummaryCompressionBudget};

    #[test]
    fn collapses_whitespace_and_duplicate_lines() {
        // given
        let summary = "Conversation summary:\n\n- Scope:   compact   earlier   messages.\n- Scope: compact earlier messages.\n- Current work: update runtime module.\n";

        // when
        let result = compress_summary(summary, SummaryCompressionBudget::default());

        // then
        assert_eq!(result.removed_duplicate_lines, 1);
        assert!(result
            .summary
            .contains("- Scope: compact earlier messages."));
        assert!(!result.summary.contains("  compact   earlier"));
    }

    #[test]
    fn keeps_core_lines_when_budget_is_tight() {
        // given
        let summary = [
            "Conversation summary:",
            "- Scope: 18 earlier messages compacted.",
            "- Current work: finish summary compression.",
            "- Key timeline:",
            "  - user: asked for a working implementation.",
            "  - assistant: inspected runtime compaction flow.",
            "  - tool: cargo check succeeded.",
        ]
        .join("\n");

        // when
        let result = compress_summary(
            &summary,
            SummaryCompressionBudget {
                max_chars: 120,
                max_lines: 3,
                max_line_chars: 80,
            },
        );

        // then
        assert!(result.summary.contains("Conversation summary:"));
        assert!(result
            .summary
            .contains("- Scope: 18 earlier messages compacted."));
        assert!(result
            .summary
            .contains("- Current work: finish summary compression."));
        assert!(result.omitted_lines > 0);
    }

    #[test]
    fn provides_a_default_text_only_helper() {
        // given
        let summary = "Summary:\n\nA short line.";

        // when
        let compressed = compress_summary_text(summary);

        // then
        assert_eq!(compressed, "Summary:\nA short line.");
    }
}
