//! Per-project persistent memory store.
//!
//! Provides file-based memory storage scoped to each workspace, stored under
//! `~/.claw/projects/<workspace-hash>/memory/`. Each memory entry is a
//! markdown file with YAML frontmatter containing name, description, and type.
//! An index file (`MEMORY.md`) provides a summary loaded into conversation
//! context.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::config::default_config_home;
use crate::session_control::workspace_fingerprint;

const INDEX_FILENAME: &str = "MEMORY.md";
const INDEX_MAX_LINES: usize = 200;

/// Categorizes what kind of information a memory entry holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// Information about the user's role, goals, and preferences.
    User,
    /// Guidance on how to approach work — corrections and confirmations.
    Feedback,
    /// Ongoing project context not derivable from code or git.
    Project,
    /// Pointers to where information lives in external systems.
    Reference,
    /// Unrecognized or missing type field.
    Unknown,
}

impl MemoryType {
    fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "user" => Self::User,
            "feedback" => Self::Feedback,
            "project" => Self::Project,
            "reference" => Self::Reference,
            _ => Self::Unknown,
        }
    }

    /// Returns the canonical string representation used in frontmatter.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Feedback => "feedback",
            Self::Project => "project",
            Self::Reference => "reference",
            Self::Unknown => "unknown",
        }
    }
}

/// Metadata parsed from a single memory file's YAML frontmatter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEntry {
    /// Kebab-case identifier slug from the `name:` field.
    pub name: String,
    /// One-line summary used for relevance matching.
    pub description: String,
    /// Category of this memory entry.
    pub memory_type: MemoryType,
    /// Filesystem path to the memory file.
    pub path: PathBuf,
}

/// Manages the memory directory for a specific workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryStore {
    memory_dir: PathBuf,
    workspace_root: PathBuf,
}

impl MemoryStore {
    /// Create a `MemoryStore` for the given workspace root.
    ///
    /// The memory directory is resolved as:
    /// `~/.claw/projects/<workspace_fingerprint>/memory/`
    #[must_use]
    pub fn for_workspace(workspace_root: &Path) -> Self {
        let canonical =
            fs::canonicalize(workspace_root).unwrap_or_else(|_| workspace_root.to_path_buf());
        let fingerprint = workspace_fingerprint(&canonical);
        let memory_dir = default_config_home()
            .join("projects")
            .join(fingerprint)
            .join("memory");
        Self {
            memory_dir,
            workspace_root: canonical,
        }
    }

    /// Create a `MemoryStore` with an explicit memory directory path.
    /// Useful for testing.
    #[must_use]
    pub fn with_dir(memory_dir: PathBuf, workspace_root: PathBuf) -> Self {
        Self {
            memory_dir,
            workspace_root,
        }
    }

    /// The resolved memory directory path.
    #[must_use]
    pub fn memory_dir(&self) -> &Path {
        &self.memory_dir
    }

    /// The workspace root this store is bound to.
    #[must_use]
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Path to the `MEMORY.md` index file.
    #[must_use]
    pub fn index_path(&self) -> PathBuf {
        self.memory_dir.join(INDEX_FILENAME)
    }

    /// Read the `MEMORY.md` index file content, truncated to [`INDEX_MAX_LINES`].
    ///
    /// Returns `None` if the file doesn't exist or is empty.
    #[must_use]
    pub fn index_content(&self) -> Option<String> {
        let content = fs::read_to_string(self.index_path()).ok()?;
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(truncate_to_lines(trimmed, INDEX_MAX_LINES))
    }

    /// Returns whether the memory directory exists on disk.
    #[must_use]
    pub fn exists(&self) -> bool {
        self.memory_dir.is_dir()
    }

    /// Create the memory directory if it doesn't exist.
    pub fn ensure_dir(&self) -> io::Result<()> {
        fs::create_dir_all(&self.memory_dir)
    }

