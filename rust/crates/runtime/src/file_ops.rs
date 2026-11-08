use std::cmp::Reverse;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

use glob::Pattern;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use walkdir::{DirEntry, WalkDir};

use crate::line_range_selector::{self, LineRangeSelector};

/// Maximum file size that can be read (10 MB).
const MAX_READ_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum file size that can be written (10 MB).
const MAX_WRITE_SIZE: usize = 10 * 1024 * 1024;

const GLOB_SEARCH_IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    ".build",
    "target",
    "dist",
    "coverage",
];

/// Check whether a file appears to contain binary content by examining
/// the first chunk for NUL bytes.
fn is_binary_file(path: &Path) -> io::Result<bool> {
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    let mut buffer = [0u8; 8192];
    let bytes_read = file.read(&mut buffer)?;
    Ok(buffer[..bytes_read].contains(&0))
}

/// Validate that a resolved path stays within the given workspace root.
/// Returns the canonical path on success, or an error if the path escapes
/// the workspace boundary (e.g. via `../` traversal or symlink).
#[allow(dead_code)]
fn validate_workspace_boundary(resolved: &Path, workspace_root: &Path) -> io::Result<()> {
    let workspace_root = normalize_for_comparison(workspace_root);
    let comparable = normalize_for_comparison(resolved);
    if resolves_within(resolved, &workspace_root) {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!(
            "path {} escapes workspace boundary {}",
            comparable.display(),
            workspace_root.display()
        ),
    ))
}

/// Whether `resolved` names a location inside `workspace_root` once the
/// filesystem resolves it. Windows accepts several spellings of one location
/// that [`Path::starts_with`] treats as different: letter case (a working
/// directory typed as `c:\users\...` for `C:\Users\...`), the `\\.\` device
/// namespace, and 8.3 short names. Existing ancestors are canonicalized so
/// equivalent names are accepted while symlink and parent traversal escapes
/// remain outside the workspace.
fn resolves_within(resolved: &Path, workspace_root: &Path) -> bool {
    if let Ok(canonical) = resolved.canonicalize() {
        if normalize_for_comparison(&canonical).starts_with(workspace_root) {
            return true;
        }
    }

    // Writes can target a path whose parent directories do not exist yet, so
    // canonicalizing the whole path may not be possible. Resolve the nearest
    // existing ancestor (including symlinks), then check its canonical path
    // and file identity before applying only the missing suffix.
    let mut existing_ancestor = resolved;
    loop {
        match fs::symlink_metadata(existing_ancestor) {
            Ok(_) => break,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let Some(parent) = existing_ancestor.parent() else {
                    return false;
                };
                existing_ancestor = parent;
            }
            Err(_) => return false,
        }
    }

    let Ok(canonical_ancestor) = existing_ancestor.canonicalize() else {
        // In particular, do not treat a dangling symlink as a missing path.
        return false;
    };
    #[cfg(windows)]
    if has_workspace_ancestor(&canonical_ancestor, workspace_root) {
        return resolved
            .strip_prefix(existing_ancestor)
            .is_ok_and(|suffix| {
                suffix.components().all(|component| {
                    !matches!(
                        component,
                        Component::ParentDir | Component::Prefix(_) | Component::RootDir
                    )
                })
            });
    }

    let Ok(missing_suffix) = resolved.strip_prefix(existing_ancestor) else {
        return false;
    };
    let mut canonical_path = normalize_for_comparison(&canonical_ancestor);
    for component in missing_suffix.components() {
        match component {
            Component::CurDir => {}
            // A parent component after a missing component could later
            // traverse a symlink once the missing directory is created.
            Component::ParentDir => return false,
            Component::Normal(name) => canonical_path.push(name),
            Component::Prefix(_) | Component::RootDir => return false,
        }
    }

    canonical_path.starts_with(workspace_root)
}

/// Compare Windows directory identities when canonical paths use different
/// long/8.3 spellings. Path strings alone cannot establish this relationship.
#[cfg(windows)]
fn has_workspace_ancestor(path: &Path, workspace_root: &Path) -> bool {
    let mut ancestor = path;
    loop {
        if same_file::is_same_file(ancestor, workspace_root).unwrap_or(false) {
            return true;
        }
        let Some(parent) = ancestor.parent() else {
            return false;
        };
        if parent == ancestor {
            return false;
        }
        ancestor = parent;
    }
}

/// Text payload returned by file-reading operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextFilePayload {
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub content: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "truncationFooter")]
    pub truncation_footer: Option<String>,
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

/// Output envelope for full-file write operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WriteFileOutput {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub content: String,
    #[serde(rename = "structuredPatch")]
    pub structured_patch: Vec<StructuredPatchHunk>,
    #[serde(rename = "originalFile")]
    pub original_file: Option<String>,
    #[serde(rename = "gitDiff")]
    pub git_diff: Option<serde_json::Value>,
}

/// Output envelope for targeted string-replacement edits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditFileOutput {
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "oldString")]
    pub old_string: String,
    #[serde(rename = "newString")]
    pub new_string: String,
    #[serde(rename = "originalFile")]
    pub original_file: String,
    #[serde(rename = "structuredPatch")]
    pub structured_patch: Vec<StructuredPatchHunk>,
    #[serde(rename = "userModified")]
    pub user_modified: bool,
    #[serde(rename = "replaceAll")]
    pub replace_all: bool,
    #[serde(rename = "gitDiff")]
    pub git_diff: Option<serde_json::Value>,
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
pub fn read_file(
    path: &str,
    offset: Option<usize>,
    limit: Option<usize>,
) -> io::Result<ReadFileOutput> {
    read_file_with_selector(path, offset, limit, None)
}

