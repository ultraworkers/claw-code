//! Verifies that an agent definition's declared `tools:` / `skills:` list is
//! captured into `AgentSummary` so the spawn path can constrain the sub-agent
//! (rather than always granting the full general-purpose write tool set).

use std::path::PathBuf;

fn unique_temp_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time after epoch")
        .as_nanos();
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("agents-tools-{nanos}-{unique}"))
}

#[test]
fn agent_summary_captures_declared_tools_and_skills() {
    let root = unique_temp_dir();
    let agents_dir = root.join(".claw").join("agents");
    std::fs::create_dir_all(&agents_dir).expect("agents dir");
    std::fs::write(
        agents_dir.join("restricted.md"),
        "---\nname: restricted\ndescription: read-only reviewer\nmodel: claude-sonnet-4\ntools: [\"read_file\", \"grep_search\"]\nskills: [\"review\"]\n---\n\nYou review code read-only.\n",
    )
    .expect("write agent file");

    let discovery = agents::AgentDiscovery::new(&root);
    let found = discovery
        .find("restricted")
        .expect("restricted agent should be discovered");

    assert_eq!(
        found.tools.as_deref(),
        Some(&["read_file".to_string(), "grep_search".to_string()][..]),
        "declared tools must be captured on the summary"
    );
    assert_eq!(
        found.skills.as_deref(),
        Some(&["review".to_string()][..]),
        "declared skills must be captured on the summary"
    );

    std::fs::remove_dir_all(root).ok();
}
