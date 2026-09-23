use crate::{Manifest, PackageId, PackageProvider, ResolvedGraph};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

pub const LOCKFILE_VERSION: u32 = crate::compatibility::CURRENT_LOCKFILE_VERSION;

/// A pinned native artifact for one target.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedNative {
    pub target: String,
    pub url: String,
    /// Trusted expected checksum, e.g. `sha256:<hex>`.
    pub checksum: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub system_libraries: Vec<String>,
}

/// One pinned package: source identity plus exact revision/checksum and any
/// resolved per-target native artifacts.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub source: String,
    pub revision: String,
    /// BLAKE3 checksum of the pinned package source.
    pub checksum: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub native: Vec<LockedNative>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Lockfile {
    #[serde(rename = "lock-version")]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub package: Vec<LockedPackage>,
}

impl Lockfile {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            version: LOCKFILE_VERSION,
            package: Vec::new(),
        }
    }

    /// Builds a deterministic v2 lockfile from a resolved graph, pinning the
    /// native artifact for `target`.
    pub fn from_graph(
        graph: &ResolvedGraph,
        target: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut package = Vec::with_capacity(graph.packages.len());
        for resolved in graph.packages.values() {
            let mut dependencies = resolved
                .dependencies
                .iter()
                .map(|id| format!("{} {} {}", id.name, id.version, id.source))
                .collect::<Vec<_>>();
            dependencies.sort();
            let native = native_entries(&resolved.snapshot, &resolved.id.name, target)?;
            package.push(LockedPackage {
                name: resolved.id.name.clone(),
                version: resolved.id.version.to_string(),
                source: resolved.id.source.identity(),
                revision: resolved.revision.clone(),
                checksum: resolved.checksum.clone(),
                dependencies,
                native,
            });
        }
        package.sort_by(|left, right| {
            (&left.name, &left.version, &left.source).cmp(&(
                &right.name,
                &right.version,
                &right.source,
            ))
        });
        Ok(Self {
            version: LOCKFILE_VERSION,
            package,
        })
    }
}

fn native_entries(
    snapshot: &crate::PackageSnapshot,
    name: &str,
    target: &str,
) -> Result<Vec<LockedNative>, Box<dyn std::error::Error>> {
    if let Some(artifact) = crate::artifact::resolve_for_target(snapshot, target)? {
        return Ok(vec![LockedNative {
            target: target.to_owned(),
            url: artifact.url,
            checksum: artifact.checksum,
            size: artifact.size,
            system_libraries: artifact.system_libraries,
        }]);
    }
    if declares_native_build(snapshot) {
        return Err(format!(
            "package `{name}` declares `[native] build` but has no native artifact for target `{target}`"
        )
        .into());
    }
    Ok(Vec::new())
}

fn declares_native_build(snapshot: &crate::PackageSnapshot) -> bool {
    snapshot
        .files
        .get("vpm.toml")
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .and_then(|text| toml::from_str::<Manifest>(text).ok())
        .is_some_and(|manifest| manifest.native.build.is_some())
}

/// Reads and validates `vpm.lock`, returning `None` when it does not exist.
pub fn read(root: &Path) -> Result<Option<Lockfile>, Box<dyn std::error::Error>> {
    let path = root.join("vpm.lock");
    if !path.is_file() {
        return Ok(None);
    }
    let lock: Lockfile = toml::from_str(&std::fs::read_to_string(&path)?)?;
    validate(&lock)?;
    Ok(Some(lock))
}

/// Writes `vpm.lock` atomically with the canonical field ordering.
pub fn write(root: &Path, lock: &Lockfile) -> Result<(), Box<dyn std::error::Error>> {
    let text = toml::to_string(lock)?;
    crate::project::atomic_write(&root.join("vpm.lock"), &text)?;
    Ok(())
}

