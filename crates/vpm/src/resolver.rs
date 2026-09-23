use crate::{
    Dependency, Manifest, PackageProvider, PackageSnapshot, PackageSource, ProviderError,
    ProviderKind,
};
use semver::Version;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PackageId {
    pub source: PackageSource,
    pub name: String,
    pub version: Version,
}
#[derive(Clone, Debug)]
pub struct ResolvedPackage {
    pub id: PackageId,
    pub revision: String,
    pub checksum: String,
    pub dependencies: Vec<PackageId>,
    pub snapshot: PackageSnapshot,
}
#[derive(Clone, Debug, Default)]
pub struct ResolvedGraph {
    pub packages: BTreeMap<PackageId, ResolvedPackage>,
}

pub struct Resolver<'a, P: PackageProvider> {
    provider: &'a P,
    packages: BTreeMap<PackageId, ResolvedPackage>,
    names: BTreeMap<String, PackageId>,
    visiting: Vec<PackageId>,
}
impl<'a, P: PackageProvider> Resolver<'a, P> {
    #[must_use]
    pub fn new(provider: &'a P) -> Self {
        Self {
            provider,
            packages: BTreeMap::new(),
            names: BTreeMap::new(),
            visiting: Vec::new(),
        }
    }
    pub fn resolve(
        mut self,
        requests: &[(PackageSource, Option<String>)],
    ) -> Result<ResolvedGraph, ProviderError> {
        for (source, requested) in requests {
            self.package(source, requested.as_deref())?;
        }
        Ok(ResolvedGraph {
            packages: self.packages,
        })
    }
    fn package(
        &mut self,
        source: &PackageSource,
        requested: Option<&str>,
    ) -> Result<PackageId, ProviderError> {
        let versions = self.provider.list_versions(source)?;
        let version =
            crate::version::select(&versions, requested).map_err(ProviderError::NotFound)?;
        let id = PackageId {
            source: source.clone(),
            name: source.name().into(),
            version,
        };
        if self.packages.contains_key(&id) {
            return Ok(id);
        }
        if let Some(existing) = self.names.get(&id.name)
            && existing != &id
        {
            return Err(ProviderError::Integrity(format!(
                "dependency namespace/version conflict: `{}` resolves to both {} and {}",
                id.name, existing.version, id.version
            )));
        }
        if let Some(index) = self.visiting.iter().position(|item| item == &id) {
            let cycle = self.visiting[index..]
                .iter()
                .map(|item| item.name.as_str())
                .chain(std::iter::once(id.name.as_str()))
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(ProviderError::Integrity(format!(
                "dependency cycle: {cycle}"
            )));
        }
        self.names.insert(id.name.clone(), id.clone());
        self.visiting.push(id.clone());
        let snapshot = self.provider.fetch(source, &id.version)?;
        let manifest_bytes = snapshot
            .files
            .get("vpm.toml")
            .ok_or_else(|| ProviderError::Integrity(format!("{source} has no vpm.toml")))?;
        let manifest: Manifest = toml::from_str(
            std::str::from_utf8(manifest_bytes)
                .map_err(|e| ProviderError::Integrity(e.to_string()))?,
        )
        .map_err(|e| ProviderError::Integrity(e.to_string()))?;
        manifest.validate().map_err(ProviderError::Integrity)?;
        if manifest.package.name != id.name || manifest.package.version != id.version.to_string() {
            return Err(ProviderError::Integrity(format!(
                "remote manifest identity mismatch for {id:?}"
            )));
        }
        let mut dependencies = Vec::new();
        for (name, dependency) in &manifest.dependencies {
            let (child_source, version) = dependency_request(name, dependency)?;
            dependencies.push(self.package(&child_source, Some(version))?);
        }
        dependencies.sort();
        let checksum = checksum(&snapshot);
        self.visiting.pop();
        self.packages.insert(
            id.clone(),
            ResolvedPackage {
                id: id.clone(),
                revision: snapshot.revision.clone(),
                checksum,
                dependencies,
                snapshot,
            },
        );
        Ok(id)
    }
}
fn dependency_request<'a>(
    name: &str,
    dependency: &'a Dependency,
) -> Result<(PackageSource, &'a str), ProviderError> {
    match dependency {
        Dependency::Registry(version) => {
            Ok((PackageSource::Registry { name: name.into() }, version))
        }
        Dependency::Hosted { source, version } => Ok((
            PackageSource::parse(source).map_err(ProviderError::Integrity)?,
            version,
        )),
        Dependency::Detailed {
            package,
            source,
            version,
        } => {
            let source = match source {
                Some(source) => PackageSource::parse(source).map_err(ProviderError::Integrity)?,
                None => PackageSource::Registry {
                    name: package.clone().unwrap_or_else(|| name.to_owned()),
                },
            };
            Ok((source, version))
        }
    }
}
pub(crate) fn checksum(snapshot: &PackageSnapshot) -> String {
    let mut hash = blake3::Hasher::new();
    for (path, data) in &snapshot.files {
        if is_checksum_excluded(path) {
            continue;
        }
        hash.update(path.as_bytes());
        hash.update(&(data.len() as u64).to_le_bytes());
        hash.update(data);
    }
    hash.finalize().to_hex().to_string()
}

/// Registry/CI-generated metadata that is not part of the source checksum.
pub(crate) fn is_checksum_excluded(path: &str) -> bool {
    path == crate::artifact::FILE
}