/// Reads a text file with optional GJC-style line range selector.
///
/// The selector takes precedence over offset/limit if provided.
/// Selector syntax: `:50`, `:50-200`, `:50+150`, `:raw`, `:conflicts`, `:5-16,960-973`
pub fn read_file_with_selector(
    path: &str,
    offset: Option<usize>,
    limit: Option<usize>,
    selector: Option<&str>,
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
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    // Special handling for conflicts selector
    if selector == Some("conflicts") {
        let conflicts = extract_merge_conflicts(&lines);
        let (selected, num_lines) = if conflicts.is_empty() {
            (format!("No unresolved git merge conflicts in {}.", path), 1)
        } else {
            let display_path = absolute_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path);
            let summary = format_conflict_summary(&conflicts, display_path);
            let summary_lines = summary.lines().count();
            (summary, summary_lines)
        };

        return Ok(ReadFileOutput {
            kind: String::from("text"),
            file: TextFilePayload {
                file_path: absolute_path.to_string_lossy().into_owned(),
                content: selected,
                num_lines,
                start_line: 1,
                total_lines,
            },
            truncation_footer: None,
        });
    }

    // Determine which lines to select
    let (start_index, end_index, truncation_footer) = if let Some(sel) = selector {
        // Parse and apply selector
        let parsed = line_range_selector::parse_selector(sel)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
        apply_line_selector(&lines, parsed, path)
    } else {
        // Use offset/limit for backward compatibility
        let start = offset.unwrap_or(0).min(lines.len());
        let end = limit.map_or(lines.len(), |lim| {
            start.saturating_add(lim).min(lines.len())
        });
        (start, end, None)
    };

    let selected = if start_index < end_index {
        lines[start_index..end_index].join("\n")
    } else {
        String::new()
    };

    Ok(ReadFileOutput {
        kind: String::from("text"),
        file: TextFilePayload {
            file_path: absolute_path.to_string_lossy().into_owned(),
            content: selected,
            num_lines: end_index.saturating_sub(start_index),
            start_line: start_index.saturating_add(1),
            total_lines,
        },
        truncation_footer,
    })
}

/// Represents a merge conflict block in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConflictBlock {
    start_line: usize,        // 1-based line number where <<<<<<< starts
    separator_line: usize,    // 1-based line number where ======= is
    end_line: usize,          // 1-based line number where >>>>>>> is
    base_line: Option<usize>, // 1-based line number where ||||||| is (for 3-way)
    ours_label: Option<String>,
    base_label: Option<String>,
    theirs_label: Option<String>,
}

/// Scans lines for unresolved git merge conflict blocks.
fn extract_merge_conflicts(lines: &[&str]) -> Vec<ConflictBlock> {
    let mut blocks = Vec::new();
    let mut i = 0;

    const OURS_PREFIX: &str = "<<<<<<<";
    const BASE_PREFIX: &str = "|||||||";
    const SEPARATOR: &str = "=======";
    const THEIRS_PREFIX: &str = ">>>>>>>";

    while i < lines.len() {
        let line = lines[i];

        // Check for start of conflict block
        if let Some(label) = extract_marker_label(line, OURS_PREFIX) {
            let ours_label = if label.is_empty() {
                None
            } else {
                Some(label.to_string())
            };
            let start_line = i + 1; // Convert to 1-based
            let mut separator_line = None;
            let mut base_line = None;
            let mut base_label = None;
            let mut end_line = None;
            let mut theirs_label = None;
            let mut j = i + 1;

            // Look for separator, optional base marker, and end marker
            while j < lines.len() {
                let current = lines[j];

                // Check for base marker (3-way)
                if let Some(label) = extract_marker_label(current, BASE_PREFIX) {
                    if base_line.is_none() {
                        base_line = Some(j + 1);
                        base_label = if label.is_empty() {
                            None
                        } else {
                            Some(label.to_string())
                        };
                        j += 1;
                        continue;
                    }
                }

                // Check for separator
                if current.trim() == SEPARATOR && separator_line.is_none() {
                    separator_line = Some(j + 1);
                    j += 1;
                    continue;
                }

                // Check for end marker
                if let Some(label) = extract_marker_label(current, THEIRS_PREFIX) {
                    if separator_line.is_some() {
                        end_line = Some(j + 1);
                        theirs_label = if label.is_empty() {
                            None
                        } else {
                            Some(label.to_string())
                        };
                        j += 1;
                        break;
                    }
                }

                j += 1;
            }

            // Only add if we found both separator and end marker
            if let (Some(sep_line), Some(end)) = (separator_line, end_line) {
                blocks.push(ConflictBlock {
                    start_line,
                    separator_line: sep_line,
                    end_line: end,
                    base_line,
                    ours_label,
                    base_label,
                    theirs_label,
                });
                i = j;
                continue;
            }
        }

        i += 1;
    }

    blocks
}

/// Extract the optional label from a marker line.
/// Returns the label text (stripped of leading/trailing spaces) if the line matches the prefix,
/// or None otherwise.
fn extract_marker_label<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    if let Some(rest) = line.strip_prefix(prefix) {
        // Accept either just the prefix or prefix + single space + label
        if rest.is_empty() {
            Some("")
        } else if let Some(label) = rest.strip_prefix(' ') {
            Some(label.trim())
        } else {
            None
        }
    } else {
        None
    }
}

/// Format conflict blocks as a summary index, like GJC's implementation.
fn format_conflict_summary(conflicts: &[ConflictBlock], display_path: &str) -> String {
    let mut output = Vec::new();
    let total = conflicts.len();
    let word = if total == 1 { "conflict" } else { "conflicts" };

    output.push(format!(
        "⚠ {} unresolved {} in {}",
        total, word, display_path
    ));

    // Extract labels for header
    let ours_label = conflicts.iter().find_map(|c| c.ours_label.as_ref());
    let theirs_label = conflicts.iter().find_map(|c| c.theirs_label.as_ref());
    let base_label = conflicts.iter().find_map(|c| c.base_label.as_ref());
    let any_base = conflicts.iter().any(|c| c.base_line.is_some());

    if let Some(label) = ours_label {
        output.push(format!("- ours = {}", label));
    }
    if let Some(label) = theirs_label {
        output.push(format!("- theirs = {}", label));
    }
    if any_base {
        let base_name = base_label
            .map(|l| l.to_string())
            .unwrap_or_else(|| "(no label)".to_string());
        output.push(format!("- base = {}", base_name));
    }

    output.push(
        "NOTICE: Bulk-resolve with `write({ path: \"conflict://*\", content })`, or address a single block with `write({ path: \"conflict://<N>\", content })`. Inspect a block by reading `conflict://<N>` (add `/ours` / `/theirs` / `/base` for a single side).".to_string()
    );
    output.push(
        "`content` shorthand: `@ours` / `@theirs` / `@base` / `@both` lines expand to the recorded sections; `@both` = ours-then-theirs. Non-token lines pass through verbatim.".to_string()
    );
    output.push(String::new());

    // Format each conflict as a one-liner index
    let id_width = total.to_string().len();
    for (idx, conflict) in conflicts.iter().enumerate() {
        let id = idx + 1;
        let range = if conflict.start_line == conflict.end_line {
            format!("L{}", conflict.start_line)
        } else {
            format!("L{}-{}", conflict.start_line, conflict.end_line)
        };
        let kind = if conflict.base_line.is_some() {
            "  (3-way)"
        } else {
            ""
        };
        output.push(format!(
            "#{:>width$}  {}{}",
            id,
            range,
            kind,
            width = id_width
        ));
    }

    output.join("\n")
}

