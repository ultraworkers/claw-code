use std::cmp::Reverse;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64;

use crate::boundary::{
    canonicalize_maybe_missing, classify_boundary, BoundaryCheck, BoundaryPolicy, PolicyOutcome,
};

/// Maximum file size that can be read (10 MB).
const MAX_READ_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum file size that can be written (10 MB).
const MAX_WRITE_SIZE: usize = 10 * 1024 * 1024;

/// Check whether a file appears to contain binary content by examining
/// the first chunk for NUL bytes.
fn is_binary_file(path: &Path) -> io::Result<bool> {
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    let mut buffer = [0u8; 8192];
    let bytes_read = file.read(&mut buffer)?;
    Ok(buffer[..bytes_read].contains(&0))
}

/// Normalize path for output by converting backslashes to forward slashes.
/// This ensures consistent path format in JSON responses across platforms.
pub fn normalize_path_for_output(path: &Path) -> String {
    dunce::simplified(path)
        .as_os_str()
        .to_string_lossy()
        .replace('\\', "/")
}

/// Normalize a path string relative to a base directory for output.
fn normalize_path_for_output_in_dir(base: &Path, rel_path: &str) -> String {
    let full = base.join(rel_path);
    normalize_path_for_output(&full)
}

/// Text payload returned by file-reading operations.
/// Content is returned by default (`full: true`); pass `full: false`
/// for a token-light payload that omits `content`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextFilePayload {
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub checksum: String,
    #[serde(rename = "bytesRead")]
    pub bytes_read: usize,
    #[serde(rename = "numLines")]
    pub num_lines: usize,
    #[serde(rename = "startLine")]
    pub start_line: usize,
    #[serde(rename = "totalLines")]
    pub total_lines: usize,
}

/// Output envelope for the `read_file` tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadFileOutput {
    #[serde(rename = "type")]
    pub kind: String,
    pub file: TextFilePayload,
}

/// Structured patch hunk emitted by write and edit operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructuredPatchHunk {
    #[serde(rename = "oldStart")]
    pub old_start: usize,
    #[serde(rename = "oldLines")]
    pub old_lines: usize,
    #[serde(rename = "newStart")]
    pub new_start: usize,
    #[serde(rename = "newLines")]
    pub new_lines: usize,
    pub lines: Vec<String>,
}

/// Syntax validation result for write/edit operations.
/// Binary or unknown types are `Skipped`; parse errors carry the error message and line.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyntaxCheck {
    Valid,
    Invalid {
        message: String,
        line: Option<usize>,
    },
    Skipped,
}

/// Output envelope for full-file write operations.
/// Includes a content preview (truncated) so the model can verify the
/// new contents without re-reading the file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriteFileOutput {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub checksum: String,
    #[serde(rename = "bytesWritten")]
    pub bytes_written: usize,
    #[serde(rename = "linesWritten")]
    pub lines_written: usize,
    /// Truncated preview of the file content *after* the write, so the
    /// model can verify the change. `None` only when the file is too
    /// large to preview. By default the preview is included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_preview: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syntax: Option<SyntaxCheck>,
}

/// Output envelope for targeted string-replacement edits.
/// Includes a content preview (truncated) so the model can verify the
/// change without re-reading the file. The full file is not echoed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditFileOutput {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "oldString")]
    pub old_string: String,
    #[serde(rename = "newString")]
    pub new_string: String,
    #[serde(rename = "newChecksum")]
    pub new_checksum: String,
    #[serde(rename = "bytesChanged")]
    pub bytes_changed: isize,
    #[serde(rename = "linesChanged")]
    pub lines_changed: usize,
    /// Number of times `old_string` matched in the file. Useful for
    /// detecting ambiguity: if > 1 and `replace_all` was not requested,
    /// the caller may have hit the wrong occurrence and should re-read
    /// the file to verify.
    #[serde(rename = "occurrencesMatched", default)]
    pub occurrences_matched: usize,
    #[serde(rename = "diffSummary")]
    pub diff_summary: String,
    /// Truncated preview of the file content *after* the edit, so the
    /// model can verify the change. `None` only when the file is too
    /// large to preview. By default the preview is included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_preview: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syntax: Option<SyntaxCheck>,
}

/// Result of a glob-based filename search.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GlobSearchOutput {
    #[serde(rename = "durationMs")]
    pub duration_ms: u128,
    #[serde(rename = "numFiles")]
    pub num_files: usize,
    pub filenames: Vec<String>,
    pub truncated: bool,
}

/// Parameters accepted by the grep-style search tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GrepSearchInput {
    pub pattern: String,
    pub path: Option<String>,
    pub glob: Option<String>,
    #[serde(rename = "output_mode")]
    pub output_mode: Option<String>,
    #[serde(rename = "-B")]
    pub before: Option<usize>,
    #[serde(rename = "-A")]
    pub after: Option<usize>,
    #[serde(rename = "-C")]
    pub context_short: Option<usize>,
    pub context: Option<usize>,
    #[serde(rename = "-n")]
    pub line_numbers: Option<bool>,
    #[serde(rename = "-i")]
    pub case_insensitive: Option<bool>,
    #[serde(rename = "type")]
    pub file_type: Option<String>,
    pub head_limit: Option<usize>,
    pub offset: Option<usize>,
    pub multiline: Option<bool>,
}

/// Result payload returned by the grep-style search tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GrepSearchOutput {
    pub mode: Option<String>,
    #[serde(rename = "numFiles")]
    pub num_files: usize,
    pub filenames: Vec<String>,
    pub content: Option<String>,
    #[serde(rename = "numLines")]
    pub num_lines: Option<usize>,
    #[serde(rename = "numMatches")]
    pub num_matches: Option<usize>,
    #[serde(rename = "appliedLimit")]
    pub applied_limit: Option<usize>,
    #[serde(rename = "appliedOffset")]
    pub applied_offset: Option<usize>,
}

