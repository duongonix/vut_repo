//! Backend-independent control-flow and ownership IR.
mod builder;
pub mod future;
mod interface;
mod ir;
mod layout;
mod lower;
mod monomorph;
#[cfg(test)]
mod tests;
mod uses;

pub use future::{AwaitLive, FRAME_CHILD_OFFSET, FRAME_STATE_OFFSET, FrameLayout};
pub use ir::*;
pub use layout::*;
pub use lower::lower;
