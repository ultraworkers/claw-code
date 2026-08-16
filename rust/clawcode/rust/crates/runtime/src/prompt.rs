use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{user_home_dir, ConfigError, ConfigLoader, RuntimeConfig};
use crate::git_context::GitContext;

/// Errors raised while assembling the final system prompt.
#[derive(Debug)]
pub enum PromptBuildError {
    Io(std::io::Error),
    Config(ConfigError),
}

impl std::fmt::Display for PromptBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Config(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for PromptBuildError {}

impl From<std::io::Error> for PromptBuildError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<ConfigError> for PromptBuildError {
    fn from(value: ConfigError) -> Self {
        Self::Config(value)
    }
}

/// Marker separating static prompt scaffolding from dynamic runtime context.
pub const SYSTEM_PROMPT_DYNAMIC_BOUNDARY: &str = "__SYSTEM_PROMPT_DYNAMIC_BOUNDARY__";
/// Human-readable default frontier model name embedded into generated prompts.
pub const FRONTIER_MODEL_NAME: &str = "Claude Opus 4.6";
const MAX_INSTRUCTION_FILE_CHARS: usize = 4_000;
const MAX_TOTAL_INSTRUCTION_CHARS: usize = 12_000;

/// Contents of an instruction file included in prompt construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextFile {
    pub path: PathBuf,
    pub content: String,
}

/// Project-local context injected into the rendered system prompt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectContext {
    pub cwd: PathBuf,
    pub current_date: String,
    pub git_status: Option<String>,
    pub git_diff: Option<String>,
    pub git_context: Option<GitContext>,
    pub instruction_files: Vec<ContextFile>,
}

impl ProjectContext {
    pub fn discover(
        cwd: impl Into<PathBuf>,
        current_date: impl Into<String>,
    ) -> std::io::Result<Self> {
        let cwd = cwd.into();
        let instruction_files = discover_instruction_files(&cwd)?;
        Ok(Self {
            cwd,
            current_date: current_date.into(),
            git_status: None,
            git_diff: None,
            git_context: None,
            instruction_files,
        })
    }

    pub fn discover_with_git(
        cwd: impl Into<PathBuf>,
        current_date: impl Into<String>,
    ) -> std::io::Result<Self> {
        let mut context = Self::discover(cwd, current_date)?;
        context.git_status = read_git_status(&context.cwd);
        context.git_diff = read_git_diff(&context.cwd);
        context.git_context = GitContext::detect(&context.cwd);
        Ok(context)
    }
}

/// Builder for the runtime system prompt and dynamic environment sections.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SystemPromptBuilder {
    output_style_name: Option<String>,
    output_style_prompt: Option<String>,
    os_name: Option<String>,
    os_version: Option<String>,
    append_sections: Vec<String>,
    project_context: Option<ProjectContext>,
    config: Option<RuntimeConfig>,
}

impl SystemPromptBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_output_style(mut self, name: impl Into<String>, prompt: impl Into<String>) -> Self {
        self.output_style_name = Some(name.into());
        self.output_style_prompt = Some(prompt.into());
        self
    }

    #[must_use]
    pub fn with_os(mut self, os_name: impl Into<String>, os_version: impl Into<String>) -> Self {
        self.os_name = Some(os_name.into());
        self.os_version = Some(os_version.into());
        self
    }

    #[must_use]
    pub fn with_project_context(mut self, project_context: ProjectContext) -> Self {
        self.project_context = Some(project_context);
        self
    }

    #[must_use]
    pub fn with_runtime_config(mut self, config: RuntimeConfig) -> Self {
        self.config = Some(config);
        self
    }

    #[must_use]
    pub fn append_section(mut self, section: impl Into<String>) -> Self {
        self.append_sections.push(section.into());
        self
    }

    #[must_use]
    pub fn build(&self) -> Vec<String> {
        let mut sections = Vec::new();
        if let (Some(name), Some(prompt)) = (&self.output_style_name, &self.output_style_prompt) {
            sections.push(format!("# Output Style: {name}\n{prompt}"));
        }
        sections.push(SYSTEM_PROMPT_DYNAMIC_BOUNDARY.to_string());
        sections.push(self.environment_section());
        if let Some(project_context) = &self.project_context {
            sections.push(render_project_context(project_context));
            if !project_context.instruction_files.is_empty() {
                sections.push(render_instruction_files(&project_context.instruction_files));
            }
        }
        if let Some(config) = &self.config {
            sections.push(render_config_section(config));
        }
        sections.extend(self.append_sections.iter().cloned());
        sections
    }

    #[must_use]
    pub fn render(&self) -> String {
        self.build().join("\n\n")
    }

    fn environment_section(&self) -> String {
        let mut lines = vec!["# Environment".to_string()];
        lines.extend(prepend_bullets(vec![
            format!("Model family: {FRONTIER_MODEL_NAME}"),
            format!(
                "Platform: {} {}",
                self.os_name.as_deref().unwrap_or("unknown"),
                self.os_version.as_deref().unwrap_or("unknown")
            ),
        ]));
        lines.join("\n")
    }
}

