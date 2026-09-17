//! SHA-256 checksums for release artifacts.
use std::fs::File;
use std::io;
use std::path::Path;

use sha2::{Digest, Sha256};

/// Returns the lowercase hex SHA-256 of a file.
///
/// # Errors
/// Returns I/O errors from opening or reading the file.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut hasher = Sha256::new();
    let mut file = File::open(path)?;
    io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// Returns the lowercase hex SHA-256 of a byte slice.
#[cfg(test)]
#[must_use]
pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_published_test_vector() {
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
