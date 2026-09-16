//! Versioned incremental fingerprints, dependency invalidation, and artifact storage.
mod cache;
mod graph;
mod key;

pub use cache::{ArtifactCache, CacheError};
pub use graph::DependencyGraph;
pub use key::{CacheKey, CacheKeyBuilder, fingerprint_roots};

/// Increment when an incompatible persisted representation changes.
pub const CACHE_SCHEMA: u32 = 1;
