//! Bounds-check elimination.
//!
//! Sound cases handled:
//!
//! - **Variadic access** (`VariadicAt`) with a constant index and a constant
//!   element count is marked in-bounds.
//! - **Array access** (`ArrayAt`) on an array with a statically known length (a
//!   `ConstructArray`) and a constant index is rewritten to the unchecked
//!   builtin.
//! - **List access** (`ListAt`) inside a loop whose condition is
//!   `counter < list.len()` is rewritten to the unchecked builtin. The fact is
//!   established on the taken branch and holds in every block that branch
//!   dominates. The loop counter and list receiver are read through locals, so
//!   values are canonicalized to the value they ultimately derive from.
//!
//! Accesses that are not provably in bounds keep their check and still trap.
use std::collections::HashMap;

use crate::analyze::{Cfg, DominatorTree};
use crate::{
    BinaryOp, BlockId, BuiltinFunction, Function, Instruction, LayoutTable, LocalId, Program,
    Terminator, ValueId,
};

use super::OptimizationReport;

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    let Program {
        functions, layouts, ..
    } = program;
    for function in functions.iter_mut() {
        report.bounds_checks_elided += eliminate(function, layouts);
    }
    report
}

#[expect(
    clippy::too_many_lines,
    reason = "the bounds-elimination cases share one canonicalization and dominator pass"
)]
fn eliminate(function: &mut Function, layouts: &LayoutTable) -> usize {
    let canonical = canonicalize(function);
    let integers = collect_integers(function);
    let array_lengths = collect_array_lengths(function, layouts);
    let list_len = collect_list_len(function, &canonical);
    let facts = branch_facts(function, &canonical);
    let cfg = Cfg::build(function);
    let dominators = DominatorTree::build(&cfg, function.entry);

    let mut elided = 0;
    for (block_index, block) in function.blocks.iter_mut().enumerate() {
        let block_id = BlockId(block_index);
        for instruction in &mut block.instructions {
            let Instruction::RuntimeCall {
                function: builtin,
                arguments,
                ..
            } = instruction
            else {
                continue;
            };
            match builtin {
                BuiltinFunction::ListAt => {
                    let (Some(list), Some(index)) = (arguments.first(), arguments.get(1)) else {
                        continue;
                    };
                    let list = canonical_value(&canonical, *list);
                    let index = canonical_value(&canonical, *index);
                    let safe = facts.iter().any(|(fact_index, fact_bound, block)| {
                        *fact_index == index
                            && list_len.get(fact_bound).copied() == Some(list)
                            && dominators.dominates(*block, block_id)
                    });
                    if safe {
                        *builtin = BuiltinFunction::ListAtUnchecked;
                        elided += 1;
                    }
                }
                BuiltinFunction::ArrayAt => {
                    let (Some(array), Some(index)) = (arguments.first(), arguments.get(1)) else {
                        continue;
                    };
                    let (Some(length), Some(index)) =
                        (array_lengths.get(array), integers.get(index))
                    else {
                        continue;
                    };
                    if *index >= 0 && usize::try_from(*index).is_ok_and(|index| index < *length) {
                        *builtin = BuiltinFunction::ArrayAtUnchecked;
                        elided += 1;
                    }
                }
                _ => {}
            }
        }
    }

    // Variadic in-bounds marking (constant index and constant length).
    for block in &mut function.blocks {
        for instruction in &mut block.instructions {
            let Instruction::VariadicAt {
                index,
                len,
                in_bounds,
                ..
            } = instruction
            else {
                continue;
            };
            if *in_bounds {
                continue;
            }
            let (Some(index), Some(len)) = (integers.get(index), integers.get(len)) else {
                continue;
            };
            if *index >= 0 && index < len {
                *in_bounds = true;
                elided += 1;
            }
        }
    }

    // Variadic in-bounds from a dominating `counter < len` loop fact.
    for (block_index, block) in function.blocks.iter_mut().enumerate() {
        let block_id = BlockId(block_index);
        for instruction in &mut block.instructions {
            let Instruction::VariadicAt {
                index,
                len,
                in_bounds,
                ..
            } = instruction
            else {
                continue;
            };
            if *in_bounds {
                continue;
            }
            let index = canonical_value(&canonical, *index);
            let len = canonical_value(&canonical, *len);
            let safe = facts.iter().any(|(counter, bound, block)| {
                *counter == index && *bound == len && dominators.dominates(*block, block_id)
            });
            if safe {
                *in_bounds = true;
                elided += 1;
            }
        }
    }
    elided
}