/// Reads a text file and returns a line-windowed payload.
///
/// When `full` is `Some(true)` (default) the entire selected window is returned
/// in `content`; when `Some(false)`, `content` is `None` (token-light mode).
pub fn read_file(
    path: &str,
    offset: Option<usize>,
    limit: Option<usize>,
    full: Option<bool>,
) -> io::Result<ReadFileOutput> {
    let absolute_path = normalize_path(path)?;

    // Check file size before reading
    let metadata = fs::metadata(&absolute_path)?;
    if metadata.len() > MAX_READ_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "file is too large ({} bytes, max {} bytes)",
                metadata.len(),
                MAX_READ_SIZE
            ),
        ));
    }

    // Detect binary files
    if is_binary_file(&absolute_path)? {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file appears to be binary",
        ));
    }

    let content = fs::read_to_string(&absolute_path)?;
    let checksum = format!("{:016x}", xxh3_64(content.as_bytes()));
    let lines: Vec<&str> = content.lines().collect();
    let start_index = offset.unwrap_or(0).min(lines.len());
    let end_index = limit.map_or(lines.len(), |limit| {
        start_index.saturating_add(limit).min(lines.len())
    });
    let selected = lines[start_index..end_index].join("\n");
    let bytes_read = selected.len();

    let content = if full == Some(false) {
        None
    } else {
        Some(selected)
    };

    Ok(ReadFileOutput {
        kind: String::from("text"),
        file: TextFilePayload {
            file_path: normalize_path_for_output(&absolute_path),
            content,
            checksum,
            bytes_read,
            num_lines: end_index.saturating_sub(start_index),
            start_line: start_index.saturating_add(1),
            total_lines: lines.len(),
        },
    })
}

/// Maximum bytes for an echoed `content_preview` on write/edit results.
/// 2 KiB keeps the tool_result envelope small while still giving the
/// model enough text to verify a single targeted change.
const CONTENT_PREVIEW_MAX: usize = 2_048;

/// Files larger than this threshold skip the `content_preview` in
/// `new_file` output because the full content already exists in the
/// `ToolUse` input.  For files ≤ this size the preview is included
/// so the model can verify without re-reading.
const CONTENT_PREVIEW_SKIP_THRESHOLD: usize = 512;

/// Returns a truncated preview of the given content, or `None` when
/// the content is empty. The preview is wrapped in [`CONTENT_PREVIEW_MAX`]
/// bytes; truncation is indicated by a trailing marker so the model
/// knows the echo was clipped.
fn preview_for(content: &str) -> Option<String> {
    if content.is_empty() {
        return None;
    }
    if content.len() <= CONTENT_PREVIEW_MAX {
        return Some(content.to_owned());
    }
    let mut end = CONTENT_PREVIEW_MAX;
    while end > 0 && !content.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = String::with_capacity(end + 64);
    out.push_str(&content[..end]);
    out.push_str("\n…[truncated, full content written to file]");
    Some(out)
}

/// Creates a new file and returns metadata plus a truncated content preview.
/// When `force` is false (default), fails if the file already exists — use `edit_file` to modify.
/// When `force` is true, overwrites the existing file entirely.
pub fn new_file(path: &str, content: &str, force: bool) -> io::Result<WriteFileOutput> {
    if content.len() > MAX_WRITE_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "content is too large ({} bytes, max {} bytes)",
                content.len(),
                MAX_WRITE_SIZE
            ),
        ));
    }

    let absolute_path = normalize_path_allow_missing(path)?;

    if absolute_path.exists() && absolute_path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "path '{}' is a directory, cannot create file",
                absolute_path.display()
            ),
        ));
    }

    let is_existing = absolute_path.exists();

    if is_existing && !force {
        let existing = fs::read_to_string(&absolute_path).unwrap_or_default();
        let line_count = existing.lines().count();
        let byte_count = existing.len();
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "File already exists at '{}' ({} lines, {} bytes). \
                 Use `edit_file` to modify existing files, \
                 or set `force: true` to overwrite entirely.",
                absolute_path.display(),
                line_count,
                byte_count
            ),
        ));
    }

    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if is_existing {
        // Overwrite mode: truncate + write
        fs::write(&absolute_path, content)?;
    } else {
        // Atomic create: fails if file was created between our exists() check and now.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&absolute_path)?;
        file.write_all(content.as_bytes())?;
    }

    let checksum = format!("{:016x}", xxh3_64(content.as_bytes()));
    let bytes_written = content.len();
    let lines_written = if content.is_empty() {
        0
    } else {
        content.lines().count()
    };

    Ok(WriteFileOutput {
        kind: if is_existing {
            String::from("overwrite")
        } else {
            String::from("create")
        },
        file_path: normalize_path_for_output(&absolute_path),
        checksum,
        bytes_written,
        lines_written,
        content_preview: if content.len() <= CONTENT_PREVIEW_SKIP_THRESHOLD {
            preview_for(content)
        } else {
            None
        },
        syntax: Some(validate_syntax(&absolute_path, content)),
    })
}

