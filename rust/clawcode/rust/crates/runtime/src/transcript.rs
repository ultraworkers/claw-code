use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::session::{ContentBlock, ConversationMessage, MessageRole};

/// Best-effort append-only Markdown mirror of the terminal-visible
/// conversation, stored per workspace (and per day) so future AI sessions can
/// `rg` the plain-text history even after the JSONL session file rotates,
/// deletes old rotations, or filters large tool results.
///
/// Writing is intentionally best-effort: a transcript write failure must
/// never break, roll back, or slow down the live session.
#[derive(Debug, Clone)]
pub struct TranscriptWriter {
    path: PathBuf,
}

impl TranscriptWriter {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one message to the transcript as Markdown. Creates parent
    /// directories on demand. Errors are returned but callers should treat
    /// them as non-fatal (see [`Self::append_message_best_effort`]).
    pub fn append_message(
        &self,
        session_id: &str,
        message: &ConversationMessage,
    ) -> std::io::Result<()> {
        let markdown = render_message(session_id, message);
        if markdown.trim().is_empty() {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(markdown.as_bytes())?;
        file.flush()
    }

    /// Same as [`Self::append_message`] but swallows every error.
    pub fn append_message_best_effort(&self, session_id: &str, message: &ConversationMessage) {
        let _ = self.append_message(session_id, message);
    }
}

/// Compute the per-session transcript path:
/// `<config_home>/transcripts/<YYYY-MM-DD>-<HH>.md`.
///
/// The file is named after the session start time rounded to the hour (so a
/// day accumulates at most 24 files across all sessions, instead of one file
/// per second), and lives directly under the config-home `transcripts/`
/// directory — no per-workspace fingerprint subfolder. Each session mirrors
/// the terminal-visible conversation as Markdown, so a future AI session can
/// `rg` the plain-text history.
#[must_use]
pub fn transcript_path_for(created_at_ms: u64) -> std::io::Result<PathBuf> {
    let timestamp =
        jiff::Timestamp::from_millisecond(i64::try_from(created_at_ms).unwrap_or(i64::MAX))
            .unwrap_or_else(|_| jiff::Timestamp::now());
    let stamp = timestamp
        .to_zoned(jiff::tz::TimeZone::system())
        .strftime("%Y-%m-%d-%H")
        .to_string();
    Ok(crate::config::default_config_home()
        .join("transcripts")
        .join(format!("{stamp}.md")))
}

/// Render a single message as Markdown suitable for grep-friendly retrieval.
///
/// Format:
/// ```markdown
///
/// ## session-123 · 2026-08-13 15:30:02 · User
///
/// user text
///
/// ### Thinking
///
/// thinking text
///
/// ### ToolUse: bash
///
/// ```json
/// {...}
/// ```
///
/// ### ToolResult: bash
///
/// ```text
/// output
/// ```
/// ```
#[must_use]
pub fn render_message(session_id: &str, message: &ConversationMessage) -> String {
    if message.blocks.is_empty() {
        return String::new();
    }
    let role = role_label(message.role);
    let timestamp = current_wall_clock();
    let mut out = String::new();
    out.push_str("\n## ");
    out.push_str(session_id);
    out.push_str(" · ");
    out.push_str(&timestamp);
    out.push_str(" · ");
    out.push_str(role);
    out.push('\n');

    for block in &message.blocks {
        render_block(&mut out, block);
    }
    out
}

fn role_label(role: MessageRole) -> &'static str {
    match role {
        MessageRole::System => "System",
        MessageRole::User => "User",
        MessageRole::Assistant => "Assistant",
        MessageRole::Tool => "Tool",
    }
}

