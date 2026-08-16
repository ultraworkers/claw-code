//! Extract absolute paths the user named in free-form input.
//!
//! Used by the REPL input handler to pre-trust paths the user
//! explicitly typed or dropped into input. The active
//! `WorkspacePolicy::Prompt` consults this trust set first, so the
//! LLM can read the file without a confirmation prompt.
//!
//! Path types we recognise:
//!
//! * Windows drive-letter paths: `C:\Users\me\file.txt`,
//!   `D:/path/to/file`
//! * Windows UNC paths: `\\server\share\file.txt`
//! * POSIX absolute paths: `/home/me/file.txt`
//! * Quoted forms: `"C:\Users\me\file.txt"`,
//!   `'C:\Users\me\file.txt'`
//! * Home-relative: `~/file.txt` (expanded against `HOME` /
//!   `USERPROFILE`)
//!
//! Relative paths and bare words are *not* treated as user trust
//! signals — only an explicit absolute path is. URLs are also
//! excluded.

use std::collections::BTreeSet;
use std::env;
use std::path::{Path, PathBuf};

/// Extract every absolute path the user named in their input. The
/// returned paths are not necessarily canonicalised — callers that
/// want canonical forms should call `Path::canonicalize` per entry.
/// Duplicates are removed.
pub fn extract_absolute_paths(input: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from);

    for raw in split_path_tokens(input) {
        if let Some(p) = normalise_candidate(&raw, home.as_deref()) {
            if seen.insert(p.clone()) {
                out.push(p);
            }
        }
    }
    out
}

