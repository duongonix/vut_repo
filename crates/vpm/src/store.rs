use crate::{PackageId, ResolvedGraph};
use fs2::FileExt as _;
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};

pub struct Store {
    root: PathBuf,
}
impl Store {
    #[cfg(test)]
    pub(crate) fn at(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn global() -> Result<Self, io::Error> {
        let root = std::env::var_os("VPM_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE").map(|path| PathBuf::from(path).join(".vpm"))
            })
            .or_else(|| std::env::var_os("HOME").map(|path| PathBuf::from(path).join(".vpm")))
            .or_else(|| dirs::home_dir().map(|p| p.join(".vpm")))
            .ok_or_else(|| io::Error::other("cannot determine VPM home"))?;
        Ok(Self { root })
    }
    /// Installs every not-yet-present package and returns how many packages
    /// were newly written to the store.
    pub fn install(&self, graph: &ResolvedGraph) -> Result<usize, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(self.root.join("packages"))?;
        let lock_path = self.root.join("install.lock");
        let lock = File::create(lock_path)?;
        lock.lock_exclusive()?;
        let result = self.install_locked(graph);
        lock.unlock()?;
        result
    }
    pub fn has(&self, id: &PackageId, checksum: &str) -> Result<bool, io::Error> {
        let path = self.path(id);
        if !path.is_dir() {
            return Ok(false);
        }
        let marker = std::fs::read_to_string(path.join(".vpm-checksum"))?;
        Ok(marker.trim() == checksum && hash_directory(&path)? == checksum)
    }
    #[must_use]
    pub fn build_cache(&self, target: &str, release: bool) -> PathBuf {
        let target = target
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '_'
                }
            })
            .collect::<String>();
        self.root
            .join("cache/build")
            .join(target)
            .join(if release { "release" } else { "debug" })
    }
    fn install_locked(&self, graph: &ResolvedGraph) -> Result<usize, Box<dyn std::error::Error>> {
        let mut installed = 0;
        for package in graph.packages.values() {
            crate::package_validation::validate(&package.snapshot, &package.id, &package.checksum)?;
            let destination = self.path(&package.id);
            if destination.is_dir() {
                if !self.has(&package.id, &package.checksum)? {
                    return Err(format!(
                        "installed package `{}` failed immutability check",
                        package.id.name
                    )
                    .into());
                }
                continue;
            }
            let parent = destination.parent().ok_or("invalid store destination")?;
            std::fs::create_dir_all(parent)?;
            let staging = parent.join(format!(".{}.{}.tmp", package.id.name, std::process::id()));
            if staging.exists() {
                std::fs::remove_dir_all(&staging)?;
            }
            std::fs::create_dir(&staging)?;
            for (relative, data) in &package.snapshot.files {
                safe_write(&staging, relative, data)?;
            }
            std::fs::write(staging.join(".vpm-checksum"), &package.checksum)?;
            match std::fs::rename(&staging, &destination) {
                Ok(()) => installed += 1,
                Err(_error) if destination.exists() => {
                    std::fs::remove_dir_all(staging)?;
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(installed)
    }
    #[must_use]
    pub fn path(&self, id: &PackageId) -> PathBuf {
        let mut path = self.root.join("packages");
        match &id.source {
            crate::PackageSource::Registry { .. } => path.push("registry"),
            crate::PackageSource::Hosted {
                provider,
                owner,
                repository,
                path: parts,
            } => {
                path.push(match provider {
                    crate::ProviderKind::GitLab => "gitlab",
                    _ => "github",
                });
                path.push(owner);
                path.push(repository);
                for part in parts {
                    path.push(part);
                }
            }
        }
        if matches!(id.source, crate::PackageSource::Registry { .. }) {
            path.push(&id.name);
        }
        path.push(id.version.to_string());
        path
    }
}
fn hash_directory(root: &Path) -> io::Result<String> {
    fn visit(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
        let mut entries = std::fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, files)?;
            } else if path.file_name().is_none_or(|name| name != ".vpm-checksum") {
                files.push(
                    path.strip_prefix(root)
                        .map_err(io::Error::other)?
                        .to_owned(),
                );
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    let mut hash = blake3::Hasher::new();
    for relative in files {
        let key = relative.to_string_lossy().replace('\\', "/");
        let data = std::fs::read(root.join(&relative))?;
        hash.update(key.as_bytes());
        hash.update(&(data.len() as u64).to_le_bytes());
        hash.update(&data);
    }
    Ok(hash.finalize().to_hex().to_string())
}
fn safe_write(root: &Path, relative: &str, data: &[u8]) -> io::Result<()> {
    let mut path = root.to_owned();
    for component in Path::new(relative).components() {
        match component {
            std::path::Component::Normal(value) => path.push(value),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unsafe package path",
                ));
            }
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PackageSnapshot, PackageSource, ResolvedGraph, resolver::ResolvedPackage};
    use semver::Version;
    use std::collections::BTreeMap;
    #[test]
    fn installation_is_atomic_source_aware_and_detects_mutation() {
        let root = std::env::temp_dir().join(format!("vpm-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let store = Store { root: root.clone() };
        let snapshot = PackageSnapshot {
            files: BTreeMap::from([
                (
                    "vpm.toml".into(),
                    b"[package]\nname='math'\nversion='1.0.0'\n".to_vec(),
                ),
                (
                    "src/lib.vut".into(),
                    b"fn answer() -> int:\n  42\n".to_vec(),
                ),
            ]),
            revision: "abc".into(),
        };
        let checksum = crate::resolver::checksum(&snapshot);
        let id = PackageId {
            source: PackageSource::parse("math").unwrap(),
            name: "math".into(),
            version: Version::parse("1.0.0").unwrap(),
        };
        let package = ResolvedPackage {
            id: id.clone(),
            revision: "abc".into(),
            checksum: checksum.clone(),
            dependencies: Vec::new(),
            snapshot,
        };
        let graph = ResolvedGraph {
            packages: BTreeMap::from([(id.clone(), package)]),
        };
        store.install(&graph).unwrap();
        assert!(store.has(&id, &checksum).unwrap());
        std::fs::write(store.path(&id).join("src/lib.vut"), "mutated").unwrap();
        assert!(!store.has(&id, &checksum).unwrap());
        assert!(store.install(&graph).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_snapshot_never_becomes_an_installed_package() {
        let root = std::env::temp_dir().join(format!("vpm-invalid-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let store = Store { root: root.clone() };
        let snapshot = PackageSnapshot {
            files: BTreeMap::from([(
                "vpm.toml".into(),
                b"[package]\nname='wrong'\nversion='1.0.0'\n".to_vec(),
            )]),
            revision: "abc".into(),
        };
        let id = PackageId {
            source: PackageSource::parse("math").unwrap(),
            name: "math".into(),
            version: Version::parse("1.0.0").unwrap(),
        };
        let package = ResolvedPackage {
            id: id.clone(),
            revision: "abc".into(),
            checksum: crate::resolver::checksum(&snapshot),
            dependencies: Vec::new(),
            snapshot,
        };
        assert!(
            store
                .install(&ResolvedGraph {
                    packages: BTreeMap::from([(id.clone(), package)])
                })
                .is_err()
        );
        assert!(!store.path(&id).exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
