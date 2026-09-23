//! Linker backend selection and platform diagnostics.
//!
//! A backend turns a [`LinkPlan`](crate::LinkPlan) into a native executable.
//! Selection is delegated to [`crate::resolver`], which is environment-driven and
//! identical for Debug, Release, and installed Vut.
pub mod rustc;
pub mod system;

use crate::error::LinkError;
use crate::resolver::{self, ResolvedLinker};
use crate::target::{Flavor, TargetProfile};

/// Concrete linker backend family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendKind {
    /// Development/migration fallback driven by `rustc`.
    Rustc,
    /// Platform-native driver (`link.exe` on MSVC, `cc` elsewhere).
    System,
    /// LLVM `lld` driver (`lld-link`, `ld.lld`, `ld64.lld`).
    Lld,
}

/// A native linker implementation.
pub trait LinkerBackend {
    /// Machine-stable backend name.
    fn name(&self) -> &'static str;
    /// Links the plan into `plan.output`.
    ///
    /// # Errors
    /// Returns tool discovery, classification, and linker diagnostics.
    fn link(&self, plan: &LinkPlan) -> Result<(), LinkError>;
}

/// Linker program name a backend uses for a target.
#[must_use]
pub fn driver_program(profile: &TargetProfile, kind: BackendKind) -> &'static str {
    match kind {
        BackendKind::Rustc => "rustc",
        BackendKind::Lld => match profile.flavor() {
            Flavor::Msvc => "lld-link",
            Flavor::Darwin => "ld64.lld",
            _ => "ld.lld",
        },
        BackendKind::System => match profile.flavor() {
            Flavor::Msvc => "link.exe",
            _ => "cc",
        },
    }
}

/// Actionable installation hint when a platform toolchain/SDK is missing.
#[must_use]
pub fn toolchain_hint(profile: &TargetProfile) -> &'static str {
    match profile.flavor() {
        Flavor::Msvc => {
            "install Visual Studio Build Tools with the C++ workload and the Windows SDK, \
             or install a Vut distribution that bundles its linker"
        }
        Flavor::Darwin => "install the Xcode Command Line Tools with `xcode-select --install`",
        Flavor::Gnu | Flavor::Musl if !profile.is_windows() => {
            "install a C toolchain (`cc` and binutils)"
        }
        _ => "install the platform native linker and its SDK",
    }
}

/// Selects the backend for a target by resolving the linker.
///
/// # Errors
/// Returns [`LinkError`] when the target is malformed or no linker can be found.
pub fn select(target: &str) -> Result<Box<dyn LinkerBackend>, LinkError> {
    let profile = TargetProfile::parse(target)?;
    let resolved = resolver::resolve(&profile)?;
    Ok(backend_for(resolved, profile))
}

/// Builds the backend for an already-resolved linker.
#[must_use]
pub fn backend_for(resolved: ResolvedLinker, profile: TargetProfile) -> Box<dyn LinkerBackend> {
    match resolved.kind {
        BackendKind::Rustc => Box::new(rustc::RustcBackend::new(resolved)),
        BackendKind::System | BackendKind::Lld => {
            Box::new(system::SystemBackend::new(resolved, profile))
        }
    }
}

use crate::plan::LinkPlan;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_programs_follow_flavor_and_backend() {
        let msvc = TargetProfile::parse("x86_64-pc-windows-msvc").unwrap();
        assert_eq!(driver_program(&msvc, BackendKind::System), "link.exe");
        assert_eq!(driver_program(&msvc, BackendKind::Lld), "lld-link");
        assert_eq!(driver_program(&msvc, BackendKind::Rustc), "rustc");

        let linux = TargetProfile::parse("aarch64-unknown-linux-gnu").unwrap();
        assert_eq!(driver_program(&linux, BackendKind::System), "cc");
        assert_eq!(driver_program(&linux, BackendKind::Lld), "ld.lld");

        let darwin = TargetProfile::parse("aarch64-apple-darwin").unwrap();
        assert_eq!(driver_program(&darwin, BackendKind::Lld), "ld64.lld");
    }

    #[test]
    fn hints_match_the_platform() {
        let msvc = TargetProfile::parse("x86_64-pc-windows-msvc").unwrap();
        assert!(toolchain_hint(&msvc).contains("Windows SDK"));
        let darwin = TargetProfile::parse("aarch64-apple-darwin").unwrap();
        assert!(toolchain_hint(&darwin).contains("Xcode"));
    }

    #[test]
    fn selection_rejects_malformed_targets() {
        assert!(select("nonsense").is_err());
    }
}
