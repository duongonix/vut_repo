//! Canonical Vut installation layout and artifact discovery.
//!
//! One implementation shared by the compiler, CLI, VPM and packaging tooling so
//! `VUT_HOME` resolution cannot drift between components.
mod discovery;
mod home;

pub use discovery::{
    RUNTIME_DIR_ENV, RUNTIME_LIBRARY_ENV, STARTUP_OBJECT_ENV, STDLIB_PATH_ENV, STDLIB_RUNTIME_ENV,
    is_windows_target, runtime_library, startup_object, stdlib_root, stdlib_runtime_library,
};
pub use home::{HOME_ENV, Home};
