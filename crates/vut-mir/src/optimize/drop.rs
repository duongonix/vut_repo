//! Drop optimization.
//!
//! Removes redundant `Drop`s: a second `Drop` of a local with no intervening
//! store destroys a value that was already destroyed. Deterministic destruction
//! is preserved — only provably redundant drops are removed.
use std::collections::HashSet;

use super::OptimizationReport;
use crate::{Instruction, LocalId, Program};

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    for function in &mut program.functions {
        report.drops_elided += remove_duplicate_drops(function);
    }
    report
}

fn remove_duplicate_drops(function: &mut crate::Function) -> usize {
    let mut removed = 0;
    for block in &mut function.blocks {
        let mut dropped: HashSet<LocalId> = HashSet::new();
        block.instructions.retain(|instruction| match instruction {
            Instruction::Store { local, .. }
            | Instruction::Copy { local, .. }
            | Instruction::Move { local, .. } => {
                dropped.remove(local);
                true
            }
            Instruction::Drop(local) if !dropped.insert(*local) => {
                removed += 1;
                false
            }
            _ => true,
        });
    }
    removed
}
