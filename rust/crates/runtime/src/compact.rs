use std::sync::OnceLock;
use std::time::Instant;
use tiktoken_rs::CoreBPE;

use crate::compression_config::CompressionConfig;
use crate::session::{ContentBlock, ConversationMessage, MessageRole, Session};
use crate::summary_compression::{compress_summary, compress_summary_text, SummaryCompressionBudget};

/// Lazily initialized cl100k_base encoder. Returns `None` if tiktoken
/// initialization fails — the system degrades gracefully to a byte-count
/// heuristic instead of panicking at startup.
fn get_cl100k_encoder() -> &'static OnceLock<Option<CoreBPE>> {
    static ENCODER: OnceLock<Option<CoreBPE>> = OnceLock::new();
    ENCODER.get_or_init(|| {
        match tiktoken_rs::cl100k_base() {
            Ok(bpe) => Some(bpe),
            Err(e) => {
                eprintln!("[compact] tiktoken init failed, using byte-count fallback: {e}");
                None
            }
        }
    });
    &ENCODER
}

const COMPACT_CONTINUATION_PREAMBLE: &str =
    "This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.\n\n";
const COMPACT_RECENT_MESSAGES_NOTE: &str =
    "The most recent messages of the conversation are preserved below.";
const COMPACT_DIRECT_RESUME_INSTRUCTION: &str = "Continue the conversation from where it left off without asking the user any further questions. Resume directly — do not acknowledge the summary, do not recap what was happening, and do not preface with continuation text.";

/// Thresholds controlling when and how a session is compacted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionConfig {
    /// Number of recent messages to preserve as a **minimum** guarantee.
    /// When zero, uses only the token-based budget (`preserve_recent_tokens`).
    /// Kept for backward compatibility — existing callers that pass
    /// `preserve_recent_messages: N` will still preserve at least N messages.
    pub preserve_recent_messages: usize,
    /// Token budget for the preserved tail. The function
    /// `find_token_tail_start` walks backwards from the end of the session
    /// until the accumulated token estimate reaches this budget.
    /// Default: 2000 tokens (~1500 English words).
    pub preserve_recent_tokens: usize,
    /// Hard cap on total estimated tokens before compaction triggers.
    pub max_estimated_tokens: usize,
    /// Number of complete user→assistant turn pairs to preserve from the end.
    /// A turn = a User message immediately followed by an Assistant message.
    /// When 0 (default), turn-based preservation is disabled and only the
    /// token-budget and message-minimum dimensions apply.
    pub preserve_last_n_turns: usize,
    /// Summary compression budget. When `None`, falls back to
    /// `CompressionConfig::global()` summary settings.
    pub summary_budget: Option<SummaryCompressionBudget>,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            preserve_recent_messages: 4,
            preserve_recent_tokens: 2000,
            max_estimated_tokens: 10_000,
            preserve_last_n_turns: 0,
            summary_budget: None,
        }
    }
}

impl CompactionConfig {
    pub fn from_config(config: &CompressionConfig) -> Self {
        Self {
            preserve_recent_messages: config.compact_preserve_recent_messages,
            preserve_recent_tokens: config.compact_preserve_recent_tokens,
            max_estimated_tokens: config.compact_max_estimated_tokens,
            preserve_last_n_turns: config.compact_preserve_last_n_turns,
            summary_budget: Some(SummaryCompressionBudget::from_config(config)),
        }
    }
}

/// Result of compacting a session into a summary plus preserved tail messages.
#[derive(Debug, Clone, PartialEq)]
pub struct CompactionResult {
    pub summary: String,
    pub formatted_summary: String,
    pub compacted_session: Session,
    pub removed_message_count: usize,
}

/// Roughly estimates the token footprint of the current session transcript.
#[must_use]
pub fn estimate_session_tokens(session: &Session) -> usize {
    session.messages.iter().map(estimate_message_tokens).sum()
}

/// Walk backwards from the end of `messages` to find the earliest index that
/// fits within the token budget. Returns a **lower bound** — the caller may
/// push the boundary further back due to tool-pair walkback.
fn find_token_tail_start(messages: &[ConversationMessage], token_budget: usize) -> usize {
    if token_budget == 0 {
        return 0;
    }

    let mut accumulated = 0usize;
    for (i, msg) in messages.iter().enumerate().rev() {
        accumulated = accumulated.saturating_add(estimate_message_tokens(msg));
        if accumulated > token_budget {
            return i + 1;
        }
    }
    0
}

/// Walks messages backward counting turns by User boundaries and returns
/// the first index to KEEP (within the slice).
///
/// A "turn" is everything from one User message up to (but not including)
/// the next User message. This handles both simple Q&A (User→Assistant)
/// and tool-using sessions (User→Assistant→Tool→...→Assistant) correctly —
/// the User message is the turn boundary, not adjacency.
///
/// Returns `messages.len()` as a no-op sentinel when `preserve_turns` is 0
/// or insufficient turns exist.
fn find_turn_tail_start(messages: &[ConversationMessage], preserve_turns: usize) -> usize {
    if preserve_turns == 0 || messages.is_empty() {
        return messages.len();
    }

    let mut turn_count = 0usize;
    let mut i = messages.len();

    while i > 0 && turn_count < preserve_turns {
        i -= 1;
        if messages[i].role == MessageRole::User {
            turn_count += 1;
        }
    }

    if turn_count >= preserve_turns {
        i
    } else {
        messages.len()
    }
}

/// Returns `true` when the session exceeds the configured compaction budget.
#[must_use]
pub fn should_compact(session: &Session, config: CompactionConfig) -> bool {
    let start = compacted_summary_prefix_len(session);
    let compactable = &session.messages[start..];

    // Minimum message guarantee for backward compatibility.
    let below_message_min = if config.preserve_recent_messages > 0 {
        compactable.len() <= config.preserve_recent_messages
    } else {
        false
    };

    if below_message_min {
        return false;
    }

    let total_tokens: usize = compactable.iter().map(estimate_message_tokens).sum();
    total_tokens >= config.max_estimated_tokens
}