/// Apply a parsed line selector to file lines and return the selected range.
///
/// Returns (start_index, end_index, truncation_footer).
fn apply_line_selector(
    lines: &[&str],
    selector: LineRangeSelector,
    _file_path: &str,
) -> (usize, usize, Option<String>) {
    match selector {
        LineRangeSelector::Raw => {
            // Return entire file without truncation footer for raw mode
            (0, lines.len(), None)
        }
        LineRangeSelector::Conflicts => {
            // Conflicts are handled specially in read_file_with_selector, so this shouldn't be reached
            (0, lines.len(), None)
        }
        LineRangeSelector::Lines(ranges) => {
            // Collect all lines specified by ranges
            let mut selected_indices = std::collections::BTreeSet::new();
            for range in ranges {
                let (start_zero, end_zero) = range.zero_indexed();
                for i in start_zero..=end_zero.min(lines.len().saturating_sub(1)) {
                    selected_indices.insert(i);
                }
            }

            if selected_indices.is_empty() {
                return (0, 0, None);
            }

            // Find contiguous blocks and return the first one (or could merge all)
            let indices: Vec<usize> = selected_indices.iter().copied().collect();
            let first = indices[0];
            let last = indices[indices.len() - 1];

            // Check if it's a contiguous range
            let is_contiguous = (last - first + 1) == indices.len();

            if is_contiguous {
                (first, last + 1, None)
            } else {
                // Return all selected indices as one continuous block from first to last
                // In the future, this could be improved to show multiple blocks
                (first, last + 1, None)
            }
        }
        LineRangeSelector::None => {
            // No selector, return entire file
            (0, lines.len(), None)
        }
    }
}

/// Replaces a file's contents and returns patch metadata.
pub fn write_file(path: &str, content: &str) -> io::Result<WriteFileOutput> {
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
    let original_file = fs::read_to_string(&absolute_path).ok();
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&absolute_path, content)?;

    Ok(WriteFileOutput {
        kind: if original_file.is_some() {
            String::from("update")
        } else {
            String::from("create")
        },
        file_path: absolute_path.to_string_lossy().into_owned(),
        content: content.to_owned(),
        structured_patch: make_patch(original_file.as_deref().unwrap_or(""), content),
        original_file,
        git_diff: None,
    })
}

/// Performs an in-file string replacement and returns patch metadata.
pub fn edit_file(
    path: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> io::Result<EditFileOutput> {
    let absolute_path = normalize_path(path)?;
    let original_file = fs::read_to_string(&absolute_path)?;
    if old_string == new_string {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "old_string and new_string must differ",
        ));
    }
    if !original_file.contains(old_string) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "old_string not found in file",
        ));
    }

    let updated = if replace_all {
        original_file.replace(old_string, new_string)
    } else {
        original_file.replacen(old_string, new_string, 1)
    };
    fs::write(&absolute_path, &updated)?;

    Ok(EditFileOutput {
        file_path: absolute_path.to_string_lossy().into_owned(),
        old_string: old_string.to_owned(),
        new_string: new_string.to_owned(),
        original_file: original_file.clone(),
        structured_patch: make_patch(&original_file, &updated),
        user_modified: false,
        replace_all,
        git_diff: None,
    })
}

/// Expands a glob pattern and returns matching filenames.
pub fn glob_search(pattern: &str, path: Option<&str>) -> io::Result<GlobSearchOutput> {
    glob_search_impl(pattern, path, None)
}

fn glob_search_impl(
    pattern: &str,
    path: Option<&str>,
    workspace_root: Option<&Path>,
) -> io::Result<GlobSearchOutput> {
    let started = Instant::now();
    let base_dir = path
        .map(normalize_path)
        .transpose()?
        .unwrap_or(std::env::current_dir()?);
    let canonical_root = workspace_root.map(canonicalize_workspace_root);
    if let Some(root) = canonical_root.as_deref() {
        validate_workspace_boundary(&base_dir, root)?;
    }
    let search_pattern = if Path::new(pattern).is_absolute() {
        pattern.to_owned()
    } else {
        base_dir.join(pattern).to_string_lossy().into_owned()
    };

    #[cfg(windows)]
    let search_pattern = normalize_for_comparison(Path::new(&search_pattern))
        .to_string_lossy()
        .replace('\\', "/");

    // The `glob` crate does not support brace expansion ({a,b,c}).
    // Expand braces into multiple patterns so patterns like
    // `Assets/**/*.{cs,uxml,uss}` work correctly.
    let expanded = expand_braces(&search_pattern);

    let mut seen = HashSet::new();
    let mut matches = Vec::new();
    for pat in &expanded {
        let compiled = Pattern::new(pat)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
        let walk_root = derive_glob_walk_root(pat);
        if let Some(root) = canonical_root.as_deref() {
            let canonical_walk_root = walk_root
                .canonicalize()
                .unwrap_or_else(|_| walk_root.clone());
            validate_workspace_boundary(&canonical_walk_root, root)?;
        }
        let entries = WalkDir::new(&walk_root)
            .into_iter()
            .filter_entry(|entry| !should_skip_glob_dir(entry));
        for entry in entries.flatten() {
            let candidate = entry.path();
            if entry.file_type().is_file()
                && compiled.matches_path(candidate)
                && seen.insert(candidate.to_path_buf())
            {
                if let Some(root) = canonical_root.as_deref() {
                    let canonical_candidate = candidate.canonicalize()?;
                    validate_workspace_boundary(&canonical_candidate, root)?;
                }
                matches.push(candidate.to_path_buf());
            }
        }
    }

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
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    Ok(GlobSearchOutput {
        duration_ms: started.elapsed().as_millis(),
        num_files: filenames.len(),
        filenames,
        truncated,
    })
}

