use std::collections::{BTreeMap, HashMap};
use std::fmt::{Display, Formatter};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use crate::conversation::merge_tool_result_messages;
use crate::image_cache::ImageCache;
use crate::image_store::ImageStore;
use crate::json::{JsonError, JsonValue};
use crate::usage::TokenUsage;

const SESSION_VERSION: u32 = 1;
const ROTATE_AFTER_BYTES: u64 = 256 * 1024;
const MAX_ROTATED_FILES: usize = 3;
static SESSION_ID_COUNTER: AtomicU64 = AtomicU64::new(0);
static LAST_TIMESTAMP_MS: AtomicU64 = AtomicU64::new(0);
static LAST_SEC: AtomicU64 = AtomicU64::new(0);

/// Speaker role associated with a persisted conversation message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Structured message content stored inside a [`Session`].
#[derive(Debug, Clone, PartialEq)]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        tool_name: String,
        output: String,
        is_error: bool,
    },
    Image {
        /// MIME type (e.g. "image/png").
        mime_type: String,
        /// Base64-encoded image payload.
        data: String,
        /// Original filename (if available), used for display.
        filename: Option<String>,
    },
    ImageRef {
        /// SHA-256 hex hash of the compressed image data.
        hash_hex: String,
        /// MIME type (e.g. "image/png").
        mime_type: String,
        /// Original filename (if available).
        filename: Option<String>,
    },
    /// Model thinking/reasoning content.
    Thinking {
        thinking: String,
        signature: Option<String>,
    },
    /// Redacted model thinking returned by the provider. The `data` ciphertext
    /// must be persisted and echoed back verbatim for the tool-use round-trip;
    /// unlike a normal thinking block it carries no signature.
    RedactedThinking {
        data: String,
    },
}

/// One conversation message with optional token-usage metadata.
#[derive(Debug, Clone)]
pub struct ConversationMessage {
    pub role: MessageRole,
    pub blocks: Vec<ContentBlock>,
    pub usage: Option<TokenUsage>,
    /// In-memory timestamp for time-based context filtering (not persisted).
    /// Used by `context.rs` to expire WebSearch/WebFetch results after a TTL.
    pub created_at: Instant,
    /// Populated on first call to `estimate_message_tokens`. Messages are
    /// append-only within a session, so this cache is never invalidated.
    /// Serialisation skips this field (it is derived from content).
    pub cached_tokens: OnceLock<usize>,
    /// Cached serialised `InputMessage` JSON Value, populated by
    /// `convert_messages_cached` after the first conversion.  Survives within
    /// a single `filter_for_api` batch and is reused across retries.
    /// Serialisation skips this field.
    pub cached_input_message: OnceLock<serde_json::Value>,
}

impl PartialEq for ConversationMessage {
    fn eq(&self, other: &Self) -> bool {
        self.role == other.role && self.blocks == other.blocks && self.usage == other.usage
        // cached_tokens intentionally excluded — it's a computation cache
    }
}

impl Eq for ConversationMessage {}

/// Metadata describing the latest compaction that summarized a session.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionCompaction {
    pub count: u32,
    pub removed_message_count: usize,
    pub summary: String,
    /// Ratio of estimated tokens removed by the last compaction, if known.
    /// `None` means "not yet compacted" or "ratio is stale".
    /// Persisted via custom JSON using i64-millionths encoding to avoid NaN/Inf.
    pub last_savings_ratio: Option<f64>,
}

/// Provenance recorded when a session is forked from another session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFork {
    pub parent_session_id: String,
    pub branch_name: Option<String>,
}

/// A single user prompt recorded with a timestamp for history tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPromptEntry {
    pub timestamp_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionPersistence {
    path: PathBuf,
}

/// Persisted conversational state for the runtime and CLI session manager.
///
/// `workspace_root` binds the session to the worktree it was created in. The
/// global session store under `~/.local/share/opencode` is shared across every
/// `opencode serve` instance, so without an explicit workspace root parallel
/// lanes can race and report success while writes land in the wrong CWD. See
/// ROADMAP.md item 41 (Phantom completions root cause) for the full
/// background.
#[derive(Debug, Clone)]
pub struct Session {
    pub version: u32,
    pub session_id: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub messages: Vec<ConversationMessage>,
    pub compaction: Option<SessionCompaction>,
    pub fork: Option<SessionFork>,
    pub workspace_root: Option<PathBuf>,
    pub prompt_history: Vec<SessionPromptEntry>,
    /// The model used in this session, persisted so resumed sessions can
    /// report which model was originally used.
    /// Timestamp of last successful health check (ROADMAP #38)
    pub last_health_check_ms: Option<u64>,
    pub model: Option<String>,
    persistence: Option<SessionPersistence>,
    pub image_cache: ImageCache,
}

impl PartialEq for Session {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
            && self.session_id == other.session_id
            && self.created_at_ms == other.created_at_ms
            && self.updated_at_ms == other.updated_at_ms
            && self.messages == other.messages
            && self.compaction == other.compaction
            && self.fork == other.fork
            && self.workspace_root == other.workspace_root
            && self.prompt_history == other.prompt_history
            && self.last_health_check_ms == other.last_health_check_ms
    }
}

/// Errors raised while loading, parsing, or saving sessions.
#[derive(Debug)]
pub enum SessionError {
    Io(std::io::Error),
    Json(JsonError),
    Format(String),
}

