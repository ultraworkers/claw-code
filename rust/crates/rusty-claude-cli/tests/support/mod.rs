use std::ffi::OsString;

// Keep credentials, proxy variables and user configuration out of subprocess
// tests, but preserve the operating-system environment needed to load DLLs,
// initialize networking and find the installed test interpreters on Windows.
pub fn process_environment() -> Vec<(&'static str, OsString)> {
    #[cfg(windows)]
    {
        [
            "PATH",
            "SYSTEMROOT",
            "WINDIR",
            "SYSTEMDRIVE",
            "COMSPEC",
            "PATHEXT",
            "TEMP",
            "TMP",
        ]
        .into_iter()
        .filter_map(|name| std::env::var_os(name).map(|value| (name, value)))
        .collect()
    }
    #[cfg(not(windows))]
    {
        vec![("PATH", OsString::from("/usr/bin:/bin"))]
    }
}
