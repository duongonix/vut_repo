use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CacheKey(String);
impl CacheKey {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct CacheKeyBuilder(blake3::Hasher);
impl CacheKeyBuilder {
    #[must_use]
    pub fn new(namespace: &str) -> Self {
        let mut hash = blake3::Hasher::new();
        hash.update(&crate::CACHE_SCHEMA.to_le_bytes());
        hash.update(namespace.as_bytes());
        Self(hash)
    }
    #[must_use]
    pub fn field(mut self, name: &str, value: impl AsRef<[u8]>) -> Self {
        let value = value.as_ref();
        self.0.update(name.as_bytes());
        self.0.update(&(value.len() as u64).to_le_bytes());
        self.0.update(value);
        self
    }
    #[must_use]
    pub fn finish(self) -> CacheKey {
        CacheKey(self.0.finalize().to_hex().to_string())
    }
}

/// Produces a stable fingerprint for all Vut source files below the supplied roots.
///
/// # Errors
///
/// Returns an I/O error when a source root cannot be traversed or a source file
/// cannot be read.
pub fn fingerprint_roots(roots: &[(PathBuf, Vec<String>)]) -> Result<String, std::io::Error> {
    let mut files = Vec::new();
    for (root, prefix) in roots {
        visit(root, root, prefix, &mut files)?;
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hash = blake3::Hasher::new();
    for (key, path) in files {
        let data = std::fs::read(path)?;
        hash.update(key.as_bytes());
        hash.update(&(data.len() as u64).to_le_bytes());
        hash.update(&data);
    }
    Ok(hash.finalize().to_hex().to_string())
}
fn visit(
    root: &Path,
    current: &Path,
    prefix: &[String],
    files: &mut Vec<(String, PathBuf)>,
) -> Result<(), std::io::Error> {
    let mut entries = std::fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            visit(root, &path, prefix, files)?;
        } else if path.extension().is_some_and(|value| value == "vut") {
            let relative = path
                .strip_prefix(root)
                .map_err(std::io::Error::other)?
                .to_string_lossy()
                .replace('\\', "/");
            files.push((format!("{}/{relative}", prefix.join(".")), path));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys_include_schema_namespace_and_fields() {
        let a = CacheKeyBuilder::new("object").field("target", "a").finish();
        let b = CacheKeyBuilder::new("object").field("target", "b").finish();
        assert_ne!(a, b);
        assert_eq!(a.as_str().len(), 64);
    }
}
