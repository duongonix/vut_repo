//! Sparse constant propagation and folding.
//!
//! Folds integer/boolean `Binary` operations whose operands are known constants,
//! applies value-preserving algebraic identities ([`numeric::simplify`]),
//! propagates constants through copyable scalar locals (`Store`/`Copy`/`Move`),
//! and folds `Branch` on a constant condition. Values are single-assignment, so
//! a constant definition is valid across the whole function.
use std::collections::HashMap;

use super::numeric::{self, Folded, Simplified};
use super::{OptimizationReport, effects};
use crate::{BinaryOp, Instruction, LayoutTable, LocalId, Program, Terminator, ValueId};

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    let Program {
        functions, layouts, ..
    } = program;
    for function in functions.iter_mut() {
        fold_function(function, layouts, &mut report);
    }
    report
}

fn fold_function(
    function: &mut crate::Function,
    layouts: &LayoutTable,
    report: &mut OptimizationReport,
) {
    loop {
        let (integers, booleans) = collect_constants(function);
        let mut changed = false;
        let mut aliases: Vec<(ValueId, ValueId)> = Vec::new();
        for block in &mut function.blocks {
            for instruction in &mut block.instructions {
                let Instruction::Binary {
                    operand_type,
                    value,
                    op,
                    left,
                    right,
                } = instruction
                else {
                    continue;
                };
                let output = *value;
                let (op, left, right, operand_type) = (*op, *left, *right, *operand_type);

                if left == right
                    && is_integer(operand_type, layouts)
                    && let Some(folded) = comparison_identity(op)
                {
                    *instruction = constant(output, folded);
                    report.constants_folded += 1;
                    changed = true;
                    continue;
                }

                let a = integers.get(&left).copied();
                let b = integers.get(&right).copied();
                if let (Some(a), Some(b)) = (a, b)
                    && let Some(folded) = numeric::fold(op, a, b, operand_type, layouts)
                {
                    *instruction = constant(output, folded);
                    report.constants_folded += 1;
                    report.constants_propagated += 1;
                    changed = true;
                    continue;
                }
                if let Some(simplified) = numeric::simplify(op, a, b, operand_type, layouts) {
                    match simplified {
                        Simplified::Left => aliases.push((output, left)),
                        Simplified::Right => aliases.push((output, right)),
                        Simplified::Constant(folded) => {
                            *instruction = constant(output, folded);
                            report.constants_folded += 1;
                            changed = true;
                        }
                    }
                }
            }
            if let Terminator::Branch {
                condition,
                then_block,
                else_block,
            } = block.terminator
                && let Some(value) = booleans.get(&condition).copied()
            {
                block.terminator = Terminator::Jump(if value { then_block } else { else_block });
                report.dead_branches += 1;
                changed = true;
            }
        }
        for (from, to) in aliases {
            if effects::rewrite_uses(function, from, to) {
                report.constants_propagated += 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn is_integer(operand_type: Option<vut_hir::TypeId>, layouts: &LayoutTable) -> bool {
    operand_type.is_some_and(|ty| layouts.types[ty.0].repr == crate::ValueRepr::Integer)
}

fn comparison_identity(op: BinaryOp) -> Option<Folded> {
    match op {
        BinaryOp::Equal | BinaryOp::LessEqual | BinaryOp::GreaterEqual => {
            Some(Folded::Boolean(true))
        }
        BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::Greater => Some(Folded::Boolean(false)),
        _ => None,
    }
}

fn constant(value: ValueId, folded: Folded) -> Instruction {
    match folded {
        Folded::Integer(literal) => Instruction::ConstInt { value, literal },
        Folded::Boolean(literal) => Instruction::ConstBool { value, literal },
    }
}

/// Collects constant SSA values, including constants stored into and read back
/// out of copyable scalar locals within a block.
fn collect_constants(
    function: &crate::Function,
) -> (HashMap<ValueId, i64>, HashMap<ValueId, bool>) {
    let mut integers: HashMap<ValueId, i64> = HashMap::new();
    let mut booleans: HashMap<ValueId, bool> = HashMap::new();
    for block in &function.blocks {
        let mut local_integer: HashMap<LocalId, i64> = HashMap::new();
        let mut local_boolean: HashMap<LocalId, bool> = HashMap::new();
        for instruction in &block.instructions {
            match instruction {
                Instruction::ConstInt { value, literal } => {
                    integers.insert(*value, *literal);
                }
                Instruction::ConstBool { value, literal } => {
                    booleans.insert(*value, *literal);
                }
                Instruction::Store { local, value } => {
                    local_integer.remove(local);
                    local_boolean.remove(local);
                    if let Some(constant) = integers.get(value) {
                        local_integer.insert(*local, *constant);
                    } else if let Some(constant) = booleans.get(value) {
                        local_boolean.insert(*local, *constant);
                    }
                }
                Instruction::Copy { value, local } | Instruction::Move { value, local } => {
                    if let Some(constant) = local_integer.get(local) {
                        integers.insert(*value, *constant);
                    } else if let Some(constant) = local_boolean.get(local) {
                        booleans.insert(*value, *constant);
                    }
                }
                Instruction::FieldStore { base, .. } => {
                    local_integer.remove(base);
                    local_boolean.remove(base);
                }
                _ => {}
            }
        }
    }
    (integers, booleans)
}