/// Normalizes a compaction summary into user-facing continuation text.
#[must_use]
pub fn format_compact_summary(summary: &str) -> String {
    let without_analysis = strip_tag_block(summary, "analysis");
    let formatted = if let Some(content) = extract_tag_block(&without_analysis, "summary") {
        without_analysis.replace(
            &format!("<summary>{content}</summary>"),
            &format!("Summary:\n{}", content.trim()),
        )
    } else {
        without_analysis
    };

    collapse_blank_lines(&formatted).trim().to_string()
}

/// Builds the synthetic system message used after session compaction.
#[must_use]
pub fn get_compact_continuation_message(
    summary: &str,
    suppress_follow_up_questions: bool,
    recent_messages_preserved: bool,
) -> String {
    let mut base = format!(
        "{COMPACT_CONTINUATION_PREAMBLE}{}",
        format_compact_summary(summary)
    );

    if recent_messages_preserved {
        base.push_str("\n\n");
        base.push_str(COMPACT_RECENT_MESSAGES_NOTE);
    }

    if suppress_follow_up_questions {
        base.push('\n');
        base.push_str(COMPACT_DIRECT_RESUME_INSTRUCTION);
    }

    base
}

/// Compacts a session by summarizing older messages and preserving the recent tail.
#[must_use]
pub fn compact_session(session: &Session, config: CompactionConfig) -> CompactionResult {
    if !should_compact(session, config) {
        return CompactionResult {
            summary: String::new(),
            formatted_summary: String::new(),
            compacted_session: session.clone(),
            removed_message_count: 0,
        };
    }

    let existing_summary = session
        .messages
        .first()
        .and_then(extract_existing_compacted_summary);
    let compacted_prefix_len = usize::from(existing_summary.is_some());
    // Three-tailed approach: find boundary from token budget, turn count,
    // and message minimum, then take the most conservative (earliest) index.
    let post_prefix = &session.messages[compacted_prefix_len..];
    let from_token_budget = find_token_tail_start(post_prefix, config.preserve_recent_tokens);
    let from_token_absolute = compacted_prefix_len + from_token_budget;
    let from_message_min = session
        .messages
        .len()
        .saturating_sub(config.preserve_recent_messages);
    let from_turn_budget = find_turn_tail_start(post_prefix, config.preserve_last_n_turns);
    let from_turn_absolute = compacted_prefix_len + from_turn_budget;
    // When max_estimated_tokens is 0 the caller requests unconditional
    // compaction (used by auto-compaction after crossing the threshold).
    // In this mode the token budget must not override the message minimum,
    // otherwise sessions with a generous preserve_recent_tokens (2000)
    // that easily fits all messages would skip compaction entirely.
    // The turn-preservation dimension is still honored: find_turn_tail_start
    // returns the len() sentinel when turns are disabled, so min() degrades
    // to from_message_min in that case.
    let raw_keep_from = if config.max_estimated_tokens == 0 {
        std::cmp::min(from_message_min, from_turn_absolute)
    } else {
        std::cmp::min(
            std::cmp::min(from_token_absolute, from_turn_absolute),
            from_message_min,
        )
    };
    // Ensure we do not split a tool-use / tool-result pair at the compaction
    // boundary. If the first preserved message is a user message whose first
    // block is a ToolResult, the assistant message with the matching ToolUse
    // was slated for removal — that produces an orphaned tool role message on
    // the OpenAI-compat path (400: tool message must follow assistant with
    // tool_calls). Walk the boundary back until we start at a safe point.
    let keep_from = {
        let mut k = raw_keep_from;
        // If the first preserved message is a tool-result turn, ensure its
        // paired assistant tool-use turn is preserved too. Without this fix,
        // the OpenAI-compat adapter sends an orphaned 'tool' role message
        // with no preceding assistant 'tool_calls', which providers reject
        // with a 400. We walk back only if the immediately preceding message
        // is NOT an assistant message that contains a ToolUse block (i.e. the
        // pair is actually broken at the boundary).
        loop {
            if k == 0 || k <= compacted_prefix_len {
                break;
            }
            let first_preserved = &session.messages[k];
            let starts_with_tool_result = first_preserved
                .blocks
                .first()
                .is_some_and(|b| matches!(b, ContentBlock::ToolResult { .. }));
            if !starts_with_tool_result {
                break;
            }
            // Check the message just before the current boundary.
            let preceding = &session.messages[k - 1];
            let preceding_has_tool_use = preceding
                .blocks
                .iter()
                .any(|b| matches!(b, ContentBlock::ToolUse { .. }));
            if preceding_has_tool_use {
                // Pair is intact — walk back one more to include the assistant turn.
                k = k.saturating_sub(1);
                break;
            }
            // Preceding message has no ToolUse but we have a ToolResult —
            // this is already an orphaned pair; walk back to try to fix it.
            k = k.saturating_sub(1);
        }
        k
    };
    // Safety: keep_from must never be less than compacted_prefix_len or the
    // slice access on the next line would panic. The should_compact guard
    // should prevent this, but clamp defensively.
    let keep_from = keep_from.max(compacted_prefix_len);

    // Wire-role alternation fix: the continuation is emitted as a System
    // message, which convert.rs maps to the "user" wire role. If the preserved
    // tail would then start with ANOTHER user-role message (User or Tool), the
    // Anthropic API rejects the request with "roles must alternate" (or
    // "Cannot have 2 or more assistant messages"). Walk the boundary FORWARD to
    // the next Assistant message so the wire sequence begins [user(cont),
    // assistant, ...]. The user/tool messages we skip are folded into the
    // summary rather than dropped — they still contribute to `removed`.
    let keep_from = {
        let mut k = keep_from;
        while k < session.messages.len() {
            let first_preserved = &session.messages[k];
            let is_user_wire_role = matches!(
                first_preserved.role,
                MessageRole::User | MessageRole::Tool
            );
            if !is_user_wire_role {
                break;
            }
            k += 1;
        }
        k
    };

    // If all three dimensions agree to keep everything, there is nothing
    // to compact — return early to avoid wasted I/O and summary churn.
    if keep_from == compacted_prefix_len {
        return CompactionResult {
            summary: existing_summary.clone().unwrap_or_default(),
            formatted_summary: existing_summary
                .as_deref()
                .map(format_compact_summary)
                .unwrap_or_default(),
            compacted_session: session.clone(),
            removed_message_count: 0,
        };
    }

    let removed = &session.messages[compacted_prefix_len..keep_from];
    let preserved = session.messages[keep_from..].to_vec();
    let raw_summary =
        merge_compact_summaries(existing_summary.as_deref(), &summarize_messages(removed));
    let summary = match config.summary_budget {
        Some(budget) => compress_summary(&raw_summary, budget).summary,
        None => compress_summary_text(&raw_summary),
    };
    // Guard against silent context loss: if compression produced an empty
    // summary but messages would still be discarded, keep the removed messages
    // instead. Dropping context without any trace is never better than keeping
    // it (F-7). Reachable with degenerate configs such as
    // CLAW_SUMMARY_MAX_CHARS=0 or CLAW_SUMMARY_MAX_LINES=0.
    if summary.trim().is_empty() {
        return CompactionResult {
            summary: existing_summary.clone().unwrap_or_default(),
            formatted_summary: existing_summary
                .as_deref()
                .map(format_compact_summary)
                .unwrap_or_default(),
            compacted_session: session.clone(),
            removed_message_count: 0,
        };
    }
    let formatted_summary = format_compact_summary(&summary);
    let continuation = get_compact_continuation_message(&summary, true, !preserved.is_empty());

    let mut compacted_messages = vec![ConversationMessage {
        role: MessageRole::System,
        blocks: vec![ContentBlock::Text { text: continuation }],
        usage: None,
        created_at: Instant::now(),
        cached_tokens: OnceLock::new(),
        cached_input_message: OnceLock::new(),
    }];
    compacted_messages.extend(preserved);

    let mut compacted_session = session.clone();
    compacted_session.messages = compacted_messages;
    compacted_session.record_compaction(summary.clone(), removed.len());

    CompactionResult {
        summary,
        formatted_summary,
        compacted_session,
        removed_message_count: removed.len(),
    }
}