impl Display for SessionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::Format(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<std::io::Error> for SessionError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<JsonError> for SessionError {
    fn from(value: JsonError) -> Self {
        Self::Json(value)
    }
}

impl Session {
    #[must_use]
    pub fn new() -> Self {
        let now = current_time_millis();
        Self {
            version: SESSION_VERSION,
            session_id: generate_session_id(),
            created_at_ms: now,
            updated_at_ms: now,
            messages: Vec::new(),
            compaction: None,
            fork: None,
            workspace_root: None,
            prompt_history: Vec::new(),
            last_health_check_ms: None,
            model: None,
            persistence: None,
            image_cache: ImageCache::new(),
        }
    }

    #[must_use]
    pub fn with_persistence_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.persistence = Some(SessionPersistence { path: path.into() });
        self
    }

    /// Bind this session to the workspace root it was created in.
    ///
    /// This is the per-worktree counterpart to the global session store and
    /// lets downstream tooling reject writes that drift to the wrong CWD when
    /// multiple `opencode serve` instances share `~/.local/share/opencode`.
    #[must_use]
    pub fn with_workspace_root(mut self, workspace_root: impl Into<PathBuf>) -> Self {
        self.workspace_root = Some(workspace_root.into());
        self
    }

    #[must_use]
    pub fn workspace_root(&self) -> Option<&Path> {
        self.workspace_root.as_deref()
    }

    #[must_use]
    pub fn persistence_path(&self) -> Option<&Path> {
        self.persistence.as_ref().map(|value| value.path.as_path())
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), SessionError> {
        let path = path.as_ref();
        let snapshot = self.render_jsonl_snapshot()?;
        rotate_session_file_if_needed(path)?;
        write_atomic(path, &snapshot)?;
        cleanup_rotated_logs(path)?;
        Ok(())
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, SessionError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path)?;
        let session = match JsonValue::parse(&contents) {
            Ok(value)
                if value
                    .as_object()
                    .is_some_and(|object| object.contains_key("messages")) =>
            {
                Self::from_json(&value)?
            }
            Err(_) | Ok(_) => Self::from_jsonl(&contents)?,
        };
        Ok(session.with_persistence_path(path.to_path_buf()))
    }

    pub fn push_message(&mut self, message: ConversationMessage) -> Result<(), SessionError> {
        self.touch();
        self.messages.push(message);
        let persist_result = {
            let message_ref = self.messages.last().ok_or_else(|| {
                SessionError::Format("message was just pushed but missing".to_string())
            })?;
            self.append_persisted_message(message_ref)
        };
        if let Err(error) = persist_result {
            self.messages.pop();
            return Err(error);
        }
        Ok(())
    }

    pub fn push_user_text(&mut self, text: impl Into<String>) -> Result<(), SessionError> {
        self.push_message(ConversationMessage::user_text(text))
    }

    pub fn push_user_content(
        &mut self,
        content_blocks: Vec<ContentBlock>,
    ) -> Result<(), SessionError> {
        self.push_message(ConversationMessage::user_content(content_blocks))
    }

    pub fn record_compaction(&mut self, summary: impl Into<String>, removed_message_count: usize) {
        self.touch();
        let count = self.compaction.as_ref().map_or(1, |value| value.count + 1);
        let last_savings_ratio = self.compaction.as_ref().and_then(|c| c.last_savings_ratio);
        self.compaction = Some(SessionCompaction {
            count,
            removed_message_count,
            summary: summary.into(),
            last_savings_ratio,
        });
    }

    /// Override the savings ratio on the existing compaction record.
    /// Used by `maybe_auto_compact` to set the ratio computed after compaction.
    pub fn set_compaction_savings_ratio(&mut self, ratio: Option<f64>) {
        if let Some(ref mut compaction) = self.compaction {
            compaction.last_savings_ratio = ratio;
        }
    }

    #[must_use]
    pub fn fork(&self, branch_name: Option<String>) -> Self {
        let now = current_time_millis();
        Self {
            version: self.version,
            session_id: generate_session_id(),
            created_at_ms: now,
            updated_at_ms: now,
            messages: self.messages.clone(),
            compaction: self.compaction.clone(),
            fork: Some(SessionFork {
                parent_session_id: self.session_id.clone(),
                branch_name: normalize_optional_string(branch_name),
            }),
            workspace_root: self.workspace_root.clone(),
            prompt_history: self.prompt_history.clone(),
            last_health_check_ms: self.last_health_check_ms,
            model: self.model.clone(),
            persistence: None,
            image_cache: ImageCache::new(),
        }
    }

    pub fn to_json(&self) -> Result<JsonValue, SessionError> {
        let mut object = BTreeMap::new();
        object.insert(
            "version".to_string(),
            JsonValue::Number(i64::from(self.version)),
        );
        object.insert(
            "session_id".to_string(),
            JsonValue::String(self.session_id.clone()),
        );
        object.insert(
            "created_at_ms".to_string(),
            JsonValue::Number(i64_from_u64(self.created_at_ms, "created_at_ms")?),
        );
        object.insert(
            "updated_at_ms".to_string(),
            JsonValue::Number(i64_from_u64(self.updated_at_ms, "updated_at_ms")?),
        );
        object.insert(
            "messages".to_string(),
            JsonValue::Array(
                self.messages
                    .iter()
                    .map(ConversationMessage::to_json)
                    .collect(),
            ),
        );
        if let Some(compaction) = &self.compaction {
            object.insert("compaction".to_string(), compaction.to_json()?);
        }
        if let Some(fork) = &self.fork {
            object.insert("fork".to_string(), fork.to_json());
        }
        if let Some(workspace_root) = &self.workspace_root {
            object.insert(
                "workspace_root".to_string(),
                JsonValue::String(workspace_root_to_string(workspace_root)?),
            );
        }
        if !self.prompt_history.is_empty() {
            object.insert(
                "prompt_history".to_string(),
                JsonValue::Array(
                    self.prompt_history
                        .iter()
                        .map(SessionPromptEntry::to_jsonl_record)
                        .collect(),
                ),
            );
        }
        Ok(JsonValue::Object(object))
    }

    pub fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("session must be an object".to_string()))?;
        let version = object
            .get("version")
            .and_then(JsonValue::as_i64)
            .ok_or_else(|| SessionError::Format("missing version".to_string()))?;
        let version = u32::try_from(version)
            .map_err(|_| SessionError::Format("version out of range".to_string()))?;
        let messages = object
            .get("messages")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| SessionError::Format("missing messages".to_string()))?
            .iter()
            .map(ConversationMessage::from_json)
            .collect::<Result<Vec<_>, _>>()?;
        let now = current_time_millis();
        let session_id = object
            .get("session_id")
            .and_then(JsonValue::as_str)
            .map_or_else(generate_session_id, ToOwned::to_owned);
        let created_at_ms = object
            .get("created_at_ms")
            .map(|value| required_u64_from_value(value, "created_at_ms"))
            .transpose()?
            .unwrap_or(now);
        let updated_at_ms = object
            .get("updated_at_ms")
            .map(|value| required_u64_from_value(value, "updated_at_ms"))
            .transpose()?
            .unwrap_or(created_at_ms);
        let compaction = object
            .get("compaction")
            .map(SessionCompaction::from_json)
            .transpose()?;
        let fork = object.get("fork").map(SessionFork::from_json).transpose()?;
        let workspace_root = object
            .get("workspace_root")
            .and_then(JsonValue::as_str)
            .map(PathBuf::from);
        let prompt_history = object
            .get("prompt_history")
            .and_then(JsonValue::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(SessionPromptEntry::from_json_opt)
                    .collect()
            })
            .unwrap_or_default();
        let model = object
            .get("model")
            .and_then(JsonValue::as_str)
            .map(String::from);
        Ok(Self {
            version,
            session_id,
            created_at_ms,
            updated_at_ms,
            messages: Self::normalize_legacy_tool_messages(messages),
            compaction,
            fork,
            workspace_root,
            prompt_history,
            last_health_check_ms: None,
            model,
            persistence: None,
            image_cache: ImageCache::new(),
        })
    }

    fn from_jsonl(contents: &str) -> Result<Self, SessionError> {
        let mut version = SESSION_VERSION;
        let mut session_id = None;
        let mut created_at_ms = None;
        let mut updated_at_ms = None;
        let mut messages = Vec::new();
        let mut compaction = None;
        let mut fork = None;
        let mut workspace_root = None;
        let mut model = None;
        let mut prompt_history = Vec::new();

        for (line_number, raw_line) in contents.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            let value = JsonValue::parse(line).map_err(|error| {
                SessionError::Format(format!(
                    "invalid JSONL record at line {}: {}",
                    line_number + 1,
                    error
                ))
            })?;
            let object = value.as_object().ok_or_else(|| {
                SessionError::Format(format!(
                    "JSONL record at line {} must be an object",
                    line_number + 1
                ))
            })?;
            match object
                .get("type")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| {
                    SessionError::Format(format!(
                        "JSONL record at line {} missing type",
                        line_number + 1
                    ))
                })? {
                "session_meta" => {
                    version = required_u32(object, "version")?;
                    session_id = Some(required_string(object, "session_id")?);
                    created_at_ms = Some(required_u64(object, "created_at_ms")?);
                    updated_at_ms = Some(required_u64(object, "updated_at_ms")?);
                    fork = object.get("fork").map(SessionFork::from_json).transpose()?;
                    workspace_root = object
                        .get("workspace_root")
                        .and_then(JsonValue::as_str)
                        .map(PathBuf::from);
                    model = object
                        .get("model")
                        .and_then(JsonValue::as_str)
                        .map(String::from);
                }
                "message" => {
                    let message_value = object.get("message").ok_or_else(|| {
                        SessionError::Format(format!(
                            "JSONL record at line {} missing message",
                            line_number + 1
                        ))
                    })?;
                    messages.push(ConversationMessage::from_json(message_value)?);
                }
                "compaction" => {
                    compaction = Some(SessionCompaction::from_json(&JsonValue::Object(
                        object.clone(),
                    ))?);
                }
                "prompt_history" => {
                    if let Some(entry) =
                        SessionPromptEntry::from_json_opt(&JsonValue::Object(object.clone()))
                    {
                        prompt_history.push(entry);
                    }
                }
                other => {
                    return Err(SessionError::Format(format!(
                        "unsupported JSONL record type at line {}: {other}",
                        line_number + 1
                    )))
                }
            }
        }

        let now = current_time_millis();
        Ok(Self {
            version,
            session_id: session_id.unwrap_or_else(generate_session_id),
            created_at_ms: created_at_ms.unwrap_or(now),
            updated_at_ms: updated_at_ms.unwrap_or(created_at_ms.unwrap_or(now)),
            messages: Self::normalize_legacy_tool_messages(messages),
            compaction,
            fork,
            workspace_root,
            prompt_history,
            last_health_check_ms: None,
            model,
            persistence: None,
            image_cache: ImageCache::new(),
        })
    }

    /// Merge consecutive tool-role messages into one. Sessions saved before the
    /// parallel-tool-result merge fix may contain one `tool_result` message per
    /// parallel call, which the Anthropic API rejects ("`tool_use` ids were
    /// found without `tool_result` blocks immediately after"). Normalising on
    /// load keeps resumed sessions wire-valid.
    fn normalize_legacy_tool_messages(messages: Vec<ConversationMessage>) -> Vec<ConversationMessage> {
        let mut result = Vec::with_capacity(messages.len());
        for message in messages {
            if message.role == MessageRole::Tool
                && result
                    .last()
                    .is_some_and(|last: &ConversationMessage| last.role == MessageRole::Tool)
            {
                let last = result.pop().expect("last tool message just checked");
                result.push(merge_tool_result_messages(vec![last, message]));
            } else {
                result.push(message);
            }
        }
        result
    }

    /// Record a user prompt with the current wall-clock timestamp.
    ///
    /// The entry is appended to the in-memory history and, when a persistence
    /// path is configured, incrementally written to the JSONL session file.
    pub fn push_prompt_entry(&mut self, text: impl Into<String>) -> Result<(), SessionError> {
        let timestamp_ms = current_time_millis();
        let entry = SessionPromptEntry {
            timestamp_ms,
            text: text.into(),
        };
        self.prompt_history.push(entry);
        let entry_ref = self.prompt_history.last().expect("entry was just pushed");
        self.append_persisted_prompt_entry(entry_ref)
    }

    fn render_jsonl_snapshot(&self) -> Result<String, SessionError> {
        let mut lines = vec![self.meta_record()?.render()];
        if let Some(compaction) = &self.compaction {
            lines.push(compaction.to_jsonl_record()?.render());
        }
        lines.extend(
            self.prompt_history
                .iter()
                .map(|entry| entry.to_jsonl_record().render()),
        );
        lines.extend(
            self.messages
                .iter()
                .map(|message| message_record(&filter_toolresult_for_persist(message)).render()),
        );
        let mut rendered = lines.join("\n");
        rendered.push('\n');
        Ok(rendered)
    }

    fn append_persisted_message(&self, message: &ConversationMessage) -> Result<(), SessionError> {
        let Some(path) = self.persistence_path() else {
            return Ok(());
        };

        // Filter WebFetch ToolResult content before persisting to JSONL.
        // Full content is still in memory (self.messages) and visible to the AI,
        // but we only store a short marker in the file to avoid bloating it
        // with repeated web page content on cache hits.
        let filtered = filter_toolresult_for_persist(message);

        let needs_bootstrap = !path.exists() || fs::metadata(path)?.len() == 0;
        if needs_bootstrap {
            self.save_to_path(path)?;
            return Ok(());
        }

        let mut file = OpenOptions::new().append(true).open(path)?;
        let pos = file.metadata()?.len();
        let record = message_record(&filtered).render();
        if let Err(e) = writeln!(file, "{record}") {
            // Truncate to known-good position to prevent partial JSONL corruption
            let _ = file.set_len(pos);
            return Err(SessionError::Io(e));
        }
        Ok(())
    }

    fn append_persisted_prompt_entry(
        &self,
        entry: &SessionPromptEntry,
    ) -> Result<(), SessionError> {
        let Some(path) = self.persistence_path() else {
            return Ok(());
        };

        let needs_bootstrap = !path.exists() || fs::metadata(path)?.len() == 0;
        if needs_bootstrap {
            self.save_to_path(path)?;
            return Ok(());
        }

        let mut file = OpenOptions::new().append(true).open(path)?;
        writeln!(file, "{}", entry.to_jsonl_record().render())?;
        Ok(())
    }

    fn meta_record(&self) -> Result<JsonValue, SessionError> {
        let mut object = BTreeMap::new();
        object.insert(
            "type".to_string(),
            JsonValue::String("session_meta".to_string()),
        );
        object.insert(
            "version".to_string(),
            JsonValue::Number(i64::from(self.version)),
        );
        object.insert(
            "session_id".to_string(),
            JsonValue::String(self.session_id.clone()),
        );
        object.insert(
            "created_at_ms".to_string(),
            JsonValue::Number(i64_from_u64(self.created_at_ms, "created_at_ms")?),
        );
        object.insert(
            "updated_at_ms".to_string(),
            JsonValue::Number(i64_from_u64(self.updated_at_ms, "updated_at_ms")?),
        );
        if let Some(fork) = &self.fork {
            object.insert("fork".to_string(), fork.to_json());
        }
        if let Some(workspace_root) = &self.workspace_root {
            object.insert(
                "workspace_root".to_string(),
                JsonValue::String(workspace_root_to_string(workspace_root)?),
            );
        }
        if let Some(model) = &self.model {
            object.insert("model".to_string(), JsonValue::String(model.clone()));
        }
        Ok(JsonValue::Object(object))
    }

    fn touch(&mut self) {
        self.updated_at_ms = current_time_millis();
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

pub fn externalize_content_block_image(
    block: &mut ContentBlock,
    store: &ImageStore,
    b64_cache: &mut HashMap<String, String>,
) -> Result<(), SessionError> {
    match block {
        ContentBlock::Image {
            mime_type,
            data,
            filename,
        } => {
            // data is already base64 of final compressed bytes from input.rs
            // Store directly — no double compression
            let raw_bytes = base64::engine::general_purpose::STANDARD
                .decode(data.as_bytes())
                .map_err(|e| SessionError::Format(format!("base64 decode: {e}")))?;
            let hash_hex = store.store(&raw_bytes, mime_type)?;
            // Write .b64 sidecar for future fast-path loads
            let raw_path = store.path_for(&hash_hex, mime_type);
            let b64_path = raw_path.with_extension(format!("{}.b64", raw_path.extension().unwrap_or_default().to_string_lossy()));
            let _ = std::fs::write(&b64_path, data.as_bytes());
            // Cache the same base64 string for hot-path reuse
            b64_cache.insert(hash_hex.clone(), data.clone());
            let stored_mime = mime_type.clone();
            *block = ContentBlock::ImageRef {
                hash_hex,
                mime_type: stored_mime,
                filename: filename.take(),
            };
        }
        ContentBlock::ImageRef {
            hash_hex,
            mime_type,
            ..
        } => {
            // Image already stored by input.rs, just populate the base64 cache
            if !b64_cache.contains_key(hash_hex) {
                match store.load_base64(hash_hex, mime_type) {
                    Ok(b64) => {
                        b64_cache.insert(hash_hex.clone(), b64);
                    }
                    Err(e) => {
                        eprintln!("[IMAGE] Failed to cache base64 for {hash_hex}: {e}");
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

#[allow(dead_code)]
pub fn resolve_content_block_image(block: &mut ContentBlock, store: &ImageStore) -> Result<(), SessionError> {
    if let ContentBlock::ImageRef {
        hash_hex,
        mime_type,
        filename,
    } = block
    {
        let base64_data = store.load_base64(hash_hex, mime_type)?;
        *block = ContentBlock::Image {
            mime_type: mime_type.clone(),
            data: base64_data,
            filename: filename.take(),
        };
    }
    Ok(())
}

pub fn externalize_message_images(
    msg: &mut ConversationMessage,
    store: &ImageStore,
    b64_cache: &mut HashMap<String, String>,
) -> Result<(), SessionError> {
    for block in &mut msg.blocks {
        externalize_content_block_image(block, store, b64_cache)?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn resolve_message_images(msg: &mut ConversationMessage, store: &ImageStore) -> Result<(), SessionError> {
    for block in &mut msg.blocks {
        resolve_content_block_image(block, store)?;
    }
    Ok(())
}

impl ConversationMessage {
    #[must_use]
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text { text: text.into() }],
            usage: None,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        }
    }

    #[must_use]
    pub fn user_content(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: MessageRole::User,
            blocks,
            usage: None,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        }
    }

    #[must_use]
    pub fn assistant(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: MessageRole::Assistant,
            blocks,
            usage: None,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        }
    }

    #[must_use]
    pub fn assistant_with_usage(blocks: Vec<ContentBlock>, usage: Option<TokenUsage>) -> Self {
        Self {
            role: MessageRole::Assistant,
            blocks,
            usage,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        }
    }

    #[must_use]
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        tool_name: impl Into<String>,
        output: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: tool_use_id.into(),
                tool_name: tool_name.into(),
                output: output.into(),
                is_error,
            }],
            usage: None,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        }
    }

    #[must_use]
    pub fn to_json(&self) -> JsonValue {
        let mut object = BTreeMap::new();
        object.insert(
            "role".to_string(),
            JsonValue::String(
                match self.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                }
                .to_string(),
            ),
        );
        object.insert(
            "blocks".to_string(),
            JsonValue::Array(self.blocks.iter().map(ContentBlock::to_json).collect()),
        );
        if let Some(usage) = self.usage {
            object.insert("usage".to_string(), usage_to_json(usage));
        }
        JsonValue::Object(object)
    }

    fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("message must be an object".to_string()))?;
        let role = match object
            .get("role")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| SessionError::Format("missing role".to_string()))?
        {
            "system" => MessageRole::System,
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "tool" => MessageRole::Tool,
            other => {
                return Err(SessionError::Format(format!(
                    "unsupported message role: {other}"
                )))
            }
        };
        let blocks = object
            .get("blocks")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| SessionError::Format("missing blocks".to_string()))?
            .iter()
            .map(ContentBlock::from_json)
            .collect::<Result<Vec<_>, _>>()?;
        let usage = object.get("usage").map(usage_from_json).transpose()?;
        Ok(Self {
            role,
            blocks,
            usage,
            created_at: Instant::now(),
            cached_tokens: OnceLock::new(),
            cached_input_message: OnceLock::new(),
        })
    }
}

