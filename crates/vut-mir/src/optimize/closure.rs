//! Closure / function-value optimization.
//!
//! A `CallIndirect` whose callee is provably a non-capturing function value
//! (`MakeFunction`, possibly copied through a local) can be turned into a direct
//! `Call`: a non-capturing function value is just the code pointer, so the call
//! has no environment argument and the target is known statically. Capturing
//! closures (`MakeClosure`) carry a tagged environment and are never rewritten.
use std::collections::HashMap;

use super::OptimizationReport;
use crate::{Function, Instruction, LocalId, Program, SymbolId, ValueId};

pub fn run(program: &mut Program) -> OptimizationReport {
    let mut report = OptimizationReport::default();
    for function in &mut program.functions {
        report.call_indirect_devirtualized += devirtualize(function);
    }
    report
}

fn devirtualize(function: &mut Function) -> usize {
    let known = known_functions(function);
    if known.is_empty() {
        return 0;
    }
    let mut changed = 0;
    for block in &mut function.blocks {
        for instruction in &mut block.instructions {
            let (value, result_type, callee, arguments) = match instruction {
                Instruction::CallIndirect {
                    value,
                    result_type,
                    callee,
                    arguments,
                    ..
                } => (*value, *result_type, *callee, arguments.clone()),
                _ => continue,
            };
            let Some(symbol) = known.get(&callee).copied() else {
                continue;
            };
            *instruction = Instruction::Call {
                value,
                result_type,
                target: symbol,
                arguments,
            };
            changed += 1;
        }
    }
    changed
}

/// Maps every value that provably holds a non-capturing function pointer to its
/// symbol, following `Copy`/`Move`/`Store` through copyable locals.
fn known_functions(function: &Function) -> HashMap<ValueId, SymbolId> {
    let mut known: HashMap<ValueId, SymbolId> = HashMap::new();
    for block in &function.blocks {
        let mut local_symbol: HashMap<LocalId, SymbolId> = HashMap::new();
        for instruction in &block.instructions {
            match instruction {
                Instruction::MakeFunction { value, symbol } => {
                    known.insert(*value, *symbol);
                }
                Instruction::Copy { value, local } | Instruction::Move { value, local } => {
                    if let Some(symbol) = local_symbol.get(local).copied() {
                        known.insert(*value, symbol);
                    }
                }
                Instruction::Store { local, value } => {
                    local_symbol.remove(local);
                    if let Some(symbol) = known.get(value).copied() {
                        local_symbol.insert(*local, symbol);
                    }
                }
                Instruction::FieldStore { base, .. } | Instruction::Drop(base) => {
                    local_symbol.remove(base);
                }
                _ => {}
            }
        }
    }
    known
}