fn compacted_summary_prefix_len(session: &Session) -> usize {
    usize::from(
        session
            .messages
            .first()
            .and_then(extract_existing_compacted_summary)
            .is_some(),
    )
}

fn summarize_messages(messages: &[ConversationMessage]) -> String {
    let user_messages = messages
        .iter()
        .filter(|message| message.role == MessageRole::User)
        .count();
    let assistant_messages = messages
        .iter()
        .filter(|message| message.role == MessageRole::Assistant)
        .count();
    let tool_messages = messages
        .iter()
        .filter(|message| message.role == MessageRole::Tool)
        .count();

    let mut tool_names = messages
        .iter()
        .flat_map(|message| message.blocks.iter())
        .filter_map(|block| match block {
            ContentBlock::ToolUse { name, .. } => Some(name.as_str()),
            ContentBlock::ToolResult { tool_name, .. } => Some(tool_name.as_str()),
            ContentBlock::Text { .. } | ContentBlock::Thinking { .. } | ContentBlock::RedactedThinking { .. } => None,
            ContentBlock::Image { .. } | ContentBlock::ImageRef { .. } => None,
        })
        .collect::<Vec<_>>();
    tool_names.sort_unstable();
    tool_names.dedup();

    let mut lines = vec![
        "<summary>".to_string(),
        "Conversation summary:".to_string(),
        format!(
            "- Scope: {} earlier messages compacted (user={}, assistant={}, tool={}).",
            messages.len(),
            user_messages,
            assistant_messages,
            tool_messages
        ),
    ];

    if !tool_names.is_empty() {
        lines.push(format!("- Tools mentioned: {}.", tool_names.join(", ")));
    }

    let recent_user_requests = collect_recent_role_summaries(messages, MessageRole::User, 3);
    if !recent_user_requests.is_empty() {
        lines.push("- Recent user requests:".to_string());
        lines.extend(
            recent_user_requests
                .into_iter()
                .map(|request| format!("  - {request}")),
        );
    }

    let user_verbatim = collect_user_input_verbatim(messages, 2000);
    if !user_verbatim.is_empty() {
        lines.push("- User input verbatim (exact commands, file paths, flags, error codes, function names, and URLs that appear in the user's own messages — the user's own direct inputs, not tool outputs):".to_string());
        lines.extend(
            user_verbatim
                .into_iter()
                .map(|text| format!("  - {text}")),
        );
    }

    let pending_work = infer_pending_work(messages);
    if !pending_work.is_empty() {
        lines.push("- Pending work:".to_string());
        lines.extend(pending_work.into_iter().map(|item| format!("  - {item}")));
    }

    let key_files = collect_key_files(messages);
    if !key_files.is_empty() {
        lines.push(format!("- Key files referenced: {}.", key_files.join(", ")));
    }

    if let Some(current_work) = infer_current_work(messages) {
        lines.push(format!("- Current work: {current_work}"));
    }

    lines.push("- Key timeline:".to_string());
    for message in messages {
        let role = match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        let content = message
            .blocks
            .iter()
            .map(summarize_block)
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push(format!("  - {role}: {content}"));
    }
    lines.push("</summary>".to_string());
    lines.join("\n")
}

fn merge_compact_summaries(existing_summary: Option<&str>, new_summary: &str) -> String {
    let Some(existing_summary) = existing_summary else {
        return new_summary.to_string();
    };

    let previous_highlights = extract_summary_highlights(&format_compact_summary(existing_summary));
    let new_formatted_summary = format_compact_summary(new_summary);
    let new_highlights = extract_summary_highlights(&new_formatted_summary);
    let new_timeline = extract_summary_timeline(&new_formatted_summary);

    let mut lines = vec!["<summary>".to_string(), "Conversation summary:".to_string()];

    if !previous_highlights.is_empty() {
        lines.push("- Previously compacted context:".to_string());
        lines.extend(
            previous_highlights
                .into_iter()
                .map(|line| format!("  {line}")),
        );
    }

    if !new_highlights.is_empty() {
        lines.push("- Newly compacted context:".to_string());
        lines.extend(new_highlights.into_iter().map(|line| format!("  {line}")));
    }

    if !new_timeline.is_empty() {
        lines.push("- Key timeline:".to_string());
        lines.extend(new_timeline.into_iter().map(|line| format!("  {line}")));
    }

    lines.push("</summary>".to_string());
    lines.join("\n")
}

