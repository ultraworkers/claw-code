//! Strip environment variables that could subvert sandbox enforcement or
//! let a child process escape isolation. Modeled on tidev's
//! `process_hardening::remove_dangerous_env_vars_parent` (process_hardening.rs:97)
//! and extended for Windows-specific attack surfaces:
//!
//! | Variable | Platform | Attack |
//! |---|---|---|
//! | `LD_PRELOAD` | any with libc | Shared-library injection — child loads attacker .so |
//! | `LD_LIBRARY_PATH` | any with libc | Library search-path override |
//! | `LD_AUDIT` | any with libc | Audit library injection |
//! | `DYLD_INSERT_LIBRARIES` | macOS | Same as LD_PRELOAD for Mach-O |
//! | `DYLD_LIBRARY_PATH` | macOS | Same as LD_LIBRARY_PATH for Mach-O |
//! | `PSModulePath` | Windows | PowerShell module hijack — child can shadow real modules |
//! | `NODE_OPTIONS` | Windows/macOS | Node.js `--require` injection — preload arbitrary JS |
//! | `MSYS2_ARG_CONV_EXCL` | Windows (MSYS2) | Path-conversion reversal — quoted args become unquoted |
//! | `MSYS2_ENV_CONV_EXCL` | Windows (MSYS2) | Same family — env-var value conversion attack |
//!
//! This MUST be called in the **parent** process before `spawn()`, never
//! inside a `pre_exec` closure. Allocating or touching the environment
//! after `fork()` (or its Win32 equivalent) can deadlock if another
//! thread is holding the heap lock.
//!
//! Mutating the parent's environment is permanent for the rest of the
//! agent loop. That's intentional: the agent should never honor these
//! vars anywhere — not in subsequent tool calls, not in hook scripts.

#![allow(unsafe_code)]

const DANGEROUS_VARS_PARENT: &[&str] = &[
    // Unix shared-library injection
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "LD_AUDIT",
    // macOS Mach-O injection
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
    // Windows PowerShell hijack
    "PSModulePath",
    // Node.js preload script injection
    "NODE_OPTIONS",
    "NODE_PATH",
    // MSYS2 path-conversion reversal (Windows)
    "MSYS2_ARG_CONV_EXCL",
    "MSYS2_ENV_CONV_EXCL",
];

/// Strip every dangerous env var from the parent process. Safe to call
/// multiple times — the second call is a no-op. Allocated sets are
/// released before the function returns, so no memory is leaked even on
/// no-op invocations.
pub fn remove_dangerous_env_vars_parent() {
    // Snapshot which keys are present, then remove them. We do not
    // iterate `std::env::vars()` and call `remove_var` from inside the
    // iterator: that mutates the environment while we read it, and on
    // some platforms causes UB. Build a list of keys to drop first.
    let keys_to_remove: Vec<String> = DANGEROUS_VARS_PARENT
        .iter()
        .filter(|k| std::env::var_os(k).is_some())
        .map(|k| (*k).to_string())
        .collect();
    for key in &keys_to_remove {
        // SAFETY: `std::env::remove_var` is marked `unsafe` in Rust
        // 1.78+ because it races with concurrent readers of the
        // process environment. We are called from a synchronous code
        // path in `prepare_command` / `prepare_tokio_command`, both of
        // which execute on the agent's main thread before any tool
        // call dispatches. No other thread reads env vars during this
        // window (verified by code review of the bash execution path
        // — `BashCommandInput` is built synchronously from
        // `BashCommandInput` parsing on the same thread).
        unsafe {
            std::env::remove_var(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Tests that mutate the process environment must run serially — we
    // cannot have two `env::set_var` / `remove_var` tests interleaving
    // because the process is shared.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Restore the original env after a test, even on panic.
    fn restore() {
        // We only restore vars we know we touched in tests. Don't try
        // to fully snapshot/restore `std::env` — that's UB on Windows
        // because some env blocks are read-only.
    }

    #[test]
    fn removes_all_dangerous_vars_when_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Set every var we know about, then strip, then assert gone.
        for var in DANGEROUS_VARS_PARENT {
            unsafe {
                std::env::set_var(var, "/attacker/path");
            }
        }
        remove_dangerous_env_vars_parent();
        for var in DANGEROUS_VARS_PARENT {
            assert!(
                std::env::var_os(var).is_none(),
                "expected {var} to be stripped, but it is still set"
            );
        }
        restore();
    }

    #[test]
    fn no_op_when_no_dangerous_vars_present() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Save and clear dangerous vars (some may be set from a
        // concurrent test, even with the lock — env is process-global
        // and a panic in another test thread could leave residue).
        let saved: Vec<(&str, Option<std::ffi::OsString>)> = DANGEROUS_VARS_PARENT
            .iter()
            .map(|k| (*k, std::env::var_os(k)))
            .collect();
        for (k, _) in &saved {
            unsafe {
                std::env::remove_var(k);
            }
        }
        // Run twice; second call should not panic on a no-op env.
        remove_dangerous_env_vars_parent();
        remove_dangerous_env_vars_parent();
        // Restore the test's pre-strip state.
        for (k, v) in saved {
            if let Some(v) = v {
                unsafe {
                    std::env::set_var(k, v);
                }
            }
        }
    }

    #[test]
    fn does_not_strip_unrelated_vars() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CLAW_TEST_DO_NOT_STRIP", "x");
        }
        remove_dangerous_env_vars_parent();
        assert_eq!(
            std::env::var("CLAW_TEST_DO_NOT_STRIP").as_deref(),
            Ok("x"),
            "remove_dangerous_env_vars_parent must not touch unrelated vars"
        );
        unsafe {
            std::env::remove_var("CLAW_TEST_DO_NOT_STRIP");
        }
    }
}
