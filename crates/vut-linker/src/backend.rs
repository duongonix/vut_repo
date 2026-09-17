//! Linker backend selection.
//!
//! A backend turns a [`LinkPlan`](crate::LinkPlan) into a native executable.
//! The default remains the `rustc` backend during migration; the standalone
//! system/lld backends can be selected explicitly while M-LINK.3-5 enable and
//! verify them per target. Production release linking must not depend on
//! `rustc` once the migration completes.
pub mod rustc;
pub mod system;

use crate::error::LinkError;
use crate::plan::LinkPlan;
use crate::target::TargetProfile;

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

/// Selects the backend for a target.
///
/// `VUT_LINKER` (`rustc`, `system`/`cc`/`cl`, `lld`) is a development override.
/// The default is [`BackendKind::Rustc`] until M-LINK.6 flips production to the
/// standalone backends.
///
/// # Errors
/// Returns [`LinkError`] when the target triple cannot be parsed.
pub fn select(target: &str) -> Result<Box<dyn LinkerBackend>, LinkError> {
    let profile = TargetProfile::parse(target)?;
    let kind = requested_kind();
    Ok(match kind {
        BackendKind::Rustc => Box::new(rustc::RustcBackend),
        BackendKind::System | BackendKind::Lld => {
            Box::new(system::SystemBackend::new(kind, profile))
        }
    })
}

/// Reads the `VUT_LINKER` development override.
#[must_use]
pub fn requested_kind() -> BackendKind {
    match std::env::var("VUT_LINKER")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "system" | "cc" | "cl" => BackendKind::System,
        "lld" | "lld-link" => BackendKind::Lld,
        _ => BackendKind::Rustc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_parses_the_target_and_defaults_to_rustc() {
        // `requested_kind` reads the environment; without VUT_LINKER set the
        // default is rustc.
        if std::env::var_os("VUT_LINKER").is_none() {
            assert_eq!(requested_kind(), BackendKind::Rustc);
        }
        assert!(select("x86_64-unknown-linux-gnu").is_ok());
    }

    #[test]
    fn selection_rejects_malformed_targets() {
        assert!(select("nonsense").is_err());
    }
}