/// Runs a regex search over workspace files with optional context lines.
pub fn grep_search(input: &GrepSearchInput) -> io::Result<GrepSearchOutput> {
    grep_search_impl(input, None)
}

fn grep_search_impl(
    input: &GrepSearchInput,
    workspace_root: Option<&Path>,
) -> io::Result<GrepSearchOutput> {
    let base_path = input
        .path
        .as_deref()
        .map(normalize_path)
        .transpose()?
        .unwrap_or(std::env::current_dir()?);
    let canonical_root = workspace_root.map(canonicalize_workspace_root);
    if let Some(root) = canonical_root.as_deref() {
        validate_workspace_boundary(&base_path, root)?;
    }

    let regex = RegexBuilder::new(&input.pattern)
        .case_insensitive(input.case_insensitive.unwrap_or(false))
        .dot_matches_new_line(input.multiline.unwrap_or(false))
        .build()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;

    let glob_filter = input
        .glob
        .as_deref()
        .map(Pattern::new)
        .transpose()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
    let file_type = input.file_type.as_deref();
    let output_mode = input
        .output_mode
        .clone()
        .unwrap_or_else(|| String::from("files_with_matches"));
    let context = input.context.or(input.context_short).unwrap_or(0);

    let mut filenames = Vec::new();
    let mut content_lines = Vec::new();
    let mut total_matches = 0usize;

    for file_path in collect_search_files(&base_path)? {
        if let Some(root) = canonical_root.as_deref() {
            let canonical_file = file_path.canonicalize()?;
            validate_workspace_boundary(&canonical_file, root)?;
        }
        if !matches_optional_filters(&file_path, glob_filter.as_ref(), file_type) {
            continue;
        }

        let Ok(file_contents) = fs::read_to_string(&file_path) else {
            continue;
        };

        if output_mode == "count" {
            let count = regex.find_iter(&file_contents).count();
            if count > 0 {
                filenames.push(file_path.to_string_lossy().into_owned());
                total_matches += count;
            }
            continue;
        }

        let lines: Vec<&str> = file_contents.lines().collect();
        let mut matched_lines = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            if regex.is_match(line) {
                total_matches += 1;
                matched_lines.push(index);
            }
        }

        if matched_lines.is_empty() {
            continue;
        }

        filenames.push(file_path.to_string_lossy().into_owned());
        if output_mode == "content" {
            for index in matched_lines {
                let start = index.saturating_sub(input.before.unwrap_or(context));
                let end = (index + input.after.unwrap_or(context) + 1).min(lines.len());
                for (current, line) in lines.iter().enumerate().take(end).skip(start) {
                    let prefix = if input.line_numbers.unwrap_or(true) {
                        format!("{}:{}:", file_path.to_string_lossy(), current + 1)
                    } else {
                        format!("{}:", file_path.to_string_lossy())
                    };
                    content_lines.push(format!("{prefix}{line}"));
                }
            }
        }
    }

    let (filenames, applied_limit, applied_offset) =
        apply_limit(filenames, input.head_limit, input.offset);
    if output_mode == "content" {
        return Ok(build_grep_content_output(
            output_mode,
            filenames,
            content_lines,
            input.head_limit,
            input.offset,
        ));
    }

    Ok(GrepSearchOutput {
        mode: Some(output_mode.clone()),
        num_files: filenames.len(),
        filenames,
        content: None,
        num_lines: None,
        num_matches: (output_mode == "count").then_some(total_matches),
        applied_limit,
        applied_offset,
    })
}

fn build_grep_content_output(
    output_mode: String,
    filenames: Vec<String>,
    content_lines: Vec<String>,
    head_limit: Option<usize>,
    offset: Option<usize>,
) -> GrepSearchOutput {
    let (lines, limit, offset) = apply_limit(content_lines, head_limit, offset);
    GrepSearchOutput {
        mode: Some(output_mode),
        num_files: filenames.len(),
        filenames,
        num_lines: Some(lines.len()),
        content: Some(lines.join("\n")),
        num_matches: None,
        applied_limit: limit,
        applied_offset: offset,
    }
}

fn canonicalize_workspace_root(workspace_root: &Path) -> PathBuf {
    let canonical = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());
    normalize_for_comparison(&canonical)
}

/// Normalize Windows' extended-length path prefix before comparing paths.
///
/// `std::fs::canonicalize` returns paths with a `\\?\` prefix on Windows,
/// while a workspace root may still be represented as a regular path when it
/// cannot be canonicalized (for example, while a workspace is bootstrapping).
/// These paths refer to the same location but do not compare equal with
/// [`Path::starts_with`]. Keep the comparison representation stable without
/// changing the paths returned to callers.
fn normalize_for_comparison(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(unc_path) = raw.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{unc_path}"));
    }
    if let Some(dos_path) = raw.strip_prefix(r"\\?\") {
        return PathBuf::from(dos_path);
    }
    path.to_path_buf()
}

fn should_skip_glob_dir(entry: &DirEntry) -> bool {
    entry.file_type().is_dir()
        && entry
            .file_name()
            .to_str()
            .is_some_and(|name| GLOB_SEARCH_IGNORED_DIRS.contains(&name))
}

fn derive_glob_walk_root(pattern: &str) -> PathBuf {
    let path = Path::new(pattern);
    let mut prefix = PathBuf::new();
    let mut saw_component = false;

    for component in path.components() {
        let text = component.as_os_str().to_string_lossy();
        if component_contains_glob(&text) {
            break;
        }
        prefix.push(component.as_os_str());
        saw_component = true;
    }

    if saw_component {
        prefix
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }
}

fn component_contains_glob(component: &str) -> bool {
    component.contains('*') || component.contains('?') || component.contains('[')
}

