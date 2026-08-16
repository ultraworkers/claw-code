#![allow(clippy::must_use_candidate)]
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchFreshness {
    Fresh,
    Stale {
        commits_behind: usize,
        missing_fixes: Vec<String>,
    },
    Diverged {
        ahead: usize,
        behind: usize,
        missing_fixes: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleBranchPolicy {
    AutoRebase,
    AutoMergeForward,
    WarnOnly,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaleBranchEvent {
    BranchStaleAgainstMain {
        branch: String,
        commits_behind: usize,
        missing_fixes: Vec<String>,
    },
    RebaseAttempted {
        branch: String,
        result: String,
    },
    MergeForwardAttempted {
        branch: String,
        result: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaleBranchAction {
    Noop,
    Warn { message: String },
    Block { message: String },
    Rebase,
    MergeForward,
}

pub fn check_freshness(branch: &str, main_ref: &str) -> BranchFreshness {
    check_freshness_in(branch, main_ref, Path::new("."))
}

pub fn apply_policy(freshness: &BranchFreshness, policy: StaleBranchPolicy) -> StaleBranchAction {
    match freshness {
        BranchFreshness::Fresh => StaleBranchAction::Noop,
        BranchFreshness::Stale {
            commits_behind,
            missing_fixes,
        } => match policy {
            StaleBranchPolicy::WarnOnly => StaleBranchAction::Warn {
                message: format!(
                    "Branch is {commits_behind} commit(s) behind main. Missing fixes: {}",
                    if missing_fixes.is_empty() {
                        "(none)".to_string()
                    } else {
                        missing_fixes.join("; ")
                    }
                ),
            },
            StaleBranchPolicy::Block => StaleBranchAction::Block {
                message: format!(
                    "Branch is {commits_behind} commit(s) behind main and must be updated before proceeding."
                ),
            },
            StaleBranchPolicy::AutoRebase => StaleBranchAction::Rebase,
            StaleBranchPolicy::AutoMergeForward => StaleBranchAction::MergeForward,
        },
        BranchFreshness::Diverged {
            ahead,
            behind,
            missing_fixes,
        } => match policy {
            StaleBranchPolicy::WarnOnly => StaleBranchAction::Warn {
                message: format!(
                    "Branch has diverged: {ahead} commit(s) ahead, {behind} commit(s) behind main. Missing fixes: {}",
                    format_missing_fixes(missing_fixes)
                ),
            },
            StaleBranchPolicy::Block => StaleBranchAction::Block {
                message: format!(
                    "Branch has diverged ({ahead} ahead, {behind} behind) and must be reconciled before proceeding. Missing fixes: {}",
                    format_missing_fixes(missing_fixes)
                ),
            },
            StaleBranchPolicy::AutoRebase => StaleBranchAction::Rebase,
            StaleBranchPolicy::AutoMergeForward => StaleBranchAction::MergeForward,
        },
    }
}

pub(crate) fn check_freshness_in(
    branch: &str,
    main_ref: &str,
    repo_path: &Path,
) -> BranchFreshness {
    let behind = rev_list_count(main_ref, branch, repo_path);
    let ahead = rev_list_count(branch, main_ref, repo_path);

    if behind == 0 {
        return BranchFreshness::Fresh;
    }

    if ahead > 0 {
        return BranchFreshness::Diverged {
            ahead,
            behind,
            missing_fixes: missing_fix_subjects(main_ref, branch, repo_path),
        };
    }

    let missing_fixes = missing_fix_subjects(main_ref, branch, repo_path);
    BranchFreshness::Stale {
        commits_behind: behind,
        missing_fixes,
    }
}

fn format_missing_fixes(missing_fixes: &[String]) -> String {
    if missing_fixes.is_empty() {
        "(none)".to_string()
    } else {
        missing_fixes.join("; ")
    }
}

fn rev_list_count(a: &str, b: &str, repo_path: &Path) -> usize {
    let output = Command::new("git")
        .args(["rev-list", "--count", &format!("{b}..{a}")])
        .current_dir(repo_path)
        .output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .trim()
            .parse::<usize>()
            .unwrap_or(0),
        _ => 0,
    }
}

fn missing_fix_subjects(a: &str, b: &str, repo_path: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["log", "--format=%s", &format!("{b}..{a}")])
        .current_dir(repo_path)
        .output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("runtime-stale-branch-{nanos}"))
    }

    fn init_repo(path: &Path) {
        fs::create_dir_all(path).expect("create repo dir");
        run(path, &["init", "--quiet", "-b", "main"]);
        run(path, &["config", "core.autocrlf", "false"]);
        run(path, &["config", "user.email", "tests@example.com"]);
        run(path, &["config", "user.name", "Stale Branch Tests"]);
        fs::write(path.join("init.txt"), "initial\n").expect("write init file");
        run(path, &["add", "."]);
        run(path, &["commit", "-m", "initial commit", "--quiet"]);
    }

    fn run(cwd: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .unwrap_or_else(|e| panic!("git {} failed to execute: {e}", args.join(" ")));
        assert!(
            status.success(),
            "git {} exited with {status}",
            args.join(" ")
        );
    }

    fn commit_file(repo: &Path, name: &str, msg: &str) {
        fs::write(repo.join(name), format!("{msg}\n")).expect("write file");
        run(repo, &["add", name]);
        run(repo, &["commit", "-m", msg, "--quiet"]);
    }

    #[test]
    fn fresh_branch_passes() {
        let root = temp_dir();
        init_repo(&root);

        // given
        run(&root, &["checkout", "-b", "topic"]);

        // when
        let freshness = check_freshness_in("topic", "main", &root);

        // then
        assert_eq!(freshness, BranchFreshness::Fresh);

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn fresh_branch_ahead_of_main_still_fresh() {
        let root = temp_dir();
        init_repo(&root);

        // given
        run(&root, &["checkout", "-b", "topic"]);
        commit_file(&root, "feature.txt", "add feature");

        // when
        let freshness = check_freshness_in("topic", "main", &root);

        // then
        assert_eq!(freshness, BranchFreshness::Fresh);

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn stale_branch_detected_with_correct_behind_count_and_missing_fixes() {
        let root = temp_dir();
        init_repo(&root);

        // given
        run(&root, &["checkout", "-b", "topic"]);
        run(&root, &["checkout", "main"]);
        commit_file(&root, "fix1.txt", "fix: resolve timeout");
        commit_file(&root, "fix2.txt", "fix: handle null pointer");

        // when
        let freshness = check_freshness_in("topic", "main", &root);

        // then
        match freshness {
            BranchFreshness::Stale {
                commits_behind,
                missing_fixes,
            } => {
                assert_eq!(commits_behind, 2);
                assert_eq!(missing_fixes.len(), 2);
                assert_eq!(missing_fixes[0], "fix: handle null pointer");
                assert_eq!(missing_fixes[1], "fix: resolve timeout");
            }
            other => panic!("expected Stale, got {other:?}"),
        }

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn diverged_branch_detection() {
        let root = temp_dir();
        init_repo(&root);

        // given
        run(&root, &["checkout", "-b", "topic"]);
        commit_file(&root, "topic_work.txt", "topic work");
        run(&root, &["checkout", "main"]);
        commit_file(&root, "main_fix.txt", "main fix");

        // when
        let freshness = check_freshness_in("topic", "main", &root);

        // then
        match freshness {
            BranchFreshness::Diverged {
                ahead,
                behind,
                missing_fixes,
            } => {
                assert_eq!(ahead, 1);
                assert_eq!(behind, 1);
                assert_eq!(missing_fixes, vec!["main fix".to_string()]);
            }
            other => panic!("expected Diverged, got {other:?}"),
        }

        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn policy_noop_for_fresh_branch() {
        // given
        let freshness = BranchFreshness::Fresh;

        // when
        let action = apply_policy(&freshness, StaleBranchPolicy::WarnOnly);

        // then
        assert_eq!(action, StaleBranchAction::Noop);
    }

    #[test]
    fn policy_warn_for_stale_branch() {
        // given
        let freshness = BranchFreshness::Stale {
            commits_behind: 3,
            missing_fixes: vec!["fix: timeout".into(), "fix: null ptr".into()],
        };

        // when
        let action = apply_policy(&freshness, StaleBranchPolicy::WarnOnly);

        // then
        match action {
            StaleBranchAction::Warn { message } => {
                assert!(message.contains("3 commit(s) behind"));
                assert!(message.contains("fix: timeout"));
                assert!(message.contains("fix: null ptr"));
            }
            other => panic!("expected Warn, got {other:?}"),
        }
    }

    #[test]
    fn policy_block_for_stale_branch() {
        // given
        let freshness = BranchFreshness::Stale {
            commits_behind: 1,
            missing_fixes: vec!["hotfix".into()],
        };

        // when
        let action = apply_policy(&freshness, StaleBranchPolicy::Block);

        // then
        match action {
            StaleBranchAction::Block { message } => {
                assert!(message.contains("1 commit(s) behind"));
            }
            other => panic!("expected Block, got {other:?}"),
        }
    }

    #[test]
    fn policy_auto_rebase_for_stale_branch() {
        // given
        let freshness = BranchFreshness::Stale {
            commits_behind: 2,
            missing_fixes: vec![],
        };

        // when
        let action = apply_policy(&freshness, StaleBranchPolicy::AutoRebase);

        // then
        assert_eq!(action, StaleBranchAction::Rebase);
    }

    #[test]
    fn policy_auto_merge_forward_for_diverged_branch() {
        // given
        let freshness = BranchFreshness::Diverged {
            ahead: 5,
            behind: 2,
            missing_fixes: vec!["fix: merge main".into()],
        };

        // when
        let action = apply_policy(&freshness, StaleBranchPolicy::AutoMergeForward);

        // then
        assert_eq!(action, StaleBranchAction::MergeForward);
    }

    #[test]
    fn policy_warn_for_diverged_branch() {
        // given
        let freshness = BranchFreshness::Diverged {
            ahead: 3,
            behind: 1,
            missing_fixes: vec!["main hotfix".into()],
        };

        // when
        let action = apply_policy(&freshness, StaleBranchPolicy::WarnOnly);

        // then
        match action {
            StaleBranchAction::Warn { message } => {
                assert!(message.contains("diverged"));
                assert!(message.contains("3 commit(s) ahead"));
                assert!(message.contains("1 commit(s) behind"));
                assert!(message.contains("main hotfix"));
            }
            other => panic!("expected Warn, got {other:?}"),
        }
    }
}
