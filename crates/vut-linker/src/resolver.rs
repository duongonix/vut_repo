//! Single source of truth for selecting and locating a native linker.
//!
//! Resolution is environment-driven and identical for Debug, Release, and
//! installed `vut`; it never consults `cfg!(debug_assertions)`. Order:
//!
//! 1. `VUT_LINKER` explicit override (`rustc`, `system`/`cc`/`cl`, `lld`).
//! 2. Vut-managed/bundled `lld` (see [`from_bundled`]).
//! 3. Platform toolchain discovery (`link.exe`/`cc` on `PATH`).
//! 4. `rustc` development fallback.
//! 5. Actionable error.
use std::path::PathBuf;

use crate::backend::{BackendKind, driver_program};
use crate::error::{LinkError, LinkFailure};
use crate::path_lookup::{find_first_on_path, find_on_path};
use crate::target::{Arch, Flavor, TargetProfile};
use crate::toolchain::{self, MsvcArch};

/// Where a resolved linker came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkerSource {
    /// `VUT_LINKER` was set.
    Override,
    /// A Vut-managed/bundled linker.
    Bundled,
    /// A discovered platform toolchain.
    System,
    /// The `rustc` development fallback.
    Rustc,
}

impl LinkerSource {
    /// Short machine-stable label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Override => "override",
            Self::Bundled => "bundled",
            Self::System => "system",
            Self::Rustc => "rustc",
        }
    }
}

/// A located linker: driver program, backend family, and the environment and
/// library search paths it needs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedLinker {
    pub kind: BackendKind,
    /// Program path or bare name to execute.
    pub program: PathBuf,
    /// Library search directories to add (`/LIBPATH:` on MSVC, `-L` elsewhere).
    pub search_paths: Vec<PathBuf>,
    /// Environment variables to set for the linker process.
    pub environment: Vec<(String, String)>,
    pub source: LinkerSource,
}

impl ResolvedLinker {
    fn new(kind: BackendKind, program: impl Into<PathBuf>, source: LinkerSource) -> Self {
        Self {
            kind,
            program: program.into(),
            search_paths: Vec::new(),
            environment: Vec::new(),
            source,
        }
    }
}

/// Resolves the linker for a target.
///
/// # Errors
/// Returns a classified [`LinkError`] with an actionable hint when no linker can
/// be found.
pub fn resolve(profile: &TargetProfile) -> Result<ResolvedLinker, LinkError> {
    if let Some(resolved) = from_override(profile) {
        return Ok(resolved);
    }
    if let Some(resolved) = from_bundled(profile) {
        return Ok(resolved);
    }
    if let Some(resolved) = from_system(profile) {
        return Ok(resolved);
    }
    if let Some(resolved) = from_rustc(profile) {
        return Ok(resolved);
    }
    Err(no_linker_error(profile))
}

/// `VUT_LINKER` override. Unknown values fall through to discovery.
fn from_override(profile: &TargetProfile) -> Option<ResolvedLinker> {
    let value = std::env::var("VUT_LINKER").ok()?;
    let lowered = value.to_ascii_lowercase();
    let kind = match lowered.as_str() {
        "rustc" => BackendKind::Rustc,
        "system" | "cc" | "cl" => BackendKind::System,
        "lld" | "lld-link" => BackendKind::Lld,
        _ => return None,
    };
    Some(ResolvedLinker::new(
        kind,
        driver_program(profile, kind),
        LinkerSource::Override,
    ))
}

/// A Vut-managed/bundled linker shipped with the installation.
///
/// A bundled `lld` replaces the platform linker. On Windows MSVC it still needs
/// the MSVC/UCRT import libraries, which are discovered (never redistributed).
fn from_bundled(profile: &TargetProfile) -> Option<ResolvedLinker> {
    let name = driver_program(profile, BackendKind::Lld);
    let program = bundled_linker_path(name)?;
    let mut resolved = ResolvedLinker::new(BackendKind::Lld, program, LinkerSource::Bundled);
    if profile.flavor() == Flavor::Msvc
        && let Some(toolchain) = toolchain::discover_msvc(msvc_arch(profile.arch()))
    {
        let lib_env = toolchain.lib_env();
        let include_env = toolchain.include_env();
        resolved.search_paths = toolchain.lib_paths;
        resolved.environment.push(("LIB".into(), lib_env));
        resolved.environment.push(("INCLUDE".into(), include_env));
    }
    Some(resolved)
}

