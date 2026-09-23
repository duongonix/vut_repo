//! Platform toolchain discovery.
//!
//! On Windows, a linker (`link.exe`) is not useful without the MSVC and Windows
//! SDK import libraries. This module discovers an installed Visual Studio
//! toolchain (via `vswhere`) and derives the `link.exe` path plus the `LIB` and
//! `INCLUDE` directories it needs, so the compiler can link without the user
//! running `vcvars`.
//!
//! Nothing is redistributed: discovery reuses an existing official installation.
use std::path::{Path, PathBuf};

/// A discovered MSVC + Windows SDK toolchain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MsvcToolchain {
    /// `link.exe` path.
    pub link: PathBuf,
    /// Import-library search directories (`LIB`).
    pub lib_paths: Vec<PathBuf>,
    /// Header search directories (`INCLUDE`).
    pub include_paths: Vec<PathBuf>,
}

impl MsvcToolchain {
    /// `LIB` value (`;`-joined) for the linker process.
    #[must_use]
    pub fn lib_env(&self) -> String {
        join_paths(&self.lib_paths)
    }
    /// `INCLUDE` value (`;`-joined) for the compiler process.
    #[must_use]
    pub fn include_env(&self) -> String {
        join_paths(&self.include_paths)
    }
}

fn join_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(";")
}

/// Target CPU architecture used to pick the MSVC host tools and library dirs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MsvcArch {
    X64,
    Aarch64,
    X86,
}

impl MsvcArch {
    fn msvc_dir(self) -> &'static str {
        match self {
            Self::X64 => "x64",
            Self::Aarch64 => "arm64",
            Self::X86 => "x86",
        }
    }
    /// Host tools directory prefix (`Hostx64`/`Hostx86`).
    fn host_prefix(self) -> &'static str {
        match self {
            Self::X86 => "Hostx86",
            _ => "Hostx64",
        }
    }
}

/// Discovers an installed MSVC + Windows SDK toolchain for `arch`.
///
/// Returns `None` off Windows or when no Visual Studio toolchain is installed.
#[must_use]
pub fn discover_msvc(arch: MsvcArch) -> Option<MsvcToolchain> {
    discover_msvc_impl(arch)
}

#[cfg(windows)]
fn discover_msvc_impl(arch: MsvcArch) -> Option<MsvcToolchain> {
    let installation = vswhere_installation_path()?;
    let msvc = newest_dir(&installation.join("VC").join("Tools").join("MSVC"))?;
    let link = msvc
        .join("bin")
        .join(arch.host_prefix())
        .join(arch.msvc_dir())
        .join("link.exe");
    if !link.is_file() {
        return None;
    }
    let mut lib_paths = vec![msvc.join("lib").join(arch.msvc_dir())];
    let mut include_paths = vec![msvc.join("include")];
    if let Some(sdk) = windows_sdk_root() {
        let sdk_lib = newest_dir(&sdk.join("Lib"));
        if let Some(sdk_lib) = sdk_lib {
            lib_paths.push(sdk_lib.join("ucrt").join(arch.msvc_dir()));
            lib_paths.push(sdk_lib.join("um").join(arch.msvc_dir()));
        }
        let sdk_include = newest_dir(&sdk.join("Include"));
        if let Some(sdk_include) = sdk_include {
            include_paths.push(sdk_include.join("ucrt"));
            include_paths.push(sdk_include.join("um"));
            include_paths.push(sdk_include.join("shared"));
        }
    }
    lib_paths.retain(|path| path.is_dir());
    include_paths.retain(|path| path.is_dir());
    Some(MsvcToolchain {
        link,
        lib_paths,
        include_paths,
    })
}

#[cfg(not(windows))]
fn discover_msvc_impl(_arch: MsvcArch) -> Option<MsvcToolchain> {
    None
}

/// Runs `vswhere.exe` to locate the newest Visual Studio installation.
#[cfg(windows)]
fn vswhere_installation_path() -> Option<PathBuf> {
    let vswhere = vswhere_path()?;
    let output = std::process::Command::new(vswhere)
        .args([
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property",
            "installationPath",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// The well-known `vswhere.exe` location.
#[cfg(windows)]
fn vswhere_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    for key in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(root) = std::env::var_os(key) {
            candidates.push(
                PathBuf::from(root)
                    .join("Microsoft Visual Studio")
                    .join("Installer")
                    .join("vswhere.exe"),
            );
        }
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

/// The Windows 10/11 SDK root (`Windows Kits\10`).
#[cfg(windows)]
fn windows_sdk_root() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(root) = std::env::var_os("WindowsSdkDir") {
        candidates.push(PathBuf::from(root));
    }
    for key in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(root) = std::env::var_os(key) {
            candidates.push(PathBuf::from(root).join("Windows Kits").join("10"));
        }
    }
    candidates.into_iter().find(|candidate| candidate.is_dir())
}

/// Returns the newest versioned subdirectory of `root`.
fn newest_dir(root: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut best: Option<(Vec<u64>, PathBuf)> = None;
    for entry in entries.flatten() {
        if !entry.file_type().ok()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let version = version_key(&name);
        if version.is_empty() {
            continue;
        }
        let path = entry.path();
        if best.as_ref().is_none_or(|(current, _)| version > *current) {
            best = Some((version, path));
        }
    }
    best.map(|(_, path)| path)
}

fn version_key(name: &str) -> Vec<u64> {
    name.split('.')
        .map_while(|part| part.parse::<u64>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_keys_compare_numerically() {
        assert!(version_key("14.44.35207") > version_key("14.40.0"));
        assert!(version_key("10.0.26100.0") > version_key("10.0.22621.0"));
        assert!(version_key("not-a-version").is_empty());
    }

    #[test]
    fn arch_directories_follow_msvc_layout() {
        assert_eq!(MsvcArch::X64.msvc_dir(), "x64");
        assert_eq!(MsvcArch::X64.host_prefix(), "Hostx64");
        assert_eq!(MsvcArch::Aarch64.msvc_dir(), "arm64");
        assert_eq!(MsvcArch::Aarch64.host_prefix(), "Hostx64");
    }

    #[test]
    fn msvc_discovery_is_optional_and_safe() {
        // Either a toolchain is found and `link.exe` exists, or none is
        // reported (non-Windows, or no Visual Studio). It must never panic.
        if let Some(toolchain) = discover_msvc(MsvcArch::X64) {
            assert!(toolchain.link.is_file(), "{}", toolchain.link.display());
            assert!(!toolchain.lib_env().is_empty() || toolchain.lib_paths.is_empty());
        }
    }
}