fn summarize_block(block: &ContentBlock) -> String {
    let raw = match block {
        &ContentBlock::Image { .. } => "[image]".to_string(),
        &ContentBlock::ImageRef { .. } => "[image]".to_string(),
        ContentBlock::Text { text } => text.clone(),
        ContentBlock::ToolUse { name, input, .. } => {
            format!("tool_use {name}({input})")
        }
        ContentBlock::ToolResult {
            tool_name,
            output,
            is_error,
            ..
        } => format!(
            "tool_result {tool_name}: {}{output}",
            if *is_error { "error " } else { "" }
        ),
        ContentBlock::Thinking { thinking, .. } => {
            let truncated: String = thinking.chars().take(200).collect();
            format!("thinking: {truncated}")
        }
        ContentBlock::RedactedThinking { .. } => {
            "thinking: [redacted by provider]".to_string()
        }
    };
    truncate_summary(&raw, 160)
}

fn collect_recent_role_summaries(
    messages: &[ConversationMessage],
    role: MessageRole,
    limit: usize,
) -> Vec<String> {
    messages
        .iter()
        .filter(|message| message.role == role)
        .rev()
        .filter_map(|message| first_text_block(message))
        .take(limit)
        .map(|text| truncate_summary(text, 160))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn infer_pending_work(messages: &[ConversationMessage]) -> Vec<String> {
    messages
        .iter()
        .rev()
        .filter_map(first_text_block)
        .filter(|text| {
            let lowered = text.to_ascii_lowercase();
            lowered.contains("todo")
                || lowered.contains("next")
                || lowered.contains("pending")
                || lowered.contains("follow up")
                || lowered.contains("remaining")
        })
        .take(3)
        .map(|text| truncate_summary(text, 160))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn collect_key_files(messages: &[ConversationMessage]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    messages
        .iter()
        .flat_map(|message| message.blocks.iter())
        .flat_map(|block| match block {
            ContentBlock::Text { text } => extract_file_candidates(text),
            ContentBlock::ToolUse { input, .. } => {
                let input_str = input.to_string();
                extract_file_candidates(&input_str)
            }
            ContentBlock::ToolResult { output, .. } => extract_file_candidates(output),
            ContentBlock::Image { .. }
            | ContentBlock::ImageRef { .. }
            | ContentBlock::Thinking { .. }
            | ContentBlock::RedactedThinking { .. } => vec![],
        })
        .filter(|f| seen.insert(f.clone()))
        .take(8)
        .collect()
}

/// Collects verbatim text from user messages in the compacted segment.
/// Each user message's text content is included up to `max_chars` per message
/// (default 2000, enough to capture commands, file paths, and error messages).
fn collect_user_input_verbatim(messages: &[ConversationMessage], max_chars: usize) -> Vec<String> {
    messages
        .iter()
        .filter(|message| message.role == MessageRole::User)
        .flat_map(|message| all_text_blocks(message))
        .map(|text| truncate_summary(text, max_chars))
        .collect()
}

fn all_text_blocks(message: &ConversationMessage) -> Vec<&str> {
    message
        .blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } if !text.trim().is_empty() => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn infer_current_work(messages: &[ConversationMessage]) -> Option<String> {
    messages
        .iter()
        .rev()
        .filter_map(first_text_block)
        .find(|text| !text.trim().is_empty())
        .map(|text| truncate_summary(text, 200))
}

fn first_text_block(message: &ConversationMessage) -> Option<&str> {
    message.blocks.iter().find_map(|block| match block {
        ContentBlock::Text { text } if !text.trim().is_empty() => Some(text.as_str()),
        ContentBlock::ToolUse { .. }
        | ContentBlock::ToolResult { .. }
        | ContentBlock::Text { .. }
        | ContentBlock::Thinking { .. }
        | ContentBlock::RedactedThinking { .. } => None,
        ContentBlock::Image { .. } | ContentBlock::ImageRef { .. } => None,
    })
}

fn has_interesting_extension(candidate: &str) -> bool {
    std::path::Path::new(candidate)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ["rs", "ts", "tsx", "js", "json", "md"]
                .iter()
                .any(|expected| extension.eq_ignore_ascii_case(expected))
        })
}

