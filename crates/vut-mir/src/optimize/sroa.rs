//! Scalar replacement of aggregates and allocation cleanup.
//!
//! - Removes `Allocate` instructions whose result is never used (a dead heap
//!   allocation would otherwise leak).
//! - Scalar-replaces a `Construct` aggregate that does not escape and is only
//!   read through `Field`: each `Field` result is aliased to the corresponding
//!   constructor operand and the aggregate is removed. Restricted to copyable,
//!   non-managed aggregates so no ownership is changed.
//!
//! Stack promotion of remaining `Allocate` results is deferred (it needs a place
//! abstraction in the backend).
use super::OptimizationReport;
use super::effects;
use crate::{Instruction, LayoutTable, Program, ValueId};

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    let Program {
        functions, layouts, ..
    } = program;
    for function in functions.iter_mut() {
        report.allocations_elided += remove_dead_allocations(function);
        report.aggregates_scalarized += scalarize(function, layouts);
    }
    report
}

fn remove_dead_allocations(function: &mut crate::Function) -> usize {
    let counts = effects::use_counts(function);
    let mut removed = 0;
    for block in &mut function.blocks {
        block.instructions.retain(|instruction| {
            if let Instruction::Allocate { value, .. } = instruction
                && counts.get(value).copied().unwrap_or(0) == 0
            {
                removed += 1;
                return false;
            }
            true
        });
    }
    removed
}

fn scalarize(function: &mut crate::Function, layouts: &LayoutTable) -> usize {
    let mut aliases: Vec<(ValueId, ValueId)> = Vec::new();
    let mut scalarized: Vec<ValueId> = Vec::new();

    for block in &function.blocks {
        for instruction in &block.instructions {
            let Instruction::Construct { value, ty, fields } = instruction else {
                continue;
            };
            let info = layouts.types[ty.0];
            if !info.is_copy || info.needs_drop || info.contains_managed {
                continue;
            }
            let mut pending: Vec<(ValueId, ValueId)> = Vec::new();
            let mut only_fields = true;
            for block in &function.blocks {
                for instruction in &block.instructions {
                    if let Instruction::Field {
                        value: field_value,
                        base,
                        name,
                    } = instruction
                        && base == value
                    {
                        if let Some((_, operand)) = fields.iter().find(|(field, _)| field == name) {
                            pending.push((*field_value, *operand));
                        } else {
                            only_fields = false;
                        }
                    }
                }
            }
            // Ensure no use of the aggregate is anything other than a `Field`.
            let uses = effects::use_counts(function)
                .get(value)
                .copied()
                .unwrap_or(0);
            let field_uses = pending.len();
            if only_fields && field_uses > 0 && uses == field_uses {
                aliases.extend(pending);
                scalarized.push(*value);
            }
        }
    }
    if scalarized.is_empty() {
        return 0;
    }
    for (from, to) in aliases {
        effects::rewrite_uses(function, from, to);
    }
    for block in &mut function.blocks {
        block.instructions.retain(|instruction| match instruction {
            Instruction::Construct { value, .. } if scalarized.contains(value) => false,
            Instruction::Field { base, .. } if scalarized.contains(base) => false,
            _ => true,
        });
    }
    scalarized.len()
}
