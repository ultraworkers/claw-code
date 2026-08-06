//! Scratch probe: dump GitHub runner unshare semantics (temporary, PRs will be closed).
#![cfg(target_os = "linux")]

use std::process::Command;

fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new("unshare").args(args).output();
    match out {
        Ok(o) => (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).trim().to_string(),
            String::from_utf8_lossy(&o.stderr).trim().to_string(),
        ),
        Err(e) => (-1, String::new(), format!("spawn error: {e}")),
    }
}

fn sh(cmd: &str) -> String {
    Command::new("sh")
        .args(["-lc", cmd])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

#[test]
fn dump_unshare_semantics() {
    let mut report = String::new();
    report.push_str(&format!("uid line: {}\n", sh("id")));
    for f in ["/etc/subuid", "/etc/subgid"] {
        report.push_str(&format!("--- {f} ---\n"));
        if let Ok(s) = std::fs::read_to_string(f) {
            report.push_str(&s);
        } else {
            report.push_str("(unreadable)\n");
        }
    }
    for k in ["/proc/sys/kernel/unprivileged_userns_clone", "/proc/sys/kernel/apparmor_restrict_unprivileged_userns"] {
        report.push_str(&format!("{k} = {}\n", std::fs::read_to_string(k).unwrap_or_else(|_| "(n/a)".into())));
    }
    for (name, args) in [
        ("plain", &["--user", "--map-root-user", "true"][..]),
        ("auto", &["--user", "--map-root-user", "--map-auto", "true"][..]),
        (
            "plain-full",
            &["--user", "--map-root-user", "--mount", "--ipc", "--pid", "--uts", "--fork", "sh", "-lc", "echo alpha"][..],
        ),
        (
            "auto-full",
            &["--user", "--map-root-user", "--map-auto", "--mount", "--ipc", "--pid", "--uts", "--fork", "sh", "-lc", "echo alpha"][..],
        ),
        (
            "auto-full-echo-multi",
            &["--user", "--map-root-user", "--map-auto", "--mount", "--ipc", "--pid", "--uts", "--fork", "sh", "-lc", "echo alpha from bash"][..],
        ),
    ] {
        let (rc, so, se) = run(args);
        report.push_str(&format!("[{name}] rc={rc} stdout={so:?} stderr={se:?}\n"));
    }
    panic!("PROBE REPORT:\n{report}");
}
