//! Durable mapping from SDK correlation keys to their active terminal session.

use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{SdkError, SdkResult};

use super::reconciliation::atomic_write_json;

const MAX_CORRELATION_KEY_BYTES: usize = 512;
static TERMINAL_SCOPE_WRITE_LOCK: Mutex<()> = Mutex::new(());

/// Persisted terminal-session scope for one correlation key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalScopeRecord {
    pub correlation_key: String,
    pub session_id: String,
    pub revision: u64,
    pub updated_at_ms: u64,
}

/// File-backed mapping of correlation keys to terminal sessions.
#[derive(Debug, Clone)]
pub struct TerminalScopeStore {
    root: PathBuf,
}

impl TerminalScopeStore {
    /// Creates a store handle without creating its root directory.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Sets the session associated with a correlation key.
    pub fn set(
        &self,
        correlation_key: impl Into<String>,
        session_id: impl Into<String>,
    ) -> SdkResult<TerminalScopeRecord> {
        let correlation_key = correlation_key.into();
        validate_correlation_key(&correlation_key)?;
        let _guard = TERMINAL_SCOPE_WRITE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = self.read_record(&correlation_key)?;
        let revision = previous
            .as_ref()
            .map_or(Some(1), |record| record.revision.checked_add(1))
            .ok_or_else(|| SdkError::internal("terminal scope revision is exhausted"))?;
        let previous_updated_at = previous.map_or(0, |record| record.updated_at_ms);
        let record = TerminalScopeRecord {
            correlation_key: correlation_key.clone(),
            session_id: session_id.into(),
            revision,
            updated_at_ms: current_timestamp_ms().max(previous_updated_at.saturating_add(1)),
        };
        atomic_write_json(&self.record_path(&correlation_key), &record)?;
        Ok(record)
    }

    /// Rebuilds a mapping from durable invocation history without moving it backward.
    pub fn restore_if_newer(
        &self,
        correlation_key: &str,
        session_id: &str,
        revision: u64,
    ) -> SdkResult<()> {
        validate_correlation_key(correlation_key)?;
        let _guard = TERMINAL_SCOPE_WRITE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = self.read_record(correlation_key)?;
        match previous.as_ref() {
            Some(record) if record.revision > revision => return Ok(()),
            Some(record) if record.revision == revision && record.session_id == session_id => {
                return Ok(())
            }
            Some(record) if record.revision == revision => {
                return Err(SdkError::internal(format!(
                    "terminal scope revision {revision} conflicts for key '{correlation_key}'"
                )));
            }
            _ => {}
        }
        let record = TerminalScopeRecord {
            correlation_key: correlation_key.to_string(),
            session_id: session_id.to_string(),
            revision,
            updated_at_ms: current_timestamp_ms().max(
                previous
                    .map_or(0, |record| record.updated_at_ms)
                    .saturating_add(1),
            ),
        };
        atomic_write_json(&self.record_path(correlation_key), &record)
    }

    /// Loads the session associated with a correlation key, if present.
    pub fn get(&self, correlation_key: &str) -> SdkResult<Option<TerminalScopeRecord>> {
        validate_correlation_key(correlation_key)?;
        self.read_record(correlation_key)
    }

    fn read_record(&self, correlation_key: &str) -> SdkResult<Option<TerminalScopeRecord>> {
        let path = self.record_path(correlation_key);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(SdkError::internal(format!(
                    "read terminal scope '{}': {error}",
                    path.display()
                )));
            }
        };
        let record: TerminalScopeRecord = serde_json::from_slice(&bytes).map_err(|error| {
            SdkError::internal(format!(
                "parse terminal scope '{}': {error}",
                path.display()
            ))
        })?;
        if record.correlation_key != correlation_key {
            return Err(SdkError::internal(format!(
                "terminal scope key does not match requested key for '{}'",
                path.display()
            )));
        }
        Ok(Some(record))
    }

    /// Builds the record path for a correlation key.
    ///
    /// Files are named from a fixed-width digest instead of the key's hex
    /// expansion: a maximal 512-byte key expands to 1024 hex characters, which
    /// overflows the 1024-byte total-path limit enforced by APFS before any
    /// chunking can help, and `File name too long` would be reported for a
    /// generated leaf name the caller never chose. The key itself is still
    /// stored in `TerminalScopeRecord::correlation_key` and checked on read.
    fn record_path(&self, correlation_key: &str) -> PathBuf {
        self.root
            .join(format!("{}.json", encode_key_for_path(correlation_key)))
    }
}

fn validate_correlation_key(correlation_key: &str) -> SdkResult<()> {
    if correlation_key.len() <= MAX_CORRELATION_KEY_BYTES {
        Ok(())
    } else {
        Err(SdkError::invalid_input("correlation key exceeds 512 bytes"))
    }
}