/// Split `input` into path-like tokens. We split on whitespace and
/// common punctuation that often surrounds a path in prose (`,`,
/// `;`, `,`, `(`, `)`). Quote stripping happens later so a token
/// like `"C:\Users\me\a.txt"` round-trips.
fn split_path_tokens(input: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_dquote = false;
    let mut in_squote = false;
    for ch in input.chars() {
        match ch {
            '"' if !in_squote => {
                in_dquote = !in_dquote;
                if !in_dquote && !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            '\'' if !in_dquote => {
                in_squote = !in_squote;
                if !in_squote && !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c if c.is_whitespace() && !in_dquote && !in_squote => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c if (c == ',' || c == ';' || c == '(' || c == ')')
                && !in_dquote && !in_squote =>
            {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// If the raw token looks like an absolute path, return a normalised
/// `PathBuf`. Returns `None` for relative paths, URL-like tokens,
/// or other non-paths.
fn normalise_candidate(raw: &str, home: Option<&Path>) -> Option<PathBuf> {
    let stripped = raw
        .trim()
        .trim_end_matches(|c: char| ",;".contains(c))
        .trim_matches(|c: char| c == '"' || c == '\'');
    if stripped.is_empty() {
        return None;
    }
    if looks_like_url(stripped) {
        return None;
    }
    let expanded = if let Some(rest) = stripped.strip_prefix("~/") {
        match home {
            Some(h) => h.join(rest),
            None => return None,
        }
    } else if let Some(rest) = stripped.strip_prefix("~\\") {
        match home {
            Some(h) => h.join(rest),
            None => return None,
        }
    } else {
        PathBuf::from(stripped)
    };
    if !is_absolute(&expanded) {
        return None;
    }
    Some(expanded)
}

/// Returns `true` if `path` is absolute under either the OS
/// definition or a Windows drive-letter form. The Windows check is
/// necessary because `Path::is_absolute` returns `false` for
/// `C:/Users/me` (forward slashes) on some platforms, but a user
/// who types `C:/Users/me` almost certainly means the absolute
/// path `C:\Users\me`.
fn is_absolute(path: &Path) -> bool {
    if path.is_absolute() {
        return true;
    }
    let s = path.to_string_lossy();
    let bytes = s.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn looks_like_url(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("file://")
        || lower.starts_with("ftp://")
        || lower.starts_with("ssh://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    fn ends_with_either(p: &std::path::Path, backslash_form: &str, slash_form: &str) -> bool {
        let s = p.to_string_lossy();
        s.ends_with(backslash_form) || s.ends_with(slash_form)
    }

    #[cfg(not(windows))]
    fn ends_with_either(p: &std::path::Path, _backslash_form: &str, slash_form: &str) -> bool {
        p.to_string_lossy().ends_with(slash_form)
    }

    #[test]
    fn extracts_windows_drive_letter_path() {
        let paths = extract_absolute_paths(r#"look at C:\Users\me\file.txt please"#);
        assert_eq!(paths.len(), 1);
        assert!(ends_with_either(
            &paths[0],
            r"Users\me\file.txt",
            "Users/me/file.txt"
        ));
    }

    #[test]
    fn extracts_unix_absolute_path() {
        if cfg!(windows) {
            // On Windows, `/var/log/system.log` is not absolute;
            // skip the assertion.
            return;
        }
        let paths = extract_absolute_paths("read /var/log/system.log");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], PathBuf::from("/var/log/system.log"));
    }

    #[test]
    fn extracts_quoted_path_and_strips_quotes() {
        let paths = extract_absolute_paths(r#"open "C:\Users\me\a.txt" for me"#);
        assert_eq!(paths.len(), 1);
        let s = paths[0].to_string_lossy().into_owned();
        assert!(!s.contains('"'), "quotes should be stripped: {s}");
    }

    #[test]
    fn extracts_multiple_paths_in_one_message() {
        if cfg!(windows) {
            return;
        }
        let paths = extract_absolute_paths("compare /tmp/a.txt and /tmp/b.txt");
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn ignores_relative_paths() {
        let paths = extract_absolute_paths("read ./local.txt and ../sibling.txt");
        assert!(paths.is_empty(), "relative paths must not be trusted: {paths:?}");
    }

    #[test]
    fn ignores_urls() {
        let paths = extract_absolute_paths("see https://example.com and http://foo/bar");
        assert!(paths.is_empty());
    }

    #[test]
    fn handles_brace_wrapped_path() {
        if cfg!(windows) {
            return;
        }
        let paths = extract_absolute_paths("(see /var/log/app.log)");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], PathBuf::from("/var/log/app.log"));
    }

    #[test]
    fn handles_comma_separated_path() {
        if cfg!(windows) {
            return;
        }
        let paths = extract_absolute_paths("/tmp/a.txt,/tmp/b.txt");
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn single_quote_path_is_extracted() {
        if cfg!(windows) {
            return;
        }
        let paths = extract_absolute_paths("'/var/data/secret.json'");
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn deduplicates_repeated_paths() {
        if cfg!(windows) {
            return;
        }
        let paths = extract_absolute_paths("/tmp/a.txt and /tmp/a.txt again");
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn empty_input_returns_empty() {
        let paths = extract_absolute_paths("");
        assert!(paths.is_empty());
    }

    #[test]
    fn pure_prose_with_no_paths_returns_empty() {
        let paths = extract_absolute_paths("hello, please summarise the meeting notes");
        assert!(paths.is_empty());
    }

    #[test]
    fn tilde_path_is_expanded_against_home() {
        // Set HOME/USERPROFILE for the duration of the test.
        let prev_home = env::var_os("HOME");
        let prev_profile = env::var_os("USERPROFILE");
        let test_home = if cfg!(windows) {
            env::set_var("USERPROFILE", r"C:\Users\tester");
            env::remove_var("HOME");
            r"C:\Users\tester"
        } else {
            env::set_var("HOME", "/home/tester");
            env::remove_var("USERPROFILE");
            "/home/tester"
        };
        let paths = extract_absolute_paths("read ~/docs/notes.md");
        if let Some(home) = prev_home.as_ref() {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }
        if let Some(profile) = prev_profile.as_ref() {
            env::set_var("USERPROFILE", profile);
        } else {
            env::remove_var("USERPROFILE");
        }
        assert_eq!(paths.len(), 1, "got: {paths:?}");
        let s = paths[0].to_string_lossy();
        if cfg!(windows) {
            assert!(s.starts_with(test_home), "tilde should expand to {test_home}: {s}");
        } else {
            assert!(s.starts_with("/home/tester"), "tilde should expand to home: {s}");
        }
    }

    #[test]
    fn file_url_is_rejected() {
        let paths = extract_absolute_paths("see file:///etc/passwd");
        assert!(paths.is_empty(), "file:// URLs must be excluded: {paths:?}");
    }
}