    /// Count of memory entry files (`.md` files excluding `MEMORY.md`).
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.list_entry_paths().len()
    }

    /// List all memory entries by scanning the directory and parsing frontmatter.
    ///
    /// Files that fail to parse (no frontmatter, missing required fields) are
    /// silently skipped — a malformed file should not break the system.
    #[must_use]
    pub fn list_entries(&self) -> Vec<MemoryEntry> {
        self.list_entry_paths()
            .into_iter()
            .filter_map(|path| {
                let content = fs::read_to_string(&path).ok()?;
                parse_memory_frontmatter(&content, path)
            })
            .collect()
    }

    /// List paths of all `.md` files in the memory directory, excluding
    /// the index file. Returns an empty vec if the directory doesn't exist.
    fn list_entry_paths(&self) -> Vec<PathBuf> {
        let entries = match fs::read_dir(&self.memory_dir) {
            Ok(entries) => entries,
            Err(_) => return Vec::new(),
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| is_memory_file(path))
            .collect();
        paths.sort();
        paths
    }
}

/// Parse YAML frontmatter from a memory file's content.
///
/// Expected format:
/// ```text
/// ---
/// name: short-kebab-slug
/// description: one-line summary
/// type: user|feedback|project|reference
/// ---
/// ```
///
/// Returns `None` if the file lacks valid frontmatter or the required
/// `name` field.
fn parse_memory_frontmatter(content: &str, path: PathBuf) -> Option<MemoryEntry> {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return None;
    }

    let mut name = None;
    let mut description = None;
    let mut memory_type = MemoryType::Unknown;

    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        // Skip indented lines (nested YAML mappings) — only top-level keys matter
        if line.starts_with(' ') || line.starts_with('\t') {
            continue;
        }
        if let Some(value) = strip_frontmatter_key(trimmed, "name") {
            if !value.is_empty() {
                name = Some(value);
            }
        } else if let Some(value) = strip_frontmatter_key(trimmed, "description") {
            if !value.is_empty() {
                description = Some(value);
            }
        } else if let Some(value) = strip_frontmatter_key(trimmed, "type") {
            memory_type = MemoryType::from_str(&value);
        }
    }

    let name = name?;
    Some(MemoryEntry {
        name,
        description: description.unwrap_or_default(),
        memory_type,
        path,
    })
}

/// Extract the value for a top-level YAML key, stripping a single outer quote pair.
fn strip_frontmatter_key(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.strip_prefix(':')?;
    let trimmed = rest.trim();
    let value = strip_outer_quotes(trimmed);
    Some(value.to_string())
}

/// Strip a single matching pair of outer quotes (single or double).
fn strip_outer_quotes(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &s[1..s.len() - 1];
        }
    }
    s
}

/// Returns `true` if the path is a `.md` file that is not the index.
fn is_memory_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let ext_ok = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md"));
    let not_index = path
        .file_name()
        .and_then(|f| f.to_str())
        .is_some_and(|f| !f.eq_ignore_ascii_case(INDEX_FILENAME));
    ext_ok && not_index
}