impl ContentBlock {
    #[must_use]
    pub fn to_json(&self) -> JsonValue {
        let mut object = BTreeMap::new();
        match self {
            Self::Text { text } => {
                object.insert("type".to_string(), JsonValue::String("text".to_string()));
                object.insert("text".to_string(), JsonValue::String(text.clone()));
            }
            Self::ToolUse { id, name, input } => {
                object.insert(
                    "type".to_string(),
                    JsonValue::String("tool_use".to_string()),
                );
                object.insert("id".to_string(), JsonValue::String(id.clone()));
                object.insert("name".to_string(), JsonValue::String(name.clone()));
                object.insert("input".to_string(), JsonValue::String(input.to_string()));
            }
            Self::Image {
                mime_type,
                data,
                filename,
            } => {
                object.insert("type".to_string(), JsonValue::String("image".to_string()));
                object.insert(
                    "mime_type".to_string(),
                    JsonValue::String(mime_type.clone()),
                );
                object.insert("data".to_string(), JsonValue::String(data.clone()));
                if let Some(name) = filename {
                    object.insert("filename".to_string(), JsonValue::String(name.clone()));
                }
            }
            Self::ImageRef {
                hash_hex,
                mime_type,
                filename,
            } => {
                object.insert(
                    "type".to_string(),
                    JsonValue::String("image_ref".to_string()),
                );
                object.insert(
                    "hash_hex".to_string(),
                    JsonValue::String(hash_hex.clone()),
                );
                object.insert(
                    "mime_type".to_string(),
                    JsonValue::String(mime_type.clone()),
                );
                if let Some(name) = filename {
                    object.insert("filename".to_string(), JsonValue::String(name.clone()));
                }
            }
            Self::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } => {
                object.insert(
                    "type".to_string(),
                    JsonValue::String("tool_result".to_string()),
                );
                object.insert(
                    "tool_use_id".to_string(),
                    JsonValue::String(tool_use_id.clone()),
                );
                object.insert(
                    "tool_name".to_string(),
                    JsonValue::String(tool_name.clone()),
                );
                object.insert("output".to_string(), JsonValue::String(output.clone()));
                object.insert("is_error".to_string(), JsonValue::Bool(*is_error));
            }
            Self::Thinking { thinking, signature } => {
                object.insert("type".to_string(), JsonValue::String("thinking".to_string()));
                // Persist the thinking content so a resumed session can echo the
                // block back to the Anthropic API verbatim (content + signature).
                // `from_json` restores both fields; the field is optional so old
                // session files (signature-only) still load.
                if !thinking.is_empty() {
                    object.insert("thinking".to_string(), JsonValue::String(thinking.clone()));
                }
                if let Some(sig) = signature {
                    object.insert("signature".to_string(), JsonValue::String(sig.clone()));
                }
            }
            Self::RedactedThinking { data } => {
                object.insert(
                    "type".to_string(),
                    JsonValue::String("redacted_thinking".to_string()),
                );
                // Persist the ciphertext verbatim so a resumed session can echo
                // the redacted block back to the Anthropic API unchanged.
                object.insert("data".to_string(), JsonValue::String(data.clone()));
            }
        }
        JsonValue::Object(object)
    }

    fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("block must be an object".to_string()))?;
        match object
            .get("type")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| SessionError::Format("missing block type".to_string()))?
        {
            "text" => Ok(Self::Text {
                text: required_string(object, "text")?,
            }),
            "tool_use" => {
                let input_str = required_string(object, "input")?;
                let input = serde_json::from_str(&input_str)
                    .unwrap_or_else(|_| serde_json::Value::String(input_str.clone()));
                Ok(Self::ToolUse {
                    id: required_string(object, "id")?,
                    name: required_string(object, "name")?,
                    input,
                })
            }
            "image" => Ok(Self::Image {
                mime_type: required_string(object, "mime_type")?,
                data: required_string(object, "data")?,
                filename: object
                    .get("filename")
                    .and_then(JsonValue::as_str)
                    .map(ToOwned::to_owned),
            }),
            "image_ref" => Ok(Self::ImageRef {
                hash_hex: required_string(object, "hash_hex")?,
                mime_type: required_string(object, "mime_type")?,
                filename: object
                    .get("filename")
                    .and_then(JsonValue::as_str)
                    .map(ToOwned::to_owned),
            }),
            "tool_result" => Ok(Self::ToolResult {
                tool_use_id: required_string(object, "tool_use_id")?,
                tool_name: required_string(object, "tool_name")?,
                output: required_string(object, "output")?,
                is_error: object
                    .get("is_error")
                    .and_then(JsonValue::as_bool)
                    .ok_or_else(|| SessionError::Format("missing is_error".to_string()))?,
            }),
            "thinking" => Ok(Self::Thinking {
                // Backward-compatible: old sessions have `thinking` field, new ones don't.
                // Either way, we restore with empty thinking content (only signature matters
                // for API round-trip).
                thinking: object
                    .get("thinking")
                    .and_then(JsonValue::as_str)
                    .map(ToOwned::to_owned)
                    .unwrap_or_default(),
                signature: object
                    .get("signature")
                    .and_then(JsonValue::as_str)
                    .map(ToOwned::to_owned),
            }),
            "redacted_thinking" => Ok(Self::RedactedThinking {
                data: object
                    .get("data")
                    .and_then(JsonValue::as_str)
                    .map(ToOwned::to_owned)
                    .unwrap_or_default(),
            }),
            other => Err(SessionError::Format(format!(
                "unsupported block type: {other}"
            ))),
        }
    }
}

