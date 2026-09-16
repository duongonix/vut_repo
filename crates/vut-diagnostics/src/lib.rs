//! Shared structured diagnostics facade.
mod diagnostic;
mod render;
pub use diagnostic::*;
pub use render::{ColorChoice, Renderer};