/// Encodes a correlation key as a fixed-width filesystem-safe path component.
fn encode_key_for_path(key: &str) -> String {
    const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(key.as_bytes());
    // The first 16 bytes are far more than the number of records one store
    // holds, and the full key survives in the record for exact matching.
    let mut encoded = String::with_capacity(32);
    for byte in digest.iter().take(16) {
        encoded.push(char::from(HEX_DIGITS[usize::from(*byte >> 4)]));
        encoded.push(char::from(HEX_DIGITS[usize::from(*byte & 0x0f)]));
    }
    encoded
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Component;
    use uuid::Uuid;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("sdk-terminal-scope-{}", Uuid::new_v4()));
            fs::create_dir(&path).expect("create test directory");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn terminal_scope_survives_store_restart() {
        let root = TestDirectory::new();
        TerminalScopeStore::new(&root.0)
            .set("request-1", "session-1")
            .expect("set terminal scope");

        let loaded = TerminalScopeStore::new(&root.0)
            .get("request-1")
            .expect("load terminal scope")
            .expect("scope exists");
        assert_eq!(loaded.correlation_key, "request-1");
        assert_eq!(loaded.session_id, "session-1");
    }

    #[test]
    fn scope_revisions_increase_and_recovery_never_moves_backward() {
        let root = TestDirectory::new();
        let store = TerminalScopeStore::new(&root.0);
        let first = store.set("request-1", "session-1").unwrap();
        let second = store.set("request-1", "session-2").unwrap();
        assert_eq!(second.revision, first.revision + 1);
        assert!(second.updated_at_ms > first.updated_at_ms);

        store
            .restore_if_newer("request-1", "session-1", first.revision)
            .expect("stale scope restoration should be ignored");
        assert_eq!(
            store.get("request-1").unwrap().unwrap().session_id,
            "session-2"
        );
        assert!(store
            .restore_if_newer("request-1", "other-session", second.revision)
            .is_err());
    }

    #[test]
    fn encoded_keys_are_isolated_and_cannot_traverse_paths() {
        let root = TestDirectory::new();
        let store = TerminalScopeStore::new(&root.0);
        store.set("same", "session-a").expect("set first scope");
        store
            .set("../same", "session-b")
            .expect("set path-like key");
        store
            .set("../sämé", "session-c")
            .expect("set Unicode path-like key");

        assert_eq!(
            store
                .get("same")
                .expect("get first")
                .expect("exists")
                .session_id,
            "session-a"
        );
        assert_eq!(
            store
                .get("../same")
                .expect("get path-like key")
                .expect("exists")
                .session_id,
            "session-b"
        );
        assert_eq!(
            store
                .get("../sämé")
                .expect("get Unicode path-like key")
                .expect("exists")
                .session_id,
            "session-c"
        );
        assert_eq!(fs::read_dir(&root.0).expect("read root").count(), 3);
    }

    #[test]
    fn malformed_terminal_scope_json_is_rejected() {
        let root = TestDirectory::new();
        let store = TerminalScopeStore::new(&root.0);
        let path = root
            .0
            .join(format!("{}.json", encode_key_for_path("request")));
        fs::write(path, "{").expect("write malformed scope");
        assert!(store.get("request").is_err());
    }

    #[test]
    fn long_hex_keys_use_safe_path_components() {
        let root = TestDirectory::new();
        let store = TerminalScopeStore::new(&root.0);
        let key = "k".repeat(512);
        store
            .set(&key, "session-long")
            .expect("long key should persist");
        assert_eq!(
            store.get(&key).expect("get long key").unwrap().session_id,
            "session-long"
        );

        // A maximum-length key is 1024 hex characters. Expanding it into the
        // path used to exceed the filesystem's total-path limit, so assert on
        // the path shape rather than only on the round trip: the store works in
        // a short temporary root even when the generated path is unusable.
        let record_path = single_record_path(&root.0);
        let longest_component = record_path
            .components()
            .filter_map(|component| match component {
                Component::Normal(part) => Some(part.to_string_lossy().len()),
                _ => None,
            })
            .max()
            .expect("record path has a component");
        assert!(
            longest_component <= 255,
            "path component must stay within the filesystem filename limit, got {longest_component}"
        );
        assert!(
            record_path.as_os_str().len() < 1024,
            "record path must stay within the APFS total-path limit, got {} bytes",
            record_path.as_os_str().len()
        );
    }

    /// Returns the single record file under a store root.
    fn single_record_path(root: &std::path::Path) -> PathBuf {
        let entries = fs::read_dir(root)
            .expect("read store root")
            .collect::<Result<Vec<_>, _>>()
            .expect("read store entries");
        assert_eq!(entries.len(), 1, "store should hold exactly one record");
        entries[0].path()
    }

    #[test]
    fn oversized_correlation_keys_are_rejected() {
        let root = TestDirectory::new();
        let store = TerminalScopeStore::new(&root.0);
        let key = "k".repeat(513);
        assert!(store.set(&key, "session-long").is_err());
        assert!(store.get(&key).is_err());
        assert!(!root.0.exists() || fs::read_dir(&root.0).unwrap().next().is_none());
    }
}