/// Formats each item as an indented bullet for prompt sections.
#[must_use]
pub fn prepend_bullets(items: Vec<String>) -> Vec<String> {
    items.into_iter().map(|item| format!(" - {item}")).collect()
}

fn discover_instruction_files(cwd: &Path) -> std::io::Result<Vec<ContextFile>> {
    // Single-file resolution: project-local first, then global fallback.
    // Return the first candidate found; stop immediately.
    let try_one = |path: PathBuf| -> std::io::Result<Option<ContextFile>> {
        push_context_file_only(&path).map(|opt| opt.map(|content| ContextFile { path, content }))
    };

    // Project-local candidates
    for candidate in [
        cwd.join("CLAUDE.md"),
        cwd.join(".claw").join("CLAUDE.md"),
        cwd.join(".claude").join("CLAUDE.md"),
    ] {
        if let Some(file) = try_one(candidate)? {
            return Ok(vec![file]);
        }
    }

    // Global fallback candidates (user home)
    if let Some(home) = user_home_dir() {
        for candidate in [
            home.join(".claw").join("CLAUDE.md"),
            home.join(".claude").join("CLAUDE.md"),
        ] {
            if let Some(file) = try_one(candidate)? {
                return Ok(vec![file]);
            }
        }
    }

    Ok(Vec::new())
}

