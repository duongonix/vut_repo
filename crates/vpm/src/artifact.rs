//! Resolved native artifact metadata (`native-artifacts.toml`).
//!
//! This file is controlled by the registry/trusted CI, not by the publisher,
//! and is excluded from the package source BLAKE3 checksum. It maps each
//! supported target to an absolute download URL, a trusted expected SHA-256,
//! and the artifact size. A self-computed checksum after download is not
//! trusted; only the expected checksum in this file is.

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub const FORMAT: u32 = 1;
pub(crate) const FILE: &str = "native-artifacts.toml";

const fn default_format() -> u32 {
    FORMAT
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub url: String,
    /// Trusted expected checksum, e.g. `sha256:<hex>`.
    pub checksum: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub system_libraries: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactManifest {
    #[serde(default = "default_format")]
    pub format: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub target: BTreeMap<String, Artifact>,
}

impl ArtifactManifest {
    pub fn parse(text: &str) -> Result<Self, String> {
        let manifest: Self = toml::from_str(text)
            .map_err(|error| format!("invalid native artifact metadata: {error}"))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT {
            return Err(format!(
                "unsupported native artifact metadata format {}; expected {FORMAT}",
                self.format
            ));
        }
        for (target, artifact) in &self.target {
            if target.trim().is_empty() {
                return Err("native artifact target name is empty".into());
            }
            if artifact.url.trim().is_empty() {
                return Err(format!("native artifact for `{target}` has an empty url"));
            }
            if !is_sha256(&artifact.checksum) {
                return Err(format!(
                    "native artifact for `{target}` has an invalid SHA-256 checksum"
                ));
            }
        }
        Ok(())
    }

    /// Selects the artifact for an exact target.
    pub fn select(&self, target: &str) -> Result<&Artifact, String> {
        self.target.get(target).ok_or_else(|| {
            let available = self.target.keys().cloned().collect::<Vec<_>>().join(", ");
            format!("no native artifact for target `{target}`; available: {available}")
        })
    }
}

/// Resolves the target artifact for a package snapshot, if it declares one.
///
/// Returns `None` when the package has no `native-artifacts.toml`.
pub fn resolve_for_target(
    snapshot: &crate::PackageSnapshot,
    target: &str,
) -> Result<Option<Artifact>, String> {
    let Some(bytes) = snapshot.files.get(FILE) else {
        return Ok(None);
    };
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "native artifact metadata is not valid UTF-8".to_owned())?;
    let manifest = ArtifactManifest::parse(text)?;
    manifest
        .select(target)
        .map(|artifact| Some(artifact.clone()))
}

/// A resolved native library ready to be linked into a program.
#[derive(Clone, Debug)]
pub struct NativeLibrary {
    pub path: PathBuf,
    pub system_libraries: Vec<String>,
}

/// Verifies and caches an artifact, returning its local absolute path.
///
/// The cache is content-addressed by the trusted SHA-256. A cached file is
/// accepted only when its size matches; otherwise it is re-downloaded and
/// re-verified.
pub fn ensure_cached(
    artifact: &Artifact,
    cache_root: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let hex = artifact
        .checksum
        .strip_prefix("sha256:")
        .ok_or("native artifact checksum must be `sha256:<hex>`")?;
    let directory = cache_root.join("artifacts");
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(hex);
    if path.is_file() && std::fs::metadata(&path)?.len() == artifact.size {
        return Ok(path);
    }
    if path.is_file() {
        std::fs::remove_file(&path)?;
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("vpm/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client.get(&artifact.url).send().map_err(|error| {
        format!(
            "could not download native artifact `{}`: {error}",
            artifact.url
        )
    })?;
    if !response.status().is_success() {
        return Err(format!(
            "could not download native artifact `{}`: HTTP {}",
            artifact.url,
            response.status().as_u16()
        )
        .into());
    }
    let bytes = response
        .bytes()
        .map_err(|error| format!("could not read native artifact `{}`: {error}", artifact.url))?;
    if bytes.len() as u64 != artifact.size {
        return Err(format!(
            "native artifact `{}` size mismatch: expected {}, got {}",
            artifact.url,
            artifact.size,
            bytes.len()
        )
        .into());
    }
    let actual = hex_sha256(&bytes);
    if actual != hex {
        return Err(format!(
            "native artifact `{}` checksum mismatch: expected sha256:{hex}, got sha256:{actual}",
            artifact.url
        )
        .into());
    }
    let temp = directory.join(format!(".{hex}.{}.tmp", std::process::id()));
    std::fs::write(&temp, &bytes)?;
    std::fs::rename(&temp, &path)?;
    Ok(path)
}

pub(crate) fn hex_sha256(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        })
}

fn is_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::io::{Read as _, Write as _};

    #[test]
    fn parses_and_selects_targets() {
        let parsed = ArtifactManifest::parse(
            "format = 1\n[target.x86_64-pc-windows-msvc]\nurl = 'https://e/math.lib'\nchecksum = 'sha256:0000000000000000000000000000000000000000000000000000000000000000'\nsize = 3\nsystem_libraries = ['user32']\n",
        )
        .unwrap();
        assert!(parsed.select("x86_64-pc-windows-msvc").is_ok());
        assert!(parsed.select("aarch64-apple-darwin").is_err());
        assert!(ArtifactManifest::parse("format = 9\n").is_err());
        assert!(
            ArtifactManifest::parse(
                "format = 1\n[target.x]\nurl = ''\nchecksum = 'sha256:bad'\nsize = 1\n"
            )
            .is_err()
        );
    }

    #[test]
    fn downloads_verifies_and_caches() {
        let bytes = b"lib";
        let checksum = format!("sha256:{}", hex_sha256(bytes));
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let _ = stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nlib"
            )
            .unwrap();
        });
        let root = std::env::temp_dir().join(format!("vpm-artifact-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let artifact = Artifact {
            url: format!("http://{address}/math.lib"),
            checksum: checksum.clone(),
            size: 3,
            system_libraries: Vec::new(),
        };
        let path = ensure_cached(&artifact, &root).unwrap();
        assert!(path.is_file());
        // Second call reuses the cache without contacting the server.
        let cached = ensure_cached(&artifact, &root).unwrap();
        assert_eq!(path, cached);
        server.join().unwrap();

        let wrong = Artifact {
            checksum: format!("sha256:{}", "1".repeat(64)),
            size: 3,
            ..artifact
        };
        assert!(ensure_cached(&wrong, &root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolve_reads_the_snapshot_file() {
        let target = "x86_64-pc-windows-msvc";
        let snapshot = crate::PackageSnapshot {
            files: BTreeMap::from([(
                FILE.into(),
                format!("format = 1\n[target.{target}]\nurl = 'https://e/math.lib'\nchecksum = 'sha256:{}'\nsize = 3\n", "0".repeat(64)).into_bytes(),
            )]),
            revision: "r".into(),
        };
        assert!(resolve_for_target(&snapshot, target).unwrap().is_some());
        assert!(resolve_for_target(&snapshot, "other").is_err());
    }
}