fn collect_search_files(base_path: &Path) -> io::Result<Vec<PathBuf>> {
    if base_path.is_file() {
        return Ok(vec![base_path.to_path_buf()]);
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(base_path) {
        let entry = entry.map_err(|error| io::Error::other(error.to_string()))?;
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    Ok(files)
}

fn matches_optional_filters(
    path: &Path,
    glob_filter: Option<&Pattern>,
    file_type: Option<&str>,
) -> bool {
    if let Some(glob_filter) = glob_filter {
        let path_string = path.to_string_lossy();
        if !glob_filter.matches(&path_string) && !glob_filter.matches_path(path) {
            return false;
        }
    }

    if let Some(file_type) = file_type {
        let extension = path.extension().and_then(|extension| extension.to_str());
        if extension != Some(file_type) {
            return false;
        }
    }

    true
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

fn normalize_path(path: &str) -> io::Result<PathBuf> {
    let candidate = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir()?.join(path)
    };
    candidate.canonicalize()
}

fn normalize_path_allow_missing(path: &str) -> io::Result<PathBuf> {
    let candidate = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir()?.join(path)
    };

    if let Ok(canonical) = candidate.canonicalize() {
        return Ok(canonical);
    }

    if let Some(parent) = candidate.parent() {
        let canonical_parent = parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_path_buf());
        if let Some(name) = candidate.file_name() {
            return Ok(canonical_parent.join(name));
        }
    }

    Ok(candidate)
}

/// Read a file with workspace boundary enforcement.
#[allow(dead_code)]
pub fn read_file_in_workspace(
    path: &str,
    offset: Option<usize>,
    limit: Option<usize>,
    workspace_root: &Path,
) -> io::Result<ReadFileOutput> {
    read_file_in_workspace_with_selector(path, offset, limit, None, workspace_root)
}

/// Read a file with workspace boundary enforcement and optional line range selector.
#[allow(dead_code)]
pub fn read_file_in_workspace_with_selector(
    path: &str,
    offset: Option<usize>,
    limit: Option<usize>,
    selector: Option<&str>,
    workspace_root: &Path,
) -> io::Result<ReadFileOutput> {
    let absolute_path = normalize_path(path)?;
    let canonical_root = canonicalize_workspace_root(workspace_root);
    validate_workspace_boundary(&absolute_path, &canonical_root)?;
    read_file_with_selector(path, offset, limit, selector)
}

/// Write a file with workspace boundary enforcement.
#[allow(dead_code)]
pub fn write_file_in_workspace(
    path: &str,
    content: &str,
    workspace_root: &Path,
) -> io::Result<WriteFileOutput> {
    let absolute_path = normalize_path_allow_missing(path)?;
    let canonical_root = canonicalize_workspace_root(workspace_root);
    validate_workspace_boundary(&absolute_path, &canonical_root)?;
    write_file(path, content)
}

/// Edit a file with workspace boundary enforcement.
#[allow(dead_code)]
pub fn edit_file_in_workspace(
    path: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
    workspace_root: &Path,
) -> io::Result<EditFileOutput> {
    let absolute_path = normalize_path(path)?;
    let canonical_root = canonicalize_workspace_root(workspace_root);
    validate_workspace_boundary(&absolute_path, &canonical_root)?;
    edit_file(path, old_string, new_string, replace_all)
}

/// Expand a glob pattern with workspace boundary enforcement.
#[allow(dead_code)]
pub fn glob_search_in_workspace(
    pattern: &str,
    path: Option<&str>,
    workspace_root: &Path,
) -> io::Result<GlobSearchOutput> {
    glob_search_impl(pattern, path, Some(workspace_root))
}

/// Search file contents with workspace boundary enforcement.
#[allow(dead_code)]
pub fn grep_search_in_workspace(
    input: &GrepSearchInput,
    workspace_root: &Path,
) -> io::Result<GrepSearchOutput> {
    grep_search_impl(input, Some(workspace_root))
}

/// Check whether a path is a symlink that resolves outside the workspace.
#[allow(dead_code)]
pub fn is_symlink_escape(path: &Path, workspace_root: &Path) -> io::Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_symlink() {
        return Ok(false);
    }
    let resolved = path.canonicalize()?;
    let canonical_root = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());
    Ok(!resolved.starts_with(&canonical_root))
}