/// Maps each value read through a local to the value that local holds, so two
/// reads of the same local canonicalize to the same representative.
fn canonicalize(function: &Function) -> HashMap<ValueId, ValueId> {
    let mut local_value: HashMap<LocalId, ValueId> = HashMap::new();
    let mut canonical: HashMap<ValueId, ValueId> = HashMap::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            match instruction {
                Instruction::Store { local, value } => {
                    let resolved = canonical_value(&canonical, *value);
                    local_value.insert(*local, resolved);
                }
                Instruction::Copy { value, local }
                | Instruction::Move { value, local }
                | Instruction::Borrow { value, local } => {
                    let resolved = local_value
                        .get(local)
                        .map_or(*value, |source| canonical_value(&canonical, *source));
                    canonical.insert(*value, resolved);
                }
                _ => {}
            }
        }
    }
    canonical
}

fn canonical_value(canonical: &HashMap<ValueId, ValueId>, value: ValueId) -> ValueId {
    let mut current = value;
    let mut guard = 0_u32;
    while let Some(next) = canonical.get(&current).copied() {
        if next == current || guard > 4096 {
            break;
        }
        current = next;
        guard += 1;
    }
    current
}

fn collect_integers(function: &Function) -> HashMap<ValueId, i64> {
    let mut integers = HashMap::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            if let Instruction::ConstInt { value, literal } = instruction {
                integers.insert(*value, *literal);
            }
        }
    }
    integers
}

fn collect_array_lengths(function: &Function, layouts: &LayoutTable) -> HashMap<ValueId, usize> {
    let mut arrays = HashMap::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            if let Instruction::ConstructArray { value, ty, .. } = instruction
                && let Some((_, length)) = layouts.arrays.get(ty)
            {
                arrays.insert(*value, *length);
            }
        }
    }
    arrays
}

fn collect_list_len(
    function: &Function,
    canonical: &HashMap<ValueId, ValueId>,
) -> HashMap<ValueId, ValueId> {
    let mut lengths = HashMap::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            if let Instruction::RuntimeCall {
                value: Some(value),
                function: BuiltinFunction::ListLen,
                arguments,
                ..
            } = instruction
                && let Some(list) = arguments.first()
            {
                lengths.insert(*value, canonical_value(canonical, *list));
            }
        }
    }
    lengths
}

/// Facts `counter < bound` established on a taken branch, canonicalized.
///
/// Both `counter < bound` and its reverse `bound > counter` are recognized.
fn branch_facts(
    function: &Function,
    canonical: &HashMap<ValueId, ValueId>,
) -> Vec<(ValueId, ValueId, BlockId)> {
    let mut facts = Vec::new();
    for block in &function.blocks {
        let Terminator::Branch {
            condition,
            then_block,
            ..
        } = &block.terminator
        else {
            continue;
        };
        let Some(Instruction::Binary { op, left, right, .. }) = block
            .instructions
            .iter()
            .find(|instruction| matches!(instruction, Instruction::Binary { value, .. } if value == condition))
        else {
            continue;
        };
        let (counter, bound) = match op {
            BinaryOp::Less => (*left, *right),
            BinaryOp::Greater => (*right, *left),
            _ => continue,
        };
        facts.push((
            canonical_value(canonical, counter),
            canonical_value(canonical, bound),
            *then_block,
        ));
    }
    facts
}
