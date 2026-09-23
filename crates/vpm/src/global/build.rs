//! Builds one published package command-line entry point into an executable.
//!
//! A package may declare several `[[bin]]` entries. Each is compiled as its own
//! executable, so the package source is staged with every *other* bin removed.
//! This keeps exactly one `fn main` in the compiled program.

use crate::{Manifest, PackageId, ResolvedGraph};
use std::path::{Path, PathBuf};
use vut_compiler::{BuildMode, CompilerConfig, CompilerSession};

/// Compiles `bin` from `package` (plus its resolved dependency roots) to
/// `output`.
pub fn build_bin(
    store: &crate::store::Store,
    graph: &ResolvedGraph,
    package: &PackageId,
    manifest: &Manifest,
    bin: &crate::manifest::Bin,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let stage = stage_dir(store, package, bin);
    if stage.exists() {
        std::fs::remove_dir_all(&stage)?;
    }
    let source = stage.join("src");
    copy_tree(&store.path(package).join("src"), &source)?;
    for other in &manifest.bin {
        if other.path == bin.path {
            continue;
        }
        let path = stage.join(other.path.replace('\\', "/"));
        if path.is_file() {
            std::fs::remove_file(path)?;
        }
    }

    let mut roots = vec![(source, Vec::new())];
    for id in graph.packages.keys() {
        if id == package {
            continue;
        }
        roots.push((store.path(id).join("src"), vec![id.name.clone()]));
    }

    let mut config =
        CompilerConfig::for_target(CompilerConfig::default().target, BuildMode::Release);
    config.runtime_library = vut_paths::runtime_library(&config.target);
    config.startup_object = vut_paths::startup_object(&config.target);
    config.output_dir = Some(store.build_cache(&config.target, true));
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    CompilerSession::new(config).emit_executable_from_roots(&roots, output)?;
    Ok(())
}

fn stage_dir(
    store: &crate::store::Store,
    package: &PackageId,
    bin: &crate::manifest::Bin,
) -> PathBuf {
    store
        .cache_dir()
        .join("exec")
        .join(component(&package.name))
        .join(component(&package.version.to_string()))
        .join(component(&bin.name))
}

/// Sanitises one value into a safe single path component.
pub(super) fn component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PackageSnapshot, PackageSource, ResolvedGraph, resolver::ResolvedPackage};
    use semver::Version;
    use std::collections::BTreeMap;

    #[test]
    fn builds_and_runs_a_package_bin() {
        let target = CompilerConfig::default().target;
        if vut_paths::runtime_library(&target).is_none() {
            eprintln!("skipping: no runtime library for {target}");
            return;
        }
        let root = std::env::temp_dir().join(format!("vpm-global-build-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let store = crate::store::Store::at(root.join("store"));
        let id = PackageId {
            source: PackageSource::parse("hello").unwrap(),
            name: "hello".into(),
            version: Version::parse("1.0.0").unwrap(),
        };
        let package_root = store.path(&id);
        std::fs::create_dir_all(package_root.join("src/bin")).unwrap();
        std::fs::write(
            package_root.join("vpm.toml"),
            "[package]\nname='hello'\nversion='1.0.0'\n\n[[bin]]\nname='hello'\npath='src/bin/hello.vut'\n",
        )
        .unwrap();
        std::fs::write(
            package_root.join("src/bin/hello.vut"),
            "fn main():\n  out(\"global hello\")\n",
        )
        .unwrap();

        let snapshot = PackageSnapshot {
            files: BTreeMap::new(),
            revision: "r".into(),
        };
        let graph = ResolvedGraph {
            packages: BTreeMap::from([(
                id.clone(),
                ResolvedPackage {
                    id: id.clone(),
                    revision: "r".into(),
                    checksum: String::new(),
                    dependencies: Vec::new(),
                    snapshot,
                },
            )]),
        };
        let manifest: Manifest = toml::from_str(
            "[package]\nname='hello'\nversion='1.0.0'\n\n[[bin]]\nname='hello'\npath='src/bin/hello.vut'\n",
        )
        .unwrap();
        let output = root.join(if cfg!(windows) { "hello.exe" } else { "hello" });
        build_bin(&store, &graph, &id, &manifest, &manifest.bin[0], &output).unwrap();
        let output = std::process::Command::new(&output).output().unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "global hello"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