fn render_block(out: &mut String, block: &ContentBlock) {
    match block {
        ContentBlock::Text { text } => {
            if !text.trim().is_empty() {
                out.push_str("\n");
                out.push_str(text);
                out.push('\n');
            }
        }
        ContentBlock::Thinking { thinking, .. } => {
            if !thinking.trim().is_empty() {
                out.push_str("\n### Thinking\n\n");
                out.push_str(thinking.trim_end());
                out.push('\n');
            }
        }
        ContentBlock::RedactedThinking { data } => {
            let digest = short_hash(data);
            out.push_str(&format!("\n### RedactedThinking (ciphertext {digest})\n"));
        }
        ContentBlock::ToolUse { name, input, .. } => {
            out.push_str(&format!("\n### ToolUse: {name}\n\n```json\n"));
            out.push_str(&serde_json::to_string_pretty(input).unwrap_or_else(|_| input.to_string()));
            out.push_str("\n```\n");
        }
        ContentBlock::ToolResult {
            tool_name, output, is_error, ..
        } => {
            let suffix = if *is_error { " (error)" } else { "" };
            out.push_str(&format!("\n### ToolResult: {tool_name}{suffix}\n\n```text\n"));
            out.push_str(output);
            if !output.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
        ContentBlock::Image { mime_type, filename, .. }
        | ContentBlock::ImageRef { mime_type, filename, .. } => {
            let name = filename
                .as_deref()
                .map_or_else(String::new, |name| format!(" {name}"));
            out.push_str(&format!("\n[image: {mime_type}{name}]\n"));
        }
    }
}

/// Short stable fingerprint of a ciphertext payload, for grepping redacted
/// thinking blocks without dumping the ciphertext into the transcript.
fn short_hash(data: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hasher;
    hasher.write(data.as_bytes());
    format!("{:016x}", hasher.finish())
}

fn current_wall_clock() -> String {
    jiff::Timestamp::now()
        .to_zoned(jiff::tz::TimeZone::system())
        .strftime("%Y-%m-%d %H:%M:%S")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{render_message, transcript_path_for, TranscriptWriter};
    use crate::session::{ContentBlock, ConversationMessage, MessageRole};
    use std::time::Instant;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn msg(role: MessageRole, blocks: Vec<ContentBlock>) -> ConversationMessage {
        ConversationMessage {
            role,
            blocks,
            usage: None,
            created_at: Instant::now(),
            cached_tokens: Default::default(),
            cached_input_message: Default::default(),
        }
    }

    #[test]
    fn renders_user_and_assistant_text_with_header() {
        let rendered = render_message(
            "session-1",
            &msg(MessageRole::User, vec![ContentBlock::Text { text: "hello".into() }]),
        );
        assert!(rendered.contains("## session-1 · "));
        assert!(rendered.contains("· User\n"));
        assert!(rendered.contains("\nhello\n"));

        let rendered = render_message(
            "session-1",
            &msg(
                MessageRole::Assistant,
                vec![ContentBlock::Text {
                    text: "world".into(),
                }],
            ),
        );
        assert!(rendered.contains("· Assistant\n"));
        assert!(rendered.contains("\nworld\n"));
    }

    #[test]
    fn renders_thinking_and_tool_blocks() {
        let rendered = render_message(
            "s",
            &msg(
                MessageRole::Assistant,
                vec![
                    ContentBlock::Thinking {
                        thinking: "reason step".into(),
                        signature: None,
                    },
                    ContentBlock::ToolUse {
                        id: "t1".into(),
                        name: "bash".into(),
                        input: serde_json::json!({"command": "ls"}),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id: "t1".into(),
                        tool_name: "bash".into(),
                        output: "src\n".into(),
                        is_error: false,
                    },
                ],
            ),
        );
        assert!(rendered.contains("### Thinking\n\nreason step"));
        assert!(rendered.contains("### ToolUse: bash\n\n```json"));
        assert!(rendered.contains("\"command\": \"ls\""));
        assert!(rendered.contains("### ToolResult: bash\n\n```text\nsrc\n```"));
    }

    #[test]
    fn marks_error_results_and_images() {
        let rendered = render_message(
            "s",
            &msg(
                MessageRole::Tool,
                vec![
                    ContentBlock::ToolResult {
                        tool_use_id: "t1".into(),
                        tool_name: "bash".into(),
                        output: "boom".into(),
                        is_error: true,
                    },
                    ContentBlock::ImageRef {
                        hash_hex: "abc".into(),
                        mime_type: "image/png".into(),
                        filename: Some("shot.png".into()),
                    },
                ],
            ),
        );
        assert!(rendered.contains("### ToolResult: bash (error)\n\n```text\nboom\n```"));
        assert!(rendered.contains("[image: image/png shot.png]"));
    }

    #[test]
    fn empty_messages_render_nothing() {
        let rendered = render_message("s", &msg(MessageRole::User, Vec::new()));
        assert!(rendered.is_empty());
    }

    #[test]
    fn appends_to_transcript_file_best_effort() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("claw-transcript-test-{nanos}.md"));
        let writer = TranscriptWriter::new(&path);
        writer.append_message("s1", &msg(MessageRole::User, vec![ContentBlock::Text { text: "a".into() }])).expect("append ok");
        writer.append_message("s1", &msg(MessageRole::Assistant, vec![ContentBlock::Text { text: "b".into() }])).expect("append ok");
        let contents = std::fs::read_to_string(&path).expect("read ok");
        assert_eq!(contents.matches("## s1 · ").count(), 2);
        assert!(contents.contains("\na\n"));
        assert!(contents.contains("\nb\n"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn transcript_path_lives_under_config_home_per_session() {
        // Isolate CLAW_CONFIG_HOME so the test is hermetic.
        let lock = crate::test_env_lock();
        let home = std::env::temp_dir().join(format!(
            "claw-transcript-home-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let original = std::env::var_os("CLAW_CONFIG_HOME");
        std::env::set_var("CLAW_CONFIG_HOME", &home);
        // 2026-08-14 07:48:54 UTC+8
        let created_at_ms = 1786664926000u64;
        let path = transcript_path_for(created_at_ms).expect("path ok");
        match original {
            Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
            None => std::env::remove_var("CLAW_CONFIG_HOME"),
        }
        drop(lock);
        // <config_home>/transcripts/<YYYY-MM-DD>-<HH>.md — no fingerprint subfolder.
        assert!(path.starts_with(&home));
        let segments: Vec<_> = path.components().collect();
        assert_eq!(segments[segments.len() - 2].as_os_str(), "transcripts");
        let name = segments[segments.len() - 1].as_os_str().to_string_lossy();
        assert!(
            name.ends_with(".md") && name.contains('-') && name.len() == "YYYY-MM-DD-HH.md".len(),
            "hour file must be YYYY-MM-DD-HH.md, got {name}"
        );
        assert_eq!(name, "2026-08-14-07.md", "hour must derive from created_at_ms");
        std::fs::remove_dir_all(&home).ok();
    }
}