fn extract_file_candidates(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .filter_map(|token| {
            let candidate = token.trim_matches(|char: char| {
                matches!(char, ',' | '.' | ':' | ';' | ')' | '(' | '"' | '\'' | '`')
            });
            if candidate.contains('/') && has_interesting_extension(candidate) {
                Some(candidate.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn truncate_summary(content: &str, max_chars: usize) -> String {
    if content.chars().count() <= max_chars {
        return content.to_string();
    }
    let mut truncated = content.chars().take(max_chars).collect::<String>();
    truncated.push('…');
    truncated
}

/// Token-count heuristic with tiktoken when available, byte-count fallback.
pub fn estimate_text_tokens(text: &str) -> usize {
    match get_cl100k_encoder().get().and_then(|v| v.as_ref()) {
        Some(encoder) => encoder.encode_with_special_tokens(text).len(),
        None => text.len() / 4 + 1,
    }
}

/// Public-facing token estimation for an image block.
/// Uses the same formula as `estimate_message_tokens`: bytes/750 + 20.
/// Input is the raw base64 string length (33% inflated vs raw bytes).
pub fn estimate_image_block_tokens(base64_data: &str) -> usize {
    let bytes = base64_data.len() * 3 / 4;
    bytes / 750 + 20
}

/// Returns cached token count if available, otherwise computes it.
/// Uses tiktoken cl100k_base when available, falls back to `bytes/4`.
/// Includes role overhead (4 system/user, 3 assistant, 5 tool) and block
/// framing (3 per ToolUse/ToolResult) to stay consistent with the public
/// [`context::estimate_message_tokens`] which delegates here.
pub(crate) fn estimate_message_tokens(message: &ConversationMessage) -> usize {
    // Use cached value if already computed.
    if let Some(cached) = message.cached_tokens.get() {
        return *cached;
    }

    let mut total: usize = match message.role {
        MessageRole::System => 4,
        MessageRole::User => 4,
        MessageRole::Assistant => 3,
        MessageRole::Tool => 5,
    };

    total += message
        .blocks
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => estimate_text_tokens(text),
            ContentBlock::ToolUse { name, input, .. } => {
                let input_str = input.to_string();
                3 + estimate_text_tokens(name) + estimate_text_tokens(&input_str)
            }
            ContentBlock::ToolResult {
                tool_name, output, ..
            } => {
                3 + estimate_text_tokens(tool_name) + estimate_text_tokens(output)
            }
            ContentBlock::Image { data, .. } => {
                // Base64 is 33% inflated → decode bytes = len * 3/4.
                // Anthropic-style image token estimate: bytes / 750 + 20.
                let bytes = data.len() * 3 / 4;
                bytes / 750 + 20
            }
            ContentBlock::ImageRef { .. } => {
                // ImageRef has no inline data; use a fixed estimate.
                100
            }
            ContentBlock::Thinking { thinking, .. } => estimate_text_tokens(thinking),
            ContentBlock::RedactedThinking { data, .. } => estimate_text_tokens(data),
        })
        .sum::<usize>();

    // Populate cache. This is a best-effort write; if another thread raced
    // here first, the value was already set and ours is discarded.
    let _ = message.cached_tokens.set(total);
    total
}

fn extract_tag_block(content: &str, tag: &str) -> Option<String> {
    let start = format!("<{tag}>");
    let end = format!("</{tag}>");
    let start_index = content.find(&start)? + start.len();
    let end_index = content[start_index..].find(&end)? + start_index;
    Some(content[start_index..end_index].to_string())
}

fn strip_tag_block(content: &str, tag: &str) -> String {
    let start = format!("<{tag}>");
    let end = format!("</{tag}>");
    if let (Some(start_index), Some(end_index_rel)) = (content.find(&start), content.find(&end)) {
        let end_index = end_index_rel + end.len();
        let mut stripped = String::new();
        stripped.push_str(&content[..start_index]);
        stripped.push_str(&content[end_index..]);
        stripped
    } else {
        content.to_string()
    }
}

fn collapse_blank_lines(content: &str) -> String {
    let mut result = String::new();
    let mut last_blank = false;
    for line in content.lines() {
        let is_blank = line.trim().is_empty();
        if is_blank && last_blank {
            continue;
        }
        result.push_str(line);
        result.push('\n');
        last_blank = is_blank;
    }
    result
}

fn extract_existing_compacted_summary(message: &ConversationMessage) -> Option<String> {
    if message.role != MessageRole::System {
        return None;
    }

    let text = first_text_block(message)?;
    let summary = text.strip_prefix(COMPACT_CONTINUATION_PREAMBLE)?;
    let summary = summary
        .split_once(&format!("\n\n{COMPACT_RECENT_MESSAGES_NOTE}"))
        .map_or(summary, |(value, _)| value);
    let summary = summary
        .split_once(&format!("\n{COMPACT_DIRECT_RESUME_INSTRUCTION}"))
        .map_or(summary, |(value, _)| value);
    Some(summary.trim().to_string())
}

fn extract_summary_highlights(summary: &str) -> Vec<String> {
    // Summary must already be formatted (caller should pass format_compact_summary output).
    let mut lines = Vec::new();
    let mut in_timeline = false;

    for line in summary.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() || trimmed == "Summary:" || trimmed == "Conversation summary:" {
            continue;
        }
        if trimmed == "- Key timeline:" {
            in_timeline = true;
            continue;
        }
        if in_timeline {
            continue;
        }
        lines.push(trimmed.to_string());
    }

    lines
}

fn extract_summary_timeline(summary: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut in_timeline = false;

    for line in summary.lines() {
        let trimmed = line.trim_end();
        if trimmed == "- Key timeline:" {
            in_timeline = true;
            continue;
        }
        if !in_timeline {
            continue;
        }
        if trimmed.is_empty() {
            break;
        }
        lines.push(trimmed.to_string());
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::{
        collect_key_files, compact_session, find_turn_tail_start, format_compact_summary,
        get_compact_continuation_message, infer_pending_work, should_compact, CompactionConfig,
    };
    use crate::session::{ContentBlock, ConversationMessage, MessageRole, Session};
    use crate::summary_compression::SummaryCompressionBudget;
    use std::sync::OnceLock;
    use std::time::Instant;

    #[test]
    fn formats_compact_summary_like_upstream() {
        let summary = "<analysis>scratch</analysis>\n<summary>Kept work</summary>";
        assert_eq!(format_compact_summary(summary), "Summary:\nKept work");
    }

    #[test]
    fn leaves_small_sessions_unchanged() {
        let mut session = Session::new();
        session.messages = vec![ConversationMessage::user_text("hello")];

        let result = compact_session(&session, CompactionConfig::default());
        assert_eq!(result.removed_message_count, 0);
        assert_eq!(result.compacted_session, session);
        assert!(result.summary.is_empty());
        assert!(result.formatted_summary.is_empty());
    }

    #[test]
    fn compacts_older_messages_into_a_system_summary() {
        let mut session = Session::new();
        session.messages = vec![
            ConversationMessage::user_text("one ".repeat(200)),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "two ".repeat(200),
            }]),
            ConversationMessage::tool_result("1", "bash", "ok ".repeat(200), false),
            ConversationMessage {
                role: MessageRole::Assistant,
                blocks: vec![ContentBlock::Text {
                    text: "recent".to_string(),
                }],
                usage: None,
                created_at: Instant::now(),
                cached_tokens: OnceLock::new(),
                cached_input_message: OnceLock::new(),
            },
        ];

        let result = compact_session(
            &session,
            CompactionConfig {
                preserve_recent_messages: 2,
                preserve_recent_tokens: 1,
                max_estimated_tokens: 1,
                preserve_last_n_turns: 0,
                summary_budget: None,
            },
        );

        // With the tool-use/tool-result boundary fix, the compaction preserves
        // one extra message to avoid an orphaned tool result at the boundary.
        // messages[1] (assistant) must be kept along with messages[2] (tool result).
        assert!(
            result.removed_message_count <= 2,
            "expected at most 2 removed, got {}",
            result.removed_message_count
        );
        assert_eq!(
            result.compacted_session.messages[0].role,
            MessageRole::System
        );
        assert!(matches!(
            &result.compacted_session.messages[0].blocks[0],
            ContentBlock::Text { text } if text.contains("Summary:")
        ));
        assert!(result.formatted_summary.contains("Scope:"));
        assert!(result.formatted_summary.contains("Key timeline:"));
        assert!(should_compact(
            &session,
            CompactionConfig {
                preserve_recent_messages: 2,
                preserve_recent_tokens: 1,
                max_estimated_tokens: 1,
                preserve_last_n_turns: 0,
                summary_budget: None,
            }
        ));
        // Note: with the tool-use/tool-result boundary guard the compacted session
        // may preserve one extra message at the boundary, so token reduction is
        // not guaranteed for small sessions. The invariant that matters is that
        // the removed_message_count is non-zero (something was compacted).
        assert!(
            result.removed_message_count > 0,
            "compaction must remove at least one message"
        );
    }

    #[test]
    fn keeps_previous_compacted_context_when_compacting_again() {
        let mut initial_session = Session::new();
        initial_session.messages = vec![
            ConversationMessage::user_text("Investigate rust/crates/runtime/src/compact.rs"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "I will inspect the compact flow.".to_string(),
            }]),
            ConversationMessage::user_text("Also update rust/crates/runtime/src/conversation.rs"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "Next: preserve prior summary context during auto compact.".to_string(),
            }]),
        ];
        let config = CompactionConfig {
            preserve_recent_messages: 2,
            preserve_recent_tokens: 1,
            max_estimated_tokens: 1,
            preserve_last_n_turns: 0,
            summary_budget: None,
        };

        let first = compact_session(&initial_session, config);
        let mut follow_up_messages = first.compacted_session.messages.clone();
        follow_up_messages.extend([
            ConversationMessage::user_text("Please add regression tests for compaction."),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "Working on regression coverage now.".to_string(),
            }]),
        ]);

        let mut second_session = Session::new();
        second_session.messages = follow_up_messages;
        let second = compact_session(&second_session, config);

        assert!(second
            .formatted_summary
            .contains("Previously compacted context:"));
        assert!(second
            .formatted_summary
            .contains("Scope: 2 earlier messages compacted"));
        assert!(second
            .formatted_summary
            .contains("Newly compacted context:"));
        assert!(second
            .formatted_summary
            .contains("Also update rust/crates/runtime/src/conversation.rs"));
        assert!(matches!(
            &second.compacted_session.messages[0].blocks[0],
            ContentBlock::Text { text }
                if text.contains("Previously compacted context:")
                    && text.contains("Newly compacted context:")
        ));
        // F-3 wire-role fix: the continuation (System → "user") must not be
        // followed by another user-role message. The boundary walked forward to
        // the next Assistant, so the tail alternates correctly and the boundary
        // User message was folded into the summary instead of dropped.
        let messages = &second.compacted_session.messages;
        assert!(
            !messages[1]
                .blocks
                .iter()
                .any(|b| matches!(b, ContentBlock::Text { text } if text == "Please add regression tests for compaction.")),
            "the boundary user message should be summarized, not preserved verbatim after the continuation"
        );
        assert!(
            second
                .formatted_summary
                .contains("Please add regression tests for compaction."),
            "the boundary user message content must survive in the summary"
        );
    }

    #[test]
    fn ignores_existing_compacted_summary_when_deciding_to_recompact() {
        let summary = "<summary>Conversation summary:\n- Scope: earlier work preserved.\n- Key timeline:\n  - user: large preserved context\n</summary>";
        let mut session = Session::new();
        session.messages = vec![
            ConversationMessage {
                role: MessageRole::System,
                blocks: vec![ContentBlock::Text {
                    text: get_compact_continuation_message(summary, true, true),
                }],
                usage: None,
                created_at: Instant::now(),
                cached_tokens: OnceLock::new(),
                cached_input_message: OnceLock::new(),
            },
            ConversationMessage::user_text("tiny"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "recent".to_string(),
            }]),
        ];

        assert!(!should_compact(
            &session,
            CompactionConfig {
                preserve_recent_messages: 2,
                preserve_recent_tokens: 1,
                max_estimated_tokens: 1,
                preserve_last_n_turns: 0,
                summary_budget: None,
            }
        ));
    }

    #[test]
    fn truncates_long_blocks_in_summary() {
        let summary = super::summarize_block(&ContentBlock::Text {
            text: "x".repeat(400),
        });
        assert!(summary.ends_with('…'));
        assert!(summary.chars().count() <= 161);
    }

    #[test]
    fn extracts_key_files_from_message_content() {
        let files = collect_key_files(&[ConversationMessage::user_text(
            "Update rust/crates/runtime/src/compact.rs and rust/crates/claw-cli/src/main.rs next.",
        )]);
        assert!(files.contains(&"rust/crates/runtime/src/compact.rs".to_string()));
        assert!(files.contains(&"rust/crates/claw-cli/src/main.rs".to_string()));
    }

    /// Regression: compaction must not split an assistant(ToolUse) /
    /// user(ToolResult) pair at the boundary. An orphaned tool-result message
    /// without the preceding assistant `tool_calls` causes a 400 on the
    /// OpenAI-compat path (gaebal-gajae repro 2026-04-09).
    #[test]
    fn continuation_does_not_create_consecutive_user_wire_roles() {
        // A plain Q&A session has roles [U,A,U,A,U,A]. With
        // preserve_recent_messages = 4 the naive tail starts at a User message.
        // The continuation is emitted as System (wire "user"), so starting the
        // tail at a User/Tool message would produce [user(cont), user, ...] —
        // consecutive same-role messages that the Anthropic API rejects with
        // "roles must alternate". The boundary must walk forward to the next
        // Assistant message instead.
        let mut session = Session::new();
        session.messages = vec![
            ConversationMessage::user_text("one"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "two".to_string(),
            }]),
            ConversationMessage::user_text("three"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "four".to_string(),
            }]),
            ConversationMessage::user_text("five"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "six".to_string(),
            }]),
        ];

        let result = compact_session(
            &session,
            CompactionConfig {
                preserve_recent_messages: 4,
                preserve_recent_tokens: 1,
                max_estimated_tokens: 1,
                ..CompactionConfig::default()
            },
        );

        let messages = &result.compacted_session.messages;
        assert!(result.removed_message_count > 0);

        // The compacted session must NOT start with a standalone System
        // continuation immediately followed by a user-role message (wire: user,user).
        let first = &messages[0];
        assert_eq!(first.role, MessageRole::System);
        assert!(
            !messages
                .get(1)
                .is_some_and(|m| matches!(m.role, MessageRole::User | MessageRole::Tool)),
            "continuation must not create consecutive user wire messages: second={:?}",
            messages.get(1).map(|m| m.role)
        );
        assert_eq!(
            messages[1].role,
            MessageRole::Assistant,
            "tail should start at an Assistant message after boundary walk-forward"
        );
    }

    #[test]
    fn continuation_stays_separate_when_tail_starts_with_assistant() {
        // When the preserved tail starts with an Assistant message, the
        // continuation is emitted as a separate System message — the wire
        // sequence [user(cont), assistant, ...] alternates correctly.
        let mut session = Session::new();
        let tool_id = "call_tool_1";
        session.messages = vec![
            ConversationMessage::user_text("Search the codebase"),
            ConversationMessage::assistant(vec![ContentBlock::ToolUse {
                id: tool_id.to_string(),
                name: "search".to_string(),
                input: serde_json::json!({ "q": "TODO" }),
            }]),
            ConversationMessage::tool_result(tool_id, "search", "found 3 TODOs", false),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "Done.".to_string(),
            }]),
            ConversationMessage::user_text("Now run tests"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "Running.".to_string(),
            }]),
        ];

        let result = compact_session(
            &session,
            CompactionConfig {
                preserve_recent_messages: 1,
                preserve_recent_tokens: 1,
                max_estimated_tokens: 1,
                ..CompactionConfig::default()
            },
        );

        let messages = &result.compacted_session.messages;
        assert!(messages[0].role == MessageRole::System);
        assert_eq!(messages[1].role, MessageRole::Assistant);
    }

    #[test]
    fn zero_token_mode_still_honors_preserve_turns() {
        // In 0-mode (max_estimated_tokens == 0) the caller requests
        // unconditional compaction. The turn-preservation dimension must still
        // be respected — a user who sets CLAW_COMPACT_PRESERVE_TURNS expects a
        // whole user→assistant turn (including its tool trail) to survive, not
        // just the single message minimum.
        let tool_id = "call_tool_1";
        let mut session = Session::new();
        session.messages = vec![
            ConversationMessage::user_text("Search the codebase"),
            ConversationMessage::assistant(vec![ContentBlock::ToolUse {
                id: tool_id.to_string(),
                name: "search".to_string(),
                input: serde_json::json!({ "q": "TODO" }),
            }]),
            ConversationMessage::tool_result(tool_id, "search", "found 3 TODOs", false),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "I found the TODOs.".to_string(),
            }]),
            ConversationMessage::user_text("Now fix them"),
            ConversationMessage::assistant(vec![ContentBlock::ToolUse {
                id: tool_id.to_string(),
                name: "edit".to_string(),
                input: serde_json::json!({ "file": "src/lib.rs" }),
            }]),
            ConversationMessage::tool_result(tool_id, "edit", "patched", false),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "Fixed.".to_string(),
            }]),
        ];

        let result = compact_session(
            &session,
            CompactionConfig {
                preserve_recent_messages: 1,
                preserve_last_n_turns: 1,
                max_estimated_tokens: 0,
                ..CompactionConfig::default()
            },
        );

        let messages = &result.compacted_session.messages;
        assert!(result.removed_message_count > 0);
        assert_eq!(messages[0].role, MessageRole::System);
        // The final turn (assistant tool-use + tool result + final answer)
        // must survive. With preserve_recent_messages=1 alone only the last
        // message would remain; turn preservation keeps the whole trail.
        assert!(messages.len() >= 4, "got {} messages", messages.len());
        assert!(
            messages
                .iter()
                .skip(1)
                .any(|m| m.blocks.iter().any(|b| matches!(b, ContentBlock::ToolUse { .. }))),
            "the final turn's tool-use should be preserved"
        );
        assert!(
            messages
                .iter()
                .skip(1)
                .any(|m| m.blocks.iter().any(|b| matches!(b, ContentBlock::ToolResult { .. }))),
            "the final turn's tool result should be preserved"
        );
        assert_eq!(messages.last().unwrap().role, MessageRole::Assistant);
    }

    #[test]
    fn empty_summary_keeps_context() {
        // A degenerate summary budget (e.g. CLAW_SUMMARY_MAX_CHARS=0) must not
        // cause the removed messages to be discarded with no trace.
        let mut session = Session::new();
        session.messages = vec![
            ConversationMessage::user_text("one"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "two".to_string(),
            }]),
            ConversationMessage::user_text("three"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "four".to_string(),
            }]),
            ConversationMessage::user_text("five"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "six".to_string(),
            }]),
        ];

        let result = compact_session(
            &session,
            CompactionConfig {
                preserve_recent_messages: 2,
                preserve_recent_tokens: 1,
                max_estimated_tokens: 1,
                summary_budget: Some(SummaryCompressionBudget {
                    max_chars: 0,
                    max_lines: 0,
                    max_line_chars: 0,
                }),
                ..CompactionConfig::default()
            },
        );

        assert_eq!(
            result.removed_message_count, 0,
            "messages must not be dropped when the summary is empty"
        );
        assert_eq!(result.compacted_session, session);
    }

    #[test]
    fn compaction_does_not_split_tool_use_tool_result_pair() {
        use crate::session::{ContentBlock, Session};

        let tool_id = "call_abc";
        let mut session = Session::default();
        // Turn 1: user prompt
        session
            .push_message(ConversationMessage::user_text("Search for files"))
            .unwrap();
        // Turn 2: assistant calls a tool
        session
            .push_message(ConversationMessage::assistant(vec![
                ContentBlock::ToolUse {
                    id: tool_id.to_string(),
                    name: "search".to_string(),
                    input: serde_json::json!({"q": "*.rs"}),
                },
            ]))
            .unwrap();
        // Turn 3: tool result
        session
            .push_message(ConversationMessage::tool_result(
                tool_id,
                "search",
                "found 5 files",
                false,
            ))
            .unwrap();
        // Turn 4: assistant final response
        session
            .push_message(ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "Done.".to_string(),
            }]))
            .unwrap();

        // Compact preserving only 1 recent message — without the fix this
        // would cut the boundary so that the tool result (turn 3) is first,
        // without its preceding assistant tool_calls (turn 2).
        let config = CompactionConfig {
            preserve_recent_messages: 1,
            ..CompactionConfig::default()
        };
        let result = compact_session(&session, config);
        // After compaction, no two consecutive messages should have the pattern
        // tool_result immediately following a non-assistant message (i.e. an
        // orphaned tool result without a preceding assistant ToolUse).
        let messages = &result.compacted_session.messages;
        for i in 1..messages.len() {
            let curr_is_tool_result = messages[i]
                .blocks
                .first()
                .is_some_and(|b| matches!(b, ContentBlock::ToolResult { .. }));
            if curr_is_tool_result {
                let prev_has_tool_use = messages[i - 1]
                    .blocks
                    .iter()
                    .any(|b| matches!(b, ContentBlock::ToolUse { .. }));
                assert!(
                    prev_has_tool_use,
                    "message[{}] is a ToolResult but message[{}] has no ToolUse: {:?}",
                    i,
                    i - 1,
                    &messages[i - 1].blocks
                );
            }
        }
    }

    #[test]
    fn infers_pending_work_from_recent_messages() {
        let pending = infer_pending_work(&[
            ConversationMessage::user_text("done"),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "Next: update tests and follow up on remaining CLI polish.".to_string(),
            }]),
        ]);
        assert_eq!(pending.len(), 1);
        assert!(pending[0].contains("Next: update tests"));
    }

    // ---- find_turn_tail_start tests ----

    fn atext(s: &str) -> ConversationMessage {
        ConversationMessage::assistant(vec![ContentBlock::Text { text: s.into() }])
    }

    fn tooluse(id: &str, name: &str) -> ContentBlock {
        ContentBlock::ToolUse {
            id: id.into(),
            name: name.into(),
            input: serde_json::json!({}),
        }
    }

    #[test]
    fn turn_disabled_by_default() {
        assert_eq!(CompactionConfig::default().preserve_last_n_turns, 0);
    }

    #[test]
    fn turn_disabled_returns_sentinel() {
        let msgs = [
            ConversationMessage::user_text("hello"),
            atext("hi"),
        ];
        assert_eq!(find_turn_tail_start(&msgs, 0), 2);
    }

    #[test]
    fn turn_empty_slice_returns_sentinel() {
        let msgs: [ConversationMessage; 0] = [];
        assert_eq!(find_turn_tail_start(&msgs, 2), 0);
    }

    #[test]
    fn turn_preserves_exact_turns() {
        let msgs = [
            ConversationMessage::user_text("q1"),
            atext("a1"),
            ConversationMessage::user_text("q2"),
            atext("a2"),
            ConversationMessage::user_text("q3"),
            atext("a3"),
        ];
        // preserve_last_n_turns: 2 → keep last 2 pairs = indices [2..]
        assert_eq!(find_turn_tail_start(&msgs, 2), 2);
    }

    #[test]
    fn turn_not_enough_pairs_falls_back() {
        let msgs = [
            ConversationMessage::user_text("q1"),
            atext("a1"),
        ];
        // Need 3 turns but only 1 User → sentinel (len=2)
        assert_eq!(find_turn_tail_start(&msgs, 3), 2);
    }

    #[test]
    fn turn_partial_turn_at_end() {
        let msgs = [
            ConversationMessage::user_text("q1"),
            atext("a1"),
            ConversationMessage::user_text("q2"),
        ];
        // preserve_turns=2: User0 + User2 = 2 turns found → keep everything (0)
        assert_eq!(find_turn_tail_start(&msgs, 2), 0);
    }

    #[test]
    fn turn_counts_user_boundary_over_tool_messages() {
        // Sessions with tool calls: User→Assistant(ToolUse)→Tool→Assistant
        // should count as one turn because User is the boundary, not adjacency.
        let msgs = [
            ConversationMessage::user_text("q1"),
            ConversationMessage::assistant(vec![tooluse("t1", "test")]),
            ConversationMessage::tool_result("t1", "test", "ok", false),
            atext("a1"),
        ];
        // 1 User found = 1 turn → keep everything
        assert_eq!(find_turn_tail_start(&msgs, 1), 0);
    }

    #[test]
    fn turn_last_turn_isolation() {
        let msgs = [
            ConversationMessage::user_text("q1"),
            atext("a1"),
            ConversationMessage::user_text("q2"),
            atext("a2"),
            ConversationMessage::user_text("q3"),
            atext("a3"),
        ];
        // 1 turn → keep only the last pair = indices [4..]
        assert_eq!(find_turn_tail_start(&msgs, 1), 4);
        // 3 turns → keep everything
        assert_eq!(find_turn_tail_start(&msgs, 3), 0);
    }

    #[test]
    fn turn_compact_integration() {
        // Verify that turning on turn preservation actually changes
        // the compaction boundary vs the message-minimum baseline.
        let mut session = Session::new();
        session.messages = vec![
            ConversationMessage::user_text("first user input"),
            ConversationMessage::assistant(vec![tooluse("t1", "test")]),
            ConversationMessage::tool_result("t1", "test", "some long output for token count", false),
            atext("first assistant result"),
            ConversationMessage::user_text("second user input"),
            atext("second assistant result"),
        ];

        // With preserve_last_n_turns: 1 and small token budget,
        // the turn dimension should keep at least the last turn.
        let config = CompactionConfig {
            preserve_recent_messages: 1,
            preserve_recent_tokens: 1,
            max_estimated_tokens: 1,
            preserve_last_n_turns: 1,
            summary_budget: None,
        };
        let result = compact_session(&session, config);
        // At minimum the last turn (2 messages: user+assistant) is kept.
        assert!(result.removed_message_count > 0);
        assert_eq!(
            result.compacted_session.messages.last().unwrap().role,
            MessageRole::Assistant
        );
    }
}
