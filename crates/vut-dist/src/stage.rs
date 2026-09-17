//! Distribution assembly: staging tree, manifest, archive and checksum.
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::archive;
use crate::checksum;
use crate::cli::Args;
use crate::manifest::{FORMAT_VERSION, Manifest};

/// Result of assembling one distribution.
pub struct Artifact {
    pub archive: PathBuf,
    pub checksum: String,
    pub staging: PathBuf,
}

/// Assembles a distribution artifact for the requested target.
///
/// # Errors
/// Returns an error when the target is invalid, a required artifact is missing,
/// or I/O/encoding fails.
pub fn assemble(args: &Args) -> Result<Artifact, Box<dyn Error>> {
    let profile = vut_linker::TargetProfile::parse(&args.target)?;
    let windows = profile.is_windows();
    let version = env!("CARGO_PKG_VERSION");
    let dist_name = format!("vut-v{version}-{}", args.target);
    let staging = args.out.join(&dist_name);
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    let bin_dir = staging.join("bin");
    let runtime_dir = staging.join("lib").join("runtime").join(&args.target);
    let std_dir = staging.join("std");
    std::fs::create_dir_all(&bin_dir)?;
    std::fs::create_dir_all(&runtime_dir)?;

    stage_binaries(&args.bin_dir, &bin_dir, windows)?;
    stage_runtime(&profile, args, &runtime_dir)?;
    stage_stdlib(args, &std_dir)?;

    let archive_name = format!("{dist_name}.{}", if windows { "zip" } else { "tar.gz" });
    let manifest = Manifest {
        format_version: FORMAT_VERSION,
        vut: version.to_owned(),
        vpm: args
            .vpm_version
            .clone()
            .unwrap_or_else(|| version.to_owned()),
        stdlib: version.to_owned(),
        runtime: version.to_owned(),
        abi_version: vut_runtime::abi::VERSION,
        target: args.target.clone(),
        archive: archive_name.clone(),
        build_commit: args
            .build_commit
            .clone()
            .or_else(|| std::env::var("GITHUB_SHA").ok()),
        minimum_os: args.minimum_os.clone(),
    };
    std::fs::write(staging.join("manifest.json"), manifest.to_json())?;

    let archive_path = args.out.join(&archive_name);
    if windows {
        archive::zip_tree(&staging, &archive_path)?;
    } else {
        archive::tar_gz_tree(&staging, &archive_path)?;
    }
    let checksum = checksum::sha256_file(&archive_path)?;
    append_checksum(&args.out, &checksum, &archive_name)?;

    Ok(Artifact {
        archive: archive_path,
        checksum,
        staging,
    })
}

fn stage_binaries(
    bin_root: &Path,
    destination: &Path,
    windows: bool,
) -> Result<(), Box<dyn Error>> {
    for name in ["vut", "vpm", "vut-lsp"] {
        let file = executable(name, windows);
        let source = bin_root.join(&file);
        if !source.is_file() {
            return Err(format!("missing binary `{}`", source.display()).into());
        }
        std::fs::copy(&source, destination.join(&file))?;
    }
    Ok(())
}

fn stage_runtime(
    profile: &vut_linker::TargetProfile,
    args: &Args,
    destination: &Path,
) -> Result<(), Box<dyn Error>> {
    let windows = profile.is_windows();
    let core = find_in(&args.bin_dir, &core_source_names(windows)).ok_or_else(|| {
        format!(
            "core runtime archive not found in `{}`",
            args.bin_dir.display()
        )
    })?;
    std::fs::copy(&core, destination.join(core_distribution_name(windows)))?;

    let stdlib = find_in(&args.bin_dir, &stdlib_source_names(windows)).ok_or_else(|| {
        format!(
            "native stdlib archive not found in `{}`",
            args.bin_dir.display()
        )
    })?;
    std::fs::copy(&stdlib, destination.join(stdlib_distribution_name(windows)))?;

    let startup = match &args.startup {
        Some(path) => path.clone(),
        None => match find_in(&args.bin_dir, &[profile.startup_object_name()]) {
            Some(path) => path,
            None => vut_linker::StartupObject::build(profile, &args.out.join("startup-cache"))?,
        },
    };
    std::fs::copy(&startup, destination.join(profile.startup_object_name()))?;
    Ok(())
}

fn stage_stdlib(args: &Args, destination: &Path) -> Result<(), Box<dyn Error>> {
    let root = args.std.clone().unwrap_or_else(default_stdlib_root);
    if !root.is_dir() {
        return Err(format!("stdlib source `{}` was not found", root.display()).into());
    }
    copy_tree(&root, destination)?;
    Ok(())
}

fn append_checksum(out: &Path, checksum: &str, archive_name: &str) -> std::io::Result<()> {
    use std::io::Write as _;
    let path = out.join("SHA256SUMS");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{checksum}  {archive_name}")
}

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn find_in(directory: &Path, names: &[&str]) -> Option<PathBuf> {
    names
        .iter()
        .map(|name| directory.join(name))
        .find(|candidate| candidate.is_file())
}

fn executable(name: &str, windows: bool) -> String {
    if windows {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

fn core_source_names(windows: bool) -> Vec<&'static str> {
    if windows {
        vec!["libvut_runtime.rlib", "vut_runtime.lib", "vut-core.lib"]
    } else {
        vec!["libvut_runtime.a", "libvut-core.a"]
    }
}

fn stdlib_source_names(windows: bool) -> Vec<&'static str> {
    if windows {
        vec!["vut_stdlib_native.lib", "vut-stdlib.lib"]
    } else {
        vec!["libvut_stdlib_native.a", "libvut-stdlib.a"]
    }
}

fn core_distribution_name(windows: bool) -> &'static str {
    if windows {
        "vut-core.lib"
    } else {
        "libvut-core.a"
    }
}

fn stdlib_distribution_name(windows: bool) -> &'static str {
    if windows {
        "vut-stdlib.lib"
    } else {
        "libvut-stdlib.a"
    }
}

fn default_stdlib_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vut-stdlib/std")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distribution_names_follow_the_platform() {
        assert_eq!(core_distribution_name(true), "vut-core.lib");
        assert_eq!(core_distribution_name(false), "libvut-core.a");
        assert_eq!(stdlib_distribution_name(true), "vut-stdlib.lib");
        assert_eq!(stdlib_distribution_name(false), "libvut-stdlib.a");
        assert_eq!(executable("vut", true), "vut.exe");
        assert_eq!(executable("vut", false), "vut");
    }

    #[test]
    fn copy_tree_reproduces_nested_layout() {
        let root = std::env::temp_dir().join(format!("vut-dist-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let source = root.join("src");
        std::fs::create_dir_all(source.join("os")).unwrap();
        std::fs::write(source.join("os/mod.vut"), b"module").unwrap();
        std::fs::write(source.join("_internal").with_extension(""), b"").ok();
        let destination = root.join("dst");
        copy_tree(&source, &destination).unwrap();
        assert!(destination.join("os/mod.vut").is_file());
        std::fs::remove_dir_all(root).unwrap();
    }
}