impl SessionCompaction {
    pub fn to_json(&self) -> Result<JsonValue, SessionError> {
        let mut object = BTreeMap::new();
        object.insert(
            "count".to_string(),
            JsonValue::Number(i64::from(self.count)),
        );
        object.insert(
            "removed_message_count".to_string(),
            JsonValue::Number(i64_from_usize(
                self.removed_message_count,
                "removed_message_count",
            )?),
        );
        object.insert(
            "summary".to_string(),
            JsonValue::String(self.summary.clone()),
        );
        if let Some(ratio) = self.last_savings_ratio {
            let safe = if ratio.is_finite() { ratio } else { 0.0 };
            object.insert(
                "last_savings_ratio".to_string(),
                JsonValue::Number((safe * 1_000_000.0).round() as i64),
            );
        }
        Ok(JsonValue::Object(object))
    }

    pub fn to_jsonl_record(&self) -> Result<JsonValue, SessionError> {
        let mut object = BTreeMap::new();
        object.insert(
            "type".to_string(),
            JsonValue::String("compaction".to_string()),
        );
        object.insert(
            "count".to_string(),
            JsonValue::Number(i64::from(self.count)),
        );
        object.insert(
            "removed_message_count".to_string(),
            JsonValue::Number(i64_from_usize(
                self.removed_message_count,
                "removed_message_count",
            )?),
        );
        object.insert(
            "summary".to_string(),
            JsonValue::String(self.summary.clone()),
        );
        if let Some(ratio) = self.last_savings_ratio {
            let safe = if ratio.is_finite() { ratio } else { 0.0 };
            object.insert(
                "last_savings_ratio".to_string(),
                JsonValue::Number((safe * 1_000_000.0).round() as i64),
            );
        }
        Ok(JsonValue::Object(object))
    }

    fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("compaction must be an object".to_string()))?;
        let last_savings_ratio = object
            .get("last_savings_ratio")
            .and_then(JsonValue::as_i64)
            .map(|v| v as f64 / 1_000_000.0);
        Ok(Self {
            count: required_u32(object, "count")?,
            removed_message_count: required_usize(object, "removed_message_count")?,
            summary: required_string(object, "summary")?,
            last_savings_ratio,
        })
    }
}

impl SessionFork {
    #[must_use]
    pub fn to_json(&self) -> JsonValue {
        let mut object = BTreeMap::new();
        object.insert(
            "parent_session_id".to_string(),
            JsonValue::String(self.parent_session_id.clone()),
        );
        if let Some(branch_name) = &self.branch_name {
            object.insert(
                "branch_name".to_string(),
                JsonValue::String(branch_name.clone()),
            );
        }
        JsonValue::Object(object)
    }

    fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("fork metadata must be an object".to_string()))?;
        Ok(Self {
            parent_session_id: required_string(object, "parent_session_id")?,
            branch_name: object
                .get("branch_name")
                .and_then(JsonValue::as_str)
                .map(ToOwned::to_owned),
        })
    }
}

impl SessionPromptEntry {
    #[must_use]
    pub fn to_jsonl_record(&self) -> JsonValue {
        let mut object = BTreeMap::new();
        object.insert(
            "type".to_string(),
            JsonValue::String("prompt_history".to_string()),
        );
        object.insert(
            "timestamp_ms".to_string(),
            JsonValue::Number(i64::try_from(self.timestamp_ms).unwrap_or(i64::MAX)),
        );
        object.insert("text".to_string(), JsonValue::String(self.text.clone()));
        JsonValue::Object(object)
    }

    fn from_json_opt(value: &JsonValue) -> Option<Self> {
        let object = value.as_object()?;
        let timestamp_ms = object
            .get("timestamp_ms")
            .and_then(JsonValue::as_i64)
            .and_then(|value| u64::try_from(value).ok())?;
        let text = object.get("text").and_then(JsonValue::as_str)?.to_string();
        Some(Self { timestamp_ms, text })
    }
}

/// Replace large ToolResult outputs AND ToolUse inputs with short markers
/// before persisting to JSONL. The full content remains in `self.messages`
/// (in-memory) so the AI can still read it during the current turn.
///
/// Applies to tools that produce large outputs or accept large inputs:
/// - WebFetch: web page content
/// - read_file: file content
/// - new_file: file content in `content` input field + output echo
/// - edit_file: code diff in `old_string`/`new_string` input fields + output
/// - bash: command output
/// - grep_search: search results
fn filter_toolresult_for_persist(message: &ConversationMessage) -> ConversationMessage {
    /// Tools whose ToolResult output should be replaced with a marker in JSONL.
    const FILTER_TOOLS: &[&str] = &[
        "WebFetch", "read_file", "new_file", "edit_file", "bash", "grep_search",
    ];
    /// Tools whose ToolUse input should be replaced with a marker in JSONL.
    const FILTER_INPUT_TOOLS: &[&str] = &["new_file", "edit_file"];
    /// Minimum output size (bytes) to trigger filtering.
    const MIN_SIZE: usize = 500;

    let blocks = message
        .blocks
        .iter()
        .map(|block| match block {
            // Filter ToolResult output
            ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } if !is_error
                && output.len() > MIN_SIZE
                && FILTER_TOOLS.contains(&tool_name.as_str()) =>
            {
                let marker = persist_marker(tool_name, output);
                ContentBlock::ToolResult {
                    tool_use_id: tool_use_id.clone(),
                    tool_name: tool_name.clone(),
                    output: marker,
                    is_error: *is_error,
                }
            }
            // Filter ToolUse input (new_file content, edit_file old_string/new_string)
            ContentBlock::ToolUse { id, name, input }
                if input.to_string().len() > MIN_SIZE
                    && FILTER_INPUT_TOOLS.contains(&name.as_str()) =>
            {
                let input_str = input.to_string();
                let marker = input_marker(name, &input_str);
                ContentBlock::ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                    input: serde_json::Value::String(marker),
                }
            }
            other => other.clone(),
        })
        .collect();
    ConversationMessage {
        role: message.role,
        blocks,
        usage: message.usage.clone(),
        created_at: message.created_at,
        cached_tokens: message.cached_tokens.clone(),
        cached_input_message: OnceLock::new(),
    }
}

