//! MIR optimization pipeline.
//!
//! The pipeline is a sequence of small, independently tested passes over the
//! ownership-aware MIR. Every pass preserves language semantics: ownership,
//! deterministic `Drop`, traps, side effects, and async suspension. Observable
//! instructions are never removed (see [`effects`]).
//!
//! A verifier runs before and after the batch; if the optimized program fails to
//! verify, the optimization is discarded and the input MIR is kept. This makes
//! the optimizer fail-safe: a buggy pass degrades to no optimization rather than
//! miscompiling.
mod async_opt;
mod bounds;
mod branch;
mod closure;
mod collections;
mod copy_elision;
mod copy_prop;
mod cse;
mod dce;
mod drop;
pub(crate) mod effects;
mod inline;
mod loops;
mod manager;
mod memory;
mod move_prop;
mod numeric;
mod rc;
mod sccp;
mod sroa;
mod strings;
#[cfg(test)]
mod tests;
mod verify;

use crate::Program;

pub use crate::analyze::escape::{EscapeSummary, analyze as escape_analysis};
pub use verify::{VerificationError, verify};

/// Optimization level for a build.
///
/// `O0` runs no passes and is the reference/correctness path used by
/// differential testing. `O2` is the default release level (`vut build
/// --release`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum OptimizationLevel {
    #[default]
    O0,
    O1,
    O2,
    O3,
}

impl OptimizationLevel {
    #[must_use]
    pub const fn is_optimizing(self) -> bool {
        !matches!(self, Self::O0)
    }
}

/// What the optimizer changed, per category.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OptimizationReport {
    pub constants_folded: usize,
    pub constants_propagated: usize,
    pub dead_instructions: usize,
    pub dead_branches: usize,
    pub unreachable_blocks: usize,
    pub copies_propagated: usize,
    pub retains_elided: usize,
    pub releases_elided: usize,
    pub drops_elided: usize,
    pub make_unique_elided: usize,
    pub copies_elided: usize,
    pub allocations_elided: usize,
    pub bounds_checks_elided: usize,
    pub async_reloads_elided: usize,
    pub cse_eliminated: usize,
    pub dead_stores_removed: usize,
    pub instructions_hoisted: usize,
    pub aggregates_scalarized: usize,
    pub strings_folded: usize,
    pub call_sites_inlined: usize,
    pub call_indirect_devirtualized: usize,
    pub async_frame_states_elided: usize,
    pub async_spills_elided: usize,
    /// Phases whose output failed verification and were reverted to their input.
    pub reverted_phases: usize,
}

impl OptimizationReport {
    fn merge(&mut self, other: Self) {
        self.constants_folded += other.constants_folded;
        self.constants_propagated += other.constants_propagated;
        self.dead_instructions += other.dead_instructions;
        self.dead_branches += other.dead_branches;
        self.unreachable_blocks += other.unreachable_blocks;
        self.copies_propagated += other.copies_propagated;
        self.retains_elided += other.retains_elided;
        self.releases_elided += other.releases_elided;
        self.drops_elided += other.drops_elided;
        self.make_unique_elided += other.make_unique_elided;
        self.copies_elided += other.copies_elided;
        self.allocations_elided += other.allocations_elided;
        self.bounds_checks_elided += other.bounds_checks_elided;
        self.async_reloads_elided += other.async_reloads_elided;
        self.cse_eliminated += other.cse_eliminated;
        self.dead_stores_removed += other.dead_stores_removed;
        self.instructions_hoisted += other.instructions_hoisted;
        self.aggregates_scalarized += other.aggregates_scalarized;
        self.strings_folded += other.strings_folded;
        self.call_sites_inlined += other.call_sites_inlined;
        self.call_indirect_devirtualized += other.call_indirect_devirtualized;
        self.async_frame_states_elided += other.async_frame_states_elided;
        self.async_spills_elided += other.async_spills_elided;
        self.reverted_phases += other.reverted_phases;
    }
}

/// Optimizes `program` at `level`, returning what changed.
///
/// The input program is restored unchanged when the optimized program fails
/// verification.
pub fn optimize(program: &mut Program, level: OptimizationLevel) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    if !level.is_optimizing() {
        return report;
    }
    let backup = program.clone();
    run_passes(program, level, &mut report);
    if verify(program).is_err() {
        *program = backup;
        return OptimizationReport::default();
    }
    report
}

fn run_passes(program: &mut Program, level: OptimizationLevel, report: &mut OptimizationReport) {
    manager::PassManager::for_level(level).run(program, report);
}
