//! Distribution assembly: staging tree, manifest, version metadata, archive and
//! checksum.
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::archive;
use crate::checksum;
use crate::cli::AssembleArgs;
use crate::manifest::{FORMAT_VERSION, Manifest};
use crate::version::VersionInfo;

/// Build profile recorded for every `vut-dist` artifact (release-only tool).
const PROFILE: &str = "release";

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
pub fn assemble(args: &AssembleArgs) -> Result<Artifact, Box<dyn Error>> {
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
    stage_legal(args, &staging)?;

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
        channel: args.channel.clone(),
        profile: PROFILE.to_owned(),
        cranelift: args.cranelift.clone(),
        build_commit: args
            .build_commit
            .clone()
            .or_else(|| std::env::var("GITHUB_SHA").ok()),
        minimum_os: args.minimum_os.clone(),
    };
    let version_info = VersionInfo {
        vut: manifest.vut.clone(),
        vpm: manifest.vpm.clone(),
        stdlib: manifest.stdlib.clone(),
        runtime: manifest.runtime.clone(),
        abi_version: manifest.abi_version,
        manifest_format: manifest.format_version,
        channel: manifest.channel.clone(),
        profile: manifest.profile.clone(),
        cranelift: manifest.cranelift.clone(),
        target: manifest.target.clone(),
        build_commit: manifest.build_commit.clone(),
    };
    std::fs::write(staging.join("manifest.json"), manifest.to_json())?;
    std::fs::write(staging.join("version.json"), version_info.to_json())?;

    let archive_path = args.out.join(&archive_name);
    if windows {
        archive::zip_tree(&staging, &archive_path)?;
    } else {
        archive::tar_gz_tree(&staging, &archive_path)?;
    }
    // Keep the per-target manifest next to the archive so the release-level
    // manifest can be merged without unpacking every artifact.
    std::fs::write(
        args.out.join(format!("{dist_name}.manifest.json")),
        manifest.to_json(),
    )?;
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

/// Copies the license (required) and third-party notices (optional) into the
/// artifact so a distribution is self-describing.
fn stage_legal(args: &AssembleArgs, destination: &Path) -> Result<(), Box<dyn Error>> {
    let license = args
        .license
        .clone()
        .unwrap_or_else(default_license)
        .canonicalize()
        .map_err(|_| "the LICENSE file was not found; pass --license <path>")?;
    std::fs::copy(&license, destination.join("LICENSE"))?;

    let notices = args.notices.clone().unwrap_or_else(default_notices);
    if notices.is_file() {
        std::fs::copy(&notices, destination.join("THIRD-PARTY-NOTICES.md"))?;
    }
    Ok(())
}

fn stage_runtime(
    profile: &vut_linker::TargetProfile,
    args: &AssembleArgs,
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

fn stage_stdlib(args: &AssembleArgs, destination: &Path) -> Result<(), Box<dyn Error>> {
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
    // A distribution ships the linkable static archive, never the development
    // `.rlib` (which lacks the embedded Rust std the standalone linker needs).
    if windows {
        vec!["vut_runtime.lib", "vut-core.lib", "libvut_runtime.rlib"]
    } else {
        vec!["libvut_runtime.a", "libvut-core.a", "libvut_runtime.rlib"]
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

fn default_license() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../LICENSE")
}

fn default_notices() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../THIRD-PARTY-NOTICES.md")
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