/// Truncate content to at most `max_lines` lines, appending a notice if
/// truncation occurred.
fn truncate_to_lines(content: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= max_lines {
        return content.to_string();
    }
    let truncated: String = lines[..max_lines].join("\n");
    let remaining = lines.len() - max_lines;
    format!("{truncated}\n\n[... {remaining} more lines truncated]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn for_workspace_produces_stable_path() {
        let workspace = PathBuf::from("/tmp/test-project");
        let store = MemoryStore::with_dir(
            PathBuf::from("/home/user/.claw/projects/abc123/memory"),
            workspace.clone(),
        );
        assert_eq!(store.workspace_root(), workspace);
        assert_eq!(
            store.memory_dir(),
            Path::new("/home/user/.claw/projects/abc123/memory")
        );
    }

    #[test]
    fn index_path_resolves_correctly() {
        let store =
            MemoryStore::with_dir(PathBuf::from("/tmp/memory"), PathBuf::from("/tmp/project"));
        assert_eq!(store.index_path(), PathBuf::from("/tmp/memory/MEMORY.md"));
    }

    #[test]
    fn index_content_returns_none_when_missing() {
        let dir = temp_dir();
        let store = MemoryStore::with_dir(dir.path().join("memory"), dir.path().to_path_buf());
        assert_eq!(store.index_content(), None);
    }

    #[test]
    fn index_content_returns_none_when_empty() {
        let dir = temp_dir();
        let memory_dir = dir.path().join("memory");
        fs::create_dir_all(&memory_dir).unwrap();
        fs::write(memory_dir.join("MEMORY.md"), "   \n\n  ").unwrap();

        let store = MemoryStore::with_dir(memory_dir, dir.path().to_path_buf());
        assert_eq!(store.index_content(), None);
    }

    #[test]
    fn index_content_truncates_at_limit() {
        let dir = temp_dir();
        let memory_dir = dir.path().join("memory");
        fs::create_dir_all(&memory_dir).unwrap();

        let lines: Vec<String> = (1..=250).map(|i| format!("- entry {i}")).collect();
        fs::write(memory_dir.join("MEMORY.md"), lines.join("\n")).unwrap();

        let store = MemoryStore::with_dir(memory_dir, dir.path().to_path_buf());
        let content = store.index_content().unwrap();
        assert!(content.contains("- entry 200"));
        assert!(!content.contains("- entry 201"));
        assert!(content.contains("[... 50 more lines truncated]"));
    }

    #[test]
    fn exists_reflects_directory_state() {
        let dir = temp_dir();
        let memory_dir = dir.path().join("memory");
        let store = MemoryStore::with_dir(memory_dir.clone(), dir.path().to_path_buf());

        assert!(!store.exists());
        fs::create_dir_all(&memory_dir).unwrap();
        assert!(store.exists());
    }

    #[test]
    fn ensure_dir_creates_directory() {
        let dir = temp_dir();
        let memory_dir = dir.path().join("deep").join("nested").join("memory");
        let store = MemoryStore::with_dir(memory_dir.clone(), dir.path().to_path_buf());

        assert!(!memory_dir.exists());
        store.ensure_dir().unwrap();
        assert!(memory_dir.is_dir());
    }

    #[test]
    fn entry_count_with_no_directory() {
        let dir = temp_dir();
        let store = MemoryStore::with_dir(dir.path().join("nonexistent"), dir.path().to_path_buf());
        assert_eq!(store.entry_count(), 0);
    }

    #[test]
    fn entry_count_excludes_index() {
        let dir = temp_dir();
        let memory_dir = dir.path().join("memory");
        fs::create_dir_all(&memory_dir).unwrap();
        fs::write(memory_dir.join("MEMORY.md"), "# Index").unwrap();
        fs::write(
            memory_dir.join("user_role.md"),
            "---\nname: user-role\ndescription: test\ntype: user\n---\ncontent",
        )
        .unwrap();
        fs::write(
            memory_dir.join("feedback_style.md"),
            "---\nname: feedback-style\ndescription: test2\ntype: feedback\n---\ncontent",
        )
        .unwrap();
        // non-md file should be ignored
        fs::write(memory_dir.join("notes.txt"), "not a memory").unwrap();

        let store = MemoryStore::with_dir(memory_dir, dir.path().to_path_buf());
        assert_eq!(store.entry_count(), 2);
    }

    #[test]
    fn list_entries_parses_frontmatter() {
        let dir = temp_dir();
        let memory_dir = dir.path().join("memory");
        fs::create_dir_all(&memory_dir).unwrap();
        fs::write(
            memory_dir.join("user_role.md"),
            "---\nname: user-role\ndescription: Senior Rust developer\ntype: user\n---\nDetails here",
        )
        .unwrap();
        fs::write(
            memory_dir.join("feedback_no_comments.md"),
            "---\nname: no-inline-comments\ndescription: Don't add comments to code\ntype: feedback\n---\nWhy: user prefers clean code",
        )
        .unwrap();

        let store = MemoryStore::with_dir(memory_dir, dir.path().to_path_buf());
        let entries = store.list_entries();
        assert_eq!(entries.len(), 2);

        // Entries sorted by filename: feedback_no_comments.md comes first
        let first = &entries[0];
        assert_eq!(first.name, "no-inline-comments");
        assert!(first.path.ends_with("feedback_no_comments.md"));
        assert_eq!(first.memory_type, MemoryType::Feedback);

        let user_entry = entries.iter().find(|e| e.name == "user-role").unwrap();
        assert_eq!(user_entry.description, "Senior Rust developer");
        assert_eq!(user_entry.memory_type, MemoryType::User);
    }

    #[test]
    fn list_entries_skips_malformed_files() {
        let dir = temp_dir();
        let memory_dir = dir.path().join("memory");
        fs::create_dir_all(&memory_dir).unwrap();

        // Valid entry
        fs::write(
            memory_dir.join("valid.md"),
            "---\nname: valid\ndescription: ok\ntype: project\n---\ncontent",
        )
        .unwrap();
        // No frontmatter
        fs::write(memory_dir.join("no_front.md"), "just content").unwrap();
        // Frontmatter but no name
        fs::write(
            memory_dir.join("no_name.md"),
            "---\ndescription: orphan\ntype: user\n---\ncontent",
        )
        .unwrap();

        let store = MemoryStore::with_dir(memory_dir, dir.path().to_path_buf());
        let entries = store.list_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "valid");
        assert_eq!(entries[0].memory_type, MemoryType::Project);
    }

    #[test]
    fn parse_frontmatter_handles_quoted_values() {
        let content =
            "---\nname: \"quoted-name\"\ndescription: 'single quoted'\ntype: reference\n---\n";
        let entry = parse_memory_frontmatter(content, PathBuf::from("test.md")).unwrap();
        assert_eq!(entry.name, "quoted-name");
        assert_eq!(entry.description, "single quoted");
        assert_eq!(entry.memory_type, MemoryType::Reference);
    }

    #[test]
    fn parse_frontmatter_unknown_type() {
        let content = "---\nname: test\ndescription: desc\ntype: weird\n---\n";
        let entry = parse_memory_frontmatter(content, PathBuf::from("test.md")).unwrap();
        assert_eq!(entry.memory_type, MemoryType::Unknown);
    }

    #[test]
    fn parse_frontmatter_missing_type_defaults_to_unknown() {
        let content = "---\nname: test\ndescription: desc\n---\n";
        let entry = parse_memory_frontmatter(content, PathBuf::from("test.md")).unwrap();
        assert_eq!(entry.memory_type, MemoryType::Unknown);
    }

    #[test]
    fn memory_type_roundtrip() {
        for mt in [
            MemoryType::User,
            MemoryType::Feedback,
            MemoryType::Project,
            MemoryType::Reference,
        ] {
            assert_eq!(MemoryType::from_str(mt.as_str()), mt);
        }
    }

    #[test]
    fn truncate_to_lines_no_op_when_under_limit() {
        let content = "line1\nline2\nline3";
        assert_eq!(truncate_to_lines(content, 10), content);
    }

    #[test]
    fn truncate_to_lines_exact_limit() {
        let content = "a\nb\nc";
        assert_eq!(truncate_to_lines(content, 3), content);
    }

    #[test]
    fn is_memory_file_filters_correctly() {
        let dir = temp_dir();
        let memory_dir = dir.path().join("memory");
        fs::create_dir_all(&memory_dir).unwrap();
        let md_file = memory_dir.join("user_role.md");
        let index_file = memory_dir.join("MEMORY.md");
        let txt_file = memory_dir.join("notes.txt");
        fs::write(&md_file, "content").unwrap();
        fs::write(&index_file, "index").unwrap();
        fs::write(&txt_file, "text").unwrap();

        assert!(is_memory_file(&md_file));
        assert!(!is_memory_file(&index_file));
        assert!(!is_memory_file(&txt_file));
        assert!(!is_memory_file(&memory_dir)); // directory, not file
    }

    #[test]
    fn list_entries_sorted_alphabetically() {
        let dir = temp_dir();
        let memory_dir = dir.path().join("memory");
        fs::create_dir_all(&memory_dir).unwrap();
        for name in ["zebra.md", "alpha.md", "middle.md"] {
            fs::write(
                memory_dir.join(name),
                format!(
                    "---\nname: {}\ndescription: d\ntype: user\n---\n",
                    name.trim_end_matches(".md")
                ),
            )
            .unwrap();
        }

        let store = MemoryStore::with_dir(memory_dir, dir.path().to_path_buf());
        let names: Vec<_> = store
            .list_entries()
            .iter()
            .map(|e| e.name.clone())
            .collect();
        assert_eq!(names, vec!["alpha", "middle", "zebra"]);
    }
}