fn validate(lock: &Lockfile) -> Result<(), Box<dyn std::error::Error>> {
    crate::compatibility::require_lockfile(lock.version)?;
    let mut identities = BTreeSet::new();
    for package in &lock.package {
        semver::Version::parse(&package.version)
            .map_err(|error| format!("invalid locked version for `{}`: {error}", package.name))?;
        parse_identity(&package.source)?;
        if package.revision.trim().is_empty() {
            return Err(format!("locked package `{}` has an empty revision", package.name).into());
        }
        if !is_blake3(&package.checksum) {
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
        for native in &package.native {
            if native.target.trim().is_empty() {
                return Err(format!(
                    "locked package `{}` has a native entry with an empty target",
                    package.name
                )
                .into());
            }
            if native.url.trim().is_empty() {
                return Err(format!(
                    "locked package `{}` has a native entry with an empty url",
                    package.name
                )
                .into());
            }
            if !is_sha256(&native.checksum) {
                return Err(format!(
                    "locked package `{}` has a native entry with an invalid SHA-256 checksum",
                    package.name
                )
                .into());
            }
        }
    }
    Ok(())
}

fn is_blake3(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

/// True when the lockfile pins every direct manifest dependency to the exact
/// requested source and version.
#[must_use]
pub fn covers_manifest(lock: &Lockfile, manifest: &Manifest) -> bool {
    manifest.dependencies.iter().all(|(name, dependency)| {
        let Some((source, version)) = manifest_dependency(name, dependency) else {
            return false;
        };
        lock.package.iter().any(|package| {
            package.name == *name && package.version == version && package.source == source
        })
    })
}

/// Installs every pinned package, fetching only packages missing from the
/// store, and verifying each against its pinned revision and checksum.
///
/// Also downloads and verifies the pinned native artifact for `target`,
/// returning the resolved native libraries for the link plan. Returns how many
/// packages were newly written to the store.
pub fn install_pinned<P: PackageProvider>(
    root: &Path,
    store: &crate::store::Store,
    lock: &Lockfile,
    provider: &P,
    target: &str,
) -> Result<(usize, crate::native_plan::NativePlan), Box<dyn std::error::Error>> {
    let mut installed = 0;
    let mut native = Vec::new();
    for package in &lock.package {
        let source = parse_identity(&package.source)?;
        let id = PackageId {
            source,
            name: package.name.clone(),
            version: semver::Version::parse(&package.version)?,
        };
        if !store.has(&id, &package.checksum)? {
            let snapshot = provider.fetch(&id.source, &id.version)?;
            if snapshot.revision != package.revision {
                return Err(format!(
                    "locked package `{}` {} was modified upstream (revision mismatch)",
                    package.name, package.version
                )
                .into());
            }
            if store.install_snapshot(&id, &snapshot, &package.checksum)? {
                installed += 1;
            }
        }
        for entry in package.native.iter().filter(|entry| entry.target == target) {
            let artifact = crate::artifact::Artifact {
                url: entry.url.clone(),
                checksum: entry.checksum.clone(),
                size: entry.size,
                system_libraries: entry.system_libraries.clone(),
            };
            let path = crate::artifact::ensure_cached(&artifact, &store.cache_dir())?;
            native.push(crate::artifact::NativeLibrary {
                path,
                system_libraries: entry.system_libraries.clone(),
            });
        }
    }
    if lock.version < LOCKFILE_VERSION {
        let mut upgraded = lock.clone();
        upgraded.version = LOCKFILE_VERSION;
        write(root, &upgraded)?;
    }
    Ok((
        installed,
        crate::native_plan::NativePlan::from_resolved(native),
    ))
}

fn manifest_dependency(name: &str, dependency: &crate::Dependency) -> Option<(String, String)> {
    crate::project::dependency_source(name, dependency)
        .ok()
        .map(|(source, version)| (source.identity(), version))
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
    let lock = read(root)?.ok_or("vpm.lock is missing")?;
    let mut packages = lock.package;
    packages.sort_by(|left, right| {
        (&left.name, &left.version, &left.source).cmp(&(&right.name, &right.version, &right.source))
    });
    let mut roots = vec![(root.join("src"), Vec::new())];
    for package in packages {
        let source = parse_identity(&package.source)?;
        let id = PackageId {
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
    use crate::{PackageSnapshot, PackageSource, resolver::ResolvedPackage};
    use semver::Version;
    use std::collections::BTreeMap;

    fn graph() -> ResolvedGraph {
        let snapshot = PackageSnapshot {
            files: BTreeMap::from([
                (
                    "vpm.toml".into(),
                    b"[package]\nname='math'\nversion='1.0.0'\n".to_vec(),
                ),
                (
                    "src/mod.vut".into(),
                    b"fn answer() -> int:\n  42\n".to_vec(),
                ),
            ]),
            revision: "revision-1".into(),
        };
        let checksum = crate::resolver::checksum(&snapshot);
        let id = PackageId {
            source: PackageSource::parse("math").unwrap(),
            name: "math".into(),
            version: Version::parse("1.0.0").unwrap(),
        };
        ResolvedGraph {
            packages: BTreeMap::from([(
                id.clone(),
                ResolvedPackage {
                    id,
                    revision: "revision-1".into(),
                    checksum,
                    dependencies: Vec::new(),
                    snapshot,
                },
            )]),
        }
    }

    const TARGET: &str = "x86_64-pc-windows-msvc";

    #[test]
    fn v2_round_trips_with_deterministic_ordering() {
        let lock = Lockfile::from_graph(&graph(), TARGET).unwrap();
        assert_eq!(lock.version, 2);
        let text = toml::to_string(&lock).unwrap();
        assert!(text.contains("lock-version = 2"), "{text}");
        assert!(text.contains("checksum ="), "{text}");
        let reparsed: Lockfile = toml::from_str(&text).unwrap();
        validate(&reparsed).unwrap();
        assert_eq!(reparsed.package[0].name, "math");
    }

    #[test]
    fn pins_the_target_native_artifact() {
        let mut graph = graph();
        let resolved = graph.packages.values_mut().next().unwrap();
        resolved.snapshot.files.insert(
            crate::artifact::FILE.into(),
            format!(
                "format = 1\n[target.{TARGET}]\nurl = 'https://e/math.lib'\nchecksum = 'sha256:{}'\nsize = 3\nsystem_libraries = ['user32']\n",
                "0".repeat(64)
            )
            .into_bytes(),
        );
        let lock = Lockfile::from_graph(&graph, TARGET).unwrap();
        assert_eq!(lock.package[0].native.len(), 1);
        assert_eq!(lock.package[0].native[0].target, TARGET);
        assert_eq!(lock.package[0].native[0].system_libraries, ["user32"]);
        // A missing target artifact is an explicit error.
        assert!(Lockfile::from_graph(&graph, "other").is_err());
    }

    #[test]
    fn rejects_invalid_native_checksum_and_missing_revision() {
        let mut lock = Lockfile::from_graph(&graph(), TARGET).unwrap();
        lock.package[0].native.push(LockedNative {
            target: "x86_64-pc-windows-msvc".into(),
            url: "https://example.com/math.lib".into(),
            checksum: "sha256:abc".into(),
            size: 1,
            system_libraries: Vec::new(),
        });
        assert!(validate(&lock).is_err());
        lock.package[0].native.clear();
        lock.package[0].revision = String::new();
        assert!(validate(&lock).is_err());
    }

    struct FixedProvider(PackageSnapshot);
    impl PackageProvider for FixedProvider {
        fn list_versions(
            &self,
            _source: &PackageSource,
        ) -> Result<Vec<String>, crate::ProviderError> {
            Ok(Vec::new())
        }
        fn fetch(
            &self,
            _source: &PackageSource,
            _version: &Version,
        ) -> Result<PackageSnapshot, crate::ProviderError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn pinned_install_uses_locked_revision_and_rejects_mutation() {
        let root = std::env::temp_dir().join(format!("vpm-pinned-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let graph = graph();
        let lock = Lockfile::from_graph(&graph, TARGET).unwrap();
        let snapshot = &graph.packages.values().next().unwrap().snapshot;
        let store = crate::store::Store::at(root.join("store"));

        let (installed, native) = install_pinned(
            &root,
            &store,
            &lock,
            &FixedProvider(snapshot.clone()),
            TARGET,
        )
        .unwrap();
        assert_eq!(installed, 1);
        assert!(native.libraries.is_empty());
        assert!(
            store
                .has(
                    &crate::PackageId {
                        source: PackageSource::parse("math").unwrap(),
                        name: "math".into(),
                        version: Version::parse("1.0.0").unwrap(),
                    },
                    &lock.package[0].checksum
                )
                .unwrap()
        );

        let mut mutated = snapshot.clone();
        mutated.revision = "revision-2".into();
        let store = crate::store::Store::at(root.join("store2"));
        assert!(install_pinned(&root, &store, &lock, &FixedProvider(mutated), TARGET).is_err());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pinned_install_propagates_native_artifacts_transitively() {
        let root = std::env::temp_dir().join(format!("vpm-native-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let store = crate::store::Store::at(root.join("store"));
        let bytes = b"lib";
        let checksum = format!("sha256:{}", crate::artifact::hex_sha256(bytes));
        let cache = store.cache_dir().join("artifacts");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join(checksum.strip_prefix("sha256:").unwrap()), bytes).unwrap();

        let mut graph = graph();
        let resolved = graph.packages.values_mut().next().unwrap();
        resolved.snapshot.files.insert(
            crate::artifact::FILE.into(),
            format!(
                "format = 1\n[target.{TARGET}]\nurl = 'http://127.0.0.1:9/unused.lib'\nchecksum = '{checksum}'\nsize = 3\n"
            )
            .into_bytes(),
        );
        let lock = Lockfile::from_graph(&graph, TARGET).unwrap();
        let snapshot = graph.packages.values().next().unwrap().snapshot.clone();
        let (_installed, plan) =
            install_pinned(&root, &store, &lock, &FixedProvider(snapshot), TARGET).unwrap();
        assert_eq!(plan.libraries.len(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn v1_lockfiles_are_readable() {
        let lock: Lockfile = toml::from_str(
            "lock-version = 1\n\n[[package]]\nname = 'math'\nversion = '1.0.0'\nsource = 'registry:math'\nrevision = 'r'\nchecksum = '0000000000000000000000000000000000000000000000000000000000000000'\n",
        )
        .unwrap();
        validate(&lock).unwrap();
    }
}
