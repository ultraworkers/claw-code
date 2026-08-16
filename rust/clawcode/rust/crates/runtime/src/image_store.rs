use base64::Engine;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ImageStore {
    base_path: PathBuf,
}

impl ImageStore {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub fn try_new(base_path: impl AsRef<Path>) -> std::io::Result<Self> {
        fs::create_dir_all(base_path.as_ref())?;
        Ok(Self {
            base_path: base_path.as_ref().to_path_buf(),
        })
    }

    pub fn hash_data(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash: [u8; 32] = hasher.finalize().into();
        hex_encode(&hash)
    }

    pub fn store(&self, data: &[u8], mime_type: &str) -> std::io::Result<String> {
        let hash_hex = Self::hash_data(data);
        let ext = mime_to_ext(mime_type);
        let prefix = &hash_hex[..2];
        let dir = self.base_path.join(prefix);
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{hash_hex}.{ext}"));
        if !path.exists() {
            fs::write(&path, data)?;
        }
        Ok(hash_hex)
    }

    pub fn load(&self, hash_hex: &str, mime_type: &str) -> std::io::Result<Vec<u8>> {
        let ext = mime_to_ext(mime_type);
        let prefix = &hash_hex[..2];
        let raw_path = self.base_path.join(prefix).join(format!("{hash_hex}.{ext}"));
        match fs::read(&raw_path) {
            Ok(data) => Ok(data),
            Err(raw_err) => {
                // Fallback: try .b64 sidecar and decode. Preserve the original
                // raw-read error so a missing raw file is not masked by a
                // sidecar read/decode failure.
                let b64_path = self.base_path.join(prefix).join(format!("{hash_hex}.{ext}.b64"));
                let b64_str = match fs::read_to_string(&b64_path) {
                    Ok(s) => s,
                    Err(_) => return Err(raw_err),
                };
                base64::engine::general_purpose::STANDARD
                    .decode(b64_str.trim())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            }
        }
    }

    pub fn load_base64(&self, hash_hex: &str, mime_type: &str) -> std::io::Result<String> {
        let ext = mime_to_ext(mime_type);
        let prefix = &hash_hex[..2];
        // Prefer sidecar .b64 file (written once by input.rs) — zero re-encode
        let b64_path = self.base_path.join(prefix).join(format!("{hash_hex}.{ext}.b64"));
        if b64_path.exists() {
            return fs::read_to_string(&b64_path);
        }
        // Fall back: read raw bytes and base64-encode
        let data = self.load(hash_hex, mime_type)?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&data))
    }

    pub fn contains(&self, hash_hex: &str, mime_type: &str) -> bool {
        let ext = mime_to_ext(mime_type);
        let prefix = &hash_hex[..2];
        let raw_path = self.base_path.join(prefix).join(format!("{hash_hex}.{ext}"));
        let b64_path = self.base_path.join(prefix).join(format!("{hash_hex}.{ext}.b64"));
        raw_path.exists() || b64_path.exists()
    }

    pub fn path_for(&self, hash_hex: &str, mime_type: &str) -> PathBuf {
        let ext = mime_to_ext(mime_type);
        let prefix = &hash_hex[..2];
        self.base_path.join(prefix).join(format!("{hash_hex}.{ext}"))
    }

    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

fn mime_to_ext(mime_type: &str) -> &'static str {
    match mime_type {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        _ => "bin",
    }
}

fn hex_encode(hash: &[u8; 32]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(64);
    for byte in hash {
        let _ = write!(s, "{byte:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn with_tmp_store(f: impl FnOnce(ImageStore)) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::SeqCst);
        let tmp = std::env::temp_dir().join(format!(
            "image_store_test_{}_{}",
            std::process::id(),
            unique
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let store = ImageStore::new(tmp.clone());
        f(store);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_store_and_load() {
        with_tmp_store(|store| {
            let data = b"test image bytes";
            let hash = store.store(data, "image/png").unwrap();
            assert_eq!(hash.len(), 64);
            let loaded = store.load(&hash, "image/png").unwrap();
            assert_eq!(loaded, data);
        });
    }

    #[test]
    fn test_hash_consistency() {
        let data = b"same data";
        let hash1 = ImageStore::hash_data(data);
        let hash2 = ImageStore::hash_data(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_dedup_same_file() {
        with_tmp_store(|store| {
            let data = b"dedup test data";
            let hash1 = store.store(data, "image/jpeg").unwrap();
            let hash2 = store.store(data, "image/jpeg").unwrap();
            assert_eq!(hash1, hash2);
            let dir = store.base_path().join(&hash1[..2]);
            let count = fs::read_dir(dir).unwrap().count();
            assert_eq!(count, 1);
        });
    }

    #[test]
    fn test_contains() {
        with_tmp_store(|store| {
            let data = b"exists check";
            let hash = store.store(data, "image/png").unwrap();
            assert!(store.contains(&hash, "image/png"));
            assert!(!store.contains(&hash, "image/jpeg"));
        });
    }

    #[test]
    fn test_load_base64() {
        with_tmp_store(|store| {
            let data = b"base64 me";
            let hash = store.store(data, "image/png").unwrap();
            let b64 = store.load_base64(&hash, "image/png").unwrap();
            assert_eq!(b64, base64::engine::general_purpose::STANDARD.encode(data));
        });
    }

    #[test]
    fn test_mime_to_ext() {
        assert_eq!(mime_to_ext("image/jpeg"), "jpg");
        assert_eq!(mime_to_ext("image/png"), "png");
        assert_eq!(mime_to_ext("image/webp"), "webp");
        assert_eq!(mime_to_ext("image/gif"), "gif");
        assert_eq!(mime_to_ext("application/octet-stream"), "bin");
    }

    #[test]
    fn test_path_for() {
        let store = ImageStore::new(PathBuf::from("/tmp/images"));
        let path = store.path_for("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890", "image/jpeg");
        let sep = std::path::MAIN_SEPARATOR;
        assert!(path.to_string_lossy().contains(&format!("ab{sep}abcdef12")));
    }
}
