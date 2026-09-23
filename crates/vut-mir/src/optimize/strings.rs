//! String optimization: compile-time constant concatenation.
//!
//! `"a" + "b"` is folded into the single literal `"ab"`, removing a runtime
//! allocation and copy. Only literal `ConstString` operands are folded, so the
//! result is byte-identical to the runtime concatenation.
use std::collections::HashMap;

use super::OptimizationReport;
use crate::{Instruction, Program, ValueId};

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    for function in &mut program.functions {
        report.strings_folded += fold(function);
    }
    report
}

fn fold(function: &mut crate::Function) -> usize {
    let mut folded = 0;
    loop {
        let mut constants: HashMap<ValueId, String> = HashMap::new();
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let Instruction::ConstString { value, literal } = instruction {
                    constants.insert(*value, literal.clone());
                }
            }
        }
        let mut changed = false;
        for block in &mut function.blocks {
            for instruction in &mut block.instructions {
                let (value, left, right) = match instruction {
                    Instruction::ConcatString { value, left, right } => (*value, *left, *right),
                    _ => continue,
                };
                let (Some(a), Some(b)) = (constants.get(&left), constants.get(&right)) else {
                    continue;
                };
                let literal = format!("{a}{b}");
                *instruction = Instruction::ConstString {
                    value,
                    literal: literal.clone(),
                };
                constants.insert(value, literal);
                folded += 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    folded
}