/// Searches Vut-managed locations for a bundled linker program.
fn bundled_linker_path(name: &str) -> Option<PathBuf> {
    let mut directories: Vec<PathBuf> = Vec::new();
    if let Some(dir) = std::env::var_os("VUT_LINKER_DIR") {
        directories.push(PathBuf::from(dir));
    }
    if let Some(home) = std::env::var_os("VUT_HOME") {
        directories.push(PathBuf::from(home).join("bin"));
    } else if let Some(home) = user_home() {
        directories.push(home.join(".vut").join("bin"));
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        directories.push(parent.to_path_buf());
    }
    directories
        .into_iter()
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

fn user_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

/// Platform toolchain discovery: MSVC (via `vswhere`) on Windows, `cc`/`clang`
/// elsewhere.
fn from_system(profile: &TargetProfile) -> Option<ResolvedLinker> {
    match profile.flavor() {
        Flavor::Msvc => from_msvc(profile),
        Flavor::Darwin => {
            let program = find_first_on_path(&["cc", "clang"])?.1;
            Some(ResolvedLinker::new(
                BackendKind::System,
                program,
                LinkerSource::System,
            ))
        }
        _ => {
            let program = find_first_on_path(&["cc", "clang", "gcc"])?.1;
            Some(ResolvedLinker::new(
                BackendKind::System,
                program,
                LinkerSource::System,
            ))
        }
    }
}

/// Discovers an installed MSVC toolchain, then falls back to `link.exe` on
/// `PATH` (a Developer Command Prompt).
fn from_msvc(profile: &TargetProfile) -> Option<ResolvedLinker> {
    if let Some(toolchain) = toolchain::discover_msvc(msvc_arch(profile.arch())) {
        let lib_env = toolchain.lib_env();
        let include_env = toolchain.include_env();
        let mut resolved = ResolvedLinker::new(
            BackendKind::System,
            toolchain.link.clone(),
            LinkerSource::System,
        );
        resolved.search_paths = toolchain.lib_paths;
        resolved.environment.push(("LIB".into(), lib_env));
        resolved.environment.push(("INCLUDE".into(), include_env));
        return Some(resolved);
    }
    let program = find_on_path("link.exe")?;
    let mut resolved = ResolvedLinker::new(BackendKind::System, program, LinkerSource::System);
    // `LIB` already points the MSVC linker at the SDK/CRT; expose it as explicit
    // search paths so behavior does not depend on the child process environment.
    resolved.search_paths = env_search_paths("LIB");
    Some(resolved)
}

fn msvc_arch(arch: Arch) -> MsvcArch {
    match arch {
        Arch::Aarch64 => MsvcArch::Aarch64,
        Arch::X86_64 | Arch::Other => MsvcArch::X64,
    }
}

/// `rustc` development fallback.
fn from_rustc(_profile: &TargetProfile) -> Option<ResolvedLinker> {
    let program = find_on_path("rustc")?;
    Some(ResolvedLinker::new(
        BackendKind::Rustc,
        program,
        LinkerSource::Rustc,
    ))
}

/// Splits a `;`/`:`-separated environment variable into existing directories.
fn env_search_paths(name: &str) -> Vec<PathBuf> {
    let Some(value) = std::env::var_os(name) else {
        return Vec::new();
    };
    std::env::split_paths(&value)
        .filter(|path| path.is_dir())
        .collect()
}

fn no_linker_error(profile: &TargetProfile) -> LinkError {
    let driver = driver_program(profile, BackendKind::System);
    LinkError::classified(
        format!(
            "no native linker found for {} (looked for `{driver}` on PATH)",
            profile.triple()
        ),
        LinkFailure::ToolNotFound,
    )
    .with_hint(crate::backend::toolchain_hint(profile))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host_profile() -> TargetProfile {
        TargetProfile::parse(&target_lexicon::HOST.to_string()).expect("host triple")
    }

    #[test]
    fn resolves_a_linker_for_the_host() {
        // The resolver is environment-driven and must find a linker wherever
        // `cargo test` itself can run (a Rust toolchain is present).
        let resolved = resolve(&host_profile()).expect("host linker");
        assert!(matches!(
            resolved.source,
            LinkerSource::Override
                | LinkerSource::Bundled
                | LinkerSource::System
                | LinkerSource::Rustc
        ));
    }

    #[test]
    fn resolution_is_deterministic() {
        let first = resolve(&host_profile()).map(|resolved| resolved.source);
        let second = resolve(&host_profile()).map(|resolved| resolved.source);
        assert_eq!(first, second);
    }

    #[test]
    fn missing_linker_error_is_actionable() {
        let profile = TargetProfile::parse("x86_64-pc-windows-msvc").unwrap();
        let error = no_linker_error(&profile);
        assert_eq!(error.failure(), LinkFailure::ToolNotFound);
        assert!(error.message().contains("x86_64-pc-windows-msvc"));
    }
}
