use crate::{BinaryOp, Instruction, Program, Terminator, ValueId};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OptimizationReport {
    pub constants_folded: usize,
    pub dead_instructions: usize,
    pub dead_branches: usize,
}

pub fn optimize(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    for function in &mut program.functions {
        for block in &mut function.blocks {
            let mut integers = HashMap::new();
            let mut booleans = HashMap::new();
            for instruction in &mut block.instructions {
                match instruction {
                    Instruction::ConstInt { value, literal } => {
                        integers.insert(*value, *literal);
                    }
                    Instruction::ConstBool { value, literal } => {
                        booleans.insert(*value, *literal);
                    }
                    Instruction::Binary {
                        value,
                        op,
                        left,
                        right,
                    } => {
                        let output = *value;
                        if let (Some(a), Some(b)) =
                            (integers.get(left).copied(), integers.get(right).copied())
                        {
                            if let Some(folded) = fold_int(*op, a, b) {
                                *instruction = Instruction::ConstInt {
                                    value: output,
                                    literal: folded,
                                };
                                integers.insert(output, folded);
                                report.constants_folded += 1;
                            } else if let Some(folded) = fold_bool(*op, a, b) {
                                *instruction = Instruction::ConstBool {
                                    value: output,
                                    literal: folded,
                                };
                                booleans.insert(output, folded);
                                report.constants_folded += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
            if let Terminator::Branch {
                condition,
                then_block,
                else_block,
            } = block.terminator
                && let Some(value) = booleans.get(&condition)
            {
                block.terminator = Terminator::Jump(if *value { then_block } else { else_block });
                report.dead_branches += 1;
            }
            report.dead_instructions += eliminate_dead(&mut block.instructions, &block.terminator);
        }
    }
    report
}
fn fold_int(op: BinaryOp, a: i64, b: i64) -> Option<i64> {
    match op {
        BinaryOp::Add => a.checked_add(b),
        BinaryOp::Subtract => a.checked_sub(b),
        BinaryOp::Multiply => a.checked_mul(b),
        BinaryOp::Divide if b != 0 => a.checked_div(b),
        BinaryOp::Modulo if b != 0 => a.checked_rem(b),
        _ => None,
    }
}
fn fold_bool(op: BinaryOp, a: i64, b: i64) -> Option<bool> {
    Some(match op {
        BinaryOp::Equal => a == b,
        BinaryOp::NotEqual => a != b,
        BinaryOp::Less => a < b,
        BinaryOp::LessEqual => a <= b,
        BinaryOp::Greater => a > b,
        BinaryOp::GreaterEqual => a >= b,
        _ => return None,
    })
}
fn eliminate_dead(instructions: &mut Vec<Instruction>, terminator: &Terminator) -> usize {
    let mut used = HashSet::new();
    match terminator {
        Terminator::Return(Some(value)) => {
            used.insert(*value);
        }
        Terminator::Branch { condition, .. } => {
            used.insert(*condition);
        }
        _ => {}
    }
    let before = instructions.len();
    let mut kept = Vec::with_capacity(before);
    for instruction in instructions.drain(..).rev() {
        let keep = if side_effecting(&instruction) {
            true
        } else {
            result(&instruction).is_none_or(|value| used.contains(&value))
        };
        if keep {
            for value in operands(&instruction) {
                used.insert(value);
            }
            kept.push(instruction);
        }
    }
    kept.reverse();
    *instructions = kept;
    before - instructions.len()
}
fn side_effecting(value: &Instruction) -> bool {
    matches!(
        value,
        Instruction::Store { .. }
            | Instruction::Drop(_)
            | Instruction::Allocate { .. }
            | Instruction::Retain { .. }
            | Instruction::Release { .. }
            | Instruction::Call { .. }
            | Instruction::RuntimeCall { .. }
            | Instruction::IteratorInit { .. }
            | Instruction::IteratorNext { .. }
            | Instruction::Construct { .. }
            | Instruction::ConstructResult { .. }
    )
}
fn result(value: &Instruction) -> Option<ValueId> {
    match value {
        Instruction::ConstNull { value }
        | Instruction::ConstInt { value, .. }
        | Instruction::ConstFloat { value, .. }
        | Instruction::ConstBool { value, .. }
        | Instruction::ConstString { value, .. }
        | Instruction::FormatValue { value, .. }
        | Instruction::ConcatString { value, .. }
        | Instruction::Copy { value, .. }
        | Instruction::Move { value, .. }
        | Instruction::Allocate { value, .. }
        | Instruction::Binary { value, .. }
        | Instruction::Unary { value, .. }
        | Instruction::Field { value, .. }
        | Instruction::Construct { value, .. }
        | Instruction::ConstructResult { value, .. }
        | Instruction::ConstructEnum { value, .. }
        | Instruction::EnumTag { result: value, .. }
        | Instruction::EnumPayload { value, .. }
        | Instruction::TypeRetain { value, .. }
        | Instruction::TypeRelease { value, .. }
        | Instruction::ResultState { result: value, .. }
        | Instruction::ResultPayload { value, .. }
        | Instruction::ConstructVariadicBuffer { value, .. }
        | Instruction::VariadicAt { value, .. } => Some(*value),
        Instruction::Call { value, .. } | Instruction::RuntimeCall { value, .. } => *value,
        Instruction::IteratorNext { has_value, .. } => Some(*has_value),
        _ => None,
    }
}
fn operands(value: &Instruction) -> Vec<ValueId> {
    match value {
        Instruction::Store { value, .. }
        | Instruction::Retain { value, .. }
        | Instruction::Release { value, .. }
        | Instruction::EnumTag { value, .. }
        | Instruction::ResultState { value, .. } => vec![*value],
        Instruction::ResultPayload { source, .. } | Instruction::EnumPayload { source, .. } => {
            vec![*source]
        }
        Instruction::ConstructResult { payload, .. } => vec![*payload],
        Instruction::ConstructEnum { payload, .. } => payload.clone(),
        Instruction::Binary { left, right, .. } | Instruction::ConcatString { left, right, .. } => {
            vec![*left, *right]
        }
        Instruction::Unary { operand, .. } | Instruction::FormatValue { operand, .. } => {
            vec![*operand]
        }
        Instruction::Call { arguments, .. } | Instruction::RuntimeCall { arguments, .. } => {
            arguments.clone()
        }
        Instruction::Field { base, .. } => vec![*base],
        Instruction::Construct { fields, .. } => fields.iter().map(|(_, v)| *v).collect(),
        Instruction::IteratorInit {
            iterable,
            length_value,
            ..
        } => {
            let mut values = vec![*iterable];
            values.extend(length_value.iter().copied());
            values
        }
        Instruction::IteratorNext { iterator, .. } => vec![*iterator],
        Instruction::ConstructVariadicBuffer { elements, .. } => elements.clone(),
        Instruction::VariadicAt {
            data, len, index, ..
        } => vec![*data, *len, *index],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BasicBlock, BlockId, Function, LayoutTable};
    use vut_resolver::SymbolId;
    #[test]
    fn folds_constants_removes_dead_values_and_branches() {
        let mut program = Program {
            external_functions: Vec::new(),
            interface_vtables: Vec::new(),
            functions: vec![Function {
                symbol: SymbolId(0),
                receiver: None,
                parameter_count: 0,
                locals: vec![],
                blocks: vec![
                    BasicBlock {
                        instructions: vec![
                            Instruction::ConstInt {
                                value: ValueId(0),
                                literal: 2,
                            },
                            Instruction::ConstInt {
                                value: ValueId(1),
                                literal: 3,
                            },
                            Instruction::Binary {
                                value: ValueId(2),
                                op: BinaryOp::Add,
                                left: ValueId(0),
                                right: ValueId(1),
                            },
                            Instruction::ConstBool {
                                value: ValueId(3),
                                literal: true,
                            },
                        ],
                        terminator: Terminator::Branch {
                            condition: ValueId(3),
                            then_block: BlockId(1),
                            else_block: BlockId(2),
                        },
                    },
                    BasicBlock {
                        instructions: vec![],
                        terminator: Terminator::Return(None),
                    },
                    BasicBlock {
                        instructions: vec![],
                        terminator: Terminator::Return(None),
                    },
                ],
                entry: BlockId(0),
                return_type: None,
                is_async: false,
                frame_param: None,
                is_poll: false,
                out_param: None,
            }],
            layouts: LayoutTable {
                types: vec![],
                pointer_size: 8,
                fields: HashMap::new(),
                arrays: HashMap::new(),
                lists: HashMap::new(),
                maps: HashMap::new(),
                results: HashMap::new(),
                enums: HashMap::new(),
                callables: HashMap::new(),
                aggregates: std::collections::HashSet::new(),
            },
            diagnostics: vut_diagnostics::DiagnosticSink::new(),
            frames: HashMap::new(),
            awaits: HashMap::new(),
        };
        let report = optimize(&mut program);
        assert_eq!(report.constants_folded, 1);
        assert_eq!(report.dead_branches, 1);
        assert!(report.dead_instructions > 0);
    }
}
