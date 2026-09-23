//! Copy elision.
//!
//! A `CopyAggregate` whose source is a freshly constructed aggregate with no
//! other use copies storage that is already uniquely owned. The copy is elided
//! and the result aliases the source directly.
use std::collections::HashSet;

use super::effects;
use crate::{Instruction, Program, ValueId};

use super::OptimizationReport;

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    for function in &mut program.functions {
        report.copies_elided += elide_fresh_copy_aggregate(function);
    }
    report
}

fn elide_fresh_copy_aggregate(function: &mut crate::Function) -> usize {
    let mut fresh: HashSet<ValueId> = HashSet::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            match instruction {
                Instruction::Construct { value, .. }
                | Instruction::ConstructArray { value, .. }
                | Instruction::ConstructEnum { value, .. } => {
                    fresh.insert(*value);
                }
                _ => {}
            }
        }
    }
    if fresh.is_empty() {
        return 0;
    }
    let counts = effects::use_counts(function);
    let mut elided: HashSet<ValueId> = HashSet::new();
    let mut replacements: Vec<(ValueId, ValueId)> = Vec::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            let Instruction::CopyAggregate { value, source, .. } = instruction else {
                continue;
            };
            if fresh.contains(source) && counts.get(source).copied().unwrap_or(0) == 1 {
                elided.insert(*value);
                replacements.push((*value, *source));
            }
        }
    }
    if elided.is_empty() {
        return 0;
    }
    for (from, to) in replacements {
        effects::rewrite_uses(function, from, to);
    }
    let mut removed = 0;
    for block in &mut function.blocks {
        block.instructions.retain(|instruction| {
            if let Instruction::CopyAggregate { value, .. } = instruction
                && elided.contains(value)
            {
                removed += 1;
                return false;
            }
            true
        });
    }
    removed
}
