use crate::CacheKey;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{fmt, path::PathBuf};
const MAGIC: &[u8; 8] = b"VUTCACHE";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[derive(Debug)]
pub struct CacheError(String);
impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for CacheError {}
pub struct ArtifactCache {
    root: PathBuf,
}
impl ArtifactCache {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    /// Reads and verifies an artifact, treating a corrupt entry as a cache miss.
    ///
    /// # Errors
    ///
    /// Returns an error when the cache entry cannot be read for a reason other
    /// than it not existing.
    pub fn get(&self, key: &CacheKey) -> Result<Option<Vec<u8>>, CacheError> {
        let path = self.path(key);
        let bytes = match std::fs::read(&path) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(CacheError(e.to_string())),
        };
        if bytes.len() < 40 || &bytes[..8] != MAGIC {
            let _ = std::fs::remove_file(path);
            return Ok(None);
        }
        let payload = &bytes[40..];
        if blake3::hash(payload).as_bytes() != &bytes[8..40] {
            let _ = std::fs::remove_file(path);
            return Ok(None);
        }
        Ok(Some(payload.to_vec()))
    }
    /// Atomically stores an artifact and its integrity digest.
    ///
    /// # Errors
    ///
    /// Returns an error when the cache directory or artifact cannot be written.
    pub fn put(&self, key: &CacheKey, payload: &[u8]) -> Result<(), CacheError> {
        std::fs::create_dir_all(&self.root).map_err(|e| CacheError(e.to_string()))?;
        let path = self.path(key);
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = path.with_extension(format!("{}.{}.tmp", std::process::id(), sequence));
        let mut bytes = Vec::with_capacity(40 + payload.len());
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(blake3::hash(payload).as_bytes());
        bytes.extend_from_slice(payload);
        std::fs::write(&temp, bytes).map_err(|e| CacheError(e.to_string()))?;
        if let Err(error) = std::fs::rename(&temp, &path) {
            if path.exists() {
                // A concurrent content-addressed writer won the race. Its entry
                // has the same key, so keep it and discard our private temp file.
                let _ = std::fs::remove_file(&temp);
            } else {
                let _ = std::fs::remove_file(&temp);
                return Err(CacheError(error.to_string()));
            }
        }
        Ok(())
    }
    fn path(&self, key: &CacheKey) -> PathBuf {
        self.root.join(format!("{}.bin", key.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_and_corruption_recovery() {
        let root = std::env::temp_dir().join(format!("vut-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let cache = ArtifactCache::new(&root);
        let key = crate::CacheKeyBuilder::new("test").finish();
        cache.put(&key, b"object").unwrap();
        assert_eq!(cache.get(&key).unwrap(), Some(b"object".to_vec()));
        std::fs::write(cache.path(&key), b"broken").unwrap();
        assert_eq!(cache.get(&key).unwrap(), None);
        std::fs::remove_dir_all(root).unwrap();
    }
}