pub struct Providers {
    github: crate::GitHubProvider,
    gitlab: crate::GitLabProvider,
}
impl Providers {
    pub fn new() -> Result<Self, ProviderError> {
        Ok(Self {
            github: crate::GitHubProvider::new()?,
            gitlab: crate::GitLabProvider::new()?,
        })
    }
}
impl PackageProvider for Providers {
    fn list_versions(&self, source: &PackageSource) -> Result<Vec<String>, ProviderError> {
        match source.provider() {
            ProviderKind::Registry | ProviderKind::GitHub => self.github.list_versions(source),
            ProviderKind::GitLab => self.gitlab.list_versions(source),
        }
    }
    fn fetch(
        &self,
        source: &PackageSource,
        version: &Version,
    ) -> Result<PackageSnapshot, ProviderError> {
        match source.provider() {
            ProviderKind::Registry | ProviderKind::GitHub => self.github.fetch(source, version),
            ProviderKind::GitLab => self.gitlab.fetch(source, version),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    struct Mock {
        packages: BTreeMap<(String, String), PackageSnapshot>,
        calls: RefCell<usize>,
    }
    impl PackageProvider for Mock {
        fn list_versions(&self, s: &PackageSource) -> Result<Vec<String>, ProviderError> {
            Ok(self
                .packages
                .keys()
                .filter(|(source, _)| source == &s.identity())
                .map(|(_, v)| v.clone())
                .collect())
        }
        fn fetch(&self, s: &PackageSource, v: &Version) -> Result<PackageSnapshot, ProviderError> {
            *self.calls.borrow_mut() += 1;
            self.packages
                .get(&(s.identity(), v.to_string()))
                .cloned()
                .ok_or_else(|| ProviderError::NotFound(v.to_string()))
        }
    }
    fn snap(name: &str, version: &str, deps: &str) -> PackageSnapshot {
        let manifest =
            format!("[package]\nname='{name}'\nversion='{version}'\n[dependencies]\n{deps}");
        PackageSnapshot {
            files: BTreeMap::from([
                ("vpm.toml".into(), manifest.into_bytes()),
                ("src/mod.vut".into(), b"fn value() -> int:\n  1\n".to_vec()),
            ]),
            revision: format!("rev-{name}-{version}"),
        }
    }
    #[test]
    fn resolves_transitive_diamond_once() {
        let mock = Mock {
            packages: BTreeMap::from([
                (
                    ("registry:a".into(), "1.0.0".into()),
                    snap("a", "1.0.0", "core='1.0.0'"),
                ),
                (
                    ("registry:b".into(), "1.0.0".into()),
                    snap("b", "1.0.0", "core='1.0.0'"),
                ),
                (
                    ("registry:core".into(), "1.0.0".into()),
                    snap("core", "1.0.0", ""),
                ),
            ]),
            calls: RefCell::new(0),
        };
        let graph = Resolver::new(&mock)
            .resolve(&[
                (PackageSource::parse("a").unwrap(), None),
                (PackageSource::parse("b").unwrap(), None),
            ])
            .unwrap();
        assert_eq!(graph.packages.len(), 3);
        assert_eq!(*mock.calls.borrow(), 3);
    }
    #[test]
    fn rejects_conflicting_versions_and_cycles() {
        let conflict = Mock {
            packages: BTreeMap::from([
                (
                    ("registry:a".into(), "1.0.0".into()),
                    snap("a", "1.0.0", "core='1.0.0'"),
                ),
                (
                    ("registry:b".into(), "1.0.0".into()),
                    snap("b", "1.0.0", "core='2.0.0'"),
                ),
                (
                    ("registry:core".into(), "1.0.0".into()),
                    snap("core", "1.0.0", ""),
                ),
                (
                    ("registry:core".into(), "2.0.0".into()),
                    snap("core", "2.0.0", ""),
                ),
            ]),
            calls: RefCell::new(0),
        };
        assert!(
            Resolver::new(&conflict)
                .resolve(&[
                    (PackageSource::parse("a").unwrap(), None),
                    (PackageSource::parse("b").unwrap(), None)
                ])
                .is_err()
        );
        let cycle = Mock {
            packages: BTreeMap::from([
                (
                    ("registry:a".into(), "1.0.0".into()),
                    snap("a", "1.0.0", "b='1.0.0'"),
                ),
                (
                    ("registry:b".into(), "1.0.0".into()),
                    snap("b", "1.0.0", "a='1.0.0'"),
                ),
            ]),
            calls: RefCell::new(0),
        };
        assert!(
            Resolver::new(&cycle)
                .resolve(&[(PackageSource::parse("a").unwrap(), None)])
                .is_err()
        );
    }

    #[test]
    fn rejects_same_name_from_different_sources() {
        let mock = Mock {
            packages: BTreeMap::from([
                (
                    ("registry:math".into(), "1.0.0".into()),
                    snap("math", "1.0.0", ""),
                ),
                (
                    ("github:alice/vut-packages/math".into(), "1.0.0".into()),
                    snap("math", "1.0.0", ""),
                ),
            ]),
            calls: RefCell::new(0),
        };
        let error = Resolver::new(&mock)
            .resolve(&[
                (PackageSource::parse("math").unwrap(), None),
                (
                    PackageSource::parse("alice/vut-packages/math").unwrap(),
                    None,
                ),
            ])
            .unwrap_err();
        assert!(error.to_string().contains("conflict"), "{error}");
    }

    #[test]
    fn rejects_remote_manifest_identity_mismatch() {
        let mock = Mock {
            packages: BTreeMap::from([(
                ("registry:math".into(), "1.0.0".into()),
                snap("different_name", "1.0.0", ""),
            )]),
            calls: RefCell::new(0),
        };
        let error = Resolver::new(&mock)
            .resolve(&[(PackageSource::parse("math").unwrap(), None)])
            .unwrap_err();
        assert!(error.to_string().contains("identity mismatch"));
    }
}
