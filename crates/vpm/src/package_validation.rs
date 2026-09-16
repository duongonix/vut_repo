use crate::{Manifest, PackageId, PackageSnapshot};
use std::path::{Component, Path};

pub(crate) fn validate(
    snapshot: &PackageSnapshot,
    id: &PackageId,
    expected_checksum: &str,
) -> Result<(), String> {
    if crate::resolver::checksum(snapshot) != expected_checksum {
        return Err(format!(
            "package `{}` checksum does not match resolved contents",
            id.name
        ));
    }
    let manifest_bytes = snapshot
        .files
        .get("vpm.toml")
        .ok_or_else(|| format!("package `{}` has no vpm.toml", id.name))?;
    let manifest_text = std::str::from_utf8(manifest_bytes)
        .map_err(|_| "package manifest is not valid UTF-8".to_owned())?;
    let manifest: Manifest = toml::from_str(manifest_text)
        .map_err(|error| format!("invalid package manifest: {error}"))?;
    manifest.validate()?;
    if manifest.package.name != id.name {
        return Err(format!(
            "package name mismatch: expected `{}`, found `{}`",
            id.name, manifest.package.name
        ));
    }
    if manifest.package.version != id.version.to_string() {
        return Err(format!(
            "package version mismatch: expected `{}`, found `{}`",
            id.version, manifest.package.version
        ));
    }
    if snapshot.revision.trim().is_empty() {
        return Err(format!(
            "package `{}` has an empty source revision",
            id.name
        ));
    }
    let mut has_source = false;
    for path in snapshot.files.keys() {
        if !safe_relative(path) {
            return Err(format!(
                "package `{}` contains unsafe path `{path}`",
                id.name
            ));
        }
        if path.starts_with("src/")
            && Path::new(path)
                .extension()
                .is_some_and(|extension| extension == "vut")
        {
            has_source = true;
        }
    }
    if !has_source {
        return Err(format!(
            "package `{}` has no Vut source below src/",
            id.name
        ));
    }
    Ok(())
}

fn safe_relative(value: &str) -> bool {
    !value.is_empty()
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PackageSource;
    use semver::Version;
    use std::collections::BTreeMap;
    fn fixture() -> (PackageSnapshot, PackageId) {
        (
            PackageSnapshot {
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
                revision: "commit".into(),
            },
            PackageId {
                source: PackageSource::parse("math").unwrap(),
                name: "math".into(),
                version: Version::parse("1.0.0").unwrap(),
            },
        )
    }
    #[test]
    fn validates_identity_layout_integrity_and_paths() {
        let (mut snapshot, id) = fixture();
        let checksum = crate::resolver::checksum(&snapshot);
        assert!(validate(&snapshot, &id, &checksum).is_ok());
        assert!(validate(&snapshot, &id, "bad").is_err());
        snapshot.files.insert("../escape".into(), vec![]);
        let checksum = crate::resolver::checksum(&snapshot);
        assert!(validate(&snapshot, &id, &checksum).is_err());
    }
}
