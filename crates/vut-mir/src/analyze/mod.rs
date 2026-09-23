//! Reusable MIR analyses shared by the optimizer passes and the verifier.
//!
//! Analyses are pure functions of a `Function` (or `Program`); they are rebuilt
//! after a mutation rather than incrementally invalidated. This keeps them
//! correct by construction; the pass manager bounds how often they are rebuilt.

pub mod callgraph;
pub mod cfg;
pub mod def_use;
pub mod devirt;
pub mod dominators;
pub mod escape;
pub mod liveness;
pub mod loops;
#[cfg(test)]
mod tests;

pub use callgraph::CallGraph;
pub use cfg::Cfg;
pub use def_use::DefUse;
pub use devirt::DevirtSummary;
pub use dominators::DominatorTree;
pub use liveness::Liveness;
pub use loops::LoopInfo;
