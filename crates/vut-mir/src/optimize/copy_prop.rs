//! Copy propagation for cheap (copyable) locals.
//!
//! When a local is assigned by `Copy`/`Move` and read again by a later `Copy`
//! with no intervening store, the later read yields the same value. Forwarding
//! it lets DCE remove the redundant copy. Restricted to copyable locals so
//! aggregate copy semantics are never changed.
use std::collections::HashMap;

use super::effects;
use crate::{Instruction, LayoutTable, LocalId, Program, ValueId};

use super::OptimizationReport;

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    let Program {
        functions, layouts, ..
    } = program;
    for function in functions.iter_mut() {
        report.copies_propagated += propagate(function, layouts);
    }
    report
}

fn is_copy_local(function: &crate::Function, layouts: &LayoutTable, local: LocalId) -> bool {
    function
        .locals
        .get(local.0)
        .and_then(|local| local.ty)
        .is_some_and(|ty| layouts.types[ty.0].is_copy)
}

fn propagate(function: &mut crate::Function, layouts: &LayoutTable) -> usize {
    let mut replacements: Vec<(ValueId, ValueId)> = Vec::new();
    for block in &function.blocks {
        let mut current: HashMap<LocalId, ValueId> = HashMap::new();
        for instruction in &block.instructions {
            match instruction {
                Instruction::Copy { value, local } | Instruction::Move { value, local } => {
                    if !is_copy_local(function, layouts, *local) {
                        continue;
                    }
                    match current.get(local).copied() {
                        Some(source) => replacements.push((*value, source)),
                        None => {
                            current.insert(*local, *value);
                        }
                    }
                }
                Instruction::Store { local, value } => {
                    // Store-to-load forwarding: a later read of `local` yields
                    // the stored value.
                    if is_copy_local(function, layouts, *local) {
                        current.insert(*local, *value);
                    } else {
                        current.remove(local);
                    }
                }
                Instruction::FieldStore { base: local, .. } | Instruction::Drop(local) => {
                    current.remove(local);
                }
                _ => {}
            }
        }
    }
    let mut changed = 0;
    for (from, to) in replacements {
        if effects::rewrite_uses(function, from, to) {
            changed += 1;
        }
    }
    changed
}