/// Performs an in-file string replacement and returns metadata plus a
/// truncated content preview so the model can verify the change.
pub fn edit_file(
    path: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
    expected_checksum: Option<&str>,
) -> io::Result<EditFileOutput> {
    let absolute_path = normalize_path(path)?;
    let original_content_raw = fs::read_to_string(&absolute_path)?;

    if let Some(expected) = expected_checksum {
        let actual = format!("{:016x}", xxh3_64(original_content_raw.as_bytes()));
        if actual != expected {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("expected checksum {expected} but current file checksum is {actual}"),
            ));
        }
    }

    // Normalize CRLF → LF so matching is consistent with read_file output
    // which strips \r via .lines().join("\n").
    let original_content = original_content_raw.replace("\r\n", "\n");

    if old_string == new_string {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "old_string and new_string must differ",
        ));
    }
    if !original_content.contains(old_string) {
        let line_count = original_content.lines().count();
        let tail: Vec<&str> = original_content
            .lines()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let tail_preview = tail.join("\n");
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "old_string not found in file ({} lines). \
                 The file may have been modified since you last read it. \
                 Last 5 lines of the file:
---
{}
---
\
                 Please call read_file to see the current content before retrying.",
                line_count, tail_preview
            ),
        ));
    }

    let occurrences_matched = original_content.matches(old_string).count();

    let new_content = if replace_all {
        original_content.replace(old_string, new_string)
    } else {
        original_content.replacen(old_string, new_string, 1)
    };
    fs::write(&absolute_path, &new_content)?;

    let new_checksum = format!("{:016x}", xxh3_64(new_content.as_bytes()));
    let bytes_changed = new_content.len() as isize - original_content.len() as isize;

    let patch = make_patch(&original_content, &new_content);
    let lines_changed: usize = patch.iter().map(|h| h.lines.len()).sum();

    let diff_summary = if serde_json::to_string(&patch).map_or(true, |s| s.len() > 2048) {
        serde_json::json!({
            "truncated": true,
            "hunks_count": patch.len(),
            "first_hunk_range": patch.first().map(|h| {
                format!("@@ -{},{} +{},{} @@", h.old_start, h.old_lines, h.new_start, h.new_lines)
            }).unwrap_or_default(),
            "total_lines_changed": lines_changed,
        })
        .to_string()
    } else {
        serde_json::to_string(&patch).unwrap_or_default()
    };

    Ok(EditFileOutput {
        kind: String::from("edit"),
        file_path: normalize_path_for_output(&absolute_path),
        old_string: old_string.to_owned(),
        new_string: new_string.to_owned(),
        new_checksum,
        bytes_changed,
        lines_changed,
        occurrences_matched,
        diff_summary,
        content_preview: preview_for(&new_content),
        syntax: Some(validate_syntax(&absolute_path, &new_content)),
    })
}

/// Expands a glob pattern and returns matching filenames.
pub fn glob_search(pattern: &str, path: Option<&str>) -> io::Result<GlobSearchOutput> {
    let started = Instant::now();
    let base_dir = path
        .map(normalize_path)
        .transpose()?
        .unwrap_or(std::env::current_dir()?);

    // `fd` is used only to enumerate files. Its `--glob` flag matches file
    // basenames, not relative paths, which breaks patterns containing a path
    // separator such as `nested/*.rs` (fd 10 on Windows). Matching is done in
    // Rust with the `glob` crate against the relative path from the search
    // root, so path-style patterns behave consistently on every platform.
    let mut cmd = std::process::Command::new("fd");
    cmd.arg("--type").arg("f")
       .arg("--hidden").arg("--no-ignore")
       .current_dir(&base_dir);

    let output = cmd.output().map_err(|e| {
        io::Error::new(io::ErrorKind::NotFound, format!("fd not found: {e}"))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(io::ErrorKind::InvalidInput, stderr.to_string()));
    }

    // Rust-side glob matching against the relative path. Normalise Windows
    // separators to `/` so `nested/*.rs` works regardless of platform.
    // The `glob` crate (0.3) does not support `{a,b}` brace alternation, so
    // expand braces into a list of alternative patterns and match any of them.
    let glob_expression = pattern.replace('\\', "/");
    let matchers = expand_brace_patterns(&glob_expression).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("invalid glob pattern: {e}"))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut matches: Vec<PathBuf> = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let rel = line.replace('\\', "/");
            let matched = matchers.iter().any(|p| p.matches(&rel));
            matched.then(|| base_dir.join(line))
        })
        .collect();

    matches.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
            .map(Reverse)
    });

    let truncated = matches.len() > 100;
    let filenames = matches
        .into_iter()
        .take(100)
        .map(|path| normalize_path_for_output(&path))
        .collect::<Vec<_>>();

    Ok(GlobSearchOutput {
        duration_ms: started.elapsed().as_millis(),
        num_files: filenames.len(),
        filenames,
        truncated,
    })
}

/// Expand `{a,b}` brace alternations in a glob pattern into a list of
/// alternative patterns. The `glob` crate (0.3) does not support brace
/// syntax natively, so `*.{rs,toml}` is expanded to `["*.rs", "*.toml"]`.
/// Nested and empty braces are not supported; a malformed brace sequence is
/// left as-is (matching fd's lenient behaviour).
fn expand_brace_patterns(pattern: &str) -> io::Result<Vec<glob::Pattern>> {
    // Split at the first brace pair, expand it, and recurse on the suffix so
    // multiple `{...}` groups (e.g. `a{b,c}d{e,f}`) all expand correctly.
    let Some(open) = pattern.find('{') else {
        let single = glob::Pattern::new(pattern).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("invalid glob pattern `{pattern}`: {e}"))
        })?;
        return Ok(vec![single]);
    };
    let Some(close_rel) = pattern[open..].find('}') else {
        // Unclosed brace — treat as literal.
        let single = glob::Pattern::new(pattern).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("invalid glob pattern `{pattern}`: {e}"))
        })?;
        return Ok(vec![single]);
    };
    let close = open + close_rel;
    let prefix = &pattern[..open];
    let body = &pattern[open + 1..close];
    let suffix = &pattern[close + 1..];

    let choices: Vec<&str> = body.split(',').filter(|c| !c.is_empty()).collect();
    if choices.is_empty() {
        // `{}` — treat as literal braces.
        let single = glob::Pattern::new(pattern).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("invalid glob pattern `{pattern}`: {e}"))
        })?;
        return Ok(vec![single]);
    }

    let mut patterns = Vec::new();
    for choice in choices {
        let combined = format!("{prefix}{choice}{suffix}");
        patterns.extend(expand_brace_patterns(&combined)?);
    }
    Ok(patterns)
}

