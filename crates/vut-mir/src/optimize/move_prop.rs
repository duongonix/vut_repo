//! Move propagation.
//!
//! A `Drop` of a local that was already moved out in the same block (and not
//! reassigned) is redundant: the value's owner has already transferred. Removing
//! it avoids a second destruction.
use super::OptimizationReport;
use crate::{Instruction, LocalId, Program};
use std::collections::HashSet;

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    for function in &mut program.functions {
        report.drops_elided += remove_drops_after_move(function);
    }
    report
}

fn remove_drops_after_move(function: &mut crate::Function) -> usize {
    let mut removed = 0;
    for block in &mut function.blocks {
        let mut moved: HashSet<LocalId> = HashSet::new();
        block.instructions.retain(|instruction| match instruction {
            Instruction::Move { local, .. } => {
                moved.insert(*local);
                true
            }
            Instruction::Store { local, .. } | Instruction::Copy { local, .. } => {
                moved.remove(local);
                true
            }
            Instruction::Drop(local) if moved.contains(local) => {
                removed += 1;
                false
            }
            _ => true,
        });
    }
    removed
}
