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
    for path in snapshot.files.keys() {
        if !safe_relative(path) {
            return Err(format!(
                "package `{}` contains unsafe path `{path}`",
                id.name
            ));
        }
    }
    let has_library = snapshot.files.contains_key("src/mod.vut");
    if !has_library && manifest.bin.is_empty() {
        return Err(format!(
            "package `{}` has neither a `src/mod.vut` library nor any `[[bin]]` entry",
            id.name
        ));
    }
    for bin in &manifest.bin {
        if !snapshot.files.contains_key(&bin.path) {
            return Err(format!(
                "package `{}` bin `{}` declares missing path `{}`",
                id.name, bin.name, bin.path
            ));
        }
    }
    validate_native(snapshot, &manifest, &id.name)?;
    Ok(())
}

fn validate_native(
    snapshot: &PackageSnapshot,
    manifest: &Manifest,
    name: &str,
) -> Result<(), String> {
    if !manifest.native.libraries.is_empty() {
        return Err(format!(
            "package `{name}` must not ship prebuilt native libraries; publish native source with `[native] build`"
        ));
    }
    if let Some(build_path) = &manifest.native.build {
        let bytes = snapshot.files.get(build_path).ok_or_else(|| {
            format!("package `{name}` declares native build `{build_path}` but the file is missing")
        })?;
        let text = std::str::from_utf8(bytes)
            .map_err(|_| format!("package `{name}` native build file is not valid UTF-8"))?;
        crate::native::NativeBuild::parse(text)
            .map_err(|error| format!("package `{name}`: {error}"))?;
    }
    if let Some(path) = snapshot
        .files
        .keys()
        .find(|path| crate::native::is_prebuilt_artifact(path))
    {
        return Err(format!(
            "package `{name}` must not contain prebuilt native artifact `{path}`"
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
                        "src/mod.vut".into(),
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
    #[test]
    fn validates_registry_native_source_packages() {
        let (mut snapshot, id) = fixture();
        snapshot.files.insert(
            "vpm.toml".into(),
            b"[package]\nname='math'\nversion='1.0.0'\n\n[native]\nbuild='native/build.toml'\n"
                .to_vec(),
        );
        snapshot.files.insert(
            "native/build.toml".into(),
            b"backend = 'cc'\n[cc]\nsources = ['native/src/math.c']\n".to_vec(),
        );
        snapshot.files.insert(
            "native/src/math.c".into(),
            b"int vut_math_add(int a, int b) { return a + b; }".to_vec(),
        );
        let checksum = crate::resolver::checksum(&snapshot);
        assert!(validate(&snapshot, &id, &checksum).is_ok());

        let mut binary = snapshot.clone();
        binary
            .files
            .insert("native/lib/math.lib".into(), b"binary".to_vec());
        let checksum = crate::resolver::checksum(&binary);
        assert!(validate(&binary, &id, &checksum).is_err());

        let mut prebuilt = snapshot.clone();
        prebuilt.files.insert(
            "vpm.toml".into(),
            b"[package]\nname='math'\nversion='1.0.0'\n\n[native]\nlibraries=['native/math.lib']\n"
                .to_vec(),
        );
        let checksum = crate::resolver::checksum(&prebuilt);
        assert!(validate(&prebuilt, &id, &checksum).is_err());
    }

    #[test]
    fn accepts_cli_only_packages_and_requires_bin_sources() {
        let (snapshot, id) = fixture();
        let mut cli = snapshot.clone();
        cli.files.remove("src/mod.vut");
        cli.files.insert(
            "vpm.toml".into(),
            b"[package]\nname='math'\nversion='1.0.0'\n\n[[bin]]\nname='math'\npath='src/bin/math.vut'\n".to_vec(),
        );
        cli.files.insert(
            "src/bin/math.vut".into(),
            b"fn main():\n  out(\"hi\")\n".to_vec(),
        );
        let checksum = crate::resolver::checksum(&cli);
        assert!(validate(&cli, &id, &checksum).is_ok());

        let mut missing_bin = cli;
        missing_bin.files.remove("src/bin/math.vut");
        let checksum = crate::resolver::checksum(&missing_bin);
        assert!(validate(&missing_bin, &id, &checksum).is_err());
    }
}
