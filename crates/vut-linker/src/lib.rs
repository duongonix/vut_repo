//! Platform native-linker boundary.
mod linker;
pub use linker::{LinkError, LinkPlan, NativeLinker, SystemLinker};