/// Generate a short marker for JSONL persistence, including file path when available.
fn persist_marker(tool_name: &str, output: &str) -> String {
    match tool_name {
        "read_file" => {
            let path = extract_json_str(output, "filePath").unwrap_or_default();
            let lines = extract_json_num(output, "numLines")
                .or_else(|| extract_json_num(output, "lineCount"))
                .unwrap_or("?".into());
            format!("[read_file: {path}, {lines} lines]")
        }
        "new_file" => {
            let path = extract_json_str(output, "filePath")
                .or_else(|| extract_json_str(output, "path"))
                .unwrap_or_default();
            format!("[new_file: {path}]")
        }
        "edit_file" => {
            let path = extract_json_str(output, "filePath")
                .or_else(|| extract_json_str(output, "path"))
                .unwrap_or_default();
            let diff = extract_json_str(output, "diffPath").unwrap_or_default();
            format!("[edit_file: {path}, diff={diff}]")
        }
        "bash" => {
            format!("[bash: {} chars]", output.chars().count())
        }
        "WebFetch" => {
            format!("[WebFetch: {} chars]", output.chars().count())
        }
        "grep_search" => {
            let files = extract_json_num(output, "num_files").unwrap_or("?".into());
            format!("[grep_search: {files} files]")
        }
        _ => format!("[{tool_name}: {} chars cached]", output.chars().count()),
    }
}

/// Generate a short marker for ToolUse input fields in JSONL.
/// Preserves file path but drops large content/old_string/new_string.
fn input_marker(tool_name: &str, input: &str) -> String {
    match tool_name {
        "new_file" => {
            let path = extract_json_str(input, "path").unwrap_or_default();
            let chars = input.chars().count();
            format!(r#"{{"path":"{path}","content":"[{chars} chars]"}}"#)
        }
        "edit_file" => {
            let path = extract_json_str(input, "path").unwrap_or_default();
            let replace_all = if input.contains("\"replace_all\":true")
                || input.contains("\"replace_all\": true")
            {
                "true"
            } else {
                "false"
            };
            let chars = input.chars().count();
            format!(
                r#"{{"path":"{path}","old_string":"[{chars} chars]","new_string":"[{chars} chars]","replace_all":{replace_all}}}"#
            )
        }
        _ => format!(r#"{{"_filtered":"{chars} chars"}}"#, chars = input.chars().count()),
    }
}

/// Extract a string value from JSON by key (fast path, no full parse).
fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\":");
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let rest = rest.trim_start();
    if rest.starts_with('"') {
        let end = rest[1..].find('"')?;
        Some(rest[1..1 + end].to_string())
    } else {
        None
    }
}

/// Extract a numeric value from JSON by key (fast path, no full parse).
fn extract_json_num(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\":");
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let rest = rest.trim_start();
    if rest.starts_with("null") {
        return Some("null".to_string());
    }
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-' && c != '.')
        .unwrap_or(rest.len());
    if end > 0 {
        Some(rest[..end].to_string())
    } else {
        None
    }
}

fn message_record(message: &ConversationMessage) -> JsonValue {
    let mut object = BTreeMap::new();
    object.insert("type".to_string(), JsonValue::String("message".to_string()));
    object.insert("message".to_string(), message.to_json());
    JsonValue::Object(object)
}

fn usage_to_json(usage: TokenUsage) -> JsonValue {
    let mut object = BTreeMap::new();
    object.insert(
        "input_tokens".to_string(),
        JsonValue::Number(i64::from(usage.input_tokens)),
    );
    object.insert(
        "output_tokens".to_string(),
        JsonValue::Number(i64::from(usage.output_tokens)),
    );
    object.insert(
        "cache_creation_input_tokens".to_string(),
        JsonValue::Number(i64::from(usage.cache_creation_input_tokens)),
    );
    object.insert(
        "cache_read_input_tokens".to_string(),
        JsonValue::Number(i64::from(usage.cache_read_input_tokens)),
    );
    JsonValue::Object(object)
}

fn usage_from_json(value: &JsonValue) -> Result<TokenUsage, SessionError> {
    let object = value
        .as_object()
        .ok_or_else(|| SessionError::Format("usage must be an object".to_string()))?;
    Ok(TokenUsage {
        input_tokens: required_u32(object, "input_tokens")?,
        output_tokens: required_u32(object, "output_tokens")?,
        cache_creation_input_tokens: required_u32(object, "cache_creation_input_tokens")?,
        cache_read_input_tokens: required_u32(object, "cache_read_input_tokens")?,
    })
}

fn required_string(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<String, SessionError> {
    object
        .get(key)
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))
}

fn required_u32(object: &BTreeMap<String, JsonValue>, key: &str) -> Result<u32, SessionError> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))?;
    u32::try_from(value).map_err(|_| SessionError::Format(format!("{key} out of range")))
}

fn required_u64(object: &BTreeMap<String, JsonValue>, key: &str) -> Result<u64, SessionError> {
    let value = object
        .get(key)
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))?;
    required_u64_from_value(value, key)
}

fn required_u64_from_value(value: &JsonValue, key: &str) -> Result<u64, SessionError> {
    let value = value
        .as_i64()
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))?;
    u64::try_from(value).map_err(|_| SessionError::Format(format!("{key} out of range")))
}

fn required_usize(object: &BTreeMap<String, JsonValue>, key: &str) -> Result<usize, SessionError> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))?;
    usize::try_from(value).map_err(|_| SessionError::Format(format!("{key} out of range")))
}

fn i64_from_u64(value: u64, key: &str) -> Result<i64, SessionError> {
    i64::try_from(value)
        .map_err(|_| SessionError::Format(format!("{key} out of range for JSON number")))
}

fn i64_from_usize(value: usize, key: &str) -> Result<i64, SessionError> {
    i64::try_from(value)
        .map_err(|_| SessionError::Format(format!("{key} out of range for JSON number")))
}

fn workspace_root_to_string(path: &Path) -> Result<String, SessionError> {
    let path = dunce::simplified(path).to_owned();
    let s = path.to_str().ok_or_else(|| {
        SessionError::Format(format!(
            "workspace_root is not valid UTF-8: {}",
            path.display()
        ))
    })?;
    Ok(s.to_string())
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn current_time_millis() -> u64 {
    let wall_clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default();
    let mut candidate = wall_clock;
    loop {
        let previous = LAST_TIMESTAMP_MS.load(Ordering::Relaxed);
        if candidate <= previous {
            candidate = previous.saturating_add(1);
        }
        match LAST_TIMESTAMP_MS.compare_exchange(
            previous,
            candidate,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => return candidate,
            Err(actual) => candidate = actual.saturating_add(1),
        }
    }
}

fn current_time_secs() -> u64 {
    let wall_clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut candidate = wall_clock;
    loop {
        let previous = LAST_SEC.load(Ordering::Relaxed);
        if candidate <= previous {
            candidate = previous.saturating_add(1);
        }
        match LAST_SEC.compare_exchange(previous, candidate, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => return candidate,
            Err(actual) => candidate = actual.saturating_add(1),
        }
    }
}

fn hhmmss_from_epoch(secs: u64) -> String {
    jiff::Timestamp::new(secs as i64, 0)
        .unwrap()
        .to_zoned(jiff::tz::TimeZone::system())
        .strftime("%H%M%S")
        .to_string()
}

fn generate_session_id() -> String {
    let secs = current_time_secs();
    format!("session-{}-{secs}", hhmmss_from_epoch(secs))
}

fn write_atomic(path: &Path, contents: &str) -> Result<(), SessionError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp_path = temporary_path_for(path);
    fs::write(&temp_path, contents)?;
    fs::rename(temp_path, path)?;
    Ok(())
}

