use serde::Deserialize;
use std::{collections::HashSet, path::Path};

pub const LOCKFILE_VERSION: u32 = crate::compatibility::CURRENT_LOCKFILE_VERSION;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Lockfile {
    #[serde(rename = "lock-version")]
    version: u32,
    #[serde(default)]
    package: Vec<LockedPackage>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LockedPackage {
    name: String,
    version: String,
    source: String,
    revision: String,
    checksum: String,
    #[serde(default)]
    dependencies: Vec<String>,
}

fn read(path: &Path) -> Result<Lockfile, Box<dyn std::error::Error>> {
    let lock: Lockfile = toml::from_str(&std::fs::read_to_string(path)?)?;
    crate::compatibility::require_lockfile(lock.version)?;
    let mut identities = HashSet::new();
    for package in &lock.package {
        semver::Version::parse(&package.version)
            .map_err(|error| format!("invalid locked version for `{}`: {error}", package.name))?;
        parse_identity(&package.source)?;
        if package.revision.trim().is_empty() {
            return Err(format!("locked package `{}` has an empty revision", package.name).into());
        }
        if package.checksum.len() != 64
            || !package
                .checksum
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(format!(
                "locked package `{}` has an invalid BLAKE3 checksum",
                package.name
            )
            .into());
        }
        if !identities.insert((&package.name, &package.version, &package.source)) {
            return Err(format!(
                "duplicate locked package `{}` {}",
                package.name, package.version
            )
            .into());
        }
        for dependency in &package.dependencies {
            if dependency.split_whitespace().count() < 3 {
                return Err(
                    format!("invalid dependency identity `{dependency}` in lockfile").into(),
                );
            }
        }
    }
    Ok(lock)
}

pub fn all_installed(
    root: &Path,
    store: &crate::store::Store,
    manifest: &crate::Manifest,
) -> Result<bool, Box<dyn std::error::Error>> {
    let path = root.join("vpm.lock");
    if !path.is_file() {
        return Ok(false);
    }
    let lock = read(&path)?;
    if lock.package.is_empty() {
        return Ok(false);
    }
    for (name, dependency) in &manifest.dependencies {
        let (source, version) = match dependency {
            crate::Dependency::Registry(version) => (format!("registry:{name}"), version),
            crate::Dependency::Hosted { source, version } => {
                let parsed = crate::PackageSource::parse(source)?;
                (parsed.identity(), version)
            }
            crate::Dependency::Detailed {
                package,
                source,
                version,
            } => {
                let source = if let Some(source) = source {
                    crate::PackageSource::parse(source)?.identity()
                } else {
                    format!("registry:{}", package.as_ref().unwrap_or(name))
                };
                (source, version)
            }
        };
        if !lock
            .package
            .iter()
            .any(|item| item.name == *name && item.version == *version && item.source == source)
        {
            return Ok(false);
        }
    }
    for package in lock.package {
        let source = parse_identity(&package.source)?;
        let id = crate::PackageId {
            source,
            name: package.name,
            version: semver::Version::parse(&package.version)?,
        };
        if !store.has(&id, &package.checksum)? {
            return Ok(false);
        }
    }
    Ok(true)
}
fn parse_identity(value: &str) -> Result<crate::PackageSource, String> {
    if let Some(name) = value.strip_prefix("registry:") {
        return crate::PackageSource::parse(name);
    }
    if let Some(path) = value.strip_prefix("github:") {
        return crate::PackageSource::parse(path);
    }
    if value.starts_with("gitlab:") {
        return crate::PackageSource::parse(value);
    }
    Err(format!("invalid locked source `{value}`"))
}

type SourceRoot = (std::path::PathBuf, Vec<String>);
pub fn source_roots(
    root: &Path,
    store: &crate::store::Store,
) -> Result<Vec<SourceRoot>, Box<dyn std::error::Error>> {
    let lock = read(&root.join("vpm.lock"))?;
    let mut roots = vec![(root.join("src"), Vec::new())];
    for package in lock.package {
        let source = parse_identity(&package.source)?;
        let id = crate::PackageId {
            source,
            name: package.name.clone(),
            version: semver::Version::parse(&package.version)?,
        };
        roots.push((store.path(&id).join("src"), vec![package.name]));
    }
    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PackageId, PackageSnapshot, ResolvedGraph, resolver::ResolvedPackage};
    use semver::Version;
    use std::collections::BTreeMap;

    #[test]
    fn valid_lock_and_store_are_reusable_offline() {
        let root = std::env::temp_dir().join(format!("vpm-lock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("vpm.toml"),
            "[package]\nname='app'\nversion='0.1.0'\n[dependencies]\nmath='1.0.0'\n",
        )
        .unwrap();
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
            revision: "revision-1".into(),
        };
        let checksum = crate::resolver::checksum(&snapshot);
        let id = PackageId {
            source: crate::PackageSource::parse("math").unwrap(),
            name: "math".into(),
            version: Version::parse("1.0.0").unwrap(),
        };
        let graph = ResolvedGraph {
            packages: BTreeMap::from([(
                id.clone(),
                ResolvedPackage {
                    id,
                    revision: "revision-1".into(),
                    checksum: checksum.clone(),
                    dependencies: Vec::new(),
                    snapshot,
                },
            )]),
        };
        let store = crate::store::Store::at(root.join("global"));
        store.install(&graph).unwrap();
        std::fs::write(
            root.join("vpm.lock"),
            format!("lock-version = 1\n\n[[package]]\nname = 'math'\nversion = '1.0.0'\nsource = 'registry:math'\nrevision = 'revision-1'\nchecksum = '{checksum}'\n"),
        )
        .unwrap();
        let manifest = crate::Manifest::read(&root).unwrap();
        assert!(all_installed(&root, &store, &manifest).unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }
}