/// Expand shell-style brace groups in a glob pattern.
///
/// Handles one level of braces: `foo.{a,b,c}` → `["foo.a", "foo.b", "foo.c"]`.
/// Nested braces are not expanded (uncommon in practice).
/// Patterns without braces pass through unchanged.
fn expand_braces(pattern: &str) -> Vec<String> {
    let Some(open) = pattern.find('{') else {
        return vec![pattern.to_owned()];
    };
    let Some(close) = pattern[open..].find('}').map(|i| open + i) else {
        // Unmatched brace — treat as literal.
        return vec![pattern.to_owned()];
    };
    let prefix = &pattern[..open];
    let suffix = &pattern[close + 1..];
    let alternatives = &pattern[open + 1..close];
    alternatives
        .split(',')
        .flat_map(|alt| expand_braces(&format!("{prefix}{alt}{suffix}")))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(any(unix, windows))]
    use super::write_file_in_workspace;
    use super::{
        component_contains_glob, derive_glob_walk_root, edit_file, expand_braces, glob_search,
        grep_search, is_symlink_escape, normalize_for_comparison, read_file,
        read_file_in_workspace, write_file, GrepSearchInput, MAX_WRITE_SIZE,
    };

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
        let write_output = write_file(path.to_string_lossy().as_ref(), "one\ntwo\nthree")
            .expect("write should succeed");
        assert_eq!(write_output.kind, "create");

        let read_output = read_file(path.to_string_lossy().as_ref(), Some(1), Some(1))
            .expect("read should succeed");
        assert_eq!(read_output.file.content, "two");
    }

    #[test]
    fn edits_file_contents() {
        let path = temp_path("edit.txt");
        write_file(path.to_string_lossy().as_ref(), "alpha beta alpha")
            .expect("initial write should succeed");
        let output = edit_file(path.to_string_lossy().as_ref(), "alpha", "omega", true)
            .expect("edit should succeed");
        assert!(output.replace_all);
    }

    #[test]
    fn rejects_binary_files() {
        let path = temp_path("binary-test.bin");
        std::fs::write(&path, b"\x00\x01\x02\x03binary content").expect("write should succeed");
        let result = read_file(path.to_string_lossy().as_ref(), None, None);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("binary"));
    }

    #[test]
    fn rejects_oversized_writes() {
        let path = temp_path("oversize-write.txt");
        let huge = "x".repeat(MAX_WRITE_SIZE + 1);
        let result = write_file(path.to_string_lossy().as_ref(), &huge);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("too large"));
    }

    #[test]
    fn enforces_workspace_boundary() {
        let workspace = temp_path("workspace-boundary");
        std::fs::create_dir_all(&workspace).expect("workspace dir should be created");
        let inside = workspace.join("inside.txt");
        write_file(inside.to_string_lossy().as_ref(), "safe content")
            .expect("write inside workspace should succeed");

        // Reading inside workspace should succeed
        let result =
            read_file_in_workspace(inside.to_string_lossy().as_ref(), None, None, &workspace);
        assert!(result.is_ok());

        // Reading outside workspace should fail
        let outside = temp_path("outside-boundary.txt");
        write_file(outside.to_string_lossy().as_ref(), "unsafe content")
            .expect("write outside should succeed");
        let result =
            read_file_in_workspace(outside.to_string_lossy().as_ref(), None, None, &workspace);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(error.to_string().contains("escapes workspace"));
    }

    #[test]
    fn detects_symlink_escape() {
        let workspace = temp_path("symlink-workspace");
        std::fs::create_dir_all(&workspace).expect("workspace dir should be created");
        let outside = temp_path("symlink-target.txt");
        std::fs::write(&outside, "target content").expect("target should write");

        #[cfg(unix)]
        {
            let link_path = workspace.join("escape-link.txt");
            std::os::unix::fs::symlink(&outside, &link_path).expect("symlink should create");
            assert!(is_symlink_escape(&link_path, &workspace).expect("check should succeed"));
        }

        // Non-symlink file should not be an escape
        let normal = workspace.join("normal.txt");
        std::fs::write(&normal, "normal content").expect("normal file should write");
        assert!(!is_symlink_escape(&normal, &workspace).expect("check should succeed"));
    }

    #[test]
    #[cfg(unix)]
    fn workspace_read_rejects_symlink_escape_regression_3007_class() {
        let workspace = temp_path("workspace-read-symlink-escape");
        let outside = temp_path("workspace-read-symlink-target");
        std::fs::create_dir_all(&workspace).expect("workspace dir should be created");
        std::fs::create_dir_all(&outside).expect("outside dir should be created");
        let outside_file = outside.join("secret.txt");
        std::fs::write(&outside_file, "outside secret").expect("outside file should write");

        let link_path = workspace.join("linked-secret.txt");
        std::os::unix::fs::symlink(&outside_file, &link_path).expect("symlink should create");

        let result =
            read_file_in_workspace(link_path.to_string_lossy().as_ref(), None, None, &workspace);

        assert!(result.is_err(), "symlink escape must be rejected");
        let error = result.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(
            error.to_string().contains("escapes workspace"),
            "error should explain workspace escape: {error}"
        );

        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn normalizes_windows_extended_length_prefixes_for_boundary_checks() {
        let dos_path = PathBuf::from(r"C:\workspace\file.txt");
        let extended_dos_path = PathBuf::from(r"\\?\C:\workspace\file.txt");
        assert_eq!(
            normalize_for_comparison(&extended_dos_path),
            dos_path,
            "extended DOS paths should compare like their regular form"
        );

        let unc_path = PathBuf::from(r"\\server\share\workspace\file.txt");
        let extended_unc_path = PathBuf::from(r"\\?\UNC\server\share\workspace\file.txt");
        assert_eq!(
            normalize_for_comparison(&extended_unc_path),
            unc_path,
            "extended UNC paths should compare like their regular form"
        );
    }

    // Backslash-separated paths only parse into path components on Windows.
    #[test]
    #[cfg(windows)]
    fn accepts_equivalent_extended_path_at_workspace_boundary() {
        let root = PathBuf::from(r"C:\workspace");
        let resolved = PathBuf::from(r"\\?\C:\workspace\src\main.rs");
        super::validate_workspace_boundary(&resolved, &root)
            .expect("equivalent Windows path representations should be accepted");
    }

    #[test]
    #[cfg(unix)]
    fn accepts_a_path_that_resolves_inside_the_workspace() {
        let workspace = temp_path("boundary-alias-workspace");
        let aliases = temp_path("boundary-alias-links");
        std::fs::create_dir_all(workspace.join("src")).expect("workspace dir should be created");
        std::fs::create_dir_all(&aliases).expect("alias dir should be created");
        let alias = aliases.join("src");
        std::os::unix::fs::symlink(workspace.join("src"), &alias).expect("symlink should create");
        let root = workspace
            .canonicalize()
            .expect("workspace should canonicalize");

        super::validate_workspace_boundary(&alias, &root)
            .expect("a spelling that resolves inside the workspace should be accepted");
        super::validate_workspace_boundary(&aliases, &root)
            .expect_err("a path that resolves outside the workspace must stay rejected");

        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&aliases);
    }

    // Windows resolves one location from spellings that compare unequal as
    // paths, letter case and the `\\.\` device namespace among them.
    #[test]
    #[cfg(windows)]
    fn accepts_other_windows_spellings_of_a_workspace_path() {
        let workspace = temp_path("Boundary-Case-Workspace");
        std::fs::create_dir_all(workspace.join("src")).expect("workspace dir should be created");
        let root = workspace
            .canonicalize()
            .expect("workspace should canonicalize");

        let lower = PathBuf::from(workspace.join("src").to_string_lossy().to_lowercase());
        super::validate_workspace_boundary(&lower, &root)
            .expect("a lower-cased spelling of a workspace path should be accepted");

        let device = PathBuf::from(format!(r"\\.\{}", workspace.join("src").display()));
        super::validate_workspace_boundary(&device, &root)
            .expect("a device-namespace spelling of a workspace path should be accepted");

        let parent = workspace
            .parent()
            .expect("workspace has a parent")
            .to_path_buf();
        super::validate_workspace_boundary(&parent, &root)
            .expect_err("the workspace's parent must stay rejected");

        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    #[cfg(windows)]
    fn writes_inside_workspace_through_short_path_with_missing_nested_directories() {
        let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
            return;
        };
        let long_temp = PathBuf::from(local_app_data).join("Temp");
        let short_temp = std::env::temp_dir();
        if !long_temp.is_dir()
            || !short_temp.is_dir()
            || long_temp == short_temp
            || !same_file::is_same_file(&long_temp, &short_temp).unwrap_or(false)
        {
            // Some Windows hosts disable short-name generation or configure
            // TEMP outside the user's local application data directory.
            return;
        }

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let workspace = long_temp.join(format!("clawd-native-short-path-{unique}"));
        std::fs::create_dir_all(&workspace).expect("workspace dir should be created");
        let root = workspace
            .canonicalize()
            .expect("workspace should canonicalize");
        let short_workspace = short_temp.join(
            workspace
                .file_name()
                .expect("workspace should have a file name"),
        );
        let target = short_workspace.join("nested").join("demo.txt");

        write_file_in_workspace(&target.to_string_lossy(), "safe content", &root)
            .expect("an equivalent short path inside the workspace should be accepted");
        assert_eq!(
            std::fs::read_to_string(workspace.join("nested/demo.txt"))
                .expect("written file should be readable"),
            "safe content"
        );

        let update = write_file_in_workspace(&target.to_string_lossy(), "updated content", &root)
            .expect("an existing file through an equivalent short path should be accepted");
        assert_eq!(update.kind, "update");
        assert_eq!(update.original_file.as_deref(), Some("safe content"));
        assert_eq!(
            std::fs::read_to_string(workspace.join("nested/demo.txt"))
                .expect("updated file should be readable"),
            "updated content"
        );

        let escaping_target = short_workspace
            .join("not-created")
            .join("..")
            .join("..")
            .join("escape.txt");
        write_file_in_workspace(&escaping_target.to_string_lossy(), "must not escape", &root)
            .expect_err("parent traversal from a missing path must stay outside the boundary");

        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    #[cfg(unix)]
    fn workspace_write_rejects_parent_symlink_escape_regression_3007_class() {
        let workspace = temp_path("workspace-write-symlink-escape");
        let outside = temp_path("workspace-write-symlink-target");
        std::fs::create_dir_all(&workspace).expect("workspace dir should be created");
        std::fs::create_dir_all(&outside).expect("outside dir should be created");

        let link_dir = workspace.join("linked-outside");
        std::os::unix::fs::symlink(&outside, &link_dir).expect("symlink dir should create");
        let escaped_child = link_dir.join("created.txt");

        let result = write_file_in_workspace(
            escaped_child.to_string_lossy().as_ref(),
            "must not escape",
            &workspace,
        );

        assert!(result.is_err(), "parent symlink escape must be rejected");
        let error = result.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(
            error.to_string().contains("escapes workspace"),
            "error should explain workspace escape: {error}"
        );
        assert!(
            !outside.join("created.txt").exists(),
            "write should not create through an escaping symlink"
        );

        let escaped_nested_child = link_dir.join("not-created").join("nested.txt");
        let result = write_file_in_workspace(
            escaped_nested_child.to_string_lossy().as_ref(),
            "must not escape",
            &workspace,
        );
        assert!(
            result.is_err(),
            "a missing descendant under an escaping symlink must be rejected"
        );
        assert!(
            !outside.join("not-created").exists(),
            "write should not create missing directories through an escaping symlink"
        );

        let _ = std::fs::remove_dir_all(&workspace);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn globs_and_greps_directory() {
        let dir = temp_path("search-dir");
        std::fs::create_dir_all(&dir).expect("directory should be created");
        let file = dir.join("demo.rs");
        write_file(
            file.to_string_lossy().as_ref(),
            "fn main() {\n println!(\"hello\");\n}\n",
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
    fn expand_braces_no_braces() {
        assert_eq!(expand_braces("*.rs"), vec!["*.rs"]);
    }

    #[test]
    fn expand_braces_single_group() {
        let mut result = expand_braces("Assets/**/*.{cs,uxml,uss}");
        result.sort();
        assert_eq!(
            result,
            vec!["Assets/**/*.cs", "Assets/**/*.uss", "Assets/**/*.uxml",]
        );
    }

    #[test]
    fn expand_braces_nested() {
        let mut result = expand_braces("src/{a,b}.{rs,toml}");
        result.sort();
        assert_eq!(
            result,
            vec!["src/a.rs", "src/a.toml", "src/b.rs", "src/b.toml"]
        );
    }

    #[test]
    fn expand_braces_unmatched() {
        assert_eq!(expand_braces("foo.{bar"), vec!["foo.{bar"]);
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

    #[test]
    fn glob_search_skips_common_heavy_directories() {
        let dir = temp_path("glob-ignored-dirs");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("docs")).unwrap();
        std::fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
        std::fs::create_dir_all(dir.join(".build/checkouts/pkg")).unwrap();
        std::fs::create_dir_all(dir.join("target/debug/deps")).unwrap();

        std::fs::write(dir.join("src/AGENTS.md"), "src").unwrap();
        std::fs::write(dir.join("docs/AGENTS.md"), "docs").unwrap();
        std::fs::write(dir.join("node_modules/pkg/AGENTS.md"), "node_modules").unwrap();
        std::fs::write(dir.join(".build/checkouts/pkg/AGENTS.md"), ".build").unwrap();
        std::fs::write(dir.join("target/debug/deps/AGENTS.md"), "target").unwrap();

        let result =
            glob_search("**/AGENTS.md", Some(dir.to_str().unwrap())).expect("glob should succeed");

        assert_eq!(result.num_files, 2, "ignored dirs should be pruned");
        assert!(result
            .filenames
            .iter()
            .any(|path| Path::new(path).ends_with("src/AGENTS.md")));
        assert!(result
            .filenames
            .iter()
            .any(|path| Path::new(path).ends_with("docs/AGENTS.md")));
        assert!(!result
            .filenames
            .iter()
            .any(|path| path.contains("node_modules")
                || path.contains(".build")
                || path.contains("/target/")));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn derive_glob_walk_root_stops_at_first_glob_component() {
        let root = derive_glob_walk_root("/tmp/demo/**/AGENTS.md");
        assert_eq!(root, PathBuf::from("/tmp/demo"));
        assert!(component_contains_glob("**"));
        assert!(component_contains_glob("*.rs"));
        assert!(!component_contains_glob("src"));
    }

    #[test]
    fn read_with_selector_single_line() {
        let path = temp_path("selector-single.txt");
        write_file(
            path.to_string_lossy().as_ref(),
            "line1\nline2\nline3\nline4\nline5",
        )
        .expect("write should succeed");

        let output =
            super::read_file_with_selector(path.to_string_lossy().as_ref(), None, None, Some("2"))
                .expect("read with selector should succeed");
        assert_eq!(output.file.content, "line2");
        assert_eq!(output.file.start_line, 2);
        assert_eq!(output.file.num_lines, 1);
    }

    #[test]
    fn read_with_selector_range() {
        let path = temp_path("selector-range.txt");
        write_file(
            path.to_string_lossy().as_ref(),
            "line1\nline2\nline3\nline4\nline5",
        )
        .expect("write should succeed");

        let output = super::read_file_with_selector(
            path.to_string_lossy().as_ref(),
            None,
            None,
            Some("2-4"),
        )
        .expect("read with selector should succeed");
        assert_eq!(output.file.content, "line2\nline3\nline4");
        assert_eq!(output.file.start_line, 2);
        assert_eq!(output.file.num_lines, 3);
    }

    #[test]
    fn read_with_selector_plus_count() {
        let path = temp_path("selector-plus.txt");
        write_file(
            path.to_string_lossy().as_ref(),
            "line1\nline2\nline3\nline4\nline5",
        )
        .expect("write should succeed");

        let output = super::read_file_with_selector(
            path.to_string_lossy().as_ref(),
            None,
            None,
            Some("2+2"),
        )
        .expect("read with selector should succeed");
        assert_eq!(output.file.content, "line2\nline3");
        assert_eq!(output.file.start_line, 2);
        assert_eq!(output.file.num_lines, 2);
    }

    #[test]
    fn read_with_selector_raw() {
        let path = temp_path("selector-raw.txt");
        write_file(
            path.to_string_lossy().as_ref(),
            "line1\nline2\nline3\nline4\nline5",
        )
        .expect("write should succeed");

        let output = super::read_file_with_selector(
            path.to_string_lossy().as_ref(),
            None,
            None,
            Some("raw"),
        )
        .expect("read with selector should succeed");
        assert_eq!(output.file.content, "line1\nline2\nline3\nline4\nline5");
        assert_eq!(output.file.start_line, 1);
        assert_eq!(output.file.num_lines, 5);
    }

    #[test]
    fn read_with_selector_none() {
        let path = temp_path("selector-none.txt");
        write_file(
            path.to_string_lossy().as_ref(),
            "line1\nline2\nline3\nline4\nline5",
        )
        .expect("write should succeed");

        let output =
            super::read_file_with_selector(path.to_string_lossy().as_ref(), None, None, None)
                .expect("read with selector should succeed");
        assert_eq!(output.file.content, "line1\nline2\nline3\nline4\nline5");
        assert_eq!(output.file.start_line, 1);
        assert_eq!(output.file.num_lines, 5);
    }

    #[test]
    fn read_with_invalid_selector() {
        let path = temp_path("selector-invalid.txt");
        write_file(
            path.to_string_lossy().as_ref(),
            "line1\nline2\nline3\nline4\nline5",
        )
        .expect("write should succeed");

        let result = super::read_file_with_selector(
            path.to_string_lossy().as_ref(),
            None,
            None,
            Some("invalid-selector"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn read_with_selector_conflicts_no_conflicts() {
        let path = temp_path("conflicts-none.txt");
        write_file(
            path.to_string_lossy().as_ref(),
            "const a = 1;\nconst b = 2;\n",
        )
        .expect("write should succeed");

        let output = super::read_file_with_selector(
            path.to_string_lossy().as_ref(),
            None,
            None,
            Some("conflicts"),
        )
        .expect("read with conflicts selector should succeed");
        assert!(output
            .file
            .content
            .contains("No unresolved git merge conflicts"));
    }

    #[test]
    fn read_with_selector_conflicts_two_way() {
        let path = temp_path("conflicts-two-way.txt");
        let content = "<<<<<<< A\na-ours\n=======\na-theirs\n>>>>>>> A\nmiddle\n<<<<<<< B\nb-ours\n=======\nb-theirs\n>>>>>>> B\ntail";
        write_file(path.to_string_lossy().as_ref(), content).expect("write should succeed");

        let output = super::read_file_with_selector(
            path.to_string_lossy().as_ref(),
            None,
            None,
            Some("conflicts"),
        )
        .expect("read with conflicts selector should succeed");

        let text = &output.file.content;
        // Should report 2 conflicts
        assert!(text.contains("2 unresolved conflicts"), "text: {}", text);
        // Should have indices for both blocks
        assert!(text.contains("#1"), "missing #1: {}", text);
        assert!(text.contains("#2"), "missing #2: {}", text);
        // Should have line ranges
        assert!(text.contains("L1-5"), "missing L1-5: {}", text);
        assert!(text.contains("L7-11"), "missing L7-11: {}", text);
    }

    #[test]
    fn read_with_selector_conflicts_three_way() {
        let path = temp_path("conflicts-three-way.txt");
        let content = "head\n<<<<<<< HEAD\nours body\n||||||| common ancestor\nbase body\n=======\ntheirs body\n>>>>>>> feat\ntail";
        write_file(path.to_string_lossy().as_ref(), content).expect("write should succeed");

        let output = super::read_file_with_selector(
            path.to_string_lossy().as_ref(),
            None,
            None,
            Some("conflicts"),
        )
        .expect("read with conflicts selector should succeed");

        let text = &output.file.content;
        // Should report 1 conflict
        assert!(text.contains("1 unresolved conflict"), "text: {}", text);
        // Should mark as 3-way
        assert!(text.contains("3-way"), "missing 3-way marker: {}", text);
        // Should show line range
        assert!(text.contains("L2-8"), "missing L2-8: {}", text);
    }
}
