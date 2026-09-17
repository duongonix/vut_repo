//! Platform native-linker boundary.
//!
//! Turns a [`LinkPlan`] into a native executable without requiring `rustc` in
//! production. Backends are selected in [`backend`]; the measured target
//! profiles and system libraries live in [`target`] and [`system_libs`].
mod backend;
mod error;
mod plan;
mod process;
mod startup;
mod system_libs;
mod target;

pub use backend::{BackendKind, LinkerBackend};
pub use error::{LinkError, LinkFailure};
pub use plan::LinkPlan;
pub use startup::StartupObject;
pub use target::{Arch, Flavor, Kernel, TargetProfile};

/// Links a native executable.
pub trait NativeLinker {
    /// Links a compiler object with any native libraries through the selected
    /// backend.
    ///
    /// # Errors
    /// Returns tool discovery, classification, and linker diagnostics.
    fn link(&self, plan: &LinkPlan) -> Result<(), LinkError>;
}

/// Facade that links through the backend selected for `plan.target`.
pub struct SystemLinker;

impl NativeLinker for SystemLinker {
    fn link(&self, plan: &LinkPlan) -> Result<(), LinkError> {
        backend::select(&plan.target)?.link(plan)
    }
}