/// Runs a regex search over workspace files with optional context lines.
pub fn grep_search(input: &GrepSearchInput) -> io::Result<GrepSearchOutput> {
    let base_path = input
        .path
        .as_deref()
        .map(normalize_path)
        .transpose()?
        .unwrap_or(std::env::current_dir()?);

    let output_mode = input
        .output_mode
        .clone()
        .unwrap_or_else(|| String::from("files_with_matches"));
    let context = input.context.or(input.context_short).unwrap_or(0);

    let mut cmd = std::process::Command::new("rg");
    cmd.arg("--no-heading")
       .arg("--color").arg("never")
       .arg("--line-number")
       // Force rg to always print the file path prefix. When rg searches a
       // single file argument it omits the path by default, which breaks the
       // `path:...` parsing in the count/content modes below (a bare `2` or
       // `1:text` line would be silently dropped, reporting 0 matches).
       .arg("--with-filename");

    if input.case_insensitive.unwrap_or(false) {
        cmd.arg("--ignore-case");
    }
    if input.multiline.unwrap_or(false) {
        cmd.arg("--multiline");
    }

    match output_mode.as_str() {
        "count" => { cmd.arg("--count-matches"); }
        "content" => {
            let before = input.before.unwrap_or(context);
            let after = input.after.unwrap_or(context);
            if before > 0 || after > 0 {
                cmd.arg("-B").arg(before.to_string())
                   .arg("-A").arg(after.to_string());
            }
        }
        _ => { cmd.arg("--files-with-matches"); }
    }

    if let Some(ref glob_pat) = input.glob {
        cmd.arg("--glob").arg(glob_pat);
    }
    if let Some(ref file_type) = input.file_type {
        cmd.arg("--type").arg(file_type);
    }

    cmd.arg("--").arg(&input.pattern);

    if base_path.is_file() {
        cmd.arg(&base_path);
    } else {
        cmd.current_dir(&base_path);
    }

    let output = cmd.output().map_err(|e| {
        io::Error::new(io::ErrorKind::NotFound, format!("rg not found: {e}"))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() && output.status.code() != Some(1) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, stderr.to_string()));
    }

    let offset = input.offset.unwrap_or(0);
    let head_limit = input.head_limit;

    match output_mode.as_str() {
        "count" => {
            let mut filenames = Vec::new();
            let mut total_matches = 0usize;
            for line in stdout.lines() {
                if let Some((path, count_str)) = line.rsplit_once(':') {
                    if let Ok(count) = count_str.parse::<usize>() {
                        total_matches += count;
                        filenames.push(normalize_path_for_output_in_dir(&base_path, path));
                    }
                }
            }
            let (filenames, applied_limit, applied_offset) =
                apply_limit(filenames, head_limit, Some(offset));
            Ok(GrepSearchOutput {
                mode: Some(output_mode),
                num_files: filenames.len(),
                filenames,
                content: None,
                num_lines: None,
                num_matches: Some(total_matches),
                applied_limit,
                applied_offset: applied_offset,
            })
        }
        "content" => {
            let mut content_lines = Vec::new();
            let mut filenames_set = std::collections::HashSet::new();
            for line in stdout.lines() {
                if line.is_empty() { continue; }
                if let Some(path) = line.split(':').next() {
                    filenames_set.insert(normalize_path_for_output_in_dir(&base_path, path));
                }
                content_lines.push(line.to_string());
            }
            let (content_lines, applied_limit, applied_offset) =
                apply_limit(content_lines, head_limit, Some(offset));
            let filenames: Vec<String> = filenames_set.into_iter().collect();
            Ok(GrepSearchOutput {
                mode: Some(output_mode),
                num_files: filenames.len(),
                filenames,
                content: Some(content_lines.join("\n")),
                num_lines: Some(content_lines.len()),
                num_matches: None,
                applied_limit,
                applied_offset: applied_offset,
            })
        }
        _ => {
            let mut filenames: Vec<String> = stdout
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| normalize_path_for_output_in_dir(&base_path, l))
                .collect();
            filenames.sort();
            filenames.dedup();
            let (filenames, applied_limit, applied_offset) =
                apply_limit(filenames, head_limit, Some(offset));
            Ok(GrepSearchOutput {
                mode: Some(output_mode),
                num_files: filenames.len(),
                filenames,
                content: None,
                num_lines: None,
                num_matches: None,
                applied_limit,
                applied_offset: applied_offset,
             })
        }
    }
}