fn temporary_path_for(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("session");
    path.with_file_name(format!(
        "{file_name}.tmp-{}-{}",
        current_time_millis(),
        SESSION_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn rotate_session_file_if_needed(path: &Path) -> Result<(), SessionError> {
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(());
    };
    if metadata.len() < ROTATE_AFTER_BYTES {
        return Ok(());
    }
    let rotated_path = rotated_log_path(path);
    fs::rename(path, rotated_path)?;
    Ok(())
}

fn rotated_log_path(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("session");
    path.with_file_name(format!("{stem}.rot-{}.jsonl", current_time_millis()))
}

fn cleanup_rotated_logs(path: &Path) -> Result<(), SessionError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("session");
    let prefix = format!("{stem}.rot-");
    let mut rotated_paths = fs::read_dir(parent)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|entry_path| {
            entry_path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| {
                    name.starts_with(&prefix)
                        && Path::new(name)
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
                })
        })
        .collect::<Vec<_>>();

    rotated_paths.sort_by_key(|entry_path| {
        fs::metadata(entry_path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH)
    });

    let remove_count = rotated_paths.len().saturating_sub(MAX_ROTATED_FILES);
    for stale_path in rotated_paths.into_iter().take(remove_count) {
        fs::remove_file(stale_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        cleanup_rotated_logs, current_time_millis, rotate_session_file_if_needed, ContentBlock,
        ConversationMessage, MessageRole, Session, SessionFork,
    };
    use crate::json::JsonValue;
    use crate::usage::TokenUsage;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn session_timestamps_are_monotonic_under_tight_loops() {
        let first = current_time_millis();
        let second = current_time_millis();
        let third = current_time_millis();

        assert!(first < second);
        assert!(second < third);
    }

    #[test]
    fn persists_and_restores_session_jsonl() {
        let mut session = Session::new();
        session
            .push_user_text("hello")
            .expect("user message should append");
        session
            .push_message(ConversationMessage::assistant_with_usage(
                vec![
                    ContentBlock::Text {
                        text: "thinking".to_string(),
                    },
                    ContentBlock::ToolUse {
                        id: "tool-1".to_string(),
                        name: "bash".to_string(),
                        input: serde_json::Value::String("echo hi".to_string()),
                    },
                ],
                Some(TokenUsage {
                    input_tokens: 10,
                    output_tokens: 4,
                    cache_creation_input_tokens: 1,
                    cache_read_input_tokens: 2,
                }),
            ))
            .expect("assistant message should append");
        session
            .push_message(ConversationMessage::tool_result(
                "tool-1", "bash", "hi", false,
            ))
            .expect("tool result should append");

        let path = temp_session_path("jsonl");
        session.save_to_path(&path).expect("session should save");
        let restored = Session::load_from_path(&path).expect("session should load");
        fs::remove_file(&path).expect("temp file should be removable");

        assert_eq!(restored, session);
        assert_eq!(restored.messages[2].role, MessageRole::Tool);
        assert_eq!(
            restored.messages[1].usage.expect("usage").total_tokens(),
            17
        );
        assert_eq!(restored.session_id, session.session_id);
    }

    #[test]
    fn thinking_content_round_trips_through_jsonl() {
        let mut session = Session::new();
        session
            .push_user_text("think and answer")
            .expect("user message should append");
        session
            .push_message(ConversationMessage::assistant(vec![ContentBlock::Thinking {
                thinking: "reasoning text".to_string(),
                signature: Some("sig123".to_string()),
            }]))
            .expect("assistant message should append");

        let path = temp_session_path("thinking");
        session.save_to_path(&path).expect("session should save");
        let restored = Session::load_from_path(&path).expect("session should load");
        fs::remove_file(&path).expect("temp file should be removable");

        let block = &restored.messages[1].blocks[0];
        assert!(matches!(
            block,
            ContentBlock::Thinking { thinking, signature }
                if thinking == "reasoning text" && signature.as_deref() == Some("sig123")
        ));
    }

    #[test]
    fn redacted_thinking_round_trips_through_jsonl() {
        let mut session = Session::new();
        session
            .push_user_text("think and answer")
            .expect("user message should append");
        session
            .push_message(ConversationMessage::assistant(vec![ContentBlock::RedactedThinking {
                data: "ciphertext_blob_abc".to_string(),
            }]))
            .expect("assistant message should append");

        let path = temp_session_path("redacted_thinking");
        session.save_to_path(&path).expect("session should save");
        let restored = Session::load_from_path(&path).expect("session should load");
        fs::remove_file(&path).expect("temp file should be removable");

        let block = &restored.messages[1].blocks[0];
        assert!(matches!(
            block,
            ContentBlock::RedactedThinking { data }
                if data == "ciphertext_blob_abc"
        ));
    }

    #[test]
    fn load_merges_legacy_split_tool_result_messages() {
        let mut session = Session::new();
        session
            .push_user_text("do parallel tools")
            .expect("user message should append");
        session
            .push_message(ConversationMessage::assistant(vec![
                ContentBlock::ToolUse {
                    id: "tool-a".to_string(),
                    name: "bash".to_string(),
                    input: serde_json::Value::String("echo a".to_string()),
                },
                ContentBlock::ToolUse {
                    id: "tool-b".to_string(),
                    name: "bash".to_string(),
                    input: serde_json::Value::String("echo b".to_string()),
                },
            ]))
            .expect("assistant message should append");
        session
            .push_message(ConversationMessage::tool_result("tool-a", "bash", "a", false))
            .expect("tool result should append");
        session
            .push_message(ConversationMessage::tool_result("tool-b", "bash", "b", false))
            .expect("tool result should append");

        // Serialize the pre-merge layout directly: two consecutive tool messages.
        let path = temp_session_path("legacy-tools");
        session.save_to_path(&path).expect("session should save");
        let restored = Session::load_from_path(&path).expect("session should load");
        fs::remove_file(&path).expect("temp file should be removable");

        let tool_messages: Vec<_> = restored
            .messages
            .iter()
            .filter(|message| message.role == MessageRole::Tool)
            .collect();
        assert_eq!(
            tool_messages.len(),
            1,
            "split tool results must be merged on load"
        );
        assert_eq!(tool_messages[0].blocks.len(), 2);
    }

    #[test]
    fn loads_legacy_session_json_object() {        let path = temp_session_path("legacy");
        let legacy = JsonValue::Object(
            [
                ("version".to_string(), JsonValue::Number(1)),
                (
                    "messages".to_string(),
                    JsonValue::Array(vec![ConversationMessage::user_text("legacy").to_json()]),
                ),
            ]
            .into_iter()
            .collect(),
        );
        fs::write(&path, legacy.render()).expect("legacy file should write");

        let restored = Session::load_from_path(&path).expect("legacy session should load");
        fs::remove_file(&path).expect("temp file should be removable");

        assert_eq!(restored.messages.len(), 1);
        assert_eq!(
            restored.messages[0],
            ConversationMessage::user_text("legacy")
        );
        assert!(!restored.session_id.is_empty());
    }

    #[test]
    fn appends_messages_to_persisted_jsonl_session() {
        let path = temp_session_path("append");
        let mut session = Session::new().with_persistence_path(path.clone());
        session
            .save_to_path(&path)
            .expect("initial save should succeed");
        session
            .push_user_text("hi")
            .expect("user append should succeed");
        session
            .push_message(ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "hello".to_string(),
            }]))
            .expect("assistant append should succeed");

        let restored = Session::load_from_path(&path).expect("session should replay from jsonl");
        fs::remove_file(&path).expect("temp file should be removable");

        assert_eq!(restored.messages.len(), 2);
        assert_eq!(restored.messages[0], ConversationMessage::user_text("hi"));
    }

    #[test]
    fn persists_compaction_metadata() {
        let path = temp_session_path("compaction");
        let mut session = Session::new();
        session
            .push_user_text("before")
            .expect("message should append");
        session.record_compaction("summarized earlier work", 4);
        session.save_to_path(&path).expect("session should save");

        let restored = Session::load_from_path(&path).expect("session should load");
        fs::remove_file(&path).expect("temp file should be removable");

        let compaction = restored.compaction.expect("compaction metadata");
        assert_eq!(compaction.count, 1);
        assert_eq!(compaction.removed_message_count, 4);
        assert!(compaction.summary.contains("summarized"));
    }

    #[test]
    fn forks_sessions_with_branch_metadata_and_persists_it() {
        let path = temp_session_path("fork");
        let mut session = Session::new();
        session
            .push_user_text("before fork")
            .expect("message should append");

        let forked = session
            .fork(Some("investigation".to_string()))
            .with_persistence_path(path.clone());
        forked
            .save_to_path(&path)
            .expect("forked session should save");

        let restored = Session::load_from_path(&path).expect("forked session should load");
        fs::remove_file(&path).expect("temp file should be removable");

        assert_ne!(restored.session_id, session.session_id);
        assert_eq!(
            restored.fork,
            Some(SessionFork {
                parent_session_id: session.session_id,
                branch_name: Some("investigation".to_string()),
            })
        );
        assert_eq!(restored.messages, forked.messages);
    }

    #[test]
    fn rotates_and_cleans_up_large_session_logs() {
        // given
        let path = temp_session_path("rotation");
        let oversized_length =
            usize::try_from(super::ROTATE_AFTER_BYTES + 10).expect("rotate threshold should fit");
        fs::write(&path, "x".repeat(oversized_length)).expect("oversized file should write");

        // when
        rotate_session_file_if_needed(&path).expect("rotation should succeed");

        // then
        assert!(
            !path.exists(),
            "original path should be rotated away before rewrite"
        );

        for _ in 0..5 {
            let rotated = super::rotated_log_path(&path);
            fs::write(&rotated, "old").expect("rotated file should write");
        }
        cleanup_rotated_logs(&path).expect("cleanup should succeed");

        let rotated_count = rotation_files(&path).len();
        assert!(rotated_count <= super::MAX_ROTATED_FILES);
        for rotated in rotation_files(&path) {
            fs::remove_file(rotated).expect("rotated file should be removable");
        }
    }

    #[test]
    fn rejects_jsonl_record_without_type() {
        // given
        let path = write_temp_session_file(
            "missing-type",
            r#"{"message":{"role":"user","blocks":[{"type":"text","text":"hello"}]}}"#,
        );

        // when
        let error = Session::load_from_path(&path)
            .expect_err("session should reject JSONL records without a type");

        // then
        assert!(error.to_string().contains("missing type"));
        fs::remove_file(path).expect("temp file should be removable");
    }

    #[test]
    fn rejects_jsonl_message_record_without_message_payload() {
        // given
        let path = write_temp_session_file("missing-message", r#"{"type":"message"}"#);

        // when
        let error = Session::load_from_path(&path)
            .expect_err("session should reject JSONL message records without message payload");

        // then
        assert!(error.to_string().contains("missing message"));
        fs::remove_file(path).expect("temp file should be removable");
    }

    #[test]
    fn rejects_jsonl_record_with_unknown_type() {
        // given
        let path = write_temp_session_file("unknown-type", r#"{"type":"mystery"}"#);

        // when
        let error = Session::load_from_path(&path)
            .expect_err("session should reject unknown JSONL record types");

        // then
        assert!(error.to_string().contains("unsupported JSONL record type"));
        fs::remove_file(path).expect("temp file should be removable");
    }

    #[test]
    fn rejects_legacy_session_json_without_messages() {
        // given
        let session = JsonValue::Object(
            [("version".to_string(), JsonValue::Number(1))]
                .into_iter()
                .collect(),
        );

        // when
        let error = Session::from_json(&session)
            .expect_err("legacy session objects should require messages");

        // then
        assert!(error.to_string().contains("missing messages"));
    }

    #[test]
    fn normalizes_blank_fork_branch_name_to_none() {
        // given
        let session = Session::new();

        // when
        let forked = session.fork(Some("   ".to_string()));

        // then
        assert_eq!(forked.fork.expect("fork metadata").branch_name, None);
    }

    #[test]
    fn rejects_unknown_content_block_type() {
        // given
        let block = JsonValue::Object(
            [("type".to_string(), JsonValue::String("unknown".to_string()))]
                .into_iter()
                .collect(),
        );

        // when
        let error = ContentBlock::from_json(&block)
            .expect_err("content blocks should reject unknown types");

        // then
        assert!(error.to_string().contains("unsupported block type"));
    }

    #[test]
    fn persists_workspace_root_round_trip_and_forks_inherit_it() {
        // given
        let path = temp_session_path("workspace-root");
        let workspace_root = PathBuf::from("/tmp/b4-phantom-diag");
        let mut session = Session::new().with_workspace_root(workspace_root.clone());
        session
            .push_user_text("write to the right cwd")
            .expect("user message should append");

        // when
        session
            .save_to_path(&path)
            .expect("workspace-bound session should save");
        let restored = Session::load_from_path(&path).expect("session should load");
        let forked = restored.fork(Some("phantom-diag".to_string()));
        fs::remove_file(&path).expect("temp file should be removable");

        // then
        assert_eq!(restored.workspace_root(), Some(workspace_root.as_path()));
        assert_eq!(forked.workspace_root(), Some(workspace_root.as_path()));
    }

    fn temp_session_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("runtime-session-{label}-{nanos}.json"))
    }

    fn write_temp_session_file(label: &str, contents: &str) -> PathBuf {
        let path = temp_session_path(label);
        fs::write(&path, format!("{contents}\n")).expect("temp session file should write");
        path
    }

    fn rotation_files(path: &Path) -> Vec<PathBuf> {
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .expect("temp path should have file stem")
            .to_string();
        fs::read_dir(path.parent().expect("temp path should have parent"))
            .expect("temp dir should read")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|entry_path| {
                entry_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|name| {
                        name.starts_with(&format!("{stem}.rot-"))
                            && Path::new(name)
                                .extension()
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
                    })
            })
            .collect()
    }
}

