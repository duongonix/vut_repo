//! Dead instruction elimination.
//!
//! Uses the shared effect model ([`super::effects`]) so that observable
//! instructions (`Spawn`, `AwaitFuture`, drops, RC operations, calls, traps, …)
//! are never removed. Only pure instructions whose results are unused are
//! dropped.
use std::collections::HashSet;

use super::effects;
use crate::{Function, Program, ValueId};

use super::OptimizationReport;

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    for function in &mut program.functions {
        report.dead_instructions += eliminate(function);
    }
    report
}

fn eliminate(function: &mut Function) -> usize {
    // Seed liveness from terminators and observable instructions.
    let mut used: HashSet<ValueId> = HashSet::new();
    for block in &function.blocks {
        let mut terminator = Vec::new();
        effects::terminator_uses(&block.terminator, &mut terminator);
        used.extend(terminator);
        for instruction in &block.instructions {
            if effects::is_observable(instruction) {
                let mut operands = Vec::new();
                effects::operands(instruction, &mut operands);
                used.extend(operands);
            }
        }
    }
    // Propagate through pure instructions whose results are used.
    loop {
        let mut changed = false;
        for block in &function.blocks {
            for instruction in &block.instructions {
                let mut results = Vec::new();
                effects::defined_values(instruction, &mut results);
                if results.iter().any(|value| used.contains(value)) {
                    let mut operands = Vec::new();
                    effects::operands(instruction, &mut operands);
                    for operand in operands {
                        changed |= used.insert(operand);
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    // Remove dead pure instructions.
    let mut removed = 0;
    for block in &mut function.blocks {
        let before = block.instructions.len();
        block.instructions.retain(|instruction| {
            if effects::is_observable(instruction) {
                return true;
            }
            let mut results = Vec::new();
            effects::defined_values(instruction, &mut results);
            if results.is_empty() {
                // No result and not observable: keep to be safe.
                return true;
            }
            results.iter().any(|value| used.contains(value))
        });
        removed += before - block.instructions.len();
    }
    removed
}