fn apply_limit<T>(
    items: Vec<T>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> (Vec<T>, Option<usize>, Option<usize>) {
    let offset_value = offset.unwrap_or(0);
    let mut items = items.into_iter().skip(offset_value).collect::<Vec<_>>();
    let explicit_limit = limit.unwrap_or(250);
    if explicit_limit == 0 {
        return (items, None, (offset_value > 0).then_some(offset_value));
    }

    let truncated = items.len() > explicit_limit;
    items.truncate(explicit_limit);
    (
        items,
        truncated.then_some(explicit_limit),
        (offset_value > 0).then_some(offset_value),
    )
}

fn make_patch(original: &str, updated: &str) -> Vec<StructuredPatchHunk> {
    let mut lines = Vec::new();
    for line in original.lines() {
        lines.push(format!("-{line}"));
    }
    for line in updated.lines() {
        lines.push(format!("+{line}"));
    }

    vec![StructuredPatchHunk {
        old_start: 1,
        old_lines: original.lines().count(),
        new_start: 1,
        new_lines: updated.lines().count(),
        lines,
    }]
}

/// Expand environment variables in a path string.
/// Supports `%VAR%` (Windows) and `${VAR}` (Unix) syntax.
/// Non-existent variables are left as-is.
fn expand_env_vars(path: &str) -> String {
    let mut result = String::with_capacity(path.len() + 64);
    let mut chars = path.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch == '%' {
            let mut var_name = String::new();
            let mut closed = false;
            while let Some((_, c)) = chars.next() {
                if c == '%' {
                    closed = true;
                    break;
                }
                var_name.push(c);
            }
            if closed {
                if let Ok(val) = std::env::var(&var_name) {
                    result.push_str(&val);
                } else {
                    result.push('%');
                    result.push_str(&var_name);
                    result.push('%');
                }
            } else {
                result.push('%');
                result.push_str(&var_name);
            }
        } else if ch == '$' && chars.peek().is_some_and(|(_, c)| *c == '{') {
            chars.next();
            let mut var_name = String::new();
            let mut closed = false;
            while let Some((_, c)) = chars.next() {
                if c == '}' {
                    closed = true;
                    break;
                }
                var_name.push(c);
            }
            if closed {
                if let Ok(val) = std::env::var(&var_name) {
                    result.push_str(&val);
                } else {
                    result.push('$');
                    result.push('{');
                    result.push_str(&var_name);
                    result.push('}');
                }
            } else {
                result.push('$');
                result.push('{');
                result.push_str(&var_name);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn normalize_path_resolve(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut result = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    if result.as_os_str().is_empty() && !path.as_os_str().is_empty() {
        result.push(".");
    }
    result
}

fn normalize_path(path: &str) -> io::Result<PathBuf> {
    let expanded = expand_env_vars(strip_file_url(path));
    let candidate = if Path::new(&expanded).is_absolute() {
        PathBuf::from(&expanded)
    } else {
        std::env::current_dir()?.join(&expanded)
    };
    let cleaned = dunce::simplified(&candidate).to_path_buf();
    if !cleaned.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("file not found: {}", candidate.display()),
        ));
    }
    Ok(normalize_path_resolve(&cleaned))
}

fn normalize_path_allow_missing(path: &str) -> io::Result<PathBuf> {
    let expanded = expand_env_vars(strip_file_url(path));
    let candidate = if Path::new(&expanded).is_absolute() {
        PathBuf::from(&expanded)
    } else {
        std::env::current_dir()?.join(&expanded)
    };
    let cleaned = dunce::simplified(&candidate).to_path_buf();
    Ok(normalize_path_resolve(&cleaned))
}

fn strip_file_url(path: &str) -> &str {
    let Some(rest) = path.strip_prefix("file://") else {
        return path;
    };
    if rest.is_empty() || !rest.starts_with('/') {
        return rest;
    }
    let bytes = rest.as_bytes();
    if bytes.len() >= 3 && bytes[1].is_ascii_alphabetic() && bytes[2] == b':' {
        &rest[1..]
    } else {
        rest
    }
}

/// Read a file with workspace boundary enforcement that consults a
/// `BoundaryPolicy` on out-of-workspace paths. When the path is
/// inside the workspace, behavior is identical to
/// `read_file_in_workspace`. When the path escapes the workspace, the
/// policy decides: `Block` denies, `Allow` permits silently, and
/// `Prompt` asks the human.
#[allow(dead_code)]
pub fn read_file_with_policy(
    path: &str,
    offset: Option<usize>,
    limit: Option<usize>,
    workspace_root: &Path,
    policy: &BoundaryPolicy,
    full: Option<bool>,
) -> io::Result<ReadFileOutput> {
    let absolute_path = normalize_path(path)?;
    let canonical_root = dunce::simplified(
        &workspace_root
            .canonicalize()
            .unwrap_or_else(|_| workspace_root.to_path_buf()),
    )
    .to_path_buf();
    let canonical_path = canonicalize_maybe_missing(&absolute_path);
    let check = classify_boundary(&canonical_path, &canonical_root);
    if matches!(check, BoundaryCheck::OutOfWorkspace { .. }) {
        match policy.enforce_outside(&canonical_path, &canonical_root) {
            PolicyOutcome::Proceed | PolicyOutcome::Approved { .. } => {}
            PolicyOutcome::Denied(msg) => {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, msg));
            }
        }
    }
    // `full` flows through: callers that want the default LLM-friendly
    // echo pass `None` (or `Some(true)`); callers that need the
    // legacy token-light payload pass `Some(false)`. A prior version
    // hardcoded `None` here, which silently ignored `full: false`
    // and always echoed the content.
    read_file(
        canonical_path.to_string_lossy().as_ref(),
        offset,
        limit,
        full,
    )
}

/// Write a file with workspace boundary enforcement that consults a
/// `BoundaryPolicy` on out-of-workspace paths. See
/// `read_file_with_policy` for the policy contract.
#[allow(dead_code)]
pub fn new_file_with_policy(
    path: &str,
    content: &str,
    force: bool,
    workspace_root: &Path,
    policy: &BoundaryPolicy,
) -> io::Result<WriteFileOutput> {
    let absolute_path = normalize_path_allow_missing(path)?;
    let canonical_root = dunce::simplified(
        &workspace_root
            .canonicalize()
            .unwrap_or_else(|_| workspace_root.to_path_buf()),
    )
    .to_path_buf();
    let canonical_path = canonicalize_maybe_missing(&absolute_path);
    let check = classify_boundary(&canonical_path, &canonical_root);
    if matches!(check, BoundaryCheck::OutOfWorkspace { .. }) {
        match policy.enforce_outside(&canonical_path, &canonical_root) {
            PolicyOutcome::Proceed | PolicyOutcome::Approved { .. } => {}
            PolicyOutcome::Denied(msg) => {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, msg));
            }
        }
    }
    new_file(canonical_path.to_string_lossy().as_ref(), content, force)
}

/// Edit a file with workspace boundary enforcement that consults a
/// `BoundaryPolicy` on out-of-workspace paths. See
/// `read_file_with_policy` for the policy contract.
#[allow(dead_code)]
pub fn edit_file_with_policy(
    path: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
    expected_checksum: Option<&str>,
    workspace_root: &Path,
    policy: &BoundaryPolicy,
) -> io::Result<EditFileOutput> {
    let absolute_path = normalize_path(path)?;
    let canonical_root = dunce::simplified(
        &workspace_root
            .canonicalize()
            .unwrap_or_else(|_| workspace_root.to_path_buf()),
    )
    .to_path_buf();
    let canonical_path = canonicalize_maybe_missing(&absolute_path);
    let check = classify_boundary(&canonical_path, &canonical_root);
    if matches!(check, BoundaryCheck::OutOfWorkspace { .. }) {
        match policy.enforce_outside(&canonical_path, &canonical_root) {
            PolicyOutcome::Proceed | PolicyOutcome::Approved { .. } => {}
            PolicyOutcome::Denied(msg) => {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, msg));
            }
        }
    }
    edit_file(
        canonical_path.to_string_lossy().as_ref(),
        old_string,
        new_string,
        replace_all,
        expected_checksum,
    )
}

