//! Structural-interface semantic model.
mod cache;
mod shape;
pub use cache::SatisfactionCache;
pub use shape::{InterfaceMethod, InterfaceShape, Satisfaction, merge_requirement};
