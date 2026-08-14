//! Verifies the project-ancestor walk in `discover_agent_roots` stops at the
//! user's home boundary.
//!
//! Regression for the F-2 defect: when the working directory sits *outside*
//! the home directory (e.g. the cwd is a sibling of `~`), the old code
//! compared canonicalized ancestors for exact equality against the canonical
//! home, so it never matched and climbed all the way to the drive root --
//! picking up `.claw/agents` at or above the home as if they were project
//! scope. The walk must stop at any ancestor that is at-or-above home
//! (`home.starts_with(ancestor)`), not just at the exact home path.
//!
//! This test mutates the process environment, so it lives in its own binary
//! and runs as the single test here to avoid cross-test pollution.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn unique_temp_dir() -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time after epoch")
        .as_nanos();
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("agents-home-boundary-{nanos}-{unique}"))
}

#[test]
fn project_walk_stops_at_or_above_home_boundary() {
    let _guard = env_lock();

    // Real (canonicalizable) home with a user agent.
    let base = unique_temp_dir();
    let home = base.join("home");
    let home_agents = home.join(".claw").join("agents");
    std::fs::create_dir_all(&home_agents).expect("home agents dir");
    std::fs::write(home_agents.join("user-agent.md"), "---\nname: user-agent\n---\n").expect("write");

    // Cwd is a *sibling* of home (outside the home boundary): its project
    // agent dir must be discovered, but a decoy `.claw/agents` sitting at the
    // home's parent level must NOT be treated as project scope.
    let project = base.join("project");
    let project_agents = project.join(".claw").join("agents");
    std::fs::create_dir_all(&project_agents).expect("project agents dir");
    std::fs::write(
        project_agents.join("proj-agent.md"),
        "---\nname: proj-agent\n---\n",
    )
    .expect("write");

    let decoy_agents = base.join(".claw").join("agents");
    std::fs::create_dir_all(&decoy_agents).expect("decoy agents dir");
    std::fs::write(decoy_agents.join("decoy.md"), "---\nname: decoy\n---\n").expect("write");

    // Pin the home env vars so the walk has a real boundary, regardless of
    // what the host shell set.
    let saved_home = std::env::var_os("HOME");
    let saved_userprofile = std::env::var_os("USERPROFILE");
    std::env::set_var("HOME", &home);
    std::env::set_var("USERPROFILE", &home);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let roots = agents::discover_agent_roots(&project);
        (roots, project_agents.clone(), decoy_agents.clone())
    }));

    match saved_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
    match saved_userprofile {
        Some(value) => std::env::set_var("USERPROFILE", value),
        None => std::env::remove_var("USERPROFILE"),
    }
    std::fs::remove_dir_all(&base).ok();

    let (roots, project_agents, decoy_agents) =
        result.unwrap_or_else(|payload| std::panic::resume_unwind(payload));

    assert!(
        roots.contains(&project_agents),
        "project-level agent root must be discovered, got: {roots:?}"
    );
    assert!(
        !roots.contains(&decoy_agents),
        "home-parent decoy must NOT be treated as project scope, got: {roots:?}"
    );
}

#[allow(unused_imports)]
use std::sync::MutexGuard;