/// Expands a glob pattern with workspace boundary enforcement.
/// Filters out any matching files that escape the workspace root.
// Not yet wired through the tool dispatch chain; see ROADMAP for the
// BoundaryPolicy threading work that will connect this.
#[allow(dead_code)]
pub fn glob_search_with_policy(
    pattern: &str,
    path: Option<&str>,
    workspace_root: &Path,
    policy: &BoundaryPolicy,
) -> io::Result<GlobSearchOutput> {
    let result = glob_search(pattern, path)?;
    let canonical_root = canonicalize_maybe_missing(workspace_root);
    let filtered: Vec<String> = result
        .filenames
        .into_iter()
        .filter(|f| {
            let check = classify_boundary(Path::new(f), &canonical_root);
            if matches!(check, BoundaryCheck::InWorkspace) {
                return true;
            }
            matches!(
                policy.enforce_outside(Path::new(f), &canonical_root),
                PolicyOutcome::Proceed | PolicyOutcome::Approved { .. }
            )
        })
        .collect();
    let num_files = filtered.len();
    let truncated = num_files > 100;
    Ok(GlobSearchOutput {
        num_files,
        filenames: filtered.into_iter().take(100).collect(),
        truncated,
        ..result
    })
}

/// Runs a regex search with workspace boundary enforcement.
/// Only searches files that pass the workspace boundary check.
// Not yet wired through the tool dispatch chain; see ROADMAP for the
// BoundaryPolicy threading work that will connect this.
#[allow(dead_code)]
pub fn grep_search_with_policy(
    input: &GrepSearchInput,
    workspace_root: &Path,
    policy: &BoundaryPolicy,
) -> io::Result<GrepSearchOutput> {
    let canonical_root = canonicalize_maybe_missing(workspace_root);
    let base_path = input
        .path
        .as_deref()
        .map(normalize_path)
        .transpose()?
        .unwrap_or(std::env::current_dir()?);

    if matches!(
        classify_boundary(&base_path, &canonical_root),
        BoundaryCheck::OutOfWorkspace { .. }
    ) && matches!(
        policy.enforce_outside(&base_path, &canonical_root),
        PolicyOutcome::Denied(_)
    ) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "search root {} escapes workspace boundary",
                base_path.display()
            ),
        ));
    }

    let input_with_filter = GrepSearchInput {
        path: Some(base_path.to_string_lossy().into_owned()),
        ..input.clone()
    };
    grep_search(&input_with_filter)
}

