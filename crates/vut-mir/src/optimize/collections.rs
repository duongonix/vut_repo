//! Collection optimization.
//!
//! A `MakeUnique` whose operand is a freshly constructed collection with no
//! other use is a no-op: the collection is already uniquely referenced, so the
//! runtime detach would return it unchanged. The detach is elided and the
//! operand aliased directly.
use std::collections::HashSet;

use super::effects;
use crate::{BuiltinFunction, Instruction, Program, ValueId};

use super::OptimizationReport;

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    for function in &mut program.functions {
        report.make_unique_elided += elide_fresh_make_unique(function);
    }
    report
}

fn is_fresh_constructor(function: BuiltinFunction) -> bool {
    matches!(
        function,
        BuiltinFunction::ListNew | BuiltinFunction::MapNew | BuiltinFunction::BytesNew
    )
}

fn elide_fresh_make_unique(function: &mut crate::Function) -> usize {
    let mut fresh: HashSet<ValueId> = HashSet::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            match instruction {
                Instruction::RuntimeCall {
                    value: Some(value),
                    function: builtin,
                    ..
                } if is_fresh_constructor(*builtin) => {
                    fresh.insert(*value);
                }
                Instruction::ConstructArray { value, .. } => {
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
            let Instruction::MakeUnique { value, operand, .. } = instruction else {
                continue;
            };
            if fresh.contains(operand) && counts.get(operand).copied().unwrap_or(0) == 1 {
                elided.insert(*value);
                replacements.push((*value, *operand));
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
            if let Instruction::MakeUnique { value, .. } = instruction
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
