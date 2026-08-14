use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CachedImage {
    pub bytes: Vec<u8>,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Default)]
pub struct ImageCache {
    entries: HashMap<[u8; 32], CachedImage>,
}

impl ImageCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn hash_original(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&result);
        arr
    }

    pub fn get(&self, hash: &[u8; 32]) -> Option<&CachedImage> {
        self.entries.get(hash)
    }

    pub fn insert(&mut self, hash: [u8; 32], cached: CachedImage) {
        self.entries.insert(hash, cached);
    }

    pub fn get_or_insert_with(
        &mut self,
        hash: &[u8; 32],
        compress_fn: impl FnOnce() -> CachedImage,
    ) -> &CachedImage {
        self.entries.entry(*hash).or_insert_with(compress_fn)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = ImageCache::new();
        let data = b"test image data";
        let hash = ImageCache::hash_original(data);

        let cached = CachedImage {
            bytes: data.to_vec(),
            mime_type: "image/png".to_string(),
            width: 1,
            height: 1,
        };
        cache.insert(hash, cached);

        let result = cache.get(&hash);
        assert!(result.is_some());
        assert_eq!(result.unwrap().mime_type, "image/png");
    }

    #[test]
    fn test_hash_consistency() {
        let data = b"same data";
        let hash1 = ImageCache::hash_original(data);
        let hash2 = ImageCache::hash_original(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_different_data() {
        let hash1 = ImageCache::hash_original(b"data1");
        let hash2 = ImageCache::hash_original(b"data2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_get_or_insert_with_only_calls_fn_once() {
        let mut cache = ImageCache::new();
        let data = b"unique data";
        let hash = ImageCache::hash_original(data);

        let call_count = std::cell::Cell::new(0);
        {
            let _result = cache.get_or_insert_with(&hash, || {
                call_count.set(call_count.get() + 1);
                CachedImage {
                    bytes: data.to_vec(),
                    mime_type: "image/jpeg".to_string(),
                    width: 10,
                    height: 10,
                }
            });
            assert_eq!(_result.width, 10);
        }
        assert_eq!(call_count.get(), 1);

        {
            let _result = cache.get_or_insert_with(&hash, || {
                call_count.set(call_count.get() + 1);
                CachedImage {
                    bytes: data.to_vec(),
                    mime_type: "image/jpeg".to_string(),
                    width: 10,
                    height: 10,
                }
            });
            assert_eq!(_result.width, 10);
        }
        assert_eq!(call_count.get(), 1);
    }

    #[test]
    fn test_empty_cache() {
        let cache = ImageCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }
}