/// Validate file syntax based on extension.
/// Returns `Valid` for well-formed JSON/TOML, `Invalid(reason)` for parse errors,
/// or `Skipped` for unsupported or binary file types.
fn validate_syntax(path: &Path, content: &str) -> SyntaxCheck {
    match path.extension().and_then(|e| e.to_str()) {
        Some("json") => match serde_json::from_str::<serde_json::Value>(content) {
            Ok(_) => SyntaxCheck::Valid,
            Err(e) => SyntaxCheck::Invalid {
                message: e.to_string(),
                line: Some(e.line()),
            },
        },
        Some("toml") => match toml::from_str::<toml::Value>(content) {
            Ok(_) => SyntaxCheck::Valid,
            Err(e) => SyntaxCheck::Invalid {
                message: e.to_string(),
                line: None,
            },
        },
        _ => SyntaxCheck::Skipped,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        edit_file, glob_search, grep_search, new_file, new_file_with_policy,
        preview_for, read_file, read_file_with_policy, GrepSearchInput, MAX_WRITE_SIZE,
    };
    use crate::boundary::{BoundaryDecision, BoundaryPolicy, Prompter, PrompterError};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        std::env::temp_dir().join(format!("clawd-native-{name}-{unique}"))
    }

    #[test]
    fn reads_and_writes_files() {
        let path = temp_path("read-write.txt");
        let write_output = new_file(path.to_string_lossy().as_ref(), "one\ntwo\nthree", false)
            .expect("write should succeed");
        assert_eq!(write_output.kind, "create");

        let read_output = read_file(
            path.to_string_lossy().as_ref(),
            Some(1),
            Some(1),
            Some(true),
        )
        .expect("read should succeed");
        assert_eq!(read_output.file.content, Some("two".to_string()));
    }

    #[test]
    fn rejects_binary_files() {
        let path = temp_path("binary-test.bin");
        std::fs::write(&path, b"\x00\x01\x02\x03binary content").expect("write should succeed");
        let result = read_file(path.to_string_lossy().as_ref(), None, None, None);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("binary"));
    }

    #[test]
    fn rejects_oversized_writes() {
        let path = temp_path("oversize-write.txt");
        let huge = "x".repeat(MAX_WRITE_SIZE + 1);
        let result = new_file(path.to_string_lossy().as_ref(), &huge, false);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("too large"));
    }

    #[test]
    fn globs_and_greps_directory() {
        let dir = temp_path("search-dir");
        std::fs::create_dir_all(&dir).expect("directory should be created");
        let file = dir.join("demo.rs");
        new_file(
            file.to_string_lossy().as_ref(),
            "fn main() {\n println!(\"hello\");\n}\n",
            false,
        )
        .expect("file write should succeed");

        let globbed = glob_search("**/*.rs", Some(dir.to_string_lossy().as_ref()))
            .expect("glob should succeed");
        assert_eq!(globbed.num_files, 1);

        let grep_output = grep_search(&GrepSearchInput {
            pattern: String::from("hello"),
            path: Some(dir.to_string_lossy().into_owned()),
            glob: Some(String::from("**/*.rs")),
            output_mode: Some(String::from("content")),
            before: None,
            after: None,
            context_short: None,
            context: None,
            line_numbers: Some(true),
            case_insensitive: Some(false),
            file_type: None,
            head_limit: Some(10),
            offset: Some(0),
            multiline: Some(false),
        })
        .expect("grep should succeed");
        assert!(grep_output.content.unwrap_or_default().contains("hello"));
    }

    #[test]
    fn glob_search_with_braces_finds_files() {
        let dir = temp_path("glob-braces");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.join("b.toml"), "[package]").unwrap();
        std::fs::write(dir.join("c.txt"), "hello").unwrap();

        let result =
            glob_search("*.{rs,toml}", Some(dir.to_str().unwrap())).expect("glob should succeed");
        assert_eq!(
            result.num_files, 2,
            "should match .rs and .toml but not .txt"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Test-only scripted prompter mirroring the one in
    /// `boundary::tests::ScriptedPrompter`. We keep a local copy so
    /// `file_ops` tests do not depend on `boundary::tests`.
    struct ScriptedPrompter {
        decisions: Mutex<VecDeque<Result<BoundaryDecision, PrompterError>>>,
    }

    impl ScriptedPrompter {
        fn new(decisions: Vec<BoundaryDecision>) -> Self {
            Self {
                decisions: Mutex::new(decisions.into_iter().map(Ok).collect()),
            }
        }
    }

    impl Prompter for ScriptedPrompter {
        fn ask(
            &self,
            _path: &std::path::Path,
            _workspace: &std::path::Path,
        ) -> Result<BoundaryDecision, PrompterError> {
            self.decisions
                .lock()
                .expect("scripted prompter mutex poisoned")
                .pop_front()
                .unwrap_or(Err(PrompterError::NoTty))
        }
    }

    fn outside_workspace_setup(label: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let workspace = temp_path(&format!("policy-ws-{label}"));
        let outside = temp_path(&format!("policy-out-{label}"));
        std::fs::create_dir_all(&workspace).expect("create workspace");
        std::fs::create_dir_all(&outside).expect("create outside");
        (workspace, outside)
    }

    #[test]
    fn read_file_with_policy_block_denies_outside_workspace() {
        let (workspace, outside) = outside_workspace_setup("block-read");
        let file = outside.join("data.txt");
        new_file(file.to_string_lossy().as_ref(), "secret", false).expect("write outside");
        let result = read_file_with_policy(
            file.to_string_lossy().as_ref(),
            None,
            None,
            &workspace,
            &BoundaryPolicy::Block,
            None,
        );
        let err = result.expect_err("block policy must reject");
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(err.to_string().contains("escapes workspace"));
        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn read_file_with_policy_allow_permits_outside_workspace() {
        let (workspace, outside) = outside_workspace_setup("allow-read");
        let file = outside.join("data.txt");
        new_file(file.to_string_lossy().as_ref(), "ok", false).expect("write outside");
        let result = read_file_with_policy(
            file.to_string_lossy().as_ref(),
            None,
            None,
            &workspace,
            &BoundaryPolicy::Allow,
            None,
        );
        // The read should succeed; the policy admitted the access.
        let payload = result.expect("allow policy must permit");
        // Checksum is set even when content is not echoed.
        assert!(!payload.file.checksum.is_empty());
        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn read_file_with_policy_prompt_allow_once_returns_file() {
        let (workspace, outside) = outside_workspace_setup("prompt-once");
        let file = outside.join("data.txt");
        new_file(file.to_string_lossy().as_ref(), "once", false).expect("write outside");
        let prompter = Arc::new(ScriptedPrompter::new(vec![BoundaryDecision::AllowOnce]));
        let session = Arc::new(Mutex::new(BTreeSet::<crate::boundary::ApprovedRoot>::new()));
        let policy = BoundaryPolicy::Prompt {
            prompter: prompter.clone(),
            session_approved: session.clone(),
            user_typed: Arc::new(Mutex::new(BTreeSet::<crate::boundary::ApprovedRoot>::new())),
        };
        let result = read_file_with_policy(
            file.to_string_lossy().as_ref(),
            None,
            None,
            &workspace,
            &policy,
            None,
        );
        let payload = result.expect("AllowOnce should admit the read");
        assert!(!payload.file.checksum.is_empty());
        assert!(session.lock().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn read_file_with_policy_prompt_deny_blocks_with_user_facing_error() {
        let (workspace, outside) = outside_workspace_setup("prompt-deny");
        let file = outside.join("data.txt");
        new_file(file.to_string_lossy().as_ref(), "secret", false).expect("write outside");
        let prompter = Arc::new(ScriptedPrompter::new(vec![BoundaryDecision::Deny]));
        let session = Arc::new(Mutex::new(BTreeSet::<crate::boundary::ApprovedRoot>::new()));
        let policy = BoundaryPolicy::Prompt {
            prompter: prompter.clone(),
            session_approved: session.clone(),
            user_typed: Arc::new(Mutex::new(BTreeSet::<crate::boundary::ApprovedRoot>::new())),
        };
        let result = read_file_with_policy(
            file.to_string_lossy().as_ref(),
            None,
            None,
            &workspace,
            &policy,
            None,
        );
        let err = result.expect_err("Deny must reject");
        assert!(err.to_string().contains("user denied access"));
        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn read_file_with_policy_prompt_allow_session_skips_second_prompt() {
        let (workspace, outside) = outside_workspace_setup("prompt-sess");
        let file = outside.join("data.txt");
        new_file(file.to_string_lossy().as_ref(), "sess", false).expect("write outside");
        let prompter = Arc::new(ScriptedPrompter::new(vec![BoundaryDecision::AllowAlways]));
        let session = Arc::new(Mutex::new(BTreeSet::<crate::boundary::ApprovedRoot>::new()));
        let policy = BoundaryPolicy::Prompt {
            prompter: prompter.clone(),
            session_approved: session.clone(),
            user_typed: Arc::new(Mutex::new(BTreeSet::<crate::boundary::ApprovedRoot>::new())),
        };
        let _ = read_file_with_policy(
            file.to_string_lossy().as_ref(),
            None,
            None,
            &workspace,
            &policy,
            None,
        )
        .expect("first read should succeed");
        // The scripted prompter is now empty; a second read would
        // surface a `NoTty` error if it were invoked.
        let payload = read_file_with_policy(
            file.to_string_lossy().as_ref(),
            None,
            None,
            &workspace,
            &policy,
            None,
        )
        .expect("second read must not re-prompt");
        assert!(!payload.file.checksum.is_empty());
        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn read_file_with_policy_in_workspace_skips_policy_check() {
        let (workspace, _outside) = outside_workspace_setup("in-ws");
        let inside = workspace.join("in.txt");
        new_file(inside.to_string_lossy().as_ref(), "inside", false).expect("write inside");
        // Even with Block policy, an in-workspace path proceeds
        // without consulting the prompter.
        let prompter = Arc::new(ScriptedPrompter::new(vec![]));
        let session = Arc::new(Mutex::new(BTreeSet::<crate::boundary::ApprovedRoot>::new()));
        let policy = BoundaryPolicy::Prompt {
            prompter: prompter.clone(),
            session_approved: session.clone(),
            user_typed: Arc::new(Mutex::new(BTreeSet::<crate::boundary::ApprovedRoot>::new())),
        };
        let result = read_file_with_policy(
            inside.to_string_lossy().as_ref(),
            None,
            None,
            &workspace,
            &policy,
            None,
        );
        let payload = result.expect("in-workspace read should succeed");
        assert!(!payload.file.checksum.is_empty());
        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    fn new_file_with_policy_strict_denies_outside_workspace() {
        let (workspace, outside) = outside_workspace_setup("write-strict");
        let target = outside.join("new.txt");
        let result = new_file_with_policy(
            target.to_string_lossy().as_ref(),
            "x",
            false,
            &workspace,
            &BoundaryPolicy::Block,
        );
        let err = result.expect_err("strict policy must reject");
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(err.to_string().contains("escapes workspace"));
        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn new_file_with_policy_allow_writes_to_outside_workspace() {
        let (workspace, outside) = outside_workspace_setup("write-allow");
        let target = outside.join("new.txt");
        let result = new_file_with_policy(
            target.to_string_lossy().as_ref(),
            "ok",
            false,
            &workspace,
            &BoundaryPolicy::Allow,
        );
        let payload = result.expect("allow policy must permit write");
        assert!(target.exists(), "file should be created");
        let written = std::fs::read_to_string(&target).expect("read back");
        assert_eq!(written, "ok");
        assert!(payload.bytes_written > 0);
        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn read_file_with_policy_respects_full_false_opt_out() {
        // Regression: `full: false` must propagate through the
        // policy wrapper. A prior version hardcoded `None` here,
        // which silently echoed content even when the caller asked
        // for a token-light payload.
        let workspace = temp_path("full-false-workspace");
        std::fs::create_dir_all(&workspace).expect("workspace dir should be created");
        let inside = workspace.join("echo.txt");
        new_file(inside.to_string_lossy().as_ref(), "echo this", false)
            .expect("write should succeed");
        let payload_tokenlight = read_file_with_policy(
            inside.to_string_lossy().as_ref(),
            None,
            None,
            &workspace,
            &BoundaryPolicy::Allow,
            Some(false),
        )
        .expect("token-light read should succeed");
        assert!(
            payload_tokenlight.file.content.is_none(),
            "full=false must suppress the content echo"
        );
        let payload_echo = read_file_with_policy(
            inside.to_string_lossy().as_ref(),
            None,
            None,
            &workspace,
            &BoundaryPolicy::Allow,
            None,
        )
        .expect("default read should succeed");
        assert_eq!(
            payload_echo
                .file
                .content
                .as_deref()
                .expect("content present by default"),
            "echo this"
        );
        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    fn new_file_echoes_content_preview() {
        let path = temp_path("preview-write.txt");
        let payload = new_file(path.to_string_lossy().as_ref(), "alpha\nbeta\ngamma", false)
            .expect("write should succeed");
        let preview = payload
            .content_preview
            .as_deref()
            .expect("content_preview must be populated by default");
        assert!(preview.contains("alpha"));
        assert!(preview.contains("beta"));
        assert!(preview.contains("gamma"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn new_file_truncates_oversized_content_preview() {
        let path = temp_path("preview-large.txt");
        let large = "a".repeat(8_000);
        let payload =
            new_file(path.to_string_lossy().as_ref(), &large, false).expect("write should succeed");
        // Large files skip the content_preview because the full content
        // already exists in the ToolUse input (avoiding context doubling).
        assert!(
            payload.content_preview.is_none(),
            "content_preview should be None for files larger than CONTENT_PREVIEW_SKIP_THRESHOLD"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn edit_file_echoes_content_preview_of_new_file() {
        let path = temp_path("preview-edit.txt");
        new_file(
            path.to_string_lossy().as_ref(),
            "first\nsecond\nthird",
            false,
        )
        .expect("seed write should succeed");
        let payload = edit_file(
            path.to_string_lossy().as_ref(),
            "second",
            "SECOND-EDITED",
            false,
            None,
        )
        .expect("edit should succeed");
        let preview = payload
            .content_preview
            .as_deref()
            .expect("content_preview must be populated by default");
        // Preview must reflect the *post-edit* state so the model can
        // verify the change without re-reading the file.
        assert!(preview.contains("SECOND-EDITED"));
        assert!(!preview.contains("first\nsecond\nthird\nsecond"));
        // The full new content must still be on disk.
        let on_disk = std::fs::read_to_string(&path).expect("read back");
        assert_eq!(on_disk, "first\nSECOND-EDITED\nthird");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn edit_file_matches_across_crlf_line_endings() {
        let path = temp_path("crlf-edit.txt");
        // Seed a file with Windows CRLF line endings.
        std::fs::write(&path, "first\r\nsecond\r\nthird\r\n").expect("seed");
        let payload = edit_file(
            path.to_string_lossy().as_ref(),
            "second",
            "SECOND",
            false,
            None,
        )
        .expect("edit should succeed with LF old_string vs CRLF file");
        assert!(payload
            .content_preview
            .as_deref()
            .unwrap()
            .contains("SECOND"));
        let on_disk = std::fs::read_to_string(&path).expect("read back");
        assert_eq!(on_disk, "first\nSECOND\nthird\n");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn preview_for_handles_empty_and_small_and_oversized() {
        assert_eq!(preview_for(""), None);
        assert_eq!(preview_for("hi"), Some("hi".to_owned()));
        let big = "x".repeat(5_000);
        let clipped = preview_for(&big).expect("non-empty");
        assert!(clipped.contains("[truncated"));
        assert!(clipped.len() < big.len());
    }
}
