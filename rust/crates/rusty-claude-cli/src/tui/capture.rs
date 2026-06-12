//! Safe stdout/stderr capture for TUI turns.
//!
//! Replaces the unsafe `libc::dup/dup2` hack. The `gag` crate redirects
//! file descriptors to an in-process pipe for the duration of the closure.
//! Captured output can then be rendered inside the TUI instead of reaching
//! the terminal directly.

use std::io::Read;
use gag::BufferRedirect;

/// Runs `f` with stdout and stderr redirected into memory.
///
/// Returns `(f_result, captured_stdout, captured_stderr)`.  If capture
/// setup fails we fall back to running `f` uncaptured.
pub fn capture_output<F, T>(f: F) -> (T, String, String)
where
    F: FnOnce() -> T,
{
    let Ok(stdout_gag) = BufferRedirect::stdout() else {
        let result = f();
        return (result, String::new(), String::new());
    };
    let Ok(stderr_gag) = BufferRedirect::stderr() else {
        let result = f();
        drop(stdout_gag);
        return (result, String::new(), String::new());
    };

    let result = f();

    let stdout_str = read_redirect(stdout_gag).unwrap_or_default();
    let stderr_str = read_redirect(stderr_gag).unwrap_or_default();

    (result, stdout_str, stderr_str)
}

fn read_redirect(mut redirect: BufferRedirect) -> Result<String, std::io::Error> {
    let mut buf = String::new();
    redirect.read_to_string(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_basic() {
        #[allow(clippy::let_and_return)]
        let captured = capture_output(|| {
            println!("out");
            eprintln!("err");
            42
        });
        assert_eq!(captured.0, 42);
    }

    #[test]
    fn test_capture_empty_output() {
        #[allow(clippy::let_and_return)]
        let (result, stdout, stderr) = capture_output(|| 1);
        assert_eq!(result, 1);
        assert_eq!(stdout, String::new());
        assert_eq!(stderr, String::new());
    }

    #[test]
    fn test_capture_panic_safety() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            capture_output(|| panic!("boom"))
        }));
        assert!(result.is_err());
    }
}
