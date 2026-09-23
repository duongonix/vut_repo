//! Dead store elimination for copyable scalar locals.
//!
//! Within a block, a `Store` to a copyable scalar local that is overwritten by a
//! later `Store` before any read is dead. Restricted to copyable, non-managed
//! locals so removing a store can never leak or drop an owned value.
use std::collections::{HashMap, HashSet};

use super::OptimizationReport;
use crate::{Function, Instruction, LayoutTable, LocalId, Program};

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    let Program {
        functions, layouts, ..
    } = program;
    for function in functions.iter_mut() {
        report.dead_stores_removed += eliminate(function, layouts);
    }
    report
}

fn is_copy_scalar(function: &Function, layouts: &LayoutTable, local: LocalId) -> bool {
    function
        .locals
        .get(local.0)
        .and_then(|local| local.ty)
        .is_some_and(|ty| {
            let info = layouts.types[ty.0];
            info.is_copy && !info.needs_drop
        })
}

fn eliminate(function: &mut Function, layouts: &LayoutTable) -> usize {
    let copy_scalars: HashSet<LocalId> = (0..function.locals.len())
        .map(LocalId)
        .filter(|local| is_copy_scalar(function, layouts, *local))
        .collect();
    let mut removed = 0;
    for block in &mut function.blocks {
        let mut last_store: HashMap<LocalId, usize> = HashMap::new();
        let mut dead: HashSet<usize> = HashSet::new();
        for (index, instruction) in block.instructions.iter().enumerate() {
            match instruction {
                Instruction::Store { local, .. } if copy_scalars.contains(local) => {
                    if let Some(previous) = last_store.insert(*local, index) {
                        dead.insert(previous);
                    }
                }
                Instruction::Store { local, .. } => {
                    last_store.remove(local);
                }
                Instruction::Copy { local, .. }
                | Instruction::Borrow { local, .. }
                | Instruction::Move { local, .. }
                | Instruction::Drop(local)
                | Instruction::FieldStore { base: local, .. }
                | Instruction::BorrowField { base: local, .. } => {
                    // A read makes the previous store live.
                    last_store.remove(local);
                }
                _ => {}
            }
        }
        if dead.is_empty() {
            continue;
        }
        let mut index = 0_usize;
        block.instructions.retain(|_| {
            let keep = !dead.contains(&index);
            index += 1;
            keep
        });
        removed += dead.len();
    }
    removed
}