/// Read a single file if it exists and is non-empty.
fn push_context_file_only(path: &Path) -> std::io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(content) if !content.trim().is_empty() => Ok(Some(content)),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn read_git_status(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["--no-optional-locks", "status", "--short", "--branch"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn read_git_diff(cwd: &Path) -> Option<String> {
    let mut sections = Vec::new();

    let staged = read_git_output(cwd, &["diff", "--cached"])?;
    if !staged.trim().is_empty() {
        sections.push(format!("Staged changes:\n{}", staged.trim_end()));
    }

    let unstaged = read_git_output(cwd, &["diff"])?;
    if !unstaged.trim().is_empty() {
        sections.push(format!("Unstaged changes:\n{}", unstaged.trim_end()));
    }

    if sections.is_empty() {
        None
    } else {
        Some(sections.join("\n\n"))
    }
}

fn read_git_output(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn render_project_context(project_context: &ProjectContext) -> String {
    let mut lines = vec!["# Project context".to_string()];
    let mut bullets = vec![
        format!("Today's date is {}.", project_context.current_date),
        format!("Working directory: {}", project_context.cwd.display()),
    ];
    if !project_context.instruction_files.is_empty() {
        bullets.push(format!(
            "Claude instruction files discovered: {}.",
            project_context.instruction_files.len()
        ));
    }
    lines.extend(prepend_bullets(bullets));
    if let Some(status) = &project_context.git_status {
        lines.push(String::new());
        lines.push("Git status snapshot:".to_string());
        lines.push(status.clone());
    }
    // `GitContext::render()` below already emits branch + recent commits +
    // staged files. Rendering `recent_commits` again here duplicated the same
    // five commits into the system prompt on every turn.
    if let Some(git_context) = &project_context.git_context {
        let rendered = git_context.render();
        if !rendered.is_empty() {
            lines.push(String::new());
            lines.push(rendered);
        }
    }
    lines.join("\n")
}

fn render_instruction_files(files: &[ContextFile]) -> String {
    let mut sections = vec!["# Claude instructions".to_string()];
    let mut remaining_chars = MAX_TOTAL_INSTRUCTION_CHARS;
    for file in files {
        if remaining_chars == 0 {
            sections.push(
                "_Additional instruction content omitted after reaching the prompt budget._"
                    .to_string(),
            );
            break;
        }

        let raw_content = truncate_instruction_content(&file.content, remaining_chars);
        let rendered_content = render_instruction_content(&raw_content);
        let consumed = rendered_content.chars().count().min(remaining_chars);
        remaining_chars = remaining_chars.saturating_sub(consumed);

        sections.push(format!("## {}", describe_instruction_file(file, files)));
        sections.push(rendered_content);
    }
    sections.join("\n\n")
}

#[allow(dead_code)]
fn normalize_instruction_content(content: &str) -> String {
    collapse_blank_lines(content).trim().to_string()
}

fn describe_instruction_file(file: &ContextFile, files: &[ContextFile]) -> String {
    let path = display_context_path(&file.path);
    let scope = files
        .iter()
        .filter_map(|candidate| candidate.path.parent())
        .find(|parent| file.path.starts_with(parent))
        .map_or_else(
            || "workspace".to_string(),
            |parent| parent.display().to_string(),
        );
    format!("{path} (scope: {scope})")
}

fn truncate_instruction_content(content: &str, remaining_chars: usize) -> String {
    let hard_limit = MAX_INSTRUCTION_FILE_CHARS.min(remaining_chars);
    let trimmed = content.trim();
    if trimmed.chars().count() <= hard_limit {
        return trimmed.to_string();
    }

    let mut output = trimmed.chars().take(hard_limit).collect::<String>();
    output.push_str("\n\n[truncated]");
    output
}

fn render_instruction_content(content: &str) -> String {
    truncate_instruction_content(content, MAX_INSTRUCTION_FILE_CHARS)
}

fn display_context_path(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[allow(dead_code)]
fn collapse_blank_lines(content: &str) -> String {
    let mut result = String::new();
    let mut previous_blank = false;
    for line in content.lines() {
        let is_blank = line.trim().is_empty();
        if is_blank && previous_blank {
            continue;
        }
        result.push_str(line.trim_end());
        result.push('\n');
        previous_blank = is_blank;
    }
    result
}

/// Loads config and project context, then renders the system prompt text.
pub fn load_system_prompt(
    cwd: impl Into<PathBuf>,
    current_date: impl Into<String>,
    os_name: impl Into<String>,
    os_version: impl Into<String>,
) -> Result<Vec<String>, PromptBuildError> {
    let cwd = cwd.into();
    let project_context = ProjectContext::discover_with_git(&cwd, current_date.into())?;
    let config = ConfigLoader::default_for(&cwd).load()?;
    Ok(SystemPromptBuilder::new()
        .with_os(os_name, os_version)
        .with_project_context(project_context)
        .with_runtime_config(config)
        .build())
}

fn render_config_section(config: &RuntimeConfig) -> String {
    let mut lines = vec!["# Runtime config".to_string()];
    if config.loaded_entries().is_empty() {
        lines.extend(prepend_bullets(vec![
            "No Claw Code settings files loaded.".to_string()
        ]));
    } else {
        lines.extend(prepend_bullets(
            config
                .loaded_entries()
                .iter()
                .map(|entry| format!("Loaded {:?}: {}", entry.source, entry.path.display()))
                .collect(),
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        collapse_blank_lines, display_context_path, normalize_instruction_content,
        render_instruction_content, render_instruction_files, truncate_instruction_content,
        ContextFile, ProjectContext, SystemPromptBuilder, SYSTEM_PROMPT_DYNAMIC_BOUNDARY,
    };
    use crate::config::ConfigLoader;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("runtime-prompt-{nanos}"))
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        crate::test_env_lock()
    }

    fn ensure_valid_cwd() {
        if std::env::current_dir().is_err() {
            std::env::set_current_dir(env!("CARGO_MANIFEST_DIR"))
                .expect("test cwd should be recoverable");
        }
    }

    #[test]
    fn discovers_instruction_files_from_ancestor_chain() {
        let root = temp_dir();
        let nested = root.join("apps").join("api");
        fs::create_dir_all(nested.join(".claw")).expect("nested claw dir");
        fs::write(root.join("CLAUDE.md"), "root instructions").expect("write root instructions");
        fs::write(root.join("CLAUDE.local.md"), "local instructions")
            .expect("write local instructions");
        fs::create_dir_all(root.join("apps")).expect("apps dir");
        fs::create_dir_all(root.join("apps").join(".claw")).expect("apps claw dir");
        fs::write(root.join("apps").join("CLAUDE.md"), "apps instructions")
            .expect("write apps instructions");
        fs::write(
            root.join("apps").join(".claw").join("CLAUDE.md"),
            "apps claw claude md",
        )
        .expect("write apps claw claude md");
        fs::write(nested.join(".claw").join("CLAUDE.md"), "nested claw claude md")
            .expect("write nested claw claude md");

        let context = ProjectContext::discover(&nested, "2026-03-31").expect("context should load");
        let contents = context
            .instruction_files
            .iter()
            .map(|file| file.content.as_str())
            .collect::<Vec<_>>();

        // New logic: only the first candidate in project root is loaded.
        // cwd = root/apps/api → api/CLAUDE.md missing, api/.claw/CLAUDE.md found.
        assert_eq!(contents, vec!["nested claw claude md"]);
        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn discovers_claw_and_claude_rules_recursively() {
        let root = temp_dir();
        let nested = root.join("sub").join("project");
        fs::create_dir_all(nested.join(".claude").join("rules").join("subdir")).expect("rules dir");
        fs::create_dir_all(nested.join(".claw").join("rules")).expect("claw rules dir");
        fs::write(
            nested.join(".claude").join("CLAUDE.md"),
            "dot claude instructions",
        )
        .expect("write dot claude instructions");
        fs::write(
            nested.join(".claw").join("CLAUDE.md"),
            "dot claw instructions",
        )
        .expect("write dot claw instructions");
        fs::write(
            nested.join(".claude").join("rules").join("react.md"),
            "# React rules\nAlways use hooks",
        )
        .expect("write react rule");
        fs::write(
            nested.join(".claude").join("rules").join("subdir").join("deep.md"),
            "deep rule in subdir",
        )
        .expect("write deep rule");
        fs::write(
            nested.join(".claw").join("rules").join("claw-style.md"),
            "claw style rule",
        )
        .expect("write claw style rule");

        let context = ProjectContext::discover(&nested, "2026-03-31").expect("context should load");
        let contents: Vec<&str> = context
            .instruction_files
            .iter()
            .map(|file| file.content.as_str())
            .collect();

        // New logic: only the first candidate found in project root.
        // CLAUDE.md missing, .claw/CLAUDE.md found first → "dot claw instructions".
        assert_eq!(contents, vec!["dot claw instructions"]);

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn dedupes_identical_instruction_content_across_scopes() {
        let root = temp_dir();
        let nested = root.join("apps").join("api");
        fs::create_dir_all(&nested).expect("nested dir");
        fs::write(root.join("CLAUDE.md"), "same rules\n\n").expect("write root");
        fs::write(nested.join("CLAUDE.md"), "same rules\n").expect("write nested");

        let context = ProjectContext::discover(&nested, "2026-03-31").expect("context should load");
        assert_eq!(context.instruction_files.len(), 1);
        assert_eq!(
            normalize_instruction_content(&context.instruction_files[0].content),
            "same rules"
        );
        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn truncates_large_instruction_content_for_rendering() {
        let rendered = render_instruction_content(&"x".repeat(4500));
        assert!(rendered.contains("[truncated]"));
        assert!(rendered.len() < 4_100);
    }

    #[test]
    fn normalizes_and_collapses_blank_lines() {
        let normalized = normalize_instruction_content("line one\n\n\nline two\n");
        assert_eq!(normalized, "line one\n\nline two");
        assert_eq!(collapse_blank_lines("a\n\n\n\nb\n"), "a\n\nb\n");
    }

    #[test]
    fn displays_context_paths_compactly() {
        assert_eq!(
            display_context_path(Path::new("/tmp/project/.claw/CLAUDE.md")),
            "CLAUDE.md"
        );
    }

    #[test]
    fn discover_with_git_includes_status_snapshot() {
        let _guard = env_lock();
        ensure_valid_cwd();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&root)
            .status()
            .expect("git init should run");
        std::process::Command::new("git")
            .args(["config", "core.autocrlf", "false"])
            .current_dir(&root)
            .status()
            .expect("git config autocrlf should run");
        fs::write(root.join("CLAUDE.md"), "rules").expect("write instructions");
        fs::write(root.join("tracked.txt"), "hello").expect("write tracked file");

        let context =
            ProjectContext::discover_with_git(&root, "2026-03-31").expect("context should load");

        let status = context.git_status.expect("git status should be present");
        assert!(status.contains("## No commits yet on") || status.contains("## "));
        assert!(status.contains("?? CLAUDE.md"));
        assert!(status.contains("?? tracked.txt"));
        assert!(context.git_diff.is_none());

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn discover_with_git_includes_recent_commits_and_renders_them() {
        // given: a git repo with three commits and a current branch
        let _guard = env_lock();
        ensure_valid_cwd();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        std::process::Command::new("git")
            .args(["init", "--quiet", "-b", "main"])
            .current_dir(&root)
            .status()
            .expect("git init should run");
        std::process::Command::new("git")
            .args(["config", "core.autocrlf", "false"])
            .current_dir(&root)
            .status()
            .expect("git config autocrlf should run");
        std::process::Command::new("git")
            .args(["config", "user.email", "tests@example.com"])
            .current_dir(&root)
            .status()
            .expect("git config email should run");
        std::process::Command::new("git")
            .args(["config", "user.name", "Runtime Prompt Tests"])
            .current_dir(&root)
            .status()
            .expect("git config name should run");
        for (file, message) in [
            ("a.txt", "first commit"),
            ("b.txt", "second commit"),
            ("c.txt", "third commit"),
        ] {
            fs::write(root.join(file), "x\n").expect("write commit file");
            std::process::Command::new("git")
                .args(["add", file])
                .current_dir(&root)
                .status()
                .expect("git add should run");
            std::process::Command::new("git")
                .args(["commit", "-m", message, "--quiet"])
                .current_dir(&root)
                .status()
                .expect("git commit should run");
        }
        fs::write(root.join("d.txt"), "staged\n").expect("write staged file");
        std::process::Command::new("git")
            .args(["add", "d.txt"])
            .current_dir(&root)
            .status()
            .expect("git add staged should run");

        // when: discovering project context with git auto-include
        let context =
            ProjectContext::discover_with_git(&root, "2026-03-31").expect("context should load");
        let rendered = SystemPromptBuilder::new()
            .with_os("linux", "6.8")
            .with_project_context(context.clone())
            .render();

        // then: branch, recent commits and staged files are present in context
        let gc = context
            .git_context
            .as_ref()
            .expect("git context should be present");
        let commits: String = gc
            .recent_commits
            .iter()
            .map(|c| c.subject.clone())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(commits.contains("first commit"));
        assert!(commits.contains("second commit"));
        assert!(commits.contains("third commit"));
        assert_eq!(gc.recent_commits.len(), 3);

        let status = context.git_status.as_deref().expect("status snapshot");
        assert!(status.contains("## main"));
        assert!(status.contains("A  d.txt"));

        assert!(rendered.contains("Git branch:"));
        assert!(rendered.contains("Recent commits:"));
        assert!(rendered.contains("first commit"));
        assert!(rendered.contains("Git status snapshot:"));
        assert!(rendered.contains("## main"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn discover_with_git_includes_diff_snapshot_for_tracked_changes() {
        let _guard = env_lock();
        ensure_valid_cwd();
        let root = temp_dir();
        fs::create_dir_all(&root).expect("root dir");
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&root)
            .status()
            .expect("git init should run");
        std::process::Command::new("git")
            .args(["config", "core.autocrlf", "false"])
            .current_dir(&root)
            .status()
            .expect("git config autocrlf should run");
        std::process::Command::new("git")
            .args(["config", "user.email", "tests@example.com"])
            .current_dir(&root)
            .status()
            .expect("git config email should run");
        std::process::Command::new("git")
            .args(["config", "user.name", "Runtime Prompt Tests"])
            .current_dir(&root)
            .status()
            .expect("git config name should run");
        fs::write(root.join("tracked.txt"), "hello\n").expect("write tracked file");
        std::process::Command::new("git")
            .args(["add", "tracked.txt"])
            .current_dir(&root)
            .status()
            .expect("git add should run");
        std::process::Command::new("git")
            .args(["commit", "-m", "init", "--quiet"])
            .current_dir(&root)
            .status()
            .expect("git commit should run");
        fs::write(root.join("tracked.txt"), "hello\nworld\n").expect("rewrite tracked file");

        let context =
            ProjectContext::discover_with_git(&root, "2026-03-31").expect("context should load");

        let diff = context.git_diff.expect("git diff should be present");
        assert!(diff.contains("Unstaged changes:"));
        assert!(diff.contains("tracked.txt"));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn load_system_prompt_reads_claude_files_and_config() {
        let root = temp_dir();
        fs::create_dir_all(root.join(".claw")).expect("claw dir");
        fs::write(root.join("CLAUDE.md"), "Project rules").expect("write instructions");
        fs::write(
            root.join(".claw").join("settings.json"),
            r#"{"permissionMode":"acceptEdits"}"#,
        )
        .expect("write settings");

        let _guard = env_lock();
        ensure_valid_cwd();
        let previous = std::env::current_dir().expect("cwd");
        let original_home = std::env::var("HOME").ok();
        let original_claw_home = std::env::var("CLAW_CONFIG_HOME").ok();
        std::env::set_var("HOME", &root);
        std::env::set_var("CLAW_CONFIG_HOME", root.join("missing-home"));
        std::env::set_current_dir(&root).expect("change cwd");
        let prompt = super::load_system_prompt(&root, "2026-03-31", "linux", "6.8")
            .expect("system prompt should load")
            .join(
                "

",
            );
        std::env::set_current_dir(previous).expect("restore cwd");
        if let Some(value) = original_home {
            std::env::set_var("HOME", value);
        } else {
            std::env::remove_var("HOME");
        }
        if let Some(value) = original_claw_home {
            std::env::set_var("CLAW_CONFIG_HOME", value);
        } else {
            std::env::remove_var("CLAW_CONFIG_HOME");
        }

        assert!(prompt.contains("Project rules"));
        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn renders_claude_code_style_sections_with_project_context() {
        let root = temp_dir();
        fs::create_dir_all(root.join(".claw")).expect("claw dir");
        fs::write(root.join("CLAUDE.md"), "Project rules").expect("write CLAUDE.md");
        fs::write(
            root.join(".claw").join("settings.json"),
            r#"{"permissionMode":"acceptEdits"}"#,
        )
        .expect("write settings");

        let project_context =
            ProjectContext::discover(&root, "2026-03-31").expect("context should load");
        let config = ConfigLoader::new(&root, root.join("missing-home"))
            .load()
            .expect("config should load");
        let prompt = SystemPromptBuilder::new()
            .with_output_style("Concise", "Prefer short answers.")
            .with_os("linux", "6.8")
            .with_project_context(project_context)
            .with_runtime_config(config)
            .render();

        assert!(prompt.contains("# Project context"));
        assert!(prompt.contains("# Claude instructions"));
        assert!(prompt.contains("Project rules"));
        assert!(prompt.contains(SYSTEM_PROMPT_DYNAMIC_BOUNDARY));

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn truncates_instruction_content_to_budget() {
        let content = "x".repeat(5_000);
        let rendered = truncate_instruction_content(&content, 4_000);
        assert!(rendered.contains("[truncated]"));
        assert!(rendered.chars().count() <= 4_000 + "\n\n[truncated]".chars().count());
    }

    #[test]
    fn discovers_claw_rules_markdown() {
        let root = temp_dir();
        let nested = root.join("apps").join("api");
        fs::create_dir_all(nested.join(".claw").join("rules")).expect("nested claw rules dir");
        fs::write(
            nested.join(".claw").join("rules").join("lint.md"),
            "lint rules",
        )
        .expect("write lint.md");

        let context = ProjectContext::discover(&nested, "2026-03-31").expect("context should load");
        // New logic: rules directories are no longer loaded; no CLAUDE.md in project
        // root → at most one global fallback (~/.claw/CLAUDE.md or ~/.claude/CLAUDE.md).
        assert!(context.instruction_files.len() <= 1);
        if let Some(file) = context.instruction_files.first() {
            assert!(
                file.path.ends_with("CLAUDE.md"),
                "global fallback should be CLAUDE.md"
            );
            assert!(!file.content.trim().is_empty());
        }

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[test]
    fn renders_instruction_file_metadata() {
        let rendered = render_instruction_files(&[ContextFile {
            path: PathBuf::from("/tmp/project/CLAUDE.md"),
            content: "Project rules".to_string(),
        }]);
        assert!(rendered.contains("# Claude instructions"));
        assert!(rendered.contains("scope: /tmp/project"));
        assert!(rendered.contains("Project rules"));
    }
}