/// Returns the shared sessions directory.
/// All workspaces share a single `~/.claw/sessions/` directory; workspace
/// isolation is enforced at the session metadata level (workspace_root field),
/// not at the filesystem level.
/// Called by external consumers (e.g. clawhip) to enumerate sessions for a CWD.
#[allow(dead_code)]
pub fn workspace_sessions_dir(cwd: &std::path::Path) -> Result<std::path::PathBuf, SessionError> {
    let store = crate::session_control::SessionStore::from_cwd(cwd)
        .map_err(|e| SessionError::Io(std::io::Error::other(e.to_string())))?;
    Ok(store.sessions_dir().to_path_buf())
}

#[cfg(test)]
mod workspace_sessions_dir_tests {
    use super::*;
    use std::fs;

    #[test]
    fn workspace_sessions_dir_returns_shared_path_for_valid_cwd() {
        let tmp = std::env::temp_dir().join("claw-session-dir-test");
        fs::create_dir_all(&tmp).expect("create temp dir");

        let result = workspace_sessions_dir(&tmp);
        assert!(
            result.is_ok(),
            "workspace_sessions_dir should succeed for a valid CWD, got: {result:?}"
        );
        let dir = result.unwrap();
        assert!(!dir.as_os_str().is_empty());
        // Two calls with the same CWD should produce identical paths (deterministic)
        let result2 = workspace_sessions_dir(&tmp).unwrap();
        assert_eq!(dir, result2, "workspace_sessions_dir must be deterministic");

        fs::remove_dir_all(&tmp).ok();
    }
}
