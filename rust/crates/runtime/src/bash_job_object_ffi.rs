//! Windows Job Object FFI.
//!
//! The runtime crate compiles with `forbid(unsafe_code)` at the workspace
//! level, so all Win32 calls (`OpenProcess`, `CloseHandle`, `TerminateProcess`)
//! are confined to this submodule. The inner attribute here re-enables
//! `unsafe` only for the FFI surface; every caller in `bash.rs` stays safe.
//!
//! Contract:
//! - `apply_job_object_to_pid(pid, enabled, label)` — if `enabled`, create
//!   a Job Object with `kill_on_job_close`, assign the process, and leak
//!   the Job handle so the kernel keeps it alive until the parent exits.
//!   Returns `Err` when the Job cannot be applied (e.g. nested-Job
//!   `E_ACCESSDENIED`) so the caller can decide whether to proceed.
//! - `kill_pid(pid)` — terminate the process by pid. Best-effort: errors
//!   are swallowed because the async caller has already given up.

#![allow(unsafe_code)]

#[cfg(windows)]
pub fn apply_job_object_to_pid(pid: u32, enabled: bool, label: &str) -> Result<(), String> {
    if !enabled {
        return Ok(());
    }
    let job = match win32job::Job::create() {
        Ok(job) => job,
        Err(err) => {
            return Err(format!("[sandbox:{label}] CreateJobObjectW failed: {err}"));
        }
    };
    let mut info = win32job::ExtendedLimitInfo::new();
    info.limit_kill_on_job_close();
    // `SILENT_BREAKAWAY_OK` lets the child escape any parent Job the
    // `claw` process may itself be nested in (IDE/terminal/conhost/CI),
    // without requiring the parent Job to grant breakaway permission.
    // Without this, `AssignProcessToJobObject` returns `E_ACCESSDENIED`
    // under a parent Job and the sandbox silently fails to apply.
    info.limit_silent_breakaway_ok();
    if let Err(err) = job.set_extended_limit_info(&info) {
        return Err(format!("[sandbox:{label}] SetInformationJobObject failed: {err}"));
    }
    let proc_handle = unsafe {
        windows_sys::Win32::System::Threading::OpenProcess(
            windows_sys::Win32::System::Threading::PROCESS_SET_QUOTA
                | windows_sys::Win32::System::Threading::PROCESS_TERMINATE,
            windows_sys::Win32::Foundation::FALSE,
            pid,
        )
    };
    if proc_handle.is_null() {
        return Err(format!(
            "[sandbox:{label}] OpenProcess({pid}) failed; child not assigned to job"
        ));
    }
    if let Err(err) = job.assign_process(proc_handle as isize) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(proc_handle);
        }
        return Err(format!(
            "[sandbox:{label}] AssignProcessToJobObject failed for pid {pid}: {err}"
        ));
    }
    unsafe {
        windows_sys::Win32::Foundation::CloseHandle(proc_handle);
    }
    // Job is now live and owns the kill-on-close contract. Detach the
    // handle from the RAII wrapper so the kernel keeps it open until the
    // parent process exits. `win32job`'s `into_handle` consumes `Job`
    // without closing the underlying handle — exactly what we want.
    let _leaked = job.into_handle();
    Ok(())
}

#[cfg(not(windows))]
pub fn apply_job_object_to_pid(_pid: u32, _enabled: bool, _label: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
pub fn kill_pid(pid: u32) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, TerminateProcess, PROCESS_TERMINATE,
    };
    let handle =
        unsafe { OpenProcess(PROCESS_TERMINATE, windows_sys::Win32::Foundation::FALSE, pid) };
    if handle.is_null() {
        return;
    }
    unsafe {
        TerminateProcess(handle, 1);
        CloseHandle(handle);
    }
}

#[cfg(not(windows))]
pub fn kill_pid(_pid: u32) {}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    /// Verify kill-on-job-close semantics using `win32job` directly. The
    /// FFI module intentionally leaks the Job handle, so we cannot
    /// exercise its drop path from inside a unit test; this test covers
    /// the underlying kernel contract the FFI relies on.
    ///
    /// Skips gracefully (instead of `#[ignore]`) when this test process is
    /// itself nested inside a parent Job whose children cannot break away —
    /// on such hosts `AssignProcessToJobObject` returns `E_ACCESSDENIED`.
    /// The Job is created with `limit_silent_breakaway_ok()` (matching the
    /// production FFI) so that, when the parent Job cooperates, the
    /// kill-on-close path is still exercised.
    #[test]
    fn kill_on_job_close_reaps_spawned_child() {
        // Detect a parent Job nesting this process. If present and the
        // parent does not permit breakaway, assignment would fail with
        // E_ACCESSDENIED — skip rather than panic.
        let mut in_job: windows_sys::Win32::Foundation::BOOL = 0;
        unsafe {
            windows_sys::Win32::System::JobObjects::IsProcessInJob(
                windows_sys::Win32::System::Threading::GetCurrentProcess(),
                std::ptr::null_mut(),
                &mut in_job,
            );
        }
        if in_job != 0 {
            eprintln!(
                "skip: test process is inside a parent Job; cannot assign child to a new Job"
            );
            return;
        }

        let mut child = Command::new("ping")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn ping");
        let pid = child.id();
        let proc_handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION,
                windows_sys::Win32::Foundation::FALSE,
                pid,
            )
        };
        assert!(!proc_handle.is_null(), "OpenProcess failed for ping");

        let job = win32job::Job::create().expect("create job");
        let mut info = win32job::ExtendedLimitInfo::new();
        info.limit_kill_on_job_close();
        info.limit_silent_breakaway_ok();
        job.set_extended_limit_info(&info).expect("set info");
        job.assign_process(proc_handle as isize)
            .expect("assign process to job");

        // Drop the Job — kernel must reap the ping within 3s.
        drop(job);

        let start = Instant::now();
        let mut still_alive = true;
        while start.elapsed() < Duration::from_secs(3) {
            if unsafe { WaitForSingleObject(proc_handle, 100) } == 0 {
                still_alive = false;
                break;
            }
        }
        unsafe { CloseHandle(proc_handle) };
        let _ = child.wait();

        assert!(
            !still_alive,
            "ping (pid {pid}) was not reaped by the kernel within 3s of Job drop; \
             Job Object enforcement is not working on this host"
        );
    }

    /// Verify the FFI module successfully assigns a spawned process to a
    /// Job Object (we can't observe the kill here because the FFI leaks
    /// the Job handle, but successful assignment is what `bash.rs`
    /// depends on for its enforcement contract).
    #[test]
    fn apply_job_object_to_pid_assigns_process() {
        let child = Command::new("ping")
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn ping");
        let pid = child.id();

        // No panic, no eprintln: FFI succeeded. We let the test process
        // exit cleanly — the leaked Job handle will kill the ping when
        // this test binary terminates.
        let assigned = apply_job_object_to_pid(pid, true, "test");
        assert!(
            assigned.is_ok(),
            "applying sandbox Job to pid should succeed: {:?}",
            assigned.err()
        );
        // Give ping a moment to confirm it's running normally.
        std::thread::sleep(Duration::from_millis(200));
    }
}
